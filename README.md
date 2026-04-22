# SuperPower Terminal

Windows 本地终端模拟器，基于 Rust + wgpu + ConPTY。

## 构建与运行

需要 Rust 工具链（`cargo`）。

```powershell
cargo run -p superpower-app
```

Release 构建：

```powershell
cargo build --release
```

## 当前状态

已具备基础可用能力：

- 支持 `cmd.exe` / PowerShell 的基础命令行工作流
- 支持 `vim` / `less` / `fzf` 等全屏 TUI 的核心协议路径（alternate screen、鼠标报告、bracketed paste）
- 支持 scrollback、选区高亮、双击选词、三击选行、复制粘贴
- 支持 IME 中文输入与基础 preedit 可视化
- 支持多标签页、主题切换、工具栏与设置面板
- 支持运行时字号调节与 DPI 响应
- 支持系统字体发现与 fallback 链

## 技术栈

| 层级 | 技术 |
|------|------|
| 终端解析 | `vte` |
| PTY | `portable-pty` (ConPTY) |
| 渲染 | `wgpu` + `fontdue` atlas + DirectWrite |
| 字体发现 | `ttf-parser` + Windows 系统字体扫描 |
| 窗口 | `winit` |
| 配置 | TOML + `serde` |
| 剪贴板 | `arboard` |

## Workspace 结构

- `crates/superpower-core` — 终端核心（Cell、Grid、Cursor、Parser、Selection、DamageTracker）
- `crates/superpower-pty` — PTY 封装（`PtySession`、`PtyEvent`）
- `crates/superpower-renderer` — GPU 渲染器（字形 atlas、文本渲染、UI 绘制）
- `crates/superpower-app` — 应用层（配置、事件循环、UI 壳）

## 配置

运行时自动加载 TOML 配置文件，默认路径由 `superpower_app::config::Config::config_path()` 决定。

默认配置：

- Shell: `cmd.exe`
- 字体: `Consolas 14px`

## License

MIT
