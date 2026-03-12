use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::SandboxPolicy;
use std::env;

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

/// A simple preset pairing an approval policy with a sandbox policy.
#[derive(Debug, Clone)]
pub struct ApprovalPreset {
    /// Stable identifier for the preset.
    pub id: &'static str,
    /// Display label shown in UIs.
    pub label: &'static str,
    /// Short human description shown next to the label in UIs.
    pub description: &'static str,
    /// Approval policy to apply.
    pub approval: AskForApproval,
    /// Sandbox policy to apply.
    pub sandbox: SandboxPolicy,
}

/// Built-in list of approval presets that pair approval and sandbox policy.
///
/// Keep this UI-agnostic so it can be reused by both TUI and MCP server.
pub fn builtin_approval_presets() -> Vec<ApprovalPreset> {
    vec![
        ApprovalPreset {
            id: "read-only",
            label: t("Read Only", "只读"),
            description: t(
                "Codex can read files in the current workspace. Approval is required to edit files or access the internet.",
                "Codex 可读取当前工作区中的文件。编辑文件或访问互联网需要审批。",
            ),
            approval: AskForApproval::OnRequest,
            sandbox: SandboxPolicy::new_read_only_policy(),
        },
        ApprovalPreset {
            id: "auto",
            label: t("Default", "默认"),
            description: t(
                "Codex can read and edit files in the current workspace, and run commands. Approval is required to access the internet or edit other files. (Identical to Agent mode)",
                "Codex 可以读取并编辑当前工作区中的文件，并运行命令。访问互联网或编辑其他文件需要审批。（与 Agent 模式相同）",
            ),
            approval: AskForApproval::OnRequest,
            sandbox: SandboxPolicy::new_workspace_write_policy(),
        },
        ApprovalPreset {
            id: "full-access",
            label: t("Full Access", "完全访问"),
            description: t(
                "Codex can edit files outside this workspace and access the internet without asking for approval. Exercise caution when using.",
                "Codex 可在无需审批的情况下编辑工作区外文件并访问互联网。使用时请谨慎。",
            ),
            approval: AskForApproval::Never,
            sandbox: SandboxPolicy::DangerFullAccess,
        },
    ]
}
