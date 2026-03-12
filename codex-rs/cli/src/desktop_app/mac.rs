use anyhow::Context as _;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use tempfile::Builder;
use tokio::process::Command;

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

pub async fn run_mac_app_open_or_install(
    workspace: PathBuf,
    download_url: String,
) -> anyhow::Result<()> {
    if let Some(app_path) = find_existing_codex_app_path() {
        eprintln!(
            "{}",
            if is_zh_locale() {
                format!(
                    "正在打开 Codex 桌面版：{app_path}...",
                    app_path = app_path.display()
                )
            } else {
                format!(
                    "Opening Codex Desktop at {app_path}...",
                    app_path = app_path.display()
                )
            }
        );
        open_codex_app(&app_path, &workspace).await?;
        return Ok(());
    }
    eprintln!(
        "{}",
        t(
            "Codex Desktop not found; downloading installer...",
            "未找到 Codex 桌面版，正在下载安装包...",
        )
    );
    let installed_app = download_and_install_codex_to_user_applications(&download_url)
        .await
        .with_context(|| {
            t(
                "failed to download/install Codex Desktop",
                "下载/安装 Codex 桌面版失败",
            )
            .to_string()
        })?;
    eprintln!(
        "{}",
        if is_zh_locale() {
            format!(
                "正在从 {installed_app} 启动 Codex 桌面版...",
                installed_app = installed_app.display()
            )
        } else {
            format!(
                "Launching Codex Desktop from {installed_app}...",
                installed_app = installed_app.display()
            )
        }
    );
    open_codex_app(&installed_app, &workspace).await?;
    Ok(())
}

fn find_existing_codex_app_path() -> Option<PathBuf> {
    candidate_codex_app_paths()
        .into_iter()
        .find(|candidate| candidate.is_dir())
}

fn candidate_codex_app_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("/Applications/Codex.app")];
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join("Applications").join("Codex.app"));
    }
    paths
}

async fn open_codex_app(app_path: &Path, workspace: &Path) -> anyhow::Result<()> {
    eprintln!(
        "{}",
        if is_zh_locale() {
            format!(
                "正在打开工作区 {workspace}...",
                workspace = workspace.display()
            )
        } else {
            format!(
                "Opening workspace {workspace}...",
                workspace = workspace.display()
            )
        }
    );
    let status = Command::new("open")
        .arg("-a")
        .arg(app_path)
        .arg(workspace)
        .status()
        .await
        .with_context(|| t("failed to invoke `open`", "调用 `open` 失败").to_string())?;

    if status.success() {
        return Ok(());
    }

    anyhow::bail!(
        "{}",
        if is_zh_locale() {
            format!(
                "`open -a {app_path} {workspace}` 退出，状态 {status}",
                app_path = app_path.display(),
                workspace = workspace.display()
            )
        } else {
            format!(
                "`open -a {app_path} {workspace}` exited with {status}",
                app_path = app_path.display(),
                workspace = workspace.display()
            )
        }
    );
}

async fn download_and_install_codex_to_user_applications(dmg_url: &str) -> anyhow::Result<PathBuf> {
    let temp_dir = Builder::new()
        .prefix("codex-app-installer-")
        .tempdir()
        .with_context(|| t("failed to create temp dir", "创建临时目录失败").to_string())?;
    let tmp_root = temp_dir.path().to_path_buf();
    let _temp_dir = temp_dir;

    let dmg_path = tmp_root.join("Codex.dmg");
    download_dmg(dmg_url, &dmg_path).await?;

    eprintln!(
        "{}",
        t(
            "Mounting Codex Desktop installer...",
            "正在挂载 Codex 桌面版安装包...",
        )
    );
    let mount_point = mount_dmg(&dmg_path).await?;
    eprintln!(
        "{}",
        if is_zh_locale() {
            format!(
                "安装包已挂载到 {mount_point}。",
                mount_point = mount_point.display()
            )
        } else {
            format!(
                "Installer mounted at {mount_point}.",
                mount_point = mount_point.display()
            )
        }
    );
    let result = async {
        let app_in_volume = find_codex_app_in_mount(&mount_point).with_context(|| {
            t(
                "failed to locate Codex.app in mounted dmg",
                "在已挂载的 dmg 中未找到 Codex.app",
            )
            .to_string()
        })?;
        install_codex_app_bundle(&app_in_volume).await
    }
    .await;

    let detach_result = detach_dmg(&mount_point).await;
    if let Err(err) = detach_result {
        eprintln!(
            "{}",
            if is_zh_locale() {
                format!(
                    "警告：卸载 {mount_point} 失败：{err}",
                    mount_point = mount_point.display()
                )
            } else {
                format!(
                    "warning: failed to detach dmg at {mount_point}: {err}",
                    mount_point = mount_point.display()
                )
            }
        );
    }

    result
}

