# Browser Integration

Chromium browser integration for SymbioteIDE with AI-powered control and automation.

## Features

- **Embedded Browser**: Full Chromium browser as VS Code tabs
- **AI Control**: Natural language browser commands
- **Element Selection**: Interactive element highlighting and inspection
- **Automated Testing**: AI-driven browser test automation
- **Pop-out Windows**: Open browser tabs in separate windows
- **Developer Tools**: Full DevTools integration

## Architecture

```
browser/
├── browser-service.ts          # Core Playwright browser management
├── browser-editor-provider.ts  # VS Code custom editor integration
├── ai-browser-controller.ts    # Natural language command processing
└── index.ts                   # Module entry point
```

## Usage

### Open Browser Tab

```typescript
// Command palette
"Open Browser"

// Programmatically
vscode.commands.executeCommand('symbiote.browser.open', 'https://example.com');
```

### AI Commands

Natural language commands are processed by the AI controller:

- "Go to Google and search for TypeScript"
- "Click the login button"
- "Fill in the email field with test@example.com"
- "Take a screenshot of the entire page"
- "Extract all links from the page"

### Element Selection

1. Click the element selector button in the browser toolbar
2. Hover over elements to highlight them
3. Click to select and view element details
4. Use AI chat to interact with selected elements

### Automated Testing

Create test scenarios in JSON:

```json
{
  "name": "Login Flow Test",
  "steps": [
    {
      "description": "Navigate to login page",
      "command": "Go to https://example.com/login"
    },
    {
      "description": "Fill in credentials",
      "command": "Type test@example.com in the email field and password123 in the password field",
      "assertions": [
        {
          "type": "equals",
          "target": "input[type='email']",
          "value": "test@example.com"
        }
      ]
    },
    {
      "description": "Submit form",
      "command": "Click the submit button",
      "assertions": [
        {
          "type": "contains",
          "target": "body",
          "value": "Welcome"
        }
      ]
    }
  ]
}
```

## API

### Browser Service

```typescript
// Initialize browser
await browserService.initialize(options);

// Create new tab
const tab = await browserService.createTab(url);

// Navigate
await browserService.navigate(tabId, url);

// Interact with page
await browserService.click(tabId, selector);
await browserService.type(tabId, selector, text);
await browserService.screenshot(tabId, options);

// Execute JavaScript
const result = await browserService.evaluate(tabId, script, ...args);
```

### AI Controller

```typescript
// Process natural language command
const result = await aiBrowserController.processCommand(
  "Click the search button",
  { url: currentUrl, title: pageTitle }
);

// Run test scenario
const testResult = await aiBrowserController.executeTestScenario(scenario);
```

## Configuration

Browser options can be configured:

```typescript
{
  headless: false,        // Show browser window
  devtools: true,         // Enable DevTools
  viewport: {             // Default viewport size
    width: 1280,
    height: 720
  },
  userDataDir: string     // Browser profile directory
}
```

## Integration Points

- **Orchestration Engine**: AI command processing
- **Task Manager**: Browser automation tasks
- **Code Review**: Visual regression testing
- **Knowledge Base**: Web content extraction

## Security

- Sandboxed iframe execution
- Request interception for monitoring
- Secure context isolation
- Content Security Policy enforcement