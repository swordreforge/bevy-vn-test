# P2-6 事件/标记系统 设计文档

> 对应 Artemis 引擎的 View, Event, EventMN, EventCut, SetGlobalFlag, RouteFlag, GameMode, NextDay 标签

---

## 1. 总览

### 1.1 范围

| 子系统 | 标签 | 复杂度 | 说明 |
|--------|------|--------|------|
| View | `View`, `ViewEnd` | 高 | 角色名卡 + 羽毛笔动画 + mask 转场 |
| DrawScene | `Event`, `EventMN`, `EventCut` | 中 | 全屏事件 CG 显示，可复用 ShowCg 管线 |
| GlobalFlags | `SetGlobalFlag`, `RouteFlag` | 低 | 持久化 flag 存储 + 聚合逻辑 |
| GameMode | `GameMode` | 低 | 显示/模式切换 |
| NextDay | `NextDay` (macro) | 低 | 已由现有命令组成，确保 CallScript 解析 |

### 1.2 架构决策

新增 `src/plugins/event_system/` 模块，独立于 rendering/script_runner，包含：
- `mod.rs` — Plugin 注册
- `view.rs` — View 系统（状态机 + 动画）
- `draw_scene.rs` — Event/EventMN 全屏场景
- `flags.rs` — 全局标志持久化 + RouteFlag 逻辑

不新增文件仅修改的不单独成文件：
- `src/script.rs` — 新增 ScriptCmd 变体
- `tools/artemis-export/src/mapper.rs` — 新增标签映射

---

## 2. View 子系统（高复杂度）

### 2.1 原版 Lua 逻辑

```lua
function tags.View(e, param)
    local char = param["0"]
    -- skip中：只更新窗口色
    if get_skip() then
        e:tag{"calllua", function="tags.ChangeWindowColor", ["0"]=mw}
        return 1
    end
    -- 第1步：暗転（msgoff, reset_bg, reset_fg, trans fade=1000）
    msgoff(e, {}); reserve_check(); reset_bg(); reset_fg()
    trans(e, { fade=1000 })
    -- 第2步：描画组件（base + 名卡+隐藏 + 羽毛笔）
    lyc2{ id="1.50.0", eq=true, file=path..file }           -- 背景
    lyc2{ id="1.50.1", eq=true, file=path..name, x=x, y=393, visible="0" }  -- 名卡(隐藏)
    lyc2{ id="1.50.2.0", eq=true, file=path.."view_pen", x=640, y=0 }       -- 羽毛笔
    lyprop{ id="1.50.2", left=640, top=0 }
    trans(e, { fade=200 })
    -- 第3步：羽毛笔多关键帧动画（4套预设路径）
    tween{ id="1.50.2.0", x="650,300,"..tweentable.x, time=tweentable.t, ease="in"}
    tween{ id="1.50.2.0", y="-100,50,"..tweentable.y, time=tweentable.t, ease="in"}
    eqwait(200)
    -- 第4步：mask转场显示名卡
    lyprop{ id="1.50.1", visible="1" }
    trans(e, { fade=tweentable.r, rule=mask })         -- mask遮罩转场
    eqwait(1000)
    -- 第5步：消去
    lydel{ id="1.50" }
    trans(e, { fade=250 })
    -- 第6步：设置窗口色
    tags.ChangeWindowColor(mw)
end
```

### 2.2 ScriptCmd 变体

```rust
ScriptCmd::View { char_id: String },
```

`ViewEnd` 在 mapper 中展开为 `View { char_id: "ViewEnd" }`（与 Lua `tags.ViewEnd` → `tags.View(e, { ["0"]="ViewEnd" })` 一致）。

### 2.3 ViewTable 数据嵌入

view_table 和 view_tweentable 作为 Rust 静态数据嵌入引擎（`src/plugins/event_system/view_data.rs`），直接从 `var.lua` 翻译：

