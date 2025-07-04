/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { localize } from '../../../../nls.js';
import { registerAction2, Action2 } from '../../../../platform/actions/common/actions.js';
import { ServicesAccessor } from '../../../../platform/instantiation/common/instantiation.js';
import { registerSingleton } from '../../../../platform/instantiation/common/extensions.js';
import { IEnhancedWebviewService, EnhancedWebviewService } from './enhancedWebviewService.js';
import { IQuickInputService } from '../../../../platform/quickinput/common/quickInput.js';
import { IEditorService } from '../../../services/editor/common/editorService.js';
import { URI } from '../../../../base/common/uri.js';
import { Categories } from '../../../../platform/action/common/actionCommonCategories.js';
import { IViewsRegistry, Extensions as ViewExtensions, IViewDescriptor } from '../../../common/views.js';
import { Registry } from '../../../../platform/registry/common/platform.js';
import { SyncDescriptor } from '../../../../platform/instantiation/common/descriptors.js';
import { ViewPaneContainer } from '../../../browser/parts/views/viewPaneContainer.js';
import { IConfigurationRegistry, Extensions as ConfigurationExtensions } from '../../../../platform/configuration/common/configurationRegistry.js';
import { ContextKeyExpr } from '../../../../platform/contextkey/common/contextkey.js';
import { BrowserView } from './browserView.js';
import { IWorkbenchContribution } from '../../../common/contributions.js';
import { IWorkbenchLayoutService } from '../../../services/layout/browser/layoutService.js';
import { DisposableStore } from '../../../../base/common/lifecycle.js';

// Register Enhanced Webview Service
registerSingleton(IEnhancedWebviewService, EnhancedWebviewService, true);

// Browser Panel ID
const BROWSER_PANEL_ID = 'workbench.panel.browser';

// Register Browser View Container
const browserViewContainer = Registry.as<IViewsRegistry>(ViewExtensions.ViewsRegistry).registerViewContainer({
	id: BROWSER_PANEL_ID,
	title: localize('browser', 'Browser'),
	ctorDescriptor: new SyncDescriptor(ViewPaneContainer, [BROWSER_PANEL_ID]),
	hideIfEmpty: false,
	order: 10,
	icon: 'globe',
}, 1 /* ViewContainerLocation.Panel */);

// Browser Contribution for initializing the view
export class BrowserContribution implements IWorkbenchContribution {
	private readonly disposables = new DisposableStore();

	constructor(
		@IWorkbenchLayoutService private readonly layoutService: IWorkbenchLayoutService,
		@IEnhancedWebviewService private readonly webviewService: IEnhancedWebviewService
	) {
		// Initialize browser features when needed
	}

	dispose(): void {
		this.disposables.dispose();
	}
}

