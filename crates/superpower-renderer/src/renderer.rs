use bytemuck::{Pod, Zeroable};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use superpower_core::{char_width, CellFlags, Color, CursorShape, TerminalHandler};
use ttf_parser::{name_id, Face};
use winit::dpi::PhysicalSize;

use crate::dw_renderer::{DwRasterizer, FontBackend};

/// 背景顶点数据
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct BgVertex {
    position: [f32; 2],
    color: [f32; 3],
}

/// 字形缓存键
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct GlyphKey {
    character: char,
    bold: bool,
    italic: bool,
    font_index: usize,
}

/// 字形信息
#[derive(Debug, Clone)]
struct GlyphInfo {
    /// 在纹理图集中的位置 (x, y)
    x: u32,
    y: u32,
    /// 字形宽度
    width: u32,
    /// 字形高度
    height: u32,
    /// 实际栅格左边距
    bearing_x: i32,
    /// 实际栅格上边距
    bearing_y: i32,
}

/// 文本布局辅助参数，避免在多个渲染 helper 之间散传标量
#[derive(Debug, Clone, Copy)]
struct RenderLayout {
    screen_size: [f32; 2],
    cell_size: [f32; 2],
    padding: [f32; 2],
}

/// 已加载的单个字体面
#[derive(Debug)]
struct LoadedFont {
    /// 对外展示的字体名
    name: String,
    /// 字体在 fallback 链中的顺序
    font: fontdue::Font,
}

/// 渲染器初始化参数
pub struct RendererOptions {
    pub font_family: String,
    pub font_size: f32,
    pub default_foreground: Color,
    pub default_background: Color,
    pub padding_x: u32,
    pub padding_y: u32,
}

/// 终端渲染器
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    // 背景渲染管线
    bg_pipeline: wgpu::RenderPipeline,
    bg_vertex_buffer: wgpu::Buffer,
    // 前景（字形）渲染管线
    fg_pipeline: wgpu::RenderPipeline,
    fg_vertex_buffer: wgpu::Buffer,
    // 字形纹理
    glyph_texture: wgpu::Texture,
    glyph_bind_group: wgpu::BindGroup,
    // 字形缓存
    glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    // 字体光栅化链
    fonts: Vec<LoadedFont>,
    font_size: f32,
    font_family: String,
    font_backend: FontBackend,
    default_foreground: Color,
    default_background: Color,
    padding_x: u32,
    padding_y: u32,
    // 终端尺寸
    cell_width: f32,
    cell_height: f32,
    // 屏幕尺寸
    width: u32,
    height: u32,
    scale_factor: f64,
}

