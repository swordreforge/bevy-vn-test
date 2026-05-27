# 路径与旁白图片系统分析文档

## 一、资产路径分析

### 1.1 双重目录结构

项目存在两套资产目录：

| 目录 | 用途 | 内容 |
|------|------|------|
| `root/image/` | 原始 Artemis 引擎导出（完整） | 全部图片资产 |
| `assets/images/` | Bevy 运行时加载（子集） | 复制出的工作集 |

`assets/images/` 中所有文件均为 `.png` 或 `.jpg`，**无音频**。

### 1.2 `root/image/` 完整结构

```
root/image/
├── bg/              *.jpg 背景图
├── ev/              *.png 事件/CG
│   └── mono/
├── face/            *.png 面部立绘
├── fg/{char_dir}/   *.png 角色立绘tati_*.png
├── obj/
│   ├── blur/        *.jpg 模糊特效层
│   ├── bust/        *.jpg 半身像
│   ├── dic/         *.png/*.jpg 旁白介绍图（tx/非tx/msk/bg变体）
│   ├── end/         *.png/*.jpg 结局画面
│   ├── next/        *.jpg 下一章预告
│   ├── rain/        *.png 雨特效层
│   └── bgmname/     *.png BGM曲名标签
├── anime/           *.png 动画序列帧
├── font/            *.otf/*.ttf 字体
├── image/           UI/系统图
│   ├── extra/
│   ├── main/
│   │   ├── glyph/
│   │   ├── mw/
│   │   └── select/
│   └── ui/
├── rule/
├── thumbnail/
├── view/
└── warning.png
```

**注意 `root/image/obj/` 根目录没有文件，只有子目录** — 所有 obj 文件都在子目录中。

### 1.3 `assets/images/` 运行时子集

```
assets/images/
├── bg/              *.jpg ← OK
├── ev/              *.png ← OK
│   └── mono/
├── fg/{char_dir}/   *.png ← OK
├── face/            *.png ← OK
├── obj/
│   └── dic/         *.png ← **唯一存在的 obj 子目录**
├── title/           *.png/*.jpg
└── logo00.png
```

### 1.4 核心问题：`obj/` 子目录缺失 + 文件路径不匹配

#### 问题 A：`assets/images/obj/` 只有 `dic/`

`blur/`、`bust/`、`end/`、`next/`、`rain/`、`bgmname/` 全部缺失。脚本中 DrawSprite 引用这些目录下的文件都会失败。

#### 问题 B：mapper 输出的 RON 只有基础文件名，无子目录前缀

你提供的示例 RON：
```ron
DrawSprite(
    id: "01",
    file: "aiy00010_01",  // ← 只有基础名，无 dic/ 前缀，无扩展名
    ...
)
```

实际文件位置：
```
root/image/obj/dic/aiy00010_01.png   ← 存在
```

当前 runtime 代码构造路径：
```rust
// handle_draw_sprite 第 603 行
let full_path = format!("images/obj/{}", path);
// → "images/obj/aiy00010_01.png"
// → 期望文件: assets/images/obj/aiy00010_01.png
// → 但实际文件是: assets/images/obj/dic/aiy00010_01.png
// → "dic/" 缺失，加载失败
```

**运行时代码里的路径构造不包含子目录判断**，不会尝试搜索 `dic/`、`blur/`、
`rain/` 等子目录，也不会尝试 `.jpg` 作为备选扩展名。

### 1.5 关于 `/dlc` 目录

没有 `dlc/` 目录存在于项目任何位置。Artemis 引擎可能会将部分资产视为
可下载内容（DLC）存储在单独的目录中，但在导出结果中未见。

---

## 二、旁白介绍（dic）图片系统分析

### 2.1 文件命名模式

`root/image/obj/dic/` 中的文件命名模式多样：

