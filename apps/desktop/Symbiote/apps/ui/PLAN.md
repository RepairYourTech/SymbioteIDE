# UI - Modern Web Application Plan

## Goals & Vision

The `ui` app provides a modern, responsive web application for Symbiote that delivers a premium user experience across all devices and browsers. It offers:

- **Progressive Web App**: Modern PWA with offline capabilities and native-like experience
- **Responsive Design**: Seamless experience across desktop, tablet, and mobile devices
- **Real-Time Collaboration**: Live collaboration features with WebSocket integration
- **Component-Based Architecture**: Modular, reusable components with consistent design
- **Performance Optimized**: Fast loading, smooth animations, and efficient rendering
- **Accessibility First**: WCAG 2.1 compliant with comprehensive accessibility features
- **Theme System**: Customizable themes with dark/light mode support
- **Micro-Frontend Architecture**: Scalable architecture with independent feature modules

This web application serves as the primary interface for users accessing Symbiote through browsers.

## Enhanced Architecture & Design

### Advanced Application Framework
```rust
/// Main application with comprehensive state management
pub struct SymbioteApp {
    router: AppRouter,
    state_manager: StateManager,
    auth_manager: AuthManager,
    theme_manager: ThemeManager,
    notification_manager: NotificationManager,
    websocket_manager: WebSocketManager,
    performance_monitor: PerformanceMonitor,
    accessibility_manager: AccessibilityManager,
    error_boundary: ErrorBoundary,
    config: AppConfig,
}

impl SymbioteApp {
    /// Initialize application with configuration
    pub async fn new(config: AppConfig) -> Result<Self, AppError>;

    /// Start application and mount to DOM
    pub async fn start(self, mount_point: &str) -> Result<(), AppError>;

    /// Handle route changes
    pub async fn navigate_to(&mut self, route: Route) -> Result<(), AppError>;

    /// Update global application state
    pub async fn update_state(&mut self, state_update: StateUpdate) -> Result<(), AppError>;

    /// Get current application state
    pub fn get_state(&self) -> &AppState;

    /// Subscribe to state changes
    pub fn subscribe_to_state(&mut self, callback: StateCallback) -> SubscriptionId;

    /// Unsubscribe from state changes
    pub fn unsubscribe(&mut self, subscription_id: SubscriptionId) -> Result<(), AppError>;

    /// Show notification to user
    pub async fn show_notification(&mut self, notification: Notification) -> Result<NotificationId, AppError>;

    /// Hide notification
    pub async fn hide_notification(&mut self, notification_id: NotificationId) -> Result<(), AppError>;

    /// Switch theme
    pub async fn set_theme(&mut self, theme: Theme) -> Result<(), AppError>;

    /// Get current theme
    pub fn get_theme(&self) -> &Theme;

    /// Handle authentication
    pub async fn authenticate(&mut self, credentials: Credentials) -> Result<AuthResult, AppError>;

    /// Logout user
    pub async fn logout(&mut self) -> Result<(), AppError>;

    /// Check authentication status
    pub fn is_authenticated(&self) -> bool;

    /// Get current user
    pub fn get_current_user(&self) -> Option<&User>;

    /// Handle WebSocket connection
    pub async fn connect_websocket(&mut self) -> Result<(), AppError>;

    /// Disconnect WebSocket
    pub async fn disconnect_websocket(&mut self) -> Result<(), AppError>;

    /// Send WebSocket message
    pub async fn send_websocket_message(&mut self, message: WebSocketMessage) -> Result<(), AppError>;

    /// Handle application errors
    pub async fn handle_error(&mut self, error: AppError) -> ErrorHandlingResult;

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> PerformanceMetrics;

    /// Configure accessibility settings
    pub async fn configure_accessibility(&mut self, settings: AccessibilitySettings) -> Result<(), AppError>;

    /// Export application state
    pub async fn export_state(&self, format: ExportFormat) -> Result<Vec<u8>, AppError>;

    /// Import application state
    pub async fn import_state(&mut self, data: &[u8], format: ImportFormat) -> Result<(), AppError>;

    /// Shutdown application gracefully
    pub async fn shutdown(self) -> Result<(), AppError>;
}
```

