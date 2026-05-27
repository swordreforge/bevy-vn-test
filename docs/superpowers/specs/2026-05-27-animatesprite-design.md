# P2-1 帧动画系统（AnimateSprite）设计文档

> 对应 Artemis 引擎的 `AnimateSprite` 标签
> 实现日期：2026-05-27

---

## 1. 总览

### 1.1 范围

| 子系统 | 标签 | 复杂度 | 说明 |
|--------|------|--------|------|
| AnimateSprite | `AnimateSprite` | 中 | 帧动画系统，81 个动画帧资源（16 套序列） |

### 1.2 架构决策

- **不新增模块**：在已有 `rendering` 插件中新增 handler 和 component
- **复用现有基础设施**：`SpriteOverlayManager`(id→Entity)、`TextureCache`、`SpriteAnchor`、`SpriteBlendMode`、`ZIndex`、`sprite_depth_scale`
- **动画驱动**：新增 `AnimatedSprite` 组件 + `advance_animated_sprites` 系统每帧推进
- **Mode A only**：原版 `AnimateSpriteTable` 全部注释掉，Mode B（sprite strip）暂不实现
- **Mask 参数忽略**：原版 `AnimateSprite()` 函数未使用 `p.mask`

### 1.3 资产验证

| 序列 | 帧数 | 路径模式 |
|------|------|---------|
| `aiy00030_03` | 5 | `images/anime/aiy00030_03_01.png` ~ `_05.png` |
| `aiy00030_09` | 5 | `images/anime/aiy00030_09_01.png` ~ `_05.png` |
| `aiy00030_12` | 6 | `images/anime/aiy00030_12_01.png` ~ `_06.png` |
| `aiy00070_01` | 10 | `images/anime/aiy00070_01_01.png` ~ `_10.png` |
| `aiy00150_01` | 10 | `images/anime/aiy00150_01_01.png` ~ `_10.png` |
| `aiy10190_14` | 3 | `images/anime/aiy10190_14_01.png` ~ `_03.png` |
| `aiy10190_40` | 3 | `images/anime/aiy10190_40_01.png` ~ `_03.png` |
| `aiy10190_41` | 3 | `images/anime/aiy10190_41_01.png` ~ `_03.png` |
| `aiy10200_04` | 3 | `images/anime/aiy10200_04_01.png` ~ `_03.png` |
| `aiy10200_08` | 3 | `images/anime/aiy10200_08_01.png` ~ `_03.png` |
| `aiy10200_09` | 4 | `images/anime/aiy10200_09_01.png` ~ `_04.png` |
| `aiy20130_01` | 10 | `images/anime/aiy20130_01_01.png` ~ `_10.png` |
| `aiy20200_14` | 6 | `images/anime/aiy20200_14_01.png` ~ `_06.png` |
| `aiy20200_anm01` | 4 | `images/anime/aiy20200_anm01_01.png` ~ `_04.png` |
| `aiy40330_07` | 3 | `images/anime/aiy40330_07_01.png` ~ `_03.png` |
| `aiy40330_13` | 3 | `images/anime/aiy40330_13_01.png` ~ `_03.png` |

配置文件：`root/system/csv.lua:103` — `anime_path = "image/anime/"`

---

## 2. AnimateSprite 子系统

### 2.1 原版 Lua 逻辑

`root/system/adv/grph.lua:971` — `AnimateSprite(p)` 函数：

```lua
function AnimateSprite(p)
    local id = getSpriteID(p.id, p.priority)
    local subid = id .. ".0"
    local name = p.file
    local file = init.anime_path .. name   -- "image/anime/" + name

    -- Mode A（当前有效）：独立帧文件
    -- 加载所有帧为隐藏图层
    for i = 1, tonumber(p.max) do
        local ext = "_" .. string.format("%02d", i)
        lyc2{ id="anime." .. i, file=(file .. ext), alpha="0" }
    end
    -- 用 anime 标签定义帧时序
    for i = 1, tonumber(p.max) do
        if i == 1 then
            e:tag{"anime", id=subid, mode="init", file=(file .. "_01"), layermode=tbl[drawmode]}
        else
            e:tag{"anime", id=subid, mode="add", file=(file .. ext), layermode=tbl[drawmode], time=(i * p.time)}
        end
    end
    -- 结束行为
    if style == 0 then
        e:tag{"anime", id=subid, mode="end", time="50000"}  -- 保持最后一帧
    elseif style == 1 then
        e:tag{"anime", id=subid, mode="end", time=(p.time * (p.max + 1))}  -- 播放一次
    end

    -- 位置/缩放/透明度（与 DrawSprite 相同）
    local z = tonumber(p.z) or 0; local r = tonumber(p.r) or 0
    local alpha = getAlpha(p.alpha); local scale = getSpriteDepthScale(z)
    local m = getSpritePOS(p)
    e:tag{"lyprop", id=subid, left=m.x, top=m.y, anchorx=p.ax, anchory=p.ay, rotate=-r}
    e:tag{"lyprop", id=id, left="0", top="0", anchorx="640", anchory="360", alpha=alpha}
    e:tag{"lydel", id="anime"}
    flip()
end
```

