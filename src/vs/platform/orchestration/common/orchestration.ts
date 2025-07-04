/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Event } from '../../../base/common/event.js';
import { IDisposable } from '../../../base/common/lifecycle.js';
import { URI } from '../../../base/common/uri.js';
import { createDecorator } from '../../instantiation/common/instantiation.js';

export const IOrchestrationService = createDecorator<IOrchestrationService>('orchestrationService');
export const IModelRegistryService = createDecorator<IModelRegistryService>('modelRegistryService');

export interface IModelConfig {
	id: string;
	provider: string;
	displayName: string;
	capabilities: string[];
	maxTokens: number;
	costPer1kInput: number;
	costPer1kOutput: number;
	contextWindow: number;
	priority?: number;
	enabled?: boolean;
	rateLimit?: {
		requestsPerMinute: number;
		tokensPerMinute: number;
	};
}

export interface IModelProvider {
	id: string;
	name: string;
	initialize(): Promise<void>;
	isAvailable(): boolean;
	generateResponse(prompt: string, config?: any): Promise<IModelResponse>;
	streamResponse?(prompt: string, config?: any): AsyncIterable<string>;
	dispose(): void;
}

export interface IModelResponse {
	content: string;
	modelId: string;
	usage?: {
		promptTokens: number;
		completionTokens: number;
		totalTokens: number;
	};
	metadata?: any;
}

export interface IOrchestrationRequest {
	prompt: string;
	modelId?: string;
	capabilities?: string[];
	maxTokens?: number;
	temperature?: number;
	stream?: boolean;
	metadata?: any;
}

export interface IOrchestrationResult {
	response: IModelResponse;
	modelId: string;
	duration: number;
	cost?: number;
	cached?: boolean;
}

export interface ITaskAnalysis {
	capabilities: string[];
	complexity: 'low' | 'medium' | 'high';
	estimatedTokens: number;
	suggestedModels: string[];
	metadata?: any;
}

export interface IOrchestrationService {
	readonly _serviceBrand: undefined;

	/**
	 * Event fired when a model is registered or unregistered
	 */
	readonly onDidChangeModels: Event<void>;

	/**
	 * Event fired when orchestration completes
	 */
	readonly onDidCompleteRequest: Event<IOrchestrationResult>;

	/**
	 * Register a model provider
	 */
	registerProvider(provider: IModelProvider): IDisposable;

	/**
	 * Send a request to the orchestration engine
	 */
	sendRequest(request: IOrchestrationRequest): Promise<IOrchestrationResult>;

	/**
	 * Stream a request to the orchestration engine
	 */
	streamRequest(request: IOrchestrationRequest): AsyncIterable<string>;

	/**
	 * Analyze a task to determine optimal routing
	 */
	analyzeTask(prompt: string): Promise<ITaskAnalysis>;

	/**
	 * Get available models
	 */
	getAvailableModels(): IModelConfig[];

	/**
	 * Get model by ID
	 */
	getModel(modelId: string): IModelConfig | undefined;

	/**
	 * Update model configuration
	 */
	updateModelConfig(modelId: string, config: Partial<IModelConfig>): void;
}

export interface IModelRegistryService {
	readonly _serviceBrand: undefined;

	/**
	 * Event fired when models change
	 */
	readonly onDidChangeModels: Event<void>;

	/**
	 * Register a model
	 */
	registerModel(config: IModelConfig): void;

	/**
	 * Unregister a model
	 */
	unregisterModel(modelId: string): void;

	/**
	 * Get all registered models
	 */
	getModels(): IModelConfig[];

	/**
	 * Get model by ID
	 */
	getModel(modelId: string): IModelConfig | undefined;

	/**
	 * Get models by capability
	 */
	getModelsByCapability(capability: string): IModelConfig[];

	/**
	 * Update model configuration
	 */
	updateModel(modelId: string, config: Partial<IModelConfig>): void;
}

export enum OrchestrationMode {
	Single = 'single',
	Parallel = 'parallel',
	Sequential = 'sequential',
	Consensus = 'consensus'
}

export interface IOrchestrationStrategy {
	mode: OrchestrationMode;
	modelIds?: string[];
	fallbackModelId?: string;
	consensusThreshold?: number;
	timeout?: number;
}

export interface ICacheConfig {
	enabled: boolean;
	ttl: number;
	maxSize: number;
	semanticCaching?: boolean;
}

export interface IOrchestrationConfig {
	defaultStrategy: IOrchestrationStrategy;
	cache: ICacheConfig;
	monitoring: {
		enabled: boolean;
		metricsInterval: number;
	};
	rateLimiting: {
		enabled: boolean;
		globalLimit: number;
	};
}