use serde::{Deserialize, Serialize};
use zvariant::Type;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct PackageRecord {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub summary: String,
    pub section: String,
    pub installed_size_kib: u64,
    pub essential: bool,
    pub priority: String,
    pub source: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct UpdateRecord {
    pub name: String,
    pub current_version: String,
    pub candidate_version: String,
    pub architecture: String,
    pub source: String,
    pub security: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct AppRecord {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub summary: String,
    pub package: String,
    pub icon: String,
    pub homepage: String,
    pub installed: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct SystemSnapshot {
    pub installed: Vec<PackageRecord>,
    pub updates: Vec<UpdateRecord>,
}