### Comprehensive State Management
```rust
/// Global application state with reactive updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    /// User authentication state
    pub auth: AuthState,

    /// Current route and navigation state
    pub navigation: NavigationState,

    /// UI theme and appearance settings
    pub theme: ThemeState,

    /// Application configuration
    pub config: ConfigState,

    /// Real-time data and updates
    pub realtime: RealtimeState,

    /// Error and notification state
    pub notifications: NotificationState,

    /// Performance and monitoring data
    pub performance: PerformanceState,

    /// Accessibility settings
    pub accessibility: AccessibilityState,

    /// Feature-specific states
    pub features: FeatureStates,
}

/// State manager with reactive updates and persistence
pub struct StateManager {
    current_state: AppState,
    subscribers: HashMap<SubscriptionId, StateCallback>,
    persistence: StatePersistence,
    validator: StateValidator,
    history: StateHistory,
    middleware: Vec<Box<dyn StateMiddleware>>,
}

impl StateManager {
    /// Create new state manager
    pub fn new(initial_state: AppState, config: StateConfig) -> Self;

    /// Update state with validation and notifications
    pub async fn update_state(&mut self, update: StateUpdate) -> Result<(), StateError>;

    /// Get current state
    pub fn get_state(&self) -> &AppState;

    /// Get specific state slice
    pub fn get_state_slice<T>(&self, selector: StateSelector) -> Option<&T>;

    /// Subscribe to state changes
    pub fn subscribe(&mut self, selector: StateSelector, callback: StateCallback) -> SubscriptionId;

    /// Unsubscribe from state changes
    pub fn unsubscribe(&mut self, subscription_id: SubscriptionId) -> Result<(), StateError>;

    /// Persist state to storage
    pub async fn persist_state(&self) -> Result<(), StateError>;

    /// Load state from storage
    pub async fn load_state(&mut self) -> Result<(), StateError>;

    /// Validate state consistency
    pub async fn validate_state(&self) -> Result<ValidationResult, StateError>;

    /// Get state history
    pub fn get_history(&self, limit: usize) -> Vec<StateSnapshot>;

    /// Undo last state change
    pub async fn undo(&mut self) -> Result<(), StateError>;

    /// Redo last undone change
    pub async fn redo(&mut self) -> Result<(), StateError>;

    /// Reset state to initial values
    pub async fn reset_state(&mut self) -> Result<(), StateError>;

    /// Add middleware for state processing
    pub fn add_middleware(&mut self, middleware: Box<dyn StateMiddleware>);

    /// Remove middleware
    pub fn remove_middleware(&mut self, middleware_id: MiddlewareId) -> Result<(), StateError>;

    /// Export state for backup
    pub async fn export_state(&self, format: ExportFormat) -> Result<Vec<u8>, StateError>;

    /// Import state from backup
    pub async fn import_state(&mut self, data: &[u8], format: ImportFormat) -> Result<(), StateError>;
}
```

### Advanced Component System
```rust
/// Base component trait for all UI components
pub trait Component: Send + Sync {
    type Props: Clone + PartialEq;
    type State: Clone;

    /// Component name for debugging
    fn name(&self) -> &'static str;

    /// Render component with props and state
    fn render(&self, props: &Self::Props, state: &Self::State) -> VirtualNode;

    /// Handle component lifecycle events
    fn on_mount(&mut self, props: &Self::Props) -> Result<(), ComponentError>;
    fn on_update(&mut self, old_props: &Self::Props, new_props: &Self::Props) -> Result<(), ComponentError>;
    fn on_unmount(&mut self) -> Result<(), ComponentError>;

    /// Handle user events
    fn handle_event(&mut self, event: Event) -> Result<EventResult, ComponentError>;

    /// Validate props
    fn validate_props(props: &Self::Props) -> Result<(), ValidationError>;

    /// Get component metadata
    fn get_metadata(&self) -> ComponentMetadata;

    /// Check if component should update
    fn should_update(&self, old_props: &Self::Props, new_props: &Self::Props, old_state: &Self::State, new_state: &Self::State) -> bool;

    /// Get accessibility information
    fn get_accessibility_info(&self) -> AccessibilityInfo;

    /// Handle error in component
    fn handle_error(&mut self, error: ComponentError) -> ErrorRecovery;
}

/// Advanced component manager with optimization
pub struct ComponentManager {
    components: HashMap<ComponentId, Box<dyn Component>>,
    virtual_dom: VirtualDom,
    renderer: DomRenderer,
    event_system: EventSystem,
    performance_monitor: ComponentPerformanceMonitor,
    accessibility_checker: AccessibilityChecker,
}

impl ComponentManager {
    /// Register new component
    pub fn register_component<T: Component + 'static>(&mut self, component: T) -> ComponentId;

    /// Unregister component
    pub fn unregister_component(&mut self, component_id: ComponentId) -> Result<(), ComponentError>;

    /// Render component tree
    pub async fn render_tree(&mut self, root_component: ComponentId) -> Result<RenderResult, ComponentError>;

    /// Update component props
    pub async fn update_component_props(&mut self, component_id: ComponentId, props: ComponentProps) -> Result<(), ComponentError>;

    /// Handle component event
    pub async fn handle_component_event(&mut self, component_id: ComponentId, event: Event) -> Result<(), ComponentError>;

    /// Get component performance metrics
    pub fn get_component_metrics(&self, component_id: ComponentId) -> Option<ComponentMetrics>;

    /// Validate component accessibility
    pub async fn validate_accessibility(&self, component_id: ComponentId) -> Result<AccessibilityReport, ComponentError>;

    /// Optimize component rendering
    pub async fn optimize_rendering(&mut self) -> Result<OptimizationResult, ComponentError>;

    /// Get component tree structure
    pub fn get_component_tree(&self) -> ComponentTree;

    /// Find components by criteria
    pub fn find_components(&self, criteria: ComponentCriteria) -> Vec<ComponentId>;

    /// Export component configuration
    pub async fn export_components(&self, format: ExportFormat) -> Result<Vec<u8>, ComponentError>;

    /// Import component configuration
    pub async fn import_components(&mut self, data: &[u8], format: ImportFormat) -> Result<(), ComponentError>;
}
```

