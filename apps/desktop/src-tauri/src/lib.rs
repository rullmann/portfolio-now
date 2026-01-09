mod commands;
pub mod currency;
pub mod db;
pub mod fifo;
mod models;
pub mod pdf_import;
pub mod performance;
pub mod pp;
pub mod protobuf;
pub mod quotes;

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
            commands::file::export_database_to_portfolio,
            // Quotes
            commands::quotes::fetch_quotes,
            commands::quotes::sync_security_prices,
            commands::quotes::sync_all_prices,
            commands::quotes::fetch_historical_prices,
            commands::quotes::fetch_exchange_rates,
            commands::quotes::fetch_exchange_rate,
            commands::quotes::fetch_historical_exchange_rates,
            commands::quotes::get_available_quote_providers,
            commands::quotes::search_external_securities,
            // New PP Import commands
            commands::import::import_pp_file,
            commands::import::get_imports,
            commands::import::delete_import,
            commands::import::rebuild_fifo_lots,
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
            commands::data::get_invested_capital_history,
            // Security logo commands
            commands::data::upload_security_logo,
            commands::data::delete_security_logo,
            commands::data::get_security_logo,
            // FIFO cost basis history
            commands::data::get_fifo_cost_basis_history,
            // CRUD commands
            commands::crud::create_security,
            commands::crud::update_security,
            commands::crud::delete_security,
            commands::crud::search_securities,
            commands::crud::get_security,
            commands::crud::create_account,
            commands::crud::update_account,
            commands::crud::delete_account,
            commands::crud::create_pp_portfolio_new,
            commands::crud::update_pp_portfolio,
            commands::crud::delete_pp_portfolio,
            // Transaction CRUD
            commands::crud::create_transaction,
            commands::crud::delete_transaction,
            commands::crud::get_transaction,
            // Performance
            commands::performance::calculate_performance,
            commands::performance::get_period_returns,
            // Currency
            commands::currency::get_exchange_rate,
            commands::currency::convert_currency,
            commands::currency::get_latest_exchange_rate,
            commands::currency::get_base_currency,
            commands::currency::get_holdings_in_base_currency,
            // CSV Import/Export
            commands::csv::export_transactions_csv,
            commands::csv::export_holdings_csv,
            commands::csv::export_securities_csv,
            commands::csv::export_accounts_csv,
            commands::csv::preview_csv,
            commands::csv::import_transactions_csv,
            commands::csv::import_prices_csv,
            // Reports
            commands::reports::generate_dividend_report,
            commands::reports::generate_realized_gains_report,
            commands::reports::generate_tax_report,
            commands::reports::get_dividend_yield,
            // Taxonomy Management
            commands::taxonomy::get_taxonomies,
            commands::taxonomy::get_taxonomy,
            commands::taxonomy::create_taxonomy,
            commands::taxonomy::update_taxonomy,
            commands::taxonomy::delete_taxonomy,
            commands::taxonomy::get_classifications,
            commands::taxonomy::get_classification_tree,
            commands::taxonomy::create_classification,
            commands::taxonomy::update_classification,
            commands::taxonomy::delete_classification,
            commands::taxonomy::get_classification_assignments,
            commands::taxonomy::get_security_assignments,
            commands::taxonomy::assign_security,
            commands::taxonomy::remove_assignment,
            commands::taxonomy::get_taxonomy_allocation,
            commands::taxonomy::create_standard_taxonomies,
            // Corporate Actions
            commands::corporate_actions::preview_stock_split,
            commands::corporate_actions::apply_stock_split,
            commands::corporate_actions::undo_stock_split,
            commands::corporate_actions::apply_spin_off,
            commands::corporate_actions::get_split_history,
            commands::corporate_actions::get_split_adjusted_price,
            // Watchlist Management
            commands::watchlist::get_watchlists,
            commands::watchlist::get_watchlist,
            commands::watchlist::create_watchlist,
            commands::watchlist::rename_watchlist,
            commands::watchlist::delete_watchlist,
            commands::watchlist::get_watchlist_securities,
            commands::watchlist::add_to_watchlist,
            commands::watchlist::remove_from_watchlist,
            commands::watchlist::add_securities_to_watchlist,
            commands::watchlist::get_watchlists_for_security,
            // PDF Import
            commands::pdf_import::get_supported_banks,
            commands::pdf_import::preview_pdf_import,
            commands::pdf_import::import_pdf_transactions,
            commands::pdf_import::extract_pdf_raw_text,
            commands::pdf_import::parse_pdf_text,
            commands::pdf_import::detect_pdf_bank,
            // Investment Plans
            commands::investment_plans::get_investment_plans,
            commands::investment_plans::get_investment_plan,
            commands::investment_plans::create_investment_plan,
            commands::investment_plans::update_investment_plan,
            commands::investment_plans::delete_investment_plan,
            commands::investment_plans::get_investment_plan_executions,
            commands::investment_plans::execute_investment_plan,
            commands::investment_plans::get_plans_due_for_execution,
            // Rebalancing
            commands::rebalancing::preview_rebalance,
            commands::rebalancing::execute_rebalance,
            commands::rebalancing::calculate_deviation,
            commands::rebalancing::suggest_rebalance_by_taxonomy,
            // Benchmark
            commands::benchmark::get_benchmarks,
            commands::benchmark::add_benchmark,
            commands::benchmark::remove_benchmark,
            commands::benchmark::compare_to_benchmark,
            commands::benchmark::get_benchmark_comparison_data,
            // Brandfetch (Logo API)
            commands::brandfetch::fetch_security_logo,
            commands::brandfetch::get_cached_logo,
            commands::brandfetch::clear_logo_cache,
            commands::brandfetch::fetch_logos_batch,
            commands::brandfetch::reload_all_logos,
            commands::brandfetch::is_logo_cached,
            commands::brandfetch::get_cached_logo_data,
            commands::brandfetch::save_logo_to_cache,
            // PDF Export
            commands::pdf_export::export_portfolio_summary_pdf,
            commands::pdf_export::export_holdings_pdf,
            commands::pdf_export::export_performance_pdf,
            commands::pdf_export::export_dividend_pdf,
            commands::pdf_export::export_tax_report_pdf,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
