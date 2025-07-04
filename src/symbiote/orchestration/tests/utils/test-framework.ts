/**
 * Simple Test Framework (without external dependencies)
 */

interface TestCase {
  name: string;
  fn: () => void | Promise<void>;
  timeout?: number;
}

interface TestSuite {
  name: string;
  tests: TestCase[];
  beforeEach?: () => void | Promise<void>;
  afterEach?: () => void | Promise<void>;
  beforeAll?: () => void | Promise<void>;
  afterAll?: () => void | Promise<void>;
}

export class TestRunner {
  private suites: TestSuite[] = [];
  private results: Map<string, { passed: number; failed: number; errors: any[] }> = new Map();

  describe(name: string, fn: () => void): void {
    const suite: TestSuite = {
      name,
      tests: []
    };
    
    // Set current suite context
    const previousSuite = this.currentSuite;
    this.currentSuite = suite;
    
    // Execute suite definition
    fn();
    
    // Add suite and restore context
    this.suites.push(suite);
    this.currentSuite = previousSuite;
  }

  private currentSuite?: TestSuite;

  it(name: string, fn: () => void | Promise<void>, timeout?: number): void {
    if (!this.currentSuite) {
      throw new Error('it() must be called within describe()');
    }
    
    this.currentSuite.tests.push({ name, fn, timeout });
  }

  beforeEach(fn: () => void | Promise<void>): void {
    if (!this.currentSuite) {
      throw new Error('beforeEach() must be called within describe()');
    }
    this.currentSuite.beforeEach = fn;
  }

  afterEach(fn: () => void | Promise<void>): void {
    if (!this.currentSuite) {
      throw new Error('afterEach() must be called within describe()');
    }
    this.currentSuite.afterEach = fn;
  }

  beforeAll(fn: () => void | Promise<void>): void {
    if (!this.currentSuite) {
      throw new Error('beforeAll() must be called within describe()');
    }
    this.currentSuite.beforeAll = fn;
  }

  afterAll(fn: () => void | Promise<void>): void {
    if (!this.currentSuite) {
      throw new Error('afterAll() must be called within describe()');
    }
    this.currentSuite.afterAll = fn;
  }

  async run(): Promise<void> {
    console.log('\n🧪 Running tests...\n');
    
    for (const suite of this.suites) {
      console.log(`\n📦 ${suite.name}`);
      
      const suiteResult = { passed: 0, failed: 0, errors: [] as any[] };
      
      // Run beforeAll
      if (suite.beforeAll) {
        try {
          await suite.beforeAll();
        } catch (error) {
          console.error(`  ❌ beforeAll failed: ${error}`);
          continue;
        }
      }
      
      // Run tests
      for (const test of suite.tests) {
        try {
          // Run beforeEach
          if (suite.beforeEach) {
            await suite.beforeEach();
          }
          
          // Run test with timeout
          await this.runWithTimeout(test.fn, test.timeout || 5000);
          
          console.log(`  ✅ ${test.name}`);
          suiteResult.passed++;
          
        } catch (error) {
          console.log(`  ❌ ${test.name}`);
          console.error(`     ${error}`);
          suiteResult.failed++;
          suiteResult.errors.push({ test: test.name, error });
        } finally {
          // Run afterEach
          if (suite.afterEach) {
            try {
              await suite.afterEach();
            } catch (error) {
              console.error(`     afterEach failed: ${error}`);
            }
          }
        }
      }
      
      // Run afterAll
      if (suite.afterAll) {
        try {
          await suite.afterAll();
        } catch (error) {
          console.error(`  ❌ afterAll failed: ${error}`);
        }
      }
      
      this.results.set(suite.name, suiteResult);
    }
    
    this.printSummary();
  }

  private async runWithTimeout(fn: () => void | Promise<void>, timeout: number): Promise<void> {
    return new Promise(async (resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new Error(`Test timed out after ${timeout}ms`));
      }, timeout);
      
