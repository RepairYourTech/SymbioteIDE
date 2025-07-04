/**
 * Code Action Executor
 * 
 * Handles code manipulations and diff previews
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { 
  CodeAction,
  FileChange,
  DiffPreview,
  TextDiff,
  DiffHunk,
  DiffLine
} from '../../../types/chat-api';

export class CodeActionExecutor {
  private pendingChanges: Map<string, FileChange[]> = new Map();
  
  /**
   * Preview code action changes
   */
  async preview(action: CodeAction): Promise<DiffPreview> {
    const preview: DiffPreview = {
      summary: action.description,
      filesChanged: action.files.length,
      insertions: 0,
      deletions: 0,
      changes: []
    };
    
    // Process each file change
    for (const fileChange of action.files) {
      const diff = await this.generateDiff(fileChange);
      
      if (diff) {
        // Count insertions and deletions
        diff.hunks.forEach(hunk => {
          hunk.lines.forEach(line => {
            if (line.type === 'add') preview.insertions++;
            if (line.type === 'delete') preview.deletions++;
          });
        });
        
        preview.changes.push({
          ...fileChange,
          diff
        });
      } else {
        preview.changes.push(fileChange);
      }
    }
    
    // Store pending changes
    this.pendingChanges.set(action.id, action.files);
    
    return preview;
  }
  
  /**
   * Apply code action changes
   */
  async apply(action: CodeAction): Promise<void> {
    const edit = new vscode.WorkspaceEdit();
    
    for (const fileChange of action.files) {
      switch (fileChange.type) {
        case 'create':
          await this.applyCreate(edit, fileChange);
          break;
          
        case 'modify':
          await this.applyModify(edit, fileChange);
          break;
          
        case 'delete':
          await this.applyDelete(edit, fileChange);
          break;
          
        case 'rename':
          await this.applyRename(edit, fileChange);
          break;
      }
    }
    
    // Apply all edits
    const success = await vscode.workspace.applyEdit(edit);
    
    if (!success) {
      throw new Error('Failed to apply code changes');
    }
    
    // Clear pending changes
    this.pendingChanges.delete(action.id);
    
    // Show affected files
    if (action.files.length > 0) {
      const firstFile = action.files[0];
      const uri = vscode.Uri.file(firstFile.path);
      const document = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(document);
    }
  }
  
  /**
   * Generate unified diff for a file change
   */
  async generateDiff(fileChange: FileChange): Promise<TextDiff | undefined> {
    if (fileChange.type === 'create' || !fileChange.content) {
      return undefined;
    }
    
    try {
      const uri = vscode.Uri.file(fileChange.path);
      const document = await vscode.workspace.openTextDocument(uri);
      const original = document.getText();
      const modified = fileChange.content;
      
      // Generate diff hunks
      const hunks = this.computeDiff(original, modified);
      
      return {
        original,
        modified,
        hunks
      };
    } catch (error) {
      // File doesn't exist yet
      return undefined;
    }
  }
  
  /**
   * Clear pending changes
   */
  clearPending(actionId?: string): void {
    if (actionId) {
      this.pendingChanges.delete(actionId);
    } else {
      this.pendingChanges.clear();
    }
  }
  
  // Private methods
  
  private async applyCreate(edit: vscode.WorkspaceEdit, fileChange: FileChange): Promise<void> {
    if (!fileChange.content) {
      throw new Error('Content required for file creation');
    }
    
    const uri = vscode.Uri.file(fileChange.path);
    
    // Ensure directory exists
    const dir = path.dirname(fileChange.path);
    await vscode.workspace.fs.createDirectory(vscode.Uri.file(dir));
    
    // Create file
    edit.createFile(uri, { overwrite: false });
    edit.insert(uri, new vscode.Position(0, 0), fileChange.content);
  }
  
  private async applyModify(edit: vscode.WorkspaceEdit, fileChange: FileChange): Promise<void> {
    if (!fileChange.content) {
      throw new Error('Content required for file modification');
    }
    
    const uri = vscode.Uri.file(fileChange.path);
    
    try {
      const document = await vscode.workspace.openTextDocument(uri);
      const fullRange = new vscode.Range(
        document.positionAt(0),
        document.positionAt(document.getText().length)
      );
      
      edit.replace(uri, fullRange, fileChange.content);
    } catch (error) {
      // File doesn't exist, create it
      await this.applyCreate(edit, fileChange);
    }
  }
  
  private async applyDelete(edit: vscode.WorkspaceEdit, fileChange: FileChange): Promise<void> {
    const uri = vscode.Uri.file(fileChange.path);
    edit.deleteFile(uri, { recursive: false, ignoreIfNotExists: true });
  }
  
  private async applyRename(edit: vscode.WorkspaceEdit, fileChange: FileChange): Promise<void> {
    if (!fileChange.oldPath) {
      throw new Error('Old path required for rename');
    }
    
    const oldUri = vscode.Uri.file(fileChange.oldPath);
    const newUri = vscode.Uri.file(fileChange.path);
    
    edit.renameFile(oldUri, newUri, { overwrite: true });
  }
  
  private computeDiff(original: string, modified: string): DiffHunk[] {
    // This is a simplified diff implementation
    // In production, you'd use a proper diff library like diff-match-patch
    
    const originalLines = original.split('\n');
    const modifiedLines = modified.split('\n');
    const hunks: DiffHunk[] = [];
    
    let i = 0, j = 0;
    let currentHunk: DiffHunk | null = null;
    
    while (i < originalLines.length || j < modifiedLines.length) {
      if (i >= originalLines.length) {
        // Rest are additions
        if (!currentHunk) {
          currentHunk = {
            oldStart: i,
            oldLines: 0,
            newStart: j,
            newLines: 0,
            lines: []
          };
        }
        
        currentHunk.lines.push({
          type: 'add',
          content: modifiedLines[j],
          lineNumber: j + 1
        });
        currentHunk.newLines++;
        j++;
      } else if (j >= modifiedLines.length) {
        // Rest are deletions
        if (!currentHunk) {
          currentHunk = {
            oldStart: i,
            oldLines: 0,
            newStart: j,
            newLines: 0,
            lines: []
          };
        }
        
        currentHunk.lines.push({
          type: 'delete',
          content: originalLines[i],
          lineNumber: i + 1
        });
        currentHunk.oldLines++;
        i++;
      } else if (originalLines[i] === modifiedLines[j]) {
        // Context line
        if (currentHunk) {
          // Add some context and close hunk
          currentHunk.lines.push({
            type: 'context',
            content: originalLines[i]
          });
          hunks.push(currentHunk);
          currentHunk = null;
        }
        i++;
        j++;
      } else {
        // Lines differ
        if (!currentHunk) {
          currentHunk = {
            oldStart: i,
            oldLines: 0,
            newStart: j,
            newLines: 0,
            lines: []
          };
          
          // Add some context before
          if (i > 0) {
            currentHunk.lines.push({
              type: 'context',
              content: originalLines[i - 1]
            });
          }
        }
        
        // Simple heuristic: treat as delete + add
        currentHunk.lines.push({
          type: 'delete',
          content: originalLines[i],
          lineNumber: i + 1
        });
        currentHunk.oldLines++;
        i++;
        
        currentHunk.lines.push({
          type: 'add',
          content: modifiedLines[j],
          lineNumber: j + 1
        });
        currentHunk.newLines++;
        j++;
      }
    }
    
    if (currentHunk) {
      hunks.push(currentHunk);
    }
    
    return hunks;
  }
}