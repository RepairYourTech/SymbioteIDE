/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Emitter, Event } from '../../../base/common/event.js';
import { Disposable, IDisposable, toDisposable } from '../../../base/common/lifecycle.js';
import { ILogService } from '../../log/common/log.js';
import { ITelemetryService } from '../../telemetry/common/telemetry.js';
import { 
	IOrchestrationService, 
	IModelProvider, 
	IOrchestrationRequest, 
	IOrchestrationResult,
	ITaskAnalysis,
	IModelConfig,
	IModelRegistryService,
	IModelResponse,
	OrchestrationMode,
	IOrchestrationStrategy
} from './orchestration.js';

export class OrchestrationService extends Disposable implements IOrchestrationService {
	readonly _serviceBrand: undefined;

	private readonly _onDidChangeModels = this._register(new Emitter<void>());
	readonly onDidChangeModels: Event<void> = this._onDidChangeModels.event;

	private readonly _onDidCompleteRequest = this._register(new Emitter<IOrchestrationResult>());
	readonly onDidCompleteRequest: Event<IOrchestrationResult> = this._onDidCompleteRequest.event;

	private readonly providers = new Map<string, IModelProvider>();
	private readonly activeRequests = new Set<string>();

	constructor(
		@IModelRegistryService private readonly modelRegistry: IModelRegistryService,
		@ILogService private readonly logService: ILogService,
		@ITelemetryService private readonly telemetryService: ITelemetryService
	) {
		super();
		this._register(this.modelRegistry.onDidChangeModels(() => this._onDidChangeModels.fire()));
	}

	registerProvider(provider: IModelProvider): IDisposable {
		if (this.providers.has(provider.id)) {
			throw new Error(`Provider ${provider.id} already registered`);
		}

		this.providers.set(provider.id, provider);
		this.logService.info(`[OrchestrationService] Registered provider: ${provider.id}`);
		
		provider.initialize().catch(err => {
			this.logService.error(`[OrchestrationService] Failed to initialize provider ${provider.id}:`, err);
		});

		return toDisposable(() => {
			this.providers.delete(provider.id);
			provider.dispose();
			this.logService.info(`[OrchestrationService] Unregistered provider: ${provider.id}`);
		});
	}

	async sendRequest(request: IOrchestrationRequest): Promise<IOrchestrationResult> {
		const requestId = this.generateRequestId();
		this.activeRequests.add(requestId);

		const startTime = Date.now();
		
		try {
			this.logService.debug(`[OrchestrationService] Processing request ${requestId}`);
			
			// Analyze task if no specific model requested
			let modelId = request.modelId;
			if (!modelId && request.capabilities) {
				const analysis = await this.analyzeTask(request.prompt);
				modelId = analysis.suggestedModels[0];
			}

			// Get model and provider
			const model = this.getModel(modelId || this.getDefaultModelId());
			if (!model) {
				throw new Error(`Model ${modelId} not found`);
			}

			const provider = this.providers.get(model.provider);
			if (!provider || !provider.isAvailable()) {
				throw new Error(`Provider ${model.provider} not available`);
			}

			// Generate response
			const response = await provider.generateResponse(request.prompt, {
				maxTokens: request.maxTokens || model.maxTokens,
				temperature: request.temperature,
				...request.metadata
			});

			const duration = Date.now() - startTime;
			const cost = this.calculateCost(model, response.usage);

			const result: IOrchestrationResult = {
				response,
				modelId: model.id,
				duration,
				cost,
				cached: false
			};

			this._onDidCompleteRequest.fire(result);
			
			// Telemetry
			this.telemetryService.publicLog2('orchestration.request.completed', {
				modelId: model.id,
				duration,
				promptTokens: response.usage?.promptTokens || 0,
				completionTokens: response.usage?.completionTokens || 0,
				cost
			});

			return result;

		} catch (error) {
			this.logService.error(`[OrchestrationService] Request ${requestId} failed:`, error);
			
			this.telemetryService.publicLog2('orchestration.request.failed', {
				error: error.message,
				duration: Date.now() - startTime
			});
			
			throw error;
		} finally {
			this.activeRequests.delete(requestId);
		}
	}

	async *streamRequest(request: IOrchestrationRequest): AsyncIterable<string> {
		const modelId = request.modelId || this.getDefaultModelId();
		const model = this.getModel(modelId);
		
		if (!model) {
			throw new Error(`Model ${modelId} not found`);
		}

		const provider = this.providers.get(model.provider);
		if (!provider || !provider.isAvailable()) {
			throw new Error(`Provider ${model.provider} not available`);
		}

		if (!provider.streamResponse) {
			throw new Error(`Provider ${model.provider} does not support streaming`);
		}

		yield* provider.streamResponse(request.prompt, {
			maxTokens: request.maxTokens || model.maxTokens,
			temperature: request.temperature,
			...request.metadata
		});
	}

	async analyzeTask(prompt: string): Promise<ITaskAnalysis> {
		// Simple task analysis - in production this would use NLP
		const capabilities: string[] = [];
		const promptLower = prompt.toLowerCase();

		if (promptLower.includes('code') || promptLower.includes('programming')) {
			capabilities.push('code-generation');
		}
		if (promptLower.includes('debug') || promptLower.includes('error')) {
			capabilities.push('debugging');
		}
		if (promptLower.includes('explain') || promptLower.includes('understand')) {
			capabilities.push('explanation');
		}
		if (promptLower.includes('refactor') || promptLower.includes('improve')) {
			capabilities.push('code-refactoring');
		}

		const complexity = prompt.length > 500 ? 'high' : prompt.length > 200 ? 'medium' : 'low';
		const estimatedTokens = Math.ceil(prompt.length / 4) * 2; // Rough estimate

		const suggestedModels = this.modelRegistry.getModels()
			.filter(m => m.enabled && capabilities.every(c => m.capabilities.includes(c)))
			.sort((a, b) => (b.priority || 0) - (a.priority || 0))
			.slice(0, 3)
			.map(m => m.id);

		return {
			capabilities,
			complexity,
			estimatedTokens,
			suggestedModels
		};
	}

	getAvailableModels(): IModelConfig[] {
		return this.modelRegistry.getModels().filter(m => {
			const provider = this.providers.get(m.provider);
			return m.enabled && provider && provider.isAvailable();
		});
	}

	getModel(modelId: string): IModelConfig | undefined {
		return this.modelRegistry.getModel(modelId);
	}

	updateModelConfig(modelId: string, config: Partial<IModelConfig>): void {
		this.modelRegistry.updateModel(modelId, config);
	}

	private getDefaultModelId(): string {
		const models = this.getAvailableModels();
		if (models.length === 0) {
			throw new Error('No models available');
		}
		return models[0].id;
	}

	private generateRequestId(): string {
		return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
	}

	private calculateCost(model: IModelConfig, usage?: { promptTokens: number; completionTokens: number }): number {
		if (!usage) {
			return 0;
		}
		return (usage.promptTokens * model.costPer1kInput / 1000) + 
			   (usage.completionTokens * model.costPer1kOutput / 1000);
	}

	override dispose(): void {
		super.dispose();
		this.providers.forEach(provider => provider.dispose());
		this.providers.clear();
	}
}