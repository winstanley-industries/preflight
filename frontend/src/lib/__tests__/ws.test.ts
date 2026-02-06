import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { WsEvent } from "../types";

// Mock WebSocket class that captures callbacks
class MockWebSocket {
  static OPEN = 1;
  static CONNECTING = 0;
  static CLOSING = 2;
  static CLOSED = 3;

  static instances: MockWebSocket[] = [];

  readyState = MockWebSocket.CONNECTING;
  onopen: ((ev: Event) => void) | null = null;
  onclose: ((ev: CloseEvent) => void) | null = null;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  url: string;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    // Fire onclose synchronously, matching real browser behavior
    // where onclose fires after close() is called
    this.onclose?.(new CloseEvent("close"));
  }

  send = vi.fn();

  simulateOpen() {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.(new Event("open"));
  }

  simulateMessage(data: string) {
    this.onmessage?.(new MessageEvent("message", { data }));
  }

  simulateClose() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.(new CloseEvent("close"));
  }

  simulateError() {
    this.onerror?.(new Event("error"));
  }
}

// Install mock before importing ws module
vi.stubGlobal("WebSocket", MockWebSocket);

// Dynamic import after mock is installed
const { connect, disconnect, onEvent, onReconnect, onStatus } =
  await import("../ws");

describe("ws", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    MockWebSocket.instances = [];
    disconnect(); // reset module state
  });

  afterEach(() => {
    disconnect();
    vi.useRealTimers();
  });

  it("connect() creates a WebSocket with correct URL", () => {
    connect();
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].url).toContain("/api/ws");
  });

  it("connect() is a no-op when already connected", () => {
    connect();
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();
    connect(); // second call
    expect(MockWebSocket.instances).toHaveLength(1);
  });

  it("onStatus notifies 'connected' on open", () => {
    const statusCb = vi.fn();
    onStatus(statusCb);
    connect();
    MockWebSocket.instances[0].simulateOpen();
    expect(statusCb).toHaveBeenCalledWith("connected");
  });

  it("onStatus notifies 'disconnected' on intentional close", () => {
    const statusCb = vi.fn();
    onStatus(statusCb);
    connect();
    MockWebSocket.instances[0].simulateOpen();
    disconnect();
    expect(statusCb).toHaveBeenCalledWith("disconnected");
  });

  it("onStatus notifies 'reconnecting' on unintentional close", () => {
    const statusCb = vi.fn();
    onStatus(statusCb);
    connect();
    MockWebSocket.instances[0].simulateOpen();
    statusCb.mockClear();
    MockWebSocket.instances[0].simulateClose();
    expect(statusCb).toHaveBeenCalledWith("reconnecting");
  });

  it("onEvent dispatches events to matching listeners", () => {
    const cb = vi.fn();
    onEvent("review_created", cb);
    connect();
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    const event: WsEvent = {
      event_type: "review_created",
      review_id: "r-1",
      payload: { id: "r-1" },
      timestamp: "2026-01-01T00:00:00Z",
    };
    ws.simulateMessage(JSON.stringify(event));
    expect(cb).toHaveBeenCalledWith(event);
  });

  it("onEvent does not dispatch to non-matching listeners", () => {
    const cb = vi.fn();
    onEvent("thread_created", cb);
    connect();
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    const event: WsEvent = {
      event_type: "review_created",
      review_id: "r-1",
      payload: {},
      timestamp: "2026-01-01T00:00:00Z",
    };
    ws.simulateMessage(JSON.stringify(event));
    expect(cb).not.toHaveBeenCalled();
  });

  it("unsubscribe function removes the listener", () => {
    const cb = vi.fn();
    const unsub = onEvent("review_created", cb);
    unsub();
    connect();
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();
    ws.simulateMessage(
      JSON.stringify({
        event_type: "review_created",
        review_id: "r-1",
        payload: {},
        timestamp: "",
      }),
    );
    expect(cb).not.toHaveBeenCalled();
  });

  it("malformed messages are silently ignored", () => {
    const cb = vi.fn();
    onEvent("review_created", cb);
    connect();
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();
    ws.simulateMessage("not json");
    expect(cb).not.toHaveBeenCalled();
  });

  it("auto-reconnects with exponential backoff", () => {
    connect();
    const ws1 = MockWebSocket.instances[0];
    ws1.simulateOpen();
    ws1.simulateClose(); // unintentional close

    // After 1000ms, should reconnect
    expect(MockWebSocket.instances).toHaveLength(1);
    vi.advanceTimersByTime(1000);
    expect(MockWebSocket.instances).toHaveLength(2);

    // Close again without opening — delay was doubled to 2000 inside the
    // timer callback that created ws2, so next reconnect waits 2000ms
    const ws2 = MockWebSocket.instances[1];
    ws2.simulateClose();
    vi.advanceTimersByTime(1999);
    expect(MockWebSocket.instances).toHaveLength(2);
    vi.advanceTimersByTime(1);
    expect(MockWebSocket.instances).toHaveLength(3);
  });

  it("resets reconnect delay on successful connection", () => {
    connect();
    const ws1 = MockWebSocket.instances[0];
    ws1.simulateOpen();
    ws1.simulateClose();

    // Advance past first reconnect
    vi.advanceTimersByTime(1000);
    const ws2 = MockWebSocket.instances[1];
    ws2.simulateOpen(); // resets delay
    ws2.simulateClose();

    // Should reconnect after 1000ms (reset), not 2000ms
    vi.advanceTimersByTime(1000);
    expect(MockWebSocket.instances).toHaveLength(3);
  });

  it("disconnect() prevents auto-reconnect", () => {
    connect();
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();
    disconnect();

    vi.advanceTimersByTime(60000);
    expect(MockWebSocket.instances).toHaveLength(1);
  });

  it("onReconnect callbacks fire on reconnection", () => {
    const reconnectCb = vi.fn();
    onReconnect(reconnectCb);
    connect();
    const ws1 = MockWebSocket.instances[0];
    ws1.simulateOpen();
    ws1.simulateClose();

    vi.advanceTimersByTime(1000);
    const ws2 = MockWebSocket.instances[1];
    ws2.simulateOpen(); // triggers reconnect callbacks
    expect(reconnectCb).toHaveBeenCalledTimes(1);
  });

  it("onStatus unsubscribe stops notifications", () => {
    const statusCb = vi.fn();
    const unsub = onStatus(statusCb);
    unsub();
    connect();
    MockWebSocket.instances[0].simulateOpen();
    expect(statusCb).not.toHaveBeenCalled();
  });

  it("onReconnect unsubscribe stops notifications", () => {
    const cb = vi.fn();
    const unsub = onReconnect(cb);
    unsub();
    connect();
    MockWebSocket.instances[0].simulateOpen();
    MockWebSocket.instances[0].simulateClose();
    vi.advanceTimersByTime(1000);
    MockWebSocket.instances[1].simulateOpen();
    expect(cb).not.toHaveBeenCalled();
  });
});
