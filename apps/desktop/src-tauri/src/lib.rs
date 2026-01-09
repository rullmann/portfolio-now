mod commands;
mod db;
pub mod fifo;
mod models;
pub mod pp;
pub mod protobuf;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize database
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("portfolio.db");
            db::init_database(&db_path)?;

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Legacy portfolio commands
            commands::portfolio::get_portfolios,
            commands::portfolio::create_portfolio,
            commands::portfolio::update_portfolio,
            commands::portfolio::delete_portfolio,
            // File commands
            commands::file::create_new_portfolio,
            commands::file::open_portfolio_file,
            commands::file::save_portfolio_file,
            commands::file::get_file_extension,
            commands::file::validate_portfolio_file,
            commands::file::get_portfolio_stats,
            // Quotes
            commands::quotes::fetch_quotes,
            // New PP Import commands
            commands::import::import_pp_file,
            commands::import::get_imports,
            commands::import::delete_import,
            // New PP Data query commands
            commands::data::get_securities,
            commands::data::get_accounts,
            commands::data::get_pp_portfolios,
            commands::data::get_transactions,
            commands::data::get_price_history,
            commands::data::get_holdings,
            commands::data::get_all_holdings,
            commands::data::get_portfolio_summary,
            commands::data::get_portfolio_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
