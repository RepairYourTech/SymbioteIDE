/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

// Mock VS Code API for testing
global.vscode = {
  window: {
    showInformationMessage: jest.fn(),
    showErrorMessage: jest.fn(),
    showWarningMessage: jest.fn(),
    createOutputChannel: jest.fn(() => ({
      appendLine: jest.fn(),
      append: jest.fn(),
      clear: jest.fn(),
      dispose: jest.fn(),
    })),
  },
  workspace: {
    getConfiguration: jest.fn(() => ({
      get: jest.fn(),
      has: jest.fn(),
      inspect: jest.fn(),
      update: jest.fn(),
    })),
    workspaceFolders: [],
  },
  ExtensionContext: jest.fn(),
  Uri: {
    file: jest.fn((path) => ({ fsPath: path })),
    parse: jest.fn(),
  },
  EventEmitter: jest.fn(() => ({
    fire: jest.fn(),
    event: jest.fn(),
    dispose: jest.fn(),
  })),
} as any;

// Set test environment variables
process.env.NODE_ENV = 'test';
process.env.SYMBIOTE_TEST = '1';

// Increase timeout for async operations
jest.setTimeout(10000);

// Global test utilities
global.testUtils = {
  delay: (ms: number) => new Promise((resolve) => setTimeout(resolve, ms)),
  
  mockApiResponse: (data: any, delay = 0) => {
    return jest.fn().mockImplementation(() => 
      global.testUtils.delay(delay).then(() => data)
    );
  },
  
  createMockAgent: (overrides = {}) => ({
    id: 'test-agent-1',
    name: 'Test Agent',
    type: 'development',
    status: 'idle',
    createdAt: new Date(),
    capabilities: ['code-generation', 'testing'],
    metadata: {},
    ...overrides,
  }),
  
  createMockTask: (overrides = {}) => ({
    id: 'test-task-1',
    type: 'code-generation',
    prompt: 'Test prompt',
    context: {},
    constraints: {},
    priority: 'normal',
    metadata: {},
    ...overrides,
  }),
};

// Clean up after each test
afterEach(() => {
  jest.clearAllMocks();
});