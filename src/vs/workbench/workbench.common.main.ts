/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

//#region --- editor/workbench core

import '../editor/editor.all.js';

import './api/browser/extensionHost.contribution.js';
import './browser/workbench.contribution.js';

//#endregion

//#region --- workbench actions

import './browser/actions/textInputActions.js';
import './browser/actions/developerActions.js';
import './browser/actions/helpActions.js';
import './browser/actions/layoutActions.js';
import './browser/actions/listCommands.js';
import './browser/actions/navigationActions.js';
import './browser/actions/windowActions.js';
import './browser/actions/workspaceActions.js';
import './browser/actions/workspaceCommands.js';
import './browser/actions/quickAccessActions.js';
import './browser/actions/widgetNavigationCommands.js';

//#endregion

//#region --- API Extension Points

import './services/actions/common/menusExtensionPoint.js';
import './api/common/configurationExtensionPoint.js';
import './api/browser/viewsExtensionPoint.js';

//#endregion

//#region --- workbench parts

import './browser/parts/editor/editor.contribution.js';
import './browser/parts/editor/editorParts.js';
import './browser/parts/paneCompositePartService.js';
import './browser/parts/banner/bannerPart.js';
import './browser/parts/statusbar/statusbarPart.js';

//#endregion

//#region --- workbench services

import '../platform/actions/common/actions.contribution.js';
import '../platform/undoRedo/common/undoRedoService.js';
import './services/workspaces/common/editSessionIdentityService.js';
import './services/workspaces/common/canonicalUriService.js';
import './services/extensions/browser/extensionUrlHandler.js';
import './services/keybinding/common/keybindingEditing.js';
import './services/decorations/browser/decorationsService.js';
import './services/dialogs/common/dialogService.js';
import './services/progress/browser/progressService.js';
import './services/editor/browser/codeEditorService.js';
import './services/preferences/browser/preferencesService.js';
import './services/configuration/common/jsonEditingService.js';
import './services/textmodelResolver/common/textModelResolverService.js';
import './services/editor/browser/editorService.js';
import './services/editor/browser/editorResolverService.js';
import './services/history/browser/historyService.js';
import './services/activity/browser/activityService.js';
import './services/keybinding/browser/keybindingService.js';
import './services/untitled/common/untitledTextEditorService.js';
import './services/textresourceProperties/common/textResourcePropertiesService.js';
import './services/textfile/common/textEditorService.js';
import './services/language/common/languageService.js';
import './services/model/common/modelService.js';
import './services/notebook/common/notebookDocumentService.js';
import './services/commands/common/commandService.js';
import './services/themes/browser/workbenchThemeService.js';
import './services/label/common/labelService.js';
import './services/extensions/common/extensionManifestPropertiesService.js';
import './services/extensionManagement/common/extensionGalleryService.js';
import './services/extensionManagement/browser/extensionEnablementService.js';
import './services/extensionManagement/browser/builtinExtensionsScannerService.js';
import './services/extensionRecommendations/common/extensionIgnoredRecommendationsService.js';
import './services/extensionRecommendations/common/workspaceExtensionsConfig.js';
import './services/extensionManagement/common/extensionFeaturesManagemetService.js';
import './services/notification/common/notificationService.js';
import './services/userDataProfile/browser/userDataProfileImportExportService.js';
import './services/userDataProfile/browser/userDataProfileManagement.js';
import './services/userDataProfile/common/remoteUserDataProfiles.js';
import './services/remote/common/remoteExplorerService.js';
import './services/remote/common/remoteExtensionsScanner.js';
import './services/terminal/common/embedderTerminalService.js';
import './services/workingCopy/common/workingCopyService.js';
import './services/workingCopy/common/workingCopyFileService.js';
import './services/workingCopy/common/workingCopyEditorService.js';
import './services/filesConfiguration/common/filesConfigurationService.js';
import './services/views/browser/viewDescriptorService.js';
import './services/views/browser/viewsService.js';
import './services/quickinput/browser/quickInputService.js';
import './services/authentication/browser/authenticationService.js';
import './services/authentication/browser/authenticationExtensionsService.js';
import './services/authentication/browser/authenticationUsageService.js';
import './services/authentication/browser/authenticationAccessService.js';
import './services/authentication/browser/dynamicAuthenticationProviderStorageService.js';
import './services/authentication/browser/authenticationQueryService.js';
import '../platform/hover/browser/hoverService.js';
import '../platform/userInteraction/browser/userInteractionServiceImpl.js';
import './services/assignment/common/assignmentService.js';
import './services/outline/browser/outlineService.js';
import './services/languageDetection/browser/languageDetectionWorkerServiceImpl.js';
import '../editor/common/services/languageFeaturesService.js';
// SideX: removed — semantic tokens disabled (contributions removed in editor.all.ts)
// import '../editor/common/services/semanticTokensStylingService.js';
import '../editor/common/services/treeViewsDndService.js';
import './services/textMate/browser/textMateTokenizationFeature.contribution.js';
import './services/treeSitter/browser/treeSitter.contribution.js';
import './services/userActivity/common/userActivityService.js';
import './services/userActivity/browser/userActivityBrowser.js';
import './services/userAttention/browser/userAttentionBrowser.js';
import './services/editor/browser/editorPaneService.js';
import './services/editor/common/customEditorLabelService.js';
import './services/dataChannel/browser/dataChannelService.js';
import './services/log/common/defaultLogLevels.js';

