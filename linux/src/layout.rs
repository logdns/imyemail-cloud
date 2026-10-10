pub const APP_ID: &str = "email.imy.cloud";
pub const BRAND: &str = "imyemail-cloud-native";
pub const SUPPORT_URL: &str = "mailto:hello@imy.email?subject=imyemail-cloud%20Linux%20Issue";
pub const TERMS_URL: &str = "https://imy.email";
#[cfg_attr(not(feature = "gtk"), allow(dead_code))]
pub const NAV_MIN: i32 = 240;
pub const NAV_WIDTH: i32 = 280;
#[cfg_attr(not(feature = "gtk"), allow(dead_code))]
pub const NAV_MAX: i32 = 320;
#[cfg_attr(not(feature = "gtk"), allow(dead_code))]
pub const LIST_MIN: i32 = 360;
pub const LIST_WIDTH: i32 = 420;
#[cfg_attr(not(feature = "gtk"), allow(dead_code))]
pub const LIST_MAX: i32 = 480;
#[allow(dead_code)]
pub const BRAND_COLOR: &str = "#3D6BFE";

#[allow(dead_code)]
pub fn layout_hint() -> &'static str {
    crate::i18n::t(
        "A 导航 240-320px | B 列表 360-480px | C 阅读 弹性剩余 · 添加账号/写信走 MailEngine",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_three_pane_widths() {
        assert!((240..=320).contains(&NAV_WIDTH));
        assert!((360..=480).contains(&LIST_WIDTH));
        assert_eq!(APP_ID, "email.imy.cloud");
        assert_eq!(BRAND, "imyemail-cloud-native");
        assert_eq!(SUPPORT_URL, "mailto:hello@imy.email?subject=imyemail-cloud%20Linux%20Issue");
        assert_eq!(TERMS_URL, "https://imy.email");
    }
}
