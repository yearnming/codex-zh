use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::IntoStaticStr;

/// Commands that can be invoked by starting a message with a leading slash.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr, IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum SlashCommand {
    // DO NOT ALPHA-SORT! Enum order is presentation order in the popup, so
    // more frequently used commands should be listed first.
    Model,
    Fast,
    Approvals,
    Permissions,
    #[strum(serialize = "setup-default-sandbox")]
    ElevateSandbox,
    #[strum(serialize = "sandbox-add-read-dir")]
    SandboxReadRoot,
    Experimental,
    Skills,
    Review,
    Rename,
    New,
    Resume,
    Fork,
    Init,
    Compact,
    Plan,
    Collab,
    Agent,
    // Undo,
    Diff,
    Copy,
    Mention,
    Status,
    DebugConfig,
    Statusline,
    Theme,
    Mcp,
    Apps,
    Logout,
    Quit,
    Exit,
    Feedback,
    Rollout,
    Ps,
    Clean,
    Clear,
    Personality,
    Realtime,
    Settings,
    TestApproval,
    MultiAgents,
    // Debugging commands.
    #[strum(serialize = "debug-m-drop")]
    MemoryDrop,
    #[strum(serialize = "debug-m-update")]
    MemoryUpdate,
}

fn t(en: &'static str, zh: &'static str) -> &'static str {
    if crate::is_zh_locale() { zh } else { en }
}

