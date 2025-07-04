#!/bin/bash
# SymbioteIDE Development Environment Setup Script

echo "Setting up SymbioteIDE development environment..."

# Check Node.js version
NODE_VERSION=$(node -v | cut -d'v' -f2)
REQUIRED_VERSION="18.15.0"

if ! command -v node &> /dev/null; then
    echo "Error: Node.js is not installed"
    exit 1
fi

echo "✓ Node.js version: $NODE_VERSION"

# Check Python
if ! command -v python3 &> /dev/null; then
    echo "Warning: Python 3 is not installed. Some native modules may fail to build."
fi

# Check build tools
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    if ! command -v gcc &> /dev/null || ! command -v g++ &> /dev/null || ! command -v make &> /dev/null; then
        echo "Error: Build tools not found. Please install build-essential:"
        echo "  sudo apt-get install build-essential"
        exit 1
    fi
elif [[ "$OSTYPE" == "darwin"* ]]; then
    if ! command -v clang &> /dev/null; then
        echo "Error: Xcode Command Line Tools not found. Please install:"
        echo "  xcode-select --install"
        exit 1
    fi
fi

echo "✓ Build tools detected"

# Create necessary directories
mkdir -p out
mkdir -p out-extensions
mkdir -p .build
mkdir -p .download

echo "✓ Created build directories"

# Check if VS Code repository needs to be cloned
if [ ! -d "vscode" ]; then
    echo "Cloning VS Code repository..."
    git clone https://github.com/microsoft/vscode.git vscode
    cd vscode
    git remote add upstream https://github.com/microsoft/vscode.git
    cd ..
    echo "✓ VS Code repository cloned"
fi

# Set up environment variables
export SYMBIOTE_DEV=1
export NODE_ENV=development

echo "✓ Environment variables set"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
fi

echo ""
echo "✅ Development environment setup complete!"
echo ""
echo "Next steps:"
echo "1. Run 'npm run dev' to start the development server"
echo "2. Open the project in your editor"
echo "3. Check out the documentation in docs/symbiote/"
echo ""
echo "Happy coding! 🚀"