```rust
struct ViewEntry {
    name_file: &'static str,   // "view_name01"
    base_file: &'static str,   // "view_base01"
    mask_file: &'static str,   // "view_mask02"
    name_w: u16,               // 名卡宽度
    name_x: u16,               // 名卡X偏移
    pen_type: u8,              // 羽毛笔动画类型 1-4
    window_color: u8,          // 消息窗口色
}

struct ViewTween {
    x_points: &'static [f32],  // X轴关键帧
    y_points: &'static [f32],  // Y轴关键帧
    step_wait: u64,            // 每步等待ms
    reveal_time: u64,          // mask转场时间ms
}
```

### 2.4 状态机设计

专用 `ViewState` 组件驱动整个序列（`src/plugins/event_system/view.rs`）：

```rust
enum ViewPhase {
    FadeOut,          // 暗転 1000ms → 复用 ScreenOverlay black
    PrepareScene,     // 加载并放置base/name(隐藏)/pen精灵 → lydel旧 + DrawSprite*3
    FadeIn,           // フェードイン 200ms → 复用 ScreenOverlay alpha 0→1
    PenTween,         // 羽毛笔多关键帧路径动画 → 自定义ViewTween系统
    PenWait,          // eqwait(200ms) → 复用 auto_timer
    RevealName,       // mask转场显示名卡 → 自定义 MaskTransition
    DisplayWait,      // 静止 1000ms → 复用 auto_timer
    FadeOutScene,     // 消去 250ms → 复用 ScreenOverlay
    SetWindowColor,   // 应用窗口色 → 复用 ChangeWindowColor
    Done,             // 移除自身，脚本继续
}

struct ViewState {
    phase: ViewPhase,
    char_id: String,
    timer: Timer,
    entities: Vec<Entity>,  // 创建的精灵实体（用于后续清理）
}
```

View 状态机在 `advance_view` 系统中每帧运行。Phase 转换通过 timer 超时或动画完成触发。`ViewPhase::Done` 时移除 `ViewState` 组件，脚本继续下一条指令。

### 2.5 跳过模式（Skip）

skip 模式直接跳转到 `SetWindowColor` phase，不播放动画。与 Lua 原版 `if get_skip() then tags.ChangeWindowColor(...) return 1 end` 一致。

### 2.6 羽毛笔多关键帧

原版 Lua 使用 `tween{ x="650,300,"..tweentable.x }` 格式，其中前两个数值 `650,300` 是起始坐标偏移，后面跟着路径点。翻译为：

```rust
// 例如 pen_type=2: x_points = [650, 300, 136, 180, 214, 218, 389, 401, 485, 495, 650]
// tween 起始位置: x=640
// tween 第一帧: x=650 移动到 650
// tween 路径: 通过中间各点
```

实现方式：`ViewTweenSystem` 使用 `AnimationCurve` 逐点插值，每到达一个关键帧点更新精灵位置，到达最后一个点停止。

### 2.7 Mask 转场（名卡显示）

原版使用 `trans(e, { fade=1800, rule="view_mask02" })`，这是一个**纹理遮罩过渡**：名卡按照 mask 纹理的形状逐渐显现。

实现方案（优先级从高到低）：

1. **方案 A（推荐）：淡入近似** — 直接 alpha 淡入（`ImageNode.color.a` 0→1），耗时设为 `reveal_time`。视觉上不完全一致但效果合理。
2. **方案 B：自定义 UiMaterial** — 实现 `view_mask01/02/03` 的遮罩过渡 shader。需要新增 Bevy `UiMaterial` + WGSL 片段着色器，通过 `ImageNode.color.a` 驱动遮罩 lerp。还原度最高。

设计文档阶段先标注方案 A 为 v1 实现，方案 B 为后续打磨项。

### 2.8 名卡定位

原版名卡定位：`y=393`，`x` 从 view_table 取。全屏坐标映射（1280x720）：

```
名卡 y = 393（从顶部算）
名卡 x = view_table[name_x]
```

