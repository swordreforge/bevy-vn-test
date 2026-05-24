# Phase 6: Affection Condition Branching + CG Gallery

## Overview

Two independent sub-systems:
1. **Affection Condition Branching** — evaluate `AffectionMap` values in script `Condition`-like commands
2. **CG Gallery** — replace placeholder with full 3-column grid, thumbnail→fullscreen viewer, unlock tracking

---

## Part 1: Affection Condition Branching

### New ScriptCmd variant

Add to `src/script.rs`:

```rust
AffectionCondition {
    char_id: String,
    value: i32,
    operator: ConditionOp,
    goto: String,
}
```

Reuses existing `ConditionOp` enum (Greater, Less, Equal, GreaterEqual, LessEqual).

### ScriptRunner handling

In `process_advance` match arm: read char_id from `AffectionMap`, evaluate with operator, jump if met. Same pattern as existing `Condition` arm but sourced from `AffectionMap` instead of `engine.flags`.

### Example

```ron
AffectionChange(char_id: "nayuta", delta: 1),
// ... later ...
AffectionCondition(char_id: "nayuta", value: 3, operator: GreaterEqual, goto: "affection_route"),
AffectionCondition(char_id: "nayuta", value: 2, operator: LessEqual, goto: "normal_route"),
```

---

## Part 2: CG Gallery

### Gallery State

```rust
#[derive(Resource, Default)]
pub struct GalleryState {
    pub fullscreen: Option<String>, // None = grid mode, Some(file) = viewing fullscreen CG
}
```

### Gallery Layout

Full dark overlay (ZIndex 5) containing:

1. **Header row**: "← Back" button (top-left) + "CG Gallery" title
2. **3-column scrollable grid**: each cell is a 16:9 aspect-ratio thumbnail
3. **Fullscreen viewer**: spawned on top when a thumbnail is clicked

### CG Manifest

Static list of all known CG filenames (defined in gallery plugin). Each file maps to `images/ev/{file}`. Locked/unlocked determined by `UnlockState.cg_unlocked`.

```rust
const ALL_CG_FILES: &[&str] = &["eve_010101.png"];
```

### Grid Cell Behavior

- **Unlocked** (`file in cg_unlocked`): show CG thumbnail via `ImageNode`, clickable
- **Locked** (`file not in cg_unlocked`): dark placeholder, no interaction

### Fullscreen Viewer

- Click unlocked thumbnail → spawn fullscreen `ImageNode` (ZIndex 6) covering entire window
- Click anywhere on fullscreen CG → despawn fullscreen entity, return to grid
- Escape key also dismisses fullscreen

### Back Button

Click "← Back" (Node with Text, click/touch detection via `Interaction` component on button entity) or press Escape:
- Grid mode → transition `AppState::Gallery` → `AppState::Menu`
- Fullscreen mode → despawn fullscreen entity, return to grid

### Gallery Cleanup

`OnExit(AppState::Gallery)` despawns all entities with `GalleryRoot` or `GalleryFullscreen` component.

---

## Part 3: Unlock Mechanism

### Auto-unlock on ShowCg

In `ScriptRunner::process_advance`, the `ShowCg` match arm also writes the file to `UnlockState.cg_unlocked`.

### Explicit UnlockCg command

New `ScriptCmd` variant:

```rust
UnlockCg { file: String },
```

ScriptRunner match arm: insert file into `UnlockState.cg_unlocked`. No-op beyond that (no visual display).

---

## Part 4: Components

Add to `src/components.rs`:

```rust
pub struct GalleryRoot;
pub struct GalleryThumbnail(pub String);      // file name
pub struct GalleryLocked;                     // marker for locked slot
pub struct GalleryFullscreen;
pub struct GalleryBackButton;
```

---

## Part 5: File Changes Summary

| File | Change |
|------|--------|
| `src/script.rs` | +`AffectionCondition`, +`UnlockCg` variants |
| `src/resources.rs` | +`GalleryState` resource |
| `src/components.rs` | +5 marker components |
| `src/plugins/script_runner.rs` | +match arms for both new variants. Auto-unlock on ShowCg. Inject `UnlockState` + `AffectionMap`. |
| `src/plugins/gallery.rs` | Full rewrite: setup, grid, fullscreen, interaction, cleanup |
| `src/plugins/mod.rs` | (no change — gallery already registered) |
| `src/main.rs` | (no change) |
| `assets/scripts/test.bscript.ron` | (optional) add UnlockCg + affection branch demo |

---

## Part 6: Integration Flow

```
Gameplay → Menu → Gallery (AppState::Gallery)
  → Grid view (3 cols, unlocked=image, locked=dark) 
  → Click thumbnail → fullscreen CG (ZIndex 6)
  → Click anywhere → back to grid
  → ← Back / Escape → Menu

While in Gameplay:
  ShowCg → renders image + auto-unlocks CG for gallery
  UnlockCg → unlocks CG without displaying
  AffectionCondition → branches script based on affection value
```
