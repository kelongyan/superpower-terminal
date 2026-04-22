use superpower_core::Color;
use superpower_renderer::{ChromeScene, Rect, TextAlign, UiQuad, UiText};

/// 一体化顶部 chrome 高度
const TOP_BAR_HEIGHT: f32 = 52.0;
/// 底部状态栏高度
const STATUS_BAR_HEIGHT: f32 = 28.0;
/// 右侧设置面板宽度
const SETTINGS_PANEL_WIDTH: f32 = 288.0;
/// 主内容区边距
const CONTENT_GAP: f32 = 12.0;
/// 通用按钮高度
const BUTTON_HEIGHT: f32 = 32.0;
/// 标签页高度
const TAB_HEIGHT: f32 = 36.0;
const TERMINAL_PANEL_RADIUS: f32 = 14.0;
const PANEL_RADIUS: f32 = 16.0;
const SECTION_RADIUS: f32 = 12.0;
const BUTTON_RADIUS: f32 = 10.0;
const TAB_RADIUS: f32 = 12.0;

/// 可切换的内建主题
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    GraphiteDark,
    PaperLight,
    TerminalAmber,
}

impl ThemePreset {
    /// 返回主题展示名称
    pub fn label(&self) -> &'static str {
        match self {
            ThemePreset::GraphiteDark => "Graphite Dark",
            ThemePreset::PaperLight => "Paper Light",
            ThemePreset::TerminalAmber => "Terminal Amber",
        }
    }

    /// 按预设顺序循环到下一个主题
    pub fn next(&self) -> Self {
        match self {
            ThemePreset::GraphiteDark => ThemePreset::PaperLight,
            ThemePreset::PaperLight => ThemePreset::TerminalAmber,
            ThemePreset::TerminalAmber => ThemePreset::GraphiteDark,
        }
    }
}

/// 应用层主题颜色集合
#[derive(Debug, Clone, Copy)]
pub struct AppTheme {
    pub preset: ThemePreset,
    pub window_bg: Color,
    pub toolbar_bg: Color,
    pub toolbar_fg: Color,
    pub tab_bg: Color,
    pub tab_active_bg: Color,
    pub panel_bg: Color,
    pub panel_section_bg: Color,
    pub status_bg: Color,
    pub terminal_panel_bg: Color,
    pub border: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub button_bg: Color,
    pub button_hover_bg: Color,
    pub button_active_bg: Color,
    pub danger_bg: Color,
    pub danger_hover_bg: Color,
    pub tab_hover_bg: Color,
    pub terminal_foreground: Color,
    pub terminal_background: Color,
}

