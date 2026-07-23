use std::collections::HashSet;

use genixbit_package_model::PackageRecord;

pub const ALL_STACK_CATEGORIES: &str = "All categories";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StackPackage {
    pub name: &'static str,
    pub role: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoftwareStack {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub icon: &'static str,
    pub packages: &'static [StackPackage],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StackStatus {
    pub installed: usize,
    pub total: usize,
}

impl StackStatus {
    pub fn status_text(&self) -> String {
        if self.total == 0 {
            return "No packages defined".to_owned();
        }
        if self.installed == self.total {
            return "Complete".to_owned();
        }
        if self.installed == 0 {
            return "Not installed".to_owned();
        }
        format!("{} of {} installed", self.installed, self.total)
    }
}

const AI_PACKAGES: &[StackPackage] = &[
    StackPackage { name: "python3", role: "Python runtime" },
    StackPackage { name: "python3-venv", role: "Virtual environments" },
    StackPackage { name: "python3-pip", role: "Python package installer" },
    StackPackage { name: "git", role: "Source control" },
];
const WEB_PACKAGES: &[StackPackage] = &[
    StackPackage { name: "nodejs", role: "JavaScript runtime" },
    StackPackage { name: "npm", role: "JavaScript package manager" },
    StackPackage { name: "git", role: "Source control" },
    StackPackage { name: "build-essential", role: "Native build toolchain" },
];
const NATIVE_PACKAGES: &[StackPackage] = &[
    StackPackage { name: "build-essential", role: "Compiler and build tools" },
    StackPackage { name: "cmake", role: "Cross-platform build system" },
    StackPackage { name: "pkg-config", role: "Library metadata lookup" },
    StackPackage { name: "gdb", role: "Native debugger" },
];
const CREATIVE_PACKAGES: &[StackPackage] = &[
    StackPackage { name: "gimp", role: "Raster image editor" },
    StackPackage { name: "inkscape", role: "Vector graphics editor" },
    StackPackage { name: "blender", role: "3D creation suite" },
];
const PRODUCTIVITY_PACKAGES: &[StackPackage] = &[
    StackPackage { name: "libreoffice", role: "Office suite" },
    StackPackage { name: "evince", role: "Document viewer" },
    StackPackage { name: "file-roller", role: "Archive manager" },
];

pub fn software_stacks() -> &'static [SoftwareStack] {
    &[
        SoftwareStack { id: "ai-python", title: "AI & Python", description: "A local Python foundation for machine learning, automation and data work.", category: "AI", icon: "applications-science-symbolic", packages: AI_PACKAGES },
        SoftwareStack { id: "web-development", title: "Web Development", description: "JavaScript, source control and native build tools for modern web applications.", category: "Development", icon: "applications-engineering-symbolic", packages: WEB_PACKAGES },
        SoftwareStack { id: "native-development", title: "Native Development", description: "Compiler, debugger and build-system essentials for native Linux software.", category: "Development", icon: "applications-system-symbolic", packages: NATIVE_PACKAGES },
        SoftwareStack { id: "creative-studio", title: "Creative Studio", description: "Image, vector and 3D creation applications for visual production.", category: "Design", icon: "applications-graphics-symbolic", packages: CREATIVE_PACKAGES },
        SoftwareStack { id: "productivity", title: "Productivity", description: "Office, document and archive tools for everyday work.", category: "Productivity", icon: "applications-office-symbolic", packages: PRODUCTIVITY_PACKAGES },
    ]
}

pub fn installed_names(packages: &[PackageRecord]) -> HashSet<&str> {
    packages.iter().map(|package| package.name.as_str()).collect()
}

pub fn stack_status(stack: &SoftwareStack, installed: &HashSet<&str>) -> StackStatus {
    StackStatus {
        installed: stack.packages.iter().filter(|package| installed.contains(package.name)).count(),
        total: stack.packages.len(),
    }
}

pub fn filter_stacks<'a>(query: &str, category: &str) -> Vec<&'a SoftwareStack> {
    let query = query.trim().to_ascii_lowercase();
    software_stacks()
        .iter()
        .filter(|stack| {
            let category_matches = category.is_empty()
                || category == ALL_STACK_CATEGORIES
                || stack.category == category;
            let query_matches = query.is_empty()
                || stack.title.to_ascii_lowercase().contains(&query)
                || stack.description.to_ascii_lowercase().contains(&query)
                || stack.category.to_ascii_lowercase().contains(&query)
                || stack.packages.iter().any(|package| {
                    package.name.to_ascii_lowercase().contains(&query)
                        || package.role.to_ascii_lowercase().contains(&query)
                });
            category_matches && query_matches
        })
        .collect()
}

pub fn filters_active(query: &str, category: &str) -> bool {
    !query.trim().is_empty()
        || (!category.is_empty() && category != ALL_STACK_CATEGORIES)
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::PackageRecord;

    use super::*;

    fn package(name: &str) -> PackageRecord {
        PackageRecord { name: name.to_owned(), ..PackageRecord::default() }
    }

    #[test]
    fn reports_partial_stack_progress() {
        let packages = [package("python3"), package("git")];
        let names = installed_names(&packages);
        let status = stack_status(&software_stacks()[0], &names);
        assert_eq!(status, StackStatus { installed: 2, total: 4 });
        assert_eq!(status.status_text(), "2 of 4 installed");
    }

    #[test]
    fn filters_by_package_role_and_category() {
        assert_eq!(filter_stacks("debugger", ALL_STACK_CATEGORIES)[0].id, "native-development");
        assert_eq!(filter_stacks("", "Design")[0].id, "creative-studio");
    }

    #[test]
    fn detects_active_filters() {
        assert!(!filters_active("", ALL_STACK_CATEGORIES));
        assert!(filters_active("python", ALL_STACK_CATEGORIES));
        assert!(filters_active("", "Development"));
    }
}
