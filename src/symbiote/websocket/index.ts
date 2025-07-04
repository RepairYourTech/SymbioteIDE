/**
 * WebSocket Module Exports
 */

export * from './types';
export { WebSocketServer } from './websocket-server';
export { WebSocketClient } from './websocket-client';
export { RoomManager } from './room-manager';
export { StreamManager } from './stream-manager';
export { AuthManager } from './auth-manager';
export { WebSocketIntegration } from './vscode-integration';

// Re-export commonly used types
export type {
  WebSocketServerConfig,
  WebSocketClientConfig,
  SocketAuth,
  SocketClient,
  AgentStatus,
  TaskProgress,
  StreamResponse,
  CollabCursor,
  CollabSelection,
  CollabEdit,
  SystemMetrics,
  Room,
  SocketMessage
} from './types';

export {
  SocketNamespace,
  SocketEvent,
  RoomType
} from './types';