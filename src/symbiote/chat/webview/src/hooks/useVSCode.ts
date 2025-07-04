import { useRef } from 'react';
import { VSCodeAPI } from '../types';

declare function acquireVsCodeApi(): VSCodeAPI;

let vscodeApi: VSCodeAPI | null = null;

export const useVSCode = (): VSCodeAPI => {
  const apiRef = useRef<VSCodeAPI>();
  
  if (!apiRef.current) {
    if (!vscodeApi) {
      vscodeApi = acquireVsCodeApi();
    }
    apiRef.current = vscodeApi;
  }
  
  return apiRef.current;
};