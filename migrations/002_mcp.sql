PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS mcp_clients (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  token_hash TEXT NOT NULL UNIQUE,
  scopes TEXT NOT NULL,
  transport TEXT NOT NULL DEFAULT 'http',
  is_enabled INTEGER NOT NULL DEFAULT 1,
  created_by INTEGER NULL,
  last_used_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_mcp_clients_name ON mcp_clients(name);
CREATE UNIQUE INDEX IF NOT EXISTS uq_mcp_clients_token_hash ON mcp_clients(token_hash);
CREATE INDEX IF NOT EXISTS idx_mcp_clients_is_enabled ON mcp_clients(is_enabled);

CREATE TABLE IF NOT EXISTS mcp_audit_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  client_id INTEGER NULL,
  transport TEXT NOT NULL,
  action_type TEXT NOT NULL,
  target TEXT NOT NULL,
  scope TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL,
  request_id TEXT NOT NULL DEFAULT '',
  actor_ip TEXT NOT NULL DEFAULT '',
  error_code TEXT NOT NULL DEFAULT '',
  payload_digest TEXT NOT NULL DEFAULT '',
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(client_id) REFERENCES mcp_clients(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_mcp_audit_logs_client_id ON mcp_audit_logs(client_id);
CREATE INDEX IF NOT EXISTS idx_mcp_audit_logs_created_at ON mcp_audit_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_mcp_audit_logs_action_type ON mcp_audit_logs(action_type);
