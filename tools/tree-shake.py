#!/usr/bin/env python3
"""
Tree-shaking analysis for bevy-vn assets.

Scans all .bscript.ron files, extracts every asset reference,
cross-references against files on disk, and reports unused assets.

Usage:
    python tools/tree-shake.py                        # full report
    python tools/tree-shake.py --unreferenced-only    # just unused items
    python tools/tree-shake.py --json                 # machine-readable JSON
    python tools/tree-shake.py --voice-detail         # show all 1500+ unused voice files
"""

import argparse
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

ASSETS = Path(__file__).resolve().parent.parent / "assets"


# ── Reference extraction ──────────────────────────────────────────────

BGM_RE     = re.compile(r'\bPlayBgm\s*\(\s*id\s*:\s*"([^"]+)"')
SE_RE      = re.compile(r'\b(?:PlaySe|LoopSe)\s*\([^}]*?file\s*:\s*"([^"]+)"')
VOICE_RE   = re.compile(r'\bPlayVoice\s*\([^}]*?file\s*:\s*"([^"]+)"')
CG_RE      = re.compile(r'\b(?:ShowCg|UnlockCg)\s*\([^}]*?file\s*:\s*"([^"]+)"')
BG_RE      = re.compile(r'\b(?:SetBg|ScrollBg)\s*\([^}]*?file\s*:\s*"([^"]+)"')
FG_RE      = re.compile(r'\bShowFg\s*\([^}]*?char_id\s*:\s*"([^"]+)"')
FACE_RE    = re.compile(r'\bShowFace\s*\([^}]*?char_id\s*:\s*"([^"]+)"')
SPRITE_RE  = re.compile(r'\bDrawSprite\s*\([^}]*?file\s*:\s*"([^"]+)"')
ANIME_RE   = re.compile(r'\bAnimateSprite\s*\([^}]*?file\s*:\s*"([^"]+)"')
PLAYMOVIE_RE = re.compile(r'\bPlayMovie\s*\([^}]*?file\s*:\s*"([^"]+)"')
DRAWSPRITEEX_RE = re.compile(r'\bDrawSpriteEx\s*\([^}]*?file\s*:\s*"([^"]+)"')
RAIN_RE    = re.compile(r'\bRainMja\s*\([^}]*?file\s*:\s*"([^"]+)"')


def map_video_file(asb_path: str) -> str:
    """Mirrors src/resources.rs:map_video_file."""
    if asb_path.endswith(".mpg"):
        return asb_path[:-4] + ".ogv"
    elif asb_path.endswith(".ogv"):
        return asb_path
    else:
        return asb_path + ".ogv"


def extract_refs(script_dir: Path):
    refs = {
        "bgm": set(),
        "se": set(),
        "voice": set(),
        "cg": set(),
        "bg": set(),
        "fg": set(),
        "face": set(),
        "sprite": set(),
        "anime": set(),
        "playmovie": set(),
        "drawspriteex": set(),
        "rain_file": set(),
    }
    for fpath in sorted(script_dir.glob("*.bscript.ron")):
        text = fpath.read_text(encoding="utf-8", errors="replace")
        for m in BGM_RE.finditer(text):           refs["bgm"].add(m.group(1))
        for m in SE_RE.finditer(text):             refs["se"].add(m.group(1))
        for m in VOICE_RE.finditer(text):          refs["voice"].add(m.group(1))
        for m in CG_RE.finditer(text):             refs["cg"].add(m.group(1))
        for m in BG_RE.finditer(text):             refs["bg"].add(m.group(1))
        for m in FG_RE.finditer(text):             refs["fg"].add(m.group(1))
        for m in FACE_RE.finditer(text):           refs["face"].add(m.group(1))
        for m in SPRITE_RE.finditer(text):         refs["sprite"].add(m.group(1))
        for m in ANIME_RE.finditer(text):          refs["anime"].add(m.group(1))
        for m in PLAYMOVIE_RE.finditer(text):      refs["playmovie"].add(m.group(1))
        for m in DRAWSPRITEEX_RE.finditer(text):   refs["drawspriteex"].add(m.group(1))
        for m in RAIN_RE.finditer(text):           refs["rain_file"].add(m.group(1))
    return refs


def parse_obj_index(path: Path):
    """Return dict mapping sprite name → relative path."""
    text = path.read_text(encoding="utf-8", errors="replace")
    return dict(re.findall(r'"([^"]+)"\s*:\s*"([^"]+)"', text))


