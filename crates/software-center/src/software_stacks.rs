use std::collections::HashSet;

pub const ALL_STACK_CATEGORIES: &str = "All categories";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackPackage {
    pub name: &'static str,
    pub role: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SoftwareStack {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub icon: &'static str,
    pub packages: &'static [StackPackage],
}

const AI_ML_PACKAGES: &[StackPackage] = &[
    StackPackage {
        name: "python3",
        role: "Python runtime",
    },
    StackPackage {
        name: "python3-pip",
        role: "Python package installer",
    },
    StackPackage {
        name: "python3-venv",
        role: "Isolated Python environments",
    },
    StackPackage {
        name: "git",
        role: "Source control",
    },
    StackPackage {
        name: "build-essential",
        role: "Native compiler toolchain",
    },
    StackPackage {
        name: "cmake",
        role: "Cross-platform build configuration",
    },
    StackPackage {
        name: "pkg-config",
        role: "Native library discovery",
    },
];

const WEB_PACKAGES: &[StackPackage] = &[
    StackPackage {
        name: "nodejs",
        role: "JavaScript runtime",
    },
    StackPackage {
        name: "npm",
        role: "JavaScript package manager",
    },
    StackPackage {
        name: "git",
        role: "Source control",
    },
    StackPackage {
        name: "nginx",
        role: "Local web server and reverse proxy",
    },
    StackPackage {
        name: "postgresql-client",
        role: "PostgreSQL command-line client",
    },
    StackPackage {
        name: "redis-tools",
        role: "Redis command-line tools",
    },
];

const DESKTOP_PACKAGES: &[StackPackage] = &[
    StackPackage {
        name: "build-essential",
        role: "Native compiler toolchain",
    },
    StackPackage {
        name: "meson",
        role: "Modern build system",
    },
    StackPackage {
        name: "ninja-build",
        role: "Fast build executor",
    },
    StackPackage {
        name: "libgtk-4-dev",
        role: "GTK4 development headers",
    },
    StackPackage {
        name: "libadwaita-1-dev",
        role: "Libadwaita development headers",
    },
    StackPackage {
        name: "flatpak-builder",
        role: "Desktop application packaging",
    },
];

const CLOUD_PACKAGES: &[StackPackage] = &[
    StackPackage {
        name: "podman",
        role: "Rootless container engine",
    },
    StackPackage {
        name: "buildah",
        role: "OCI image builder",
    },
    StackPackage {
        name: "skopeo",
        role: "Container image inspection",
    },
    StackPackage {
        name: "docker.io",
        role: "Docker-compatible container engine",
    },
    StackPackage {
        name: "kubernetes-client",
        role: "Kubernetes command-line client",
    },
    StackPackage {
        name: "ansible",
        role: "Infrastructure automation",
    },
];

const CREATIVE_PACKAGES: &[StackPackage] = &[
    StackPackage {
        name: "gimp",
        role: "Raster image editing",
    },
    StackPackage {
        name: "inkscape",
        role: "Vector graphics design",
    },
    StackPackage {
        name: "krita",
        role: "Digital painting",
    },
    StackPackage {
        name: "blender",
        role: "3D modelling and animation",
    },
    StackPackage {
        name: "ffmpeg",
        role: "Audio and video processing",
    },
    StackPackage {
        name: "audacity",
        role: "Audio editing",
    },
];

const PRODUCTIVITY_PACKAGES: &[StackPackage] = &[
    StackPackage {
        name: "libreoffice",
        role: "Office productivity suite",
    },
    StackPackage {
        name: "thunderbird",
        role: "Email and calendar client",
    },
    StackPackage {
        name: "keepassxc",
        role: "Password management",
    },
    StackPackage {
        name: "remmina",
        role: "Remote desktop client",
    },
    StackPackage {
        name: "syncthing",
        role: "Peer-to-peer file synchronization",
    },
];

const STACKS: &[SoftwareStack] = &[
    SoftwareStack {
        id: "ai-ml-foundation",
        title: "AI & ML Foundation",
        description: "Python, isolated environments, source control and native build tools for local AI development.",
        category: "AI & Data",
        icon: "applications-science-symbolic",
        packages: AI_ML_PACKAGES,
    },
    SoftwareStack {
        id: "web-development",
        title: "Web Development",
        description: "JavaScript, web serving, database clients and developer tooling for modern web applications.",
        category: "Development",
        icon: "applications-development-symbolic",
        packages: WEB_PACKAGES,
    },
    SoftwareStack {
        id: "native-desktop",
        title: "Native Desktop Development",
        description: "GTK4, Libadwaita, Meson and Flatpak tooling for native GenixBit OS applications.",
        category: "Development",
        icon: "applications-engineering-symbolic",
        packages: DESKTOP_PACKAGES,
    },
    SoftwareStack {
        id: "cloud-containers",
        title: "Cloud & Containers",
        description: "Container engines, image tooling, Kubernetes access and infrastructure automation.",
        category: "Cloud",
        icon: "network-server-symbolic",
        packages: CLOUD_PACKAGES,
    },
    SoftwareStack {
        id: "creative-studio",
        title: "Creative Studio",
        description: "Image, illustration, 3D, audio and video tools for a complete local creative workflow.",
        category: "Creative",
        icon: "applications-graphics-symbolic",
        packages: CREATIVE_PACKAGES,
    },
    SoftwareStack {
        id: "productivity-workspace",
        title: "Productivity Workspace",
        description: "Office, communications, credentials, remote access and file synchronization tools.",
        category: "Productivity",
        icon: "applications-office-symbolic",
        packages: PRODUCTIVITY_PACKAGES,
    },
];

pub fn software_stacks() -> &'static [SoftwareStack] {
    STACKS
}

