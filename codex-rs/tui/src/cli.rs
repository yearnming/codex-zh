use clap::Parser;
use clap::ValueHint;
use codex_utils_cli::ApprovalModeCliArg;
use codex_utils_cli::CliConfigOverrides;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    /// 可选：启动会话时的用户提示词。
    #[arg(value_name = "PROMPT", value_hint = clap::ValueHint::Other)]
    pub prompt: Option<String>,

    /// 可选：附加到初始提示中的图片。
    #[arg(long = "image", short = 'i', value_name = "FILE", value_delimiter = ',', num_args = 1..)]
    pub images: Vec<PathBuf>,

    // 内部控制：由顶层 `codex resume` 子命令设置。
    // 这些不会作为基础 `codex` 命令的公开参数展示。
    #[clap(skip)]
    pub resume_picker: bool,

    #[clap(skip)]
    pub resume_last: bool,

    /// 内部：通过 id（UUID）恢复指定会话。由顶层
    /// `codex resume <SESSION_ID>` 包装器设置；不会作为公开参数展示。
    #[clap(skip)]
    pub resume_session_id: Option<String>,

    /// 内部：显示所有会话（禁用 cwd 过滤并显示 CWD 列）。
    #[clap(skip)]
    pub resume_show_all: bool,

    // 内部控制：由顶层 `codex fork` 子命令设置。
    // 这些不会作为基础 `codex` 命令的公开参数展示。
    #[clap(skip)]
    pub fork_picker: bool,

    #[clap(skip)]
    pub fork_last: bool,

    /// 内部：通过 id（UUID）分叉指定会话。由顶层
    /// `codex fork <SESSION_ID>` 包装器设置；不会作为公开参数展示。
    #[clap(skip)]
    pub fork_session_id: Option<String>,

    /// 内部：显示所有会话（禁用 cwd 过滤并显示 CWD 列）。
    #[clap(skip)]
    pub fork_show_all: bool,

    /// Agent 使用的模型。
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// 便捷选项：选择本地开源模型提供方。等同于 -c model_provider=oss；
    /// 会检查本地 LM Studio 或 Ollama 服务是否在运行。
    #[arg(long = "oss", default_value_t = false)]
    pub oss: bool,

    /// 指定本地提供方（lmstudio 或 ollama）。
    /// 未配合 --oss 指定时，将使用配置默认值或弹出选择。
    #[arg(long = "local-provider")]
    pub oss_provider: Option<String>,

    /// config.toml 中的配置 profile，用于指定默认选项。
    #[arg(long = "profile", short = 'p')]
    pub config_profile: Option<String>,

    /// 选择执行模型生成的 shell 命令时的沙箱策略。
    #[arg(long = "sandbox", short = 's')]
    pub sandbox_mode: Option<codex_utils_cli::SandboxModeCliArg>,

    /// 配置模型在执行命令前何时需要人工审批。
    #[arg(long = "ask-for-approval", short = 'a')]
    pub approval_policy: Option<ApprovalModeCliArg>,

    /// 低门槛沙箱自动执行的便捷别名（-a on-request, --sandbox workspace-write）。
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    /// 跳过所有确认提示，并在无沙箱环境下执行命令。
    /// 极其危险，仅用于外部已沙箱化的环境。
    #[arg(
        long = "dangerously-bypass-approvals-and-sandbox",
        alias = "yolo",
        default_value_t = false,
        conflicts_with_all = ["approval_policy", "full_auto"]
    )]
    pub dangerously_bypass_approvals_and_sandbox: bool,

    /// 指定 Agent 使用的工作根目录。
    #[clap(long = "cd", short = 'C', value_name = "DIR")]
    pub cwd: Option<PathBuf>,

    /// 启用实时网页搜索。启用后，原生 Responses `web_search` 工具可供模型使用（无需逐次审批）。
    #[arg(long = "search", default_value_t = false)]
    pub web_search: bool,

    /// 除主工作区外还需可写的附加目录。
    #[arg(long = "add-dir", value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub add_dir: Vec<PathBuf>,

    /// 禁用备用屏幕模式
    ///
    /// 以内联模式运行 TUI，保留终端滚动历史。这在 Zellij 等严格遵循
    /// xterm 规范并禁用备用屏幕滚动历史的终端复用器中很有用。
    #[arg(long = "no-alt-screen", default_value_t = false)]
    pub no_alt_screen: bool,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,
}
