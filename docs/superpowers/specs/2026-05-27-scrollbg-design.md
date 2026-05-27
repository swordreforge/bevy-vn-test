# ScrollBG 背景滚动设计

## 概述
实现背景滚动/平移动画，与原始 Artemis 引擎的 `ScrollBG` Ethornel 标签兼容。

## 参数映射

| Lua 参数 | 字段 | 类型 | 说明 |
|----------|------|------|------|
| `param["0"]` | file | String | 背景图片文件名 |
| `param["1"]` | x1 | f32 | 起始 X 偏移 (px) |
| `param["2"]` | y1 | f32 | 起始 Y 偏移 (px) |
| `param["4"]` | x2 | f32 | 结束 X 偏移 (px) |
| `param["5"]` | y2 | f32 | 结束 Y 偏移 (px) |
| `param["9"]` | fade | u64 | 动画时长 (ms) |
| `param["10"]` | wait | bool | 是否阻塞脚本 |

a1/a2（透明度）在本次实现中忽略。

## 行为规则

1. **图片尺寸决定显示区域**：背景节点设置为图片的自然像素尺寸，而非 100vw/100vh。视口自动裁剪超出的边缘。
2. **文件去重**：若 ScrollBG 的文件与当前背景相同，复用现有贴图（仅平移）。若不同，加载新图片。
3. **SetBg 与滚动的交互**：SetBg 到达时，取消进行中的滚动（新的 SetBg 直接替换背景）。
4. **阻塞**：若 `wait=true`，脚本执行通过 auto_timer 机制阻塞至动画完成（与 `Wait` 命令模式相同）。

## 变更文件

### 1. `src/script.rs`
添加 `ScriptCmd::ScrollBg` 变体。

### 2. `src/rendering_messages.rs`
添加 `ScrollBgMessage`。

### 3. `src/components.rs`
添加 `BgScroll` 组件，驱动背景节点的 left/top 插值。

### 4. `src/plugins/rendering.rs`
- `handle_scroll_bg` 系统：加载图片，调整节点尺寸为自然像素大小，设置初始位置 (x1,y1)，若需要动画则插入 BgScroll。
- `update_bg_scroll` 系统：每帧更新 left/top 插值，完成后移除组件。

### 5. `src/plugins/script_runner.rs`
处理 `ScriptCmd::ScrollBg`：发送消息；若 wait，设置 auto_timer 并 break。

### 6. `tools/artemis-export/src/mapper.rs`
将 `ScrollBG` ASB 标签映射到 `ScriptCmd::ScrollBg`。

## 动画
- 使用 `1.0 - (1.0 - t)²` ease-out 二次缓动（与项目中其他动画一致）
- BgScroll 组件帧率无关，基于 Timer
