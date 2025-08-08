# Global vs Project Scoping System
## AI Master Tool - Multi-Level Resource Management

**Version:** 1.0  
**Date:** January 2025  
**Status:** Draft

---

## Overview

AI Master Tool implements a sophisticated scoping system that allows resources, knowledge, and settings to exist at multiple levels: Global (system-wide), Workspace, Project, and User. This enables powerful knowledge sharing while maintaining project isolation when needed.

## Scope Hierarchy

```
┌─────────────────────────────────────┐
│          GLOBAL SCOPE               │
│  (Shared across all projects)       │
│  • Global notebooks & journals      │
│  • Shared code snippets            │
│  • Common patterns library         │
│  • Global AI memory               │
└────────────┬────────────────────────┘
             │
    ┌────────▼────────────────────┐
    │    WORKSPACE SCOPE          │
    │  (Organization level)        │
    │  • Team standards            │
    │  • Shared components          │
    │  • Workspace templates        │
    └────────┬─────────────────────┘
             │
    ┌────────▼────────────────────┐
    │    PROJECT SCOPE            │
    │  (Individual project)        │
    │  • Project notebooks          │
    │  • Project-specific AI        │
    │  • Local patterns             │
    └────────┬─────────────────────┘
             │
    ┌────────▼────────────────────┐
    │    USER SCOPE               │
    │  (Personal preferences)      │
    │  • Personal shortcuts        │
    │  • Custom settings           │
    │  • Private notes             │
    └─────────────────────────────┘
```

## Core Architecture

### 1. Scope Manager

```rust
pub struct ScopeManager {
    global_scope: GlobalScope,
    workspace_scopes: HashMap<WorkspaceId, WorkspaceScope>,
    project_scopes: HashMap<ProjectId, ProjectScope>,
    user_scopes: HashMap<UserId, UserScope>,
    resolution_chain: ResolutionChain,
    
    pub async fn resolve_resource<T: Resource>(
        &self,
        resource_id: &ResourceId,
        context: &Context
    ) -> Result<T> {
        // Try project scope first (most specific)
        if let Some(project_id) = context.project_id {
            if let Some(resource) = self.project_scopes
                .get(&project_id)?
                .get_resource::<T>(resource_id).await? {
                return Ok(resource);
            }
        }
        
        // Try workspace scope
        if let Some(workspace_id) = context.workspace_id {
            if let Some(resource) = self.workspace_scopes
                .get(&workspace_id)?
                .get_resource::<T>(resource_id).await? {
                return Ok(resource);
            }
        }
        
        // Try user scope
        if let Some(user_id) = context.user_id {
            if let Some(resource) = self.user_scopes
                .get(&user_id)?
                .get_resource::<T>(resource_id).await? {
                return Ok(resource);
            }
        }
        
        // Finally, try global scope
        self.global_scope.get_resource::<T>(resource_id).await
    }
}
```

### 2. Global Scope Resources

```typescript
class GlobalScope {
  // Global AI Memory
  aiMemory: GlobalAIMemory = {
    patterns: PatternLibrary,
    learnings: LearningDatabase,
    codeSnippets: SnippetLibrary,
    bestPractices: BestPracticeDB,
  };
  
  // Global Notebooks
  notebooks: Map<NotebookId, GlobalNotebook> = new Map();
  
  // Global Journals
  journals: Map<JournalId, GlobalJournal> = new Map();
  
  // Global Component Library
  componentLibrary: ComponentLibrary = {
    uiComponents: Map<ComponentId, Component>,
    codeTemplates: Map<TemplateId, Template>,
    designPatterns: Map<PatternId, Pattern>,
  };
  
  // Global Knowledge Graph
  knowledgeGraph: KnowledgeGraph = {
    concepts: Map<ConceptId, Concept>,
    relationships: Edge[],
    index: SearchIndex,
  };
  
  // Global Tools Configuration
  toolsConfig: GlobalToolsConfig = {
    aiProviders: AIProviderConfig[],
    integrations: IntegrationConfig[],
    shortcuts: GlobalShortcut[],
  };
  
  async addGlobalResource(resource: Resource): Promise<void> {
    // Validate resource doesn't conflict
    await this.validateNoConflict(resource);
    
    // Add to appropriate collection
    await this.addToCollection(resource);
    
    // Update search index
    await this.updateSearchIndex(resource);
    
    // Notify all projects of new global resource
    await this.notifyProjects(resource);
  }
}
```

