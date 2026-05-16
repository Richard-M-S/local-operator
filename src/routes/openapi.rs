use axum::Json;
use serde_json::{json, Value};

use crate::domains::employment::models::DEFAULT_EMPLOYMENT_PROFILE_ID;

pub async fn openapi_json() -> Json<Value> {
    Json(json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Local Operator Tool API",
            "version": "0.1.0",
            "description": "Task-oriented OpenAPI tool surface for chat clients such as Open WebUI. Prefer the task lifecycle operations for work that may need tools, web access, durable tracking, artifacts, retries, or multiple steps."
        },
        "servers": [
            {
                "url": "/",
                "description": "Local Operator base URL"
            }
        ],
        "security": [
            { "BearerAuth": [] }
        ],
        "paths": {
            "/api/operator/chat": {
                "post": {
                    "operationId": "chatWithLocalOperator",
                    "summary": "Ask Local Operator to handle a natural-language request",
                    "description": "Fallback conversational endpoint. Do not prefer this for tool use, web search, employment workflows, artifact inspection, or multi-step work. For actionable user requests, use createTaskFromNaturalLanguage, then runTaskRequest, then showLatestArtifacts or continueFromArtifact.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ChatRequest" },
                                "examples": {
                                    "employmentSearch": {
                                        "summary": "Search employment opportunities",
                                        "value": {
                                            "message": "Search for jobs using my profile criteria and create opportunities for the best matches",
                                            "include_home": false,
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Chat response",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ChatResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/task-requests": {
                "post": {
                    "operationId": "createTaskFromNaturalLanguage",
                    "summary": "Create a durable task from natural language",
                    "description": "Use createTaskFromNaturalLanguage when the user asks Local Operator to do something that may require tools, web access, durable tracking, artifacts, retries, auditability, or multiple steps. This is the preferred first tool for requests like searching the web, reading URLs, finding jobs, creating opportunities, checking system status, or starting any task where the client does not know the internal task type. After this succeeds, call runTaskRequest with the returned task_request.id.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateTaskFromMessageRequest" },
                                "examples": {
                                    "employmentSearch": {
                                        "summary": "Create an employment search task",
                                        "value": {
                                            "message": "Search for Salesforce architect jobs using my profile criteria and create opportunities for the best matches.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "source": "open_webui"
                                        }
                                    },
                                    "readUrl": {
                                        "summary": "Create a URL reading task",
                                        "value": {
                                            "message": "Read https://example.com/job-posting and extract the useful details.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "source": "open_webui"
                                        }
                                    },
                                    "systemStatus": {
                                        "summary": "Create a system status task",
                                        "value": {
                                            "message": "Generate a system status report.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "source": "open_webui"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Created task request and task",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CreateTaskFromMessageResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/task-requests/{task_request_id}/run": {
                "post": {
                    "operationId": "runTaskRequest",
                    "summary": "Run a task created from natural language",
                    "description": "Use runTaskRequest immediately after createTaskFromNaturalLanguage when the user expects the task to execute. It runs the OpTask linked to the TaskRequest and returns a compact response with status, summary, artifact IDs, and next actions. Prefer this over profile-scoped internal run routes for Open WebUI tool use.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/TaskRequestId" }
                    ],
                    "requestBody": {
                        "required": false,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/RunTaskRequest" },
                                "examples": {
                                    "validateProfile": {
                                        "summary": "Run and validate profile ownership",
                                        "value": {
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID
                                        }
                                    },
                                    "runWithoutBody": {
                                        "summary": "Run using the task request alone",
                                        "value": {}
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Task run response",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/TaskRunResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/artifacts/latest": {
                "get": {
                    "operationId": "showLatestArtifacts",
                    "summary": "Show latest useful artifacts",
                    "description": "Use showLatestArtifacts when the user asks what happened, what was found, what the results were, what a task produced, or asks to see previous task output. Pass task_request_id when available to retrieve artifacts from the most recent task. Use include_content=false for a quick listing, and include_content=true when the user wants the actual saved results or artifact body.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        {
                            "name": "profile_id",
                            "in": "query",
                            "description": "Limit results to one profile. Use this when the user asks for recent results but no task_request_id is known.",
                            "schema": { "type": "string", "format": "uuid", "default": DEFAULT_EMPLOYMENT_PROFILE_ID },
                            "example": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        {
                            "name": "task_request_id",
                            "in": "query",
                            "description": "Best filter for follow-up questions like 'show me the results' after a task was created or run.",
                            "schema": { "type": "string", "format": "uuid" },
                            "example": "00000000-0000-0000-0000-000000000000"
                        },
                        {
                            "name": "task_id",
                            "in": "query",
                            "description": "Filter to artifacts produced by a specific OpTask.",
                            "schema": { "type": "string", "format": "uuid" }
                        },
                        {
                            "name": "run_id",
                            "in": "query",
                            "description": "Filter to artifacts produced by one task run.",
                            "schema": { "type": "string", "format": "uuid" }
                        },
                        {
                            "name": "artifact_type",
                            "in": "query",
                            "description": "Use this when the user asks for a specific kind of output, such as search results or scored matches.",
                            "schema": { "type": "string", "example": "search_result_set" },
                            "examples": {
                                "searchResults": { "value": "search_result_set" },
                                "readablePage": { "value": "readable_web_page" },
                                "scoredMatches": { "value": "scored_opportunity_matches" }
                            }
                        },
                        {
                            "name": "limit",
                            "in": "query",
                            "description": "Number of recent artifacts to return.",
                            "schema": { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 }
                        },
                        {
                            "name": "include_content",
                            "in": "query",
                            "description": "Set true when the user wants to read or answer from artifact contents. Leave false when only artifact IDs and names are needed.",
                            "schema": { "type": "boolean", "default": false }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Latest artifacts",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/LatestArtifactsResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/artifacts/{artifact_id}/continue": {
                "post": {
                    "operationId": "continueFromArtifact",
                    "summary": "Continue working from an artifact",
                    "description": "Use continueFromArtifact when the user wants to take the next step based on prior output, for example 'read the top results', 'score these jobs', 'create opportunities from those matches', 'summarize that page', or 'recommend next actions from this snapshot'. This creates a new TaskRequest and OpTask seeded from the artifact, runs it, and returns generated artifacts.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ArtifactId" }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ContinueArtifactRequest" },
                                "examples": {
                                    "scoreTopResults": {
                                        "summary": "Score the top search results",
                                        "value": {
                                            "message": "Read the top 3 results and score them against my profile.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID
                                        }
                                    },
                                    "createOpportunities": {
                                        "summary": "Create records from scored matches",
                                        "value": {
                                            "message": "Create opportunities for the matches that pass the score threshold.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID
                                        }
                                    },
                                    "summarizeSnapshot": {
                                        "summary": "Summarize a non-employment artifact",
                                        "value": {
                                            "message": "Summarize this artifact and recommend the next action.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Continuation task response",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ContinueArtifactResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/op-task-artifacts/{artifact_id}/content": {
                "get": {
                    "operationId": "getArtifactContent",
                    "summary": "Get full artifact content",
                    "description": "Use getArtifactContent when showLatestArtifacts returned an artifact ID and the user wants to read, quote, inspect, or answer from the full saved artifact content. Prefer showLatestArtifacts first when you do not know the artifact ID. Prefer continueFromArtifact when the user wants to take another action based on the artifact.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ArtifactId" }
                    ],
                    "responses": {
                        "200": {
                            "description": "Full artifact content",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ArtifactContentResponse" },
                                    "examples": {
                                        "searchResults": {
                                            "summary": "Search result set content",
                                            "value": {
                                                "artifact_id": "00000000-0000-0000-0000-000000000000",
                                                "name": "Employment opportunities search results",
                                                "artifact_type": "search_result_set",
                                                "content_text": "1. Salesforce Architect ...",
                                                "content_json": {
                                                    "query": "Salesforce architect jobs remote",
                                                    "results": []
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator/command": {
                "post": {
                    "operationId": "runLocalOperatorCommand",
                    "summary": "Run a mapped Local Operator command",
                    "description": "Runs short mapped commands such as status, docker status, or Home Assistant summaries. Potentially risky actions require confirm=true.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CommandRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Command response",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CommandResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/tools/execute": {
                "post": {
                    "operationId": "executeRegisteredTool",
                    "summary": "Execute a registered Local Operator tool",
                    "description": "Low-level tool execution. Use only when the exact tool name and arguments are known.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ToolExecuteRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Raw tool result",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object", "additionalProperties": true }
                                }
                            }
                        }
                    }
                }
            },
            "/api/employment/profiles/{profile_id}/op-tasks": {
                "post": {
                    "operationId": "createEmploymentOpTask",
                    "summary": "Create an employment-profile scoped task",
                    "description": "Create a task such as employment.search_opportunities. Run the returned task with runEmploymentOpTask.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ProfileId" }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateOpTaskRequest" },
                                "examples": {
                                    "employmentSearchTask": {
                                        "summary": "Create employment.search_opportunities task",
                                        "value": {
                                            "name": "Search opportunities",
                                            "task_type": "employment.search_opportunities",
                                            "description": "Search web/job sources from profile criteria.",
                                            "input_json": {
                                                "limit": 10,
                                                "create_opportunities": true
                                            },
                                            "enabled": true
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Created task",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "task": { "$ref": "#/components/schemas/OpTask" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/employment/profiles/{profile_id}/op-tasks/{task_id}/run": {
                "post": {
                    "operationId": "runEmploymentOpTask",
                    "summary": "Run an employment-profile scoped task",
                    "description": "Runs a previously created task and returns the task run, including any artifacts such as search_result_set.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ProfileId" },
                        { "$ref": "#/components/parameters/TaskId" }
                    ],
                    "responses": {
                        "200": {
                            "description": "Task run",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "run": { "$ref": "#/components/schemas/OpTaskRun" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/employment/profiles/{profile_id}/op-task-artifacts": {
                "get": {
                    "operationId": "listEmploymentTaskArtifacts",
                    "summary": "List task artifacts for an employment profile",
                    "description": "Use this to inspect durable task outputs. Filter artifact_type=search_result_set to retrieve saved search results.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ProfileId" },
                        {
                            "name": "artifact_type",
                            "in": "query",
                            "schema": { "type": "string", "example": "search_result_set" }
                        },
                        {
                            "name": "include_content",
                            "in": "query",
                            "schema": { "type": "boolean", "default": false }
                        },
                        {
                            "name": "limit",
                            "in": "query",
                            "schema": { "type": "integer", "minimum": 1, "maximum": 200, "default": 50 }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Artifacts",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "artifacts": {
                                                "type": "array",
                                                "items": { "$ref": "#/components/schemas/TaskArtifact" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/employment/profiles/{profile_id}/op-task-artifacts/{artifact_id}/content": {
                "get": {
                    "operationId": "getEmploymentTaskArtifactContent",
                    "summary": "Get full task artifact content",
                    "description": "Fetches the durable content_text and content_json for an artifact, including saved search_result_set results.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ProfileId" },
                        { "$ref": "#/components/parameters/ArtifactId" }
                    ],
                    "responses": {
                        "200": {
                            "description": "Artifact content",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ArtifactContentResponse" }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "BearerAuth": {
                    "type": "http",
                    "scheme": "bearer"
                }
            },
            "parameters": {
                "ProfileId": {
                    "name": "profile_id",
                    "in": "path",
                    "required": true,
                    "description": "Employment/profile scope for profile-specific routes.",
                    "schema": { "type": "string", "format": "uuid", "default": DEFAULT_EMPLOYMENT_PROFILE_ID }
                },
                "TaskId": {
                    "name": "task_id",
                    "in": "path",
                    "required": true,
                    "description": "OpTask identifier returned by createTaskFromNaturalLanguage or task listing endpoints.",
                    "schema": { "type": "string", "format": "uuid" }
                },
                "ArtifactId": {
                    "name": "artifact_id",
                    "in": "path",
                    "required": true,
                    "description": "Artifact identifier returned by runTaskRequest, showLatestArtifacts, continueFromArtifact, or getArtifactContent.",
                    "schema": { "type": "string", "format": "uuid" }
                },
                "TaskRequestId": {
                    "name": "task_request_id",
                    "in": "path",
                    "required": true,
                    "description": "TaskRequest identifier returned by createTaskFromNaturalLanguage. Use it with runTaskRequest and showLatestArtifacts.",
                    "schema": { "type": "string", "format": "uuid" }
                }
            },
            "schemas": {
                "ChatRequest": {
                    "type": "object",
                    "required": ["message"],
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Natural-language request for Local Operator."
                        },
                        "include_home": {
                            "type": "boolean",
                            "default": true
                        },
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        }
                    }
                },
                "ChatResponse": {
                    "type": "object",
                    "properties": {
                        "ok": { "type": "boolean" },
                        "mode": { "type": "string" },
                        "message": { "type": "string" },
                        "data": { "type": "object", "additionalProperties": true }
                    }
                },
                "CommandRequest": {
                    "type": "object",
                    "required": ["input"],
                    "properties": {
                        "input": { "type": "string" },
                        "confirm": { "type": "boolean", "default": false }
                    }
                },
                "CommandResponse": {
                    "type": "object",
                    "properties": {
                        "ok": { "type": "boolean" },
                        "mode": { "type": "string" },
                        "message": { "type": "string" },
                        "data": { "type": "object", "additionalProperties": true }
                    }
                },
                "ToolExecuteRequest": {
                    "type": "object",
                    "required": ["tool"],
                    "properties": {
                        "tool": {
                            "type": "string",
                            "description": "Registered tool name, for example system.get_status."
                        },
                        "args": {
                            "type": "object",
                            "additionalProperties": true,
                            "default": {}
                        },
                        "confirm": {
                            "type": "boolean",
                            "default": false
                        }
                    }
                },
                "CreateTaskFromMessageRequest": {
                    "type": "object",
                    "required": ["message"],
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Plain-English request to classify into a durable OpTask."
                        },
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        "source": {
                            "type": "string",
                            "default": "api",
                            "example": "open_webui"
                        }
                    }
                },
                "CreateTaskFromMessageResponse": {
                    "type": "object",
                    "properties": {
                        "task_request": { "$ref": "#/components/schemas/TaskRequest" },
                        "task": { "$ref": "#/components/schemas/OpTask" },
                        "intent": { "type": "string" },
                        "suggested_next_action": {
                            "type": "object",
                            "additionalProperties": true
                        }
                    }
                },
                "TaskRequest": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "profile_id": { "type": "string", "format": "uuid" },
                        "source": { "type": "string" },
                        "user_request": { "type": "string" },
                        "intent": { "type": "string", "nullable": true },
                        "status": { "type": "string" },
                        "op_task_id": { "type": "string", "format": "uuid", "nullable": true },
                        "run_id": { "type": "string", "format": "uuid", "nullable": true },
                        "primary_artifact_id": { "type": "string", "format": "uuid", "nullable": true }
                    }
                },
                "RunTaskRequest": {
                    "type": "object",
                    "properties": {
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Optional profile ID guard. If supplied, the task request must belong to this profile.",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        }
                    }
                },
                "TaskRunArtifactSummary": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "artifact_type": { "type": "string" },
                        "name": { "type": "string" }
                    }
                },
                "TaskRunResponse": {
                    "type": "object",
                    "properties": {
                        "ok": { "type": "boolean" },
                        "task_request_id": { "type": "string", "format": "uuid" },
                        "task_id": { "type": "string", "format": "uuid" },
                        "run_id": { "type": "string", "format": "uuid" },
                        "status": {
                            "type": "string",
                            "enum": ["Pending", "Running", "Succeeded", "Failed", "Cancelled"]
                        },
                        "summary": { "type": "string", "nullable": true },
                        "artifacts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/TaskRunArtifactSummary" }
                        },
                        "next_actions": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "next_suggested_action": {
                            "type": "object",
                            "additionalProperties": true
                        }
                    }
                },
                "LatestArtifactSummary": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "profile_id": { "type": "string", "format": "uuid" },
                        "run_id": { "type": "string", "format": "uuid" },
                        "work_item_id": { "type": "string", "format": "uuid", "nullable": true },
                        "artifact_type": { "type": "string" },
                        "name": { "type": "string" },
                        "location": { "type": "string", "nullable": true },
                        "created_at": { "type": "string", "format": "date-time" },
                        "metadata": { "type": "object", "additionalProperties": true, "nullable": true },
                        "content_text": { "type": "string", "nullable": true },
                        "content_json": { "type": "object", "additionalProperties": true, "nullable": true }
                    }
                },
                "LatestArtifactsResponse": {
                    "type": "object",
                    "properties": {
                        "artifacts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/LatestArtifactSummary" }
                        },
                        "filters": {
                            "type": "object",
                            "additionalProperties": true
                        },
                        "next_actions": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                },
                "ContinueArtifactRequest": {
                    "type": "object",
                    "required": ["message"],
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Plain-English instruction for how to continue from the artifact."
                        },
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        "source": {
                            "type": "string",
                            "default": "artifact_continue"
                        }
                    }
                },
                "OpTaskRunSummary": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "status": {
                            "type": "string",
                            "enum": ["Pending", "Running", "Succeeded", "Failed", "Cancelled"]
                        },
                        "summary": { "type": "string", "nullable": true }
                    }
                },
                "ContinueArtifactResponse": {
                    "type": "object",
                    "properties": {
                        "ok": { "type": "boolean" },
                        "intent": { "type": "string" },
                        "task_request": { "$ref": "#/components/schemas/TaskRequest" },
                        "task": { "$ref": "#/components/schemas/OpTask" },
                        "run": { "$ref": "#/components/schemas/OpTaskRunSummary" },
                        "artifacts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/TaskRunArtifactSummary" }
                        },
                        "next_actions": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "next_suggested_action": {
                            "type": "object",
                            "additionalProperties": true
                        }
                    }
                },
                "CreateOpTaskRequest": {
                    "type": "object",
                    "required": ["name", "task_type"],
                    "properties": {
                        "name": { "type": "string" },
                        "task_type": {
                            "type": "string",
                            "enum": [
                                "employment.search_opportunities",
                                "reader.search_web",
                                "reader.read_url",
                                "system.status_report"
                            ]
                        },
                        "description": { "type": "string", "nullable": true },
                        "input_json": {
                            "type": "object",
                            "additionalProperties": true,
                            "description": "For employment.search_opportunities: { limit?: number, create_opportunities?: boolean }."
                        },
                        "enabled": { "type": "boolean", "default": true }
                    }
                },
                "OpTask": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "profile_id": { "type": "string", "format": "uuid" },
                        "task_type": { "type": "string" },
                        "name": { "type": "string" },
                        "description": { "type": "string", "nullable": true },
                        "input_json": { "type": "object", "additionalProperties": true },
                        "status": { "type": "string" }
                    }
                },
                "OpTaskRun": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "profile_id": { "type": "string", "format": "uuid" },
                        "task_id": { "type": "string", "format": "uuid" },
                        "status": { "type": "string" },
                        "summary": { "type": "string", "nullable": true },
                        "work_items": {
                            "type": "array",
                            "items": { "type": "object", "additionalProperties": true }
                        },
                        "artifacts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/TaskArtifact" }
                        }
                    }
                },
                "TaskArtifact": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "profile_id": { "type": "string", "format": "uuid" },
                        "run_id": { "type": "string", "format": "uuid" },
                        "work_item_id": { "type": "string", "format": "uuid", "nullable": true },
                        "name": { "type": "string" },
                        "artifact_type": { "type": "string" },
                        "location": { "type": "string", "nullable": true },
                        "metadata": { "type": "object", "additionalProperties": true, "nullable": true },
                        "content_text": { "type": "string", "nullable": true },
                        "content_json": { "type": "object", "additionalProperties": true, "nullable": true }
                    }
                },
                "ArtifactContentResponse": {
                    "type": "object",
                    "properties": {
                        "artifact_id": { "type": "string", "format": "uuid" },
                        "name": { "type": "string" },
                        "artifact_type": { "type": "string" },
                        "content_text": { "type": "string", "nullable": true },
                        "content_json": { "type": "object", "additionalProperties": true, "nullable": true }
                    }
                }
            }
        }
    }))
}
