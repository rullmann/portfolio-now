//! Brandfetch Logo API integration for fetching company and ETF provider logos.
//!
//! Uses intelligent domain detection:
//! 1. ETF provider detection from name keywords
//! 2. Domain derivation from company name (heuristics)
//! 3. Static mappings for known exceptions
//!
//! URL format: https://cdn.brandfetch.io/{domain}/h/64/w/64/icon?c={client_id}

use base64::Engine;
use serde::Serialize;
use tauri::command;

/// ETF provider domain mappings (detected by keyword in name)
const ETF_PROVIDERS: &[(&str, &str)] = &[
    ("ishares", "blackrock.com"),
    ("blackrock", "blackrock.com"),
    ("vanguard", "vanguard.com"),
    ("spdr", "ssga.com"),
    ("state street", "ssga.com"),
    ("invesco", "invesco.com"),
    ("xtrackers", "dws.com"),
    ("dws", "dws.com"),
    ("lyxor", "amundi.com"),
    ("amundi", "amundi.com"),
    ("wisdomtree", "wisdomtree.com"),
    ("vaneck", "vaneck.com"),
    ("fidelity", "fidelity.com"),
    ("schwab", "schwab.com"),
    ("ark invest", "ark-invest.com"),
    ("proshares", "proshares.com"),
    ("global x", "globalxetfs.com"),
    ("first trust", "ftportfolios.com"),
    ("jpmorgan", "jpmorgan.com"),
    ("goldman sachs", "gsam.com"),
    ("ubs etf", "ubs.com"),
    ("hsbc", "hsbc.com"),
    ("comstage", "lyxor.com"),
    ("ossiam", "ossiam.com"),
    ("franklin", "franklintempleton.com"),
    ("bnp paribas", "bnpparibas.com"),
];

/// Known exceptions where company name doesn't match domain
const DOMAIN_EXCEPTIONS: &[(&str, &str)] = &[
    // Tech companies with different domains
    ("alphabet", "google.com"),
    ("meta platforms", "meta.com"),
    ("booking holdings", "booking.com"),
    // German companies
    ("deutsche telekom", "telekom.com"),
    ("deutsche post", "dhl.com"),
    ("deutsche bank", "db.com"),
    ("deutsche boerse", "deutsche-boerse.com"),
    ("muenchener rueck", "munichre.com"),
    ("munich re", "munichre.com"),
    // Holding companies
    ("berkshire hathaway", "berkshirehathaway.com"),
    // Abbreviated names
    ("lvmh", "lvmh.com"),
    ("basf", "basf.com"),
    ("bmw", "bmw.com"),
    ("sap", "sap.com"),
    ("asml", "asml.com"),
    // Companies with non-obvious domains
    ("procter & gamble", "pg.com"),
    ("procter gamble", "pg.com"),
    ("johnson & johnson", "jnj.com"),
    ("johnson johnson", "jnj.com"),
    ("coca-cola", "coca-cola.com"),
    ("coca cola", "coca-cola.com"),
    ("jpmorgan", "jpmorgan.com"),
    ("jp morgan", "jpmorgan.com"),
    ("goldman sachs", "goldmansachs.com"),
    ("morgan stanley", "morganstanley.com"),
    ("bank of america", "bankofamerica.com"),
    ("american express", "americanexpress.com"),
    ("wells fargo", "wellsfargo.com"),
    ("united health", "unitedhealthgroup.com"),
    ("unitedhealth", "unitedhealthgroup.com"),
    ("home depot", "homedepot.com"),
    ("estee lauder", "esteelauder.com"),
    ("general mills", "generalmills.com"),
    ("general electric", "ge.com"),
    ("general motors", "gm.com"),
    ("exxon mobil", "exxonmobil.com"),
    ("exxonmobil", "exxonmobil.com"),
    ("conocophillips", "conocophillips.com"),
    ("lockheed martin", "lockheedmartin.com"),
    ("raytheon", "rtx.com"),
    ("3m", "3m.com"),
    ("at&t", "att.com"),
    // Swiss companies
    ("nestle", "nestle.com"),
    ("nestlé", "nestle.com"),
    ("roche", "roche.com"),
    ("novartis", "novartis.com"),
    ("ubs group", "ubs.com"),
    ("credit suisse", "credit-suisse.com"),
    ("swiss re", "swissre.com"),
    ("zurich insurance", "zurich.com"),
    // French companies
    ("l'oreal", "loreal.com"),
    ("l'oréal", "loreal.com"),
    ("loreal", "loreal.com"),
    ("bnp paribas", "bnpparibas.com"),
    ("credit agricole", "credit-agricole.com"),
    ("societe generale", "societegenerale.com"),
    ("air liquide", "airliquide.com"),
    ("saint-gobain", "saint-gobain.com"),
    ("pernod ricard", "pernod-ricard.com"),
    ("schneider electric", "schneider-electric.com"),
    ("totalenergies", "totalenergies.com"),
    ("total energies", "totalenergies.com"),
    // Spanish companies
    ("banco santander", "santander.com"),
    // UK companies
    ("lloyds banking", "lloydsbank.com"),
    ("royal dutch shell", "shell.com"),
    ("astrazeneca", "astrazeneca.com"),
    ("glaxosmithkline", "gsk.com"),
    ("rolls-royce", "rolls-royce.com"),
    ("rolls royce", "rolls-royce.com"),
    ("british american tobacco", "bat.com"),
    // Japanese
    ("softbank", "softbank.com"),
    ("toyota motor", "toyota.com"),
    // Chinese
    ("alibaba", "alibaba.com"),
    ("tencent", "tencent.com"),
    ("baidu", "baidu.com"),
    // Others
    ("taiwan semiconductor", "tsmc.com"),
    ("novo nordisk", "novonordisk.com"),
];

