# Settings - Hierarchical Configuration System Plan

## Goals & Vision

The `settings` crate provides a comprehensive configuration management system for Symbiote. It offers:

- **Hierarchical Configuration**: Multi-level settings with inheritance and overrides
- **Multiple Sources**: Files, environment variables, CLI arguments, and databases
- **Type Safety**: Strongly-typed configuration with validation
- **Hot Reloading**: Dynamic configuration updates without restarts
- **Encryption**: Secure storage of sensitive configuration values
- **Versioning**: Configuration change tracking and rollback
- **Validation**: Schema validation and constraint checking

This crate ensures consistent, secure, and manageable configuration across all Symbiote components.

## UI Design Specifications

### Configuration Management Interface

#### Main Settings Dashboard
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⚙️ Symbiote Settings                         [🔍] [💾] [🔄] [📤] [🔒]      │
├─────────────────────────────────────────────────────────────────────────────┤
│ 📂 Categories │              Configuration Panel              │ 🔍 Search   │
│ (25% width)   │              (60% width)                     │ (15% width) │
│               │                                              │             │
│ 🎯 General    │ 🎯 General Settings                          │ Quick Find: │
│ ├─ Profile    │ ┌─────────────────────────────────────────┐  │ [_______]   │
│ ├─ Appearance │ │ User Profile                            │  │             │
│ └─ Language   │ │ ├─ Name: [John Developer        ]      │  │ Recent:     │
│               │ │ ├─ Email: [john@company.com     ]      │  │ • AI Models │
│ 🤖 AI & Models│ │ ├─ Role: [Senior Developer ▼]          │  │ • Security  │
│ ├─ Providers  │ │ └─ Timezone: [UTC-8 ▼]                │  │ • Workflows │
│ ├─ Models     │ │                                         │  │ • Git       │
│ ├─ Costs      │ │ Workspace Preferences                   │  │             │
│ └─ Limits     │ │ ├─ Default Project: [symbiote-ide ▼]   │  │ Categories: │
│               │ │ ├─ Auto-save: ☑️ Every 30 seconds      │  │ [🎯 General]│
│ 🔒 Security   │ │ ├─ Backup: ☑️ Daily backups            │  │ [🤖 AI]     │
│ ├─ Auth       │ │ └─ Telemetry: ☑️ Anonymous usage       │  │ [🔒 Security│
│ ├─ Permissions│ │                                         │  │ [⚡ Workflow│
│ ├─ Audit      │ │ Appearance                              │  │ [🔧 System] │
│ └─ Compliance │ │ ├─ Theme: [🌙 Dark ▼]                  │  │ [🌐 Network]│
│               │ │ ├─ Font Size: [14px ▼]                 │  │ [📊 Monitor]│
│ ⚡ Workflows   │ │ ├─ Sidebar: [Left ▼]                   │  │ [🔌 Plugins]│
│ ├─ Execution  │ │ └─ Animations: ☑️ Enabled              │  │             │
│ ├─ Nodes      │ │                                         │  │ Modified:   │
│ ├─ Triggers   │ │ [💾 Save Changes] [🔄 Reset] [📤 Export]│  │ • 3 settings│
│ └─ Monitoring │ └─────────────────────────────────────────┘  │ • 2 min ago │
│               │                                              │             │
│ 🔧 System     │ 🔄 Configuration Status                      │ [💾 Apply]  │
│ ├─ Performance│ ┌─────────────────────────────────────────┐  │ [❌ Discard]│
│ ├─ Storage    │ │ ✅ All configurations valid             │  │             │
│ ├─ Backup     │ │ ⚠️ 2 settings require restart           │  │             │
│ └─ Updates    │ │ 🔄 Auto-sync: Enabled                   │  │             │
│               │ │ 📊 Last backup: 2 hours ago             │  │             │
│ [+ Add Custom]│ └─────────────────────────────────────────┘  │             │
│               │                                              │             │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### AI Models Configuration
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🤖 AI Models & Providers Configuration                               [✕]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🔑 API Keys & Authentication                                                │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ OpenAI                                                                  │ │
│ │ ├─ API Key: [sk-proj-••••••••••••••••••••••••••••••••] [🔄] [🧪]      │ │
│ │ ├─ Organization: [org-••••••••••••••••••••] (Optional)                 │ │
│ │ ├─ Status: ✅ Connected (Last verified: 2 min ago)                     │ │
│ │ └─ Usage: $47.32 this month (Limit: $100)                             │ │
│ │                                                                         │ │
│ │ Anthropic                                                               │ │
│ │ ├─ API Key: [sk-ant-••••••••••••••••••••••••••••••••] [🔄] [🧪]       │ │
│ │ ├─ Status: ✅ Connected (Last verified: 5 min ago)                     │ │
│ │ └─ Usage: $23.18 this month (Limit: $50)                              │ │
│ │                                                                         │ │
│ │ Google (Gemini)                                                         │ │
│ │ ├─ API Key: [AIza••••••••••••••••••••••••••••••••••] [🔄] [🧪]        │ │
│ │ ├─ Status: ⚠️ Rate limited (Retry in 15 min)                          │ │
│ │ └─ Usage: $12.45 this month (Limit: $25)                              │ │
│ │                                                                         │ │
│ │ [+ Add Provider] [📊 Usage Analytics] [⚙️ Advanced Settings]          │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🎯 Model Selection & Preferences                                           │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Default Models by Task:                                                 │ │
│ │                                                                         │ │
│ │ 💻 Code Generation:                                                     │ │
│ │ ├─ Primary: [GPT-4 Turbo ▼]        │ Fallback: [Claude-3 Opus ▼]      │ │
│ │ ├─ Temperature: [0.3    ]          │ Max Tokens: [4096    ]            │ │
│ │ └─ Cost Limit: [$0.10 per request] │ Timeout: [30s     ]               │ │
│ │                                                                         │ │
│ │ 📝 Text Generation:                                                     │ │
│ │ ├─ Primary: [Claude-3 Haiku ▼]     │ Fallback: [GPT-3.5 Turbo ▼]      │ │
│ │ ├─ Temperature: [0.7    ]          │ Max Tokens: [2048    ]            │ │
│ │ └─ Cost Limit: [$0.05 per request] │ Timeout: [20s     ]               │ │
│ │                                                                         │ │
│ │ 🧠 Analysis & Reasoning:                                               │ │
│ │ ├─ Primary: [GPT-4 Turbo ▼]        │ Fallback: [Claude-3 Opus ▼]      │ │
│ │ ├─ Temperature: [0.1    ]          │ Max Tokens: [8192    ]            │ │
│ │ └─ Cost Limit: [$0.20 per request] │ Timeout: [60s     ]               │ │
│ │                                                                         │ │
│ │ 🎨 Creative Writing:                                                    │ │
│ │ ├─ Primary: [Claude-3 Opus ▼]      │ Fallback: [GPT-4 Turbo ▼]        │ │
│ │ ├─ Temperature: [0.9    ]          │ Max Tokens: [4096    ]            │ │
│ │ └─ Cost Limit: [$0.15 per request] │ Timeout: [45s     ]               │ │
│ │                                                                         │ │
│ │ [+ Add Task Type] [🧪 Test Configuration] [📊 Performance Stats]      │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 💰 Cost Management                                                          │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Monthly Budget: [$200    ] │ Current Usage: $82.95 (41%)               │ │
│ │ Daily Limit: [$10     ]   │ Today's Usage: $3.47 (35%)                │ │
│ │                                                                         │ │
│ │ Alerts:                                                                 │ │
│ │ ├─ ☑️ 80% of monthly budget                                            │ │
│ │ ├─ ☑️ 90% of daily limit                                               │ │
│ │ ├─ ☑️ Unusual spending patterns                                        │ │
│ │ └─ ☑️ Provider rate limits                                             │ │
│ │                                                                         │ │
│ │ Auto-optimization:                                                      │ │
│ │ ├─ ☑️ Switch to cheaper models when possible                           │ │
│ │ ├─ ☑️ Cache responses to reduce costs                                  │ │
│ │ ├─ ☑️ Batch requests when beneficial                                   │ │
│ │ └─ ☑️ Use free models for non-critical tasks                          │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│                                    [💾 Save Configuration] [🔄 Reset All] │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Security & Permissions Settings
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔒 Security & Permissions Configuration                              [✕]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ 🛡️ Permission Profiles                                                      │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Active Profile: [Zero Trust ▼]                                         │ │
│ │                                                                         │ │
│ │ ● Zero Trust (Maximum Security)                                        │ │
│ │   ├─ All actions require explicit approval                             │ │
│ │   ├─ Network access restricted to whitelist                           │ │
│ │   ├─ File system access sandboxed                                      │ │
│ │   ├─ Real-time monitoring enabled                                      │ │
│ │   └─ Compliance: SOC2, GDPR, HIPAA ready                              │ │
│ │                                                                         │ │
│ │ ○ Human-in-the-Loop (Balanced)                                         │ │
│ │   ├─ Critical actions require approval                                 │ │
│ │   ├─ Routine operations automated                                      │ │
│ │   ├─ Configurable approval thresholds                                  │ │
│ │   └─ Compliance: SOC2, GDPR compliant                                 │ │
│ │                                                                         │ │
│ │ ○ Full Automation (Development Only)                                   │ │
│ │   ├─ All actions automated                                             │ │
│ │   ├─ Minimal security restrictions                                     │ │
│ │   ├─ Audit logging only                                                │ │
│ │   └─ Compliance: Basic audit trail                                     │ │
│ │                                                                         │ │
│ │ [⚙️ Customize Profile] [📋 View Policy Details] [🧪 Test Profile]     │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔐 Authentication & Access                                                  │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Multi-Factor Authentication:                                            │ │
│ │ ├─ 📱 TOTP (Google Authenticator): ☑️ Enabled                         │ │
│ │ ├─ 📧 Email verification: ☑️ Enabled                                   │ │
│ │ ├─ 🔑 Hardware keys (FIDO2): ☐ Disabled                               │ │
│ │ └─ 📞 SMS backup: ☐ Disabled (Not recommended)                        │ │
│ │                                                                         │ │
│ │ Session Management:                                                     │ │
│ │ ├─ Session timeout: [4 hours ▼]                                       │ │
│ │ ├─ Concurrent sessions: [3 devices ▼]                                 │ │
│ │ ├─ Remember device: [30 days ▼]                                       │ │
│ │ └─ Force logout on suspicious activity: ☑️ Enabled                    │ │
│ │                                                                         │ │
│ │ API Access:                                                             │ │
│ │ ├─ Personal access tokens: [2 active] [📋 Manage]                     │ │
│ │ ├─ OAuth applications: [1 authorized] [📋 Manage]                     │ │
│ │ ├─ Webhook signatures: ☑️ Required                                     │ │
│ │ └─ Rate limiting: [1000 req/hour ▼]                                   │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🔍 Audit & Compliance                                                       │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Audit Logging:                                                          │ │
│ │ ├─ Log level: [Detailed ▼]                                            │ │
│ │ ├─ Retention: [1 year ▼]                                              │ │
│ │ ├─ Export format: [JSON ▼]                                            │ │
│ │ └─ Real-time alerts: ☑️ Enabled                                        │ │
│ │                                                                         │ │
│ │ Compliance Standards:                                                   │ │
│ │ ├─ ☑️ SOC 2 Type II                                                    │ │
│ │ ├─ ☑️ GDPR (EU)                                                        │ │
│ │ ├─ ☑️ CCPA (California)                                                │ │
│ │ ├─ ☐ HIPAA (Healthcare)                                                │ │
│ │ └─ ☐ PCI DSS (Payment)                                                 │ │
│ │                                                                         │ │
│ │ Data Protection:                                                        │ │
│ │ ├─ Encryption at rest: [AES-256 ▼]                                    │ │
│ │ ├─ Encryption in transit: [TLS 1.3 ▼]                                 │ │
│ │ ├─ Key rotation: [90 days ▼]                                          │ │
│ │ └─ Backup encryption: ☑️ Enabled                                       │ │
│ │                                                                         │ │
│ │ [📊 View Audit Log] [📤 Export Compliance Report] [🔍 Security Scan]  │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│                                    [💾 Apply Security Settings] [🔄 Reset] │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Advanced System Configuration
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔧 Advanced System Configuration                                     [✕]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ ⚡ Performance & Resources                                                   │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Resource Allocation:                                                    │ │
│ │ ├─ CPU Cores: [8 cores] (Available: 16)                               │ │
│ │ ├─ Memory: [16 GB] (Available: 32 GB)                                 │ │
│ │ ├─ Storage: [500 GB] (Available: 1 TB)                                │ │
│ │ └─ GPU: [NVIDIA RTX 4090] ☑️ Enabled for AI workloads                 │ │
│ │                                                                         │ │
│ │ Performance Tuning:                                                     │ │
│ │ ├─ Worker threads: [Auto ▼] (Currently: 12)                           │ │
│ │ ├─ Connection pool: [100 connections ▼]                               │ │
│ │ ├─ Cache size: [2 GB ▼]                                               │ │
│ │ ├─ Batch size: [1000 items ▼]                                         │ │
│ │ └─ Garbage collection: [Adaptive ▼]                                   │ │
│ │                                                                         │ │
│ │ Monitoring:                                                             │ │
│ │ ├─ CPU usage: 34% (Normal)                                            │ │
│ │ ├─ Memory usage: 12.3 GB (77%)                                        │ │
│ │ ├─ Disk I/O: 45 MB/s (Low)                                            │ │
│ │ └─ Network: 2.1 MB/s (Normal)                                         │ │
│ │                                                                         │ │
│ │ [📊 Detailed Metrics] [⚡ Auto-Optimize] [🔧 Manual Tuning]           │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 💾 Storage & Backup                                                         │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Storage Configuration:                                                  │ │
│ │ ├─ Data directory: [/var/lib/symbiote] [📁 Browse]                    │ │
│ │ ├─ Temp directory: [/tmp/symbiote] [📁 Browse]                        │ │
│ │ ├─ Log directory: [/var/log/symbiote] [📁 Browse]                     │ │
│ │ └─ Cache directory: [/var/cache/symbiote] [📁 Browse]                 │ │
│ │                                                                         │ │
│ │ Backup Strategy:                                                        │ │
│ │ ├─ Frequency: [Daily ▼] at [02:00 ▼]                                  │ │
│ │ ├─ Retention: [30 days ▼]                                             │ │
│ │ ├─ Compression: [ZSTD ▼] (Fast, good ratio)                           │ │
│ │ ├─ Encryption: ☑️ AES-256                                              │ │
│ │ └─ Remote backup: [AWS S3 ▼] [⚙️ Configure]                          │ │
│ │                                                                         │ │
│ │ Cleanup Policies:                                                       │ │
│ │ ├─ Temp files: [Delete after 7 days ▼]                               │ │
│ │ ├─ Log files: [Rotate after 100 MB ▼]                                 │ │
│ │ ├─ Cache: [LRU eviction at 80% full ▼]                                │ │
│ │ └─ Old backups: [Delete after retention period ▼]                     │ │
│ │                                                                         │ │
│ │ [🔄 Backup Now] [📊 Storage Analytics] [🧹 Cleanup Now]               │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ 🌐 Network & Connectivity                                                   │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ Network Configuration:                                                  │ │
│ │ ├─ Listen address: [0.0.0.0:8080] [⚙️ Advanced]                       │ │
│ │ ├─ TLS certificate: [Auto (Let's Encrypt) ▼]                          │ │
│ │ ├─ HTTP/2: ☑️ Enabled                                                  │ │
│ │ ├─ WebSocket: ☑️ Enabled                                               │ │
│ │ └─ CORS origins: [*.symbiote.dev] [+ Add]                             │ │
│ │                                                                         │ │
│ │ Proxy & Load Balancing:                                                 │ │
│ │ ├─ Reverse proxy: [Nginx ▼] [⚙️ Configure]                            │ │
│ │ ├─ Load balancer: [Round Robin ▼]                                     │ │
│ │ ├─ Health checks: [/health] every [30s ▼]                             │ │
│ │ └─ Circuit breaker: ☑️ Enabled (5 failures)                           │ │
│ │                                                                         │ │
│ │ Rate Limiting:                                                          │ │
│ │ ├─ Global: [1000 req/min ▼]                                           │ │
│ │ ├─ Per IP: [100 req/min ▼]                                            │ │
│ │ ├─ Per user: [500 req/min ▼]                                          │ │
│ │ └─ Burst allowance: [50 requests ▼]                                   │ │
│ │                                                                         │ │
│ │ [🧪 Test Connectivity] [📊 Network Stats] [🔧 Advanced Settings]      │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│                                    [💾 Apply All Settings] [🔄 Reset All] │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Database Schema

### Configuration Persistence

```sql
-- Configuration layers and hierarchy
CREATE TABLE configuration_layers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    layer_name VARCHAR(255) NOT NULL UNIQUE,
    layer_type VARCHAR(100) NOT NULL, -- 'system', 'user', 'project', 'environment'
    priority INTEGER NOT NULL,
    source_type VARCHAR(100) NOT NULL, -- 'file', 'database', 'environment', 'cli'
    source_path VARCHAR(500),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Configuration values with hierarchy support
CREATE TABLE configuration_values (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    layer_id UUID REFERENCES configuration_layers(id),
    config_key VARCHAR(500) NOT NULL,
    config_value JSONB NOT NULL,
    value_type VARCHAR(100) NOT NULL, -- 'string', 'number', 'boolean', 'array', 'object'
    is_sensitive BOOLEAN DEFAULT false,
    is_encrypted BOOLEAN DEFAULT false,
    encryption_key_id VARCHAR(255),
    constraints JSONB, -- Validation constraints
    description TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    updated_by VARCHAR(255),
    metadata JSONB,
    UNIQUE(layer_id, config_key)
);

-- Configuration schemas for validation
CREATE TABLE configuration_schemas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_name VARCHAR(255) NOT NULL UNIQUE,
    schema_version VARCHAR(50) NOT NULL,
    schema_definition JSONB NOT NULL, -- JSON Schema definition
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    metadata JSONB
);

-- Configuration change history
CREATE TABLE configuration_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    layer_id UUID REFERENCES configuration_layers(id),
    config_key VARCHAR(500) NOT NULL,
    old_value JSONB,
    new_value JSONB,
    change_type VARCHAR(100), -- 'created', 'updated', 'deleted', 'encrypted', 'decrypted'
    change_reason TEXT,
    changed_at TIMESTAMP DEFAULT NOW(),
    changed_by VARCHAR(255),
    session_id VARCHAR(255),
    rollback_id UUID, -- Reference to rollback operation
    metadata JSONB
);

-- Configuration validation rules
CREATE TABLE configuration_validators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    validator_name VARCHAR(255) NOT NULL UNIQUE,
    validator_type VARCHAR(100) NOT NULL, -- 'builtin', 'custom', 'schema'
    config_pattern VARCHAR(500), -- Key pattern this validator applies to
    validation_rules JSONB NOT NULL,
    error_message_template TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Configuration access policies
CREATE TABLE configuration_access_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_name VARCHAR(255) NOT NULL UNIQUE,
    config_pattern VARCHAR(500) NOT NULL, -- Key pattern this policy applies to
    access_rules JSONB NOT NULL, -- Access control rules
    allowed_operations JSONB, -- Array of allowed operations
    required_permissions JSONB, -- Required permissions
    audit_level VARCHAR(50), -- 'none', 'basic', 'detailed'
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    metadata JSONB
);

-- Configuration watchers and subscriptions
CREATE TABLE configuration_watchers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    watcher_id VARCHAR(255) NOT NULL UNIQUE,
    config_pattern VARCHAR(500) NOT NULL, -- Key pattern to watch
    callback_url VARCHAR(500), -- Webhook URL for notifications
    callback_type VARCHAR(100), -- 'webhook', 'internal', 'queue'
    debounce_ms INTEGER DEFAULT 1000,
    is_active BOOLEAN DEFAULT true,
    last_triggered TIMESTAMP,
    trigger_count INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    metadata JSONB
);

-- Configuration environments and contexts
CREATE TABLE configuration_environments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    environment_name VARCHAR(255) NOT NULL UNIQUE,
    environment_type VARCHAR(100), -- 'development', 'staging', 'production', 'test'
    parent_environment_id UUID REFERENCES configuration_environments(id),
    inheritance_rules JSONB, -- Rules for inheriting from parent
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    metadata JSONB
);

-- Configuration rollback points
CREATE TABLE configuration_rollbacks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rollback_name VARCHAR(255) NOT NULL,
    rollback_description TEXT,
    snapshot_data JSONB NOT NULL, -- Complete configuration snapshot
    created_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255),
    applied_at TIMESTAMP,
    applied_by VARCHAR(255),
    is_applied BOOLEAN DEFAULT false,
    metadata JSONB
);

-- Indexes for performance
CREATE INDEX idx_configuration_layers_type ON configuration_layers(layer_type);
CREATE INDEX idx_configuration_layers_priority ON configuration_layers(priority);
CREATE INDEX idx_configuration_values_layer ON configuration_values(layer_id);
CREATE INDEX idx_configuration_values_key ON configuration_values(config_key);
CREATE INDEX idx_configuration_values_sensitive ON configuration_values(is_sensitive);
CREATE INDEX idx_configuration_history_layer ON configuration_history(layer_id);
CREATE INDEX idx_configuration_history_key ON configuration_history(config_key);
CREATE INDEX idx_configuration_history_changed_at ON configuration_history(changed_at);
CREATE INDEX idx_configuration_validators_pattern ON configuration_validators(config_pattern);
CREATE INDEX idx_configuration_access_policies_pattern ON configuration_access_policies(config_pattern);
CREATE INDEX idx_configuration_watchers_pattern ON configuration_watchers(config_pattern);
CREATE INDEX idx_configuration_environments_type ON configuration_environments(environment_type);
CREATE INDEX idx_configuration_rollbacks_created_at ON configuration_rollbacks(created_at);
```

## Architecture & Design

### Core Modules

```
settings/
├── src/
│   ├── lib.rs                 # Public API exports
│   ├── hierarchy/             # Hierarchical configuration
│   │   ├── mod.rs
│   │   ├── layers.rs         # Configuration layers
│   │   ├── inheritance.rs    # Value inheritance rules
│   │   ├── overrides.rs      # Override mechanisms
│   │   └── resolution.rs     # Value resolution logic
│   ├── sources/              # Configuration sources
│   │   ├── mod.rs
│   │   ├── file.rs           # File-based configuration
│   │   ├── environment.rs    # Environment variables
│   │   ├── cli.rs            # Command-line arguments
│   │   ├── database.rs       # Database-stored settings
│   │   └── remote.rs         # Remote configuration services
│   ├── types/                # Configuration types
│   │   ├── mod.rs
│   │   ├── values.rs         # Configuration value types
│   │   ├── schemas.rs        # Configuration schemas
│   │   └── constraints.rs    # Validation constraints
│   ├── validation/           # Configuration validation
│   │   ├── mod.rs
│   │   ├── schema.rs         # Schema validation
│   │   ├── constraints.rs    # Constraint checking
│   │   └── sanitization.rs   # Value sanitization
│   ├── encryption/           # Secure configuration
│   │   ├── mod.rs
│   │   ├── sensitive.rs      # Sensitive value handling
│   │   ├── vault_integration.rs # Vault integration
│   │   └── key_management.rs # Encryption key management
│   ├── watching/             # Configuration watching
│   │   ├── mod.rs
│   │   ├── file_watcher.rs   # File system watching
│   │   ├── change_detection.rs # Change detection
│   │   └── notifications.rs  # Change notifications
│   ├── versioning/           # Configuration versioning
│   │   ├── mod.rs
│   │   ├── history.rs        # Change history
│   │   ├── rollback.rs       # Configuration rollback
│   │   └── migration.rs      # Configuration migration
│   └── serialization/        # Format support
│       ├── mod.rs
│       ├── json.rs           # JSON format
│       ├── yaml.rs           # YAML format
│       ├── toml.rs           # TOML format
│       └── env.rs            # Environment format
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── basic_config.rs
    └── hierarchical_settings.rs
```

### Key Design Principles

1. **Hierarchy First**: Natural configuration inheritance and overrides
2. **Type Safety**: Compile-time configuration validation
3. **Security**: Encrypted storage of sensitive values
4. **Flexibility**: Support for multiple configuration formats
5. **Observability**: Comprehensive configuration change tracking

## APIs & Interfaces

### Configuration Manager

```rust
pub struct ConfigurationManager {
    hierarchy: ConfigurationHierarchy,
    sources: Vec<Box<dyn ConfigurationSource>>,
    watchers: Vec<Box<dyn ConfigurationWatcher>>,
    validator: ConfigurationValidator,
    encryptor: Option<ConfigurationEncryptor>,
    version_manager: VersionManager,
}

impl ConfigurationManager {
    pub fn new() -> ConfigurationManagerBuilder;
    
    pub async fn load(&mut self) -> SettingsResult<()>;
    
    pub async fn reload(&mut self) -> SettingsResult<()>;
    
    pub fn get<T>(&self, key: &str) -> SettingsResult<T>
    where
        T: DeserializeOwned + ConfigurationValue;
    
    pub fn get_optional<T>(&self, key: &str) -> SettingsResult<Option<T>>
    where
        T: DeserializeOwned + ConfigurationValue;
    
    pub async fn set<T>(&mut self, key: &str, value: T) -> SettingsResult<()>
    where
        T: Serialize + ConfigurationValue;
    
    pub async fn unset(&mut self, key: &str) -> SettingsResult<()>;
    
    pub fn validate(&self) -> SettingsResult<ValidationReport>;
    
    pub async fn save(&self) -> SettingsResult<()>;
    
    pub fn subscribe_to_changes(&self, callback: ConfigurationChangeCallback) -> SettingsResult<SubscriptionId>;
    
    pub fn unsubscribe(&self, subscription_id: SubscriptionId) -> SettingsResult<()>;
}

pub struct ConfigurationManagerBuilder {
    sources: Vec<Box<dyn ConfigurationSource>>,
    watchers: Vec<Box<dyn ConfigurationWatcher>>,
    encryption_enabled: bool,
    validation_enabled: bool,
    versioning_enabled: bool,
}

impl ConfigurationManagerBuilder {
    pub fn add_file_source<P: AsRef<Path>>(mut self, path: P) -> Self;
    
    pub fn add_environment_source(mut self, prefix: Option<String>) -> Self;
    
    pub fn add_cli_source(mut self, args: Vec<String>) -> Self;
    
    pub fn add_database_source(mut self, connection: DatabaseConnection) -> Self;
    
    pub fn enable_encryption(mut self, vault: Arc<VaultManager>) -> Self;
    
    pub fn enable_validation(mut self, schema: ConfigurationSchema) -> Self;
    
    pub fn enable_versioning(mut self, storage: Box<dyn VersionStorage>) -> Self;
    
    pub fn enable_hot_reload(mut self) -> Self;
    
    pub async fn build(self) -> SettingsResult<ConfigurationManager>;
}
```

### Configuration Hierarchy

```rust
pub struct ConfigurationHierarchy {
    layers: Vec<ConfigurationLayer>,
    resolution_strategy: ResolutionStrategy,
}

impl ConfigurationHierarchy {
    pub fn new(strategy: ResolutionStrategy) -> Self;
    
    pub fn add_layer(&mut self, layer: ConfigurationLayer);
    
    pub fn remove_layer(&mut self, layer_id: &str);
    
    pub fn resolve_value(&self, key: &str) -> Option<ConfigurationValue>;
    
    pub fn get_effective_configuration(&self) -> EffectiveConfiguration;
    
    pub fn explain_resolution(&self, key: &str) -> ResolutionExplanation;
}

#[derive(Debug, Clone)]
pub struct ConfigurationLayer {
    pub id: String,
    pub name: String,
    pub priority: u32,
    pub source: String,
    pub values: HashMap<String, ConfigurationValue>,
    pub metadata: LayerMetadata,
}

#[derive(Debug, Clone)]
pub enum ResolutionStrategy {
    FirstWins,      // First layer with value wins
    LastWins,       // Last layer with value wins
    Merge,          // Merge values where possible
    Explicit,       // Explicit override rules
}

#[derive(Debug, Clone)]
pub struct ResolutionExplanation {
    pub key: String,
    pub final_value: Option<ConfigurationValue>,
    pub resolution_path: Vec<ResolutionStep>,
    pub conflicts: Vec<ResolutionConflict>,
}
```

### Configuration Sources

```rust
pub trait ConfigurationSource: Send + Sync {
    fn name(&self) -> &str;
    
    fn priority(&self) -> u32;
    
    async fn load(&self) -> SettingsResult<ConfigurationLayer>;
    
    async fn save(&self, layer: &ConfigurationLayer) -> SettingsResult<()>;
    
    fn supports_watching(&self) -> bool;
    
    fn create_watcher(&self) -> SettingsResult<Box<dyn ConfigurationWatcher>>;
}

pub struct FileConfigurationSource {
    path: PathBuf,
    format: ConfigurationFormat,
    encoding: Encoding,
    permissions: FilePermissions,
}

impl FileConfigurationSource {
    pub fn new<P: AsRef<Path>>(path: P, format: ConfigurationFormat) -> Self;
    
    pub fn with_encoding(mut self, encoding: Encoding) -> Self;
    
    pub fn with_permissions(mut self, permissions: FilePermissions) -> Self;
}

pub struct EnvironmentConfigurationSource {
    prefix: Option<String>,
    separator: String,
    case_sensitive: bool,
    include_system: bool,
}

impl EnvironmentConfigurationSource {
    pub fn new() -> Self;
    
    pub fn with_prefix<S: Into<String>>(mut self, prefix: S) -> Self;
    
    pub fn with_separator<S: Into<String>>(mut self, separator: S) -> Self;
    
    pub fn case_sensitive(mut self, sensitive: bool) -> Self;
}

pub struct DatabaseConfigurationSource {
    connection: DatabaseConnection,
    table_name: String,
    key_column: String,
    value_column: String,
    metadata_column: Option<String>,
}

impl DatabaseConfigurationSource {
    pub fn new(connection: DatabaseConnection) -> Self;
    
    pub fn with_table<S: Into<String>>(mut self, table: S) -> Self;
    
    pub fn with_columns<K, V>(mut self, key_column: K, value_column: V) -> Self
    where
        K: Into<String>,
        V: Into<String>;
}
```

### Configuration Values

```rust
pub trait ConfigurationValue: Send + Sync + Clone {
    fn value_type(&self) -> ConfigurationValueType;
    
    fn is_sensitive(&self) -> bool;
    
    fn validate(&self, constraints: &[Constraint]) -> SettingsResult<()>;
    
    fn merge(&self, other: &Self) -> SettingsResult<Self>;
    
    fn serialize(&self) -> SettingsResult<serde_json::Value>;
    
    fn deserialize(value: &serde_json::Value) -> SettingsResult<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigurationValueType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
    Null,
    Sensitive(Box<ConfigurationValueType>),
}

#[derive(Debug, Clone)]
pub struct SensitiveValue<T> {
    value: T,
    encryption_key_id: Option<String>,
    access_policy: AccessPolicy,
}

impl<T> SensitiveValue<T>
where
    T: ConfigurationValue,
{
    pub fn new(value: T) -> Self;
    
    pub fn with_encryption(mut self, key_id: String) -> Self;
    
    pub fn with_access_policy(mut self, policy: AccessPolicy) -> Self;
    
    pub fn expose(&self, context: &AccessContext) -> SettingsResult<&T>;
    
    pub fn is_encrypted(&self) -> bool;
}
```

### Validation System

```rust
pub struct ConfigurationValidator {
    schema: ConfigurationSchema,
    constraints: Vec<Constraint>,
    custom_validators: HashMap<String, Box<dyn CustomValidator>>,
}

impl ConfigurationValidator {
    pub fn new(schema: ConfigurationSchema) -> Self;
    
    pub fn add_constraint(&mut self, constraint: Constraint);
    
    pub fn add_custom_validator<V>(&mut self, name: String, validator: V)
    where
        V: CustomValidator + 'static;
    
    pub fn validate(&self, configuration: &EffectiveConfiguration) -> SettingsResult<ValidationReport>;
    
    pub fn validate_value(&self, key: &str, value: &ConfigurationValue) -> SettingsResult<()>;
}

#[derive(Debug, Clone)]
pub struct ConfigurationSchema {
    pub properties: HashMap<String, PropertySchema>,
    pub required: Vec<String>,
    pub additional_properties: bool,
    pub metadata: SchemaMetadata,
}

#[derive(Debug, Clone)]
pub struct PropertySchema {
    pub property_type: ConfigurationValueType,
    pub description: Option<String>,
    pub default: Option<ConfigurationValue>,
    pub constraints: Vec<Constraint>,
    pub sensitive: bool,
    pub deprecated: bool,
}

#[derive(Debug, Clone)]
pub enum Constraint {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Range { min: Option<f64>, max: Option<f64> },
    OneOf(Vec<ConfigurationValue>),
    Custom { name: String, parameters: serde_json::Value },
}

pub trait CustomValidator: Send + Sync {
    fn validate(&self, value: &ConfigurationValue, parameters: &serde_json::Value) -> SettingsResult<()>;
    
    fn description(&self) -> &str;
}
```

## Implementation Details

### Technology Stack

- **Serialization**: Serde with JSON, YAML, TOML support
- **File Watching**: notify crate for file system events
- **Encryption**: Integration with vault crate for sensitive values
- **Database**: SQLx for database-backed configuration
- **Validation**: jsonschema for schema validation
- **Hot Reload**: tokio channels for change notifications
- **CLI Parsing**: clap for command-line argument parsing

### Key Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
toml = "0.8"
tokio = { version = "1.0", features = ["full"] }
notify = "6.0"
jsonschema = "0.17"
clap = { version = "4.0", features = ["derive"] }
regex = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
thiserror = "1.0"
tracing = "0.1"
dashmap = "5.0"
parking_lot = "0.12"
symbiote-core = { path = "../symbiote-core" }
symbiote-vault = { path = "../vault" }
symbiote-storage = { path = "../storage" }

[dev-dependencies]
tempfile = "3.0"
tokio-test = "0.4"
```

### Configuration Formats

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigurationFormat {
    Json,
    Yaml,
    Toml,
    Env,
    Ini,
    Properties,
    Auto, // Auto-detect from file extension
}

impl ConfigurationFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            "env" => Some(Self::Env),
            "ini" => Some(Self::Ini),
            "properties" => Some(Self::Properties),
            _ => None,
        }
    }
    
    pub fn serialize<T>(&self, value: &T) -> SettingsResult<String>
    where
        T: Serialize;
    
    pub fn deserialize<T>(&self, content: &str) -> SettingsResult<T>
    where
        T: DeserializeOwned;
}
```

### Hot Reload Implementation

```rust
pub struct ConfigurationWatcher {
    file_watcher: RecommendedWatcher,
    change_sender: mpsc::UnboundedSender<ConfigurationChange>,
    debounce_duration: Duration,
}

impl ConfigurationWatcher {
    pub fn new(debounce_duration: Duration) -> SettingsResult<(Self, mpsc::UnboundedReceiver<ConfigurationChange>)>;
    
    pub fn watch_file<P: AsRef<Path>>(&mut self, path: P) -> SettingsResult<()>;
    
    pub fn watch_directory<P: AsRef<Path>>(&mut self, path: P, recursive: bool) -> SettingsResult<()>;
    
    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> SettingsResult<()>;
}

#[derive(Debug, Clone)]
pub struct ConfigurationChange {
    pub change_type: ChangeType,
    pub source: String,
    pub key: Option<String>,
    pub old_value: Option<ConfigurationValue>,
    pub new_value: Option<ConfigurationValue>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { from: String, to: String },
}
```

## Testing Strategy

### Unit Tests

- **Hierarchy Resolution**: Test value inheritance and override logic
- **Source Loading**: Test all configuration source implementations
- **Validation**: Test schema validation and constraint checking
- **Encryption**: Test sensitive value encryption and decryption
- **Serialization**: Test all supported configuration formats

### Integration Tests

- **End-to-End**: Test complete configuration loading and resolution
- **Hot Reload**: Test file watching and change notifications
- **Cross-Platform**: Test file operations on all platforms
- **Performance**: Test configuration loading performance
- **Concurrent Access**: Test thread safety and concurrent modifications

### Property-Based Tests

- **Hierarchy Consistency**: Test that hierarchy resolution is deterministic
- **Serialization Roundtrip**: Test that serialization preserves values
- **Validation Correctness**: Test that validation rules are enforced

## Integration Points

### Upstream Dependencies

- **symbiote-core**: Uses error types, configuration traits, and DI
- **vault**: Uses secure storage for sensitive configuration values
- **storage**: Uses database for persistent configuration storage

### Downstream Consumers

- **All Symbiote Components**: Configuration management and settings
- **Agent Framework**: Agent-specific configuration and preferences
- **Workflow Engine**: Workflow execution configuration
- **Trading System**: Trading strategy and risk management settings
- **UI Applications**: User interface preferences and themes

### External Integrations

- **Configuration Management**: Consul, etcd, AWS Parameter Store
- **Secret Management**: HashiCorp Vault, AWS Secrets Manager
- **Monitoring**: Configuration change monitoring and alerting
- **CI/CD**: Configuration deployment and validation

## Acceptance Criteria

### Functional Requirements

- [ ] Hierarchical configuration with inheritance and overrides
- [ ] Multiple configuration sources (files, environment, CLI, database)
- [ ] Type-safe configuration with compile-time validation
- [ ] Hot reload with change notifications
- [ ] Encrypted storage of sensitive configuration values
- [ ] Configuration versioning and rollback capabilities
- [ ] Comprehensive validation with custom constraints

### Non-Functional Requirements

- [ ] Sub-millisecond configuration value retrieval
- [ ] Support for 10,000+ configuration keys
- [ ] Memory usage under 50MB for typical configurations
- [ ] Cross-platform file watching support
- [ ] Thread-safe concurrent access
- [ ] Graceful handling of configuration errors

### Quality Gates

- [ ] All tests pass with 95%+ coverage
- [ ] Performance benchmarks meet targets
- [ ] Security audit passes for sensitive value handling
- [ ] Documentation complete with examples
- [ ] Cross-platform compatibility verified
- [ ] Hot reload functionality tested under load

## Dependencies & Prerequisites

### Build Dependencies

- **Rust Toolchain**: 1.70+ with serde support
- **Development Tools**: cargo-watch for development hot reload
- **Schema Tools**: JSON Schema validation tools

### Runtime Dependencies

- **File System**: Read/write access for configuration files
- **Environment**: Access to environment variables
- **Database**: Optional database connection for persistent storage
- **Vault**: Optional vault integration for sensitive values

### Development Prerequisites

- **Configuration Examples**: Sample configuration files for testing
- **Schema Definitions**: JSON Schema files for validation
- **Testing Tools**: Configuration validation and testing utilities
- **Documentation**: Configuration best practices and examples

This settings system provides the flexible, secure, and maintainable configuration foundation that enables all Symbiote components to be properly configured and managed across different environments and use cases.
