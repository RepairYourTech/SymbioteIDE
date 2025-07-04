/**
 * Request Validation Middleware
 */

import { Request, Response, NextFunction } from 'express';

// Simple validation system
interface ValidationRule {
  required?: boolean;
  type?: 'string' | 'number' | 'boolean' | 'object' | 'array';
  enum?: any[];
  min?: number;
  max?: number;
  minLength?: number;
  items?: ValidationRule;
  properties?: { [key: string]: ValidationRule };
}

// Validation schemas
const schemas: { [key: string]: { [key: string]: ValidationRule } } = {
  executeTask: {
    task: {
      required: true,
      type: 'object',
      properties: {
        id: { required: true, type: 'string' },
        type: { required: true, type: 'string' },
        prompt: { required: true, type: 'string' },
        context: {
          type: 'object',
          properties: {
            files: { type: 'array', items: { type: 'string' } },
            variables: { type: 'object' },
            history: { type: 'array' },
            workspacePath: { type: 'string' },
            custom: { type: 'object' }
          }
        },
        constraints: {
          type: 'object',
          properties: {
            maxTokens: { type: 'number', min: 1 },
            temperature: { type: 'number', min: 0, max: 2 },
            requiredCapabilities: { type: 'array', items: { type: 'string' } },
            excludeModels: { type: 'array', items: { type: 'string' } },
            costLimit: {
              type: 'object',
              properties: {
                amount: { type: 'number', min: 0 },
                currency: { type: 'string' }
              }
            },
            timeout: { type: 'number', min: 0 }
          }
        },
        metadata: { type: 'object' },
        priority: { type: 'string', enum: ['urgent', 'high', 'normal', 'low'] },
        sessionId: { type: 'string' }
      }
    },
    options: {
      type: 'object',
      properties: {
        priority: { type: 'string', enum: ['Critical', 'High', 'Normal', 'Low', 'Background'] },
        timeout: { type: 'number', min: 0 },
        maxRetries: { type: 'number', min: 0 },
        preferredModels: { type: 'array', items: { type: 'string' } },
        excludeModels: { type: 'array', items: { type: 'string' } },
        metadata: { type: 'object' }
      }
    }
  },

  executeBatch: {
    tasks: {
      required: true,
      type: 'array',
      minLength: 1,
      items: {
        type: 'object',
        properties: {
          id: { required: true, type: 'string' },
          type: { required: true, type: 'string' },
          prompt: { required: true, type: 'string' }
        }
      }
    },
    options: {
      type: 'object',
      properties: {
        maxConcurrency: { type: 'number', min: 1 },
        continueOnError: { type: 'boolean' },
        priority: { type: 'string', enum: ['Critical', 'High', 'Normal', 'Low', 'Background'] }
      }
    }
  },

  streamTask: {
    task: {
      required: true,
      type: 'object',
      properties: {
        id: { required: true, type: 'string' },
        type: { required: true, type: 'string' },
        prompt: { required: true, type: 'string' }
      }
    },
    options: {
      type: 'object',
      properties: {
        priority: { type: 'string', enum: ['Critical', 'High', 'Normal', 'Low', 'Background'] },
        includePartialResults: { type: 'boolean' }
      }
    }
  },

  costEstimate: {
    task: {
      required: true,
      type: 'object',
      properties: {
        id: { required: true, type: 'string' },
        type: { required: true, type: 'string' },
        prompt: { required: true, type: 'string' }
      }
    },
    preferredModels: {
      type: 'array',
      items: { type: 'string' }
    }
  },

  updateConfig: {
    config: {
      required: true,
      type: 'object'
    },
    validateOnly: {
      type: 'boolean'
    }
  }
};

export function validationMiddleware(schemaName: keyof typeof schemas) {
  return (req: Request, res: Response, next: NextFunction) => {
    const schema = schemas[schemaName];
    
    if (!schema) {
      return next(new Error(`Validation schema '${schemaName}' not found`));
    }

    const errors = validateObject(req.body, schema, 'body');

    if (errors.length > 0) {
      res.status(400).json({
        error: {
          code: 'VALIDATION_ERROR',
          message: 'Request validation failed',
          details: errors,
          timestamp: new Date().toISOString()
        }
      });
      return;
    }

    next();
  };
}

function validateObject(
  obj: any, 
  schema: { [key: string]: ValidationRule }, 
  path: string
): Array<{ field: string; message: string }> {
  const errors: Array<{ field: string; message: string }> = [];

  // Check required fields
  for (const [key, rule] of Object.entries(schema)) {
    if (rule.required && !(key in obj)) {
      errors.push({
        field: `${path}.${key}`,
        message: `${key} is required`
      });
    }
  }

  // Validate present fields
  for (const [key, value] of Object.entries(obj || {})) {
    const rule = schema[key];
    if (!rule) continue;

    const fieldPath = `${path}.${key}`;

    // Type validation
    if (rule.type) {
      const actualType = Array.isArray(value) ? 'array' : typeof value;
      if (actualType !== rule.type) {
        errors.push({
          field: fieldPath,
          message: `Expected ${rule.type} but got ${actualType}`
        });
        continue;
      }
    }

    // Enum validation
    if (rule.enum && !rule.enum.includes(value)) {
      errors.push({
        field: fieldPath,
        message: `Value must be one of: ${rule.enum.join(', ')}`
      });
    }

    // Number validations
    if (typeof value === 'number') {
      if (rule.min !== undefined && value < rule.min) {
        errors.push({
          field: fieldPath,
          message: `Value must be at least ${rule.min}`
        });
      }
      if (rule.max !== undefined && value > rule.max) {
        errors.push({
          field: fieldPath,
          message: `Value must be at most ${rule.max}`
        });
      }
    }

    // Array validations
    if (Array.isArray(value)) {
      if (rule.minLength !== undefined && value.length < rule.minLength) {
        errors.push({
          field: fieldPath,
          message: `Array must have at least ${rule.minLength} items`
        });
      }
      if (rule.items) {
        value.forEach((item, index) => {
          if (rule.items!.type && typeof item !== rule.items!.type) {
            errors.push({
              field: `${fieldPath}[${index}]`,
              message: `Expected ${rule.items!.type} but got ${typeof item}`
            });
          }
          if (rule.items!.properties) {
            errors.push(...validateObject(item, rule.items!.properties, `${fieldPath}[${index}]`));
          }
        });
      }
    }

    // Object validations
    if (rule.properties && typeof value === 'object' && !Array.isArray(value)) {
      errors.push(...validateObject(value, rule.properties, fieldPath));
    }
  }

  return errors;
}