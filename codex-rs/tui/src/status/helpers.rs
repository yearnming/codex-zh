use crate::exec_command::relativize_to_home;
use crate::text_formatting;
use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use codex_core::AuthManager;
use codex_core::auth::AuthMode as CoreAuthMode;
use codex_core::config::Config;
use codex_core::project_doc::discover_project_doc_paths;
use codex_protocol::account::PlanType;
use std::env;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

use super::account::StatusAccountDisplay;

fn normalize_locale(value: &str) -> String {
    value.replace('_', "-").replace('.', "-").to_lowercase()
}

fn is_zh_locale() -> bool {
    let locale = env::var("CODEX_LOCALE").ok().filter(|v| !v.is_empty());
    let Some(locale) = locale else {
        return true;
    };
    normalize_locale(&locale).starts_with("zh")
}

fn t(en: &'static str, zh: &'static str) -> &'static str {
    if is_zh_locale() { zh } else { en }
}

fn normalize_agents_display_path(path: &Path) -> String {
    dunce::simplified(path).display().to_string()
}

pub(crate) fn compose_model_display(
    model_name: &str,
    entries: &[(&str, String)],
) -> (String, Vec<String>) {
    let mut details: Vec<String> = Vec::new();
    if let Some((_, effort)) = entries.iter().find(|(k, _)| *k == "reasoning effort") {
        let effort = effort.trim();
        let effort_lower = effort.to_ascii_lowercase();
        let effort_display = if is_zh_locale() {
            match effort_lower.as_str() {
                "low" => "低",
                "medium" => "中",
                "high" => "高",
                "xhigh" | "extra-high" | "extra_high" => "超高",
                "none" => "无",
                other => other,
            }
        } else {
            effort_lower.as_str()
        };
        details.push(if is_zh_locale() {
            format!("推理 {effort_display}")
        } else {
            format!("reasoning {effort_display}")
        });
    }
    if let Some((_, summary)) = entries.iter().find(|(k, _)| *k == "reasoning summaries") {
        let summary = summary.trim();
        if summary.eq_ignore_ascii_case("none") || summary.eq_ignore_ascii_case("off") {
            details.push(t("summaries off", "摘要已关闭").to_string());
        } else if !summary.is_empty() {
            let summary_lower = summary.to_ascii_lowercase();
            if is_zh_locale() {
                let summary_display = match summary_lower.as_str() {
                    "auto" => "自动",
                    "brief" => "简要",
                    "detailed" => "详细",
                    "full" => "完整",
                    other => other,
                };
                details.push(format!("摘要 {summary_display}"));
            } else {
                details.push(format!("summaries {summary_lower}"));
            }
        }
    }

    (model_name.to_string(), details)
}

pub(crate) fn compose_agents_summary(config: &Config) -> String {
    match discover_project_doc_paths(config) {
        Ok(paths) => {
            let mut rels: Vec<String> = Vec::new();
            for p in paths {
                let file_name = p
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| t("<unknown>", "未知").to_string());
                let display = if let Some(parent) = p.parent() {
                    if parent == config.cwd {
                        file_name.clone()
                    } else {
                        let mut cur = config.cwd.as_path();
                        let mut ups = 0usize;
                        let mut reached = false;
                        while let Some(c) = cur.parent() {
                            if cur == parent {
                                reached = true;
                                break;
                            }
                            cur = c;
                            ups += 1;
                        }
                        if reached {
                            let up = format!("..{}", std::path::MAIN_SEPARATOR);
                            format!("{}{}", up.repeat(ups), file_name)
                        } else if let Ok(stripped) = p.strip_prefix(&config.cwd) {
                            normalize_agents_display_path(stripped)
                        } else {
                            normalize_agents_display_path(&p)
                        }
                    }
                } else {
                    normalize_agents_display_path(&p)
                };
                rels.push(display);
            }
            if rels.is_empty() {
                t("<none>", "无").to_string()
            } else {
                rels.join(", ")
            }
        }
        Err(_) => t("<none>", "无").to_string(),
    }
}

pub(crate) fn compose_account_display(
    auth_manager: &AuthManager,
    plan: Option<PlanType>,
) -> Option<StatusAccountDisplay> {
    let auth = auth_manager.auth_cached()?;

    match auth.auth_mode() {
        CoreAuthMode::ApiKey => Some(StatusAccountDisplay::ApiKey),
        CoreAuthMode::Chatgpt => {
            let email = auth.get_account_email();
            let plan = plan
                .map(|plan_type| title_case(format!("{plan_type:?}").as_str()))
                .or_else(|| Some(t("Unknown", "未知").to_string()));
            Some(StatusAccountDisplay::ChatGpt { email, plan })
        }
    }
}

pub(crate) fn format_tokens_compact(value: i64) -> String {
    let value = value.max(0);
    if value == 0 {
        return "0".to_string();
    }
    if value < 1_000 {
        return value.to_string();
    }

    let value_f64 = value as f64;
    let (scaled, suffix) = if value >= 1_000_000_000_000 {
        (value_f64 / 1_000_000_000_000.0, "T")
    } else if value >= 1_000_000_000 {
        (value_f64 / 1_000_000_000.0, "B")
    } else if value >= 1_000_000 {
        (value_f64 / 1_000_000.0, "M")
    } else {
        (value_f64 / 1_000.0, "K")
    };

    let decimals = if scaled < 10.0 {
        2
    } else if scaled < 100.0 {
        1
    } else {
        0
    };

    let mut formatted = format!("{scaled:.decimals$}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }

    format!("{formatted}{suffix}")
}

pub(crate) fn format_directory_display(directory: &Path, max_width: Option<usize>) -> String {
    let formatted = if let Some(rel) = relativize_to_home(directory) {
        if rel.as_os_str().is_empty() {
            "~".to_string()
        } else {
            format!("~{}{}", std::path::MAIN_SEPARATOR, rel.display())
        }
    } else {
        directory.display().to_string()
    };

    if let Some(max_width) = max_width {
        if max_width == 0 {
            return String::new();
        }
        if UnicodeWidthStr::width(formatted.as_str()) > max_width {
            return text_formatting::center_truncate_path(&formatted, max_width);
        }
    }

    formatted
}

pub(crate) fn format_reset_timestamp(dt: DateTime<Local>, captured_at: DateTime<Local>) -> String {
    let time = dt.format("%H:%M").to_string();
    if dt.date_naive() == captured_at.date_naive() {
        time
    } else if is_zh_locale() {
        format!("{time} 于 {}月{}日", dt.month(), dt.day())
    } else {
        format!("{time} on {}", dt.format("%-d %b"))
    }
}

pub(crate) fn title_case(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut chars = s.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return String::new(),
    };
    let rest: String = chars.as_str().to_ascii_lowercase();
    first.to_uppercase().collect::<String>() + &rest
}
