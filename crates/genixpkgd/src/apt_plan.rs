use anyhow::{bail, ensure};
use tokio::process::Command;

const APT_GET: &str = "apt-get";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AptCommandPlan {
    operation: String,
    package: String,
    arguments: Vec<String>,
    simulation: bool,
}

impl AptCommandPlan {
    pub fn simulation(operation: &str, package: &str) -> anyhow::Result<Self> {
        validate_package_name(package)?;
        let mut arguments = vec!["--simulate".to_owned()];
        match operation {
            "install" => arguments.extend(["install".to_owned(), package.to_owned()]),
            "remove" => arguments.extend(["remove".to_owned(), package.to_owned()]),
            "upgrade" => arguments.extend([
                "install".to_owned(),
                "--only-upgrade".to_owned(),
                package.to_owned(),
            ]),
            _ => bail!("unsupported APT operation {operation}"),
        }

        Ok(Self {
            operation: operation.to_owned(),
            package: package.to_owned(),
            arguments,
            simulation: true,
        })
    }

    pub fn command(&self) -> anyhow::Result<Command> {
        ensure!(
            self.simulation,
            "mutating APT execution plans are disabled in this milestone"
        );
        ensure!(
            self.arguments
                .first()
                .is_some_and(|argument| argument == "--simulate"),
            "APT execution plan is missing the mandatory simulation guard"
        );

        let mut command = Command::new(APT_GET);
        command
            .args(&self.arguments)
            .env("LC_ALL", "C")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .kill_on_drop(true);
        Ok(command)
    }

    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn package(&self) -> &str {
        &self.package
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
}

fn validate_package_name(package: &str) -> anyhow::Result<()> {
    let mut characters = package.chars();
    let valid = package.len() <= 128
        && characters
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.' | ':')
        });
    ensure!(valid, "invalid Debian package name");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::AptCommandPlan;

    #[test]
    fn creates_fixed_install_remove_and_upgrade_arguments() {
        let install =
            AptCommandPlan::simulation("install", "curl").expect("install plan should be created");
        assert_eq!(install.operation(), "install");
        assert_eq!(install.package(), "curl");
        assert_eq!(install.arguments(), ["--simulate", "install", "curl"]);

        let remove =
            AptCommandPlan::simulation("remove", "nano").expect("remove plan should be created");
        assert_eq!(remove.arguments(), ["--simulate", "remove", "nano"]);

        let upgrade = AptCommandPlan::simulation("upgrade", "libgtk-4-1:amd64")
            .expect("upgrade plan should be created");
        assert_eq!(
            upgrade.arguments(),
            [
                "--simulate",
                "install",
                "--only-upgrade",
                "libgtk-4-1:amd64"
            ]
        );
    }

    #[test]
    fn rejects_shell_and_option_injection() {
        for package in [
            "",
            "../curl",
            "curl;reboot",
            "$(id)",
            "--allow-unauthenticated",
        ] {
            assert!(
                AptCommandPlan::simulation("install", package).is_err(),
                "{package}"
            );
        }
    }

    #[test]
    fn rejects_unknown_operations() {
        assert!(AptCommandPlan::simulation("refresh", "curl").is_err());
    }

    #[test]
    fn refuses_a_plan_without_the_simulation_guard() {
        let plan = AptCommandPlan {
            operation: "install".to_owned(),
            package: "curl".to_owned(),
            arguments: vec!["install".to_owned(), "curl".to_owned()],
            simulation: false,
        };
        assert!(plan.command().is_err());
    }
}