async fn install_codex_app_bundle(app_in_volume: &Path) -> anyhow::Result<PathBuf> {
    for applications_dir in candidate_applications_dirs()? {
        eprintln!(
            "{}",
            if is_zh_locale() {
                format!(
                    "正在将 Codex 桌面版安装到 {applications_dir}...",
                    applications_dir = applications_dir.display()
                )
            } else {
                format!(
                    "Installing Codex Desktop into {applications_dir}...",
                    applications_dir = applications_dir.display()
                )
            }
        );
        std::fs::create_dir_all(&applications_dir).with_context(|| {
            if is_zh_locale() {
                format!(
                    "创建应用目录失败：{applications_dir}",
                    applications_dir = applications_dir.display()
                )
            } else {
                format!(
                    "failed to create applications dir {applications_dir}",
                    applications_dir = applications_dir.display()
                )
            }
        })?;

        let dest_app = applications_dir.join("Codex.app");
        if dest_app.is_dir() {
            return Ok(dest_app);
        }

        match copy_app_bundle(app_in_volume, &dest_app).await {
            Ok(()) => return Ok(dest_app),
            Err(err) => {
                eprintln!(
                    "{}",
                    if is_zh_locale() {
                        format!(
                            "警告：安装 Codex.app 到 {applications_dir} 失败：{err}",
                            applications_dir = applications_dir.display()
                        )
                    } else {
                        format!(
                            "warning: failed to install Codex.app to {applications_dir}: {err}",
                            applications_dir = applications_dir.display()
                        )
                    }
                );
            }
        }
    }

    anyhow::bail!(
        "{}",
        t(
            "failed to install Codex.app to any applications directory",
            "无法将 Codex.app 安装到任何 Applications 目录",
        )
    );
}

fn candidate_applications_dirs() -> anyhow::Result<Vec<PathBuf>> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    dirs.push(user_applications_dir()?);
    Ok(dirs)
}

async fn download_dmg(url: &str, dest: &Path) -> anyhow::Result<()> {
    eprintln!("{}", t("Downloading installer...", "正在下载安装包..."));
    let status = Command::new("curl")
        .arg("-fL")
        .arg("--retry")
        .arg("3")
        .arg("--retry-delay")
        .arg("1")
        .arg("-o")
        .arg(dest)
        .arg(url)
        .status()
        .await
        .with_context(|| t("failed to invoke `curl`", "调用 `curl` 失败").to_string())?;

    if status.success() {
        return Ok(());
    }
    anyhow::bail!(
        "{}",
        if is_zh_locale() {
            format!("curl 下载失败，状态 {status}")
        } else {
            format!("curl download failed with {status}")
        }
    );
}

