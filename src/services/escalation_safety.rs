use crate::error::AppError;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EscalationPrivacyClass {
    TechnicalOnly,
    Personal,
    Employment,
    Secret,
}

#[derive(Clone, Debug, Serialize)]
pub struct EscalationPolicyDecision {
    pub allowed: bool,
    pub requires_confirmation: bool,
    pub privacy_classification: EscalationPrivacyClass,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RedactionReport {
    pub redacted_keys: usize,
    pub redacted_text_values: usize,
    pub redacted_patterns: Vec<String>,
}

pub struct EscalationRedactionResult {
    pub redacted_text: Option<String>,
    pub redacted_json: Value,
    pub redaction_report: RedactionReport,
    pub privacy_classification: EscalationPrivacyClass,
    pub policy_decision: EscalationPolicyDecision,
}

pub fn redact_request_for_escalation(
    content_text: Option<&str>,
    content_json: &Value,
    confirm: bool,
) -> EscalationRedactionResult {
    let mut redaction_report = RedactionReport::default();

    let redacted_text = content_text.map(|text| redact_text(text.trim(), &mut redaction_report));
    let redacted_json = redact_json_value(content_json, &mut redaction_report);
    let combined_for_classification =
        json_for_classification(redacted_text.as_deref().unwrap_or_default(), &redacted_json);

    let privacy_classification =
        classify_escalation_privacy(&combined_for_classification, &redaction_report);
    let policy_decision =
        decide_escalation_policy(privacy_classification, confirm, &redaction_report);

    EscalationRedactionResult {
        redacted_text,
        redacted_json,
        redaction_report,
        privacy_classification,
        policy_decision,
    }
}

pub fn ensure_no_escalation_secret(
    content_text: Option<&str>,
    content_json: &Value,
) -> Result<(), AppError> {
    let text = format!(
        "{} {}",
        content_text.unwrap_or_default(),
        serde_json::to_string(content_json).unwrap_or_default()
    );
    let lower = text.to_lowercase();
    if contains_any(
        &lower,
        &[
            "password",
            "api_key",
            "apikey",
            "secret",
            "authorization",
            "bearer ",
            "private_key",
            "token",
            "cookie",
        ],
    ) || text.split_whitespace().any(looks_like_secret_token)
    {
        return Err(AppError::PolicyDenied(
            "escalation artifact contains secrets and is blocked".to_string(),
        ));
    }

    Ok(())
}

fn json_for_classification(content_text: &str, content_json: &Value) -> Value {
    serde_json::json!({
        "content_text": content_text,
        "content_json": content_json,
    })
}

fn classify_escalation_privacy(
    context: &Value,
    redaction_report: &RedactionReport,
) -> EscalationPrivacyClass {
    if redaction_report.redacted_keys > 0 || redaction_report.redacted_text_values > 0 {
        return EscalationPrivacyClass::Secret;
    }

    let text = context_search_text(context).to_lowercase();
    if contains_any(
        &text,
        &[
            "resume",
            "employment",
            "job",
            "opportunity",
            "salary",
            "cover letter",
            "interview",
            "candidate",
            "employer",
            "career",
            "salesforce architect",
        ],
    ) {
        return EscalationPrivacyClass::Employment;
    }

    if contains_any(
        &text,
        &[
            "email",
            "phone",
            "address",
            "medical",
            "health",
            "family",
            "personal",
            "ssn",
            "social security",
        ],
    ) || looks_like_email_or_phone(&text)
    {
        return EscalationPrivacyClass::Personal;
    }

    EscalationPrivacyClass::TechnicalOnly
}

fn decide_escalation_policy(
    classification: EscalationPrivacyClass,
    confirm: bool,
    redaction_report: &RedactionReport,
) -> EscalationPolicyDecision {
    match classification {
        EscalationPrivacyClass::TechnicalOnly => EscalationPolicyDecision {
            allowed: true,
            requires_confirmation: false,
            privacy_classification: classification,
            reason: "Technical-only escalation is allowed without confirmation.".to_string(),
        },
        EscalationPrivacyClass::Personal | EscalationPrivacyClass::Employment if confirm => {
            EscalationPolicyDecision {
                allowed: true,
                requires_confirmation: true,
                privacy_classification: classification,
                reason: "Confirmed personal or employment escalation.".to_string(),
            }
        }
        EscalationPrivacyClass::Personal | EscalationPrivacyClass::Employment => EscalationPolicyDecision {
            allowed: false,
            requires_confirmation: true,
            privacy_classification: classification,
            reason: format!("{:?} escalation requires explicit confirmation", classification),
        },
        EscalationPrivacyClass::Secret => EscalationPolicyDecision {
            allowed: false,
            requires_confirmation: false,
            privacy_classification: classification,
            reason: format!(
                "Escalation blocked because secrets were detected and redacted ({} keys, {} text values).",
                redaction_report.redacted_keys, redaction_report.redacted_text_values
            ),
        },
    }
}

fn context_search_text(value: &Value) -> String {
    match value {
        Value::Object(map) => map
            .iter()
            .flat_map(|(key, value)| [key.clone(), context_search_text(value)])
            .collect::<Vec<_>>()
            .join(" "),
        Value::Array(items) => items
            .iter()
            .map(context_search_text)
            .collect::<Vec<_>>()
            .join(" "),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn looks_like_email_or_phone(text: &str) -> bool {
    text.split_whitespace().any(|word| {
        let trimmed = word.trim_matches(|ch: char| {
            matches!(ch, ',' | '.' | ';' | ':' | '\"' | '\'' | ')' | ']' | '}')
        });
        let digit_count = trimmed.chars().filter(|ch| ch.is_ascii_digit()).count();
        (trimmed.contains('@') && trimmed.contains('.')) || digit_count >= 10
    })
}

fn redact_json_value(value: &Value, report: &mut RedactionReport) -> Value {
    match value {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, value) in map {
                if is_sensitive_key(key) {
                    report.redacted_keys += 1;
                    push_redaction_pattern(report, "sensitive_key");
                    redacted.insert(key.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    redacted.insert(key.clone(), redact_json_value(value, report));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_json_value(item, report))
                .collect(),
        ),
        Value::String(text) => {
            let redacted = redact_text(text, report);
            Value::String(redacted)
        }
        _ => value.clone(),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_lowercase();
    [
        "token",
        "secret",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "authorization",
        "auth",
        "bearer",
        "cookie",
        "session",
        "credential",
        "private_key",
    ]
    .iter()
    .any(|marker| key.contains(marker))
}

fn redact_text(text: &str, report: &mut RedactionReport) -> String {
    let mut changed = false;
    let redacted_words = text
        .split_whitespace()
        .map(|word| {
            let trimmed = word.trim_matches(|ch: char| {
                matches!(ch, ',' | '.' | ';' | ':' | '\"' | '\'' | ')' | ']' | '}')
            });
            let lower = trimmed.to_lowercase();
            if lower.starts_with("bearer ")
                || lower.starts_with("sk-")
                || lower.starts_with("ghp_")
                || lower.starts_with("xoxb-")
                || looks_like_long_secret(trimmed)
            {
                changed = true;
                "[REDACTED]".to_string()
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>();

    let mut redacted = redacted_words.join(" ");
    for marker in ["password=", "token=", "api_key=", "secret="] {
        if redacted.to_lowercase().contains(marker) {
            redacted = redact_assignments(&redacted, marker);
            changed = true;
        }
    }

    if changed {
        report.redacted_text_values += 1;
        push_redaction_pattern(report, "sensitive_text");
    }

    redacted
}

fn looks_like_secret_token(value: &str) -> bool {
    let trimmed = value.trim_matches(|ch: char| {
        matches!(ch, ',' | '.' | ';' | ':' | '"' | '\'' | ')' | ']' | '}')
    });
    if trimmed.len() == 36 && trimmed.chars().filter(|ch| *ch == '-').count() == 4 {
        return false;
    }

    trimmed.starts_with("sk-")
        || trimmed.starts_with("ghp_")
        || trimmed.starts_with("xoxb-")
        || (trimmed.len() >= 32
            && trimmed
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')))
}

fn looks_like_long_secret(value: &str) -> bool {
    if value.len() == 36 && value.chars().filter(|ch| *ch == '-').count() == 4 {
        return false;
    }

    value.len() >= 32
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

fn redact_assignments(value: &str, marker: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            if word.to_lowercase().starts_with(marker) {
                format!("{}[REDACTED]", &word[..marker.len().min(word.len())])
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_redaction_pattern(report: &mut RedactionReport, pattern: &str) {
    if !report
        .redacted_patterns
        .iter()
        .any(|existing| existing == pattern)
    {
        report.redacted_patterns.push(pattern.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_personal_requires_confirmation() {
        let result = redact_request_for_escalation(
            Some("User email is jane@example.com"),
            &json!({"topic": "personal planning"}),
            false,
        );
        assert!(!result.policy_decision.allowed);
        assert!(result.policy_decision.requires_confirmation);
        assert_eq!(
            result.policy_decision.privacy_classification,
            EscalationPrivacyClass::Personal
        );
    }

    #[test]
    fn technical_request_allowed_without_confirmation() {
        let result = redact_request_for_escalation(
            Some("Collect logs for cache warmup"),
            &json!({"notes": "safe maintenance task"}),
            false,
        );
        assert!(result.policy_decision.allowed);
        assert!(!result.policy_decision.requires_confirmation);
        assert_eq!(
            result.policy_decision.privacy_classification,
            EscalationPrivacyClass::TechnicalOnly
        );
    }

    #[test]
    fn redacts_string_secret_text_and_reports() {
        let result = redact_request_for_escalation(
            Some("Use token=abc123def456ghijklmnopqrstuvwx and continue."),
            &json!({"notes": "routine check"}),
            false,
        );
        assert_eq!(
            result.policy_decision.privacy_classification,
            EscalationPrivacyClass::Secret
        );
        assert_eq!(result.redaction_report.redacted_text_values, 1);
        assert_eq!(
            result.redacted_text.unwrap(),
            "Use token=[REDACTED] and continue."
        );
    }
}
