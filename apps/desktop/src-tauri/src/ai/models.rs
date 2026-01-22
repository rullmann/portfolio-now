//! Centralized registry of vision-capable AI models.
//!
//! This is the single source of truth for all AI models that support image/vision input.
//! Updated: January 2026

use serde::Serialize;

// ============================================================================
// Types
// ============================================================================

/// A vision-capable AI model
#[derive(Debug, Clone, Serialize)]
pub struct VisionModel {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub provider: &'static str,
}

/// Model info for frontend (owned strings for serialization)
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl From<&VisionModel> for ModelInfo {
    fn from(m: &VisionModel) -> Self {
        ModelInfo {
            id: m.id.to_string(),
            name: m.name.to_string(),
            description: m.description.to_string(),
        }
    }
}

// ============================================================================
// Vision Models Registry (January 2026)
// ============================================================================

/// All vision-capable models across all providers
///
/// IMPORTANT: Only include models with confirmed vision/image input support!
/// - Claude Opus 4.5 has NO vision in API (as of Jan 2026)
/// - OpenAI o-series (o1, o3, o4) have NO vision
/// - GPT-4.1 is coding-focused, no vision
pub const VISION_MODELS: &[VisionModel] = &[
    // -------------------------------------------------------------------------
    // Claude (Anthropic) - https://docs.anthropic.com/en/docs/about-claude/models
    // Note: Opus 4.5 does NOT have vision support in API!
    // -------------------------------------------------------------------------
    VisionModel {
        id: "claude-sonnet-4-5-20250514",
        name: "Claude Sonnet 4.5",
        description: "Beste Qualität mit Vision",
        provider: "claude",
    },
    VisionModel {
        id: "claude-haiku-4-5-20251015",
        name: "Claude Haiku 4.5",
        description: "Schnell & günstig",
        provider: "claude",
    },

    // -------------------------------------------------------------------------
    // OpenAI - https://platform.openai.com/docs/models
    // o3/o4-mini have vision + web search, GPT-4.1/4o have vision only
    // -------------------------------------------------------------------------
    VisionModel {
        id: "o3",
        name: "o3",
        description: "Smartest, Vision + Web-Suche",
        provider: "openai",
    },
    VisionModel {
        id: "o4-mini",
        name: "o4-mini",
        description: "Schnell, Vision + Web-Suche",
        provider: "openai",
    },
    VisionModel {
        id: "gpt-5-mini",
        name: "GPT-5 Mini",
        description: "Neuestes GPT-5, schnell & günstig",
        provider: "openai",
    },
    VisionModel {
        id: "gpt-4.1",
        name: "GPT-4.1",
        description: "Coding-fokussiert, 1M Kontext",
        provider: "openai",
    },
    VisionModel {
        id: "gpt-4o",
        name: "GPT-4o",
        description: "Flagship Multimodal",
        provider: "openai",
    },
    VisionModel {
        id: "gpt-4o-mini",
        name: "GPT-4o Mini",
        description: "Schnell & günstig",
        provider: "openai",
    },

    // -------------------------------------------------------------------------
    // Google Gemini - https://ai.google.dev/gemini-api/docs/models
    // -------------------------------------------------------------------------
    VisionModel {
        id: "gemini-2.5-flash",
        name: "Gemini 2.5 Flash",
        description: "Schnell & günstig, Free Tier",
        provider: "gemini",
    },
    VisionModel {
        id: "gemini-2.5-pro",
        name: "Gemini 2.5 Pro",
        description: "Beste Qualität (stabil)",
        provider: "gemini",
    },
    VisionModel {
        id: "gemini-3-flash-preview",
        name: "Gemini 3 Flash",
        description: "Neuestes Modell (Preview)",
        provider: "gemini",
    },
    VisionModel {
        id: "gemini-3-pro-preview",
        name: "Gemini 3 Pro",
        description: "Neuestes Pro (Preview)",
        provider: "gemini",
    },

    // -------------------------------------------------------------------------
    // Perplexity - https://docs.perplexity.ai/guides/model-cards
    // -------------------------------------------------------------------------
    VisionModel {
        id: "sonar-pro",
        name: "Sonar Pro",
        description: "Vision + Web-Suche",
        provider: "perplexity",
    },
    VisionModel {
        id: "sonar",
        name: "Sonar",
        description: "Schnell + Web-Suche",
        provider: "perplexity",
    },
];

// ============================================================================
// Deprecated Model Mappings
// ============================================================================

/// Maps deprecated/non-vision models to their vision-capable replacements
pub const DEPRECATED_MODELS: &[(&str, &str)] = &[
    // Claude - Opus has no vision, map to Sonnet
    ("claude-opus-4-5-20251101", "claude-sonnet-4-5-20250514"),
    ("claude-3-opus-20240229", "claude-sonnet-4-5-20250514"),
    ("claude-3-sonnet-20240229", "claude-sonnet-4-5-20250514"),
    ("claude-3-haiku-20240307", "claude-haiku-4-5-20251015"),
    ("claude-3-5-sonnet-20241022", "claude-sonnet-4-5-20250514"),
    ("claude-3-5-haiku-20241022", "claude-haiku-4-5-20251015"),
    ("claude-3-7-sonnet", "claude-sonnet-4-5-20250514"),
    ("claude-2.1", "claude-sonnet-4-5-20250514"),

    // OpenAI - o-series has no vision, old models deprecated
    ("o3", "gpt-4.1"),
    ("o3-pro", "gpt-4.1"),
    ("o4-mini", "gpt-4o-mini"),
    ("o1", "gpt-4o"),
    ("o1-preview", "gpt-4o"),
    ("o1-mini", "gpt-4o-mini"),
    ("gpt-4-vision-preview", "gpt-4o"),
    ("gpt-4-turbo", "gpt-4.1"),
    ("gpt-4-turbo-preview", "gpt-4o"),

    // Gemini - old/invalid model names
    ("gemini-pro-vision", "gemini-2.5-flash"),
    ("gemini-2.0-flash", "gemini-2.5-flash"),
    ("gemini-2.0-flash-exp", "gemini-2.5-flash"),
    ("gemini-1.5-pro", "gemini-2.5-pro"),
    ("gemini-1.5-flash", "gemini-2.5-flash"),
    // Invalid names (without -preview suffix)
    ("gemini-3-flash", "gemini-2.5-flash"),
    ("gemini-3-pro", "gemini-2.5-pro"),

    // Perplexity - reasoning models have no vision
    ("sonar-reasoning", "sonar-pro"),
    ("sonar-reasoning-pro", "sonar-pro"),
    ("sonar-deep-research", "sonar-pro"),
];

