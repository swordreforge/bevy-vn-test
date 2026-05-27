# Bevy VN Engine — 项目进度

> 基于 Bevy 0.18 的视觉小说引擎，支持 Artemis 引擎资源导入

---

## 当前状态 (2026-05-27)

**Phase 0-7 核心引擎就绪 ✅** — 脚本驱动全流程：对话/立绘/背景/CG/精灵覆盖层/转场/BGM+BGMX/SE/语音/选项/Wait 时序均可用，画廊 + Debug 键

---

## 已完成功能

### 项目基础
- [x] Bevy 0.18 项目创建，依赖配置（bevy, serde, ron, anyhow）
- [x] 分层 Plugin 架构（src/plugins/）
- [x] 构建配置优化（dev opt-level 1, release LTO）

### 状态管理 (src/state.rs)
- [x] `AppState` 状态机：Boot → Title → Gameplay → Menu/SaveLoad/Gallery/Settings
- [x] 使用 Bevy 0.18 States API（OnEnter/OnExit/in_state）

### 标题画面 (src/plugins/title.rs)
- [x] 黑色背景 + 居中 "Visual Novel Engine" 标题文字
- [x] 点击/触摸 → 切换到 Gameplay 状态
- [x] 退出时自动清理 UI 实体

### 输入处理 (src/plugins/inputs.rs)
- [x] AdvanceEvent — 左键/触摸触发（非 Title 状态时）
- [x] MenuToggleEvent — Escape 键触发

### 对话系统 (src/plugins/dialogue.rs)
- [x] 底部半透明对话框（200px 高）
- [x] 角色名显示（橙色, 24px）
- [x] 对话文字显示（白色, 20px）
- [x] 逐字显示进度追踪
- [x] Gameplay 退出时自动清理 UI

### 脚本系统 (src/script.rs)
- [x] ScriptCmd enum（20 种指令类型）
  - dialogue, choice, set_bg, show_fg/hide_fg, show_cg/hide_cg
  - play_bgm/stop_bgm, play_bgmx/stop_bgmx, play_se, play_voice
  - affection_change, jump, call/return, condition
  - save_point, clear_text, wait, play_movie, label
- [x] ChoiceOption, ConditionOp 等辅助类型
- [x] ScriptEngine 资源（记录执行位置和调用栈）

### 好感度系统 (src/plugins/affection.rs)
- [x] AffectionMap 资源（HashMap<角色ID, 好感度值>）

### 存档系统 (src/plugins/save_load.rs)
- [x] SaveData 结构体（脚本状态、好感度、CG解锁、游戏时间等）
- [x] SaveManager 资源（15 个存档槽位）

### 设置 (src/plugins/settings.rs)
- [x] Settings 资源（BGM/SE/语音音量、文字速度、自动/跳过模式）
- [x] 占位 UI

### CG 画廊 (src/plugins/gallery.rs)
- [x] UnlockState 资源（CG/BGM/场景解锁追踪）
- [x] 占位 UI

### Artemis 原始素材
- [x] 使用 pfs_unpacker 解包 root.pfs（v8, 38807 文件）
- [x] 解包 root.pfs.000（v6, 5647 文件）
- [x] 原始素材目录：`/home/swordreforge/Downloads/game-source/`

---

## 项目结构

```
bevy-vn/
├── Cargo.toml                    # Rust 项目配置
├── .gitignore
├── PROGRESS.md                   # 本文档
├── assets/                       # 运行时资源（Phase 2 起使用）
│   ├── config/
│   ├── fonts/
│   └── scripts/
└── src/
    ├── main.rs                   # 入口，注册所有 Plugin
    ├── state.rs                  # AppState 枚举
    ├── components.rs             # UI 组件标记 (DialogueBox, etc.)
    ├── resources.rs              # 全局资源 (AffectionMap, SaveManager, Settings, UnlockState, DialogueState)
    ├── events.rs                 # （预留）
    ├── script.rs                 # ScriptCmd 指令定义 + ScriptEngine
    └── plugins/
        ├── mod.rs                # 模块声明
        ├── title.rs              # 标题画面
        ├── inputs.rs             # 全局输入
        ├── affection.rs          # 好感度
        ├── save_load.rs          # 存档/读档
        ├── dialogue.rs           # 对话 UI
        ├── settings.rs           # 设置
        └── gallery.rs            # CG 画廊
```

### 外部目录

```
/Downloads/
├── bevy-0.18.1/         # Bevy 引擎源码（参考）
├── bevy-vn/             # 本项目
└── game-source/         # Artemis 原始解包素材
```

---

## 待实现功能

### Phase 2: 脚本系统
- [x] .bscript.ron 文件加载器（ScriptLoader）
- [x] ScriptRunner 系统（顺序执行指令）
- [x] 跳转/调用栈/条件分支
- [x] 文本逐字显示动画
- [x] 用户点击推进到下一句
- [x] 示例脚本数据驱动测试

