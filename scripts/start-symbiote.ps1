# Start SymbioteIDE Docker Infrastructure - Windows PowerShell

# Enable strict mode
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Colors for output
function Write-Info {
    param($Message)
    Write-Host "[INFO] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param($Message)
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error-Message {
    param($Message)
    Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
    Write-Host $Message
}

# Check if running as administrator
function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# Check Docker installation
function Test-Docker {
    try {
        $dockerVersion = docker --version 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "Docker not found"
        }
        Write-Info "Docker version: $dockerVersion"
        
        # Check if Docker is running
        docker info | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Write-Error-Message "Docker daemon is not running!"
            Write-Host "Please start Docker Desktop and try again."
            exit 1
        }
        
        # Check Docker Compose
        $composeVersion = docker compose version 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Info "Docker Compose V2 detected"
            $script:ComposeCmd = "docker compose"
        } else {
            $composeVersion = docker-compose --version 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Info "Docker Compose V1 detected"
                $script:ComposeCmd = "docker-compose"
            } else {
                Write-Error-Message "Docker Compose is not installed!"
                exit 1
            }
        }
        
        return $true
    } catch {
        Write-Error-Message "Docker is not installed!"
        Write-Host "Please install Docker Desktop from: https://desktop.docker.com/win/stable/Docker%20Desktop%20Installer.exe"
        exit 1
    }
}

# Check WSL2
function Test-WSL2 {
    try {
        $wslVersion = wsl --status 2>&1
        if ($wslVersion -match "WSL 2") {
            Write-Info "WSL2 is installed and configured"
            return $true
        } else {
            Write-Warn "WSL2 is not configured. Docker Desktop performance may be limited."
            return $false
        }
    } catch {
        Write-Warn "WSL is not installed. Docker Desktop will use Hyper-V backend."
        return $false
    }
}

# Check system resources
function Test-SystemResources {
    # Check memory
    $totalMemGB = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 2)
    if ($totalMemGB -lt 8) {
        Write-Warn "System has less than 8GB RAM ($totalMemGB GB). Performance may be limited."
    } else {
        Write-Info "System memory: $totalMemGB GB"
    }
    
    # Check disk space
    $drive = Get-PSDrive -Name C
    $freeSpaceGB = [math]::Round($drive.Free / 1GB, 2)
    if ($freeSpaceGB -lt 20) {
        Write-Error-Message "Less than 20GB disk space available ($freeSpaceGB GB)!"
        exit 1
    } else {
        Write-Info "Available disk space: $freeSpaceGB GB"
    }
}

# Setup directories
function Initialize-Directories {
    Write-Info "Setting up directories..."
    
    $script:WorkspacePath = "$env:USERPROFILE\SymbioteWorkspace"
    $script:ConfigPath = "$env:USERPROFILE\.symbiote"
    
    # Create directories
    @(
        $WorkspacePath,
        $ConfigPath,
        "$ConfigPath\prometheus",
        "$ConfigPath\grafana\provisioning",
        "$ConfigPath\mem0"
    ) | ForEach-Object {
        if (!(Test-Path $_)) {
            New-Item -ItemType Directory -Path $_ -Force | Out-Null
        }
    }
    
    Write-Info "Workspace: $WorkspacePath"
    Write-Info "Config: $ConfigPath"
    
    # Set environment variables
    $env:WORKSPACE_PATH = $WorkspacePath
    $env:CONFIG_PATH = $ConfigPath
}

# Detect local LLM
function Test-LocalLLM {
    Write-Info "Checking for local LLM..."
    
    # Check Ollama
    try {
        $ollamaList = ollama list 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Info "Ollama detected"
            $script:UseOllama = $false
            $script:LocalLLMType = "ollama"
            $script:LocalLLMEndpoint = "http://host.docker.internal:11434"
            return
        }
    } catch {}
    
    # Check LM Studio
    try {
        $response = Invoke-RestMethod -Uri "http://localhost:1234/v1/models" -TimeoutSec 2
        Write-Info "LM Studio detected"
        $script:UseOllama = $false
        $script:LocalLLMType = "lm-studio"
        $script:LocalLLMEndpoint = "http://host.docker.internal:1234"
        return
    } catch {}
    
    Write-Warn "No local LLM detected. Ollama will be included in the stack."
    $script:UseOllama = $true
}

