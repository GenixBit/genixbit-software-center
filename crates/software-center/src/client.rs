use anyhow::Context;
use genixbit_package_model::{AppRecord, PackageRecord, SystemSnapshot, UpdateRecord};
use zbus::{Connection, proxy};

#[proxy(
    interface = "com.genixbit.PackageManager1",
    default_service = "com.genixbit.PackageManager1",
    default_path = "/com/genixbit/PackageManager1"
)]
trait PackageManager {
    async fn version(&self) -> zbus::Result<String>;
    async fn list_installed(&self) -> zbus::Result<Vec<PackageRecord>>;
    async fn check_updates(&self) -> zbus::Result<Vec<UpdateRecord>>;
    async fn search_catalog(&self, query: &str) -> zbus::Result<Vec<AppRecord>>;
}

pub async fn load_snapshot() -> anyhow::Result<SystemSnapshot> {
    let connection = connect().await?;
    let proxy = PackageManagerProxy::new(&connection)
        .await
        .context("failed to create package-manager proxy")?;

    let installed = proxy
        .list_installed()
        .await
        .context("failed to load installed packages")?;
    let updates = proxy
        .check_updates()
        .await
        .context("failed to check for package updates")?;

    Ok(SystemSnapshot { installed, updates })
}

pub async fn search_catalog(query: &str) -> anyhow::Result<Vec<AppRecord>> {
    let connection = connect().await?;
    let proxy = PackageManagerProxy::new(&connection)
        .await
        .context("failed to create package-manager proxy")?;
    proxy
        .search_catalog(query)
        .await
        .context("failed to search the AppStream catalogue")
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
