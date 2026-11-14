#!/usr/bin/env bash
# Write the desktop-only source of the three programs into sources/ (or OUT_DIR): the files of the commits pinned in
# packaging/sources.json, never git history, without the web-only and private parts.
#
#   packaging/export-sources.sh [OUT_DIR]
#
# Each program is read with `git archive <pinned ref>` from a clone that has that commit (the working tree is not
# read, so local edits never leak). Clones default to siblings of this repo; KHIANKRUB_SRC, KHITKRUB_SRC and
# PHUTKRUB_SRC override (the release workflow passes its checkouts). Needs git, python3 and cargo (Cargo.lock is
# brought in line with the smaller workspace by `cargo update --workspace`, which changes no dependency version).
#
# Rule (owner, 2026-11-14): desktop source public, web-only source private. What is left out:
#   all three   apps/*-web (the web app), packaging/web (site, publish scripts, Facebook and release drafts),
#               HANDOFF*.md (working notes), the owner's rules in CLAUDE.md
#   KhianKrub   apps/khiankrub-drive-server, apps/khiankrub-drive-worker (Drive backends), NEW-MAC.md,
#               loading-*.html (drafts)
# Personal details are removed from every program: personal email addresses (the public support address
# kruboffice@gmail.com stays) and the author fields of Office documents.
# The script fails if anything it should have removed is still there, or if a file looks like a secret.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SUITE_ROOT="$(cd "$ROOT/.." && pwd)"
OUT="${1:-$ROOT/sources}"
SOURCES_JSON="$ROOT/packaging/sources.json"

# Per program: clone, display name, upstream, web app crate, extra paths to drop (bash 3.2 has no associative arrays).
src_of() { case "$1" in khiankrub) echo "${KHIANKRUB_SRC:-$SUITE_ROOT/khiankrub}" ;; khitkrub) echo "${KHITKRUB_SRC:-$SUITE_ROOT/khitkrub}" ;; phutkrub) echo "${PHUTKRUB_SRC:-$SUITE_ROOT/phutkrub}" ;; esac; }
name_of() { case "$1" in khiankrub) echo "KhianKrub (เขียนครับ)" ;; khitkrub) echo "KhitKrub (คิดครับ)" ;; phutkrub) echo "PhutKrub (พูดครับ)" ;; esac; }
upstream_of() { case "$1" in khiankrub) echo storytold/wordcraft ;; khitkrub) echo storytold/gridcraft ;; phutkrub) echo storytold/deckcraft ;; esac; }
web_of() { case "$1" in khiankrub) echo wordcraft-web ;; khitkrub) echo gridcraft-web ;; phutkrub) echo deckcraft-web ;; esac; }
bin_of() { case "$1" in khiankrub) echo wordcraft ;; khitkrub) echo gridcraft ;; phutkrub) echo deckcraft ;; esac; }
# Removed from every program: the web app, packaging/web, the handoff notes; then per program.
COMMON_DROP="packaging/web HANDOFF*.md"
extra_drop_of() { case "$1" in khiankrub) echo "apps/khiankrub-drive-server apps/khiankrub-drive-worker NEW-MAC.md loading-*.html" ;; *) echo "" ;; esac; }

json() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]][sys.argv[3]])' "$SOURCES_JSON" "$1" "$2"; }

mkdir -p "$OUT"
for app in khiankrub khitkrub phutkrub; do
  src="$(src_of "$app")"; web="$(web_of "$app")"
  ref="$(json "$app" ref)"
  version="$(json "$app" version)"
  dest="$OUT/$app"
  echo "== $app $version ($ref) from $src"
  git -C "$src" cat-file -e "$ref^{commit}" || { echo "error: $src does not have commit $ref" >&2; exit 1; }
  rm -rf "$dest"
  mkdir -p "$dest"
  git -C "$src" archive --format=tar "$ref" | tar -x -C "$dest"

  (
    cd "$dest"
    shopt -s nullglob
    # shellcheck disable=SC2086 # the globs are meant to expand here
    for p in apps/"$web" $COMMON_DROP $(extra_drop_of "$app"); do rm -rf -- "$p"; done
  )

  # The owner's private rules in CLAUDE.md, and the web app in the layer test's list of exempt apps.
  python3 - "$app" "$dest" "$web" <<'PY'
import re, sys
app, dest, web = sys.argv[1:4]

def edit(path, fn):
    p = f"{dest}/{path}"
    try:
        text = open(p, encoding="utf-8").read()
    except FileNotFoundError:
        return
    new = fn(text)
    if new != text:
        open(p, "w", encoding="utf-8").write(new)

