/**
 * Error Handling Middleware
 */

import { Request, Response, NextFunction } from 'express';
import { APIError } from '../types/api-interfaces';

export function errorHandler(
  err: any,
  req: Request,
  res: Response,
  next: NextFunction
): void {
  // Log error
  console.error(`Error in request ${req.id}:`, err);

  // Determine status code
  const statusCode = err.statusCode || err.status || 500;

  // Create error response
  const error: APIError = {
    code: err.code || 'INTERNAL_ERROR',
    message: err.message || 'An internal error occurred',
    details: err.details,
    timestamp: new Date().toISOString(),
    requestId: req.id
  };

  // Add stack trace in development
  if (process.env.NODE_ENV === 'development') {
    error.details = {
      ...error.details,
      stack: err.stack
    };
  }

  // Send response
  res.status(statusCode).json({ error });
}