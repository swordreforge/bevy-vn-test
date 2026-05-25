# artemis-export: Artemis Engine Script Converter

## Overview

`artemis-export` is a CLI tool within the `bevy-vn` workspace that converts Artemis visual novel engine assets (`.asb` binary scripts + `.lua` config files) into Bevy VN's `ScriptCmd` RON format. It's a single-pass converter: parse → map recognized patterns → write `.bscript.ron`.

## Scope

- **Input**: 204 `.asb` files in `game-source/scenario/` + 34 `.lua` config files
- **Output**: One `.bscript.ron` file per `.asb` (e.g. `aiy70330.asb` → `aiy70330.bscript.ron`)
- **Non-goals**: PFS repacking (handled by `pfs_unpacker pack`), Android adapter, Lua runtime execution

## Project Structure

```
tools/artemis-export/
├── Cargo.toml                   # depends on bevy-vn lib for ScriptCmd types
└── src/
    ├── main.rs                  # CLI entry: --input <game-source> --output <assets/scripts>
    ├── asb.rs                   # .asb binary parser
    ├── lua_config.rs            # Lua config metadata extraction
    └── mapper.rs                # Artemis command → ScriptCmd mapping
```

`bevy-vn/Cargo.toml` adds workspace member + library target for type sharing.

## Pipeline

### Step 1: Lua config pre-pass (lua_config.rs)

Parse `game-source/system/extra/*.lua` and `system/adv/*.lua` to extract:

| Lua file | Extracted data |
|---|---|
| `extra/cg.lua` | CG set → image file mapping, thumbnail paths |
| `extra/bgm.lua` | BGM ID → ogg file mapping, track titles |
| `extra/scene.lua` | Scene ID → .asb file + label mapping |
| `adv/fg.lua` | FG character → sprite file mapping |
| `adv/sound.lua` | Voice/SE file naming conventions |
| `csv.lua` | UI layout definitions (used for reference) |

This is **text-based extraction** (regex/pattern matching on known Lua table shapes), not a full Lua parser. We look for assignments like `csv.extra_cgmode["set_name"] = { ... }` and `csv.extra_bgm["bgm_id"] = { ... }`.

### Step 2: ASB binary parser (asb.rs)

Parse the Artemis Script Binary format. The file structure is:

```
[4-byte magic "ASB\0"] [12-byte header/version] [blocks...]
```

Each block (label or command):
```
[0x00 block marker: 1 byte]
[string length: 4 bytes u32 LE]
[string data: length bytes]
[padding to 4-byte alignment]
[parameters...]
```

Parameter encoding depends on the command type — positional values (int or string) following the same length-prefixed string format. The parser produces an intermediate representation:

```rust
struct AsbScript {
    blocks: Vec<AsbBlock>,
}

struct AsbBlock {
    label: String,
    commands: Vec<AsbCommand>,
}

struct AsbCommand {
    tag: String,
    params: Vec<AsbParam>,
}

enum AsbParam {
    Int(i32),
    Str(String),
}
```

### Step 3: Command mapper (mapper.rs)

Iterate each `AsbCommand`, match known patterns to produce `ScriptCmd` variants. Unrecognized/rendering commands are silently skipped.

### Step 4: Output

Write one RON file per `.asb` with the script data, structured as `Vec<ScriptCmd>`.

## Command Mapping Reference

### Narrative commands (converted)

| Artemis pattern | → ScriptCmd | Notes |
|---|---|---|
| `calllua { function="set_bg", file=..., fade=... }` | `SetBg { file, transition, duration }` | Resolve file via Lua `init.bg_path`, fade/transition mapped from param |
| `calllua { function="tags.bgm", file=..., ... }` | `PlayBgm { id, volume, fade_in }` | File maps via `csv.extra_bgm` |
| `calllua { function="bgm_stop", ... }` | `StopBgm { id, fade_out }` | |
| `calllua { function="se_play", file=..., ... }` | `PlaySe { file, volume }` | File padded to 5 digits |
| `calllua { function="voice_play", file=... }` | `PlayVoice { file }` | |
| `calllua { function="tags.wt" / tags.wtx" }` | `Wait { duration }` | Duration from param or defaults |
| `calllua { function=choice-related-* }` | `Choice { options }` | Choice options constructed from context |
| Jump / Call / Return | `Jump / Call / Return` | Cross-file jump: `file.asb,label` |
| SetGlobalFlag { index, value } | `AffectionChange` | Only when mapping known affection flags |
| `calllua { function="affection_change", ... }` | `AffectionChange { char_id, delta }` | |
| `calllua { function="*show_fg*", ... }` | `ShowFg { char_id, expression, position }` | |
| `calllua { function="*hide_fg*", ... }` | `HideFg { char_id }` | |
| `calllua { function="*show_cg*", ... }` | `ShowCg { file }` | Resolve CG image path |
| `calllua { function="*hide_cg*" }` | `HideCg` | |
| `calllua { function="*save_point*" }` | `SavePoint` | |
| `calllua { function="tags.dialogue", ... }` | `Dialogue { speaker, text }` | |

### Dialogue text extraction

Dialogue text in Artemis scripts is typically embedded via `calllua { function="tags.dialogue" }` or a dedicated `message()` call, or sometimes as a native ASB text command. The mapper will need to recognize the actual pattern used in this game's scripts (to be determined during implementation by inspecting `.asb` content).

Fallback: if dialogue pattern is ambiguous, the mapper emits a `Dialogue { speaker: None, text: "<raw>" }` for debugging.

### Rendering/system commands (skipped)

All of these are silently ignored:
`lyc`, `lydel`, `lyprop`, `lyedit`, `lytween`, `lytweendel`, `flip`, `trans`, `btn_start`, `btnstat`, `alkeystart`, `key_stop_adv`, `delonpush`, `setonpush`, `lyevent`, `lyc2`, `lyc2sys`, `chgmsg`, `rp`, `/chgmsg`, `wait`, `eqwait`, `sestop`, `seplay`, `sstop`, `sefade`, `sepan`, `splay`, `sfade`, `calllua` (when function is a rendering Lua helper like `title_bgmstart`, `extra_*`, `reset_*`, `btn_*`, `slider_*`, `msgon/msgoff`, etc.)

Heuristic: if the `calllua` function name starts with `tags.`, it's a user-defined tag → map if known, skip otherwise. If it starts with `btn_`, `extra_`, `reset_`, `slider_`, `config_`, `sys_`, `se_` → skip.

## CLI Interface

```
artemis-export --input /path/to/game-source --output assets/scripts
```

Optional flags:
- `--verbose` — log skipped commands for debugging
- `--dry-run` — count files/conversions without writing
- `--list-skipped` — print all skipped commands for mapping gap analysis

## Error Handling

- Unparseable `.asb` file: log warning, skip file (don't abort batch)
- Unrecognized Lua config format: log warning, use defaults/path heuristics
- Output directory created automatically
- RON serialization errors: hard fail (shouldn't happen with valid ScriptCmd types)

## Testing

- Unit tests for ASB block/command parser against known small files (`aiy70330.asb` = 653 bytes)
- Unit tests for mapper: known patterns → expected ScriptCmd output
- Integration: convert all 204 files, verify output count matches input count
- Verify output compiles via `cargo check` in bevy-vn (triggers ScriptCmd deserialization)
- Manual: smoke test with bevy-vn engine using a converted script

## Dependencies

Same as `bevy-vn` workspace: `serde`, `ron`, `anyhow`. No new external deps needed — binary parsing is all manual `Read + u32::from_le_bytes`.