pub fn stack_categories() -> Vec<&'static str> {
    let mut categories = vec![ALL_STACK_CATEGORIES];
    for stack in STACKS {
        if !categories.contains(&stack.category) {
            categories.push(stack.category);
        }
    }
    categories
}

pub fn find_stack(id: &str) -> Option<&'static SoftwareStack> {
    STACKS.iter().find(|stack| stack.id == id)
}

pub fn filter_stacks(query: &str, category: &str) -> Vec<&'static SoftwareStack> {
    let query = query.trim().to_ascii_lowercase();
    STACKS
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

pub fn stack_installed_count(stack: &SoftwareStack, installed_names: &HashSet<String>) -> usize {
    stack
        .packages
        .iter()
        .filter(|package| installed_names.contains(package.name))
        .count()
}

pub fn stack_filters_active(query: &str, category: &str) -> bool {
    !query.trim().is_empty()
        || (!category.is_empty() && category != ALL_STACK_CATEGORIES)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        ALL_STACK_CATEGORIES, filter_stacks, find_stack, software_stacks, stack_categories,
        stack_filters_active, stack_installed_count,
    };

    #[test]
    fn definitions_have_stable_unique_ids_and_packages() {
        let mut ids = HashSet::new();
        for stack in software_stacks() {
            assert!(ids.insert(stack.id));
            assert!(!stack.packages.is_empty());
            let mut packages = HashSet::new();
            for package in stack.packages {
                assert!(packages.insert(package.name));
                assert!(!package.role.trim().is_empty());
            }
        }
    }

    #[test]
    fn categories_preserve_editorial_order() {
        assert_eq!(
            stack_categories(),
            [
                ALL_STACK_CATEGORIES,
                "AI & Data",
                "Development",
                "Cloud",
                "Creative",
                "Productivity",
            ]
        );
    }

    #[test]
    fn filters_by_title_package_role_and_category() {
        assert_eq!(filter_stacks("machine", ALL_STACK_CATEGORIES)[0].id, "ai-ml-foundation");
        assert_eq!(filter_stacks("libgtk-4-dev", ALL_STACK_CATEGORIES)[0].id, "native-desktop");
        assert_eq!(filter_stacks("remote desktop", ALL_STACK_CATEGORIES)[0].id, "productivity-workspace");
        assert_eq!(filter_stacks("", "Cloud")[0].id, "cloud-containers");
        assert!(filter_stacks("blender", "Development").is_empty());
    }

    #[test]
    fn reports_installed_progress() {
        let stack = find_stack("web-development").expect("web stack");
        let installed = HashSet::from(["nodejs".to_owned(), "git".to_owned()]);
        assert_eq!(stack_installed_count(stack, &installed), 2);
    }

    #[test]
    fn detects_active_filters() {
        assert!(!stack_filters_active("", ALL_STACK_CATEGORIES));
        assert!(stack_filters_active("python", ALL_STACK_CATEGORIES));
        assert!(stack_filters_active("", "Creative"));
    }
}
