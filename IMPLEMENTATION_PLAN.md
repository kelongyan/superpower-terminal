# SuperPower — 分阶段实现方案（按当前代码校准）

> 本文档基于当前仓库实际实现情况更新，目标是让计划书与代码真实状态保持一致，并明确离“可正常使用终端”还差的关键工作。

---

## 当前总体状态

当前项目已经不是“只有最小闭环的 demo”，而是一个**可运行、接近可正常使用的 Windows 本地终端雏形**：

- Cargo workspace 与 4 个核心 crate 已建立
- PTY → 解析器 → Grid → 渲染器 → 窗口 的主链路已跑通
- 已支持基本 ANSI/CSI/OSC、颜色、滚动区域、scrollback、视口滚动、复制粘贴、配置文件、DPI 变化响应
- 已支持选区高亮、双击选词、三击选行
- 已支持更完整的键盘输入，包括功能键、基础 Ctrl/Alt 组合键、应用光标键模式
- 已支持 `alternate screen`、`bracketed paste`、鼠标报告模式
- 已支持 IME 中文输入提交链路、基础 preedit 可视化、应用小键盘模式、shell 退出提示
- 渲染层采用 `wgpu + fontdue atlas` 作为当前主路径
- 字体链路已支持根据 `font.family` 发现系统字体，并建立 Windows fallback 链
- `DirectWrite` 目前已完成“可用性探测 + 度量接入 + 主字体单字形光栅化接入”

**当前判断**：

- 对于 `cmd.exe` / PowerShell 下的基础命令输入、输出、滚动、复制粘贴，已经具备可正常使用能力
- 对于 `vim` / `less` / `fzf` / `tmux` 这类全屏 TUI，核心协议路径已经接通
- 当前剩余差距主要在真实 GUI 手工回归，而不是主链路能力缺失

因此，项目当前最合理的推进方式不是“重新铺 Phase 1”，而是：

1. 先补齐“达到可正常使用终端”所需的兼容能力
2. 再推进 DirectWrite 与字体链路深化
3. 最后做体验增强与高级能力

---

## 已完成部分

### Phase 1 — 已完成

#### 1. Workspace 与模块拆分
- 已建立 `superpower-core`
- 已建立 `superpower-renderer`
- 已建立 `superpower-pty`
- 已建立 `superpower-app`

#### 2. 终端核心
- `Cell / Grid / Cursor / DamageTracker / Selection` 已实现
- `vte::Perform` 已接入
- 已支持：
  - 基本文字输出
  - 光标移动
  - 清屏/清行
  - SGR 颜色
  - OSC 标题
  - 滚动区域
  - scrollback + viewport
  - 宽字符基础标记
  - 私有模式中的应用光标键模式与光标显示状态

#### 3. PTY
- 已通过 `portable-pty` 启动本地 shell
- 已支持 shell 参数传入
- 已支持 resize
- 已支持 PTY 输出读取与退出事件

#### 4. 应用层
- 已集成 `winit` 事件循环
- 已支持键盘输入、功能键、基础 Ctrl/Alt 组合键
- 已支持鼠标拖拽选择、双击选词、三击选行、复制、粘贴
- 已支持滚轮滚动
- 已支持 IME 提交事件
- 已支持 shell 退出提示
- 已支持配置文件读取

#### 5. 渲染层
- 已完成 `wgpu` 基础渲染管线
- 已实现背景块绘制
- 已实现字形 atlas + glyph cache
- 已实现光标绘制
- 已实现字形 bearing 定位修正
- 已实现选区高亮渲染
- 已接入损伤追踪判断，静止时跳过无意义 render

---

## 当前已落地但仍未完成的部分

### Phase 2 — 进行中

#### 2.1 字体链路
当前状态：
- 已能检测 DirectWrite 工厂是否可用
- 已可在度量阶段使用 DirectWrite 分支
- 已支持根据 `font.family` 扫描系统字体并加载真实字体
- 已建立常见 Windows 字体 fallback 链
- 已将 DirectWrite 接入主字体单字形位图输出
- 当前 fallback 链和大部分非主字体字符仍来自 `fontdue`

#### 2.2 滚动缓冲与视口
当前状态：
- 已实现 scrollback 存储
- 已支持鼠标滚轮 / Shift+PageUp/PageDown/Home/End
- 已支持视口 offset
- 已修正边界显示与选择一致性
- 已实现 alternate screen buffer

#### 2.3 鼠标选择与剪贴板
当前状态：
- 已支持基本拖拽选择
- 已支持选区高亮渲染
- 已支持双击选词 / 三击选行
- 已支持复制 / 粘贴
- 已支持鼠标报告模式
- 已支持 bracketed paste
- 已支持拖拽选择自动滚动
- 仍未实现块选择