/// Common company name suffixes to remove
const NAME_SUFFIXES: &[&str] = &[
    " incorporated",
    " inc.",
    " inc",
    " corporation",
    " corp.",
    " corp",
    " limited",
    " ltd.",
    " ltd",
    " plc",
    " ag",
    " se",
    " sa",
    " nv",
    " n.v.",
    " co.",
    " & co",
    " co",
    " gmbh",
    " kg",
    " kgaa",
    " s.a.",
    " s.p.a.",
    " spa",
    " oyj",
    " ab",
    " asa",
    " a/s",
    " holding",
    " holdings",
    " group",
    " international",
    " intl",
    " company",
    " companies",
    // Stock class indicators
    " class a",
    " class b",
    " class c",
    " cl a",
    " cl b",
    " cl.a",
    " cl.b",
    " -a",
    " -b",
    " a",
    " b",
    // German specific
    " namens-aktien",
    " nam.-akt.",
    " inhaber-aktien",
    " inh.-akt.",
    " vorzugsaktien",
    " vz.",
    " st.",
    " stammaktien",
    // Common additions
    " registered",
    " reg.",
    " common stock",
    " common",
    " ord.",
    " ordinary",
    " shares",
    " share",
    " adr",
    " ads",
];

/// Result returned to frontend
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoResult {
    pub security_id: i64,
    pub logo_url: Option<String>,
    pub domain: Option<String>,
}

/// Detect if security is an ETF based on name and return provider domain
fn detect_etf_provider(name: &str) -> Option<&'static str> {
    let name_lower = name.to_lowercase();

    // Check for ETF provider keywords
    for (keyword, domain) in ETF_PROVIDERS {
        if name_lower.contains(keyword) {
            return Some(domain);
        }
    }

    // Check for common ETF patterns with provider at start
    if name_lower.contains("etf") || name_lower.contains("ucits") || name_lower.contains("index") {
        for (keyword, domain) in ETF_PROVIDERS {
            if name_lower.starts_with(keyword) {
                return Some(domain);
            }
        }
    }

    None
}

/// Check for known domain exceptions
fn check_domain_exception(name: &str) -> Option<&'static str> {
    let name_lower = name.to_lowercase();

    for (keyword, domain) in DOMAIN_EXCEPTIONS {
        if name_lower.contains(keyword) {
            return Some(domain);
        }
    }

    None
}

