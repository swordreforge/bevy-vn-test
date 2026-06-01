"""
OCR BGM title images (musname_XXX.png) → extract Japanese/English titles.

The decorative serif font is hard for Tesseract. This script:
  1. Tries OCR (with aggressive preprocessing)
  2. Fuzzy-matches against the official OST track list
  3. Falls back to a manually verified override map
  4. Builds a deduplicated bidirectional mapping (title ↔ [BGM IDs])

Outputs:
  - assets/scripts/bgm_index.ron   (RON for the engine)
  - tools/bgm_title_map.json       (full data for manual verification)
  - tools/ocr_debug/*.png          (preprocessed images for inspection)

Manual fix: edit OVERRIDE_MAP below, re-run.
"""

import json
import re
import subprocess
import sys
from difflib import SequenceMatcher
from pathlib import Path

try:
    import pytesseract
    from PIL import Image, ImageFilter, ImageOps
except ImportError:
    print("pip install pytesseract Pillow")
    sys.exit(1)
try:
    subprocess.run(["tesseract", "--version"], capture_output=True)
except FileNotFoundError:
    print("tesseract not found. Install: sudo pacman -S tesseract tesseract-data-jpn tesseract-data-eng")
    sys.exit(1)

ASSETS_DIR = Path(__file__).resolve().parent.parent / "assets"
MUSNAME_DIR = ASSETS_DIR / "image" / "obj" / "bgmname"
OUTPUT_RON = ASSETS_DIR / "scripts" / "bgm_index.ron"
OUTPUT_JSON = Path(__file__).resolve().parent / "bgm_title_map.json"
DEBUG_DIR = Path(__file__).resolve().parent / "ocr_debug"
DEBUG_DIR.mkdir(exist_ok=True)

# ── Official OST track list (complete) ──
OST_TRACKS: list[str] = [
    "Abgrund", "Asphodelus short ver.", "One of Episodes", "Blind Alley",
    "Ash", "Halbmond", "Crawler", "Escualo", "Inertia", "Amaranth", "Kanon",
    "Crossandra", "Far Afield", "Una Atadura", "Saint Twinkle", "Innocence",
    "Rain", "Adagietta", "Phalaenopsis", "Luce di Luna", "Blood Stain",
    "Schranz Shot", "Heavy Strokes", "Dot Brain", "Reflections",
    "Wave & Dream", "Not in Time", "Deepness", "The G Plot", "Stairs",
    "The Wind, the Cloud, the Earth", "Plume", "Lira", "Musa", "Cross",
    "Bless", "Images", "Past", "Close My Eyes", "Tears of Hope short ver.",
    "Distorted", "Die Macht", "In the Spiral", "Refreblue", "Roots",
    "Repulsion", "La Rosa", "Ascension", "Around Flower", "To the dear world",
    "Tears of Hope", "Asphodelus", "Shape of the Absurd",
]

