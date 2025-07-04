/**
 * AI Browser Controller - Natural language browser control
 */

import { browserService, BrowserTab } from './browser-service';
import { OrchestrationEngine } from '../orchestration/orchestration-engine';
import { Logger } from '../utils/logger';
import { ElementHandle } from 'playwright';

export interface BrowserCommand {
  action: 'navigate' | 'click' | 'type' | 'select' | 'scroll' | 'wait' | 'extract' | 'screenshot' | 'back' | 'forward' | 'reload';
  target?: string;
  value?: string;
  options?: any;
}

export interface BrowserContext {
  url: string;
  title: string;
  selectedElement?: {
    selector: string;
    info: any;
  };
  pageContent?: string;
  screenshot?: string;
}

export class AIBrowserController {
  private logger = new Logger('AIBrowserController');
  private orchestrationEngine: OrchestrationEngine;
  
  constructor(orchestrationEngine: OrchestrationEngine) {
    this.orchestrationEngine = orchestrationEngine;
  }
  
  /**
   * Process natural language browser command
   */
  async processCommand(
    input: string,
    context: BrowserContext,
    tabId?: string
  ): Promise<{
    response: string;
    actions?: any[];
    error?: string;
  }> {
    try {
      // Get active tab if not specified
      if (!tabId) {
        const activeTab = browserService.getActiveTab();
        if (!activeTab) {
          return {
            response: 'No active browser tab found. Please open a browser tab first.',
            error: 'No active tab'
          };
        }
        tabId = activeTab.id;
      }
      
      // Parse command using AI
      const commands = await this.parseCommand(input, context);
      
      // Execute commands
      const results = [];
      for (const command of commands) {
        const result = await this.executeCommand(command, tabId);
        results.push(result);
      }
      
      // Generate response
      const response = await this.generateResponse(input, commands, results, context);
      
      return {
        response,
        actions: this.generateUIActions(commands, results)
      };
      
    } catch (error) {
      this.logger.error('Failed to process browser command', error);
      return {
        response: `Failed to process command: ${error}`,
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }
  
  /**
   * Parse natural language into browser commands
   */
  private async parseCommand(input: string, context: BrowserContext): Promise<BrowserCommand[]> {
    const prompt = `
You are an AI browser automation assistant. Parse the user's natural language request into specific browser commands.

Context:
- Current URL: ${context.url}
- Page Title: ${context.title}
${context.selectedElement ? `- Selected Element: ${context.selectedElement.selector} (${context.selectedElement.info.tagName})` : ''}

User Request: "${input}"

Available Commands:
- navigate: Go to a URL (target: url)
- click: Click an element (target: selector)
- type: Type text into an input (target: selector, value: text)
- select: Select an option (target: selector, value: option)
- scroll: Scroll the page (options: { direction: 'up'|'down'|'top'|'bottom', amount?: number })
- wait: Wait for element or time (target: selector or options: { timeout: ms })
- extract: Extract data from page (target: selector, options: { attribute?: string })
- screenshot: Take a screenshot (options: { fullPage?: boolean, selector?: string })
- back: Go back in history
- forward: Go forward in history
- reload: Reload the page

Return a JSON array of commands. Be smart about selectors - use IDs when available, then classes, then other attributes.

Examples:
"Click the login button" -> [{"action": "click", "target": "#login-btn"}]
"Fill in my email as test@example.com" -> [{"action": "type", "target": "input[type='email']", "value": "test@example.com"}]
"Go to Google and search for AI" -> [
  {"action": "navigate", "target": "https://google.com"},
  {"action": "wait", "target": "input[name='q']"},
  {"action": "type", "target": "input[name='q']", "value": "AI"},
  {"action": "click", "target": "input[type='submit']"}
]

Provide reasoning for your command choices.`;
    
    const result = await this.orchestrationEngine.processQuery({
      prompt,
      temperature: 0.1,
      responseFormat: 'json'
    });
    
    try {
      const parsed = JSON.parse(result.response);
      return Array.isArray(parsed) ? parsed : [parsed];
    } catch (error) {
      this.logger.error('Failed to parse AI response', error);
      
      // Fallback to simple command extraction
      return this.extractSimpleCommands(input);
    }
  }
  
  /**
   * Simple command extraction fallback
   */
  private extractSimpleCommands(input: string): BrowserCommand[] {
    const commands: BrowserCommand[] = [];
    const lower = input.toLowerCase();
    
    // Navigation
    if (lower.includes('go to') || lower.includes('navigate to') || lower.includes('open')) {
      const urlMatch = input.match(/(?:go to|navigate to|open)\s+(\S+)/i);
      if (urlMatch) {
        let url = urlMatch[1];
        if (!url.startsWith('http')) {
          url = `https://${url}`;
        }
        commands.push({ action: 'navigate', target: url });
      }
    }
    
    // Click
    if (lower.includes('click')) {
      const targetMatch = input.match(/click\s+(?:on\s+)?(?:the\s+)?(.+?)(?:\s+button|\s+link|\s+element)?$/i);
      if (targetMatch) {
        const target = this.guessSelector(targetMatch[1]);
        commands.push({ action: 'click', target });
      }
    }
    
    // Type
    if (lower.includes('type') || lower.includes('fill') || lower.includes('enter')) {
      const match = input.match(/(?:type|fill|enter)\s+(?:in\s+)?["'](.+?)["']/i);
      if (match) {
        commands.push({ action: 'type', value: match[1] });
      }
    }
    
    // Screenshot
    if (lower.includes('screenshot') || lower.includes('capture')) {
      commands.push({ 
        action: 'screenshot',
        options: { fullPage: lower.includes('full') || lower.includes('entire') }
      });
    }
    
    return commands;
  }
  
  /**
   * Guess selector from natural language
   */
  private guessSelector(text: string): string {
    const lower = text.toLowerCase().trim();
    
    // Common button/link text
    const commonSelectors: Record<string, string[]> = {
      'login': ['#login', '.login', '[aria-label*="login"]', 'button:has-text("Login")', 'a:has-text("Login")'],
      'sign in': ['#signin', '.signin', '[aria-label*="sign in"]', 'button:has-text("Sign In")', 'a:has-text("Sign In")'],
      'submit': ['[type="submit"]', '#submit', '.submit', 'button:has-text("Submit")'],
      'search': ['[type="search"]', '#search', '.search', '[aria-label*="search"]', 'input[placeholder*="search"]'],
      'email': ['[type="email"]', '#email', '[name="email"]', 'input[placeholder*="email"]'],
      'password': ['[type="password"]', '#password', '[name="password"]', 'input[placeholder*="password"]'],
      'username': ['#username', '[name="username"]', 'input[placeholder*="username"]'],
    };
    
    // Check common selectors
    for (const [key, selectors] of Object.entries(commonSelectors)) {
      if (lower.includes(key)) {
        return selectors[0]; // Return first match
      }
    }
    
    // Generic selector based on text
    return `*:has-text("${text}")`;
  }
  
  /**
   * Execute browser command
   */
  private async executeCommand(command: BrowserCommand, tabId: string): Promise<any> {
    try {
      switch (command.action) {
        case 'navigate':
          if (!command.target) throw new Error('Navigate requires a target URL');
          await browserService.navigate(tabId, command.target);
          return { success: true, url: command.target };
          
        case 'click':
          if (!command.target) throw new Error('Click requires a target selector');
          await browserService.click(tabId, command.target);
          return { success: true, clicked: command.target };
          
        case 'type':
          if (!command.value) throw new Error('Type requires a value');
          if (command.target) {
            await browserService.type(tabId, command.target, command.value);
          } else {
            // Type in focused element
            await browserService.evaluate(tabId, (text) => {
              const active = document.activeElement as HTMLInputElement;
              if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) {
                active.value = text;
                active.dispatchEvent(new Event('input', { bubbles: true }));
              }
            }, command.value);
          }
          return { success: true, typed: command.value };
          
        case 'select':
          if (!command.target || !command.value) throw new Error('Select requires target and value');
          await browserService.select(tabId, command.target, command.value);
          return { success: true, selected: command.value };
          
        case 'scroll':
          const { direction = 'down', amount = 100 } = command.options || {};
          await browserService.evaluate(tabId, (dir, amt) => {
            if (dir === 'top') window.scrollTo(0, 0);
            else if (dir === 'bottom') window.scrollTo(0, document.body.scrollHeight);
            else if (dir === 'up') window.scrollBy(0, -amt);
            else window.scrollBy(0, amt);
          }, direction, amount);
          return { success: true, scrolled: direction };
          
        case 'wait':
          if (command.target) {
            await browserService.waitForSelector(tabId, command.target, command.options);
          } else if (command.options?.timeout) {
            await new Promise(resolve => setTimeout(resolve, command.options.timeout));
          }
          return { success: true, waited: command.target || `${command.options?.timeout}ms` };
          
        case 'extract':
          if (!command.target) throw new Error('Extract requires a target selector');
          const data = await browserService.evaluate(tabId, (selector, attribute) => {
            const elements = document.querySelectorAll(selector);
            return Array.from(elements).map((el: Element) => {
              if (attribute) {
                return el.getAttribute(attribute);
              }
              return {
                text: el.textContent?.trim(),
                html: el.outerHTML,
                attributes: Array.from(el.attributes).reduce((acc: Record<string, string>, attr: Attr) => {
                  acc[attr.name] = attr.value;
                  return acc;
                }, {} as Record<string, string>)
              };
            });
          }, command.target, command.options?.attribute);
          return { success: true, data };
          
        case 'screenshot':
          const screenshot = await browserService.screenshot(tabId, command.options);
          return { success: true, screenshot: screenshot.toString('base64') };
          
        case 'back':
          await browserService.goBack(tabId);
          return { success: true, action: 'back' };
          
        case 'forward':
          await browserService.goForward(tabId);
          return { success: true, action: 'forward' };
          
        case 'reload':
          await browserService.reload(tabId);
          return { success: true, action: 'reload' };
          
        default:
          throw new Error(`Unknown command action: ${command.action}`);
      }
    } catch (error) {
      this.logger.error(`Failed to execute command ${command.action}`, error);
      return { 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error',
        command 
      };
    }
  }
  
  /**
   * Generate natural language response
   */
  private async generateResponse(
    input: string,
    commands: BrowserCommand[],
    results: any[],
    context: BrowserContext
  ): Promise<string> {
    const successCount = results.filter(r => r.success).length;
    const failureCount = results.length - successCount;
    
    if (failureCount === 0) {
      // All commands successful
      const actions = commands.map((cmd, i) => {
        const result = results[i];
        switch (cmd.action) {
          case 'navigate':
            return `Navigated to ${result.url}`;
          case 'click':
            return `Clicked on ${cmd.target}`;
          case 'type':
            return `Typed "${cmd.value}"`;
          case 'screenshot':
            return 'Captured screenshot';
          case 'extract':
            return `Extracted ${result.data?.length || 0} elements`;
          default:
            return `Performed ${cmd.action}`;
        }
      });
      
      return `Successfully completed all actions:\n${actions.map((a, i) => `${i + 1}. ${a}`).join('\n')}`;
    } else if (successCount > 0) {
      // Partial success
      const errors = results
        .map((r, i) => r.success ? null : `${commands[i].action}: ${r.error}`)
        .filter(Boolean);
      
      return `Completed ${successCount} of ${commands.length} actions. Errors:\n${errors.join('\n')}`;
    } else {
      // All failed
      return `Failed to execute commands. Please check the selectors and try again.`;
    }
  }
  
  /**
   * Generate UI actions for webview
   */
  private generateUIActions(commands: BrowserCommand[], results: any[]): any[] {
    const actions: any[] = [];
    
    commands.forEach((cmd, i) => {
      const result = results[i];
      if (!result.success) return;
      
      switch (cmd.action) {
        case 'click':
        case 'type':
        case 'select':
          if (cmd.target) {
            actions.push({
              type: 'highlight',
              selectors: [cmd.target]
            });
          }
          break;
          
        case 'extract':
          if (result.data?.length > 0) {
            actions.push({
              type: 'code',
              code: JSON.stringify(result.data, null, 2),
              language: 'json'
            });
          }
          break;
          
        case 'screenshot':
          if (result.screenshot) {
            actions.push({
              type: 'image',
              data: result.screenshot,
              format: 'base64'
            });
          }
          break;
      }
    });
    
    return actions;
  }
  
  /**
   * Execute automated test scenario
   */
  async executeTestScenario(
    scenario: {
      name: string;
      steps: Array<{
        description: string;
        command: string;
        assertions?: Array<{
          type: 'exists' | 'contains' | 'equals';
          target: string;
          value?: string;
        }>;
      }>;
    },
    tabId?: string
  ): Promise<{
    success: boolean;
    results: any[];
    report: string;
  }> {
    const results: any[] = [];
    let allPassed = true;
    
    // Get or create tab
    if (!tabId) {
      const tab = await browserService.createTab();
      tabId = tab.id;
    }
    
    this.logger.info(`Executing test scenario: ${scenario.name}`);
    
    for (const step of scenario.steps) {
      this.logger.info(`Step: ${step.description}`);
      
      try {
        // Execute command
        const context: BrowserContext = {
          url: browserService.getActiveTab()?.url || '',
          title: browserService.getActiveTab()?.title || ''
        };
        
        const commandResult = await this.processCommand(step.command, context, tabId);
        
        // Run assertions if any
        const assertionResults: any[] = [];
        if (step.assertions) {
          for (const assertion of step.assertions) {
            const assertResult = await this.runAssertion(assertion, tabId);
            assertionResults.push(assertResult);
            if (!assertResult.passed) allPassed = false;
          }
        }
        
        results.push({
          step: step.description,
          command: step.command,
          commandResult,
          assertions: assertionResults
        });
        
      } catch (error) {
        allPassed = false;
        results.push({
          step: step.description,
          command: step.command,
          error: error instanceof Error ? error.message : 'Unknown error'
        });
      }
    }
    
    // Generate report
    const report = this.generateTestReport(scenario, results);
    
    return {
      success: allPassed,
      results,
      report
    };
  }
  
  /**
   * Run assertion
   */
  private async runAssertion(
    assertion: {
      type: 'exists' | 'contains' | 'equals';
      target: string;
      value?: string;
    },
    tabId: string
  ): Promise<{ passed: boolean; message: string }> {
    try {
      switch (assertion.type) {
        case 'exists':
          const exists = await browserService.querySelector(tabId, assertion.target);
          return {
            passed: exists !== null,
            message: exists ? `Element ${assertion.target} exists` : `Element ${assertion.target} not found`
          };
          
        case 'contains':
          if (!assertion.value) throw new Error('Contains assertion requires a value');
          const text = await browserService.evaluate(tabId, (selector) => {
            const el = document.querySelector(selector);
            return el?.textContent || '';
          }, assertion.target);
          
          const contains = text.includes(assertion.value);
          return {
            passed: contains,
            message: contains 
              ? `Element contains "${assertion.value}"`
              : `Element does not contain "${assertion.value}" (found: "${text}")`
          };
          
        case 'equals':
          if (!assertion.value) throw new Error('Equals assertion requires a value');
          const actualValue = await browserService.evaluate(tabId, (selector) => {
            const el = document.querySelector(selector) as HTMLInputElement;
            return el?.value || el?.textContent || '';
          }, assertion.target);
          
          const equals = actualValue === assertion.value;
          return {
            passed: equals,
            message: equals
              ? `Value equals "${assertion.value}"`
              : `Value does not equal "${assertion.value}" (found: "${actualValue}")`
          };
          
        default:
          throw new Error(`Unknown assertion type: ${assertion.type}`);
      }
    } catch (error) {
      return {
        passed: false,
        message: `Assertion failed: ${error}`
      };
    }
  }
  
  /**
   * Generate test report
   */
  private generateTestReport(scenario: any, results: any[]): string {
    const passed = results.filter(r => !r.error && r.assertions?.every((a: any) => a.passed)).length;
    const failed = results.length - passed;
    
    let report = `# Test Report: ${scenario.name}\n\n`;
    report += `**Status:** ${failed === 0 ? '✅ PASSED' : '❌ FAILED'}\n`;
    report += `**Passed:** ${passed}/${results.length}\n\n`;
    
    report += `## Steps:\n\n`;
    
    results.forEach((result, index) => {
      const stepNum = index + 1;
      const status = result.error ? '❌' : 
                     (result.assertions?.every((a: any) => a.passed) ? '✅' : '⚠️');
      
      report += `### ${stepNum}. ${status} ${result.step}\n`;
      report += `**Command:** ${result.command}\n`;
      
      if (result.error) {
        report += `**Error:** ${result.error}\n`;
      } else if (result.assertions?.length > 0) {
        report += `**Assertions:**\n`;
        result.assertions.forEach((assertion: any) => {
          report += `- ${assertion.passed ? '✅' : '❌'} ${assertion.message}\n`;
        });
      }
      
      report += '\n';
    });
    
    return report;
  }
}

// Export singleton instance
export const aiBrowserController = new AIBrowserController(new OrchestrationEngine());