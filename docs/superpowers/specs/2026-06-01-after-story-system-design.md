# After Story System Design

## Overview

Add per-route after-stories to the route selection system. Each heroine route's after-story
unlocks when that route is completed. A separate "extra after-story" section unlocks when
all routes are cleared. After-stories are accessed from a dedicated menu entry (not from
the route selection grid).

## Data Model

### New Types

```rust
pub struct AfterStoryEntry {
    pub script: String,   // "aiy70110"
    pub name: String,     // "后日谈"
    pub order: u32,       // within-group sort order
}
```

### RouteEntry Changes

```rust
pub struct RouteEntry {
    // existing fields unchanged
    pub after_stories: Vec<AfterStoryEntry>,  // NEW
}
```

### RouteConfig Changes

```rust
pub struct RouteConfig {
    pub common: RouteEntry,
    pub heroines: Vec<RouteEntry>,
    pub extra: Option<RouteEntry>,
    pub extra_after_stories: Vec<AfterStoryEntry>,  // NEW — each has own unlock_flag
    pub route_unlock_flags: Vec<u32>,
    pub all_routes_cleared_flag: u32,
    pub full_completion_flag: u32,
    pub ending_flag_range: (u32, u32),
    pub ending_count: u32,
}
```

### routes.ron layout

```ron
common: (index: 0, name: "序章", script: "aiy00010", always_unlocked: true),
heroines: [
    (index: 1, name: "Fione", script: "aiy10010", unlock_flag: 51,
     hero_work: Some(1), ending_flags: [151, 152, 153],
     after_stories: [
         (script: "aiy70110", name: "后日谈", order: 1),
         (script: "aiy70120", name: "后日谈 2", order: 2),
         (script: "aiy70130", name: "后日谈 3", order: 3),
     ]),
    (index: 2, name: "Eris", script: "aiy20010", unlock_flag: 52,
     hero_work: Some(2), ending_flags: [154, 155, 156],
     after_stories: [
         (script: "aiy70210", name: "后日谈", order: 1),
         (script: "aiy70220", name: "后日谈 2", order: 2),
     ]),
    (index: 3, name: "Colette", script: "aiy30010", unlock_flag: 53,
     hero_work: Some(3), ending_flags: [157, 158, 159],
     after_stories: [
         (script: "aiy70320", name: "后日谈", order: 1),
     ]),
    (index: 4, name: "Lysia", script: "aiy40010", unlock_flag: 54,
     hero_work: Some(4), ending_flags: [160, 161, 162],
     after_stories: [
         (script: "aiy70410", name: "后日谈", order: 1),
         (script: "aiy70420", name: "后日谈 2", order: 2),
     ]),
    (index: 5, name: "Lavi", script: "aiy50010", unlock_flag: 55,
     hero_work: Some(5), ending_flags: [163, 164, 165],
     after_stories: [
         (script: "aiy70510", name: "后日谈", order: 1),
         (script: "aiy70520", name: "后日谈 2", order: 2),
     ]),
],
extra_after_stories: [
    (script: "aiy71410", name: "终章 后日谈", unlock_flag: 113, order: 1),
    (script: "aiy71510", name: "终章 后日谈 2", unlock_flag: 113, order: 2),
    (script: "aiy71610", name: "终章 后日谈 3", unlock_flag: 113, order: 3),
],
```

## Unlock Logic

- **Per-heroine after-story**: unlocked when `engine.global_flags.get(&entry.unlock_flag) >= 1`.
  Same check used by route selection for "route unlocked" — if the route is playable,
  its after-story is accessible.

- **Extra after-story**: unlocked when `all_routes_cleared_flag` (113) is set.

## State

### SelectedRoute change

```rust
pub struct SelectedRoute(pub Option<String>, pub bool);
//                                              ↑ is_after_story
```

When `is_after_story` is true, the `Halt` handler in script_runner skips `RouteEnd`
and transitions back to `AppState::AfterStory` instead.

### New resource

