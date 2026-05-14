CREATE TABLE IF NOT EXISTS employment_opportunities (
  id TEXT PRIMARY KEY,
  source_url TEXT NOT NULL,
  source_name TEXT,
  title TEXT,
  company TEXT,
  location TEXT,
  remote_type TEXT,
  salary_min INTEGER,
  salary_max INTEGER,
  description_text TEXT,
  extracted_json TEXT,
  fit_score INTEGER,
  status TEXT NOT NULL,
  skip_reason TEXT,
  source_artifact_id TEXT,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_employment_opportunities_source_url
ON employment_opportunities(source_url);

CREATE INDEX IF NOT EXISTS idx_employment_opportunities_status
ON employment_opportunities(status);

CREATE INDEX IF NOT EXISTS idx_employment_opportunities_fit_score
ON employment_opportunities(fit_score);