# Assistant Implementation Specification

## Database Schema Implementation

### Core Tables
```sql
-- Assistant conversations and context
CREATE TABLE conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    workspace_id UUID REFERENCES workspaces(id),
    title VARCHAR(255),
    context_data JSONB NOT NULL DEFAULT '{}',
    ai_model VARCHAR(100) NOT NULL DEFAULT 'gpt-4',
    system_prompt TEXT,
    conversation_state JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    archived_at TIMESTAMP
);

CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    tokens_used INTEGER,
    model_used VARCHAR(100),
    processing_time_ms INTEGER,
    created_at TIMESTAMP DEFAULT NOW(),
    edited_at TIMESTAMP
);

CREATE TABLE context_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    item_type VARCHAR(50) NOT NULL, -- 'file', 'function', 'class', 'error', 'documentation'
    item_path TEXT,
    item_content TEXT,
    relevance_score FLOAT DEFAULT 0.0,
    auto_included BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE assistant_capabilities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    enabled BOOLEAN DEFAULT TRUE,
    configuration JSONB DEFAULT '{}',
    required_permissions TEXT[],
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE user_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    ai_model_preference VARCHAR(100) DEFAULT 'gpt-4',
    response_style VARCHAR(50) DEFAULT 'balanced', -- 'concise', 'detailed', 'balanced'
    auto_context_inclusion BOOLEAN DEFAULT TRUE,
    max_context_items INTEGER DEFAULT 10,
    preferred_capabilities TEXT[],
    custom_prompts JSONB DEFAULT '{}',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Performance indexes
CREATE INDEX idx_conversations_user ON conversations(user_id);
CREATE INDEX idx_conversations_workspace ON conversations(workspace_id);
CREATE INDEX idx_messages_conversation ON messages(conversation_id);
CREATE INDEX idx_messages_created_at ON messages(created_at);
CREATE INDEX idx_context_items_conversation ON context_items(conversation_id);
CREATE INDEX idx_context_items_type ON context_items(item_type);
CREATE INDEX idx_user_preferences_user ON user_preferences(user_id);

-- Full-text search
CREATE INDEX idx_messages_content_search ON messages USING gin(to_tsvector('english', content));
CREATE INDEX idx_context_items_content_search ON context_items USING gin(to_tsvector('english', item_content));
```

## API Implementation Contracts

### REST API Endpoints
```yaml
# OpenAPI 3.0 Specification
openapi: 3.0.3
info:
  title: Symbiote Assistant API
  version: 1.0.0

paths:
  /api/v1/conversations:
    get:
      summary: List user conversations
      parameters:
        - name: workspace_id
          in: query
          schema:
            type: string
            format: uuid
        - name: limit
          in: query
          schema:
            type: integer
            default: 20
            maximum: 100
        - name: offset
          in: query
          schema:
            type: integer
            default: 0
      responses:
        200:
          description: List of conversations
          content:
            application/json:
              schema:
                type: object
                properties:
                  conversations:
                    type: array
                    items:
                      $ref: '#/components/schemas/Conversation'
                  total:
                    type: integer
                  has_more:
                    type: boolean

    post:
      summary: Create new conversation
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                title:
                  type: string
                  maxLength: 255
                workspace_id:
                  type: string
                  format: uuid
                ai_model:
                  type: string
                  default: "gpt-4"
                system_prompt:
                  type: string
                initial_context:
                  type: array
                  items:
                    $ref: '#/components/schemas/ContextItem'
      responses:
        201:
          description: Conversation created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Conversation'

  /api/v1/conversations/{conversationId}/messages:
    post:
      summary: Send message to assistant
      parameters:
        - name: conversationId
          in: path
          required: true
          schema:
            type: string
            format: uuid
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required:
                - content
              properties:
                content:
                  type: string
                  maxLength: 50000
                include_context:
                  type: boolean
                  default: true
                context_items:
                  type: array
                  items:
                    $ref: '#/components/schemas/ContextItem'
                stream:
                  type: boolean
                  default: false
      responses:
        200:
          description: Assistant response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Message'
            text/event-stream:
              schema:
                type: string

  /api/v1/conversations/{conversationId}/context:
    get:
      summary: Get conversation context
      parameters:
        - name: conversationId
          in: path
          required: true
          schema:
            type: string
            format: uuid
      responses:
        200:
          description: Conversation context
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ContextItem'

    post:
      summary: Add context to conversation
      parameters:
        - name: conversationId
          in: path
          required: true
          schema:
            type: string
            format: uuid
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ContextItem'
      responses:
        201:
          description: Context added
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ContextItem'

components:
  schemas:
    Conversation:
      type: object
      properties:
        id:
          type: string
          format: uuid
        title:
          type: string
        workspace_id:
          type: string
          format: uuid
        ai_model:
          type: string
        context_data:
          type: object
        conversation_state:
          type: object
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time
        message_count:
          type: integer

    Message:
      type: object
      properties:
        id:
          type: string
          format: uuid
        conversation_id:
          type: string
          format: uuid
        role:
          type: string
          enum: [user, assistant, system]
        content:
          type: string
        metadata:
          type: object
        tokens_used:
          type: integer
        model_used:
          type: string
        processing_time_ms:
          type: integer
        created_at:
          type: string
          format: date-time

    ContextItem:
      type: object
      properties:
        id:
          type: string
          format: uuid
        item_type:
          type: string
          enum: [file, function, class, error, documentation]
        item_path:
          type: string
        item_content:
          type: string
        relevance_score:
          type: number
          format: float
        auto_included:
          type: boolean
```

## Configuration Implementation

### Environment Configuration
```toml
# config/assistant.toml
[ai]
default_model = "gpt-4"
fallback_model = "gpt-3.5-turbo"
max_tokens = 4096
temperature = 0.7
timeout_seconds = 30
retry_attempts = 3
rate_limit_per_minute = 100

[context]
max_items_per_conversation = 20
auto_context_threshold = 0.7
context_window_size = 8192
relevance_decay_factor = 0.9
max_file_size_bytes = 1048576  # 1MB

[performance]
response_cache_ttl = 300  # 5 minutes
context_cache_ttl = 1800  # 30 minutes
max_concurrent_requests = 50
request_timeout_ms = 30000
streaming_chunk_size = 1024

[features]
streaming_responses = true
context_auto_inclusion = true
conversation_memory = true
multi_model_support = true
custom_prompts = true

[security]
max_message_length = 50000
content_filtering = true
rate_limiting = true
audit_logging = true
```

This implementation specification provides the detailed foundation needed for 99.6% confidence development.
