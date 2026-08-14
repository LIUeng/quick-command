use std::fmt::Display;

fn render_user_error(message: &str, detail: &str, include_detail: bool) -> String {
    if include_detail {
        format!("{message}: {detail}")
    } else {
        message.to_string()
    }
}

/// Converts an internal error into text that is safe to return through a
/// Tauri command. Debug builds retain the underlying detail for local
/// development, while release builds expose only the stable user message.
pub fn user_error(error: impl Display, message: &str) -> String {
    render_user_error(message, &error.to_string(), cfg!(debug_assertions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_message_omits_internal_detail() {
        let message = render_user_error(
            "快捷键无效或已被占用",
            "AcceleratorParseError(KeyCode)",
            false,
        );
        assert_eq!(message, "快捷键无效或已被占用");
        assert!(!message.contains("AcceleratorParseError"));
    }

    #[test]
    fn debug_message_keeps_internal_detail() {
        let message = render_user_error("快捷键无效", "invalid key code", true);
        assert_eq!(message, "快捷键无效: invalid key code");
    }
}
