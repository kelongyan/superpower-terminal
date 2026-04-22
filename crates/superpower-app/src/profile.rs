use serde::Deserialize;
use std::collections::HashMap;
use crate::config::Config;

/// Profile 配置
#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    /// Profile 名称
    pub name: String,
    /// Shell 程序
    #[serde(default)]
    pub shell: Option<String>,
    /// Shell 参数
    #[serde(default)]
    pub shell_args: Option<Vec<String>>,
    /// 工作目录
    #[serde(default)]
    pub working_directory: Option<String>,
    /// 字体族
    #[serde(default)]
    pub font_family: Option<String>,
    /// 字体大小
    #[serde(default)]
    pub font_size: Option<f32>,
    /// 前景色
    #[serde(default)]
    pub foreground: Option<String>,
    /// 背景色
    #[serde(default)]
    pub background: Option<String>,
    /// 环境变量
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl Profile {
    /// 创建默认 Profile
    pub fn default_profile() -> Self {
        Self {
            name: "Default".to_string(),
            shell: None,
            shell_args: None,
            working_directory: None,
            font_family: None,
            font_size: None,
            foreground: None,
            background: None,
            env: HashMap::new(),
        }
    }

    /// 合并 Profile 设置到基础配置
    pub fn merge_into_config(&self, base: &Config) -> Config {
        let mut config = base.clone();

        if let Some(shell) = &self.shell {
            config.shell.program = shell.clone();
        }
        if let Some(args) = &self.shell_args {
            config.shell.args = args.clone();
        }
        if let Some(family) = &self.font_family {
            config.font.family = family.clone();
        }
        if let Some(size) = self.font_size {
            config.font.size = size;
        }
        if let Some(fg) = &self.foreground {
            config.colors.foreground = fg.clone();
        }
        if let Some(bg) = &self.background {
            config.colors.background = bg.clone();
        }

        config
    }

    /// 获取工作目录（如果设置）
    pub fn working_directory(&self) -> Option<&str> {
        self.working_directory.as_deref()
    }

    /// 获取环境变量
    pub fn env_vars(&self) -> &HashMap<String, String> {
        &self.env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile() {
        let profile = Profile::default_profile();
        assert_eq!(profile.name, "Default");
        assert!(profile.shell.is_none());
    }

    #[test]
    fn test_profile_merge() {
        let base = Config::default();
        let mut profile = Profile::default_profile();
        profile.shell = Some("powershell".to_string());
        profile.font_size = Some(16.0);

        let merged = profile.merge_into_config(&base);
        assert_eq!(merged.shell.program, "powershell");
        assert_eq!(merged.font.size, 16.0);
    }
}
