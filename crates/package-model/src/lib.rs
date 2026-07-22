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
pub struct PackageDetailRecord {
    pub found: bool,
    pub name: String,
    pub version: String,
    pub candidate_version: String,
    pub architecture: String,
    pub summary: String,
    pub description: String,
    pub section: String,
    pub installed_size_kib: u64,
    pub essential: bool,
    pub priority: String,
    pub source: String,
    pub origin: String,
    pub maintainer: String,
    pub homepage: String,
    pub depends: Vec<String>,
    pub recommends: Vec<String>,
    pub suggests: Vec<String>,
    pub upgradable: bool,
    pub security_update: bool,
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
    pub categories: Vec<String>,
    pub installed: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct FeaturedCollection {
    pub id: String,
    pub title: String,
    pub description: String,
    pub query: String,
    pub icon: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct CatalogPage {
    pub items: Vec<AppRecord>,
    pub offset: u64,
    pub limit: u64,
    pub total: u64,
    pub has_more: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct SystemHealth {
    pub dpkg_status_readable: bool,
    pub apt_available: bool,
    pub appstream_available: bool,
    pub reboot_required: bool,
    pub installed_count: u64,
    pub installed_size_kib: u64,
    pub essential_count: u64,
    pub broken_package_count: u64,
    pub update_count: u64,
    pub security_update_count: u64,
    pub update_sources: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct SystemSnapshot {
    pub installed: Vec<PackageRecord>,
    pub updates: Vec<UpdateRecord>,
    pub health: SystemHealth,
}
