//! 编辑器主题与语义颜色 Token。
//!
//! C# 只使用 `ThemeRole` 语义角色，不直接写 RGBA。切换主题时 Rust 侧
//! 返回对应调色板，C# 重建编辑器 UI 即可完成运行时切换。

use crate::types::Color;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ThemeMode {
    Dark = 0,
    Light = 1,
}

impl ThemeMode {
    pub fn from_u32(value: u32) -> Self {
        if value == 1 { Self::Light } else { Self::Dark }
    }
}

/// 语义颜色角色。数值必须与 C# `UI.ThemeRole` 保持一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ThemeRole {
    Canvas = 0,
    Panel = 1,
    Elevated = 2,
    Hover = 3,
    Active = 4,
    Border = 5,
    BorderStrong = 6,
    TextPrimary = 7,
    TextSecondary = 8,
    TextMuted = 9,
    TextDisabled = 10,
    Accent = 11,
    AccentHover = 12,
    AccentMuted = 13,
    Success = 14,
    SuccessHover = 15,
    Warning = 16,
    Danger = 17,
    DangerHover = 18,
    SelectionBg = 19,
    SelectionFg = 20,
    ButtonPrimaryBg = 21,
    ButtonPrimaryHover = 22,
    ButtonPrimaryPressed = 23,
    ButtonSecondaryBg = 24,
    ButtonSecondaryHover = 25,
    ButtonSecondaryPressed = 26,
    InputBg = 27,
    InputBorder = 28,
    InputFocus = 29,
    TabBg = 30,
    TabActiveBg = 31,
    TabActiveFg = 32,
    ScrollbarTrack = 33,
    ScrollbarThumb = 34,
    ScrollbarThumbHover = 35,
    ToolbarBg = 36,
    MenuBg = 37,
    MenuHover = 38,
    TooltipBg = 39,
    TooltipFg = 40,
    Overlay = 41,
    Shadow = 42,
    TextOnAccent = 43,
}

pub const ROLE_COUNT: u32 = 44;

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub canvas: Color,
    pub panel: Color,
    pub elevated: Color,
    pub hover: Color,
    pub active: Color,
    pub border: Color,
    pub border_strong: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_disabled: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_muted: Color,
    pub success: Color,
    pub success_hover: Color,
    pub warning: Color,
    pub danger: Color,
    pub danger_hover: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub button_primary_bg: Color,
    pub button_primary_hover: Color,
    pub button_primary_pressed: Color,
    pub button_secondary_bg: Color,
    pub button_secondary_hover: Color,
    pub button_secondary_pressed: Color,
    pub input_bg: Color,
    pub input_border: Color,
    pub input_focus: Color,
    pub tab_bg: Color,
    pub tab_active_bg: Color,
    pub tab_active_fg: Color,
    pub scrollbar_track: Color,
    pub scrollbar_thumb: Color,
    pub scrollbar_thumb_hover: Color,
    pub toolbar_bg: Color,
    pub menu_bg: Color,
    pub menu_hover: Color,
    pub tooltip_bg: Color,
    pub tooltip_fg: Color,
    pub overlay: Color,
    pub shadow: Color,
    pub text_on_accent: Color,
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    }
}

pub const DARK: ThemeColors = ThemeColors {
    canvas: rgba(14, 17, 22, 255),
    panel: rgba(21, 26, 33, 255),
    elevated: rgba(28, 35, 44, 255),
    hover: rgba(35, 44, 55, 255),
    active: rgba(42, 53, 66, 255),
    border: rgba(42, 52, 65, 255),
    border_strong: rgba(58, 70, 86, 255),
    text_primary: rgba(230, 237, 243, 255),
    text_secondary: rgba(157, 167, 179, 255),
    text_muted: rgba(110, 118, 129, 255),
    text_disabled: rgba(77, 85, 96, 255),
    accent: rgba(76, 141, 255, 255),
    accent_hover: rgba(108, 161, 255, 255),
    accent_muted: rgba(45, 78, 140, 255),
    success: rgba(63, 185, 80, 255),
    success_hover: rgba(88, 207, 104, 255),
    warning: rgba(240, 163, 58, 255),
    danger: rgba(229, 83, 75, 255),
    danger_hover: rgba(240, 113, 106, 255),
    selection_bg: rgba(37, 66, 112, 255),
    selection_fg: rgba(230, 237, 243, 255),
    button_primary_bg: rgba(47, 111, 235, 255),
    button_primary_hover: rgba(76, 141, 255, 255),
    button_primary_pressed: rgba(35, 86, 190, 255),
    button_secondary_bg: rgba(35, 44, 55, 255),
    button_secondary_hover: rgba(45, 56, 69, 255),
    button_secondary_pressed: rgba(28, 35, 44, 255),
    input_bg: rgba(16, 20, 26, 255),
    input_border: rgba(58, 70, 86, 255),
    input_focus: rgba(76, 141, 255, 255),
    tab_bg: rgba(21, 26, 33, 255),
    tab_active_bg: rgba(28, 35, 44, 255),
    tab_active_fg: rgba(230, 237, 243, 255),
    scrollbar_track: rgba(21, 26, 33, 255),
    scrollbar_thumb: rgba(58, 70, 86, 255),
    scrollbar_thumb_hover: rgba(82, 96, 115, 255),
    toolbar_bg: rgba(14, 17, 22, 255),
    menu_bg: rgba(28, 35, 44, 255),
    menu_hover: rgba(42, 53, 66, 255),
    tooltip_bg: rgba(28, 35, 44, 255),
    tooltip_fg: rgba(230, 237, 243, 255),
    overlay: rgba(0, 0, 0, 140),
    shadow: rgba(0, 0, 0, 90),
    text_on_accent: rgba(255, 255, 255, 255),
};

