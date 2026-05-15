CREATE TABLE IF NOT EXISTS task_requests (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  source TEXT NOT NULL,
  user_request TEXT NOT NULL,
  intent TEXT,
  status TEXT NOT NULL,
  op_task_id TEXT,
  run_id TEXT,
  primary_artifact_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES employment_profiles(id) ON DELETE CASCADE,
  FOREIGN KEY(op_task_id) REFERENCES op_tasks(id) ON DELETE SET NULL,
  FOREIGN KEY(run_id) REFERENCES op_task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY(primary_artifact_id) REFERENCES task_artifacts(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_task_requests_profile_id ON task_requests(profile_id);
CREATE INDEX IF NOT EXISTS idx_task_requests_status ON task_requests(status);
CREATE INDEX IF NOT EXISTS idx_task_requests_created_at ON task_requests(created_at);
