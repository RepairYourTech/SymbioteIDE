# SymbioteIDE Development Makefile

.PHONY: help install clean build dev test lint format package release setup

# Default target
help:
	@echo "SymbioteIDE Development Commands:"
	@echo "  make install    - Install dependencies"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make build      - Build the project"
	@echo "  make dev        - Start development mode"
	@echo "  make test       - Run tests"
	@echo "  make lint       - Run linter"
	@echo "  make format     - Format code"
	@echo "  make package    - Package application"
	@echo "  make release    - Create release build"
	@echo "  make setup      - Initial setup"

# Install dependencies
install:
	npm install
	npm run download-builtin-extensions || true

# Clean build artifacts
clean:
	npm run clean

# Clean everything including node_modules
clean-all:
	npm run clean:all

# Build the project
build: clean
	npm run compile
	npm run update-branding

# Start development mode
dev:
	npm run dev

# Quick development mode (SymbioteIDE modules only)
dev-quick:
	npm run dev:quick

# Run tests
test:
	npm test

# Run tests with coverage
test-coverage:
	npm run test:coverage

# Run linter
lint:
	npm run lint:check

# Fix linting issues
lint-fix:
	npm run lint

# Format code
format:
	npm run format

# Check formatting
format-check:
	npm run format:check

# Type checking
typecheck:
	npm run typecheck

# Package application for current platform
package: build
	npm run package

# Package application for all platforms
package-all: build
	npm run package:all

# Create release build
release: lint test build
	npm run release

# Initial setup
setup:
	./scripts/setup-dev.sh
	npm install
	npx husky install

# Update VS Code
update-vscode:
	npm run update-distro

# Generate icons
icons:
	npm run generate-icons

# Start web version
web:
	npm run start:web

# Docker commands
docker-build:
	docker build -t symbiote-ide:latest .

docker-run:
	docker run -p 3000:3000 symbiote-ide:latest

# CI/CD commands
ci-test:
	npm run lint:check
	npm run format:check
	npm run typecheck
	npm test

# Development shortcuts
d: dev
t: test
l: lint
f: format
b: build