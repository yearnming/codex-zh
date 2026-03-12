use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::ArgGroup;
use codex_core::config::Config;
use codex_core::config::edit::ConfigEditsBuilder;
use codex_core::config::find_codex_home;
use codex_core::config::load_global_mcp_servers;
use codex_core::config::types::McpServerConfig;
use codex_core::config::types::McpServerTransportConfig;
use codex_core::mcp::McpManager;
use codex_core::mcp::auth::McpOAuthLoginSupport;
use codex_core::mcp::auth::compute_auth_statuses;
use codex_core::mcp::auth::oauth_login_support;
use codex_core::plugins::PluginsManager;
use codex_protocol::protocol::McpAuthStatus;
use codex_rmcp_client::delete_oauth_tokens;
use codex_rmcp_client::perform_oauth_login;
use codex_utils_cli::CliConfigOverrides;
use codex_utils_cli::format_env_display::format_env_display;

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

/// 子命令：
/// - `list`   — 列出已配置的服务器（支持 `--json`）
/// - `get`    — 显示单个服务器（支持 `--json`）
/// - `add`    — 向 `~/.codex/config.toml` 添加服务器启动配置
/// - `remove` — 删除服务器配置
/// - `login`  — 使用 OAuth 登录 MCP 服务器
/// - `logout` — 删除 MCP 服务器 OAuth 凭据
#[derive(Debug, clap::Parser)]
pub struct McpCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub subcommand: McpSubcommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum McpSubcommand {
    List(ListArgs),
    Get(GetArgs),
    Add(AddArgs),
    Remove(RemoveArgs),
    Login(LoginArgs),
    Logout(LogoutArgs),
}

#[derive(Debug, clap::Parser)]
pub struct ListArgs {
    /// 以 JSON 输出已配置的服务器。
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, clap::Parser)]
pub struct GetArgs {
    /// 要显示的 MCP 服务器名称。
    pub name: String,

    /// 以 JSON 输出服务器配置。
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, clap::Parser)]
#[command(override_usage = "codex mcp add [OPTIONS] <NAME> (--url <URL> | -- <COMMAND>...)")]
pub struct AddArgs {
    /// MCP 服务器配置名称。
    pub name: String,

    #[command(flatten)]
    pub transport_args: AddMcpTransportArgs,
}

#[derive(Debug, clap::Args)]
#[command(
    group(
        ArgGroup::new("transport")
            .args(["command", "url"])
            .required(true)
            .multiple(false)
    )
)]
pub struct AddMcpTransportArgs {
    #[command(flatten)]
    pub stdio: Option<AddMcpStdioArgs>,

    #[command(flatten)]
    pub streamable_http: Option<AddMcpStreamableHttpArgs>,
}

#[derive(Debug, clap::Args)]
pub struct AddMcpStdioArgs {
    /// 用于启动 MCP 服务器的命令。
    /// 若为 streamable HTTP 服务器，请使用 --url。
    #[arg(
            trailing_var_arg = true,
            num_args = 0..,
        )]
    pub command: Vec<String>,

    /// 启动服务器时设置的环境变量。
    /// 仅适用于 stdio 服务器。
    #[arg(
        long,
        value_parser = parse_env_pair,
        value_name = "KEY=VALUE",
    )]
    pub env: Vec<(String, String)>,
}

#[derive(Debug, clap::Args)]
pub struct AddMcpStreamableHttpArgs {
    /// streamable HTTP MCP 服务器的 URL。
    #[arg(long)]
    pub url: String,

    /// 用于读取 bearer token 的可选环境变量。
    /// 仅适用于 streamable HTTP 服务器。
    #[arg(
        long = "bearer-token-env-var",
        value_name = "ENV_VAR",
        requires = "url"
    )]
    pub bearer_token_env_var: Option<String>,
}

#[derive(Debug, clap::Parser)]
pub struct RemoveArgs {
    /// 要移除的 MCP 服务器配置名称。
    pub name: String,
}

#[derive(Debug, clap::Parser)]
pub struct LoginArgs {
    /// 要进行 OAuth 登录的 MCP 服务器名称。
    pub name: String,

    /// 以逗号分隔的 OAuth scope 列表。
    #[arg(long, value_delimiter = ',', value_name = "SCOPE,SCOPE")]
    pub scopes: Vec<String>,
}

