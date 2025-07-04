/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { registerSingleton } from '../../../../platform/instantiation/common/extensions.js';
import { IAgentService, IAgentRegistryService } from '../common/adk.js';
import { AgentService } from '../common/agentService.js';
import { AgentRegistryService } from '../common/agentRegistryService.js';

// Register ADK services for browser/web environment
registerSingleton(IAgentRegistryService, AgentRegistryService, true);
registerSingleton(IAgentService, AgentService, true);

export { AgentService, AgentRegistryService };