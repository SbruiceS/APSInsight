//! Types shared by every contract: versions, typed errors, products, capability
//! discovery, and the value checks (units, CRS, hashes, timestamps) that keep
//! semantic information from being lost at a boundary.

use serde::{Deserialize, Serialize};
use std::fmt;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// `major.minor`. A provider offering `offered` satisfies a consumer that needs
/// `required` when the majors are equal and `offered.minor >= required.minor`.
/// A major change is breaking; a minor change only adds optional fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractVersion {
    pub major: u16,
    pub minor: u16,
}

impl ContractVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub fn satisfies(self, required: ContractVersion) -> bool {
        self.major == required.major && self.minor >= required.minor
    }

    pub fn parse(s: &str) -> Result<Self> {
        let bad = || ContractError::invalid("contract_version", format!("`{s}` is not `major.minor`"));
        let (a, b) = s.split_once('.').ok_or_else(bad)?;
        let num = |p: &str| if !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()) { p.parse::<u16>().map_err(|_| bad()) } else { Err(bad()) };
        Ok(Self::new(num(a)?, num(b)?))
    }
}

impl fmt::Display for ContractVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl Serialize for ContractVersion {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ContractVersion {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        ContractVersion::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ContractError {
    /// The request or response violates the contract. Permanent.
    #[error("invalid {field}: {reason}")]
    Invalid { field: String, reason: String },
    /// The product is not installed or not reachable. Retryable only if the
    /// product is later deployed; callers must degrade gracefully.
    #[error("{product} is unavailable: {reason}")]
    Unavailable { product: String, reason: String },
    /// Provider and consumer contract versions are incompatible. Permanent.
    #[error("contract {offered} does not satisfy required {required}")]
    Incompatible { required: ContractVersion, offered: ContractVersion },
    /// The provider reported a failure.
    #[error("provider error {provider_code}: {message}")]
    Provider { provider_code: String, message: String, retryable: bool },
}

impl ContractError {
    pub fn invalid(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Invalid { field: field.into(), reason: reason.into() }
    }

    pub fn retryable(&self) -> bool {
        matches!(self, Self::Provider { retryable: true, .. })
    }
}

pub type Result<T> = std::result::Result<T, ContractError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Product {
    Agriql,
    KutajkutambiAi,
    Agricuda,
    Apsnetwork,
    NityagaRtos,
    /// Optional hardware accelerators such as GreenPlus FPGA.
    Accelerator,
}

impl Product {
    pub const ALL: [Product; 6] = [Self::Agriql, Self::KutajkutambiAi, Self::Agricuda, Self::Apsnetwork, Self::NityagaRtos, Self::Accelerator];

    pub fn name(self) -> &'static str {
        match self {
            Self::Agriql => "AgriQL",
            Self::KutajkutambiAi => "KutajKutambi AI",
            Self::Agricuda => "AgriCUDA",
            Self::Apsnetwork => "APSNetwork",
            Self::NityagaRtos => "Nityaga RTOS",
            Self::Accelerator => "Hardware accelerator (e.g. GreenPlus FPGA)",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Agriql => "agriql",
            Self::KutajkutambiAi => "kutajkutambi-ai",
            Self::Agricuda => "agricuda",
            Self::Apsnetwork => "apsnetwork",
            Self::NityagaRtos => "nityaga-rtos",
            Self::Accelerator => "accelerator",
        }
    }

    /// The contract version this build of APSInsight speaks.
    pub fn contract_version(self) -> ContractVersion {
        ContractVersion::new(1, 0)
    }

    /// The operations defined by the contract (independent of availability).
    pub fn operations(self) -> &'static [&'static str] {
        match self {
            Self::Agriql => &["query.submit"],
            Self::KutajkutambiAi => &["inference.run", "agent.task"],
            Self::Agricuda => &["compute.execute", "compute.cpu-fallback"],
            Self::Apsnetwork => &["telemetry.ingest", "edge-job.deliver"],
            Self::NityagaRtos => &["device.capabilities", "device.command"],
            Self::Accelerator => &["accelerator.load", "compute.execute"],
        }
    }

