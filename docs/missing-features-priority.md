# 缺失功能优先级文档

> 依据 `root/scenario/main/` 下原始 `.asb` 二进制脚本的真实标签分析

---

## 总览

| 缺失类别 | 涉及标签 | 影响范围 |
|---------|---------|---------|
| 图层/渲染系统 | DrawSprite, DrawSpriteWithFiltering, FadeSprite, MoveSprite, DrawBG | 所有脚本 ✅ |
| 面部头像 | Face | 所有脚本（每文件 66~104 次） ✅ |
| 画面特效 | Quake, StartShakingOfAllObjects, Flash, Fadeout, WhiteoutBySA | 多数脚本 |
| 消息窗口控制 | Window, DisableWindow, ChangeWindowColor | 多数脚本 |
| 音频增强 | LoopSE, StopStreamingSE, BgmVol, BgmX | 多数脚本 |
| 背景系统 | ScrollBG ✅, DrawBG | 部分脚本 |
| 动画系统 | AnimateSprite | 部分脚本 |
| 其他 | PlayMovie, RegisterTextToHistory, ChangeWindowDesign | 少量脚本 |

---

## P0 — 严重缺失（核心体验受损）

### P0-1 ~~Face 面部头像系统~~ ✅ 已完成

| 项目 | 内容 |
|------|------|
| **状态** | ✅ 已实现 |
| **改动** | `Face` → `ShowFace`，`ClrFace` → `HideFace`；头像作为对话框子节点 276×144 显示，底部对齐 |
| **提交** | `36ae2f0` |

### P0-2 ~~精灵/覆盖物系统（DrawSprite + FadeSprite + MoveSprite）~~ ✅ 已完成

| 项目 | 内容 |
|------|------|
| **状态** | ✅ 已实现 |
| **改动** | `DrawSprite`/`DrawSpriteWithFiltering`/`FadeSprite`/`FadeSpriteWithFiltering`/`MoveSprite` 映射到对应 `ScriptCmd`；精灵以独立 UI 节点实体管理，支持 ID 复用、alpha、Z 排序、深度缩放、旋转、fade-out 渐隐、move tween |
| **资源** | `root/image/obj/`、`root/image/anime/`（81 帧动画）等 |
| **提交** | `5d9f0cc` — 深度缩放 + 旋转 + fade-in + MoveSprite z-tween

**需要实现的功能**：
- ~~任意位置（x,y）放置精灵~~ ✅
- ~~透明度（alpha，0~255）~~ ✅
- ~~深度缩放（基于 z 值的 `getSpriteDepthScale`）~~ ✅ 使用 `1/(1 + z*0.001)` 公式通过 `Transform::scale` 实现
- ~~旋转（rotation）~~ ✅ 通过 `Transform::rotation` 实现，ASB 角度值自动转弧度
- 锚点（anchorx, anchory） ⏳ 数据已从 ASB 映射并传至 `DrawSpriteMessage`，但 Bevy 0.18 的 UI 节点无双 `TransformOrigin` 组件，定位默认以节点中心为原点。需要等 Bevy 升级或自行计算偏移量
- 混合模式：normal / add / multiply / screen ⏳ `SpriteBlendMode` 枚举已定义，`SpriteOverlay::blend_mode` 已存储，但需要自定义 `UiMaterial` 实现不同 blend state。目前仅 `Normal` 有效，其余 fallback 到 normal
- Mask 遮罩 ⏳ 暂未实现（需要 stencil buffer 或 clipping parent wrapper）
- ~~精灵间 Tween 动画（alpha/位置/深度缩放）~~ ✅
- 多关键帧移动（`move_sprite_ex`） ⏳ 暂未映射

### P0-3 ~~全屏过渡效果（Fadeout / Whiteout / Blackout）~~ ✅ 已完成

| 项目 | 内容 |
|------|------|
| **状态** | ✅ 已实现 |
| **标签** | `Fadeout`, `WhiteoutBySA`, `Blackout` |
| **改动** | `Fadeout` 在映射层拆分为 `ScreenOverlay → Wait → SetBg → ClearOverlay` 序列；持久 `ScreenOverlayRoot` UI 节点控制全屏遮罩 alpha tween；`Blackout`/`WhiteoutBySA` 直接映射为 `ScreenOverlay` |
| **提交** | `61f0109`