### Comprehensive Error Handling
```rust
/// Comprehensive error types for UI application
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authentication error: {message}")]
    Authentication {
        message: String,
        error_code: AuthErrorCode,
        retry_allowed: bool,
    },

    #[error("Navigation error: {route}")]
    Navigation {
        route: String,
        reason: String,
        fallback_route: Option<String>,
    },

    #[error("State management error: {operation}")]
    StateManagement {
        operation: String,
        state_path: String,
        recovery_action: StateRecoveryAction,
    },

    #[error("Component error: {component_name}")]
    Component {
        component_name: String,
        error_type: ComponentErrorType,
        component_id: ComponentId,
        props_snapshot: Option<String>,
    },

    #[error("WebSocket error: {message}")]
    WebSocket {
        message: String,
        connection_state: ConnectionState,
        reconnect_strategy: ReconnectStrategy,
    },

    #[error("API error: {endpoint}")]
    Api {
        endpoint: String,
        status_code: Option<u16>,
        error_message: String,
        retry_after: Option<Duration>,
    },

    #[error("Rendering error: {details}")]
    Rendering {
        details: String,
        component_stack: Vec<String>,
        virtual_dom_state: String,
    },

    #[error("Performance issue: {metric} exceeded threshold")]
    Performance {
        metric: String,
        value: f64,
        threshold: f64,
        optimization_suggestions: Vec<String>,
    },

    #[error("Accessibility violation: {violation}")]
    Accessibility {
        violation: String,
        severity: AccessibilitySeverity,
        element_path: String,
        fix_suggestions: Vec<String>,
    },

    #[error("Theme error: {theme_name}")]
    Theme {
        theme_name: String,
        error_type: ThemeErrorType,
        fallback_theme: Option<String>,
    },

    #[error("Configuration error: {parameter}")]
    Configuration {
        parameter: String,
        value: String,
        valid_options: Vec<String>,
        default_value: String,
    },

    #[error("Storage error: {operation}")]
    Storage {
        operation: String,
        storage_type: StorageType,
        quota_exceeded: bool,
        cleanup_suggestions: Vec<String>,
    },

    #[error("Network error: {message}")]
    Network {
        message: String,
        error_type: NetworkErrorType,
        offline_mode_available: bool,
    },

    #[error("Security violation: {violation}")]
    Security {
        violation: String,
        severity: SecuritySeverity,
        action_taken: SecurityAction,
    },

    #[error("Validation error: {field}")]
    Validation {
        field: String,
        value: String,
        constraint: String,
        suggestion: Option<String>,
    },

    #[error("Internal error: {message}")]
    Internal {
        message: String,
        error_id: String,
        stack_trace: String,
        contact_support: bool,
    },
}

impl AppError {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            AppError::Authentication { retry_allowed, .. } => *retry_allowed,
            AppError::Navigation { fallback_route, .. } => fallback_route.is_some(),
            AppError::WebSocket { reconnect_strategy, .. } => !matches!(reconnect_strategy, ReconnectStrategy::None),
            AppError::Api { retry_after, .. } => retry_after.is_some(),
            AppError::Network { offline_mode_available, .. } => *offline_mode_available,
            AppError::Theme { fallback_theme, .. } => fallback_theme.is_some(),
            AppError::Storage { quota_exceeded: false, .. } => true,
            AppError::Security { .. } => false,
            AppError::Internal { .. } => false,
            _ => true,
        }
    }

    /// Get suggested recovery action
    pub fn recovery_action(&self) -> RecoveryAction {
        match self {
            AppError::Navigation { fallback_route: Some(route), .. } => RecoveryAction::NavigateToFallback(route.clone()),
            AppError::WebSocket { reconnect_strategy, .. } => RecoveryAction::ReconnectWebSocket(*reconnect_strategy),
            AppError::Api { retry_after: Some(delay), .. } => RecoveryAction::RetryAfter(*delay),
            AppError::Performance { optimization_suggestions, .. } => RecoveryAction::ApplyOptimizations(optimization_suggestions.clone()),
            AppError::Storage { cleanup_suggestions, .. } => RecoveryAction::CleanupStorage(cleanup_suggestions.clone()),
            AppError::Network { offline_mode_available: true, .. } => RecoveryAction::EnableOfflineMode,
            _ => RecoveryAction::ShowErrorMessage,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            AppError::Authentication { .. } => "Please check your login credentials and try again.".to_string(),
            AppError::Network { .. } => "Network connection issue. Please check your internet connection.".to_string(),
            AppError::Storage { quota_exceeded: true, .. } => "Storage space is full. Please free up some space.".to_string(),
            AppError::Performance { .. } => "The application is running slowly. Some optimizations will be applied.".to_string(),
            _ => "An unexpected error occurred. Please try again.".to_string(),
        }
    }
}
```

## Architecture & Design

### Core Modules

