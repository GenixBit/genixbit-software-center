pub const PAGE_MARGIN: i32 = 24;
pub const PAGE_SPACING: i32 = 12;
pub const CONTROL_SPACING: i32 = 8;
pub const SIDEBAR_WIDTH: i32 = 230;
pub const WINDOW_WIDTH: i32 = 1180;
pub const WINDOW_HEIGHT: i32 = 760;

pub const CSS: &str = include_str!("../../../data/com.genixbit.SoftwareCenter.css");

#[cfg(test)]
fn css_is_complete() -> bool {
    [
        ".genixbit-brand-title",
        ".genixbit-brand-subtitle",
        ".genixbit-status-success",
        ".genixbit-status-warning",
        ".genixbit-status-error",
        ".genixbit-card",
    ]
    .iter()
    .all(|selector| CSS.contains(selector))
}

#[cfg(test)]
mod tests {
    use super::{CSS, css_is_complete};

    #[test]
    fn packaged_css_contains_all_public_tokens() {
        assert!(css_is_complete());
        assert!(CSS.contains("@define-color genixbit_accent"));
        assert!(CSS.contains("@define-color genixbit_success"));
        assert!(CSS.contains("@define-color genixbit_warning"));
        assert!(CSS.contains("@define-color genixbit_error"));
    }
}
