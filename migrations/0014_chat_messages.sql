CREATE TABLE IF NOT EXISTS chat_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  task_request_id TEXT,
  run_id TEXT,
  artifact_id TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
  FOREIGN KEY(task_request_id) REFERENCES task_requests(id) ON DELETE SET NULL,
  FOREIGN KEY(run_id) REFERENCES op_task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY(artifact_id) REFERENCES task_artifacts(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_chat_messages_session_id ON chat_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_task_request_id ON chat_messages(task_request_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at);
