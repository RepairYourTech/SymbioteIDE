# Centralized Supabase Authentication

This is a centralized authentication system for SymbioteIDE using Supabase.

## Features

- **Required Authentication**: Users must authenticate to use SymbioteIDE
- **Centralized Control**: Authentication is managed through a single Supabase project
- **Session Management**: Automatic token refresh and secure storage
- **VS Code Integration**: Status bar indicator and authentication provider

## How It Works

1. **On Extension Activation**: 
   - Checks for existing session
   - If not authenticated, prompts for login
   - Blocks all features until authenticated

2. **Authentication Required**:
   - All commands check authentication status
   - If session expires, prompts to re-login
   - No access without valid credentials

3. **User Management**:
   - Users are managed in the central Supabase dashboard
   - Account creation controlled by admin
   - Access can be revoked by disabling users

## Usage

### For Users:
- Install SymbioteIDE extension
- On first use, you'll be prompted to sign in
- Enter the email and password provided to you
- Once authenticated, full access to all features

### For Admin:
- Manage users in Supabase dashboard
- Create accounts for authorized users
- Monitor usage and revoke access as needed

## Security

- Credentials are hardcoded in the extension (public anon key only)
- Auth tokens stored securely using VS Code's SecretStorage API
- Sessions persist between VS Code restarts
- Automatic token refresh

## Architecture

```
auth/
├── types.ts              # User and session types
├── supabase-client.ts    # Supabase client with hardcoded credentials
├── auth-manager.ts       # Core auth logic and auth gates
└── auth-provider.ts      # VS Code authentication provider
```

The auth system ensures only authorized users can access SymbioteIDE features.