impl Renderer {
    pub async fn new(window: Arc<winit::window::Window>, options: RendererOptions) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor().max(1.0);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12 | wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("SuperPower GPU"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // === 背景渲染管线 ===
        let bg_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bg Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/bg.wgsl").into()),
        });

        let bg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bg Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Bg Pipeline"),
            layout: Some(&bg_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &bg_shader,
                entry_point: Some("vs_main"),
                buffers: &[BgVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &bg_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // 背景顶点缓冲区（预分配较大空间）
        let bg_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bg Vertex Buffer"),
            size: 1024 * 1024, // 1MB 预分配
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // === 前景（字形）渲染管线 ===
        let fg_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fg Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fg.wgsl").into()),
        });

        let glyph_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Glyph Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let fg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Fg Pipeline Layout"),
            bind_group_layouts: &[&glyph_bind_group_layout],
            push_constant_ranges: &[],
        });

        let fg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Fg Pipeline"),
            layout: Some(&fg_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &fg_shader,
                entry_point: Some("vs_main"),
                buffers: &[FgVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fg_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // 前景顶点缓冲区
        let fg_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fg Vertex Buffer"),
            size: 4 * 1024 * 1024, // 4MB 预分配
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 字形纹理图集
        let atlas_size = 2048u32;
        let glyph_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let glyph_texture_view = glyph_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let glyph_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Glyph Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let glyph_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Glyph Bind Group"),
            layout: &glyph_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&glyph_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&glyph_sampler),
                },
            ],
        });

        let scaled_font_size = (options.font_size as f64 * scale_factor) as f32;
        let dw_rasterizer = DwRasterizer::new(&options.font_family, scaled_font_size)
            .expect("Failed to initialize font rasterizer");

        // 构建字体链：优先用户配置字体，再加载常见 Windows fallback，最后退回内嵌字体。
        let fonts = build_font_chain(&options.font_family).expect("Failed to build font chain");
        tracing::info!(
            "Renderer font chain: {}",
            fonts
                .iter()
                .map(|font| font.name.as_str())
                .collect::<Vec<_>>()
                .join(" -> ")
        );

        let (cell_width, cell_height) = if dw_rasterizer.is_initialized() {
            (dw_rasterizer.cell_width(), dw_rasterizer.cell_height())
        } else {
            let primary_font = &fonts[0].font;
            let metrics = primary_font
                .horizontal_line_metrics(scaled_font_size)
                .unwrap();
            (
                primary_font.rasterize('M', scaled_font_size).0.width as f32,
                metrics.new_line_size,
            )
        };

        Self {
            device,
            queue,
            surface,
            config,
            bg_pipeline,
            bg_vertex_buffer,
            fg_pipeline,
            fg_vertex_buffer,
            glyph_texture,
            glyph_bind_group,
            glyph_cache: HashMap::new(),
            fonts,
            font_size: options.font_size,
            font_family: options.font_family,
            font_backend: dw_rasterizer.metrics().source,
            default_foreground: options.default_foreground,
            default_background: options.default_background,
            padding_x: options.padding_x,
            padding_y: options.padding_y,
            cell_width,
            cell_height,
            width: size.width,
            height: size.height,
            scale_factor,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.width = new_size.width;
            self.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// 获取当前缩放后的字体大小
    fn scaled_font_size(&self) -> f32 {
        (self.font_size as f64 * self.scale_factor) as f32
    }

    /// 获取横向物理 padding
    fn effective_padding_x(&self) -> f32 {
        self.padding_x as f32 * self.scale_factor as f32
    }

    /// 获取纵向物理 padding
    fn effective_padding_y(&self) -> f32 {
        self.padding_y as f32 * self.scale_factor as f32
    }

    /// 获取可用于终端排版的物理宽度
    fn content_width(&self) -> f32 {
        (self.width as f32 - self.effective_padding_x() * 2.0).max(self.cell_width)
    }

    /// 获取可用于终端排版的物理高度
    fn content_height(&self) -> f32 {
        (self.height as f32 - self.effective_padding_y() * 2.0).max(self.cell_height)
    }

    /// 获取主字体，用于基础度量和无 fallback 的默认路径
    fn primary_font(&self) -> &fontdue::Font {
        &self.fonts[0].font
    }

    /// 为字符选择最合适的字体索引，优先使用链路前面的字体
    fn select_font_index(&self, character: char) -> usize {
        self.fonts
            .iter()
            .position(|font| font.font.has_glyph(character))
            .unwrap_or(0)
    }

    pub fn update_font_metrics(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor.max(1.0);
        let scaled_font_size = self.scaled_font_size();
        if let Ok(dw_rasterizer) = DwRasterizer::new(&self.font_family, scaled_font_size) {
            if dw_rasterizer.is_initialized() {
                self.cell_width = dw_rasterizer.cell_width();
                self.cell_height = dw_rasterizer.cell_height();
                self.font_backend = dw_rasterizer.metrics().source;
            } else if let Some(metrics) = self
                .primary_font()
                .horizontal_line_metrics(scaled_font_size)
            {
                self.cell_width =
                    self.primary_font().rasterize('M', scaled_font_size).0.width as f32;
                self.cell_height = metrics.new_line_size;
                self.font_backend = FontBackend::FontdueFallback;
            }
        }
        self.glyph_cache.clear();
    }

    pub fn needs_render(&self, handler: &TerminalHandler) -> bool {
        handler.terminal.damage.is_dirty()
    }

    /// 获取终端可显示的行列数
    pub fn terminal_size(&self) -> (usize, usize) {
        let cols = (self.content_width() / self.cell_width) as usize;
        let rows = (self.content_height() / self.cell_height) as usize;
        (rows.max(1), cols.max(1))
    }

    /// 获取单元格宽度（像素）
    pub fn cell_width(&self) -> f32 {
        self.cell_width
    }

    /// 获取单元格高度（像素）
    pub fn cell_height(&self) -> f32 {
        self.cell_height
    }

    /// 获取横向物理 padding，供事件层做坐标换算
    pub fn padding_x(&self) -> f32 {
        self.effective_padding_x()
    }

    /// 获取纵向物理 padding，供事件层做坐标换算
    pub fn padding_y(&self) -> f32 {
        self.effective_padding_y()
    }

    /// 渲染一帧
    pub fn render(&mut self, handler: &TerminalHandler) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // 收集脏行
        let dirty_rows = handler.terminal.damage.dirty_rows();
        // 光栅化脏行中的新字形，脏行索引基于当前视口而不是底层可见区
        self.rasterize_dirty_glyphs(&handler.terminal, &dirty_rows);

        // 构建背景与光标顶点数据
        let mut bg_vertices = self.build_bg_vertices(&handler.terminal);
        let cursor_vertices = self.build_cursor_vertices(&handler.terminal);
        let bg_vertex_count = bg_vertices.len() as u32;
        let cursor_vertex_count = cursor_vertices.len() as u32;
        bg_vertices.extend_from_slice(&cursor_vertices);
        self.queue.write_buffer(
            &self.bg_vertex_buffer,
            0,
            bytemuck::cast_slice(&bg_vertices),
        );

        // 构建前景顶点数据
        let fg_vertices = self.build_fg_vertices(&handler.terminal);
        let fg_vertex_count = fg_vertices.len() as u32;
        self.queue.write_buffer(
            &self.fg_vertex_buffer,
            0,
            bytemuck::cast_slice(&fg_vertices),
        );

        // 渲染
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.default_background.r as f64 / 255.0,
                            g: self.default_background.g as f64 / 255.0,
                            b: self.default_background.b as f64 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if bg_vertex_count > 0 {
                render_pass.set_pipeline(&self.bg_pipeline);
                render_pass.set_vertex_buffer(0, self.bg_vertex_buffer.slice(..));
                render_pass.draw(0..bg_vertex_count, 0..1);
            }

            if fg_vertex_count > 0 {
                render_pass.set_pipeline(&self.fg_pipeline);
                render_pass.set_bind_group(0, &self.glyph_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.fg_vertex_buffer.slice(..));
                render_pass.draw(0..fg_vertex_count, 0..1);
            }

            if cursor_vertex_count > 0 {
                render_pass.set_pipeline(&self.bg_pipeline);
                render_pass.set_vertex_buffer(0, self.bg_vertex_buffer.slice(..));
                render_pass.draw(
                    bg_vertex_count..(bg_vertex_count + cursor_vertex_count),
                    0..1,
                );
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn rasterize_dirty_glyphs(
        &mut self,
        terminal: &superpower_core::Terminal,
        dirty_rows: &[usize],
    ) {
        let font_size = self.scaled_font_size();
        let visible_lines = terminal.grid.visible_lines();

        for &row in dirty_rows {
            if row >= visible_lines.len() {
                continue;
            }
            for cell in visible_lines[row] {
                if cell.character == ' ' || cell.character == '\0' {
                    continue;
                }
                self.ensure_glyph_cached(
                    cell.character,
                    (cell.flags & CellFlags::BOLD) != CellFlags::EMPTY,
                    (cell.flags & CellFlags::ITALIC) != CellFlags::EMPTY,
                    font_size,
                );
            }
        }

        if let Some(preedit) = terminal.ime_preedit() {
            for ch in preedit.text.chars().filter(|ch| *ch != ' ') {
                self.ensure_glyph_cached(ch, false, false, font_size);
            }
        }
    }

    /// 确保某个字符对应字形已经进入 atlas
    fn ensure_glyph_cached(&mut self, character: char, bold: bool, italic: bool, font_size: f32) {
        if character == '\0' {
            return;
        }

        let atlas_size = 2048u32;
        let key = GlyphKey {
            character,
            bold,
            italic,
            font_index: self.select_font_index(character),
        };

        if self.glyph_cache.contains_key(&key) {
            return;
        }

        // 从 fallback 链中选择可覆盖该字符的字体进行光栅化
        let selected_font = &self.fonts[key.font_index].font;
        let (metrics, bitmap) = selected_font.rasterize(character, font_size);

        // 计算图集中的位置（简单的行扫描分配）
        let glyph_count = self.glyph_cache.len() as u32;
        let glyphs_per_row = atlas_size / (metrics.width as u32 + 1).max(1);
        let gx = (glyph_count % glyphs_per_row) * (metrics.width as u32 + 1);
        let gy = (glyph_count / glyphs_per_row) * (metrics.height as u32 + 1);

        if gy + metrics.height as u32 >= atlas_size {
            return;
        }

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.glyph_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: gx, y: gy, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &bitmap,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(metrics.width as u32),
                rows_per_image: Some(metrics.height as u32),
            },
            wgpu::Extent3d {
                width: metrics.width as u32,
                height: metrics.height as u32,
                depth_or_array_layers: 1,
            },
        );

        self.glyph_cache.insert(
            key,
            GlyphInfo {
                x: gx,
                y: gy,
                width: metrics.width as u32,
                height: metrics.height as u32,
                bearing_x: metrics.xmin,
                bearing_y: metrics.ymin,
            },
        );
    }

    fn build_bg_vertices(&self, terminal: &superpower_core::Terminal) -> Vec<BgVertex> {
        let mut vertices = Vec::new();
        let lines = terminal.grid.visible_lines();
        let w = self.width as f32;
        let h = self.height as f32;
        let cw = self.cell_width;
        let ch = self.cell_height;
        let padding_x = self.effective_padding_x();
        let padding_y = self.effective_padding_y();
        let layout = RenderLayout {
            screen_size: [w, h],
            cell_size: [cw, ch],
            padding: [padding_x, padding_y],
        };

        for (row_idx, row) in lines.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                let is_selected = terminal
                    .selection
                    .as_ref()
                    .is_some_and(|selection| selection.contains(row_idx, col_idx));
                let background = if is_selected {
                    blend_color(cell.background, self.default_foreground, 0.35)
                } else {
                    cell.background
                };

                if background == self.default_background {
                    continue;
                }

                let x0 = padding_x + col_idx as f32 * cw;
                let y0 = padding_y + row_idx as f32 * ch;
                let x1 = x0 + cw;
                let y1 = y0 + ch;

                append_bg_quad(&mut vertices, background, [x0, y0, x1, y1], [w, h]);
            }
        }

        self.append_preedit_bg_vertices(&mut vertices, terminal, layout);
        vertices
    }

    fn build_fg_vertices(&self, terminal: &superpower_core::Terminal) -> Vec<FgVertex> {
        let mut vertices = Vec::new();
        let lines = terminal.grid.visible_lines();
        let w = self.width as f32;
        let h = self.height as f32;
        let cw = self.cell_width;
        let ch = self.cell_height;
        let atlas_size = 2048.0f32;
        let padding_x = self.effective_padding_x();
        let padding_y = self.effective_padding_y();
        let layout = RenderLayout {
            screen_size: [w, h],
            cell_size: [cw, ch],
            padding: [padding_x, padding_y],
        };

        for (row_idx, row) in lines.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if cell.character == ' ' || cell.character == '\0' {
                    continue;
                }

                let key = GlyphKey {
                    character: cell.character,
                    bold: (cell.flags & CellFlags::BOLD) != CellFlags::EMPTY,
                    italic: (cell.flags & CellFlags::ITALIC) != CellFlags::EMPTY,
                    font_index: self.select_font_index(cell.character),
                };

                let glyph = match self.glyph_cache.get(&key) {
                    Some(g) => g,
                    None => continue,
                };

                let x0 = padding_x + col_idx as f32 * cw;
                let y0 = padding_y + row_idx as f32 * ch;
                let draw_x0 = x0 + glyph.bearing_x as f32;
                let draw_y0 = y0 + (ch - glyph.height as f32) - glyph.bearing_y as f32;
                let draw_x1 = draw_x0 + glyph.width as f32;
                let draw_y1 = draw_y0 + glyph.height as f32;

                // NDC
                let nx0 = draw_x0 / w * 2.0 - 1.0;
                let ny0 = 1.0 - draw_y0 / h * 2.0;
                let nx1 = draw_x1 / w * 2.0 - 1.0;
                let ny1 = 1.0 - draw_y1 / h * 2.0;

                // UV 坐标（字形在图集中的位置）
                let u0 = glyph.x as f32 / atlas_size;
                let v0 = glyph.y as f32 / atlas_size;
                let u1 = (glyph.x + glyph.width) as f32 / atlas_size;
                let v1 = (glyph.y + glyph.height) as f32 / atlas_size;

                let r = cell.foreground.r as f32 / 255.0;
                let g = cell.foreground.g as f32 / 255.0;
                let b = cell.foreground.b as f32 / 255.0;

                vertices.extend_from_slice(&[
                    FgVertex {
                        position: [nx0, ny0],
                        tex_coords: [u0, v0],
                        color: [r, g, b],
                    },
                    FgVertex {
                        position: [nx1, ny0],
                        tex_coords: [u1, v0],
                        color: [r, g, b],
                    },
                    FgVertex {
                        position: [nx0, ny1],
                        tex_coords: [u0, v1],
                        color: [r, g, b],
                    },
                    FgVertex {
                        position: [nx0, ny1],
                        tex_coords: [u0, v1],
                        color: [r, g, b],
                    },
                    FgVertex {
                        position: [nx1, ny0],
                        tex_coords: [u1, v0],
                        color: [r, g, b],
                    },
                    FgVertex {
                        position: [nx1, ny1],
                        tex_coords: [u1, v1],
                        color: [r, g, b],
                    },
                ]);
            }
        }

        self.append_preedit_fg_vertices(&mut vertices, terminal, layout);

        vertices
    }

    /// 使用纯色背景管线绘制光标，避免依赖字形纹理采样
    fn build_cursor_vertices(&self, terminal: &superpower_core::Terminal) -> Vec<BgVertex> {
        if !terminal.cursor.visible || terminal.grid.is_scrolled() {
            return Vec::new();
        }

        let w = self.width as f32;
        let h = self.height as f32;
        let cw = self.cell_width;
        let ch = self.cell_height;
        let padding_x = self.effective_padding_x();
        let padding_y = self.effective_padding_y();
        let row = terminal.cursor.row;
        let col = terminal.cursor.col;
        let x0 = padding_x + col as f32 * cw;
        let y0 = padding_y + row as f32 * ch;

        let (cursor_w, cursor_h, cursor_x0, cursor_y0) = match terminal.cursor.shape {
            CursorShape::Block => (cw, ch, x0, y0),
            CursorShape::Underline => (cw, 2.0, x0, y0 + ch - 2.0),
            CursorShape::Beam => (2.0, ch, x0, y0),
        };

        let cursor_color = blend_color(self.default_background, self.default_foreground, 0.85);
        let mut vertices = Vec::new();
        append_bg_quad(
            &mut vertices,
            cursor_color,
            [
                cursor_x0,
                cursor_y0,
                cursor_x0 + cursor_w,
                cursor_y0 + cursor_h,
            ],
            [w, h],
        );
        vertices
    }

    /// 为 IME preedit 绘制下划线与预编辑光标区域
    fn append_preedit_bg_vertices(
        &self,
        vertices: &mut Vec<BgVertex>,
        terminal: &superpower_core::Terminal,
        layout: RenderLayout,
    ) {
        let Some(preedit) = terminal.ime_preedit() else {
            return;
        };

        let [w, h] = layout.screen_size;
        let [cw, ch] = layout.cell_size;
        let [padding_x, padding_y] = layout.padding;
        let mut visual_col = terminal.cursor.col;
        let row = terminal.cursor.row;
        let underline_color = blend_color(self.default_background, self.default_foreground, 0.6);
        let cursor_color = blend_color(self.default_background, self.default_foreground, 0.85);
        let cursor_start = preedit.cursor_range.map(|(start, _)| start).unwrap_or(0);
        let cursor_end = preedit
            .cursor_range
            .map(|(_, end)| end)
            .unwrap_or(cursor_start);

        for (char_index, ch_text) in preedit.text.chars().enumerate() {
            let width = char_width(ch_text).max(1);
            let x0 = padding_x + visual_col as f32 * cw;
            let x1 = x0 + cw * width as f32;
            let y0 = padding_y + row as f32 * ch;
            let underline_y0 = y0 + ch - 2.0;
            let underline_y1 = y0 + ch;
            append_bg_quad(
                vertices,
                underline_color,
                [x0, underline_y0, x1, underline_y1],
                [w, h],
            );

            if char_index >= cursor_start && char_index < cursor_end.max(cursor_start + 1) {
                append_bg_quad(
                    vertices,
                    cursor_color,
                    [x0, y0, (x0 + 2.0).min(x1), y0 + ch],
                    [w, h],
                );
            }

            visual_col += width;
        }
    }

    /// 为 IME preedit 追加前景字形顶点
    fn append_preedit_fg_vertices(
        &self,
        vertices: &mut Vec<FgVertex>,
        terminal: &superpower_core::Terminal,
        layout: RenderLayout,
    ) {
        let Some(preedit) = terminal.ime_preedit() else {
            return;
        };

        let [w, h] = layout.screen_size;
        let [cw, ch] = layout.cell_size;
        let [padding_x, padding_y] = layout.padding;
        let atlas_size = 2048.0f32;
        let mut visual_col = terminal.cursor.col;
        let row = terminal.cursor.row;
        let color = blend_color(self.default_foreground, Color::new(0x6A, 0xC1, 0xFF), 0.2);
        let r = color.r as f32 / 255.0;
        let g = color.g as f32 / 255.0;
        let b = color.b as f32 / 255.0;

        for ch_text in preedit.text.chars() {
            if ch_text == ' ' {
                visual_col += 1;
                continue;
            }

            let key = GlyphKey {
                character: ch_text,
                bold: false,
                italic: false,
                font_index: self.select_font_index(ch_text),
            };
            let Some(glyph) = self.glyph_cache.get(&key) else {
                visual_col += char_width(ch_text).max(1);
                continue;
            };

            let x0 = padding_x + visual_col as f32 * cw;
            let y0 = padding_y + row as f32 * ch;
            let draw_x0 = x0 + glyph.bearing_x as f32;
            let draw_y0 = y0 + (ch - glyph.height as f32) - glyph.bearing_y as f32;
            let draw_x1 = draw_x0 + glyph.width as f32;
            let draw_y1 = draw_y0 + glyph.height as f32;

            let nx0 = draw_x0 / w * 2.0 - 1.0;
            let ny0 = 1.0 - draw_y0 / h * 2.0;
            let nx1 = draw_x1 / w * 2.0 - 1.0;
            let ny1 = 1.0 - draw_y1 / h * 2.0;

            let u0 = glyph.x as f32 / atlas_size;
            let v0 = glyph.y as f32 / atlas_size;
            let u1 = (glyph.x + glyph.width) as f32 / atlas_size;
            let v1 = (glyph.y + glyph.height) as f32 / atlas_size;

            vertices.extend_from_slice(&[
                FgVertex {
                    position: [nx0, ny0],
                    tex_coords: [u0, v0],
                    color: [r, g, b],
                },
                FgVertex {
                    position: [nx1, ny0],
                    tex_coords: [u1, v0],
                    color: [r, g, b],
                },
                FgVertex {
                    position: [nx0, ny1],
                    tex_coords: [u0, v1],
                    color: [r, g, b],
                },
                FgVertex {
                    position: [nx0, ny1],
                    tex_coords: [u0, v1],
                    color: [r, g, b],
                },
                FgVertex {
                    position: [nx1, ny0],
                    tex_coords: [u1, v0],
                    color: [r, g, b],
                },
                FgVertex {
                    position: [nx1, ny1],
                    tex_coords: [u1, v1],
                    color: [r, g, b],
                },
            ]);

            visual_col += char_width(ch_text).max(1);
        }
    }
}