### P0-4 ~~消息窗口控制（Window / DisableWindow / ChangeWindowColor）~~ ✅ 已完成

| 项目 | 内容 |
|------|------|
| **状态** | ✅ 已实现 |
| **标签** | `Window`, `DisableWindow`, `ChangeWindowColor`, `ChangeWindowDesign` |
| **改动** | `Window`/`DisableWindow`/`EnableWindow` → `ScriptCmd::Window` 控制 `DialogueUiRoot` 显隐 + `WindowOverride` 阻止自动覆盖；`ChangeWindowColor`（0=默认,1=蓝,2=绿,3=红）和 `ChangeWindowDesign`（0=normal,1=small）通过 `apply_window_appearance` 系统实时应用 |
| **提交** | `61f0109`

---

## P1 — 高优先级（明显缺失）

### P1-1 ~~画面震动系统（Quake / StartShaking）~~ ✅ 已完成

| 项目 | 内容 |
|------|------|
| **状态** | ✅ 已实现 |
| **标签** | `Quake` |
| **改动** | `Quake` 标签 → `ScriptCmd::Quake { power, time }` → `QuakeState` 资源 → `quake_update` 系统对 Camera2d 施加随机偏移，强度随时间衰减 |
| **备注** | `StartShakingOfAllObjects` / `TerminateShakingOfAllObjects` 尚未映射（当前 Quake 单次触发即完成） |

### P1-2 ~~循环 SE（LoopSE / StopStreamingSE）~~ ✅ 已完成

| 项目 | 内容 |
|------|------|
| **状态** | ✅ 已实现 |
| **标签** | `LoopSE`, `StopStreamingSE` |
| **改动** | `LoopSE` → `ScriptCmd::LoopSe { file, volume, channel }` → `handle_loop_se` 用 `PlaybackMode::Loop` 播放，`SeManager` 通过 `HashMap<u32, Entity>` 追踪每个 channel 的实体；`StopStreamingSE` → `ScriptCmd::StopStreamingSe { channel }` → `handle_stop_streaming_se` 查找 channel 并 despawn；calllua `se_stop` 也映射到此命令 |
| **音频路径** | `audio/se/loop/` — 118 个循环 SE 文件 |

### P1-3 BGM 精细控制（BgmVol / BgmX）✅ 已完成

| 项目 | 内容 |
|------|------|
| **标签** | `BgmVol`, `BgmX` |
| **使用量** | 序章 BgmVol 2 次 + BgmX 2 次。aiy40010 等也有 |
| **影响** | 无法渐变 BGM 音量，无法交叉淡入淡出切换 BGM |
| **状态** | ✅ BgmVol 已实现 |
| **改动** | `BgmVol` 标签 + `ChangeVolumeOfBGM`/`bgm_fade` calllua → `ScriptCmd::BgmVol { channel, volume }` → `commands.queue` 写入 `Settings.bgm_volume`，`apply_audio_settings` 每帧同步到 `AudioSink` |
| **音量映射** | MIN=0/128, LOW=30/128, NORM=80/128, HIGH=128/128 |
| **待做** | BgmX 第二 BGM 层 + 交叉淡入淡出 |

### P1-4 ~~Flash 闪光效果~~ ✅ 已完成

| 项目 | 内容 |
|------|------|
| **状态** | ✅ 已实现 |
| **标签** | `Flash` |
| **改动** | `Flash` 标签 → `ScriptCmd::Flash { color, time, alpha }` → 复用 `ScreenOverlayRoot` + `OverlayTween`，从指定透明度 `alpha/255` 渐出到 0，完成后自动隐藏 |
| **备注** | 目前单次闪烁。原版 Lua 支持多次闪烁（rep），暂未实现 |

### P1-5 ~~背景滚动（ScrollBG）~~ ✅ 已完成