/// Clean company name by removing common suffixes and noise
fn clean_company_name(name: &str) -> String {
    let mut cleaned = name.to_lowercase();

    // Remove stock exchange suffixes like .DE, .VI, etc.
    if let Some(pos) = cleaned.rfind('.') {
        let suffix = &cleaned[pos..];
        if suffix.len() <= 4 && suffix.chars().skip(1).all(|c| c.is_alphabetic()) {
            cleaned = cleaned[..pos].to_string();
        }
    }

    // Remove German stock patterns like "DL-,001", "EO 0,3", "O.N.", "NSY"
    // These often appear after the company name
    let german_patterns = [
        " dl-", " dl ", " eo ", " o.n.", " nsy", " eo0", " dl0",
        " on ", " inh", " nam", " vink", " reg.shs", " shs ",
    ];
    for pattern in german_patterns {
        if let Some(pos) = cleaned.find(pattern) {
            cleaned = cleaned[..pos].to_string();
        }
    }

    // Remove common suffixes (longest first for proper matching)
    let mut suffixes: Vec<&str> = NAME_SUFFIXES.to_vec();
    suffixes.sort_by(|a, b| b.len().cmp(&a.len()));

    for suffix in &suffixes {
        if cleaned.ends_with(suffix) {
            cleaned = cleaned[..cleaned.len() - suffix.len()].trim().to_string();
        }
    }

    // Remove trailing patterns with numbers/special chars
    // E.g., "nvidia corp. dl" -> remove " dl"
    let trailing_patterns = [" dl", " eo", " cl", " ns", " on"];
    for pattern in trailing_patterns {
        if cleaned.ends_with(pattern) {
            cleaned = cleaned[..cleaned.len() - pattern.len()].trim().to_string();
        }
    }

    // Apply suffix removal again after pattern cleanup
    for suffix in &suffixes {
        if cleaned.ends_with(suffix) {
            cleaned = cleaned[..cleaned.len() - suffix.len()].trim().to_string();
        }
    }

    // Remove percentage patterns like "0,0 %"
    if cleaned.contains('%') {
        if let Some(pos) = cleaned.find('%') {
            let before_percent = &cleaned[..pos];
            if let Some(start) = before_percent.rfind(|c: char| !c.is_numeric() && c != ',' && c != '.' && c != ' ') {
                cleaned = cleaned[..=start].trim().to_string();
            }
        }
    }

    // Final cleanup: remove any remaining trailing punctuation
    cleaned = cleaned.trim_end_matches(|c: char| c == '.' || c == ',' || c == '-' || c == ' ').to_string();

    cleaned.trim().to_string()
}

/// Derive domain from company name using heuristics
fn derive_domain_from_name(name: &str) -> Option<String> {
    let cleaned = clean_company_name(name);

    if cleaned.is_empty() {
        return None;
    }

    // Split into words
    let words: Vec<&str> = cleaned.split_whitespace().collect();

    if words.is_empty() {
        return None;
    }

    // Take first word (company name is usually first)
    let first_word = words[0];

    // Clean the word for domain use
    let domain_base: String = first_word
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();

    // Minimum length check
    if domain_base.len() < 2 {
        // Try first two words combined
        if words.len() >= 2 {
            let combined: String = format!("{}{}", words[0], words[1])
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect();
            if combined.len() >= 3 {
                return Some(format!("{}.com", combined));
            }
        }
        return None;
    }

    Some(format!("{}.com", domain_base))
}

/// Get domain for a security using intelligent detection
fn get_domain_for_security(name: &str, _ticker: Option<&str>, _isin: Option<&str>) -> Option<String> {
    // Skip commodities and crypto (no company logos)
    let name_lower = name.to_lowercase();
    if name_lower.contains("gold") && (name_lower.contains("physi") || name_lower.contains("xetra")) {
        return None;
    }
    if name_lower.contains("bitcoin") || name_lower.contains("ethereum") || name_lower.contains("crypto") {
        return None;
    }

    // 1. Check if it's an ETF - use provider domain
    if let Some(domain) = detect_etf_provider(name) {
        return Some(domain.to_string());
    }

    // 2. Check for known exceptions
    if let Some(domain) = check_domain_exception(name) {
        return Some(domain.to_string());
    }

    // 3. Derive domain from company name
    derive_domain_from_name(name)
}

