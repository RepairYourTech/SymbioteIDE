# Shared Types Package Plan

## Overview
TypeScript type definitions shared across all Symbiote applications (desktop, website, mobile).

## Technology Stack
- **Language**: TypeScript
- **Build**: TSC (TypeScript Compiler)
- **Package Manager**: NPM
- **Testing**: Vitest

## Key Types

### User & Authentication
```typescript
interface User {
  id: string;
  email: string;
  displayName: string;
  subscription: SubscriptionTier;
  createdAt: Date;
  lastLoginAt: Date;
}

interface SubscriptionTier {
  id: string;
  name: 'free' | 'pro' | 'team' | 'enterprise';
  features: string[];
  limits: UsageLimits;
}
```

### Projects & Development
```typescript
interface Project {
  id: string;
  name: string;
  type: ProjectType;
  status: ProjectStatus;
  createdAt: Date;
  updatedAt: Date;
  settings: ProjectSettings;
}

type ProjectType = 'web' | 'mobile' | 'desktop' | 'ai' | 'game' | 'data-science';
```

### AI & Agents
```typescript
interface Agent {
  id: string;
  name: string;
  type: AgentType;
  status: AgentStatus;
  capabilities: string[];
  configuration: AgentConfig;
}

interface AIResponse {
  id: string;
  prompt: string;
  response: string;
  model: string;
  tokens: number;
  cost: number;
  timestamp: Date;
}
```

### API Responses
```typescript
interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: ApiError;
  timestamp: Date;
}

interface ApiError {
  code: string;
  message: string;
  details?: Record<string, any>;
}
```

### Events & Notifications
```typescript
interface NotificationEvent {
  id: string;
  type: NotificationType;
  title: string;
  message: string;
  priority: 'low' | 'medium' | 'high' | 'critical';
  timestamp: Date;
  read: boolean;
}
```

## Package Structure
```
src/
├── auth/           # Authentication types
├── projects/       # Project-related types
├── agents/         # AI agent types
├── api/           # API response types
├── events/        # Event and notification types
├── ui/            # UI component types
└── index.ts       # Main exports
```

## Build & Distribution
- Compiled to CommonJS and ESM
- Type declarations included
- Published to NPM registry
- Versioned with semantic versioning

## Usage
```typescript
import { User, Project, Agent } from '@symbiote/shared-types';
```
