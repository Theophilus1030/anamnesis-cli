#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;

use db::Database;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let db = Database::new(app.handle()).expect("failed to initialize database");
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_projects,
            commands::create_project,
            commands::delete_project,
            commands::get_pages,
            commands::create_page,
            commands::delete_page,
            commands::get_page_meta,
            commands::save_page_meta,
            commands::save_alto,
            commands::get_alto,
            commands::save_sentence,
            commands::get_sentence,
            commands::run_kraken_ocr,
            commands::check_default_models,
            commands::update_page_name,
            commands::update_project_name
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
