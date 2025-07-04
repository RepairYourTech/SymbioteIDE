/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { IModelProvider, IModelResponse } from '../orchestration.js';
import { IConfigurationService } from '../../../configuration/common/configuration.js';
import { ILogService } from '../../../log/common/log.js';

export class AnthropicProvider implements IModelProvider {
	readonly id = 'anthropic';
	readonly name = 'Anthropic';

	private apiKey: string | undefined;
	private endpoint = 'https://api.anthropic.com/v1';
	private initialized = false;

	constructor(
		@IConfigurationService private readonly configurationService: IConfigurationService,
		@ILogService private readonly logService: ILogService
	) {
		this.loadConfiguration();
		this.configurationService.onDidChangeConfiguration(e => {
			if (e.affectsConfiguration('orchestration.providers.anthropic')) {
				this.loadConfiguration();
			}
		});
	}

	private loadConfiguration(): void {
		this.apiKey = this.configurationService.getValue<string>('orchestration.providers.anthropic.apiKey');
		const customEndpoint = this.configurationService.getValue<string>('orchestration.providers.anthropic.endpoint');
		if (customEndpoint) {
			this.endpoint = customEndpoint;
		}
	}

	async initialize(): Promise<void> {
		if (!this.apiKey) {
			this.logService.warn('[AnthropicProvider] No API key configured');
			return;
		}
		this.initialized = true;
		this.logService.info('[AnthropicProvider] Initialized');
	}

	isAvailable(): boolean {
		return this.initialized && !!this.apiKey;
	}

	async generateResponse(prompt: string, config?: any): Promise<IModelResponse> {
		if (!this.isAvailable()) {
			throw new Error('Anthropic provider not available');
		}

		const model = config?.model || 'claude-3-sonnet-20240229';
		const maxTokens = config?.maxTokens || 4096;
		const temperature = config?.temperature ?? 0.7;

		try {
			const response = await fetch(`${this.endpoint}/messages`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'x-api-key': this.apiKey!,
					'anthropic-version': '2023-06-01'
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
				throw new Error(`Anthropic API error: ${error}`);
			}

			const data = await response.json();

			// Calculate token usage from content length (approximation)
			const promptTokens = Math.ceil(prompt.length / 4);
			const completionTokens = Math.ceil(data.content[0].text.length / 4);

			return {
				content: data.content[0].text,
				modelId: model,
				usage: {
					promptTokens,
					completionTokens,
					totalTokens: promptTokens + completionTokens
				},
				metadata: {
					id: data.id,
					model: data.model,
					stopReason: data.stop_reason
				}
			};
		} catch (error) {
			this.logService.error('[AnthropicProvider] Generation failed:', error);
			throw error;
		}
	}

	async *streamResponse(prompt: string, config?: any): AsyncIterable<string> {
		if (!this.isAvailable()) {
			throw new Error('Anthropic provider not available');
		}

		const model = config?.model || 'claude-3-sonnet-20240229';
		const maxTokens = config?.maxTokens || 4096;
		const temperature = config?.temperature ?? 0.7;

		try {
			const response = await fetch(`${this.endpoint}/messages`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'x-api-key': this.apiKey!,
					'anthropic-version': '2023-06-01'
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
				throw new Error(`Anthropic API error: ${error}`);
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
						try {
							const json = JSON.parse(data);
							if (json.type === 'content_block_delta' && json.delta?.text) {
								yield json.delta.text;
							}
						} catch (e) {
							// Ignore parse errors
						}
					}
				}
			}
		} catch (error) {
			this.logService.error('[AnthropicProvider] Stream failed:', error);
			throw error;
		}
	}

	dispose(): void {
		// Clean up resources if needed
	}
}