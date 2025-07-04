/**
 * Platform Detector - Detects OS, Docker, and system capabilities
 */

import * as os from 'os';
import * as fs from 'fs/promises';
import * as path from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';
import { Logger } from '../utils/logger';

const execAsync = promisify(exec);

export enum Platform {
  Windows = 'windows',
  WindowsWSL = 'wsl',
  MacOS = 'macos',
  Linux = 'linux',
  Unknown = 'unknown'
}

export enum DockerType {
  DockerDesktop = 'docker-desktop',
  DockerCE = 'docker-ce',
  Podman = 'podman',
  None = 'none'
}

export interface SystemInfo {
  platform: Platform;
  platformVersion: string;
  architecture: string;
  isWSL: boolean;
  isWSL2: boolean;
  dockerType: DockerType;
  dockerVersion?: string;
  dockerComposeVersion?: string;
  cpuCount: number;
  totalMemoryGB: number;
  freeMemoryGB: number;
  availableDiskGB: number;
  hasGPU: boolean;
  gpuInfo?: string;
  localLLM?: {
    type: 'ollama' | 'lm-studio' | 'llama-cpp' | 'none';
    endpoint?: string;
    models?: string[];
  };
}

export class PlatformDetector {
  private logger = new Logger('PlatformDetector');
  
  /**
   * Detect all system information
   */
  async detectSystem(): Promise<SystemInfo> {
    const platform = await this.detectPlatform();
    const docker = await this.detectDocker();
    const resources = await this.detectResources();
    const gpu = await this.detectGPU();
    const localLLM = await this.detectLocalLLM();
    
    return {
      platform: platform.platform,
      platformVersion: platform.version,
      architecture: os.arch(),
      isWSL: platform.isWSL,
      isWSL2: platform.isWSL2,
      dockerType: docker.type,
      dockerVersion: docker.version,
      dockerComposeVersion: docker.composeVersion,
      cpuCount: os.cpus().length,
      totalMemoryGB: resources.totalMemoryGB,
      freeMemoryGB: resources.freeMemoryGB,
      availableDiskGB: resources.availableDiskGB,
      hasGPU: gpu.hasGPU,
      gpuInfo: gpu.info,
      localLLM
    };
  }
  
  /**
   * Detect operating system and platform
   */
  private async detectPlatform(): Promise<{
    platform: Platform;
    version: string;
    isWSL: boolean;
    isWSL2: boolean;
  }> {
    const platform = os.platform();
    const release = os.release();
    
    // Check for WSL
    let isWSL = false;
    let isWSL2 = false;
    
    if (platform === 'linux') {
      try {
        const procVersion = await fs.readFile('/proc/version', 'utf-8');
        isWSL = procVersion.toLowerCase().includes('microsoft');
        
        if (isWSL) {
          // Check if WSL2
          const wslInterop = await fs.readFile('/proc/sys/fs/binfmt_misc/WSLInterop', 'utf-8')
            .catch(() => '');
          isWSL2 = wslInterop.includes('WSLInterop');
          
          // Alternative WSL2 check
          if (!isWSL2) {
            const kernelVersion = procVersion.match(/(\d+\.\d+\.\d+)/);
            if (kernelVersion) {
              const [major, minor] = kernelVersion[1].split('.').map(Number);
              isWSL2 = major >= 5 || (major === 4 && minor >= 19);
            }
          }
        }
      } catch (error) {
        // Not WSL
      }
    }
    
    let detectedPlatform: Platform;
    
    switch (platform) {
      case 'win32':
        detectedPlatform = Platform.Windows;
        break;
      case 'darwin':
        detectedPlatform = Platform.MacOS;
        break;
      case 'linux':
        detectedPlatform = isWSL ? Platform.WindowsWSL : Platform.Linux;
        break;
      default:
        detectedPlatform = Platform.Unknown;
    }
    
    return {
      platform: detectedPlatform,
      version: release,
      isWSL,
      isWSL2
    };
  }
  
