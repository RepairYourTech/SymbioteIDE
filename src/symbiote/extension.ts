/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import * as vscode from 'vscode';
import { AgentBuilderProvider } from './agent-builder/extension/agent-builder-provider';
import { MCPExtension } from './mcp';
import { OrchestrationEngine } from './orchestration';
import { CredentialManager } from './credentials';
import { ChatViewProvider } from './chat/providers/chat-view-provider';
import { CodeIntelligenceManager } from './code-intelligence/neo4j-manager';
import { QdrantSearchManager } from './search/qdrant-manager';
import { TerminalManager } from './terminal/terminal-manager';
import { AgentRegistryManager } from './agent-registry';
import { NotebookProvider } from './notebooks/notebook-provider';
import { ContextSystemManager } from './context-system';
import { ADKAgentFactory } from './agents/adk';
import { DashboardProvider, DashboardDataCollector } from './dashboard';
import { WebSocketManager } from './websocket/websocket-manager';
import { MemoryManager } from './memory/unified-context-api';
import { MCPServerManager } from './mcp/mcp-server-manager';
import { ADKClient } from './adk/adk-client';
import { AuthManager, SymbioteAuthenticationProvider, initializeSupabase, AuthGuard } from './auth';

export async function activate(context: vscode.ExtensionContext) {
    console.log('SymbioteIDE is now active!');

    // Initialize Supabase client with context for SecretStorage
    initializeSupabase(context);

    // Initialize authentication
    const authManager = new AuthManager();
    const isAuthenticated = await authManager.initialize();
    
    // Set auth manager in AuthGuard
    AuthGuard.setAuthManager(authManager);
    
    // Register VS Code authentication provider
    SymbioteAuthenticationProvider.register(context, authManager);

    // Add auth manager to disposables early
    context.subscriptions.push(authManager);

    // Register auth command (always available)
    context.subscriptions.push(
        vscode.commands.registerCommand('symbiote.showAuthMenu', () => {
            authManager.showAuthMenu();
        })
    );

    // If not authenticated, require login before proceeding
    if (!isAuthenticated) {
        const loggedIn = await authManager.requireAuth();
        if (!loggedIn) {
            vscode.window.showErrorMessage('SymbioteIDE requires authentication. Extension activation cancelled.');
            return {
                authManager,
                isAuthenticated: false
            };
        }
    }

    // User is authenticated - initialize all services
    vscode.window.showInformationMessage(`Welcome to SymbioteIDE, ${authManager.getUser()?.email}!`);

    // Initialize core services
    const credentialManager = new CredentialManager(context);
    const orchestrationEngine = new OrchestrationEngine();
    const mcpExtension = new MCPExtension(context);
    const codeIntelligence = new CodeIntelligenceManager(context);
    const searchManager = new QdrantSearchManager();
    const terminalManager = new TerminalManager(context);
    const agentRegistry = new AgentRegistryManager();
    const contextSystem = new ContextSystemManager(context);
    const adkFactory = new ADKAgentFactory();
    const wsManager = new WebSocketManager();
    const memoryManager = new MemoryManager();
    const mcpServerManager = new MCPServerManager(context);
    const adkClient = new ADKClient();

    // Initialize services
    await credentialManager.initialize();
    await orchestrationEngine.initialize();
    await mcpExtension.activate();
    await codeIntelligence.initialize();
    await searchManager.initialize();
    await contextSystem.initialize();
    await wsManager.initialize();
    await memoryManager.initialize();
    await mcpServerManager.initialize();
    await adkClient.connect();

    // Register ADK with agent registry
    agentRegistry.registerFactory('adk', adkFactory);

    // Register Visual Agent Builder
    context.subscriptions.push(
        AgentBuilderProvider.register(context)
    );

    // Register chat view
    const chatProvider = new ChatViewProvider(context, orchestrationEngine);
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(
            ChatViewProvider.viewType,
            chatProvider
        )
    );

    // Register notebook provider
    const notebookProvider = new NotebookProvider(context, orchestrationEngine);
    context.subscriptions.push(
        vscode.notebook.registerNotebookCellStatusBarItemProvider(
            'symbiote-notebook',
            notebookProvider
        )
    );

    // Initialize dashboard data collector
    const dashboardDataCollector = new DashboardDataCollector(
        orchestrationEngine,
        mcpServerManager,
        memoryManager,
        adkClient,
        wsManager
    );
    await dashboardDataCollector.initialize();

    // Register dashboard provider
    const dashboardProvider = new DashboardProvider(context, dashboardDataCollector, wsManager);
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(
            DashboardProvider.viewType,
            dashboardProvider
        )
    );

    // Register commands (auth command already registered above)
    context.subscriptions.push(
        vscode.commands.registerCommand('symbiote.showChat', () => {
            if (!authManager.isAuthenticated()) {
                authManager.requireAuth();
                return;
            }
            vscode.commands.executeCommand('workbench.view.extension.symbiote-chat-view');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('symbiote.newAgentWorkflow', async () => {
            if (!authManager.isAuthenticated()) {
                authManager.requireAuth();
                return;
            }
            
            const workflowName = await vscode.window.showInputBox({
                prompt: 'Enter workflow name',
                placeHolder: 'my-workflow'
            });
            
            if (workflowName) {
                const uri = vscode.Uri.file(`${workflowName}.aflow`);
                const doc = await vscode.workspace.openTextDocument(uri.with({ scheme: 'untitled' }));
                await vscode.window.showTextDocument(doc);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('symbiote.configureCredentials', () => {
            if (!authManager.isAuthenticated()) {
                authManager.requireAuth();
                return;
            }
            credentialManager.openConfiguration();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('symbiote.searchCodebase', () => {
            if (!authManager.isAuthenticated()) {
                authManager.requireAuth();
                return;
            }
            searchManager.searchInterface();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('symbiote.openTerminal', () => {
            if (!authManager.isAuthenticated()) {
                authManager.requireAuth();
                return;
            }
            terminalManager.createAITerminal();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('symbiote.showContextView', () => {
            if (!authManager.isAuthenticated()) {
                authManager.requireAuth();
                return;
            }
            contextSystem.showContextView();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('symbiote.showDashboard', () => {
            if (!authManager.isAuthenticated()) {
                authManager.requireAuth();
                return;
            }
            vscode.commands.executeCommand('workbench.view.extension.symbiote-dashboard-view');
        })
    );

    // Set up status bar
    const statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100
    );
    statusBarItem.text = '$(zap) SymbioteIDE';
    statusBarItem.tooltip = 'SymbioteIDE - AI-Powered Development';
    statusBarItem.command = 'symbiote.showChat';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    // Set up activity bar icon
    vscode.commands.executeCommand('setContext', 'symbiote.activated', true);

    // Handle configuration changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(e => {
            if (e.affectsConfiguration('symbiote')) {
                orchestrationEngine.reloadConfiguration();
                mcpExtension.reloadServers();
            }
        })
    );

    return {
        authManager,
        orchestrationEngine,
        credentialManager,
        mcpExtension,
        codeIntelligence,
        searchManager,
        terminalManager,
        agentRegistry,
        contextSystem,
        adkFactory,
        wsManager,
        memoryManager,
        mcpServerManager,
        adkClient,
        dashboardDataCollector,
        dashboardProvider
    };
}

export function deactivate() {
    console.log('SymbioteIDE is now deactivated');
}