# ── Manual override map (verified: for each BGM ID, the correct title) ──
# Fill this in after reviewing OCR output + OST track list.
# Empty string = use OCR/fuzzy; non-empty = force this title.
OVERRIDE_MAP: dict[str, str] = {
    # 02xx — Disc 2
    "0201": "Far Afield",
    "0202": "Una Atadura",
    "0203": "Saint Twinkle",
    "0204": "Innocence",
    "0205": "Rain",
    "0206": "Phalaenopsis",    # OCR: Phalasnoosls → Disc 2-7
    "0208": "Adagietta",       # OCR: Adauisiig  → Disc 2-6 (no 0207)
    # 03xx — Disc 1 tracks (selection)
    "0301": "Blind Alley",     # OCR ~Blind Alley → Disc 1-4
    "0302": "Ash",             # OCR ~Ash → Disc 1-5
    "0303": "Halbmond",        # OCR ~Halbmond → Disc 1-6
    "0304": "Crawler",         # OCR ~Crawler → Disc 1-7
    "0305": "Escualo",         # OCR ? → Disc 1-8
    "0306": "Inertia",         # OCR ~Inertia → Disc 1-9
    "0307": "Luce di Luna",    # verified by user → Disc 2-8
    "0308": "Kanon",           # OCR ~Kanon → Disc 1-11
    "0309": "Amaranth",        # OCR ~Amaranth → Disc 1-10
    "0310": "Crossandra",      # OCR ~Crossandra → Disc 1-12
    "0311": "Die Macht",       # OCR ~Die Macht → Disc 4-3
    # 04xx — mixed selection from various discs
    "0401": "One of Episodes",  # OCR: Ons of Bolsodsy → Disc 1-3
    "0402": "Blood Stain",      # OCR: Blood Statn → Disc 2-9
    "0403": "Schranz Shot",     # OCR: Sehranz Shot → Disc 2-10
    "0404": "Dot Brain",        # OCR: Dot Braln → Disc 2-12
    "0405": "Reflections",      # OCR: リフレクション/Reflection-ish → Disc 3-1
    "0407": "Wave & Dream",     # OCR: Ways & Diguin → Disc 3-2
    "0408": "Not in Time",      # OCR: Notin Thns/Wotin Time → Disc 3-3
    "0409": "Deepness",         # OCR: Dagunsss/デ ィ ー プ テ ネ ス → Disc 3-4
    "0410": "Heavy Strokes",    # OCR: ヘ ケ ィ ー ス ト ロ ー ク → Disc 2-11
    "0411": "Stairs",           # OCR: Sinirs → Disc 3-6
    "0412": "Plume",            # OCR: Phiing/Plmma → Disc 3-8
    "0413": "In the Spiral",    # OCR: n the Soira → Disc 4-4
    "0414": "The Wind, the Cloud, the Earth",  # OCR: The Wind ihe Cloudl → Disc 3-7
    "0415": "Cross",            # OCR: Cross → Disc 3-11
    "0416": "Bless",            # OCR: Blsogs/Blegs → Disc 3-12
    "0417": "Images",           # OCR: Tinnoss/fmages → Disc 3-13
    "0418": "Past",             # OCR: Pugt/Pagt → Disc 3-14
    "0419": "Shape of the Absurd",  # OCR: Shuavs/Shaps offha → Disc 4-14
    "0420": "Ascension",        # OCR: Ageanslon/Agesngion → Disc 4-9
    "0421": "Around Flower",    # OCR: Arvonnd Flowse/Arommd gloyyar → Disc 4-10
    # 05xx — Disc 4 & Disc 3
    "0501": "The G Plot",       # OCR: The び Plot → Disc 3-5
    "0502": "Distorted",        # OCR: Distorisd → Disc 4-2
    "0503": "Roots",            # OCR: Rooig/Rools → Disc 4-6
    "0504": "La Rosa",          # OCR: TaRosn/Ta Roaa → Disc 4-8
    "0505": "Roots",            # verified by user → Disc 4-6 (title image shows "Roots / Largo ver")
    "0506": "Repulsion",        # verified by user → Disc 4-7
    "0507": "Refreblue",        # verified by user → Disc 4-5
}


def bgm_id_from_filename(fname: str) -> str | None:
    m = re.match(r"musname_(\d+)\.png$", fname)
    if not m:
        return None
    num = m.group(1)
    if len(num) == 1:
        return "0" + num
    if len(num) == 3:
        return "0" + num
    return num


def preprocess(img: Image.Image) -> Image.Image:
    if img.mode == "RGBA":
        bg = Image.new("RGBA", img.size, (255, 255, 255, 255))
        img = Image.alpha_composite(bg, img)
    img = img.convert("L")
    w, h = img.size
    img = img.resize((w * 3, h * 3), Image.LANCZOS)
    for _ in range(3):
        img = img.filter(ImageFilter.SHARPEN)
    img = img.point(lambda p: 255 if p > 140 else 0)
    pix = list(img.getdata())
    avg = sum(pix) / len(pix)
    return ImageOps.invert(img) if avg < 128 else img


def ocr_image(img_path: Path) -> str:
    processed = preprocess(Image.open(img_path))
    processed.save(DEBUG_DIR / img_path.name)
    for lang, psm in [("eng", 7), ("jpn+eng", 6)]:
        text = pytesseract.image_to_string(processed, config=f"--oem 3 --psm {psm} -l {lang}")
        lines = [l.strip() for l in text.splitlines() if l.strip()]
        if lines:
            return lines[0]
    return ""


