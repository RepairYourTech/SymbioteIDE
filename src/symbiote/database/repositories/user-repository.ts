/**
 * User Repository
 */

import { BaseRepository } from './base-repository';
import { User } from '../models/user';

export class UserRepository extends BaseRepository<User> {
  constructor() {
    super('users');
  }
  
  /**
   * Find user by email
   */
  async findByEmail(email: string): Promise<User | null> {
    return this.findOneWhere({ email });
  }
  
  /**
   * Find user by username
   */
  async findByUsername(username: string): Promise<User | null> {
    return this.findOneWhere({ username });
  }
  
  /**
   * Create new user
   */
  async createUser(data: {
    email: string;
    username: string;
    fullName?: string;
    avatarUrl?: string;
    preferences?: any;
  }): Promise<User> {
    const user = await this.insert({
      email: data.email,
      username: data.username,
      full_name: data.fullName,
      avatar_url: data.avatarUrl,
      preferences: data.preferences || {}
    });
    
    return user;
  }
  
  /**
   * Update user preferences
   */
  async updatePreferences(
    userId: string,
    preferences: Record<string, any>
  ): Promise<User | null> {
    // Get current preferences
    const user = await this.findById(userId);
    if (!user) return null;
    
    // Merge preferences
    const mergedPreferences = {
      ...user.preferences,
      ...preferences
    };
    
    // Update user
    return this.update(userId, {
      preferences: mergedPreferences
    });
  }
  
  /**
   * Update last login
   */
  async updateLastLogin(userId: string): Promise<void> {
    await this.update(userId, {
      last_login_at: new Date()
    });
  }
  
  /**
   * Get or create user
   */
  async getOrCreate(data: {
    email: string;
    username: string;
    fullName?: string;
    avatarUrl?: string;
  }): Promise<User> {
    // Try to find existing user
    let user = await this.findByEmail(data.email);
    
    if (!user) {
      // Create new user
      user = await this.createUser(data);
    } else {
      // Update last login
      await this.updateLastLogin(user.id);
    }
    
    return user;
  }
  
  /**
   * Search users
   */
  async search(query: string): Promise<User[]> {
    const sql = `
      SELECT * FROM ${this.table}
      WHERE username ILIKE $1
         OR email ILIKE $1
         OR full_name ILIKE $1
      ORDER BY username
      LIMIT 20
    `;
    
    return this.query(sql, [`%${query}%`]);
  }
}