/**
 * Tailwind Processor
 * 
 * Handles Tailwind CSS processing for notebook cells with JIT compilation
 */

import { Logger } from '../../utils/logger';
import { BuildConfig, CellMetadata } from '../types';

export interface TailwindConfig {
  content: string[];
  theme?: any;
  plugins?: any[];
  darkMode?: 'media' | 'class';
  prefix?: string;
}

export class TailwindProcessor {
  private logger = new Logger('TailwindProcessor');
  private cssCache = new Map<string, string>();
  private configCache = new Map<string, TailwindConfig>();
  
  /**
   * Process Tailwind CSS for a cell
   */
  async processTailwind(
    cellId: string,
    content: string,
    metadata?: CellMetadata
  ): Promise<string> {
    try {
      // Get or create Tailwind config
      const config = this.getConfig(cellId, metadata);
      
      // Check cache
      const cacheKey = this.getCacheKey(content, config);
      const cached = this.cssCache.get(cacheKey);
      if (cached) {
        return cached;
      }
      
      // Process with JIT compiler
      const css = await this.compileTailwind(content, config);
      
      // Cache result
      this.cssCache.set(cacheKey, css);
      
      return css;
      
    } catch (error) {
      this.logger.error('Failed to process Tailwind CSS', error);
      throw error;
    }
  }
  
  /**
   * Update Tailwind configuration
   */
  updateConfig(cellId: string, config: Partial<TailwindConfig>): void {
    const existing = this.configCache.get(cellId) || this.getDefaultConfig();
    
    this.configCache.set(cellId, {
      ...existing,
      ...config
    });
    
    // Clear CSS cache for this cell
    this.clearCellCache(cellId);
  }
  
  /**
   * Get Tailwind IntelliSense data
   */
  async getIntelliSense(
    cellId: string,
    prefix: string
  ): Promise<{
    classes: string[];
    colors: Record<string, string>;
    spacing: Record<string, string>;
  }> {
    const config = this.getConfig(cellId);
    
    // Generate class list based on config
    const classes = await this.generateClassList(config, prefix);
    
    // Extract theme values
    const colors = this.extractColors(config);
    const spacing = this.extractSpacing(config);
    
    return { classes, colors, spacing };
  }
  
  /**
   * Generate CSS for component export
   */
  async generateExportCSS(
    cellId: string,
    usedClasses: string[]
  ): Promise<string> {
    const config = this.getConfig(cellId);
    
    // Generate minimal CSS containing only used classes
    const css = await this.compileTailwind(
      usedClasses.join(' '),
      {
        ...config,
        content: [{ raw: usedClasses.join(' ') }]
      }
    );
    
    return css;
  }
  
  /**
   * Clear cache
   */
  clearCache(): void {
    this.cssCache.clear();
  }
  
  /**
   * Clear cache for specific cell
   */
  clearCellCache(cellId: string): void {
    // Remove entries that belong to this cell
    const keysToDelete: string[] = [];
    
    this.cssCache.forEach((_, key) => {
      if (key.startsWith(cellId)) {
        keysToDelete.push(key);
      }
    });
    
    keysToDelete.forEach(key => this.cssCache.delete(key));
  }
  
  // Private methods
  
  private getConfig(cellId: string, metadata?: CellMetadata): TailwindConfig {
    // Check for custom config in metadata
    if (metadata?.buildConfig?.tailwindConfig) {
      return metadata.buildConfig.tailwindConfig;
    }
    
    // Check cached config
    const cached = this.configCache.get(cellId);
    if (cached) {
      return cached;
    }
    
    // Return default config
    return this.getDefaultConfig();
  }
  
  private getDefaultConfig(): TailwindConfig {
    return {
      content: [],
      theme: {
        extend: {
          colors: {
            'notebook': {
              bg: 'var(--vscode-notebook-cellEditorBackground)',
              border: 'var(--vscode-notebook-cellBorderColor)',
              hover: 'var(--vscode-notebook-cellHoverBackground)'
            }
          }
        }
      },
      darkMode: 'class',
      plugins: []
    };
  }
  