// Register Actions
class OpenWebBrowserAction extends Action2 {
	constructor() {
		super({
			id: 'webview.action.openBrowser',
			title: localize('openWebBrowser', 'Open Web Browser'),
			category: Categories.View,
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const layoutService = accessor.get(IWorkbenchLayoutService);
		layoutService.setPartHidden(false, 2 /* Parts.PANEL_PART */);
		layoutService.openPanel(BROWSER_PANEL_ID, true);
	}
}

class NavigateToUrlAction extends Action2 {
	constructor() {
		super({
			id: 'webview.action.navigateToUrl',
			title: localize('navigateToUrl', 'Navigate to URL'),
			category: localize('browser', 'Browser'),
			f1: true,
			precondition: ContextKeyExpr.equals('activePanel', BROWSER_PANEL_ID)
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const quickInputService = accessor.get(IQuickInputService);
		const webviewService = accessor.get(IEnhancedWebviewService);

		const url = await quickInputService.input({
			placeHolder: localize('enterUrl', 'Enter URL (e.g., https://example.com)'),
			prompt: localize('navigatePrompt', 'Navigate to a web page'),
			validateInput: (value) => {
				try {
					new URL(value.startsWith('http') ? value : `https://${value}`);
					return undefined;
				} catch {
					return localize('invalidUrl', 'Please enter a valid URL');
				}
			}
		});

		if (url) {
			const fullUrl = url.startsWith('http') ? url : `https://${url}`;
			await webviewService.navigate(URI.parse(fullUrl));
		}
	}
}

class ShowBookmarksAction extends Action2 {
	constructor() {
		super({
			id: 'webview.action.showBookmarks',
			title: localize('showBookmarks', 'Show Bookmarks'),
			category: localize('browser', 'Browser'),
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const quickInputService = accessor.get(IQuickInputService);
		const webviewService = accessor.get(IEnhancedWebviewService);
		const editorService = accessor.get(IEditorService);

		const bookmarks = webviewService.getBookmarks();
		
		if (bookmarks.length === 0) {
			await quickInputService.pick([], {
				placeHolder: localize('noBookmarks', 'No bookmarks saved')
			});
			return;
		}

		const picks = bookmarks.map(b => ({
			label: b.title,
			description: b.uri.toString(),
			detail: localize('bookmarkedOn', 'Bookmarked on {0}', b.timestamp.toLocaleDateString())
		}));

		const selected = await quickInputService.pick(picks, {
			placeHolder: localize('selectBookmark', 'Select a bookmark to open'),
			canPickMany: false
		});

		if (selected) {
			const bookmark = bookmarks.find(b => b.title === selected.label);
			if (bookmark) {
				await webviewService.navigate(bookmark.uri);
			}
		}
	}
}

class ClearBrowsingDataAction extends Action2 {
	constructor() {
		super({
			id: 'webview.action.clearBrowsingData',
			title: localize('clearBrowsingData', 'Clear Browsing Data'),
			category: localize('browser', 'Browser'),
			f1: true
		});
	}

	async run(accessor: ServicesAccessor): Promise<void> {
		const quickInputService = accessor.get(IQuickInputService);
		const webviewService = accessor.get(IEnhancedWebviewService);

		const options = await quickInputService.pick([
			{ id: 'history', label: localize('browsingHistory', 'Browsing History') },
			{ id: 'cache', label: localize('cachedData', 'Cached Images and Files') },
			{ id: 'cookies', label: localize('cookiesData', 'Cookies and Site Data') }
		], {
			placeHolder: localize('selectDataToClear', 'Select data to clear'),
			canPickMany: true
		});

		if (options && options.length > 0) {
			await webviewService.clearBrowsingData({
				history: options.some(o => o.id === 'history'),
				cache: options.some(o => o.id === 'cache'),
				cookies: options.some(o => o.id === 'cookies')
			});
		}
	}
}

// Register all actions
registerAction2(OpenWebBrowserAction);
registerAction2(NavigateToUrlAction);
registerAction2(ShowBookmarksAction);
registerAction2(ClearBrowsingDataAction);

// Register Configuration
Registry.as<IConfigurationRegistry>(ConfigurationExtensions.Configuration).registerConfiguration({
	id: 'webBrowser',
	title: localize('webBrowserConfigurationTitle', 'Web Browser'),
	type: 'object',
	properties: {
		'webBrowser.enabled': {
			type: 'boolean',
			default: true,
			description: localize('webBrowser.enabled', 'Enable the integrated web browser.')
		},
		'webBrowser.homepage': {
			type: 'string',
			default: 'https://github.com',
			description: localize('webBrowser.homepage', 'Default homepage for new browser tabs.')
		},
		'webBrowser.searchEngine': {
			type: 'string',
			enum: ['google', 'bing', 'duckduckgo'],
			enumDescriptions: [
				localize('searchEngine.google', 'Google'),
				localize('searchEngine.bing', 'Bing'),
				localize('searchEngine.duckduckgo', 'DuckDuckGo')
			],
			default: 'google',
			description: localize('webBrowser.searchEngine', 'Default search engine.')
		},
		'webBrowser.security.sandbox': {
			type: 'boolean',
			default: true,
			description: localize('webBrowser.security.sandbox', 'Enable sandboxing for web content.')
		},
		'webBrowser.tabs.maximum': {
			type: 'number',
			default: 20,
			minimum: 1,
			maximum: 100,
			description: localize('webBrowser.tabs.maximum', 'Maximum number of browser tabs.')
		},
		'webBrowser.cache.enabled': {
			type: 'boolean',
			default: true,
			description: localize('webBrowser.cache.enabled', 'Enable caching of web content.')
		},
		'webBrowser.devTools.enabled': {
			type: 'boolean',
			default: true,
			description: localize('webBrowser.devTools.enabled', 'Enable developer tools for web content.')
		}
	}
});