#[derive(Debug, clap::Parser)]
pub struct LogoutArgs {
    /// 要注销的 MCP 服务器名称。
    pub name: String,
}

impl McpCli {
    pub async fn run(self) -> Result<()> {
        let McpCli {
            config_overrides,
            subcommand,
        } = self;

        match subcommand {
            McpSubcommand::List(args) => {
                run_list(&config_overrides, args).await?;
            }
            McpSubcommand::Get(args) => {
                run_get(&config_overrides, args).await?;
            }
            McpSubcommand::Add(args) => {
                run_add(&config_overrides, args).await?;
            }
            McpSubcommand::Remove(args) => {
                run_remove(&config_overrides, args).await?;
            }
            McpSubcommand::Login(args) => {
                run_login(&config_overrides, args).await?;
            }
            McpSubcommand::Logout(args) => {
                run_logout(&config_overrides, args).await?;
            }
        }

        Ok(())
    }
}

async fn run_add(config_overrides: &CliConfigOverrides, add_args: AddArgs) -> Result<()> {
    // Validate any provided overrides even though they are not currently applied.
    let overrides = config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = Config::load_with_cli_overrides(overrides)
        .await
        .with_context(|| t("failed to load configuration", "加载配置失败").to_string())?;

    let AddArgs {
        name,
        transport_args,
    } = add_args;

    validate_server_name(&name)?;

    let codex_home = find_codex_home()
        .with_context(|| t("failed to resolve CODEX_HOME", "无法解析 CODEX_HOME").to_string())?;
    let mut servers = load_global_mcp_servers(&codex_home)
        .await
        .with_context(|| {
            if is_zh_locale() {
                format!("从 {} 加载 MCP 服务器失败", codex_home.display())
            } else {
                format!("failed to load MCP servers from {}", codex_home.display())
            }
        })?;

    let transport = match transport_args {
        AddMcpTransportArgs {
            stdio: Some(stdio), ..
        } => {
            let mut command_parts = stdio.command.into_iter();
            let command_bin = command_parts
                .next()
                .ok_or_else(|| anyhow!(t("command is required", "必须提供命令")))?;
            let command_args: Vec<String> = command_parts.collect();

            let env_map = if stdio.env.is_empty() {
                None
            } else {
                Some(stdio.env.into_iter().collect::<HashMap<_, _>>())
            };
            McpServerTransportConfig::Stdio {
                command: command_bin,
                args: command_args,
                env: env_map,
                env_vars: Vec::new(),
                cwd: None,
            }
        }
        AddMcpTransportArgs {
            streamable_http:
                Some(AddMcpStreamableHttpArgs {
                    url,
                    bearer_token_env_var,
                }),
            ..
        } => McpServerTransportConfig::StreamableHttp {
            url,
            bearer_token_env_var,
            http_headers: None,
            env_http_headers: None,
        },
        AddMcpTransportArgs { .. } => bail!(
            "{}",
            t(
                "exactly one of --command or --url must be provided",
                "必须且只能提供 --command 或 --url 之一",
            )
        ),
    };

    let new_entry = McpServerConfig {
        transport: transport.clone(),
        enabled: true,
        required: false,
        disabled_reason: None,
        startup_timeout_sec: None,
        tool_timeout_sec: None,
        enabled_tools: None,
        disabled_tools: None,
        scopes: None,
        oauth_resource: None,
    };

    servers.insert(name.clone(), new_entry);

    ConfigEditsBuilder::new(&codex_home)
        .replace_mcp_servers(&servers)
        .apply()
        .await
        .with_context(|| {
            if is_zh_locale() {
                format!("写入 MCP 服务器配置失败：{}", codex_home.display())
            } else {
                format!("failed to write MCP servers to {}", codex_home.display())
            }
        })?;

    println!(
        "{}",
        if is_zh_locale() {
            format!("已添加全局 MCP 服务器 '{name}'。")
        } else {
            format!("Added global MCP server '{name}'.")
        }
    );

    match oauth_login_support(&transport).await {
        McpOAuthLoginSupport::Supported(oauth_config) => {
            println!(
                "{}",
                t(
                    "Detected OAuth support. Starting OAuth flow…",
                    "检测到 OAuth 支持，正在开始 OAuth 流程…",
                )
            );
            perform_oauth_login(
                &name,
                &oauth_config.url,
                config.mcp_oauth_credentials_store_mode,
                oauth_config.http_headers,
                oauth_config.env_http_headers,
                &Vec::new(),
                None,
                config.mcp_oauth_callback_port,
                config.mcp_oauth_callback_url.as_deref(),
            )
            .await?;
            println!("{}", t("Successfully logged in.", "登录成功。"));
        }
        McpOAuthLoginSupport::Unsupported => {}
        McpOAuthLoginSupport::Unknown(_) => println!(
            "{}",
            if is_zh_locale() {
                format!(
                    "MCP 服务器可能需要登录，也可能不需要。请运行 `codex mcp login {name}` 登录。"
                )
            } else {
                format!(
                    "MCP server may or may not require login. Run `codex mcp login {name}` to login."
                )
            }
        ),
    }

    Ok(())
}

