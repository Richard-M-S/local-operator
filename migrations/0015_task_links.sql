CREATE TABLE IF NOT EXISTS task_links (
  id TEXT PRIMARY KEY,
  source_type TEXT NOT NULL,
  source_id TEXT NOT NULL,
  target_type TEXT NOT NULL,
  target_id TEXT NOT NULL,
  relationship TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_task_links_source ON task_links(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_task_links_target ON task_links(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_task_links_relationship ON task_links(relationship);