/// 系统字体面索引项，用于把 family 名解析到具体文件与集合索引
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SystemFontFace {
    path: PathBuf,
    collection_index: u32,
    family_names: Vec<String>,
}

/// 常见的 Windows fallback 字体族
const WINDOWS_FALLBACK_FAMILIES: &[&str] = &[
    "Segoe UI Symbol",
    "Segoe UI Emoji",
    "Microsoft YaHei UI",
    "Microsoft YaHei",
    "SimSun",
    "Arial Unicode MS",
    "Arial",
];

/// 构建字体链，优先使用用户指定字体，再补入常见系统 fallback，最后追加内嵌字体
fn build_font_chain(requested_family: &str) -> Result<Vec<LoadedFont>, String> {
    let mut chain = Vec::new();
    let mut seen_faces = HashSet::new();
    let system_faces = scan_system_font_faces();

    for family in std::iter::once(requested_family).chain(WINDOWS_FALLBACK_FAMILIES.iter().copied())
    {
        if let Some(face) = find_system_font_face(&system_faces, family) {
            if seen_faces.insert((face.path.clone(), face.collection_index)) {
                if let Ok(font) = load_system_font_face(face) {
                    chain.push(font);
                }
            }
        }
    }

    // 任何情况下都保留项目自带字体作为最后兜底，避免系统字体发现失败导致不可渲染。
    chain.push(load_embedded_font()?);

    if chain.is_empty() {
        Err("No usable font in chain".to_string())
    } else {
        Ok(chain)
    }
}

