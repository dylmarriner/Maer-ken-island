//! Typed loaders for the real-world reference packs (Phase 0c Task 3).
//!
//! A pack is one JSON file under `fixtures/reference/<domain>/`. It names
//! its sources (citation and licence), the date its values were compiled,
//! and a list of items. Each item is one published value, range or table,
//! with a unit, the source it came from and how confident the transcription
//! is. Validation tests use `Confidence` to choose their tolerance: a
//! `Low` value may only gate a test as an order-of-magnitude check.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The six reference domains; each is one directory under
/// `fixtures/reference/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceDomain {
    Climate,
    Hydrology,
    Ecology,
    Geology,
    Humans,
    Labour,
}

impl ReferenceDomain {
    pub const ALL: [ReferenceDomain; 6] = [
        Self::Climate,
        Self::Hydrology,
        Self::Ecology,
        Self::Geology,
        Self::Humans,
        Self::Labour,
    ];

    /// The directory name under `fixtures/reference/`.
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Climate => "climate",
            Self::Hydrology => "hydrology",
            Self::Ecology => "ecology",
            Self::Geology => "geology",
            Self::Humans => "humans",
            Self::Labour => "labour",
        }
    }
}

/// How faithfully a value represents its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Order-of-magnitude or field-practice figure; wide tolerance only.
    Low,
    /// Representative value from the source; moderate tolerance.
    Medium,
    /// Directly stated by the source (a formula, standard or measured mean).
    High,
}

/// A published source a pack cites.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceSource {
    /// Short key items use to name this source.
    pub key: String,
    /// Full bibliographic citation.
    pub citation: String,
    /// Licence or copyright status of the values taken from it.
    pub licence: String,
}

/// A table of numbers: one labelled row per category, one column per
/// quantity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceTable {
    /// What the row labels name (e.g. `biome`, `age_years`).
    pub row_label: String,
    pub columns: Vec<String>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRow {
    pub label: String,
    pub values: Vec<f64>,
}

impl ReferenceTable {
    /// The value in `row`, `column`, if both exist.
    pub fn get(&self, row: &str, column: &str) -> Option<f64> {
        let col = self.columns.iter().position(|c| c == column)?;
        let row = self.rows.iter().find(|r| r.label == row)?;
        row.values.get(col).copied()
    }

    /// Every value in `column`, in row order, paired with its row label.
    pub fn column(&self, column: &str) -> Option<Vec<(&str, f64)>> {
        let col = self.columns.iter().position(|c| c == column)?;
        Some(
            self.rows
                .iter()
                .map(|r| (r.label.as_str(), r.values[col]))
                .collect(),
        )
    }
}

/// The value an item records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceValue {
    /// A single published value.
    Point(f64),
    /// A published range, with its typical (central) value when the source
    /// gives one.
    Range {
        min: f64,
        max: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        typical: Option<f64>,
    },
    Table(ReferenceTable),
}

/// One reference value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceItem {
    pub key: String,
    pub description: String,
    pub unit: String,
    /// Key of the pack source this value came from.
    pub source: String,
    pub confidence: Confidence,
    pub value: ReferenceValue,
    /// Conditions, caveats or the derivation, when needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl ReferenceItem {
    pub fn point(&self) -> Option<f64> {
        match self.value {
            ReferenceValue::Point(v) => Some(v),
            _ => None,
        }
    }

    /// `(min, max)` of a range item.
    pub fn range(&self) -> Option<(f64, f64)> {
        match self.value {
            ReferenceValue::Range { min, max, .. } => Some((min, max)),
            _ => None,
        }
    }

    /// The point value, or a range's typical value (its midpoint when the
    /// source gives none).
    pub fn central(&self) -> Option<f64> {
        match self.value {
            ReferenceValue::Point(v) => Some(v),
            ReferenceValue::Range { min, max, typical } => {
                Some(typical.unwrap_or(0.5 * (min + max)))
            }
            ReferenceValue::Table(_) => None,
        }
    }

    pub fn table(&self) -> Option<&ReferenceTable> {
        match &self.value {
            ReferenceValue::Table(t) => Some(t),
            _ => None,
        }
    }
}

