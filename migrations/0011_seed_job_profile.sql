UPDATE employment_profiles
SET
  display_name = 'Salesforce Platform Architect / Automation + Remote OE-Compatible Admin',
  criteria = COALESCE(
    NULLIF(criteria, ''),
    'Primary track: prioritize Salesforce Platform Architect, senior Salesforce admin, automation, platform operations, integration, governance, and systems improvement roles with strong remote flexibility. OE-compatible track: only consider clearly remote roles with bounded scope, low meeting load, limited on-call, low travel, no strict availability conflicts, and no confidentiality or legal conflict concerns. Score both tracks separately, flag risks explicitly, and require manual review for conflicts, availability constraints, legal terms, confidentiality issues, unclear scope, or anything that could affect current obligations.'
  ),
  updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE id = '00000000-0000-0000-0000-000000000001'
  AND display_name = 'Default';