    /// The register domain this product's contract features belong to.
    pub fn register_domain(self) -> &'static str {
        match self {
            Self::Agriql => "future-agriql-integration-contract",
            Self::KutajkutambiAi => "future-kutajkutambi-ai-integration-contract",
            Self::Agricuda => "future-agricuda-integration-contract",
            Self::Apsnetwork => "future-apsnetwork-integration-contract",
            Self::NityagaRtos => "future-nityaga-rtos-integration-contract",
            Self::Accelerator => "hardware-and-fpga-research-interfaces",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Availability {
    Unavailable { reason: String },
    Available { provider_version: String, offered_contract: ContractVersion },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub product: Product,
    pub product_name: String,
    pub contract_version: ContractVersion,
    pub availability: Availability,
    pub operations: Vec<String>,
    /// Always true: APSInsight works without any ASPF product.
    pub optional: bool,
}

impl CapabilityReport {
    /// Checks that an available provider speaks a compatible contract.
    pub fn check_compatible(&self) -> Result<()> {
        match &self.availability {
            Availability::Unavailable { .. } => Ok(()),
            Availability::Available { offered_contract, .. } if offered_contract.satisfies(self.contract_version) => Ok(()),
            Availability::Available { offered_contract, .. } => {
                Err(ContractError::Incompatible { required: self.contract_version, offered: *offered_contract })
            }
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self.availability, Availability::Available { .. })
    }
}

/// Every adapter, real or not, can describe itself.
pub trait Adapter: Send + Sync {
    fn product(&self) -> Product;
    fn discover(&self) -> CapabilityReport;
}

/// The default adapter for every product: the product does not exist yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unavailable(pub Product);

impl Unavailable {
    pub fn error(&self) -> ContractError {
        ContractError::Unavailable { product: self.0.name().into(), reason: UNAVAILABLE_REASON.into() }
    }
}

pub const UNAVAILABLE_REASON: &str = "planned ASPF product; not yet created and not deployed with this build";

impl Adapter for Unavailable {
    fn product(&self) -> Product {
        self.0
    }

    fn discover(&self) -> CapabilityReport {
        CapabilityReport {
            product: self.0,
            product_name: self.0.name().into(),
            contract_version: self.0.contract_version(),
            availability: Availability::Unavailable { reason: UNAVAILABLE_REASON.into() },
            operations: self.0.operations().iter().map(|s| s.to_string()).collect(),
            optional: true,
        }
    }
}

/// Capability reports for every product, using the default (unavailable) adapters.
pub fn discover_all() -> Vec<CapabilityReport> {
    Product::ALL.iter().map(|p| Unavailable(*p).discover()).collect()
}

// ----- value checks shared by the contracts -----

pub(crate) fn non_empty(field: &str, v: &str, max: usize) -> Result<()> {
    if v.trim().is_empty() {
        return Err(ContractError::invalid(field, "must not be empty"));
    }
    if v.len() > max {
        return Err(ContractError::invalid(field, format!("longer than {max} bytes")));
    }
    Ok(())
}

/// Unit symbols are UCUM-style case-sensitive codes, e.g. `kg/ha`, `mm`, `Cel`,
/// `m3/s`, `1` (dimensionless). An empty unit is never accepted: dimensionless
/// values must say `1`.
pub fn check_unit(field: &str, unit: &str) -> Result<()> {
    non_empty(field, unit, 32)?;
    if unit.bytes().all(|b| b.is_ascii_alphanumeric() || b"/.^-_%[]()'".contains(&b)) {
        Ok(())
    } else {
        Err(ContractError::invalid(field, format!("`{unit}` is not a unit code")))
    }
}

/// CRS identifiers: `EPSG:<code>` or `OGC:CRS84`.
pub fn check_crs(field: &str, crs: &str) -> Result<()> {
    let ok = crs == "OGC:CRS84" || crs.strip_prefix("EPSG:").is_some_and(|c| !c.is_empty() && c.len() <= 6 && c.bytes().all(|b| b.is_ascii_digit()));
    if ok { Ok(()) } else { Err(ContractError::invalid(field, format!("`{crs}` is not EPSG:<code> or OGC:CRS84"))) }
}

pub fn check_sha256(field: &str, h: &str) -> Result<()> {
    if h.len() == 64 && h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        Ok(())
    } else {
        Err(ContractError::invalid(field, "must be 64 lowercase hex characters"))
    }
}

pub fn parse_time(field: &str, s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|_| ContractError::invalid(field, format!("`{s}` is not an RFC 3339 timestamp")))
}

pub(crate) fn finite(field: &str, v: f64) -> Result<()> {
    if v.is_finite() { Ok(()) } else { Err(ContractError::invalid(field, "must be finite")) }
}