# ── Scanning assets on disk ───────────────────────────────────────────

def scan_assets():
    files = {}

    def _ls(p):
        return sorted(p.name for p in p.glob("*")) if p.exists() else []

    files["bgm"] = _ls(ASSETS / "audio/bgm")
    files["se"] = _ls(ASSETS / "audio/se") if (ASSETS / "audio/se").exists() else []
    files["voice"] = _ls(ASSETS / "audio/voice") if (ASSETS / "audio/voice").exists() else []
    files["bg"] = _ls(ASSETS / "image/bg")
    files["ev"] = _ls(ASSETS / "image/ev")
    files["ev_mono"] = _ls(ASSETS / "image/ev/mono")
    files["thumbnail"] = _ls(ASSETS / "image/thumbnail")
    files["anime"] = _ls(ASSETS / "image/anime")
    files["movie"] = _ls(ASSETS / "movie")

    # FG per character folder
    files["fg"] = {}
    fg_dir = ASSETS / "image/fg"
    if fg_dir.exists():
        for subdir in sorted(fg_dir.iterdir()):
            if subdir.is_dir():
                files["fg"][subdir.name] = sorted(p.name for p in subdir.glob("*"))

    files["face"] = _ls(ASSETS / "image/face")

    # obj subdirectories
    files["obj"] = {}
    obj_dir = ASSETS / "image/obj"
    if obj_dir.exists():
        for subdir in sorted(obj_dir.iterdir()):
            if subdir.is_dir():
                files["obj"][subdir.name] = sorted(p.name for p in subdir.glob("*"))

    return files


# ── Analysis ──────────────────────────────────────────────────────────

