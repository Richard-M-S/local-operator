use crate::context::{models::ContextKind, ContextService};
use crate::domains::employment::{models::EmploymentOpportunity, repository::EmploymentRepository};
use crate::op_tasks::models::{
    OpTask, OpTaskRun, OpTaskRunStatus, ReadUrlInput, SearchWebInput, TaskArtifact,
};
use crate::readers::{models::SearchResultItem, ReaderService};
use crate::services::execution::{ExecutionContext, ModelExecutionService, ToolExecutionService};
use crate::services::llm_router::LlmRouter;
use anyhow::{anyhow, Context};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing;
use uuid::Uuid;

#[derive(Clone)]
pub struct OpTaskRunner {
    tool_execution: ToolExecutionService,
    model_execution: ModelExecutionService,
    readers: ReaderService,
    llm_router: LlmRouter,
    employment_repo: EmploymentRepository,
    context: ContextService,
}

impl OpTaskRunner {
    pub fn new(
        tool_execution: ToolExecutionService,
        model_execution: ModelExecutionService,
        readers: ReaderService,
        llm_router: LlmRouter,
        employment_repo: EmploymentRepository,
        context: ContextService,
    ) -> Self {
        Self {
            tool_execution,
            model_execution,
            readers,
            llm_router,
            employment_repo,
            context,
        }
    }

    pub async fn execute(&self, task: OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        run.status = OpTaskRunStatus::Running;
        run.started_at = Some(Utc::now());

        match task.task_type.as_str() {
            "system.status_report" => self.run_status_report(&task, run).await,
            "reader.read_url" => self.run_read_url(&task, run).await,
            "reader.search_web" => self.run_search_web(&task, run).await,
            "employment.search_opportunities" => {
                self.run_employment_search_opportunities(&task, run).await
            }
            _ => {
                let message = format!("unsupported task type: {}", task.task_type);
                if let Ok(step_id) = start_step(&mut run, "unsupported_task") {
                    finish_step_with_error(&mut run, step_id, &message);
                }
                run.status = OpTaskRunStatus::Failed;
                run.completed_at = Some(Utc::now());
                run.summary = Some(message);
                Ok(run)
            }
        }
    }