羽毛笔初始位置：`x=640, y=0`，然后 tween 起始偏移 `650,300`。

### 2.9 ViewEnd

`ViewEnd` 在 view_table 中是一条特殊记录（`name="view_name99"`, `base="viewend_base"`, `pen_type=4`, `window_color=0`）。其羽毛笔动画类型 4 有一个空的动画路径（pen_type=4 的路径实际与 2 相同但 window_color 不同）。

`ScriptCmd::ViewEnd` 在 mapper 中展开为 `ScriptCmd::View { char_id: "ViewEnd" }`。

### 2.10 资产验证

| 资产文件 | 用途 | 验证状态 |
|---------|------|---------|
| `image/view/view_base*.png` (11 文件) | View 背景图 | ✅ 所有 1280×720，尺寸正确 |
| `image/view/view_name*.png` (12 文件) | 角色名卡 | ✅ 460×110 / 580×110 / 220×110 |
| `image/view/view_pen.png` | 羽毛笔精灵 | ✅ 460×370 |
| `image/view/viewend_base.png` | ViewEnd 专用背景 | ✅ 1280×720 |
| `image/rule/view_mask01.png` (3 文件) | 遮罩纹理 | ✅ 1280×720 |
| 无 | view_table 数据 | 已从 var.lua 提取完毕 |

---

## 3. DrawScene 子系统（Event / EventMN / EventCut）

### 3.1 原版逻辑

```lua
function tags.Event(e, param)
    msgoff(e, param); reset_fg()
    param["2"] = "eve_"; param["3"] = 2
    tags.DrawScreen(e, param)
end

function tags.DrawScreen(e, param)
    local file = param["0"]; local Type = param["1"]
    local path = param["2"]
    -- 根据 Type 选择 DrawScene 或 DrawSceneWithMask
    if Type == "SUDDEN" then DrawScene(file, 0)
    elseif Type == "FAST" then DrawScene(file, "FAST")
    elseif Type == "SLOW" then DrawScene(file, "BG_SLOW")
    elseif Type == "CROSS" or Type2 == 2 or Type2 == 3 then DrawScene(file, "BG_TIME")
    elseif Type == "NORM" then DrawSceneWithMask(file, MASK***)
    else DrawSceneWithMask(file, "MASK0"..Type)
    end
end
```

`DrawScene` 是 Artemis 引擎的一个基本图元："全屏显示一张场景图，带过渡效果"。已与我们现有的 `ShowCg` 功能高度重叠。

### 3.2 ScriptCmd 变体

不新增独立变体。在 mapper 中展开为已有命令的序列：

```
Event(file, type, path) →
  ScriptCmd::Window { show: false, time: 0 }    // msgoff
  ScriptCmd::HideFg { char_id: "*", transition: None }  // reset_fg
  ScriptCmd::ShowCg { file: "ev/eve_XXYYZZ", transition: Transition::Fade }  // DrawScene
```

其中 `ShowCg` 已支持 `file`, `transition` 字段。`path` 前缀决定 asset 路径：
- Event → `ev/eve_`（即 `{file}` 为 `eve_XXYYZZ`）
- EventMN → `ev/mono/mon_`
- EventCut → 无资产，映射为 `ev/cut_`（预留路径）

### 3.3 过渡类型映射

mapper 中 `DrawScene` 的 Type 参数映射到 `ShowCg.transition`：

| DrawScene Type | ShowCg transition | 说明 |
|---------------|-------------------|------|
| SUDDEN | `Transition::Instant` | 瞬间显示 |
| FAST | `Transition::Fade` + duration=200 | 快速淡入 |
| SLOW | `Transition::Fade` + duration=1000 | 慢速淡入 |
| CROSS / type2=2/3 | `Transition::Fade` + duration=500 | 标准交叉淡入淡出 |
| NORM | `Transition::Fade` + duration=500 | 随机 mask 效果暂用淡入替代 |
| MASK0xx | `Transition::Fade` + duration=500 | 同上 |