impl AppTheme {
    /// 根据主题预设生成完整配色
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::GraphiteDark => Self {
                preset,
                window_bg: Color::from_u32(0x191B24),
                toolbar_bg: Color::from_u32(0x232530),
                toolbar_fg: Color::from_u32(0xF4F6FA),
                tab_bg: Color::from_u32(0x2B2E3A),
                tab_active_bg: Color::from_u32(0x343847),
                panel_bg: Color::from_u32(0x1D202A),
                panel_section_bg: Color::from_u32(0x292C39),
                status_bg: Color::from_u32(0x202330),
                terminal_panel_bg: Color::from_u32(0x191B24),
                border: Color::from_u32(0x3B4153),
                text_primary: Color::from_u32(0xF1F3F8),
                text_secondary: Color::from_u32(0xA8B0C0),
                accent: Color::from_u32(0x75C7FF),
                accent_soft: Color::from_u32(0x2D3A4F),
                button_bg: Color::from_u32(0x2D303D),
                button_hover_bg: Color::from_u32(0x373B49),
                button_active_bg: Color::from_u32(0x41475A),
                danger_bg: Color::from_u32(0x6D3542),
                danger_hover_bg: Color::from_u32(0x834555),
                tab_hover_bg: Color::from_u32(0x323644),
                terminal_foreground: Color::from_u32(0xDCE3EF),
                terminal_background: Color::from_u32(0x191B24),
            },
            ThemePreset::PaperLight => Self {
                preset,
                window_bg: Color::from_u32(0xEEF1F4),
                toolbar_bg: Color::from_u32(0xF8FAFC),
                toolbar_fg: Color::from_u32(0x16202A),
                tab_bg: Color::from_u32(0xDEE5ED),
                tab_active_bg: Color::from_u32(0xFFFFFF),
                panel_bg: Color::from_u32(0xFAFBFD),
                panel_section_bg: Color::from_u32(0xE8EDF3),
                status_bg: Color::from_u32(0xE3E9F0),
                terminal_panel_bg: Color::from_u32(0xFFFFFF),
                border: Color::from_u32(0xBCC8D4),
                text_primary: Color::from_u32(0x1B2630),
                text_secondary: Color::from_u32(0x5D6B79),
                accent: Color::from_u32(0x1F78C8),
                accent_soft: Color::from_u32(0xD8E9F8),
                button_bg: Color::from_u32(0xE0E8F0),
                button_hover_bg: Color::from_u32(0xD5E1EC),
                button_active_bg: Color::from_u32(0xC4D9ED),
                danger_bg: Color::from_u32(0xE8C9CF),
                danger_hover_bg: Color::from_u32(0xDEB8C0),
                tab_hover_bg: Color::from_u32(0xE7EDF4),
                terminal_foreground: Color::from_u32(0x2A3847),
                terminal_background: Color::from_u32(0xFFFFFF),
            },
            ThemePreset::TerminalAmber => Self {
                preset,
                window_bg: Color::from_u32(0x1B150F),
                toolbar_bg: Color::from_u32(0x251C13),
                toolbar_fg: Color::from_u32(0xFFE9C5),
                tab_bg: Color::from_u32(0x302316),
                tab_active_bg: Color::from_u32(0x47331C),
                panel_bg: Color::from_u32(0x21180F),
                panel_section_bg: Color::from_u32(0x322315),
                status_bg: Color::from_u32(0x241A10),
                terminal_panel_bg: Color::from_u32(0x181108),
                border: Color::from_u32(0x6A4B20),
                text_primary: Color::from_u32(0xFFD79A),
                text_secondary: Color::from_u32(0xC6A56E),
                accent: Color::from_u32(0xF4B942),
                accent_soft: Color::from_u32(0x5D431A),
                button_bg: Color::from_u32(0x3C2C18),
                button_hover_bg: Color::from_u32(0x493621),
                button_active_bg: Color::from_u32(0x5A4121),
                danger_bg: Color::from_u32(0x6A2E1C),
                danger_hover_bg: Color::from_u32(0x84402A),
                tab_hover_bg: Color::from_u32(0x3A2B1B),
                terminal_foreground: Color::from_u32(0xFFCA70),
                terminal_background: Color::from_u32(0x0F0904),
            },
        }
    }

    /// 返回主题名称
    pub fn name(&self) -> &'static str {
        self.preset.label()
    }
}

/// 标签页摘要，用于构建 UI
#[derive(Debug, Clone)]
pub struct TabView {
    pub title: String,
    pub is_active: bool,
    pub is_exited: bool,
}

/// 状态栏展示信息
#[derive(Debug, Clone)]
pub struct StatusView {
    pub shell_label: String,
    pub terminal_cols: usize,
    pub terminal_rows: usize,
    pub is_scrolled: bool,
    pub theme_name: String,
    pub exit_code: Option<i32>,
}

/// 构建 UI 所需的状态快照
#[derive(Debug, Clone)]
pub struct UiBuildState {
    pub window_width: f32,
    pub window_height: f32,
    pub theme: AppTheme,
    pub settings_open: bool,
    pub tabs: Vec<TabView>,
    pub active_tab: usize,
    pub hovered_action: Option<UiAction>,
    pub font_size: f32,
    pub status: StatusView,
}

