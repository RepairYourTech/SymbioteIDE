/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { registerAction2, Action2 } from '../../../platform/actions/common/actions.js';
import { ServicesAccessor } from '../../../platform/instantiation/common/instantiation.js';
import { IQuickInputService, IQuickPickItem } from '../../../platform/quickinput/common/quickInput.js';
import { IAgentService, IAgentRegistryService, IAgentDefinition } from '../../../services/adk/common/adk.js';
import { localize } from '../../../nls.js';
import { Categories } from '../../../platform/action/common/actionCommonCategories.js';
import { INotificationService } from '../../../platform/notification/common/notification.js';
import { IEditorService } from '../../../services/editor/common/editorService.js';
import { IViewsService } from '../../../services/views/common/viewsService.js';

// Register ADK actions
class CreateAgentAction extends Action2 {
	constructor() {
		super({
			id: 'adk.createAgent',
			title: localize('createAgent', 'Create AI Agent'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const quickInputService = accessor.get(IQuickInputService);
		const agentService = accessor.get(IAgentService);
		const agentRegistry = accessor.get(IAgentRegistryService);
		const notificationService = accessor.get(INotificationService);

		const definitions = agentRegistry.getAgentDefinitions();
		if (definitions.length === 0) {
			notificationService.warn(localize('noAgentDefinitions', 'No agent definitions available.'));
			return;
		}

		const picks: IQuickPickItem[] = definitions.map(def => ({
			id: def.id,
			label: def.name,
			description: def.version,
			detail: def.description
		}));

		const selected = await quickInputService.pick(picks, {
			placeHolder: localize('selectAgentType', 'Select an agent type to create')
		});

		if (selected && selected.id) {
			const definition = agentRegistry.getAgentDefinition(selected.id);
			if (definition) {
				try {
					const agent = await agentService.createAgent(definition);
					notificationService.info(localize('agentCreated', 'Created agent: {0} ({1})', agent.definition.name, agent.id));
				} catch (error) {
					notificationService.error(localize('agentCreationFailed', 'Failed to create agent: {0}', error.message));
				}
			}
		}
	}
}

class ManageAgentsAction extends Action2 {
	constructor() {
		super({
			id: 'adk.manageAgents',
			title: localize('manageAgents', 'Manage AI Agents'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const quickInputService = accessor.get(IQuickInputService);
		const agentService = accessor.get(IAgentService);
		const notificationService = accessor.get(INotificationService);

		const agents = agentService.getActiveAgents();
		if (agents.length === 0) {
			notificationService.info(localize('noActiveAgents', 'No active agents.'));
			return;
		}

		const picks = agents.map(agent => ({
			id: agent.id,
			label: agent.definition.name,
			description: `${agent.state} - Created: ${agent.createdAt.toLocaleString()}`,
			detail: agent.definition.description,
			buttons: [{
				iconClass: 'codicon-stop',
				tooltip: localize('terminateAgent', 'Terminate Agent')
			}]
		}));

		const selected = await quickInputService.pick(picks, {
			placeHolder: localize('selectAgent', 'Select an agent to manage'),
			canPickMany: false
		});

		if (selected && selected.id) {
			const actions = [
				{ id: 'suspend', label: localize('suspendAgent', 'Suspend Agent') },
				{ id: 'resume', label: localize('resumeAgent', 'Resume Agent') },
				{ id: 'terminate', label: localize('terminateAgent', 'Terminate Agent') }
			];

			const action = await quickInputService.pick(actions, {
				placeHolder: localize('selectAction', 'Select an action')
			});

			if (action) {
				try {
					switch (action.id) {
						case 'suspend':
							await agentService.suspendAgent(selected.id);
							notificationService.info(localize('agentSuspended', 'Agent suspended'));
							break;
						case 'resume':
							await agentService.resumeAgent(selected.id);
							notificationService.info(localize('agentResumed', 'Agent resumed'));
							break;
						case 'terminate':
							await agentService.terminateAgent(selected.id);
							notificationService.info(localize('agentTerminated', 'Agent terminated'));
							break;
					}
				} catch (error) {
					notificationService.error(localize('actionFailed', 'Action failed: {0}', error.message));
				}
			}
		}
	}
}

class RunAgentTaskAction extends Action2 {
	constructor() {
		super({
			id: 'adk.runAgentTask',
			title: localize('runAgentTask', 'Run Agent Task'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const quickInputService = accessor.get(IQuickInputService);
		const agentService = accessor.get(IAgentService);
		const notificationService = accessor.get(INotificationService);
		const editorService = accessor.get(IEditorService);

		const agents = agentService.getActiveAgents();
		if (agents.length === 0) {
			notificationService.warn(localize('noActiveAgents', 'No active agents available.'));
			return;
		}

		const agentPicks = agents.map(agent => ({
			id: agent.id,
			label: agent.definition.name,
			description: agent.state,
			detail: agent.definition.description
		}));

		const selectedAgent = await quickInputService.pick(agentPicks, {
			placeHolder: localize('selectAgentForTask', 'Select an agent to run the task')
		});

		if (!selectedAgent || !selectedAgent.id) {
			return;
		}

		const taskDescription = await quickInputService.input({
			placeHolder: localize('taskDescriptionPlaceholder', 'Describe the task for the agent'),
			prompt: localize('taskDescriptionPrompt', 'What should the agent do?')
		});

		if (!taskDescription) {
			return;
		}

		const taskId = `task_${Date.now()}`;
		
		try {
			const progress = notificationService.withProgress({
				location: 15, // Notification location
				title: localize('runningAgentTask', 'Running agent task...')
			}, async () => {
				const result = await agentService.executeTask({
					id: taskId,
					agentId: selectedAgent.id,
					description: taskDescription,
					input: {}
				});
				
				return result;
			});

			const result = await progress;
			
			if (result.success) {
				// Open result in new editor
				await editorService.openEditor({
					resource: undefined,
					contents: result.output,
					options: {
						pinned: true
					}
				});

				notificationService.info(localize('taskCompleted', 'Task completed successfully (Duration: {0}ms)', result.duration));
			} else {
				notificationService.error(localize('taskFailed', 'Task failed: {0}', result.error));
			}

		} catch (error) {
			notificationService.error(localize('taskExecutionError', 'Failed to execute task: {0}', error.message));
		}
	}
}

// Register actions
registerAction2(CreateAgentAction);
registerAction2(ManageAgentsAction);
registerAction2(RunAgentTaskAction);