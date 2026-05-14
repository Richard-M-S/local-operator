CREATE TABLE IF NOT EXISTS employment_profiles (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  email TEXT,
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT
);

INSERT OR IGNORE INTO employment_profiles (
  id,
  display_name,
  email,
  notes,
  created_at,
  updated_at
) VALUES (
  '00000000-0000-0000-0000-000000000001',
  'Default',
  NULL,
  'Migrated default profile for existing single-user data.',
  strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
  NULL
);

ALTER TABLE saved_contexts
ADD COLUMN profile_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001';

ALTER TABLE op_tasks
ADD COLUMN profile_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001';

ALTER TABLE op_task_runs
ADD COLUMN profile_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001';

ALTER TABLE task_artifacts
ADD COLUMN profile_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001';

ALTER TABLE employment_opportunities
ADD COLUMN profile_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001';

CREATE INDEX IF NOT EXISTS idx_saved_contexts_profile_id
ON saved_contexts(profile_id);

CREATE INDEX IF NOT EXISTS idx_op_tasks_profile_id
ON op_tasks(profile_id);

CREATE INDEX IF NOT EXISTS idx_op_task_runs_profile_id
ON op_task_runs(profile_id);

CREATE INDEX IF NOT EXISTS idx_task_artifacts_profile_id
ON task_artifacts(profile_id);

CREATE INDEX IF NOT EXISTS idx_employment_opportunities_profile_id
ON employment_opportunities(profile_id);

CREATE INDEX IF NOT EXISTS idx_employment_opportunities_profile_source_url
ON employment_opportunities(profile_id, source_url);