/// 一次命中测试对应的动作
#[derive(Debug, Clone)]
pub enum UiAction {
    CreateTab,
    ToggleSettings,
    CycleTheme,
    SelectTheme(ThemePreset),
    ActivateTab(usize),
    CloseTab(usize),
    DecreaseFont,
    IncreaseFont,
    CopySelection,
    PasteClipboard,
    ClearTerminal,
    ScrollToBottom,
}

impl UiAction {
    pub fn same_target(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::CreateTab, Self::CreateTab)
            | (Self::ToggleSettings, Self::ToggleSettings)
            | (Self::CycleTheme, Self::CycleTheme)
            | (Self::DecreaseFont, Self::DecreaseFont)
            | (Self::IncreaseFont, Self::IncreaseFont)
            | (Self::CopySelection, Self::CopySelection)
            | (Self::PasteClipboard, Self::PasteClipboard)
            | (Self::ClearTerminal, Self::ClearTerminal)
            | (Self::ScrollToBottom, Self::ScrollToBottom) => true,
            (Self::SelectTheme(a), Self::SelectTheme(b)) => a == b,
            (Self::ActivateTab(a), Self::ActivateTab(b)) => a == b,
            (Self::CloseTab(a), Self::CloseTab(b)) => a == b,
            _ => false,
        }
    }
}

/// 单个可交互命中区域
#[derive(Debug, Clone)]
pub struct HitTarget {
    pub rect: Rect,
    pub action: UiAction,
}

/// 应用层布局结果
#[derive(Debug, Clone)]
pub struct AppLayout {
    pub top_bar: Rect,
    pub content: Rect,
    pub terminal_panel: Rect,
    pub terminal_viewport: Rect,
    pub settings_panel: Option<Rect>,
    pub status_bar: Rect,
}

/// 一帧 UI 构建结果，供渲染和命中测试复用
#[derive(Debug, Clone)]
pub struct UiModel {
    pub layout: AppLayout,
    pub chrome: ChromeScene,
    pub hit_targets: Vec<HitTarget>,
}

impl UiModel {
    /// 根据鼠标坐标返回命中的交互动作
    pub fn hit_test(&self, x: f64, y: f64) -> Option<UiAction> {
        self.hit_targets
            .iter()
            .find(|target| target.rect.contains(x, y))
            .map(|target| target.action.clone())
    }
}

