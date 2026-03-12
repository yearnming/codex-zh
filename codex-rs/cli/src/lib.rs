pub mod debug_sandbox;
mod exit_status;
pub mod login;

use clap::Parser;
use codex_utils_cli::CliConfigOverrides;

#[derive(Debug, Parser)]
pub struct SeatbeltCommand {
    /// 便捷别名：低摩擦的沙箱自动执行（禁网沙箱，允许写入 cwd 与 TMPDIR）
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    /// 命令运行时通过 `log stream` 捕获 macOS 沙箱拒绝记录，并在退出后打印
    #[arg(long = "log-denials", default_value_t = false)]
    pub log_denials: bool,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// 在 seatbelt 下运行的完整命令参数。
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct LandlockCommand {
    /// 便捷别名：低摩擦的沙箱自动执行（禁网沙箱，允许写入 cwd 与 TMPDIR）
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// 在 landlock 下运行的完整命令参数。
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct WindowsCommand {
    /// 便捷别名：低摩擦的沙箱自动执行（禁网沙箱，允许写入 cwd 与 TMPDIR）
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// 在 Windows 受限令牌沙箱下运行的完整命令参数。
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}
