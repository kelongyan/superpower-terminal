//! DirectWrite 字体光栅化模块
//!
//! 使用 Windows DirectWrite API 进行高质量字体光栅化，
//! 特别是 CJK 字体的复杂文本排版和亚像素渲染。
//!
//! 当前为框架实现，光栅化仍使用 fontdue 作为后备。

use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
};

#[derive(Debug, Clone)]
pub struct FontMetrics {
    pub font_size: f32,
    pub cell_width: f32,
    pub cell_height: f32,
    pub source: FontBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontBackend {
    DirectWrite,
    FontdueFallback,
}

/// DirectWrite 字体光栅化器
pub struct DwRasterizer {
    metrics: FontMetrics,
    initialized: bool,
}

impl DwRasterizer {
    /// 创建新的 DirectWrite 光栅化器
    pub fn new(font_family: &str, font_size: f32) -> Result<Self, String> {
        // 尝试初始化 DirectWrite
        let initialized = Self::try_init_dwrite(font_family, font_size);

        // 计算单元格大小
        let metrics = if initialized {
            FontMetrics {
                font_size,
                cell_width: font_size * 0.6,
                cell_height: font_size * 1.35,
                source: FontBackend::DirectWrite,
            }
        } else {
            FontMetrics {
                font_size,
                cell_width: font_size * 0.6,
                cell_height: font_size * 1.2,
                source: FontBackend::FontdueFallback,
            }
        };

        if initialized {
            tracing::info!(
                "DirectWrite initialized with font '{}' size {}",
                font_family,
                font_size
            );
        } else {
            tracing::info!("DirectWrite unavailable, using fontdue fallback");
        }

        Ok(Self {
            metrics,
            initialized,
        })
    }

    /// 尝试初始化 DirectWrite 工厂
    fn try_init_dwrite(_font_family: &str, _font_size: f32) -> bool {
        unsafe { DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_SHARED).is_ok() }
    }

    pub fn metrics(&self) -> &FontMetrics {
        &self.metrics
    }

    /// 获取单元格宽度
    pub fn cell_width(&self) -> f32 {
        self.metrics.cell_width
    }

    /// 获取单元格高度
    pub fn cell_height(&self) -> f32 {
        self.metrics.cell_height
    }

    /// 获取字体大小
    pub fn font_size(&self) -> f32 {
        self.metrics.font_size
    }

    /// 是否已初始化 DirectWrite
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