```
ui/
├── src/
│   ├── main.rs               # Application entry point
│   ├── lib.rs                # Library exports
│   ├── app/                  # Application core
│   │   ├── mod.rs
│   │   ├── app.rs            # Main app component
│   │   ├── router.rs         # Application routing
│   │   ├── state.rs          # Global state management
│   │   ├── auth.rs           # Authentication
│   │   └── error_boundary.rs # Error handling
│   ├── components/           # Reusable components
│   │   ├── mod.rs
│   │   ├── common/           # Common components
│   │   ├── forms/            # Form components
│   │   ├── layout/           # Layout components
│   │   ├── navigation/       # Navigation components
│   │   ├── data_display/     # Data display components
│   │   └── feedback/         # Feedback components
│   ├── pages/                # Page components
│   │   ├── mod.rs
│   │   ├── home.rs           # Home page
│   │   ├── assistant.rs      # AI Assistant page
│   │   ├── containers.rs     # Container management page
│   │   ├── ide.rs            # IDE page
│   │   ├── trader.rs         # Trading page
│   │   ├── workflow.rs       # Workflow builder page
│   │   ├── settings.rs       # Settings page
│   │   └── auth.rs           # Authentication pages
│   ├── features/             # Feature modules
│   │   ├── mod.rs
│   │   ├── assistant/        # Assistant feature module
│   │   ├── containers/       # Container feature module
│   │   ├── ide/              # IDE feature module
│   │   ├── trader/           # Trading feature module
│   │   ├── workflow/         # Workflow feature module
│   │   └── collaboration/    # Collaboration features
│   ├── services/             # API and service layer
│   │   ├── mod.rs
│   │   ├── api.rs            # API client
│   │   ├── websocket.rs      # WebSocket service
│   │   ├── auth.rs           # Authentication service
│   │   ├── storage.rs        # Local storage service
│   │   └── notifications.rs  # Notification service
│   ├── hooks/                # Custom hooks
│   │   ├── mod.rs
│   │   ├── use_api.rs        # API hooks
│   │   ├── use_websocket.rs  # WebSocket hooks
│   │   ├── use_auth.rs       # Authentication hooks
│   │   ├── use_theme.rs      # Theme hooks
│   │   └── use_collaboration.rs # Collaboration hooks
│   ├── utils/                # Utility functions
│   │   ├── mod.rs
│   │   ├── formatting.rs     # Data formatting
│   │   ├── validation.rs     # Input validation
│   │   ├── constants.rs      # Application constants
│   │   └── helpers.rs        # Helper functions
│   └── styles/               # Styling system
│       ├── mod.rs
│       ├── theme.rs          # Theme system
│       ├── components.rs     # Component styles
│       └── globals.rs        # Global styles
├── assets/                   # Static assets
│   ├── icons/
│   ├── images/
│   ├── fonts/
│   └── manifest.json
├── public/                   # Public files
│   ├── index.html
│   ├── sw.js                 # Service worker
│   └── robots.txt
├── tests/
│   ├── integration/
│   └── unit/
└── examples/
    ├── component_showcase.rs
    └── feature_demo.rs
```

### Key Design Principles

1. **User-Centric Design**: Intuitive, accessible interface optimized for productivity
2. **Performance First**: Fast loading, smooth interactions, and efficient rendering
3. **Component-Driven**: Modular, reusable components with consistent design
4. **Real-Time Ready**: Built-in support for real-time collaboration and updates
5. **Mobile-First**: Responsive design that works seamlessly across all devices

## APIs & Interfaces

### Main Application

```rust
use leptos::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    
    let (theme, set_theme) = create_signal(Theme::default());
    let (user, set_user) = create_signal(None::<User>);
    let (notifications, set_notifications) = create_signal(Vec::<Notification>::new());
    
    provide_context(theme);
    provide_context(set_theme);
    provide_context(user);
    provide_context(set_user);
    
    view! {
        <Html lang="en" class=move || theme.get().css_class()>
            <Head>
                <Title text="Symbiote - AI-Native Development Platform"/>
                <Meta name="description" content="Symbiote AI-Native Development Platform"/>
                <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>
                <Link rel="manifest" href="/manifest.json"/>
                <Link rel="icon" type_="image/x-icon" href="/favicon.ico"/>
            </Head>
            <Body class="antialiased">
                <Router>
                    <ErrorBoundary fallback=|errors| view! { <ErrorPage errors=errors/> }>
                        <AuthProvider>
                            <NotificationProvider notifications=notifications set_notifications=set_notifications>
                                <CollaborationProvider>
                                    <MainLayout>
                                        <Routes>
                                            <Route path="/" view=HomePage/>
                                            <Route path="/assistant" view=AssistantPage/>
                                            <Route path="/containers" view=ContainersPage/>
                                            <Route path="/ide" view=IdePage/>
                                            <Route path="/trader" view=TraderPage/>
                                            <Route path="/workflow" view=WorkflowPage/>
                                            <Route path="/settings" view=SettingsPage/>
                                            <Route path="/auth/*" view=AuthPages/>
                                            <Route path="/*any" view=NotFoundPage/>
                                        </Routes>
                                    </MainLayout>
                                </CollaborationProvider>
                            </NotificationProvider>
                        </AuthProvider>
                    </ErrorBoundary>
                </Router>
            </Body>
        </Html>
    }
}

#[component]
pub fn MainLayout(children: Children) -> impl IntoView {
    let user = use_context::<ReadSignal<Option<User>>>().unwrap();
    let theme = use_context::<ReadSignal<Theme>>().unwrap();
    
    view! {
        <div class="min-h-screen bg-background text-foreground">
            <Header/>
            <div class="flex">
                <Sidebar/>
                <main class="flex-1 p-6">
                    {children()}
                </main>
            </div>
            <NotificationContainer/>
            <CommandPalette/>
        </div>
    }
}
```

### Component System

