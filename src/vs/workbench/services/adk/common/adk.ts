/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Event } from '../../../../base/common/event.js';
import { IDisposable } from '../../../../base/common/lifecycle.js';
import { URI } from '../../../../base/common/uri.js';
import { createDecorator } from '../../../../platform/instantiation/common/instantiation.js';

export const IAgentService = createDecorator<IAgentService>('agentService');
export const IAgentRegistryService = createDecorator<IAgentRegistryService>('agentRegistryService');

export interface IAgentCapability {
	id: string;
	name: string;
	description: string;
}

export interface IAgentDefinition {
	id: string;
	name: string;
	description: string;
	version: string;
	author?: string;
	capabilities: IAgentCapability[];
	requiredTools?: string[];
	modelRequirements?: {
		minContextWindow?: number;
		requiredCapabilities?: string[];
		preferredModels?: string[];
	};
}

export interface IAgentInstance {
	id: string;
	definition: IAgentDefinition;
	state: AgentState;
	createdAt: Date;
	lastActiveAt: Date;
}

export enum AgentState {
	Idle = 'idle',
	Running = 'running',
	Suspended = 'suspended',
	Failed = 'failed',
	Terminated = 'terminated'
}

export interface IAgentTask {
	id: string;
	agentId: string;
	description: string;
	input: any;
	priority?: number;
	timeout?: number;
	metadata?: Record<string, any>;
}

export interface IAgentResult {
	taskId: string;
	agentId: string;
	success: boolean;
	output?: any;
	error?: string;
	duration: number;
	metadata?: Record<string, any>;
}

export interface IAgentTool {
	id: string;
	name: string;
	description: string;
	execute(params: any): Promise<any>;
}

export interface IAgentContext {
	workspaceUri?: URI;
	activeEditorUri?: URI;
	selectedText?: string;
	variables: Map<string, any>;
	tools: Map<string, IAgentTool>;
}

export interface IAgentService {
	readonly _serviceBrand: undefined;

	/**
	 * Event fired when an agent state changes
	 */
	readonly onDidChangeAgentState: Event<{ agentId: string; state: AgentState }>;

	/**
	 * Event fired when a task completes
	 */
	readonly onDidCompleteTask: Event<IAgentResult>;

	/**
	 * Create a new agent instance
	 */
	createAgent(definition: IAgentDefinition): Promise<IAgentInstance>;

	/**
	 * Execute a task with an agent
	 */
	executeTask(task: IAgentTask): Promise<IAgentResult>;

	/**
	 * Get an agent instance by ID
	 */
	getAgent(agentId: string): IAgentInstance | undefined;

	/**
	 * Get all active agents
	 */
	getActiveAgents(): IAgentInstance[];

	/**
	 * Suspend an agent
	 */
	suspendAgent(agentId: string): Promise<void>;

	/**
	 * Resume an agent
	 */
	resumeAgent(agentId: string): Promise<void>;

	/**
	 * Terminate an agent
	 */
	terminateAgent(agentId: string): Promise<void>;

	/**
	 * Register a tool for agents to use
	 */
	registerTool(tool: IAgentTool): IDisposable;

	/**
	 * Get available tools
	 */
	getAvailableTools(): IAgentTool[];
}

export interface IAgentRegistryService {
	readonly _serviceBrand: undefined;

	/**
	 * Event fired when agent definitions change
	 */
	readonly onDidChangeAgents: Event<void>;

	/**
	 * Register an agent definition
	 */
	registerAgent(definition: IAgentDefinition): void;

	/**
	 * Unregister an agent definition
	 */
	unregisterAgent(agentId: string): void;

	/**
	 * Get all registered agent definitions
	 */
	getAgentDefinitions(): IAgentDefinition[];

	/**
	 * Get agent definition by ID
	 */
	getAgentDefinition(agentId: string): IAgentDefinition | undefined;

	/**
	 * Get agents by capability
	 */
	getAgentsByCapability(capabilityId: string): IAgentDefinition[];

	/**
	 * Import agent definitions from a file
	 */
	importAgents(uri: URI): Promise<IAgentDefinition[]>;

	/**
	 * Export agent definitions to a file
	 */
	exportAgents(agentIds: string[], uri: URI): Promise<void>;
}

export interface IAgentFactory {
	createAgent(definition: IAgentDefinition, context: IAgentContext): IAgent;
}

export interface IAgent {
	readonly id: string;
	readonly definition: IAgentDefinition;
	
	initialize(): Promise<void>;
	execute(task: IAgentTask): Promise<IAgentResult>;
	suspend(): Promise<void>;
	resume(): Promise<void>;
	dispose(): void;
}

export interface IAgentOrchestrator {
	/**
	 * Coordinate multiple agents to complete a complex task
	 */
	orchestrate(task: IOrchestrationTask): Promise<IOrchestrationResult>;

	/**
	 * Create an orchestration plan
	 */
	createPlan(task: IOrchestrationTask): Promise<IOrchestrationPlan>;

	/**
	 * Execute an orchestration plan
	 */
	executePlan(plan: IOrchestrationPlan): Promise<IOrchestrationResult>;
}

export interface IOrchestrationTask {
	id: string;
	description: string;
	requiredCapabilities: string[];
	constraints?: {
		maxAgents?: number;
		timeout?: number;
		budget?: number;
	};
	input: any;
}

export interface IOrchestrationPlan {
	taskId: string;
	steps: IOrchestrationStep[];
	estimatedDuration?: number;
	estimatedCost?: number;
}

export interface IOrchestrationStep {
	id: string;
	agentId: string;
	task: IAgentTask;
	dependencies: string[];
}

export interface IOrchestrationResult {
	taskId: string;
	success: boolean;
	steps: IAgentResult[];
	output?: any;
	error?: string;
	duration: number;
	cost?: number;
}