/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { OrchestrationService } from '../common/orchestrationService.js';
import { IModelRegistryService } from '../common/orchestration.js';
import { ILogService } from '../../log/common/log.js';
import { ITelemetryService } from '../../telemetry/common/telemetry.js';
import { IStorageService } from '../../storage/common/storage.js';
import { registerSingleton } from '../../instantiation/common/extensions.js';
import { IOrchestrationService } from '../common/orchestration.js';
import { ModelRegistryService } from '../common/modelRegistryService.js';

// Register services for browser/web environment
registerSingleton(IModelRegistryService, ModelRegistryService, true);
registerSingleton(IOrchestrationService, OrchestrationService, true);

export { OrchestrationService };