def analyze(refs, obj_index, disk_files):
    report = {}

    # --- BGM ---
    bgm_refs = refs["bgm"]
    bgm_ids_on_disk = set()
    for f in disk_files["bgm"]:
        m = re.match(r"bgm_(.+)\.ogg", f)
        if not m:
            continue
        raw = m.group(1)
        # split stereo: bgm_XXXX_a.ogg  →  ID = XXXX
        if raw.endswith("_a"):
            bgm_ids_on_disk.add(raw[:-2])
        elif raw.endswith("_b"):
            pass  # accounted for by _a
        else:
            bgm_ids_on_disk.add(raw)
    unused_bgm = sorted(bgm_ids_on_disk - bgm_refs)
    report["bgm"] = {
        "on_disk_ids": len(bgm_ids_on_disk),
        "referenced": len(bgm_refs),
        "unused": unused_bgm,
    }

    # --- SE ---
    se_refs = refs["se"]
    se_stems_on_disk = set()
    for f in disk_files["se"]:
        stem = f.replace("_a.ogg", "").replace("_b.ogg", "").replace(".ogg", "")
        se_stems_on_disk.add(stem)
    unused_se = sorted(se_stems_on_disk - se_refs)
    report["se"] = {
        "on_disk_files": len(disk_files["se"]),
        "on_disk_stems": len(se_stems_on_disk),
        "referenced": len(se_refs),
        "unused": unused_se,
    }

    # --- Voice ---
    voice_refs = refs["voice"]
    voice_stems = {f.replace(".ogg", "") for f in disk_files["voice"]}
    unused_voice = sorted(voice_stems - voice_refs)
    report["voice"] = {
        "on_disk": len(disk_files["voice"]),
        "referenced": len(voice_refs),
        "unused": unused_voice,
    }

    # --- CG ---
    # CGs can also be referenced via SetBg (used as backgrounds)
    all_cg_script_refs = refs["cg"]
    all_cg_script_refs |= {b for b in refs["bg"] if b.startswith(("eve_", "mon_", "hcg_"))}
    cg_stems = set()
    for f in disk_files["ev"]:
        cg_stems.add(Path(f).stem)
    for f in disk_files["ev_mono"]:
        cg_stems.add("mono/" + Path(f).stem)
    unused_cg = sorted(cg_stems - all_cg_script_refs)
    report["cg"] = {
        "on_disk": len(disk_files["ev"]) + len(disk_files["ev_mono"]),
        "referenced": len(all_cg_script_refs),
        "unused": unused_cg,
    }

    # --- Thumbnails ---
    thumb_stems = {Path(f).stem for f in disk_files["thumbnail"]}
    # Thumbnail naming: eve_0101.png → matches CG eve_010101.png
    unused_thumbs = []
    for t in sorted(thumb_stems):
        if not any(cg.startswith(t) for cg in all_cg_script_refs):
            unused_thumbs.append(t)
    report["thumbnail"] = {
        "on_disk": len(thumb_stems),
        "unused": unused_thumbs,
    }

    # --- FG ---
    fg_refs = refs["fg"]
    fg_ids_on_disk = set()
    for folder, files in disk_files["fg"].items():
        for f in files:
            m = re.match(r"tati_(\d+)\.png", f)
            if m:
                fg_ids_on_disk.add(m.group(1))
    unused_fg = sorted(fg_ids_on_disk - fg_refs)
    report["fg"] = {
        "on_disk": len(fg_ids_on_disk),
        "referenced": len(fg_refs),
        "unused": unused_fg,
    }

    # --- Face ---
    face_refs = refs["face"]
    face_ids_on_disk = set()
    for f in disk_files["face"]:
        m = re.match(r"face_(\d+)\.png", f)
        if m:
            face_ids_on_disk.add(m.group(1))
    unused_face = sorted(face_ids_on_disk - face_refs)
    report["face"] = {
        "on_disk": len(face_ids_on_disk),
        "referenced": len(face_refs),
        "unused": unused_face,
    }

    # --- obj_index sprites ---
    sprite_refs = refs["sprite"]
    anime_refs = refs["anime"]
    bg_refs = refs["bg"]

    unused_entries = []
    used_paths = set()
    for name, path in obj_index.items():
        if name in sprite_refs or name in anime_refs or name in bg_refs:
            used_paths.add(path)
        else:
            unused_entries.append(name)

    # Orphan obj files (on disk but not in any obj_index entry)
    obj_files_on_disk = set()
    for subdir, fnames in disk_files["obj"].items():
        for f in fnames:
            obj_files_on_disk.add(f"image/obj/{subdir}/{f}")
    unused_obj_files = sorted(obj_files_on_disk - set(obj_index.values()))

    report["obj_index"] = {
        "entries": len(obj_index),
        "unused_entries": sorted(unused_entries),
        "unused_files": unused_obj_files,
    }

    # --- Anime files on disk ---
    # Anime files are named {base}_{NN}.png where base is the AnimateSprite file ref.
    # Derive base by stripping last _{digits} before .png
    anime_base_from_disk = set()
    for f in disk_files["anime"]:
        m = re.match(r"(.+)_\d+\.png", f)
        if m:
            anime_base_from_disk.add(m.group(1))
    # But anime refs may resolve via obj_index; also check disk directly
    unused_anime = sorted(anime_base_from_disk - anime_refs)
    report["anime"] = {
        "on_disk_frames": len(disk_files["anime"]),
        "on_disk_bases": len(anime_base_from_disk),
        "referenced": len(anime_refs),
        "unused_bases": unused_anime,
    }

    # --- Direct BG files (not in obj_index) ---
    bg_skipped = {"NULL", "nextsc_03", "nextsc_04"}  # special/sentinel values
    bg_direct_refs = set()
    for bg in bg_refs:
        if bg in obj_index or bg in bg_skipped:
            continue
        stem = Path(bg).stem  # strips .jpg/.png
        bg_direct_refs.add(stem)
    bg_stems_on_disk = {Path(f).stem for f in disk_files["bg"]}
    unused_bg = sorted(bg_stems_on_disk - bg_direct_refs)
    report["bg_direct"] = {
        "on_disk": len(bg_stems_on_disk),
        "referenced_scripts": len(bg_refs),
        "referenced_direct": len(bg_direct_refs),
        "unused": unused_bg,
    }

    # --- Movie files ---
    # PlayMovie: map via map_video_file (.mpg → .ogv)
    playmovie_stems = set()
    for f in refs["playmovie"]:
        playmovie_stems.add(Path(map_video_file(f)).stem)

    # DrawSpriteEx: map via map_video_file
    drawspriteex_stems = set()
    for f in refs["drawspriteex"]:
        drawspriteex_stems.add(Path(map_video_file(f)).stem)

    # RainMja: file appended with .ogv
    rain_stems = refs["rain_file"]

    all_movie_ref_stems = playmovie_stems | drawspriteex_stems | rain_stems

    movie_stems_on_disk = {Path(f).stem for f in disk_files["movie"]}
    unused_movie = sorted(movie_stems_on_disk - all_movie_ref_stems)
    missing_movie = sorted(all_movie_ref_stems - movie_stems_on_disk)

    bonus_explain = {}
    for s in sorted(all_movie_ref_stems):
        if s in playmovie_stems and s in drawspriteex_stems:
            bonus_explain[s] = "PlayMovie+DrawSpriteEx"
        elif s in playmovie_stems:
            bonus_explain[s] = "PlayMovie"
        elif s in drawspriteex_stems:
            bonus_explain[s] = "DrawSpriteEx"
        elif s in rain_stems:
            bonus_explain[s] = "RainMja"

    report["movie"] = {
        "on_disk": len(movie_stems_on_disk),
        "referenced_stems": len(all_movie_ref_stems),
        "unused": unused_movie,
        "missing": missing_movie,
        "ref_breakdown": bonus_explain,
    }

    return report