pub const LIGHT: ThemeColors = ThemeColors {
    canvas: rgba(245, 247, 250, 255),
    panel: rgba(255, 255, 255, 255),
    elevated: rgba(255, 255, 255, 255),
    hover: rgba(238, 242, 247, 255),
    active: rgba(226, 232, 240, 255),
    border: rgba(216, 222, 231, 255),
    border_strong: rgba(188, 197, 209, 255),
    text_primary: rgba(31, 41, 51, 255),
    text_secondary: rgba(91, 103, 115, 255),
    text_muted: rgba(138, 149, 163, 255),
    text_disabled: rgba(176, 184, 194, 255),
    accent: rgba(47, 111, 235, 255),
    accent_hover: rgba(35, 92, 200, 255),
    accent_muted: rgba(209, 225, 252, 255),
    success: rgba(26, 127, 55, 255),
    success_hover: rgba(22, 108, 47, 255),
    warning: rgba(178, 106, 0, 255),
    danger: rgba(207, 34, 46, 255),
    danger_hover: rgba(180, 25, 36, 255),
    selection_bg: rgba(209, 225, 252, 255),
    selection_fg: rgba(31, 41, 51, 255),
    button_primary_bg: rgba(47, 111, 235, 255),
    button_primary_hover: rgba(35, 92, 200, 255),
    button_primary_pressed: rgba(28, 74, 160, 255),
    button_secondary_bg: rgba(238, 242, 247, 255),
    button_secondary_hover: rgba(226, 232, 240, 255),
    button_secondary_pressed: rgba(216, 222, 231, 255),
    input_bg: rgba(255, 255, 255, 255),
    input_border: rgba(216, 222, 231, 255),
    input_focus: rgba(47, 111, 235, 255),
    tab_bg: rgba(245, 247, 250, 255),
    tab_active_bg: rgba(255, 255, 255, 255),
    tab_active_fg: rgba(31, 41, 51, 255),
    scrollbar_track: rgba(245, 247, 250, 255),
    scrollbar_thumb: rgba(188, 197, 209, 255),
    scrollbar_thumb_hover: rgba(138, 149, 163, 255),
    toolbar_bg: rgba(255, 255, 255, 255),
    menu_bg: rgba(255, 255, 255, 255),
    menu_hover: rgba(238, 242, 247, 255),
    tooltip_bg: rgba(31, 41, 51, 255),
    tooltip_fg: rgba(255, 255, 255, 255),
    overlay: rgba(0, 0, 0, 80),
    shadow: rgba(0, 0, 0, 30),
    text_on_accent: rgba(255, 255, 255, 255),
};

static THEME_MODE: AtomicU8 = AtomicU8::new(ThemeMode::Dark as u8);

pub fn mode() -> ThemeMode {
    ThemeMode::from_u32(THEME_MODE.load(Ordering::Relaxed) as u32)
}

pub fn set_mode(next: ThemeMode) -> ThemeMode {
    THEME_MODE.store(next as u8, Ordering::Relaxed);
    next
}

pub fn colors() -> &'static ThemeColors {
    match mode() {
        ThemeMode::Dark => &DARK,
        ThemeMode::Light => &LIGHT,
    }
}

pub fn color(role: u32) -> Color {
    let t = colors();
    match role {
        0 => t.canvas,
        1 => t.panel,
        2 => t.elevated,
        3 => t.hover,
        4 => t.active,
        5 => t.border,
        6 => t.border_strong,
        7 => t.text_primary,
        8 => t.text_secondary,
        9 => t.text_muted,
        10 => t.text_disabled,
        11 => t.accent,
        12 => t.accent_hover,
        13 => t.accent_muted,
        14 => t.success,
        15 => t.success_hover,
        16 => t.warning,
        17 => t.danger,
        18 => t.danger_hover,
        19 => t.selection_bg,
        20 => t.selection_fg,
        21 => t.button_primary_bg,
        22 => t.button_primary_hover,
        23 => t.button_primary_pressed,
        24 => t.button_secondary_bg,
        25 => t.button_secondary_hover,
        26 => t.button_secondary_pressed,
        27 => t.input_bg,
        28 => t.input_border,
        29 => t.input_focus,
        30 => t.tab_bg,
        31 => t.tab_active_bg,
        32 => t.tab_active_fg,
        33 => t.scrollbar_track,
        34 => t.scrollbar_thumb,
        35 => t.scrollbar_thumb_hover,
        36 => t.toolbar_bg,
        37 => t.menu_bg,
        38 => t.menu_hover,
        39 => t.tooltip_bg,
        40 => t.tooltip_fg,
        41 => t.overlay,
        42 => t.shadow,
        43 => t.text_on_accent,
        _ => t.canvas,
    }
}

/// 打包为 RGBA8888，C# 通过 `ui_theme_color` 解包。
pub fn pack_rgba(color: Color) -> u32 {
    let r = (color.r.clamp(0.0, 1.0) * 255.0).round() as u32;
    let g = (color.g.clamp(0.0, 1.0) * 255.0).round() as u32;
    let b = (color.b.clamp(0.0, 1.0) * 255.0).round() as u32;
    let a = (color.a.clamp(0.0, 1.0) * 255.0).round() as u32;
    (r << 24) | (g << 16) | (b << 8) | a
}
