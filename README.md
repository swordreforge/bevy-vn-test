# Aiyoku no Eustia — Bevy VN

> 用 [Bevy 0.18](https://bevyengine.org) 游戏引擎从零重写的视觉小说引擎，旨在替代原版 Artemis 引擎在 Android 设备上因虚拟机无限递归而崩溃的问题。

## 项目起源

**Aiyoku no Eustia**（日文：穢翼のユースティア）原由 August 社使用其 **Artemis Engine** 开发。该引擎在 Windows 上运行良好，但在 Android 移植版本中，Artemis 内置的 Lua 解释器存在严重的**栈溢出 bug**——游戏运行到特定剧本分支时会陷入**无限递归 panic**，导致应用直接闪退。

本项目的目标是：
1. 用 Rust + Bevy 从零实现一个可运行的 VN 引擎
2. 完全兼容原游戏的所有剧本、CG、立绘、BGM/SE 和视频资源
3. 解决 Android 端原版 Artemis VM 崩溃的核心问题
4. 作为 Bevy 0.18 游戏引擎的学习研究项目

游戏资源使用 [pfs-rs](https://github.com/sakarie9/pfs-rs) 从 `.pfs` 压缩包提取（感谢 [Sakari](https://sakari.top) 的逆向分析 [Parsing PFS Files Used by the Artemis Engine](https://sakari.top/posts/2025/artemis-pfs/)）。

## 架构概览

```
bevy-vn/
├── src/
│   ├── main.rs              # 桌面端入口
│   ├── lib.rs               # 共享 App 构建 (build_app)，Android 入口也调用它
│   ├── state.rs             # AppState 状态机 (Boot→Splash→Title→Gameplay→...)
│   ├── script.rs            # ScriptCmd 枚举 (78 种指令) + ScriptEngine 运行时
│   ├── resources.rs         # 全局资源：AffectionMap、SaveData、Settings、UnlockState...
│   ├── components.rs        # ECS Component 定义
│   ├── messages.rs          # Bevy 0.18 Message 类型 (替代旧版 Event)
│   ├── asset_pak.rs         # 自研 PAK 打包 + Zstd 解压 AssetReader
│   ├── build.rs             # 编译期代码生成：扫描脚本/CG/BGM/SE
│   ├── plugins/
│   │   ├── script_loader.rs # 解析 .bscript.ron → ScriptEngine
│   │   ├── script_runner.rs # 核心：执行 ScriptCmd，驱动整个游戏逻辑
│   │   ├── rendering.rs     # 双缓冲背景、立绘三槽位、CG、Sprite 覆盖层
│   │   ├── audio.rs         # 使用 rodio 的 BGM (AB段拼接)/SE/Voice 播放
│   │   ├── dialogue.rs      # 对话框 UI 与文字逐字显示
│   │   ├── choice.rs        # 选项分支 UI
│   │   ├── screen_transition.rs  # 场景切换黑幕过渡
│   │   ├── title.rs / splash.rs / menu.rs / settings.rs / gallery.rs / backlog.rs
│   │   ├── save_load.rs     # JSON 存档 + AutoSave
│   │   ├── affection.rs     # 好感度系统
│   │   ├── routing.rs / route_end.rs / after_story.rs  # 路线选择与通关
│   │   ├── event_system/    # 角色 View (立ち絵鑑賞) 子系统
│   │   ├── video/           # GStreamer (桌面) / FFmpeg (Android) 视频播放
│   │   └── inputs.rs        # 用户输入处理
│   └── ...
├── tools/
│   ├── artemis-export/      # ASB/IET → .bscript.ron 转换工具
│   └── asset_packer/        # 资源 PAK 打包工具
├── assets/
│   ├── scripts/*.bscript.ron     # 编译后的剧本 (gitignored, 需转换)
│   ├── scripts/obj_index.ron     # 立绘/背景资源索引
│   ├── image/{bg,fg,ev,obj,face,anime}/
│   ├── audio/{bgm,se,voice}/
│   ├── movie/               # Ogg Theora 视频
│   └── shaders/
└── routes.ron               # 路线配置 (共通/女主/extra/后日谈)
```

## 资源提取：从 PTF 到剧本

### 1. PFS 解包

原游戏资源封装在 `.pfs` 格式中（Artemis 引擎专有容器格式）。Sakari 的文章 [Parsing PFS Files Used by the Artemis Engine](https://sakari.top/posts/2025/artemis-pfs/) 提供了完整的逆向分析，对应的 Rust 解包工具为 [pfs-rs](https://github.com/sakarie9/pfs-rs)。

**PFS 文件结构**（以 `pf8` 变体为例）：

```
Offset  Size  Field
0x00    3     Magic: "pf8"（或 "pf6"）
0x03    4     Index Size（从 0x07 到 header 末尾的总大小）
0x07    4     File Count (N)
0x0B    var   File Entries 数组（每个条目: NameLen[4] + Name[nameLen] + Sep[4] + DataOffset[4] + FileSize[4]）
0x0B+S  4     File Size Count（通常 = N+1）
0x0F+S  8*N   File Size Offsets 数组（8字节指针，相对自身起始位置指向 FileEntry.size）
剩余          Padding（零填充）
末尾    4     Index End Offset（指向 File Size Count 字段，相对于 0x07）
```

**加密机制**：
- `pf8` 变体使用自定义 XOR 流加密
- 密钥 = SHA-1(IndexData)，其中 IndexData 是从 `0x07` 开始、长度为 Index Size 的字节序列
- 每个文件的文件数据独立加密，密钥流在每个文件起始位置重置：`C[i] = P[i] ^ key[i % 20]`
- Header 和索引结构保持明文

**解包后得到的关键目录**：
- `scenario/*.asb` — 二进制剧本文件（主剧情脚本）
- `scenario/*.iet` — 文本格式剧本（IET = Independent Event Table，用于独立的短事件）
- `image/{bg,fg,ev,obj,face,anime}/` — 各种图像资源
- `audio/{bgm,se,voice}/` — Ogg Vorbis 音频
- `movie/*.mpg` — OP 动画（需转为 Ogg Theora）
- `system/csv.lua` — Lua 配置文件，定义 CG 集、BGM 表、FG 路径映射等

### 2. ASB 二进制剧本结构

`.asb` 文件是 Artemis 引擎的二进制剧本格式。`tools/artemis-export/src/asb.rs` 实现了完整解析器。

**二进制布局：**

```
[0..4]   magic:    "ASB\0"
[4]      version:  u8 (must be 0)
[5..9]   count:    u32 LE (block + command items)

Items (顺序存储, type=1 是 Label, type=0 是 Command):
 type=1 (Label):
   [u32 LE: string length] [UTF-8 string] [\0]
 type=0 (Command):
   [u32 LE: tag string length] [UTF-8 tag] [\0]
   [u32 LE: line number]
   [u32 LE: attribute count N]
   重复 N 次:
     [u32 LE: key string length] [UTF-8 key] [\0]
     [u32 LE: value string length] [UTF-8 value] [\0]
```

**解析策略**（`parse_asb()`）：
- 采用游标式逐字节读取（`read_u32_le()`、`read_string()`）
- 遇到 `type=1` 创建新 Block（带 label 名），遇到 `type=0` 收集到当前 Block
- 每个 Command 的属性解析为 `HashMap<String, String>`，用位置索引 `"0"`、`"1"` 等作为 key

```rust
// asb.rs 核心数据结构
struct AsbScript { blocks: Vec<AsbBlock> }
struct AsbBlock { label: String, commands: Vec<AsbCommand> }
struct AsbCommand { tag: String, attrs: HashMap<String, String> }
```

### 3. ASB → ScriptCmd 映射

`tools/artemis-export/src/mapper.rs` 是核心翻译层，将 ASB 原生标签转换为引擎内部 `ScriptCmd` 枚举。

**标签映射处理流程**：

```
ASB 标签 (如 "Tati", "BgmPlay", "Fadeout", "DrawScene")
  │
  ├── map_command() — 直接映射 (约 70+ 标签)
  │     • 字符操作: Tati/TatiFa → ShowFg, Face → ShowFace, ClrTati → HideFg
  │     • 背景/事件: Back → SetBg, DrawScene/Event/EventMN → ShowCg
  │     • 音频: BgmPlay → PlayBgm, SEPlay → PlaySe, Voice → PlayVoice
  │     • 过渡: Fadeout/Blackout → ScreenOverlay, FadeFilm → ClearOverlay
  │     • 画面效果: Quake/Jishin → 屏幕震动, Flash → 闪光
  │     • 选择支: sel_init/sel_text/select/exswitch → Choice 系统
  │
  ├── map_calllua() — calllua 标签映射
  │     • 原 Artemis 用 Lua 函数调用实现的部分功能
  │     • 通过函数名模式匹配: set_bg/bgm_play/se_play/show_fg/hide_fg...
  │     • 若匹配失败则 fallthrough 到 NoOp
  │
  └── NoOp: Size/flip/lyprop/lydel/lyevent/trans... (旧引擎遗留标签，无需实现)
```

**选择支分支修正**（`fix_choice_branches()`）：
- 原 ASB 中每个选择分支结束后用 `Halt`（终止脚本），但游戏需要继续
- 检测连续的选择分支 label 列表，将非第一个分支后的 `Halt` 替换为 `Jump(convergence)`——跳转到所有分支汇合后的首个非分支 label

### 4. IET 文本剧本解析

`.iet` 文件是纯文本格式，`tools/artemis-export/src/iet.rs` 实现了解析。格式如：

```
*label_name
[CallScript aiy00010]
[if estimate="$t.tmp == 0"]
[CallScript aiy10230]
[else]
[CallScript aiy10200]
[/if]
[return]
```

**if/else 编译**：将 Lua 风格的 `if estimate="..."` 编译为 `Condition{op, val, goto else_label}` + `Jump endif_label` + 标签对。条件表达式中的 `$t.tmp` 被解析为 `tmp` 变量，通过取反魔术（如 `==` → `NotEqual`）实现 if-true 时跳 else 的语义。

## 运行时架构

### 资源调度与帧数控制

**Frame Pacing**（`src/lib.rs`）：

```rust
app.add_plugins(FramepacePlugin)
   .insert_resource(FramepaceSettings {
       limiter: Limiter::from_framerate(60.0),  // 锁 60 FPS
   })
   .insert_resource(WinitSettings {
       focused_mode: UpdateMode::reactive(Duration::from_secs_f64(1.0 / 60.0)),
       // reactive 模式：只在有窗口事件或 Res 变化时唤醒，省电
       ..default()
   });
```

- `bevy_framepace` 提供精确帧率限制，避免移动设备过热
- `UpdateMode::reactive()` 让游戏在无输入时休眠（类似 `winit` 的 `Poll` vs `Wait` 模式）
- 文字逐字显示使用 `time.delta_secs_f64() * chars_per_sec` 而非固定 tick，保证不同帧率下速度一致

**资源加载与 PAK 系统**（`src/asset_pak.rs`）：

- **编译期扫描**：`build.rs` 在编译时扫描 `assets/scripts/`、`assets/image/ev/`、`assets/audio/bgm/`、`assets/audio/se/`，生成 `game_data.rs`，包含所有脚本名、CG 文件名列表、BGM ID 列表
- **PAK 打包**：`tools/asset_packer` 将资源打包为 `.pak` 文件（BPAK 格式 + Zstd 压缩），减少文件系统 IO
- **AssetReader 链**：`create_asset_reader()` 按优先级尝试：
  1. 多 Bundle PAK（`assets_pak/data.pak`, `bgm.pak`...）
  2. 单体 `assets.pak`
  3. 回退到文件系统 `assets/`
- **Android 特殊处理**：从 APK 的 `assets/` 目录直接 mmap PAK 文件，无需解压到外部存储

### 实体生命周期与定时销毁

Bevy ECS 模式下，实体不会自动销毁，**必须显式管理生命周期**。本项目的经验：

**1. 场景切换时的批量清理**（`rendering.rs` -> `cleanup_rendering()`）：

```rust
fn cleanup_rendering(mut commands: Commands, query: Query<Entity, Or<(
    With<BackgroundRoot>, With<SpriteSlotMarker>,
    With<CgRoot>, With<SpriteOverlay>, With<ScreenOverlayRoot>
)>>, ...) {
    for entity in &query { commands.entity(entity).despawn(); }
    // 同时重置所有状态资源为 Default
}
```

在进入 Title 状态时触发，确保回到主菜单时所有渲染实体被销毁。

**2. 剧本跳转/调用时的现场清理**（`script_runner.rs` -> `clear_scene_sprites()`）：

```rust
fn clear_scene_sprites(overlay_mgr, commands, hide_fg_writer, hide_cg_writer, overlay_query) {
    // 1. 销毁所有 SpriteOverlay (DrawSprite 创建的实体)
    for (_, entity) in overlay_mgr.sprites.drain() { commands.entity(entity).despawn(); }
    // 2. 隐藏所有立绘
    hide_fg_writer.write(HideFgMessage { char_id: "all".into(), ... });
    // 3. 隐藏 CG
    hide_cg_writer.write(HideCgMessage { ... });
    // 4. 重置屏幕覆盖层
    for (entity, ..) in overlay_query.iter_mut() { vis = Hidden; /* 移除 Tween */ }
}
```

在 `Jump`、`Call`、`CallScript`、`Condition`、`Return` 时都会调用此函数，防止场景残留。

**3. Sprite 渐出后的自动销毁**（`rendering.rs` -> `update_sprite_tweens()`）：

```rust
if tween.timer.just_finished() {
    match tween.kind {
        TweenKind::FadeOut => {
            overlay_mgr.sprites.remove(&overlay.id);
            commands.entity(entity).despawn();  // 渐出完成后自动回收
        }
        _ => { /* 保留实体，但移除 Tween 组件 */ }
    }
}
```

**4. 音频实体的生命周期**：
- 语音（Voice）：每次播放前 `despawn` 旧实体，`PlaybackSettings::DESPAWN` 自动回收
- BGM：暂存 `Entity` 在 `BgmManager`，stop 时 `despawn`
- SE OneShot：`PlaybackSettings::DESPAWN`，播放完毕后自动销毁
- SE Loop：存于 `SeManager.entities[channel]`，切换时 `despawn` 旧通道

**5. 视频播放**（`plugins/video/`）：
- 使用 GStreamer 管道（桌面）或 FFmpeg（Android）
- 每帧将视频帧解码为 RGBA 像素，更新 `Image` 句柄
- 视频结束时触发 `PendingVideo` 完成检测，销毁实体并恢复脚本执行

### 核心经验教训

#### 1. 无限递归 panic 的根源与解决

原 Artemis Android 崩溃的本质：
- Artemis 脚本使用 Lua 作为中间层，每个 `calllua` 标签触发 Lua VM 调用
- 某些剧本分支（如多嵌套 `if/else` + 选择支）导致 Lua 调用栈链过长
- Android 上默认栈空间较小（通常 1MB），超出后 SIGSEGV

本项目用 Rust 直接执行剧本指令，**没有递归 VM 调用**：
- `ScriptCmd` 是扁平的枚举，`ScriptEngine` 用迭代器 + `call_stack: Vec<(String, usize)>` 模拟调用
- `call_label()` / `return_from_call()` 只是压栈/弹栈当前位置指针
- 不存在栈溢出路径

#### 2. Bevy 0.18 API 迁移陷阱

| 旧概念 (pre-0.18) | Bevy 0.18 |
|---|---|
| `Event` / `EventWriter` | `Message` / `MessageWriter` (用 `app.add_message::<T>()`) |
| `get_single_mut()` | `single_mut()` |
| `Style` 结构体 | 直接 `Node` 字段 (`width`, `height`, `position_type`...) |
| `TextBundle` | 拆分 `Text` + `TextFont` + `TextColor` 组件 |
| `touch_input` 资源 | `Touches` 资源 (`.any_just_pressed()`) |

#### 3. 双缓冲背景系统

采用双缓冲（`[Entity; 2]` + `active_idx`）实现背景交叉淡入淡出：

```
active_idx = 0 → 当前显示 background[0]
设置新背景 → 写入 background[1]，启用 Fade Tween
Fade 完成后 → active_idx = 1，隐藏 background[0]
```

无需创建/销毁实体，避免实体 ID 暴涨。

#### 4. 立绘三槽位池化

立绘复用 3 个预置 Slot（Left/Center/Right），每个 Slot 位固定一个 Entity：
- `ShowFg` → 替换 Slot 的纹理
- `HideFg` → 清空 Slot
- 避免每次对话创建新实体，减少 ECS 开销

#### 5. PAK 文件格式与 Zstd 压缩

自定义 BPAK 格式：

```
[Data Section: zstd 压缩的数据块]
[Index Section: 文件名→offset/compressed_size/uncompressed_size]
[Footer (20 bytes): Magic "BPAK" + Version + IndexOffset + EntryCount]
```

- 使用 `memmap2` 零拷贝读取 footer 和 index
- `zstd` 按需解压单个文件，不浪费内存
- Android 端从 APK 直接 `AssetManager::open()` 读取 PAK，无需解压到外部存储

#### 6. BGM A/B 段拼接

Artemis 引擎的 BGM 分 `_a.ogg`（前奏）和 `_b.ogg`（循环主段）。运行时：
- 使用 `rodio` 解码为 PCM 数据
- 将 A/B 段 PCM 拼接为 WAV
- 注册为 `AudioSource` 资源播放
- 整体作为 Loop 播放，模拟原引擎的 A→B→(B...) 行为

#### 7. 编译期代码生成

`build.rs` 实现了编译期资源注册，生成 `game_data.rs`：
- 所有剧本的 `include_str!` + 文件名列表
- CG 文件路径函数（自动检测 `.png`/`.jpg`/`.jpeg` 扩展名）
- BGM ID 列表（扫描 `bgm_*_a.ogg` 模式）
- SE 拆分检测（`_a.ogg` 文件列表）

好处：修改资源文件后只需 `cargo build` 触发重新扫描，无需手动配置。

## 构建与运行

```bash
# 桌面构建运行
cargo run

# 开发检查（快速）
cargo check

# Release 构建（LTO thin + strip）
cargo build --release
```

### Android 交叉编译

**Cargo.toml** 中的关键依赖：

```toml
[target.'cfg(target_os = "android")'.dependencies]
ffmpeg-the-third = { version = "5", features = ["build"] }
```

`ffmpeg-the-third` 在 Android 上**从源码编译 FFmpeg**（首次构建极慢），其 build.rs 会调用 `./configure` + `make`，需要 NDK 提供整套交叉编译工具链。

**NDK 工具链兼容性问题**：

现代 NDK（r27+）已移除传统 `aarch64-linux-android-ar`、`aarch64-linux-android-nm` 等独立工具，统一使用 `llvm-ar`、`llvm-nm`。但 `ffmpeg-sys-the-third` 的 configure 脚本硬编码了目标前缀工具名，`android/build.sh` 的解决方式：

```bash
# Step 0: 为缺失的工具创建符号链接包装器
WRAPPER_DIR="/tmp/android-toolchain"
for tool in ar nm strings objdump dlltool; do
    wrapper="$WRAPPER_DIR/aarch64-linux-android21-$tool"
    if [ ! -f "$wrapper" ]; then
        ln -sf "$NDK_BIN/llvm-$tool" "$wrapper"  # llvm-ar → aarch64-linux-android21-ar
    fi
done
# ranlib 特殊处理
cat > "$WRAPPER_DIR/aarch64-linux-android21-ranlib" << 'EOF'
#!/bin/bash
exec llvm-ar -s "$@"
EOF
chmod +x "$WRAPPER_DIR/aarch64-linux-android21-ranlib"
PATH="$WRAPPER_DIR:$NDK_BIN:$PATH"
```

并将这些包装器目录和 NDK bin 目录优先加入 `$PATH`。

**构建流程**（`android/build.sh`）：

1. **配置环境变量**：`CC_aarch64-linux-android`、`AR_aarch64-linux-android`、`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER` 指向 NDK 的 clang/llvm-ar
2. **交叉编译 Rust**：`cargo build --target aarch64-linux-android --features android`
3. **拷贝 `.so`**：将 `libbevy_vn.so` 和 `libc++_shared.so` 到 `jniLibs/arm64-v8a/`
4. **Strip**：`llvm-strip` 减小体积
5. **PAK 打包**：运行 `asset-packer` 工具将游戏资源打包为 Zstd 压缩的 PAK 包，拷贝到 `app/src/main/assets/assets_pak/`
6. **Gradle 构建 APK**：`gradle assembleRelease`

> **注意**：FFmpeg 从源码编译需要较长时间（取决于机器，通常数分钟）。PAK 包支持缓存（`zstd_tmp/`），第二次构建可跳过资源打包步骤。

```bash
# 一键 Android 构建
cd android && bash build.sh release

# 或 debug 构建
bash build.sh debug
```

### 桌面前置依赖

- **Rust 1.85+**
- **GStreamer 开发包**（用于视频播放，桌面端使用 `gstreamer` crate）
- **资产**：使用 [pfs-rs](https://github.com/sakarie9/pfs-rs) 从游戏 `.pfs` 包提取资源到 `assets/` 目录，并运行 `tools/artemis-export` 转换剧本

## 许可

本项目仅用于学习研究。