def fuzzy_best(text: str) -> tuple[str, float]:
    best, best_score = "", 0.0
    for c in OST_TRACKS:
        score = SequenceMatcher(None, text.lower(), c.lower()).ratio()
        if c.lower() in text.lower() or text.lower() in c.lower():
            score = max(score, 0.85)
        if score > best_score:
            best_score, best = score, c
    return best, best_score


def main():
    png_files = sorted(MUSNAME_DIR.glob("musname_*.png"))
    print(f"Found {len(png_files)} title images\n")

    results: list[dict] = []
    for img_path in png_files:
        bgm_id = bgm_id_from_filename(img_path.name)
        if bgm_id is None:
            continue

        ocr_text = ocr_image(img_path)

        if bgm_id in OVERRIDE_MAP and OVERRIDE_MAP[bgm_id]:
            title, src, score = OVERRIDE_MAP[bgm_id], "override", 1.0
        elif ocr_text:
            title, score = fuzzy_best(ocr_text)
            src = f"fuzzy({score:.0%})" if score >= 0.5 else "ocr_raw"
            if score < 0.5:
                title = ocr_text
        else:
            title, src, score = f"BGM {bgm_id}", "fallback", 0.0

        results.append({"bgm_id": bgm_id, "file": img_path.name, "title": title,
                        "ocr_raw": ocr_text, "src": src, "score": score})

        flag = "✓" if score >= 0.8 else ("?" if score >= 0.5 else "✗")
        print(f"  {flag} BGM {bgm_id:>4s}  {title:45s}  ({src})")

    # ── Dedup check ──
    id_to_title = {r["bgm_id"]: r["title"] for r in results}
    title_to_ids: dict[str, list[str]] = {}
    for r in results:
        title_to_ids.setdefault(r["title"], []).append(r["bgm_id"])

    dups = {t: ids for t, ids in title_to_ids.items() if len(ids) > 1}
    if dups:
        print(f"\n⚠  {len(dups)} duplicate title(s) — same title for multiple BGM IDs:")
        for title, ids in sorted(dups.items()):
            print(f"    \"{title}\" → {ids}")

    # ── JSON ──
    json_output = {"results": results, "id_to_title": id_to_title,
                   "title_to_ids": title_to_ids,
                   "unmatched_no_override": [r for r in results if r["src"] == "ocr_raw"]}
    OUTPUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_JSON, "w", encoding="utf-8") as f:
        json.dump(json_output, f, ensure_ascii=False, indent=2)
    print(f"\nWrote {OUTPUT_JSON}")

    # ── RON ──
    ron = "(\n" + "\n".join(
        f'    (id: "{bgm_id}", title: "{title}"),'
        for bgm_id, title in sorted(id_to_title.items())
    ) + "\n)\n"
    with open(OUTPUT_RON, "w", encoding="utf-8") as f:
        f.write(ron)
    print(f"Wrote {OUTPUT_RON}")

    # ── Summary ──
    counts = {"override": 0, "high": 0, "medium": 0, "low": 0, "fallback": 0}
    for r in results:
        if r["src"] == "override":          counts["override"] += 1
        elif r["score"] >= 0.8:             counts["high"] += 1
        elif r["score"] >= 0.5:             counts["medium"] += 1
        elif r["src"] == "ocr_raw":         counts["low"] += 1
        else:                               counts["fallback"] += 1
    print(f"\n{'Done.':15s} {len(results)} total")
    print(f"{'Override:':15s} {counts['override']}")
    print(f"{'OCR high:':15s} {counts['high']}")
    print(f"{'OCR medium:':15s} {counts['medium']}")
    print(f"{'OCR low (needs review):':15s} {counts['low']}")
    print(f"{'Fallback:':15s} {counts['fallback']}")

    low = [r for r in results if r["src"] == "ocr_raw"]
    if low:
        print(f"\n── Manual review needed ({len(low)}) ──")
        for r in low:
            print(f"  BGM {r['bgm_id']}: OCR={r['ocr_raw']!r}")
        print("\nEdit OVERRIDE_MAP in this script, set the correct title, re-run.")


if __name__ == "__main__":
    main()
