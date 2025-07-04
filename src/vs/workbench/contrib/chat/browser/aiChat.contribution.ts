/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { localize } from '../../../../nls.js';
import { registerAction2, Action2 } from '../../../../platform/actions/common/actions.js';
import { ServicesAccessor } from '../../../../platform/instantiation/common/instantiation.js';
import { registerSingleton } from '../../../../platform/instantiation/common/extensions.js';
import { IAIChatService, AIChatService, IAICodeAction } from './aiChatService.js';
import { IQuickInputService } from '../../../../platform/quickinput/common/quickInput.js';
import { IEditorService } from '../../../services/editor/common/editorService.js';
import { INotificationService } from '../../../../platform/notification/common/notification.js';
import { Categories } from '../../../../platform/action/common/actionCommonCategories.js';
import { IConfigurationRegistry, Extensions as ConfigurationExtensions } from '../../../../platform/configuration/common/configurationRegistry.js';
import { Registry } from '../../../../platform/registry/common/platform.js';
import { ContextKeyExpr } from '../../../../platform/contextkey/common/contextkey.js';
import { CONTEXT_CHAT_FOCUSED } from '../common/chatContextKeys.js';
import { IAgentService } from '../../../services/adk/common/adk.js';
import { IViewsService } from '../../../services/views/common/viewsService.js';
import { CHAT_VIEW_ID } from '../common/chat.js';

// Register AI Chat Service
registerSingleton(IAIChatService, AIChatService, true);

