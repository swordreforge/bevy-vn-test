# BGM Gallery 设计文档

**重要**: BGM 画廊不创建独立 `AppState`/标题按钮。而是在现有 Gallery (`AppState::Gallery`) 内添加一个 CG ↔ BGM 模式切换滑块，共用同一套 UI 框架（返回按钮、翻页导航、背景），切换时只替换网格内容。

## 1. 数据来源

### 1.1 BGM 文件

**路径**: `assets/audio/bgm/bgm_{ID}_[ab].ogg`

**62 个 BGM ID**，按系列分布：

| 系列 | IDs | 数量 | 有 musname |
|------|-----|------|-----------|
| 短ID | `01`, `02` | 2 | ❌ |
| 01xx | `0101` ~ `0105` | 5 | ❌ |
| 02xx | `0201` ~ `0206`, `0208` | 7 | ✅ 全部 |
| 03xx | `0301` ~ `0311` | 11 | ✅ 全部 |
| 04xx | `0401` ~ `0405`, `0407` ~ `0421` | 20 | ✅ 全部 |
| 05xx | `0501` ~ `0507` | 7 | ✅ 全部 |
| 09xx | `0901` ~ `0910` | 10 | ❌ 全部 |

> 注意: 大部分 BGM 有 `_a.ogg` + `_b.ogg` 两层（seamless loop），播放时由 `audio.rs` 的 `concat_ogg_bytes` 拼接。

**音频路径规则**（已有，`audio.rs:96-102`）：
```
audio/bgm/bgm_{id}_a.ogg
audio/bgm/bgm_{id}_b.ogg
```

### 1.2 标题图片

**路径**: `assets/image/obj/bgmname/musname_{3位ID}.png`

共 **45 个** PNG 文件，覆盖 02xx / 03xx / 04xx / 05xx 系列。映射规则：去掉 BGM ID 的前导零后作为文件名：

| BGM ID | musname 文件 |
|--------|-------------|
| `0201` | `musname_201.png` |
| `0403` | `musname_403.png` |
| `0507` | `musname_507.png` |

**缺少的 17 个 BGM**（01, 01xx, 02, 09xx）需要 fallback 方案。

## 2. 需要新增/修改的代码

### 2.1 新增文件

- `docs/bgm-gallery-design.md` — 本文档

### 2.2 现有文件修改

| 文件 | 修改内容 |
|------|---------|
| `src/plugins/gallery.rs` | Gallery 内加模式切换开关 (CG ↔ BGM)，BGM 视图的网格/分页/播放逻辑全部内聚在此 |
| `src/resources.rs` | `GalleryState` 加 `GalleryMode` 枚举；`UnlockState.bgm_unlocked` 不再 `allow(dead_code)` |
| `src/components.rs` | 新增 `GalleryModeToggle`、`BgmCard`、`BgmPlaying`、`GalleryCgGrid`、`GalleryBgmGrid` 等组件 |
| `src/script.rs` | 新增 `ScriptCmd::BgmUnlock { id }`（可选，后期） |
| `src/plugins/script_runner.rs` | 处理 `BgmUnlock`（可选，后期） |
| `build.rs` | 新增 `all_bgm_ids()` 函数，扫描 `audio/bgm/` 生成唯一 ID 列表 |
| `unlock_state.json`（运行时） | `bgm_unlocked` 字段会自动被序列化 |

## 3. UI 层级设计

```
AppState::Gallery (保持不变，内部通过 mode 切换两套视图)

ZIndex 5: GalleryRoot (全屏半透明背景, srgba(0.05, 0.05, 0.1, 0.95))
│
├── ZIndex 5a: ← Back 按钮 (左上, 80x36)
├── ZIndex 5b: 标题区域
│   ├── "CG Gallery" / "BGM Gallery" (根据 mode 切换)
│   └── [CG] ──●── [BGM]  模式切换滑块
├── ZIndex 5c: 翻页导航 (◀ Page X / Y ▶)
│
├── ZIndex 5d: [CG mode]  CG 网格 (现有逻辑，GalleryGridContent)
│
└── ZIndex 5d: [BGM mode] BGM 卡片网格 (FlexWrap Row)
    ├── 卡片 1 (300x80, Button)
    │   ├── [musname.png / 标题文字 / "[LOCKED]"]
    │   └── [▶ / ⏹ 状态图标]
    ├── 卡片 2
    └── ...
```

## 4. 卡片状态机

```
          ┌──────────────────────────────────────┐
          │                                      │
          ▼                                      │
   ┌──────────┐   点击     ┌──────────┐   点击    │
   │  LOCKED  │ ──────►   │ PLAYING  │ ──────►   │
   │ [LOCKED] │  (不应出现) │ ▶ +标题  │  STOP     │
   └──────────┘           └──────────┘           │
        ▲                      │                 │
        │                      │ 播放结束        │
        │                      ▼                 │
        │              ┌──────────────┐          │
        │              │   UNLOCKED   │ ─────────┘
        │              │ ✓ + 标题 (可再播)
        └──────────────┘
```

实际流程:
1. **锁定** (不在 `bgm_unlocked` 中): 显示 "[LOCKED]"，无法点击
2. **未播放但已解锁** (在 `bgm_unlocked` 中): 显示标题 + ▶，点击播放
3. **正在播放**: 显示标题 + ⏹ 或 ▶ 切换，再次点击停止
4. **已播放完毕**: 状态同"未播放但已解锁"，可再次点击

## 5. 播放控制方案

