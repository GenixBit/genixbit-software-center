const TEST_OVERRIDE_ENV: &str = "GENIXPKGD_ALLOW_TEST_TRANSACTIONS";

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

    pub fn authorize_transaction_control(&self, action: &str) -> zbus::fdo::Result<()> {
        if self.session_bus && self.allow_test_transactions {
            return Ok(());
        }

        Err(zbus::fdo::Error::AccessDenied(format!(
            "PolicyKit authorization is required for {action}; transaction control remains fail-closed until caller identity verification is enabled"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::AuthorizationHelper;

    #[test]
    fn production_authorization_is_fail_closed() {
        let helper = AuthorizationHelper {
            session_bus: false,
            allow_test_transactions: true,
        };
        assert!(helper.authorize_transaction_control("queue").is_err());
    }

    #[test]
    fn explicit_session_test_override_is_allowed() {
        let helper = AuthorizationHelper {
            session_bus: true,
            allow_test_transactions: true,
        };
        assert!(helper.authorize_transaction_control("queue").is_ok());
    }
}
