# local-operator

Local Operator is a local-first control and automation layer for:

- Docker / server management
- Home Assistant inspection and control
- future AI-assisted operations

## Current Status

Working today:
- Axum service startup
- `/health` endpoint
- config scaffolding
- initial project layout

Planned next:
- `/api/status`
- first real system tool
- Docker inspection tools
- Home Assistant reads

## v0.1 Goal

Operator v0.1 should be able to:

- report local system health
- inspect Docker container status
- inspect selected Home Assistant entity states
- summarize basic issues
- perform a small set of safe actions
- refuse higher-risk actions cleanly
- log what it did

## Safety Model

Actions are grouped by risk tier:

- **Tier 0**: read-only, safe to run automatically
- **Tier 1**: low-risk write actions
- **Tier 2**: confirmation-required actions
- **Tier 3**: blocked in v0.1

Examples of blocked v0.1 actions:

- unlocking doors
- opening garage doors
- disarming alarms
- arbitrary shell execution

## Planned Architecture

The project is organized around a few major areas:

- `routes/` — API surface
- `services/` — orchestration, policy, execution flow
- `tools/` — concrete capabilities like Docker, system, and Home Assistant
- `models/` — API, plan, policy, and audit data structures
- `db/` — persistence and audit support
- `adapters/` — integration clients for external systems

## Roadmap

### Near-term
- health endpoint
- status endpoint
- first real system tool
- first real Docker tool
- tracked Home Assistant state reads

### After that
- audit persistence
- CLI interface
- policy enforcement by action
- natural-language command routing
- controlled self-update path

## Run

```bash
cargo run
