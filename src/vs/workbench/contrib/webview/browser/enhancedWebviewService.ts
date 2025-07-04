/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Emitter, Event } from '../../../../base/common/event.js';
import { Disposable, IDisposable } from '../../../../base/common/lifecycle.js';
import { URI } from '../../../../base/common/uri.js';
import { IInstantiationService } from '../../../../platform/instantiation/common/instantiation.js';
import { ILogService } from '../../../../platform/log/common/log.js';
import { IStorageService, StorageScope, StorageTarget } from '../../../../platform/storage/common/storage.js';
import { INotificationService } from '../../../../platform/notification/common/notification.js';
import { IContextMenuService } from '../../../../platform/contextview/browser/contextView.js';
import { Action } from '../../../../base/common/actions.js';
import { localize } from '../../../../nls.js';
import { WebviewService } from './webviewService.js';
import { IWebview, IWebviewElement, WebviewInitInfo } from './webview.js';
import { createDecorator } from '../../../../platform/instantiation/common/instantiation.js';

export const IEnhancedWebviewService = createDecorator<IEnhancedWebviewService>('enhancedWebviewService');

export interface IWebviewTab {
	id: string;
	title: string;
	uri: URI;
	webview: IWebview;
	favicon?: URI;
	isLoading: boolean;
	canGoBack: boolean;
	canGoForward: boolean;
}

export interface IBrowserHistory {
	back(): void;
	forward(): void;
	canGoBack(): boolean;
	canGoForward(): boolean;
	getHistory(): URI[];
}

export interface IWebviewDevTools {
	open(): void;
	close(): void;
	isOpen(): boolean;
}

export interface IEnhancedWebviewService extends WebviewService {
	readonly _serviceBrand: undefined;

	/**
	 * Event fired when a new tab is created
	 */
	readonly onDidCreateTab: Event<IWebviewTab>;

	/**
	 * Event fired when a tab is closed
	 */
	readonly onDidCloseTab: Event<string>;

	/**
	 * Event fired when active tab changes
	 */
	readonly onDidChangeActiveTab: Event<IWebviewTab | undefined>;

	/**
	 * Create a new browser tab
	 */
	createTab(uri: URI, options?: { background?: boolean }): Promise<IWebviewTab>;

	/**
	 * Get all open tabs
	 */
	getTabs(): IWebviewTab[];

	/**
	 * Get active tab
	 */
	getActiveTab(): IWebviewTab | undefined;

	/**
	 * Set active tab
	 */
	setActiveTab(tabId: string): void;

	/**
	 * Close a tab
	 */
	closeTab(tabId: string): void;

	/**
	 * Get browser history for a webview
	 */
	getHistory(webviewId: string): IBrowserHistory;

	/**
	 * Get dev tools for a webview
	 */
	getDevTools(webviewId: string): IWebviewDevTools;

	/**
	 * Navigate to URL in active tab
	 */
	navigate(uri: URI): Promise<void>;

	/**
	 * Reload active tab
	 */
	reload(): void;

	/**
	 * Stop loading active tab
	 */
	stop(): void;

	/**
	 * Add bookmark
	 */
	addBookmark(uri: URI, title: string): void;

	/**
	 * Get bookmarks
	 */
	getBookmarks(): Array<{ uri: URI; title: string; timestamp: Date }>;

	/**
	 * Clear browsing data
	 */
	clearBrowsingData(options: {
		cache?: boolean;
		cookies?: boolean;
		history?: boolean;
	}): Promise<void>;
}

const BOOKMARKS_STORAGE_KEY = 'symbiote.webview.bookmarks';
const HISTORY_STORAGE_KEY = 'symbiote.webview.history';

export class EnhancedWebviewService extends WebviewService implements IEnhancedWebviewService {
	declare readonly _serviceBrand: undefined;

	private readonly _onDidCreateTab = this._register(new Emitter<IWebviewTab>());
	readonly onDidCreateTab: Event<IWebviewTab> = this._onDidCreateTab.event;

	private readonly _onDidCloseTab = this._register(new Emitter<string>());
	readonly onDidCloseTab: Event<string> = this._onDidCloseTab.event;

	private readonly _onDidChangeActiveTab = this._register(new Emitter<IWebviewTab | undefined>());
	readonly onDidChangeActiveTab: Event<IWebviewTab | undefined> = this._onDidChangeActiveTab.event;

	private tabs = new Map<string, IWebviewTab>();
	private activeTabId: string | undefined;
	private histories = new Map<string, NavigationHistory>();
	private bookmarks: Array<{ uri: URI; title: string; timestamp: Date }> = [];
	private tabCounter = 0;

	constructor(
		@IInstantiationService instantiationService: IInstantiationService,
		@IStorageService private readonly storageService: IStorageService,
		@ILogService private readonly logService: ILogService,
		@INotificationService private readonly notificationService: INotificationService,
		@IContextMenuService private readonly contextMenuService: IContextMenuService
	) {
		super(instantiationService);
		this.loadBookmarks();
	}

