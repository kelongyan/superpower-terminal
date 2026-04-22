use serde::Deserialize;
use std::collections::HashMap;

/// 终端配置
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub font: FontConfig,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub scrollback: ScrollbackConfig,
    #[serde(default)]
    pub colors: ColorsConfig,
    #[serde(default)]
    pub shortcuts: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ShellConfig {
    #[serde(default = "default_shell_program")]
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
}

fn default_shell_program() -> String {
    "cmd.exe".to_string()
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            program: default_shell_program(),
            args: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FontConfig {
    #[serde(default = "default_font_family")]
    pub family: String,
    #[serde(default = "default_font_size")]
    pub size: f32,
}

fn default_font_family() -> String {
    "Consolas".to_string()
}

fn default_font_size() -> f32 {
    14.0
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size: default_font_size(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WindowConfig {
    #[serde(default = "default_window_width")]
    pub width: u32,
    #[serde(default = "default_window_height")]
    pub height: u32,
    #[serde(default = "default_padding")]
    pub padding_x: u32,
    #[serde(default = "default_padding")]
    pub padding_y: u32,
}

fn default_window_width() -> u32 {
    900
}
fn default_window_height() -> u32 {
    600
}
fn default_padding() -> u32 {
    2
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: default_window_width(),
            height: default_window_height(),
            padding_x: default_padding(),
            padding_y: default_padding(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ScrollbackConfig {
    #[serde(default = "default_scrollback_limit")]
    pub limit: usize,
}

fn default_scrollback_limit() -> usize {
    10000
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self {
            limit: default_scrollback_limit(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ColorsConfig {
    #[serde(default = "default_fg")]
    pub foreground: String,
    #[serde(default = "default_bg")]
    pub background: String,
}

fn default_fg() -> String {
    "#D4D4D4".to_string()
}
fn default_bg() -> String {
    "#1E1E1E".to_string()
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            foreground: default_fg(),
            background: default_bg(),
        }
    }
}

impl Config {
    /// 从 TOML 字符串加载配置
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    /// 加载默认配置
    pub fn default_config() -> Self {
        Self::default()
    }

    /// 从文件路径加载配置，如果文件不存在则返回默认配置
    pub fn load_from_file(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match Self::from_toml(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!("Failed to parse config file {:?}: {}", path, e);
                    Self::default()
                }
            },
            Err(e) => {
                tracing::info!("Config file {:?} not found ({}), using defaults", path, e);
                Self::default()
            }
        }
    }

    /// 获取配置文件路径 (%APPDATA%/superpower/config.toml)
    pub fn config_path() -> std::path::PathBuf {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(appdata)
            .join("superpower")
            .join("config.toml")
    }

    /// 解析十六进制颜色
    pub fn parse_color(s: &str) -> Option<superpower_core::Color> {
        let s = s.strip_prefix('#')?;
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(superpower_core::Color::new(r, g, b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.shell.program, "cmd.exe");
        assert_eq!(config.font.family, "Consolas");
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.scrollback.limit, 10000);
    }

    #[test]
    fn test_parse_toml() {
        let toml = r##"
[shell]
program = "powershell"
args = ["-NoLogo"]

[font]
family = "Cascadia Code"
size = 13.0

[scrollback]
limit = 5000

[colors]
foreground = "#FFFFFF"
background = "#000000"
"##;
        let config = Config::from_toml(toml).unwrap();
        assert_eq!(config.shell.program, "powershell");
        assert_eq!(config.font.family, "Cascadia Code");
        assert_eq!(config.font.size, 13.0);
        assert_eq!(config.scrollback.limit, 5000);
    }

    #[test]
    fn test_parse_color() {
        let color = Config::parse_color("#FF0000").unwrap();
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 0);
        assert_eq!(color.b, 0);
    }
}
