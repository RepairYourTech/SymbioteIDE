# SymbioteIDE Docker Infrastructure

This directory contains the Docker configuration for running SymbioteIDE's complete infrastructure stack.

## Quick Start

### Windows
```powershell
# Run in PowerShell
.\scripts\start-symbiote.ps1
```

### macOS/Linux/WSL
```bash
# Run in terminal
./scripts/start-symbiote.sh
```

## Manual Setup

### 1. Start all services
```bash
# Default setup (without Ollama)
docker compose up -d

# With Ollama for local AI
docker compose up -d ollama
```

### 2. Platform-specific setup

#### Windows
```bash
docker compose -f docker-compose.yml -f docker-compose.windows.yml up -d
```

#### macOS
```bash
docker compose -f docker-compose.yml -f docker-compose.macos.yml up -d
```

## Services

### Required Services
- **SymbioteIDE** (port 8080) - Main IDE interface
- **Orchestrator** (port 9000) - Multi-model AI orchestration
- **Neo4j** (port 7474/7687) - Graph database for code relationships
- **Qdrant** (port 6333/6334) - Vector database for semantic search
- **Redis** (port 6379) - Caching and session management
- **Mem0** (port 8000) - Persistent memory layer
- **RabbitMQ** (port 5672/15672) - Message queue for agents
- **Langfuse** (port 3030) - LLM observability
- **PostgreSQL** (port 5432) - Database for Langfuse
- **Prometheus** (port 9090) - Metrics collection
- **Grafana** (port 3031) - Metrics visualization

### Optional Services
- **Ollama** (port 11434) - Local LLM runtime (only if needed)

## Configuration

### Environment Variables
- `WORKSPACE_PATH` - Directory for workspace files
- `CONFIG_PATH` - Directory for configuration files

### Default Credentials
- Neo4j: `neo4j / symbiote123`
- RabbitMQ: `symbiote / symbiote123`
- Redis: `symbiote123`
- Grafana: `admin / admin`
- PostgreSQL: `langfuse / langfuse123`

## Local LLM Options

SymbioteIDE supports multiple local LLM providers:
- **Ollama** - Included in stack if no other LLM detected
- **LM Studio** - Auto-detected at `http://localhost:1234`
- **llama.cpp** - Auto-detected at `http://localhost:8080`
- **Custom** - Configure any OpenAI-compatible endpoint

## Troubleshooting

### Check service health
```bash
docker compose ps
docker compose logs <service-name>
```

### Restart a service
```bash
docker compose restart <service-name>
```

### Stop all services
```bash
docker compose down
```

### Remove all data (fresh start)
```bash
docker compose down -v
```

## Resource Requirements

### Minimum
- 8GB RAM
- 20GB disk space
- Docker Desktop or Docker CE

### Recommended
- 16GB+ RAM
- 50GB+ disk space
- GPU for local AI models

## Platform Notes

### Windows
- Requires Docker Desktop with WSL2 backend
- Use PowerShell script for best experience
- Named volumes used for better performance

### macOS
- Docker Desktop required
- Optimized for both Intel and Apple Silicon
- Lower memory limits due to VM overhead

### WSL
- WSL2 strongly recommended over WSL1
- Shares Docker Desktop with Windows host
- Path translation handled automatically