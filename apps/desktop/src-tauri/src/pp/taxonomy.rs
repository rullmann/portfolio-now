//! Taxonomy model for Portfolio Performance.
//!
//! Taxonomies provide hierarchical classification of securities.

use serde::{Deserialize, Serialize};

/// A taxonomy for classifying securities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taxonomy {
    pub id: String,
    pub name: String,
    pub source: Option<String>,
    pub dimensions: Vec<String>,
    pub root: Option<Classification>,
}

impl Taxonomy {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            source: None,
            dimensions: Vec::new(),
            root: None,
        }
    }

    /// Get all classifications flattened
    pub fn all_classifications(&self) -> Vec<&Classification> {
        let mut result = Vec::new();
        if let Some(ref root) = self.root {
            Self::collect_classifications(root, &mut result);
        }
        result
    }

    fn collect_classifications<'a>(
        classification: &'a Classification,
        result: &mut Vec<&'a Classification>,
    ) {
        result.push(classification);
        for child in &classification.children {
            Self::collect_classifications(child, result);
        }
    }
}

/// A classification within a taxonomy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Classification {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    /// Weight (10000 = 100%)
    pub weight: Option<i32>,
    pub rank: Option<i32>,
    pub children: Vec<Classification>,
    pub assignments: Vec<ClassificationAssignment>,
    /// Custom data attributes
    pub data: std::collections::HashMap<String, String>,
}

impl Classification {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            color: None,
            weight: None,
            rank: None,
            children: Vec::new(),
            assignments: Vec::new(),
            data: std::collections::HashMap::new(),
        }
    }

    /// Get weight as percentage (0.0 - 100.0)
    pub fn weight_percent(&self) -> Option<f64> {
        self.weight.map(|w| w as f64 / 100.0)
    }
}

/// Assignment of a security to a classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationAssignment {
    /// Type of investment vehicle ("security" or "account")
    pub vehicle_class: String,
    /// UUID of the referenced entity
    pub vehicle_uuid: String,
    /// Weight (10000 = 100%)
    pub weight: i32,
    pub rank: Option<i32>,
}

impl ClassificationAssignment {
    pub fn security(security_uuid: String, weight: i32) -> Self {
        Self {
            vehicle_class: "security".to_string(),
            vehicle_uuid: security_uuid,
            weight,
            rank: None,
        }
    }

    pub fn account(account_uuid: String, weight: i32) -> Self {
        Self {
            vehicle_class: "account".to_string(),
            vehicle_uuid: account_uuid,
            weight,
            rank: None,
        }
    }

    /// Get weight as percentage (0.0 - 100.0)
    pub fn weight_percent(&self) -> f64 {
        self.weight as f64 / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taxonomy_flattening() {
        let mut taxonomy = Taxonomy::new("tax-1".to_string(), "Asset Allocation".to_string());

        let mut root = Classification::new("root".to_string(), "Root".to_string());
        root.children.push(Classification::new("c1".to_string(), "Stocks".to_string()));
        root.children.push(Classification::new("c2".to_string(), "Bonds".to_string()));
        root.children[0].children.push(Classification::new(
            "c1-1".to_string(),
            "US Stocks".to_string(),
        ));

        taxonomy.root = Some(root);

        let all = taxonomy.all_classifications();
        assert_eq!(all.len(), 4); // root, stocks, bonds, us stocks
    }

    #[test]
    fn test_weight_conversion() {
        let classification = Classification {
            id: "test".to_string(),
            name: "Test".to_string(),
            weight: Some(6000), // 60%
            ..Classification::new(String::new(), String::new())
        };

        assert_eq!(classification.weight_percent(), Some(60.0));
    }
}
