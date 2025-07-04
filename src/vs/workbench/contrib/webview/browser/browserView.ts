/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { localize } from '../../../../nls.js';
import { DisposableStore } from '../../../../base/common/lifecycle.js';
import { IContextMenuService } from '../../../../platform/contextview/browser/contextView.js';
import { IInstantiationService } from '../../../../platform/instantiation/common/instantiation.js';
import { IThemeService } from '../../../../platform/theme/common/themeService.js';
import { IEnhancedWebviewService, IWebviewTab } from './enhancedWebviewService.js';
import { IKeybindingService } from '../../../../platform/keybinding/common/keybinding.js';
import { URI } from '../../../../base/common/uri.js';
import * as DOM from '../../../../base/browser/dom.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import { InputBox } from '../../../../base/browser/ui/inputbox/inputBox.js';
import { ActionBar } from '../../../../base/browser/ui/actionbar/actionbar.js';
import { Action, IAction } from '../../../../base/common/actions.js';
import { Codicon } from '../../../../base/common/codicons.js';
import { ThemeIcon } from '../../../../base/common/themables.js';
import './media/browserView.css';

export class BrowserView {
	private readonly disposables = new DisposableStore();
	private container!: HTMLElement;
	private tabsContainer!: HTMLElement;
	private navigationBar!: HTMLElement;
	private contentContainer!: HTMLElement;
	private urlInput!: InputBox;
	private tabElements = new Map<string, HTMLElement>();

	constructor(
		parent: HTMLElement,
		@IEnhancedWebviewService private readonly webviewService: IEnhancedWebviewService,
		@IInstantiationService private readonly instantiationService: IInstantiationService,
		@IContextMenuService private readonly contextMenuService: IContextMenuService,
		@IThemeService private readonly themeService: IThemeService,
		@IKeybindingService private readonly keybindingService: IKeybindingService
	) {
		this.create(parent);
		this.registerListeners();
	}

	private create(parent: HTMLElement): void {
		this.container = DOM.append(parent, DOM.$('.browser-view'));

		// Create tabs bar
		this.tabsContainer = DOM.append(this.container, DOM.$('.tabs-container'));
		this.createNewTabButton();

		// Create navigation bar
		this.navigationBar = DOM.append(this.container, DOM.$('.navigation-bar'));
		this.createNavigationControls();

		// Create content area
		this.contentContainer = DOM.append(this.container, DOM.$('.content-container'));
		
		// Create initial tab
		this.webviewService.createTab(URI.parse('https://www.github.com'));
	}

	private createNewTabButton(): void {
		const newTabButton = DOM.append(this.tabsContainer, DOM.$('.new-tab-button'));
		newTabButton.innerHTML = '<i class="codicon codicon-add"></i>';
		newTabButton.title = localize('newTab', 'New Tab');
		
		this.disposables.add(DOM.addDisposableListener(newTabButton, DOM.EventType.CLICK, () => {
			this.webviewService.createTab(URI.parse('https://www.github.com'));
		}));
	}

	private createNavigationControls(): void {
		const actions: IAction[] = [
			new Action('back', localize('back', 'Back'), ThemeIcon.asClassName(Codicon.arrowLeft), false, () => {
				const tab = this.webviewService.getActiveTab();
				if (tab) {
					this.webviewService.getHistory(tab.id).back();
				}
			}),
			new Action('forward', localize('forward', 'Forward'), ThemeIcon.asClassName(Codicon.arrowRight), false, () => {
				const tab = this.webviewService.getActiveTab();
				if (tab) {
					this.webviewService.getHistory(tab.id).forward();
				}
			}),
			new Action('reload', localize('reload', 'Reload'), ThemeIcon.asClassName(Codicon.refresh), true, () => {
				this.webviewService.reload();
			}),
			new Action('home', localize('home', 'Home'), ThemeIcon.asClassName(Codicon.home), true, () => {
				this.webviewService.navigate(URI.parse('https://www.github.com'));
			})
		];

		const actionBar = this.disposables.add(new ActionBar(this.navigationBar, {
			orientation: 0, // Horizontal
			actionViewItemProvider: undefined,
			ariaLabel: localize('browserActions', 'Browser Actions')
		}));

		actionBar.push(actions, { icon: true, label: false });

		// URL input
		const urlContainer = DOM.append(this.navigationBar, DOM.$('.url-input-container'));
		this.urlInput = this.disposables.add(new InputBox(urlContainer, this.contextMenuService, {
			placeholder: localize('urlPlaceholder', 'Enter URL'),
			ariaLabel: localize('urlInput', 'URL Input')
		}));

		this.disposables.add(this.urlInput.onDidChange(value => {
			if (value && value.includes('\n')) {
				this.navigateToUrl(value.trim());
			}
		}));

		// More actions
		const moreActions: IAction[] = [
			new Action('bookmark', localize('bookmark', 'Bookmark'), ThemeIcon.asClassName(Codicon.bookmark), true, () => {
				const tab = this.webviewService.getActiveTab();
				if (tab) {
					this.webviewService.addBookmark(tab.uri, tab.title);
				}
			}),
			new Action('devtools', localize('devTools', 'Developer Tools'), ThemeIcon.asClassName(Codicon.tools), true, () => {
				const tab = this.webviewService.getActiveTab();
				if (tab) {
					this.webviewService.getDevTools(tab.id).open();
				}
			}),
			new Action('settings', localize('settings', 'Settings'), ThemeIcon.asClassName(Codicon.settingsGear), true, () => {
				// Open browser settings
			})
		];

		const moreActionBar = this.disposables.add(new ActionBar(this.navigationBar, {
			orientation: 0,
			actionViewItemProvider: undefined,
			ariaLabel: localize('moreBrowserActions', 'More Browser Actions')
		}));

		moreActionBar.push(moreActions, { icon: true, label: false });
	}

