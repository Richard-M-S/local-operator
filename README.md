# local-operator

Local Operator is a local-first automation service for server operations, profile-scoped employment workflows, durable task execution, reusable artifacts, context promotion, chat memory, and Home Assistant inspection.

It runs as a Rust/Axum API with SQLite persistence and a React/Vite operator console. The API can also be used from OpenAI-compatible chat clients through `/v1/models`, `/v1/chat/completions`, and the generated OpenAPI tool document at `/openapi.json`.

## Current Status

Working today:

- Axum API server with SQLite migrations and persistence
- optional bearer-token auth for protected routes
- audited wrapper services for tool and model execution
- system, Docker, and Home Assistant read-only tool modules
- Home Assistant overview plus normalized energy/HVAC snapshots for climate, weather, power, energy, battery, pricing, and helper entities
- OpenAI-compatible model and chat-completion routes backed by the local LLM router
- chat/session memory that records sessions, messages, task requests, task runs, artifacts, and follow-up artifact references
- natural-language operator chat that can read URLs, search the web, search employment opportunities, and create task-backed artifacts
- task request intake that turns Open WebUI/API/console requests into durable Op Tasks
- Op Tasks with planned work items, manual runs, run history, artifact references, and artifact content storage
- task-oriented OpenAPI operations for creating tasks, running tasks, showing artifacts, and continuing from artifacts
- URL reading through `reader.read_url` and web search through `reader.search_web`
- profile criteria driven employment search through `employment.search_opportunities`
- artifact listing, latest-artifact discovery, artifact continuation, detail, content, and save-to-context promotion routes
- saved context notes scoped to employment profiles, with list, create, get, and search routes
- employment profiles with criteria, notes, and email fields
- employment opportunities scoped to profiles, including parse, score, cover-letter, archive, reject, restore, and artifact/source duplicate awareness
- domain catalog for Home Operations, Research, Code, Infrastructure, Knowledge, Calendar, and Operator task-improvement workflows
- Operator meta workflow for failed-task diagnosis, patch-plan artifacts, implementation task-set artifacts, and read-only self-review
- manual and optional OpenAI API ChatGPT escalation artifacts with redaction/policy metadata and response linking
- meta task continuation surfaces for operator diagnostics and escalation responses
- Operator Console for chat, profile setup, tasks, artifacts, daily review, and opportunity review

Still intentionally basic:

- no scheduler yet; Op Tasks are created and run manually or from chat
- employment parsing, scoring, and cover-letter generation depend on local LLM quality
- context search uses SQL-style matching, not embeddings
- duplicate protection is mostly application-level
- Home Assistant tools are read-only; this project does not currently actuate devices
- operator meta tasks do not write code, change config, restart services, or execute implementation tasks

## Run

The default API config is in `config/default.toml`.

## Development Tooling

Recommended local tools for maintenance and debugging:

- `rg` (ripgrep) for fast file and symbol search during review.
- `grep`, `sed`, and `cat` remain acceptable fallbacks when `rg` is unavailable.

Optional productivity helper:

- `Makefile` targets below for a consistent build loop:

```bash
make run
make check
make lint
make test
make fmt
make ci
make search q="operator_meta"
make run-console
```

If `rg` is not installed, `make search` uses `grep` automatically.

Example:

```bash
rg -n "operator_meta" src
```

```bash
cargo run
```

Default server settings:

- host: `0.0.0.0`
- port: `8080`
- database: `sqlite:///opt/local-operator/data/operator.db`
- auth: disabled by default
- auth token env when enabled: `OPERATOR_API_TOKEN`
- Home Assistant URL: `http://localhost:8123`
- Home Assistant token env: `HA_TOKEN`
- local LLM provider: Ollama at `http://localhost:11434`

Run the console:

```bash
cd operator-console
npm run dev
```

The console defaults to `http://localhost:8080` as the API base and stores the API base, token, profile id, artifact filter, opportunity status filter, and fit filter in browser local storage.

## Console

The Operator Console is the easiest way to use the current workflow.

