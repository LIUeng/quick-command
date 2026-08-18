use crate::{
    commands,
    models::{AppData, DirectoryRecord, Settings, Workspace},
    store::Store,
};
use serde_json::{json, Value};
use std::{fs, path::Path, sync::Mutex};
use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY},
    webview::InvokeRequest,
    App, WebviewWindow, WebviewWindowBuilder,
};
use uuid::Uuid;

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("quick-command-flow-{name}-{}", Uuid::new_v4()))
}

fn fixture_data(root: &Path) -> AppData {
    AppData {
        settings: Settings {
            shortcut: "Command+Space".into(),
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

fn build_test_app(root: &Path, data: AppData) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let store = Store {
        path: root.join("state.json"),
        data: Mutex::new(data),
    };
    let app = mock_builder()
        .manage(store)
        .invoke_handler(tauri::generate_handler![
            commands::get_launcher_state,
            commands::search_projects,
            commands::execute_command,
            commands::confirm_operation,
            commands::set_active_context,
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    (app, webview)
}

fn invoke(
    webview: &WebviewWindow<MockRuntime>,
    command: &str,
    body: Value,
) -> Result<Value, Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: command.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.into(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

#[test]
fn search_flow_crosses_the_tauri_boundary_and_rejects_unknown_commands() {
    let root = fixture_root("search");
    let project = root.join("example");
    fs::create_dir_all(&project).unwrap();
    let mut data = fixture_data(&root);
    data.directories.push(DirectoryRecord {
        path: project.to_string_lossy().into_owned(),
        name: "example".into(),
        use_count: 2,
        last_used_at: Some(10),
    });
    let (_app, webview) = build_test_app(&root, data);

    let response = invoke(
        &webview,
        "search_projects",
        json!({ "query": "code example" }),
    )
    .unwrap();
    assert_eq!(response["executable"], "code");
    assert_eq!(
        response["results"][0]["path"],
        project.to_string_lossy().as_ref()
    );
    assert_eq!(response["actions"][0]["kind"], "open-file");

    let error = invoke(
        &webview,
        "search_projects",
        json!({ "query": "custom-command value" }),
    )
    .unwrap_err();
    assert!(error
        .as_str()
        .is_some_and(|message| message.contains("只执行已登记的安全命令")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn presentation_and_navigation_flows_persist_across_reload() {
    let root = fixture_root("context");
    let child = root.join("child");
    fs::create_dir_all(&child).unwrap();
    fs::write(root.join("README.md"), "hello").unwrap();
    let state_path = root.join("state.json");
    let (app, webview) = build_test_app(&root, fixture_data(&root));

    let presented = invoke(
        &webview,
        "execute_command",
        json!({ "query": "ls", "targetPath": null }),
    )
    .unwrap();
    assert_eq!(presented["kind"], "presented");
    assert_eq!(presented["output"]["type"], "directory");
    assert!(presented["output"]["entries"]
        .as_array()
        .is_some_and(|entries| entries.iter().any(|entry| entry["name"] == "README.md")));

    let context = invoke(
        &webview,
        "execute_command",
        json!({ "query": "cd child", "targetPath": null }),
    )
    .unwrap();
    assert_eq!(context["kind"], "context-updated");
    assert_eq!(
        context["path"],
        child.canonicalize().unwrap().to_string_lossy().as_ref()
    );

    drop(webview);
    drop(app);
    let reloaded = Store::load(state_path).unwrap();
    let data = reloaded.data.lock().unwrap();
    assert_eq!(
        data.active_context.as_deref(),
        Some(child.canonicalize().unwrap().to_string_lossy().as_ref())
    );
    assert_eq!(data.history.len(), 2);
    assert_eq!(
        data.history[0].action.as_ref().map(|action| action.kind),
        Some(crate::models::HistoryActionKind::ChangeContext)
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_context_can_be_selected_and_the_original_flow_retried() {
    let root = fixture_root("context-selection");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("README.md"), "hello").unwrap();
    let mut data = fixture_data(&root);
    data.active_context = None;
    let (app, webview) = build_test_app(&root, data);

    let needs_context = invoke(
        &webview,
        "execute_command",
        json!({ "query": "ls", "targetPath": null }),
    )
    .unwrap();
    assert_eq!(needs_context["kind"], "needs-context");

    let selected = invoke(
        &webview,
        "set_active_context",
        json!({ "path": root.to_string_lossy() }),
    )
    .unwrap();
    assert_eq!(
        selected["activeContext"],
        root.canonicalize().unwrap().to_string_lossy().as_ref()
    );

    let retried = invoke(
        &webview,
        "execute_command",
        json!({ "query": "ls", "targetPath": null }),
    )
    .unwrap();
    assert_eq!(retried["kind"], "presented");

    drop(webview);
    drop(app);
    let reloaded = Store::load(root.join("state.json")).unwrap();
    assert_eq!(
        reloaded.data.lock().unwrap().active_context.as_deref(),
        Some(root.canonicalize().unwrap().to_string_lossy().as_ref())
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn confirmed_mkdir_flow_creates_and_persists_only_after_confirmation() {
    let root = fixture_root("mkdir");
    fs::create_dir(&root).unwrap();
    let state_path = root.join("state.json");
    let (app, webview) = build_test_app(&root, fixture_data(&root));

    let preview = invoke(
        &webview,
        "execute_command",
        json!({ "query": "mkdir example", "targetPath": null }),
    )
    .unwrap();
    assert_eq!(preview["kind"], "confirmation");
    assert!(!root.join("example").exists());

    let confirmation = &preview["confirmation"];
    let completed = invoke(
        &webview,
        "confirm_operation",
        json!({
            "query": "mkdir example",
            "operationKind": confirmation["kind"],
            "targetPath": confirmation["targetPath"],
            "workspacePath": confirmation["workspacePath"]
        }),
    )
    .unwrap();
    assert_eq!(completed["kind"], "operation-completed");
    assert!(root.join("example").is_dir());

    drop(webview);
    drop(app);
    let reloaded = Store::load(state_path).unwrap();
    let data = reloaded.data.lock().unwrap();
    let expected_target = root.canonicalize().unwrap().join("example");
    assert!(data
        .directories
        .iter()
        .any(|directory| directory.path == expected_target.to_string_lossy()));
    assert_eq!(
        data.history[0].action.as_ref().map(|action| action.kind),
        Some(crate::models::HistoryActionKind::CreateDirectory)
    );

    fs::remove_dir_all(root).unwrap();
}
