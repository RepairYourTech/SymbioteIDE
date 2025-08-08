-- Initial database schema for SymbioteIDE

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    due_date INTEGER,
    start_date INTEGER,
    owner TEXT,
    team_members TEXT,
    tags TEXT,
    repository_url TEXT,
    documentation_url TEXT,
    metadata TEXT
);

CREATE TABLE IF NOT EXISTS epics (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    due_date INTEGER,
    owner TEXT,
    tags TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,
    priority INTEGER NOT NULL,
    task_type TEXT NOT NULL,
    parent_id TEXT,
    project_id TEXT,
    epic_id TEXT,
    assigned_to TEXT,
    created_by TEXT NOT NULL,
    reviewers TEXT,
    active_agents TEXT,
    agent_history TEXT,
    collaboration_mode TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    due_date INTEGER,
    start_date INTEGER,
    estimated_hours REAL,
    actual_hours REAL,
    completion_percentage INTEGER DEFAULT 0,
    completion_notes TEXT,
    blockers TEXT,
    tags TEXT,
    labels TEXT,
    components TEXT,
    files_affected TEXT,
    attachments TEXT,
    external_links TEXT,
    metadata TEXT,
    semantic_embedding TEXT,
    knowledge_graph_id TEXT,
    FOREIGN KEY (parent_id) REFERENCES tasks(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (epic_id) REFERENCES epics(id)
);

CREATE TABLE IF NOT EXISTS task_dependencies (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    depends_on_task_id TEXT NOT NULL,
    dependency_type TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    notes TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id),
    FOREIGN KEY (depends_on_task_id) REFERENCES tasks(id)
);

CREATE TABLE IF NOT EXISTS task_comments (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    author TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    comment_type TEXT NOT NULL,
    mentions TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE TABLE IF NOT EXISTS time_entries (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    duration_minutes INTEGER,
    description TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);