import { InstantiationType, registerSingleton } from '../platform/instantiation/common/extensions.js';
import { GlobalExtensionEnablementService } from '../platform/extensionManagement/common/extensionEnablementService.js';
import {
	IAllowedExtensionsService,
	IGlobalExtensionEnablementService
} from '../platform/extensionManagement/common/extensionManagement.js';
import { ContextViewService } from '../platform/contextview/browser/contextViewService.js';
import { IContextViewService } from '../platform/contextview/browser/contextView.js';
import { IListService, ListService } from '../platform/list/browser/listService.js';
import { MarkerDecorationsService } from '../editor/common/services/markerDecorationsService.js';
import { IMarkerDecorationsService } from '../editor/common/services/markerDecorations.js';
import { IMarkerService } from '../platform/markers/common/markers.js';
import { MarkerService } from '../platform/markers/common/markerService.js';
import { ContextKeyService } from '../platform/contextkey/browser/contextKeyService.js';
import { IContextKeyService } from '../platform/contextkey/common/contextkey.js';
import { ITextResourceConfigurationService } from '../editor/common/services/textResourceConfiguration.js';
import { TextResourceConfigurationService } from '../editor/common/services/textResourceConfigurationService.js';
import { IDownloadService } from '../platform/download/common/download.js';
import { DownloadService } from '../platform/download/common/downloadService.js';
import { OpenerService } from '../editor/browser/services/openerService.js';
import { IOpenerService } from '../platform/opener/common/opener.js';
import {
	ExtensionStorageService,
	IExtensionStorageService
} from '../platform/extensionManagement/common/extensionStorage.js';
// Null UserDataSync stubs (avoid importing heavy userDataSync modules)
import '../platform/userDataSync/common/nullUserDataSync.js';
import { AllowedExtensionsService } from '../platform/extensionManagement/common/allowedExtensionsService.js';
import { IWebWorkerService } from '../platform/webWorker/browser/webWorkerService.js';
import { WebWorkerService } from '../platform/webWorker/browser/webWorkerServiceImpl.js';

registerSingleton(IAllowedExtensionsService, AllowedExtensionsService, InstantiationType.Delayed);
registerSingleton(IGlobalExtensionEnablementService, GlobalExtensionEnablementService, InstantiationType.Delayed);
registerSingleton(IExtensionStorageService, ExtensionStorageService, InstantiationType.Delayed);
registerSingleton(IContextViewService, ContextViewService, InstantiationType.Delayed);
registerSingleton(IListService, ListService, InstantiationType.Delayed);
registerSingleton(IMarkerDecorationsService, MarkerDecorationsService, InstantiationType.Delayed);
registerSingleton(IMarkerService, MarkerService, InstantiationType.Delayed);
registerSingleton(IContextKeyService, ContextKeyService, InstantiationType.Delayed);
registerSingleton(ITextResourceConfigurationService, TextResourceConfigurationService, InstantiationType.Delayed);
registerSingleton(IDownloadService, DownloadService, InstantiationType.Delayed);
registerSingleton(IOpenerService, OpenerService, InstantiationType.Delayed);
registerSingleton(IWebWorkerService, WebWorkerService, InstantiationType.Delayed);

