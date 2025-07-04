/**
 * Browser Integration Module
 */

import * as vscode from 'vscode';
import { browserService } from './browser-service';
import { registerBrowserEditor } from './browser-editor-provider';
import { aiBrowserController } from './ai-browser-controller';
import { Logger } from '../utils/logger';

const logger = new Logger('BrowserIntegration');

export interface BrowserIntegration {
  service: typeof browserService;
  aiController: typeof aiBrowserController;
}

/**
 * Activate browser integration
 */
export async function activate(context: vscode.ExtensionContext): Promise<BrowserIntegration> {
  logger.info('Activating browser integration');
  
  // Register custom editor
  context.subscriptions.push(
    registerBrowserEditor(context)
  );
  
  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('symbiote.browser.open', async (url?: string) => {
      try {
        // Initialize browser if needed
        if (!browserService.isRunning()) {
          await browserService.initialize();
        }
        
        // Create tab
        const tab = await browserService.createTab(url);
        
        // Open in editor
        const uri = vscode.Uri.parse(`browser:${tab.url}`);
        await vscode.commands.executeCommand('vscode.openWith', uri, 'symbiote.browser');
        
      } catch (error) {
        logger.error('Failed to open browser', error);
        vscode.window.showErrorMessage(`Failed to open browser: ${error}`);
      }
    })
  );
  
  context.subscriptions.push(
    vscode.commands.registerCommand('symbiote.browser.openCurrent', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      
      // Try to extract URL from current document
      const text = editor.document.getText();
      const urlMatch = text.match(/https?:\/\/[^\s]+/);
      
      if (urlMatch) {
        await vscode.commands.executeCommand('symbiote.browser.open', urlMatch[0]);
      } else {
        vscode.window.showErrorMessage('No URL found in current document');
      }
    })
  );
  
  context.subscriptions.push(
    vscode.commands.registerCommand('symbiote.browser.aiCommand', async () => {
      // Get user input
      const input = await vscode.window.showInputBox({
        prompt: 'Enter browser command (e.g., "Go to Google and search for TypeScript")',
        placeHolder: 'Natural language browser command...'
      });
      
      if (!input) return;
      
      try {
        const tab = browserService.getActiveTab();
        if (!tab) {
          vscode.window.showErrorMessage('No active browser tab');
          return;
        }
        
        const context = {
          url: tab.url,
          title: tab.title
        };
        
        const result = await aiBrowserController.processCommand(input, context);
        
        if (result.error) {
          vscode.window.showErrorMessage(result.error);
        } else {
          vscode.window.showInformationMessage(result.response);
        }
        
      } catch (error) {
        logger.error('Failed to process AI command', error);
        vscode.window.showErrorMessage(`Command failed: ${error}`);
      }
    })
  );
  
  context.subscriptions.push(
    vscode.commands.registerCommand('symbiote.browser.runTest', async () => {
      // Get test file
      const testFiles = await vscode.window.showOpenDialog({
        canSelectFiles: true,
        canSelectFolders: false,
        canSelectMany: false,
        filters: {
          'Test Files': ['json', 'yaml', 'yml']
        }
      });
      
      if (!testFiles || testFiles.length === 0) return;
      
      try {
        // Read test scenario
        const content = await vscode.workspace.fs.readFile(testFiles[0]);
        const scenario = JSON.parse(content.toString());
        
        // Run test
        const result = await aiBrowserController.executeTestScenario(scenario);
        
        // Show report
        const doc = await vscode.workspace.openTextDocument({
          content: result.report,
          language: 'markdown'
        });
        
        await vscode.window.showTextDocument(doc);
        
      } catch (error) {
        logger.error('Failed to run test', error);
        vscode.window.showErrorMessage(`Test failed: ${error}`);
      }
    })
  );
  
  // Register status bar item
  const statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  
  statusBarItem.text = '$(browser) Browser';
  statusBarItem.tooltip = 'Open browser';
  statusBarItem.command = 'symbiote.browser.open';
  statusBarItem.show();
  
  context.subscriptions.push(statusBarItem);
  
  // Cleanup on deactivate
  context.subscriptions.push({
    dispose: async () => {
      await browserService.close();
    }
  });
  
  logger.info('Browser integration activated');
  
  return {
    service: browserService,
    aiController: aiBrowserController
  };
}

// Export types and services
export { browserService, BrowserTab, BrowserOptions } from './browser-service';
export { aiBrowserController, BrowserCommand, BrowserContext } from './ai-browser-controller';
export { BrowserEditorProvider } from './browser-editor-provider';