async fn run_remove(config_overrides: &CliConfigOverrides, remove_args: RemoveArgs) -> Result<()> {
    config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;

    let RemoveArgs { name } = remove_args;

    validate_server_name(&name)?;

    let codex_home = find_codex_home()
        .with_context(|| t("failed to resolve CODEX_HOME", "无法解析 CODEX_HOME").to_string())?;
    let mut servers = load_global_mcp_servers(&codex_home)
        .await
        .with_context(|| {
            if is_zh_locale() {
                format!("从 {} 加载 MCP 服务器失败", codex_home.display())
            } else {
                format!("failed to load MCP servers from {}", codex_home.display())
            }
        })?;

    let removed = servers.remove(&name).is_some();

    if removed {
        ConfigEditsBuilder::new(&codex_home)
            .replace_mcp_servers(&servers)
            .apply()
            .await
            .with_context(|| {
                if is_zh_locale() {
                    format!("写入 MCP 服务器配置失败：{}", codex_home.display())
                } else {
                    format!("failed to write MCP servers to {}", codex_home.display())
                }
            })?;
    }

    if removed {
        println!(
            "{}",
            if is_zh_locale() {
                format!("已移除全局 MCP 服务器 '{name}'。")
            } else {
                format!("Removed global MCP server '{name}'.")
            }
        );
    } else {
        println!(
            "{}",
            if is_zh_locale() {
                format!("未找到名为 '{name}' 的 MCP 服务器。")
            } else {
                format!("No MCP server named '{name}' found.")
            }
        );
    }

    Ok(())
}

async fn run_login(config_overrides: &CliConfigOverrides, login_args: LoginArgs) -> Result<()> {
    let overrides = config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = Config::load_with_cli_overrides(overrides)
        .await
        .with_context(|| t("failed to load configuration", "加载配置失败").to_string())?;
    let mcp_manager = McpManager::new(Arc::new(PluginsManager::new(config.codex_home.clone())));
    let mcp_servers = mcp_manager.effective_servers(&config, None);

    let LoginArgs { name, scopes } = login_args;

    let Some(server) = mcp_servers.get(&name) else {
        bail!(
            "{}",
            if is_zh_locale() {
                format!("未找到名为 '{name}' 的 MCP 服务器。")
            } else {
                format!("No MCP server named '{name}' found.")
            }
        );
    };

    let (url, http_headers, env_http_headers) = match &server.transport {
        McpServerTransportConfig::StreamableHttp {
            url,
            http_headers,
            env_http_headers,
            ..
        } => (url.clone(), http_headers.clone(), env_http_headers.clone()),
        _ => bail!(
            "{}",
            t(
                "OAuth login is only supported for streamable HTTP servers.",
                "OAuth 登录仅支持 streamable HTTP 服务器。",
            )
        ),
    };

    let mut scopes = scopes;
    if scopes.is_empty() {
        scopes = server.scopes.clone().unwrap_or_default();
    }

    perform_oauth_login(
        &name,
        &url,
        config.mcp_oauth_credentials_store_mode,
        http_headers,
        env_http_headers,
        &scopes,
        server.oauth_resource.as_deref(),
        config.mcp_oauth_callback_port,
        config.mcp_oauth_callback_url.as_deref(),
    )
    .await?;
    println!(
        "{}",
        if is_zh_locale() {
            format!("已成功登录 MCP 服务器 '{name}'。")
        } else {
            format!("Successfully logged in to MCP server '{name}'.")
        }
    );
    Ok(())
}

