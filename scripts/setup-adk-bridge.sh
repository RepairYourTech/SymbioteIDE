#!/bin/bash

# Setup script for Google ADK Bridge Server

echo "Setting up Google ADK Bridge Server for SymbioteIDE..."

# Check if Python is installed
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is not installed. Please install Python 3.10 or higher."
    exit 1
fi

# Check Python version
PYTHON_VERSION=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
if [[ $(echo "$PYTHON_VERSION < 3.10" | bc) -eq 1 ]]; then
    echo "Error: Python 3.10 or higher is required. Found Python $PYTHON_VERSION"
    exit 1
fi

echo "Found Python $PYTHON_VERSION"

# Create virtual environment
echo "Creating Python virtual environment..."
python3 -m venv .venv-adk

# Activate virtual environment
source .venv-adk/bin/activate

# Upgrade pip
echo "Upgrading pip..."
pip install --upgrade pip

# Install Google ADK and dependencies
echo "Installing Google ADK..."
pip install google-adk uvicorn[standard] fastapi python-dotenv

# Check if GOOGLE_API_KEY is set
if [ -z "$GOOGLE_API_KEY" ]; then
    echo ""
    echo "Warning: GOOGLE_API_KEY environment variable is not set."
    echo "You'll need to set it before using Google ADK:"
    echo "  export GOOGLE_API_KEY='your-api-key'"
    echo ""
fi

# Create systemd service file (optional)
if [ "$1" == "--service" ]; then
    echo "Creating systemd service..."
    cat > /tmp/symbiote-adk-bridge.service << EOF
[Unit]
Description=SymbioteIDE ADK Bridge Server
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$(pwd)
Environment="PATH=$(pwd)/.venv-adk/bin:$PATH"
Environment="GOOGLE_API_KEY=$GOOGLE_API_KEY"
ExecStart=$(pwd)/.venv-adk/bin/python src/symbiote/adk/python-bridge/adk_server.py
Restart=always

[Install]
WantedBy=multi-user.target
EOF

    sudo mv /tmp/symbiote-adk-bridge.service /etc/systemd/system/
    sudo systemctl daemon-reload
    echo "Service created. You can start it with:"
    echo "  sudo systemctl start symbiote-adk-bridge"
    echo "  sudo systemctl enable symbiote-adk-bridge"
fi

echo ""
echo "Setup complete! To run the ADK bridge server:"
echo "  source .venv-adk/bin/activate"
echo "  python src/symbiote/adk/python-bridge/adk_server.py"
echo ""
echo "Or use the npm script:"
echo "  npm run adk:bridge"