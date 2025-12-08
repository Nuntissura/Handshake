export type DebugEventType = "doc-save" | "doc-load" | "canvas-save" | "canvas-load";

export type DebugEvent = {
  id: string;
  type: DebugEventType;
  targetId?: string;
  result: "ok" | "error";
  message?: string;
  ts: number;
};

type Listener = (events: DebugEvent[]) => void;

const MAX_EVENTS = 20;
let events: DebugEvent[] = [];
const listeners = new Set<Listener>();

export function logEvent(event: Omit<DebugEvent, "id" | "ts"> & { ts?: number }) {
  const entry: DebugEvent = {
    id: `dbg-${Date.now()}-${Math.floor(Math.random() * 10_000)}`,
    ts: event.ts ?? Date.now(),
    ...event,
  };
  events = [entry, ...events].slice(0, MAX_EVENTS);
  listeners.forEach((listener) => listener(events));
}

export function subscribeDebugEvents(listener: Listener) {
  listeners.add(listener);
  listener(events);
  return () => {
    listeners.delete(listener);
  };
}

export function getDebugEvents() {
  return events;
}