/// One reference pack file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencePack {
    /// Unique across all packs; matches the file stem.
    pub id: String,
    pub domain: ReferenceDomain,
    pub title: String,
    /// Date (YYYY-MM-DD) the values were compiled from their sources.
    pub retrieved: String,
    /// Licence of the pack as a whole; each source states its own too.
    pub licence: String,
    pub sources: Vec<ReferenceSource>,
    pub items: Vec<ReferenceItem>,
}

/// Why a pack could not be loaded or is malformed.
#[derive(Debug)]
pub enum ReferenceError {
    Io(PathBuf, std::io::Error),
    Parse(PathBuf, serde_json::Error),
    Invalid { pack: String, reason: String },
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(path, err) => write!(f, "could not read {}: {err}", path.display()),
            Self::Parse(path, err) => write!(f, "could not parse {}: {err}", path.display()),
            Self::Invalid { pack, reason } => write!(f, "reference pack {pack}: {reason}"),
        }
    }
}

impl std::error::Error for ReferenceError {}

fn invalid(pack: &str, reason: impl Into<String>) -> ReferenceError {
    ReferenceError::Invalid {
        pack: pack.to_string(),
        reason: reason.into(),
    }
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

impl ReferencePack {
    /// Reads and validates one pack.
    pub fn load(path: &Path) -> Result<Self, ReferenceError> {
        let bytes = std::fs::read(path).map_err(|e| ReferenceError::Io(path.to_path_buf(), e))?;
        let pack: Self = serde_json::from_slice(&bytes)
            .map_err(|e| ReferenceError::Parse(path.to_path_buf(), e))?;
        pack.validate()?;
        Ok(pack)
    }

    /// Checks that the pack is complete and internally consistent: every
    /// source has a citation and licence, every item names a known source,
    /// keys are unique, and every number is finite and well ordered.
    pub fn validate(&self) -> Result<(), ReferenceError> {
        let id = self.id.as_str();
        if id.is_empty() || self.title.trim().is_empty() {
            return Err(invalid(id, "missing id or title"));
        }
        if !is_iso_date(&self.retrieved) {
            return Err(invalid(
                id,
                format!("retrieved {:?} is not YYYY-MM-DD", self.retrieved),
            ));
        }
        if self.licence.trim().is_empty() {
            return Err(invalid(id, "missing pack licence"));
        }
        if self.sources.is_empty() || self.items.is_empty() {
            return Err(invalid(id, "a pack needs at least one source and one item"));
        }
        let mut source_keys = BTreeSet::new();
        for s in &self.sources {
            if s.key.is_empty() || s.citation.trim().is_empty() || s.licence.trim().is_empty() {
                return Err(invalid(
                    id,
                    format!("source {:?} lacks a citation or licence", s.key),
                ));
            }
            if !source_keys.insert(s.key.as_str()) {
                return Err(invalid(id, format!("duplicate source key {:?}", s.key)));
            }
        }
        let mut item_keys = BTreeSet::new();
        for item in &self.items {
            let key = item.key.as_str();
            if key.is_empty() || item.description.trim().is_empty() || item.unit.trim().is_empty() {
                return Err(invalid(
                    id,
                    format!("item {key:?} lacks a key, description or unit"),
                ));
            }
            if !item_keys.insert(key) {
                return Err(invalid(id, format!("duplicate item key {key:?}")));
            }
            if !source_keys.contains(item.source.as_str()) {
                return Err(invalid(
                    id,
                    format!("item {key:?} cites unknown source {:?}", item.source),
                ));
            }
            match &item.value {
                ReferenceValue::Point(v) => {
                    if !v.is_finite() {
                        return Err(invalid(id, format!("item {key:?} is not finite")));
                    }
                }
                ReferenceValue::Range { min, max, typical } => {
                    if !(min.is_finite() && max.is_finite() && min <= max) {
                        return Err(invalid(id, format!("item {key:?} range is not ordered")));
                    }
                    if let Some(t) = typical {
                        if !(t.is_finite() && min <= t && t <= max) {
                            return Err(invalid(
                                id,
                                format!("item {key:?} typical lies outside its range"),
                            ));
                        }
                    }
                }
                ReferenceValue::Table(table) => {
                    if table.columns.is_empty() || table.rows.is_empty() {
                        return Err(invalid(id, format!("item {key:?} table is empty")));
                    }
                    let mut labels = BTreeSet::new();
                    for row in &table.rows {
                        if !labels.insert(row.label.as_str()) {
                            return Err(invalid(
                                id,
                                format!("item {key:?} repeats row {:?}", row.label),
                            ));
                        }
                        if row.values.len() != table.columns.len()
                            || row.values.iter().any(|v| !v.is_finite())
                        {
                            return Err(invalid(
                                id,
                                format!(
                                    "item {key:?} row {:?} has wrong width or a non-finite value",
                                    row.label
                                ),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn item(&self, key: &str) -> Option<&ReferenceItem> {
        self.items.iter().find(|i| i.key == key)
    }

    pub fn source(&self, key: &str) -> Option<&ReferenceSource> {
        self.sources.iter().find(|s| s.key == key)
    }
}

/// Every pack under a reference directory, keyed by pack id.
#[derive(Debug, Clone, Default)]
pub struct ReferenceLibrary {
    packs: BTreeMap<String, ReferencePack>,
}

impl ReferenceLibrary {
    /// Loads every `*.json` under `root/<domain>/` for all six domains, in
    /// sorted path order. A pack must declare the domain of its directory and
    /// an id equal to its file stem; ids must be unique.
    pub fn load(root: &Path) -> Result<Self, ReferenceError> {
        let mut packs = BTreeMap::new();
        for domain in ReferenceDomain::ALL {
            let dir = root.join(domain.dir_name());
            let entries =
                std::fs::read_dir(&dir).map_err(|e| ReferenceError::Io(dir.clone(), e))?;
            let mut paths = Vec::new();
            for entry in entries {
                let path = entry
                    .map_err(|e| ReferenceError::Io(dir.clone(), e))?
                    .path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    paths.push(path);
                }
            }
            paths.sort();
            for path in paths {
                let pack = ReferencePack::load(&path)?;
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if pack.id != stem {
                    return Err(invalid(
                        &pack.id,
                        format!("id does not match file name {stem:?}"),
                    ));
                }
                if pack.domain != domain {
                    return Err(invalid(
                        &pack.id,
                        format!(
                            "declares {:?} but lives in {}/",
                            pack.domain,
                            domain.dir_name()
                        ),
                    ));
                }
                if packs.contains_key(&pack.id) {
                    return Err(invalid(&pack.id, "duplicate pack id"));
                }
                packs.insert(pack.id.clone(), pack);
            }
        }
        Ok(Self { packs })
    }

    pub fn pack(&self, id: &str) -> Option<&ReferencePack> {
        self.packs.get(id)
    }

    pub fn packs(&self) -> impl Iterator<Item = &ReferencePack> {
        self.packs.values()
    }

    pub fn in_domain(&self, domain: ReferenceDomain) -> impl Iterator<Item = &ReferencePack> {
        self.packs.values().filter(move |p| p.domain == domain)
    }

    /// Finds `key` in any pack of `domain`.
    pub fn item(&self, domain: ReferenceDomain, key: &str) -> Option<&ReferenceItem> {
        self.in_domain(domain).find_map(|p| p.item(key))
    }
}

/// `fixtures/reference/` in this repository.
pub fn default_reference_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/reference")
}
