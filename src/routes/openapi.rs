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
                    "description": "Use createTaskFromNaturalLanguage when the user asks Local Operator to do something that may require tools, web access, durable tracking, artifacts, retries, auditability, or multiple steps. This is the preferred first tool for requests like searching the web, reading URLs, finding jobs, creating opportunities, checking system status, preparing a manual ChatGPT escalation request, or starting any task where the client does not know the internal task type. After this succeeds, call runTaskRequest with the returned task_request.id.",
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
                                    },
                                    "reviewFailedTask": {
                                        "summary": "Create an operator diagnostic task",
                                        "value": {
                                            "message": "Review failed task run 00000000-0000-0000-0000-000000000000 and recommend a fix.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "source": "open_webui"
                                        }
                                    },
                                    "manualChatGptEscalation": {
                                        "summary": "Create a manual ChatGPT escalation request task",
                                        "value": {
                                            "message": "Escalate this task to ChatGPT in manual mode and prepare a redacted request I can paste.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "source": "open_webui"
                                        }
                                    },
                                    "openAiChatGptEscalation": {
                                        "summary": "Create an OpenAI API escalation task",
                                        "value": {
                                            "message": "Escalate this technical task using the OpenAI API and save the structured response.",
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
            "/api/domains": {
                "get": {
                    "operationId": "listLocalOperatorDomains",
                    "summary": "List Local Operator domains",
                    "description": "Returns the prioritized domain catalog used to organize task types, input schemas, planners, work item types, required tools, artifacts, model purposes, policy tiers, and continuation rules. Use this to understand what Local Operator can do now versus what is planned.",
                    "security": [{ "BearerAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Domain catalog",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/DomainCatalogResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/domains/{domain}": {
                "get": {
                    "operationId": "getLocalOperatorDomain",
                    "summary": "Get one Local Operator domain",
                    "description": "Returns one domain descriptor, including task types and continuation rules. Domain IDs include home, research, code, infrastructure, knowledge, calendar, and operator.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        {
                            "name": "domain",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string",
                                "enum": ["home", "research", "code", "infrastructure", "knowledge", "calendar", "operator"]
                            },
                            "example": "operator"
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Domain descriptor",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/DomainDescriptor" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator/meta/capabilities": {
                "get": {
                    "operationId": "getOperatorMetaCapabilities",
                    "summary": "Get OperatorMetaService capabilities",
                    "description": "Returns operator-domain service capabilities and safety boundaries. Level 1 diagnoses only, Level 2 plans only, Level 3 creates draft tasks with confirmation, Level 4 repo/code/config modification is blocked for now, and Level 5 operational changes are blocked for now.",
                    "security": [{ "BearerAuth": [] }],
                    "responses": {
                        "200": {
                            "description": "Operator meta capabilities",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "additionalProperties": true
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator/meta/state": {
                "get": {
                    "operationId": "inspectOperatorTaskState",
                    "summary": "Inspect existing task, run, and artifact state",
                    "description": "Read-only operator-domain inspection over existing OpTask repository state. Returns matching tasks, runs with embedded work_items JSON, and artifacts. Work items are not independently queryable rows yet; they are embedded in op_task_runs for the MVP.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        {
                            "name": "profile_id",
                            "in": "query",
                            "schema": { "type": "string", "format": "uuid", "default": DEFAULT_EMPLOYMENT_PROFILE_ID }
                        },
                        {
                            "name": "task_id",
                            "in": "query",
                            "schema": { "type": "string", "format": "uuid" }
                        },
                        {
                            "name": "run_id",
                            "in": "query",
                            "schema": { "type": "string", "format": "uuid" }
                        },
                        {
                            "name": "artifact_id",
                            "in": "query",
                            "schema": { "type": "string", "format": "uuid" }
                        },
                        {
                            "name": "artifact_type",
                            "in": "query",
                            "schema": { "type": "string" },
                            "examples": {
                                "diagnostic": { "value": "operator_task_diagnostic" },
                                "patchPlan": { "value": "operator_patch_plan" },
                                "taskSet": { "value": "operator_implementation_task_set" }
                            }
                        },
                        {
                            "name": "source_url",
                            "in": "query",
                            "schema": { "type": "string" }
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
                        },
                        {
                            "name": "offset",
                            "in": "query",
                            "schema": { "type": "integer", "minimum": 0, "default": 0 }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Task state snapshot",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/OperatorTaskStateSnapshot" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator-meta/review-failed-task": {
                "post": {
                    "operationId": "reviewFailedTask",
                    "summary": "Create an operator failed-task diagnostic task",
                    "description": "Debug/admin endpoint. Safety Level 1: diagnose only. Creates an operator.review_failed_task TaskRequest and OpTask for a specific failed run. For normal chat use, prefer createTaskFromNaturalLanguage with a message like 'Review failed task run ... and suggest fixes', then runTaskRequest.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/OperatorReviewFailedTaskRequest" },
                                "examples": {
                                    "reviewRun": {
                                        "summary": "Review a failed run",
                                        "value": {
                                            "run_id": "00000000-0000-0000-0000-000000000000",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "include_task": true,
                                            "include_artifacts": true,
                                            "include_recent_audit": true,
                                            "include_repo_context": false,
                                            "escalate_if_needed": false,
                                            "source": "open_webui"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Created diagnostic task request",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CreateTaskFromMessageResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator-meta/review-recent-tasks": {
                "post": {
                    "operationId": "reviewRecentTasks",
                    "summary": "Inspect recent operator task state",
                    "description": "Debug/admin endpoint. Safety Level 1: diagnose only. Returns recent task, run, and artifact state for operator review without executing code or changing configuration. Use this to inspect recent failures before choosing reviewFailedTask.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": false,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/OperatorReviewRecentTasksRequest" },
                                "examples": {
                                    "recent": {
                                        "summary": "Inspect recent task state",
                                        "value": {
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "include_content": false,
                                            "limit": 25
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Recent operator task state",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/OperatorTaskStateSnapshot" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator-meta/generate-patch-plan": {
                "post": {
                    "operationId": "generatePatchPlan",
                    "summary": "Create a patch-plan task from an operator diagnostic",
                    "description": "Debug/admin endpoint. Safety Level 2: plan only. Creates an operator.generate_patch_plan TaskRequest and OpTask from an operator_task_diagnostic artifact. The task produces an operator_patch_plan artifact when run and does not edit files.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/OperatorGeneratePatchPlanRequest" },
                                "examples": {
                                    "fromDiagnostic": {
                                        "summary": "Generate a patch plan from a diagnostic artifact",
                                        "value": {
                                            "artifact_id": "00000000-0000-0000-0000-000000000000",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "title": "Fix failed task workflow",
                                            "source": "open_webui"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Created patch-plan task request",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CreateTaskFromMessageResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator-meta/escalations": {
                "post": {
                    "operationId": "createEscalationRequest",
                    "summary": "Create a ChatGPT escalation request artifact",
                    "description": "Debug/admin endpoint. Safety Level 2: plan/escalation packet only. Creates a chatgpt_escalation_request artifact attached to an existing run. For normal use, prefer createTaskFromNaturalLanguage with an escalation request so Local Operator collects and redacts context first.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateChatGptEscalationRequestArtifact" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Created escalation request artifact",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ChatGptEscalationArtifactResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator-meta/escalations/{artifact_id}/response": {
                "post": {
                    "operationId": "submitEscalationResponse",
                    "summary": "Save a ChatGPT escalation response artifact",
                    "description": "Debug/admin endpoint. Safety Level 2: response capture only. Saves a chatgpt_escalation_response artifact and links it back to the request artifact identified by artifact_id. Never executes recommended actions automatically.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ArtifactId" }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SaveChatGptEscalationResponseArtifact" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Saved escalation response artifact",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ChatGptEscalationArtifactResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator-meta/artifacts/{artifact_id}/convert-to-tasks": {
                "post": {
                    "operationId": "convertRecommendationToTasks",
                    "summary": "Create an implementation task-set task from an operator artifact",
                    "description": "Debug/admin endpoint. Safety Level 2: plan only. Creates an operator.convert_recommendation_to_tasks TaskRequest and OpTask from an operator_patch_plan artifact. The task produces an operator_implementation_task_set artifact when run; it does not execute implementation tasks automatically.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ArtifactId" }
                    ],
                    "requestBody": {
                        "required": false,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/OperatorConvertRecommendationToTasksRequest" },
                                "examples": {
                                    "convertPatchPlan": {
                                        "summary": "Convert a patch plan into task specs",
                                        "value": {
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
                            "description": "Created implementation task-set request",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CreateTaskFromMessageResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/operator-meta/diagnostics": {
                "get": {
                    "operationId": "showOperatorDiagnostics",
                    "summary": "Show recent operator diagnostics",
                    "description": "Debug/admin endpoint. Safety Level 1: diagnose only. Lists recent operator_task_diagnostic artifacts. Use this when the user asks to show operator diagnostics, failed-task reviews, or self-review output.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        {
                            "name": "profile_id",
                            "in": "query",
                            "schema": { "type": "string", "format": "uuid", "default": DEFAULT_EMPLOYMENT_PROFILE_ID }
                        },
                        {
                            "name": "run_id",
                            "in": "query",
                            "schema": { "type": "string", "format": "uuid" }
                        },
                        {
                            "name": "include_content",
                            "in": "query",
                            "schema": { "type": "boolean", "default": false }
                        },
                        {
                            "name": "limit",
                            "in": "query",
                            "schema": { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Operator diagnostics",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ShowOperatorDiagnosticsResponse" }
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
                            "description": "Use this when the user asks for a specific kind of output, such as search results, scored matches, or operator self-review artifacts.",
                            "schema": { "type": "string", "example": "search_result_set" },
                            "examples": {
                                "searchResults": { "value": "search_result_set" },
                                "readablePage": { "value": "readable_web_page" },
                                "scoredMatches": { "value": "scored_opportunity_matches" },
                                "operatorDiagnostic": { "value": "operator_task_diagnostic" },
                                "operatorPatchPlan": { "value": "operator_patch_plan" },
                                "operatorImplementationTaskSet": { "value": "operator_implementation_task_set" }
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
                                    },
                                    "approveEscalationFollowUps": {
                                        "summary": "Create follow-up tasks from a ChatGPT escalation response",
                                        "value": {
                                            "message": "Create follow-up tasks from the recommended next steps.",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "confirm": true,
                                            "create_tasks": true
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
            "/api/artifacts/chatgpt-escalation-requests": {
                "post": {
                    "operationId": "createChatGptEscalationRequestArtifact",
                    "summary": "Create a ChatGPT escalation request artifact",
                    "description": "Creates a chatgpt_escalation_request artifact using existing TaskArtifact storage. Use this when Local Operator needs to capture a structured request for ChatGPT escalation. The artifact must belong to an existing OpTaskRun and must include structured JSON content.",
                    "security": [{ "BearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateChatGptEscalationRequestArtifact" },
                                "examples": {
                                    "patchReviewRequest": {
                                        "summary": "Escalate a patch review question",
                                        "value": {
                                            "run_id": "00000000-0000-0000-0000-000000000000",
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "name": "ChatGPT escalation request",
                                            "metadata": {
                                                "purpose": "review",
                                                "priority": "normal"
                                            },
                                            "content_text": "Please review this proposed task workflow.",
                                            "content_json": {
                                                "question": "Review the plan and identify risks.",
                                                "context_artifact_ids": [],
                                                "desired_output": "Concise review with risks and suggested changes."
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Created ChatGPT escalation request artifact",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ChatGptEscalationArtifactResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/artifacts/{artifact_id}/chatgpt-escalation-response": {
                "post": {
                    "operationId": "saveChatGptEscalationResponseArtifact",
                    "summary": "Save a ChatGPT escalation response artifact",
                    "description": "Creates a chatgpt_escalation_response artifact using existing TaskArtifact storage and links it back to the chatgpt_escalation_request artifact identified by artifact_id. The response must include structured JSON content.",
                    "security": [{ "BearerAuth": [] }],
                    "parameters": [
                        { "$ref": "#/components/parameters/ArtifactId" }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SaveChatGptEscalationResponseArtifact" },
                                "examples": {
                                    "reviewResponse": {
                                        "summary": "Save ChatGPT review output",
                                        "value": {
                                            "profile_id": DEFAULT_EMPLOYMENT_PROFILE_ID,
                                            "name": "ChatGPT escalation response",
                                            "metadata": {
                                                "model": "chatgpt",
                                                "purpose": "review"
                                            },
                                            "response_text": "The plan is sound; main risks are missing retry policy and unclear ownership.",
                                            "content_json": {
                                                "summary": "The plan is sound with two risks.",
                                                "findings": [
                                                    "Add retry policy.",
                                                    "Clarify ownership."
                                                ],
                                                "recommended_next_steps": []
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Saved ChatGPT escalation response artifact",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ChatGptEscalationArtifactResponse" }
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
                        },
                        "confirm": {
                            "type": "boolean",
                            "default": false,
                            "description": "Set true to confirm personal or employment ChatGPT escalation. Technical-only escalation does not require confirmation; secrets are always blocked. OpenAI API mode still sends only redacted request content."
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
                "OperatorReviewFailedTaskRequest": {
                    "type": "object",
                    "required": ["run_id"],
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Failed OpTaskRun ID to diagnose."
                        },
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        "include_task": { "type": "boolean", "default": true },
                        "include_artifacts": { "type": "boolean", "default": true },
                        "include_recent_audit": { "type": "boolean", "default": true },
                        "include_repo_context": {
                            "type": "boolean",
                            "default": false,
                            "description": "Reserved for later code/repo inspection. Current MVP remains read-only."
                        },
                        "escalate_if_needed": { "type": "boolean", "default": false },
                        "source": { "type": "string", "default": "operator_meta" }
                    }
                },
                "OperatorReviewRecentTasksRequest": {
                    "type": "object",
                    "properties": {
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        "include_content": { "type": "boolean", "default": false },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 200, "default": 25 },
                        "offset": { "type": "integer", "minimum": 0, "default": 0 }
                    }
                },
                "OperatorGeneratePatchPlanRequest": {
                    "type": "object",
                    "required": ["artifact_id"],
                    "properties": {
                        "artifact_id": {
                            "type": "string",
                            "format": "uuid",
                            "description": "operator_task_diagnostic artifact ID."
                        },
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        "title": { "type": "string" },
                        "source": { "type": "string", "default": "operator_meta" }
                    }
                },
                "OperatorConvertRecommendationToTasksRequest": {
                    "type": "object",
                    "properties": {
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        "source": { "type": "string", "default": "operator_meta" }
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
                "DomainCatalogResponse": {
                    "type": "object",
                    "properties": {
                        "domains": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DomainDescriptor" }
                        }
                    }
                },
                "DomainDescriptor": {
                    "type": "object",
                    "properties": {
                        "domain": { "type": "string" },
                        "display_name": { "type": "string" },
                        "priority": { "type": "integer" },
                        "description": { "type": "string" },
                        "task_types": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DomainTaskType" }
                        },
                        "planner": { "$ref": "#/components/schemas/DomainPlanner" },
                        "work_item_types": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "tools": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DomainTool" }
                        },
                        "artifact_types": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DomainArtifactType" }
                        },
                        "model_purposes": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DomainModelPurpose" }
                        },
                        "policy_tiers": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DomainPolicyTier" }
                        },
                        "safety_levels": {
                            "type": "array",
                            "description": "Domain-specific safety boundaries. The operator domain uses five explicit levels from diagnose-only through blocked operational changes.",
                            "items": { "$ref": "#/components/schemas/DomainSafetyLevel" }
                        },
                        "continuation_rules": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DomainContinuationRule" }
                        }
                    }
                },
                "DomainTaskType": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "status": {
                            "type": "string",
                            "enum": ["active", "planned", "alias", "continuation_rule"]
                        },
                        "description": { "type": "string" },
                        "input_schema": {
                            "type": "object",
                            "additionalProperties": true
                        }
                    }
                },
                "DomainPlanner": {
                    "type": "object",
                    "properties": {
                        "strategy": { "type": "string" },
                        "planner_module": { "type": "string" },
                        "notes": { "type": "string" }
                    }
                },
                "DomainTool": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "purpose": { "type": "string" },
                        "required_now": { "type": "boolean" }
                    }
                },
                "DomainArtifactType": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "description": { "type": "string" }
                    }
                },
                "DomainModelPurpose": {
                    "type": "object",
                    "properties": {
                        "purpose": { "type": "string" },
                        "model_route": { "type": "string" }
                    }
                },
                "DomainPolicyTier": {
                    "type": "object",
                    "properties": {
                        "operation": { "type": "string" },
                        "risk_tier": {
                            "type": "string",
                            "enum": ["Tier0", "Tier1", "Tier2", "Tier3"]
                        },
                        "requires_confirmation": { "type": "boolean" }
                    }
                },
                "DomainSafetyLevel": {
                    "type": "object",
                    "properties": {
                        "level": { "type": "integer", "minimum": 1, "maximum": 5 },
                        "name": { "type": "string" },
                        "description": { "type": "string" },
                        "allowed": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "requires_confirmation": { "type": "boolean" },
                        "status": {
                            "type": "string",
                            "enum": ["active", "blocked_for_now", "planned"]
                        }
                    }
                },
                "DomainContinuationRule": {
                    "type": "object",
                    "properties": {
                        "source_artifact_type": { "type": "string" },
                        "continuations": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
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
                        "content_json": { "type": "object", "additionalProperties": true, "nullable": true },
                        "allowed_continuations": {
                            "type": "array",
                            "description": "Allowed continuation actions for this artifact type.",
                            "items": { "type": "string" }
                        }
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
                        },
                        "confirm": {
                            "type": "boolean",
                            "default": false,
                            "description": "Set true to approve creating follow-up OpTasks from a chatgpt_escalation_response artifact. Recommended actions are extracted without creating tasks when false."
                        },
                        "create_tasks": {
                            "type": "boolean",
                            "default": false,
                            "description": "Set true when the user approves creating draft OpTasks from escalation recommendations. Created tasks are linked to the response artifact, saved paused/draft, and are not executed automatically."
                        }
                    }
                },
                "RecommendedEscalationAction": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "detail": { "type": "string", "nullable": true },
                        "suggested_task_type": {
                            "type": "string",
                            "description": "Best-effort Local Operator task type inferred from the recommendation."
                        },
                        "input_json": {
                            "type": "object",
                            "additionalProperties": true
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
                        "source_artifact_type": { "type": "string" },
                        "allowed_continuations": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "task_request": { "$ref": "#/components/schemas/TaskRequest" },
                        "task": { "$ref": "#/components/schemas/OpTask" },
                        "run": { "$ref": "#/components/schemas/OpTaskRunSummary" },
                        "artifacts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/TaskRunArtifactSummary" }
                        },
                        "recommended_actions": {
                            "type": "array",
                            "description": "Present when continuing from a chatgpt_escalation_response artifact. These are extracted candidate follow-up tasks.",
                            "items": { "$ref": "#/components/schemas/RecommendedEscalationAction" }
                        },
                        "created_tasks": {
                            "type": "array",
                            "description": "Draft follow-up OpTasks created after explicit approval. These are linked back to the escalation response artifact, saved paused/draft, and are not executed automatically.",
                            "items": { "$ref": "#/components/schemas/OpTask" }
                        },
                        "requires_confirmation": {
                            "type": "boolean",
                            "description": "True when recommended actions were found but follow-up task creation has not been approved."
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
                "CreateChatGptEscalationRequestArtifact": {
                    "type": "object",
                    "required": ["run_id", "content_json"],
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Existing OpTaskRun that owns the request artifact."
                        },
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Optional profile guard. If supplied, it must match the run profile.",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        "confirm": {
                            "type": "boolean",
                            "default": false,
                            "description": "Required when the escalation request contains personal or employment context. Technical-only requests do not require confirmation; secrets are always blocked."
                        },
                        "work_item_id": {
                            "type": "string",
                            "format": "uuid",
                            "nullable": true
                        },
                        "name": {
                            "type": "string",
                            "default": "ChatGPT escalation request"
                        },
                        "metadata": {
                            "type": "object",
                            "additionalProperties": true,
                            "nullable": true
                        },
                        "content_text": {
                            "type": "string",
                            "nullable": true
                        },
                        "content_json": {
                            "type": "object",
                            "additionalProperties": true,
                            "description": "Structured JSON payload for ChatGPT escalation. Must be an object or array."
                        }
                    }
                },
                "SaveChatGptEscalationResponseArtifact": {
                    "type": "object",
                    "description": "Paste back a manual ChatGPT response. Provide content_json for structured output, or response_text for plain pasted text; plain text is wrapped into structured JSON automatically.",
                    "properties": {
                        "profile_id": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Optional profile guard. If supplied, it must match the request artifact profile.",
                            "default": DEFAULT_EMPLOYMENT_PROFILE_ID
                        },
                        "work_item_id": {
                            "type": "string",
                            "format": "uuid",
                            "nullable": true
                        },
                        "name": {
                            "type": "string",
                            "default": "ChatGPT escalation response"
                        },
                        "metadata": {
                            "type": "object",
                            "additionalProperties": true,
                            "nullable": true
                        },
                        "content_text": {
                            "type": "string",
                            "nullable": true
                        },
                        "response_text": {
                            "type": "string",
                            "nullable": true,
                            "description": "Plain pasted ChatGPT response. Use this for manual mode if no structured JSON was produced."
                        },
                        "content_json": {
                            "type": "object",
                            "additionalProperties": true,
                            "nullable": true,
                            "description": "Structured JSON response from ChatGPT. Must be an object or array when supplied."
                        }
                    }
                },
                "ChatGptEscalationArtifactResponse": {
                    "type": "object",
                    "properties": {
                        "artifact": { "$ref": "#/components/schemas/TaskArtifact" },
                        "linked_request_artifact_id": {
                            "type": "string",
                            "format": "uuid",
                            "nullable": true,
                            "description": "Present for response artifacts and points back to the chatgpt_escalation_request artifact."
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
                                "system.status_report",
                                "system.escalate_to_chatgpt",
                                "operator.escalate_to_chatgpt",
                                "operator.review_failed_task",
                                "operator.generate_patch_plan",
                                "operator.convert_recommendation_to_tasks",
                                "artifact.summarize"
                            ]
                        },
                        "description": { "type": "string", "nullable": true },
                        "input_json": {
                            "type": "object",
                            "additionalProperties": true,
                            "description": "For employment.search_opportunities: { limit?: number, create_opportunities?: boolean }. For operator.review_failed_task: { run_id: uuid, include_task?: boolean, include_artifacts?: boolean, include_recent_audit?: boolean, include_repo_context?: boolean, escalate_if_needed?: boolean }. For operator.generate_patch_plan: { artifact_id: operator_task_diagnostic uuid, title?: string }. For operator.convert_recommendation_to_tasks: { artifact_id: operator_patch_plan uuid }. For operator.escalate_to_chatgpt/system.escalate_to_chatgpt: { mode: 'manual' | 'openai', confirm?: boolean, user_request: string, desired_output?: string, context_text?: string, context_json?: object }."
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
                "OperatorTaskStateSnapshot": {
                    "type": "object",
                    "properties": {
                        "filters": {
                            "type": "object",
                            "additionalProperties": true
                        },
                        "tasks": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/OpTask" }
                        },
                        "runs": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/OpTaskRun" }
                        },
                        "artifacts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/TaskArtifact" }
                        },
                        "note": {
                            "type": "string",
                            "description": "Explains the current MVP storage detail that work items are embedded in run JSON rather than independent rows."
                        }
                    }
                },
                "ShowOperatorDiagnosticsResponse": {
                    "type": "object",
                    "properties": {
                        "artifacts": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/TaskArtifact" }
                        },
                        "next_actions": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
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
