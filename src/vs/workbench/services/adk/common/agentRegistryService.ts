/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Emitter, Event } from '../../../../base/common/event.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { URI } from '../../../../base/common/uri.js';
import { IFileService } from '../../../../platform/files/common/files.js';
import { IStorageService, StorageScope, StorageTarget } from '../../../../platform/storage/common/storage.js';
import { IAgentDefinition, IAgentRegistryService } from './adk.js';

const AGENTS_STORAGE_KEY = 'symbiote.adk.agents';

export class AgentRegistryService extends Disposable implements IAgentRegistryService {
	readonly _serviceBrand: undefined;

	private readonly _onDidChangeAgents = this._register(new Emitter<void>());
	readonly onDidChangeAgents: Event<void> = this._onDidChangeAgents.event;

	private agents = new Map<string, IAgentDefinition>();

	constructor(
		@IStorageService private readonly storageService: IStorageService,
		@IFileService private readonly fileService: IFileService
	) {
		super();
		this.loadAgents();
		this.registerDefaultAgents();
	}

	registerAgent(definition: IAgentDefinition): void {
		this.agents.set(definition.id, definition);
		this.saveAgents();
		this._onDidChangeAgents.fire();
	}

	unregisterAgent(agentId: string): void {
		if (this.agents.delete(agentId)) {
			this.saveAgents();
			this._onDidChangeAgents.fire();
		}
	}

	getAgentDefinitions(): IAgentDefinition[] {
		return Array.from(this.agents.values());
	}

	getAgentDefinition(agentId: string): IAgentDefinition | undefined {
		return this.agents.get(agentId);
	}

	getAgentsByCapability(capabilityId: string): IAgentDefinition[] {
		return this.getAgentDefinitions().filter(agent => 
			agent.capabilities.some(cap => cap.id === capabilityId)
		);
	}

	async importAgents(uri: URI): Promise<IAgentDefinition[]> {
		try {
			const content = await this.fileService.readFile(uri);
			const text = content.value.toString();
			const definitions = JSON.parse(text) as IAgentDefinition[];

			for (const definition of definitions) {
				this.registerAgent(definition);
			}

			return definitions;
		} catch (error) {
			throw new Error(`Failed to import agents: ${error.message}`);
		}
	}

	async exportAgents(agentIds: string[], uri: URI): Promise<void> {
		const definitions = agentIds
			.map(id => this.agents.get(id))
			.filter(def => def !== undefined) as IAgentDefinition[];

		const content = JSON.stringify(definitions, null, 2);
		await this.fileService.writeFile(uri, Buffer.from(content));
	}

	private loadAgents(): void {
		const stored = this.storageService.get(AGENTS_STORAGE_KEY, StorageScope.APPLICATION);
		if (stored) {
			try {
				const agents = JSON.parse(stored) as IAgentDefinition[];
				agents.forEach(agent => this.agents.set(agent.id, agent));
			} catch (e) {
				console.error('Failed to load agents from storage:', e);
			}
		}
	}

	private saveAgents(): void {
		const agents = Array.from(this.agents.values());
		this.storageService.store(AGENTS_STORAGE_KEY, JSON.stringify(agents), StorageScope.APPLICATION, StorageTarget.USER);
	}

	private registerDefaultAgents(): void {
		// Only register if no agents exist
		if (this.agents.size > 0) {
			return;
		}

		const defaultAgents: IAgentDefinition[] = [
			{
				id: 'code-reviewer',
				name: 'Code Reviewer',
				description: 'Reviews code for quality, best practices, and potential issues',
				version: '1.0.0',
				author: 'SymbioteIDE',
				capabilities: [
					{
						id: 'code-analysis',
						name: 'Code Analysis',
						description: 'Analyzes code structure and patterns'
					},
					{
						id: 'best-practices',
						name: 'Best Practices',
						description: 'Checks adherence to coding best practices'
					},
					{
						id: 'security-review',
						name: 'Security Review',
						description: 'Identifies potential security vulnerabilities'
					}
				],
				requiredTools: ['workspace.readFile'],
				modelRequirements: {
					minContextWindow: 8000,
					requiredCapabilities: ['code-generation', 'debugging']
				}
			},
			{
				id: 'test-generator',
				name: 'Test Generator',
				description: 'Generates comprehensive test suites for code',
				version: '1.0.0',
				author: 'SymbioteIDE',
				capabilities: [
					{
						id: 'unit-tests',
						name: 'Unit Test Generation',
						description: 'Creates unit tests for functions and classes'
					},
					{
						id: 'integration-tests',
						name: 'Integration Test Generation',
						description: 'Creates integration tests for modules'
					},
					{
						id: 'test-coverage',
						name: 'Test Coverage Analysis',
						description: 'Analyzes and improves test coverage'
					}
				],
				requiredTools: ['workspace.readFile', 'workspace.writeFile'],
				modelRequirements: {
					minContextWindow: 16000,
					requiredCapabilities: ['code-generation']
				}
			},
			{
				id: 'refactoring-assistant',
				name: 'Refactoring Assistant',
				description: 'Helps refactor code for better maintainability',
				version: '1.0.0',
				author: 'SymbioteIDE',
				capabilities: [
					{
						id: 'code-refactoring',
						name: 'Code Refactoring',
						description: 'Suggests and implements code refactoring'
					},
					{
						id: 'pattern-detection',
						name: 'Pattern Detection',
						description: 'Identifies code patterns and anti-patterns'
					},
					{
						id: 'optimization',
						name: 'Code Optimization',
						description: 'Optimizes code for performance'
					}
				],
				requiredTools: ['workspace.readFile', 'workspace.writeFile'],
				modelRequirements: {
					minContextWindow: 32000,
					requiredCapabilities: ['code-generation', 'code-refactoring']
				}
			},
			{
				id: 'documentation-writer',
				name: 'Documentation Writer',
				description: 'Generates and maintains code documentation',
				version: '1.0.0',
				author: 'SymbioteIDE',
				capabilities: [
					{
						id: 'api-docs',
						name: 'API Documentation',
						description: 'Generates API documentation'
					},
					{
						id: 'inline-docs',
						name: 'Inline Documentation',
						description: 'Adds inline comments and docstrings'
					},
					{
						id: 'readme-generation',
						name: 'README Generation',
						description: 'Creates and updates README files'
					}
				],
				requiredTools: ['workspace.readFile', 'workspace.writeFile'],
				modelRequirements: {
					minContextWindow: 8000,
					requiredCapabilities: ['explanation', 'general']
				}
			},
			{
				id: 'debugging-specialist',
				name: 'Debugging Specialist',
				description: 'Advanced debugging and error analysis',
				version: '1.0.0',
				author: 'SymbioteIDE',
				capabilities: [
					{
						id: 'error-analysis',
						name: 'Error Analysis',
						description: 'Analyzes error messages and stack traces'
					},
					{
						id: 'bug-detection',
						name: 'Bug Detection',
						description: 'Identifies potential bugs in code'
					},
					{
						id: 'fix-suggestion',
						name: 'Fix Suggestion',
						description: 'Suggests fixes for identified issues'
					}
				],
				requiredTools: ['workspace.readFile'],
				modelRequirements: {
					minContextWindow: 16000,
					requiredCapabilities: ['debugging', 'code-generation']
				}
			}
		];

		defaultAgents.forEach(agent => this.registerAgent(agent));
	}
}