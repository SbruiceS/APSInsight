//! The feature register and the domain/capability catalogue derived from it.
//!
//! `register/catalog.json` is produced by `tools/analyse-register.mjs`, which proves
//! that the 3,100 features are exactly 100 domains x 31 capabilities.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const CATALOG_JSON: &str = include_str!("../../../register/catalog.json");
const REGISTER_JSON: &str = include_str!("../../../register/APSInsight_Feature_Register.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub index: usize,
    pub slug: String,
    pub title: String,
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityKind {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub index: usize,
    pub slug: String,
    pub name: String,
    pub entities: Vec<EntityKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub register: String,
    pub features: usize,
    pub capabilities: Vec<Capability>,
    pub domains: Vec<Domain>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub domain: String,
    pub title: String,
    pub requirement: String,
}

pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| serde_json::from_str(CATALOG_JSON).expect("embedded catalog.json is valid"))
}

pub fn features() -> &'static [Feature] {
    static FEATURES: OnceLock<Vec<Feature>> = OnceLock::new();
    FEATURES.get_or_init(|| serde_json::from_str(REGISTER_JSON).expect("embedded register is valid"))
}

impl Catalog {
    pub fn domain(&self, slug: &str) -> Option<&Domain> {
        self.domains.iter().find(|d| d.slug == slug)
    }

    pub fn entity(&self, domain: &str, entity: &str) -> Option<(&Domain, &EntityKind)> {
        let d = self.domain(domain)?;
        d.entities.iter().find(|e| e.slug == entity).map(|e| (d, e))
    }

    pub fn capability(&self, slug: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|c| c.slug == slug)
    }

    pub fn entity_kind_count(&self) -> usize {
        self.domains.iter().map(|d| d.entities.len()).sum()
    }

    /// Resolves a feature id (`APSI-F0001`..) to its domain and capability.
    pub fn feature_coordinates(&self, feature_id: &str) -> Option<(&Domain, &Capability)> {
        let n: usize = feature_id.strip_prefix("APSI-F")?.parse().ok()?;
        let i = n.checked_sub(1)?;
        let per = self.capabilities.len();
        Some((self.domains.get(i / per)?, self.capabilities.get(i % per)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_matches_register() {
        let c = catalog();
        assert_eq!(c.features, 3100);
        assert_eq!(c.domains.len(), 100);
        assert_eq!(c.capabilities.len(), 31);
        assert_eq!(c.entity_kind_count(), 800);
        assert_eq!(features().len(), 3100);
        for f in features() {
            let (d, cap) = c.feature_coordinates(&f.id).unwrap();
            assert_eq!(d.name, f.domain, "{}", f.id);
            assert_eq!(cap.title, f.title, "{}", f.id);
        }
    }

    #[test]
    fn malformed_feature_ids_are_rejected() {
        let c = catalog();
        for bad in ["", "APSI-F0000", "APSI-F3101", "APSI-Fxx", "F0001"] {
            assert!(c.feature_coordinates(bad).is_none(), "{bad}");
        }
    }
}