/// A measured or derived quantity. The unit is mandatory; missing values are
/// explicit (`value: null` with `quality: missing`), never a sentinel number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub value: Option<f64>,
    pub unit: String,
    #[serde(default)]
    pub quality: QualityFlag,
    /// Standard uncertainty in the same unit, when known.
    #[serde(default)]
    pub uncertainty: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityFlag {
    #[default]
    Good,
    Suspect,
    Estimated,
    Missing,
}

impl Quantity {
    pub fn validate(&self, field: &str) -> Result<()> {
        check_unit(&format!("{field}.unit"), &self.unit)?;
        match (self.value, self.quality) {
            (None, QualityFlag::Missing) => {}
            (None, _) => return Err(ContractError::invalid(format!("{field}.value"), "null value must have quality `missing`")),
            (Some(_), QualityFlag::Missing) => return Err(ContractError::invalid(format!("{field}.quality"), "`missing` requires a null value")),
            (Some(v), _) => finite(&format!("{field}.value"), v)?,
        }
        if let Some(u) = self.uncertainty {
            finite(&format!("{field}.uncertainty"), u)?;
            if u < 0.0 {
                return Err(ContractError::invalid(format!("{field}.uncertainty"), "must be >= 0"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compatibility_rules() {
        let v1_0 = ContractVersion::new(1, 0);
        let v1_2 = ContractVersion::new(1, 2);
        let v2_0 = ContractVersion::new(2, 0);
        assert!(v1_2.satisfies(v1_0));
        assert!(!v1_0.satisfies(v1_2));
        assert!(!v2_0.satisfies(v1_0));
        assert_eq!(ContractVersion::parse("1.2").unwrap(), v1_2);
        for bad in ["", "1", "1.", ".1", "a.b", "1.2.3", "-1.0", "99999.0"] {
            assert!(ContractVersion::parse(bad).is_err(), "{bad}");
        }
        assert_eq!(serde_json::to_string(&v1_2).unwrap(), "\"1.2\"");
        assert!(serde_json::from_str::<ContractVersion>("\"x\"").is_err());
    }

    #[test]
    fn every_product_is_unavailable_and_optional_by_default() {
        let all = discover_all();
        assert_eq!(all.len(), 6);
        for r in &all {
            assert!(!r.is_available() && r.optional && !r.operations.is_empty());
            assert!(r.check_compatible().is_ok());
        }
        let err = Unavailable(Product::Agriql).error();
        assert!(!err.retryable());
        assert!(matches!(err, ContractError::Unavailable { .. }));
    }

    #[test]
    fn incompatible_providers_are_detected() {
        let mut r = Unavailable(Product::Agricuda).discover();
        r.availability = Availability::Available { provider_version: "0.9".into(), offered_contract: ContractVersion::new(2, 0) };
        assert!(matches!(r.check_compatible(), Err(ContractError::Incompatible { .. })));
        r.availability = Availability::Available { provider_version: "1.4".into(), offered_contract: ContractVersion::new(1, 3) };
        assert!(r.check_compatible().is_ok());
    }

    #[test]
    fn value_checks_reject_lossy_inputs() {
        assert!(check_unit("u", "kg/ha").is_ok() && check_unit("u", "1").is_ok() && check_unit("u", "Cel").is_ok());
        assert!(check_unit("u", "").is_err() && check_unit("u", "kg ha").is_err() && check_unit("u", "<script>").is_err());
        assert!(check_crs("c", "EPSG:4326").is_ok() && check_crs("c", "OGC:CRS84").is_ok());
        assert!(check_crs("c", "EPSG:").is_err() && check_crs("c", "WGS84").is_err() && check_crs("c", "EPSG:43a6").is_err());
        assert!(check_sha256("h", &"a".repeat(64)).is_ok() && check_sha256("h", &"A".repeat(64)).is_err());
        assert!(parse_time("t", "2026-10-15T08:00:00Z").is_ok() && parse_time("t", "2026-10-15").is_err());
        let q = |value, quality| Quantity { value, unit: "mm".into(), quality, uncertainty: None };
        assert!(q(Some(1.0), QualityFlag::Good).validate("q").is_ok());
        assert!(q(None, QualityFlag::Missing).validate("q").is_ok());
        assert!(q(None, QualityFlag::Good).validate("q").is_err());
        assert!(q(Some(1.0), QualityFlag::Missing).validate("q").is_err());
        assert!(q(Some(f64::NAN), QualityFlag::Good).validate("q").is_err());
        assert!(Quantity { uncertainty: Some(-1.0), ..q(Some(1.0), QualityFlag::Good) }.validate("q").is_err());
    }
}
