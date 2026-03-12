//! Shared popup-related constants for bottom pane widgets.

use crossterm::event::KeyCode;
use ratatui::text::Line;

use crate::key_hint;

/// Maximum number of rows any popup should attempt to display.
/// Keep this consistent across all popups for a uniform feel.
pub(crate) const MAX_POPUP_ROWS: usize = 8;

/// Standard footer hint text used by popups.
pub(crate) fn standard_popup_hint_line() -> Line<'static> {
    Line::from(vec![
        if crate::is_zh_locale() {
            "按 ".into()
        } else {
            "Press ".into()
        },
        key_hint::plain(KeyCode::Enter).into(),
        if crate::is_zh_locale() {
            " 确认或按 ".into()
        } else {
            " to confirm or ".into()
        },
        key_hint::plain(KeyCode::Esc).into(),
        if crate::is_zh_locale() {
            " 返回".into()
        } else {
            " to go back".into()
        },
    ])
}