	async createTab(uri: URI, options?: { background?: boolean }): Promise<IWebviewTab> {
		const tabId = `tab_${++this.tabCounter}`;
		
		const webview = this.createWebviewElement({
			title: 'Loading...',
			options: {
				enableFindWidget: true,
				retainContextWhenHidden: true
			},
			contentOptions: {
				allowScripts: true,
				localResourceRoots: []
			}
		});

		const tab: IWebviewTab = {
			id: tabId,
			title: uri.toString(),
			uri,
			webview,
			isLoading: true,
			canGoBack: false,
			canGoForward: false
		};

		this.tabs.set(tabId, tab);
		this.histories.set(tabId, new NavigationHistory());

		// Set up webview event handlers
		this.setupWebviewHandlers(tab);

		// Navigate to URI
		await this.navigateTab(tab, uri);

		if (!options?.background) {
			this.setActiveTab(tabId);
		}

		this._onDidCreateTab.fire(tab);
		return tab;
	}

	getTabs(): IWebviewTab[] {
		return Array.from(this.tabs.values());
	}

	getActiveTab(): IWebviewTab | undefined {
		return this.activeTabId ? this.tabs.get(this.activeTabId) : undefined;
	}

	setActiveTab(tabId: string): void {
		const tab = this.tabs.get(tabId);
		if (tab) {
			this.activeTabId = tabId;
			this._updateActiveWebview(tab.webview);
			this._onDidChangeActiveTab.fire(tab);
		}
	}

	closeTab(tabId: string): void {
		const tab = this.tabs.get(tabId);
		if (!tab) {
			return;
		}

		tab.webview.dispose();
		this.tabs.delete(tabId);
		this.histories.delete(tabId);
		
		if (this.activeTabId === tabId) {
			const remainingTabs = Array.from(this.tabs.keys());
			this.activeTabId = remainingTabs[0];
			this._onDidChangeActiveTab.fire(this.getActiveTab());
		}

		this._onDidCloseTab.fire(tabId);
	}

	getHistory(webviewId: string): IBrowserHistory {
		const history = this.histories.get(webviewId);
		if (!history) {
			throw new Error(`No history for webview ${webviewId}`);
		}

		return {
			back: () => {
				const uri = history.back();
				if (uri) {
					const tab = this.tabs.get(webviewId);
					if (tab) {
						this.navigateTab(tab, uri);
					}
				}
			},
			forward: () => {
				const uri = history.forward();
				if (uri) {
					const tab = this.tabs.get(webviewId);
					if (tab) {
						this.navigateTab(tab, uri);
					}
				}
			},
			canGoBack: () => history.canGoBack(),
			canGoForward: () => history.canGoForward(),
			getHistory: () => history.getHistory()
		};
	}

	getDevTools(webviewId: string): IWebviewDevTools {
		const tab = this.tabs.get(webviewId);
		if (!tab) {
			throw new Error(`No tab for webview ${webviewId}`);
		}

		let isOpen = false;

		return {
			open: () => {
				if (!isOpen) {
					// In a real implementation, this would open actual dev tools
					this.notificationService.info(localize('devToolsOpened', 'Developer tools opened for {0}', tab.title));
					isOpen = true;
				}
			},
			close: () => {
				if (isOpen) {
					this.notificationService.info(localize('devToolsClosed', 'Developer tools closed'));
					isOpen = false;
				}
			},
			isOpen: () => isOpen
		};
	}

	async navigate(uri: URI): Promise<void> {
		const activeTab = this.getActiveTab();
		if (activeTab) {
			await this.navigateTab(activeTab, uri);
		}
	}

	reload(): void {
		const activeTab = this.getActiveTab();
		if (activeTab) {
			activeTab.webview.reload();
		}
	}

	stop(): void {
		const activeTab = this.getActiveTab();
		if (activeTab) {
			activeTab.isLoading = false;
			// In a real implementation, would stop loading
		}
	}

	addBookmark(uri: URI, title: string): void {
		this.bookmarks.push({
			uri,
			title,
			timestamp: new Date()
		});
		this.saveBookmarks();
		this.notificationService.info(localize('bookmarkAdded', 'Bookmark added: {0}', title));
	}

	getBookmarks(): Array<{ uri: URI; title: string; timestamp: Date }> {
		return [...this.bookmarks];
	}

	async clearBrowsingData(options: {
		cache?: boolean;
		cookies?: boolean;
		history?: boolean;
	}): Promise<void> {
		if (options.history) {
			this.histories.clear();
			this.tabs.forEach(tab => {
				this.histories.set(tab.id, new NavigationHistory());
			});
		}

		if (options.cookies || options.cache) {
			// In a real implementation, would clear actual data
			this.logService.info('[EnhancedWebview] Clearing browsing data', options);
		}

		this.notificationService.info(localize('browsingDataCleared', 'Browsing data cleared'));
	}

