# Transition Animations Design

## Overview
Implement cross-fade transitions for asset changes (BG/FG/CG) during gameplay and fade-to-black transitions between AppStates, wiring up the existing `Transition` enum and script fields that were designed but never implemented.

## Scope
- **Asset transitions:** BG cross-fade (dual-buffer), FG/CG fade-in/out
- **State transitions:** Fade-to-black overlay between AppStates
- **Transition type:** `Fade` (alpha lerp). `Dissolve` falls back to instant. `Instant`/`None` stays instant.
- **Out of scope:** Slide transitions, sprite transforms, non-alpha effects

## Data Flow

### Rendering Messages
Add `transition` and `duration` fields to all 5 rendering messages in `src/rendering_messages.rs`:
```rust
pub struct SetBgMessage {
    pub file: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

pub struct ShowFgMessage {
    pub char_id: String,
    pub expression: String,
    pub position: FgPosition,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

pub struct HideFgMessage {
    pub char_id: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

pub struct ShowCgMessage {
    pub file: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

pub struct HideCgMessage {
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}
```

### Script Runner
In `src/plugins/script_runner.rs`, replace `transition: _` and `duration: _` pattern matches with pass-through to message constructors. Both normal mode and skip mode blocks need updating.

## BG Cross-Fade (Dual-Buffer)

### Resource Changes (`src/resources.rs`)
Extend `BgState`:
```rust
pub struct BgCrossFade {
    pub timer: Timer,
}

pub struct BgState {
    pub entities: [Entity; 2],
    pub active_idx: usize,
    pub fade: Option<BgCrossFade>,
}
```

### Handler (`src/plugins/rendering.rs`)
**`handle_set_bg` — Fade path:**
1. Load texture into inactive buffer entity (via texture cache)
2. Set inactive buffer `BackgroundColor` alpha to 0.0, `Visibility::Visible`
3. Store `BgCrossFade { timer: Timer::from_seconds(duration, TimerMode::Once) }`

**Instant/None path:** same as current (visibility toggle, no fade)

**Mid-fade overlap:** if `handle_set_bg` fires while a fade is active, the existing fade completes instantly (snap to end state → clear → start new fade).

### Update System: `update_bg_fade`
1. If `bg_state.fade` is `None`, return
2. Advance timer. `t = timer.fraction()`
3. Active buffer alpha: `1.0 - t` (1→0)
4. Inactive buffer alpha: `t` (0→1)
5. Apply alphas via `BackgroundColor::srgba(0.0, 0.0, 0.0, alpha)` on both entities
6. On timer completion: hide old active (`Visibility::Hidden`), swap `active_idx`, set `fade = None`

## FG Sprite Fade

### Resource Changes
Extend `SpriteSlotInfo`:
```rust
pub struct SpriteFade {
    pub timer: Timer,
    pub kind: SpriteFadeKind,
}

pub enum SpriteFadeKind {
    FadeIn,
    FadeOut,
}

pub struct SpriteSlotInfo {
    pub char_id: String,
    pub expression: String,
    pub entity: Entity,
    pub texture: Option<Handle<Image>>,
    pub fade: Option<SpriteFade>,
}
```

### Handler
**`handle_show_fg` — Fade path:**
1. Load texture, set entity `ImageNode`
2. Set `BackgroundColor` alpha to 0.0
3. Set `Visibility::Visible`
4. Start `SpriteFade { timer, kind: FadeIn }`

**`handle_hide_fg` — Fade path:**
1. Start `SpriteFade { timer, kind: FadeOut }`
2. Alpha is handled by update system; visibility hidden on completion

**Instant/None path:** same as current behavior

### Update System: `update_fg_fade`
1. For each sprite slot with active fade, advance timer
2. `FadeIn`: alpha = `t` (0→1)
3. `FadeOut`: alpha = `1.0 - t` (1→0)
4. Apply alpha via `BackgroundColor`
5. On `FadeOut` completion: set `Visibility::Hidden`, clear fade
6. On `FadeIn` completion: clear fade

## CG Fade

### Resource Changes
Extend `CgState`:
```rust
pub struct CgFade {
    pub timer: Timer,
    pub kind: CgFadeKind,
}

pub enum CgFadeKind {
    FadeIn,
    FadeOut,
}

pub struct CgState {
    pub active: bool,
    pub entity: Option<Entity>,
    pub texture: Option<Handle<Image>>,
    pub fade: Option<CgFade>,
}
```

### Handler
**`handle_show_cg` — Fade path:**
1. Despawn old CG (same as current)
2. Spawn new CG entity at alpha 0.0, `Visibility::Visible`
3. Start `CgFade { timer, kind: FadeIn }`

**`handle_hide_cg` — Fade path:**
1. If CG exists, start `CgFade { timer, kind: FadeOut }`

**Instant/None path:** same as current (instant spawn and despawn)

### Update System: `update_cg_fade`
Same pattern as FG update system.

## State Transitions (Fade-to-Black)