### Phase 3: 对话 + 立绘 + 背景 + 精灵覆盖层 + 转场 ✅
- [x] 立绘系统（显示/隐藏/切换/表情/位置）— 3 slot pooled entities, on-demand AssetServer loading
- [x] 背景系统（图片切换/双缓冲）— dual-buffer for cross-fade
- [x] CG 全屏显示 — overlay entity (ZIndex 2), auto-cleanup on hide
- [x] 全屏转场（Fadeout/Blackout/WhiteoutBySA）— ScreenOverlayRoot + OverlayTween (ease-out-quad)
- [x] 精灵覆盖层系统（DrawSprite/FadeSprite/MoveSprite）— ZIndex capped at 2, alpha via ImageNode.color
- [x] 旁白覆盖层 — 改为走 DrawSprite（_tx 触发 narration_wait），删除了 NarrationOverlay 资源
- [x] 旁白自动推进 — DrawSprite _tx → Wait 自动设 timer 推进
- [x] 精灵居中 — SpriteAnchor + Assets<Image> 纹理尺寸驱动，同步 SpriteTween
- [x] 角色立绘精灵加载 — on-demand + TextureCache
- [x] ObjFileIndex — 扫描 root/image/obj/ → RON 文件，运行时 O(1) HashMap 查询

### Phase 4: 音频 + 选项 ✅
- [x] BGM 播放/停止（PlaybackSettings::LOOP, AudioSink volume control）
- [x] BGM 拼接（_a + _b.ogg → rodio PCM 拼接 → WAV 循环）
- [x] BGM 执行顺序修复（handle_play_bgm → process_pending_bgm 同帧有序，`.chain()` 保证）
- [x] SE 播放（PlaybackSettings::DESPAWN, auto-cleanup）
- [x] 语音播放（flat file path, DESPAWN mode）
- [ ] 音量控制（Settings 已定义字段, UI deferred to Phase 7）
- [x] 选项分支 UI 和交互（center overlay, ZIndex 4, hover/press colors）
- [x] 测试脚本覆盖：PlayBgm/PlaySe/PlayVoice/Choice 分支（good_end/bad_end）

### Phase 5: 存档系统 ✅
- [x] 存档文件 I/O（JSON 序列化到 saves/ 目录）
- [x] 存档/读档 UI 界面（15 槽网格，ZIndex 5，确认对话框）
- [x] Menu 状态（Escape 切换，Save/Load/Settings/Gallery/Title 按钮）
- [x] 存档缩略图（CG 原图裁剪 256×144 PNG，通过 image crate 实时处理）
- [ ] 自动存档/快速存档（deferred）

### Phase 6: 好感度 + 画廊 ✅
- [x] AffectionCondition 脚本命令（基于 AffectionMap 分支）
- [x] UnlockCg 脚本命令（显式解锁 CG）
- [x] ShowCg 自动解锁 CG
- [x] 画廊界面（3×3 网格，ZIndex 5，缩略图/锁定占位符）
- [x] 全屏 CG 查看（点击/Escape 关闭）
- [x] 分页导航（左右箭头，Page x/Total 文字）
- [x] Gallery UI 重构：提取 `populate_gallery_grid` 辅助函数，消除三份重复网格构建代码
- [x] Safe Mode 勾选框 — 开启后自动隐藏 `hcg***` 开头的 NSFW 图片（267/357 张被屏蔽）
- [x] Debug 解锁键（Gallery 界面按 U 解锁全部 357 张 CG）

### Phase 7: 设置 + 打磨 ✅
- [x] Settings interactivity (sliders + toggles wired to runtime)
- [x] 过渡动画系统 (BG cross-fade, FG/CG fade, state fade-to-black)
- [x] 窗口控制（Window/DisableWindow/EnableWindow/ChangeWindowColor/ChangeWindowDesign）
- [x] 对话框 ZIndex 3，菜单 ZIndex 5，精灵覆盖层 ZIndex 2（正确层级）
- [x] Wait 命令时序修复：非 skip 模式下都会暂停指定时长（不再只作用于 auto/narration）
- [x] BgmVol — BGM 音量实时控制（MIN/LOW/NORM/HIGH 映射到 0~1）
- [x] Quake — 屏幕震动（Camera2d 随机偏移，强度随时间衰减）
- [x] Flash — 全屏闪光（复用 ScreenOverlayRoot + OverlayTween，支持颜色/透明度/时长）
- [x] LoopSE / StopStreamingSE — 循环SE系统（SeManager 追踪 channel→Entity，LOOP 模式播放，定向停止）
- [x] BgmX — 第二 BGM 层（BgmXManager + AudioType::BgmX，独立音轨 `audio/bgm/bgmx_{id}.ogg`）
- [x] 交叉淡入淡出 — BgmFade 组件驱动音量渐变，PlayBgm/StopBgm 的 fade_in/fade_out 参数已实装，支持跨层交叉淡入淡出
- [ ] Android 适配 (deferred to sub-phase)
- [ ] .asb 二进制解析器
- [ ] Lua 配置提取器
- [ ] 批量转换 pipeline
- [ ] PFS 重新打包

---

## 技术备忘

### Bevy 0.18 API 注意事项

| 旧 API | Bevy 0.18 API |
|--------|--------------|
| `Event` / `EventWriter` / `add_event` | `Message` / `MessageWriter` / `add_message` |
| `TouchInput` | `Touches`（`.any_just_pressed()`） |
| `get_single_mut()` | `single_mut()` |
| `Style` | 直接使用 `Node` 字段 |
| `Text` + `TextFont` + `TextColor` | 分离为三个组件（不再是单一 `TextBundle`） |

### 资源文件统计

| 封包 | 版本 | 文件数 | 主要类型 |
|------|------|--------|---------|
| `root.pfs` | v8 | 38,807 | .ogg (语音/BGM/SE), .asb (脚本), .lua, .ogv |
| `root.pfs.000` | v6 | 5,647 | .png (CG/立绘/背景/UI), .jpg (部分背景), .otf |
