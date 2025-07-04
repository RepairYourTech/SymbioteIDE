/**
 * Resource Monitor
 * 
 * Monitors and enforces resource limits for sandboxed processes
 */

import { EventEmitter } from 'events';
import * as os from 'os';
import { ResourceLimits } from '../types';

export interface ResourceMetrics {
  cpu: {
    percentage: number;
    time: number;
  };
  memory: {
    usedMB: number;
    heapMB: number;
    rssMB: number;
  };
  disk: {
    readMBps: number;
    writeMBps: number;
  };
  fileHandles: number;
  threads: number;
}

export interface LimitExceededEvent {
  type: 'cpu' | 'memory' | 'disk' | 'fileHandles' | 'threads';
  current: number;
  limit: number;
  duration?: number;
}

export class ResourceMonitor extends EventEmitter {
  private pid: number;
  private limits: ResourceLimits;
  private interval: NodeJS.Timer | null = null;
  private metrics: ResourceMetrics;
  private startTime: number;
  private lastCpuUsage: any = null;
  private limitExceededTimers: Map<string, number> = new Map();
  
  constructor(pid: number, limits: ResourceLimits) {
    super();
    this.pid = pid;
    this.limits = limits;
    this.startTime = Date.now();
    
    this.metrics = {
      cpu: { percentage: 0, time: 0 },
      memory: { usedMB: 0, heapMB: 0, rssMB: 0 },
      disk: { readMBps: 0, writeMBps: 0 },
      fileHandles: 0,
      threads: 0
    };
  }
  
  /**
   * Start monitoring
   */
  async start(): Promise<void> {
    // Get initial CPU usage
    this.lastCpuUsage = await this.getCpuUsage();
    
    // Start monitoring interval
    this.interval = setInterval(() => {
      this.checkResources();
    }, 100); // Check every 100ms
  }
  
  /**
   * Stop monitoring
   */
  async stop(): Promise<void> {
    if (this.interval) {
      clearInterval(this.interval);
      this.interval = null;
    }
  }
  
  /**
   * Get current metrics
   */
  async getMetrics(): Promise<ResourceMetrics> {
    await this.updateMetrics();
    return { ...this.metrics };
  }
  
  /**
   * Update resource limits
   */
  async updateLimits(limits: ResourceLimits): Promise<void> {
    this.limits = limits;
  }
  
  /**
   * Check resources and enforce limits
   */
  private async checkResources(): Promise<void> {
    try {
      await this.updateMetrics();
      
      // Check CPU limit
      if (this.metrics.cpu.percentage > this.limits.maxCpuPercent) {
        this.handleLimitExceeded('cpu', this.metrics.cpu.percentage, this.limits.maxCpuPercent);
      } else {
        this.clearLimitExceeded('cpu');
      }
      
      // Check memory limit
      if (this.metrics.memory.usedMB > this.limits.maxMemoryMB) {
        this.handleLimitExceeded('memory', this.metrics.memory.usedMB, this.limits.maxMemoryMB);
        
        // Try to enforce memory limit
        await this.enforceMemoryLimit();
      } else {
        this.clearLimitExceeded('memory');
      }
      
      // Check disk I/O limit
      const totalIOMBps = this.metrics.disk.readMBps + this.metrics.disk.writeMBps;
      if (totalIOMBps > this.limits.maxDiskIOMBps) {
        this.handleLimitExceeded('disk', totalIOMBps, this.limits.maxDiskIOMBps);
      } else {
        this.clearLimitExceeded('disk');
      }
      
      // Check file handles
      if (this.metrics.fileHandles > this.limits.maxFileHandles) {
        this.handleLimitExceeded('fileHandles', this.metrics.fileHandles, this.limits.maxFileHandles);
      } else {
        this.clearLimitExceeded('fileHandles');
      }
      
      // Check threads
      if (this.metrics.threads > this.limits.maxThreads) {
        this.handleLimitExceeded('threads', this.metrics.threads, this.limits.maxThreads);
      } else {
        this.clearLimitExceeded('threads');
      }
      
      // Check execution time
      const elapsedSeconds = (Date.now() - this.startTime) / 1000;
      if (elapsedSeconds > this.limits.maxExecutionTime) {
        this.emit('timeout', { elapsed: elapsedSeconds, limit: this.limits.maxExecutionTime });
      }
      
    } catch (error) {
      // Process might have died
      this.stop();
    }
  }
  
