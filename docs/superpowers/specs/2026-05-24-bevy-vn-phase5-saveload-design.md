# Phase 5: Save/Load System Design

## Goal

A functional save/load system with a 15-slot grid UI, JSON file I/O, and state restoration on load.

## Flow

```
Gameplay → Escape → Menu state → [Save] / [Load] buttons → SaveLoad state
  → 15-slot grid overlay (ZIndex 5)
  → Click slot → confirmation dialog → execute → return to Menu
```

## Menu Plugin (NEW)

- Registered for `AppState::Menu`
- `OnEnter(Menu)`: spawns UI buttons — Save, Load, Settings, Gallery, Back to Title
- Each button sets `SaveLoadMode` resource and transitions to corresponding state
- `OnExit(Menu)`: despawns all menu UI entities

## SaveLoadMode Resource

```rust
#[derive(Resource)]
pub struct SaveLoadMode(pub bool); // true = Save, false = Load
```

Set by Menu buttons before transitioning to `AppState::SaveLoad`.

## SaveLoadPlugin (enhanced)

### OnEnter(SaveLoad)
1. Read `SaveLoadMode` resource
2. Spawn overlay root (ZIndex 5, full-screen dark backdrop)
3. Spawn 15 slot entities in a 5×3 grid
4. Each slot shows:
   - Slot number (1-15)
   - If empty: `[EMPTY]`
   - If filled: thumbnail placeholder, scene name, timestamp, play time
5. In Load mode: empty slots grayed out

### Interactions
- **Save mode**: click any slot → if empty: save immediately. If filled: show "Overwrite?" confirm → save.
- **Load mode**: click filled slot → "Load?" confirm → restore state.
- **Escape**: despawn overlay → back to Menu.
- **Click on backdrop** (outside grid): despawn overlay → back to Menu.

### Confirmation Dialog
- Simple text overlay: "Save to slot X?" / "Load slot X?" with Yes/No buttons.
- Uses separate marker components, spawned on demand, despawned after choice.

### Save Implementation (`SaveManager::save_slot`)
```
1. Snapshot ScriptEngine (current_script, current_line, call_stack, flags)
2. Snapshot AffectionMap
3. Build SaveData struct
4. Ensure saves/ directory exists (std::fs::create_dir_all)
5. Write JSON to saves/slot_{idx}.json
6. Update SaveManager.slots[idx]
```

### Load Implementation (`SaveManager::load_slot`)
```
1. Read JSON from saves/slot_{idx}.json
2. Deserialize into SaveData
3. Restore ScriptEngine fields (line, call_stack, flags from SaveData)
4. Restore AffectionMap
5. Transition to Gameplay state
```

### Option: Re-enter Gameplay on Load
- After loading, transition to `AppState::Gameplay`.
- The ScriptRunner will resume from the saved line on the next advance.
- Visual state restoration (bg, sprites, bgm) is **deferred** — the script will naturally catch up as the player advances.

## Files to Create/Modify

### NEW: `src/plugins/menu.rs`
- MenuPlugin with menu_ui_setup / menu_ui_cleanup
- Button spawn helpers for Save/Load/Settings/Gallery/Title

### MODIFY: `src/plugins/save_load.rs`
- Full implementation replacing the placeholder
- Systems: setup_save_load_ui, handle_slot_click, confirm_dialog, execute_save, execute_load, cleanup_save_load
- File I/O: save_to_disk, load_from_disk

### MODIFY: `src/resources.rs`
- Add `SaveLoadMode` resource
- Add save_slot/load_slot methods to SaveManager

### MODIFY: `src/plugins/mod.rs`
- Add `pub mod menu;`

### MODIFY: `src/main.rs`
- Add `mod menu;` import and `MenuPlugin` registration

## SaveData (existing, augmented)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub timestamp: String,
    pub scene_name: String,       // human-readable label
    pub script_path: String,      // e.g. "test.bscript.ron"
    pub script_line: usize,
    pub call_stack: Vec<(String, usize)>,
    pub flags: HashMap<String, i32>,
    pub affection: HashMap<String, i32>,
    pub play_time: u64,
}
```

The existing struct is sufficient. No new fields needed for Phase 5.

## Deferred

- **Thumbnails**: placeholder text shown; actual capture deferred (Phase 7+).
- **Visual state restoration**: on load, only script/affection state restored. The player may see a brief incorrect bg until script commands replay. Full restoration deferred.
- **Auto-save / Quick-save**: deferred (Phase 5 follow-up or Phase 7).
- **Android save path**: uses `std::env::current_dir()/saves/` for now; Android adjustment deferred.

## Testing

1. Start game → click through title → Escape → Menu appears
2. Click Save → 15-slot grid with empty/filled indicators
3. Click empty slot → save created → slot updates to show data
4. Escape → back to Menu → Load → same grid, filled slots active
5. Click filled slot → confirm → state restored → in Gameplay at saved position
6. Overwrite: click filled slot in Save mode → "Overwrite?" confirm → works
