/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Emitter, Event } from '../../../../base/common/event.js';
import { Disposable, IDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { ILogService } from '../../../../platform/log/common/log.js';
import { IOrchestrationService } from '../../../../platform/orchestration/common/orchestration.js';
import { IStorageService } from '../../../../platform/storage/common/storage.js';
import { ITelemetryService } from '../../../../platform/telemetry/common/telemetry.js';
import { 
	IAgentService, 
	IAgentInstance, 
	IAgentDefinition, 
	IAgentTask, 
	IAgentResult,
	IAgentTool,
	AgentState,
	IAgent,
	IAgentContext,
	IAgentFactory
} from './adk.js';
import { URI } from '../../../../base/common/uri.js';
import { IWorkspaceContextService } from '../../../../platform/workspace/common/workspace.js';
import { IEditorService } from '../../editor/common/editorService.js';

class AgentInstance implements IAgent {
	private _state: AgentState = AgentState.Idle;
	private _initialized = false;

	constructor(
		readonly id: string,
		readonly definition: IAgentDefinition,
		private readonly context: IAgentContext,
		private readonly orchestrationService: IOrchestrationService,
		private readonly logService: ILogService
	) {}

	async initialize(): Promise<void> {
		if (this._initialized) {
			return;
		}

		this.logService.info(`[Agent ${this.id}] Initializing...`);
		this._state = AgentState.Running;
		this._initialized = true;
	}

	async execute(task: IAgentTask): Promise<IAgentResult> {
		const startTime = Date.now();
		
		try {
			if (this._state !== AgentState.Running) {
				throw new Error(`Agent ${this.id} is not running`);
			}

			this.logService.debug(`[Agent ${this.id}] Executing task ${task.id}`);

			// Use orchestration service to process the task
			const result = await this.orchestrationService.sendRequest({
				prompt: this.buildPrompt(task),
				capabilities: this.definition.capabilities.map(c => c.id),
				metadata: {
					agentId: this.id,
					taskId: task.id,
					...task.metadata
				}
			});

			const duration = Date.now() - startTime;

			return {
				taskId: task.id,
				agentId: this.id,
				success: true,
				output: result.response.content,
				duration,
				metadata: {
					modelUsed: result.modelId,
					tokensUsed: result.response.usage?.totalTokens
				}
			};

		} catch (error) {
			this.logService.error(`[Agent ${this.id}] Task ${task.id} failed:`, error);
			
			return {
				taskId: task.id,
				agentId: this.id,
				success: false,
				error: error.message,
				duration: Date.now() - startTime
			};
		}
	}

	private buildPrompt(task: IAgentTask): string {
		const contextInfo = this.context.workspaceUri ? 
			`Working in workspace: ${this.context.workspaceUri.path}` : '';
		
		const toolsInfo = this.context.tools.size > 0 ?
			`Available tools: ${Array.from(this.context.tools.keys()).join(', ')}` : '';

		return `
You are ${this.definition.name}, an AI agent with the following capabilities:
${this.definition.capabilities.map(c => `- ${c.name}: ${c.description}`).join('\n')}

${contextInfo}
${toolsInfo}

Task: ${task.description}
Input: ${JSON.stringify(task.input)}

Please complete this task and provide a structured response.
		`.trim();
	}

	async suspend(): Promise<void> {
		this._state = AgentState.Suspended;
		this.logService.info(`[Agent ${this.id}] Suspended`);
	}

	async resume(): Promise<void> {
		this._state = AgentState.Running;
		this.logService.info(`[Agent ${this.id}] Resumed`);
	}

	dispose(): void {
		this._state = AgentState.Terminated;
		this.logService.info(`[Agent ${this.id}] Disposed`);
	}

	get state(): AgentState {
		return this._state;
	}
}

export class AgentService extends Disposable implements IAgentService {
	readonly _serviceBrand: undefined;

	private readonly _onDidChangeAgentState = this._register(new Emitter<{ agentId: string; state: AgentState }>());
	readonly onDidChangeAgentState: Event<{ agentId: string; state: AgentState }> = this._onDidChangeAgentState.event;

	private readonly _onDidCompleteTask = this._register(new Emitter<IAgentResult>());
	readonly onDidCompleteTask: Event<IAgentResult> = this._onDidCompleteTask.event;

	private readonly agents = new Map<string, AgentInstance>();
	private readonly tools = new Map<string, IAgentTool>();
	private agentCounter = 0;

	constructor(
		@IOrchestrationService private readonly orchestrationService: IOrchestrationService,
		@IWorkspaceContextService private readonly workspaceService: IWorkspaceContextService,
		@IEditorService private readonly editorService: IEditorService,
		@ILogService private readonly logService: ILogService,
		@ITelemetryService private readonly telemetryService: ITelemetryService,
		@IStorageService private readonly storageService: IStorageService
	) {
		super();
		this.registerBuiltinTools();
	}

	async createAgent(definition: IAgentDefinition): Promise<IAgentInstance> {
		const agentId = `agent_${++this.agentCounter}_${Date.now()}`;
		
		const context = this.createAgentContext();
		const agent = new AgentInstance(
			agentId,
			definition,
			context,
			this.orchestrationService,
			this.logService
		);

		await agent.initialize();
		this.agents.set(agentId, agent);

		const instance: IAgentInstance = {
			id: agentId,
			definition,
			state: agent.state,
			createdAt: new Date(),
			lastActiveAt: new Date()
		};

		this._onDidChangeAgentState.fire({ agentId, state: agent.state });

		this.telemetryService.publicLog2('agent.created', {
			agentId,
			definitionId: definition.id,
			capabilities: definition.capabilities.map(c => c.id).join(',')
		});

		return instance;
	}

	async executeTask(task: IAgentTask): Promise<IAgentResult> {
		const agent = this.agents.get(task.agentId);
		if (!agent) {
			throw new Error(`Agent ${task.agentId} not found`);
		}

		const result = await agent.execute(task);
		this._onDidCompleteTask.fire(result);

		this.telemetryService.publicLog2('agent.task.completed', {
			agentId: task.agentId,
			taskId: task.id,
			success: result.success,
			duration: result.duration
		});

		return result;
	}

	getAgent(agentId: string): IAgentInstance | undefined {
		const agent = this.agents.get(agentId);
		if (!agent) {
			return undefined;
		}

		return {
			id: agentId,
			definition: agent.definition,
			state: agent.state,
			createdAt: new Date(), // Should track this properly
			lastActiveAt: new Date()
		};
	}

	getActiveAgents(): IAgentInstance[] {
		const activeAgents: IAgentInstance[] = [];
		
		for (const [id, agent] of this.agents) {
			if (agent.state !== AgentState.Terminated) {
				activeAgents.push({
					id,
					definition: agent.definition,
					state: agent.state,
					createdAt: new Date(), // Should track this properly
					lastActiveAt: new Date()
				});
			}
		}

		return activeAgents;
	}

	async suspendAgent(agentId: string): Promise<void> {
		const agent = this.agents.get(agentId);
		if (!agent) {
			throw new Error(`Agent ${agentId} not found`);
		}

		await agent.suspend();
		this._onDidChangeAgentState.fire({ agentId, state: agent.state });
	}

	async resumeAgent(agentId: string): Promise<void> {
		const agent = this.agents.get(agentId);
		if (!agent) {
			throw new Error(`Agent ${agentId} not found`);
		}

		await agent.resume();
		this._onDidChangeAgentState.fire({ agentId, state: agent.state });
	}

	async terminateAgent(agentId: string): Promise<void> {
		const agent = this.agents.get(agentId);
		if (!agent) {
			throw new Error(`Agent ${agentId} not found`);
		}

		agent.dispose();
		this.agents.delete(agentId);
		this._onDidChangeAgentState.fire({ agentId, state: AgentState.Terminated });
	}

	registerTool(tool: IAgentTool): IDisposable {
		if (this.tools.has(tool.id)) {
			throw new Error(`Tool ${tool.id} already registered`);
		}

		this.tools.set(tool.id, tool);
		this.logService.info(`[AgentService] Registered tool: ${tool.id}`);

		return toDisposable(() => {
			this.tools.delete(tool.id);
			this.logService.info(`[AgentService] Unregistered tool: ${tool.id}`);
		});
	}

	getAvailableTools(): IAgentTool[] {
		return Array.from(this.tools.values());
	}

	private createAgentContext(): IAgentContext {
		const workspace = this.workspaceService.getWorkspace();
		const activeEditor = this.editorService.activeEditor;

		return {
			workspaceUri: workspace.folders[0]?.uri,
			activeEditorUri: activeEditor?.resource,
			selectedText: undefined, // Would need to get from active editor
			variables: new Map(),
			tools: new Map(this.tools)
		};
	}

	private registerBuiltinTools(): void {
		// Register some basic built-in tools
		this.registerTool({
			id: 'workspace.readFile',
			name: 'Read File',
			description: 'Read contents of a file in the workspace',
			execute: async (params: { path: string }) => {
				// Implementation would read file contents
				return `File contents of ${params.path}`;
			}
		});

		this.registerTool({
			id: 'workspace.writeFile',
			name: 'Write File',
			description: 'Write contents to a file in the workspace',
			execute: async (params: { path: string; content: string }) => {
				// Implementation would write file contents
				return `Written to ${params.path}`;
			}
		});
	}

	override dispose(): void {
		super.dispose();
		for (const agent of this.agents.values()) {
			agent.dispose();
		}
		this.agents.clear();
		this.tools.clear();
	}
}