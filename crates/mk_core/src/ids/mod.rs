use serde::{Deserialize, Serialize};
/**
 * Purpose
 * - Identifier types for Maer'Ken world and Marr'Kena planet entities.
 * - Provides type-safe string identifiers for all canon entities.
 *
 * Invariants
 * - All identifiers are &'static str literals (no runtime allocation).
 * - World-level identifiers never reference planet-specific parameters.
 * - Planet-level identifiers are scoped to Marr'Kena MK-I only.
 *
 * Failure Modes
 * - Invalid identifier usage → compile-time error.
 * - Attempt to mutate identifiers → runtime panic.
 * - Mixing world/planet identifiers → logic error.
 *
 * Debug Notes
 * - All identifiers are compile-time constants.
 * - Use as_str() to get string representation for debugging.
 * - Identifiers are Copy and Clone for efficiency.
 */
use std::fmt;

// ========== WORLD-LEVEL IDENTIFIERS (Maer'Ken) ==========

/// Maer'Ken Continuum identifier - the world system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinuumId(&'static str);

impl Serialize for ContinuumId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ContinuumId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        match s.as_str() {
            "MK-CONTINUUM-0" => Ok(ContinuumId::MK_CONTINUUM_0),
            _ => Err(serde::de::Error::custom("Unknown ContinuumId")),
        }
    }
}

impl ContinuumId {
    pub const MK_CONTINUUM_0: Self = Self("MK-CONTINUUM-0");

    #[inline(always)]
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Default for ContinuumId {
    fn default() -> Self {
        Self::MK_CONTINUUM_0
    }
}

impl fmt::Display for ContinuumId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Marr'Kena System identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemId(&'static str);

impl Serialize for SystemId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SystemId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        match s.as_str() {
            "MK-SYS-01" => Ok(SystemId::MK_SYS_01),
            _ => Err(serde::de::Error::custom("Unknown SystemId")),
        }
    }
}

impl SystemId {
    pub const MK_SYS_01: Self = Self("MK-SYS-01");

    #[inline(always)]
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Default for SystemId {
    fn default() -> Self {
        Self::MK_SYS_01
    }
}

impl fmt::Display for SystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Aurelion Marr star identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarId(&'static str);

impl Serialize for StarId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StarId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        match s.as_str() {
            "MK-STAR-01" => Ok(StarId::MK_STAR_01),
            _ => Err(serde::de::Error::custom("Unknown StarId")),
        }
    }
}

impl StarId {
    pub const MK_STAR_01: Self = Self("MK-STAR-01");

    #[inline(always)]
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Default for StarId {
    fn default() -> Self {
        Self::MK_STAR_01
    }
}

impl fmt::Display for StarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ========== PLANET-LEVEL IDENTIFIERS (Marr'Kena) ==========

/// Marr'Kena planet identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetId(&'static str);

impl Serialize for PlanetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PlanetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        match s.as_str() {
            "MK-I" => Ok(PlanetId::MK_I),
            _ => Err(serde::de::Error::custom("Unknown PlanetId")),
        }
    }
}

impl PlanetId {
    pub const MK_I: Self = Self("MK-I");

    #[inline(always)]
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl Default for PlanetId {
    fn default() -> Self {
        Self::MK_I
    }
}

impl fmt::Display for PlanetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Geological/Biological epoch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Epoch {
    /// Pre-life period
    AkenVoid,
    /// Microbial life period
    KenicGenesis,
    /// Complex life period (CURRENT)
    #[default]
    MarrKenBloom,
}

impl Serialize for Epoch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Epoch::AkenVoid => serializer.serialize_str("AkenVoid"),
            Epoch::KenicGenesis => serializer.serialize_str("KenicGenesis"),
            Epoch::MarrKenBloom => serializer.serialize_str("MarrKenBloom"),
        }
    }
}

impl<'de> Deserialize<'de> for Epoch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        match s.as_str() {
            "AkenVoid" => Ok(Epoch::AkenVoid),
            "KenicGenesis" => Ok(Epoch::KenicGenesis),
            "MarrKenBloom" => Ok(Epoch::MarrKenBloom),
            _ => Err(serde::de::Error::custom("Unknown Epoch")),
        }
    }
}

impl fmt::Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AkenVoid => write!(f, "AkenVoid"),
            Self::KenicGenesis => write!(f, "KenicGenesis"),
            Self::MarrKenBloom => write!(f, "MarrKenBloom"),
        }
    }
}

/// Planet classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanetClass {
    /// Animals-only, pre-sapient world
    #[default]
    KenzIe,
}

impl Serialize for PlanetClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PlanetClass::KenzIe => serializer.serialize_str("KenzIe"),
        }
    }
}

impl<'de> Deserialize<'de> for PlanetClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        match s.as_str() {
            "KenzIe" => Ok(PlanetClass::KenzIe),
            _ => Err(serde::de::Error::custom("Unknown PlanetClass")),
        }
    }
}

impl fmt::Display for PlanetClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KenzIe => write!(f, "KenzIe"),
        }
    }
}

/// Kenz-ie subclass classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KenzIeSubclass {
    /// Pre-sapient intelligence ceiling
    #[default]
    Alpha,
}

impl Serialize for KenzIeSubclass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            KenzIeSubclass::Alpha => serializer.serialize_str("Alpha"),
        }
    }
}

impl<'de> Deserialize<'de> for KenzIeSubclass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        match s.as_str() {
            "Alpha" => Ok(KenzIeSubclass::Alpha),
            _ => Err(serde::de::Error::custom("Unknown KenzIeSubclass")),
        }
    }
}

impl fmt::Display for KenzIeSubclass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Alpha => write!(f, "Alpha"),
        }
    }
}
