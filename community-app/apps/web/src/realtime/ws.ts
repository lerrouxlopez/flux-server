import { useAuthStore } from "../state/auth";

type WsEventHandler = (evt: unknown) => void;

export function createRealtimeClient(params: {
  onEvent: WsEventHandler;
  onOpen?: () => void;
  onClose?: () => void;
}) {
  let ws: WebSocket | null = null;
  let stopped = false;
  let attempt = 0;

  const send = (obj: unknown) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(obj));
  };

  const stop = () => {
    stopped = true;
    ws?.close();
    ws = null;
  };

  const start = () => {
    stopped = false;
    connect();
  };

  function connect() {
    if (stopped) return;

    const token = useAuthStore.getState().accessToken ?? localStorage.getItem("access_token");
    if (!token) return;

    const url = new URL("/realtime/ws", window.location.origin);
    // Dev-only query param supported by backend when APP_ENV=local, but we prefer Authorization header.
    // Browsers can't set Authorization for WebSocket reliably, so we use query param here.
    url.searchParams.set("access_token", token);

    ws = new WebSocket(url.toString().replace(/^http/, "ws"));
    ws.onopen = () => {
      attempt = 0;
      params.onOpen?.();
    };
    ws.onclose = () => {
      params.onClose?.();
      if (stopped) return;
      attempt += 1;
      const backoffMs = Math.min(10_000, 250 * 2 ** attempt) + Math.floor(Math.random() * 250);
      window.setTimeout(connect, backoffMs);
    };
    ws.onmessage = (m) => {
      try {
        params.onEvent(JSON.parse(m.data as string));
      } catch {
        // ignore
      }
    };
  }

  return { start, stop, send };
}

