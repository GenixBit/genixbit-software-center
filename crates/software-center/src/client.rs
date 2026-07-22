use anyhow::Context;
use genixbit_package_model::{
    CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, SystemHealth,
    SystemSnapshot, UpdateRecord,
};
use zbus::{Connection, proxy};

#[proxy(
    interface = "com.genixbit.PackageManager1",
    default_service = "com.genixbit.PackageManager1",
    default_path = "/com/genixbit/PackageManager1"
)]
trait PackageManager {
    async fn version(&self) -> zbus::Result<String>;
    async fn system_snapshot(&self) -> zbus::Result<SystemSnapshot>;
    async fn system_health(&self) -> zbus::Result<SystemHealth>;
    async fn list_installed(&self) -> zbus::Result<Vec<PackageRecord>>;
    async fn check_updates(&self) -> zbus::Result<Vec<UpdateRecord>>;
    async fn package_details(&self, package: &str) -> zbus::Result<PackageDetailRecord>;
    async fn featured_collections(&self) -> zbus::Result<Vec<FeaturedCollection>>;
    async fn search_catalog_page(
        &self,
        query: &str,
        offset: u64,
        limit: u64,
    ) -> zbus::Result<CatalogPage>;
}

pub async fn load_snapshot() -> anyhow::Result<SystemSnapshot> {
    let connection = connect().await?;
    let proxy = PackageManagerProxy::new(&connection)
        .await
        .context("failed to create package-manager proxy")?;
    proxy
        .system_snapshot()
        .await
        .context("failed to load the system package snapshot")
}

pub async fn package_details(package: &str) -> anyhow::Result<PackageDetailRecord> {
    let connection = connect().await?;
    let proxy = PackageManagerProxy::new(&connection)
        .await
        .context("failed to create package-manager proxy")?;
    proxy
        .package_details(package)
        .await
        .context("failed to load package details")
}

pub async fn featured_collections() -> anyhow::Result<Vec<FeaturedCollection>> {
    let connection = connect().await?;
    let proxy = PackageManagerProxy::new(&connection)
        .await
        .context("failed to create package-manager proxy")?;
    proxy
        .featured_collections()
        .await
        .context("failed to load featured AppStream collections")
}

pub async fn search_catalog_page(
    query: &str,
    offset: u64,
    limit: u64,
) -> anyhow::Result<CatalogPage> {
    let connection = connect().await?;
    let proxy = PackageManagerProxy::new(&connection)
        .await
        .context("failed to create package-manager proxy")?;
    proxy
        .search_catalog_page(query, offset, limit)
        .await
        .context("failed to load the AppStream catalogue page")
}

async fn connect() -> anyhow::Result<Connection> {
    let use_session_bus = std::env::var("GENIXPKGD_BUS").is_ok_and(|value| value == "session");
    if use_session_bus {
        Connection::session()
            .await
            .context("failed to connect to the session D-Bus")
    } else {
        Connection::system()
            .await
            .context("failed to connect to the system D-Bus")
    }
}