```rust
// Button Component
#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional)] on_click: Option<Box<dyn Fn()>>,
    children: Children,
) -> impl IntoView {
    let button_class = move || {
        format!(
            "inline-flex items-center justify-center rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 {} {}",
            variant.css_class(),
            size.css_class()
        )
    };
    
    view! {
        <button
            class=button_class
            disabled=disabled || loading
            on:click=move |_| {
                if let Some(handler) = &on_click {
                    handler();
                }
            }
        >
            <Show when=move || loading>
                <Spinner class="mr-2 h-4 w-4"/>
            </Show>
            {children()}
        </button>
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonVariant {
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
    Link,
}

impl ButtonVariant {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Default => "bg-primary text-primary-foreground hover:bg-primary/90",
            Self::Destructive => "bg-destructive text-destructive-foreground hover:bg-destructive/90",
            Self::Outline => "border border-input bg-background hover:bg-accent hover:text-accent-foreground",
            Self::Secondary => "bg-secondary text-secondary-foreground hover:bg-secondary/80",
            Self::Ghost => "hover:bg-accent hover:text-accent-foreground",
            Self::Link => "text-primary underline-offset-4 hover:underline",
        }
    }
}

// Input Component
#[component]
pub fn Input(
    #[prop(optional)] input_type: InputType,
    #[prop(optional)] placeholder: String,
    #[prop(optional)] value: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] error: Option<String>,
    #[prop(optional)] on_input: Option<Box<dyn Fn(String)>>,
) -> impl IntoView {
    let (internal_value, set_internal_value) = create_signal(value);
    
    let input_class = move || {
        format!(
            "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 {}",
            if error.is_some() { "border-destructive" } else { "" }
        )
    };
    
    view! {
        <div class="space-y-2">
            <input
                type=input_type.as_str()
                class=input_class
                placeholder=placeholder
                value=internal_value
                disabled=disabled
                on:input=move |ev| {
                    let value = event_target_value(&ev);
                    set_internal_value(value.clone());
                    if let Some(handler) = &on_input {
                        handler(value);
                    }
                }
            />
            <Show when=move || error.is_some()>
                <p class="text-sm text-destructive">
                    {error.clone().unwrap_or_default()}
                </p>
            </Show>
        </div>
    }
}

// Additional Core Components
#[component]
pub fn Card(
    #[prop(optional)] class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("rounded-lg border bg-card text-card-foreground shadow-sm {}", class)>
            {children()}
        </div>
    }
}

#[component]
pub fn Badge(
    #[prop(optional)] variant: BadgeVariant,
    children: Children,
) -> impl IntoView {
    let badge_class = move || {
        format!(
            "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 {}",
            variant.css_class()
        )
    };

    view! {
        <div class=badge_class>
            {children()}
        </div>
    }
}

#[component]
pub fn Modal(
    #[prop(optional)] open: bool,
    #[prop(optional)] on_close: Option<Box<dyn Fn()>>,
    children: Children,
) -> impl IntoView {
    view! {
        <Show when=move || open>
            <div class="fixed inset-0 z-50 bg-background/80 backdrop-blur-sm">
                <div class="fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border bg-background p-6 shadow-lg duration-200 sm:rounded-lg">
                    <button
                        class="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                        on:click=move |_| {
                            if let Some(handler) = &on_close {
                                handler();
                            }
                        }
                    >
                        "✕"
                    </button>
                    {children()}
                </div>
            </div>
        </Show>
    }
}

#[component]
pub fn Tabs(
    #[prop(optional)] default_value: String,
    children: Children,
) -> impl IntoView {
    let (active_tab, set_active_tab) = create_signal(default_value);

    provide_context(active_tab);
    provide_context(set_active_tab);

    view! {
        <div class="w-full">
            {children()}
        </div>
    }
}

#[component]
pub fn TabsList(children: Children) -> impl IntoView {
    view! {
        <div class="inline-flex h-10 items-center justify-center rounded-md bg-muted p-1 text-muted-foreground">
            {children()}
        </div>
    }
}

#[component]
pub fn TabsTrigger(
    value: String,
    children: Children,
) -> impl IntoView {
    let active_tab = use_context::<ReadSignal<String>>().unwrap();
    let set_active_tab = use_context::<WriteSignal<String>>().unwrap();

    let is_active = move || active_tab.get() == value;

    view! {
        <button
            class=move || format!(
                "inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 {}",
                if is_active() { "bg-background text-foreground shadow-sm" } else { "hover:bg-background/50" }
            )
            on:click=move |_| set_active_tab(value.clone())
        >
            {children()}
        </button>
    }
}

#[component]
pub fn TabsContent(
    value: String,
    children: Children,
) -> impl IntoView {
    let active_tab = use_context::<ReadSignal<String>>().unwrap();

    view! {
        <Show when=move || active_tab.get() == value>
            <div class="mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2">
                {children()}
            </div>
        </Show>
    }
}
```

### State Management