  /**
   * Detect Docker installation
   */
  private async detectDocker(): Promise<{
    type: DockerType;
    version?: string;
    composeVersion?: string;
  }> {
    try {
      // Check for Docker
      const { stdout: dockerVersion } = await execAsync('docker --version');
      const versionMatch = dockerVersion.match(/Docker version ([\d.]+)/);
      const version = versionMatch ? versionMatch[1] : undefined;
      
      // Check Docker type
      let dockerType = DockerType.DockerCE;
      
      try {
        const { stdout: dockerInfo } = await execAsync('docker info --format json');
        const info = JSON.parse(dockerInfo);
        
        if (info.ServerVersion?.includes('Desktop')) {
          dockerType = DockerType.DockerDesktop;
        }
      } catch (error) {
        // Fallback detection
        if (os.platform() === 'win32' || os.platform() === 'darwin') {
          // On Windows and macOS, it's likely Docker Desktop
          dockerType = DockerType.DockerDesktop;
        }
      }
      
      // Check Docker Compose
      let composeVersion: string | undefined;
      try {
        // Try Docker Compose V2 first
        const { stdout: compose2 } = await execAsync('docker compose version');
        const compose2Match = compose2.match(/version v?([\d.]+)/);
        composeVersion = compose2Match ? compose2Match[1] : undefined;
      } catch {
        // Try Docker Compose V1
        try {
          const { stdout: compose1 } = await execAsync('docker-compose --version');
          const compose1Match = compose1.match(/version ([\d.]+)/);
          composeVersion = compose1Match ? compose1Match[1] : undefined;
        } catch {
          // No compose
        }
      }
      
      return { type: dockerType, version, composeVersion };
      
    } catch (error) {
      // Check for Podman as alternative
      try {
        const { stdout: podmanVersion } = await execAsync('podman --version');
        const versionMatch = podmanVersion.match(/podman version ([\d.]+)/);
        return {
          type: DockerType.Podman,
          version: versionMatch ? versionMatch[1] : undefined
        };
      } catch {
        return { type: DockerType.None };
      }
    }
  }
  
  /**
   * Detect system resources
   */
  private async detectResources(): Promise<{
    totalMemoryGB: number;
    freeMemoryGB: number;
    availableDiskGB: number;
  }> {
    const totalMemoryGB = os.totalmem() / (1024 * 1024 * 1024);
    const freeMemoryGB = os.freemem() / (1024 * 1024 * 1024);
    
    // Detect available disk space
    let availableDiskGB = 0;
    
    try {
      const platform = os.platform();
      
      if (platform === 'win32') {
        const { stdout } = await execAsync('wmic logicaldisk get size,freespace,caption');
        const lines = stdout.trim().split('\n').slice(1); // Skip header
        
        for (const line of lines) {
          const parts = line.trim().split(/\s+/);
          if (parts.length >= 3 && parts[0] === 'C:') {
            const freeSpace = parseInt(parts[1], 10);
            if (!isNaN(freeSpace)) {
              availableDiskGB = freeSpace / (1024 * 1024 * 1024);
              break;
            }
          }
        }
      } else {
        // Unix-like systems
        const { stdout } = await execAsync("df -BG / | tail -1 | awk '{print $4}'");
        const match = stdout.match(/(\d+)/);
        if (match) {
          availableDiskGB = parseInt(match[1], 10);
        }
      }
    } catch (error) {
      this.logger.warn('Failed to detect disk space', error);
    }
    
    return {
      totalMemoryGB: Math.round(totalMemoryGB * 10) / 10,
      freeMemoryGB: Math.round(freeMemoryGB * 10) / 10,
      availableDiskGB: Math.round(availableDiskGB * 10) / 10
    };
  }
  
  /**
   * Detect GPU availability
   */
  private async detectGPU(): Promise<{
    hasGPU: boolean;
    info?: string;
  }> {
    try {
      // Try nvidia-smi first
      const { stdout } = await execAsync('nvidia-smi --query-gpu=name,memory.total --format=csv,noheader');
      const gpuInfo = stdout.trim();
      
      if (gpuInfo) {
        return {
          hasGPU: true,
          info: `NVIDIA: ${gpuInfo}`
        };
      }
    } catch {
      // Not NVIDIA
    }
    
    try {
      // Try AMD
      const platform = os.platform();
      
      if (platform === 'linux') {
        const { stdout } = await execAsync('lspci | grep -i vga');
        if (stdout.toLowerCase().includes('amd') || stdout.toLowerCase().includes('radeon')) {
          return {
            hasGPU: true,
            info: 'AMD GPU detected'
          };
        }
      } else if (platform === 'darwin') {
        // macOS GPU detection
        const { stdout } = await execAsync('system_profiler SPDisplaysDataType');
        if (stdout.includes('Chipset Model')) {
          const match = stdout.match(/Chipset Model: (.+)/);
          if (match) {
            return {
              hasGPU: true,
              info: `macOS GPU: ${match[1]}`
            };
          }
        }
      } else if (platform === 'win32') {
        const { stdout } = await execAsync('wmic path win32_VideoController get name');
        const lines = stdout.trim().split('\n').slice(1); // Skip header
        const gpu = lines.find(line => 
          line.includes('NVIDIA') || 
          line.includes('AMD') || 
          line.includes('Radeon')
        );
        
        if (gpu) {
          return {
            hasGPU: true,
            info: gpu.trim()
          };
        }
      }
    } catch {
      // No GPU detection
    }
    
    return { hasGPU: false };
  }
  