	private registerListeners(): void {
		this.disposables.add(this.webviewService.onDidCreateTab(tab => {
			this.addTab(tab);
		}));

		this.disposables.add(this.webviewService.onDidCloseTab(tabId => {
			this.removeTab(tabId);
		}));

		this.disposables.add(this.webviewService.onDidChangeActiveTab(tab => {
			this.updateActiveTab(tab);
		}));
	}

	private addTab(tab: IWebviewTab): void {
		const tabElement = DOM.append(this.tabsContainer, DOM.$('.tab'));
		tabElement.dataset.tabId = tab.id;

		const tabIcon = DOM.append(tabElement, DOM.$('.tab-icon'));
		tabIcon.innerHTML = '<i class="codicon codicon-globe"></i>';

		const tabTitle = DOM.append(tabElement, DOM.$('.tab-title'));
		tabTitle.textContent = this.truncateTitle(tab.title);

		const tabClose = DOM.append(tabElement, DOM.$('.tab-close'));
		tabClose.innerHTML = '<i class="codicon codicon-close"></i>';

		this.disposables.add(DOM.addDisposableListener(tabElement, DOM.EventType.CLICK, () => {
			this.webviewService.setActiveTab(tab.id);
		}));

		this.disposables.add(DOM.addDisposableListener(tabClose, DOM.EventType.CLICK, (e) => {
			e.stopPropagation();
			this.webviewService.closeTab(tab.id);
		}));

		// Context menu
		this.disposables.add(DOM.addDisposableListener(tabElement, DOM.EventType.CONTEXT_MENU, (e) => {
			this.showTabContextMenu(tab, e);
		}));

		this.tabElements.set(tab.id, tabElement);

		// Add webview to content
		this.contentContainer.appendChild(tab.webview.container);
	}

	private removeTab(tabId: string): void {
		const tabElement = this.tabElements.get(tabId);
		if (tabElement) {
			tabElement.remove();
			this.tabElements.delete(tabId);
		}
	}

	private updateActiveTab(tab: IWebviewTab | undefined): void {
		// Update tab styles
		this.tabElements.forEach((element, id) => {
			if (tab && id === tab.id) {
				DOM.addClass(element, 'active');
			} else {
				DOM.removeClass(element, 'active');
			}
		});

		// Update URL input
		if (tab) {
			this.urlInput.value = tab.uri.toString();
		}

		// Show/hide webviews
		const tabs = this.webviewService.getTabs();
		tabs.forEach(t => {
			if (tab && t.id === tab.id) {
				DOM.show(t.webview.container);
			} else {
				DOM.hide(t.webview.container);
			}
		});

		// Update navigation buttons
		if (tab) {
			const history = this.webviewService.getHistory(tab.id);
			// Would update button states based on history
		}
	}

	private navigateToUrl(url: string): void {
		if (!url.startsWith('http://') && !url.startsWith('https://')) {
			url = 'https://' + url;
		}

		try {
			const uri = URI.parse(url);
			this.webviewService.navigate(uri);
		} catch (error) {
			// Handle invalid URL
		}
	}

	private showTabContextMenu(tab: IWebviewTab, event: MouseEvent): void {
		this.contextMenuService.showContextMenu({
			getAnchor: () => ({ x: event.pageX, y: event.pageY }),
			getActions: () => [
				new Action('duplicateTab', localize('duplicateTab', 'Duplicate Tab'), undefined, true, () => {
					this.webviewService.createTab(tab.uri);
				}),
				new Action('reopenClosedTab', localize('reopenClosedTab', 'Reopen Closed Tab'), undefined, false),
				new Action('separator', undefined, undefined, false),
				new Action('closeOtherTabs', localize('closeOtherTabs', 'Close Other Tabs'), undefined, true, () => {
					const tabs = this.webviewService.getTabs();
					tabs.forEach(t => {
						if (t.id !== tab.id) {
							this.webviewService.closeTab(t.id);
						}
					});
				}),
				new Action('closeTabsToRight', localize('closeTabsToRight', 'Close Tabs to the Right'), undefined, true)
			]
		});
	}

	private truncateTitle(title: string, maxLength: number = 20): string {
		if (title.length <= maxLength) {
			return title;
		}
		return title.substring(0, maxLength - 3) + '...';
	}

	dispose(): void {
		this.disposables.dispose();
	}
}