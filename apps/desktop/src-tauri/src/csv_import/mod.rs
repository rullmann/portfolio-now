//! CSV Import module with broker template support.
//!
//! This module provides broker-specific templates for importing CSV files
//! from various German and international brokers.

mod templates;

pub use templates::{
    get_all_templates, get_template, detect_broker, BrokerTemplate, BrokerDetectionResult,
    BrokerTemplateSummary,
};
