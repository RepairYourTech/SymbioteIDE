#!/usr/bin/env node

/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

console.log('Starting SymbioteIDE development services...\n');

const services = [
  {
    name: 'MCP Mock Server',
    command: 'node',
    args: [path.join(__dirname, 'mocks', 'mcp-server.js')],
    env: { PORT: 7777, DEBUG: 'symbiote:mcp:*' },
    color: '\x1b[36m', // Cyan
  },
  {
    name: 'Memory Service',
    command: 'node',
    args: [path.join(__dirname, 'mocks', 'memory-service.js')],
    env: { PORT: 7778 },
    color: '\x1b[35m', // Magenta
  },
  {
    name: 'Knowledge Graph',
    command: 'node',
    args: [path.join(__dirname, 'mocks', 'knowledge-service.js')],
    env: { PORT: 7779 },
    color: '\x1b[33m', // Yellow
  },
];

const processes = [];
const resetColor = '\x1b[0m';

// Create mocks directory if it doesn't exist
const mocksDir = path.join(__dirname, 'mocks');
if (!fs.existsSync(mocksDir)) {
  fs.mkdirSync(mocksDir, { recursive: true });
  
  // Create mock service files
  createMockService('mcp-server.js', createMCPServerMock());
  createMockService('memory-service.js', createMemoryServiceMock());
  createMockService('knowledge-service.js', createKnowledgeServiceMock());
}

function createMockService(filename, content) {
  fs.writeFileSync(path.join(mocksDir, filename), content);
}

function createMCPServerMock() {
  return `const http = require('http');
const PORT = process.env.PORT || 7777;

const server = http.createServer((req, res) => {
  console.log(\`[MCP] \${req.method} \${req.url}\`);
  
  res.setHeader('Content-Type', 'application/json');
  
  if (req.url === '/tools') {
    res.end(JSON.stringify({
      tools: [
        { id: 'mock-tool-1', name: 'Mock Tool 1', description: 'A mock tool for testing' },
        { id: 'mock-tool-2', name: 'Mock Tool 2', description: 'Another mock tool' }
      ]
    }));
  } else if (req.url === '/status') {
    res.end(JSON.stringify({ status: 'running', version: '1.0.0' }));
  } else {
    res.statusCode = 404;
    res.end(JSON.stringify({ error: 'Not found' }));
  }
});

server.listen(PORT, () => {
  console.log(\`MCP Mock Server running on port \${PORT}\`);
});`;
}

function createMemoryServiceMock() {
  return `const http = require('http');
const PORT = process.env.PORT || 7778;

const memories = new Map();

const server = http.createServer((req, res) => {
  console.log(\`[Memory] \${req.method} \${req.url}\`);
  
  res.setHeader('Content-Type', 'application/json');
  
  if (req.method === 'GET' && req.url === '/memories') {
    res.end(JSON.stringify({ memories: Array.from(memories.values()) }));
  } else if (req.method === 'POST' && req.url === '/memories') {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      const memory = JSON.parse(body);
      memory.id = Date.now().toString();
      memories.set(memory.id, memory);
      res.end(JSON.stringify(memory));
    });
  } else {
    res.statusCode = 404;
    res.end(JSON.stringify({ error: 'Not found' }));
  }
});

server.listen(PORT, () => {
  console.log(\`Memory Service running on port \${PORT}\`);
});`;
}

function createKnowledgeServiceMock() {
  return `const http = require('http');
const PORT = process.env.PORT || 7779;

const server = http.createServer((req, res) => {
  console.log(\`[Knowledge] \${req.method} \${req.url}\`);
  
  res.setHeader('Content-Type', 'application/json');
  
  if (req.url === '/search') {
    res.end(JSON.stringify({
      results: [
        { id: '1', type: 'code', content: 'function example() {}', score: 0.95 },
        { id: '2', type: 'doc', content: 'Example documentation', score: 0.87 }
      ]
    }));
  } else if (req.url === '/graph/stats') {
    res.end(JSON.stringify({
      nodes: 1234,
      relationships: 5678,
      status: 'healthy'
    }));
  } else {
    res.statusCode = 404;
    res.end(JSON.stringify({ error: 'Not found' }));
  }
});

server.listen(PORT, () => {
  console.log(\`Knowledge Service running on port \${PORT}\`);
});`;
}

// Start services
services.forEach((service) => {
  const proc = spawn(service.command, service.args, {
    env: { ...process.env, ...service.env },
    stdio: 'pipe',
  });

  proc.stdout.on('data', (data) => {
    console.log(`${service.color}[${service.name}]${resetColor} ${data.toString().trim()}`);
  });

  proc.stderr.on('data', (data) => {
    console.error(`${service.color}[${service.name}] ERROR:${resetColor} ${data.toString().trim()}`);
  });

  proc.on('exit', (code) => {
    console.log(`${service.color}[${service.name}]${resetColor} Exited with code ${code}`);
  });

  processes.push(proc);
});

// Handle shutdown
process.on('SIGINT', () => {
  console.log('\nShutting down services...');
  processes.forEach((proc) => proc.kill());
  process.exit(0);
});

console.log('All services started. Press Ctrl+C to stop.\n');