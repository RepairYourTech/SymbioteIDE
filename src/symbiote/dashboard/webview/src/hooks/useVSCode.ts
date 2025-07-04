import { useRef } from 'react';

interface VSCode {
  postMessage(message: any): void;
  getState(): any;
  setState(state: any): void;
}

declare global {
  interface Window {
    acquireVsCodeApi(): VSCode;
  }
}

export function useVSCode() {
  const vscode = useRef<VSCode>(
    window.acquireVsCodeApi()
  );
  
  return vscode.current;
}