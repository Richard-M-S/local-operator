use axum::Json;
use serde_json::{json, Value};

use crate::domains::employment::models::DEFAULT_EMPLOYMENT_PROFILE_ID;

pub async fn openapi_json() -> Json<Value> {
    Json(json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Local Operator Tool API",
            "version": "0.1.0",
            "description": "OpenAPI tool surface for calling Local Operator actions from chat clients such as Open WebUI."
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
                    "description": "Use this for general Local Operator chat and task-backed requests. It can read URLs, search for employment opportunities from profile criteria, create opportunity records when requested, and answer with top results.",
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
                    "schema": { "type": "string", "format": "uuid", "default": DEFAULT_EMPLOYMENT_PROFILE_ID }
                },
                "TaskId": {
                    "name": "task_id",
                    "in": "path",
                    "required": true,
                    "schema": { "type": "string", "format": "uuid" }
                },
                "ArtifactId": {
                    "name": "artifact_id",
                    "in": "path",
                    "required": true,
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