/// 扫描 Windows Fonts 目录，提取可用于 family 匹配的字体面信息
fn scan_system_font_faces() -> Vec<SystemFontFace> {
    let font_dir = windows_font_dir();
    let Ok(entries) = fs::read_dir(&font_dir) else {
        return Vec::new();
    };

    let mut faces = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_supported_font_file(&path) {
            continue;
        }

        let Ok(data) = fs::read(&path) else {
            continue;
        };
        let face_count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);

        for collection_index in 0..face_count {
            let Ok(face) = Face::parse(&data, collection_index) else {
                continue;
            };

            let family_names = extract_family_names(&face);
            if family_names.is_empty() {
                continue;
            }

            faces.push(SystemFontFace {
                path: path.clone(),
                collection_index,
                family_names,
            });
        }
    }

    faces
}

/// 根据 family 名匹配系统字体面
fn find_system_font_face<'a>(
    faces: &'a [SystemFontFace],
    requested_family: &str,
) -> Option<&'a SystemFontFace> {
    let normalized_requested = normalize_font_name(requested_family);
    faces.iter().find(|face| {
        face.family_names
            .iter()
            .any(|name| normalize_font_name(name) == normalized_requested)
    })
}

/// 从系统字体文件加载一个可用于 fontdue 的字体面
fn load_system_font_face(face: &SystemFontFace) -> Result<LoadedFont, String> {
    let bytes =
        fs::read(&face.path).map_err(|err| format!("Failed to read {:?}: {}", face.path, err))?;
    let font = fontdue::Font::from_bytes(
        bytes,
        fontdue::FontSettings {
            collection_index: face.collection_index,
            ..fontdue::FontSettings::default()
        },
    )
    .map_err(|err| format!("Failed to parse {:?}: {}", face.path, err))?;

    Ok(LoadedFont {
        name: font
            .name()
            .map(ToOwned::to_owned)
            .or_else(|| face.family_names.first().cloned())
            .unwrap_or_else(|| face.path.display().to_string()),
        font,
    })
}

