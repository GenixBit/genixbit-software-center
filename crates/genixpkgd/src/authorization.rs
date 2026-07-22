use std::collections::HashMap;

use zbus::{Connection, Proxy, names::UniqueName, zvariant::Value};

const TEST_OVERRIDE_ENV: &str = "GENIXPKGD_ALLOW_TEST_TRANSACTIONS";
const POLKIT_SERVICE: &str = "org.freedesktop.PolicyKit1";
const POLKIT_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
const POLKIT_INTERFACE: &str = "org.freedesktop.PolicyKit1.Authority";
const TRANSACTION_CONTROL_ACTION: &str = "com.genixbit.PackageManager1.transaction-control";
const ALLOW_USER_INTERACTION: u32 = 1;

#[derive(Clone, Debug)]
pub struct AuthorizationHelper {
    session_bus: bool,
    allow_test_transactions: bool,
}

impl AuthorizationHelper {
    pub fn from_environment() -> Self {
        let session_bus = std::env::var("GENIXPKGD_BUS").is_ok_and(|value| value == "session");
        let allow_test_transactions = std::env::var(TEST_OVERRIDE_ENV)
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
        Self {
            session_bus,
            allow_test_transactions,
        }
    }

    pub async fn authorize_transaction_control(
        &self,
        connection: &Connection,
        sender: &UniqueName<'_>,
        operation: &str,
    ) -> zbus::fdo::Result<()> {
        if self.session_test_override_allowed() {
            return Ok(());
        }
        if self.session_bus {
            return Err(zbus::fdo::Error::AccessDenied(format!(
                "session-bus transaction control is disabled for {operation}; set {TEST_OVERRIDE_ENV}=1 only in an isolated test environment"
            )));
        }

        let proxy = Proxy::new(
            connection,
            POLKIT_SERVICE,
            POLKIT_PATH,
            POLKIT_INTERFACE,
        )
        .await
        .map_err(policykit_error)?;

        let mut subject_details = HashMap::new();
        subject_details.insert("name", Value::from(sender.as_str()));
        let subject = ("system-bus-name", subject_details);
        let details = HashMap::<&str, &str>::new();
        let (is_authorized, is_challenge, _result_details): (
            bool,
            bool,
            HashMap<String, String>,
        ) = proxy
            .call(
                "CheckAuthorization",
                &(
                    subject,
                    TRANSACTION_CONTROL_ACTION,
                    details,
                    ALLOW_USER_INTERACTION,
                    "",
                ),
            )
            .await
            .map_err(policykit_error)?;

        if is_authorized {
            Ok(())
        } else if is_challenge {
            Err(zbus::fdo::Error::AccessDenied(format!(
                "authentication is required for {operation}"
            )))
        } else {
            Err(zbus::fdo::Error::AccessDenied(format!(
                "the D-Bus caller is not authorized for {operation}"
            )))
        }
    }

    fn session_test_override_allowed(&self) -> bool {
        self.session_bus && self.allow_test_transactions
    }
}

fn policykit_error(error: impl std::fmt::Display) -> zbus::fdo::Error {
    zbus::fdo::Error::AccessDenied(format!(
        "PolicyKit authorization failed closed: {error}"
    ))
}

#[cfg(test)]
mod tests {
    use super::AuthorizationHelper;

    #[test]
    fn production_never_uses_the_test_override() {
        let helper = AuthorizationHelper {
            session_bus: false,
            allow_test_transactions: true,
        };
        assert!(!helper.session_test_override_allowed());
    }

    #[test]
    fn explicit_session_test_override_is_allowed() {
        let helper = AuthorizationHelper {
            session_bus: true,
            allow_test_transactions: true,
        };
        assert!(helper.session_test_override_allowed());
    }

    #[test]
    fn session_bus_without_override_fails_closed() {
        let helper = AuthorizationHelper {
            session_bus: true,
            allow_test_transactions: false,
        };
        assert!(!helper.session_test_override_allowed());
    }
}