| 模式 | 示例 | 说明 |
|------|------|------|
| `aiy{script}_{index:02}.png` | `aiy00010_01.png` | 最常见，index = 序号 |
| `aiy{script}_tx{index:02}.png` | `aiy00010_tx03.png` | 带 `tx` 前缀 |
| `aiy{script}_bg{index:02}.png` | `aiy00010_bg01.png` | 背景变体 |
| `aiy{script}_msk{index:02}.png` | `aiy00030_msk01.png` | 蒙版 |
| `MASK{index:03}.png` | `MASK011.png` | 全局蒙版 |
| `aiy50300_09a.jpg` | `aiy50300_09a.jpg` | 带字母后缀，**jpg** |

**结论：不存在统一的命名规则。**

### 2.2 当前 NarrationOverlay 的设计问题

当前 `handle_narration_overlay` 使用硬编码命名模式：
```rust
let file = format!("images/obj/dic/aiy{}_tx{:02}.png", script_num, engine.dialogue_idx);
```

只能匹配 `_txXX.png` 这一种变体。脚本 `aiy00010` 在 `dic/` 中有 27 个文件，
命名包括 `_01.png`、`_tx01.png`、`_bg01.png`、`_bg00.png`、`_txmsk.png` 等，
但 `handle_narration_overlay` 只尝试 `_tx{idx:02}.png`，导致不命中。

### 2.3 本质：旁白图片应该走 DrawSprite

**旁白图片本质就是 obj/sprite 的一种。** 原始 ASB 脚本中，旁白图片通过
`DrawSprite` 命令在同一套主流程中执行。当前的 `handle_narration_overlay`
是帧级轮询的旁路系统，带来了额外的复杂性：

1. 每帧 Update 轮询 DialogueState
2. dialogue_idx 在 skip 模式下飞越导致索引不同步
3. 硬编码命名规则无法匹配真实文件
4. 与 DrawSprite 独立运作互不知道对方
5. 自定义 entity spawn/despawn 而非复用 sprite 管理系统
6. 不支持任何效果（淡入、移动、旋转）

正确的方案是**删除 NarrationOverlay**，让脚本直接使用 DrawSprite 来展示
旁白图片，配合后续的 Dialogue 命令控制文字。

---

## 三、路径解析方案

### 3.1 runtime fallback 搜索（推荐短期方案）

在 `handle_draw_sprite` 中增加子目录搜索 + 扩展名搜索：

```rust
fn resolve_obj_path(file: &str, asset_server: &AssetServer) -> Option<Handle<Image>> {
    let base_name = if file.contains('.') {
        file.to_string()
    } else {
        format!("{}.png", file)
    };
    let base_name_jpg = if file.contains('.') {
        None
    } else {
        Some(format!("{}.jpg", file))
    };

    // 搜索顺序：所有 obj 子目录
    let subdirs = ["", "dic/", "blur/", "rain/", "bust/", "end/", "next/", "bgmname/"];
    let exts = [&base_name];
    let exts_with_jpg = [&base_name, &base_name_jpg];

    for subdir in &subdirs {
        for ext in (if subdir == &"blur/" || subdir == &"bust/" || subdir == &"next/" { &exts_with_jpg } else { &exts }) {
            let Some(ext) = ext else { continue; };
            let path = format!("images/obj/{}{}", subdir, ext);
            // 在 asset_server 中检查是否有对应文件
            // 或维护一个预扫描的文件映射
        }
    }
    None
}
```

**限制**：Bevy 的 `Assets<Image>` 不会 report missing assets 的路径，
无法区分"文件不存在"和"尚未加载"。需要预扫描映射表。

### 3.2 映射表持久化方案（推荐方案）

#### 原理

在导出阶段扫描 `root/image/obj/` 所有子目录，将 基础文件名(不含扩展名) → 完整相对路径
的映射序列化为 `assets/obj_index.ron`。运行时直接从文件反序列化，
零 I/O 开销、零搜索延迟。

#### 导出阶段生成（tools/artemis-export）

在 mapper 或导出主流程中添加扫描步骤：

