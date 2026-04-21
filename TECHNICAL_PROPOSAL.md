# SuperPower — Windows 本地终端技术方案（按当前实现校准）

> 定位：Windows 平台高性能本地终端模拟器  
> 当前阶段：P0 核心协议能力已接通，正在补齐发布前验证与兼容性打磨

---

## 一、当前实现状态概览

### 已完成的技术栈

| 层级 | 技术选型 | 实现状态 |
|------|----------|----------|
| **终端解析** | vte-rs | 已完成基础解析，支持部分 CSI/OSC/DEC 私有模式 |
| **PTY** | portable-pty (ConPTY) | 已完成，支持 shell 启动 / resize / 退出检测 |
| **渲染** | wgpu + fontdue atlas + DirectWrite 度量 | 已完成基础渲染管线 |
| **字体发现** | ttf-parser + Windows Fonts 扫描 | 已完成真实系统字体发现与 fallback 链 |
| **窗口** | winit | 已完成事件循环 |
| **配置** | toml + serde | 已完成配置解析与基础联动 |
| **选择/剪贴板** | arboard | 已完成拖拽选择、双击选词、三击选行、复制粘贴 |

### 当前可用程度

- 对于 PowerShell / `cmd.exe` 下的基础命令行工作流，已经具备可正常使用能力
- 对于依赖 alternate screen、鼠标报告、bracketed paste 的全屏 TUI，核心路径已接通
- 当前剩余差距主要在真实 GUI 手工回归，而不是协议主链路缺失

### 当前验证基线

当前仓库可通过以下验证：

