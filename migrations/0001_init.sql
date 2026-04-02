CREATE TABLE IF NOT EXISTS audit_log (
  request_id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  raw_input TEXT NOT NULL,
  parsed_intent TEXT,
  risk_tier INTEGER NOT NULL,
  allowed INTEGER NOT NULL,
  actions_json TEXT,
  results_json TEXT,
  final_message TEXT
);