async fn run_logout(config_overrides: &CliConfigOverrides, logout_args: LogoutArgs) -> Result<()> {
    let overrides = config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = Config::load_with_cli_overrides(overrides)
        .await
        .with_context(|| t("failed to load configuration", "加载配置失败").to_string())?;
    let mcp_manager = McpManager::new(Arc::new(PluginsManager::new(config.codex_home.clone())));
    let mcp_servers = mcp_manager.effective_servers(&config, None);

    let LogoutArgs { name } = logout_args;

    let server = mcp_servers.get(&name).ok_or_else(|| {
        anyhow!(
            "{}",
            if is_zh_locale() {
                format!("配置中未找到名为 '{name}' 的 MCP 服务器。")
            } else {
                format!("No MCP server named '{name}' found in configuration.")
            }
        )
    })?;

    let url = match &server.transport {
        McpServerTransportConfig::StreamableHttp { url, .. } => url.clone(),
        _ => bail!(
            "{}",
            t(
                "OAuth logout is only supported for streamable_http transports.",
                "OAuth 登出仅支持 streamable_http 传输。",
            )
        ),
    };

    match delete_oauth_tokens(&name, &url, config.mcp_oauth_credentials_store_mode) {
        Ok(true) => println!(
            "{}",
            if is_zh_locale() {
                format!("已删除 '{name}' 的 OAuth 凭据。")
            } else {
                format!("Removed OAuth credentials for '{name}'.")
            }
        ),
        Ok(false) => println!(
            "{}",
            if is_zh_locale() {
                format!("未找到 '{name}' 的 OAuth 凭据。")
            } else {
                format!("No OAuth credentials stored for '{name}'.")
            }
        ),
        Err(err) => {
            return Err(anyhow!(
                "{}",
                if is_zh_locale() {
                    format!("删除 OAuth 凭据失败：{err}")
                } else {
                    format!("failed to delete OAuth credentials: {err}")
                }
            ));
        }
    }

    Ok(())
}

