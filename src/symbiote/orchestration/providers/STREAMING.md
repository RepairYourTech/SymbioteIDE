# Flexible Streaming System

The SymbioteIDE orchestration system is designed to work seamlessly with both streaming and non-streaming AI models, providing a unified experience regardless of the underlying model capabilities.

## Key Features

### 1. **Automatic Detection**
The system automatically detects whether a model supports native streaming:

```typescript
protected doesModelSupportStreaming(model: ModelProfile): boolean {
  // Check model capabilities or hardcoded list
  return model.capabilities?.includes('streaming') || 
         KNOWN_STREAMING_MODELS.includes(model.name);
}
```

### 2. **Streaming Simulation**
For models that don't support streaming, the system can simulate it:

```typescript
// Request streaming even for non-streaming models
const result = await adapter.execute(task, model, {
  stream: true,
  simulateStreaming: true,  // Enable simulation
  chunkSize: 50,           // Characters per chunk
  chunkDelay: 30           // Milliseconds between chunks
});
```

### 3. **Unified Interface**
The same interface works for all scenarios:

```typescript
// Native streaming (e.g., GPT-4)
await execute(task, gpt4Model, { stream: true });

// Non-streaming (e.g., older models)
await execute(task, oldModel, { stream: false });

// Simulated streaming (e.g., local models)
await execute(task, localModel, { 
  stream: true, 
  simulateStreaming: true 
});
```

## How It Works

### Native Streaming Flow
1. Model supports streaming → Use provider's streaming API
2. Parse chunks as they arrive
3. Send via WebSocket in real-time
4. Aggregate for final result

### Simulated Streaming Flow
1. Model doesn't support streaming → Get complete response
2. Split response into chunks
3. Send chunks with artificial delays
4. Mimics streaming experience

### Non-Streaming Flow
1. Streaming disabled → Traditional request/response
2. Get complete response at once
3. Return immediately

## Benefits

### For Users
- **Consistent Experience**: Same UI behavior regardless of model
- **Perceived Performance**: Simulated streaming provides feedback faster
- **Progress Indication**: See output as it's generated (or simulated)

### For Developers
- **Single API**: One interface for all models
- **Easy Migration**: Switch between models without code changes
- **Graceful Degradation**: Streaming features work with any model

## Configuration

### Global Settings
```typescript
// In orchestration config
{
  streaming: {
    enabled: true,              // Global streaming toggle
    simulateForNonStreaming: true,  // Auto-simulate for non-streaming models
    defaultChunkSize: 100,      // Default chunk size for simulation
    defaultChunkDelay: 50       // Default delay between chunks
  }
}
```

### Per-Request Settings
```typescript
// Override for specific requests
await orchestrator.executeTask(task, {
  stream: true,
  simulateStreaming: false,  // Disable simulation for this request
  callbacks: {
    onProgress: (progress) => {
      // Handle streaming updates
    }
  }
});
```

## Model Examples

### Models with Native Streaming
- OpenAI: GPT-4, GPT-3.5-turbo
- Anthropic: Claude 3, Claude 2
- Google: Gemini Pro (with streaming enabled)

### Models without Native Streaming
- Many local models (Llama, Mistral via Ollama)
- Some API providers that don't support SSE
- Older model versions

### Automatic Handling
```typescript
// The system automatically handles both cases
const models = [
  { name: 'gpt-4', provider: 'openai' },        // ✓ Native streaming
  { name: 'llama-2-7b', provider: 'local' },    // ✗ Simulated streaming
  { name: 'claude-3', provider: 'anthropic' }   // ✓ Native streaming
];

// Same code works for all
for (const model of models) {
  await execute(task, model, { stream: true });
}
```

## WebSocket Integration

When streaming is enabled (native or simulated), the system automatically:

1. Creates a stream session
2. Sends chunks via WebSocket
3. Handles errors gracefully
4. Provides completion summary

```typescript
// Client receives consistent events
socket.on('stream:start', (data) => {
  console.log('Stream started:', data.streamId);
});

socket.on('stream:data', (chunk) => {
  console.log('Received:', chunk.data);
  // Same format for native and simulated
});

socket.on('stream:end', (summary) => {
  console.log('Stream complete:', summary);
});
```

## Performance Considerations

### Native Streaming
- **Pros**: True real-time, lower latency to first token
- **Cons**: Slightly more complex error handling

### Simulated Streaming
- **Pros**: Works with any model, predictable behavior
- **Cons**: No actual latency improvement, uses more CPU

### Optimization Tips
1. Use native streaming when available
2. Adjust chunk size based on content type
3. Consider network latency for chunk delays
4. Cache complete responses for replay

## Error Handling

The system handles errors gracefully in all modes:

```typescript
try {
  await execute(task, model, { stream: true });
} catch (error) {
  // Same error format for all modes
  if (error.code === 'STREAM_ERROR') {
    // Handle streaming-specific errors
  }
}
```

## Conclusion

The flexible streaming system ensures that SymbioteIDE provides a modern, responsive experience regardless of the underlying AI model capabilities. By abstracting the differences between streaming and non-streaming models, developers can focus on building features rather than handling model-specific quirks.