// ============================================================================
// Fallback Chains
// ============================================================================

/// Fallback chain per provider (in order of preference)
pub const FALLBACK_CHAINS: &[(&str, &[&str])] = &[
    ("claude", &["claude-sonnet-4-5-20250514", "claude-haiku-4-5-20251015"]),
    ("openai", &["gpt-5-mini", "gpt-4.1", "gpt-4o", "gpt-4o-mini"]),
    ("gemini", &["gemini-2.5-flash", "gemini-2.5-pro", "gemini-3-flash-preview", "gemini-3-pro-preview"]),
    ("perplexity", &["sonar-pro", "sonar"]),
];

// ============================================================================
// Default Models
// ============================================================================

/// Default model per provider
pub const DEFAULT_MODELS: &[(&str, &str)] = &[
    ("claude", "claude-sonnet-4-5-20250514"),
    ("openai", "gpt-5-mini"),
    ("gemini", "gemini-2.5-flash"),
    ("perplexity", "sonar-pro"),
];

// ============================================================================
// Helper Functions
// ============================================================================

/// Get all vision-capable models for a provider
pub fn get_models_for_provider(provider: &str) -> Vec<&'static VisionModel> {
    VISION_MODELS
        .iter()
        .filter(|m| m.provider.eq_ignore_ascii_case(provider))
        .collect()
}

/// Get upgraded model if the given model is deprecated
pub fn get_model_upgrade(model: &str) -> Option<&'static str> {
    DEPRECATED_MODELS
        .iter()
        .find(|(old, _)| *old == model)
        .map(|(_, new)| *new)
}

/// Get fallback model for a provider when current model fails
pub fn get_fallback(provider: &str, current_model: &str) -> Option<&'static str> {
    let chain = FALLBACK_CHAINS
        .iter()
        .find(|(p, _)| p.eq_ignore_ascii_case(provider))
        .map(|(_, c)| *c)?;

    let current_idx = chain.iter().position(|m| *m == current_model)?;
    chain.get(current_idx + 1).copied()
}

/// Get default model for a provider
pub fn get_default(provider: &str) -> &'static str {
    DEFAULT_MODELS
        .iter()
        .find(|(p, _)| p.eq_ignore_ascii_case(provider))
        .map(|(_, m)| *m)
        .unwrap_or("claude-sonnet-4-5-20250514")
}

/// Check if a model ID is valid (exists in registry)
pub fn is_valid_model(model: &str) -> bool {
    VISION_MODELS.iter().any(|m| m.id == model)
}

/// Check if a model has vision/image input support
///
/// Returns true if the model is in the VISION_MODELS registry.
/// This is the same as `is_valid_model` but with a more semantic name
/// for use cases where you need to check image support specifically.
pub fn has_vision_support(model: &str) -> bool {
    VISION_MODELS.iter().any(|m| m.id == model)
}

/// Get the provider for a model ID
pub fn get_model_provider(model: &str) -> Option<&'static str> {
    VISION_MODELS
        .iter()
        .find(|m| m.id == model)
        .map(|m| m.provider)
}

/// Get a model by ID
pub fn get_model(model_id: &str) -> Option<&'static VisionModel> {
    VISION_MODELS.iter().find(|m| m.id == model_id)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_models_for_provider() {
        let claude_models = get_models_for_provider("claude");
        assert_eq!(claude_models.len(), 2);
        assert!(claude_models.iter().all(|m| m.provider == "claude"));
    }

    #[test]
    fn test_deprecated_model_upgrade() {
        assert_eq!(
            get_model_upgrade("claude-opus-4-5-20251101"),
            Some("claude-sonnet-4-5-20250514")
        );
        assert_eq!(get_model_upgrade("o1"), Some("gpt-4o"));
        assert_eq!(get_model_upgrade("nonexistent"), None);
    }

    #[test]
    fn test_fallback_chain() {
        assert_eq!(
            get_fallback("openai", "gpt-4.1"),
            Some("gpt-4o")
        );
        assert_eq!(
            get_fallback("openai", "gpt-4o"),
            Some("gpt-4o-mini")
        );
        assert_eq!(get_fallback("openai", "gpt-4o-mini"), None);
    }

    #[test]
    fn test_default_models() {
        assert_eq!(get_default("claude"), "claude-sonnet-4-5-20250514");
        assert_eq!(get_default("openai"), "gpt-5-mini");
        assert_eq!(get_default("gemini"), "gemini-2.5-flash");
    }

    #[test]
    fn test_is_valid_model() {
        assert!(is_valid_model("gpt-4.1"));
        assert!(is_valid_model("claude-sonnet-4-5-20250514"));
        assert!(!is_valid_model("o1")); // deprecated (no vision)
        assert!(!is_valid_model("nonexistent"));
    }
}
