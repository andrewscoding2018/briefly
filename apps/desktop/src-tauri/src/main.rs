use std::path::PathBuf;

use briefly_contracts::{DesktopImportLifecycle, DesktopImportResponse, ImportBatchStatus};

#[tauri::command]
async fn import_mailbox(path: String) -> DesktopImportResponse {
    import_mailbox_from_path(path)
}

fn import_mailbox_from_path(path: String) -> DesktopImportResponse {
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
        Ok(batch) => DesktopImportResponse {
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
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![import_mailbox])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::import_mailbox_from_path;
    use briefly_contracts::DesktopImportLifecycle;
    use std::path::PathBuf;

    #[test]
    fn imports_fixture_mailbox_successfully() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/mailbox/minimal-thread.mbox");

        let response = import_mailbox_from_path(fixture_path.display().to_string());

        assert_eq!(response.lifecycle, DesktopImportLifecycle::Completed);
        assert!(response.batch.is_some());
        assert_eq!(response.error_message, None);
    }

    #[test]
    fn rejects_unsupported_directory_input() {
        let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");

        let response = import_mailbox_from_path(fixtures_dir.display().to_string());

        assert_eq!(response.lifecycle, DesktopImportLifecycle::Failed);
        assert!(response.batch.is_none());
        assert!(response
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("not supported")));
    }
}
