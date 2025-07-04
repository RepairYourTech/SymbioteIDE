/**
 * Model Registry Unit Tests
 */

import { TestRunner, expect, createMockFunction } from '../utils/test-framework';
import { ModelRegistry } from '../../model-registry';
import { ModelProfile } from '../../types';

const runner = new TestRunner();

runner.describe('ModelRegistry', () => {
  let registry: ModelRegistry;
  
  runner.beforeEach(() => {
    registry = new ModelRegistry();
  });
  
  runner.it('should register a model successfully', async () => {
    const model: ModelProfile = {
      id: 'gpt-4',
      provider: 'openai',
      name: 'GPT-4',
      capabilities: ['chat', 'code'],
      contextWindow: 8192,
      costPerToken: { input: 0.03, output: 0.06 },
      supportsStreaming: true,
      supportsFunctions: true
    };
    
    await registry.registerModel(model);
    const models = await registry.listModels();
    
    expect(models.length).toBe(1);
    expect(models[0].id).toBe('gpt-4');
  });
  
  runner.it('should not register duplicate models', async () => {
    const model: ModelProfile = {
      id: 'gpt-4',
      provider: 'openai',
      name: 'GPT-4',
      capabilities: ['chat'],
      contextWindow: 8192,
      costPerToken: { input: 0.03, output: 0.06 },
      supportsStreaming: true,
      supportsFunctions: true
    };
    
    await registry.registerModel(model);
    
    expect(() => registry.registerModel(model)).toThrow('already registered');
  });
  
  runner.it('should get model by id', async () => {
    const model: ModelProfile = {
      id: 'claude-3',
      provider: 'anthropic',
      name: 'Claude 3',
      capabilities: ['chat'],
      contextWindow: 100000,
      costPerToken: { input: 0.01, output: 0.02 },
      supportsStreaming: true,
      supportsFunctions: false
    };
    
    await registry.registerModel(model);
    const retrieved = await registry.getModel('claude-3');
    
    expect(retrieved).toBeTruthy();
    expect(retrieved?.id).toBe('claude-3');
    expect(retrieved?.provider).toBe('anthropic');
  });
  
  runner.it('should return null for non-existent model', async () => {
    const model = await registry.getModel('non-existent');
    expect(model).toBeFalsy();
  });
  
  runner.it('should filter models by capabilities', async () => {
    const models: ModelProfile[] = [
      {
        id: 'gpt-4',
        provider: 'openai',
        name: 'GPT-4',
        capabilities: ['chat', 'code', 'function'],
        contextWindow: 8192,
        costPerToken: { input: 0.03, output: 0.06 },
        supportsStreaming: true,
        supportsFunctions: true
      },
      {
        id: 'claude-3',
        provider: 'anthropic', 
        name: 'Claude 3',
        capabilities: ['chat', 'code'],
        contextWindow: 100000,
        costPerToken: { input: 0.01, output: 0.02 },
        supportsStreaming: true,
        supportsFunctions: false
      },
      {
        id: 'gemini-pro',
        provider: 'google',
        name: 'Gemini Pro',
        capabilities: ['chat'],
        contextWindow: 32768,
        costPerToken: { input: 0.001, output: 0.002 },
        supportsStreaming: false,
        supportsFunctions: false
      }
    ];
    
    for (const model of models) {
      await registry.registerModel(model);
    }
    
    const codeModels = await registry.getModelsByCapability('code');
    expect(codeModels.length).toBe(2);
    expect(codeModels[0].id).toBe('gpt-4');
    expect(codeModels[1].id).toBe('claude-3');
    
    const functionModels = await registry.getModelsByCapability('function');
    expect(functionModels.length).toBe(1);
    expect(functionModels[0].id).toBe('gpt-4');
  });
  
  runner.it('should update model profile', async () => {
    const model: ModelProfile = {
      id: 'gpt-4',
      provider: 'openai',
      name: 'GPT-4',
      capabilities: ['chat'],
      contextWindow: 8192,
      costPerToken: { input: 0.03, output: 0.06 },
      supportsStreaming: true,
      supportsFunctions: true
    };
    
    await registry.registerModel(model);
    
    const updated = {
      ...model,
      costPerToken: { input: 0.02, output: 0.04 },
      capabilities: ['chat', 'code']
    };
    
    await registry.updateModel(updated);
    
    const retrieved = await registry.getModel('gpt-4');
    expect(retrieved?.costPerToken.input).toBe(0.02);
    expect(retrieved?.capabilities).toContain('code');
  });
  
  runner.it('should remove model', async () => {
    const model: ModelProfile = {
      id: 'gpt-4',
      provider: 'openai',
      name: 'GPT-4',
      capabilities: ['chat'],
      contextWindow: 8192,
      costPerToken: { input: 0.03, output: 0.06 },
      supportsStreaming: true,
      supportsFunctions: true
    };
    
    await registry.registerModel(model);
    await registry.removeModel('gpt-4');
    
    const retrieved = await registry.getModel('gpt-4');
    expect(retrieved).toBeFalsy();
  });
  
  runner.it('should get cheapest model for capability', async () => {
    const models: ModelProfile[] = [
      {
        id: 'expensive',
        provider: 'provider1',
        name: 'Expensive Model',
        capabilities: ['chat'],
        contextWindow: 8192,
        costPerToken: { input: 0.1, output: 0.2 },
        supportsStreaming: true,
        supportsFunctions: true
      },
      {
        id: 'cheap',
        provider: 'provider2',
        name: 'Cheap Model',
        capabilities: ['chat'],
        contextWindow: 4096,
        costPerToken: { input: 0.001, output: 0.002 },
        supportsStreaming: true,
        supportsFunctions: false
      },
      {
        id: 'medium',
        provider: 'provider3',
        name: 'Medium Model',
        capabilities: ['chat'],
        contextWindow: 16384,
        costPerToken: { input: 0.01, output: 0.02 },
        supportsStreaming: true,
        supportsFunctions: true
      }
    ];
    
    for (const model of models) {
      await registry.registerModel(model);
    }
    
    const cheapest = await registry.getCheapestModel('chat');
    expect(cheapest).toBeTruthy();
    expect(cheapest?.id).toBe('cheap');
  });
});

// Export runner for execution
export default runner;