```rust
#[derive(Resource)]
pub struct AfterStoryGroup(pub Option<usize>);
// heroine index (1-5) or 0 for extra-after-story group
```

Tracks which group the player is currently browsing in the AfterStory menu.

## AppState

Add `#[derive(Clone, PartialEq, Eq, Hash)]` variant:
```rust
AfterStory,
```

## UI: Two-Level Menu

### Entry points
- Title screen: "After Stories" button → `AppState::AfterStory`
- In-game menu: "After Stories" button → `AppState::AfterStory`

### Level 1: Group list
Shows one row per heroine that has `after_stories` entries, plus "终章 后日谈"
if `extra_after_stories` is non-empty.
- If unlocked: shows name + "PLAY" status, clickable → level 2
- If locked: shows name + "LOCKED", not clickable

### Level 2: Chapter list
Shows all `AfterStoryEntry` items for the selected group.
- Each shows `entry.name`
- Click → sets `SelectedRoute(script, is_after_story=true)` → `AppState::Gameplay`

### Back navigation
- Level 1 back → previous screen (Title or Menu)
- Level 2 back → Level 1

## Playback Flow

1. Player clicks after-story entry → `SelectedRoute("aiy70110", true)`
2. Script engine loads `aiy70110` → plays normally
3. Script reaches `Halt` (either from the after-story script itself or via Return to main.iet)
4. Script runner checks `selected_route.1` (is_after_story)
   - If true: skip `CompletedRoute` + `RouteEnd` → transition to `AppState::AfterStory`
   - If false: existing behavior (detect route completion → RouteEnd)
5. AfterStory plugin spawns Level 1 UI again on `OnEnter(AppState::AfterStory)`

## Cleanup

Existing `on_exit(AppState::Gameplay)` cleanup in `dialogue.rs` already handles despawning
`ChoiceUiRoot` and dialogue elements. The AfterStory menu has its own cleanup on
`OnExit(AppState::AfterStory)`.

## Files to Modify

| File | Change |
|------|--------|
| `src/state.rs` | Add `AfterStory` variant to `AppState` |
| `src/resources.rs` | Add `AfterStoryEntry`, `AfterStoryGroup`; update `RouteEntry`, `RouteConfig`, `SelectedRoute` |
| `src/plugins/mod.rs` | Register `after_story` plugin |
| `src/plugins/after_story.rs` | **New file** — two-level UI, navigation |
| `src/plugins/title.rs` | Add "After Stories" button → `AfterStory` state |
| `src/plugins/menu.rs` | Add "After Stories" button → `AfterStory` state |
| `src/plugins/script_runner.rs` | Halt handler: check `selected_route.1`, skip RouteEnd for after-stories |
| `src/lib.rs` | Register `AfterStory` state + plugin |
| `assets/routes.ron` | Add `after_stories` to each heroine, add `extra_after_stories` |
| `routes.ron` | Same |
| `android/app/src/main/assets/routes.ron` | Same |

## 小剧场 (Bonus Skits)

以下脚本不在 `csv.lua:extra_event` 内，属于全通解锁的混合角色小剧场短篇：

| 脚本 | 说明 | 行数 | 登场角色 |
|------|------|------|----------|
| `aiy70310` | 下午的某件事 | 2546 | 凯伊姆、柯蕾特、拉菲莉亚、酒場の店主、露天売り |
| `aiy70330` | CG鉴赏 `aped_06` | 72 | 无台词，纯 CG 展示 |
| `aiy80010` | 酔っぱらい群像 | 2667 | 艾莉斯、吉克、凯伊姆、梅尔特、缇娅 |
| `aiy80020` | — | 5121 | 未确认 |
| `aiy80030` | — | 1768 | 未确认 |
| `aiy80040` | — | 2550 | 未确认 |
| `aiy81010` | — | 715 | 未确认 |

解锁条件：**全路线通关 + 所有后日谈通关**（`full_completion_flag` 114）。
待后续确认内容后再细化登场角色。
