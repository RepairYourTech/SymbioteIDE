/**
 * Browser Service - Manages Playwright browser instances
 */

import * as vscode from 'vscode';
import { chromium, Browser, BrowserContext, Page, ElementHandle } from 'playwright';
import { EventEmitter } from 'events';
import { Logger } from '../utils/logger';
import * as path from 'path';
import * as os from 'os';

export interface BrowserOptions {
  headless?: boolean;
  devtools?: boolean;
  viewport?: { width: number; height: number };
  userDataDir?: string;
  executablePath?: string;
}

export interface BrowserTab {
  id: string;
  page: Page;
  context: BrowserContext;
  url: string;
  title: string;
  isActive: boolean;
  createdAt: Date;
}

export class BrowserService extends EventEmitter {
  private logger = new Logger('BrowserService');
  private browser: Browser | null = null;
  private contexts: Map<string, BrowserContext> = new Map();
  private tabs: Map<string, BrowserTab> = new Map();
  private activeTabId: string | null = null;
  private defaultOptions: BrowserOptions;
  
  constructor() {
    super();
    
    // Default options
    this.defaultOptions = {
      headless: false,
      devtools: true,
      viewport: { width: 1280, height: 720 },
      userDataDir: path.join(os.homedir(), '.symbiote', 'browser-data')
    };
  }
  
  /**
   * Initialize browser
   */
  async initialize(options?: BrowserOptions): Promise<void> {
    try {
      const mergedOptions = { ...this.defaultOptions, ...options };
      
      this.logger.info('Initializing browser...');
      
      // Launch browser
      this.browser = await chromium.launch({
        headless: mergedOptions.headless,
        devtools: mergedOptions.devtools,
        executablePath: mergedOptions.executablePath,
        args: [
          '--no-sandbox',
          '--disable-setuid-sandbox',
          '--disable-dev-shm-usage',
          '--disable-accelerated-2d-canvas',
          '--no-first-run',
          '--no-zygote',
          '--disable-gpu'
        ]
      });
      
      this.logger.info('Browser initialized successfully');
      
      // Setup event handlers
      this.browser.on('disconnected', () => {
        this.logger.warn('Browser disconnected');
        this.emit('disconnected');
      });
      
    } catch (error) {
      this.logger.error('Failed to initialize browser', error);
      throw error;
    }
  }
  
  /**
   * Create new browser tab
   */
  async createTab(url?: string, isolated: boolean = false): Promise<BrowserTab> {
    if (!this.browser) {
      throw new Error('Browser not initialized');
    }
    
    try {
      // Create context (isolated or shared)
      let context: BrowserContext;
      const contextId = isolated ? `isolated-${Date.now()}` : 'default';
      
      if (!this.contexts.has(contextId)) {
        context = await this.browser.newContext({
          viewport: this.defaultOptions.viewport,
          userAgent: 'SymbioteIDE/1.0 (Chromium)',
          locale: 'en-US',
          timezoneId: 'America/New_York'
        });
        
        // Enable request interception
        await context.route('**/*', (route) => {
          this.emit('request', {
            url: route.request().url(),
            method: route.request().method(),
            headers: route.request().headers()
          });
          route.continue();
        });
        
        this.contexts.set(contextId, context);
      } else {
        context = this.contexts.get(contextId)!;
      }
      
      // Create page
      const page = await context.newPage();
      
      // Setup page event handlers
      this.setupPageHandlers(page);
      
      // Navigate to URL if provided
      if (url) {
        await page.goto(url, { waitUntil: 'domcontentloaded' });
      }
      
      // Create tab object
      const tab: BrowserTab = {
        id: `tab-${Date.now()}`,
        page,
        context,
        url: url || 'about:blank',
        title: await page.title() || 'New Tab',
        isActive: false,
        createdAt: new Date()
      };
      
      this.tabs.set(tab.id, tab);
      
      // Set as active if first tab
      if (this.tabs.size === 1) {
        await this.activateTab(tab.id);
      }
      
      this.emit('tabCreated', tab);
      
      return tab;
      
    } catch (error) {
      this.logger.error('Failed to create tab', error);
      throw error;
    }
  }
  
  /**
   * Setup page event handlers
   */
  private setupPageHandlers(page: Page): void {
    // Navigation events
    page.on('load', () => {
      this.emit('pageLoad', { page });
    });
    
    page.on('domcontentloaded', () => {
      this.emit('domReady', { page });
    });
    
    // Console events
    page.on('console', (msg) => {
      this.emit('console', {
        type: msg.type(),
        text: msg.text(),
        location: msg.location()
      });
    });
    
    // Error events
    page.on('pageerror', (error) => {
      this.emit('pageError', { error });
    });
    
    // Dialog events
    page.on('dialog', async (dialog) => {
      this.emit('dialog', {
        type: dialog.type(),
        message: dialog.message()
      });
      
      // Auto-dismiss dialogs in headless mode
      if (this.defaultOptions.headless) {
        await dialog.accept();
      }
    });
    
    // Download events
    page.on('download', (download) => {
      this.emit('download', {
        url: download.url(),
        suggestedFilename: download.suggestedFilename()
      });
    });
  }
  