`tags.AnimateSprite`（`root/system/adv/ethomell.lua:504`）参数映射：

| param | 字段 | 说明 |
|-------|------|------|
| `param["0"]` | id | Sprite ID |
| `param["1"]` | file | 动画文件 basename |
| `param["2"]` | max | 帧数（2-32） |
| `param["3"]` | time | 每帧间隔 ms |
| `param["4"]` | style | 0=保持最后一帧, 1=播放一次 |
| `param["5"]` | mask | 忽略（函数未使用） |
| `param["6"]` | x | X 位置 |
| `param["7"]` | y | Y 位置 |
| `param["8"]` | z | 深度 |
| `param["9"]` | ax | 锚点 X |
| `param["10"]` | ay | 锚点 Y |
| `param["11"]` | r | 旋转角度（度） |
| `param["14"]` | draw | 混合模式（0=normal, 1=add, 3=multiply, 4=screen） |
| `param["15"]` | alpha | 透明度 0-255 |
| `param["16"]` | priority | 优先级/图层 |
| `param["18"]` | wait | 是否等待完成 |

### 2.2 ScriptCmd 变体

```rust
ScriptCmd::AnimateSprite {
    id: String,
    file: String,
    max: u32,
    frame_time: u64,
    style: u32,          // 0=hold last, 1=play once
    x: f32, y: f32,
    z: i32,
    anchor_x: f32, anchor_y: f32,
    rotation: f32,
    draw: u32,           // 0=Normal, 1=Add, 3=Multiply, 4=Screen
    alpha: i32,
    priority: i32,
    wait: bool,
}
```

### 2.3 AnimateSpriteMessage

```rust
#[derive(Message)]
pub struct AnimateSpriteMessage {
    pub id: String,
    pub file: String,
    pub max: u32,
    pub frame_time: u64,
    pub style: u32,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub rotation: f32,
    pub draw: u32,
    pub alpha: i32,
    pub priority: i32,
    pub wait: bool,
}
```

### 2.4 AnimatedSprite 组件

```rust
#[derive(Component)]
pub struct AnimatedSprite {
    pub frames: Vec<Handle<Image>>,
    pub current_frame: usize,
    pub timer: Timer,
    pub max_frames: usize,
    pub finished: bool,
}
```

### 2.5 rendering handler

在 `src/plugins/rendering.rs` 新增 `handle_animate_sprite`：

```
AnimateSpriteMessage
  → 通过 TextureCache 加载所有 max 帧纹理（路径: images/anime/{file}_{NN}.png）
  → 如果 id 已存在（DrawSprite/FadeSprite/MoveSprite 冲突），先 despawn
  → spawn 实体:
    - SpriteOverlay { id, blend_mode: map_draw(draw) }
    - Node { absolute, left: x, top: y }
    - ImageNode { image: frames[0], alpha }
    - SpriteAnchor { anchor_x, anchor_y, target_x: x, target_y: y }
    - Transform { scale: sprite_depth_scale(z), rotation }
    - Visibility::Visible
    - ZIndex (1 + priority).min(2)
    - AnimatedSprite { frames, current_frame: 0, timer, max_frames, finished: false }
  → 注册到 SpriteOverlayManager
```

#### 混合模式映射

原版 Lua：`{ "normal", "add", "normal", "multiply", "screen" }`，使用 `1 + draw` 索引。

| `draw` 值 | Lua 行为 | Rust 枚举 |
|-----------|---------|----------|
| 0 | normal | `SpriteBlendMode::Normal` |
| 1 | add | `SpriteBlendMode::Add` |
| 2 | normal（同 0） | `SpriteBlendMode::Normal` |
| 3 | multiply | `SpriteBlendMode::Multiply` |
| 4 | screen | `SpriteBlendMode::Screen` |
| other | — | `SpriteBlendMode::Normal`（fallback） |

### 2.6 advance_animated_sprites 系统

运行条件：`in_state(AppState::Gameplay)`

每帧处理：

```
for each entity with AnimatedSprite:
    if entity.finished → skip
    
    timer.tick(time.delta())
    if timer.just_finished():
        current_frame += 1
        if current_frame >= max_frames:
            // 播放完毕
            match style:
                0 → finished = true  // hold last frame
                1 → finished = true  // play once, hold last
            // wait 信号在 timer 触发后由 handle_auto_skip 处理
        else:
            // 切换到下一帧
            ImageNode.image = frames[current_frame]
```

### 2.7 Script Runner 处理

#### 普通模式

```
Some(ScriptCmd::AnimateSprite { id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, wait }) => {
    animate_sprite_writer.write(AnimateSpriteMessage { ... });
    if wait {
        let total_ms = max as u64 * frame_time;
        auto_skip.auto_timer = Some(Timer::from_seconds(total_ms as f32 / 1000.0, TimerMode::Once));
        break;
    }
}
```

