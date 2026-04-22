//! DirectWrite 字体光栅化模块
//!
//! 当前实现目标：
//! 1. 使用 DirectWrite 加载主字体 face
//! 2. 提供更可靠的 cell metrics
//! 3. 输出单字形灰度位图，供现有 atlas 结构复用
//! 4. 无法覆盖时退回 fontdue fallback 链

use std::mem::ManuallyDrop;

use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::Foundation::{BOOL, RECT};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_TEXTURE_ALIASED_1x1, DWriteCreateFactory, IDWriteFactory, IDWriteFont,
    IDWriteFontCollection, IDWriteFontFace, IDWriteFontFamily, IDWriteGlyphRunAnalysis,
    IDWriteRenderingParams, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_METRICS,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_GLYPH_METRICS, DWRITE_GLYPH_RUN, DWRITE_MEASURING_MODE_NATURAL,
    DWRITE_RENDERING_MODE_NATURAL,
};

#[derive(Debug, Clone)]
pub struct FontMetrics {
    pub font_size: f32,
    pub cell_width: f32,
    pub cell_height: f32,
    pub baseline: f32,
    pub design_units_per_em: u16,
    pub source: FontBackend,
}

#[derive(Debug, Clone)]
pub struct DwGlyphBitmap {
    pub width: u32,
    pub height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub advance_width: f32,
    pub bitmap: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontBackend {
    DirectWrite,
    FontdueFallback,
}

/// DirectWrite 字体光栅化器
#[derive(Debug)]
pub struct DwRasterizer {
    factory: Option<IDWriteFactory>,
    font_face: Option<IDWriteFontFace>,
    rendering_params: Option<IDWriteRenderingParams>,
    metrics: FontMetrics,
    initialized: bool,
}

impl DwRasterizer {
    /// 创建新的 DirectWrite 光栅化器
    pub fn new(font_family: &str, font_size: f32) -> Result<Self, String> {
        match unsafe { DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_SHARED) } {
            Ok(factory) => match unsafe { resolve_font_face(&factory, font_family) } {
                Ok(font_face) => {
                    let rendering_params = unsafe { factory.CreateRenderingParams().ok() };
                    let metrics = unsafe {
                        compute_font_metrics(&font_face, rendering_params.as_ref(), font_size)?
                    };

                    tracing::info!(
                        "DirectWrite initialized with font '{}' size {}",
                        font_family,
                        font_size
                    );

                    Ok(Self {
                        factory: Some(factory),
                        font_face: Some(font_face),
                        rendering_params,
                        metrics,
                        initialized: true,
                    })
                }
                Err(err) => {
                    tracing::info!(
                        "DirectWrite font '{}' unavailable ({}), using fontdue fallback",
                        font_family,
                        err
                    );
                    Ok(Self::fallback(font_size))
                }
            },
            Err(err) => {
                tracing::info!(
                    "DirectWrite factory unavailable ({:?}), using fontdue fallback",
                    err
                );
                Ok(Self::fallback(font_size))
            }
        }
    }

