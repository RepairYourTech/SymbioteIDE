# SymbioteIDE Build Guide

SymbioteIDE is a unified AI-powered development environment. This guide explains how to build SymbioteIDE from source and create distributable installers.

## Prerequisites

- Node.js 22.15.1 (use nvm: `nvm install 22.15.1 && nvm use`)
- Python 3.x (for ADK bridge)
- Git
- npm or yarn
- Platform-specific build tools:
  - **Windows**: Visual Studio 2022 with C++ workload
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `build-essential` package

## Quick Start

```bash
# Clone the repository
git clone https://github.com/symbiote-ide/symbiote-ide.git
cd symbiote-ide

# Install correct Node version
nvm install
nvm use

# Install dependencies
npm install

# Build SymbioteIDE
npm run build

# Run development instance
npm run start
```

## Build Commands

### Development

- `npm run compile` - Compile SymbioteIDE (quick build for development)
- `npm run watch` - Watch mode for development (auto-recompile on changes)
- `npm run start` - Launch SymbioteIDE in development mode
- `npm run start:dev` - Start with watch mode enabled

### Production

- `npm run build` - Full production build (includes all components and ADK setup)
- `npm run dist` - Build and create installers for all platforms
- `npm run dist:win` - Create Windows installer (.exe)
- `npm run dist:mac` - Create macOS installer (.dmg)
- `npm run dist:linux` - Create Linux packages (.deb, .rpm, .AppImage)

## Build Output

After running `npm run dist`, installers will be available in the `dist/` directory:

- **Windows**: 
  - `SymbioteIDE-Setup-{version}.exe` (NSIS installer)
  - `SymbioteIDE-{version}-win.zip` (Portable)
- **macOS**: 
  - `SymbioteIDE-{version}.dmg` (Disk image)
  - `SymbioteIDE-{version}-mac.zip` (Portable)
- **Linux**:
  - `SymbioteIDE-{version}.deb` (Debian/Ubuntu)
  - `SymbioteIDE-{version}.rpm` (Red Hat/Fedora)
  - `SymbioteIDE-{version}.AppImage` (Universal)
  - `SymbioteIDE-{version}.snap` (Snap package)

## What Gets Built

The unified build process includes:

1. **Core Editor** - The VS Code foundation
2. **AI Features** - All SymbioteIDE AI enhancements
3. **ADK Bridge** - Python environment for AI Development Kit
4. **Chat Interface** - Web-based chat UI
5. **Orchestration Engine** - Multi-model AI orchestration
6. **Extensions** - Built-in extensions

## Troubleshooting

### Node Version Issues
```bash
# Ensure you're using Node 22.15.1
node --version  # Should show v22.15.1
nvm use        # Switch to project's Node version
```

### Build Failures
```bash
# Clean build artifacts
rm -rf out/
rm -rf .build/
rm -rf dist/

# Reinstall dependencies
rm -rf node_modules/
npm install

# Try building again
npm run build
```

### Python/ADK Issues
```bash
# Ensure Python 3 is installed
python3 --version

# ADK setup is automatic during build, but can be run manually:
gulp setup-adk-env
```

## Architecture

SymbioteIDE uses a unified build system where:
- All TypeScript compilation happens in one step
- Resources are copied automatically
- Python environments are set up during build
- Electron packaging creates platform-specific installers

The build pipeline:
1. Validates integration points
2. Compiles TypeScript (editor + AI features)
3. Builds web components
4. Generates API types
5. Sets up Python environment
6. Packages with electron-builder

## Customization

### Branding
Edit `product.json` to customize:
- Application name
- Icons
- Bundle identifiers
- Credits

### Build Configuration
- `electron-builder.yml` - Installer configuration
- `build/gulpfile.js` - Build tasks
- `package.json` - Build scripts

## Code Signing

For production releases, code signing is required:

### Windows
Set environment variables:
- `CSC_LINK` - Path to .pfx certificate
- `CSC_KEY_PASSWORD` - Certificate password

### macOS
- `CSC_LINK` - Path to .p12 certificate
- `CSC_KEY_PASSWORD` - Certificate password
- `APPLE_ID` - Apple Developer ID
- `APPLE_ID_PASSWORD` - App-specific password

### Linux
No code signing required, but GPG signing recommended for repositories.

## Release Process

1. Update version in `package.json`
2. Run `npm run build` to ensure everything compiles
3. Run `npm run dist` to create installers
4. Test installers on each platform
5. Sign installers (if certificates available)
6. Upload to release server

## CI/CD Integration

For automated builds, use:
```yaml
# Example GitHub Actions
- name: Build SymbioteIDE
  run: |
    npm install
    npm run build
    npm run dist
```

## License

SymbioteIDE is proprietary software. All rights reserved.