  /**
   * Activate tab
   */
  async activateTab(tabId: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    // Deactivate current tab
    if (this.activeTabId) {
      const currentTab = this.tabs.get(this.activeTabId);
      if (currentTab) {
        currentTab.isActive = false;
      }
    }
    
    // Activate new tab
    tab.isActive = true;
    this.activeTabId = tabId;
    
    // Bring to front
    await tab.page.bringToFront();
    
    this.emit('tabActivated', tab);
  }
  
  /**
   * Close tab
   */
  async closeTab(tabId: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      return;
    }
    
    try {
      await tab.page.close();
      this.tabs.delete(tabId);
      
      // If this was the active tab, activate another
      if (this.activeTabId === tabId) {
        this.activeTabId = null;
        const remainingTabs = Array.from(this.tabs.values());
        if (remainingTabs.length > 0) {
          await this.activateTab(remainingTabs[0].id);
        }
      }
      
      this.emit('tabClosed', tabId);
      
    } catch (error) {
      this.logger.error(`Failed to close tab ${tabId}`, error);
    }
  }
  
  /**
   * Get active tab
   */
  getActiveTab(): BrowserTab | null {
    if (!this.activeTabId) {
      return null;
    }
    return this.tabs.get(this.activeTabId) || null;
  }
  
  /**
   * Get all tabs
   */
  getAllTabs(): BrowserTab[] {
    return Array.from(this.tabs.values());
  }
  
  /**
   * Navigate to URL
   */
  async navigate(tabId: string, url: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    try {
      await tab.page.goto(url, { waitUntil: 'domcontentloaded' });
      tab.url = url;
      tab.title = await tab.page.title();
      
      this.emit('navigation', { tabId, url });
      
    } catch (error) {
      this.logger.error(`Failed to navigate to ${url}`, error);
      throw error;
    }
  }
  
  /**
   * Go back
   */
  async goBack(tabId: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) return;
    
    await tab.page.goBack();
  }
  
  /**
   * Go forward
   */
  async goForward(tabId: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) return;
    
    await tab.page.goForward();
  }
  
  /**
   * Reload page
   */
  async reload(tabId: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) return;
    
    await tab.page.reload();
  }
  
  /**
   * Take screenshot
   */
  async screenshot(tabId: string, options?: {
    fullPage?: boolean;
    clip?: { x: number; y: number; width: number; height: number };
  }): Promise<Buffer> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    return await tab.page.screenshot({
      fullPage: options?.fullPage,
      clip: options?.clip
    });
  }
  
  /**
   * Execute JavaScript in page
   */
  async evaluate<T = any>(
    tabId: string,
    script: string | Function,
    ...args: any[]
  ): Promise<T> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    return await tab.page.evaluate(script, ...args);
  }
  
  /**
   * Get element by selector
   */
  async querySelector(
    tabId: string,
    selector: string
  ): Promise<ElementHandle | null> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    return await tab.page.$(selector);
  }
  
  /**
   * Get all elements by selector
   */
  async querySelectorAll(
    tabId: string,
    selector: string
  ): Promise<ElementHandle[]> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    return await tab.page.$$(selector);
  }
  
  /**
   * Click element
   */
  async click(tabId: string, selector: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    await tab.page.click(selector);
  }
  
  /**
   * Type text
   */
  async type(tabId: string, selector: string, text: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    await tab.page.type(selector, text);
  }
  
  /**
   * Fill input
   */
  async fill(tabId: string, selector: string, value: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    await tab.page.fill(selector, value);
  }
  
  /**
   * Select option
   */
  async select(tabId: string, selector: string, value: string): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    await tab.page.selectOption(selector, value);
  }
  
  /**
   * Wait for selector
   */
  async waitForSelector(
    tabId: string,
    selector: string,
    options?: { timeout?: number; state?: 'attached' | 'detached' | 'visible' | 'hidden' }
  ): Promise<ElementHandle | null> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    return await tab.page.waitForSelector(selector, options);
  }
  
  /**
   * Get page HTML
   */
  async getHTML(tabId: string): Promise<string> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    return await tab.page.content();
  }
  
  /**
   * Set viewport size
   */
  async setViewport(
    tabId: string,
    viewport: { width: number; height: number }
  ): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    await tab.page.setViewportSize(viewport);
  }
  
  /**
   * Enable/disable JavaScript
   */
  async setJavaScriptEnabled(tabId: string, enabled: boolean): Promise<void> {
    const tab = this.tabs.get(tabId);
    if (!tab) {
      throw new Error(`Tab ${tabId} not found`);
    }
    
    await tab.context.setJavaScriptEnabled(enabled);
  }
  
  /**
   * Close browser
   */
  async close(): Promise<void> {
    try {
      // Close all tabs
      for (const tab of this.tabs.values()) {
        await tab.page.close();
      }
      
      // Close all contexts
      for (const context of this.contexts.values()) {
        await context.close();
      }
      
      // Close browser
      if (this.browser) {
        await this.browser.close();
      }
      
      this.tabs.clear();
      this.contexts.clear();
      this.browser = null;
      this.activeTabId = null;
      
      this.logger.info('Browser closed');
      
    } catch (error) {
      this.logger.error('Failed to close browser', error);
    }
  }
  
  /**
   * Check if browser is running
   */
  isRunning(): boolean {
    return this.browser !== null && this.browser.isConnected();
  }
}

// Export singleton instance
export const browserService = new BrowserService();