#### 跳过模式

与 ScrollBg 一致：写入 message 但不等待，动画继续在后台播放（skip 模式下用户快速点击，动画不可感知）。

```
Some(ScriptCmd::AnimateSprite { id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, .. }) => {
    animate_sprite_writer.write(AnimateSpriteMessage { ... wait: false });
}
```

### 2.8 Mapper 映射

在 `tools/artemis-export/src/mapper.rs` 的 `map_command` 中新增：

```
"AnimateSprite" → ScriptCmd::AnimateSprite {
    id: attrs["0"],
    file: attrs["1"],
    max: attrs["2"].parse(),
    frame_time: attrs["3"].parse(),
    style: attrs["4"].parse().unwrap_or(0),
    x: attrs["6"].parse().unwrap_or(0.0),
    y: attrs["7"].parse().unwrap_or(0.0),
    z: attrs["8"].parse().unwrap_or(0),
    anchor_x: attrs["9"].parse().unwrap_or(0.0),
    anchor_y: attrs["10"].parse().unwrap_or(0.0),
    rotation: attrs["11"].parse().unwrap_or(0.0),
    draw: attrs["14"].parse().unwrap_or(0),
    alpha: attrs["15"].parse().unwrap_or(255),
    priority: attrs["16"].parse().unwrap_or(0),
    wait: attrs["18"].parse().unwrap_or("0") == "1",
}
```

### 2.9 状态流转

```
用户点击（AdvanceEvent）
  → process_advance 循环
    → 遇到 AnimateSprite{ wait: true }
      → 写入 AnimateSpriteMessage
      → auto_skip.auto_timer = max * frame_time
      → break（等待时间流逝）
  → rendering: spawn 实体, 显示第一帧
  → 每帧: advance_animated_sprites 推进帧
  → handle_auto_skip 计时
    → timer 到点 → AdvanceEvent
    → process_advance 恢复执行下一条脚本

用户点击（AdvanceEvent）
  → process_advance 循环
    → 遇到 AnimateSprite{ wait: false }
      → 写入 AnimateSpriteMessage
      → 不设 timer，继续执行下一条脚本
  → 动画异步播放
```

### 2.10 动画时长计算

| style | 动画时长 | wait 时长 | 最后一帧 |
|-------|---------|-----------|---------|
| 0 | `max * frame_time` ms | `max * frame_time` ms（如果 wait） | 保持可见 |
| 1 | `max * frame_time` ms | `max * frame_time` ms（如果 wait） | 保持可见 |

两种 style 在最后一帧后的行为表现一致（保持最后一帧可见），区别在于原版 Lua 的 `anime` 引擎行为（style 0 强制保持 50000ms，style 1 即时结束）。对于 Bevy 实现，两种 style 在动画播完后都保持最后一帧。

---

## 3. 文件变更

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/script.rs` | 修改 | 新增 `AnimateSprite` ScriptCmd 变体 |
| `src/rendering_messages.rs` | 修改 | 新增 `AnimateSpriteMessage` |
| `src/components.rs` | 修改 | 新增 `AnimatedSprite` 组件 |
| `src/plugins/script_runner.rs` | 修改 | 处理 AnimateSprite（正常+跳过模式） |
| `src/plugins/rendering.rs` | 修改 | 新增 `handle_animate_sprite` + `advance_animated_sprites` 系统 |
| `tools/artemis-export/src/mapper.rs` | 修改 | 新增 AnimateSprite 标签映射 |

---

## 4. 验收标准

| 场景 | 输入 | 预期 | 验证方式 |
|------|------|------|---------|
| 正常动画 wait=true | `AnimateSprite(file="aiy20200_14", max=6, time=200, wait=true)` | 6 帧以 200ms 间隔播放，自动继续 | 目视 + 控制台 |
| 正常动画 wait=false | `AnimateSprite(..., wait=false)` | 6 帧异步播放，脚本立即继续 | 目视 |
| Style 0 | `style=0` | 播完后保持最后一帧 | 目视 |
| Style 1 | `style=1` | 播完后保持最后一帧（同 style 0） | 目视 |
| 跳过模式 | skip 中遇到 AnimateSprite | 仅加载最后一帧，不播放动画 | 目视 |
| 混合模式 | `draw=1`（add） | 加法混合渲染 | 目视 |
| 所有 16 套序列 | 各序列播放 | 帧资源和帧数正确 | 目视 |

---

## 5. 未涵盖范围

- **Mode B（sprite strip）**：`AnimateSpriteTable` 全部注释，暂不实现
- **`alltime` 参数**：原版 Lua 标签传了但 `AnimateSprite()` 未使用
- **`mask` 参数**：同上了，函数忽略
- **动画结束回调**：目前 wait 用 timer 到期机制，不支持事件回调
