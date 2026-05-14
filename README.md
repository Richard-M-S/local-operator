# local-operator

Local Operator is a local-first automation service for server operations, tool execution, reusable context, and job-hunt workflows.

It runs as a Rust/Axum API with SQLite persistence and a React/Vite console. The project started as a local operations layer for system, Docker, and Home Assistant inspection. It now also includes an Op Task pipeline for producing artifacts, promoting useful artifacts into saved context, and turning readable job-posting artifacts into employment opportunities.

## Current Status

Working today:

- Axum API server on the configured host and port
- SQLite persistence and migrations
- health and status routes
- audit logging support
- system, Docker, and Home Assistant tool modules
- OpenAI-compatible chat/model routes backed by local LLM plumbing
- Op Tasks with manual runs, run history, work items, and artifacts
- readable URL ingestion through `reader.read_url`
- artifact listing, detail, content, and save-to-context routes
- saved context notes with list, create, get, and search routes
- employment opportunities created from readable artifacts
- opportunity parse and score routes
- Operator Console for reviewing artifacts and opportunities
- duplicate awareness when creating opportunities from artifacts or source URLs

Still intentionally basic:

- scheduling is not implemented yet
- employment parsing and scoring are early-stage
- context search is SQL LIKE, not embeddings
- artifact-to-context promotion works, but the business logic should move deeper into service code as it grows
- duplicate protection is application-level; database-level uniqueness rules can come later after existing data is reviewed

## Run

The default API config is in `config/default.toml`.

```bash
cargo run
```

Default server settings:

- host: `0.0.0.0`
- port: `8080`
- database: `sqlite:///opt/local-operator/data/operator.db`
- auth: disabled by default
- auth token env when enabled: `OPERATOR_API_TOKEN`
- Home Assistant token env: `HA_TOKEN`
- local LLM provider: Ollama at `http://localhost:11434`

Run the console:

```bash
cd operator-console
npm run dev
```

The console defaults to `http://localhost:8080` as the API base and stores API base, token, artifact type filter, and status filter in browser local storage.

## Console

The Operator Console is the easiest way to use the employment workflow.

Current console workflows:

- read a job URL into a readable artifact
- read a job URL and create an employment opportunity
- review artifacts with tabs for readable text, raw JSON, and source metadata
- review opportunities with summary, parsed fields, raw JSON, and source artifact tabs
- create, parse, and score opportunities from artifacts
- open source URLs and source artifacts
- filter opportunities by status presets
- filter artifacts by artifact type presets
- view a "Today's Work" dashboard
- warn before reusing an artifact or source URL that already has an opportunity

## Main API Routes

Health:

- `GET /health`

Status and operator:

- `GET /api/status`
- `POST /api/operator/command`
- `POST /api/operator/chat`
- `POST /api/tools/execute`
- `GET /api/audit/recent`

OpenAI compatibility:

- `GET /v1/models`
- `POST /v1/chat/completions`

Context:

- `GET /api/context`
- `POST /api/context`
- `GET /api/context/search?q=...`
- `GET /api/context/:id`

Op Tasks:

- `POST /api/op-tasks`
- `GET /api/op-tasks`
- `GET /api/op-tasks/:id`
- `POST /api/op-tasks/:id/run`
- `GET /api/op-tasks/:id/runs`
- `GET /api/op-task-runs/:id`

Artifacts:

- `GET /api/op-task-artifacts`
- `GET /api/op-task-artifacts/:id`
- `GET /api/op-task-artifacts/:id/content`
- `POST /api/op-task-artifacts/:id/save-context`

Employment:

- `GET /api/employment/opportunities`
- `POST /api/employment/opportunities`
- `GET /api/employment/opportunities/:id`
- `POST /api/employment/opportunities/:id/parse`
- `POST /api/employment/opportunities/:id/score`
- `POST /api/employment/opportunities/from-artifact/:artifact_id`

## Op Task Example