    async fn run_status_report(
        &self,
        _task: &OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let collect_step_id = start_step(&mut run, "collect_system_status")?;
        let result = match self
            .tool_execution
            .execute(
                "system.get_status",
                json!({}),
                false,
                ExecutionContext::for_work_item(_task.id, run.id, collect_step_id)
                    .with_input_summary("Collect system status"),
            )
            .await
        {
            Ok(result) => result,
            Err(err) => {
                finish_step_with_error(&mut run, collect_step_id, &err.to_string());
                return Err(anyhow!(err.to_string()))
                    .context("failed to execute system.get_status tool");
            }
        };
        finish_step(
            &mut run,
            collect_step_id,
            Some(format!("Collected status with {}.", result.tool)),
            vec![],
        );

        let summary_step_id = start_step(&mut run, "summarize_system_status")?;
        let summary_model = self.llm_router.task_summary_model();
        let mut summary = format!("System status collected by {}", result.tool);
        let mut summary_details = format!("Prepared summary with model purpose task_summary.");

        let prompt = format!(
            "Summarize the following system status output in a short, actionable paragraph:\n\n{}",
            serde_json::to_string_pretty(&result.output)
                .unwrap_or_else(|_| "<unserializable output>".to_string())
        );
        let artifact_id = Uuid::new_v4();

        match self
            .model_execution
            .ask_model(
                &summary_model,
                "You are a system status summarization assistant.",
                &prompt,
                ExecutionContext::for_work_item(_task.id, run.id, summary_step_id)
                    .with_model_purpose("task_summary")
                    .with_input_summary("Summarize system status")
                    .with_output_artifact(artifact_id),
            )
            .await
        {
            Ok(summary_text) => summary = summary_text,
            Err(err) => {
                summary = format!(
                    "System status collected, but LLM summarization failed: {}",
                    err
                );
                summary_details = format!("LLM summarization failed; fallback summary used: {err}");
            }
        }

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(summary_step_id),
            name: "system_status_report".to_string(),
            artifact_type: "status_report".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "tool": result.tool,
                "model": summary_model,
                "model_purpose": "task_summary",
                "output": result.output,
            })),
            content_text: Some(summary.clone()),
            content_json: None,
        });
        finish_step(
            &mut run,
            summary_step_id,
            Some(summary_details),
            vec![artifact_id],
        );

        run.summary = Some(summary);
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        Ok(run)
    }

    async fn run_read_url(&self, task: &OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let step_id = start_step(&mut run, "read_url")?;
        let input: ReadUrlInput = match serde_json::from_value(task.input_json.clone()) {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(anyhow!(err)).context("invalid reader.read_url input_json");
            }
        };

        let result = match self.readers.read_url(input.url.clone()).await {
            Ok(result) => result,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(err).context("failed to read URL");
            }
        };

        let title = result
            .title
            .clone()
            .unwrap_or_else(|| "read_url_result".to_string());
        let cleaned_text = result.cleaned_text.clone();
        let artifact_id = Uuid::new_v4();

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(step_id),
            name: title,
            artifact_type: "readable_web_page".to_string(),
            location: Some(input.url.clone()),
            created_at: Utc::now(),
            metadata: Some(json!({
                "source_url": input.url.clone(),
                "title": result.title.clone(),
                "detected_type": result.detected_type.clone(),
                "text_length": result.cleaned_text.len()
            })),
            content_text: Some(cleaned_text.clone()),
            content_json: Some(json!({
                "raw_text": result.raw_text,
                "cleaned_text": cleaned_text,
                "title": result.title,
                "source_url": input.url
            })),
        });

        finish_step(
            &mut run,
            step_id,
            Some("Read URL and extracted readable text.".to_string()),
            vec![artifact_id],
        );
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some("Read URL and extracted readable text.".to_string());

        Ok(run)
    }

    async fn run_search_web(&self, task: &OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let step_id = start_step(&mut run, "search_web")?;
        let input: SearchWebInput = match serde_json::from_value(task.input_json.clone()) {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(anyhow!(err)).context("invalid reader.search_web input_json");
            }
        };
        let limit = input.limit.unwrap_or(10).clamp(1, 25);
        update_step_tool_args(
            &mut run,
            step_id,
            json!({
                "query": input.query,
                "limit": limit,
            }),
        );

        let results = match self.readers.search_web(input.query.clone(), limit).await {
            Ok(results) => results,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(err).context("failed to search web");
            }
        };

        let content_text = render_search_results_text(&results.query, &results.results);
        let result_count = results.results.len();
        let query = results.query.clone();
        let artifact_id = Uuid::new_v4();

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(step_id),
            name: format!("Search results: {}", query),
            artifact_type: "search_result_set".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "query": query,
                "profile_id": run.profile_id.to_string(),
                "result_count": result_count,
                "model_purpose": task.input_json.get("model_purpose").cloned(),
                "priority": task.input_json.get("priority").cloned(),
            })),
            content_text: Some(content_text),
            content_json: Some(json!({
                "query": results.query,
                "results": results.results,
            })),
        });

        finish_step(
            &mut run,
            step_id,
            Some(format!("Search completed with {} results.", result_count)),
            vec![artifact_id],
        );
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(format!(
            "Search completed for '{}' with {} results.",
            input.query, result_count
        ));

        Ok(run)
    }

    async fn run_employment_search_opportunities(
        &self,
        task: &OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let profile_step_id = start_step(&mut run, "load_profile_context")?;
        let input: EmploymentSearchInput = match serde_json::from_value(task.input_json.clone()) {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, profile_step_id, &err.to_string());
                return Err(anyhow!(err))
                    .context("invalid employment.search_opportunities input_json");
            }
        };
        let limit = input.limit.unwrap_or(10).clamp(1, 25);
        let read_limit = limit.min(8);
        let min_score = input.min_score.unwrap_or(0).clamp(0, 100);

        let profile = match self.employment_repo.get_profile(run.profile_id).await {
            Ok(Some(profile)) => profile,
            Ok(None) => {
                let err = "employment profile not found";
                finish_step_with_error(&mut run, profile_step_id, err);
                return Err(anyhow!(err));
            }
            Err(err) => {
                finish_step_with_error(&mut run, profile_step_id, &err.to_string());
                return Err(err).context("failed to load employment profile");
            }
        };
        let context_summary = match self.load_employment_context_summary(run.profile_id).await {
            Ok(context_summary) => context_summary,
            Err(err) => {
                finish_step_with_error(&mut run, profile_step_id, &err.to_string());
                return Err(err).context("failed to load employment context");
            }
        };

        let criteria = profile
            .criteria
            .clone()
            .filter(|c| !c.trim().is_empty())
            .unwrap_or_else(|| "jobs".to_string());
        finish_step(
            &mut run,
            profile_step_id,
            Some(format!(
                "Loaded profile {} and {} context notes.",
                profile.display_name,
                context_summary.len()
            )),
            vec![],
        );

        let build_query_step_id = start_step(&mut run, "build_search_query")?;
        let search_query = build_employment_search_query(&input, &criteria);
        update_step_tool_args(
            &mut run,
            build_query_step_id,
            json!({
                "query": search_query,
                "limit": limit,
                "query_override": input.query_override,
                "user_request": input.user_request,
                "location": input.location,
                "remote_preference": input.remote_preference,
                "min_score": min_score,
            }),
        );
        finish_step(
            &mut run,
            build_query_step_id,
            Some(format!("Built search query '{}'.", search_query)),
            vec![],
        );

        let search_step_id = start_step(&mut run, "run_search")?;
        update_step_tool_args(
            &mut run,
            search_step_id,
            json!({
                "query": search_query,
                "limit": limit,
            }),
        );
        let results = match self.readers.search_web(search_query.clone(), limit).await {
            Ok(results) => results,
            Err(err) => {
                finish_step_with_error(&mut run, search_step_id, &err.to_string());
                return Err(err).context("failed to search web for opportunities");
            }
        };
        let search_results = results.results.clone();

        let content_text = if results.results.is_empty() {
            "No job opportunities found for profile criteria.".to_string()
        } else {
            render_search_results_text(&results.query, &results.results)
        };

        let result_count = results.results.len();
        let artifact_id = Uuid::new_v4();

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(search_step_id),
            name: "Employment opportunities search results".to_string(),
            artifact_type: "search_result_set".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "criteria": criteria,
                "context": context_summary,
                "query": search_query,
                "profile_id": run.profile_id.to_string(),
                "result_count": result_count,
                "search_type": "employment_opportunities",
                "create_opportunities": input.create_opportunities,
                "min_score": min_score,
                "profile_display_name": profile.display_name,
            })),
            content_text: Some(content_text),
            content_json: Some(json!({
                "criteria": profile.criteria,
                "query": results.query,
                "results": results.results,
                "profile_id": run.profile_id.to_string(),
            })),
        });
        finish_step(
            &mut run,
            search_step_id,
            Some(format!("Search completed with {} results.", result_count)),
            vec![artifact_id],
        );

        let read_step_id = start_step(&mut run, "read_result_urls")?;
        push_step_input_artifact(&mut run, read_step_id, artifact_id);
        let mut readable_pages = Vec::new();
        let mut read_failures = Vec::new();

        for result in search_results.iter().take(read_limit) {
            match self.readers.read_url(result.url.clone()).await {
                Ok(page) => {
                    let page_artifact_id = Uuid::new_v4();
                    let page_title = page.title.clone().unwrap_or_else(|| result.title.clone());
                    run.artifacts.push(TaskArtifact {
                        id: page_artifact_id,
                        profile_id: run.profile_id,
                        run_id: run.id,
                        work_item_id: Some(read_step_id),
                        name: page_title,
                        artifact_type: "readable_web_page".to_string(),
                        location: Some(page.source_url.clone()),
                        created_at: Utc::now(),
                        metadata: Some(json!({
                            "source_url": page.source_url,
                            "title": page.title,
                            "detected_type": page.detected_type,
                            "text_length": page.cleaned_text.len(),
                            "search_result_title": result.title,
                        })),
                        content_text: Some(page.cleaned_text.clone()),
                        content_json: Some(json!({
                            "raw_text": page.raw_text,
                            "cleaned_text": page.cleaned_text,
                            "title": page.title,
                            "source_url": page.source_url,
                            "search_result": result,
                        })),
                    });
                    readable_pages.push(ReadablePageForExtraction {
                        artifact_id: page_artifact_id,
                        source_url: result.url.clone(),
                        title: result.title.clone(),
                        snippet: result.snippet.clone(),
                        text: page.cleaned_text,
                    });
                }
                Err(err) => {
                    read_failures.push(json!({
                        "url": result.url,
                        "title": result.title,
                        "error": err.to_string(),
                    }));
                }
            }
        }
        let readable_artifact_ids = readable_pages
            .iter()
            .map(|page| page.artifact_id)
            .collect::<Vec<_>>();
        finish_step(
            &mut run,
            read_step_id,
            Some(format!(
                "Read {} result URLs; {} failed.",
                readable_pages.len(),
                read_failures.len()
            )),
            readable_artifact_ids.clone(),
        );

        let extract_step_id = start_step(&mut run, "extract_candidates")?;
        push_step_input_artifact(&mut run, extract_step_id, artifact_id);
        for readable_artifact_id in &readable_artifact_ids {
            push_step_input_artifact(&mut run, extract_step_id, *readable_artifact_id);
        }
        let candidates_artifact_id = Uuid::new_v4();
        let mut candidates = Vec::new();

        for page in &readable_pages {
            let parsed = self
                .model_execution
                .parse_job_opportunity(
                    &self.llm_router.task_extraction_model(),
                    &page.text,
                    ExecutionContext::for_work_item(task.id, run.id, extract_step_id)
                        .with_model_purpose("employment_candidate_extract")
                        .with_input_summary(format!("Extract candidate from {}", page.source_url))
                        .with_output_artifact(candidates_artifact_id),
                )
                .await
                .unwrap_or_else(|_| {
                    json!({
                        "title": page.title,
                        "company": null,
                        "location": null,
                        "remote_type": null,
                        "salary_min": null,
                        "salary_max": null,
                        "description_text": page.text.chars().take(4000).collect::<String>(),
                    })
                });
            candidates.push(candidate_from_parsed(
                &parsed,
                &page.source_url,
                Some(page.title.clone()),
                page.snippet.clone(),
                Some(page.artifact_id),
            ));
        }

        for result in &search_results {
            if !candidates
                .iter()
                .any(|candidate: &OpportunityCandidate| candidate.source_url == result.url)
            {
                candidates.push(candidate_from_search_result(result));
            }
        }

        run.artifacts.push(TaskArtifact {
            id: candidates_artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(extract_step_id),
            name: "Extracted opportunity candidates".to_string(),
            artifact_type: "extracted_opportunity_candidates".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "candidate_count": candidates.len(),
                "readable_page_count": readable_pages.len(),
                "read_failures": read_failures,
            })),
            content_text: Some(render_candidates_text(&candidates)),
            content_json: Some(json!({
                "candidates": candidates,
            })),
        });
        finish_step(
            &mut run,
            extract_step_id,
            Some(format!(
                "Extracted {} opportunity candidates.",
                candidates.len()
            )),
            vec![candidates_artifact_id],
        );

        let score_step_id = start_step(&mut run, "score_matches")?;
        push_step_input_artifact(&mut run, score_step_id, candidates_artifact_id);
        let scored_artifact_id = Uuid::new_v4();
        let scoring_criteria = run
            .artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .and_then(|artifact| artifact.metadata.as_ref())
            .and_then(|metadata| metadata.get("criteria"))
            .and_then(|value| value.as_str())
            .unwrap_or("Score for fit against the employment profile.");
        let mut scored_matches = Vec::new();

        for candidate in &candidates {
            let candidate_json = serde_json::to_value(candidate)
                .unwrap_or_else(|_| json!({ "source_url": candidate.source_url }));
            let scored = self
                .model_execution
                .score_job_opportunity(
                    &self.llm_router.task_reasoning_model(),
                    &candidate_json,
                    scoring_criteria,
                    ExecutionContext::for_work_item(task.id, run.id, score_step_id)
                        .with_model_purpose("employment_match_score")
                        .with_input_summary(format!("Score candidate {}", candidate.source_url))
                        .with_output_artifact(scored_artifact_id),
                )
                .await
                .unwrap_or_else(|_| heuristic_candidate_score(candidate));
            scored_matches.push(scored_match_from_output(
                candidate.clone(),
                scored,
                min_score,
            ));
        }

        run.artifacts.push(TaskArtifact {
            id: scored_artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(score_step_id),
            name: "Scored opportunity matches".to_string(),
            artifact_type: "scored_opportunity_matches".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "match_count": scored_matches.len(),
                "min_score": min_score,
                "passing_count": scored_matches.iter().filter(|item| item.passes_min_score).count(),
            })),
            content_text: Some(render_scored_matches_text(&scored_matches)),
            content_json: Some(json!({
                "matches": scored_matches,
                "min_score": min_score,
            })),
        });
        finish_step(
            &mut run,
            score_step_id,
            Some(format!(
                "Scored {} opportunity matches.",
                scored_matches.len()
            )),
            vec![scored_artifact_id],
        );

        let create_step_id = start_step(&mut run, "create_opportunities")?;
        push_step_input_artifact(&mut run, create_step_id, scored_artifact_id);
        let mut created_opportunity_ids = Vec::new();
        let mut existing_opportunity_ids = Vec::new();
        let mut skipped_opportunity_sources = Vec::new();

        if input.create_opportunities {
            for scored_match in scored_matches.iter().filter(|item| item.passes_min_score) {
                match self
                    .employment_repo
                    .find_opportunity_by_source_url(
                        run.profile_id,
                        &scored_match.candidate.source_url,
                    )
                    .await
                {
                    Ok(Some(existing)) => {
                        existing_opportunity_ids.push(existing.id.to_string());
                        if let Err(e) = self
                            .employment_repo
                            .touch_opportunity_seen_at(existing.id)
                            .await
                        {
                            tracing::warn!(
                                "Failed to touch existing opportunity {}: {}",
                                scored_match.candidate.source_url,
                                e
                            );
                        }
                        continue;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!(
                            "Failed to check existing opportunity {}: {}",
                            scored_match.candidate.source_url,
                            e
                        );
                    }
                }

                let opportunity = EmploymentOpportunity::new_discovered(
                    run.profile_id,
                    scored_match.candidate.source_url.clone(),
                    scored_match.candidate.source_name.clone(),
                    scored_match.candidate.source_artifact_id,
                );

                let opportunity = EmploymentOpportunity {
                    title: scored_match.candidate.title.clone(),
                    company: scored_match.candidate.company.clone(),
                    location: scored_match.candidate.location.clone(),
                    remote_type: scored_match.candidate.remote_type.clone(),
                    salary_min: scored_match.candidate.salary_min,
                    salary_max: scored_match.candidate.salary_max,
                    description_text: scored_match.candidate.description_text.clone(),
                    extracted_json: scored_match.candidate.extracted_json.clone(),
                    fit_score: scored_match.primary_fit_score.or(scored_match.oe_fit_score),
                    primary_fit_score: scored_match.primary_fit_score,
                    oe_fit_score: scored_match.oe_fit_score,
                    recommended_track: scored_match.recommended_track.clone(),
                    score_reason: scored_match.score_reason.clone(),
                    risk_flags: scored_match.risk_flags.clone(),
                    skip_recommendation: scored_match.skip_recommendation.clone(),
                    ..opportunity
                };

                match self.employment_repo.create_opportunity(opportunity).await {
                    Ok(created) => created_opportunity_ids.push(created.id.to_string()),
                    Err(e) => tracing::warn!(
                        "Failed to create opportunity {}: {}",
                        scored_match.candidate.source_url,
                        e
                    ),
                }
            }
        }
        for scored_match in scored_matches.iter().filter(|item| !item.passes_min_score) {
            skipped_opportunity_sources.push(scored_match.candidate.source_url.clone());
        }

        let created_opportunity_count = created_opportunity_ids.len();
        let existing_opportunity_count = existing_opportunity_ids.len();
        let created_summary_artifact_id = Uuid::new_v4();
        run.artifacts.push(TaskArtifact {
            id: created_summary_artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(create_step_id),
            name: "Created opportunities summary".to_string(),
            artifact_type: "created_opportunities_summary".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "create_opportunities": input.create_opportunities,
                "created_opportunity_count": created_opportunity_count,
                "existing_opportunity_count": existing_opportunity_count,
                "skipped_count": skipped_opportunity_sources.len(),
            })),
            content_text: Some(format!(
                "Created {} opportunities, refreshed {}, skipped {} below score threshold.",
                created_opportunity_count,
                existing_opportunity_count,
                skipped_opportunity_sources.len()
            )),
            content_json: Some(json!({
                "created_opportunity_ids": created_opportunity_ids,
                "existing_opportunity_ids": existing_opportunity_ids,
                "skipped_sources": skipped_opportunity_sources,
                "create_opportunities": input.create_opportunities,
                "min_score": min_score,
            })),
        });
        finish_step(
            &mut run,
            create_step_id,
            Some(format!(
                "Created {} opportunities and refreshed {} existing opportunities.",
                created_opportunity_count, existing_opportunity_count
            )),
            vec![created_summary_artifact_id],
        );

        let summary_step_id = start_step(&mut run, "summarize_run")?;
        push_step_input_artifact(&mut run, summary_step_id, artifact_id);
        push_step_input_artifact(&mut run, summary_step_id, candidates_artifact_id);
        push_step_input_artifact(&mut run, summary_step_id, scored_artifact_id);
        push_step_input_artifact(&mut run, summary_step_id, created_summary_artifact_id);
        let passing_count = scored_matches
            .iter()
            .filter(|item| item.passes_min_score)
            .count();
        let summary = format!(
            "Employment search completed with {} raw results, {} readable pages, {} extracted candidates, {} matches at or above score {}, and {} newly created opportunities.",
            result_count,
            readable_pages.len(),
            candidates.len(),
            passing_count,
            min_score,
            created_opportunity_count
        );
        finish_step(&mut run, summary_step_id, Some(summary.clone()), vec![]);

        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(summary);

        Ok(run)
    }

    async fn load_employment_context_summary(
        &self,
        profile_id: Uuid,
    ) -> anyhow::Result<Vec<Value>> {
        let mut items = Vec::new();

        for kind in [
            ContextKind::CareerProfile,
            ContextKind::EmploymentPreference,
            ContextKind::ResumeFact,
            ContextKind::ProjectSummary,
        ] {
            let contexts = self
                .context
                .get_relevant_context(profile_id, "", Some(kind.clone()))
                .await?;
            for context in contexts.into_iter().take(8) {
                items.push(json!({
                    "kind": context.kind,
                    "title": context.title,
                    "body_preview": context.body.chars().take(500).collect::<String>(),
                    "tags": context.tags,
                }));
            }
        }

        Ok(items)
    }
}

