CREATE TABLE IF NOT EXISTS chat_sessions (
  id TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  external_source TEXT,
  external_conversation_id TEXT,
  last_task_request_id TEXT,
  last_run_id TEXT,
  last_artifact_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES employment_profiles(id) ON DELETE CASCADE,
  FOREIGN KEY(last_task_request_id) REFERENCES task_requests(id) ON DELETE SET NULL,
  FOREIGN KEY(last_run_id) REFERENCES op_task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY(last_artifact_id) REFERENCES task_artifacts(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_chat_sessions_profile_id ON chat_sessions(profile_id);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_external_id ON chat_sessions(external_source, external_conversation_id);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_created_at ON chat_sessions(created_at);
