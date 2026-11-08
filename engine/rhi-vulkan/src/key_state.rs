//! 持续按键状态（Player 键盘 API，task-58）
//!
//! 为 Player host / C# 第三人称角色控制提供"当前按下键集合":
//! - [`KeyState`]: 纯 Rust `HashSet<u32>` 状态机（不依赖窗口，可单测）
//! - [`keycode_to_u32`]: `hezhou_platform::KeyCode` → u32（Player/FFI 键码）
//! - 常用键码常量（与 `KeyCode` 数值一致）:
//!   A=1, D=4, S=19, W=23, Space=37, Escape=39
//!
//! 使用方式（Player host 每帧查询）:
//! ```text
//! // process_events() 内部按 GLFW Press/Repeat/Release 维护 key_state
//! if renderer.is_key_down(hezhou_platform::KeyCode::W as u32) {
//!     // 前进
//! }
//! ```
//!
//! **行为说明**:
//! - Press / Repeat → 置为按下（Repeat 幂等，不改变集合）
//! - Release → 移除（重复 Release 幂等）
//! - `KeyCode::Unknown`（0）视为无效键，永远不会被记录/查询为按下
//! - 窗口失焦不会自动清空；需要时调用 `clear_key_state()` / `KeyState::clear()`

use std::collections::HashSet;

use hezhou_platform::{KeyAction, KeyCode};

/// `KeyCode` → u32（FFI/Player 使用的键码）
pub fn keycode_to_u32(keycode: KeyCode) -> u32 {
    keycode as u32
}

/// KeyCode::A = 1
pub const KEY_A: u32 = KeyCode::A as u32;
/// KeyCode::D = 4
pub const KEY_D: u32 = KeyCode::D as u32;
/// KeyCode::S = 19
pub const KEY_S: u32 = KeyCode::S as u32;
/// KeyCode::W = 23
pub const KEY_W: u32 = KeyCode::W as u32;
/// KeyCode::Space = 37
pub const KEY_SPACE: u32 = KeyCode::Space as u32;
/// KeyCode::Escape = 39
pub const KEY_ESCAPE: u32 = KeyCode::Escape as u32;

/// 持续按键状态表
///
/// 由 `UIVulkanRenderer::process_events()` 在 Key Press/Repeat/Release 事件中维护，
/// Player host 每帧用 [`KeyState::is_down`] 查询。
#[derive(Clone, Debug, Default)]
pub struct KeyState {
    down: HashSet<u32>,
}

impl KeyState {
    /// 创建空按键状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 置为按下；返回该键是否从"未按下"变为"按下"（重复 Press/Repeat 返回 false）
    ///
    /// `keycode == 0`（`KeyCode::Unknown`）被忽略。
    pub fn press(&mut self, keycode: u32) -> bool {
        if keycode == 0 {
            return false;
        }
        self.down.insert(keycode)
    }

    /// 置为释放；返回该键此前是否处于按下状态（重复 Release 返回 false）
    pub fn release(&mut self, keycode: u32) -> bool {
        self.down.remove(&keycode)
    }

    /// 按键是否持续按下（`keycode == 0` 永远 false）
    pub fn is_down(&self, keycode: u32) -> bool {
        keycode != 0 && self.down.contains(&keycode)
    }

    /// 应用一次引擎键事件（Press/Repeat → press，Release → release）
    ///
    /// 返回状态是否发生变化（Repeat/重复 Release 不改变状态）。
    pub fn apply(&mut self, keycode: u32, action: KeyAction) -> bool {
        match action {
            KeyAction::Press | KeyAction::Repeat => self.press(keycode),
            KeyAction::Release => self.release(keycode),
        }
    }

    /// 清空全部按键（窗口失焦/切场景时调用）
    pub fn clear(&mut self) {
        self.down.clear();
    }

    /// 当前按下的键码列表（升序，便于日志/测试）
    pub fn pressed_keys(&self) -> Vec<u32> {
        let mut keys: Vec<u32> = self.down.iter().copied().collect();
        keys.sort_unstable();
        keys
    }

    /// 当前按下的键数量
    pub fn len(&self) -> usize {
        self.down.len()
    }