  private async compileTailwind(
    content: string,
    config: TailwindConfig
  ): Promise<string> {
    // In a real implementation, we would use the Tailwind JIT compiler
    // For now, return a mock CSS
    
    const mockCSS = `
/* Tailwind CSS - Notebook Cell */
.container {
  width: 100%;
  margin-right: auto;
  margin-left: auto;
}

.flex {
  display: flex;
}

.items-center {
  align-items: center;
}

.justify-center {
  justify-content: center;
}

.p-4 {
  padding: 1rem;
}

.bg-blue-500 {
  background-color: #3b82f6;
}

.text-white {
  color: #ffffff;
}

.rounded-lg {
  border-radius: 0.5rem;
}

.shadow-xl {
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
}

.hover\\:scale-105:hover {
  transform: scale(1.05);
}

.transition-all {
  transition-property: all;
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
  transition-duration: 150ms;
}

/* Design system integration */
${this.generateDesignSystemCSS(config)}
    `;
    
    return mockCSS.trim();
  }
  
  private generateDesignSystemCSS(config: TailwindConfig): string {
    if (!config.theme?.extend?.colors) {
      return '';
    }
    
    const css: string[] = [];
    
    // Generate color classes
    Object.entries(config.theme.extend.colors).forEach(([name, colors]) => {
      if (typeof colors === 'object') {
        Object.entries(colors).forEach(([shade, value]) => {
          css.push(`.bg-${name}-${shade} { background-color: ${value}; }`);
          css.push(`.text-${name}-${shade} { color: ${value}; }`);
          css.push(`.border-${name}-${shade} { border-color: ${value}; }`);
        });
      }
    });
    
    return css.join('\n');
  }
  
  private async generateClassList(
    config: TailwindConfig,
    prefix: string
  ): Promise<string[]> {
    // In a real implementation, this would generate all possible classes
    // based on the Tailwind config
    
    const classes = [
      // Layout
      'container', 'flex', 'grid', 'block', 'inline-block',
      'hidden', 'relative', 'absolute', 'fixed', 'sticky',
      
      // Flexbox
      'items-start', 'items-center', 'items-end',
      'justify-start', 'justify-center', 'justify-end',
      'flex-row', 'flex-col', 'flex-wrap',
      
      // Spacing
      'p-0', 'p-1', 'p-2', 'p-4', 'p-8',
      'm-0', 'm-1', 'm-2', 'm-4', 'm-8',
      
      // Colors
      'bg-white', 'bg-black', 'bg-gray-100', 'bg-gray-900',
      'text-white', 'text-black', 'text-gray-600',
      
      // Typography
      'text-xs', 'text-sm', 'text-base', 'text-lg', 'text-xl',
      'font-thin', 'font-normal', 'font-bold',
      
      // Borders
      'border', 'border-2', 'border-4',
      'rounded', 'rounded-lg', 'rounded-full',
      
      // Effects
      'shadow', 'shadow-lg', 'shadow-xl',
      'opacity-50', 'opacity-75', 'opacity-100',
      
      // Transitions
      'transition', 'transition-all', 'duration-200',
      'hover:scale-105', 'hover:opacity-75'
    ];
    
    // Filter by prefix
    if (prefix) {
      return classes.filter(c => c.startsWith(prefix));
    }
    
    return classes;
  }
  
  private extractColors(config: TailwindConfig): Record<string, string> {
    const colors: Record<string, string> = {};
    
    // Default Tailwind colors
    colors['white'] = '#ffffff';
    colors['black'] = '#000000';
    colors['gray-500'] = '#6b7280';
    colors['blue-500'] = '#3b82f6';
    colors['red-500'] = '#ef4444';
    colors['green-500'] = '#10b981';
    
    // Extract from config
    if (config.theme?.extend?.colors) {
      Object.entries(config.theme.extend.colors).forEach(([name, value]) => {
        if (typeof value === 'string') {
          colors[name] = value;
        } else if (typeof value === 'object') {
          Object.entries(value).forEach(([shade, color]) => {
            colors[`${name}-${shade}`] = color as string;
          });
        }
      });
    }
    
    return colors;
  }
  
  private extractSpacing(config: TailwindConfig): Record<string, string> {
    const spacing: Record<string, string> = {
      '0': '0px',
      '1': '0.25rem',
      '2': '0.5rem',
      '4': '1rem',
      '8': '2rem',
      '16': '4rem',
      '32': '8rem'
    };
    
    // Extract from config
    if (config.theme?.extend?.spacing) {
      Object.assign(spacing, config.theme.extend.spacing);
    }
    
    return spacing;
  }
  
  private getCacheKey(content: string, config: TailwindConfig): string {
    const configHash = this.hashConfig(config);
    const contentHash = this.hashString(content);
    return `${configHash}-${contentHash}`;
  }
  
  private hashConfig(config: TailwindConfig): string {
    return this.hashString(JSON.stringify(config));
  }
  
  private hashString(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return hash.toString(36);
  }
}