      try {
        await fn();
        clearTimeout(timer);
        resolve();
      } catch (error) {
        clearTimeout(timer);
        reject(error);
      }
    });
  }

  private printSummary(): void {
    console.log('\n\n📊 Test Summary\n');
    
    let totalPassed = 0;
    let totalFailed = 0;
    
    for (const [suiteName, result] of this.results) {
      totalPassed += result.passed;
      totalFailed += result.failed;
      
      const status = result.failed === 0 ? '✅' : '❌';
      console.log(`${status} ${suiteName}: ${result.passed} passed, ${result.failed} failed`);
    }
    
    console.log('\n-------------------');
    console.log(`Total: ${totalPassed} passed, ${totalFailed} failed`);
    console.log(`Status: ${totalFailed === 0 ? '✅ All tests passed!' : '❌ Some tests failed'}`);
  }
}

// Assertion utilities
export const expect = (actual: any) => ({
  toBe(expected: any): void {
    if (actual !== expected) {
      throw new Error(`Expected ${actual} to be ${expected}`);
    }
  },
  
  toEqual(expected: any): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      throw new Error(`Expected ${JSON.stringify(actual)} to equal ${JSON.stringify(expected)}`);
    }
  },
  
  toBeTruthy(): void {
    if (!actual) {
      throw new Error(`Expected ${actual} to be truthy`);
    }
  },
  
  toBeFalsy(): void {
    if (actual) {
      throw new Error(`Expected ${actual} to be falsy`);
    }
  },
  
  toContain(expected: any): void {
    if (Array.isArray(actual) || typeof actual === 'string') {
      if (!actual.includes(expected)) {
        throw new Error(`Expected ${actual} to contain ${expected}`);
      }
    } else {
      throw new Error('toContain can only be used with arrays or strings');
    }
  },
  
  toThrow(expectedError?: string | RegExp): void {
    if (typeof actual !== 'function') {
      throw new Error('toThrow can only be used with functions');
    }
    
    let threw = false;
    let error: any;
    
    try {
      actual();
    } catch (e) {
      threw = true;
      error = e;
    }
    
    if (!threw) {
      throw new Error('Expected function to throw');
    }
    
    if (expectedError) {
      const message = error?.message || String(error);
      if (typeof expectedError === 'string' && !message.includes(expectedError)) {
        throw new Error(`Expected error to contain "${expectedError}" but got "${message}"`);
      } else if (expectedError instanceof RegExp && !expectedError.test(message)) {
        throw new Error(`Expected error to match ${expectedError} but got "${message}"`);
      }
    }
  },
  
  toBeGreaterThan(expected: number): void {
    if (typeof actual !== 'number' || typeof expected !== 'number') {
      throw new Error('toBeGreaterThan can only be used with numbers');
    }
    if (actual <= expected) {
      throw new Error(`Expected ${actual} to be greater than ${expected}`);
    }
  },
  
  toBeLessThan(expected: number): void {
    if (typeof actual !== 'number' || typeof expected !== 'number') {
      throw new Error('toBeLessThan can only be used with numbers');
    }
    if (actual >= expected) {
      throw new Error(`Expected ${actual} to be less than ${expected}`);
    }
  },
  
  toHaveBeenCalled(): void {
    if (!actual || !actual.calls) {
      throw new Error('toHaveBeenCalled can only be used with mock functions');
    }
    if (actual.calls.length === 0) {
      throw new Error('Expected function to have been called');
    }
  },
  
  toHaveBeenCalledWith(...args: any[]): void {
    if (!actual || !actual.calls) {
      throw new Error('toHaveBeenCalledWith can only be used with mock functions');
    }
    
    const found = actual.calls.some((call: any[]) => 
      JSON.stringify(call) === JSON.stringify(args)
    );
    
    if (!found) {
      throw new Error(`Expected function to have been called with ${JSON.stringify(args)}`);
    }
  }
});

// Mock function creator
export function createMockFunction(): any {
  const calls: any[][] = [];
  const mockFn = (...args: any[]) => {
    calls.push(args);
    return mockFn.mockReturnValue;
  };
  
  mockFn.calls = calls;
  mockFn.mockReturnValue = undefined;
  mockFn.mockReturnValueOnce = (value: any) => {
    const originalValue = mockFn.mockReturnValue;
    mockFn.mockReturnValue = value;
    setTimeout(() => { mockFn.mockReturnValue = originalValue; }, 0);
    return mockFn;
  };
  mockFn.mockImplementation = (impl: Function) => {
    const originalFn = mockFn;
    return (...args: any[]) => {
      originalFn(...args);
      return impl(...args);
    };
  };
  
  return mockFn;
}