    /// 创建 fallback 状态的 rasterizer
    fn fallback(font_size: f32) -> Self {
        Self {
            factory: None,
            font_face: None,
            rendering_params: None,
            metrics: FontMetrics {
                font_size,
                cell_width: font_size * 0.6,
                cell_height: font_size * 1.2,
                baseline: font_size,
                design_units_per_em: 1,
                source: FontBackend::FontdueFallback,
            },
            initialized: false,
        }
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

    /// 检查字符是否可由 DirectWrite 主字体覆盖
    pub fn has_glyph(&self, character: char) -> bool {
        self.lookup_glyph_index(character)
            .is_some_and(|glyph| glyph != 0)
    }

    /// 对单字形进行 DirectWrite 光栅化
    pub fn rasterize(&self, character: char) -> Option<DwGlyphBitmap> {
        let factory = self.factory.as_ref()?;
        let font_face = self.font_face.as_ref()?;
        let glyph_index = self.lookup_glyph_index(character)?;
        if glyph_index == 0 {
            return None;
        }

        let advance_width =
            unsafe { glyph_advance_width(font_face, self.metrics.font_size, glyph_index) }.ok()?;

        let glyph_indices = [glyph_index];
        let glyph_advances = [advance_width];
        let glyph_run = DWRITE_GLYPH_RUN {
            fontFace: borrow_font_face(font_face),
            fontEmSize: self.metrics.font_size,
            glyphCount: 1,
            glyphIndices: glyph_indices.as_ptr(),
            glyphAdvances: glyph_advances.as_ptr(),
            glyphOffsets: std::ptr::null(),
            isSideways: BOOL(0),
            bidiLevel: 0,
        };

        let rendering_mode = unsafe {
            font_face
                .GetRecommendedRenderingMode(
                    self.metrics.font_size,
                    1.0,
                    DWRITE_MEASURING_MODE_NATURAL,
                    self.rendering_params.as_ref()?,
                )
                .unwrap_or(DWRITE_RENDERING_MODE_NATURAL)
        };

        let analysis: IDWriteGlyphRunAnalysis = unsafe {
            factory
                .CreateGlyphRunAnalysis(
                    &glyph_run,
                    1.0,
                    None,
                    rendering_mode,
                    DWRITE_MEASURING_MODE_NATURAL,
                    0.0,
                    self.metrics.baseline,
                )
                .ok()?
        };

        let bounds = unsafe {
            analysis
                .GetAlphaTextureBounds(DWRITE_TEXTURE_ALIASED_1x1)
                .ok()?
        };
        if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
            return None;
        }

        let width = (bounds.right - bounds.left) as u32;
        let height = (bounds.bottom - bounds.top) as u32;
        let mut bitmap = vec![0u8; (width * height) as usize];
        unsafe {
            analysis
                .CreateAlphaTexture(
                    DWRITE_TEXTURE_ALIASED_1x1,
                    &bounds as *const RECT,
                    &mut bitmap,
                )
                .ok()?;
        }
        // DirectWrite 的 aliased 纹理常返回 0..16 的 coverage 值。
        // wgpu 这里使用的是 `R8Unorm`，如果不先放大到 0..255，
        // 实际渲染出来的文字会几乎透明，看起来像“只有背景块没有文字”。
        normalize_aliased_alpha_bitmap(&mut bitmap);
        if bitmap.iter().all(|alpha| *alpha == 0) {
            return None;
        }

        Some(DwGlyphBitmap {
            width,
            height,
            offset_x: bounds.left,
            offset_y: bounds.top,
            advance_width,
            bitmap,
        })
    }

    /// 查询字符对应的 glyph index
    fn lookup_glyph_index(&self, character: char) -> Option<u16> {
        let font_face = self.font_face.as_ref()?;
        let codepoints = [character as u32];
        let mut glyph_indices = [0u16; 1];
        unsafe {
            font_face
                .GetGlyphIndices(codepoints.as_ptr(), 1, glyph_indices.as_mut_ptr())
                .ok()?;
        }
        Some(glyph_indices[0])
    }
}

/// 通过 family 名解析系统字体 face
unsafe fn resolve_font_face(
    factory: &IDWriteFactory,
    family_name: &str,
) -> Result<IDWriteFontFace, String> {
    let mut collection = None::<IDWriteFontCollection>;
    factory
        .GetSystemFontCollection(&mut collection as *mut _, false)
        .map_err(|err| format!("GetSystemFontCollection failed: {:?}", err))?;
    let collection =
        collection.ok_or_else(|| "System font collection not available".to_string())?;

    let family = HSTRING::from(family_name);
    let family_name = PCWSTR::from_raw(family.as_ptr());
    let mut index = 0u32;
    let mut exists = BOOL(0);
    collection
        .FindFamilyName(family_name, &mut index, &mut exists)
        .map_err(|err| format!("FindFamilyName failed: {:?}", err))?;
    if !exists.as_bool() {
        return Err(format!(
            "Font family '{}' not found",
            family_name.to_string().unwrap_or_default()
        ));
    }

    let family: IDWriteFontFamily = collection
        .GetFontFamily(index)
        .map_err(|err| format!("GetFontFamily failed: {:?}", err))?;
    let font: IDWriteFont = family
        .GetFirstMatchingFont(
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
        )
        .map_err(|err| format!("GetFirstMatchingFont failed: {:?}", err))?;
    font.CreateFontFace()
        .map_err(|err| format!("CreateFontFace failed: {:?}", err))
}

