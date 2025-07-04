# Orchestration Engine Test Suite

Comprehensive testing framework for the SymbioteIDE Multi-Model Orchestration Engine.

## Test Structure

```
tests/
├── unit/                 # Unit tests for individual components
├── integration/          # Integration tests for component interactions
├── e2e/                 # End-to-end tests for complete workflows
├── load/                # Load and performance testing
├── chaos/               # Chaos engineering tests
├── fixtures/            # Test data and mocks
├── utils/               # Test utilities and helpers
├── test-config.ts       # Test configuration
└── run-tests.ts         # Main test runner
```

## Running Tests

```bash
# Run all tests
npm test

# Run specific test suites
npm test -- --unit
npm test -- --integration
npm test -- --e2e
npm test -- --load
npm test -- --chaos

# Run multiple suites
npm test -- --unit --integration
```

## Test Types

### Unit Tests
- Model Registry
- Task Analyzer
- Routing Engine
- Cost Tracker
- Cache Manager
- Queue Management
- Configuration Management

### Integration Tests
- Orchestration Engine workflows
- Multi-model coordination
- Queue and routing integration
- API layer integration

### End-to-End Tests
- Complete task execution flows
- API request/response cycles
- WebSocket streaming
- Error handling scenarios

### Load Tests
- Steady load scenarios
- Spike testing
- Sustained load testing
- Throughput benchmarking
- Latency analysis

### Chaos Tests
- Network delays
- Provider failures
- Memory pressure
- CPU spikes
- Rate limiting
- Invalid responses
- Timeouts
- Partial failures

## Test Configuration

Configure test behavior in `test-config.ts`:

```typescript
{
  timeouts: {
    unit: 5000,
    integration: 30000,
    e2e: 60000,
    chaos: 120000,
    load: 300000
  },
  load: {
    users: { min: 10, max: 1000 },
    duration: 300,
    thresholds: {
      p95ResponseTime: 1000,
      errorRate: 0.01,
      throughput: 100
    }
  },
  chaos: {
    scenarios: [...],
    probability: 0.1,
    duration: 60
  },
  coverage: {
    statements: 90,
    branches: 85,
    functions: 90,
    lines: 90
  }
}
```

## Writing Tests

### Unit Test Example

```typescript
runner.describe('ComponentName', () => {
  let component: Component;
  
  runner.beforeEach(() => {
    component = new Component();
  });
  
  runner.it('should perform expected behavior', async () => {
    const result = await component.method();
    expect(result).toBe(expectedValue);
  });
});
```

### Integration Test Example

```typescript
runner.it('should integrate components correctly', async () => {
  const engine = new OrchestrationEngine(config);
  const task = createMockTask();
  
  const result = await engine.executeTask(task);
  
  expect(result.status).toBe('completed');
  expect(result.model).toBeTruthy();
});
```

## Test Utilities

### Mock Helpers
- `createMockEngine()` - Mock orchestration engine
- `createMockModel()` - Mock model profile
- `createMockTask()` - Mock AI task
- `createMockResult()` - Mock task result

### Assertion Helpers
- `expect(value).toBe(expected)`
- `expect(value).toEqual(expected)`
- `expect(value).toBeTruthy()`
- `expect(value).toContain(item)`
- `expect(fn).toThrow(error)`

### Async Helpers
- `waitFor(condition, timeout)` - Wait for condition
- `delay(ms)` - Create delay
- `measureTime(fn)` - Measure execution time

## Performance Benchmarks

Target performance metrics:

- **Latency**: P95 < 1000ms, P99 < 2000ms
- **Throughput**: > 100 requests/second
- **Error Rate**: < 1%
- **Memory**: < 512MB under load
- **CPU**: < 50% under normal load

## Coverage Requirements

Minimum coverage thresholds:

- Statements: 90%
- Branches: 85%
- Functions: 90%
- Lines: 90%

## Continuous Integration

Tests are automatically run on:
- Every push to main branch
- Pull request creation/update
- Nightly builds
- Release candidates

## Debugging Tests

```bash
# Run with verbose output
DEBUG=* npm test

# Run specific test file
npm test -- --file=model-registry.test.ts

# Run with coverage
npm test -- --coverage
```