```rust
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

fn build_obj_index(root: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let obj_dir = root.join("image/obj");
    for entry in WalkDir::new(&obj_dir) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() { continue; }
        let rel = entry.path().strip_prefix(root).unwrap()
            .to_str().unwrap().to_string();
        // key = 无扩展名的文件名, 如 "aiy00010_01"
        let stem = entry.path().file_stem().unwrap()
            .to_str().unwrap().to_string();
        map.insert(stem, rel);
    }
    map
}
```

输出文件 `assets/obj_index.ron`：

```ron
{
    "aiy00010_01": "image/obj/dic/aiy00010_01.png",
    "aiy00010_tx01": "image/obj/dic/aiy00010_tx01.png",
    "aiy10190_03_01_05": "image/obj/blur/aiy10190_03_01_05.jpg",
    "rain200_03": "image/obj/rain/rain200_03.png",
    "bus_11c": "image/obj/bust/bus_11c.jpg",
    "aiy00_msk01": "image/obj/end/aiy00_msk01.png",
    "nextsc_03": "image/obj/next/nextsc_03.jpg",
    "musname_403": "image/obj/bgmname/musname_403.png",
    // ... 覆盖所有 obj/ 子目录
}
```

#### 运行时加载

```rust
#[derive(Resource, Default, Debug)]
pub struct ObjFileIndex(HashMap<String, String>);

fn load_obj_index(world: &mut World) {
    let path = Path::new("assets/obj_index.ron");
    if !path.exists() { return; }
    let content = std::fs::read_to_string(path).unwrap();
    let map: HashMap<String, String> = ron::from_str(&content).unwrap();
    world.insert_resource(ObjFileIndex(map));
}
```

调用时机：在 `main.rs` 启动时，asset 初始化之前：
```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, load_obj_index)
        // ...
}
```

或在导出流程中直接嵌入 `IntoSystemConfigs`。

#### runtime 查表

```rust
fn handle_draw_sprite(
    // ... existing params ...
    index: Res<ObjFileIndex>,
) {
    for msg in msg.read() {
        let stem = msg.file.trim_end_matches(".png")
            .trim_end_matches(".jpg");
        let full_path = index.0.get(stem)
            .cloned()
            .unwrap_or_else(|| format!("images/obj/{}.png", msg.file));
        let handle = asset_server.load(&full_path);
        // ... rest unchanged
    }
}
```

#### 优点

1. **零运行时搜索** — O(1) HashMap 查找
2. **不依赖文件系统** — 无 `Path::exists()` 探测
3. **覆盖全部子目录** — dic/blur/rain/bust/end/next/bgmname 全部建立索引
4. **扩展名自动匹配** — jpg/png 由索引决定，不再硬编码
5. **可追加** — 后续 DLC/新资产只需重新生成 `obj_index.ron`
6. **与 Bevy AssetServer 兼容** — `asset_server.load()` 直接加载完整路径

#### 缺点

- 导出后无法动态发现新增文件（需要重新生成索引文件）
- 需要 `walkdir` 依赖（导出阶段）和 `ron` 序列化支持

### 3.3 mapper 嵌入全路径（备选方案）

修改 mapper 在导出 RON 时，扫描 `root/image/obj/` 构建映射表，直接输出
完整路径到脚本 `.bscript.ron` 中：

```ron
DrawSprite(
    id: "01",
    file: "image/obj/dic/aiy00010_01.png",  // ← 直接包含子目录和扩展名
    ...
)
```

这样运行时无需索引文件，但**需要在导出脚本 RON 时能访问图片目录**。
如果导出和图片扫描可以分离（比如先用 mapper 生成 .bscript.ron，
再单独运行索引工具生成 obj_index.ron），则方案 3.2 更灵活。

---

## 四、`root/` 根目录完整结构

```
root/
├── bgm/            BGM (ogg)
├── image/          图片（含 obj/、bg/、fg/ 等）
├── movie/          视频
├── scenario/       脚本 (.asb)
├── se/             音效
├── system/         配置 (.lua)
│   └── system.ini
└── voice/          语音
```

无 `dlc/` 目录。
