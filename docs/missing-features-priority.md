# 缺失功能优先级文档

> 依据 `root/scenario/main/` 下原始 `.asb` 二进制脚本的真实标签分析

---

## 总览

| 缺失类别 | 涉及标签 | 影响范围 |
|---------|---------|---------|
| 图层/渲染系统 | DrawSprite, DrawSpriteWithFiltering, FadeSprite, MoveSprite, DrawBG | 所有脚本 |
| 面部头像 | Face | 所有脚本（每文件 66~104 次） |
| 画面特效 | Quake, StartShakingOfAllObjects, Flash, Fadeout, WhiteoutBySA | 多数脚本 |
| 消息窗口控制 | Window, DisableWindow, ChangeWindowColor | 多数脚本 |
| 音频增强 | LoopSE, StopStreamingSE, BgmVol, BgmX | 多数脚本 |
| 背景系统 | ScrollBG, DrawBG | 部分脚本 |
| 动画系统 | FadeSprite, MoveSprite, AnimateSprite | 部分脚本 |
| 其他 | PlayMovie, RegisterTextToHistory, ChangeWindowDesign | 少量脚本 |

---

## P0 — 严重缺失（核心体验受损）

### P0-1 Face 面部头像系统

| 项目 | 内容 |
|------|------|
| **问题** | `Face` 标签被 mapper 错误映射为 `ShowFg`（全身立绘），原版应显示小头像在消息窗口内（图层 `1.80.10`） |
| **使用量** | 序章 66 次，后续章节 6~104 次。**远多于 Tati/TatiFa（立绘）** |
| **影响** | 每次对话都显示全身立绘代替小头像，画面构图完全错误；且 `Face` 和 `Tati` 可以同时存在（立绘+头像），当前无法实现 |
| **资源** | `root/image/face/` — 1192 个文件 |
| **路径映射** | `images/face/{char_id}_{expression}.png` |
| **原版 Lua** | `system/adv/fg.lua` 中 `set_face()` / `del_face()` — 独立图层 `1.80.10` |

### P0-2 精灵/覆盖物系统（DrawSprite + FadeSprite + MoveSprite）

| 项目 | 内容 |
|------|------|
| **标签** | `DrawSprite`、`DrawSpriteWithFiltering`、`FadeSprite`、`FadeSpriteWithFiltering`、`MoveSprite`、`DrawBustshotWithFiltering`、`DrawSpriteEx` |
| **使用量** | 序章: DrawSprite 10 + DrawSpriteWithFiltering 12 + FadeSprite 12 + MoveSprite 11 = **45 次**。几乎所有章节脚本都有 |
| **影响** | 所有覆盖物精灵（特效层、装饰元素、表情包图、特写框等）完全丢失。加上无 FadeSprite/MoveSprite，精灵无任何动画（突现突隐） |
| **资源** | `root/image/obj/`、`root/image/anime/`（81 帧动画）等 |
| **原版 Lua** | `system/adv/grph.lua` 中 `sprite()`、`move_sprite()`、`AnimateSprite()` |

**需要实现的功能**：
- 任意位置（x,y）放置精灵
- 透明度（alpha，0~255）
- 深度缩放（基于 z 值的 `getSpriteDepthScale`）
- 旋转
- 锚点（anchorx, anchory）
- 混合模式：normal / add / multiply / screen
- Mask 遮罩
- 精灵间 Tween 动画（alpha/位置/缩放/旋转）
- 多关键帧移动（`move_sprite_ex`）

### P0-3 全屏过渡效果（Fadeout / Whiteout / Blackout）

| 项目 | 内容 |
|------|------|
| **标签** | `Fadeout`, `WhiteoutBySA`, `Blackout` |
| **使用量** | 序章 Fadeout 8 次。全篇各脚本 1~8 次 |
| **影响** | 场景切换时无渐黑/渐白过渡，画面突兀跳变 |
| **原版 Lua** | `trans()` 在 `grph.lua` — 支持 time + rule 图像 + type 的过渡系统 |

### P0-4 消息窗口控制（Window / DisableWindow / ChangeWindowColor）

| 项目 | 内容 |
|------|------|
| **标签** | `Window`, `DisableWindow`, `ChangeWindowColor`, `ChangeWindowDesign` |
| **使用量** | 序章 Window 13 次，DisableWindow 3 次，ChangeWindowColor 4 次。全篇 Window 约 145 次 |
| **影响** | 全屏 CG/Event 场景需要 `DisableWindow` 隐藏消息框，`ChangeWindowColor` 适配不同场景氛围。缺失导致全屏场景时对话框错误显示 |
| **原版 Lua** | `msgon()` / `msgoff()` 在 `grph.lua` — 控制图层 `1.80` 的 alpha 和位置 tween |

