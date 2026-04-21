# superpower-terminal

## 当前状态

SuperPower 目前已经具备接近可正常使用的基础能力：

- 已打通 `PTY -> 解析器 -> Grid -> 渲染器 -> 窗口` 主链路
- 已支持基础 ANSI/CSI/OSC、scrollback、复制粘贴、选区高亮
- 已支持双击选词、三击选行、基础 Ctrl/Alt 键盘输入
- 已支持 `alternate screen`、`bracketed paste`、鼠标报告模式
- 已支持 IME 中文输入提交链路与基础 preedit 可视化、应用小键盘模式、shell 退出提示
- 已支持配置文件、DPI 响应、系统字体发现与 fallback 链

当前项目在代码层面已经补齐了 P0 的核心兼容路径。剩余主要风险集中在真实 GUI 工作流的手工回归，以及更深层控制序列兼容性打磨。

## 后续开发清单

### P0 - 达到可正常使用终端

#### Phase A.1 全屏 TUI 兼容

- [x] 实现 `alternate screen buffer`
- [x] 接入 `bracketed paste`
- [x] 接入鼠标报告模式
- [x] 补齐与全屏 TUI 最相关的高频 `CSI / DEC private mode`
- [ ] 增加 `vim / less / fzf / git log --patch` 的实机回归

#### Phase A.2 输入完整性

- [x] 接入 IME / 中文输入提交链路
- [x] 完善应用小键盘模式序列
- [x] 补齐更多修饰键组合兼容性
- [x] 校验 `AltGr`、国际键盘布局与终端输入编码逻辑

#### Phase A.3 稳定性收口

- [x] 优化 shell 退出后的窗口行为与提示
- [x] 增加异常场景日志与错误反馈
- [x] 增加协议与事件层相关测试
- [ ] 完成 `cmd.exe` / PowerShell / `git` 的回归验证

#### Phase A.4 交互收口

- [x] 实现拖拽选择自动滚动
- [x] 实现 IME preedit 基础可视化
- [ ] 根据真实 GUI 工作流继续打磨 preedit 细节

### P1 - 字体链路深化

#### Phase B.1 DirectWrite 深化接入

- [ ] 将 DirectWrite 从“度量”推进到“真实字形光栅化”
- [ ] 保持 `fontdue` 作为 fallback，避免一次性切换风险
- [ ] 校验 atlas 结构与 DirectWrite 输出的兼容性
- [ ] 为字体加载、fallback 失败场景补齐日志

#### Phase B.2 CJK 与 fallback 提升

- [ ] 提升中日韩字符显示质量
- [ ] 优化宽字符定位与边界处理
- [ ] 提升系统字体 fallback 命中率与稳定性
- [ ] 为复杂文本场景补充回归样例

#### Phase B.3 渲染质量优化

- [ ] 优化 atlas 放置策略，减少行扫描浪费
- [ ] 评估大量输出下的 cache / atlas 退化问题
- [ ] 增加长时间运行和高频刷屏的性能验证

### P2 - 体验增强

#### Phase C.1 基础体验增强

- [ ] 配置热重载
- [ ] 主题系统
- [ ] URL 检测与点击
- [ ] 更强的快捷键体系

#### Phase C.2 高级能力

- [ ] Emoji / 彩色字形
- [ ] Ligatures
- [ ] 多标签页
- [ ] 性能监控面板

## 推荐推进顺序

1. 先完成 `P0`，把项目从“可试用”推进到“可正常使用”。
2. 再推进 `P1`，把字体链路做扎实。
3. 最后做 `P2`，提升体验和高级能力。

## 当前剩余发布前风险

- 还缺少 `vim / less / fzf / git` 的真实 GUI 手工回归
- IME 当前已具备基础 preedit 可视化，但仍未针对复杂候选/组合输入做细致打磨
- 更深层的 `CSI / DEC private mode` 仍需要根据真实 TUI trace 持续补齐
- DirectWrite 仍未进入真实字形光栅化主路径
