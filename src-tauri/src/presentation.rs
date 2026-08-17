use crate::{
    errors::user_error,
    models::{
        AppData, ParsedCommand, PresentationEntry, PresentationEntryKind, PresentationOutput,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

const MAX_DIRECTORY_ENTRIES: usize = 500;
const MAX_TEXT_FILE_BYTES: u64 = 256 * 1024;
const MAX_TEXT_FILE_LINES: usize = 5_000;

pub enum PresentationResolution {
    NeedsContext,
    Ready(PresentationResult),
}

pub struct PresentationResult {
    pub output: PresentationOutput,
    pub effective_args: Vec<String>,
    pub target_path: String,
    pub active_context: Option<String>,
}

pub fn execute(parsed: &ParsedCommand, data: &AppData) -> Result<PresentationResolution, String> {
    match parsed.executable.as_str() {
        "ls" | "ll" => list_directory(parsed, data),
        "cat" => read_text_file(parsed, data),
        _ => Err("当前命令不支持应用内展示".into()),
    }
}

fn list_directory(
    parsed: &ParsedCommand,
    data: &AppData,
) -> Result<PresentationResolution, String> {
    let (show_hidden, detailed, requested_path) = parse_list_arguments(parsed)?;
    let Some(directory) = resolve_path(requested_path.as_deref(), data)? else {
        return Ok(PresentationResolution::NeedsContext);
    };
    if !directory.is_dir() {
        return Err("目标不是文件夹，请输入可访问的目录路径".into());
    }

    let mut entries = Vec::new();
    let mut directory_count = 0;
    let mut file_count = 0;
    let mut hidden_count = 0;
    let read_dir = fs::read_dir(&directory)
        .map_err(|error| user_error(error, "无法读取目录，请检查访问权限"))?;

    for item in read_dir {
        let item = item.map_err(|error| user_error(error, "读取目录内容失败"))?;
        let name = item.file_name().to_string_lossy().into_owned();
        let hidden = name.starts_with('.');
        if hidden {
            hidden_count += 1;
            if !show_hidden {
                continue;
            }
        }
        let path = item.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| user_error(error, "无法读取文件信息"))?;
        let kind = if metadata.file_type().is_symlink() {
            PresentationEntryKind::Symlink
        } else if metadata.is_dir() {
            directory_count += 1;
            PresentationEntryKind::Directory
        } else if metadata.is_file() {
            file_count += 1;
            PresentationEntryKind::File
        } else {
            PresentationEntryKind::Other
        };
        entries.push(PresentationEntry {
            name,
            path: path.to_string_lossy().into_owned(),
            kind,
            size: metadata.is_file().then_some(metadata.len()),
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs()),
            hidden,
        });
    }

    entries.sort_by(|left, right| {
        entry_order(left.kind)
            .cmp(&entry_order(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
    let truncated = entries.len() > MAX_DIRECTORY_ENTRIES;
    entries.truncate(MAX_DIRECTORY_ENTRIES);

    let target_path = directory.to_string_lossy().into_owned();
    let mut effective_args = Vec::new();
    if show_hidden && detailed {
        effective_args.push("-al".into());
    } else if show_hidden {
        effective_args.push("-a".into());
    } else if detailed {
        effective_args.push("-l".into());
    }
    effective_args.push(target_path.clone());

    Ok(PresentationResolution::Ready(PresentationResult {
        output: PresentationOutput::Directory {
            path: target_path.clone(),
            entries,
            directory_count,
            file_count,
            hidden_count,
            detailed,
            truncated,
        },
        effective_args,
        target_path: target_path.clone(),
        active_context: Some(target_path),
    }))
}

fn read_text_file(
    parsed: &ParsedCommand,
    data: &AppData,
) -> Result<PresentationResolution, String> {
    let requested_path = parse_cat_argument(&parsed.args)?;
    let Some(path) = resolve_path(Some(requested_path), data)? else {
        return Ok(PresentationResolution::NeedsContext);
    };
    if !path.is_file() {
        return Err("目标不是普通文件，请输入可读取的文本文件路径".into());
    }
    let metadata = fs::metadata(&path).map_err(|error| user_error(error, "无法读取文件信息"))?;
    if metadata.len() > MAX_TEXT_FILE_BYTES {
        return Err("文件超过 256 KB，请使用编辑器打开或缩小查看范围".into());
    }
    let bytes = fs::read(&path).map_err(|error| user_error(error, "无法读取文件内容"))?;
    if bytes.contains(&0) {
        return Err("该文件包含二进制内容，请使用对应应用打开".into());
    }
    let content = String::from_utf8(bytes)
        .map_err(|_| "该文件不是 UTF-8 文本，请使用对应应用打开".to_string())?;
    let line_count = if content.is_empty() {
        0
    } else {
        content.lines().count()
    };
    if line_count > MAX_TEXT_FILE_LINES {
        return Err("文件超过 5000 行，请使用编辑器打开或缩小查看范围".into());
    }
    let target_path = path.to_string_lossy().into_owned();
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| target_path.clone());
    let language = path
        .extension()
        .and_then(|value| value.to_str())
        .map(language_name);
    let active_context = path
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned());

    Ok(PresentationResolution::Ready(PresentationResult {
        output: PresentationOutput::TextFile {
            path: target_path.clone(),
            name,
            content,
            size: metadata.len(),
            line_count,
            language,
        },
        effective_args: vec![target_path.clone()],
        target_path,
        active_context,
    }))
}

fn parse_list_arguments(parsed: &ParsedCommand) -> Result<(bool, bool, Option<String>), String> {
    let mut show_hidden = parsed.executable == "ll";
    let mut detailed = parsed.executable == "ll";
    let mut path = None;
    let mut options_finished = false;

    for argument in &parsed.args {
        if !options_finished && argument == "--" {
            options_finished = true;
            continue;
        }
        if !options_finished && argument.starts_with('-') && argument.len() > 1 {
            for option in argument[1..].chars() {
                match option {
                    'a' => show_hidden = true,
                    'l' => detailed = true,
                    _ => return Err(format!("暂不支持 ls -{option}，当前支持 -a 和 -l")),
                }
            }
        } else if path.replace(argument.clone()).is_some() {
            return Err("一次只能查看一个目录".into());
        }
    }
    Ok((show_hidden, detailed, path))
}

fn parse_cat_argument(args: &[String]) -> Result<&str, String> {
    match args {
        [path] if !path.starts_with('-') => Ok(path),
        [separator, path] if separator == "--" => Ok(path),
        [] => Err("请输入要查看的文本文件路径".into()),
        _ => Err("cat 当前只支持查看一个文本文件".into()),
    }
}

fn resolve_path(raw: Option<&str>, data: &AppData) -> Result<Option<PathBuf>, String> {
    let candidate = match raw {
        Some(value) if Path::new(value).is_absolute() => PathBuf::from(value),
        Some(value) => {
            let Some(context) = data.active_context.as_deref() else {
                return Ok(None);
            };
            Path::new(context).join(value)
        }
        None => {
            let Some(context) = data.active_context.as_deref() else {
                return Ok(None);
            };
            PathBuf::from(context)
        }
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|error| user_error(error, "目标路径不存在或无法访问"))?;
    validate_allowed_path(&canonical, data)?;
    Ok(Some(canonical))
}

fn validate_allowed_path(path: &Path, data: &AppData) -> Result<(), String> {
    let allowed = data
        .settings
        .workspaces
        .iter()
        .filter(|workspace| workspace.enabled)
        .any(|workspace| {
            Path::new(&workspace.path)
                .canonicalize()
                .is_ok_and(|root| path.starts_with(root))
        });
    if allowed {
        Ok(())
    } else {
        Err("目标路径不在已启用的工作区中".into())
    }
}

fn entry_order(kind: PresentationEntryKind) -> u8 {
    match kind {
        PresentationEntryKind::Directory => 0,
        PresentationEntryKind::File => 1,
        PresentationEntryKind::Symlink => 2,
        PresentationEntryKind::Other => 3,
    }
}

fn language_name(extension: &str) -> String {
    match extension.to_ascii_lowercase().as_str() {
        "rs" => "Rust",
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" => "JavaScript",
        "json" => "JSON",
        "md" => "Markdown",
        "toml" => "TOML",
        "yml" | "yaml" => "YAML",
        "css" => "CSS",
        "html" => "HTML",
        "sh" | "zsh" | "bash" => "Shell",
        "py" => "Python",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Settings, Workspace};
    use uuid::Uuid;

    fn test_data(root: &Path) -> AppData {
        AppData {
            settings: Settings {
                shortcut: String::new(),
                default_workspace: None,
                workspaces: vec![Workspace {
                    path: root.to_string_lossy().into_owned(),
                    enabled: true,
                }],
            },
            active_context: Some(root.to_string_lossy().into_owned()),
            ..AppData::default()
        }
    }

    #[test]
    fn ll_maps_to_detailed_hidden_listing() {
        let parsed = ParsedCommand {
            executable: "ll".into(),
            args: vec![],
            directory_arg_index: None,
        };
        assert_eq!(parse_list_arguments(&parsed).unwrap(), (true, true, None));
    }

    #[test]
    fn directory_output_is_structured_and_sorted() {
        let root = std::env::temp_dir().join(format!("quick-command-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("folder")).unwrap();
        fs::write(root.join("file.txt"), "hello").unwrap();
        fs::write(root.join(".hidden"), "secret").unwrap();

        let parsed = ParsedCommand {
            executable: "ls".into(),
            args: vec![],
            directory_arg_index: None,
        };
        let PresentationResolution::Ready(result) = execute(&parsed, &test_data(&root)).unwrap()
        else {
            panic!("expected presentation output");
        };
        let PresentationOutput::Directory {
            entries,
            hidden_count,
            ..
        } = result.output
        else {
            panic!("expected directory output");
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, PresentationEntryKind::Directory);
        assert_eq!(hidden_count, 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relative_paths_request_context_when_none_is_active() {
        let mut data = AppData::default();
        data.active_context = None;
        let parsed = ParsedCommand {
            executable: "cat".into(),
            args: vec!["README.md".into()],
            directory_arg_index: None,
        };
        assert!(matches!(
            execute(&parsed, &data).unwrap(),
            PresentationResolution::NeedsContext
        ));
    }

    #[test]
    fn cat_returns_text_metadata_instead_of_terminal_output() {
        let root = std::env::temp_dir().join(format!("quick-command-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("hello.rs"), "fn main() {}\n").unwrap();
        let parsed = ParsedCommand {
            executable: "cat".into(),
            args: vec!["hello.rs".into()],
            directory_arg_index: None,
        };

        let PresentationResolution::Ready(result) = execute(&parsed, &test_data(&root)).unwrap()
        else {
            panic!("expected presentation output");
        };
        let PresentationOutput::TextFile {
            content,
            language,
            line_count,
            ..
        } = result.output
        else {
            panic!("expected text output");
        };
        assert_eq!(content, "fn main() {}\n");
        assert_eq!(language.as_deref(), Some("Rust"));
        assert_eq!(line_count, 1);

        fs::remove_dir_all(root).unwrap();
    }
}
