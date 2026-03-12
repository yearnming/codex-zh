use std::env;
use std::path::PathBuf;

use codex_core::config::set_project_trust_level;
use codex_core::git_info::resolve_root_git_project_for_trust;
use codex_protocol::config_types::TrustLevel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;

use crate::key_hint;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::onboarding::onboarding_screen::StepStateProvider;
use crate::render::Insets;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::Renderable;
use crate::render::renderable::RenderableExt as _;
use crate::selection_list::selection_option_row;

use super::onboarding_screen::StepState;
pub(crate) struct TrustDirectoryWidget {
    pub codex_home: PathBuf,
    pub cwd: PathBuf,
    pub show_windows_create_sandbox_hint: bool,
    pub should_quit: bool,
    pub selection: Option<TrustDirectorySelection>,
    pub highlighted: TrustDirectorySelection,
    pub error: Option<String>,
}

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

fn trust_set_error(target: &PathBuf, err: &dyn std::fmt::Display) -> String {
    if is_zh_locale() {
        format!("设置信任目录失败：{}，错误：{err}", target.display())
    } else {
        format!("Failed to set trust for {}: {err}", target.display())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustDirectorySelection {
    Trust,
    Quit,
}

impl WidgetRef for &TrustDirectoryWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let mut column = ColumnRenderable::new();
        let is_zh = is_zh_locale();
        let header_prefix = if is_zh {
            "当前目录："
        } else {
            "You are in "
        };
        let trust_prompt = if is_zh {
            "是否信任此目录的内容？处理不受信任的内容会带来更高的提示注入风险。"
        } else {
            "Do you trust the contents of this directory? Working with untrusted contents comes with higher risk of prompt injection."
        };
        let option_yes = if is_zh {
            "是，继续"
        } else {
            "Yes, continue"
        };
        let option_no = if is_zh { "否，退出" } else { "No, quit" };
        let hint_continue = if is_zh { " 继续" } else { " to continue" };
        let hint_continue_with_sandbox = if is_zh {
            " 继续并创建沙箱..."
        } else {
            " to continue and create a sandbox..."
        };

        column.push(Line::from(vec![
            "> ".into(),
            header_prefix.bold(),
            self.cwd.to_string_lossy().to_string().into(),
        ]));
        column.push("");

        column.push(
            Paragraph::new(trust_prompt.to_string())
                .wrap(Wrap { trim: true })
                .inset(Insets::tlbr(0, 2, 0, 0)),
        );
        column.push("");

        let options: Vec<(&str, TrustDirectorySelection)> = vec![
            (option_yes, TrustDirectorySelection::Trust),
            (option_no, TrustDirectorySelection::Quit),
        ];

        for (idx, (text, selection)) in options.iter().enumerate() {
            column.push(selection_option_row(
                idx,
                text.to_string(),
                self.highlighted == *selection,
            ));
        }

        column.push("");

        if let Some(error) = &self.error {
            column.push(
                Paragraph::new(error.to_string())
                    .red()
                    .wrap(Wrap { trim: true })
                    .inset(Insets::tlbr(0, 2, 0, 0)),
            );
            column.push("");
        }

        column.push(
            Line::from(vec![
                if is_zh { "按 " } else { "Press " }.dim(),
                key_hint::plain(KeyCode::Enter).into(),
                if self.show_windows_create_sandbox_hint {
                    hint_continue_with_sandbox.dim()
                } else {
                    hint_continue.dim()
                },
            ])
            .inset(Insets::tlbr(0, 2, 0, 0)),
        );

        column.render(area, buf);
    }
}

impl KeyboardHandler for TrustDirectoryWidget {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind == KeyEventKind::Release {
            return;
        }

        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.highlighted = TrustDirectorySelection::Trust;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.highlighted = TrustDirectorySelection::Quit;
            }
            KeyCode::Char('1') | KeyCode::Char('y') => self.handle_trust(),
            KeyCode::Char('2') | KeyCode::Char('n') => self.handle_quit(),
            KeyCode::Enter => match self.highlighted {
                TrustDirectorySelection::Trust => self.handle_trust(),
                TrustDirectorySelection::Quit => self.handle_quit(),
            },
            _ => {}
        }
    }
}

impl StepStateProvider for TrustDirectoryWidget {
    fn get_step_state(&self) -> StepState {
        if self.selection.is_some() || self.should_quit {
            StepState::Complete
        } else {
            StepState::InProgress
        }
    }
}

impl TrustDirectoryWidget {
    fn handle_trust(&mut self) {
        let target =
            resolve_root_git_project_for_trust(&self.cwd).unwrap_or_else(|| self.cwd.clone());
        if let Err(e) = set_project_trust_level(&self.codex_home, &target, TrustLevel::Trusted) {
            tracing::error!("Failed to set project trusted: {e:?}");
            self.error = Some(trust_set_error(&target, &e));
        }

        self.selection = Some(TrustDirectorySelection::Trust);
    }

    fn handle_quit(&mut self) {
        self.highlighted = TrustDirectorySelection::Quit;
        self.should_quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
}

#[cfg(test)]
mod tests {
    use crate::test_backend::VT100Backend;

    use super::*;
    use crossterm::event::KeyCode;
    use crossterm::event::KeyEvent;
    use crossterm::event::KeyEventKind;
    use crossterm::event::KeyModifiers;
    use pretty_assertions::assert_eq;
    use ratatui::Terminal;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn release_event_does_not_change_selection() {
        let codex_home = TempDir::new().expect("temp home");
        let mut widget = TrustDirectoryWidget {
            codex_home: codex_home.path().to_path_buf(),
            cwd: PathBuf::from("."),
            show_windows_create_sandbox_hint: false,
            should_quit: false,
            selection: None,
            highlighted: TrustDirectorySelection::Quit,
            error: None,
        };

        let release = KeyEvent {
            kind: KeyEventKind::Release,
            ..KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        };
        widget.handle_key_event(release);
        assert_eq!(widget.selection, None);

        let press = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        widget.handle_key_event(press);
        assert!(widget.should_quit);
    }

    #[test]
    fn renders_snapshot_for_git_repo() {
        let codex_home = TempDir::new().expect("temp home");
        let widget = TrustDirectoryWidget {
            codex_home: codex_home.path().to_path_buf(),
            cwd: PathBuf::from("/workspace/project"),
            show_windows_create_sandbox_hint: false,
            should_quit: false,
            selection: None,
            highlighted: TrustDirectorySelection::Trust,
            error: None,
        };

        let mut terminal = Terminal::new(VT100Backend::new(70, 14)).expect("terminal");
        terminal
            .draw(|f| (&widget).render_ref(f.area(), f.buffer_mut()))
            .expect("draw");

        insta::assert_snapshot!(terminal.backend());
    }
}
