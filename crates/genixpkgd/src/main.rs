use anyhow::Context;
use zbus::{connection, interface};

const BUS_NAME: &str = "com.genixbit.PackageManager1";
const OBJECT_PATH: &str = "/com/genixbit/PackageManager1";

#[derive(Debug, Default)]
struct PackageManager;

#[interface(name = "com.genixbit.PackageManager1")]
impl PackageManager {
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    async fn list_installed(&self) -> Vec<String> {
        Vec::new()
    }

    async fn check_updates(&self) -> Vec<String> {
        Vec::new()
    }

    async fn install(&self, package: &str) -> zbus::fdo::Result<String> {
        validate_package_name(package)?;
        Err(zbus::fdo::Error::NotSupported(
            "APT transactions are not enabled in the foundation release".to_owned(),
        ))
    }

    async fn remove(&self, package: &str) -> zbus::fdo::Result<String> {
        validate_package_name(package)?;
        Err(zbus::fdo::Error::NotSupported(
            "APT transactions are not enabled in the foundation release".to_owned(),
        ))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "genixpkgd=info".into()),
        )
        .init();

    let use_session_bus = std::env::var("GENIXPKGD_BUS").is_ok_and(|value| value == "session");
    let builder = if use_session_bus {
        connection::Builder::session().context("failed to connect to the session D-Bus")?
    } else {
        connection::Builder::system().context("failed to connect to the system D-Bus")?
    };

    let _connection = builder
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, PackageManager)?
        .build()
        .await
        .context("failed to publish the GenixBit package-management service")?;

    tracing::info!(bus = BUS_NAME, path = OBJECT_PATH, "genixpkgd is running");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

fn validate_package_name(package: &str) -> zbus::fdo::Result<()> {
    let valid = !package.is_empty()
        && package.len() <= 128
        && package.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.' | ':')
        });

    if valid {
        Ok(())
    } else {
        Err(zbus::fdo::Error::InvalidArgs(
            "invalid package name".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::validate_package_name;

    #[test]
    fn accepts_debian_package_names() {
        for value in ["curl", "libgtk-4-1", "g++", "python3.13", "pkg:amd64"] {
            assert!(validate_package_name(value).is_ok(), "{value}");
        }
    }

    #[test]
    fn rejects_shell_and_path_input() {
        for value in ["", "../curl", "curl;reboot", "$(id)", "package name"] {
            assert!(validate_package_name(value).is_err(), "{value}");
        }
    }
}
