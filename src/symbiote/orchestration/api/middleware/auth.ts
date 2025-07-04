/**
 * Authentication Middleware
 */

import { Request, Response, NextFunction } from 'express';
import { AuthOptions } from '../types/api-interfaces';

// Simple JWT verification without external dependency
function verifyJWT(token: string, secret: string): any {
  // In production, use a proper JWT library
  // This is a simplified implementation for development
  const parts = token.split('.');
  if (parts.length !== 3) {
    throw new Error('Invalid token format');
  }
  
  try {
    const payload = JSON.parse(Buffer.from(parts[1], 'base64').toString());
    // Check expiration
    if (payload.exp && payload.exp < Date.now() / 1000) {
      const error = new Error('Token expired');
      (error as any).name = 'TokenExpiredError';
      throw error;
    }
    return payload;
  } catch (error: any) {
    if (error.name === 'TokenExpiredError') throw error;
    const jwtError = new Error('Invalid token');
    (jwtError as any).name = 'JsonWebTokenError';
    throw jwtError;
  }
}

export function authMiddleware(options: AuthOptions) {
  return async (req: Request, res: Response, next: NextFunction) => {
    // Check if path is public
    if (options.publicPaths?.some(path => req.path.startsWith(path))) {
      return next();
    }

    try {
      switch (options.type) {
        case 'apiKey':
          await validateApiKey(req, res, options);
          break;
        
        case 'jwt':
          await validateJWT(req, res, options);
          break;
        
        case 'oauth':
          await validateOAuth(req, res, options);
          break;
        
        case 'custom':
          if (options.config?.validator) {
            await options.config.validator(req, res);
          }
          break;
        
        default:
          throw new Error('Invalid authentication type');
      }

      next();
    } catch (error: any) {
      res.status(401).json({
        error: {
          code: 'UNAUTHORIZED',
          message: error.message || 'Authentication failed',
          timestamp: new Date().toISOString()
        }
      });
    }
  };
}

async function validateApiKey(req: Request, res: Response, options: AuthOptions): Promise<void> {
  const apiKey = req.headers['x-api-key'] || req.query.apiKey;

  if (!apiKey) {
    throw new Error('API key required');
  }

  // Validate API key
  if (options.config?.validKeys) {
    const validKeys = options.config.validKeys;
    if (!validKeys.includes(apiKey)) {
      throw new Error('Invalid API key');
    }
  }

  // Set user info if available
  if (options.config?.keyToUser) {
    req.user = options.config.keyToUser[apiKey as string];
  }
}

async function validateJWT(req: Request, res: Response, options: AuthOptions): Promise<void> {
  const authHeader = req.headers.authorization;
  
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    throw new Error('Bearer token required');
  }

  const token = authHeader.substring(7);
  const secret = options.config?.secret || process.env.JWT_SECRET;

  if (!secret) {
    throw new Error('JWT secret not configured');
  }

  try {
    const decoded = verifyJWT(token, secret);
    req.user = decoded;
  } catch (error: any) {
    if (error.name === 'TokenExpiredError') {
      throw new Error('Token expired');
    } else if (error.name === 'JsonWebTokenError') {
      throw new Error('Invalid token');
    }
    throw error;
  }
}

async function validateOAuth(req: Request, res: Response, options: AuthOptions): Promise<void> {
  const authHeader = req.headers.authorization;
  
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    throw new Error('Bearer token required');
  }

  const token = authHeader.substring(7);

  // Validate with OAuth provider
  if (options.config?.introspectionUrl) {
    const response = await fetch(options.config.introspectionUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Authorization': `Basic ${options.config.clientCredentials}`
      },
      body: `token=${token}`
    });

    if (!response.ok) {
      throw new Error('Token introspection failed');
    }

    const result = await response.json();
    
    if (!result.active) {
      throw new Error('Token is not active');
    }

    req.user = {
      id: result.sub,
      username: result.username,
      scope: result.scope?.split(' ') || []
    };
  }
}