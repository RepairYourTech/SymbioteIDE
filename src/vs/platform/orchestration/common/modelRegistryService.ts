/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Emitter, Event } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { IStorageService, StorageScope, StorageTarget } from '../../storage/common/storage.js';
import { IModelConfig, IModelRegistryService } from './orchestration.js';

const MODELS_STORAGE_KEY = 'symbiote.orchestration.models';

export class ModelRegistryService extends Disposable implements IModelRegistryService {
	readonly _serviceBrand: undefined;

	private readonly _onDidChangeModels = this._register(new Emitter<void>());
	readonly onDidChangeModels: Event<void> = this._onDidChangeModels.event;

	private models = new Map<string, IModelConfig>();

	constructor(
		@IStorageService private readonly storageService: IStorageService
	) {
		super();
		this.loadModels();
		this.registerDefaultModels();
	}

	registerModel(config: IModelConfig): void {
		this.models.set(config.id, { ...config, enabled: config.enabled ?? true });
		this.saveModels();
		this._onDidChangeModels.fire();
	}

	unregisterModel(modelId: string): void {
		if (this.models.delete(modelId)) {
			this.saveModels();
			this._onDidChangeModels.fire();
		}
	}

	getModels(): IModelConfig[] {
		return Array.from(this.models.values());
	}

	getModel(modelId: string): IModelConfig | undefined {
		return this.models.get(modelId);
	}

	getModelsByCapability(capability: string): IModelConfig[] {
		return this.getModels().filter(m => m.capabilities.includes(capability));
	}

	updateModel(modelId: string, config: Partial<IModelConfig>): void {
		const model = this.models.get(modelId);
		if (model) {
			this.models.set(modelId, { ...model, ...config });
			this.saveModels();
			this._onDidChangeModels.fire();
		}
	}

	private loadModels(): void {
		const stored = this.storageService.get(MODELS_STORAGE_KEY, StorageScope.APPLICATION);
		if (stored) {
			try {
				const models = JSON.parse(stored) as IModelConfig[];
				models.forEach(model => this.models.set(model.id, model));
			} catch (e) {
				console.error('Failed to load models from storage:', e);
			}
		}
	}

	private saveModels(): void {
		const models = Array.from(this.models.values());
		this.storageService.store(MODELS_STORAGE_KEY, JSON.stringify(models), StorageScope.APPLICATION, StorageTarget.USER);
	}

	private registerDefaultModels(): void {
		// Only register if no models exist
		if (this.models.size > 0) {
			return;
		}

		const defaultModels: IModelConfig[] = [
			{
				id: 'gpt-4-turbo',
				provider: 'openai',
				displayName: 'GPT-4 Turbo',
				capabilities: ['code-generation', 'debugging', 'explanation', 'code-refactoring', 'general'],
				maxTokens: 4096,
				costPer1kInput: 0.01,
				costPer1kOutput: 0.03,
				contextWindow: 128000,
				priority: 90,
				enabled: true
			},
			{
				id: 'gpt-3.5-turbo',
				provider: 'openai',
				displayName: 'GPT-3.5 Turbo',
				capabilities: ['code-generation', 'explanation', 'general'],
				maxTokens: 4096,
				costPer1kInput: 0.0005,
				costPer1kOutput: 0.0015,
				contextWindow: 16385,
				priority: 70,
				enabled: true
			},
			{
				id: 'claude-3-opus',
				provider: 'anthropic',
				displayName: 'Claude 3 Opus',
				capabilities: ['code-generation', 'debugging', 'explanation', 'code-refactoring', 'general'],
				maxTokens: 4096,
				costPer1kInput: 0.015,
				costPer1kOutput: 0.075,
				contextWindow: 200000,
				priority: 95,
				enabled: true
			},
			{
				id: 'claude-3-sonnet',
				provider: 'anthropic',
				displayName: 'Claude 3 Sonnet',
				capabilities: ['code-generation', 'debugging', 'explanation', 'general'],
				maxTokens: 4096,
				costPer1kInput: 0.003,
				costPer1kOutput: 0.015,
				contextWindow: 200000,
				priority: 85,
				enabled: true
			},
			{
				id: 'gemini-pro',
				provider: 'google',
				displayName: 'Gemini Pro',
				capabilities: ['code-generation', 'explanation', 'general'],
				maxTokens: 2048,
				costPer1kInput: 0.0005,
				costPer1kOutput: 0.0015,
				contextWindow: 32000,
				priority: 75,
				enabled: true
			},
			{
				id: 'mistral-large',
				provider: 'mistral',
				displayName: 'Mistral Large',
				capabilities: ['code-generation', 'explanation', 'general'],
				maxTokens: 4096,
				costPer1kInput: 0.008,
				costPer1kOutput: 0.024,
				contextWindow: 32000,
				priority: 80,
				enabled: true
			},
			{
				id: 'codellama-70b',
				provider: 'local',
				displayName: 'Code Llama 70B',
				capabilities: ['code-generation', 'debugging', 'code-refactoring'],
				maxTokens: 4096,
				costPer1kInput: 0,
				costPer1kOutput: 0,
				contextWindow: 100000,
				priority: 60,
				enabled: false // Disabled by default as it requires local setup
			}
		];

		defaultModels.forEach(model => this.registerModel(model));
	}
}