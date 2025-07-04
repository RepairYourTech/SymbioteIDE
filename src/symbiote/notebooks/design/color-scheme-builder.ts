/**
 * Color Scheme Builder
 * 
 * Interactive color scheme creation and management for design systems
 */

import { EventEmitter } from 'events';
import { Logger } from '../../utils/logger';
import {
  ColorScheme,
  ColorScale,
  SemanticColors,
  DesignSystem
} from '../types';

export interface ColorSchemeBuilderConfig {
  enableAIGeneration?: boolean;
  validateAccessibility?: boolean;
  defaultColorSpace?: 'rgb' | 'hsl' | 'lab';
}

export interface ColorAnalysis {
  contrast: {
    aa: boolean;
    aaa: boolean;
    ratio: number;
  };
  harmony: {
    type: 'complementary' | 'analogous' | 'triadic' | 'split-complementary';
    colors: string[];
  };
  accessibility: {
    colorBlindSafe: boolean;
    warnings: string[];
  };
}

export class ColorSchemeBuilder extends EventEmitter {
  private logger = new Logger('ColorSchemeBuilder');
  private config: Required<ColorSchemeBuilderConfig>;
  
  constructor(config: ColorSchemeBuilderConfig = {}) {
    super();
    
    this.config = {
      enableAIGeneration: config.enableAIGeneration ?? true,
      validateAccessibility: config.validateAccessibility ?? true,
      defaultColorSpace: config.defaultColorSpace ?? 'hsl'
    };
  }
  
  /**
   * Create a color scheme from a base color
   */
  async createFromBaseColor(
    baseColor: string,
    options: {
      name?: string;
      generateScale?: boolean;
      includeSemantics?: boolean;
    } = {}
  ): Promise<ColorScheme> {
    try {
      const hsl = this.hexToHsl(baseColor);
      
      // Generate color scale
      const scale = options.generateScale !== false ? 
        this.generateColorScale(hsl) : 
        this.createSingleColorScale(baseColor);
      
      // Create color scheme
      const scheme: ColorScheme = {
        primary: scale,
        secondary: this.generateComplementaryScale(hsl),
        accent: this.generateAccentScale(hsl),
        neutral: this.generateNeutralScale(),
        semantic: options.includeSemantics !== false ? 
          this.generateSemanticColors() : 
          {} as SemanticColors,
        custom: {}
      };
      
      // Validate accessibility if enabled
      if (this.config.validateAccessibility) {
        await this.validateAccessibility(scheme);
      }
      
      this.emit('scheme-created', { scheme, baseColor });
      
      return scheme;
      
    } catch (error) {
      this.logger.error('Failed to create color scheme', error);
      throw error;
    }
  }
  
  /**
   * Generate color scheme from image
   */
  async createFromImage(
    imageData: ImageData | string,
    options: {
      maxColors?: number;
      algorithm?: 'kmeans' | 'median-cut' | 'octree';
    } = {}
  ): Promise<ColorScheme> {
    try {
      // Extract dominant colors
      const colors = await this.extractColors(imageData, options);
      
      // Select primary color
      const primaryColor = colors[0] || '#3b82f6';
      
      // Create scheme from primary
      const scheme = await this.createFromBaseColor(primaryColor, {
        generateScale: true,
        includeSemantics: true
      });
      
      // Add extracted colors as custom palette
      scheme.custom['extracted'] = this.colorsToScale(colors);
      
      this.emit('scheme-created-from-image', { scheme, colors });
      
      return scheme;
      
    } catch (error) {
      this.logger.error('Failed to create scheme from image', error);
      throw error;
    }
  }
  
  /**
   * Analyze color scheme
   */
  analyzeScheme(scheme: ColorScheme): ColorAnalysis {
    const primary = scheme.primary['500'] || '#3b82f6';
    const background = '#ffffff';
    
    return {
      contrast: this.analyzeContrast(primary, background),
      harmony: this.analyzeHarmony(scheme),
      accessibility: this.checkAccessibility(scheme)
    };
  }
  
  /**
   * Generate variations of a color scheme
   */
  generateVariations(
    scheme: ColorScheme,
    count: number = 5
  ): ColorScheme[] {
    const variations: ColorScheme[] = [];
    const baseHue = this.getHueFromColor(scheme.primary['500']);
    
    for (let i = 0; i < count; i++) {
      const hueShift = (360 / count) * i;
      const newHue = (baseHue + hueShift) % 360;
      
      variations.push(this.shiftSchemeHue(scheme, hueShift));
    }
    
    return variations;
  }
  