//#endregion

//#region --- workbench contributions

// Default Account (null stub)
import './services/accounts/browser/nullDefaultAccount.js';

// Telemetry — removed: SideX has its own telemetry
// import './contrib/telemetry/browser/telemetry.contribution.js';

// Preferences
import './contrib/preferences/browser/preferences.contribution.js';
import './contrib/preferences/browser/keybindingsEditorContribution.js';
import './contrib/preferences/browser/preferencesSearch.js';

// Performance — removed: SideX has its own performance monitoring
// import './contrib/performance/browser/performance.contribution.js';

// SideX: removed — notebook, interactive, REPL (huge, ~80K lines, not needed)
// import './contrib/notebook/browser/notebook.contribution.js';
// import './contrib/interactive/browser/interactive.contribution.js';
// import './contrib/replNotebook/browser/repl.contribution.js';

// Testing
import './contrib/testing/browser/testing.contribution.js';

// Logs
import './contrib/logs/common/logs.contribution.js';

// Quickaccess
import './contrib/quickaccess/browser/quickAccess.contribution.js';

// Explorer
import './contrib/files/browser/explorerViewlet.js';
import './contrib/files/browser/fileActions.contribution.js';
import './contrib/files/browser/files.contribution.js';

// SideX: removed — bulk edit handled by Rust
// import './contrib/bulkEdit/browser/bulkEditService.js';
// import './contrib/bulkEdit/browser/preview/bulkEdit.contribution.js';

// Search
import './contrib/search/browser/search.contribution.js';
import './contrib/search/browser/searchView.js';

// Search Editor
import './contrib/searchEditor/browser/searchEditor.contribution.js';

// Sash
import './contrib/sash/browser/sash.contribution.js';

// SCM
import './contrib/scm/browser/scm.contribution.js';
import './contrib/scm/browser/git.contribution.js';

// Debug
import './contrib/debug/browser/debug.contribution.js';
import './contrib/debug/browser/debugEditorContribution.js';
import './contrib/debug/browser/breakpointEditorContribution.js';
import './contrib/debug/browser/callStackEditorContribution.js';
import './contrib/debug/browser/repl.js';
import './contrib/debug/browser/debugViewlet.js';

// Markers
import './contrib/markers/browser/markers.contribution.js';

// SideX: removed — process explorer not needed
// import './contrib/processExplorer/browser/processExplorer.contribution.js';

// SideX: removed — merge editor not needed
// import './contrib/mergeEditor/browser/mergeEditor.contribution.js';

// SideX: removed — multi diff editor not needed
// import './contrib/multiDiffEditor/browser/multiDiffEditor.contribution.js';

// Commands
import './contrib/commands/common/commands.contribution.js';

// SideX: removed — comments (PR comments, not core)
// import './contrib/comments/browser/comments.contribution.js';

// URL Support
import './contrib/url/browser/url.contribution.js';

// Webview
import './contrib/webview/browser/webview.contribution.js';
import './contrib/webviewPanel/browser/webviewPanel.contribution.js';
import './contrib/webviewView/browser/webviewView.contribution.js';
// SideX: removed — custom editor providers
// import './contrib/customEditor/browser/customEditor.contribution.js';

// SideX: removed — image preview
// import './contrib/imagePreview/browser/imagePreview.contribution.js';

// SideX: removed — external URI opener
// import './contrib/externalUriOpener/common/externalUriOpener.contribution.js';

// Extensions Management
import './contrib/extensions/browser/extensions.contribution.js';
import './contrib/extensions/browser/extensionsViewlet.js';

// Output View
import './contrib/output/browser/output.contribution.js';
import './contrib/output/browser/outputView.js';

// Terminal
import './contrib/terminal/terminal.all.js';

// Tauri Terminal Backend (registered via terminal.contribution.ts)

// External terminal
import './contrib/externalTerminal/browser/externalTerminal.contribution.js';

// Relauncher — removed: Electron-specific, not needed in Tauri
// import './contrib/relauncher/browser/relauncher.contribution.js';

