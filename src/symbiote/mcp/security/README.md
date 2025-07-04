# MCP Security System

The MCP Security System provides comprehensive security features for Model Context Protocol servers in SymbioteIDE.

## Features

### 🛡️ Trust Management
- **Trust Levels**: Unknown, Untrusted, Limited, User Trusted, Trusted, System
- **Automatic Scanning**: Security analysis before server registration
- **Trust Promotion**: Gradual trust building based on behavior

### 🔒 Permission System
- **Fine-grained Permissions**: Execute tools, read resources, filesystem access, network access
- **User Approval**: Interactive prompts for sensitive operations
- **Permission Memory**: Remember user decisions for future requests

### 📦 Process Sandboxing
- **Filesystem Isolation**: Restricted access to sensitive directories
- **Network Filtering**: Control over network connections
- **Resource Limits**: CPU, memory, and I/O constraints
- **Process Monitoring**: Real-time resource usage tracking

### 📝 Audit Logging
- **Comprehensive Logging**: All security-relevant events
- **Searchable History**: Query by server, event type, time range
- **Violation Tracking**: Record and analyze security violations
- **Compliance Support**: Export audit logs for compliance

### 🚨 Security Alerts
- **Real-time Notifications**: Immediate alerts for security issues
- **VS Code Integration**: Native notification system
- **Alert Categories**: Critical, High, Medium, Low severity
- **Action Buttons**: Quick response to security events

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP Extension                            │
├─────────────────────────────────────────────────────────────┤
│                   Security Manager                           │
├──────────────┬──────────────┬──────────────┬───────────────┤
│   Trust      │  Permission  │   Sandbox    │    Audit      │
│  Manager     │   Manager    │   Manager    │   Logger      │
├──────────────┴──────────────┴──────────────┴───────────────┤
│                    MCP Server Manager                        │
├─────────────────────────────────────────────────────────────┤
│                    MCP Server Processes                      │
└─────────────────────────────────────────────────────────────┘
```

## Configuration

### Basic Settings

```json
{
  "symbiote.mcp.security.enabled": true,
  "symbiote.mcp.security.defaultPolicy": "moderate",
  "symbiote.mcp.security.requireUserApproval": true
}
```

### Advanced Settings

```json
{
  "symbiote.mcp.security.sandbox.enabled": true,
  "symbiote.mcp.security.sandbox.filesystemAccess": "restricted",
  "symbiote.mcp.security.sandbox.networkAccess": "localhost",
  "symbiote.mcp.security.sandbox.limits.maxCpu": 50,
  "symbiote.mcp.security.sandbox.limits.maxMemory": 512
}
```

## Usage

### Security Status

The security status is displayed in the VS Code status bar:
- 🛡️ **Green**: All servers secure
- 🛡️ **Yellow**: Security warnings
- 🛡️ **Red**: Security alerts active

Click the status bar item to open the security panel.

### Managing Trust

1. **View Trust Levels**: Open Command Palette → "MCP: Show Security Panel"
2. **Change Trust**: Right-click server → "Set Trust Level"
3. **Bulk Operations**: Security panel → "Trust All" / "Untrust All"

### Handling Permissions

When a server requests a permission:

1. **Modal Dialog**: Appears with request details
2. **Options**:
   - **Allow Once**: Grant permission for this request only
   - **Allow Always**: Remember permission for this server
   - **Deny**: Block the request

### Security Alerts

When a security issue is detected:

1. **Notification**: VS Code shows alert with details
2. **Actions**:
   - **View Details**: Open full security report
   - **Trust Server**: Mark as trusted
   - **Block Server**: Prevent execution
   - **View Logs**: Examine audit trail

## Security Policies

### Strict Policy
- No automatic permissions
- All operations require approval
- Maximum sandboxing
- Comprehensive auditing

### Moderate Policy (Default)
- Common operations allowed
- Sensitive operations require approval
- Balanced sandboxing
- Standard auditing

### Permissive Policy
- Most operations allowed
- Only critical operations require approval
- Minimal sandboxing
- Basic auditing

## Best Practices

1. **Start with Unknown Trust**: Let servers earn trust gradually
2. **Review Permissions**: Regularly audit granted permissions
3. **Monitor Violations**: Investigate security violations promptly
4. **Update Trust Levels**: Adjust based on server behavior
5. **Export Audit Logs**: Keep records for compliance

## API

### Check Permission

```typescript
const allowed = await securityManager.checkPermission({
  serverName: 'my-server',
  permission: PermissionType.ExecuteTool,
  resource: 'dangerousTool',
  reason: 'Performing system maintenance'
});
```

### Set Trust Level

```typescript
await securityManager.setServerTrust(
  'my-server',
  TrustLevel.UserTrusted
);
```

### Audit Event

```typescript
await securityManager.auditToolExecution('my-server', {
  toolName: 'writeFile',
  arguments: { path: '/tmp/test.txt' },
  timestamp: new Date()
});
```

### Report Violation

```typescript
await securityManager.reportViolation(
  'suspicious-server',
  'unauthorized_access',
  { resource: '/etc/passwd' }
);
```

## Troubleshooting

### Server Won't Start
- Check security scan results in logs
- Verify trust level is sufficient
- Review denied permissions

### Too Many Prompts
- Consider granting permanent permissions
- Adjust security policy if appropriate
- Use "Remember" option in dialogs

### Performance Issues
- Disable scanning for trusted servers
- Adjust resource limits
- Review sandbox configuration

## Security Considerations

1. **Default Deny**: Unknown operations are blocked by default
2. **Least Privilege**: Grant minimum required permissions
3. **Regular Audits**: Review logs for suspicious activity
4. **Update Regularly**: Keep security definitions current
5. **Report Issues**: File security bugs immediately