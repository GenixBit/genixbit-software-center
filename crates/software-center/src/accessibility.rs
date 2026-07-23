use adw::prelude::*;
use gtk::{gio, prelude::*};

pub const REFRESH_ACCELERATOR: &str = "<Primary>r";
pub const SEARCH_ACCELERATOR: &str = "<Primary>f";
pub const QUIT_ACCELERATOR: &str = "<Primary>q";

/// Stable page destinations shared by the sidebar and application accelerators.
pub const NAVIGATION_PAGES: [(&str, &str); 10] = [
    ("dashboard", "<Alt>1"),
    ("discover", "<Alt>2"),
    ("installed", "<Alt>3"),
    ("updates", "<Alt>4"),
    ("activity", "<Alt>5"),
    ("stacks", "<Alt>6"),
    ("security", "<Alt>7"),
    ("services", "<Alt>8"),
    ("profiles", "<Alt>9"),
    ("settings", "<Alt>0"),
];

/// Pages whose primary search entry is eligible for the global Ctrl+F action.
pub const SEARCHABLE_PAGES: [&str; 6] = [
    "discover",
    "installed",
    "activity",
    "stacks",
    "security",
    "services",
];

/// Install application-level actions without introducing privileged behavior.
pub fn install_actions(
    application: &adw::Application,
    stack: &gtk::Stack,
    refresh_button: &gtk::Button,
    search_entries: Vec<(&'static str, gtk::SearchEntry)>,
) {
    debug_assert_eq!(search_entries.len(), SEARCHABLE_PAGES.len());
    debug_assert!(SEARCHABLE_PAGES.iter().all(|expected| {
        search_entries
            .iter()
            .any(|(page, _)| page == expected)
    }));

    let refresh_action = gio::SimpleAction::new("refresh", None);
    let refresh_button = refresh_button.clone();
    refresh_action.connect_activate(move |_, _| refresh_button.emit_clicked());
    application.add_action(&refresh_action);
    application.set_accels_for_action("app.refresh", &[REFRESH_ACCELERATOR]);

    let search_action = gio::SimpleAction::new("focus-search", None);
    let search_stack = stack.clone();
    search_action.connect_activate(move |_, _| {
        let Some(page_name) = search_stack.visible_child_name() else {
            return;
        };
        if let Some((_, entry)) = search_entries
            .iter()
            .find(|(page, _)| *page == page_name.as_str())
        {
            entry.grab_focus();
        }
    });
    application.add_action(&search_action);
    application.set_accels_for_action("app.focus-search", &[SEARCH_ACCELERATOR]);

    for (page, accelerator) in NAVIGATION_PAGES {
        let action_name = format!("show-{page}");
        let action = gio::SimpleAction::new(&action_name, None);
        let navigation_stack = stack.clone();
        action.connect_activate(move |_, _| navigation_stack.set_visible_child_name(page));
        application.add_action(&action);

        let detailed_action = format!("app.{action_name}");
        application.set_accels_for_action(&detailed_action, &[accelerator]);
    }

    let quit_action = gio::SimpleAction::new("quit", None);
    let quit_application = application.clone();
    quit_action.connect_activate(move |_, _| quit_application.quit());
    application.add_action(&quit_action);
    application.set_accels_for_action("app.quit", &[QUIT_ACCELERATOR]);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn navigation_pages_and_accelerators_are_unique() {
        let pages = NAVIGATION_PAGES
            .iter()
            .map(|(page, _)| *page)
            .collect::<BTreeSet<_>>();
        let accelerators = NAVIGATION_PAGES
            .iter()
            .map(|(_, accelerator)| *accelerator)
            .collect::<BTreeSet<_>>();

        assert_eq!(pages.len(), NAVIGATION_PAGES.len());
        assert_eq!(accelerators.len(), NAVIGATION_PAGES.len());
    }

    #[test]
    fn every_searchable_page_is_navigable() {
        let pages = NAVIGATION_PAGES
            .iter()
            .map(|(page, _)| *page)
            .collect::<BTreeSet<_>>();

        for page in SEARCHABLE_PAGES {
            assert!(pages.contains(page));
        }
    }

    #[test]
    fn global_accelerators_do_not_conflict_with_navigation() {
        let navigation = NAVIGATION_PAGES
            .iter()
            .map(|(_, accelerator)| *accelerator)
            .collect::<BTreeSet<_>>();

        assert!(!navigation.contains(REFRESH_ACCELERATOR));
        assert!(!navigation.contains(SEARCH_ACCELERATOR));
        assert!(!navigation.contains(QUIT_ACCELERATOR));
        assert_ne!(REFRESH_ACCELERATOR, SEARCH_ACCELERATOR);
        assert_ne!(REFRESH_ACCELERATOR, QUIT_ACCELERATOR);
        assert_ne!(SEARCH_ACCELERATOR, QUIT_ACCELERATOR);
    }
}