### 3.4 资产验证

| 资产路径 | 用途 | 验证状态 |
|---------|------|---------|
| `image/ev/eve_*.png` (89 + 1 jpg) | Event 事件 CG | ✅ 358 文件，1280×720 |
| `image/ev/mono/mon_*.png` (14) | EventMN 单色变体 | ✅ 1280×720 |
| `image/thumbnail/mon_*.png` (13) | 缩略图 | ✅ 202×112，暂不用于主流程 |

---

## 4. GlobalFlags 子系统

### 4.1 需求

原版 `SetGlobalFlag(index, value)` 将 key-value 对持久化到 `gscr.gflag` 数组，保存在 save 文件中。`getGlobalFlag` 从中读取。

`RouteFlag` 是聚合逻辑：检查一组 flag（103-111）是否 >= 1，如果全部满足则设置 flag 113；再检查 113 + 151-167，全部满足则设置 flag 114。

### 4.2 ScriptCmd 变体

```rust
ScriptCmd::SetGlobalFlag { index: u32, value: i32 },
ScriptCmd::RouteFlag,
```

### 4.3 持久化存储

扩展现有 `ScriptEngine.flags: HashMap<String, i32>` 为两个命名空间：

```rust
pub struct ScriptEngine {
    // ... existing fields ...
    pub flags: HashMap<String, i32>,           // 已有：局部/临时 flag
    pub global_flags: HashMap<u32, i32>,       // 新增：持久化全局 flag
}
```

`global_flags` 在存档/读档时序列化。`SaveData` 结构体新增字段。

`SetGlobalFlag { index, value }` 写入 `global_flags`。

`RouteFlag` 执行聚合逻辑：
```rust
fn handle_route_flag(global_flags: &mut HashMap<u32, i32>) {
    // Flag 113: 全部六位女主角恋爱线 clear
    let route_flags = [103, 105, 107, 108, 110, 111];
    if global_flags.get(&113) != Some(&1) {
        let count = route_flags.iter()
            .filter(|&f| global_flags.get(f).unwrap_or(&0) >= &1)
            .count();
        if count == route_flags.len() {
            global_flags.insert(113, 1);
        }
    }
    // Flag 114: 全 clear（含おまけ）
    let complete_flags = [113, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167];
    if global_flags.get(&114) != Some(&1) {
        let count = complete_flags.iter()
            .filter(|&f| global_flags.get(f).unwrap_or(&0) >= &1)
            .count();
        if count == complete_flags.len() {
            global_flags.insert(114, 1);
        }
    }
}
```

### 4.4 存档兼容

`SaveData` 新增 `global_flags: HashMap<u32, i32>`。旧存档加载时若字段不存在则初始化为空。

---

## 5. GameMode 子系统

### 5.1 原版逻辑

```lua
function tags.GameMode(e, param)
    local flag = tonumber(param["0"])
    if flag == 2 then
        -- キー待ちモード（click-to-advance mode）
        -- 重新绑定按键：左键/滚轮推进
    elseif flag == 1 then
        -- 全画面モード（fullscreen）
    else
        -- ウィンドウモード（windowed）
    end
end
```

### 5.2 ScriptCmd 变体

```rust
ScriptCmd::GameMode { mode: u8 },  // 0=windowed, 1=fullscreen, 2=click-to-advance
```

### 5.3 实现

- mode 0/1 → 通过 `Settings` 资源触发 Bevy 的窗口模式切换：
  ```rust
  commands.insert_resource(Settings { fullscreen: mode == 1, .. });
  ```
  rendering plugin 的窗口设置系统已支持 `MonitorSelection::Current` + `Window::from_settings`。

- mode 2 (click-to-advance) → 设置 `Settings.auto_mode = AutoMode::ClickToAdvance` 或等效。实际上该引擎默认就是点击推进，此模式只需确保不进入 auto 模式即可。

---

## 6. NextDay

### 6.1 性质