# ── Formatting ────────────────────────────────────────────────────────

def fmt_bytes(n):
    for unit in ['B', 'KB', 'MB', 'GB']:
        if n < 1024:
            return f"{n:.1f}{unit}"
        n /= 1024
    return f"{n:.1f}TB"


def file_size(base, *parts):
    p = base.joinpath(*parts)
    return p.stat().st_size if p.exists() else 0


def print_report(report, assets_dir, obj_index, show_all=True, sizes=True):
    def heading(t):
        print(f"\n{'='*70}\n  {t}\n{'='*70}")

    # ── Summary ──
    heading("SUMMARY")
    print(f"  {'Category':<22} {'On Disk':>10} {'Referenced':>12} {'Unused':>10}")
    print(f"  {'-'*56}")
    rows = [
        ("bgm", "bgm", "on_disk_ids"),
        ("se (stems)", "se", "on_disk_stems"),
        ("voice", "voice", "on_disk"),
        ("cg", "cg", "on_disk"),
        ("fg", "fg", "on_disk"),
        ("face", "face", "on_disk"),
        ("anime (bases)", "anime", "on_disk_bases"),
        ("movie (stems)", "movie", "on_disk"),
    ]
    for label, key, count_key in rows:
        r = report.get(key, {})
        print(f"  {label:<22} {r.get(count_key, 0):>10} {r.get('referenced', '-'):>12} {len(r.get('unused', [])):>10}")
    # Special: obj_index
    r = report.get("obj_index", {})
    print(f"  {'obj_index entries':<22} {r.get('entries', 0):>10} {'-':>12} {len(r.get('unused_entries', [])):>10}")
    # Fix bg_direct referenced count
    r = report.get("bg_direct", {})
    print(f"  {'bg (direct)':<22} {r.get('on_disk', 0):>10} {r.get('referenced_direct', '-'):>12} {len(r.get('unused', [])):>10}")
    print(f"  {'thumbnail':<22} {report['thumbnail'].get('on_disk', 0):>10} {'-':>12} {len(report['thumbnail'].get('unused', [])):>10}")

    # ── Size estimate ──
    if sizes:
        heading("ESTIMATED SPACE FROM UNUSED ASSETS")
        waste = {}

        def add_waste(cat, paths):
            total = 0
            for p in paths:
                f = assets_dir.joinpath(*p)
                if f.exists():
                    total += f.stat().st_size
            if total:
                waste[cat] = total

        add_waste("bgm", [("audio/bgm", f) for id_ in report["bgm"]["unused"]
                          for f in (f"bgm_{id_}_a.ogg", f"bgm_{id_}_b.ogg", f"bgm_{id_}.ogg")])

        add_waste("se", [("audio/se", f) for stem in report["se"]["unused"]
                         for f in (f"{stem}.ogg", f"{stem}_a.ogg", f"{stem}_b.ogg")])

        add_waste("voice", [("audio/voice", f"{v}.ogg") for v in report["voice"]["unused"]])

        add_waste("cg", [("image/ev", f"{stem}.png") for stem in report["cg"]["unused"] if "/" not in stem])

        add_waste("thumbnail", [("image/thumbnail", f"{t}.png") for t in report["thumbnail"]["unused"]])

        if waste:
            print(f"\n  {'Category':<22} {'Wasted':>15}")
            print(f"  {'-'*40}")
            total = 0
            for cat in sorted(waste):
                print(f"  {cat:<22} {fmt_bytes(waste[cat]):>15}")
                total += waste[cat]
            print(f"  {'-'*40}")
            print(f"  {'TOTAL':<22} {fmt_bytes(total):>15}")

    if not show_all:
        return

    # ── Detail ──
    heading("DETAILED BREAKDOWN")

    def sub(t):
        print(f"\n  ▶ {t}")

    def items(lst, max_show=200):
        if not lst:
            print("    (none)")
            return
        for x in lst[:max_show]:
            print(f"    {x}")
        if len(lst) > max_show:
            print(f"    ... and {len(lst) - max_show} more")

    for cat, label in [("bgm", "Unused BGM IDs"), ("se", "Unused SE stems"),
                        ("fg", "Unused FG char_ids"), ("face", "Unused face char_ids"),
                        ("bg_direct", "Unused background files (direct)"),
                        ("cg", "Unused CG files"), ("anime", "Unused anime bases")]:
        r = report.get(cat, {})
        u = r.get("unused", [])
        if u:
            sub(f"{label} ({len(u)})")
            items(u)

    r = report.get("obj_index", {})
    if r.get("unused_entries"):
        sub(f"Unused obj_index entries ({len(r['unused_entries'])})")
        for name in r["unused_entries"][:200]:
            print(f"    {name}  →  {obj_index.get(name, '?')}")
        if len(r["unused_entries"]) > 200:
            print(f"    ... and {len(r['unused_entries']) - 200} more")

    if r.get("unused_files"):
        sub(f"Orphan obj files (not in any index) ({len(r['unused_files'])})")
        items(r["unused_files"])

    r = report.get("thumbnail", {})
    if r.get("unused"):
        sub(f"Unused thumbnails ({len(r['unused'])})")
        items(r["unused"], 100)

    r = report.get("movie", {})
    if r.get("unused"):
        sub(f"Unused movie files ({len(r['unused'])})")
        items(r["unused"])
    if r.get("missing"):
        sub(f"Missing movie files (referenced but not found) ({len(r['missing'])})")
        for m in r["missing"]:
            how = r.get("ref_breakdown", {}).get(m, "")
            tag = f"  ({how})" if how else ""
            print(f"    {m}{tag}")

    r = report.get("voice", {})
    if r.get("unused"):
        sub(f"Unused voice files ({len(r['unused'])})")
        items(r["unused"], 30)


