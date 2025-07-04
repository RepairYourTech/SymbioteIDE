/**
 * Notebook Types
 * 
 * Type definitions for the native notebook functionality
 */

import { CodeEntityType } from '../search/qdrant/types';

export enum CellType {
  Code = 'code',
  Markdown = 'markdown',
  React = 'react',
  Design = 'design',
  Query = 'query'
}

export enum CellLanguage {
  JavaScript = 'javascript',
  TypeScript = 'typescript',
  Python = 'python',
  HTML = 'html',
  CSS = 'css',
  SQL = 'sql',
  JSON = 'json'
}

export enum ExecutionState {
  Idle = 'idle',
  Pending = 'pending',
  Running = 'running',
  Success = 'success',
  Error = 'error'
}

export interface NotebookCell {
  id: string;
  type: CellType;
  language?: CellLanguage;
  source: string;
  outputs: CellOutput[];
  metadata: CellMetadata;
  executionState: ExecutionState;
  executionCount?: number;
  createdAt: Date;
  updatedAt: Date;
}

export interface CellOutput {
  type: 'text' | 'html' | 'image' | 'react' | 'error' | 'data';
  data: any;
  metadata?: Record<string, any>;
}

export interface CellMetadata {
  collapsed?: boolean;
  scrolled?: boolean;
  tags?: string[];
  framework?: 'react' | 'vue' | 'svelte' | 'angular';
  dependencies?: string[];
  buildConfig?: BuildConfig;
  designTokens?: DesignTokens;
}

export interface BuildConfig {
  bundler?: 'vite' | 'webpack' | 'esbuild';
  cssProcessor?: 'tailwind' | 'postcss' | 'sass';
  tailwindConfig?: any;
  webpackConfig?: any;
  viteConfig?: any;
}

export interface Notebook {
  id: string;
  name: string;
  cells: NotebookCell[];
  metadata: NotebookMetadata;
  kernelSpec: KernelSpec;
  createdAt: Date;
  updatedAt: Date;
  version: string;
}

export interface NotebookMetadata {
  author?: string;
  description?: string;
  tags?: string[];
  projectId?: string;
  teamId?: string;
  language?: string;
  framework?: string;
  designSystem?: DesignSystem;
}

export interface KernelSpec {
  name: string;
  displayName: string;
  language: CellLanguage;
  argv: string[];
  env?: Record<string, string>;
}

export interface KernelInfo {
  id: string;
  name: string;
  status: KernelStatus;
  language: CellLanguage;
  executionCount: number;
  lastActivity?: Date;
}

export enum KernelStatus {
  Starting = 'starting',
  Idle = 'idle',
  Busy = 'busy',
  Terminating = 'terminating',
  Restarting = 'restarting',
  Dead = 'dead'
}

export interface ExecuteRequest {
  cellId: string;
  code: string;
  language: CellLanguage;
  metadata?: CellMetadata;
  silent?: boolean;
  storeHistory?: boolean;
  allowStdin?: boolean;
}

export interface ExecuteResult {
  cellId: string;
  executionCount: number;
  outputs: CellOutput[];
  status: 'ok' | 'error' | 'abort';
  error?: ExecutionError;
  timing?: {
    startTime: number;
    endTime: number;
    duration: number;
  };
}

export interface ExecutionError {
  name: string;
  message: string;
  stack?: string;
  lineNumber?: number;
  columnNumber?: number;
}

// Design System Types

export interface DesignSystem {
  name: string;
  version: string;
  colors: ColorScheme;
  typography: Typography;
  spacing: SpacingScale;
  shadows: ShadowScale;
  tokens: DesignTokens;
}

export interface ColorScheme {
  primary: ColorScale;
  secondary: ColorScale;
  accent: ColorScale;
  neutral: ColorScale;
  semantic: SemanticColors;
  custom: Record<string, ColorScale>;
}

export interface ColorScale {
  50: string;
  100: string;
  200: string;
  300: string;
  400: string;
  500: string;
  600: string;
  700: string;
  800: string;
  900: string;
  950?: string;
}

export interface SemanticColors {
  success: string;
  warning: string;
  error: string;
  info: string;
  successLight?: string;
  warningLight?: string;
  errorLight?: string;
  infoLight?: string;
}

export interface Typography {
  fontFamily: {
    sans: string[];
    serif: string[];
    mono: string[];
  };
  fontSize: Record<string, string>;
  fontWeight: Record<string, number>;
  lineHeight: Record<string, string>;
  letterSpacing: Record<string, string>;
}

export interface SpacingScale {
  [key: string]: string;
}

export interface ShadowScale {
  [key: string]: string;
}

export interface DesignTokens {
  colors: Record<string, any>;
  typography: Record<string, any>;
  spacing: Record<string, any>;
  shadows: Record<string, any>;
  borders: Record<string, any>;
  radii: Record<string, any>;
  zIndices: Record<string, any>;
  transitions: Record<string, any>;
}

// Component Playground Types

export interface ComponentPlaygroundState {
  component: string;
  props: Record<string, any>;
  state: Record<string, any>;
  theme: string;
  viewport: ViewportSize;
  interactions: Interaction[];
}

export interface ViewportSize {
  width: number;
  height: number;
  device?: 'mobile' | 'tablet' | 'desktop';
}

export interface Interaction {
  type: 'click' | 'hover' | 'focus' | 'drag' | 'type';
  target: string;
  value?: any;
  timestamp: number;
}

// Export Types

export interface ExportOptions {
  format: 'app' | 'component' | 'static' | 'library';
  bundler?: 'vite' | 'webpack' | 'parcel';
  framework?: string;
  outputDir: string;
  includeStyles?: boolean;
  includeAssets?: boolean;
  minify?: boolean;
}

export interface ComponentLibraryExport {
  name: string;
  version: string;
  components: ExportedComponent[];
  styles: string;
  designTokens: DesignTokens;
  documentation: string;
}

export interface ExportedComponent {
  name: string;
  displayName: string;
  description: string;
  props: ComponentProp[];
  examples: ComponentExample[];
  source: string;
}

export interface ComponentProp {
  name: string;
  type: string;
  required: boolean;
  defaultValue?: any;
  description?: string;
}

export interface ComponentExample {
  title: string;
  code: string;
  preview?: string;
}

// Events

export interface NotebookEvents {
  'cell-executed': (result: ExecuteResult) => void;
  'cell-added': (cell: NotebookCell) => void;
  'cell-deleted': (cellId: string) => void;
  'cell-moved': (cellId: string, newIndex: number) => void;
  'kernel-status-changed': (status: KernelStatus) => void;
  'design-tokens-updated': (tokens: DesignTokens) => void;
  'component-exported': (component: ExportedComponent) => void;
}