// Register Actions
class EnhancedChatAction extends Action2 {
	constructor() {
		super({
			id: 'chat.action.enhancedMessage',
			title: localize('enhancedChat', 'Send Enhanced AI Message'),
			category: Categories.AI,
			f1: true,
			precondition: CONTEXT_CHAT_FOCUSED
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const aiChatService = accessor.get(IAIChatService);
		const quickInputService = accessor.get(IQuickInputService);
		const notificationService = accessor.get(INotificationService);

		const message = await quickInputService.input({
			placeHolder: localize('enterMessage', 'Enter your message'),
			prompt: localize('aiChatPrompt', 'Ask anything about your code or project')
		});

		if (!message) {
			return;
		}

		const progress = notificationService.withProgress({
			location: 15,
			title: localize('processingAI', 'Processing AI request...')
		}, async () => {
			const response = await aiChatService.sendEnhancedMessage(message);
			return response;
		});

		await progress;
	}
}

class ExecuteCodeActionAction extends Action2 {
	constructor() {
		super({
			id: 'chat.action.executeCodeAction',
			title: localize('executeCodeAction', 'Execute Code Action from Chat'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor, action?: IAICodeAction): Promise<void> {
		const aiChatService = accessor.get(IAIChatService);
		const quickInputService = accessor.get(IQuickInputService);
		const notificationService = accessor.get(INotificationService);

		if (!action) {
			// Show picker if no action provided
			notificationService.info(localize('noRecentActions', 'No recent code actions available'));
			return;
		}

		try {
			await aiChatService.executeCodeAction(action);
			notificationService.info(localize('actionExecuted', 'Code action executed: {0}', action.title));
		} catch (error) {
			notificationService.error(localize('actionFailed', 'Failed to execute action: {0}', error.message));
		}
	}
}

class ShowChatContextAction extends Action2 {
	constructor() {
		super({
			id: 'chat.action.showContext',
			title: localize('showChatContext', 'Show AI Chat Context'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const aiChatService = accessor.get(IAIChatService);
		const editorService = accessor.get(IEditorService);

		const context = aiChatService.getCurrentContext();
		const content = JSON.stringify(context, null, 2);

		await editorService.openEditor({
			resource: undefined,
			contents: content,
			options: {
				pinned: true,
				description: localize('aiChatContext', 'AI Chat Context')
			}
		});
	}
}

class SearchCodebaseAction extends Action2 {
	constructor() {
		super({
			id: 'chat.action.searchCodebase',
			title: localize('searchCodebase', 'Search Codebase with AI'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const aiChatService = accessor.get(IAIChatService);
		const quickInputService = accessor.get(IQuickInputService);
		const editorService = accessor.get(IEditorService);
		const notificationService = accessor.get(INotificationService);

		const query = await quickInputService.input({
			placeHolder: localize('searchQuery', 'Enter search query'),
			prompt: localize('searchPrompt', 'What are you looking for in the codebase?')
		});

		if (!query) {
			return;
		}

		const progress = notificationService.withProgress({
			location: 15,
			title: localize('searching', 'Searching codebase...')
		}, async () => {
			const results = await aiChatService.searchCodebase(query);
			return results;
		});

		const results = await progress;
		
		if (results.length === 0) {
			notificationService.info(localize('noResults', 'No results found'));
			return;
		}

		// Show results
		const picks = results.map(r => ({
			label: r.uri.path,
			description: `Relevance: ${Math.round(r.relevance * 100)}%`,
			detail: r.content.substring(0, 100) + '...'
		}));

		const selected = await quickInputService.pick(picks, {
			placeHolder: localize('selectResult', 'Select a result to open')
		});

		if (selected) {
			const result = results.find(r => r.uri.path === selected.label);
			if (result) {
				await editorService.openEditor({ resource: result.uri });
			}
		}
	}
}

class RecommendAgentsAction extends Action2 {
	constructor() {
		super({
			id: 'chat.action.recommendAgents',
			title: localize('recommendAgents', 'Recommend AI Agents for Task'),
			category: Categories.AI,
			f1: true,
			precondition: CONTEXT_CHAT_FOCUSED
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const aiChatService = accessor.get(IAIChatService);
		const agentService = accessor.get(IAgentService);
		const quickInputService = accessor.get(IQuickInputService);
		const notificationService = accessor.get(INotificationService);

		const task = await quickInputService.input({
			placeHolder: localize('describeTask', 'Describe your task'),
			prompt: localize('taskPrompt', 'What do you want to accomplish?')
		});

		if (!task) {
			return;
		}

		const recommendations = await aiChatService.recommendAgents(task);
		
		if (recommendations.length === 0) {
			notificationService.info(localize('noAgentRecommendations', 'No agent recommendations for this task'));
			return;
		}

		const picks = recommendations.map(agent => ({
			id: agent.id,
			label: agent.name,
			description: agent.capabilities.map(c => c.name).join(', '),
			detail: agent.description
		}));

		const selected = await quickInputService.pick(picks, {
			placeHolder: localize('selectAgent', 'Select an agent to create'),
			canPickMany: false
		});

		if (selected && selected.id) {
			const agent = recommendations.find(a => a.id === selected.id);
			if (agent) {
				try {
					const instance = await agentService.createAgent(agent);
					notificationService.info(localize('agentCreated', 'Created agent: {0}', agent.name));
					
					// Run the task with the agent
					await agentService.executeTask({
						id: `task_${Date.now()}`,
						agentId: instance.id,
						description: task,
						input: {}
					});
				} catch (error) {
					notificationService.error(localize('agentCreationFailed', 'Failed to create agent: {0}', error.message));
				}
			}
		}
	}
}

class ToggleContextEnrichmentAction extends Action2 {
	constructor() {
		super({
			id: 'chat.action.toggleContextEnrichment',
			title: localize('toggleContextEnrichment', 'Toggle AI Context Enrichment'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const aiChatService = accessor.get(IAIChatService);
		const notificationService = accessor.get(INotificationService);
		
		// Toggle enrichment
		const currentState = true; // Would get from service
		aiChatService.setContextEnrichment(!currentState);
		
		notificationService.info(localize('contextEnrichment', 
			'Context enrichment {0}', 
			!currentState ? localize('enabled', 'enabled') : localize('disabled', 'disabled')
		));
	}
}

// Register all actions
registerAction2(EnhancedChatAction);
registerAction2(ExecuteCodeActionAction);
registerAction2(ShowChatContextAction);
registerAction2(SearchCodebaseAction);
registerAction2(RecommendAgentsAction);
registerAction2(ToggleContextEnrichmentAction);

// Register Configuration
Registry.as<IConfigurationRegistry>(ConfigurationExtensions.Configuration).registerConfiguration({
	id: 'aiChat',
	title: localize('aiChatConfigurationTitle', 'AI Chat'),
	type: 'object',
	properties: {
		'aiChat.contextEnrichment.enabled': {
			type: 'boolean',
			default: true,
			description: localize('aiChat.contextEnrichment.enabled', 'Enable automatic context enrichment for AI chat.')
		},
		'aiChat.contextEnrichment.includeOpenFiles': {
			type: 'boolean',
			default: true,
			description: localize('aiChat.contextEnrichment.includeOpenFiles', 'Include open files in chat context.')
		},
		'aiChat.contextEnrichment.includeDebugState': {
			type: 'boolean',
			default: true,
			description: localize('aiChat.contextEnrichment.includeDebugState', 'Include debug state in chat context when debugging.')
		},
		'aiChat.contextEnrichment.includeTerminalOutput': {
			type: 'boolean',
			default: false,
			description: localize('aiChat.contextEnrichment.includeTerminalOutput', 'Include recent terminal output in chat context.')
		},
		'aiChat.codeActions.autoExecute': {
			type: 'boolean',
			default: false,
			description: localize('aiChat.codeActions.autoExecute', 'Automatically execute safe code actions.')
		},
		'aiChat.codeActions.showPreview': {
			type: 'boolean',
			default: true,
			description: localize('aiChat.codeActions.showPreview', 'Show preview before executing code actions.')
		},
		'aiChat.agents.autoRecommend': {
			type: 'boolean',
			default: true,
			description: localize('aiChat.agents.autoRecommend', 'Automatically recommend agents based on chat content.')
		},
		'aiChat.search.maxResults': {
			type: 'number',
			default: 20,
			minimum: 5,
			maximum: 100,
			description: localize('aiChat.search.maxResults', 'Maximum number of search results to return.')
		},
		'aiChat.search.includeSymbols': {
			type: 'boolean',
			default: true,
			description: localize('aiChat.search.includeSymbols', 'Include code symbols in search results.')
		}
	}
});