#!/bin/bash

# Setup script for chat webview
echo "Setting up SymbioteIDE Chat Webview..."

# Navigate to chat webview directory
cd src/symbiote/chat/webview

# Install dependencies
echo "Installing chat webview dependencies..."
npm install

# Build the webview
echo "Building chat webview..."
npm run build

echo "Chat webview setup complete!"