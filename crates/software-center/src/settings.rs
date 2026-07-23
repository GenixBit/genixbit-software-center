use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

const SETTINGS_FILE: &str = "settings.conf";
const SETTINGS_DIRECTORY: &str = "genixbit-software-center";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSettings {
    pub offline_mode: bool,
    pub refresh_on_startup: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            offline_mode: true,
            refresh_on_startup: true,
        }
    }
}

impl AppSettings {
    pub fn parse(input: &str) -> Self {
        let mut settings = Self::default();
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let Some(value) = parse_bool(value.trim()) else {
                continue;
            };
            match key.trim() {
                "offline_mode" => settings.offline_mode = value,
                "refresh_on_startup" => settings.refresh_on_startup = value,
                _ => {}
            }
        }
        settings
    }

    pub fn serialize(&self) -> String {
        format!(
            "# GenixBit Software Center user settings\noffline_mode={}\nrefresh_on_startup={}\n",
            self.offline_mode, self.refresh_on_startup
        )
    }

    pub fn external_network_allowed(&self) -> bool {
        !self.offline_mode
    }

    pub fn policy_text(&self) -> String {
        let network = if self.offline_mode {
            "External network providers are blocked; local system metadata remains available."
        } else {
            "External providers may be used when a future feature explicitly configures one."
        };
        let startup = if self.refresh_on_startup {
            "Local metadata refreshes automatically when the application starts."
        } else {
            "Startup refresh is disabled; use the header refresh button to load local metadata."
        };
        format!("{network} {startup}")
    }
}

pub fn settings_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("GENIXBIT_SOFTWARE_CENTER_SETTINGS") {
        return Some(PathBuf::from(path));
    }
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Some(
            PathBuf::from(config_home)
                .join(SETTINGS_DIRECTORY)
                .join(SETTINGS_FILE),
        );
    }
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join(SETTINGS_DIRECTORY)
            .join(SETTINGS_FILE)
    })
}

pub fn load_settings(path: Option<&Path>) -> AppSettings {
    path.and_then(|path| fs::read_to_string(path).ok())
        .map(|contents| AppSettings::parse(&contents))
        .unwrap_or_default()
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("conf.tmp");
    fs::write(&temporary, settings.serialize())?;
    fs::rename(temporary, path)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::AppSettings;

    #[test]
    fn defaults_to_offline_with_startup_refresh() {
        assert_eq!(
            AppSettings::default(),
            AppSettings {
                offline_mode: true,
                refresh_on_startup: true,
            }
        );
    }

    #[test]
    fn parses_supported_boolean_forms_and_ignores_unknown_keys() {
        let settings = AppSettings::parse(
            "offline_mode=no\nrefresh_on_startup=0\nfuture_setting=true\ninvalid\n",
        );
        assert!(!settings.offline_mode);
        assert!(!settings.refresh_on_startup);
        assert!(settings.external_network_allowed());
    }

    #[test]
    fn serialization_round_trips() {
        let settings = AppSettings {
            offline_mode: false,
            refresh_on_startup: false,
        };
        assert_eq!(AppSettings::parse(&settings.serialize()), settings);
    }

    #[test]
    fn policy_text_explains_both_controls() {
        let offline = AppSettings::default().policy_text();
        assert!(offline.contains("External network providers are blocked"));
        assert!(offline.contains("refreshes automatically"));

        let manual = AppSettings {
            offline_mode: false,
            refresh_on_startup: false,
        }
        .policy_text();
        assert!(manual.contains("future feature"));
        assert!(manual.contains("header refresh button"));
    }
}