`*NextDay` 是 `macro.iet` 中定义的**标签宏**，不是 ASB 标签。当脚本通过 `CallScript` 调用 `macro.iet` 的 `NextDay` 标签时，执行以下序列（全部已实现）：

```
msgoff           → ScriptCmd::Window { show: false, time: 0 }
Blackout 000     → ScriptCmd::ScreenOverlay { color: Black, time: 000 }
DrawSprite *3    → ScriptCmd::DrawSprite x3 (next_base, next_03, next_01)
FadeFilm 500     → ScriptCmd::ClearOverlay { time: 500 }
SEPlay           → ScriptCmd::PlaySe { file: "09640", volume: "NORM" }
MoveSprite 03    → ScriptCmd::MoveSprite (2000ms tween)
Wait 2000        → ScriptCmd::Wait { duration: 2000 }
Blackout 1000    → ScriptCmd::ScreenOverlay { color: Black, time: 1000 }
SEStop           → ScriptCmd::StopStreamingSe 或 PlaySe STOP
lydel            → 清理精灵
return           → ScriptCmd::Return
```

### 6.2 实现策略

**不新增 ScriptCmd。** 已有 `CallScript` 机制可以调用 `macro.bscript.ron` 中的 `NextDay` 标签。需要确保：

1. `macro.iet` 被转换为 `macro.bscript.ron`（属于 artemis-export pipeline，不在本设计范围内但为前置依赖）
2. 游戏启动时 `macro.bscript.ron` 被加载到 `ScriptEngine.scripts` 中

如果短期内不能完成 `macro.iet` 的解析，也可以在 mapper 中将 `NextDay` 调用直接展开为内联命令序列。

### 6.3 资产验证

| 资产文件 | 用途 | 验证状态 |
|---------|------|---------|
| `image/obj/next/next.jpg` | 背景 | ✅ 1280×720 |
| `image/obj/next/nextsc_02.jpg` | 场景 2 | ✅ 1280×720 |
| `image/obj/next/nextsc_03.jpg` | 场景 3 | ✅ 1280×1200 |
| `image/obj/next/nextsc_04.jpg` | 场景 4 | ✅ 1280×920 |
| `image/obj/dic/next_base.png` | 基础日历 | ✅ 1280×720 |
| `image/obj/dic/next_01.png` | 日历元素 | ✅ 1240×680 |
| `image/obj/dic/next_02.png` | 日历元素 | ✅ 1240×680 |
| `image/obj/dic/next_03.png` | 日历元素 | ✅ 1240×680 |

---

## 7. Mapper 变更

在 `tools/artemis-export/src/mapper.rs` 的 `map_command` 函数中新增：

| ASB 标签 | 映射输出 |
|---------|---------|
| `View` | `View { char_id: attrs["0"] }` |
| `ViewEnd` | `View { char_id: "ViewEnd" }`（复用 View 命令） |
| `Event(file, type)` | `Window{false} → HideFg{*} → ShowCg { file: concat("eve_", file), transition: map_transition(type) }` |
| `EventMN(file, type)` | `Window{false} → HideFg{*} → ShowCg { file: concat("mon_", file), transition: map_transition(type) }` |
| `EventCut(file, type)` | `Window{false} → HideFg{*} → ShowCg { file: concat("cut_", file), transition: map_transition(type) }` |
| `SetGlobalFlag` | `SetGlobalFlag { index, value }` |
| `GetGlobalFlag` | `Condition`（通过现有 `Condition` 实现 flag 读取判断） |
| `RouteFlag` | `RouteFlag` |
| `GameMode` | `GameMode { mode }` |

**关于 `DrawScene` 独立标签：** `DrawScene` 有时作为独立标签出现在 ASB 中（非通过 Event/EventMN 调用），参数含 `attrs["0"]`=file, `attrs["1"]`=type, `attrs["2"]`=path_prefix。mapper 处理：映射为 `ShowCg { file: concat(path_prefix, file), transition }`。

---

## 8. 模块结构