/// 加载内嵌字体，作为最后的硬兜底
fn load_embedded_font() -> Result<LoadedFont, String> {
    let font_data = include_bytes!("../../../assets/CascadiaCode-Regular.ttf");
    let font = fontdue::Font::from_bytes(font_data.as_slice(), fontdue::FontSettings::default())
        .map_err(|err| format!("Failed to load embedded font: {}", err))?;

    Ok(LoadedFont {
        name: font
            .name()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "Embedded Cascadia Code".to_string()),
        font,
    })
}

/// 获取 Windows Fonts 目录
fn windows_font_dir() -> PathBuf {
    std::env::var("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows"))
        .join("Fonts")
}

/// 判断是否为本阶段支持的字体文件类型
fn is_supported_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "ttf" | "otf" | "ttc" | "otc"
            )
        })
}

/// 从字体 name 表中提取 family 候选名
fn extract_family_names(face: &Face<'_>) -> Vec<String> {
    let mut names = Vec::new();
    for name in face.names() {
        if !name.is_unicode() {
            continue;
        }
        if !matches!(
            name.name_id,
            name_id::TYPOGRAPHIC_FAMILY | name_id::FAMILY | name_id::FULL_NAME
        ) {
            continue;
        }
        if let Some(value) = name.to_string() {
            if !value.is_empty() && !names.contains(&value) {
                names.push(value);
            }
        }
    }
    names
}

