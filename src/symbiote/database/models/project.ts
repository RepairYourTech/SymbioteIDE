/**
 * Project Model
 */

export interface Project {
  id: string;
  user_id: string;
  name: string;
  description?: string;
  path: string;
  type: string;
  language?: string;
  framework?: string;
  metadata: Record<string, any>;
  settings: Record<string, any>;
  created_at: Date;
  updated_at: Date;
  last_opened_at?: Date;
}

export interface ProjectSettings {
  aiProvider?: string;
  autoReview?: boolean;
  lintOnSave?: boolean;
  formatOnSave?: boolean;
  customPrompts?: Record<string, string>;
  excludePaths?: string[];
  includePatterns?: string[];
}