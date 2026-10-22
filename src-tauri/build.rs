fn main() {
    // Listing every command opts the app into Tauri's deny-by-default command ACL.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "open_calendar", "calendar_notes", "calendar_day_notes", "show_calendar_note",
            "calendar_pin", "set_calendar_pin", "get_note", "window_ready", "drag_note",
            "create_note", "update_note", "save_note_content", "delete_note", "hide_note", "set_pin_mode",
            "open_settings", "design_context", "design_note_summaries", "get_note_design", "get_settings", "list_fonts", "save_settings",
            "set_shortcut_recording", "get_sync_status", "sync_now", "has_gdrive_secret",
            "gdrive_connect", "gdrive_disconnect", "suggest_folder", "app_info", "open_data_dir",
            "export_archive", "import_archive", "editor_ready", "get_transfer_preferences",
            "save_transfer_preferences", "fit_settings", "get_mcp_settings", "provision_mcp",
            "set_mcp_enabled",
            "mcp_client_status", "connect_mcp_clients",
        ]),
    )).expect("Tauri build failed");
}