/// 归一化字体名，避免空格、大小写和连接符差异导致匹配失败
fn normalize_font_name(name: &str) -> String {
    name.chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '-' && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

/// 追加一个背景矩形的两个三角形顶点
fn append_bg_quad(
    vertices: &mut Vec<BgVertex>,
    color: Color,
    rect: [f32; 4],
    screen_size: [f32; 2],
) {
    let [x0, y0, x1, y1] = rect;
    let [screen_width, screen_height] = screen_size;
    let nx0 = x0 / screen_width * 2.0 - 1.0;
    let ny0 = 1.0 - y0 / screen_height * 2.0;
    let nx1 = x1 / screen_width * 2.0 - 1.0;
    let ny1 = 1.0 - y1 / screen_height * 2.0;
    let r = color.r as f32 / 255.0;
    let g = color.g as f32 / 255.0;
    let b = color.b as f32 / 255.0;

    vertices.extend_from_slice(&[
        BgVertex {
            position: [nx0, ny0],
            color: [r, g, b],
        },
        BgVertex {
            position: [nx1, ny0],
            color: [r, g, b],
        },
        BgVertex {
            position: [nx0, ny1],
            color: [r, g, b],
        },
        BgVertex {
            position: [nx0, ny1],
            color: [r, g, b],
        },
        BgVertex {
            position: [nx1, ny0],
            color: [r, g, b],
        },
        BgVertex {
            position: [nx1, ny1],
            color: [r, g, b],
        },
    ]);
}

/// 按比例混合两种颜色，用于选区与光标等覆盖色
fn blend_color(base: Color, overlay: Color, alpha: f32) -> Color {
    let alpha = alpha.clamp(0.0, 1.0);
    let inv_alpha = 1.0 - alpha;

    Color::new(
        (base.r as f32 * inv_alpha + overlay.r as f32 * alpha).round() as u8,
        (base.g as f32 * inv_alpha + overlay.g as f32 * alpha).round() as u8,
        (base.b as f32 * inv_alpha + overlay.b as f32 * alpha).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证字体名归一化不会被空格和连接符影响
    #[test]
    fn normalize_font_name_ignores_spacing_and_case() {
        assert_eq!(normalize_font_name("Cascadia Code"), "cascadiacode");
        assert_eq!(normalize_font_name("Segoe-UI_Emoji"), "segoeuiemoji");
    }

    /// 验证内嵌字体兜底始终可加载
    #[test]
    fn embedded_font_can_be_loaded() {
        let font = load_embedded_font().expect("embedded font should load");
        assert!(font.font.has_glyph('A'));
    }
}

/// 前景顶点数据
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct FgVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    color: [f32; 3],
}

impl BgVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<BgVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl FgVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<FgVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