- `cargo test`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build`

---

## 二、当前渲染架构

```
┌────────────────────────────────────────────────────────────┐
│                    渲染管线 (当前实际)                      │
├────────────────────────────────────────────────────────────┤
│  wgpu 渲染管线                                              │
│  ├── 背景管线：背景块 / 选区高亮 / 光标纯色覆盖             │
│  ├── 前景管线：字形纹理采样 + 颜色混合                       │
│  └── Glyph Atlas：2048x2048 R8Unorm                         │
├────────────────────────────────────────────────────────────┤
│  字体链路                                                   │
│  ├── fontdue：当前真实字形光栅化主路径                      │
│  ├── DirectWrite：工厂探测 + 度量                            │
│  ├── ttf-parser：系统字体 family 发现                        │
│  └── fallback chain：配置字体 → 常见 Windows 字体 → 内嵌字体 │
├────────────────────────────────────────────────────────────┤
│  优化特性                                                   │
│  ├── 损伤追踪：仅渲染脏行                                   │
│  ├── glyph cache：HashMap<GlyphKey, GlyphInfo>              │
│  └── bearing 定位：字形精确布局                             │
└────────────────────────────────────────────────────────────┘
```

---

## 三、系统架构

```
┌─────────────────────────────────────────────────────────┐
│                    SuperPower Terminal                    │
├──────────┬────────────┬─────────────┬───────────────────┤
│  UI 层    │  渲染层      │  终端核心      │   PTY 层          │
│          │            │             │                   │
│ winit    │ wgpu       │ vte-rs      │ portable-pty      │
│ 输入事件  │ fontdue    │ Grid        │ ConPTY            │
│ 选择交互  │ DirectWrite│ DamageTracker│ 进程管理         │
│ 配置系统  │ ttf-parser │ Selection   │ 退出检测          │
└──────────┴────────────┴─────────────┴───────────────────┘
```

---

## 四、各层当前真实状态

### 4.1 PTY 层

**当前实现：**
- 使用 `portable-pty` 封装 ConPTY
- 支持自定义 shell 与参数
- PTY 输出通过 `mpsc::channel` 传递到主线程
- 支持 resize 和子进程退出检测

**仍需关注：**
- shell 退出后的窗口行为与提示体验仍较粗糙

### 4.2 终端核心

**当前实现：**

```
Terminal
├── grid: Grid                       // 可见行 + scrollback + viewport
├── cursor: Cursor                   // 光标状态
├── damage: DamageTracker            // 损伤追踪
├── selection: Option<Selection>     // 选区
├── modes: HashSet<u16>              // 标准模式
└── private_modes: HashSet<u16>      // DEC 私有模式
```

**已支持：**
- 基本文字输出、光标移动
- 清屏 / 清行（ED / EL）
- SGR 颜色（16 色 / 256 色 / 真彩色）
- OSC 标题设置
- 滚动区域（DECSTBM）
- 宽字符标记
- 私有模式中的应用光标键模式与光标显隐
- alternate screen
- 鼠标跟踪模式
- bracketed paste 状态
- scrollback + viewport 选择一致性

**仍未完成：**
- 更完整的 DEC 私有模式覆盖
- 更完整的全屏 TUI 兼容逻辑

### 4.3 渲染层

**当前实现：**
- 背景块绘制
- 前景字形绘制
- 选区高亮渲染
- 光标纯色覆盖绘制
- glyph atlas + cache
- `font.family` → 系统字体发现 → fallback 链

**当前不足：**
- 实际 rasterize 主路径仍然是 `fontdue`
- atlas 仍是简单行扫描放置策略
- Emoji / 彩色字形 / ligatures 还未开始

### 4.4 UI 层

**当前实现：**
- winit 窗口与事件循环
- 功能键、方向键、基础 Ctrl/Alt 组合键
- 应用光标键模式输入编码
- 应用小键盘模式输入编码
- 鼠标拖拽选择
- 拖拽选择自动滚动
- 双击选词、三击选行
- 滚轮滚动 / Shift+PageUp/PageDown/Home/End
- 鼠标报告模式
- bracketed paste
- Ctrl+Shift+C / Ctrl+Shift+V / 右键 / 中键 粘贴
- IME 提交事件
- IME preedit 基础可视化
- DPI 变化响应
- 配置文件加载

**当前不足：**
- IME preedit 的复杂候选 / 组合输入细节仍需继续打磨
- 更深层鼠标模式 / 私有模式仍需持续打磨

---

## 五、当前剩余风险

下面这些项决定项目何时能从“代码能力已经接通”升级为“可以放心对外宣称稳定可用”：

### 1. 全屏 TUI 兼容性
- 更完整的 CSI / DEC 私有模式支持
- `vim` / `less` / `fzf` / `git` 等真实工作流的 GUI 手工回归

### 2. 输入法与复杂输入
- IME preedit 的复杂输入场景打磨
- 更多修饰键组合与国际键盘布局回归

### 3. 字体与渲染质量
- DirectWrite 字形光栅化主路径
- CJK 字体质量进一步提升
- Emoji / 彩色字形
- atlas 管理策略优化

### 4. 稳定性与验证
- shell 退出后的窗口行为
- 实机手工回归
  - `cmd.exe`
  - PowerShell
  - `git`
  - `vim`
  - `less`
  - 中文输入
  - DPI / 多显示器
- 更多集成级测试

---

## 六、开发路线图（校准版）

### Phase 1 — 最小可用 ✅ 已完成

- ✅ 项目脚手架搭建
- ✅ ConPTY 集成
- ✅ vte-rs 解析器集成
- ✅ winit 窗口创建
- ✅ wgpu 渲染管线
- ✅ 基础文本渲染
- ✅ 键盘输入 → PTY
- ✅ 光标显示

### Phase 2 — 基础可试用能力 ✅ 大部分完成

- ✅ 字形缓存 + 纹理图集
- ✅ CJK 宽字符标记
- ✅ 损伤追踪优化
- ✅ scrollback / viewport
- ✅ 选区高亮
- ✅ 双击选词 / 三击选行
- ✅ 复制粘贴
- ✅ 配置文件系统
- ✅ DPI 感知
- ✅ True Color 支持
- ✅ 系统字体发现与 fallback 链
- ⏳ DirectWrite 字形光栅化

### Phase A — 达到“可正常使用终端”

- 已完成：alternate screen / 鼠标报告 / bracketed paste / IME 提交链路 / preedit 基础可视化 / shell 退出提示
- 待继续打磨：更完整的 DEC / CSI 覆盖、IME 复杂场景、真实工作流回归

### Phase B — 字体链路深化

- DirectWrite 字形光栅化
- CJK 渲染质量提升
- 更强的字体 fallback
- Emoji / 彩色字形前置准备

### Phase C — 体验增强

- URL 检测与点击
- 配置热重载
- 主题系统
- 多标签页
- Ligatures

---

## 七、技术决策总结

| 决策点 | 当前选择 | 理由 |
|--------|----------|------|
| 编程语言 | Rust | 安全 + 性能 + 生态 |
| PTY 方案 | portable-pty (ConPTY) | Windows 原生 |
| 终端解析 | vte-rs | 成熟可靠 |
| GPU 渲染 | wgpu | 高性能 |
| 当前字形光栅化 | fontdue | 先保证稳定增量 |
| 字体发现 | ttf-parser + 系统字体扫描 | 先打通真实字体 family |
| DirectWrite 当前职责 | 工厂探测 + 度量 | 为后续 rasterize 替换铺路 |
| 窗口管理 | winit | Rust 生态标准 |
| 配置格式 | TOML | 简洁易读 |

---

## 八、结论

项目现在已经处在“**P0 主链路能力已接通，但还需要发布前风险收敛**”的阶段。

最关键的判断标准不是“能不能显示文字”，而是：

- 能否稳定支持全屏 TUI
- 能否正确处理 IME / 复杂输入
- 能否在 Windows 字体环境下稳定输出高质量文本
- 能否通过真实 shell 工作流回归

因此，后续优先级应明确放在 **Phase A 的剩余风险收敛项**，而不是立即去做更靠后的体验增强项。
