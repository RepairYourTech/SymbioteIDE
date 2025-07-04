/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { IModelProvider, IModelResponse } from '../orchestration.js';
import { IConfigurationService } from '../../../configuration/common/configuration.js';
import { ILogService } from '../../../log/common/log.js';
import { CancellationToken } from '../../../../base/common/cancellation.js';

export class OpenAIProvider implements IModelProvider {
	readonly id = 'openai';
	readonly name = 'OpenAI';

	private apiKey: string | undefined;
	private endpoint = 'https://api.openai.com/v1';
	private initialized = false;

	constructor(
		@IConfigurationService private readonly configurationService: IConfigurationService,
		@ILogService private readonly logService: ILogService
	) {
		this.loadConfiguration();
		this.configurationService.onDidChangeConfiguration(e => {
			if (e.affectsConfiguration('orchestration.providers.openai')) {
				this.loadConfiguration();
			}
		});
	}

	private loadConfiguration(): void {
		this.apiKey = this.configurationService.getValue<string>('orchestration.providers.openai.apiKey');
		const customEndpoint = this.configurationService.getValue<string>('orchestration.providers.openai.endpoint');
		if (customEndpoint) {
			this.endpoint = customEndpoint;
		}
	}

	async initialize(): Promise<void> {
		if (!this.apiKey) {
			this.logService.warn('[OpenAIProvider] No API key configured');
			return;
		}
		this.initialized = true;
		this.logService.info('[OpenAIProvider] Initialized');
	}

	isAvailable(): boolean {
		return this.initialized && !!this.apiKey;
	}

	async generateResponse(prompt: string, config?: any): Promise<IModelResponse> {
		if (!this.isAvailable()) {
			throw new Error('OpenAI provider not available');
		}

		const model = config?.model || 'gpt-3.5-turbo';
		const maxTokens = config?.maxTokens || 2048;
		const temperature = config?.temperature ?? 0.7;

		try {
			const response = await fetch(`${this.endpoint}/chat/completions`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': `Bearer ${this.apiKey}`
				},
				body: JSON.stringify({
					model,
					messages: [{ role: 'user', content: prompt }],
					max_tokens: maxTokens,
					temperature
				})
			});

			if (!response.ok) {
				const error = await response.text();
				throw new Error(`OpenAI API error: ${error}`);
			}

			const data = await response.json();
			const choice = data.choices[0];

			return {
				content: choice.message.content,
				modelId: model,
				usage: data.usage ? {
					promptTokens: data.usage.prompt_tokens,
					completionTokens: data.usage.completion_tokens,
					totalTokens: data.usage.total_tokens
				} : undefined,
				metadata: {
					finishReason: choice.finish_reason,
					model: data.model
				}
			};
		} catch (error) {
			this.logService.error('[OpenAIProvider] Generation failed:', error);
			throw error;
		}
	}

	async *streamResponse(prompt: string, config?: any): AsyncIterable<string> {
		if (!this.isAvailable()) {
			throw new Error('OpenAI provider not available');
		}

		const model = config?.model || 'gpt-3.5-turbo';
		const maxTokens = config?.maxTokens || 2048;
		const temperature = config?.temperature ?? 0.7;

		try {
			const response = await fetch(`${this.endpoint}/chat/completions`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'Authorization': `Bearer ${this.apiKey}`
				},
				body: JSON.stringify({
					model,
					messages: [{ role: 'user', content: prompt }],
					max_tokens: maxTokens,
					temperature,
					stream: true
				})
			});

			if (!response.ok) {
				const error = await response.text();
				throw new Error(`OpenAI API error: ${error}`);
			}

			const reader = response.body?.getReader();
			if (!reader) {
				throw new Error('Response body is not readable');
			}

			const decoder = new TextDecoder();
			let buffer = '';

			while (true) {
				const { done, value } = await reader.read();
				if (done) break;

				buffer += decoder.decode(value, { stream: true });
				const lines = buffer.split('\n');
				buffer = lines.pop() || '';

				for (const line of lines) {
					if (line.startsWith('data: ')) {
						const data = line.slice(6);
						if (data === '[DONE]') {
							return;
						}
						try {
							const json = JSON.parse(data);
							const content = json.choices[0]?.delta?.content;
							if (content) {
								yield content;
							}
						} catch (e) {
							// Ignore parse errors
						}
					}
				}
			}
		} catch (error) {
			this.logService.error('[OpenAIProvider] Stream failed:', error);
			throw error;
		}
	}

	dispose(): void {
		// Clean up resources if needed
	}
}