#[derive(Debug, Deserialize)]
struct EmploymentSearchInput {
    #[serde(default)]
    query_override: Option<String>,
    #[serde(default)]
    user_request: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    remote_preference: Option<String>,
    #[serde(default)]
    create_opportunities: bool,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    min_score: Option<i64>,
}

#[derive(Debug, Clone)]
struct ReadablePageForExtraction {
    artifact_id: Uuid,
    source_url: String,
    title: String,
    snippet: Option<String>,
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpportunityCandidate {
    source_url: String,
    source_name: Option<String>,
    title: Option<String>,
    company: Option<String>,
    location: Option<String>,
    remote_type: Option<String>,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
    description_text: Option<String>,
    extracted_json: Option<Value>,
    source_artifact_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScoredOpportunityMatch {
    candidate: OpportunityCandidate,
    primary_fit_score: Option<i64>,
    oe_fit_score: Option<i64>,
    recommended_track: Option<String>,
    score_reason: Option<String>,
    risk_flags: Vec<String>,
    skip_recommendation: Option<String>,
    passes_min_score: bool,
}

fn build_employment_search_query(input: &EmploymentSearchInput, profile_criteria: &str) -> String {
    let mut parts = Vec::new();

    if let Some(query_override) = clean_optional_text(input.query_override.as_deref()) {
        parts.push(query_override);
    } else {
        if let Some(user_request) = clean_optional_text(input.user_request.as_deref()) {
            parts.push(user_request);
        }
        if !profile_criteria.trim().is_empty() {
            parts.push(profile_criteria.trim().to_string());
        }
    }

    if let Some(location) = clean_optional_text(input.location.as_deref()) {
        parts.push(location);
    }
    if let Some(remote_preference) = clean_optional_text(input.remote_preference.as_deref()) {
        parts.push(remote_preference);
    }

    let query = parts.join(" ");
    employment_search_query(&query)
}

fn candidate_from_search_result(result: &SearchResultItem) -> OpportunityCandidate {
    OpportunityCandidate {
        source_url: result.url.clone(),
        source_name: Some(result.title.clone()),
        title: Some(result.title.clone()),
        company: None,
        location: None,
        remote_type: None,
        salary_min: None,
        salary_max: None,
        description_text: result.snippet.clone(),
        extracted_json: None,
        source_artifact_id: None,
    }
}

fn candidate_from_parsed(
    parsed: &Value,
    source_url: &str,
    source_name: Option<String>,
    fallback_description: Option<String>,
    source_artifact_id: Option<Uuid>,
) -> OpportunityCandidate {
    OpportunityCandidate {
        source_url: source_url.to_string(),
        source_name,
        title: parsed
            .get("title")
            .and_then(|value| value.as_str())
            .map(clean_score_text),
        company: parsed
            .get("company")
            .and_then(|value| value.as_str())
            .map(clean_score_text),
        location: parsed
            .get("location")
            .and_then(|value| value.as_str())
            .map(clean_score_text),
        remote_type: parsed
            .get("remote_type")
            .and_then(|value| value.as_str())
            .map(clean_score_text),
        salary_min: parsed.get("salary_min").and_then(|value| value.as_i64()),
        salary_max: parsed.get("salary_max").and_then(|value| value.as_i64()),
        description_text: parsed
            .get("description_text")
            .and_then(|value| value.as_str())
            .map(clean_score_text)
            .filter(|value| !value.is_empty())
            .or(fallback_description),
        extracted_json: Some(parsed.clone()),
        source_artifact_id,
    }
}

fn scored_match_from_output(
    candidate: OpportunityCandidate,
    scored: Value,
    min_score: i64,
) -> ScoredOpportunityMatch {
    let primary_fit_score = scored
        .get("primary_fit_score")
        .and_then(|value| value.as_i64())
        .map(clamp_score);
    let oe_fit_score = scored
        .get("oe_fit_score")
        .and_then(|value| value.as_i64())
        .map(clamp_score);
    let recommended_track = scored
        .get("recommended_track")
        .and_then(|value| value.as_str())
        .map(clean_score_text);
    let score_reason = scored
        .get("score_reason")
        .and_then(|value| value.as_str())
        .map(clean_score_text);
    let risk_flags = scored
        .get("risk_flags")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str())
                .map(clean_score_text)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let skip_recommendation = scored
        .get("skip_recommendation")
        .and_then(|value| value.as_str())
        .map(clean_score_text)
        .filter(|value| !value.is_empty());
    let best_score = primary_fit_score.or(oe_fit_score).unwrap_or(0);

    ScoredOpportunityMatch {
        candidate,
        primary_fit_score,
        oe_fit_score,
        recommended_track,
        score_reason,
        risk_flags,
        skip_recommendation,
        passes_min_score: best_score >= min_score,
    }
}

fn heuristic_candidate_score(candidate: &OpportunityCandidate) -> Value {
    let text = format!(
        "{}\n{}\n{}\n{}",
        candidate.title.clone().unwrap_or_default(),
        candidate.remote_type.clone().unwrap_or_default(),
        candidate.description_text.clone().unwrap_or_default(),
        candidate.location.clone().unwrap_or_default()
    )
    .to_lowercase();
    let remote_confirmed = text.contains("remote") && !text.contains("hybrid");
    let mut primary_fit_score = if text.contains("salesforce") { 74 } else { 55 };
    if text.contains("architect") || text.contains("automation") {
        primary_fit_score += 12;
    }
    let oe_fit_score = if remote_confirmed { 72 } else { 0 };

    json!({
        "primary_fit_score": primary_fit_score.clamp(0, 100),
        "oe_fit_score": oe_fit_score,
        "recommended_track": if primary_fit_score >= 75 {
            "primary"
        } else if oe_fit_score >= 75 {
            "oe"
        } else {
            "manual_review"
        },
        "score_reason": "Heuristic score from extracted candidate text.",
        "risk_flags": if remote_confirmed {
            Vec::<String>::new()
        } else {
            vec!["on_site_or_hybrid".to_string()]
        },
        "skip_recommendation": if remote_confirmed {
            Value::Null
        } else {
            json!("OE reject: remote work is not clearly confirmed.")
        }
    })
}

fn render_candidates_text(candidates: &[OpportunityCandidate]) -> String {
    if candidates.is_empty() {
        return "No opportunity candidates extracted.".to_string();
    }

    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            format!(
                "{}. {}\n{}\n{}",
                index + 1,
                candidate.title.as_deref().unwrap_or("Untitled opportunity"),
                candidate.source_url,
                candidate.description_text.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_scored_matches_text(matches: &[ScoredOpportunityMatch]) -> String {
    if matches.is_empty() {
        return "No opportunity matches scored.".to_string();
    }

    matches
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{}. {} | primary={} oe={} pass={}\n{}\n{}",
                index + 1,
                item.candidate
                    .title
                    .as_deref()
                    .unwrap_or("Untitled opportunity"),
                item.primary_fit_score
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                item.oe_fit_score
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                item.passes_min_score,
                item.candidate.source_url,
                item.score_reason.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn clean_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_score_text(value: &str) -> String {
    value.trim().to_string()
}

fn clamp_score(value: i64) -> i64 {
    value.clamp(0, 100)
}

fn start_step(run: &mut OpTaskRun, name: &str) -> anyhow::Result<Uuid> {
    let item = run
        .work_items
        .iter_mut()
        .find(|item| item.name == name)
        .ok_or_else(|| anyhow!("planned work item '{}' not found", name))?;
    item.status = OpTaskRunStatus::Running;
    item.started_at = Some(Utc::now());
    item.completed_at = None;
    item.error = None;
    Ok(item.id)
}

fn finish_step(
    run: &mut OpTaskRun,
    step_id: Uuid,
    details: Option<String>,
    output_artifact_ids: Vec<Uuid>,
) {
    if let Some(item) = run.work_items.iter_mut().find(|item| item.id == step_id) {
        item.status = OpTaskRunStatus::Succeeded;
        item.completed_at = Some(Utc::now());
        item.details = details;
        item.output_artifact_ids.extend(output_artifact_ids);
    }
}

fn finish_step_with_error(run: &mut OpTaskRun, step_id: Uuid, error: &str) {
    if let Some(item) = run.work_items.iter_mut().find(|item| item.id == step_id) {
        item.status = OpTaskRunStatus::Failed;
        item.completed_at = Some(Utc::now());
        item.error = Some(error.to_string());
    }
}

fn update_step_tool_args(run: &mut OpTaskRun, step_id: Uuid, args: serde_json::Value) {
    if let Some(item) = run.work_items.iter_mut().find(|item| item.id == step_id) {
        item.tool_args_json = Some(args);
    }
}

fn push_step_input_artifact(run: &mut OpTaskRun, step_id: Uuid, artifact_id: Uuid) {
    if let Some(item) = run.work_items.iter_mut().find(|item| item.id == step_id) {
        item.input_artifact_ids.push(artifact_id);
    }
}

fn render_search_results_text(
    query: &str,
    results: &[crate::readers::models::SearchResultItem],
) -> String {
    if results.is_empty() {
        return format!("No search results found for '{}'.", query);
    }

    results
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{}. {}\n{}\n{}",
                index + 1,
                item.title,
                item.url,
                item.snippet.clone().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn employment_search_query(criteria: &str) -> String {
    let trimmed = criteria.trim();
    if trimmed.is_empty() {
        return "jobs".to_string();
    }

    let lower = trimmed.to_lowercase();
    if lower.contains("job") || lower.contains("opportunit") || lower.contains("role") {
        trimmed.to_string()
    } else {
        format!("jobs {}", trimmed)
    }
}
