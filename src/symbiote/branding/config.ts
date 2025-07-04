/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE Team. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

export interface SymbioteIDEBranding {
  applicationName: string;
  productName: string;
  nameLong: string;
  nameShort: string;
  urlProtocol: string;
  dataFolderName: string;
  serverDataFolderName: string;
  downloadUrl: string;
  updateUrl: string;
  quality: string;
  target: string;
  commit: string;
  settingsSearchBuildId: number;
  settingsSearchUrl: string;
  reportIssueUrl: string;
  reportMarketplaceIssueUrl: string;
  privacyStatementUrl: string;
  telemetryOptOutUrl: string;
  npsSurveyUrl: string;
  cesSurveyUrl: string;
  licenseUrl: string;
  licenseName: string;
  serverLicenseUrl: string;
  serverLicense: string[];
  serverGreeting: string[];
  serverLicensePrompt: string;
  serverApplicationName: string;
  version: string;
  date?: string;
  windowColors?: { [key: string]: string };
}

export const SYMBIOTE_IDE_BRANDING: SymbioteIDEBranding = {
  applicationName: 'symbiote-ide',
  productName: 'SymbioteIDE',
  nameLong: 'SymbioteIDE - AI-Powered Development Environment',
  nameShort: 'SymbioteIDE',
  urlProtocol: 'symbiote-ide',
  dataFolderName: '.symbiote-ide',
  serverDataFolderName: '.symbiote-ide-server',
  downloadUrl: 'https://symbiote-ide.com/download',
  updateUrl: 'https://symbiote-ide.com/updates',
  quality: 'stable',
  target: 'user',
  commit: process.env.BUILD_SOURCEVERSION || 'development',
  settingsSearchBuildId: 1,
  settingsSearchUrl: 'https://symbiote-ide.com/search',
  reportIssueUrl: 'https://github.com/symbiote-ide/symbiote-ide/issues/new',
  reportMarketplaceIssueUrl: 'https://github.com/symbiote-ide/symbiote-ide/issues',
  privacyStatementUrl: 'https://symbiote-ide.com/privacy',
  telemetryOptOutUrl: 'https://symbiote-ide.com/telemetry-opt-out',
  npsSurveyUrl: 'https://symbiote-ide.com/survey/nps',
  cesSurveyUrl: 'https://symbiote-ide.com/survey/ces',
  licenseUrl: 'https://github.com/symbiote-ide/symbiote-ide/blob/main/LICENSE',
  licenseName: 'MIT',
  serverLicenseUrl: 'https://github.com/symbiote-ide/symbiote-ide/blob/main/LICENSE',
  serverLicense: [
    '----------------------------------------',
    'SymbioteIDE Server',
    '',
    'Copyright (c) SymbioteIDE Team',
    '',
    'This software is licensed under the MIT License',
    '----------------------------------------'
  ],
  serverGreeting: [
    '',
    'Welcome to SymbioteIDE Server!',
    '',
    'AI-Powered Development at Your Fingertips',
    ''
  ],
  serverLicensePrompt: 'Do you accept the SymbioteIDE Server license terms?',
  serverApplicationName: 'symbiote-ide-server',
  version: '0.1.0',
  windowColors: {
    'activityBar.background': '#1a1a2e',
    'activityBar.foreground': '#00D4FF',
    'activityBarBadge.background': '#7B61FF',
    'activityBarBadge.foreground': '#FFFFFF',
    'titleBar.activeBackground': '#1a1a2e',
    'titleBar.activeForeground': '#00D4FF',
    'statusBar.background': '#7B61FF',
    'statusBar.foreground': '#FFFFFF',
    'statusBar.noFolderBackground': '#FF006E',
    'statusBarItem.remoteBackground': '#00D4FF',
    'statusBarItem.remoteForeground': '#1a1a2e'
  }
};

// Color theme for SymbioteIDE
export const SYMBIOTE_IDE_THEME = {
  name: 'SymbioteIDE Dark',
  type: 'dark',
  colors: {
    // Editor colors
    'editor.background': '#0f0f1e',
    'editor.foreground': '#e0e0e0',
    'editorLineNumber.foreground': '#5a5a7e',
    'editorLineNumber.activeForeground': '#00D4FF',
    'editor.selectionBackground': '#7B61FF44',
    'editor.lineHighlightBackground': '#1a1a2e',
    
    // Activity Bar
    'activityBar.background': '#1a1a2e',
    'activityBar.foreground': '#00D4FF',
    'activityBarBadge.background': '#7B61FF',
    'activityBarBadge.foreground': '#FFFFFF',
    
    // Side Bar
    'sideBar.background': '#12121f',
    'sideBar.foreground': '#d0d0d0',
    'sideBarTitle.foreground': '#00D4FF',
    
    // Status Bar
    'statusBar.background': '#7B61FF',
    'statusBar.foreground': '#FFFFFF',
    'statusBar.noFolderBackground': '#FF006E',
    'statusBarItem.remoteBackground': '#00D4FF',
    'statusBarItem.remoteForeground': '#1a1a2e',
    
    // Terminal
    'terminal.background': '#0a0a12',
    'terminal.foreground': '#e0e0e0',
    'terminal.ansiBlue': '#00D4FF',
    'terminal.ansiMagenta': '#7B61FF',
    'terminal.ansiRed': '#FF006E',
    
    // Notifications
    'notificationCenter.background': '#1a1a2e',
    'notifications.background': '#1a1a2e',
    'notifications.foreground': '#e0e0e0',
    
    // Input
    'input.background': '#1a1a2e',
    'input.foreground': '#e0e0e0',
    'input.placeholderForeground': '#5a5a7e',
    'inputOption.activeBorder': '#00D4FF',
    
    // Buttons
    'button.background': '#7B61FF',
    'button.foreground': '#FFFFFF',
    'button.hoverBackground': '#9580FF',
    
    // Dropdown
    'dropdown.background': '#1a1a2e',
    'dropdown.foreground': '#e0e0e0',
    'dropdown.border': '#00D4FF44',
    
    // AI Assistant Colors
    'symbiote.ai.primary': '#00D4FF',
    'symbiote.ai.secondary': '#7B61FF',
    'symbiote.ai.accent': '#FF006E',
    'symbiote.ai.success': '#00FF88',
    'symbiote.ai.warning': '#FFB800',
    'symbiote.ai.error': '#FF0044'
  }
};