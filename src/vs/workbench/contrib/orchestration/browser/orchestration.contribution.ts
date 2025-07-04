/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { registerAction2, Action2 } from '../../../platform/actions/common/actions.js';
import { ServicesAccessor } from '../../../platform/instantiation/common/instantiation.js';
import { IQuickInputService } from '../../../platform/quickinput/common/quickInput.js';
import { IOrchestrationService } from '../../../platform/orchestration/common/orchestration.js';
import { localize } from '../../../nls.js';
import { Categories } from '../../../platform/action/common/actionCommonCategories.js';
import { INotificationService } from '../../../platform/notification/common/notification.js';
import { IEditorService } from '../../services/editor/common/editorService.js';
import { ITextModel } from '../../../editor/common/model.js';

// Register orchestration actions
class SelectAIModelAction extends Action2 {
	constructor() {
		super({
			id: 'orchestration.selectModel',
			title: localize('selectAIModel', 'Select AI Model'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const quickInputService = accessor.get(IQuickInputService);
		const orchestrationService = accessor.get(IOrchestrationService);
		const notificationService = accessor.get(INotificationService);

		const models = orchestrationService.getAvailableModels();
		if (models.length === 0) {
			notificationService.warn(localize('noModelsAvailable', 'No AI models are currently available.'));
			return;
		}

		const picks = models.map(model => ({
			id: model.id,
			label: model.displayName,
			description: `${model.provider} - ${model.capabilities.join(', ')}`,
			detail: localize('modelDetail', 'Context: {0} tokens, Cost: ${1}/{2} per 1k tokens', 
				model.contextWindow, 
				model.costPer1kInput.toFixed(3), 
				model.costPer1kOutput.toFixed(3))
		}));

		const selected = await quickInputService.pick(picks, {
			placeHolder: localize('selectModelPlaceholder', 'Select an AI model to use')
		});

		if (selected) {
			notificationService.info(localize('modelSelected', 'Selected model: {0}', selected.label));
		}
	}
}

class AskAIAction extends Action2 {
	constructor() {
		super({
			id: 'orchestration.askAI',
			title: localize('askAI', 'Ask AI'),
			category: Categories.AI,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const quickInputService = accessor.get(IQuickInputService);
		const orchestrationService = accessor.get(IOrchestrationService);
		const notificationService = accessor.get(INotificationService);
		const editorService = accessor.get(IEditorService);

		const prompt = await quickInputService.input({
			placeHolder: localize('askAIPlaceholder', 'What would you like to ask?'),
			prompt: localize('askAIPrompt', 'Enter your question or request')
		});

		if (!prompt) {
			return;
		}

		try {
			const progress = notificationService.withProgress({
				location: 15, // Notification location
				title: localize('processingAI', 'Processing AI request...')
			}, async () => {
				const result = await orchestrationService.sendRequest({
					prompt,
					capabilities: ['general']
				});
				
				return result;
			});

			const result = await progress;
			
			// Open result in new editor
			await editorService.openEditor({
				resource: undefined,
				contents: result.response.content,
				options: {
					pinned: true
				}
			});

			notificationService.info(localize('aiComplete', 'AI request completed (Model: {0}, Time: {1}ms)', 
				result.modelId, result.duration));

		} catch (error) {
			notificationService.error(localize('aiError', 'AI request failed: {0}', error.message));
		}
	}
}

// Register actions
registerAction2(SelectAIModelAction);
registerAction2(AskAIAction);