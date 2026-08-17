#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Presentation,
    Launcher,
    Operation,
    Navigation,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Spawn,
    Capture,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextRequirement {
    None,
    Workspace,
    Directory,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathIntent {
    None,
    Directory,
    File,
    FileOrDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,
    Confirm,
    Restricted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessBehavior {
    Hide,
    ShowOutput,
    ShowMessage,
    UpdateContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDefinition {
    pub executable: &'static str,
    pub mapped_executable: &'static str,
    pub injected_args: &'static [&'static str],
    pub category: CommandCategory,
    pub execution_mode: ExecutionMode,
    pub context_requirement: ContextRequirement,
    pub path_intent: PathIntent,
    pub path_argument_index: Option<usize>,
    pub project_search: bool,
    pub risk_level: RiskLevel,
    pub success_behavior: SuccessBehavior,
}

const DEFINITIONS: &[CommandDefinition] = &[
    CommandDefinition {
        executable: "ls",
        mapped_executable: "ls",
        injected_args: &[],
        category: CommandCategory::Presentation,
        execution_mode: ExecutionMode::Capture,
        context_requirement: ContextRequirement::Directory,
        path_intent: PathIntent::Directory,
        path_argument_index: None,
        project_search: false,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::ShowOutput,
    },
    CommandDefinition {
        executable: "ll",
        mapped_executable: "ls",
        injected_args: &["-al"],
        category: CommandCategory::Presentation,
        execution_mode: ExecutionMode::Capture,
        context_requirement: ContextRequirement::Directory,
        path_intent: PathIntent::Directory,
        path_argument_index: None,
        project_search: false,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::ShowOutput,
    },
    CommandDefinition {
        executable: "cat",
        mapped_executable: "cat",
        injected_args: &[],
        category: CommandCategory::Presentation,
        execution_mode: ExecutionMode::Capture,
        context_requirement: ContextRequirement::File,
        path_intent: PathIntent::File,
        path_argument_index: Some(0),
        project_search: false,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::ShowOutput,
    },
    CommandDefinition {
        executable: "code",
        mapped_executable: "code",
        injected_args: &[],
        category: CommandCategory::Launcher,
        execution_mode: ExecutionMode::Spawn,
        context_requirement: ContextRequirement::None,
        path_intent: PathIntent::FileOrDirectory,
        path_argument_index: Some(0),
        project_search: true,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::Hide,
    },
    CommandDefinition {
        executable: "open",
        mapped_executable: "open",
        injected_args: &[],
        category: CommandCategory::Launcher,
        execution_mode: ExecutionMode::Spawn,
        context_requirement: ContextRequirement::None,
        path_intent: PathIntent::FileOrDirectory,
        path_argument_index: Some(0),
        project_search: true,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::Hide,
    },
    CommandDefinition {
        executable: "cursor",
        mapped_executable: "cursor",
        injected_args: &[],
        category: CommandCategory::Launcher,
        execution_mode: ExecutionMode::Spawn,
        context_requirement: ContextRequirement::None,
        path_intent: PathIntent::FileOrDirectory,
        path_argument_index: Some(0),
        project_search: true,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::Hide,
    },
    CommandDefinition {
        executable: "idea",
        mapped_executable: "idea",
        injected_args: &[],
        category: CommandCategory::Launcher,
        execution_mode: ExecutionMode::Spawn,
        context_requirement: ContextRequirement::None,
        path_intent: PathIntent::FileOrDirectory,
        path_argument_index: Some(0),
        project_search: true,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::Hide,
    },
    CommandDefinition {
        executable: "webstorm",
        mapped_executable: "webstorm",
        injected_args: &[],
        category: CommandCategory::Launcher,
        execution_mode: ExecutionMode::Spawn,
        context_requirement: ContextRequirement::None,
        path_intent: PathIntent::FileOrDirectory,
        path_argument_index: Some(0),
        project_search: true,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::Hide,
    },
    CommandDefinition {
        executable: "zed",
        mapped_executable: "zed",
        injected_args: &[],
        category: CommandCategory::Launcher,
        execution_mode: ExecutionMode::Spawn,
        context_requirement: ContextRequirement::None,
        path_intent: PathIntent::FileOrDirectory,
        path_argument_index: Some(0),
        project_search: true,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::Hide,
    },
    CommandDefinition {
        executable: "mkdir",
        mapped_executable: "mkdir",
        injected_args: &[],
        category: CommandCategory::Operation,
        execution_mode: ExecutionMode::Internal,
        context_requirement: ContextRequirement::Workspace,
        path_intent: PathIntent::Directory,
        path_argument_index: Some(0),
        project_search: false,
        risk_level: RiskLevel::Confirm,
        success_behavior: SuccessBehavior::ShowMessage,
    },
    CommandDefinition {
        executable: "cd",
        mapped_executable: "cd",
        injected_args: &[],
        category: CommandCategory::Navigation,
        execution_mode: ExecutionMode::Internal,
        context_requirement: ContextRequirement::Directory,
        path_intent: PathIntent::Directory,
        path_argument_index: Some(0),
        project_search: true,
        risk_level: RiskLevel::Safe,
        success_behavior: SuccessBehavior::UpdateContext,
    },
];

pub fn is_known_command(executable: &str) -> bool {
    DEFINITIONS
        .iter()
        .any(|definition| definition.executable == executable)
}

pub fn definition_for(executable: &str) -> CommandDefinition {
    DEFINITIONS
        .iter()
        .find(|definition| definition.executable == executable)
        .copied()
        .unwrap_or(CommandDefinition {
            executable: "<raw>",
            mapped_executable: "<raw>",
            injected_args: &[],
            category: CommandCategory::Raw,
            execution_mode: ExecutionMode::Spawn,
            context_requirement: ContextRequirement::None,
            path_intent: PathIntent::None,
            path_argument_index: None,
            project_search: false,
            risk_level: RiskLevel::Confirm,
            success_behavior: SuccessBehavior::Hide,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_preserves_file_or_directory_intent() {
        let definition = definition_for("code");
        assert_eq!(definition.path_intent, PathIntent::FileOrDirectory);
        assert!(definition.project_search);
        assert_eq!(definition.success_behavior, SuccessBehavior::Hide);
    }

    #[test]
    fn shell_builtins_and_aliases_use_explicit_internal_rules() {
        let cd = definition_for("cd");
        assert_eq!(cd.execution_mode, ExecutionMode::Internal);
        assert_eq!(cd.success_behavior, SuccessBehavior::UpdateContext);

        let ll = definition_for("ll");
        assert_eq!(ll.mapped_executable, "ls");
        assert_eq!(ll.injected_args, &["-al"]);
        assert_eq!(ll.execution_mode, ExecutionMode::Capture);
    }

    #[test]
    fn unknown_commands_are_not_in_the_trusted_catalog() {
        assert!(!is_known_command("custom-command"));
        let definition = definition_for("custom-command");
        assert_eq!(definition.category, CommandCategory::Raw);
        assert_eq!(definition.context_requirement, ContextRequirement::None);
        assert!(!definition.project_search);
    }
}