| 项目 | 内容 |
|------|------|
| **标签** | `ScrollBG` |
| **使用量** | aiy50010 2 次（Tia 线，重要场景） |
| **影响** | 全景/移动背景（如火车窗景、移动的观景）无法实现 |
| **原版 Lua** | `bg_scroll()` 在 `grph.lua` — 通过 x1/x2/y1/y2 + time 参数实现背景偏移动画 |
| **实现** | `ScriptCmd::ScrollBg { file, x1, y1, x2, y2, fade, wait }` → `ScrollBgMessage` → `handle_scroll_bg` 加载图片并设为自然像素尺寸 → `BgScroll` 组件驱动 ease-out 二次插值 left/top。`wait` 通过 `auto_skip.auto_timer` 阻塞。`handle_set_bg` 重置节点大小并取消滚动。ASB 映射 `ScrollBGenq` / `scroll_bg` → `ScriptCmd::ScrollBg` |
| **提交** | `5a0f9e7f` (mapper), `e5f322bb` (dead_code fix), `6fde3f66` (rendering), `ed38f639` (runner), `15dfaa98` (component), `4e48fe0a` (message), `58161e14` (script) |

---

## P2 — 中优先级（锦上添花）

### P2-1 帧动画系统（AnimateSprite）

| 项目 | 内容 |
|------|------|
| **标签** | `AnimateSprite`（对应原版 Lua 的 `AnimateSprite()`） |
| **使用量** | 当前 .asb 提取中未大量出现该标签名（可能使用不同命名），但 `root/image/anime/` 有 81 个动画资源 |
| **资源** | `root/image/anime/` — 81 帧动画图片 |
| **原版 Lua** | `AnimateSprite()` 在 `grph.lua` — 支持序列帧动画和横排组合动画 |

### P2-2 模糊效果（Blur）

| 项目 | 内容 |
|------|------|
| **标签** | `blurx`, `blur_set`, `del_blur`, `blur_reset`（通过 calllua 调用） |
| **资源** | `root/image/obj/blur/` — 模糊特效图 |
| **原版 Lua** | `blurx()` / `blur_set()` 在 `grph.lua` — 方向性模糊叠加层 |

### P2-3 雨特效（Rain）

| 项目 | 内容 |
|------|------|
| **标签** | `rain_mja`, `startRain`, `stopRain`（通过 calllua 调用） |
| **资源** | `root/image/obj/rain/`、`root/movie/aiy*_rain.ogv`（6 个雨视频） |
| **原版 Lua** | `rain_mja()` / `startRain()` / `stopRain()` 在 `grph.lua` |

### P2-4 图像波动（Image Wavering）

| 项目 | 内容 |
|------|------|
| **标签** | `image_wavering` |
| **原版 Lua** | `image_wavering()` 在 `grph.lua` — 热浪/水面波动效果，含 x 轴左右摆动 + xscale 缩放动画 |

### P2-5 电影播放（PlayMovie）

| 项目 | 内容 |
|------|------|
| **标签** | `PlayMovie`, `MovieInit`, `WaitToFinishMoviePlayingOnSprite` |
| **使用量** | aiy50010 1 次 PlayMovie + 1 次 MovieInit |
| **资源** | `root/movie/` — 6 个完整过场动画 + 6 个雨覆盖层 |
| **现状** | `ScriptCmd::PlayMovie` 已定义但 runner 未实现 |

### P2-6 过场/事件标记✅ 已完成

| 项目 | 内容 |
|------|------|
| **标签** | `ViewEnd`, `View`, `Event`, `EventMN`, `GameMode`, `NextDay`, `SetGlobalFlag`, `RouteFlag` |
| **使用量** | 分散在各脚本 |
| **影响** | 游戏模式切换、天数推进、全局标记等逻辑功能缺失 |

---

## P3 — 低优先级（细节/打磨）

### P3-1 ChangeWindowDesign

| 项目 | 内容 |
|------|------|
| **使用量** | aiy70110 1 次 |
| **影响** | 消息窗口设计主题切换 |

### P3-2 RegisterTextToHistory

| 项目 | 内容 |
|------|------|
| **使用量** | 序章 12 次，aiy20010 8 次 |
| **影响** | 我们已经通过 Dialogue 命令自动捕获到 backlog，此标签可能用于非 Dialogue 文本 |
| **备注** | 现有 backlog 系统已覆盖大部分需求，但需要确认此标签注册的内容是否不同 |

