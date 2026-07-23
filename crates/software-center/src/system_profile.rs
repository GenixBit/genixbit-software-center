use std::collections::BTreeMap;

use genixbit_package_model::PackageRecord;

const PROFILE_HEADER: &str = "GENIXBIT-SYSTEM-PROFILE\t1";
const MAX_PROFILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PROFILE_PACKAGES: usize = 20_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfilePackage {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub section: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SystemProfile {
    pub packages: Vec<ProfilePackage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionChange {
    pub name: String,
    pub current_version: String,
    pub profile_version: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProfileComparison {
    pub install_missing: Vec<String>,
    pub remove_extra: Vec<String>,
    pub protected_extra: Vec<String>,
    pub version_changes: Vec<VersionChange>,
}

impl ProfileComparison {
    pub fn is_identical(&self) -> bool {
        self.install_missing.is_empty()
            && self.remove_extra.is_empty()
            && self.protected_extra.is_empty()
            && self.version_changes.is_empty()
    }

    pub fn action_count(&self) -> usize {
        self.install_missing.len() + self.remove_extra.len() + self.version_changes.len()
    }

    pub fn status_text(&self) -> String {
        if self.is_identical() {
            return "The imported profile matches the current installed-package state.".to_owned();
        }
        format!(
            "Restore preview: {} installs, {} removals, {} version changes and {} protected extra packages. No changes will be executed.",
            self.install_missing.len(),
            self.remove_extra.len(),
            self.version_changes.len(),
            self.protected_extra.len()
        )
    }
}

impl SystemProfile {
    pub fn from_packages(packages: &[PackageRecord]) -> Self {
        let packages = packages
            .iter()
            .map(|package| {
                (
                    package.name.clone(),
                    ProfilePackage {
                        name: package.name.clone(),
                        version: package.version.clone(),
                        architecture: package.architecture.clone(),
                        section: package.section.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect();
        Self { packages }
    }

    pub fn serialize(&self) -> String {
        let mut output = String::from(PROFILE_HEADER);
        output.push('\n');
        for package in &self.packages {
            output.push_str("P\t");
            output.push_str(&encode_field(&package.name));
            output.push('\t');
            output.push_str(&encode_field(&package.version));
            output.push('\t');
            output.push_str(&encode_field(&package.architecture));
            output.push('\t');
            output.push_str(&encode_field(&package.section));
            output.push('\n');
        }
        output
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        if input.len() > MAX_PROFILE_BYTES {
            return Err("Profile exceeds the 8 MiB safety limit.".to_owned());
        }
        let mut lines = input.lines();
        if lines.next() != Some(PROFILE_HEADER) {
            return Err("Unsupported or missing GenixBit system-profile header.".to_owned());
        }

        let mut packages = BTreeMap::new();
        for (index, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if packages.len() >= MAX_PROFILE_PACKAGES {
                return Err("Profile exceeds the 20,000-package safety limit.".to_owned());
            }
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 5 || fields[0] != "P" {
                return Err(format!("Malformed profile record on line {}.", index + 2));
            }
            let package = ProfilePackage {
                name: decode_field(fields[1])?,
                version: decode_field(fields[2])?,
                architecture: decode_field(fields[3])?,
                section: decode_field(fields[4])?,
            };
            if package.name.trim().is_empty() {
                return Err(format!("Empty package name on line {}.", index + 2));
            }
            if packages.insert(package.name.clone(), package).is_some() {
                return Err(format!("Duplicate package record on line {}.", index + 2));
            }
        }
        Ok(Self {
            packages: packages.into_values().collect(),
        })
    }
}

pub fn compare_profile(current: &[PackageRecord], profile: &SystemProfile) -> ProfileComparison {
    let current = current
        .iter()
        .map(|package| (package.name.as_str(), package))
        .collect::<BTreeMap<_, _>>();
    let target = profile
        .packages
        .iter()
        .map(|package| (package.name.as_str(), package))
        .collect::<BTreeMap<_, _>>();

    let mut comparison = ProfileComparison::default();
    for (name, package) in &target {
        match current.get(name) {
            None => comparison.install_missing.push((*name).to_owned()),
            Some(installed) if installed.version != package.version => {
                comparison.version_changes.push(VersionChange {
                    name: (*name).to_owned(),
                    current_version: installed.version.clone(),
                    profile_version: package.version.clone(),
                });
            }
            Some(_) => {}
        }
    }
    for (name, package) in current {
        if !target.contains_key(name) {
            if package.essential {
                comparison.protected_extra.push(name.to_owned());
            } else {
                comparison.remove_extra.push(name.to_owned());
            }
        }
    }
    comparison
}

fn encode_field(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'%' | b'\t' | b'\n' | b'\r' => output.push_str(&format!("%{byte:02X}")),
            _ => output.push(*byte as char),
        }
    }
    output
}

fn decode_field(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("Invalid percent escape in profile field.".to_owned());
            }
            let high = hex_value(bytes[index + 1])
                .ok_or_else(|| "Invalid percent escape in profile field.".to_owned())?;
            let low = hex_value(bytes[index + 2])
                .ok_or_else(|| "Invalid percent escape in profile field.".to_owned())?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).map_err(|_| "Profile field is not valid UTF-8.".to_owned())
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::PackageRecord;

    use super::{ProfilePackage, SystemProfile, compare_profile};

    fn package(name: &str, version: &str, essential: bool) -> PackageRecord {
        PackageRecord {
            name: name.to_owned(),
            version: version.to_owned(),
            architecture: "amd64".to_owned(),
            section: "utils".to_owned(),
            essential,
            ..PackageRecord::default()
        }
    }

    #[test]
    fn profile_round_trips_escaped_fields_and_sorts_packages() {
        let profile = SystemProfile {
            packages: vec![
                ProfilePackage {
                    name: "zeta".to_owned(),
                    version: "1%2".to_owned(),
                    architecture: "amd64".to_owned(),
                    section: "line\nbreak".to_owned(),
                },
                ProfilePackage {
                    name: "alpha".to_owned(),
                    version: "1".to_owned(),
                    architecture: "all".to_owned(),
                    section: "utils".to_owned(),
                },
            ],
        };
        let parsed = SystemProfile::parse(&profile.serialize()).expect("profile round trip");
        assert_eq!(parsed.packages[0].name, "alpha");
        assert_eq!(parsed.packages[1].version, "1%2");
        assert_eq!(parsed.packages[1].section, "line\nbreak");
    }

    #[test]
    fn comparison_protects_essential_packages() {
        let current = [
            package("base-files", "1", true),
            package("curl", "1", false),
            package("git", "2", false),
        ];
        let profile = SystemProfile {
            packages: vec![
                ProfilePackage {
                    name: "curl".to_owned(),
                    version: "2".to_owned(),
                    architecture: "amd64".to_owned(),
                    section: "utils".to_owned(),
                },
                ProfilePackage {
                    name: "nano".to_owned(),
                    version: "1".to_owned(),
                    architecture: "amd64".to_owned(),
                    section: "editors".to_owned(),
                },
            ],
        };
        let comparison = compare_profile(&current, &profile);
        assert_eq!(comparison.install_missing, ["nano"]);
        assert_eq!(comparison.remove_extra, ["git"]);
        assert_eq!(comparison.protected_extra, ["base-files"]);
        assert_eq!(comparison.version_changes[0].name, "curl");
        assert_eq!(comparison.action_count(), 3);
    }

    #[test]
    fn rejects_bad_header_duplicate_and_invalid_escape() {
        assert!(SystemProfile::parse("wrong\n").is_err());
        assert!(SystemProfile::parse("GENIXBIT-SYSTEM-PROFILE\t1\nP\ta\t1\tall\tutils\nP\ta\t2\tall\tutils\n").is_err());
        assert!(SystemProfile::parse("GENIXBIT-SYSTEM-PROFILE\t1\nP\ta%ZZ\t1\tall\tutils\n").is_err());
    }
}
