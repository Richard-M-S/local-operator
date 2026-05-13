CREATE TABLE IF NOT EXISTS op_tasks (
  id TEXT PRIMARY KEY,
  task_type TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS op_task_runs (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT,
  completed_at TEXT,
  work_items_json TEXT,
  summary TEXT,
  FOREIGN KEY(task_id) REFERENCES op_tasks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS task_artifacts (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  work_item_id TEXT,
  name TEXT NOT NULL,
  artifact_type TEXT NOT NULL,
  location TEXT,
  created_at TEXT NOT NULL,
  metadata_json TEXT,
  FOREIGN KEY(run_id) REFERENCES op_task_runs(id) ON DELETE CASCADE
);
