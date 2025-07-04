/**
 * Storyboard Types
 *
 * Type definitions for the app storyboard system
 */
export declare enum StoryboardType {
    UserFlow = "user-flow",
    ComponentHierarchy = "component-hierarchy",
    StateMachine = "state-machine",
    APISequence = "api-sequence",
    DataFlow = "data-flow",
    NavigationMap = "navigation-map"
}
export declare enum SceneType {
    Screen = "screen",
    Component = "component",
    State = "state",
    API = "api",
    Decision = "decision",
    Process = "process"
}
export declare enum TransitionType {
    Navigation = "navigation",
    Action = "action",
    StateChange = "state-change",
    APICall = "api-call",
    DataFlow = "data-flow",
    Conditional = "conditional"
}
export interface Storyboard {
    id: string;
    name: string;
    description?: string;
    type: StoryboardType;
    scenes: Scene[];
    flows: Flow[];
    metadata: StoryboardMetadata;
    createdAt: Date;
    updatedAt: Date;
    version: string;
}
export interface Scene {
    id: string;
    type: SceneType;
    name: string;
    description?: string;
    position: Position;
    size?: Size;
    data: SceneData;
    style?: SceneStyle;
    metadata?: Record<string, any>;
}
export interface SceneData {
    route?: string;
    component?: string;
    layout?: string;
    componentPath?: string;
    props?: Record<string, any>;
    children?: string[];
    stateName?: string;
    initialValue?: any;
    reducers?: string[];
    endpoint?: string;
    method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
    requestSchema?: any;
    responseSchema?: any;
    condition?: string;
    branches?: Branch[];
    steps?: ProcessStep[];
    code?: string;
    documentation?: string;
    tags?: string[];
}
export interface Flow {
    id: string;
    type: TransitionType;
    source: string;
    target: string;
    label?: string;
    data?: FlowData;
    style?: FlowStyle;
    metadata?: Record<string, any>;
}
export interface FlowData {
    trigger?: 'click' | 'load' | 'submit' | 'custom';
    params?: Record<string, any>;
    actionType?: string;
    payload?: any;
    mutation?: string;
    transform?: string;
    request?: any;
    response?: any;
    errorHandling?: string;
    condition?: string;
    trueTarget?: string;
    falseTarget?: string;
}
export interface Position {
    x: number;
    y: number;
}
export interface Size {
    width: number;
    height: number;
}
export interface Branch {
    condition: string;
    targetId: string;
    label?: string;
}
export interface ProcessStep {
    id: string;
    action: string;
    description?: string;
    duration?: number;
}
export interface SceneStyle {
    backgroundColor?: string;
    borderColor?: string;
    borderWidth?: number;
    borderStyle?: 'solid' | 'dashed' | 'dotted';
    textColor?: string;
    fontSize?: number;
    fontWeight?: 'normal' | 'bold';
    icon?: string;
    shape?: 'rectangle' | 'rounded' | 'circle' | 'diamond' | 'hexagon';
}
export interface FlowStyle {
    strokeColor?: string;
    strokeWidth?: number;
    strokeDasharray?: string;
    arrowHead?: 'arrow' | 'circle' | 'diamond' | 'none';
    animated?: boolean;
    curve?: 'straight' | 'smooth' | 'step';
}
export interface StoryboardMetadata {
    author?: string;
    tags?: string[];
    framework?: 'react' | 'vue' | 'angular' | 'svelte' | 'vanilla';
    projectId?: string;
    teamId?: string;
    designSystem?: string;
    createdWith?: string;
    lastModifiedBy?: string;
}
export interface MermaidDiagram {
    type: MermaidDiagramType;
    code: string;
    theme?: MermaidTheme;
    config?: MermaidConfig;
}
export declare enum MermaidDiagramType {
    Flowchart = "flowchart",
    Sequence = "sequenceDiagram",
    State = "stateDiagram-v2",
    Class = "classDiagram",
    ER = "erDiagram",
    Journey = "journey",
    Gantt = "gantt",
    Pie = "pie",
    Git = "gitGraph"
}
export interface MermaidTheme {
    primaryColor?: string;
    primaryTextColor?: string;
    primaryBorderColor?: string;
    secondaryColor?: string;
    tertiaryColor?: string;
    background?: string;
    mainBkg?: string;
    secondBkg?: string;
    tertiaryBkg?: string;
    primaryBorderColor?: string;
    secondaryBorderColor?: string;
    tertiaryBorderColor?: string;
    lineColor?: string;
    textColor?: string;
    fontSize?: string;
    fontFamily?: string;
}
export interface MermaidConfig {
    theme?: 'default' | 'dark' | 'forest' | 'neutral' | 'base';
    themeVariables?: MermaidTheme;
    flowchart?: {
        curve?: 'basis' | 'linear' | 'cardinal';
        padding?: number;
        nodeSpacing?: number;
        rankSpacing?: number;
        diagramPadding?: number;
    };
    sequence?: {
        activationWidth?: number;
        diagramMarginX?: number;
        diagramMarginY?: number;
        boxTextMargin?: number;
        noteMargin?: number;
        messageMargin?: number;
    };
    state?: {
        dividerMargin?: number;
        sizeUnit?: number;
        padding?: number;
        textHeight?: number;
        titleShift?: number;
        noteMargin?: number;
        forkWidth?: number;
        forkHeight?: number;
        miniPadding?: number;
        fontSizeFactor?: number;
        fontSize?: number;
        labelHeight?: number;
        edgeLengthFactor?: string;
        compositTitleSize?: number;
        radius?: number;
    };
}
export interface ComponentAnalysis {
    components: ComponentInfo[];
    relationships: ComponentRelationship[];
    dependencies: DependencyInfo[];
    metrics: ComponentMetrics;
}
export interface ComponentInfo {
    name: string;
    path: string;
    type: 'functional' | 'class' | 'hook' | 'hoc';
    props?: PropInfo[];
    state?: StateInfo[];
    methods?: string[];
    hooks?: string[];
    imports: string[];
    exports: string[];
}
export interface PropInfo {
    name: string;
    type: string;
    required: boolean;
    defaultValue?: any;
    description?: string;
}
export interface StateInfo {
    name: string;
    type: string;
    initialValue?: any;
    updaters?: string[];
}
export interface ComponentRelationship {
    parent: string;
    child: string;
    type: 'renders' | 'imports' | 'extends' | 'wraps';
    props?: string[];
}
export interface DependencyInfo {
    source: string;
    target: string;
    type: 'import' | 'export' | 'dynamic';
    isCircular?: boolean;
}
export interface ComponentMetrics {
    totalComponents: number;
    averageComplexity: number;
    maxDepth: number;
    circularDependencies: number;
    unusedComponents: string[];
}
export interface StateFlow {
    stores: StateStore[];
    actions: StateAction[];
    flows: ActionFlow[];
    effects: SideEffect[];
}
export interface StateStore {
    id: string;
    name: string;
    type: 'redux' | 'mobx' | 'zustand' | 'context' | 'vuex' | 'pinia';
    schema: any;
    initialState: any;
    reducers?: string[];
    actions?: string[];
    getters?: string[];
}
export interface StateAction {
    id: string;
    name: string;
    type: string;
    payload?: any;
    storeId: string;
    async?: boolean;
}
export interface ActionFlow {
    actionId: string;
    effects: string[];
    stateChanges: StateChange[];
    triggers?: string[];
}
export interface StateChange {
    storeId: string;
    path: string;
    oldValue?: any;
    newValue?: any;
    operation: 'set' | 'update' | 'delete' | 'append';
}
export interface SideEffect {
    id: string;
    type: 'api' | 'navigation' | 'storage' | 'notification' | 'analytics';
    trigger: string;
    description?: string;
    async?: boolean;
}
export interface ExportOptions {
    format: 'markdown' | 'html' | 'pdf' | 'png' | 'svg' | 'json';
    includeMetadata?: boolean;
    includeInteractivity?: boolean;
    theme?: 'light' | 'dark' | 'auto';
    scale?: number;
    quality?: number;
}
export interface ExportResult {
    format: string;
    content: string | Buffer;
    filename: string;
    mimeType: string;
    size: number;
}
export interface StoryboardEvents {
    'scene-added': (scene: Scene) => void;
    'scene-updated': (scene: Scene) => void;
    'scene-deleted': (sceneId: string) => void;
    'flow-added': (flow: Flow) => void;
    'flow-updated': (flow: Flow) => void;
    'flow-deleted': (flowId: string) => void;
    'diagram-rendered': (diagram: MermaidDiagram) => void;
    'export-completed': (result: ExportResult) => void;
    'analysis-completed': (analysis: ComponentAnalysis) => void;
}
