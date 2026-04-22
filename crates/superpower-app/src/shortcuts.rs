use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use winit::keyboard::{KeyCode, ModifiersState};

/// 快捷键触发的动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShortcutAction {
    Copy,
    Paste,
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    ToggleSettings,
    SwitchTheme,
    Search,
    SearchNext,
    SearchPrevious,
}

/// 快捷键绑定
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Shortcut {
    pub fn new(key: &str, ctrl: bool, shift: bool, alt: bool) -> Self {
        Self {
            key: key.to_uppercase(),
            ctrl,
            shift,
            alt,
        }
    }

    /// 检查当前按键事件是否匹配此快捷键
    pub fn matches(&self, keycode: KeyCode, modifiers: ModifiersState) -> bool {
        let key_matches = keycode_to_string(keycode) == self.key;
        let ctrl_matches = modifiers.control_key() == self.ctrl;
        let shift_matches = modifiers.shift_key() == self.shift;
        let alt_matches = modifiers.alt_key() == self.alt;

        key_matches && ctrl_matches && shift_matches && alt_matches
    }
}

/// 快捷键管理器
#[derive(Debug, Clone)]
pub struct ShortcutManager {
    bindings: HashMap<Shortcut, ShortcutAction>,
}

impl ShortcutManager {
    /// 创建默认快捷键配置
    pub fn default() -> Self {
        let mut bindings = HashMap::new();

        // 复制粘贴
        bindings.insert(Shortcut::new("C", true, true, false), ShortcutAction::Copy);
        bindings.insert(Shortcut::new("V", true, true, false), ShortcutAction::Paste);

        // 标签页管理
        bindings.insert(Shortcut::new("T", true, true, false), ShortcutAction::NewTab);
        bindings.insert(Shortcut::new("W", true, true, false), ShortcutAction::CloseTab);
        bindings.insert(Shortcut::new("TAB", true, false, false), ShortcutAction::NextTab);
        bindings.insert(
            Shortcut::new("TAB", true, true, false),
            ShortcutAction::PreviousTab,
        );

        // 字号调节
        bindings.insert(
            Shortcut::new("EQUAL", true, false, false),
            ShortcutAction::IncreaseFontSize,
        );
        bindings.insert(
            Shortcut::new("MINUS", true, false, false),
            ShortcutAction::DecreaseFontSize,
        );
        bindings.insert(
            Shortcut::new("0", true, false, false),
            ShortcutAction::ResetFontSize,
        );

        // UI 控制
        bindings.insert(
            Shortcut::new("COMMA", true, false, false),
            ShortcutAction::ToggleSettings,
        );
        bindings.insert(
            Shortcut::new("P", true, true, false),
            ShortcutAction::SwitchTheme,
        );

        // 搜索
        bindings.insert(
            Shortcut::new("F", true, false, false),
            ShortcutAction::Search,
        );
        bindings.insert(
            Shortcut::new("F3", false, false, false),
            ShortcutAction::SearchNext,
        );
        bindings.insert(
            Shortcut::new("F3", false, true, false),
            ShortcutAction::SearchPrevious,
        );

        Self { bindings }
    }

    /// 从配置加载快捷键
    pub fn from_config(config_bindings: &HashMap<String, String>) -> Self {
        let mut manager = Self::default();

        for (action_str, shortcut_str) in config_bindings {
            if let (Some(action), Some(shortcut)) =
                (parse_action(action_str), parse_shortcut(shortcut_str))
            {
                manager.bindings.insert(shortcut, action);
            }
        }

        manager
    }

    /// 查找匹配的快捷键动作
    pub fn find_action(&self, keycode: KeyCode, modifiers: ModifiersState) -> Option<ShortcutAction> {
        for (shortcut, action) in &self.bindings {
            if shortcut.matches(keycode, modifiers) {
                return Some(*action);
            }
        }
        None
    }
}

/// 将 KeyCode 转换为字符串表示
fn keycode_to_string(keycode: KeyCode) -> String {
    match keycode {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::Equal => "EQUAL",
        KeyCode::Minus => "MINUS",
        KeyCode::Comma => "COMMA",
        KeyCode::Period => "PERIOD",
        KeyCode::Tab => "TAB",
        KeyCode::Space => "SPACE",
        KeyCode::F1 => "F1",
        KeyCode::F2 => "F2",
        KeyCode::F3 => "F3",
        KeyCode::F4 => "F4",
        KeyCode::F5 => "F5",
        KeyCode::F6 => "F6",
        KeyCode::F7 => "F7",
        KeyCode::F8 => "F8",
        KeyCode::F9 => "F9",
        KeyCode::F10 => "F10",
        KeyCode::F11 => "F11",
        KeyCode::F12 => "F12",
        _ => return String::new(),
    }
    .to_string()
}

/// 解析动作字符串
fn parse_action(s: &str) -> Option<ShortcutAction> {
    match s.to_lowercase().as_str() {
        "copy" => Some(ShortcutAction::Copy),
        "paste" => Some(ShortcutAction::Paste),
        "new_tab" => Some(ShortcutAction::NewTab),
        "close_tab" => Some(ShortcutAction::CloseTab),
        "next_tab" => Some(ShortcutAction::NextTab),
        "previous_tab" => Some(ShortcutAction::PreviousTab),
        "increase_font_size" => Some(ShortcutAction::IncreaseFontSize),
        "decrease_font_size" => Some(ShortcutAction::DecreaseFontSize),
        "reset_font_size" => Some(ShortcutAction::ResetFontSize),
        "toggle_settings" => Some(ShortcutAction::ToggleSettings),
        "switch_theme" => Some(ShortcutAction::SwitchTheme),
        "search" => Some(ShortcutAction::Search),
        "search_next" => Some(ShortcutAction::SearchNext),
        "search_previous" => Some(ShortcutAction::SearchPrevious),
        _ => None,
    }
}

/// 解析快捷键字符串 (格式: "Ctrl+Shift+C")
fn parse_shortcut(s: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut key = String::new();

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "shift" => shift = true,
            "alt" => alt = true,
            k => key = k.to_uppercase(),
        }
    }

    if key.is_empty() {
        return None;
    }

    Some(Shortcut::new(&key, ctrl, shift, alt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut() {
        let shortcut = parse_shortcut("Ctrl+Shift+C").unwrap();
        assert_eq!(shortcut.key, "C");
        assert!(shortcut.ctrl);
        assert!(shortcut.shift);
        assert!(!shortcut.alt);
    }

    #[test]
    fn test_parse_action() {
        assert_eq!(parse_action("copy"), Some(ShortcutAction::Copy));
        assert_eq!(parse_action("new_tab"), Some(ShortcutAction::NewTab));
        assert_eq!(parse_action("invalid"), None);
    }

    #[test]
    fn test_default_shortcuts() {
        let manager = ShortcutManager::default();
        assert!(!manager.bindings.is_empty());
    }
}
