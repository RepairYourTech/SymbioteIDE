/**
 * Unified Context VS Code Extension
 * 
 * Provides VS Code commands and UI for the Unified Context API
 */

import * as vscode from 'vscode';
import { UnifiedContextAPI } from '../unified-context-api';

export class UnifiedContextExtension {
  private unifiedContext?: UnifiedContextAPI;
  private statusBarItem: vscode.StatusBarItem;
  
  constructor(private context: vscode.ExtensionContext) {
    // Create status bar item
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100
    );
  }
  
  /**
   * Activate the extension
   */
  async activate(unifiedContext: UnifiedContextAPI): Promise<void> {
    this.unifiedContext = unifiedContext;
    
    // Register commands
    this.registerCommands();
    
    // Setup status bar
    this.setupStatusBar();
    
    // Listen to unified context events
    this.setupEventListeners();
  }
  
  /**
   * Register all commands
   */
  private registerCommands(): void {
    // Search command
    this.context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.unifiedContext.search',
        () => this.showSearchDialog()
      )
    );
    
    // Generate documentation
    this.context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.unifiedContext.generateDocs',
        () => this.generateDocumentation()
      )
    );
    
    // Generate migration guide
    this.context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.unifiedContext.migrationGuide',
        () => this.generateMigrationGuide()
      )
    );
    
    // Export knowledge
    this.context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.unifiedContext.export',
        () => this.exportKnowledge()
      )
    );
    
    // Find similar code
    this.context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.unifiedContext.findSimilar',
        () => this.findSimilarCode()
      )
    );
    
    // Learn from outcome
    this.context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.unifiedContext.learnOutcome',
        () => this.learnFromOutcome()
      )
    );
    
    // View team knowledge
    this.context.subscriptions.push(
      vscode.commands.registerCommand(
        'symbiote.unifiedContext.teamKnowledge',
        () => this.viewTeamKnowledge()
      )
    );
  }
  
  /**
   * Show search dialog
   */
  private async showSearchDialog(): Promise<void> {
    if (!this.unifiedContext) return;
    
    const query = await vscode.window.showInputBox({
      prompt: 'Search across code, memories, and knowledge graph',
      placeHolder: 'e.g., authentication, error handling, API endpoints'
    });
    
    if (!query) return;
    
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Searching unified context...',
      cancellable: false
    }, async () => {
      const results = await this.unifiedContext!.search({
        query,
        includeCode: true,
        includeMemories: true,
        includeGraph: true,
        limit: 20
      });
      
      await this.showSearchResults(query, results);
    });
  }
  
  /**
   * Generate documentation
   */
  private async generateDocumentation(): Promise<void> {
    if (!this.unifiedContext) return;
    
    const scope = await vscode.window.showQuickPick([
      { label: 'Current File', value: 'file' },
      { label: 'Current Folder', value: 'folder' },
      { label: 'Entire Workspace', value: 'workspace' }
    ], {
      placeHolder: 'Select documentation scope'
    });
    
    if (!scope) return;
    
    let target: string | undefined;
    
    if (scope.value === 'file') {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showErrorMessage('No active file');
        return;
      }
      target = editor.document.uri.fsPath;
    } else if (scope.value === 'folder') {
      const uri = await vscode.window.showOpenDialog({
        canSelectFolders: true,
        canSelectFiles: false,
        canSelectMany: false
      });
      if (!uri || uri.length === 0) return;
      target = uri[0].fsPath;
    }
    
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Generating documentation...',
      cancellable: false
    }, async () => {
      const result = await this.unifiedContext!.generateDocumentation(
        scope.value as any,
        target
      );
      
      // Show in new document
      const doc = await vscode.workspace.openTextDocument({
        content: result.content,
        language: 'markdown'
      });
      
      await vscode.window.showTextDocument(doc);
    });
  }
  
  /**
   * Generate migration guide
   */
  private async generateMigrationGuide(): Promise<void> {
    if (!this.unifiedContext) return;
    
    const fromVersion = await vscode.window.showInputBox({
      prompt: 'Enter the source version/framework',
      placeHolder: 'e.g., React 17, Angular 12'
    });
    
    if (!fromVersion) return;
    
    const toVersion = await vscode.window.showInputBox({
      prompt: 'Enter the target version/framework',
      placeHolder: 'e.g., React 18, Vue 3'
    });
    
    if (!toVersion) return;
    
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Generating migration guide...',
      cancellable: false
    }, async () => {
      const guide = await this.unifiedContext!.generateMigrationGuide(
        fromVersion,
        toVersion
      );
      
      // Show in new document
      const content = [
        `# Migration Guide: ${fromVersion} → ${toVersion}`,
        '',
        guide.content,
        '',
        '## Steps',
        ...guide.steps.map((s, i) => `${i + 1}. ${s}`),
        '',
        '## Breaking Changes',
        ...guide.breakingChanges.map(c => `- ${c}`),
        '',
        '## Recommendations',
        ...guide.recommendations.map(r => `- ${r}`)
      ].join('\n');
      
      const doc = await vscode.workspace.openTextDocument({
        content,
        language: 'markdown'
      });
      
      await vscode.window.showTextDocument(doc);
    });
  }
  
  /**
   * Export knowledge
   */
  private async exportKnowledge(): Promise<void> {
    if (!this.unifiedContext) return;
    
    const format = await vscode.window.showQuickPick([
      { label: 'Markdown', value: 'markdown' },
      { label: 'HTML', value: 'html' },
      { label: 'JSON', value: 'json' }
    ], {
      placeHolder: 'Select export format'
    });
    
    if (!format) return;
    
    const scope = await vscode.window.showQuickPick([
      { label: 'Project Knowledge', value: 'project' },
      { label: 'Team Knowledge', value: 'team' },
      { label: 'All Knowledge', value: 'all' }
    ], {
      placeHolder: 'Select export scope'
    });
    
    if (!scope) return;
    
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Exporting knowledge...',
      cancellable: false
    }, async () => {
      const content = await this.unifiedContext!.exportKnowledge(
        format.value as any,
        {
          includeCode: true,
          includeMemories: true,
          includeGraph: true,
          scope: scope.value as any
        }
      );
      
      // Save to file
      const defaultName = `knowledge-export-${new Date().toISOString().split('T')[0]}.${format.value}`;
      const uri = await vscode.window.showSaveDialog({
        defaultUri: vscode.Uri.file(defaultName),
        filters: {
          'Markdown': ['md'],
          'HTML': ['html'],
          'JSON': ['json']
        }
      });
      
      if (uri) {
        await vscode.workspace.fs.writeFile(uri, Buffer.from(content));
        vscode.window.showInformationMessage('Knowledge exported successfully');
      }
    });
  }
  
  /**
   * Find similar code
   */
  private async findSimilarCode(): Promise<void> {
    if (!this.unifiedContext) return;
    
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
      vscode.window.showErrorMessage('No active editor');
      return;
    }
    
    const selection = editor.selection;
    const code = editor.document.getText(selection);
    
    if (!code) {
      vscode.window.showErrorMessage('No code selected');
      return;
    }
    
    await vscode.window.withProgress({
      location: vscode.ProgressLocation.Notification,
      title: 'Finding similar code...',
      cancellable: false
    }, async () => {
      const context = await this.unifiedContext!.findSimilarImplementations(
        code,
        editor.document.languageId
      );
      
      await this.showCodeContext('Similar Implementations', context);
    });
  }
  
  /**
   * Learn from task outcome
   */
  private async learnFromOutcome(): Promise<void> {
    if (!this.unifiedContext) return;
    
    const taskDescription = await vscode.window.showInputBox({
      prompt: 'What task did you just complete?',
      placeHolder: 'e.g., Implemented user authentication'
    });
    
    if (!taskDescription) return;
    
    const outcome = await vscode.window.showQuickPick([
      { label: '✅ Success', value: 'success' },
      { label: '❌ Failed', value: 'failure' }
    ], {
      placeHolder: 'What was the outcome?'
    });
    
    if (!outcome) return;
    
    let solution: string | undefined;
    let error: string | undefined;
    
    if (outcome.value === 'success') {
      solution = await vscode.window.showInputBox({
        prompt: 'Briefly describe the solution',
        placeHolder: 'e.g., Used JWT tokens with refresh mechanism'
      });
    } else {
      error = await vscode.window.showInputBox({
        prompt: 'What went wrong?',
        placeHolder: 'e.g., CORS issues with third-party API'
      });
    }
    
    const learnings = await this.unifiedContext.learn({
      task: {
        taskId: `task_${Date.now()}`,
        description: taskDescription,
        type: 'implementation'
      },
      outcome: outcome.value as any,
      solution,
      error,
      duration: 0
    });
    
    vscode.window.showInformationMessage(
      `Learned ${learnings.length} insights from this task`
    );
  }
  
  /**
   * View team knowledge
   */
  private async viewTeamKnowledge(): Promise<void> {
    if (!this.unifiedContext) return;
    
    const knowledge = await this.unifiedContext.getTeamKnowledge();
    
    const content = [
      '# Team Knowledge',
      '',
      '## Shared Learnings',
      ...knowledge.sharedLearnings.map(l => `- ${l.insight}`),
      '',
      '## Common Patterns',
      ...knowledge.commonPatterns.map(p => `- **${p.name}**: ${p.description}`),
      '',
      '## Best Practices',
      ...knowledge.bestPractices.map(bp => `- ${bp.practice}`),
      '',
      '## Known Issues',
      ...knowledge.knownIssues.map(i => `- **${i.issue}**: ${i.solution || 'No solution yet'}`)
    ].join('\n');
    
    const doc = await vscode.workspace.openTextDocument({
      content,
      language: 'markdown'
    });
    
    await vscode.window.showTextDocument(doc);
  }
  
  /**
   * Setup status bar
   */
  private setupStatusBar(): void {
    this.statusBarItem.text = '$(search) Unified Context';
    this.statusBarItem.tooltip = 'Search across code, memories, and knowledge';
    this.statusBarItem.command = 'symbiote.unifiedContext.search';
    this.statusBarItem.show();
    
    this.context.subscriptions.push(this.statusBarItem);
  }
  
  /**
   * Setup event listeners
   */
  private setupEventListeners(): void {
    if (!this.unifiedContext) return;
    
    this.unifiedContext.on('search-complete', (data) => {
      this.statusBarItem.text = `$(search) UC (${data.result.metadata.sources.join(', ')})`;
    });
    
    this.unifiedContext.on('learning-stored', (data) => {
      this.statusBarItem.text = '$(lightbulb) Learning stored';
      setTimeout(() => {
        this.statusBarItem.text = '$(search) Unified Context';
      }, 3000);
    });
  }
  
  /**
   * Show search results
   */
  private async showSearchResults(query: string, results: any): Promise<void> {
    const items: vscode.QuickPickItem[] = [];
    
    // Add summary
    items.push({
      label: '$(info) Summary',
      description: results.summary.substring(0, 100),
      detail: `Relevance: ${Math.round(results.relevanceScore * 100)}%`
    });
    
    // Add code results
    if (results.codeContext) {
      items.push({
        label: '$(symbol-file) Code Results',
        kind: vscode.QuickPickItemKind.Separator
      });
      
      for (const item of results.codeContext.primary.slice(0, 5)) {
        items.push({
          label: `$(symbol-${item.entity.type.toLowerCase()}) ${item.entity.name}`,
          description: item.entity.filePath,
          detail: item.explanation
        });
      }
    }
    
    // Add memory results
    if (results.memories && results.memories.length > 0) {
      items.push({
        label: '$(history) Memory Results',
        kind: vscode.QuickPickItemKind.Separator
      });
      
      for (const mem of results.memories.slice(0, 5)) {
        items.push({
          label: `$(bookmark) ${mem.memory.type}`,
          description: mem.memory.content.substring(0, 60),
          detail: `Confidence: ${Math.round(mem.memory.metadata.confidence * 100)}%`
        });
      }
    }
    
    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: `Results for "${query}"`
    });
    
    // Handle selection if needed
  }
  
  /**
   * Show code context
   */
  private async showCodeContext(title: string, context: any): Promise<void> {
    const items: vscode.QuickPickItem[] = [];
    
    items.push({
      label: context.summary,
      kind: vscode.QuickPickItemKind.Separator
    });
    
    for (const result of context.primary) {
      items.push({
        label: `$(symbol-${result.entity.type.toLowerCase()}) ${result.entity.name}`,
        description: result.entity.filePath,
        detail: `Lines ${result.entity.startLine}-${result.entity.endLine}`
      });
    }
    
    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: title
    });
    
    if (selected && selected.description) {
      // Open the file
      const doc = await vscode.workspace.openTextDocument(selected.description);
      await vscode.window.showTextDocument(doc);
    }
  }
  
  /**
   * Deactivate the extension
   */
  deactivate(): void {
    // Cleanup handled by VS Code
  }
}