Current console areas:

- Operator chat for task-backed requests
- profile panel for selecting or editing an employment profile
- Daily Review for scanning active opportunity queues
- Tasks for creating and running profile-scoped Op Tasks
- Employment for reviewing, parsing, scoring, rejecting, archiving, restoring, and generating cover letters
- Artifacts for reviewing readable pages, search result sets, and other saved task outputs

## Profiles

Employment data is profile-scoped. The default profile id is:

```text
00000000-0000-0000-0000-000000000001
```

Legacy routes such as `/api/context`, `/api/op-tasks`, and `/api/employment/opportunities` still work by using the default profile. New clients should prefer the profile-scoped routes under:

```text
/api/employment/profiles/:profile_id/...
```

Profile criteria are used by employment search tasks and chat requests such as "search for jobs using my profile criteria and create opportunities".

## OpenAI-Compatible Chat

Local Operator exposes a minimal OpenAI-compatible surface:

- `GET /v1/models`
- `POST /v1/chat/completions`

The built-in model id is:

```text
local-operator-home
```

For `local-operator-home`, chat requests are routed through Local Operator instead of directly to an LLM. The route can:

- create URL-reading tasks when the user asks to read a URL
- create web-search tasks for search requests
- create employment-search tasks from profile criteria
- create operator diagnostic, patch-plan, task-set, and escalation tasks
- save task request, run, and artifact ids into session memory
- answer artifact follow-ups using the last artifact from the session

The chat completion request accepts OpenAI-style `messages` and also supports:

- `session_id`
- `profile_id`
- `metadata.session_id`
- `metadata.profile_id`
- `metadata.conversation_id`
- `metadata.thread_id`

If no session id is supplied, the API creates or resolves a session from the external conversation id or `user` value.

## Task Lifecycle

Local Operator treats meaningful work as task-backed execution:

1. A chat client, API caller, or console request creates a `TaskRequest`.
2. The request is classified into an `OpTask`, such as `reader.search_web`, `employment.search_opportunities`, `operator.review_failed_task`, or `operator.escalate_to_chatgpt`.
3. `TaskPlanner` or a domain planner creates ordered `OpWorkItem` steps before execution.
4. Work items include explicit `model_purpose` and model routing hints for auditability.
5. The runner executes the task and saves durable artifacts.
6. The client can inspect latest artifacts or continue from an artifact.

Preferred Open WebUI flow:

```text
POST /api/task-requests
POST /api/task-requests/:task_request_id/run
GET  /api/artifacts/latest
POST /api/artifacts/:artifact_id/continue
```

OpenAPI operation ids for this flow are:

- `createTaskFromNaturalLanguage`
- `runTaskRequest`
- `showLatestArtifacts`
- `continueFromArtifact`
- `getArtifactContent`

Use the explicit `/api/operator-meta/...` routes for debugging/admin workflows; use `/api/task-requests` for normal natural-language use.

## Domains

The domain catalog describes task types, input schemas, planners, work item types, required tools, artifact types, model purposes, policy tiers, safety levels, and continuation rules.

Routes:

- `GET /api/domains`
- `GET /api/domains/:domain`

Current domains:

- `home`: Home Assistant-backed status, energy, HVAC, and safety review
- `research`: URL reading, web search, and future web briefings
- `code`: repo review and future patch/test workflows
- `infrastructure`: local system and container monitoring
- `knowledge`: document/artifact intake and context promotion
- `calendar`: planning and future calendar workflows
- `operator`: Local Operator self-review and task-improvement workflows

## Operator Meta Workflow

The operator domain is the read-only self-improvement loop.

Implemented task types:

- `operator.review_failed_task`: loads a failed run, task definition, artifacts, recent audit entries, classifies the failure, and saves an `operator_task_diagnostic` artifact
- `operator.generate_patch_plan`: consumes an `operator_task_diagnostic` and saves an `operator_patch_plan`
- `operator.convert_recommendation_to_tasks`: consumes an `operator_patch_plan` and saves an `operator_implementation_task_set`
- `operator.escalate_to_chatgpt`: collects and redacts context, then creates a `chatgpt_escalation_request` artifact; optional OpenAI API mode saves a linked response artifact