async fn mount_dmg(dmg_path: &Path) -> anyhow::Result<PathBuf> {
    let output = Command::new("hdiutil")
        .arg("attach")
        .arg("-nobrowse")
        .arg("-readonly")
        .arg(dmg_path)
        .output()
        .await
        .with_context(|| {
            t(
                "failed to invoke `hdiutil attach`",
                "调用 `hdiutil attach` 失败",
            )
            .to_string()
        })?;

    if !output.status.success() {
        anyhow::bail!(
            "{}",
            if is_zh_locale() {
                format!(
                    "`hdiutil attach` 失败，状态 {status}：{stderr}",
                    status = output.status,
                    stderr = String::from_utf8_lossy(&output.stderr)
                )
            } else {
                format!(
                    "`hdiutil attach` failed with {status}: {stderr}",
                    status = output.status,
                    stderr = String::from_utf8_lossy(&output.stderr)
                )
            }
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_hdiutil_attach_mount_point(&stdout)
        .map(PathBuf::from)
        .with_context(|| {
            if is_zh_locale() {
                format!("无法从 hdiutil 输出解析挂载点：\n{stdout}")
            } else {
                format!("failed to parse mount point from hdiutil output:\n{stdout}")
            }
        })
}

async fn detach_dmg(mount_point: &Path) -> anyhow::Result<()> {
    let status = Command::new("hdiutil")
        .arg("detach")
        .arg(mount_point)
        .status()
        .await
        .with_context(|| {
            t(
                "failed to invoke `hdiutil detach`",
                "调用 `hdiutil detach` 失败",
            )
            .to_string()
        })?;

    if status.success() {
        return Ok(());
    }
    anyhow::bail!(
        "{}",
        if is_zh_locale() {
            format!("hdiutil detach 失败，状态 {status}")
        } else {
            format!("hdiutil detach failed with {status}")
        }
    );
}

fn find_codex_app_in_mount(mount_point: &Path) -> anyhow::Result<PathBuf> {
    let direct = mount_point.join("Codex.app");
    if direct.is_dir() {
        return Ok(direct);
    }

    for entry in std::fs::read_dir(mount_point).with_context(|| {
        if is_zh_locale() {
            format!(
                "读取 {mount_point} 失败",
                mount_point = mount_point.display()
            )
        } else {
            format!(
                "failed to read {mount_point}",
                mount_point = mount_point.display()
            )
        }
    })? {
        let entry = entry.with_context(|| {
            t(
                "failed to read mount directory entry",
                "读取挂载目录条目失败",
            )
            .to_string()
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "app") && path.is_dir() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "{}",
        if is_zh_locale() {
            format!(
                "在 {mount_point} 未找到 .app 包",
                mount_point = mount_point.display()
            )
        } else {
            format!(
                "no .app bundle found at {mount_point}",
                mount_point = mount_point.display()
            )
        }
    );
}

async fn copy_app_bundle(src_app: &Path, dest_app: &Path) -> anyhow::Result<()> {
    let status = Command::new("ditto")
        .arg(src_app)
        .arg(dest_app)
        .status()
        .await
        .with_context(|| t("failed to invoke `ditto`", "调用 `ditto` 失败").to_string())?;

    if status.success() {
        return Ok(());
    }
    anyhow::bail!(
        "{}",
        if is_zh_locale() {
            format!("ditto 复制失败，状态 {status}")
        } else {
            format!("ditto copy failed with {status}")
        }
    );
}

fn user_applications_dir() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .with_context(|| t("HOME is not set", "未设置 HOME").to_string())?;
    Ok(PathBuf::from(home).join("Applications"))
}

fn parse_hdiutil_attach_mount_point(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        if !line.contains("/Volumes/") {
            return None;
        }
        if let Some((_, mount)) = line.rsplit_once('\t') {
            return Some(mount.trim().to_string());
        }
        line.split_whitespace()
            .find(|field| field.starts_with("/Volumes/"))
            .map(str::to_string)
    })
}

#[cfg(test)]
mod tests {
    use super::parse_hdiutil_attach_mount_point;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_mount_point_from_tab_separated_hdiutil_output() {
        let output = "/dev/disk2s1\tApple_HFS\tCodex\t/Volumes/Codex\n";
        assert_eq!(
            parse_hdiutil_attach_mount_point(output).as_deref(),
            Some("/Volumes/Codex")
        );
    }

    #[test]
    fn parses_mount_point_with_spaces() {
        let output = "/dev/disk2s1\tApple_HFS\tCodex Installer\t/Volumes/Codex Installer\n";
        assert_eq!(
            parse_hdiutil_attach_mount_point(output).as_deref(),
            Some("/Volumes/Codex Installer")
        );
    }
}