  /**
   * Mix two color schemes
   */
  blendSchemes(
    scheme1: ColorScheme,
    scheme2: ColorScheme,
    ratio: number = 0.5
  ): ColorScheme {
    const blended: ColorScheme = {
      primary: this.blendColorScales(scheme1.primary, scheme2.primary, ratio),
      secondary: this.blendColorScales(scheme1.secondary, scheme2.secondary, ratio),
      accent: this.blendColorScales(scheme1.accent, scheme2.accent, ratio),
      neutral: this.blendColorScales(scheme1.neutral, scheme2.neutral, ratio),
      semantic: this.blendSemanticColors(scheme1.semantic, scheme2.semantic, ratio),
      custom: {}
    };
    
    // Merge custom colors
    Object.keys(scheme1.custom).forEach(key => {
      if (scheme2.custom[key]) {
        blended.custom[key] = this.blendColorScales(
          scheme1.custom[key],
          scheme2.custom[key],
          ratio
        );
      } else {
        blended.custom[key] = scheme1.custom[key];
      }
    });
    
    return blended;
  }
  
  /**
   * Export color scheme to various formats
   */
  exportScheme(
    scheme: ColorScheme,
    format: 'css' | 'scss' | 'json' | 'tailwind' | 'figma'
  ): string {
    switch (format) {
      case 'css':
        return this.exportAsCSS(scheme);
      case 'scss':
        return this.exportAsSCSS(scheme);
      case 'json':
        return JSON.stringify(scheme, null, 2);
      case 'tailwind':
        return this.exportAsTailwind(scheme);
      case 'figma':
        return this.exportAsFigma(scheme);
      default:
        throw new Error(`Unsupported export format: ${format}`);
    }
  }
  
  // Private methods
  
  private generateColorScale(hsl: { h: number; s: number; l: number }): ColorScale {
    const scale: Partial<ColorScale> = {};
    
    // Generate shades
    const shades = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];
    
    shades.forEach(shade => {
      let lightness: number;
      
      if (shade < 500) {
        // Lighter shades
        lightness = 95 - ((500 - shade) / 500) * 45;
      } else if (shade === 500) {
        // Base color
        lightness = hsl.l;
      } else {
        // Darker shades
        lightness = hsl.l - ((shade - 500) / 500) * hsl.l * 0.8;
      }
      
      // Adjust saturation for extreme shades
      let saturation = hsl.s;
      if (shade <= 100 || shade >= 900) {
        saturation *= 0.7;
      }
      
      scale[shade as keyof ColorScale] = this.hslToHex({
        h: hsl.h,
        s: saturation,
        l: Math.max(5, Math.min(95, lightness))
      });
    });
    
