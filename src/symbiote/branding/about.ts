/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

export interface AboutDialogInfo {
  productName: string;
  version: string;
  commit: string;
  date: string;
  electronVersion: string;
  nodeVersion: string;
  v8Version: string;
  osVersion: string;
}

export function getAboutDialogContent(info: AboutDialogInfo): string {
  return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            padding: 30px;
            margin: 0;
            background: #0f0f1e;
            color: #e0e0e0;
            text-align: center;
            user-select: none;
        }
        
        .logo {
            width: 120px;
            height: 120px;
            margin: 0 auto 20px;
            background: linear-gradient(135deg, #00D4FF, #7B61FF, #FF006E);
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 60px;
            font-weight: bold;
            color: #0f0f1e;
            box-shadow: 0 4px 20px rgba(123, 97, 255, 0.3);
        }
        
        h1 {
            margin: 0 0 10px;
            font-size: 28px;
            font-weight: 400;
            color: #ffffff;
        }
        
        .tagline {
            margin: 0 0 30px;
            font-size: 16px;
            color: #7B61FF;
            font-weight: 300;
        }
        
        .version-info {
            background: #1a1a2e;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
            border: 1px solid #2a2a3e;
        }
        
        .version-row {
            display: flex;
            justify-content: space-between;
            margin: 8px 0;
            font-size: 14px;
        }
        
        .version-label {
            color: #5a5a7e;
            font-weight: 500;
        }
        
        .version-value {
            color: #00D4FF;
            font-family: 'SF Mono', Monaco, Consolas, monospace;
        }
        
        .features {
            margin: 30px 0;
            text-align: left;
        }
        
        .feature {
            display: flex;
            align-items: center;
            margin: 10px 0;
            font-size: 14px;
        }
        
        .feature-icon {
            width: 20px;
            height: 20px;
            margin-right: 10px;
            border-radius: 4px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 12px;
        }
        
        .feature-icon.ai { background: #00D4FF; color: #0f0f1e; }
        .feature-icon.mcp { background: #7B61FF; color: #ffffff; }
        .feature-icon.memory { background: #FF006E; color: #ffffff; }
        .feature-icon.graph { background: #00FF88; color: #0f0f1e; }
        
        .links {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #2a2a3e;
        }
        
        .link {
            display: inline-block;
            margin: 0 10px;
            color: #7B61FF;
            text-decoration: none;
            font-size: 14px;
            transition: color 0.2s;
        }
        
        .link:hover {
            color: #00D4FF;
        }
        
        .copyright {
            margin-top: 20px;
            font-size: 12px;
            color: #5a5a7e;
        }
    </style>
</head>
<body>
    <div class="logo">S</div>
    <h1>${info.productName}</h1>
    <p class="tagline">AI-Powered Development Environment</p>
    
    <div class="version-info">
        <div class="version-row">
            <span class="version-label">Version</span>
            <span class="version-value">${info.version}</span>
        </div>
        <div class="version-row">
            <span class="version-label">Commit</span>
            <span class="version-value">${info.commit.substring(0, 7)}</span>
        </div>
        <div class="version-row">
            <span class="version-label">Date</span>
            <span class="version-value">${info.date}</span>
        </div>
        <div class="version-row">
            <span class="version-label">Electron</span>
            <span class="version-value">${info.electronVersion}</span>
        </div>
        <div class="version-row">
            <span class="version-label">Node.js</span>
            <span class="version-value">${info.nodeVersion}</span>
        </div>
        <div class="version-row">
            <span class="version-label">V8</span>
            <span class="version-value">${info.v8Version}</span>
        </div>
        <div class="version-row">
            <span class="version-label">OS</span>
            <span class="version-value">${info.osVersion}</span>
        </div>
    </div>
    
    <div class="features">
        <div class="feature">
            <div class="feature-icon ai">AI</div>
            <span>Multi-Model Orchestration Engine</span>
        </div>
        <div class="feature">
            <div class="feature-icon mcp">M</div>
            <span>Model Context Protocol (MCP) Support</span>
        </div>
        <div class="feature">
            <div class="feature-icon memory">M</div>
            <span>Persistent Memory with Mem0</span>
        </div>
        <div class="feature">
            <div class="feature-icon graph">G</div>
            <span>Knowledge Graph Integration</span>
        </div>
    </div>
    
    <div class="links">
        <a href="https://symbiote-ide.com" class="link">Website</a>
        <a href="https://github.com/symbiote-ide/symbiote-ide" class="link">GitHub</a>
        <a href="https://symbiote-ide.com/docs" class="link">Documentation</a>
        <a href="https://github.com/symbiote-ide/symbiote-ide/issues" class="link">Report Issue</a>
    </div>
    
    <p class="copyright">
        © 2024 SymbioteIDE Team. Licensed under MIT License.
    </p>
</body>
</html>
  `;
}

export function showAboutDialog(info: AboutDialogInfo): void {
  // This would be called from the main process to show the about dialog
  // Implementation depends on the Electron integration
  const { BrowserWindow } = require('electron');
  
  const aboutWindow = new BrowserWindow({
    width: 500,
    height: 700,
    resizable: false,
    minimizable: false,
    maximizable: false,
    backgroundColor: '#0f0f1e',
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true
    }
  });
  
  aboutWindow.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(getAboutDialogContent(info))}`);
}