# ── Deletion ──────────────────────────────────────────────────────────

def _del(path, dry_run, label=""):
    """Delete a file (or report it in dry-run mode)."""
    if not path.exists():
        return
    tag = f"  [{label}]" if label else ""
    if dry_run:
        print(f"  [DRY-RUN] would delete: {path}{tag}")
    else:
        print(f"  deleting: {path}{tag}")
        path.unlink()


def _files(rel_parts):
    """Yield absolute Paths for each relative path list."""
    for parts in rel_parts:
        yield ASSETS.joinpath(*parts)


def delete_unused(report, assets_dir, dry_run, categories=None):
    """Delete unreferenced files per category. Returns total bytes freed."""
    total = 0

    def delete_category(category, paths):
        nonlocal total
        if categories is not None and category not in categories:
            return
        count = 0
        for p in paths:
            if p.exists():
                total += p.stat().st_size
                _del(p, dry_run, category)
                count += 1
        if count:
            print(f"  [{category}] {count} file{'s' if count != 1 else ''} removed")

    # Voice
    voice_paths = [ASSETS / "audio/voice" / f"{v}.ogg" for v in report["voice"]["unused"]]
    delete_category("voice", voice_paths)

    # BGM - try _a/_b split, then single file
    bgm_paths = []
    for id_ in report["bgm"]["unused"]:
        a = ASSETS / "audio/bgm" / f"bgm_{id_}_a.ogg"
        b = ASSETS / "audio/bgm" / f"bgm_{id_}_b.ogg"
        single = ASSETS / "audio/bgm" / f"bgm_{id_}.ogg"
        bgm_paths.extend(p for p in (a, b, single) if p.exists())
    delete_category("bgm", bgm_paths)

    # SE - try stem alone, then _a/_b split
    se_paths = []
    for stem in report["se"]["unused"]:
        single = ASSETS / "audio/se" / f"{stem}.ogg"
        a = ASSETS / "audio/se" / f"{stem}_a.ogg"
        b = ASSETS / "audio/se" / f"{stem}_b.ogg"
        se_paths.extend(p for p in (single, a, b) if p.exists())
    delete_category("se", se_paths)

    # CG
    cg_paths = []
    ev_dir = ASSETS / "image/ev"
    for stem in report["cg"]["unused"]:
        if "/" in stem:
            sub, name = stem.split("/", 1)
            for ext in [".png", ".jpg", ".jpeg"]:
                p = ev_dir / sub / f"{name}{ext}"
                if p.exists():
                    cg_paths.append(p)
                    break
        else:
            for ext in [".png", ".jpg", ".jpeg"]:
                p = ev_dir / f"{stem}{ext}"
                if p.exists():
                    cg_paths.append(p)
                    break
    delete_category("cg", cg_paths)

    # Thumbnails
    thumb_paths = [ASSETS / "image/thumbnail" / f"{t}.png" for t in report["thumbnail"]["unused"]]
    delete_category("thumbnail", thumb_paths)

    # BG direct
    bg_dir = ASSETS / "image/bg"
    bg_paths = []
    for stem in report["bg_direct"]["unused"]:
        for ext in [".jpg", ".png", ".jpeg"]:
            p = bg_dir / f"{stem}{ext}"
            if p.exists():
                bg_paths.append(p)
                break
    delete_category("bg", bg_paths)

    # Movie
    movie_paths = [ASSETS / "movie" / f"{stem}.ogv" for stem in report["movie"]["unused"]]
    delete_category("movie", movie_paths)

    print(f"\n  Total freed: {fmt_bytes(total)}")
    return total


