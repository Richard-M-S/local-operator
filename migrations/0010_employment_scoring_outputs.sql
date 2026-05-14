ALTER TABLE employment_opportunities
ADD COLUMN primary_fit_score INTEGER;

ALTER TABLE employment_opportunities
ADD COLUMN oe_fit_score INTEGER;

ALTER TABLE employment_opportunities
ADD COLUMN recommended_track TEXT;

ALTER TABLE employment_opportunities
ADD COLUMN score_reason TEXT;

ALTER TABLE employment_opportunities
ADD COLUMN risk_flags_json TEXT;

ALTER TABLE employment_opportunities
ADD COLUMN skip_recommendation TEXT;

CREATE INDEX IF NOT EXISTS idx_employment_opportunities_primary_fit_score
ON employment_opportunities(primary_fit_score);

CREATE INDEX IF NOT EXISTS idx_employment_opportunities_oe_fit_score
ON employment_opportunities(oe_fit_score);

CREATE INDEX IF NOT EXISTS idx_employment_opportunities_recommended_track
ON employment_opportunities(recommended_track);