```rust
// Global State Management
#[derive(Debug, Clone)]
pub struct AppState {
    pub user: Option<User>,
    pub theme: Theme,
    pub workspace: Option<Workspace>,
    pub notifications: Vec<Notification>,
    pub loading_states: HashMap<String, bool>,
    pub error_states: HashMap<String, String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            user: None,
            theme: Theme::default(),
            workspace: None,
            notifications: Vec::new(),
            loading_states: HashMap::new(),
            error_states: HashMap::new(),
        }
    }
}

// State Actions
#[derive(Debug, Clone)]
pub enum AppAction {
    SetUser(Option<User>),
    SetTheme(Theme),
    SetWorkspace(Option<Workspace>),
    AddNotification(Notification),
    RemoveNotification(String),
    SetLoading(String, bool),
    SetError(String, Option<String>),
    ClearErrors,
}

// State Reducer
pub fn app_reducer(state: AppState, action: AppAction) -> AppState {
    match action {
        AppAction::SetUser(user) => AppState { user, ..state },
        AppAction::SetTheme(theme) => AppState { theme, ..state },
        AppAction::SetWorkspace(workspace) => AppState { workspace, ..state },
        AppAction::AddNotification(notification) => {
            let mut notifications = state.notifications;
            notifications.push(notification);
            AppState { notifications, ..state }
        },
        AppAction::RemoveNotification(id) => {
            let notifications = state.notifications.into_iter()
                .filter(|n| n.id != id)
                .collect();
            AppState { notifications, ..state }
        },
        AppAction::SetLoading(key, loading) => {
            let mut loading_states = state.loading_states;
            if loading {
                loading_states.insert(key, true);
            } else {
                loading_states.remove(&key);
            }
            AppState { loading_states, ..state }
        },
        AppAction::SetError(key, error) => {
            let mut error_states = state.error_states;
            if let Some(error) = error {
                error_states.insert(key, error);
            } else {
                error_states.remove(&key);
            }
            AppState { error_states, ..state }
        },
        AppAction::ClearErrors => AppState {
            error_states: HashMap::new(),
            ..state
        },
    }
}

// State Provider
#[component]
pub fn StateProvider(children: Children) -> impl IntoView {
    let (state, dispatch) = create_reducer(AppState::default, app_reducer);

    provide_context(state);
    provide_context(dispatch);

    view! {
        {children()}
    }
}

// State Hooks
pub fn use_app_state() -> (ReadSignal<AppState>, impl Fn(AppAction) + Clone) {
    let state = use_context::<ReadSignal<AppState>>()
        .expect("AppState context not found");
    let dispatch = use_context::<impl Fn(AppAction) + Clone>()
        .expect("AppDispatch context not found");

    (state, dispatch)
}

pub fn use_user() -> (ReadSignal<Option<User>>, impl Fn(Option<User>) + Clone) {
    let (state, dispatch) = use_app_state();
    let user = create_memo(move |_| state.get().user);
    let set_user = move |user: Option<User>| dispatch(AppAction::SetUser(user));

    (user.into(), set_user)
}

pub fn use_theme() -> (ReadSignal<Theme>, impl Fn(Theme) + Clone) {
    let (state, dispatch) = use_app_state();
    let theme = create_memo(move |_| state.get().theme);
    let set_theme = move |theme: Theme| dispatch(AppAction::SetTheme(theme));

    (theme.into(), set_theme)
}

pub fn use_loading(key: &str) -> (ReadSignal<bool>, impl Fn(bool) + Clone) {
    let (state, dispatch) = use_app_state();
    let key = key.to_string();

    let loading = create_memo(move |_| {
        state.get().loading_states.get(&key).copied().unwrap_or(false)
    });

    let set_loading = {
        let key = key.clone();
        move |loading: bool| dispatch(AppAction::SetLoading(key.clone(), loading))
    };

    (loading.into(), set_loading)
}
```

### Feature Modules

```rust
// Assistant Feature Module
pub mod assistant {
    use super::*;
    
    #[component]
    pub fn AssistantChat() -> impl IntoView {
        let (messages, set_messages) = create_signal(Vec::<ChatMessage>::new());
        let (input_value, set_input_value) = create_signal(String::new());
        let (is_loading, set_is_loading) = create_signal(false);
        
        let send_message = create_action(|message: &String| {
            let message = message.clone();
            async move {
                // Send message to assistant API
                let response = send_assistant_message(message).await?;
                Ok::<AssistantResponse, ApiError>(response)
            }
        });
        
        view! {
            <div class="flex flex-col h-full">
                <div class="flex-1 overflow-y-auto p-4 space-y-4">
                    <For
                        each=move || messages.get()
                        key=|msg| msg.id
                        children=move |message| {
                            view! {
                                <ChatMessageComponent message=message/>
                            }
                        }
                    />
                    <Show when=move || is_loading.get()>
                        <TypingIndicator/>
                    </Show>
                </div>
                <div class="border-t p-4">
                    <ChatInput
                        value=input_value
                        on_send=move |msg| {
                            send_message.dispatch(msg);
                        }
                        disabled=is_loading
                    />
                </div>
            </div>
        }
    }
    
    #[component]
    pub fn ChatMessageComponent(message: ChatMessage) -> impl IntoView {
        view! {
            <div class=format!(
                "flex {}",
                if message.is_user { "justify-end" } else { "justify-start" }
            )>
                <div class=format!(
                    "max-w-xs lg:max-w-md px-4 py-2 rounded-lg {}",
                    if message.is_user {
                        "bg-primary text-primary-foreground"
                    } else {
                        "bg-muted"
                    }
                )>
                    <p class="text-sm">{message.content}</p>
                    <p class="text-xs opacity-70 mt-1">
                        {message.timestamp.format("%H:%M").to_string()}
                    </p>
                </div>
            </div>
        }
    }
}

// Container Feature Module
pub mod containers {
    use super::*;
    
    #[component]
    pub fn ContainerDashboard() -> impl IntoView {
        let containers = create_resource(|| (), |_| async { get_containers().await });
        
        view! {
            <div class="space-y-6">
                <div class="flex justify-between items-center">
                    <h1 class="text-3xl font-bold">Containers</h1>
                    <Button on_click=Box::new(|| {
                        // Open create container dialog
                    })>
                        "Create Container"
                    </Button>
                </div>
                
                <Suspense fallback=move || view! { <ContainerSkeleton/> }>
                    {move || {
                        containers.get().map(|containers| match containers {
                            Ok(containers) => view! {
                                <ContainerGrid containers=containers/>
                            }.into_view(),
                            Err(e) => view! {
                                <ErrorMessage error=e.to_string()/>
                            }.into_view(),
                        })
                    }}
                </Suspense>
            </div>
        }
    }
    
    #[component]
    pub fn ContainerCard(container: Container) -> impl IntoView {
        let status_color = match container.status {
            ContainerStatus::Running => "text-green-500",
            ContainerStatus::Stopped => "text-red-500",
            ContainerStatus::Pending => "text-yellow-500",
        };
        
        view! {
            <div class="border rounded-lg p-4 hover:shadow-md transition-shadow">
                <div class="flex justify-between items-start mb-2">
                    <h3 class="font-semibold">{container.name}</h3>
                    <Badge variant=BadgeVariant::Secondary>
                        <span class=status_color>{container.status.to_string()}</span>
                    </Badge>
                </div>
                <p class="text-sm text-muted-foreground mb-4">{container.image}</p>
                <div class="flex space-x-2">
                    <Button size=ButtonSize::Small variant=ButtonVariant::Outline>
                        "Logs"
                    </Button>
                    <Button size=ButtonSize::Small variant=ButtonVariant::Outline>
                        "Shell"
                    </Button>
                    <Button size=ButtonSize::Small variant=ButtonVariant::Destructive>
                        "Stop"
                    </Button>
                </div>
            </div>
        }
    }
}
```