### 3. Project Scope Resources

```rust
pub struct ProjectScope {
    project_id: ProjectId,
    resources: ProjectResources,
    overrides: OverrideMap,
    inheritance: InheritanceConfig,
    
    pub struct ProjectResources {
        // Project-specific AI context
        ai_context: ProjectAIContext,
        
        // Project notebooks
        notebooks: HashMap<NotebookId, Notebook>,
        
        // Project journals
        journals: HashMap<JournalId, Journal>,
        
        // Project-specific patterns
        patterns: PatternRegistry,
        
        // Project codebase index
        codebase_index: CodebaseIndex,
        
        // Project-specific tools
        tools: ToolRegistry,
        
        // Project settings
        settings: ProjectSettings,
    }
    
    pub async fn override_global_resource(&mut self, resource_id: ResourceId) -> Result<()> {
        // Mark resource as overridden
        self.overrides.insert(resource_id, OverrideType::Full);
        
        // Copy global resource to project scope
        let global_resource = self.scope_manager.get_global_resource(&resource_id).await?;
        let project_resource = self.create_project_copy(global_resource).await?;
        
        // Allow modifications
        self.resources.add(project_resource);
        
        Ok(())
    }
}
```

### 4. Inheritance & Override System

```typescript
class InheritanceManager {
  // Resource inheritance rules
  rules: InheritanceRules = {
    notebooks: {
      inherit: true,
      allowOverride: true,
      mergeStrategy: 'extend', // Project notebooks extend global
    },
    patterns: {
      inherit: true,
      allowOverride: true,
      mergeStrategy: 'replace', // Project can replace global patterns
    },
    aiMemory: {
      inherit: true,
      allowOverride: false, // Can't override global AI learnings
      mergeStrategy: 'combine', // Combine global and project
    },
    components: {
      inherit: true,
      allowOverride: true,
      mergeStrategy: 'shadow', // Project shadows global
    },
  };
  
  async resolveInheritance(
    resourceType: ResourceType,
    resourceId: ResourceId,
    scopes: Scope[]
  ): Promise<Resource> {
    const rule = this.rules[resourceType];
    
    if (!rule.inherit) {
      // No inheritance, only check current scope
      return await this.getFromCurrentScope(resourceId, scopes[0]);
    }
    
    // Collect resources from all scopes
    const resources = await this.collectFromScopes(resourceId, scopes);
    
    // Apply merge strategy
    switch (rule.mergeStrategy) {
      case 'extend':
        return this.extendResources(resources);
        
      case 'replace':
        return resources[0]; // Most specific wins
        
      case 'combine':
        return this.combineResources(resources);
        
      case 'shadow':
        return this.shadowResources(resources);
    }
  }
}
```

### 5. Scope-Aware Indexing

```rust
pub struct ScopedIndexingSystem {
    global_index: GlobalIndex,
    project_indices: HashMap<ProjectId, ProjectIndex>,
    workspace_indices: HashMap<WorkspaceId, WorkspaceIndex>,
    
    pub async fn search(&self, query: &SearchQuery, context: &Context) -> SearchResults {
        let mut results = SearchResults::new();
        
        // Search in appropriate scopes based on context
        match query.scope {
            SearchScope::CurrentProject => {
                if let Some(project_id) = context.project_id {
                    results.add(
                        self.project_indices
                            .get(&project_id)?
                            .search(query).await?
                    );
                }
            },
            SearchScope::Workspace => {
                if let Some(workspace_id) = context.workspace_id {
                    results.add(
                        self.workspace_indices
                            .get(&workspace_id)?
                            .search(query).await?
                    );
                }
            },
            SearchScope::Global => {
                results.add(self.global_index.search(query).await?);
            },
            SearchScope::All => {
                // Search all scopes and merge
                if let Some(project_id) = context.project_id {
                    results.add_with_scope(
                        self.project_indices.get(&project_id)?.search(query).await?,
                        Scope::Project
                    );
                }
                if let Some(workspace_id) = context.workspace_id {
                    results.add_with_scope(
                        self.workspace_indices.get(&workspace_id)?.search(query).await?,
                        Scope::Workspace
                    );
                }
                results.add_with_scope(
                    self.global_index.search(query).await?,
                    Scope::Global
                );
            },
        }
        
        // Rank results considering scope relevance
        self.rank_by_scope_relevance(results, context)
    }
}
```