def drop_section(text, heading):
    # From `## heading` up to the next heading of the same or a higher level.
    return re.sub(r"(?ms)^## " + re.escape(heading) + r"\n.*?(?=^#{1,2} )", "", text)

def drop_owner_lines(text):
    # List items that carry an owner's rule or point at the handoff notes.
    keep = []
    skipping = False
    for line in text.splitlines(keepends=True):
        if line.startswith("- "):
            skipping = bool(re.search(r"\(owner, \d{4}-\d\d-\d\d\)|HANDOFF|the owner works with", line))
        elif not line.startswith("  "):
            skipping = False
        if not skipping:
            keep.append(line)
    text = "".join(keep)
    # The owner's checks rule written into a paragraph of upstream text (KhianKrub's Quality gates).
    text = re.sub(r"\*\*Checks before a commit \(owner, [^)]*\)[^*]*\*\*.*?Say in `HANDOFF\.md` which checks ran\. ", "", text)
    # A paragraph that is one owner's rule.
    text = re.sub(r"(?m)^\*\*[^*\n]*\(owner, \d{4}-\d\d-\d\d\)[^*\n]*\*\*.*\n\n?", "", text)
    return text

def claude_md(text):
    if app == "khiankrub":
        text = drop_section(text, "KrubOffice READMEs and handoff (owner's rules)")
    elif app == "khitkrub":
        # KhitKrub's own working notes come first, then the upstream (GridCraft) instructions.
        i = text.find("# GridCraft — instructions for agents")
        if i > 0:
            text = text[i:]
    elif app == "phutkrub":
        # The opening paragraph is about the private web fork (remotes, "web only").
        text = re.sub(r"(?ms)^PhutKrub is Chanon Sarasuk's.*?\n\n", "", text, count=1)
    return drop_owner_lines(text)

edit("CLAUDE.md", claude_md)
if app == "phutkrub":
    # Points at the owner's checks rule, which is not here.
    edit("CLAUDE.md", lambda t: t.replace("2. Don't run the full `cargo xtask ci` just to start; see the checks rule under Habits.\n", ""))
# A personal email address in a change-log note.
edit("FORK-CHANGES.md", lambda t: re.sub(r"; commits to the public site use `[^`\s]+@[^`\s]+`", "", t))
edit("xtask/src/layers.rs", lambda t: t.replace(f', "{web}", "xtask"]', ', "xtask"]'))

# Personal details (owner, 2026-11-14): personal email addresses, and the author fields of Office documents.
# kruboffice@gmail.com is the apps' public support address and stays.
import os, zipfile
PUBLIC_EMAILS = {"kruboffice@gmail.com"}
personal_email = re.compile(r"\b[A-Za-z0-9._%+-]+@(?:gmail\.com|rmutk\.ac\.th|icloud\.com|mac\.com|me\.com|hotmail\.com|outlook\.com|yahoo\.com)\b")
edit("crates/ui-egui/src/report.rs", lambda t: re.sub(r" \(personal Google account [^)]*\)", "", t))
for root, _, files in os.walk(dest):
    for fn in files:
        p = os.path.join(root, fn)
        if fn.lower().endswith((".docx", ".xlsx", ".pptx", ".dotx", ".xltx", ".potx")):
            with zipfile.ZipFile(p) as z:
                items = [(i, z.read(i.filename)) for i in z.infolist()]
            changed = False
            out = []
            for info, data in items:
                if info.filename == "docProps/core.xml":
                    text = data.decode("utf-8")
                    new = re.sub(r"(<(dc:creator|cp:lastModifiedBy)>)[^<]*(</\2>)", r"\1KrubOffice\3", text)
                    if new != text:
                        data, changed = new.encode("utf-8"), True
                out.append((info, data))
            if changed:
                with zipfile.ZipFile(p, "w") as z:
                    for info, data in out:
                        z.writestr(info, data)
            continue
        try:
            text = open(p, encoding="utf-8").read()
        except (UnicodeDecodeError, OSError):
            continue
        new = personal_email.sub(lambda m: m.group(0) if m.group(0).lower() in PUBLIC_EMAILS else "(personal address removed)", text)
        if new != text:
            open(p, "w", encoding="utf-8").write(new)
