/**
 * User Model
 */

export interface User {
  id: string;
  email: string;
  username: string;
  full_name?: string;
  avatar_url?: string;
  preferences: Record<string, any>;
  created_at: Date;
  updated_at: Date;
  last_login_at?: Date;
}

export interface UserPreferences {
  theme?: 'light' | 'dark' | 'auto';
  defaultProvider?: string;
  autoSave?: boolean;
  aiAssistEnabled?: boolean;
  telemetryEnabled?: boolean;
  shortcuts?: Record<string, string>;
  modelPreferences?: {
    general?: string;
    codeGeneration?: string;
    codeReview?: string;
    chat?: string;
  };
}