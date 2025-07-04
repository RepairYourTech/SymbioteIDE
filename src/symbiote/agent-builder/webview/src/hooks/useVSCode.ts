/**
 * VS Code API Hook
 * 
 * Provides access to VS Code API in webview
 */

interface VSCodeAPI {
  postMessage(message: any): void;
  getState(): any;
  setState(state: any): void;
}

declare global {
  interface Window {
    acquireVsCodeApi(): VSCodeAPI;
  }
}

let vscodeApi: VSCodeAPI | null = null;

export function useVSCode(): VSCodeAPI {
  if (!vscodeApi) {
    vscodeApi = window.acquireVsCodeApi();
  }
  
  return vscodeApi;
}