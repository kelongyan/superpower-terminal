# CODEBUDDY.md

This file provides guidance to CodeBuddy Code when working with code in this repository.

## 语言与环境规范

- 默认使用中文回复。
- 代码、命令、路径、错误信息保持原文，不翻译。
- 技术术语优先使用英文原文。
- 操作系统：Windows 11 Pro，默认 Shell：PowerShell。
- 优先使用 PowerShell 命令，避免 bash 语法。

## 常用开发命令

- 运行应用：`cargo run -p superpower-app`
- 构建整个 workspace：`cargo build`
- Release 构建：`cargo build --release`
- 检查 workspace：`cargo check`
- 运行单个 crate 的测试：`cargo test -p <crate-name>`
  - 例：`cargo test -p superpower-core`
- 格式化代码：`cargo fmt`
- Clippy 检查：`cargo clippy`

## Workspace 架构

SuperPower 是一个 Windows 极速本地终端，采用 Rust + wgpu + ConPTY 技术栈。
Cargo workspace 包含 4 个 crate：

### `crates/superpower-core`

终端核心数据模型与协议解析：

- `cell.rs` — `Cell`、`CellFlags`、`Color` 及宽字符宽度计算
- `grid.rs` — `Grid`、`Row`，管理终端行数据与 scrollback
- `cursor.rs` — `Cursor`、`CursorShape`
- `parser.rs` — 基于 `vte` 的 ANSI/CSI/OSC 解析器，输出为 `TerminalHandler`
- `selection.rs` — 选区逻辑（字、行、区域边界计算）
- `damage.rs` — `DamageTracker`，追踪脏区域以优化渲染
- `terminal.rs` — 顶层类型重导出

### `crates/superpower-pty`

PTY 进程封装：

- `pty.rs` — `PtySession`、`PtyEvent`，基于 `portable-pty` 实现

### `crates/superpower-renderer`

GPU 渲染管线：

- `renderer.rs` — 主渲染器 `Renderer`，基于 `wgpu`
  - 负责字形 atlas 管理、背景/文本顶点生成、UI quad 绘制
  - 使用 `fontdue` 作为 fallback 字体光栅化
  - 通过 `bytemuck` 保证 GPU 顶点数据对齐
- `dw_renderer.rs` — `DwRasterizer`、`FontBackend`，接入 Windows `DirectWrite`
  - 用于系统字体发现、度量、以及主字体字形光栅化
- `shaders/` — wgpu shader 文件

### `crates/superpower-app`

应用层与事件循环：

- `main.rs` — 初始化日志、加载配置、启动事件循环
- `event_loop.rs` — 核心事件循环，基于 `winit::application::ApplicationHandler`
  - 管理多标签页 `TerminalTab`（PTY + TerminalHandler + 选区状态）
  - 处理键盘输入（含 IME preedit）、鼠标选择、鼠标报告模式
  - 协调渲染器与窗口事件
- `ui.rs` — 桌面 UI 壳模型：工具栏、标签栏、设置面板、状态栏、主题预设
- `config.rs` — 配置文件（TOML），默认 shell 为 `cmd.exe`，默认字体 `Consolas 14px`

### 主数据流

```
PTY (superpower-pty)
  -> Parser (superpower-core, vte)
    -> Grid / TerminalHandler (superpower-core)
      -> Renderer (superpower-renderer, wgpu)
        -> Window (winit)
```

应用层在 `event_loop.rs` 中持有多个 `TerminalTab`，每个标签页包含独立的 PTY 和终端状态。

## 关键技术决策

- **无额外 GUI 框架**：UI 壳完全基于 `winit + wgpu` 原生实现。
- **双字体后端**：`DirectWrite` 负责系统字体发现与主字体光栅化，`fontdue` 作为 fallback 兜底。
- **atlas 渲染**：文本通过字形 atlas 批量渲染，避免逐字绘制开销。
- **配置文件**：运行时从 `config_path()` 加载 TOML，支持 shell、字体、窗口、颜色、scrollback 配置。

## 代码修改原则

- 仅做必要改动，避免过度优化。
- 保持现有代码风格与整体架构。
- 非必要不进行重构。

## 工作约束

- 默认情况下，不创建也不修改任何说明文档或文档文件。
- 不要自动生成 `README.md`、设计文档、使用说明、架构说明等内容。
- 只有在明确要求“编写文档”、“生成 README”或“写说明文档”时，才允许创建或修改相关文档。
