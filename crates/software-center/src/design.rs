pub const APP_ICON_NAME: &str = "com.genixbit.SoftwareCenter";
pub const APPLICATION_CSS: &str = include_str!("../../../data/style.css");

pub fn install() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_data(APPLICATION_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

#[cfg(test)]
mod tests {
    use super::{APP_ICON_NAME, APPLICATION_CSS};

    #[test]
    fn icon_name_matches_desktop_application_id() {
        assert_eq!(APP_ICON_NAME, "com.genixbit.SoftwareCenter");
    }

    #[test]
    fn application_css_defines_core_brand_tokens() {
        for token in [
            "genixbit_brand",
            "genixbit_brand_alt",
            "genixbit_success",
            "genixbit_warning",
            "genixbit_error",
        ] {
            assert!(APPLICATION_CSS.contains(token), "missing CSS token: {token}");
        }
        assert!(APPLICATION_CSS.contains("label.success"));
        assert!(APPLICATION_CSS.contains("label.error"));
    }
}