### New Resource
```rust
#[derive(Resource, Default)]
pub struct ScreenTransition {
    pub overlay: Option<Entity>,
    pub phase: TransitionPhase,
    pub pending_state: Option<AppState>,
}

pub enum TransitionPhase {
    Idle,
    FadingToBlack { timer: Timer },
    FadingFromBlack { timer: Timer },
}
```

### New Component
```rust
#[derive(Component)]
pub struct TransitionOverlay;
```

### Trigger Points
Replace direct `NextState<AppState>` writes at 9 locations with `ScreenTransition.pending_state = Some(target)`:

| Location | From → To |
|---|---|
| `title.rs` "Start" button | Title → Gameplay |
| `menu.rs` "Continue" | Menu → Gameplay |
| `menu.rs` "Save" | Menu → SaveLoad |
| `menu.rs` "Load" | Menu → SaveLoad |
| `menu.rs` "Gallery" | Menu → Gallery |
| `menu.rs` "Settings" | Menu → Settings |
| `save_load.rs` load button | SaveLoad → Gameplay |
| `gallery.rs` back button | Gallery → Menu |
| `settings.rs` back button | Settings → Menu |

Note: `Gameplay → Menu` (Escape) is handled differently — it triggers `MenuToggleEvent` which is then handled by `menu.rs`. The menu plugin already writes `NextState<AppState>::set(AppState::Menu)`. This should also be replaced with `ScreenTransition.pending_state`.

### Always-Active System: `handle_screen_transition`
Runs in `Update` unconditionally (not gated by any AppState).

1. **If phase is `Idle` and `pending_state` is set:**
   - Spawn fullscreen black overlay (UI Node, `TransitionOverlay` marker, ZIndex 10, `BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0))`)
   - Change phase to `FadingToBlack { timer: Timer::from_seconds(0.3, TimerMode::Once) }`

2. **If phase is `FadingToBlack`:**
   - Advance timer. Alpha = `timer.fraction()` (0→1)
   - Apply alpha to overlay's `BackgroundColor`
   - On completion: set alpha to 1.0 (full black), then `NextState::set(pending_state)`, change phase to `FadingFromBlack { timer: Timer::from_seconds(0.3, TimerMode::Once) }`

3. **If phase is `FadingFromBlack`:**
   - Advance timer. Alpha = `1.0 - timer.fraction()` (1→0)
   - Apply alpha to overlay's `BackgroundColor`
   - On completion: despawn overlay entity, set phase to `Idle`, clear `pending_state`

### ZIndex
Overlay uses ZIndex 10 to ensure it covers all existing UI layers (dialogue=3, choice=4, menu/save-load=5, settings=5, gallery fullscreen=6).

## Files to Modify

| File | Changes |
|---|---|
| `src/rendering_messages.rs` | Add transition/duration fields to 5 message structs |
| `src/plugins/script_runner.rs` | Thread transition/duration through all rendering message dispatches (both normal and skip mode) |
| `src/plugins/rendering.rs` | Branch on transition type in 5 handlers, add 3 update systems (`update_bg_fade`, `update_fg_fade`, `update_cg_fade`) |
| `src/resources.rs` | Extend `BgState`, `SpriteSlotInfo`, `CgState` with fade states; add `ScreenTransition` resource + `TransitionPhase` enum |
| `src/components.rs` | Add `TransitionOverlay` marker component |
| `src/main.rs` | Register `ScreenTransition` resource + update systems (`handle_screen_transition`, `update_bg_fade`, `update_fg_fade`, `update_cg_fade`) |
| `src/plugins/title.rs` | Swap `NextState<AppState>::set(Gameplay)` → `screen_transition.pending_state = Some(AppState::Gameplay)` |
| `src/plugins/menu.rs` | Same for 5 trigger points |
| `src/plugins/save_load.rs` | Swap load button state change |
| `src/plugins/gallery.rs` | Swap back button |
| `src/plugins/settings.rs` | Swap back button |

## Edge Cases
- **Mid-fade new request:** Existing fade snaps to completion instantly, then new fade starts
- **Leave Gameplay during fade:** `OnExit(Gameplay)` despawns all rendering entities. Fade state is cleared by resource drop/re-init on re-entry
- **No CG entity on fade-out:** `CgFade` skip if entity is `None`
- **Fast skip mode:** Skip mode advances script instantly per-command. SetBg/FG/CG messages with `transition: None` (no fade during skip) — script runner does not pass transition info during skip mode
- **During state transition fade:** Input is not blocked. The overlay covers everything so clicks are absorbed by the overlay (it has a `Node` with fullscreen dimensions). The only system that needs mouse access during transition is `handle_screen_transition`

## Ordering
Systems ordered in `Update`:
```
screen_transition → bg_fade → fg_fade → cg_fade → rendering handlers
```
Fade updates run first so they apply current alpha before any new handlers fire. New handlers write fade state for the next frame's update systems.

## Default Duration
If `duration` is `None` when transition is `Some(Fade)`, default to 0.5 seconds for asset transitions and 0.3 seconds for state transitions.