impl SlashCommand {
    /// User-visible description shown in the popup.
    pub fn description(self) -> &'static str {
        match self {
            SlashCommand::Feedback => t("send logs to maintainers", "向维护者发送日志"),
            SlashCommand::New => t(
                "start a new chat during a conversation",
                "在会话中开始新聊天",
            ),
            SlashCommand::Init => t(
                "create an AGENTS.md file with instructions for Codex",
                "创建包含 Codex 指令的 AGENTS.md 文件",
            ),
            SlashCommand::Compact => t(
                "summarize conversation to prevent hitting the context limit",
                "总结对话以避免触及上下文上限",
            ),
            SlashCommand::Review => t(
                "review my current changes and find issues",
                "审查当前改动并找出问题",
            ),
            SlashCommand::Rename => t("rename the current thread", "重命名当前会话"),
            SlashCommand::Resume => t("resume a saved chat", "恢复已保存的聊天"),
            SlashCommand::Clear => t(
                "clear the terminal and start a new chat",
                "清空终端并开始新聊天",
            ),
            SlashCommand::Fork => t("fork the current chat", "分叉当前聊天"),
            // SlashCommand::Undo => "ask Codex to undo a turn",
            SlashCommand::Quit | SlashCommand::Exit => t("exit Codex", "退出 Codex"),
            SlashCommand::Diff => t(
                "show git diff (including untracked files)",
                "显示 git diff（包含未跟踪文件）",
            ),
            SlashCommand::Copy => t(
                "copy the latest Codex output to your clipboard",
                "复制最新的 Codex 输出到剪贴板",
            ),
            SlashCommand::Mention => t("mention a file", "引用一个文件"),
            SlashCommand::Skills => t(
                "use skills to improve how Codex performs specific tasks",
                "使用技能提升 Codex 对特定任务的表现",
            ),
            SlashCommand::Status => t(
                "show current session configuration and token usage",
                "显示当前会话配置与 Token 用量",
            ),
            SlashCommand::DebugConfig => t(
                "show config layers and requirement sources for debugging",
                "显示配置层与 requirement 来源以便调试",
            ),
            SlashCommand::Statusline => t(
                "configure which items appear in the status line",
                "配置状态行显示的项目",
            ),
            SlashCommand::Theme => t("choose a syntax highlighting theme", "选择语法高亮主题"),
            SlashCommand::Ps => t("list background terminals", "列出后台终端"),
            SlashCommand::Clean => t("stop all background terminals", "停止所有后台终端"),
            SlashCommand::MemoryDrop => t("DO NOT USE", "不要使用"),
            SlashCommand::MemoryUpdate => t("DO NOT USE", "不要使用"),
            SlashCommand::Model => t(
                "choose what model and reasoning effort to use",
                "选择使用的模型与推理强度",
            ),
            SlashCommand::Fast => t(
                "toggle Fast mode to enable fastest inference at 2X plan usage",
                "切换 Fast 模式：以 2X 计划额度启用最快推理",
            ),
            SlashCommand::Personality => t(
                "choose a communication style for Codex",
                "选择 Codex 的沟通风格",
            ),
            SlashCommand::Realtime => t(
                "toggle realtime voice mode (experimental)",
                "切换实时语音模式（实验性）",
            ),
            SlashCommand::Settings => t(
                "configure realtime microphone/speaker",
                "配置实时麦克风/扬声器",
            ),
            SlashCommand::Plan => t("switch to Plan mode", "切换到 Plan 模式"),
            SlashCommand::Collab => t(
                "change collaboration mode (experimental)",
                "更改协作模式（实验性）",
            ),
            SlashCommand::Agent | SlashCommand::MultiAgents => {
                t("switch the active agent thread", "切换当前 Agent 会话")
            }
            SlashCommand::Approvals => t(
                "choose what Codex is allowed to do",
                "选择 Codex 允许执行的操作",
            ),
            SlashCommand::Permissions => t(
                "choose what Codex is allowed to do",
                "选择 Codex 允许执行的操作",
            ),
            SlashCommand::ElevateSandbox => {
                t("set up elevated agent sandbox", "设置提升权限的 Agent 沙盒")
            }
            SlashCommand::SandboxReadRoot => t(
                "let sandbox read a directory: /sandbox-add-read-dir <absolute_path>",
                "允许沙盒读取目录：/sandbox-add-read-dir <绝对路径>",
            ),
            SlashCommand::Experimental => t("toggle experimental features", "切换实验性功能"),
            SlashCommand::Mcp => t("list configured MCP tools", "列出已配置的 MCP 工具"),
            SlashCommand::Apps => t("manage apps", "管理应用"),
            SlashCommand::Logout => t("log out of Codex", "退出登录"),
            SlashCommand::Rollout => t("print the rollout file path", "打印 rollout 文件路径"),
            SlashCommand::TestApproval => t("test approval request", "测试审批请求"),
        }
    }

    /// Command string without the leading '/'. Provided for compatibility with
    /// existing code that expects a method named `command()`.
    pub fn command(self) -> &'static str {
        self.into()
    }

    /// Whether this command supports inline args (for example `/review ...`).
    pub fn supports_inline_args(self) -> bool {
        matches!(
            self,
            SlashCommand::Review
                | SlashCommand::Rename
                | SlashCommand::Plan
                | SlashCommand::Fast
                | SlashCommand::SandboxReadRoot
        )
    }

    /// Whether this command can be run while a task is in progress.
    pub fn available_during_task(self) -> bool {
        match self {
            SlashCommand::New
            | SlashCommand::Resume
            | SlashCommand::Fork
            | SlashCommand::Init
            | SlashCommand::Compact
            // | SlashCommand::Undo
            | SlashCommand::Model
            | SlashCommand::Fast
            | SlashCommand::Personality
            | SlashCommand::Approvals
            | SlashCommand::Permissions
            | SlashCommand::ElevateSandbox
            | SlashCommand::SandboxReadRoot
            | SlashCommand::Experimental
            | SlashCommand::Review
            | SlashCommand::Plan
            | SlashCommand::Clear
            | SlashCommand::Logout
            | SlashCommand::MemoryDrop
            | SlashCommand::MemoryUpdate => false,
            SlashCommand::Diff
            | SlashCommand::Copy
            | SlashCommand::Rename
            | SlashCommand::Mention
            | SlashCommand::Skills
            | SlashCommand::Status
            | SlashCommand::DebugConfig
            | SlashCommand::Ps
            | SlashCommand::Clean
            | SlashCommand::Mcp
            | SlashCommand::Apps
            | SlashCommand::Feedback
            | SlashCommand::Quit
            | SlashCommand::Exit => true,
            SlashCommand::Rollout => true,
            SlashCommand::TestApproval => true,
            SlashCommand::Realtime => true,
            SlashCommand::Settings => true,
            SlashCommand::Collab => true,
            SlashCommand::Agent | SlashCommand::MultiAgents => true,
            SlashCommand::Statusline => false,
            SlashCommand::Theme => false,
        }
    }

    fn is_visible(self) -> bool {
        match self {
            SlashCommand::SandboxReadRoot => cfg!(target_os = "windows"),
            SlashCommand::Copy => !cfg!(target_os = "android"),
            SlashCommand::Rollout | SlashCommand::TestApproval => cfg!(debug_assertions),
            _ => true,
        }
    }
}

/// Return all built-in commands in a Vec paired with their command string.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::iter()
        .filter(|command| command.is_visible())
        .map(|c| (c.command(), c))
        .collect()
}