	private async navigateTab(tab: IWebviewTab, uri: URI): Promise<void> {
		tab.isLoading = true;
		tab.uri = uri;
		
		const history = this.histories.get(tab.id);
		if (history) {
			history.navigate(uri);
			tab.canGoBack = history.canGoBack();
			tab.canGoForward = history.canGoForward();
		}

		// Load content based on URI
		if (uri.scheme === 'http' || uri.scheme === 'https') {
			// For web content, we'd use an iframe or proxy
			tab.webview.setHtml(this.createWebContent(uri));
		} else if (uri.scheme === 'file') {
			// For local files, load directly
			try {
				const response = await fetch(uri.toString());
				const content = await response.text();
				tab.webview.setHtml(content);
			} catch (error) {
				tab.webview.setHtml(this.createErrorPage(error));
			}
		}

		tab.isLoading = false;
	}

	private setupWebviewHandlers(tab: IWebviewTab): void {
		const disposables = new DisposableStore();

		// Handle messages from webview
		disposables.add(tab.webview.onDidReceiveMessage(e => {
			switch (e.type) {
				case 'navigate':
					this.navigateTab(tab, URI.parse(e.url));
					break;
				case 'openDevTools':
					this.getDevTools(tab.id).open();
					break;
			}
		}));

		// Context menu
		disposables.add(tab.webview.onDidClickLink(uri => {
			this.contextMenuService.showContextMenu({
				getAnchor: () => ({ x: 0, y: 0 }),
				getActions: () => [
					new Action('openInNewTab', localize('openInNewTab', 'Open in New Tab'), undefined, true, 
						() => this.createTab(uri, { background: true })),
					new Action('copyLink', localize('copyLink', 'Copy Link'), undefined, true, 
						() => navigator.clipboard.writeText(uri.toString()))
				]
			});
		}));
	}

	private createWebContent(uri: URI): string {
		return `
<!DOCTYPE html>
<html>
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<style>
		body { 
			margin: 0; 
			padding: 20px;
			font-family: var(--vscode-font-family);
			color: var(--vscode-foreground);
			background: var(--vscode-editor-background);
		}
		iframe {
			width: 100%;
			height: calc(100vh - 40px);
			border: 1px solid var(--vscode-panel-border);
		}
		.loading {
			text-align: center;
			padding: 50px;
		}
		.error {
			color: var(--vscode-errorForeground);
			padding: 20px;
		}
	</style>
</head>
<body>
	<div class="loading">
		<p>Loading ${uri.toString()}...</p>
		<p><small>Note: External content may be restricted due to security policies.</small></p>
	</div>
	<iframe src="${uri.toString()}" sandbox="allow-scripts allow-same-origin"></iframe>
</body>
</html>
		`;
	}

	private createErrorPage(error: any): string {
		return `
<!DOCTYPE html>
<html>
<head>
	<meta charset="UTF-8">
	<style>
		body { 
			font-family: var(--vscode-font-family);
			color: var(--vscode-errorForeground);
			background: var(--vscode-editor-background);
			padding: 20px;
		}
	</style>
</head>
<body>
	<h1>Error Loading Page</h1>
	<p>${error.message || error}</p>
</body>
</html>
		`;
	}

	private loadBookmarks(): void {
		const stored = this.storageService.get(BOOKMARKS_STORAGE_KEY, StorageScope.APPLICATION);
		if (stored) {
			try {
				const bookmarks = JSON.parse(stored);
				this.bookmarks = bookmarks.map((b: any) => ({
					uri: URI.parse(b.uri),
					title: b.title,
					timestamp: new Date(b.timestamp)
				}));
			} catch (e) {
				this.logService.error('[EnhancedWebview] Failed to load bookmarks:', e);
			}
		}
	}

	private saveBookmarks(): void {
		const data = this.bookmarks.map(b => ({
			uri: b.uri.toString(),
			title: b.title,
			timestamp: b.timestamp.toISOString()
		}));
		this.storageService.store(BOOKMARKS_STORAGE_KEY, JSON.stringify(data), StorageScope.APPLICATION, StorageTarget.USER);
	}
}

class NavigationHistory {
	private history: URI[] = [];
	private currentIndex = -1;

	navigate(uri: URI): void {
		// Remove forward history
		this.history = this.history.slice(0, this.currentIndex + 1);
		this.history.push(uri);
		this.currentIndex++;
	}

	back(): URI | undefined {
		if (this.canGoBack()) {
			this.currentIndex--;
			return this.history[this.currentIndex];
		}
		return undefined;
	}

	forward(): URI | undefined {
		if (this.canGoForward()) {
			this.currentIndex++;
			return this.history[this.currentIndex];
		}
		return undefined;
	}

	canGoBack(): boolean {
		return this.currentIndex > 0;
	}

	canGoForward(): boolean {
		return this.currentIndex < this.history.length - 1;
	}

	getHistory(): URI[] {
		return [...this.history];
	}
}