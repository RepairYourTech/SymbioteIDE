/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Registry } from '../../../base/common/platform.js';
import { IConfigurationRegistry, Extensions as ConfigurationExtensions } from '../../configuration/common/configurationRegistry.js';
import { localize } from '../../../nls.js';

const configurationRegistry = Registry.as<IConfigurationRegistry>(ConfigurationExtensions.Configuration);

configurationRegistry.registerConfiguration({
	id: 'orchestration',
	title: localize('orchestrationConfigurationTitle', "AI Orchestration"),
	type: 'object',
	properties: {
		'orchestration.enabled': {
			type: 'boolean',
			default: true,
			description: localize('orchestration.enabled', "Enable AI orchestration features.")
		},
		'orchestration.defaultProvider': {
			type: 'string',
			enum: ['openai', 'anthropic', 'google', 'mistral', 'local'],
			enumDescriptions: [
				localize('orchestration.provider.openai', "OpenAI (GPT models)"),
				localize('orchestration.provider.anthropic', "Anthropic (Claude models)"),
				localize('orchestration.provider.google', "Google (Gemini models)"),
				localize('orchestration.provider.mistral', "Mistral AI"),
				localize('orchestration.provider.local', "Local models (Ollama)")
			],
			default: 'openai',
			description: localize('orchestration.defaultProvider', "Default AI provider to use.")
		},
		'orchestration.cache.enabled': {
			type: 'boolean',
			default: true,
			description: localize('orchestration.cache.enabled', "Enable response caching.")
		},
		'orchestration.cache.ttl': {
			type: 'number',
			default: 3600,
			minimum: 0,
			description: localize('orchestration.cache.ttl', "Cache time-to-live in seconds.")
		},
		'orchestration.cache.semanticCaching': {
			type: 'boolean',
			default: false,
			description: localize('orchestration.cache.semanticCaching', "Enable semantic similarity caching (experimental).")
		},
		'orchestration.monitoring.enabled': {
			type: 'boolean',
			default: true,
			description: localize('orchestration.monitoring.enabled', "Enable performance monitoring.")
		},
		'orchestration.monitoring.logLevel': {
			type: 'string',
			enum: ['error', 'warn', 'info', 'debug'],
			default: 'info',
			description: localize('orchestration.monitoring.logLevel', "Logging level for orchestration events.")
		},
		'orchestration.rateLimiting.enabled': {
			type: 'boolean',
			default: true,
			description: localize('orchestration.rateLimiting.enabled', "Enable rate limiting.")
		},
		'orchestration.rateLimiting.requestsPerMinute': {
			type: 'number',
			default: 60,
			minimum: 1,
			description: localize('orchestration.rateLimiting.requestsPerMinute', "Maximum requests per minute.")
		},
		'orchestration.abTesting.enabled': {
			type: 'boolean',
			default: false,
			description: localize('orchestration.abTesting.enabled', "Enable A/B testing for model selection.")
		},
		'orchestration.providers.openai.apiKey': {
			type: 'string',
			default: '',
			description: localize('orchestration.providers.openai.apiKey', "OpenAI API key."),
			scope: 'machine'
		},
		'orchestration.providers.anthropic.apiKey': {
			type: 'string',
			default: '',
			description: localize('orchestration.providers.anthropic.apiKey', "Anthropic API key."),
			scope: 'machine'
		},
		'orchestration.providers.google.apiKey': {
			type: 'string',
			default: '',
			description: localize('orchestration.providers.google.apiKey', "Google AI API key."),
			scope: 'machine'
		},
		'orchestration.providers.mistral.apiKey': {
			type: 'string',
			default: '',
			description: localize('orchestration.providers.mistral.apiKey', "Mistral AI API key."),
			scope: 'machine'
		},
		'orchestration.providers.local.endpoint': {
			type: 'string',
			default: 'http://localhost:11434',
			description: localize('orchestration.providers.local.endpoint', "Local model endpoint (Ollama).")
		}
	}
});