  /**
   * Detect local LLM installations
   */
  private async detectLocalLLM(): Promise<SystemInfo['localLLM']> {
    // Check Ollama
    try {
      const { stdout } = await execAsync('ollama list');
      const models = stdout
        .split('\n')
        .slice(1) // Skip header
        .filter(line => line.trim())
        .map(line => line.split(/\s+/)[0])
        .filter(model => model);
      
      return {
        type: 'ollama',
        endpoint: 'http://localhost:11434',
        models
      };
    } catch {
      // Ollama not found
    }
    
    // Check LM Studio
    try {
      // LM Studio typically runs on port 1234
      const response = await fetch('http://localhost:1234/v1/models')
        .then(res => res.json())
        .catch(() => null);
      
      if (response?.data) {
        return {
          type: 'lm-studio',
          endpoint: 'http://localhost:1234',
          models: response.data.map((m: any) => m.id)
        };
      }
    } catch {
      // LM Studio not found
    }
    
    // Check llama.cpp server
    try {
      const response = await fetch('http://localhost:8080/health')
        .then(res => res.json())
        .catch(() => null);
      
      if (response?.status === 'ok') {
        return {
          type: 'llama-cpp',
          endpoint: 'http://localhost:8080'
        };
      }
    } catch {
      // llama.cpp not found
    }
    
    return { type: 'none' };
  }
  
  /**
   * Get Docker Compose command for platform
   */
  getDockerComposeCommand(): string[] {
    const platform = os.platform();
    
    // Base command
    const baseCmd = ['docker', 'compose'];
    
    // Add platform-specific overrides
    if (platform === 'win32') {
      return [...baseCmd, '-f', 'docker-compose.yml', '-f', 'docker-compose.windows.yml'];
    } else if (platform === 'darwin') {
      return [...baseCmd, '-f', 'docker-compose.yml', '-f', 'docker-compose.macos.yml'];
    }
    
    return baseCmd;
  }
  
  /**
   * Check if system meets minimum requirements
   */
  checkRequirements(info: SystemInfo): {
    meets: boolean;
    warnings: string[];
    errors: string[];
  } {
    const warnings: string[] = [];
    const errors: string[] = [];
    
    // Check Docker
    if (info.dockerType === DockerType.None) {
      errors.push('Docker is not installed. Please install Docker Desktop or Docker CE.');
    } else if (!info.dockerComposeVersion) {
      errors.push('Docker Compose is not installed. Please install Docker Compose V2.');
    }
    
    // Check memory
    if (info.totalMemoryGB < 8) {
      errors.push(`System has only ${info.totalMemoryGB}GB RAM. Minimum 8GB required.`);
    } else if (info.totalMemoryGB < 16) {
      warnings.push(`System has ${info.totalMemoryGB}GB RAM. 16GB or more recommended for best performance.`);
    }
    
    // Check free memory
    if (info.freeMemoryGB < 4) {
      warnings.push(`Only ${info.freeMemoryGB}GB RAM available. Close other applications for better performance.`);
    }
    
    // Check disk space
    if (info.availableDiskGB < 20) {
      errors.push(`Only ${info.availableDiskGB}GB disk space available. Minimum 20GB required.`);
    } else if (info.availableDiskGB < 50) {
      warnings.push(`Only ${info.availableDiskGB}GB disk space available. 50GB or more recommended.`);
    }
    
    // Platform-specific checks
    if (info.isWSL && !info.isWSL2) {
      errors.push('WSL1 detected. WSL2 is required for Docker Desktop.');
    }
    
    if (info.platform === Platform.Windows && info.dockerType === DockerType.DockerDesktop) {
      warnings.push('Enable WSL2 backend in Docker Desktop for better performance.');
    }
    
    // GPU checks
    if (!info.hasGPU && !info.localLLM) {
      warnings.push('No GPU detected and no local LLM found. Local AI features will be limited.');
    }
    
    return {
      meets: errors.length === 0,
      warnings,
      errors
    };
  }
}

// Export singleton instance
export const platformDetector = new PlatformDetector();