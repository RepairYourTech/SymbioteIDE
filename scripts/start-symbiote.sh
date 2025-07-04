#!/bin/bash
# Start SymbioteIDE Docker Infrastructure - Unix/Linux/macOS

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running in WSL
check_wsl() {
    if grep -qi microsoft /proc/version 2>/dev/null; then
        print_info "WSL detected"
        export IS_WSL=true
        
        # Check if WSL2
        if [[ $(uname -r) =~ "WSL2" ]] || [[ $(uname -r) =~ "microsoft-standard" ]]; then
            print_info "Running on WSL2"
            export IS_WSL2=true
        else
            print_warn "Running on WSL1 - Docker Desktop may not work properly"
        fi
    fi
}

# Check Docker installation
check_docker() {
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed!"
        echo "Please install Docker Desktop from: https://www.docker.com/products/docker-desktop"
        exit 1
    fi
    
    if ! docker info &> /dev/null; then
        print_error "Docker daemon is not running!"
        echo "Please start Docker Desktop and try again."
        exit 1
    fi
    
    print_info "Docker version: $(docker --version)"
    
    # Check Docker Compose
    if docker compose version &> /dev/null; then
        print_info "Docker Compose V2 detected"
        COMPOSE_CMD="docker compose"
    elif command -v docker-compose &> /dev/null; then
        print_info "Docker Compose V1 detected"
        COMPOSE_CMD="docker-compose"
    else
        print_error "Docker Compose is not installed!"
        exit 1
    fi
}

# Check system resources
check_resources() {
    # Check memory
    if [[ "$OSTYPE" == "darwin"* ]]; then
        TOTAL_MEM=$(sysctl -n hw.memsize | awk '{print $1/1024/1024/1024}')
    else
        TOTAL_MEM=$(free -g | awk '/^Mem:/{print $2}')
    fi
    
    if (( $(echo "$TOTAL_MEM < 8" | bc -l) )); then
        print_warn "System has less than 8GB RAM. Performance may be limited."
    else
        print_info "System memory: ${TOTAL_MEM}GB"
    fi
    
    # Check disk space
    AVAILABLE_DISK=$(df -BG . | tail -1 | awk '{print $4}' | sed 's/G//')
    if (( AVAILABLE_DISK < 20 )); then
        print_error "Less than 20GB disk space available!"
        exit 1
    else
        print_info "Available disk space: ${AVAILABLE_DISK}GB"
    fi
}

# Create necessary directories
setup_directories() {
    print_info "Setting up directories..."
    
    # Set paths based on OS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        export WORKSPACE_PATH="${HOME}/SymbioteWorkspace"
        export CONFIG_PATH="${HOME}/.symbiote"
    elif [[ "$IS_WSL" == "true" ]]; then
        # Use Windows user directory for persistence
        WIN_USER=$(cmd.exe /c "echo %USERNAME%" 2>/dev/null | tr -d '\r')
        export WORKSPACE_PATH="/mnt/c/Users/${WIN_USER}/SymbioteWorkspace"
        export CONFIG_PATH="/mnt/c/Users/${WIN_USER}/.symbiote"
    else
        export WORKSPACE_PATH="${HOME}/SymbioteWorkspace"
        export CONFIG_PATH="${HOME}/.symbiote"
    fi
    
    mkdir -p "$WORKSPACE_PATH"
    mkdir -p "$CONFIG_PATH"
    mkdir -p "$CONFIG_PATH/prometheus"
    mkdir -p "$CONFIG_PATH/grafana/provisioning"
    mkdir -p "$CONFIG_PATH/mem0"
    
    print_info "Workspace: $WORKSPACE_PATH"
    print_info "Config: $CONFIG_PATH"
}

# Detect local LLM
detect_local_llm() {
    print_info "Checking for local LLM..."
    
    # Check Ollama
    if command -v ollama &> /dev/null && ollama list &> /dev/null; then
        print_info "Ollama detected"
        export USE_OLLAMA=false
        export LOCAL_LLM_TYPE=ollama
        export LOCAL_LLM_ENDPOINT="http://host.docker.internal:11434"
    # Check LM Studio
    elif curl -s http://localhost:1234/v1/models &> /dev/null; then
        print_info "LM Studio detected"
        export USE_OLLAMA=false
        export LOCAL_LLM_TYPE=lm-studio
        export LOCAL_LLM_ENDPOINT="http://host.docker.internal:1234"
    else
        print_warn "No local LLM detected. Ollama will be included in the stack."
        export USE_OLLAMA=true
    fi
}

