# P2-1 AnimateSprite 帧动画系统 — 实现进度

> 对应 Artemis 引擎的 `AnimateSprite` 标签（`tags.AnimateSprite`）
> 原函数：`root/system/adv/grph.lua:971`
> 实现日期：2026-05-27

## 文件变更

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/script.rs` | 修改 | 新增 `ScriptCmd::AnimateSprite` 变体（15 字段：id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, wait） |
| `src/rendering_messages.rs` | 修改 | 新增 `AnimateSpriteMessage`（15 字段，#[derive(Message)]） |
| `src/components.rs` | 修改 | 新增 `AnimatedSprite` 组件（frames, current_frame, timer, max_frames, finished） |
| `src/plugins/rendering.rs` | 修改 | 新增 `handle_animate_sprite`（加载帧纹理 + 生成实体）、`advance_animated_sprites`（per-frame 推进系统） |
| `src/plugins/script_runner.rs` | 修改 | 新增跳过模式（wait:false）和正常模式（wait→auto_timer + break）处理 |
| `tools/artemis-export/src/mapper.rs` | 修改 | 新增 `"AnimateSprite"` 映射（attrs[0..18] → ScriptCmd 字段） |

## 实现状态

### Task 1 — ScriptCmd 变体 ✅
- `ScriptCmd::AnimateSprite` 插入在 ScrollBg 与 View 之间
- 15 字段与原始 AnimateSpriteTable 参数一一对应

### Task 2 — 消息与组件 ✅
- `AnimateSpriteMessage`: 15 字段，#[derive(Message)]
- `AnimatedSprite`: frames(Vec\<Handle\<Image\>\>), current_frame, timer(Timer::Repeating), max_frames, finished

### Task 3 — 渲染处理器 ✅
- `handle_animate_sprite`: 从 `images/anime/{file}_{NN}.png` 加载帧纹理
- 若 id 已存在（来自 DrawSprite/FadeSprite/MoveSprite），先 despawn 旧实体
- 生成 SpriteOverlay + Node + ImageNode + SpriteAnchor + Transform + Visibility + ZIndex + AnimatedSprite
- 融合模式映射：1→Add, 2→Multiply, 3→Screen, 默认 Normal
- `max=0` 保护：跳过以防止 frames[0] 越界
- `advance_animated_sprites`: per-frame 系统，Repeating timer 驱动，完成后设 finished = true

### Task 4 — 脚本运行器 ✅
- 跳过模式：写入 wait:false（不阻塞，不抑制渲染）
- 正常模式：写入消息；若 wait=true 设 auto_skip.auto_timer = max × frame_time 并 break
- `animate_sprite_writer` 在 ProcessAdvanceParams 中分发

### Task 5 — Mapper ✅
- 映射 attrs[0..18] 中的 15 个参数到 ScriptCmd 字段
- 默认值：x=0, y=0, z=0, anchor_x=-1, anchor_y=-1, rotation=0, draw=0, alpha=255, priority=0, wait=0

### Task 6 — 验证 ✅
- `cargo check` 通过（仅 3 个预先存在的 warning）
- `cargo check -p artemis-export` 通过
- 所有测试通过

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 架构 | 新 `AnimatedSprite` 组件，复用 DrawSprite 实体模式 | 无需重复 SpriteOverlay/TextureCache 基础设施 |
| 模式 | 只实现 Mode A（独立帧文件 `{file}_{NN}.png`） | Mode B（水平精灵条 + clip）原始 AnimateSpriteTable 全部注释 |
| 风格 | 0 和 1 都保持最后一帧 | 原始 Lua style 0 设 50000ms 超时，Bevy 实现语义一致 |
| 融合模式 | 1→Add, 2→Multiply, 3→Screen | 与 handle_draw_sprite 保持一致 |
| 等待机制 | 同 ScrollBg：auto_timer = max × frame_time 然后 break | 复用已验证的 auto_skip 模式 |
| 帧路径 | `images/anime/{file}_{NN}.png`，NN 从 01 开始 | 匹配原始 `string.format("%02d", i)` |

## 验收核对

| 子系统 | 关键文件 | 自检状态 |
|--------|---------|---------|
| ScriptCmd | `script.rs:209` | ✅ 编译通过 |
| Message + Component | `rendering_messages.rs:98`, `components.rs:213` | ✅ 编译通过 |
| 渲染处理器 | `rendering.rs:938` | ✅ 编译通过 |
| 帧动画推进 | `rendering.rs:1013` | ✅ 编译通过 |
| 脚本运行器 | `script_runner.rs` | ✅ 编译通过 |
| Mapper | `mapper.rs:138` | ✅ 34 tests pass |
| 融合模式一致性 | `rendering.rs:954` | ✅ 与 DrawSprite 一致 |

## 已知限制

- `style` 字段已保留但当前未区分行为（两种 style 都保持最后一帧）
- `obj_index` 参数已删除（原始 Lua 函数不使用）
- mask 参数已忽略（原始 Lua 函数不使用）
- ZIndex 限制为 1-2（与现有精灵系统一致，priority≥1 都映射为 ZIndex 2）
- `advance_animated_sprites` 每帧最多推进 1 帧（即使 delta 时间累计超出一个 frame_time）