  /**
   * Update metrics
   */
  private async updateMetrics(): Promise<void> {
    // Update CPU usage
    const cpuUsage = await this.getCpuUsage();
    if (this.lastCpuUsage) {
      const timeDiff = cpuUsage.timestamp - this.lastCpuUsage.timestamp;
      const cpuDiff = cpuUsage.cpu - this.lastCpuUsage.cpu;
      
      this.metrics.cpu.percentage = (cpuDiff / timeDiff) * 100;
      this.metrics.cpu.time = cpuUsage.cpu;
    }
    this.lastCpuUsage = cpuUsage;
    
    // Update memory usage
    const memoryUsage = await this.getMemoryUsage();
    this.metrics.memory = memoryUsage;
    
    // Update disk I/O
    const diskIO = await this.getDiskIO();
    this.metrics.disk = diskIO;
    
    // Update file handles and threads
    const processInfo = await this.getProcessInfo();
    this.metrics.fileHandles = processInfo.fileHandles;
    this.metrics.threads = processInfo.threads;
  }
  
  /**
   * Get CPU usage for process
   */
  private async getCpuUsage(): Promise<{ cpu: number; timestamp: number }> {
    if (process.platform === 'win32') {
      // Windows: Use wmic
      return this.getWindowsCpuUsage();
    } else {
      // Unix: Read from /proc
      return this.getUnixCpuUsage();
    }
  }
  
  /**
   * Get Unix CPU usage
   */
  private async getUnixCpuUsage(): Promise<{ cpu: number; timestamp: number }> {
    try {
      const fs = require('fs').promises;
      const stat = await fs.readFile(`/proc/${this.pid}/stat`, 'utf8');
      const fields = stat.split(' ');
      
      // Fields 13 and 14 are utime and stime (in clock ticks)
      const utime = parseInt(fields[13], 10);
      const stime = parseInt(fields[14], 10);
      const totalTime = utime + stime;
      
      // Convert to milliseconds
      const clockTicks = os.cpus()[0].speed;
      const cpu = (totalTime / clockTicks) * 1000;
      
      return { cpu, timestamp: Date.now() };
    } catch {
      return { cpu: 0, timestamp: Date.now() };
    }
  }
  
  /**
   * Get Windows CPU usage
   */
  private async getWindowsCpuUsage(): Promise<{ cpu: number; timestamp: number }> {
    try {
      const { exec } = require('child_process');
      const { promisify } = require('util');
      const execAsync = promisify(exec);
      
      const { stdout } = await execAsync(
        `wmic process where ProcessId=${this.pid} get UserModeTime,KernelModeTime`
      );
      
      const lines = stdout.trim().split('\n');
      if (lines.length >= 2) {
        const values = lines[1].trim().split(/\s+/);
        const userTime = parseInt(values[1], 10) / 10000; // Convert to ms
        const kernelTime = parseInt(values[0], 10) / 10000;
        
        return { cpu: userTime + kernelTime, timestamp: Date.now() };
      }
    } catch {
      // Fallback
    }
    
    return { cpu: 0, timestamp: Date.now() };
  }
  
  /**
   * Get memory usage
   */
  private async getMemoryUsage(): Promise<{ usedMB: number; heapMB: number; rssMB: number }> {
    if (process.platform === 'win32') {
      return this.getWindowsMemoryUsage();
    } else {
      return this.getUnixMemoryUsage();
    }
  }
  
  /**
   * Get Unix memory usage
   */
  private async getUnixMemoryUsage(): Promise<{ usedMB: number; heapMB: number; rssMB: number }> {
    try {
      const fs = require('fs').promises;
      const status = await fs.readFile(`/proc/${this.pid}/status`, 'utf8');
      
      let rss = 0;
      const lines = status.split('\n');
      for (const line of lines) {
        if (line.startsWith('VmRSS:')) {
          rss = parseInt(line.split(/\s+/)[1], 10) / 1024; // Convert KB to MB
          break;
        }
      }
      
      return { usedMB: rss, heapMB: 0, rssMB: rss };
    } catch {
      return { usedMB: 0, heapMB: 0, rssMB: 0 };
    }
  }
  