/// Build Brandfetch CDN URL for a domain
fn build_logo_url(domain: &str, client_id: &str) -> String {
    format!(
        "https://cdn.brandfetch.io/{}/h/64/w/64/icon?c={}",
        domain, client_id
    )
}

/// Get logo URL for a security
#[command]
pub fn get_security_logo_url(
    client_id: String,
    security_id: i64,
    ticker: Option<String>,
    name: String,
    isin: Option<String>,
) -> LogoResult {
    // No client ID means no logos
    if client_id.is_empty() {
        return LogoResult {
            security_id,
            logo_url: None,
            domain: None,
        };
    }

    // Get domain using intelligent detection
    let domain = match get_domain_for_security(&name, ticker.as_deref(), isin.as_deref()) {
        Some(d) => d,
        None => {
            return LogoResult {
                security_id,
                logo_url: None,
                domain: None,
            };
        }
    };

    // Build the CDN URL
    let logo_url = build_logo_url(&domain, &client_id);

    LogoResult {
        security_id,
        logo_url: Some(logo_url),
        domain: Some(domain),
    }
}

/// Batch get logo URLs for multiple securities
/// Returns domains even without API key (for local cache lookup)
#[command]
pub fn get_logo_urls_batch(
    client_id: String,
    securities: Vec<(i64, Option<String>, String, Option<String>)>, // (id, ticker, name, isin)
) -> Vec<LogoResult> {
    securities
        .into_iter()
        .map(|(security_id, ticker, name, isin)| {
            let domain = get_domain_for_security(&name, ticker.as_deref(), isin.as_deref());
            // Only build CDN URL if API key is provided
            let logo_url = if client_id.is_empty() {
                None
            } else {
                domain.as_ref().map(|d| build_logo_url(d, &client_id))
            };

            LogoResult {
                security_id,
                logo_url,
                domain,
            }
        })
        .collect()
}

// Backwards compatibility functions

#[command]
pub async fn fetch_security_logo(
    client_id: String,
    security_id: i64,
    ticker: Option<String>,
    name: String,
) -> LogoResult {
    get_security_logo_url(client_id, security_id, ticker, name, None)
}

/// Get the cache directory for logos
fn get_logo_cache_dir() -> Result<std::path::PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Could not find data directory".to_string())?;
    let cache_dir = data_dir
        .join("com.portfolio-now.app")
        .join("logos");

    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;
    }

    Ok(cache_dir)
}

/// Check if a logo is cached locally for a domain
#[command]
pub fn is_logo_cached(domain: String) -> bool {
    if let Ok(cache_dir) = get_logo_cache_dir() {
        let file_name = domain.replace('.', "_") + ".png";
        let path = cache_dir.join(&file_name);
        path.exists()
    } else {
        false
    }
}

/// Get cached logo as base64 data URL
#[command]
pub fn get_cached_logo_data(domain: String) -> Option<String> {
    let cache_dir = get_logo_cache_dir().ok()?;
    let file_name = domain.replace('.', "_") + ".png";
    let path = cache_dir.join(&file_name);

    if path.exists() {
        let data = std::fs::read(&path).ok()?;
        let base64_str = base64::engine::general_purpose::STANDARD.encode(&data);
        Some(format!("data:image/png;base64,{}", base64_str))
    } else {
        None
    }
}

