/// <reference types="vite/client" />

// Tauri API type declarations
declare module '@tauri-apps/api/core' {
  export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

declare module '@tauri-apps/api/event' {
  export interface Event<T> {
    event: string;
    windowLabel: string;
    payload: T;
    id: number;
  }
  
  export function listen<T>(
    event: string,
    handler: (event: Event<T>) => void
  ): Promise<() => void>;
  
  export function emit(event: string, payload?: unknown): Promise<void>;
}

declare module '@tauri-apps/api/window' {
  export class WebviewWindow {
    constructor(label: string, options?: any);
    static getByLabel(label: string): WebviewWindow | null;
    listen<T>(event: string, handler: (event: Event<T>) => void): Promise<() => void>;
    emit(event: string, payload?: unknown): Promise<void>;
  }
  
  export function getCurrentWebviewWindow(): WebviewWindow;
}

// Global type declarations
interface Window {
  __TAURI__?: {
    invoke: (cmd: string, args?: Record<string, unknown>) => Promise<any>;
  };
}
