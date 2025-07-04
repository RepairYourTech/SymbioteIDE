# SymbioteIDE Credential Management System

A secure, zero-knowledge credential management system for storing and managing API keys across multiple AI providers.

## Features

### 🔐 Zero-Knowledge Architecture
- Credentials are encrypted at rest using OS-provided security mechanisms
- API keys never leave the user's machine unencrypted
- No server-side storage or transmission of credentials
- Automatic memory clearing after use

### 🖥️ Cross-Platform Support
- **Windows**: Windows Credential Manager
- **macOS**: macOS Keychain Services
- **Linux**: Secret Service API (GNOME Keyring, KWallet)

### 🛡️ Security Features
- API key format validation for each provider
- Secure transmission with TLS verification
- Audit logging without exposing sensitive data
- Automatic credential leak detection
- Session-based caching with timeout
- Key rotation reminders

### 🔌 Provider Support
- Anthropic (Claude)
- OpenAI (GPT)
- Google (Gemini)
- Mistral
- Azure OpenAI
- AWS Bedrock
- Custom providers

## Installation

The credential management system is built into SymbioteIDE. No additional installation required.

## Usage

### Basic Usage

```typescript
import { createCredentialManager } from '@symbiote/credentials';

// Create credential manager
const credentialManager = await createCredentialManager();

// Add a new credential
const credential = await credentialManager.createCredential({
  provider: 'anthropic',
  name: 'My Claude API Key',
  apiKey: 'sk-ant-...'
});

// List all credentials
const credentials = await credentialManager.listCredentials();

// Use a credential (returns actual API key)
const fullCredential = await credentialManager.useCredential(credential.id);
```

### Integration with Orchestration Engine

The credential manager automatically integrates with the orchestration engine:

```typescript
import { initializeCredentialSystem } from '@symbiote/credentials';

// Initialize credential system
const credentialManager = await initializeCredentialSystem();

// Provider adapters will automatically use stored credentials
const engine = new OrchestrationEngine();
// API keys are automatically injected from credential manager
```

## Security Best Practices

### 1. Never Log API Keys
The system includes automatic sanitization to prevent accidental logging:

```typescript
import { KeySanitizer } from '@symbiote/credentials';

// Create safe logger
const logger = KeySanitizer.createSafeLogger();
logger.log('API key:', apiKey); // Automatically redacted
```

### 2. Use Zero-Knowledge Vault
Access credentials through the zero-knowledge vault:

```typescript
const vault = new ZeroKnowledgeVault();

await vault.accessCredential(
  credentialId,
  async (credential) => {
    // Use credential here
    // It will be automatically cleared after this function
  },
  getCredentialFunction
);
```

### 3. Enable Audit Logging
Track all credential operations:

```typescript
const auditLogger = new AuditLogger({
  fileOutput: true,
  logDirectory: './audit-logs'
});

// Search audit logs
const logs = await auditLogger.searchLogs({
  startDate: new Date('2024-01-01'),
  action: 'credential_access'
});
```

## API Reference

### CredentialManager

#### Methods

- `createCredential(request: CredentialCreateRequest): Promise<SecureCredential>`
- `getCredential(id: string): Promise<SecureCredential | null>`
- `useCredential(id: string): Promise<Credential | null>`
- `updateCredential(id: string, update: CredentialUpdateRequest): Promise<SecureCredential | null>`
- `rotateCredential(id: string, newApiKey: string): Promise<SecureCredential | null>`
- `deleteCredential(id: string): Promise<boolean>`
- `listCredentials(): Promise<SecureCredential[]>`
- `getCredentialsByProvider(provider: ProviderType): Promise<SecureCredential[]>`

#### Events

- `credential:added` - Emitted when a credential is added
- `credential:updated` - Emitted when a credential is updated
- `credential:deleted` - Emitted when a credential is deleted
- `credential:accessed` - Emitted when a credential is accessed
- `credential:rotation_needed` - Emitted when key rotation is recommended

### Validation

#### Supported Formats

- **Anthropic**: `sk-ant-[40+ characters]`
- **OpenAI**: `sk-[48 characters]`
- **Google**: `[39 characters]`
- **Mistral**: `[32 hex characters]`
- **Azure**: `[32 hex characters]`
- **AWS**: `AKIA[16 characters]`

## Platform-Specific Notes

### Windows
- Uses Windows Credential Manager
- Requires no additional setup
- Credentials are stored per-user

### macOS
- Uses macOS Keychain Services
- May prompt for keychain access on first use
- Credentials are stored in the login keychain

### Linux
- Requires `secret-tool` command (usually pre-installed)
- Uses Secret Service API
- Works with GNOME Keyring, KWallet, etc.

## Environment Variables

Fallback environment variables (when credentials are not found):
- `ANTHROPIC_API_KEY`
- `OPENAI_API_KEY`
- `GOOGLE_API_KEY`
- `MISTRAL_API_KEY`
- `AZURE_OPENAI_API_KEY`
- `AWS_ACCESS_KEY_ID`

## Troubleshooting

### Platform Not Supported
If your platform doesn't support secure credential storage, the system will fall back to environment variables with a warning.

### Access Denied
On macOS, you may need to grant SymbioteIDE access to the keychain. Check System Preferences > Security & Privacy.

### Linux Secret Service Not Available
Install one of the following:
- GNOME: `sudo apt install gnome-keyring`
- KDE: `sudo apt install kwalletmanager`

## Security Considerations

1. **API Keys are irreversibly encrypted** - If you lose access to your OS keychain, you'll need to re-enter your API keys
2. **No backup mechanism** - For security, there's no way to export credentials in plain text
3. **Per-machine storage** - Credentials don't sync between devices
4. **Automatic cleanup** - Credentials are cleared from memory after 5 minutes of inactivity

## Contributing

When contributing to the credential management system:

1. Never add logging that could expose API keys
2. Always use the `KeySanitizer` for any string operations
3. Test on all three platforms (Windows, macOS, Linux)
4. Add tests for any new validation patterns
5. Update documentation for new providers

## License

Part of SymbioteIDE - see main project license.