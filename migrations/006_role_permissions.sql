CREATE TABLE IF NOT EXISTS role_permissions (
  role TEXT NOT NULL,
  permission TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP::text),
  PRIMARY KEY(role, permission)
);

INSERT INTO role_permissions (role, permission, created_at)
VALUES
  ('admin', 'publish', CURRENT_TIMESTAMP::text),
  ('admin', 'moderate', CURRENT_TIMESTAMP::text),
  ('admin', 'settings', CURRENT_TIMESTAMP::text),
  ('admin', 'users', CURRENT_TIMESTAMP::text),
  ('admin', 'mcp', CURRENT_TIMESTAMP::text),
  ('admin', 'media', CURRENT_TIMESTAMP::text),
  ('admin', 'analytics', CURRENT_TIMESTAMP::text),
  ('editor', 'publish', CURRENT_TIMESTAMP::text),
  ('editor', 'moderate', CURRENT_TIMESTAMP::text),
  ('writer', 'publish', CURRENT_TIMESTAMP::text)
ON CONFLICT(role, permission) DO NOTHING;