PY

  # Cargo.lock without the removed packages; no other dependency changes.
  if ! cargo update --workspace --offline --manifest-path "$dest/Cargo.toml" 2>/dev/null; then
    cargo update --workspace --manifest-path "$dest/Cargo.toml"
  fi

  # Checks: nothing left that should have gone.
  left="$(cd "$dest" && { ls -d apps/*-web apps/*-drive-* packaging/web HANDOFF*.md NEW-MAC.md loading-*.html 2>/dev/null || true; })"
  [ -z "$left" ] || { echo "error: $app still has: $left" >&2; exit 1; }
  if grep -n -E "\(owner, [0-9]{4}-|owner's rules|HANDOFF" "$dest/CLAUDE.md"; then
    echo "error: $app CLAUDE.md still has the owner's notes" >&2; exit 1
  fi
  if grep -n -E "^name = \"($web|khiankrub-drive-server|khiankrub-drive-worker)\"" "$dest/Cargo.lock"; then
    echo "error: $app Cargo.lock still lists a removed package" >&2; exit 1
  fi
done

# Checks: nothing that looks like a secret or a local environment file.
bad_files="$(cd "$OUT" && find . \( -name '.env' -o -name '.env.*' -o -name '*.pem' -o -name '*.p12' -o -name '*.p8' \
  -o -name '*.key' -o -name 'id_rsa*' -o -name 'wrangler.toml' -o -name '.dev.vars' -o -name 'render.yaml' \) -print)"
[ -z "$bad_files" ] || { echo "error: files that may hold secrets: $bad_files" >&2; exit 1; }
if grep -r -I -n -E -- '-----BEGIN [A-Z ]*PRIVATE KEY-----|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22,}|xox[abpr]-[A-Za-z0-9-]{10,}|sk-[A-Za-z0-9]{32,}|AIza[0-9A-Za-z_-]{35}|GOCSPX-[A-Za-z0-9_-]{20,}' "$OUT"; then
  echo "error: something that looks like a key or token (above)" >&2; exit 1
fi
# Checks: no personal email address and no personal author in an Office document.
if grep -r -I -n -o -i -E '[A-Za-z0-9._%+-]+@(gmail\.com|rmutk\.ac\.th|icloud\.com|mac\.com|me\.com|hotmail\.com|outlook\.com|yahoo\.com)' "$OUT" | grep -v -i 'kruboffice@gmail\.com'; then
  echo "error: a personal email address is left (above)" >&2; exit 1
fi
if find "$OUT" -type f \( -iname '*.docx' -o -iname '*.xlsx' -o -iname '*.pptx' \) -exec unzip -p {} docProps/core.xml \; 2>/dev/null \
    | grep -i -E '<(dc:creator|cp:lastModifiedBy)>[^<]*(chanon|sarasuk)'; then
  echo "error: an Office document still names a person as its author (above)" >&2; exit 1
fi

# sources/README.md: where each tree comes from.
{
  echo "# Source code of the three programs · ซอร์สโค้ดของสามโปรแกรม"
  echo
  echo "Written by \`packaging/export-sources.sh\` at each KrubOffice release: the desktop source of the programs"
  echo "bundled in KrubOffice $(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1), the files only (no history). The web-only"
  echo "parts (the web app, the website and its publishing scripts) are not included."
  echo
  echo "ซอร์สโค้ดฉบับเดสก์ท็อปของสามโปรแกรมใน KrubOffice เขียนโดยอัตโนมัติทุกครั้งที่ออกรุ่นใหม่ ไม่รวมส่วนที่ใช้เฉพาะบนเว็บ"
  echo
  echo "| Folder | Program | Version | Commit | Based on |"
  echo "|---|---|---|---|---|"
  for app in khiankrub khitkrub phutkrub; do
    echo "| [\`$app/\`]($app/) | $(name_of "$app") | $(json "$app" version) | \`$(json "$app" ref | cut -c1-12)\` | [$(upstream_of "$app")](https://github.com/$(upstream_of "$app")) |"
  done
  echo
  echo "Each folder is a Cargo workspace (MIT OR Apache-2.0, see its \`LICENSE-*\`, \`NOTICE\` and \`ATTRIBUTION.md\`)."
  echo "Fonts: release builds embed [storytold/craft-fonts](https://github.com/storytold/craft-fonts) at"
  echo "\`$(json craft_fonts ref | cut -c1-12)\` (optional: set \`CRAFT_FONTS_DIR\` to an absolute path of a checkout). To build one program:"
  echo
  echo '```bash'
  echo "cd sources/khiankrub && cargo build --release -p $(bin_of khiankrub)"
  echo '```'
  echo
  echo "(\`-p $(bin_of khitkrub)\` in \`khitkrub/\`, \`-p $(bin_of phutkrub)\` in \`phutkrub/\`.) To build all of KrubOffice, see the main README."
} > "$OUT/README.md"

du -sh "$OUT"/*/