#### 2.4 DPI / 缩放
当前状态：
- 已响应 `ScaleFactorChanged`
- 已重新计算 cell metrics
- 已让 glyph rasterize 使用缩放后字号
- 已完成窗口 padding / 布局 / 配置联动
- 仍需继续补充多显示器、复杂缩放场景的实机验证

#### 2.5 颜色与主题联动
当前状态：
- 24-bit / 256 色解析已基本在 parser 中支持
- 默认前景色 / 背景色配置已真正进入渲染器
- 仍未形成完整主题系统与热重载能力

---

## 距离“可正常使用终端”还差哪些工作

P0 的核心协议能力已经在代码层接通。当前离“可以放心对外宣称稳定可用”还差的，主要是**发布前风险收敛**：

### 1. 全屏 TUI 兼容性
- `vim` / `less` / `fzf` / `htop` / `git log --patch` 的真实 GUI 手工回归
- 更完整的 DEC 私有模式与 CSI 覆盖，进一步减少 `Unhandled CSI` 场景

### 2. 输入完整性
- IME preedit 的复杂输入场景打磨
- 更多修饰键组合的终端编码兼容性边角测试

### 3. 字体与渲染质量
- DirectWrite 字形光栅化真正进入主链路
- CJK 显示质量继续提升
- Emoji / 彩色字形支持
- atlas 分配策略优化，降低长时间使用后的浪费和退化风险

### 4. 稳定性与可维护性
- shell 退出后的窗口行为与用户提示
- 面向真实交互场景的手工回归
  - `cmd.exe`
  - PowerShell
  - `git`
  - `vim`
  - `less`
  - 中文输入
  - DPI 缩放
  - 多显示器
- 更多集成级测试，而不仅是 parser / grid / renderer 的单元测试

---

## 调整后的阶段划分

## Phase A — 达到“可正常使用终端”

目标：把当前“可试用终端雏形”补齐到“能稳定跑基础 shell 工作流和常见 TUI”的程度。

### A.1 当前已基本完成
- 选区高亮渲染
- 宽字符 / spacer / 光标覆盖逻辑修正
- scrollback 视口显示边界清理
- 双击选词 / 三击选行
- 更完整的键盘映射
- 字体配置、颜色配置、窗口大小与 padding 配置接入

### A.2 仍需补齐的关键项
- 全屏 TUI 实机验证
- IME preedit 复杂场景打磨
- 更深层 DEC / CSI trace 驱动补齐

**当前验收状态**：核心代码路径已接通；剩余重点为真实工作流回归与兼容性打磨。

---

## Phase B — 深化字体链路

目标：让项目逐步接近原始技术方案，而不是一次性推翻现有实现。

### B.1 DirectWrite 深化接入
- 保留当前 `fontdue` 作为 fallback
- 引入 DirectWrite 度量 / 字体匹配信息
- 在不破坏 atlas 结构的前提下接入 DirectWrite raster result
- 扩大 DirectWrite 覆盖范围，从“主字体单字形”推进到“更多字体 / 更多文本场景”

### B.2 CJK / fallback 提升
- 改善宽字符定位
- 提升系统 fallback 字体命中率与稳定性
- 为中日韩字符建立更可靠的渲染策略

**验收**：英文字体与 CJK 字体显示质量明显提升，且主链路可兼容当前 renderer 结构。

---

## Phase C — 体验增强（中长期）

### C.1 终端体验增强
- URL 检测与点击
- 配置热重载
- 主题系统
- 更强的快捷键体系

### C.2 高级能力
- Emoji / 彩色字形
- Ligatures
- 多标签页
- 性能监控面板

---

## 当前推荐实施顺序

```text
Phase A.2 可正常使用终端所需缺口
  ↓
Phase B 字体链路深化
  ↓
Phase C 体验增强
```

---

## 与原计划相比的核心调整

1. 不再把 DirectWrite 视为“必须立刻整体替换 fontdue”的前提条件
2. 当前 renderer 以 `fontdue atlas + wgpu` 为现实基础，后续围绕它渐进增强
3. 当前优先级已经从“功能能跑”转为“达到稳定可用”
4. 文档中的阶段判断应以“真实代码 + 验证结果”为准，而不是早期理想计划

---

## 当前里程碑判断

- 原 Phase 1：约 100% 完成
- 原 Phase 2：约 80% 完成
- 原 Phase 3：约 10% 完成

因此，后续开发应默认从 **Phase A.2 的剩余风险收敛项** 开始，再继续推进 DirectWrite 与体验增强。
