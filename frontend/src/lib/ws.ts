import type { WsEvent, WsEventType } from "./types";

type EventCallback = (event: WsEvent) => void;
type StatusCallback = (
  status: "connected" | "reconnecting" | "disconnected",
) => void;

let socket: WebSocket | null = null;
const listeners: Map<WsEventType, Set<EventCallback>> = new Map();
const reconnectCallbacks: Set<() => void> = new Set();
const statusCallbacks: Set<StatusCallback> = new Set();
let reconnectDelay = 1000;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let intentionallyClosed = false;

function getWsUrl(): string {
  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${location.host}/api/ws`;
}

function notifyStatus(status: "connected" | "reconnecting" | "disconnected") {
  for (const cb of statusCallbacks) cb(status);
}

export function connect(): void {
  if (
    socket?.readyState === WebSocket.OPEN ||
    socket?.readyState === WebSocket.CONNECTING
  )
    return;
  intentionallyClosed = false;

  socket = new WebSocket(getWsUrl());

  socket.onopen = () => {
    reconnectDelay = 1000;
    notifyStatus("connected");
  };

  socket.onmessage = (msg) => {
    try {
      const event: WsEvent = JSON.parse(msg.data);
      const cbs = listeners.get(event.event_type);
      if (cbs) {
        for (const cb of cbs) cb(event);
      }
    } catch {
      // Ignore malformed messages
    }
  };

  socket.onclose = () => {
    socket = null;
    if (intentionallyClosed) {
      notifyStatus("disconnected");
      return;
    }
    notifyStatus("reconnecting");
    reconnectTimer = setTimeout(() => {
      reconnectDelay = Math.min(reconnectDelay * 2, 30000);
      connect();
      // Notify reconnect listeners after successful reconnection
      const origOnOpen = socket?.onopen;
      const currentSocket = socket;
      if (currentSocket) {
        currentSocket.onopen = (ev) => {
          if (origOnOpen && typeof origOnOpen === "function")
            origOnOpen.call(currentSocket, ev);
          for (const cb of reconnectCallbacks) cb();
        };
      }
    }, reconnectDelay);
  };

  socket.onerror = () => {
    // onclose will fire after onerror, which handles reconnection
  };
}

export function disconnect(): void {
  intentionallyClosed = true;
  if (reconnectTimer) clearTimeout(reconnectTimer);
  socket?.close();
  socket = null;
}

export function onEvent(
  type: WsEventType,
  callback: EventCallback,
): () => void {
  if (!listeners.has(type)) listeners.set(type, new Set());
  listeners.get(type)!.add(callback);
  return () => listeners.get(type)?.delete(callback);
}

export function onReconnect(callback: () => void): () => void {
  reconnectCallbacks.add(callback);
  return () => reconnectCallbacks.delete(callback);
}

export function onStatus(callback: StatusCallback): () => void {
  statusCallbacks.add(callback);
  return () => statusCallbacks.delete(callback);
}
