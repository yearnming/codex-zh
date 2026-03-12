#[cfg(not(target_os = "android"))]
pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| err_clipboard_unavailable(&e))?;
    cb.set_text(text.to_string())
        .map_err(|e| err_clipboard_unavailable(&e))
}

#[cfg(target_os = "android")]
pub fn copy_text_to_clipboard(_text: &str) -> Result<(), String> {
    if crate::is_zh_locale() {
        Err("Android 平台不支持复制剪贴板文本".into())
    } else {
        Err("clipboard text copy is unsupported on Android".into())
    }
}

fn err_clipboard_unavailable(err: &dyn std::fmt::Display) -> String {
    if crate::is_zh_locale() {
        format!("剪贴板不可用：{err}")
    } else {
        format!("clipboard unavailable: {err}")
    }
}