### 6. AI Memory Scoping

```typescript
class ScopedAIMemory {
  // Different memory levels
  memories: {
    global: GlobalAIMemory,
    workspace: Map<WorkspaceId, WorkspaceMemory>,
    project: Map<ProjectId, ProjectMemory>,
    session: Map<SessionId, SessionMemory>,
  };
  
  // Save learning at appropriate scope
  async savelearning(learning: Learning, scope: MemoryScope): Promise<void> {
    switch (scope) {
      case MemoryScope.Global:
        // Learnings that apply universally
        await this.memories.global.save(learning);
        break;
        
      case MemoryScope.Workspace:
        // Team-specific learnings
        await this.memories.workspace.get(this.workspaceId).save(learning);
        break;
        
      case MemoryScope.Project:
        // Project-specific patterns
        await this.memories.project.get(this.projectId).save(learning);
        break;
        
      case MemoryScope.Session:
        // Temporary, session-only memory
        await this.memories.session.get(this.sessionId).save(learning);
        break;
    }
  }
  
  // Retrieve memories with scope awareness
  async recall(query: string, context: Context): Promise<Memory[]> {
    const memories: Memory[] = [];
    
    // Always include global memories
    memories.push(...await this.memories.global.recall(query));
    
    // Add workspace memories if in workspace context
    if (context.workspaceId) {
      memories.push(...await this.memories.workspace
        .get(context.workspaceId)
        .recall(query));
    }
    
    // Add project memories if in project context
    if (context.projectId) {
      memories.push(...await this.memories.project
        .get(context.projectId)
        .recall(query));
    }
    
    // Rank by relevance and scope
    return this.rankMemories(memories, context);
  }
}
```

### 7. Component & Pattern Sharing

```rust
pub struct ComponentSharingSystem {
    visibility_rules: VisibilityRules,
    sharing_permissions: SharingPermissions,
    
    pub async fn share_component(
        &self,
        component: &Component,
        from_scope: Scope,
        to_scope: Scope,
        options: SharingOptions
    ) -> Result<()> {
        // Check permissions
        self.check_sharing_permission(&component, &from_scope, &to_scope)?;
        
        // Process component for sharing
        let shareable_component = match options.sharing_type {
            SharingType::Reference => {
                // Share as reference (changes reflect)
                self.create_reference(component)
            },
            SharingType::Copy => {
                // Share as copy (independent)
                self.create_independent_copy(component)
            },
            SharingType::Fork => {
                // Share as fork (trackable divergence)
                self.create_fork(component)
            },
        };
        
        // Add to target scope
        match to_scope {
            Scope::Global => {
                self.add_to_global_library(shareable_component).await?
            },
            Scope::Workspace(id) => {
                self.add_to_workspace_library(id, shareable_component).await?
            },
            Scope::Project(id) => {
                self.add_to_project_library(id, shareable_component).await?
            },
        }
        
        Ok(())
    }
    
    pub async fn promote_pattern(&self, pattern: &Pattern, to_scope: Scope) -> Result<()> {
        // Validate pattern quality
        let quality_score = self.analyze_pattern_quality(pattern).await?;
        if quality_score < self.quality_threshold {
            return Err(Error::PatternQualityTooLow(quality_score));
        }
        
        // Check for conflicts
        let conflicts = self.check_pattern_conflicts(pattern, &to_scope).await?;
        if !conflicts.is_empty() {
            return Err(Error::PatternConflicts(conflicts));
        }
        
        // Promote with metadata
        let promoted_pattern = PromotedPattern {
            pattern: pattern.clone(),
            origin_scope: pattern.scope.clone(),
            promoted_by: self.current_user(),
            promoted_at: Utc::now(),
            usage_stats: self.get_pattern_usage_stats(pattern).await?,
        };
        
        self.add_pattern_to_scope(promoted_pattern, to_scope).await
    }
}
```

### 8. Notebook & Journal Scoping