/// 构建整窗 UI，包括布局、绘制项和命中区域
pub fn build_ui_model(state: &UiBuildState) -> UiModel {
    let top_bar = Rect::new(0.0, 0.0, state.window_width, TOP_BAR_HEIGHT);
    let status_bar = Rect::new(
        0.0,
        (state.window_height - STATUS_BAR_HEIGHT).max(top_bar.bottom()),
        state.window_width,
        STATUS_BAR_HEIGHT,
    );
    let content = Rect::new(
        0.0,
        top_bar.bottom(),
        state.window_width,
        (status_bar.y - top_bar.bottom()).max(0.0),
    );

    let settings_panel = state.settings_open.then(|| {
        Rect::new(
            (state.window_width - SETTINGS_PANEL_WIDTH - CONTENT_GAP).max(CONTENT_GAP),
            content.y + CONTENT_GAP,
            SETTINGS_PANEL_WIDTH,
            (content.height - CONTENT_GAP * 2.0).max(0.0),
        )
    });

    let terminal_panel = if let Some(panel) = settings_panel {
        Rect::new(0.0, content.y, panel.x.max(220.0), content.height)
    } else {
        Rect::new(0.0, content.y, content.width, content.height)
    };
    let terminal_viewport = Rect::new(
        terminal_panel.x + 18.0,
        terminal_panel.y + 16.0,
        (terminal_panel.width - 36.0).max(1.0),
        (terminal_panel.height - 30.0).max(1.0),
    );

    let layout = AppLayout {
        top_bar,
        content,
        terminal_panel,
        terminal_viewport,
        settings_panel,
        status_bar,
    };

    let mut chrome = ChromeScene {
        clear_color: state.theme.window_bg,
        quads: Vec::new(),
        texts: Vec::new(),
    };
    let mut hit_targets = Vec::new();

    push_quad(&mut chrome, top_bar, state.theme.toolbar_bg);
    push_quad_rounded(
        &mut chrome,
        terminal_panel.inset(8.0, 8.0),
        state.theme.terminal_panel_bg,
        TERMINAL_PANEL_RADIUS,
    );
    push_quad(&mut chrome, status_bar, state.theme.status_bg);
    push_quad(
        &mut chrome,
        Rect::new(0.0, top_bar.bottom() - 1.0, state.window_width, 1.0),
        state.theme.border,
    );
    push_border_line(
        &mut chrome,
        status_bar.y,
        state.window_width,
        state.theme.border,
    );

    push_quad_rounded(
        &mut chrome,
        Rect::new(16.0, 14.0, 16.0, 16.0),
        state.theme.accent_soft,
        8.0,
    );

    let new_tab_rect = Rect::new(state.window_width - 48.0, 10.0, 32.0, BUTTON_HEIGHT);
    let settings_rect = Rect::new(new_tab_rect.x - 84.0, 10.0, 76.0, BUTTON_HEIGHT);
    let theme_rect = Rect::new(settings_rect.x - 84.0, 10.0, 76.0, BUTTON_HEIGHT);
    for (rect, label, action) in [
        (theme_rect, "Theme".to_string(), UiAction::CycleTheme),
        (settings_rect, "Panel".to_string(), UiAction::ToggleSettings),
        (new_tab_rect, "+".to_string(), UiAction::CreateTab),
    ] {
        let background = button_fill(state, &action, false);
        push_button(
            &mut chrome,
            &mut hit_targets,
            rect,
            label,
            action,
            background,
            state.theme.text_primary,
        );
    }

    let tabs_left = 50.0;
    let tabs_right = theme_rect.x - 14.0;
    let mut tab_x = tabs_left;
    for (index, tab) in state.tabs.iter().enumerate() {
        let title = if tab.is_exited {
            format!("{} [exit]", truncate_text(tab.title.as_str(), 16))
        } else {
            truncate_text(tab.title.as_str(), 20)
        };
        let tab_width = compute_tab_width(title.as_str());
        if tab_x + 112.0 > tabs_right {
            break;
        }

        let tab_rect = Rect::new(
            tab_x,
            9.0,
            tab_width.min((tabs_right - tab_x).max(112.0)),
            TAB_HEIGHT,
        );
        let close_rect = Rect::new(tab_rect.right() - 26.0, tab_rect.y + 8.0, 16.0, 16.0);
        let bg = if tab.is_active {
            state.theme.tab_active_bg
        } else if is_hovered(state, &UiAction::ActivateTab(index)) {
            state.theme.tab_hover_bg
        } else {
            state.theme.tab_bg
        };
        let fg = if tab.is_active {
            state.theme.text_primary
        } else {
            state.theme.text_secondary
        };

        push_quad_rounded(&mut chrome, tab_rect, bg, TAB_RADIUS);
        if tab.is_active {
            push_quad(
                &mut chrome,
                Rect::new(tab_rect.x, top_bar.bottom() - 2.0, tab_rect.width, 2.0),
                state.theme.accent,
            );
        }
        push_text(
            &mut chrome,
            Rect::new(
                tab_rect.x + 16.0,
                tab_rect.y + 8.0,
                tab_rect.width - 42.0,
                18.0,
            ),
            title,
            fg,
            TextAlign::Left,
        );
        push_button(
            &mut chrome,
            &mut hit_targets,
            close_rect,
            "x".to_string(),
            UiAction::CloseTab(index),
            button_fill(state, &UiAction::CloseTab(index), true),
            fg,
        );

        hit_targets.push(HitTarget {
            rect: tab_rect,
            action: UiAction::ActivateTab(index),
        });

        tab_x += tab_rect.width + 6.0;
    }

    if let Some(panel) = settings_panel {
        push_quad_rounded(&mut chrome, panel, state.theme.panel_bg, PANEL_RADIUS);
        push_quad(
            &mut chrome,
            Rect::new(panel.x, panel.y, 1.0, panel.height),
            state.theme.border,
        );
        push_text(
            &mut chrome,
            Rect::new(panel.x + 16.0, panel.y + 10.0, panel.width - 32.0, 22.0),
            "Settings".to_string(),
            state.theme.text_primary,
            TextAlign::Left,
        );

        let section_width = panel.width - 32.0;
        let theme_section = Rect::new(panel.x + 16.0, panel.y + 46.0, section_width, 138.0);
        let font_section = Rect::new(
            panel.x + 16.0,
            theme_section.bottom() + 16.0,
            section_width,
            92.0,
        );
        let action_section = Rect::new(
            panel.x + 16.0,
            font_section.bottom() + 16.0,
            section_width,
            178.0,
        );

        for section in [theme_section, font_section, action_section] {
            push_quad_rounded(
                &mut chrome,
                section,
                state.theme.panel_section_bg,
                SECTION_RADIUS,
            );
        }

        push_text(
            &mut chrome,
            Rect::new(
                theme_section.x + 12.0,
                theme_section.y + 8.0,
                section_width - 24.0,
                18.0,
            ),
            "Theme".to_string(),
            state.theme.text_primary,
            TextAlign::Left,
        );
        let theme_presets = [
            ThemePreset::GraphiteDark,
            ThemePreset::PaperLight,
            ThemePreset::TerminalAmber,
        ];
        let mut button_y = theme_section.y + 34.0;
        for preset in theme_presets {
            let is_active = state.theme.preset == preset;
            push_button(
                &mut chrome,
                &mut hit_targets,
                Rect::new(
                    theme_section.x + 14.0,
                    button_y,
                    section_width - 28.0,
                    BUTTON_HEIGHT,
                ),
                preset.label().to_string(),
                UiAction::SelectTheme(preset),
                if is_active {
                    state.theme.button_active_bg
                } else if is_hovered(state, &UiAction::SelectTheme(preset)) {
                    state.theme.button_hover_bg
                } else {
                    state.theme.button_bg
                },
                state.theme.text_primary,
            );
            button_y += BUTTON_HEIGHT + 6.0;
        }

        push_text(
            &mut chrome,
            Rect::new(
                font_section.x + 12.0,
                font_section.y + 8.0,
                section_width - 24.0,
                18.0,
            ),
            "Font Size".to_string(),
            state.theme.text_primary,
            TextAlign::Left,
        );
        push_button(
            &mut chrome,
            &mut hit_targets,
            Rect::new(
                font_section.x + 14.0,
                font_section.y + 34.0,
                48.0,
                BUTTON_HEIGHT,
            ),
            "-".to_string(),
            UiAction::DecreaseFont,
            button_fill(state, &UiAction::DecreaseFont, false),
            state.theme.text_primary,
        );
        push_text(
            &mut chrome,
            Rect::new(
                font_section.x + 66.0,
                font_section.y + 34.0,
                96.0,
                BUTTON_HEIGHT,
            ),
            format!("{:.1} pt", state.font_size),
            state.theme.text_primary,
            TextAlign::Center,
        );
        push_button(
            &mut chrome,
            &mut hit_targets,
            Rect::new(
                font_section.x + 176.0,
                font_section.y + 34.0,
                48.0,
                BUTTON_HEIGHT,
            ),
            "+".to_string(),
            UiAction::IncreaseFont,
            button_fill(state, &UiAction::IncreaseFont, false),
            state.theme.text_primary,
        );

        push_text(
            &mut chrome,
            Rect::new(
                action_section.x + 12.0,
                action_section.y + 8.0,
                section_width - 24.0,
                18.0,
            ),
            "Actions".to_string(),
            state.theme.text_primary,
            TextAlign::Left,
        );

        let action_buttons = [
            ("Copy", UiAction::CopySelection),
            ("Paste", UiAction::PasteClipboard),
            ("Clear", UiAction::ClearTerminal),
            ("Bottom", UiAction::ScrollToBottom),
        ];
        let mut row_y = action_section.y + 34.0;
        for (label, action) in action_buttons {
            let background = button_fill(state, &action, false);
            push_button(
                &mut chrome,
                &mut hit_targets,
                Rect::new(
                    action_section.x + 14.0,
                    row_y,
                    section_width - 28.0,
                    BUTTON_HEIGHT,
                ),
                label.to_string(),
                action,
                background,
                state.theme.text_primary,
            );
            row_y += BUTTON_HEIGHT + 6.0;
        }
    }

    let status_text = format!(
        "{} | {}x{} | {}{}",
        state.status.shell_label,
        state.status.terminal_cols,
        state.status.terminal_rows,
        if state.status.is_scrolled {
            "Scrolled"
        } else {
            "Live"
        },
        state
            .status
            .exit_code
            .map(|code| format!(" | Exit {}", code))
            .unwrap_or_default()
    );
    push_text(
        &mut chrome,
        Rect::new(
            16.0,
            status_bar.y,
            state.window_width * 0.56,
            status_bar.height,
        ),
        status_text,
        state.theme.text_secondary,
        TextAlign::Left,
    );
    push_text(
        &mut chrome,
        Rect::new(
            state.window_width * 0.58,
            status_bar.y,
            state.window_width * 0.40 - 16.0,
            status_bar.height,
        ),
        format!(
            "{}  |  Ctrl+Shift+T  |  Ctrl+Shift+C/V",
            state.status.theme_name
        ),
        state.theme.text_secondary,
        TextAlign::Right,
    );

    UiModel {
        layout,
        chrome,
        hit_targets,
    }
}

