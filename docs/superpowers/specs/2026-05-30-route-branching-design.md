# Route Branching System — Design

Date: 2026-05-30
Status: Approved

## Overview

Replace hardcoded route flag indices (103-167) with config-driven RouteConfig resource,
add native Bevy route selection UI, and implement route completion tracking.

## Components

### A. routes.ron Config (`assets/routes.ron`)

```ron
(
    heroines: [
        (index: 1, name: "Fione", script: "aiy10010", unlock_flag: 51, hero_work: 1, ending_flags: [151, 152, 153]),
        (index: 2, name: "Eris", script: "aiy20010", unlock_flag: 52, hero_work: 2, ending_flags: [154, 155, 156]),
        (index: 3, name: "Colette", script: "aiy30010", unlock_flag: 53, hero_work: 3, ending_flags: [157, 158, 159]),
        (index: 4, name: "Lysia", script: "aiy40010", unlock_flag: 54, hero_work: 4, ending_flags: [160, 161, 162]),
        (index: 5, name: "Lavi", script: "aiy50010", unlock_flag: 55, hero_work: 5, ending_flags: [163, 164, 165]),
    ],
    extra: (
        index: 6,
        name: "After Story",
        script: "aiy00010",
        always_unlocked: true,
    ),
    route_unlock_flags: [103, 105, 107, 108, 110, 111],
    all_routes_cleared_flag: 113,
    full_completion_flag: 114,
    ending_flag_range: (151, 167),
    ending_count: 22,
)
```

### B. RouteConfig Resource

Loaded at startup via `include_str!("../assets/routes.ron")` + `ron::from_str`.

```rust
#[derive(Resource)]
pub struct RouteConfig {
    pub heroines: Vec<RouteEntry>,
    pub extra: RouteEntry,
    pub route_unlock_flags: Vec<u32>,
    pub all_routes_cleared_flag: u32,
    pub full_completion_flag: u32,
    pub ending_flag_range: (u32, u32),
    pub ending_count: u32,
}

pub struct RouteEntry {
    pub index: u32,
    pub name: String,
    pub script: String,
    pub unlock_flag: u32,
    pub hero_work: Option<u32>,
    pub ending_flags: Vec<u32>,
    pub always_unlocked: bool,
}
```

- `hero_work` is `Option<u32>` (None for extra route) — must match `HEROINE_WORK_MAP`
- If routes.ron conflicts with `HEROINE_WORK_MAP`, routes.ron takes precedence

### C. RouteFlag Refactoring

Current `script_runner.rs` hardcodes:
- `hero_routes = [103, 105, 107, 108, 110, 111]` → `config.route_unlock_flags`
- `flag 113` → `config.all_routes_cleared_flag`
- `flag 114` → `config.full_completion_flag`
- `(151..=167)` → `config.ending_flag_range`

Both skip and normal paths read from `Res<RouteConfig>`.

### D. Route Completion Tracking

`ScriptEngine` gains a method:
```rust
pub fn detect_route_completion(&self, config: &RouteConfig) -> Option<u32>
```
Called when `Halt` is executed. If `current_script` starts with a route script prefix, returns the route index. Index is inserted into `UnlockState.routes_cleared`.

### E. RouteSelection State & UI

- **State**: `AppState::RouteSelection` added to enum
- **Entry from**: Title screen (button between Gallery and Settings), Menu screen
- **UI structure**: Same pattern as `gallery.rs`:
  - `OnEnter(RouteSelection)` → `setup_route_selection()`
  - `Update` handlers for button clicks, back button
  - `OnExit(RouteSelection)` → `cleanup_route_selection()`
- **Route buttons**: 120×160 px, centered grid (3 rows × 2 cols or 2 rows × 3 cols)
  - States: LOCKED (grey, no interaction), PLAY (green, clickable), CLEARED (blue, clickable but marked)
  - Lock check: `global_flags[entry.unlock_flag] >= 1`
  - Clear check: `UnlockState.routes_cleared.contains(entry.name)`
- **Click**: → `screen_transition.pending_state = Some(AppState::Gameplay)` + direct `CallScript`
- **Back button**: 80×36 px, top-left

### F. Script Loading on Route Select

New resource `SelectedRoute(Option<String>)` stores the chosen route script name.
When a route button is clicked:
1. `SelectedRoute` is set to `Some(entry.script)`
2. `screen_transition.pending_state = Some(AppState::Gameplay)`
3. On `OnEnter(Gameplay)`, `script_runner` checks `SelectedRoute`:
   - If `Some(script)`: call `engine.start_script(script)`, clear `SelectedRoute`
   - If `None`: normal flow (new game from Title)

## Files to create/modify

| File | Action |
|------|--------|
| `assets/routes.ron` | **Create** — route config |
| `src/resources.rs` | **Modify** — add RouteConfig, RouteEntry, update UnlockState |
| `src/plugins/script_runner.rs` | **Modify** — RouteFlag reads config, detect_route_completion on Halt |
| `src/plugins/mod.rs` | **Modify** — register RoutePlugin |
| `src/plugins/routing.rs` | **Create** — RoutePlugin: setup, button handlers, cleanup |
| `src/plugins/title.rs` | **Modify** — add RouteSelection button |
| `src/state.rs` | **Modify** — add RouteSelection variant |
| `src/lib.rs` | **Modify** — register RoutePlugin, init RouteConfig |
| `src/script.rs` | **Modify** — add detect_route_completion to ScriptEngine |
| `src/plugins/menu.rs` | **Modify** — add RouteSelection entry point |

## Error handling

- Missing `routes.ron` → panic at build (compile-time embed)
- Invalid RON → panic at build (compile-time embed)
- Route script not found → log error + return to Title
- `Unit` interaction on locked route → no-op (button not clickable)

## Testing

- `RouteConfig` deserialization: manual test via `cargo run` — check no panic on startup
- Route button states: visual check (locked/unlocked/cleared)
- RouteFlag correctness: compare old hardcoded values with config values
- Route completion: run a route script to `Halt`, verify `UnlockState.routes_cleared` updated