Current safety boundaries (first three levels):

- level 1: diagnose only (read task state, artifacts, summarize failure)
- level 2: plan only (generate patch plans, specs, OpenAPI update plans)
- level 3: create draft tasks (confirmation required)

Operator artifact types:

- `operator_task_diagnostic`
- `operator_gap_analysis`
- `operator_task_type_spec`
- `operator_tool_spec`
- `operator_patch_plan`
- `operator_test_plan`
- `operator_openapi_review`
- `operator_implementation_task_set`
- `chatgpt_escalation_request`
- `chatgpt_escalation_response`

Operator meta routes:

- `GET /api/operator/meta/capabilities`
- `GET /api/operator/meta/state`
- `POST /api/operator-meta/review-failed-task`
- `POST /api/operator-meta/review-recent-tasks`
- `POST /api/operator-meta/generate-patch-plan`
- `POST /api/operator-meta/escalations`
- `POST /api/operator-meta/escalations/:artifact_id/response`
- `POST /api/operator-meta/artifacts/:artifact_id/convert-to-tasks`
- `GET /api/operator-meta/diagnostics`

The explicit operator-meta routes are primarily for debugging/admin tool use. They create durable task requests and tasks where appropriate; running still goes through the normal task run endpoint.

## Artifact Continuation

Artifacts include enough metadata for clients to ask "what can happen next?" and continue from prior output.

Important continuation examples:

- `search_result_set` -> read URLs, extract candidates, score matches
- `readable_web_page` -> summarize, extract structured data, promote to context
- `operator_task_diagnostic` -> generate patch plan, escalate to ChatGPT, convert recommendation to tasks
- `operator_patch_plan` -> convert recommendation to task set
- `chatgpt_escalation_response` -> generate patch plan, convert to tasks, summarize

Continuation controls:

- `run_immediately`: `false` creates only a task request/task (no run link, status remains `created`); `true` executes and links a run.
- `run_immediately`: defaults to `true` for most clients.
- `create_tasks`: enables task-creation follow-up actions where supported.
- `confirm`: carried into confirmation-gated actions.

Latest artifact responses include `allowed_continuations`, and continuation responses include the source artifact type plus allowed continuations.

## Main API Routes

Public:

- `GET /health`
- `GET /openapi.json`
- `GET /api/tools`
- `GET /api/tools/openapi.json`

Status and operator:

- `GET /api/status`
- `GET /api/domains`
- `GET /api/domains/:domain`
- `POST /api/operator/command`
- `POST /api/operator/chat`
- `POST /api/tools/execute`
- `GET /api/audit/recent`

Task lifecycle:

- `POST /api/task-requests`
- `POST /api/task-requests/:task_request_id/run`
- `GET /api/artifacts/latest`
- `POST /api/artifacts/:artifact_id/continue`
- `POST /api/artifacts/chatgpt-escalation-requests`
- `POST /api/artifacts/:artifact_id/chatgpt-escalation-response`

Operator meta:

- `GET /api/operator/meta/capabilities`
- `GET /api/operator/meta/state`
- `POST /api/operator-meta/review-failed-task`
- `POST /api/operator-meta/review-recent-tasks`
- `POST /api/operator-meta/generate-patch-plan`
- `POST /api/operator-meta/escalations`
- `POST /api/operator-meta/escalations/:artifact_id/response`
- `POST /api/operator-meta/artifacts/:artifact_id/convert-to-tasks`
- `GET /api/operator-meta/diagnostics`

OpenAI compatibility:

- `GET /v1/models`
- `POST /v1/chat/completions`

Employment profiles:

- `GET /api/employment/profiles`
- `POST /api/employment/profiles`
- `GET /api/employment/profiles/:profile_id`
- `PUT /api/employment/profiles/:profile_id`

Profile-scoped context:

- `GET /api/employment/profiles/:profile_id/context`
- `POST /api/employment/profiles/:profile_id/context`
- `GET /api/employment/profiles/:profile_id/context/search?q=...`

Default-profile context:

- `GET /api/context`
- `POST /api/context`
- `GET /api/context/search?q=...`
- `GET /api/context/:id`

Profile-scoped Op Tasks:

- `GET /api/employment/profiles/:profile_id/op-tasks`
- `POST /api/employment/profiles/:profile_id/op-tasks`
- `POST /api/employment/profiles/:profile_id/op-tasks/:id/run`
- `GET /api/employment/profiles/:profile_id/op-tasks/:id/runs`
- `GET /api/employment/profiles/:profile_id/op-task-artifacts`
- `GET /api/employment/profiles/:profile_id/op-task-artifacts/:id`
- `GET /api/employment/profiles/:profile_id/op-task-artifacts/:id/content`
- `POST /api/employment/profiles/:profile_id/op-task-artifacts/:id/save-context`

Default-profile Op Tasks:

- `GET /api/op-tasks`
- `POST /api/op-tasks`
- `GET /api/op-tasks/:id`
- `POST /api/op-tasks/:id/run`
- `GET /api/op-tasks/:id/runs`
- `GET /api/op-task-runs/:id`
- `GET /api/op-task-artifacts`
- `GET /api/op-task-artifacts/:id`
- `GET /api/op-task-artifacts/:id/content`
- `POST /api/op-task-artifacts/:id/save-context`

Profile-scoped employment opportunities:

- `GET /api/employment/profiles/:profile_id/opportunities`
- `POST /api/employment/profiles/:profile_id/opportunities`
- `GET /api/employment/profiles/:profile_id/opportunities/:id`
- `POST /api/employment/profiles/:profile_id/opportunities/:id/parse`
- `POST /api/employment/profiles/:profile_id/opportunities/:id/score`
- `POST /api/employment/profiles/:profile_id/opportunities/:id/cover-letter`
- `POST /api/employment/profiles/:profile_id/opportunities/:id/archive`
- `POST /api/employment/profiles/:profile_id/opportunities/:id/reject`
- `POST /api/employment/profiles/:profile_id/opportunities/:id/restore`
- `POST /api/employment/profiles/:profile_id/opportunities/from-artifact/:artifact_id`

Default-profile employment opportunities:

- `GET /api/employment/opportunities`
- `POST /api/employment/opportunities`
- `GET /api/employment/opportunities/:id`
- `POST /api/employment/opportunities/:id/parse`
- `POST /api/employment/opportunities/:id/score`
- `POST /api/employment/opportunities/:id/archive`
- `POST /api/employment/opportunities/:id/reject`
- `POST /api/employment/opportunities/:id/restore`
- `POST /api/employment/opportunities/from-artifact/:artifact_id`

## Registered Tools

Tools are executed through:

```text
POST /api/tools/execute
```

Current registered tools with the default config:

- `system.get_status`
- `docker.list_containers`
- `ha.get_summary`
- `ha.get_states`
- `ha.get_entity`
- `ha.search_entities`
- `ha.get_overview`
- `ha.get_energy_hvac_snapshot`

Docker and Home Assistant tools are only registered when their config blocks are enabled.

All current Home Assistant tools are read-only. `ha.get_overview` returns a compact house summary for people, mode, locks, doors, vacuums, weather, media players, energy devices, and problem entities. `ha.get_energy_hvac_snapshot` returns a normalized planning snapshot for climate, temperature, humidity, weather, power, energy, battery, energy price, and helper entities.

Example:

```bash
curl -i -X POST http://localhost:8080/api/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "ha.get_energy_hvac_snapshot",
    "args": {},
    "confirm": false
  }'
```

## Safety Model

Tool and model execution go through shared execution services that record audit history and normalize results. Tool execution also goes through the policy engine:

- Tier 0: read-only actions, allowed automatically
- Tier 1: low-risk write actions, confirmation or config opt-in required
- Tier 2: higher-risk actions, confirmation or config opt-in required
- Tier 3: blocked by default