def main():
    parser = argparse.ArgumentParser(description="Tree-shake unused assets for bevy-vn")
    parser.add_argument("--voice-detail", action="store_true", help="Show all unused voice files")
    parser.add_argument("--unreferenced-only", action="store_true", help="Only list unused items")
    parser.add_argument("--json", action="store_true", help="Output JSON")
    parser.add_argument("--delete", nargs="*", default=None,
                        help="Delete unused files. Specify categories or omit for all: "
                             "voice bgm se cg thumbnail bg movie")
    parser.add_argument("--dry-run", nargs="*", default=None,
                        help="Dry-run: show what would be deleted without deleting. "
                             "Optionally specify categories as arguments, e.g. --dry-run bgm voice")
    args = parser.parse_args()

    script_dir = ASSETS / "scripts"
    if not script_dir.exists():
        print(f"Error: {script_dir} not found", file=sys.stderr)
        sys.exit(1)

    print("Scanning scripts...", file=sys.stderr)
    refs = extract_refs(script_dir)

    print("Parsing obj_index.ron...", file=sys.stderr)
    obj_index = parse_obj_index(script_dir / "obj_index.ron")

    print("Scanning asset directories...", file=sys.stderr)
    disk_files = scan_assets()

    print("Cross-referencing...\n", file=sys.stderr)
    report = analyze(refs, obj_index, disk_files)

    do_delete = args.delete is not None or args.dry_run is not None
    if do_delete:
        valid_cats = {"voice", "bgm", "se", "cg", "thumbnail", "bg", "movie"}
        cats = set(args.delete or []) | set(args.dry_run or [])
        if not cats:
            cats = valid_cats
        bad = cats - valid_cats
        if bad:
            print(f"Unknown categories: {sorted(bad)}. Valid: {sorted(valid_cats)}", file=sys.stderr)
            sys.exit(1)
        dry_run = args.dry_run is not None
        if dry_run:
            print("\n=== DRY RUN — no files will be deleted ===\n")
        else:
            print("\n=== DELETING UNUSED FILES ===\n")
        delete_unused(report, ASSETS, dry_run, cats)
        return

    if args.json:
        print(json.dumps(report, indent=2, default=str))
    else:
        print_report(report, ASSETS, obj_index, show_all=not args.unreferenced_only)

    # Total
    count = sum(len(report.get(c, {}).get("unused", []))
                for c in ["bgm", "se", "voice", "cg", "fg", "face", "bg_direct",
                          "anime", "obj_index", "thumbnail", "movie"])
    print(f"\nTotal unused items: {count}")
    if report["movie"]["missing"]:
        print(f"Missing movie files: {len(report['movie']['missing'])}")


if __name__ == "__main__":
    main()
