import { render, screen, cleanup } from "@testing-library/svelte";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ConnectionStatus from "../ConnectionStatus.svelte";

// Mock ws module — capture the onStatus callback so we can trigger status changes
let capturedStatusCallback: ((status: string) => void) | null = null;

vi.mock("../../lib/ws", () => ({
  onStatus: vi.fn((cb: (status: string) => void) => {
    capturedStatusCallback = cb;
    return () => {
      capturedStatusCallback = null;
    };
  }),
}));

describe("ConnectionStatus", () => {
  beforeEach(() => {
    cleanup();
    capturedStatusCallback = null;
  });

  it("starts in disconnected state with red dot and text", () => {
    render(ConnectionStatus);
    expect(screen.getByText("Disconnected")).toBeInTheDocument();
    const dot = document.querySelector(".bg-red-500");
    expect(dot).not.toBeNull();
  });

  it("shows green dot and no text when connected", async () => {
    render(ConnectionStatus);
    capturedStatusCallback?.("connected");
    await vi.waitFor(() => {
      expect(screen.queryByText("Disconnected")).not.toBeInTheDocument();
      expect(screen.queryByText(/Reconnecting/)).not.toBeInTheDocument();
    });
    const dot = document.querySelector(".bg-green-500");
    expect(dot).not.toBeNull();
  });

  it("shows yellow pulsing dot and 'Reconnecting' text when reconnecting", async () => {
    render(ConnectionStatus);
    capturedStatusCallback?.("reconnecting");
    await vi.waitFor(() => {
      expect(screen.getByText("Reconnecting\u2026")).toBeInTheDocument();
    });
    const dot = document.querySelector(".bg-yellow-500");
    expect(dot).not.toBeNull();
    expect(dot?.classList.contains("animate-pulse")).toBe(true);
  });
});
