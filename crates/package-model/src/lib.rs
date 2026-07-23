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
    pub category: String,
    pub icon: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct CuratedCollection {
    pub id: String,
    pub title: String,
    pub description: String,
    pub query: String,
    pub category: String,
    pub icon: String,
    pub applications: Vec<AppRecord>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct CatalogPage {
    pub applications: Vec<AppRecord>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct ServiceRecord {
    pub name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct TransactionChange {
    pub package: String,
    pub action: String,
    pub current_version: String,
    pub candidate_version: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct TransactionPreview {
    pub id: u64,
    pub kind: String,
    pub package: String,
    pub changes: Vec<TransactionChange>,
    pub download_size_bytes: u64,
    pub installed_size_delta_bytes: i64,
    pub requires_reboot: bool,
    pub ready: bool,
    pub summary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct TransactionRecord {
    pub id: u64,
    pub preview_id: u64,
    pub kind: String,
    pub package: String,
    pub state: String,
    pub progress_basis_points: u32,
    pub can_cancel: bool,
    pub created_unix_ms: u64,
    pub updated_unix_ms: u64,
    pub message: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct TransactionQueueSnapshot {
    pub has_active: bool,
    pub active: TransactionRecord,
    pub queued: Vec<TransactionRecord>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
pub struct TransactionEvent {
    pub sequence: u64,
    pub event: String,
    pub transaction_id: u64,
    pub preview_id: u64,
    pub kind: String,
    pub package: String,
    pub state: String,
    pub progress_basis_points: u32,
    pub level: String,
    pub message: String,
    pub created_unix_ms: u64,
}
