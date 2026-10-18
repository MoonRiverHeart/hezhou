use crate::theme::{self, ThemeMode};

#[unsafe(no_mangle)]
pub extern "C" fn ui_theme_get_mode() -> u32 {
    theme::mode() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_theme_set_mode(mode: u32) -> u32 {
    theme::set_mode(ThemeMode::from_u32(mode)) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_theme_color(role: u32) -> u32 {
    theme::pack_rgba(theme::color(role))
}

#[unsafe(no_mangle)]
pub extern "C" fn ui_theme_role_count() -> u32 {
    theme::ROLE_COUNT
}
