/**
 * WebSocket Manager
 * 
 * Main entry point for WebSocket functionality in SymbioteIDE
 */

import { WebSocketIntegration } from './vscode-integration';

// Re-export WebSocketIntegration as WebSocketManager for backward compatibility
export { WebSocketIntegration as WebSocketManager } from './vscode-integration';