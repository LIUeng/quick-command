use crate::command_catalog::{definition_for, is_known_command};
use crate::models::ParsedCommand;

const FORBIDDEN_TOKENS: &[&str] = &["|", "||", "&&", ";", ">", ">>", "<", "<<", "`"];

pub fn parse(input: &str) -> Result<ParsedCommand, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("请输入要执行的命令".into());
    }
    if FORBIDDEN_TOKENS.iter().any(|token| trimmed.contains(token)) || trimmed.contains("$(") {
        return Err("MVP 不支持管道、重定向或组合 Shell 命令".into());
    }
    let words = shell_words::split(trimmed).map_err(|_| "命令中的引号不完整".to_string())?;
    let (executable, args) = words
        .split_first()
        .ok_or_else(|| "请输入要执行的命令".to_string())?;
    if !is_known_command(executable) {
        return Err(format!(
            "暂不支持命令 {executable}；Quick Command 只执行已登记的安全命令"
        ));
    }
    let definition = definition_for(executable);
    let directory_arg_index = definition
        .project_search
        .then_some(0)
        .filter(|_| !args.is_empty());
    Ok(ParsedCommand {
        executable: executable.clone(),
        args: args.to_vec(),
        directory_arg_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_quoted_argument() {
        let value = parse("code \"my project\"").unwrap();
        assert_eq!(value.args, ["my project"]);
    }
    #[test]
    fn rejects_shell_operators() {
        assert!(parse("code foo && rm bar").is_err());
        assert!(parse("echo $(whoami)").is_err());
    }
    #[test]
    fn marks_known_directory_command() {
        assert_eq!(parse("code example").unwrap().directory_arg_index, Some(0));
    }

    #[test]
    fn rejects_commands_outside_the_trusted_catalog() {
        let error = parse("custom-command value").unwrap_err();
        assert!(error.contains("暂不支持命令 custom-command"));
        assert!(parse("/bin/echo value").is_err());
    }
}
