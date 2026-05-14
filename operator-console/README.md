# Operator Console

React/Vite console for Local Operator.

## Run

```bash
npm run dev
```

The console is designed to talk to the Local Operator API, usually:

```text
http://localhost:8080
```

API base, bearer token, artifact type filter, and opportunity status filter are stored in browser local storage.

## Current Features

- read a job URL into a readable artifact
- read a job URL and create an employment opportunity
- review artifacts by readable text, raw JSON, and source metadata
- review opportunities by summary, parsed fields, raw JSON, and source artifact
- create, parse, and score opportunities from artifacts
- warn when an artifact or source URL already has a matching opportunity
- status and artifact type filter presets
- daily review dashboard

## Build

```bash
npm run build
```