### Real-Time Collaboration

```rust
pub mod collaboration {
    use super::*;
    
    #[component]
    pub fn CollaborationProvider(children: Children) -> impl IntoView {
        let (connected_users, set_connected_users) = create_signal(Vec::<ConnectedUser>::new());
        let (cursors, set_cursors) = create_signal(HashMap::<String, CursorPosition>::new());
        
        // WebSocket connection for real-time collaboration
        let ws = use_websocket("/ws/collaboration");
        
        create_effect(move |_| {
            if let Some(message) = ws.message.get() {
                match serde_json::from_str::<CollaborationMessage>(&message) {
                    Ok(CollaborationMessage::UserJoined(user)) => {
                        set_connected_users.update(|users| users.push(user));
                    }
                    Ok(CollaborationMessage::UserLeft(user_id)) => {
                        set_connected_users.update(|users| {
                            users.retain(|u| u.id != user_id);
                        });
                    }
                    Ok(CollaborationMessage::CursorUpdate { user_id, position }) => {
                        set_cursors.update(|cursors| {
                            cursors.insert(user_id, position);
                        });
                    }
                    _ => {}
                }
            }
        });
        
        provide_context(connected_users);
        provide_context(set_connected_users);
        provide_context(cursors);
        provide_context(set_cursors);
        
        view! {
            <div class="relative">
                {children()}
                <CollaborativeCursors/>
                <UserPresenceIndicator/>
            </div>
        }
    }
    
    #[component]
    pub fn CollaborativeCursors() -> impl IntoView {
        let cursors = use_context::<ReadSignal<HashMap<String, CursorPosition>>>().unwrap();
        
        view! {
            <div class="fixed inset-0 pointer-events-none z-50">
                <For
                    each=move || cursors.get().into_iter().collect::<Vec<_>>()
                    key=|(user_id, _)| user_id.clone()
                    children=move |(user_id, position)| {
                        view! {
                            <div
                                class="absolute w-4 h-4 transform -translate-x-1/2 -translate-y-1/2 pointer-events-none"
                                style=format!("left: {}px; top: {}px;", position.x, position.y)
                            >
                                <div class="w-4 h-4 bg-blue-500 rounded-full border-2 border-white shadow-lg">
                                    <div class="absolute top-4 left-1/2 transform -translate-x-1/2 bg-blue-500 text-white text-xs px-2 py-1 rounded whitespace-nowrap">
                                        {user_id}
                                    </div>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        }
    }
}
```

### API Service Layer

```rust
pub mod services {
    use super::*;
    
    pub struct ApiClient {
        base_url: String,
        auth_token: Option<String>,
        client: reqwest::Client,
    }
    
    impl ApiClient {
        pub fn new(base_url: String) -> Self {
            Self {
                base_url,
                auth_token: None,
                client: reqwest::Client::new(),
            }
        }
        
        pub async fn get<T>(&self, endpoint: &str) -> Result<T, ApiError>
        where
            T: serde::de::DeserializeOwned,
        {
            let url = format!("{}{}", self.base_url, endpoint);
            let mut request = self.client.get(&url);
            
            if let Some(token) = &self.auth_token {
                request = request.bearer_auth(token);
            }
            
            let response = request.send().await?;
            
            if response.status().is_success() {
                let data = response.json::<T>().await?;
                Ok(data)
            } else {
                Err(ApiError::HttpError(response.status()))
            }
        }
        
        pub async fn post<T, R>(&self, endpoint: &str, data: &T) -> Result<R, ApiError>
        where
            T: serde::Serialize,
            R: serde::de::DeserializeOwned,
        {
            let url = format!("{}{}", self.base_url, endpoint);
            let mut request = self.client.post(&url).json(data);
            
            if let Some(token) = &self.auth_token {
                request = request.bearer_auth(token);
            }
            
            let response = request.send().await?;
            
            if response.status().is_success() {
                let result = response.json::<R>().await?;
                Ok(result)
            } else {
                Err(ApiError::HttpError(response.status()))
            }
        }
    }
    
    // WebSocket service
    pub fn use_websocket(url: &str) -> WebSocketService {
        let (message, set_message) = create_signal(None::<String>);
        let (connected, set_connected) = create_signal(false);
        
        // WebSocket connection logic here
        
        WebSocketService {
            message: message.into(),
            connected: connected.into(),
            send: Box::new(move |msg| {
                // Send message implementation
            }),
        }
    }
    
    pub struct WebSocketService {
        pub message: Signal<Option<String>>,
        pub connected: Signal<bool>,
        pub send: Box<dyn Fn(String)>,
    }
}
```

