/**
 * Terminal API - Terminal management API interface
 */

export interface TerminalAPI {
  createTerminal(options?: TerminalOptions): Promise<string>;
  getTerminal(id: string): Promise<Terminal | null>;
  listTerminals(): Promise<Terminal[]>;
  
  sendCommand(terminalId: string, command: string): Promise<void>;
  sendInput(terminalId: string, data: string): Promise<void>;
  
  resize(terminalId: string, cols: number, rows: number): Promise<void>;
  clear(terminalId: string): Promise<void>;
  
  killTerminal(terminalId: string): Promise<void>;
  killAllTerminals(): Promise<void>;
  
  onOutput(terminalId: string, callback: (data: string) => void): void;
  onExit(terminalId: string, callback: (code: number) => void): void;
}

export interface TerminalOptions {
  name?: string;
  cwd?: string;
  env?: Record<string, string>;
  shell?: string;
  cols?: number;
  rows?: number;
}

export interface Terminal {
  id: string;
  name: string;
  pid?: number;
  status: 'running' | 'exited';
  exitCode?: number;
  createdAt: Date;
  cwd: string;
  env: Record<string, string>;
}

export interface TerminalOutput {
  terminalId: string;
  data: string;
  timestamp: Date;
}

export class TerminalAPIImpl implements TerminalAPI {
  private terminals: Map<string, Terminal> = new Map();
  private outputCallbacks: Map<string, Array<(data: string) => void>> = new Map();
  private exitCallbacks: Map<string, Array<(code: number) => void>> = new Map();

  async createTerminal(options?: TerminalOptions): Promise<string> {
    const id = `term_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    const terminal: Terminal = {
      id,
      name: options?.name || `Terminal ${id}`,
      status: 'running',
      createdAt: new Date(),
      cwd: options?.cwd || process.cwd(),
      env: options?.env || {}
    };

    this.terminals.set(id, terminal);
    return id;
  }

  async getTerminal(id: string): Promise<Terminal | null> {
    return this.terminals.get(id) || null;
  }

  async listTerminals(): Promise<Terminal[]> {
    return Array.from(this.terminals.values());
  }

  async sendCommand(terminalId: string, command: string): Promise<void> {
    const terminal = this.terminals.get(terminalId);
    if (!terminal) {
      throw new Error(`Terminal ${terminalId} not found`);
    }

    if (terminal.status !== 'running') {
      throw new Error(`Terminal ${terminalId} is not running`);
    }

    // Simulate command execution
    this.emitOutput(terminalId, `$ ${command}\n`);
    this.emitOutput(terminalId, `Command executed: ${command}\n`);
  }

  async sendInput(terminalId: string, data: string): Promise<void> {
    const terminal = this.terminals.get(terminalId);
    if (!terminal) {
      throw new Error(`Terminal ${terminalId} not found`);
    }

    if (terminal.status !== 'running') {
      throw new Error(`Terminal ${terminalId} is not running`);
    }

    // Echo input
    this.emitOutput(terminalId, data);
  }

  async resize(terminalId: string, cols: number, rows: number): Promise<void> {
    const terminal = this.terminals.get(terminalId);
    if (!terminal) {
      throw new Error(`Terminal ${terminalId} not found`);
    }

    // Update terminal size (simulation)
    console.log(`Terminal ${terminalId} resized to ${cols}x${rows}`);
  }

  async clear(terminalId: string): Promise<void> {
    const terminal = this.terminals.get(terminalId);
    if (!terminal) {
      throw new Error(`Terminal ${terminalId} not found`);
    }

    // Clear terminal output
    this.emitOutput(terminalId, '\x1b[2J\x1b[0f');
  }

  async killTerminal(terminalId: string): Promise<void> {
    const terminal = this.terminals.get(terminalId);
    if (!terminal) {
      throw new Error(`Terminal ${terminalId} not found`);
    }

    terminal.status = 'exited';
    terminal.exitCode = 0;
    
    // Emit exit event
    this.emitExit(terminalId, 0);
    
    // Clean up callbacks
    this.outputCallbacks.delete(terminalId);
    this.exitCallbacks.delete(terminalId);
  }

  async killAllTerminals(): Promise<void> {
    for (const terminal of this.terminals.values()) {
      if (terminal.status === 'running') {
        await this.killTerminal(terminal.id);
      }
    }
  }

  onOutput(terminalId: string, callback: (data: string) => void): void {
    if (!this.outputCallbacks.has(terminalId)) {
      this.outputCallbacks.set(terminalId, []);
    }
    this.outputCallbacks.get(terminalId)!.push(callback);
  }

  onExit(terminalId: string, callback: (code: number) => void): void {
    if (!this.exitCallbacks.has(terminalId)) {
      this.exitCallbacks.set(terminalId, []);
    }
    this.exitCallbacks.get(terminalId)!.push(callback);
  }

  private emitOutput(terminalId: string, data: string): void {
    const callbacks = this.outputCallbacks.get(terminalId);
    if (callbacks) {
      for (const callback of callbacks) {
        callback(data);
      }
    }
  }

  private emitExit(terminalId: string, code: number): void {
    const callbacks = this.exitCallbacks.get(terminalId);
    if (callbacks) {
      for (const callback of callbacks) {
        callback(code);
      }
    }
  }
}

export default TerminalAPIImpl;