### P3-3 ~~BgmX 交叉淡入淡出~~ ✅ 已实现

| 项目 | 内容 |
|------|------|
| **使用量** | 序章 2 次 |
| **影响** | BGM 切换时音频平滑过渡 |

### P3-4 DrawBG 多层背景

| 项目 | 内容 |
|------|------|
| **使用量** | 序章 DrawBG 7 次，aiy20010 3 次 |
| **影响** | 可能用于多层视差背景合成 |

---

## 各章节 - 标签使用量汇总

| 脚本 | Tati/Fa | Face | DrawSprite* | FadeSprite | MoveSprite | Quake | Fadeout | Window | LoopSE | ScrollBG |
|------|---------|------|-------------|------------|------------|-------|---------|--------|--------|----------|
| aiy00010 | 60 | 66 | 22 | 12 | 11 | 5 | 8 | 13 | 4 | 0 |
| aiy10010 | 160 | 5 | 1 | 0 | 1 | 1 | 11 | 12 | 0 | 0 |
| aiy20010 | 74 | 12 | 9 | 8 | 4 | 0 | 5 | 5 | 0 | 0 |
| aiy30010 | 112 | 25 | 1 | 1 | 0 | 0 | 3 | 3 | 1 | 0 |
| aiy40010 | 115 | 6 | 2 | 0 | 0 | 0 | 5 | 5 | 0 | 0 |
| aiy50010 | 107 | 9 | 3 | 1 | 0 | 1 | 4 | 5 | 3 | 2 ✅ |
| aiy70110 | 66 | 2 | 2 | 0 | 0 | 0 | 8 | 10 | 3 | 0 |
| aiy80010 | 120 | 6 | 1 | 0 | 0 | 1 | 4 | 7 | 0 | 0 |
| aiy81010 | 25 | 4 | 9 | 7 | 4 | 2 | 2 | 2 | 1 | 0 |

> `DrawSprite*` = DrawSprite + DrawSpriteWithFiltering + DrawBustshotWithFiltering + DrawSpriteEx

---

## 建议实施顺序

```
Phase 1 (P0) — 核心修复
├── ✅ Face 头像系统（重新映射，对话框内子节点定位）
├── ✅ 精灵系统 DrawSprite（位置+alpha+图片 + FadeSprite/MoveSprite tween）  
│   └── 深度缩放 ✅ | 旋转 ✅ | 锚点 ⏳ | 混合模式 ⏳ | Mask ⏳
├── ✅ P0-3 全屏过渡 Fadeout / Whiteout / Blackout
├── ✅ P0-4 窗口控制 Window / DisableWindow / ChangeWindowColor
```
Phase 1 (P0) — 核心修复
├── ✅ Face 头像系统（重新映射，对话框内子节点定位）
├── ✅ 精灵系统 DrawSprite（位置+alpha+图片 + FadeSprite/MoveSprite tween）  
│   └── 深度缩放 ✅ | 旋转 ✅ | 锚点 ⏳ | 混合模式 ⏳ | Mask ⏳
├── 窗口控制 Window / DisableWindow
├── ✅ 全屏过渡 Fadeout / Whiteout / Blackout
├── ✅ 窗口控制 Window / DisableWindow

Phase 2 (P1) — 常规体验
├── ✅ 画面震动 Quake / StartShaking
├── ✅ 循环 SE LoopSE / StopStreamingSE
├── ✅ BGM 控制 BgmVol
├── ✅ BgmX + 交叉淡入淡出
├── ✅ Flash 闪光
├── ✅ 背景滚动 ScrollBG

Phase 3 (P2) — 视觉增强
├── 帧动画 AnimateSprite
├── 模糊效果 Blur
├── 雨特效 Rain
├── 图像波动 Image Wavering
├── 电影播放 PlayMovie
└── 事件/标记系统

Phase 4 (P3) — 打磨
├── ✅ ChangeWindowDesign
├── RegisterTextToHistory
├── ✅ BgmX 交叉淡入淡出
└── DrawBG 多层背景
```
