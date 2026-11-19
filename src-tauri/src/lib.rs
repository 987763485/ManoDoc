mod assets;
mod batch;
mod chunk;
mod clean;
mod commands;
mod corpus;
mod engine;
mod logging;
mod models;
mod preflight;
mod presets;
mod ocr;
mod security;
mod storage;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|_app| {
            // 数据目录失败不应阻塞 UI 启动；具体命令会返回可读错误。
            let _ = storage::ensure_data_dirs();
            logging::init(&storage::load_settings());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_app_data_dir,
            commands::get_engine_info,
            commands::get_ocr_status,
            commands::install_ocr_models,
            commands::inspect_file,
            commands::convert_file,
            commands::cancel_conversion,
            commands::scan_batch_inputs,
            commands::start_batch,
            commands::cancel_batch,
            commands::retry_batch_file,
            commands::list_history,
            commands::clear_history,
            commands::preflight_pdf,
            commands::get_logs_dir,
            commands::export_diagnostics,
            commands::list_presets,
            commands::save_preset,
            commands::delete_preset,
            commands::duplicate_preset,
            commands::export_presets,
            commands::import_presets,
            commands::path_kind,
            commands::open_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running DullDoc");
}