async fn run_list(config_overrides: &CliConfigOverrides, list_args: ListArgs) -> Result<()> {
    let overrides = config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = Config::load_with_cli_overrides(overrides)
        .await
        .with_context(|| t("failed to load configuration", "加载配置失败").to_string())?;
    let mcp_manager = McpManager::new(Arc::new(PluginsManager::new(config.codex_home.clone())));
    let mcp_servers = mcp_manager.effective_servers(&config, None);

    let mut entries: Vec<_> = mcp_servers.iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    let auth_statuses =
        compute_auth_statuses(mcp_servers.iter(), config.mcp_oauth_credentials_store_mode).await;

    if list_args.json {
        let json_entries: Vec<_> = entries
            .into_iter()
            .map(|(name, cfg)| {
                let auth_status = auth_statuses
                    .get(name.as_str())
                    .map(|entry| entry.auth_status)
                    .unwrap_or(McpAuthStatus::Unsupported);
                let transport = match &cfg.transport {
                    McpServerTransportConfig::Stdio {
                        command,
                        args,
                        env,
                        env_vars,
                        cwd,
                    } => serde_json::json!({
                        "type": "stdio",
                        "command": command,
                        "args": args,
                        "env": env,
                        "env_vars": env_vars,
                        "cwd": cwd,
                    }),
                    McpServerTransportConfig::StreamableHttp {
                        url,
                        bearer_token_env_var,
                        http_headers,
                        env_http_headers,
                    } => {
                        serde_json::json!({
                            "type": "streamable_http",
                            "url": url,
                            "bearer_token_env_var": bearer_token_env_var,
                            "http_headers": http_headers,
                            "env_http_headers": env_http_headers,
                        })
                    }
                };

                serde_json::json!({
                    "name": name,
                    "enabled": cfg.enabled,
                    "disabled_reason": cfg.disabled_reason.as_ref().map(ToString::to_string),
                    "transport": transport,
                    "startup_timeout_sec": cfg
                        .startup_timeout_sec
                        .map(|timeout| timeout.as_secs_f64()),
                    "tool_timeout_sec": cfg
                        .tool_timeout_sec
                        .map(|timeout| timeout.as_secs_f64()),
                    "auth_status": auth_status,
                })
            })
            .collect();
        let output = serde_json::to_string_pretty(&json_entries)?;
        println!("{output}");
        return Ok(());
    }

    if entries.is_empty() {
        println!(
            "{}",
            t(
                "No MCP servers configured yet. Try `codex mcp add my-tool -- my-command`.",
                "尚未配置 MCP 服务器。试试 `codex mcp add my-tool -- my-command`。",
            )
        );
        return Ok(());
    }

    let mut stdio_rows: Vec<[String; 7]> = Vec::new();
    let mut http_rows: Vec<[String; 5]> = Vec::new();

    for (name, cfg) in entries {
        match &cfg.transport {
            McpServerTransportConfig::Stdio {
                command,
                args,
                env,
                env_vars,
                cwd,
            } => {
                let args_display = if args.is_empty() {
                    "-".to_string()
                } else {
                    args.join(" ")
                };
                let env_display = format_env_display(env.as_ref(), env_vars);
                let cwd_display = cwd
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "-".to_string());
                let status = format_mcp_status(cfg);
                let auth_status = auth_statuses
                    .get(name.as_str())
                    .map(|entry| entry.auth_status)
                    .unwrap_or(McpAuthStatus::Unsupported)
                    .to_string();
                stdio_rows.push([
                    name.clone(),
                    command.clone(),
                    args_display,
                    env_display,
                    cwd_display,
                    status,
                    auth_status,
                ]);
            }
            McpServerTransportConfig::StreamableHttp {
                url,
                bearer_token_env_var,
                ..
            } => {
                let status = format_mcp_status(cfg);
                let auth_status = auth_statuses
                    .get(name.as_str())
                    .map(|entry| entry.auth_status)
                    .unwrap_or(McpAuthStatus::Unsupported)
                    .to_string();
                let bearer_token_display =
                    bearer_token_env_var.as_deref().unwrap_or("-").to_string();
                http_rows.push([
                    name.clone(),
                    url.clone(),
                    bearer_token_display,
                    status,
                    auth_status,
                ]);
            }
        }
    }

    if !stdio_rows.is_empty() {
        let header_name = t("Name", "名称");
        let header_command = t("Command", "命令");
        let header_args = t("Args", "参数");
        let header_env = t("Env", "环境");
        let header_cwd = t("Cwd", "目录");
        let header_status = t("Status", "状态");
        let header_auth = t("Auth", "认证");

        let mut widths = [
            header_name.len(),
            header_command.len(),
            header_args.len(),
            header_env.len(),
            header_cwd.len(),
            header_status.len(),
            header_auth.len(),
        ];
        for row in &stdio_rows {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(cell.len());
            }
        }

        println!(
            "{name:<name_w$}  {command:<cmd_w$}  {args:<args_w$}  {env:<env_w$}  {cwd:<cwd_w$}  {status:<status_w$}  {auth:<auth_w$}",
            name = header_name,
            command = header_command,
            args = header_args,
            env = header_env,
            cwd = header_cwd,
            status = header_status,
            auth = header_auth,
            name_w = widths[0],
            cmd_w = widths[1],
            args_w = widths[2],
            env_w = widths[3],
            cwd_w = widths[4],
            status_w = widths[5],
            auth_w = widths[6],
        );

        for row in &stdio_rows {
            println!(
                "{name:<name_w$}  {command:<cmd_w$}  {args:<args_w$}  {env:<env_w$}  {cwd:<cwd_w$}  {status:<status_w$}  {auth:<auth_w$}",
                name = row[0].as_str(),
                command = row[1].as_str(),
                args = row[2].as_str(),
                env = row[3].as_str(),
                cwd = row[4].as_str(),
                status = row[5].as_str(),
                auth = row[6].as_str(),
                name_w = widths[0],
                cmd_w = widths[1],
                args_w = widths[2],
                env_w = widths[3],
                cwd_w = widths[4],
                status_w = widths[5],
                auth_w = widths[6],
            );
        }
    }

    if !stdio_rows.is_empty() && !http_rows.is_empty() {
        println!();
    }

    if !http_rows.is_empty() {
        let header_name = t("Name", "名称");
        let header_url = t("Url", "地址");
        let header_token = t("Bearer Token Env Var", "Bearer Token 环境变量");
        let header_status = t("Status", "状态");
        let header_auth = t("Auth", "认证");

        let mut widths = [
            header_name.len(),
            header_url.len(),
            header_token.len(),
            header_status.len(),
            header_auth.len(),
        ];
        for row in &http_rows {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(cell.len());
            }
        }

        println!(
            "{name:<name_w$}  {url:<url_w$}  {token:<token_w$}  {status:<status_w$}  {auth:<auth_w$}",
            name = header_name,
            url = header_url,
            token = header_token,
            status = header_status,
            auth = header_auth,
            name_w = widths[0],
            url_w = widths[1],
            token_w = widths[2],
            status_w = widths[3],
            auth_w = widths[4],
        );

        for row in &http_rows {
            println!(
                "{name:<name_w$}  {url:<url_w$}  {token:<token_w$}  {status:<status_w$}  {auth:<auth_w$}",
                name = row[0].as_str(),
                url = row[1].as_str(),
                token = row[2].as_str(),
                status = row[3].as_str(),
                auth = row[4].as_str(),
                name_w = widths[0],
                url_w = widths[1],
                token_w = widths[2],
                status_w = widths[3],
                auth_w = widths[4],
            );
        }
    }

    Ok(())
}

