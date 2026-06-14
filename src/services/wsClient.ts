/**
 * WebSocket 客户端
 *
 * 连接后端 WebSocket 服务器 (ws://127.0.0.1:8765),
 * 支持认证、事件订阅、心跳、自动重连。
 */

import { PetState, UnifiedAgentEvent, type WSMessage, type WSMessagePayload, type EventType } from "@agent-pet-hub/protocol";

// ─── 配置 ──────────────────────────────────────────────────────────────

const DEFAULT_WS_URL = "ws://127.0.0.1:8765";

/**
 * 生成随机认证 token（使用 ULID 格式，26 字符）。
 * 用于首次启动时生成随机 token，生产环境应通过 get_settings 配置覆盖。
 */
export function generateToken(): string {
  const timePart = Date.now().toString(36).padStart(10, "0").slice(-10);
  const randomPart = crypto.getRandomValues(new Uint8Array(13));
  const randomStr = Array.from(randomPart)
    .map((b) => b.toString(36))
    .join("")
    .slice(0, 16)
    .padEnd(16, "0");
  return timePart + randomStr;
}

const RECONNECT_INTERVAL_MS = 3000;
const MAX_RECONNECT_ATTEMPTS = 10;
const HEARTBEAT_INTERVAL_MS = 30000;

// ─── 事件处理器类型 ───────────────────────────────────────────────────

type EventCallback = (event: UnifiedAgentEvent) => void;
type StateChangeCallback = (oldState: PetState, newState: PetState) => void;
type ConnectedCallback = () => void;
type DisconnectedCallback = (reason?: string) => void;

// ─── WebSocketClient 类 ───────────────────────────────────────────────

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private url: string;
  private authToken: string;
  private reconnectAttempts = 0;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private isManuallyClosed = false;

  // Event listeners
  private eventCallbacks: Set<EventCallback> = new Set();
  private stateChangeCallbacks: Set<StateChangeCallback> = new Set();
  private connectedCallbacks: Set<ConnectedCallback> = new Set();
  private disconnectedCallbacks: Set<DisconnectedCallback> = new Set();

  constructor(url: string = DEFAULT_WS_URL, authToken: string = "") {
    this.url = url;
    this.authToken = authToken;
  }

  /** 连接 WebSocket 服务器 */
  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) return;

    this.isManuallyClosed = false;
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      this.reconnectAttempts = 0;
      this._authenticated = false;
      this.authenticate();
    };

    this.ws.onmessage = (event) => {
      try {
        const message: WSMessage = JSON.parse(event.data);
        this.handleMessage(message);
      } catch (error) {
        console.error("Failed to parse WS message:", error);
      }
    };

    this.ws.onclose = (event) => {
      this.triggerDisconnected(event.reason || "Connection closed");
      if (!this.isManuallyClosed) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = (error) => {
      console.error("WebSocket error:", error);
    };
  }

  /** 断开连接 */
  disconnect(): void {
    this.isManuallyClosed = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  /** 发送事件到 Agent */
  sendEvent(event: Partial<UnifiedAgentEvent>): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    // 认证后才能发送事件
    if (!this._authenticated) return;
    const message: WSMessage = {
      type: "command",
      timestamp: new Date().toISOString(),
      payload: event as unknown as WSMessagePayload,
    };
    this.ws.send(JSON.stringify(message));
  }

  /** 订阅事件类型 */
  subscribe(eventTypes: EventType[]): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    const message: WSMessage = {
      type: "subscribe",
      timestamp: new Date().toISOString(),
      payload: { eventTypes },
    };
    this.ws.send(JSON.stringify(message));
  }

  /** 事件回调注册 */
  onEvent(callback: EventCallback): () => void {
    this.eventCallbacks.add(callback);
    return () => this.eventCallbacks.delete(callback);
  }

  onStateChange(callback: StateChangeCallback): () => void {
    this.stateChangeCallbacks.add(callback);
    return () => this.stateChangeCallbacks.delete(callback);
  }

  onConnected(callback: ConnectedCallback): () => void {
    this.connectedCallbacks.add(callback);
    return () => this.connectedCallbacks.delete(callback);
  }

  onDisconnected(callback: DisconnectedCallback): () => void {
    this.disconnectedCallbacks.add(callback);
    return () => this.disconnectedCallbacks.delete(callback);
  }

  /** 检查连接状态 */
  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN && this._authenticated;
  }

  // ─── 私有方法 ──────────────────────────────────────────────────────

  private _authenticated = false;

  private authenticate(): void {
    const message: WSMessage = {
      type: "auth",
      timestamp: new Date().toISOString(),
      payload: { token: this.authToken },
    };
    this.ws!.send(JSON.stringify(message));
  }

  private handleMessage(message: WSMessage): void {
    switch (message.type) {
      case "auth_ack": {
        const payload = message.payload as { authorized: boolean };
        if (payload.authorized) {
          this._authenticated = true;
          this.startHeartbeat();
          this.triggerConnected();
        } else {
          // 认证失败：关闭连接并触发重连
          this._authenticated = false;
          console.error("WS auth failed, reconnecting...");
          this.ws!.close(4001, "Authentication failed");
        }
        break;
      }
      case "event": {
        const payload = message.payload as { event: UnifiedAgentEvent };
        if (payload.event) {
          // 过滤 raw 字段，防止敏感数据泄露到回调（raw 最大 8KB 原始 JSON）
          const { raw, ...safeEvent } = payload.event;
          this.eventCallbacks.forEach(cb => cb(safeEvent as UnifiedAgentEvent));
          // 状态变更检测（后续可加入状态比较逻辑）
          const _prevState = (safeEvent as unknown as { petState: PetState }).petState as unknown as PetState;
          void _prevState;
          break;
        }
        break;
      }
      case "pong":
        break;
      case "error": {
        const payload = message.payload as { message: string };
        console.error("WS server error:", payload?.message);
        break;
      }
    }
  }

  private startHeartbeat(): void {
    this.heartbeatTimer = setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        const message: WSMessage = {
          type: "ping",
          timestamp: new Date().toISOString(),
          payload: {} as WSMessagePayload,
        };
        this.ws.send(JSON.stringify(message));
      }
    }, HEARTBEAT_INTERVAL_MS);
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
      console.error("Max reconnect attempts reached");
      return;
    }
    const delay = RECONNECT_INTERVAL_MS * Math.pow(2, this.reconnectAttempts);
    this.reconnectAttempts++;
    console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
    this.reconnectTimer = setTimeout(() => this.connect(), delay);
  }

  private triggerConnected(): void {
    this.connectedCallbacks.forEach(cb => cb());
  }

  private triggerDisconnected(reason: string): void {
    this.disconnectedCallbacks.forEach(cb => cb(reason));
  }
}
