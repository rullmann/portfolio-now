//! Symbol Validation Module
//!
//! Automatically validates and corrects quote source configurations for securities.
//! Uses a code-first approach with AI fallback for complex cases.
//!
//! ## Workflow
//!
//! 1. Check cache (pp_symbol_mapping table)
//! 2. Search provider APIs (Yahoo, TradingView, Portfolio Report, CoinGecko)
//! 3. Verify quote fetch works
//! 4. AI fallback for failed validations (if enabled)
//! 5. Store results in database

pub mod ai_fallback;
pub mod engine;
pub mod providers;
pub mod types;

pub use ai_fallback::*;
pub use engine::*;
pub use providers::*;
pub use types::*;