```
src/plugins/
├── mod.rs                          # pub mod event_system;
└── event_system/
    ├── mod.rs                      # EventSystemPlugin, 注册所有系统
    ├── view.rs                     # ViewState + advance_view 系统
    ├── view_data.rs                # view_table + view_tweentable 静态数据
    ├── draw_scene.rs               # DrawScene 处理（Event/EventMN 复用 ShowCg）
    └── flags.rs                    # SetGlobalFlag + RouteFlag 处理
```

`GameMode` 直接由 `script_runner` 处理（写入 Settings），不放入 event_system 模块。

---

## 9. 实现顺序

```
Phase 1 — 基础设施
├── ScriptCmd 变体：View, SetGlobalFlag, RouteFlag, GameMode
├── ScriptRunner 处理：SetGlobalFlag, RouteFlag, GameMode
├── Mapper 映射：SetGlobalFlag, RouteFlag, GameMode → Done
├── SaveData 扩展：global_flags 持久化

Phase 2 — View 系统
├── view_data.rs：view_table + view_tweentable 静态数据
├── view.rs：ViewState + advance_view 状态机（v1: alpha 淡入替代 mask 转场）
├── Mapper 映射：View, ViewEnd

Phase 3 — DrawScene + Event
├── draw_scene.rs：Event/EventMN/EventCut → ShowCg 映射
├── Mapper 映射：Event, EventMN, EventCut, DrawScene

Phase 4 — NextDay 兼容保证
├── 验证 CallScript 到 macro.iet 的 *NextDay 路径
├── 或 mapper 内联展开

Phase 5 — 打磨
├── View mask 转场：自定义 UiMaterial（方案 B）
├── RouteFlag 完整逻辑验证
```

---

## 10. 验收标准

### 10.1 View 验收

| 场景 | 输入 | 预期 | 验证方式 |
|------|------|------|---------|
| 正常 View | `View("EUS")` | 暗転→base显示→羽毛笔动画→名卡显示→1s→消去→窗口色改变 | 目视 |
| Skip 中 View | 跳过模式下遇到 View | 直接应用窗口色，不播放动画 | 目视 |
| ViewEnd | `ViewEnd` | 与 View 相同但用 ViewEnd 专用资产 | 目视 |
| 每个角色 | 12 个 view_table 条目 | 各角色名卡正确显示 | 逐一验证 |
| 羽毛笔动画 | 4 种 pen_type | 路径不同但都正确 | 目视 |

### 10.2 Event 验收

| 场景 | 输入 | 预期 | 验证方式 |
|------|------|------|---------|
| Event | `Event("010101")` | 隐藏窗口 → 隐藏立绘 → 显示 eve_010101 | 目视 + 控制台 |
| EventMN | `EventMN("010101")` | 显示 mon_010101 单色版 | 目视 |

### 10.3 Flag 验收

| 场景 | 输入 | 预期 | 验证方式 |
|------|------|------|---------|
| SetGlobalFlag | `SetGlobalFlag(103, 1)` | global_flags[103] = 1 | 存档→读取验证 |
| RouteFlag | 6 个 route flag 全设 | flag 113 = 1 | 存档验证 |
| 完整 RouteFlag | 全部 18 个 flag 全设 | flag 114 = 1 | 存档验证 |
| GameMode 0/1 | `GameMode(1)` | 窗口切成全屏 | 目视 |

---

## 11. 未涵盖范围

- **EventCut** 资产不存在（`cut_*` 0 文件），映射到 `ScriptCmd::DrawScene` 但不会有实际效果。如果日后发现使用场景再补资产。
- **RouteFlag** 调用 `system_save()`，引擎暂不自动存档（目前手动保存已实现）。
- **DrawScreen 的 NORM 随机 mask 效果** 使用随机 MASK001-015，v1 统一用普通淡入替代。
- **羽毛笔 ease="in"** 是原版 tween 的缓动参数。v1 用 ease-out 二次缓动。
