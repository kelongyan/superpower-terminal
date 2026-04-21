use superpower_core::Color;
use superpower_renderer::{ChromeScene, Rect, TextAlign, UiQuad, UiText};

/// 顶部工具栏高度
const TOOLBAR_HEIGHT: f32 = 48.0;
/// 标签栏高度
const TAB_BAR_HEIGHT: f32 = 40.0;
/// 底部状态栏高度
const STATUS_BAR_HEIGHT: f32 = 30.0;
/// 右侧设置面板宽度
const SETTINGS_PANEL_WIDTH: f32 = 300.0;
/// 主内容区边距
const CONTENT_GAP: f32 = 12.0;
/// 通用按钮高度
const BUTTON_HEIGHT: f32 = 32.0;

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
    pub button_active_bg: Color,
    pub danger_bg: Color,
    pub terminal_foreground: Color,
    pub terminal_background: Color,
}

impl AppTheme {
    /// 根据主题预设生成完整配色
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::GraphiteDark => Self {
                preset,
                window_bg: Color::from_u32(0x11161C),
                toolbar_bg: Color::from_u32(0x1A222C),
                toolbar_fg: Color::from_u32(0xF3F6F9),
                tab_bg: Color::from_u32(0x202A35),
                tab_active_bg: Color::from_u32(0x2B3A4A),
                panel_bg: Color::from_u32(0x161E27),
                panel_section_bg: Color::from_u32(0x1F2934),
                status_bg: Color::from_u32(0x141B23),
                terminal_panel_bg: Color::from_u32(0x11161C),
                border: Color::from_u32(0x2F3E4E),
                text_primary: Color::from_u32(0xE9EEF4),
                text_secondary: Color::from_u32(0x9EACBA),
                accent: Color::from_u32(0x4EB4FF),
                accent_soft: Color::from_u32(0x264B67),
                button_bg: Color::from_u32(0x233041),
                button_active_bg: Color::from_u32(0x335272),
                danger_bg: Color::from_u32(0x6D3542),
                terminal_foreground: Color::from_u32(0xD6E2F0),
                terminal_background: Color::from_u32(0x0E141B),
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
                button_active_bg: Color::from_u32(0xC4D9ED),
                danger_bg: Color::from_u32(0xE8C9CF),
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
                button_active_bg: Color::from_u32(0x5A4121),
                danger_bg: Color::from_u32(0x6A2E1C),
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

/// 单个可交互命中区域
#[derive(Debug, Clone)]
pub struct HitTarget {
    pub rect: Rect,
    pub action: UiAction,
}

/// 应用层布局结果
#[derive(Debug, Clone)]
pub struct AppLayout {
    pub toolbar: Rect,
    pub tab_bar: Rect,
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
    let toolbar = Rect::new(0.0, 0.0, state.window_width, TOOLBAR_HEIGHT);
    let tab_bar = Rect::new(0.0, toolbar.bottom(), state.window_width, TAB_BAR_HEIGHT);
    let status_bar = Rect::new(
        0.0,
        (state.window_height - STATUS_BAR_HEIGHT).max(tab_bar.bottom()),
        state.window_width,
        STATUS_BAR_HEIGHT,
    );
    let content = Rect::new(
        0.0,
        tab_bar.bottom(),
        state.window_width,
        (status_bar.y - tab_bar.bottom()).max(0.0),
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
        Rect::new(
            CONTENT_GAP,
            content.y + CONTENT_GAP,
            (panel.x - CONTENT_GAP * 2.0).max(220.0),
            (content.height - CONTENT_GAP * 2.0).max(120.0),
        )
    } else {
        Rect::new(
            CONTENT_GAP,
            content.y + CONTENT_GAP,
            (content.width - CONTENT_GAP * 2.0).max(220.0),
            (content.height - CONTENT_GAP * 2.0).max(120.0),
        )
    };
    let terminal_viewport = terminal_panel.inset(16.0, 14.0);

    let layout = AppLayout {
        toolbar,
        tab_bar,
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

    push_quad(&mut chrome, toolbar, state.theme.toolbar_bg);
    push_quad(&mut chrome, tab_bar, state.theme.tab_bg);
    push_quad(&mut chrome, terminal_panel, state.theme.terminal_panel_bg);
    push_quad(&mut chrome, status_bar, state.theme.status_bg);
    push_border_line(
        &mut chrome,
        toolbar.bottom(),
        state.window_width,
        state.theme.border,
    );
    push_border_line(
        &mut chrome,
        tab_bar.bottom(),
        state.window_width,
        state.theme.border,
    );
    push_border_line(
        &mut chrome,
        status_bar.y,
        state.window_width,
        state.theme.border,
    );

    push_text(
        &mut chrome,
        Rect::new(18.0, 0.0, 220.0, TOOLBAR_HEIGHT),
        "SuperPower Desktop".to_string(),
        state.theme.toolbar_fg,
        TextAlign::Left,
    );

    let toolbar_buttons = [
        (
            Rect::new(state.window_width - 324.0, 8.0, 92.0, BUTTON_HEIGHT),
            "Theme".to_string(),
            UiAction::CycleTheme,
        ),
        (
            Rect::new(state.window_width - 222.0, 8.0, 96.0, BUTTON_HEIGHT),
            if state.settings_open {
                "Hide Panel".to_string()
            } else {
                "Settings".to_string()
            },
            UiAction::ToggleSettings,
        ),
        (
            Rect::new(state.window_width - 114.0, 8.0, 96.0, BUTTON_HEIGHT),
            "New Tab".to_string(),
            UiAction::CreateTab,
        ),
    ];

    for (rect, label, action) in toolbar_buttons {
        push_button(
            &mut chrome,
            &mut hit_targets,
            rect,
            label,
            action,
            state.theme.button_bg,
            state.theme.text_primary,
        );
    }

    let mut tab_x = 12.0;
    for (index, tab) in state.tabs.iter().enumerate() {
        let tab_rect = Rect::new(tab_x, tab_bar.y + 5.0, 182.0, TAB_BAR_HEIGHT - 10.0);
        let close_rect = Rect::new(tab_rect.right() - 28.0, tab_rect.y + 4.0, 24.0, 22.0);
        let bg = if tab.is_active {
            state.theme.tab_active_bg
        } else {
            state.theme.tab_bg
        };
        let fg = if tab.is_active {
            state.theme.text_primary
        } else {
            state.theme.text_secondary
        };

        push_quad(&mut chrome, tab_rect, bg);
        push_text(
            &mut chrome,
            Rect::new(
                tab_rect.x + 10.0,
                tab_rect.y,
                tab_rect.width - 42.0,
                tab_rect.height,
            ),
            if tab.is_exited {
                format!("{} [exit]", tab.title)
            } else {
                tab.title.clone()
            },
            fg,
            TextAlign::Left,
        );
        push_button(
            &mut chrome,
            &mut hit_targets,
            close_rect,
            "x".to_string(),
            UiAction::CloseTab(index),
            if tab.is_active {
                state.theme.button_active_bg
            } else {
                state.theme.button_bg
            },
            fg,
        );

        hit_targets.push(HitTarget {
            rect: tab_rect,
            action: UiAction::ActivateTab(index),
        });

        tab_x += tab_rect.width + 8.0;
    }

    if let Some(panel) = settings_panel {
        push_quad(&mut chrome, panel, state.theme.panel_bg);
        push_quad(
            &mut chrome,
            Rect::new(panel.x, panel.y, 1.0, panel.height),
            state.theme.border,
        );
        push_text(
            &mut chrome,
            Rect::new(panel.x + 16.0, panel.y + 10.0, panel.width - 32.0, 28.0),
            "Settings".to_string(),
            state.theme.text_primary,
            TextAlign::Left,
        );

        let section_width = panel.width - 32.0;
        let theme_section = Rect::new(panel.x + 16.0, panel.y + 48.0, section_width, 132.0);
        let font_section = Rect::new(
            panel.x + 16.0,
            theme_section.bottom() + 16.0,
            section_width,
            88.0,
        );
        let action_section = Rect::new(
            panel.x + 16.0,
            font_section.bottom() + 16.0,
            section_width,
            178.0,
        );

        for section in [theme_section, font_section, action_section] {
            push_quad(&mut chrome, section, state.theme.panel_section_bg);
        }

        push_text(
            &mut chrome,
            Rect::new(
                theme_section.x + 12.0,
                theme_section.y + 6.0,
                section_width - 24.0,
                24.0,
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
        let mut button_y = theme_section.y + 36.0;
        for preset in theme_presets {
            let is_active = state.theme.preset == preset;
            push_button(
                &mut chrome,
                &mut hit_targets,
                Rect::new(
                    theme_section.x + 12.0,
                    button_y,
                    section_width - 24.0,
                    BUTTON_HEIGHT,
                ),
                preset.label().to_string(),
                UiAction::SelectTheme(preset),
                if is_active {
                    state.theme.button_active_bg
                } else {
                    state.theme.button_bg
                },
                state.theme.text_primary,
            );
            button_y += BUTTON_HEIGHT + 8.0;
        }

        push_text(
            &mut chrome,
            Rect::new(
                font_section.x + 12.0,
                font_section.y + 6.0,
                section_width - 24.0,
                24.0,
            ),
            "Font Size".to_string(),
            state.theme.text_primary,
            TextAlign::Left,
        );
        push_button(
            &mut chrome,
            &mut hit_targets,
            Rect::new(
                font_section.x + 12.0,
                font_section.y + 38.0,
                56.0,
                BUTTON_HEIGHT,
            ),
            "-".to_string(),
            UiAction::DecreaseFont,
            state.theme.button_bg,
            state.theme.text_primary,
        );
        push_text(
            &mut chrome,
            Rect::new(
                font_section.x + 74.0,
                font_section.y + 38.0,
                84.0,
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
                font_section.x + 164.0,
                font_section.y + 38.0,
                56.0,
                BUTTON_HEIGHT,
            ),
            "+".to_string(),
            UiAction::IncreaseFont,
            state.theme.button_bg,
            state.theme.text_primary,
        );

        push_text(
            &mut chrome,
            Rect::new(
                action_section.x + 12.0,
                action_section.y + 6.0,
                section_width - 24.0,
                24.0,
            ),
            "Terminal Actions".to_string(),
            state.theme.text_primary,
            TextAlign::Left,
        );

        let action_buttons = [
            ("Copy", UiAction::CopySelection),
            ("Paste", UiAction::PasteClipboard),
            ("Clear", UiAction::ClearTerminal),
            ("Bottom", UiAction::ScrollToBottom),
        ];
        let mut row_y = action_section.y + 38.0;
        for (label, action) in action_buttons {
            push_button(
                &mut chrome,
                &mut hit_targets,
                Rect::new(
                    action_section.x + 12.0,
                    row_y,
                    section_width - 24.0,
                    BUTTON_HEIGHT,
                ),
                label.to_string(),
                action,
                state.theme.button_bg,
                state.theme.text_primary,
            );
            row_y += BUTTON_HEIGHT + 8.0;
        }
    }

    let status_text = format!(
        "{}  |  {}x{}  |  {}  |  {}{}",
        state.status.shell_label,
        state.status.terminal_cols,
        state.status.terminal_rows,
        if state.status.is_scrolled {
            "Scrolled"
        } else {
            "Live"
        },
        state.status.theme_name,
        state
            .status
            .exit_code
            .map(|code| format!("  |  Exit {}", code))
            .unwrap_or_default()
    );
    push_text(
        &mut chrome,
        Rect::new(
            16.0,
            status_bar.y,
            state.window_width - 32.0,
            status_bar.height,
        ),
        status_text,
        state.theme.text_secondary,
        TextAlign::Left,
    );

    UiModel {
        layout,
        chrome,
        hit_targets,
    }
}

/// 追加一个纯色矩形
fn push_quad(chrome: &mut ChromeScene, rect: Rect, color: Color) {
    chrome.quads.push(UiQuad { rect, color });
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
    push_quad(chrome, rect, background);
    push_text(chrome, rect, label, foreground, TextAlign::Center);
    hit_targets.push(HitTarget { rect, action });
}

/// 追加一条横向边界线
fn push_border_line(chrome: &mut ChromeScene, y: f32, width: f32, color: Color) {
    push_quad(chrome, Rect::new(0.0, y, width, 1.0), color);
}
