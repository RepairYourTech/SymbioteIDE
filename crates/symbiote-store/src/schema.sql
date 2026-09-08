CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    revision INTEGER NOT NULL CHECK(revision >= 0),
    lead_id TEXT NOT NULL REFERENCES roles(id) DEFERRABLE INITIALLY DEFERRED,
    body TEXT NOT NULL CHECK(json_valid(body))
) STRICT;
CREATE TABLE roots (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    body TEXT NOT NULL CHECK(json_valid(body)),
    UNIQUE(id, project_id)
) STRICT;
CREATE TABLE roles (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    body TEXT NOT NULL CHECK(json_valid(body)),
    UNIQUE(id, project_id)
) STRICT;
CREATE TABLE streams (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    root_id TEXT NOT NULL,
    worktree_id TEXT NOT NULL UNIQUE,
    branch TEXT NOT NULL,
    body TEXT NOT NULL CHECK(json_valid(body)),
    UNIQUE(id, project_id, root_id),
    UNIQUE(root_id, branch),
    FOREIGN KEY(root_id, project_id) REFERENCES roots(id, project_id)
) STRICT;
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    root_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    stream_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision >= 0),
    body TEXT NOT NULL CHECK(json_valid(body)),
    FOREIGN KEY(root_id, project_id) REFERENCES roots(id, project_id),
    FOREIGN KEY(role_id, project_id) REFERENCES roles(id, project_id),
    FOREIGN KEY(stream_id, project_id, root_id) REFERENCES streams(id, project_id, root_id)
) STRICT;
CREATE TABLE journal (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT CHECK(sequence > 0),
    project_id TEXT NOT NULL REFERENCES projects(id),
    command_id TEXT NOT NULL UNIQUE,
    revision INTEGER NOT NULL CHECK(revision >= 0),
    request TEXT NOT NULL CHECK(json_valid(request)),
    payload TEXT NOT NULL CHECK(json_valid(payload))
) STRICT;
CREATE INDEX journal_project_cursor ON journal(project_id, sequence);
CREATE TRIGGER journal_no_update BEFORE UPDATE ON journal BEGIN SELECT RAISE(ABORT, 'append-only journal'); END;
CREATE TRIGGER journal_no_delete BEFORE DELETE ON journal BEGIN SELECT RAISE(ABORT, 'append-only journal'); END;
PRAGMA application_id = 0x53594d42;
PRAGMA user_version = 1;
