/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Emitter, Event } from '../../../../base/common/event.js';
import { Disposable, IDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { URI } from '../../../../base/common/uri.js';
import { ILogService } from '../../../../platform/log/common/log.js';
import { IStorageService, StorageScope, StorageTarget } from '../../../../platform/storage/common/storage.js';
import { IOrchestrationService } from '../../../../platform/orchestration/common/orchestration.js';
import { IAgentService, IAgentDefinition } from '../../../services/adk/common/adk.js';
import { IAIDebugService } from '../../debug/browser/aiDebugService.js';
import { IEditorService } from '../../../services/editor/common/editorService.js';
import { IWorkspaceContextService } from '../../../../platform/workspace/common/workspace.js';
import { createDecorator } from '../../../../platform/instantiation/common/instantiation.js';
import { ChatService } from '../common/chatServiceImpl.js';
import { IChatService, IChatModel, IChatResponseModel } from '../common/chatService.js';
import { IChatRequest, IChatResponse } from '../common/chatModel.js';
import { ILanguageModelChatMetadata } from '../common/languageModels.js';

export const IAIChatService = createDecorator<IAIChatService>('aiChatService');

export interface IAIChatContext {
	workspaceUri?: URI;
	activeFileUri?: URI;
	selectedText?: string;
	terminalOutput?: string;
	debugState?: any;
	openFiles?: URI[];
	recentErrors?: string[];
}

export interface IAICodeAction {
	id: string;
	title: string;
	description?: string;
	kind: 'create' | 'edit' | 'delete' | 'refactor' | 'debug' | 'test';
	uri?: URI;
	content?: string;
	range?: { startLine: number; endLine: number };
	preview?: string;
}

export interface IAIChatEnhancement {
	context?: IAIChatContext;
	codeActions?: IAICodeAction[];
	suggestedAgents?: IAgentDefinition[];
	debugInsights?: any[];
	searchResults?: Array<{ uri: URI; content: string; relevance: number }>;
}

export interface IAIChatService extends IChatService {
	readonly _serviceBrand: undefined;

	/**
	 * Event fired when code actions are generated
	 */
	readonly onDidGenerateCodeActions: Event<IAICodeAction[]>;

	/**
	 * Event fired when context is updated
	 */
	readonly onDidUpdateContext: Event<IAIChatContext>;

	/**
	 * Send an enhanced AI message with context
	 */
	sendEnhancedMessage(message: string, context?: IAIChatContext): Promise<IChatResponseModel>;

	/**
	 * Generate code actions from chat response
	 */
	generateCodeActions(response: string, context?: IAIChatContext): Promise<IAICodeAction[]>;

	/**
	 * Execute a code action
	 */
	executeCodeAction(action: IAICodeAction): Promise<void>;

	/**
	 * Get current context
	 */
	getCurrentContext(): IAIChatContext;

	/**
	 * Update context
	 */
	updateContext(context: Partial<IAIChatContext>): void;

	/**
	 * Search codebase for relevant context
	 */
	searchCodebase(query: string): Promise<Array<{ uri: URI; content: string; relevance: number }>>;

	/**
	 * Get agent recommendations for current task
	 */
	recommendAgents(message: string): Promise<IAgentDefinition[]>;

	/**
	 * Enable/disable context enrichment
	 */
	setContextEnrichment(enabled: boolean): void;
}

const CONTEXT_STORAGE_KEY = 'symbiote.chat.ai.context';
const HISTORY_STORAGE_KEY = 'symbiote.chat.ai.history';

export class AIChatService extends ChatService implements IAIChatService {
	declare readonly _serviceBrand: undefined;

	private readonly _onDidGenerateCodeActions = this._register(new Emitter<IAICodeAction[]>());
	readonly onDidGenerateCodeActions: Event<IAICodeAction[]> = this._onDidGenerateCodeActions.event;

	private readonly _onDidUpdateContext = this._register(new Emitter<IAIChatContext>());
	readonly onDidUpdateContext: Event<IAIChatContext> = this._onDidUpdateContext.event;

	private currentContext: IAIChatContext = {};
	private contextEnrichmentEnabled = true;
	private codeActionCounter = 0;

	constructor(
		@IOrchestrationService private readonly orchestrationService: IOrchestrationService,
		@IAgentService private readonly agentService: IAgentService,
		@IAIDebugService private readonly aiDebugService: IAIDebugService,
		@IEditorService private readonly editorService: IEditorService,
		@IWorkspaceContextService private readonly workspaceService: IWorkspaceContextService,
		@IStorageService private readonly storageService: IStorageService,
		@ILogService logService: ILogService
	) {
		super(logService);
		this.loadContext();
		this.registerContextProviders();
	}

	private registerContextProviders(): void {
		// Update context on editor changes
		this._register(this.editorService.onDidActiveEditorChange(() => {
			if (this.contextEnrichmentEnabled) {
				this.updateEditorContext();
			}
		}));

		// Update context on workspace changes
		this._register(this.workspaceService.onDidChangeWorkspaceFolders(() => {
			this.updateWorkspaceContext();
		}));
	}

	async sendEnhancedMessage(message: string, context?: IAIChatContext): Promise<IChatResponseModel> {
		// Enrich context if enabled
		const enrichedContext = this.contextEnrichmentEnabled ? 
			await this.enrichContext(context || this.currentContext) : 
			context || this.currentContext;

		// Build enhanced prompt with context
		const enhancedPrompt = this.buildEnhancedPrompt(message, enrichedContext);

		// Use orchestration service to get response
		const response = await this.orchestrationService.sendRequest({
			prompt: enhancedPrompt,
			capabilities: ['chat', 'code-generation', 'explanation']
		});

		// Process response and generate code actions
		const codeActions = await this.generateCodeActions(response.response.content, enrichedContext);
		if (codeActions.length > 0) {
			this._onDidGenerateCodeActions.fire(codeActions);
		}

		// Create chat response model
		// This would integrate with the actual chat model
		return {
			content: response.response.content,
			metadata: {
				modelId: response.modelId,
				duration: response.duration,
				cost: response.cost
			}
		} as any;
	}

	async generateCodeActions(response: string, context?: IAIChatContext): Promise<IAICodeAction[]> {
		const actions: IAICodeAction[] = [];

		// Use AI to extract code actions from response
		try {
			const extractionResponse = await this.orchestrationService.sendRequest({
				prompt: this.buildCodeActionExtractionPrompt(response),
				capabilities: ['code-analysis']
			});

			const extracted = this.parseCodeActions(extractionResponse.response.content);
			actions.push(...extracted);

		} catch (error) {
			this.logService.error('[AIChatService] Failed to extract code actions:', error);
		}

		return actions;
	}

	async executeCodeAction(action: IAICodeAction): Promise<void> {
		switch (action.kind) {
			case 'create':
				if (action.uri && action.content) {
					await this.editorService.openEditor({
						resource: action.uri,
						contents: action.content,
						options: { pinned: true }
					});
				}
				break;

			case 'edit':
				if (action.uri) {
					const editor = await this.editorService.openEditor({ resource: action.uri });
					// Apply edits
					if (action.content && action.range && editor) {
						// Would apply the edit to the model
					}
				}
				break;

			case 'refactor':
			case 'debug':
			case 'test':
				// Handle other action types
				break;

			case 'delete':
				// Handle delete
				break;
		}

		this.logService.info(`[AIChatService] Executed code action: ${action.title}`);
	}

	getCurrentContext(): IAIChatContext {
		return { ...this.currentContext };
	}

	updateContext(context: Partial<IAIChatContext>): void {
		this.currentContext = { ...this.currentContext, ...context };
		this._onDidUpdateContext.fire(this.currentContext);
		this.saveContext();
	}

	async searchCodebase(query: string): Promise<Array<{ uri: URI; content: string; relevance: number }>> {
		// This would integrate with search service
		// For now, return mock results
		return [];
	}

	async recommendAgents(message: string): Promise<IAgentDefinition[]> {
		// Analyze message to determine required capabilities
		const analysis = await this.orchestrationService.sendRequest({
			prompt: `Analyze this request and suggest AI agent capabilities needed: "${message}"`,
			capabilities: ['analysis']
		});

		// Get matching agents
		const capabilities = this.extractCapabilities(analysis.response.content);
		const recommendations: IAgentDefinition[] = [];

		for (const capability of capabilities) {
			const agents = this.agentService.getAgentDefinitions()
				.filter(agent => agent.capabilities.some(c => c.id === capability));
			recommendations.push(...agents);
		}

		return recommendations.slice(0, 3); // Top 3 recommendations
	}

	setContextEnrichment(enabled: boolean): void {
		this.contextEnrichmentEnabled = enabled;
		if (enabled) {
			this.updateEditorContext();
			this.updateWorkspaceContext();
		}
	}

	private async enrichContext(context: IAIChatContext): Promise<IAIChatContext> {
		const enriched = { ...context };

		// Add open files
		const openEditors = this.editorService.editors;
		enriched.openFiles = openEditors
			.map(e => e.resource)
			.filter(uri => uri !== undefined) as URI[];

		// Add debug insights if debugging
		try {
			const debugInsights = await this.aiDebugService.analyzeDebugState();
			if (debugInsights.length > 0) {
				enriched.debugState = debugInsights;
			}
		} catch {
			// Ignore if not debugging
		}

		return enriched;
	}

	private buildEnhancedPrompt(message: string, context: IAIChatContext): string {
		let prompt = message;

		if (context.workspaceUri) {
			prompt = `Working in: ${context.workspaceUri.path}\n\n${prompt}`;
		}

		if (context.activeFileUri) {
			prompt = `Active file: ${context.activeFileUri.path}\n\n${prompt}`;
		}

		if (context.selectedText) {
			prompt = `Selected code:\n\`\`\`\n${context.selectedText}\n\`\`\`\n\n${prompt}`;
		}

		if (context.recentErrors && context.recentErrors.length > 0) {
			prompt = `Recent errors:\n${context.recentErrors.join('\n')}\n\n${prompt}`;
		}

		return prompt;
	}

	private buildCodeActionExtractionPrompt(response: string): string {
		return `
Extract code actions from this response. Return a JSON array of actions:

Response:
${response}

Format:
[{
  "title": "action title",
  "description": "what it does",
  "kind": "create|edit|delete|refactor|debug|test",
  "uri": "file path if applicable",
  "content": "code content if applicable",
  "range": { "startLine": number, "endLine": number } // if editing
}]
		`.trim();
	}

	private parseCodeActions(content: string): IAICodeAction[] {
		try {
			const json = content.match(/\[[\s\S]*\]/)?.[0];
			if (!json) return [];

			const parsed = JSON.parse(json);
			return parsed.map((action: any) => ({
				id: `action_${++this.codeActionCounter}`,
				title: action.title || 'Untitled Action',
				description: action.description,
				kind: action.kind || 'edit',
				uri: action.uri ? URI.file(action.uri) : undefined,
				content: action.content,
				range: action.range
			}));
		} catch {
			return [];
		}
	}

	private extractCapabilities(content: string): string[] {
		// Simple extraction - in production would use NLP
		const capabilities: string[] = [];
		
		if (content.includes('code') || content.includes('programming')) {
			capabilities.push('code-generation');
		}
		if (content.includes('debug') || content.includes('error')) {
			capabilities.push('debugging');
		}
		if (content.includes('test')) {
			capabilities.push('unit-tests');
		}
		if (content.includes('refactor')) {
			capabilities.push('code-refactoring');
		}

		return capabilities;
	}

	private updateEditorContext(): void {
		const activeEditor = this.editorService.activeEditor;
		const update: Partial<IAIChatContext> = {};

		if (activeEditor?.resource) {
			update.activeFileUri = activeEditor.resource;
		}

		// Get selected text from active editor
		// This would need proper editor integration

		this.updateContext(update);
	}

	private updateWorkspaceContext(): void {
		const workspace = this.workspaceService.getWorkspace();
		if (workspace.folders.length > 0) {
			this.updateContext({
				workspaceUri: workspace.folders[0].uri
			});
		}
	}

	private loadContext(): void {
		const stored = this.storageService.get(CONTEXT_STORAGE_KEY, StorageScope.WORKSPACE);
		if (stored) {
			try {
				const context = JSON.parse(stored);
				// Convert URIs back
				if (context.workspaceUri) {
					context.workspaceUri = URI.parse(context.workspaceUri);
				}
				if (context.activeFileUri) {
					context.activeFileUri = URI.parse(context.activeFileUri);
				}
				this.currentContext = context;
			} catch (e) {
				this.logService.error('[AIChatService] Failed to load context:', e);
			}
		}
	}

	private saveContext(): void {
		const toStore = { ...this.currentContext };
		// Convert URIs to strings
		if (toStore.workspaceUri) {
			(toStore as any).workspaceUri = toStore.workspaceUri.toString();
		}
		if (toStore.activeFileUri) {
			(toStore as any).activeFileUri = toStore.activeFileUri.toString();
		}
		
		this.storageService.store(
			CONTEXT_STORAGE_KEY, 
			JSON.stringify(toStore), 
			StorageScope.WORKSPACE, 
			StorageTarget.MACHINE
		);
	}
}