/// 追加一个纯色矩形
fn push_quad(chrome: &mut ChromeScene, rect: Rect, color: Color) {
    chrome.quads.push(UiQuad {
        rect,
        color,
        radius: 0.0,
    });
}

fn push_quad_rounded(chrome: &mut ChromeScene, rect: Rect, color: Color, radius: f32) {
    chrome.quads.push(UiQuad {
        rect,
        color,
        radius,
    });
}

/// 追加一行文本
fn push_text(chrome: &mut ChromeScene, rect: Rect, text: String, color: Color, align: TextAlign) {
    chrome.texts.push(UiText {
        rect,
        text,
        color,
        align,
    });
}

/// 追加一个可点击按钮，同时登记命中区域
fn push_button(
    chrome: &mut ChromeScene,
    hit_targets: &mut Vec<HitTarget>,
    rect: Rect,
    label: String,
    action: UiAction,
    background: Color,
    foreground: Color,
) {
    push_quad_rounded(chrome, rect, background, BUTTON_RADIUS);
    push_text(chrome, rect, label, foreground, TextAlign::Center);
    hit_targets.push(HitTarget { rect, action });
}

/// 追加一条横向边界线
fn push_border_line(chrome: &mut ChromeScene, y: f32, width: f32, color: Color) {
    push_quad(chrome, Rect::new(0.0, y, width, 1.0), color);
}

fn is_hovered(state: &UiBuildState, action: &UiAction) -> bool {
    state
        .hovered_action
        .as_ref()
        .is_some_and(|hovered| hovered.same_target(action))
}

fn button_fill(state: &UiBuildState, action: &UiAction, danger: bool) -> Color {
    if danger {
        if is_hovered(state, action) {
            state.theme.danger_hover_bg
        } else {
            state.theme.danger_bg
        }
    } else if is_hovered(state, action) {
        state.theme.button_hover_bg
    } else {
        state.theme.button_bg
    }
}

/// 估算紧凑标签页宽度，避免过短和过长
fn compute_tab_width(title: &str) -> f32 {
    let char_count = title.chars().count() as f32;
    (char_count * 9.0 + 48.0).clamp(118.0, 220.0)
}

/// 按字符数截断标签和说明文本，避免 UI 文本把布局撑坏
fn truncate_text(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    if total <= max_chars {
        return text.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let mut truncated = text.chars().take(keep).collect::<String>();
    truncated.push_str("...");
    truncated
}