**简单方案（推荐初期实现）**:
- 点击 BGM 卡片 → 发送 `PlayBgmMessage { id }` 播放
- 再次点击同一卡片 → 发送 `StopBgmMessage { id: Some(id) }` 停止
- 点击不同 BGM → 自动停止前一个，播放新的
- 离开 Gallery → `StopBgmMessage { id: None }` 停止所有

**后续可扩展**:
- 暂停/恢复（需要 `audio.rs` 支持 `AudioSink` 控制）
- 进度条（需要 AudioSource 时长信息）
- 音量滑块（复用已有 `bgm_volume` 设置）

## 6. 分页设计

参考 CG gallery 的 `CGS_PER_PAGE = 9`，BGM 卡片较小，建议:

- **每页 12 个**（4 列 × 3 行）
- 62 个 BGM → 6 页（5 × 12 + 1 × 2）
- 翻页方式: ◀ / ▶ 按钮 + 键盘 ← →

**卡片布局**:
```
┌─────────────────────────────────────────────┐
│  [Card]  [Card]  [Card]  [Card]             │
│  [Card]  [Card]  [Card]  [Card]             │
│  [Card]  [Card]  [Card]  [Card]             │
│                       ◀ Page 3 / 6 ▶        │
└─────────────────────────────────────────────┘
```

卡片尺寸: ~280x70，每个卡片内部:
- 左侧: musname 图片 (或 ID 文字 fallback)
- 右侧: 状态图标 (▶ / ⏹ / ✓ / 🔒)

## 7. musname fallback 策略

对于缺少 `musname_{XXX}.png` 的 17 个 BGM:

| 策略 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| **A. 显示 BGM ID** | 直接显示文本 `"BGM 0101"` | 简单 | 不美观 |
| **B. 硬编码标题** | Rust 里写 `HashMap<&str,&str>` 映射 | 精确 | 需要手动录入 |
| **C. ron 配置文件** | `bgm_index.ron` 文件管理所有元数据 | 灵活可编辑 | 多一个文件 |
| **D. 跳过显示** | 有 musname 的才显示，无的不显示 | 避免不完美 | 缺失 17 个 BGM |

**推荐**: 方案 C（ron 配置）作为长期，方案 A（ID fallback）作为初期快速实现。

示例 `bgm_index.ron`:
```ron
(
    // 有 musname 图片的可以只写 id + 排序
    // 无 musname 的需要加 title
    bgms: [
        (id: "01",    title: Some("Aiyoku no Eustia"), sort_key: 1),
        (id: "0101",  title: Some("Prologue"),         sort_key: 2),
        // ... 有 musname 的可以不写 title
        (id: "0201",  title: None,                      sort_key: 10),
        // ...
    ]
)
```

## 8. 初期实现步骤（建议顺序）

```
Step 1: build.rs — 扫描 audio/bgm/，生成 all_bgm_ids()
        (新增函数，返回 Vec<&'static str>)

Step 2: resources.rs — GalleryState 增加 mode: GalleryMode + bgm_page 字段
        (GalleryMode 枚举: Cg, Bgm)

Step 3: components.rs — 新增组件:
        - GalleryModeToggle (模式切换容器)
        - GalleryCgGrid (CG 网格标识)
        - GalleryBgmGrid (BGM 网格标识)
        - BgmCard (BGM 卡片标记)
        - BgmPlaying (当前播放的 BGM 卡片标记)

Step 4: gallery.rs — 重构 setup_gallery 为多模式
        - 标题栏显示 "CG Gallery" / "BGM Gallery"
        - 模式切换滑块 (两个按钮或 slider 样式)
        - 根据 mode 渲染 CG 网格或 BGM 网格
        - 分页导航共用同一套 UI 但数据独立 (cg_page / bgm_page)

Step 5: gallery.rs — BGM 卡片渲染
        - 读取 bgm_index.ron (通过 include_str!)
        - 分页 (每页 12 个)
        - 解锁状态: 已解锁 → 显示标题 + ▶; 锁定 → "[LOCKED]"
        - musname 图片: asset_server.load 后显示; fallback to 标题文字

Step 6: gallery.rs — 播放控制
        - 点击已解锁卡片 → PlayBgmMessage { id }
        - 再次点击同一卡片 → StopBgmMessage { id: Some(id) }
        - 点击不同卡片 → 自动停止前一个, 播放新的
        - 离开 Gallery → StopBgmMessage { id: None }

Step 7: gallery.rs — 切换模式逻辑
        - CG ↔ BGM 时清除网格
        - 停止 BGM 播放
        - 切换到对应 mode 的分页数据
        - 不影响 safe_mode / debug_unlock 等

Step 8: (可选) 脚本解锁 — ScriptCmd::BgmUnlock
```

## 9. 相关问题

### 9.1 BGM 排序

建议按 ID 数值排序（即按系列 01/01xx/02/02xx/03xx/... 顺序），与 `all_bgm_ids()` 生成顺序一致。

### 9.2 Gallery 返回逻辑

不变（与现有 CG gallery 一致）: 如果有对话文本 → 回 `Menu`，否则回 `Title`。

### 9.3 与现有 BGM 系统的冲突

Gallery 内切换 BGM 会中断 title/gameplay BGM。离开 Gallery 时停止所有 BGM（`StopBgmMessage { id: None }`），让目标 state 的 OnEnter 重新播放自己的 BGM。

### 9.4 模式切换行为

- 切换 CG ↔ BGM 时清除当前网格内容，重绘对应视图
- 切换模式时停止正在播放的 BGM
- `GalleryState.page` 分别存储 `cg_page` 和 `bgm_page`（避免切换后丢失页码）
- 分页数量不同: CG 每页 9 个，BGM 每页 12 个