```typescript
class ScopedNotebookSystem {
  // Different notebook types by scope
  notebooks: {
    global: GlobalNotebook[],      // Shared knowledge
    workspace: WorkspaceNotebook[], // Team documentation
    project: ProjectNotebook[],     // Project-specific
    personal: PersonalNotebook[],   // User's private notes
  };
  
  // Create notebook with appropriate scope
  async createNotebook(config: NotebookConfig): Promise<Notebook> {
    const notebook = new Notebook({
      id: generateId(),
      title: config.title,
      scope: config.scope,
      visibility: config.visibility || this.getDefaultVisibility(config.scope),
      permissions: this.getDefaultPermissions(config.scope),
    });
    
    // Add to appropriate collection
    switch (config.scope) {
      case 'global':
        this.notebooks.global.push(notebook);
        // Make discoverable
        await this.indexForDiscovery(notebook);
        break;
        
      case 'workspace':
        this.notebooks.workspace.push(notebook);
        // Notify team members
        await this.notifyWorkspace(notebook);
        break;
        
      case 'project':
        this.notebooks.project.push(notebook);
        // Link to project
        await this.linkToProject(notebook, config.projectId);
        break;
        
      case 'personal':
        this.notebooks.personal.push(notebook);
        // Keep private
        notebook.setPrivate(true);
        break;
    }
    
    return notebook;
  }
  
  // Search across scopes
  async searchNotebooks(query: string, options: SearchOptions): Promise<NotebookSearchResults> {
    const results = new NotebookSearchResults();
    
    // Search based on permissions and scope
    if (options.includeGlobal && this.hasGlobalAccess()) {
      results.global = await this.searchGlobalNotebooks(query);
    }
    
    if (options.includeWorkspace && this.hasWorkspaceAccess()) {
      results.workspace = await this.searchWorkspaceNotebooks(query);
    }
    
    if (options.includeProject && this.hasProjectAccess()) {
      results.project = await this.searchProjectNotebooks(query);
    }
    
    if (options.includePersonal) {
      results.personal = await this.searchPersonalNotebooks(query);
    }
    
    // Rank by relevance and access frequency
    return this.rankNotebookResults(results);
  }
}
```

### 9. Settings & Configuration Scoping

```rust
pub struct ScopedSettings {
    // Hierarchical settings
    settings_hierarchy: SettingsHierarchy,
    
    pub async fn get_setting<T>(&self, key: &str, context: &Context) -> Option<T> {
        // Check user settings first (highest priority)
        if let Some(value) = self.get_user_setting(key, context.user_id).await {
            return Some(value);
        }
        
        // Then project settings
        if let Some(project_id) = context.project_id {
            if let Some(value) = self.get_project_setting(key, project_id).await {
                return Some(value);
            }
        }
        
        // Then workspace settings
        if let Some(workspace_id) = context.workspace_id {
            if let Some(value) = self.get_workspace_setting(key, workspace_id).await {
                return Some(value);
            }
        }
        
        // Finally global settings
        self.get_global_setting(key).await
    }
    
    pub async fn set_setting<T>(
        &mut self,
        key: &str,
        value: T,
        scope: SettingScope
    ) -> Result<()> {
        match scope {
            SettingScope::User => {
                self.set_user_setting(key, value).await
            },
            SettingScope::Project(id) => {
                // Check if setting can be overridden at project level
                if !self.can_override_at_project_level(key) {
                    return Err(Error::SettingNotOverridable(key.to_string()));
                }
                self.set_project_setting(key, value, id).await
            },
            SettingScope::Workspace(id) => {
                // Requires workspace admin permissions
                self.check_workspace_admin_permission(id)?;
                self.set_workspace_setting(key, value, id).await
            },
            SettingScope::Global => {
                // Requires system admin permissions
                self.check_system_admin_permission()?;
                self.set_global_setting(key, value).await
            },
        }
    }
}
```

### 10. Scope Visualization & Management UI

