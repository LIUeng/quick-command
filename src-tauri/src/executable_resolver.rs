use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn resolve(executable: &str) -> Result<PathBuf, String> {
    resolve_from(
        executable,
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("HOME").as_deref().map(Path::new),
    )
    .ok_or_else(|| format!("未找到命令 {executable}，请确认对应应用已安装，并已启用它的命令行工具"))
}

fn resolve_from(
    executable: &str,
    path_value: Option<&OsStr>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    candidate_paths(executable, path_value, home)
        .into_iter()
        .find(|candidate| is_executable(candidate))
}

fn candidate_paths(
    executable: &str,
    path_value: Option<&OsStr>,
    home: Option<&Path>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path_value) = path_value {
        candidates.extend(std::env::split_paths(path_value).map(|path| path.join(executable)));
    }
    if let Some(home) = home {
        candidates.push(home.join(".local/bin").join(executable));
        candidates.push(home.join("bin").join(executable));
    }
    for directory in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        candidates.push(Path::new(directory).join(executable));
    }
    candidates.extend(application_cli_paths(executable, home));
    candidates
}

fn application_cli_paths(executable: &str, home: Option<&Path>) -> Vec<PathBuf> {
    let relative_paths: &[&str] = match executable {
        "code" => &["Visual Studio Code.app/Contents/Resources/app/bin/code"],
        "cursor" => &["Cursor.app/Contents/Resources/app/bin/cursor"],
        "zed" => &["Zed.app/Contents/MacOS/zed"],
        "idea" => &[
            "IntelliJ IDEA.app/Contents/MacOS/idea",
            "IntelliJ IDEA CE.app/Contents/MacOS/idea",
        ],
        "webstorm" => &["WebStorm.app/Contents/MacOS/webstorm"],
        _ => &[],
    };
    let mut candidates = Vec::new();
    for relative in relative_paths {
        candidates.push(Path::new("/Applications").join(relative));
        if let Some(home) = home {
            candidates.push(home.join("Applications").join(relative));
        }
    }
    candidates
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[cfg(unix)]
    #[test]
    fn resolves_an_executable_from_the_provided_path() {
        let root = std::env::temp_dir().join(format!("quick-command-bin-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let executable = root.join("editor");
        fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
        let mut permissions = executable.metadata().unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).unwrap();

        assert_eq!(
            resolve_from("editor", Some(root.as_os_str()), None),
            Some(executable)
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn ignores_files_without_execute_permission() {
        let root = std::env::temp_dir().join(format!("quick-command-bin-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let executable = root.join("editor");
        fs::write(&executable, "not executable").unwrap();
        let mut permissions = executable.metadata().unwrap().permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&executable, permissions).unwrap();

        assert_eq!(resolve_from("editor", Some(root.as_os_str()), None), None);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn includes_application_bundle_cli_fallbacks() {
        let home = Path::new("/Users/example");
        let paths = candidate_paths("code", None, Some(home));
        assert!(paths.contains(&PathBuf::from(
            "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code"
        )));
        assert!(paths.contains(
            &home.join("Applications/Visual Studio Code.app/Contents/Resources/app/bin/code")
        ));
    }
}