The tools currently registered by the default config are Tier 0. The policy model exists so future write-capable tools can be added without bypassing confirmation checks.

Operator meta has an additional safety ladder:

- Level 1: Diagnose only. Read task state and artifacts, summarize failures, suggest fixes.
- Level 2: Plan only. Generate patch plans, task specs, tool specs, and OpenAPI update plans as artifacts.
- Level 3: Create draft tasks. Allowed only with confirmation; draft tasks are not executed automatically.
- Level 4: Modify repo/code/config. Blocked for now.
- Level 5: Execute operational changes such as restarting containers, changing Home Assistant automations, or altering secrets/config. Blocked for now.

ChatGPT escalation has separate privacy handling:

- technical-only escalation is low friction
- personal or employment escalation requires confirmation
- secret-bearing context is blocked
- redaction and policy summaries are stored in artifact metadata

## Op Task Examples

Create a profile-scoped URL reader task:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/op-tasks \
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
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/op-tasks/<TASK_ID>/run
```

Create a profile-scoped employment search task:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/op-tasks \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Search Opportunities",
    "task_type": "employment.search_opportunities",
    "description": "Search from the profile criteria.",
    "input_json": {
      "limit": 10,
      "create_opportunities": true
    },
    "enabled": true
  }'
```

List profile artifacts without full content:

```bash
curl -i "http://localhost:8080/api/employment/profiles/<PROFILE_ID>/op-task-artifacts?artifact_type=search_result_set&limit=20"
```

List latest artifacts across task request, task, run, profile, or artifact type:

```bash
curl -i "http://localhost:8080/api/artifacts/latest?profile_id=<PROFILE_ID>&limit=5"
```

Load artifact content separately:

```bash
curl -i http://localhost:8080/api/employment/profiles/<PROFILE_ID>/op-task-artifacts/<ARTIFACT_ID>/content
```

Continue from an artifact:

```bash
curl -i -X POST http://localhost:8080/api/artifacts/<ARTIFACT_ID>/continue \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Summarize this artifact and recommend the next action.",
    "profile_id": "<PROFILE_ID>"
  }'
```

Create and run a task from natural language:

```bash
curl -i -X POST http://localhost:8080/api/task-requests \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Search for Salesforce architect jobs using my profile criteria and create opportunities for the best matches.",
    "profile_id": "<PROFILE_ID>",
    "source": "api"
  }'

curl -i -X POST http://localhost:8080/api/task-requests/<TASK_REQUEST_ID>/run
```

## Context Example

Create a reusable context note for a profile:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/context \
  -H "Content-Type: application/json" \
  -d '{
    "kind": "career_profile",
    "title": "Preferred Role Direction",
    "body": "Prioritize Salesforce Architect, Platform Architect, and internal tooling roles over pure Salesforce Admin roles.",
    "tags": ["employment", "career", "preferences"]
  }'
```

Search profile context:

```bash
curl -i "http://localhost:8080/api/employment/profiles/<PROFILE_ID>/context/search?q=Salesforce"
```

Promote an artifact into profile context:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/op-task-artifacts/<ARTIFACT_ID>/save-context \
  -H "Content-Type: application/json" \
  -d '{
    "kind": "document_note",
    "title": "Example Domain Reference",
    "tags": ["web", "reader"],
    "body_source": "content_text"
  }'
```

Valid `body_source` values are `content_text`, `content_json`, and `metadata`.

## Employment Examples

Create an opportunity from a readable artifact:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities/from-artifact/<ARTIFACT_ID>
```

Parse it:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities/<OPPORTUNITY_ID>/parse
```

Score it:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities/<OPPORTUNITY_ID>/score
```

Generate a cover letter:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities/<OPPORTUNITY_ID>/cover-letter \
  -H "Content-Type: application/json" \
  -d '{
    "direction": "Concise, specific, and tailored to the role."
  }'
```

Archive, reject, or restore an opportunity:

```bash
curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities/<OPPORTUNITY_ID>/archive \
  -H "Content-Type: application/json" \
  -d '{ "reason": "Not a fit right now" }'

curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities/<OPPORTUNITY_ID>/reject \
  -H "Content-Type: application/json" \
  -d '{ "reason": "Too much on-site work" }'

curl -i -X POST http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities/<OPPORTUNITY_ID>/restore \
  -H "Content-Type: application/json" \
  -d '{ "reason": "Review again" }'
```

Find possible duplicates:

```bash
curl -i "http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities?source_artifact_id=<ARTIFACT_ID>"
curl -i "http://localhost:8080/api/employment/profiles/<PROFILE_ID>/opportunities?source_url=https%3A%2F%2Fexample.com"
```

`from-artifact` is idempotent at the service layer. If an opportunity already exists for the artifact id or source URL in the profile, the existing opportunity is returned instead of silently creating a duplicate.

## Operator Chat Examples

Ask Local Operator to read a URL:

```bash
curl -i -X POST http://localhost:8080/api/operator/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Read https://example.com/jobs/123",
    "include_home": false,
    "profile_id": "00000000-0000-0000-0000-000000000001"
  }'
```

Search from profile criteria and create opportunities:

```bash
curl -i -X POST http://localhost:8080/api/operator/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Search for jobs using my profile criteria and create opportunities for the best matches",
    "include_home": false,
    "profile_id": "00000000-0000-0000-0000-000000000001"
  }'
```

Create an operator diagnostic task through natural-language intake:

```bash
curl -i -X POST http://localhost:8080/api/task-requests \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Review failed task run 00000000-0000-0000-0000-000000000000 and recommend fixes.",
    "profile_id": "00000000-0000-0000-0000-000000000001",
    "source": "api"
  }'
```

Inspect operator-domain capabilities and safety boundaries:

```bash
curl -i http://localhost:8080/api/operator/meta/capabilities
```

OpenAI-compatible call using session memory:

```bash
curl -i -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "local-operator-home",
    "user": "open-webui-thread-1",
    "metadata": {
      "profile_id": "00000000-0000-0000-0000-000000000001"
    },
    "messages": [
      {
        "role": "user",
        "content": "Search for remote Salesforce architect jobs using my profile criteria and create opportunities"
      }
    ]
  }'
```

## Architecture

Development checks:

```bash
cargo fmt
cargo check
```

Console checks:

```bash
cd operator-console
npm run build
```

Important areas:

- `src/routes/`: HTTP route handlers and request/response models
- `src/routes/openapi.rs`: generated OpenAPI tool document for chat clients
- `src/routes/openai_compat.rs`: OpenAI-compatible chat/model API and session resolution
- `src/op_tasks/`: saved tasks, manual runs, work items, artifacts, and runners
- `src/readers/`: URL reading, readable text extraction, and web search
- `src/context/`: saved reusable knowledge scoped by profile
- `src/domains/catalog.rs`: domain catalog, model purposes, policy tiers, safety levels, and continuation rules
- `src/domains/employment/`: profiles, opportunities, scoring, cover letters, and employment context
- `src/domains/operator/`: operator meta models, planner, service, and debug/admin routes
- `src/session_memory.rs`: chat sessions, messages, task requests, and task links
- `src/tools/`: system, Docker, Home Assistant, and tool registry
- `src/services/`: operator orchestration, audit, policy, LLM routing, and planning
- `src/db/`: database and audit repository support
- `src/adapters/`: external integration clients

## Data Model Notes

The current migrations create tables for:

- Op Tasks, Op Task runs, work items, artifacts, and artifact content
- saved context notes
- employment opportunities and source-artifact indexes
- employment profiles and criteria
- employment scoring output fields
- task requests and task links
- chat sessions and chat messages

This makes task output durable: chat-created searches, URL reads, opportunity records, context promotions, and follow-up artifact references survive process restarts.

Work items are currently stored inside `op_task_runs.work_items` JSON. The operator meta state endpoint exposes them through runs today; promoting work items into their own table is a later maturation step.