    /// 是否没有任何按键按下
    pub fn is_empty(&self) -> bool {
        self.down.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_repeat_release() {
        let mut state = KeyState::new();
        assert!(state.is_empty());
        assert!(!state.is_down(KEY_W));
        // Press
        assert!(state.press(KEY_W));
        assert!(state.is_down(KEY_W));
        assert_eq!(state.len(), 1);
        // Repeat 幂等
        assert!(!state.press(KEY_W));
        assert_eq!(state.len(), 1);
        // Release
        assert!(state.release(KEY_W));
        assert!(!state.is_down(KEY_W));
        assert!(state.is_empty());
    }

    #[test]
    fn duplicate_release_is_idempotent() {
        let mut state = KeyState::new();
        assert!(!state.release(KEY_A));
        state.press(KEY_A);
        assert!(state.release(KEY_A));
        assert!(!state.release(KEY_A));
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn multiple_keys_and_sorted_list() {
        let mut state = KeyState::new();
        state.press(KEY_SPACE);
        state.press(KEY_W);
        state.press(KEY_A);
        state.press(KEY_S);
        state.press(KEY_D);
        assert_eq!(state.len(), 5);
        assert_eq!(
            state.pressed_keys(),
            vec![KEY_A, KEY_D, KEY_S, KEY_W, KEY_SPACE]
        );
        // 与 KeyCode 契约数值一致
        assert_eq!(
            state.pressed_keys(),
            vec![KeyCode::A as u32, KeyCode::D as u32, KeyCode::S as u32, KeyCode::W as u32, KeyCode::Space as u32]
        );
    }

    #[test]
    fn unknown_keycode_is_ignored() {
        let mut state = KeyState::new();
        assert!(!state.press(0));
        assert!(!state.is_down(0));
        assert!(state.is_empty());
        assert!(!state.release(0));
        // 任意非 0 键码都可记录（调用方负责过滤 KeyCode::Unknown）
        assert!(state.press(999));
        assert!(state.is_down(999));
    }

    #[test]
    fn apply_key_actions() {
        let mut state = KeyState::new();
        assert!(state.apply(KEY_W, KeyAction::Press));
        assert!(state.is_down(KEY_W));
        // Repeat 不改变状态
        assert!(!state.apply(KEY_W, KeyAction::Repeat));
        assert!(state.is_down(KEY_W));
        assert!(state.apply(KEY_W, KeyAction::Release));
        assert!(!state.is_down(KEY_W));
        assert!(!state.apply(KEY_W, KeyAction::Release));
    }

    #[test]
    fn clear_removes_all_keys() {
        let mut state = KeyState::new();
        state.press(KEY_W);
        state.press(KEY_SPACE);
        state.clear();
        assert!(state.is_empty());
        assert!(!state.is_down(KEY_W));
        assert!(!state.is_down(KEY_SPACE));
    }

    #[test]
    fn keycode_contract_values() {
        // 任务约定：A=1,D=4,S=19,W=23,Space=37,Escape=39
        assert_eq!(keycode_to_u32(KeyCode::A), 1);
        assert_eq!(keycode_to_u32(KeyCode::D), 4);
        assert_eq!(keycode_to_u32(KeyCode::S), 19);
        assert_eq!(keycode_to_u32(KeyCode::W), 23);
        assert_eq!(keycode_to_u32(KeyCode::Space), 37);
        assert_eq!(keycode_to_u32(KeyCode::Escape), 39);
        assert_eq!(keycode_to_u32(KeyCode::Unknown), 0);
        assert_eq!(KEY_A, 1);
        assert_eq!(KEY_D, 4);
        assert_eq!(KEY_S, 19);
        assert_eq!(KEY_W, 23);
        assert_eq!(KEY_SPACE, 37);
        assert_eq!(KEY_ESCAPE, 39);
    }

    #[test]
    fn key_state_clone_is_independent() {
        let mut state = KeyState::new();
        state.press(KEY_W);
        let cloned = state.clone();
        state.release(KEY_W);
        assert!(!state.is_down(KEY_W));
        assert!(cloned.is_down(KEY_W));
    }
}