# Start services
function Start-Services {
    Write-Info "Starting SymbioteIDE services..."
    
    # Change to project directory
    $projectDir = Split-Path -Parent $PSScriptRoot
    Push-Location $projectDir
    
    try {
        # Determine compose files
        $composeFiles = @("-f", "docker/docker-compose.yml", "-f", "docker/docker-compose.windows.yml")
        
        # Define services
        $services = @(
            "symbiote-ide",
            "orchestrator",
            "agent-runtime",
            "neo4j",
            "qdrant",
            "redis",
            "mem0",
            "rabbitmq",
            "langfuse",
            "postgres",
            "prometheus",
            "grafana"
        )
        
        if ($UseOllama) {
            $services += "ollama"
        }
        
        # Pull images
        Write-Info "Pulling Docker images..."
        $pullArgs = $composeFiles + @("pull") + $services
        & $ComposeCmd $pullArgs
        
        # Start services
        Write-Info "Starting services..."
        $upArgs = $composeFiles + @("up", "-d") + $services
        & $ComposeCmd $upArgs
        
        # Wait for services
        Write-Info "Waiting for services to be healthy..."
        Start-Sleep -Seconds 10
        
    } finally {
        Pop-Location
    }
}

# Check service health
function Test-ServiceHealth {
    Write-Info "Checking service health..."
    
    $services = @(
        @{Name="SymbioteIDE"; URL="http://localhost:8080/health"},
        @{Name="Orchestrator"; URL="http://localhost:9000/health"},
        @{Name="Neo4j"; URL="http://localhost:7474"},
        @{Name="Qdrant"; URL="http://localhost:6333/health"},
        @{Name="Mem0"; URL="http://localhost:8000/health"},
        @{Name="Langfuse"; URL="http://localhost:3030/api/public/health"},
        @{Name="Prometheus"; URL="http://localhost:9090/-/healthy"},
        @{Name="Grafana"; URL="http://localhost:3031/api/health"}
    )
    
    $allHealthy = $true
    
    foreach ($service in $services) {
        try {
            $response = Invoke-WebRequest -Uri $service.URL -TimeoutSec 5 -UseBasicParsing
            Write-Host "  ✓ " -ForegroundColor Green -NoNewline
            Write-Host $service.Name
        } catch {
            Write-Host "  ✗ " -ForegroundColor Red -NoNewline
            Write-Host $service.Name
            $allHealthy = $false
        }
    }
    
    if ($allHealthy) {
        Write-Info "All services are healthy!"
    } else {
        Write-Warn "Some services are not healthy yet. They may still be starting..."
    }
}

# Show URLs
function Show-URLs {
    Write-Host ""
    Write-Info "Service URLs:"
    Write-Host "  - SymbioteIDE:       http://localhost:8080"
    Write-Host "  - Neo4j Browser:     http://localhost:7474"
    Write-Host "  - RabbitMQ Mgmt:     http://localhost:15672"
    Write-Host "  - Langfuse:          http://localhost:3030"
    Write-Host "  - Prometheus:        http://localhost:9090"
    Write-Host "  - Grafana:           http://localhost:3031"
    Write-Host ""
    Write-Host "Default credentials:"
    Write-Host "  - Neo4j:     neo4j / symbiote123"
    Write-Host "  - RabbitMQ:  symbiote / symbiote123"
    Write-Host "  - Grafana:   admin / admin"
}

# Main execution
function Main {
    Write-Host "🚀 Starting SymbioteIDE Docker Infrastructure" -ForegroundColor Cyan
    Write-Host ""
    
    # Check if running as admin (recommended but not required)
    if (!(Test-Administrator)) {
        Write-Warn "Not running as administrator. Some features may require elevation."
    }
    
    # Run checks
    Test-Docker
    Test-WSL2
    Test-SystemResources
    Initialize-Directories
    Test-LocalLLM
    Start-Services
    Test-ServiceHealth
    Show-URLs
    
    Write-Host ""
    Write-Info "Setup complete! Opening SymbioteIDE..."
    
    # Open browser
    Start-Process "http://localhost:8080"
}

# Run main
Main