---

## P1 — 高优先级（明显缺失）

### P1-1 画面震动系统（Quake / StartShaking）

| 项目 | 内容 |
|------|------|
| **标签** | `Quake`, `StartShakingOfAllObjects`, `TerminateShakingOfAllObjects` |
| **使用量** | 序章 5 次。aiy50010、aiy81010 等章节也有使用 |
| **影响** | 战斗/爆炸/冲击场景完全无震动反馈 |
| **原版 Lua** | `quake()` 和 `start_shaking()` / `terminate_shaking()` 在 `grph.lua` |
| **参数** | 方向(dir)、强度(level)、频率(freq)、次数(rep)、衰减(atten)、衰减曲线(handle_shaking_x/y) |

### P1-2 循环 SE（LoopSE / StopStreamingSE）

| 项目 | 内容 |
|------|------|
| **标签** | `LoopSE`, `StopStreamingSE` |
| **使用量** | 序章 StopStreamingSE 18 次 + LoopSE 4 次。全篇 StopStreamingSE 约 60 次 |
| **影响** | 环境音（雨声、风声、场景氛围音）无法循环播放。`StopStreamingSE` 用于停止之前启动的循环音 |
| **资源** | `root/se/loop/` — 118 个循环 SE 文件 |
| **需要** | 支持 PlaySe 带 loop 参数 + StopSe/id 定向停止 |

### P1-3 BGM 精细控制（BgmVol / BgmX）

| 项目 | 内容 |
|------|------|
| **标签** | `BgmVol`, `BgmX` |
| **使用量** | 序章 BgmVol 2 次 + BgmX 2 次。aiy40010 等也有 |
| **影响** | 无法渐变 BGM 音量，无法交叉淡入淡出切换 BGM |
| **需要** | `PlayBgm` 支持 fade_in/fade_out 参数；`BgmVol` 实时音量变化；`BgmX` 交叉淡入淡出 |

### P1-4 Flash 闪光效果

| 项目 | 内容 |
|------|------|
| **标签** | `Flash` |
| **使用量** | aiy70110 3 次、aiy80010 1 次（高潮/结局章节） |
| **影响** | 关键剧情瞬间缺少视觉冲击 |
| **原版 Lua** | `flash()` 在 `grph.lua` — 白色覆盖层闪烁，支持多次闪烁和淡入淡出 |

### P1-5 背景滚动（ScrollBG）

| 项目 | 内容 |
|------|------|
| **标签** | `ScrollBG` |
| **使用量** | aiy50010 2 次（Tia 线，重要场景） |
| **影响** | 全景/移动背景（如火车窗景、移动的观景）无法实现 |
| **原版 Lua** | `bg_scroll()` 在 `grph.lua` — 通过 x1/x2/y1/y2 + time 参数实现背景偏移动画 |

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

### P2-6 过场/事件标记

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

### P3-3 BgmX 交叉淡入淡出

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
| aiy50010 | 107 | 9 | 3 | 1 | 0 | 1 | 4 | 5 | 3 | 2 |
| aiy70110 | 66 | 2 | 2 | 0 | 0 | 0 | 8 | 10 | 3 | 0 |
| aiy80010 | 120 | 6 | 1 | 0 | 0 | 1 | 4 | 7 | 0 | 0 |
| aiy81010 | 25 | 4 | 9 | 7 | 4 | 2 | 2 | 2 | 1 | 0 |

> `DrawSprite*` = DrawSprite + DrawSpriteWithFiltering + DrawBustshotWithFiltering + DrawSpriteEx

---

## 建议实施顺序

```
Phase 1 (P0) — 核心修复
├── Face 头像系统（重新映射，独立图层 1.80.10）
├── 精灵系统 DrawSprite（基础版：位置+alpha+图片）
├── 精灵动画 FadeSprite / MoveSprite（tween 系统）
├── 窗口控制 Window / DisableWindow
└── 全屏过渡 Fadeout / Whiteout / Blackout

Phase 2 (P1) — 常规体验
├── 画面震动 Quake / StartShaking
├── 循环 SE LoopSE / StopStreamingSE
├── BGM 控制 BgmVol / BgmX
├── Flash 闪光
└── 背景滚动 ScrollBG

Phase 3 (P2) — 视觉增强
├── 帧动画 AnimateSprite
├── 模糊效果 Blur
├── 雨特效 Rain
├── 图像波动 Image Wavering
├── 电影播放 PlayMovie
└── 事件/标记系统

Phase 4 (P3) — 打磨
├── ChangeWindowDesign
├── RegisterTextToHistory
├── BgmX 交叉淡入淡出
└── DrawBG 多层背景
```