async fn run_get(config_overrides: &CliConfigOverrides, get_args: GetArgs) -> Result<()> {
    let overrides = config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = Config::load_with_cli_overrides(overrides)
        .await
        .with_context(|| t("failed to load configuration", "加载配置失败").to_string())?;
    let mcp_manager = McpManager::new(Arc::new(PluginsManager::new(config.codex_home.clone())));
    let mcp_servers = mcp_manager.effective_servers(&config, None);

    let Some(server) = mcp_servers.get(&get_args.name) else {
        bail!(
            "{}",
            if is_zh_locale() {
                format!("未找到名为 '{name}' 的 MCP 服务器。", name = get_args.name)
            } else {
                format!("No MCP server named '{name}' found.", name = get_args.name)
            }
        );
    };

    if get_args.json {
        let transport = match &server.transport {
            McpServerTransportConfig::Stdio {
                command,
                args,
                env,
                env_vars,
                cwd,
            } => serde_json::json!({
                "type": "stdio",
                "command": command,
                "args": args,
                "env": env,
                "env_vars": env_vars,
                "cwd": cwd,
            }),
            McpServerTransportConfig::StreamableHttp {
                url,
                bearer_token_env_var,
                http_headers,
                env_http_headers,
            } => serde_json::json!({
                "type": "streamable_http",
                "url": url,
                "bearer_token_env_var": bearer_token_env_var,
                "http_headers": http_headers,
                "env_http_headers": env_http_headers,
            }),
        };
        let output = serde_json::to_string_pretty(&serde_json::json!({
            "name": get_args.name,
            "enabled": server.enabled,
            "disabled_reason": server.disabled_reason.as_ref().map(ToString::to_string),
            "transport": transport,
            "enabled_tools": server.enabled_tools.clone(),
            "disabled_tools": server.disabled_tools.clone(),
            "startup_timeout_sec": server
                .startup_timeout_sec
                .map(|timeout| timeout.as_secs_f64()),
            "tool_timeout_sec": server
                .tool_timeout_sec
                .map(|timeout| timeout.as_secs_f64()),
        }))?;
        println!("{output}");
        return Ok(());
    }

    if !server.enabled {
        if let Some(reason) = server.disabled_reason.as_ref() {
            if is_zh_locale() {
                println!("{name}（已禁用：{reason}）", name = get_args.name);
            } else {
                println!("{name} (disabled: {reason})", name = get_args.name);
            }
        } else if is_zh_locale() {
            println!("{name}（已禁用）", name = get_args.name);
        } else {
            println!("{name} (disabled)", name = get_args.name);
        }
        return Ok(());
    }

    println!("{}", get_args.name);
    let enabled_display = if server.enabled {
        t("true", "是")
    } else {
        t("false", "否")
    };
    println!("  {}: {enabled_display}", t("enabled", "已启用"));
    let format_tool_list = |tools: &Option<Vec<String>>| -> String {
        match tools {
            Some(list) if list.is_empty() => "[]".to_string(),
            Some(list) => list.join(", "),
            None => "-".to_string(),
        }
    };
    if server.enabled_tools.is_some() {
        let enabled_tools_display = format_tool_list(&server.enabled_tools);
        println!(
            "  {}: {enabled_tools_display}",
            t("enabled_tools", "已启用的工具")
        );
    }
    if server.disabled_tools.is_some() {
        let disabled_tools_display = format_tool_list(&server.disabled_tools);
        println!(
            "  {}: {disabled_tools_display}",
            t("disabled_tools", "已禁用的工具")
        );
    }
    match &server.transport {
        McpServerTransportConfig::Stdio {
            command,
            args,
            env,
            env_vars,
            cwd,
        } => {
            println!("  {}: stdio", t("transport", "传输"));
            println!("  {}: {command}", t("command", "命令"));
            let args_display = if args.is_empty() {
                "-".to_string()
            } else {
                args.join(" ")
            };
            println!("  {}: {args_display}", t("args", "参数"));
            let cwd_display = cwd
                .as_ref()
                .map(|path| path.display().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "-".to_string());
            println!("  {}: {cwd_display}", t("cwd", "目录"));
            let env_display = format_env_display(env.as_ref(), env_vars);
            println!("  {}: {env_display}", t("env", "环境"));
        }
        McpServerTransportConfig::StreamableHttp {
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
        } => {
            println!("  {}: streamable_http", t("transport", "传输"));
            println!("  {}: {url}", t("url", "地址"));
            let bearer_token_display = bearer_token_env_var.as_deref().unwrap_or("-");
            println!(
                "  {}: {bearer_token_display}",
                t("bearer_token_env_var", "Bearer Token 环境变量")
            );
            let headers_display = match http_headers {
                Some(map) if !map.is_empty() => {
                    let mut pairs: Vec<_> = map.iter().collect();
                    pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
                    pairs
                        .into_iter()
                        .map(|(k, _)| format!("{k}=*****"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
                _ => "-".to_string(),
            };
            println!("  {}: {headers_display}", t("http_headers", "HTTP 头"));
            let env_headers_display = match env_http_headers {
                Some(map) if !map.is_empty() => {
                    let mut pairs: Vec<_> = map.iter().collect();
                    pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
                    pairs
                        .into_iter()
                        .map(|(k, var)| format!("{k}={var}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
                _ => "-".to_string(),
            };
            println!(
                "  {}: {env_headers_display}",
                t("env_http_headers", "环境变量 HTTP 头")
            );
        }
    }
    if let Some(timeout) = server.startup_timeout_sec {
        println!(
            "  {}: {}",
            t("startup_timeout_sec", "启动超时(秒)"),
            timeout.as_secs_f64()
        );
    }
    if let Some(timeout) = server.tool_timeout_sec {
        println!(
            "  {}: {}",
            t("tool_timeout_sec", "工具超时(秒)"),
            timeout.as_secs_f64()
        );
    }
    println!(
        "  {}: codex mcp remove {}",
        t("remove", "删除命令"),
        get_args.name
    );

    Ok(())
}

fn parse_env_pair(raw: &str) -> Result<(String, String), String> {
    let mut parts = raw.splitn(2, '=');
    let key = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            t(
                "environment entries must be in KEY=VALUE form",
                "环境变量项必须为 KEY=VALUE 格式",
            )
            .to_string()
        })?;
    let value = parts.next().map(str::to_string).ok_or_else(|| {
        t(
            "environment entries must be in KEY=VALUE form",
            "环境变量项必须为 KEY=VALUE 格式",
        )
        .to_string()
    })?;

    Ok((key.to_string(), value))
}

fn validate_server_name(name: &str) -> Result<()> {
    let is_valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

    if is_valid {
        Ok(())
    } else {
        bail!(
            "{}",
            if is_zh_locale() {
                format!("无效的服务器名称 '{name}'（仅允许字母、数字、'-'、'_'）")
            } else {
                format!("invalid server name '{name}' (use letters, numbers, '-', '_')")
            }
        );
    }
}

fn format_mcp_status(config: &McpServerConfig) -> String {
    if config.enabled {
        t("enabled", "已启用").to_string()
    } else if let Some(reason) = config.disabled_reason.as_ref() {
        if is_zh_locale() {
            format!("已禁用：{reason}")
        } else {
            format!("disabled: {reason}")
        }
    } else {
        t("disabled", "已禁用").to_string()
    }
}