  /**
   * Get Windows memory usage
   */
  private async getWindowsMemoryUsage(): Promise<{ usedMB: number; heapMB: number; rssMB: number }> {
    try {
      const { exec } = require('child_process');
      const { promisify } = require('util');
      const execAsync = promisify(exec);
      
      const { stdout } = await execAsync(
        `wmic process where ProcessId=${this.pid} get WorkingSetSize`
      );
      
      const lines = stdout.trim().split('\n');
      if (lines.length >= 2) {
        const bytes = parseInt(lines[1].trim(), 10);
        const mb = bytes / (1024 * 1024);
        
        return { usedMB: mb, heapMB: 0, rssMB: mb };
      }
    } catch {
      // Fallback
    }
    
    return { usedMB: 0, heapMB: 0, rssMB: 0 };
  }
  
  /**
   * Get disk I/O stats
   */
  private async getDiskIO(): Promise<{ readMBps: number; writeMBps: number }> {
    // This is platform-specific and complex to implement accurately
    // For now, return placeholder values
    // In production, would use platform-specific tools or libraries
    return { readMBps: 0, writeMBps: 0 };
  }
  
  /**
   * Get process info (file handles, threads)
   */
  private async getProcessInfo(): Promise<{ fileHandles: number; threads: number }> {
    if (process.platform === 'win32') {
      return this.getWindowsProcessInfo();
    } else {
      return this.getUnixProcessInfo();
    }
  }
  
  /**
   * Get Unix process info
   */
  private async getUnixProcessInfo(): Promise<{ fileHandles: number; threads: number }> {
    try {
      const fs = require('fs').promises;
      
      // Count file descriptors
      const fds = await fs.readdir(`/proc/${this.pid}/fd`);
      const fileHandles = fds.length;
      
      // Count threads
      const tasks = await fs.readdir(`/proc/${this.pid}/task`);
      const threads = tasks.length;
      
      return { fileHandles, threads };
    } catch {
      return { fileHandles: 0, threads: 1 };
    }
  }
  
  /**
   * Get Windows process info
   */
  private async getWindowsProcessInfo(): Promise<{ fileHandles: number; threads: number }> {
    try {
      const { exec } = require('child_process');
      const { promisify } = require('util');
      const execAsync = promisify(exec);
      
      const { stdout } = await execAsync(
        `wmic process where ProcessId=${this.pid} get HandleCount,ThreadCount`
      );
      
      const lines = stdout.trim().split('\n');
      if (lines.length >= 2) {
        const values = lines[1].trim().split(/\s+/);
        
        return {
          fileHandles: parseInt(values[0], 10),
          threads: parseInt(values[1], 10)
        };
      }
    } catch {
      // Fallback
    }
    
    return { fileHandles: 0, threads: 1 };
  }
  
  /**
   * Handle limit exceeded
   */
  private handleLimitExceeded(
    type: LimitExceededEvent['type'],
    current: number,
    limit: number
  ): void {
    const key = `${type}-exceeded`;
    
    if (!this.limitExceededTimers.has(key)) {
      this.limitExceededTimers.set(key, Date.now());
      
      this.emit('limit-exceeded', {
        type,
        current,
        limit
      });
    } else {
      // Check how long limit has been exceeded
      const duration = Date.now() - this.limitExceededTimers.get(key)!;
      
      this.emit('limit-exceeded', {
        type,
        current,
        limit,
        duration
      });
    }
  }
  
  /**
   * Clear limit exceeded status
   */
  private clearLimitExceeded(type: string): void {
    const key = `${type}-exceeded`;
    this.limitExceededTimers.delete(key);
  }
  
  /**
   * Try to enforce memory limit
   */
  private async enforceMemoryLimit(): Promise<void> {
    // Platform-specific memory limiting
    if (process.platform !== 'win32') {
      try {
        const { exec } = require('child_process');
        const { promisify } = require('util');
        const execAsync = promisify(exec);
        
        // Use cgroups to limit memory (requires appropriate permissions)
        const cgroupPath = `/sys/fs/cgroup/memory/mcp_sandbox_${this.pid}`;
        
        // This would require root/sudo permissions in most cases
        // In production, would use a privileged helper process
        await execAsync(`mkdir -p ${cgroupPath}`);
        await execAsync(`echo ${this.pid} > ${cgroupPath}/cgroup.procs`);
        await execAsync(`echo ${this.limits.maxMemoryMB * 1024 * 1024} > ${cgroupPath}/memory.limit_in_bytes`);
      } catch {
        // Can't enforce hard limit, rely on monitoring
      }
    }
  }
}