    return scale as ColorScale;
  }
  
  private generateComplementaryScale(hsl: { h: number; s: number; l: number }): ColorScale {
    const complementaryHue = (hsl.h + 180) % 360;
    return this.generateColorScale({
      h: complementaryHue,
      s: hsl.s * 0.8,
      l: hsl.l
    });
  }
  
  private generateAccentScale(hsl: { h: number; s: number; l: number }): ColorScale {
    const accentHue = (hsl.h + 30) % 360;
    return this.generateColorScale({
      h: accentHue,
      s: Math.min(100, hsl.s * 1.2),
      l: hsl.l
    });
  }
  
  private generateNeutralScale(): ColorScale {
    return this.generateColorScale({
      h: 0,
      s: 0,
      l: 50
    });
  }
  
  private generateSemanticColors(): SemanticColors {
    return {
      success: '#10b981',
      warning: '#f59e0b',
      error: '#ef4444',
      info: '#3b82f6',
      successLight: '#d1fae5',
      warningLight: '#fed7aa',
      errorLight: '#fee2e2',
      infoLight: '#dbeafe'
    };
  }
  
  private createSingleColorScale(color: string): ColorScale {
    const scale: Partial<ColorScale> = {};
    const shades = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];
    
    shades.forEach(shade => {
      scale[shade as keyof ColorScale] = shade === 500 ? color : color;
    });
    
    return scale as ColorScale;
  }
  
  private async extractColors(
    imageData: ImageData | string,
    options: any
  ): Promise<string[]> {
    // In a real implementation, this would use color extraction algorithms
    // For now, return mock colors
    return [
      '#3b82f6',
      '#10b981',
      '#f59e0b',
      '#ef4444',
      '#6366f1'
    ];
  }
  
  private colorsToScale(colors: string[]): ColorScale {
    const scale: Partial<ColorScale> = {};
    const shades = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];
    
    shades.forEach((shade, index) => {
      const colorIndex = Math.floor((index / shades.length) * colors.length);
      scale[shade as keyof ColorScale] = colors[Math.min(colorIndex, colors.length - 1)];
    });
    
    return scale as ColorScale;
  }
  
  private analyzeContrast(
    foreground: string,
    background: string
  ): { aa: boolean; aaa: boolean; ratio: number } {
    const ratio = this.getContrastRatio(foreground, background);
    
    return {
      aa: ratio >= 4.5,
      aaa: ratio >= 7,
      ratio
    };
  }
  
  private analyzeHarmony(scheme: ColorScheme): any {
    const primaryHue = this.getHueFromColor(scheme.primary['500']);
    const secondaryHue = this.getHueFromColor(scheme.secondary['500']);
    
    const hueDiff = Math.abs(primaryHue - secondaryHue);
    
    let type: string;
    if (hueDiff > 150 && hueDiff < 210) {
      type = 'complementary';
    } else if (hueDiff < 60) {
      type = 'analogous';
    } else if (hueDiff === 120 || hueDiff === 240) {
      type = 'triadic';
    } else {
      type = 'split-complementary';
    }
    
    return {
      type,
      colors: [
        scheme.primary['500'],
        scheme.secondary['500'],
        scheme.accent['500']
      ]
    };
  }
  
  private checkAccessibility(scheme: ColorScheme): any {
    const warnings: string[] = [];
    
    // Check contrast ratios
    const bgColor = '#ffffff';
    
    Object.entries(scheme).forEach(([name, scale]) => {
      if (typeof scale === 'object' && scale['500']) {
        const contrast = this.getContrastRatio(scale['500'], bgColor);
        if (contrast < 4.5) {
          warnings.push(`${name} color may have insufficient contrast`);
        }
      }
    });
    
    return {
      colorBlindSafe: warnings.length === 0,
      warnings
    };
  }
  
  private async validateAccessibility(scheme: ColorScheme): Promise<void> {
    const analysis = this.checkAccessibility(scheme);
    
    if (analysis.warnings.length > 0) {
      this.emit('accessibility-warnings', analysis.warnings);
    }
  }
  
  private shiftSchemeHue(scheme: ColorScheme, hueShift: number): ColorScheme {
    const shifted: ColorScheme = {
      primary: this.shiftScaleHue(scheme.primary, hueShift),
      secondary: this.shiftScaleHue(scheme.secondary, hueShift),
      accent: this.shiftScaleHue(scheme.accent, hueShift),
      neutral: scheme.neutral, // Don't shift neutrals
      semantic: scheme.semantic, // Don't shift semantics
      custom: {}
    };
    
    // Shift custom colors
    Object.entries(scheme.custom).forEach(([key, scale]) => {
      shifted.custom[key] = this.shiftScaleHue(scale, hueShift);
    });
    
    return shifted;
  }
  
  private shiftScaleHue(scale: ColorScale, hueShift: number): ColorScale {
    const shifted: Partial<ColorScale> = {};
    
    Object.entries(scale).forEach(([shade, color]) => {
      const hsl = this.hexToHsl(color);
      hsl.h = (hsl.h + hueShift) % 360;
      shifted[shade as keyof ColorScale] = this.hslToHex(hsl);
    });
    
    return shifted as ColorScale;
  }
  
  private blendColorScales(
    scale1: ColorScale,
    scale2: ColorScale,
    ratio: number
  ): ColorScale {
    const blended: Partial<ColorScale> = {};
    
    Object.keys(scale1).forEach(shade => {
      const color1 = scale1[shade as keyof ColorScale];
      const color2 = scale2[shade as keyof ColorScale];
      
      if (color1 && color2) {
        blended[shade as keyof ColorScale] = this.blendColors(color1, color2, ratio);
      }
    });
    
    return blended as ColorScale;
  }
  
  private blendSemanticColors(
    semantic1: SemanticColors,
    semantic2: SemanticColors,
    ratio: number
  ): SemanticColors {
    const blended: Partial<SemanticColors> = {};
    
    Object.keys(semantic1).forEach(key => {
      const color1 = semantic1[key as keyof SemanticColors];
      const color2 = semantic2[key as keyof SemanticColors];
      
      if (color1 && color2) {
        blended[key as keyof SemanticColors] = this.blendColors(color1, color2, ratio);
      }
    });
    
    return blended as SemanticColors;
  }
  
  private blendColors(color1: string, color2: string, ratio: number): string {
    const rgb1 = this.hexToRgb(color1);
    const rgb2 = this.hexToRgb(color2);
    
    const blended = {
      r: Math.round(rgb1.r * (1 - ratio) + rgb2.r * ratio),
      g: Math.round(rgb1.g * (1 - ratio) + rgb2.g * ratio),
      b: Math.round(rgb1.b * (1 - ratio) + rgb2.b * ratio)
    };
    
    return this.rgbToHex(blended);
  }
  
  // Color conversion utilities
  
  private hexToRgb(hex: string): { r: number; g: number; b: number } {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? {
      r: parseInt(result[1], 16),
      g: parseInt(result[2], 16),
      b: parseInt(result[3], 16)
    } : { r: 0, g: 0, b: 0 };
  }
  
  private rgbToHex(rgb: { r: number; g: number; b: number }): string {
    return '#' + ((1 << 24) + (rgb.r << 16) + (rgb.g << 8) + rgb.b)
      .toString(16).slice(1);
  }
  
  private hexToHsl(hex: string): { h: number; s: number; l: number } {
    const rgb = this.hexToRgb(hex);
    const r = rgb.r / 255;
    const g = rgb.g / 255;
    const b = rgb.b / 255;
    
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    let h = 0;
    let s = 0;
    const l = (max + min) / 2;
    
    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      
      switch (max) {
        case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
        case g: h = ((b - r) / d + 2) / 6; break;
        case b: h = ((r - g) / d + 4) / 6; break;
      }
    }
    
    return {
      h: Math.round(h * 360),
      s: Math.round(s * 100),
      l: Math.round(l * 100)
    };
  }
  
  private hslToHex(hsl: { h: number; s: number; l: number }): string {
    const h = hsl.h / 360;
    const s = hsl.s / 100;
    const l = hsl.l / 100;
    
    let r, g, b;
    
    if (s === 0) {
      r = g = b = l;
    } else {
      const hue2rgb = (p: number, q: number, t: number) => {
        if (t < 0) t += 1;
        if (t > 1) t -= 1;
        if (t < 1/6) return p + (q - p) * 6 * t;
        if (t < 1/2) return q;
        if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
        return p;
      };
      
      const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
      const p = 2 * l - q;
      
      r = hue2rgb(p, q, h + 1/3);
      g = hue2rgb(p, q, h);
      b = hue2rgb(p, q, h - 1/3);
    }
    
    return this.rgbToHex({
      r: Math.round(r * 255),
      g: Math.round(g * 255),
      b: Math.round(b * 255)
    });
  }
  
  private getHueFromColor(color: string): number {
    return this.hexToHsl(color).h;
  }
  
  private getContrastRatio(color1: string, color2: string): number {
    const rgb1 = this.hexToRgb(color1);
    const rgb2 = this.hexToRgb(color2);
    
    const l1 = this.getRelativeLuminance(rgb1);
    const l2 = this.getRelativeLuminance(rgb2);
    
    const lighter = Math.max(l1, l2);
    const darker = Math.min(l1, l2);
    
    return (lighter + 0.05) / (darker + 0.05);
  }
  
  private getRelativeLuminance(rgb: { r: number; g: number; b: number }): number {
    const sRGB = [rgb.r / 255, rgb.g / 255, rgb.b / 255];
    
    const linearRGB = sRGB.map(val => {
      if (val <= 0.03928) {
        return val / 12.92;
      }
      return Math.pow((val + 0.055) / 1.055, 2.4);
    });
    
    return 0.2126 * linearRGB[0] + 0.7152 * linearRGB[1] + 0.0722 * linearRGB[2];
  }
  
  // Export methods
  
  private exportAsCSS(scheme: ColorScheme): string {
    const lines: string[] = [':root {'];
    
    Object.entries(scheme).forEach(([name, scale]) => {
      if (typeof scale === 'object' && !Array.isArray(scale)) {
        Object.entries(scale).forEach(([shade, color]) => {
          lines.push(`  --color-${name}-${shade}: ${color};`);
        });
      }
    });
    
    lines.push('}');
    return lines.join('\n');
  }
  
  private exportAsSCSS(scheme: ColorScheme): string {
    const lines: string[] = [];
    
    Object.entries(scheme).forEach(([name, scale]) => {
      if (typeof scale === 'object' && !Array.isArray(scale)) {
        lines.push(`$color-${name}: (`);
        Object.entries(scale).forEach(([shade, color]) => {
          lines.push(`  ${shade}: ${color},`);
        });
        lines.push(');');
        lines.push('');
      }
    });
    
    return lines.join('\n');
  }
  
  private exportAsTailwind(scheme: ColorScheme): string {
    const config = {
      theme: {
        extend: {
          colors: {}
        }
      }
    };
    
    Object.entries(scheme).forEach(([name, scale]) => {
      if (typeof scale === 'object' && !Array.isArray(scale)) {
        (config.theme.extend.colors as any)[name] = scale;
      }
    });
    
    return `module.exports = ${JSON.stringify(config, null, 2)}`;
  }
  
  private exportAsFigma(scheme: ColorScheme): string {
    // Figma plugin format
    const figmaColors: any[] = [];
    
    Object.entries(scheme).forEach(([name, scale]) => {
      if (typeof scale === 'object' && !Array.isArray(scale)) {
        Object.entries(scale).forEach(([shade, color]) => {
          figmaColors.push({
            name: `${name}/${shade}`,
            color: this.hexToRgb(color)
          });
        });
      }
    });
    
    return JSON.stringify({ colors: figmaColors }, null, 2);
  }
}