Create a reader task:

```bash
curl -i -X POST http://localhost:8080/api/op-tasks \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Read Job URL",
    "task_type": "reader.read_url",
    "description": "Read a job posting URL.",
    "input_json": {
      "url": "https://example.com"
    },
    "enabled": true
  }'
```

Run it:

```bash
curl -i -X POST http://localhost:8080/api/op-tasks/<TASK_ID>/run
```

List readable artifacts without full content:

```bash
curl -i "http://localhost:8080/api/op-task-artifacts?artifact_type=readable_web_page&limit=20"
```

Load artifact content separately:

```bash
curl -i http://localhost:8080/api/op-task-artifacts/<ARTIFACT_ID>/content
```

## Context Example

Create a reusable context note manually:

```bash
curl -i -X POST http://localhost:8080/api/context \
  -H "Content-Type: application/json" \
  -d '{
    "kind": "career_profile",
    "title": "Preferred Role Direction",
    "body": "Prioritize Salesforce Architect, Platform Architect, and internal tooling roles over pure Salesforce Admin roles.",
    "tags": ["employment", "career", "preferences"]
  }'
```

Search context:

```bash
curl -i "http://localhost:8080/api/context/search?q=Salesforce"
```

Promote an artifact into saved context:

```bash
curl -i -X POST http://localhost:8080/api/op-task-artifacts/<ARTIFACT_ID>/save-context \
  -H "Content-Type: application/json" \
  -d '{
    "kind": "document_note",
    "title": "Example Domain Reference",
    "tags": ["web", "reader"],
    "body_source": "content_text"
  }'
```

## Employment Example

Create an opportunity from a readable artifact:

```bash
curl -i -X POST http://localhost:8080/api/employment/opportunities/from-artifact/<ARTIFACT_ID>
```

Parse it:

```bash
curl -i -X POST http://localhost:8080/api/employment/opportunities/<OPPORTUNITY_ID>/parse
```

Score it:

```bash
curl -i -X POST http://localhost:8080/api/employment/opportunities/<OPPORTUNITY_ID>/score
```

Find possible duplicates:

```bash
curl -i "http://localhost:8080/api/employment/opportunities?source_artifact_id=<ARTIFACT_ID>"
curl -i "http://localhost:8080/api/employment/opportunities?source_url=https%3A%2F%2Fexample.com"
```

`from-artifact` is idempotent at the service layer. If an opportunity already exists for the artifact id or source URL, the existing opportunity is returned instead of silently creating a duplicate.

## Architecture

Important areas:

- `src/routes/`: HTTP route handlers and request/response models
- `src/op_tasks/`: saved tasks, manual runs, work items, artifacts, and runners
- `src/readers/`: URL reading and readable text extraction
- `src/context/`: saved reusable knowledge
- `src/domains/employment/`: job opportunity models, repository, and service logic
- `src/tools/`: system, Docker, Home Assistant, and tool registry
- `src/services/`: operator orchestration, audit, policy, LLM routing, and planning
- `src/db/`: database and audit repository support
- `src/adapters/`: external integration clients
- `operator-console/`: React/Vite browser console

## Safety Model

Actions are grouped by risk tier:

- Tier 0: read-only, safe to run automatically
- Tier 1: low-risk write actions
- Tier 2: confirmation-required actions
- Tier 3: blocked in v0.1

Examples of blocked v0.1 actions:

- unlocking doors
- opening garage doors
- disarming alarms
- arbitrary shell execution

## Development Checks

Backend:

```bash
cargo fmt
cargo check
```

Console:

```bash
cd operator-console
npm run build
```

## Near-Term Roadmap

- move artifact-to-context promotion logic out of the route handler
- add richer employment domain models for applications and application packets
- improve job parsing and scoring with saved context
- add scheduling only after manual Op Task runs are boring and reliable
- add stronger duplicate handling after reviewing existing opportunity data
- add deeper artifact search over `content_text`
