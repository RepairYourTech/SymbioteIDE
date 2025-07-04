/**
 * Authentication Types
 */

export interface User {
  id: string;
  email: string;
  created_at: string;
  updated_at: string;
  app_metadata?: Record<string, any>;
  user_metadata?: Record<string, any>;
}

export interface Session {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  expires_at?: number;
  token_type: string;
  user: User;
}

export interface AuthState {
  user: User | null;
  session: Session | null;
  loading: boolean;
  error: string | null;
}

export interface AuthCredentials {
  email: string;
  password: string;
}

export interface AuthError {
  message: string;
  status?: number;
}