#[cfg(any(target_os = "macos", test))]
use std::path::PathBuf;

#[cfg(any(target_os = "macos", test))]
use briefly_contracts::{
    DesktopImportLifecycle, DesktopImportResponse, FocusDashboardResponse, ImportBatchOutput,
    ImportBatchStatus,
};

#[cfg(target_os = "macos")]
#[tauri::command]
async fn import_mailbox(path: String) -> DesktopImportResponse {
    import_mailbox_from_path(path)
}

#[cfg(target_os = "macos")]
#[tauri::command]
async fn load_focus_dashboard() -> Result<FocusDashboardResponse, String> {
    load_focus_dashboard_from_default_store().map_err(|error| error.to_string())
}

#[cfg(any(target_os = "macos", test))]
fn import_mailbox_from_path(path: String) -> DesktopImportResponse {
    let store_path = default_store_path();
    import_mailbox_from_path_with_store(path, &store_path)
}

#[cfg(any(target_os = "macos", test))]
fn import_mailbox_from_path_with_store(
    path: String,
    store_path: &PathBuf,
) -> DesktopImportResponse {
    let trimmed_path = path.trim().to_string();

    if trimmed_path.is_empty() {
        return DesktopImportResponse {
            lifecycle: DesktopImportLifecycle::Failed,
            selected_path: None,
            batch: None,
            error_message: Some("Select a local .mbox file before starting import.".to_string()),
        };
    }

    let path_buf = PathBuf::from(&trimmed_path);

    match briefly_ingest::import_mbox_fixture(&path_buf) {
        Ok(batch) => match persist_and_score_batch(store_path, &batch) {
            Ok(()) => DesktopImportResponse {
                lifecycle: match batch.status {
                    ImportBatchStatus::Completed => DesktopImportLifecycle::Completed,
                    ImportBatchStatus::Partial => DesktopImportLifecycle::Partial,
                    ImportBatchStatus::Failed => DesktopImportLifecycle::Failed,
                },
                selected_path: Some(trimmed_path),
                batch: Some(batch),
                error_message: None,
            },
            Err(error) => DesktopImportResponse {
                lifecycle: DesktopImportLifecycle::Failed,
                selected_path: Some(trimmed_path),
                batch: None,
                error_message: Some(error.to_string()),
            },
        },
        Err(error) => DesktopImportResponse {
            lifecycle: DesktopImportLifecycle::Failed,
            selected_path: Some(trimmed_path),
            batch: None,
            error_message: Some(error.to_string()),
        },
    }
}

#[cfg(any(target_os = "macos", test))]
fn persist_and_score_batch(
    store_path: &PathBuf,
    batch: &ImportBatchOutput,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut store = briefly_store::Store::open_path(store_path)?;
    let report = store.persist_import_batch(batch)?;
    briefly_score::run_scoring(&mut store, Some(report.import_batch_id.as_str()))?;
    Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn load_focus_dashboard_from_default_store(
) -> Result<FocusDashboardResponse, Box<dyn std::error::Error>> {
    let store_path = default_store_path();
    load_focus_dashboard_from_store(&store_path)
}

#[cfg(any(target_os = "macos", test))]
fn load_focus_dashboard_from_store(
    store_path: &PathBuf,
) -> Result<FocusDashboardResponse, Box<dyn std::error::Error>> {
    let store = briefly_store::Store::open_path(store_path)?;
    Ok(briefly_score::load_focus_dashboard(&store)?)
}

#[cfg(any(target_os = "macos", test))]
fn default_store_path() -> PathBuf {
    if let Ok(path) = std::env::var("BRIEFLY_STORE_PATH") {
        return PathBuf::from(path);
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let context_dir = cwd.join(".context");
    let _ = std::fs::create_dir_all(&context_dir);
    context_dir.join("briefly.sqlite3")
}

#[cfg(target_os = "macos")]
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            import_mailbox,
            load_focus_dashboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("Briefly desktop shell is only supported on macOS in Phase 1.");
}

#[cfg(test)]
mod tests {
    use super::{
        default_store_path, import_mailbox_from_path, import_mailbox_from_path_with_store,
        load_focus_dashboard_from_store,
    };
    use briefly_contracts::DesktopImportLifecycle;
    use std::path::PathBuf;

    fn temp_store_path(name: &str) -> String {
        let path =
            std::env::temp_dir().join(format!("briefly-{name}-{}.sqlite3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path.display().to_string()
    }

    #[test]
    fn imports_fixture_mailbox_successfully() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/mailbox/minimal-thread.mbox");
        let store_path = PathBuf::from(temp_store_path("import-success"));

        let response =
            import_mailbox_from_path_with_store(fixture_path.display().to_string(), &store_path);

        assert_eq!(response.lifecycle, DesktopImportLifecycle::Completed);
        assert!(response.batch.is_some());
        assert_eq!(response.error_message, None);

        let dashboard =
            load_focus_dashboard_from_store(&store_path).expect("dashboard should load");
        assert!(dashboard.has_imported_mailbox);
        assert_eq!(dashboard.threads.len(), 1);
    }

    #[test]
    fn rejects_unsupported_directory_input() {
        let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
        let store_path = temp_store_path("reject-dir");
        unsafe {
            std::env::set_var("BRIEFLY_STORE_PATH", &store_path);
        }

        let response = import_mailbox_from_path(fixtures_dir.display().to_string());

        assert_eq!(response.lifecycle, DesktopImportLifecycle::Failed);
        assert!(response.batch.is_none());
        assert!(response
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("not supported")));
    }

    #[test]
    fn default_store_path_prefers_context_directory() {
        unsafe {
            std::env::remove_var("BRIEFLY_STORE_PATH");
        }

        let path = default_store_path();
        assert!(path.ends_with(".context/briefly.sqlite3"));
    }
}