```typescript
interface ScopeManagementUI {
  render(): JSX.Element {
    return (
      <ScopeManager>
        <ScopeHierarchy>
          <GlobalScope>
            <ScopeHeader>
              <Icon name="globe" />
              <Title>Global Resources</Title>
              <Count>{this.globalResourceCount}</Count>
            </ScopeHeader>
            <ResourceList>
              {this.globalResources.map(resource => (
                <ResourceItem key={resource.id}>
                  <ResourceIcon type={resource.type} />
                  <ResourceName>{resource.name}</ResourceName>
                  <UsageStats>{resource.usageAcrossProjects}</UsageStats>
                  <Actions>
                    <Button onClick={() => this.editResource(resource)}>Edit</Button>
                    <Button onClick={() => this.viewUsage(resource)}>Usage</Button>
                  </Actions>
                </ResourceItem>
              ))}
            </ResourceList>
          </GlobalScope>
          
          <WorkspaceScope>
            <ScopeHeader>
              <Icon name="team" />
              <Title>Workspace: {this.currentWorkspace.name}</Title>
            </ScopeHeader>
            <TabPanel>
              <Tab name="Components">
                <ComponentGrid>
                  {this.workspaceComponents.map(component => (
                    <ComponentCard 
                      key={component.id}
                      component={component}
                      onPromote={() => this.promoteToGlobal(component)}
                      onShare={() => this.shareComponent(component)}
                    />
                  ))}
                </ComponentGrid>
              </Tab>
              <Tab name="Patterns">
                <PatternList patterns={this.workspacePatterns} />
              </Tab>
              <Tab name="Settings">
                <SettingsPanel scope="workspace" />
              </Tab>
            </TabPanel>
          </WorkspaceScope>
          
          <ProjectScope>
            <ScopeHeader>
              <Icon name="project" />
              <Title>Project: {this.currentProject.name}</Title>
            </ScopeHeader>
            <OverrideIndicator>
              {this.projectOverrides.map(override => (
                <Override key={override.id}>
                  <OverrideIcon />
                  <OverrideName>{override.resourceName}</OverrideName>
                  <OverrideType>{override.type}</OverrideType>
                  <Button onClick={() => this.revertOverride(override)}>
                    Revert to {override.originalScope}
                  </Button>
                </Override>
              ))}
            </OverrideIndicator>
          </ProjectScope>
        </ScopeHierarchy>
        
        <ResourceInheritanceView>
          <InheritanceTree>
            {this.buildInheritanceTree().map(node => (
              <TreeNode key={node.id}>
                <NodeInfo>
                  <Scope>{node.scope}</Scope>
                  <Resource>{node.resource}</Resource>
                  <InheritanceType>{node.inheritanceType}</InheritanceType>
                </NodeInfo>
                {node.children && <Children>{node.children}</Children>}
              </TreeNode>
            ))}
          </InheritanceTree>
        </ResourceInheritanceView>
      </ScopeManager>
    );
  }
}
```

## Usage Examples

### Example 1: Creating a Global Pattern

```typescript
// User discovers a great pattern in their project
const pattern = {
  name: "Optimistic UI Update",
  code: optimisticUpdateImplementation,
  description: "Update UI immediately while API call happens",
  tags: ["performance", "ux", "async"],
};

// AI validates it's a good pattern
const validation = await ai.validatePattern(pattern);
if (validation.score > 0.8) {
  // Promote to global scope
  await scopeManager.promoteToGlobal(pattern, {
    scope: 'pattern',
    visibility: 'public',
    category: 'ui-patterns',
  });
  
  // Now available in all projects
  notify("Pattern promoted to global library!");
}
```

### Example 2: Project-Specific AI Memory

```typescript
// AI learns something specific to this project
const projectLearning = {
  type: 'code-style',
  learning: "This project prefers functional components over class components",
  confidence: 0.95,
  examples: [example1, example2],
};

// Save to project scope only
await aiMemory.save(projectLearning, MemoryScope.Project);

// In another project, this learning won't apply
// But global learnings like "React hooks best practices" will still be available
```

### Example 3: Workspace Component Library

```rust
// Team creates a design system component
let button_component = Component {
    name: "PrimaryButton",
    code: button_implementation,
    styles: button_styles,
    tests: button_tests,
    docs: button_docs,
};

// Share within workspace
scope_manager.share_component(
    button_component,
    Scope::Project(current_project),
    Scope::Workspace(team_workspace),
    SharingOptions {
        sharing_type: SharingType::Reference,
        maintain_sync: true,
    }
).await?;

// Now all team projects can use this component
// Updates to the component reflect everywhere
```

## Benefits

1. **Knowledge Preservation**: Global patterns and learnings benefit all future projects
2. **Team Collaboration**: Workspace-level sharing for team standards
3. **Project Isolation**: Keep project-specific details contained
4. **Flexible Inheritance**: Override or extend global resources as needed
5. **Progressive Enhancement**: Start project-specific, promote to global when proven
6. **Efficient Learning**: AI learns once, applies everywhere appropriate
7. **Customization**: Users can personalize without affecting others
8. **Resource Discovery**: Find and reuse components across scopes

This scoping system makes AI Master Tool a true learning platform that gets better with every project while respecting boundaries between different contexts.