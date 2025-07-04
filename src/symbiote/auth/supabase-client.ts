/**
 * Supabase Client Configuration
 */

import { createClient, SupabaseClient } from '@supabase/supabase-js';
import * as vscode from 'vscode';

// Hardcoded Supabase credentials
const SUPABASE_URL = 'https://apgzyopdlcpwjjyebgtm.supabase.co';
const SUPABASE_ANON_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImFwZ3p5b3BkbGNwd2pqeWViZ3RtIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NTE1NTk0MTAsImV4cCI6MjA2NzEzNTQxMH0.deyeFYGlL2mxtO9mX-rvfaSiQlX9FUzrFSY8GaOPk7k';

let supabaseClient: SupabaseClient | null = null;
let secretStorage: vscode.SecretStorage | null = null;

/**
 * Initialize Supabase client with hardcoded credentials
 */
export function initializeSupabase(context: vscode.ExtensionContext): SupabaseClient {
  secretStorage = context.secrets;

  supabaseClient = createClient(SUPABASE_URL, SUPABASE_ANON_KEY, {
    auth: {
      autoRefreshToken: true,
      persistSession: true,
      detectSessionInUrl: false,
      storage: {
        // Use VS Code's secure SecretStorage API
        getItem: async (key: string) => {
          if (!secretStorage) return null;
          try {
            return await secretStorage.get(key) || null;
          } catch {
            return null;
          }
        },
        setItem: async (key: string, value: string) => {
          if (!secretStorage) return;
          try {
            await secretStorage.store(key, value);
          } catch (error) {
            console.error('Failed to store auth token:', error);
          }
        },
        removeItem: async (key: string) => {
          if (!secretStorage) return;
          try {
            await secretStorage.delete(key);
          } catch (error) {
            console.error('Failed to remove auth token:', error);
          }
        }
      }
    }
  });

  return supabaseClient;
}

/**
 * Get Supabase client instance
 */
export function getSupabase(): SupabaseClient {
  if (!supabaseClient) {
    return initializeSupabase();
  }
  return supabaseClient;
}