/**
 * Logger Utility
 * 
 * Simple logger for SymbioteIDE components
 */

export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3
}

export class Logger {
  private context: string;
  private level: LogLevel;
  
  constructor(context: string, level: LogLevel = LogLevel.INFO) {
    this.context = context;
    this.level = level;
  }
  
  private log(level: LogLevel, message: string, ...args: any[]): void {
    if (level < this.level) return;
    
    const timestamp = new Date().toISOString();
    const levelStr = LogLevel[level];
    const prefix = `[${timestamp}] [${levelStr}] [${this.context}]`;
    
    switch (level) {
      case LogLevel.DEBUG:
        console.debug(prefix, message, ...args);
        break;
      case LogLevel.INFO:
        console.info(prefix, message, ...args);
        break;
      case LogLevel.WARN:
        console.warn(prefix, message, ...args);
        break;
      case LogLevel.ERROR:
        console.error(prefix, message, ...args);
        break;
    }
  }
  
  debug(message: string, ...args: any[]): void {
    this.log(LogLevel.DEBUG, message, ...args);
  }
  
  info(message: string, ...args: any[]): void {
    this.log(LogLevel.INFO, message, ...args);
  }
  
  warn(message: string, ...args: any[]): void {
    this.log(LogLevel.WARN, message, ...args);
  }
  
  error(message: string, error?: any, ...args: any[]): void {
    if (error instanceof Error) {
      this.log(LogLevel.ERROR, `${message}: ${error.message}`, error.stack, ...args);
    } else {
      this.log(LogLevel.ERROR, message, error, ...args);
    }
  }
  
  setLevel(level: LogLevel): void {
    this.level = level;
  }
}