# Start services
start_services() {
    print_info "Starting SymbioteIDE services..."
    
    cd "$(dirname "$0")/.."
    
    # Determine compose files
    COMPOSE_FILES="-f docker/docker-compose.yml"
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        COMPOSE_FILES="$COMPOSE_FILES -f docker/docker-compose.macos.yml"
    elif [[ "$IS_WSL" == "true" ]] || [[ "$OSTYPE" == "msys" ]]; then
        COMPOSE_FILES="$COMPOSE_FILES -f docker/docker-compose.windows.yml"
    fi
    
    # Build command
    if [[ "$USE_OLLAMA" == "true" ]]; then
        SERVICES="symbiote-ide orchestrator agent-runtime neo4j qdrant redis mem0 rabbitmq langfuse postgres prometheus grafana ollama"
    else
        SERVICES="symbiote-ide orchestrator agent-runtime neo4j qdrant redis mem0 rabbitmq langfuse postgres prometheus grafana"
    fi
    
    # Pull images first
    print_info "Pulling Docker images..."
    $COMPOSE_CMD $COMPOSE_FILES pull $SERVICES
    
    # Start services
    print_info "Starting services..."
    $COMPOSE_CMD $COMPOSE_FILES up -d $SERVICES
    
    # Wait for services
    print_info "Waiting for services to be healthy..."
    sleep 10
    
    # Check health
    check_health
}

# Check service health
check_health() {
    print_info "Checking service health..."
    
    SERVICES=(
        "symbiote-ide:8080/health"
        "orchestrator:9000/health"
        "neo4j:7474"
        "qdrant:6333/health"
        "mem0:8000/health"
        "langfuse:3030/api/public/health"
        "prometheus:9090/-/healthy"
        "grafana:3031/api/health"
    )
    
    ALL_HEALTHY=true
    
    for SERVICE in "${SERVICES[@]}"; do
        IFS=':' read -r NAME URL <<< "$SERVICE"
        if curl -s -f "http://localhost:$URL" > /dev/null 2>&1; then
            echo -e "  ${GREEN}✓${NC} $NAME"
        else
            echo -e "  ${RED}✗${NC} $NAME"
            ALL_HEALTHY=false
        fi
    done
    
    if [[ "$ALL_HEALTHY" == "true" ]]; then
        print_info "All services are healthy!"
    else
        print_warn "Some services are not healthy yet. They may still be starting..."
    fi
}

# Show URLs
show_urls() {
    echo ""
    print_info "Service URLs:"
    echo "  - SymbioteIDE:       http://localhost:8080"
    echo "  - Neo4j Browser:     http://localhost:7474"
    echo "  - RabbitMQ Mgmt:     http://localhost:15672"
    echo "  - Langfuse:          http://localhost:3030"
    echo "  - Prometheus:        http://localhost:9090"
    echo "  - Grafana:           http://localhost:3031"
    echo ""
    echo "Default credentials:"
    echo "  - Neo4j:     neo4j / symbiote123"
    echo "  - RabbitMQ:  symbiote / symbiote123"
    echo "  - Grafana:   admin / admin"
}

# Main execution
main() {
    echo "🚀 Starting SymbioteIDE Docker Infrastructure"
    echo ""
    
    check_wsl
    check_docker
    check_resources
    setup_directories
    detect_local_llm
    start_services
    show_urls
    
    echo ""
    print_info "Setup complete! Opening SymbioteIDE..."
    
    # Open browser
    if [[ "$OSTYPE" == "darwin"* ]]; then
        open http://localhost:8080
    elif [[ "$IS_WSL" == "true" ]]; then
        cmd.exe /c start http://localhost:8080 2>/dev/null
    elif command -v xdg-open &> /dev/null; then
        xdg-open http://localhost:8080
    fi
}

# Run main
main