// Tasks
import './contrib/tasks/browser/task.contribution.js';

// SideX: removed — emmet handled by sidex-editor
// import './contrib/emmet/browser/emmet.contribution.js';

// SideX: removed — code editor contributions handled by Rust
// import './contrib/codeEditor/browser/codeEditor.contribution.js';

// Markdown
import './contrib/markdown/browser/markdown.contribution.js';

// Keybindings Contributions
import './contrib/keybindings/browser/keybindings.contribution.js';

// Snippets
import './contrib/snippets/browser/snippets.contribution.js';

// Formatter Help
import './contrib/format/browser/format.contribution.js';

// Folding
import './contrib/folding/browser/folding.contribution.js';

// Limit Indicator
import './contrib/limitIndicator/browser/limitIndicator.contribution.js';

// SideX: removed — inlay hints handled by Rust
// import './contrib/inlayHints/browser/inlayHintsAccessibilty.js';

// Themes
import './contrib/themes/browser/themes.contribution.js';

// Update — removed: SideX uses Tauri updater
// import './contrib/update/browser/update.contribution.js';

// Surveys — removed: VS Code telemetry surveys not needed
// import './contrib/surveys/browser/nps.contribution.js';
// import './contrib/surveys/browser/languageSurveys.contribution.js';

// Welcome — removed: SideX has its own welcome experience
// import './contrib/welcomeGettingStarted/browser/gettingStarted.contribution.js';
// import './contrib/welcomeWalkthrough/browser/walkThrough.contribution.js';
// SideX: removed — welcome views
// import './contrib/welcomeViews/common/viewsWelcome.contribution.js';
// import './contrib/welcomeViews/common/newFile.contribution.js';

// SideX: removed — call hierarchy handled by sidex-lsp
// import './contrib/callHierarchy/browser/callHierarchy.contribution.js';

// SideX: removed — type hierarchy handled by sidex-lsp
// import './contrib/typeHierarchy/browser/typeHierarchy.contribution.js';

// SideX: removed — outline handled by sidex-workspace
// import './contrib/codeEditor/browser/outline/documentSymbolsOutline.js';
// import './contrib/outline/browser/outline.contribution.js';

// SideX: removed — language detection
// import './contrib/languageDetection/browser/languageDetection.contribution.js';

// Language Status
import './contrib/languageStatus/browser/languageStatus.contribution.js';

// SideX: removed — authentication contribution
// import './contrib/authentication/browser/authentication.contribution.js';

// User Data Profiles — removed: SideX handles user data profiles in Rust backend
// import './contrib/userDataProfile/browser/userDataProfile.contribution.js';

// SideX: removed — code actions handled by sidex-editor
// import './contrib/codeActions/browser/codeActions.contribution.js';

// SideX: removed — timeline
// import './contrib/timeline/browser/timeline.contribution.js';

// Local History — removed: SideX: handled by Rust backend
// import './contrib/localHistory/browser/localHistory.contribution.js';

// Workspace
import './contrib/workspace/browser/workspace.contribution.js';

// Workspaces
import './contrib/workspaces/browser/workspaces.contribution.js';

// SideX: removed — list widget contributions
// import './contrib/list/browser/list.contribution.js';

// SideX: removed — accessibility signals
import './contrib/accessibilitySignals/browser/accessibilitySignal.contribution.js';

// SideX: removed — bracket pair colorizer telemetry
// import './contrib/bracketPairColorizer2Telemetry/browser/bracketPairColorizer2Telemetry.contribution.js';

// SideX: removed — accessibility handled by Rust
// import './contrib/accessibility/browser/accessibility.contribution.js';

// SideX: removed — metered connection
// import './contrib/meteredConnection/browser/meteredConnection.contribution.js';

// SideX: removed — share feature
// import './contrib/share/browser/share.contribution.js';

// SideX: removed — synchronized scrolling
// import './contrib/scrollLocking/browser/scrollLocking.contribution.js';

// SideX: removed — drop/paste into
// import './contrib/dropOrPasteInto/browser/dropOrPasteInto.contribution.js';

// Opener
import './contrib/opener/browser/opener.contribution.js';

// Null stubs for stripped services
import '../editor/browser/services/renameSymbolTrackerService.js';

//#endregion
