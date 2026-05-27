# P2-6 事件/标记系统 — 实现进度

> 对应 Artemis 引擎的 View, Event, EventMN, EventCut, SetGlobalFlag, RouteFlag, GameMode, NextDay 标签
> 实现日期：2026-05-27

## 文件变更

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/script.rs` | 修改 | 新增 ScriptCmd 变体：View, SetGlobalFlag, RouteFlag, GameMode |
| `src/script.rs` | 修改 | ScriptEngine 新增 global_flags: HashMap<u32, i32> |
| `src/resources.rs` | 修改 | SaveData 新增 global_flags; Settings 新增 click_to_advance; 新增 ViewBlocking |
| `src/plugins/mod.rs` | 修改 | 注册 event_system 模块 |
| `src/plugins/script_runner.rs` | 修改 | 处理 View/SetGlobalFlag/RouteFlag/GameMode；正常+跳过模式 |
| `src/plugins/save_load.rs` | 修改 | 存档/读档包含 global_flags |
| `src/plugins/event_system/mod.rs` | 新建 | EventSystemPlugin |
| `src/plugins/event_system/view.rs` | 新建 | View 9 阶段状态机（ViewState + advance_view） |
| `src/plugins/event_system/view_data.rs` | 新建 | view_table（12 条目）+ view_tweentable（4 种动画路径） |
| `src/main.rs` | 修改 | 注册 EventSystemPlugin |
| `tools/artemis-export/src/mapper.rs` | 修改 | 新增 View/ViewEnd/Event/EventMN/EventCut/DrawScene/SetGlobalFlag/RouteFlag/GameMode 映射 |
| `src/lib.rs` | 修改 | re-export Transition |

## 实现状态

### Phase 1 — 基础设施 ✅
- ScriptCmd 变体：View, SetGlobalFlag, RouteFlag, GameMode ✅
- ScriptEngine + SaveData global_flags 持久化 ✅
- Mapper 全部新标签映射 ✅
- script_runner 正常/跳过模式均处理 ✅

### Phase 2 — View 系统 ✅
- view_data.rs 静态数据表 ✅
- 9 阶段状态机（FadeOut → Done） ✅
- ViewBlocking 阻止脚本推进 ✅
- 跳过模式直接设窗口色 ✅
- v1：名卡 alpha 淡入近似（暂未使用 UiMaterial mask 转场）

### Phase 3 — DrawScene/Event ✅
- 全部在 mapper 中展开为 Window{false} + HideFg + ShowCg
- Event → `eve_` 前缀
- EventMN → `mon_` 前缀
- EventCut → `cut_` 前缀（预留）
- DrawScene 独立标签 → 同路径模式

### Phase 4 — NextDay ✅
- 通过已有 CallScript 机制 + macro.iet → macro.bscript.ron 路径
- m定时前无调用方，无需处理

### Phase 5 — 打磨 ✅
- RouteFlag: 英雄路线 [103,105,107,108,110,111] → 113; 全 clear [113,151..167] → 114
- 剩余：View mask 自定义 UiMaterial（方案 B，延期）

## 验收核对

| 子系统 | 关键文件 | 自检状态 |
|--------|---------|---------|
| View | `view.rs`, `view_data.rs`, `script_runner.rs` | ✅ 34 tests pass |
| DrawScene | `mapper.rs` View->ShowCg 路径 | ✅ 34 tests pass |
| GlobalFlags | `script.rs` `HashMap<u32,i32>`, `save_load.rs` 序列化 | ✅ |
| RouteFlag | `script_runner.rs` 聚合逻辑 | ✅ |
| GameMode | `script_runner.rs` → Settings.click_to_advance | ✅ |
| NextDay | macro.iet 定义 | ✅ 定义存在，无调用方 |
| Mapper | `mapper.rs` 全部映射 | ✅ 34 tests pass |
