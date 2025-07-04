/**
 * Rate Limiting Middleware
 */

import { Request, Response, NextFunction } from 'express';
import { RateLimitOptions } from '../types/api-interfaces';

interface RateLimitStore {
  hits: number;
  resetTime: number;
}

export function rateLimitMiddleware(options: RateLimitOptions) {
  const store = new Map<string, RateLimitStore>();
  
  // Cleanup old entries periodically
  setInterval(() => {
    const now = Date.now();
    for (const [key, value] of store.entries()) {
      if (value.resetTime < now) {
        store.delete(key);
      }
    }
  }, options.windowMs);

  return (req: Request, res: Response, next: NextFunction) => {
    // Skip if path is excluded
    if (options.skipPaths?.some(path => req.path.startsWith(path))) {
      return next();
    }

    // Generate key
    const key = options.keyGenerator ? options.keyGenerator(req) : getDefaultKey(req);
    const now = Date.now();
    
    // Get or create store entry
    let entry = store.get(key);
    if (!entry || entry.resetTime < now) {
      entry = {
        hits: 0,
        resetTime: now + options.windowMs
      };
      store.set(key, entry);
    }

    // Increment hits
    entry.hits++;

    // Set headers
    res.setHeader('X-RateLimit-Limit', options.maxRequests.toString());
    res.setHeader('X-RateLimit-Remaining', Math.max(0, options.maxRequests - entry.hits).toString());
    res.setHeader('X-RateLimit-Reset', new Date(entry.resetTime).toISOString());

    // Check if limit exceeded
    if (entry.hits > options.maxRequests) {
      res.status(429).json({
        error: {
          code: 'RATE_LIMIT_EXCEEDED',
          message: 'Too many requests',
          retryAfter: Math.ceil((entry.resetTime - now) / 1000),
          timestamp: new Date().toISOString()
        }
      });
      return;
    }

    next();
  };
}

function getDefaultKey(req: Request): string {
  // Use IP address by default
  const forwarded = req.headers['x-forwarded-for'] as string;
  const ip = forwarded ? forwarded.split(',')[0] : req.ip;
  
  // Include user ID if authenticated
  if (req.user?.id) {
    return `user:${req.user.id}`;
  }
  
  return `ip:${ip}`;
}