/// 计算基础字体 metrics
unsafe fn compute_font_metrics(
    font_face: &IDWriteFontFace,
    rendering_params: Option<&IDWriteRenderingParams>,
    font_size: f32,
) -> Result<FontMetrics, String> {
    let mut metrics = DWRITE_FONT_METRICS::default();
    font_face.GetMetrics(&mut metrics);
    let design_units_per_em = metrics.designUnitsPerEm.max(1);
    let scale = font_size / design_units_per_em as f32;

    let ascent = metrics.ascent as f32 * scale;
    let descent = metrics.descent as f32 * scale;
    let line_gap = metrics.lineGap as f32 * scale;
    let baseline = ascent.ceil().max(1.0);
    let cell_height = (ascent + descent + line_gap).ceil().max(font_size.ceil());

    let m_glyph = glyph_index_for(font_face, 'M').unwrap_or(0);
    let cell_width = if m_glyph != 0 {
        glyph_advance_width(font_face, font_size, m_glyph)
            .unwrap_or(font_size * 0.6)
            .ceil()
            .max(1.0)
    } else if let Some(rendering_params) = rendering_params {
        font_face
            .GetRecommendedRenderingMode(
                font_size,
                1.0,
                DWRITE_MEASURING_MODE_NATURAL,
                rendering_params,
            )
            .ok()
            .map(|_| (font_size * 0.6).ceil())
            .unwrap_or((font_size * 0.6).ceil())
    } else {
        (font_size * 0.6).ceil()
    };

    Ok(FontMetrics {
        font_size,
        cell_width,
        cell_height,
        baseline,
        design_units_per_em,
        source: FontBackend::DirectWrite,
    })
}

/// 查询单字符 glyph index
unsafe fn glyph_index_for(font_face: &IDWriteFontFace, character: char) -> Result<u16, String> {
    let codepoints = [character as u32];
    let mut glyph_indices = [0u16; 1];
    font_face
        .GetGlyphIndices(codepoints.as_ptr(), 1, glyph_indices.as_mut_ptr())
        .map_err(|err| format!("GetGlyphIndices failed: {:?}", err))?;
    Ok(glyph_indices[0])
}

/// 查询单字符 advance width（像素）
unsafe fn glyph_advance_width(
    font_face: &IDWriteFontFace,
    font_size: f32,
    glyph_index: u16,
) -> Result<f32, String> {
    let glyph_indices = [glyph_index];
    let mut glyph_metrics = [DWRITE_GLYPH_METRICS::default(); 1];
    font_face
        .GetGdiCompatibleGlyphMetrics(
            font_size,
            1.0,
            None,
            false,
            glyph_indices.as_ptr(),
            1,
            glyph_metrics.as_mut_ptr(),
            false,
        )
        .map_err(|err| format!("GetGdiCompatibleGlyphMetrics failed: {:?}", err))?;

    let mut font_metrics = DWRITE_FONT_METRICS::default();
    font_face.GetMetrics(&mut font_metrics);
    let design_units_per_em = font_metrics.designUnitsPerEm.max(1) as f32;
    Ok(glyph_metrics[0].advanceWidth as f32 * (font_size / design_units_per_em))
}

/// 以“借用而非拥有”的方式把 font face 放进 glyph run 结构里
fn borrow_font_face(font_face: &IDWriteFontFace) -> ManuallyDrop<Option<IDWriteFontFace>> {
    let raw = Interface::as_raw(font_face);
    let borrowed: Option<IDWriteFontFace> = unsafe { std::mem::transmute(raw) };
    ManuallyDrop::new(borrowed)
}

/// 将 DirectWrite aliased 纹理的 coverage 值归一化到完整 8bit alpha 范围
fn normalize_aliased_alpha_bitmap(bitmap: &mut [u8]) {
    let Some(max_value) = bitmap.iter().copied().max() else {
        return;
    };

    // 只有在明显属于 4bit/5bit coverage 的情况下才放大，
    // 避免未来切换到其它纹理格式时误伤已经是 0..255 的位图。
    if max_value > 16 {
        return;
    }

    for alpha in bitmap {
        *alpha = ((*alpha as u16 * 255) / 16).min(255) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_aliased_alpha_bitmap;

    /// 验证 DirectWrite 的低位 coverage 值会被正确放大到 8bit alpha
    #[test]
    fn normalize_aliased_alpha_bitmap_expands_low_range_values() {
        let mut bitmap = vec![0, 8, 16];
        normalize_aliased_alpha_bitmap(&mut bitmap);
        assert_eq!(bitmap, vec![0, 127, 255]);
    }

    /// 验证已经是 8bit 范围的位图不会再次被错误放大
    #[test]
    fn normalize_aliased_alpha_bitmap_keeps_full_range_values() {
        let mut bitmap = vec![0, 64, 255];
        normalize_aliased_alpha_bitmap(&mut bitmap);
        assert_eq!(bitmap, vec![0, 64, 255]);
    }
}