/// Save logo to local cache (receives base64 data from frontend)
#[command]
pub fn save_logo_to_cache(domain: String, base64_data: String) -> Result<String, String> {
    let cache_dir = get_logo_cache_dir()?;
    let file_name = domain.replace('.', "_") + ".png";
    let path = cache_dir.join(&file_name);

    // Extract base64 data (remove "data:image/png;base64," prefix if present)
    let data_str = if let Some(pos) = base64_data.find(",") {
        &base64_data[pos + 1..]
    } else {
        &base64_data
    };

    // Decode base64
    let bytes = base64::engine::general_purpose::STANDARD.decode(data_str)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Write to file
    std::fs::write(&path, &bytes)
        .map_err(|e| format!("Failed to write logo file: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

#[command]
pub fn get_cached_logo(_security_id: i64, _ticker: Option<String>, _name: String) -> Option<String> {
    None
}

#[command]
pub fn clear_logo_cache() -> Result<u32, String> {
    let cache_dir = get_logo_cache_dir()?;

    let mut count = 0;
    if cache_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "png") {
                    if std::fs::remove_file(&path).is_ok() {
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

#[command]
pub async fn fetch_logos_batch(
    client_id: String,
    securities: Vec<(i64, Option<String>, String)>,
) -> Vec<LogoResult> {
    // Convert to new format (add None for ISIN)
    let securities_with_isin: Vec<_> = securities
        .into_iter()
        .map(|(id, ticker, name)| (id, ticker, name, None))
        .collect();
    get_logo_urls_batch(client_id, securities_with_isin)
}

/// Reload all logos: clears cache and downloads fresh logos for all securities
#[command]
pub async fn reload_all_logos(
    client_id: String,
    securities: Vec<(i64, Option<String>, String)>, // (id, ticker, name)
) -> Result<ReloadResult, String> {
    if client_id.is_empty() {
        return Err("Client ID erforderlich".to_string());
    }

    // Get unique domains
    let mut domains: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (_, ticker, name) in &securities {
        if let Some(domain) = get_domain_for_security(name, ticker.as_deref(), None) {
            domains.insert(domain);
        }
    }

    let total_domains = domains.len() as u32;

    // Clear existing cache
    let cleared = clear_logo_cache().unwrap_or(0);

    // Download all logos
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut downloaded = 0u32;
    let mut failed = 0u32;

    for domain in domains {
        let url = build_logo_url(&domain, &client_id);

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(bytes) => {
                            // Save to cache
                            let cache_dir = match get_logo_cache_dir() {
                                Ok(dir) => dir,
                                Err(_) => {
                                    failed += 1;
                                    continue;
                                }
                            };
                            let file_name = domain.replace('.', "_") + ".png";
                            let path = cache_dir.join(&file_name);

                            if std::fs::write(&path, &bytes).is_ok() {
                                downloaded += 1;
                                log::info!("Downloaded logo for {}", domain);
                            } else {
                                failed += 1;
                            }
                        }
                        Err(_) => {
                            failed += 1;
                        }
                    }
                } else {
                    // 404 etc. - domain not found, not a real error
                    log::debug!("No logo found for {}: {}", domain, response.status());
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch logo for {}: {}", domain, e);
                failed += 1;
            }
        }
    }

    Ok(ReloadResult {
        cleared,
        downloaded,
        failed,
        total_domains,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReloadResult {
    pub cleared: u32,
    pub downloaded: u32,
    pub failed: u32,
    pub total_domains: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_company_name() {
        assert_eq!(clean_company_name("Apple Inc."), "apple");
        assert_eq!(clean_company_name("Microsoft Corporation"), "microsoft");
        assert_eq!(clean_company_name("NVIDIA CORP. DL-,001"), "nvidia");
        assert_eq!(clean_company_name("LVMH EO 0,3"), "lvmh");
        assert_eq!(clean_company_name("Linde plc"), "linde");
        assert_eq!(clean_company_name("SAP SE"), "sap");
    }

    #[test]
    fn test_derive_domain() {
        assert_eq!(derive_domain_from_name("Apple Inc."), Some("apple.com".to_string()));
        assert_eq!(derive_domain_from_name("Microsoft Corporation"), Some("microsoft.com".to_string()));
        assert_eq!(derive_domain_from_name("NVIDIA CORP."), Some("nvidia.com".to_string()));
    }

    #[test]
    fn test_etf_detection() {
        assert_eq!(detect_etf_provider("iShares Core MSCI World"), Some("blackrock.com"));
        assert_eq!(detect_etf_provider("Vanguard S&P 500 ETF"), Some("vanguard.com"));
        assert_eq!(detect_etf_provider("Amundis NASDAQ 100"), Some("amundi.com"));
    }

    #[test]
    fn test_domain_exceptions() {
        assert_eq!(check_domain_exception("Alphabet Inc."), Some("google.com"));
        assert_eq!(check_domain_exception("Deutsche Telekom AG"), Some("telekom.com"));
        assert_eq!(check_domain_exception("Procter & Gamble"), Some("pg.com"));
    }
}