## Implementation Details

### Technology Stack

- **Frontend Framework**: Leptos for reactive web components
- **Styling**: Tailwind CSS with custom design system
- **Build Tool**: Trunk for Rust/WASM compilation
- **State Management**: Leptos signals and context
- **Routing**: Leptos Router for client-side routing
- **HTTP Client**: reqwest for API communication
- **WebSocket**: Native WebSocket API with Leptos integration
- **PWA**: Service worker for offline capabilities

### Key Dependencies

```toml
[dependencies]
leptos = { version = "0.6", features = ["csr"] }
leptos_router = "0.6"
leptos_meta = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
wasm-bindgen = "0.2"
web-sys = "0.3"
js-sys = "0.3"
gloo = "0.11"
reqwest = { version = "0.11", features = ["json"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde", "wasmbind"] }
thiserror = "1.0"
console_error_panic_hook = "0.1"
tracing = "0.1"
tracing-wasm = "0.2"

[build-dependencies]
trunk = "0.18"
```

### Progressive Web App Features

```javascript
// Service Worker (sw.js)
const CACHE_NAME = 'symbiote-v1';
const urlsToCache = [
  '/',
  '/static/css/main.css',
  '/static/js/main.js',
  '/manifest.json'
];

self.addEventListener('install', event => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => cache.addAll(urlsToCache))
  );
});

self.addEventListener('fetch', event => {
  event.respondWith(
    caches.match(event.request)
      .then(response => {
        if (response) {
          return response;
        }
        return fetch(event.request);
      }
    )
  );
});

// Push notification handling
self.addEventListener('push', event => {
  const options = {
    body: event.data.text(),
    icon: '/icon-192x192.png',
    badge: '/badge-72x72.png'
  };
  
  event.waitUntil(
    self.registration.showNotification('Symbiote', options)
  );
});
```

## Testing Strategy

### Unit Tests

- **Component Testing**: Test all UI components in isolation
- **Hook Testing**: Test custom hooks and state management
- **Service Testing**: Test API and WebSocket services
- **Utility Testing**: Test utility functions and helpers
- **Accessibility Testing**: Test accessibility compliance

### Integration Tests

- **User Flow Testing**: Test complete user workflows
- **API Integration**: Test integration with backend APIs
- **Real-Time Features**: Test WebSocket and collaboration features
- **Cross-Browser Testing**: Test compatibility across browsers
- **Performance Testing**: Test loading times and responsiveness

### E2E Tests

- **Critical Path Testing**: Test main user journeys
- **Mobile Testing**: Test mobile responsiveness and touch interactions
- **PWA Testing**: Test offline capabilities and installation
- **Collaboration Testing**: Test real-time collaboration features

## Integration Points

### Upstream Dependencies

- **Backend APIs**: Integration with all Symbiote backend services
- **WebSocket**: Real-time communication with backend
- **Authentication**: Integration with authentication system

### Downstream Consumers

- **End Users**: Primary interface for web-based users
- **Mobile Users**: Responsive interface for mobile devices
- **Teams**: Collaborative interface for team workflows

### External Integrations

- **Browser APIs**: Integration with modern browser capabilities
- **PWA Features**: Service worker and manifest integration
- **Analytics**: User analytics and performance monitoring
- **CDN**: Content delivery for static assets

## Acceptance Criteria

### Functional Requirements

- [ ] Responsive design working across all device sizes
- [ ] Real-time collaboration with WebSocket integration
- [ ] Progressive Web App with offline capabilities
- [ ] Complete feature parity with desktop application
- [ ] Comprehensive accessibility compliance (WCAG 2.1)
- [ ] Multi-theme support with dark/light modes
- [ ] Performance optimized with fast loading times

### Non-Functional Requirements

- [ ] Sub-3-second initial page load time
- [ ] 60fps smooth animations and interactions
- [ ] 99.9% uptime and reliability
- [ ] Cross-browser compatibility (Chrome, Firefox, Safari, Edge)
- [ ] Mobile-first responsive design
- [ ] Accessibility score of 95%+ on Lighthouse

### Quality Gates

- [ ] All tests pass with 95%+ coverage
- [ ] Performance benchmarks meet targets
- [ ] Accessibility audit passes WCAG 2.1 AA
- [ ] Cross-browser compatibility verified
- [ ] Mobile responsiveness tested on real devices
- [ ] PWA features working correctly

## Dependencies & Prerequisites

### Build Dependencies

- **Rust Toolchain**: 1.70+ with WebAssembly support
- **Node.js**: For build tools and development server
- **Trunk**: Rust/WASM build tool

### Runtime Dependencies

- **Modern Browser**: Support for WebAssembly and modern web APIs
- **Network**: Internet connectivity for API access
- **Local Storage**: Browser storage for offline capabilities

### Development Prerequisites

- **Web Development**: Modern web development knowledge
- **Rust/WASM**: Understanding of Rust WebAssembly development
- **UI/UX Design**: Knowledge of responsive design and accessibility
- **Performance Optimization**: Web performance optimization techniques

This modern web application provides a comprehensive, accessible, and performant interface for Symbiote that works seamlessly across all devices and browsers while maintaining the full feature set and real-time collaboration capabilities expected from a professional development platform.
