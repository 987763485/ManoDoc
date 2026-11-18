mod batch;
mod chunk;
mod clean;
mod commands;
mod corpus;
mod engine;
mod models;
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
            commands::open_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running DullDoc");
}
