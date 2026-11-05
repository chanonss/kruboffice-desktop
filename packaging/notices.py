"""THIRD-PARTY-NOTICES.txt: the licences and copyright notices of the open-source parts in a KrubOffice build.

KrubOffice is open source (MIT OR Apache-2.0, LICENSE-MIT and LICENSE-APACHE). The programs inside it are built
from open-source work (WordCraft, GridCraft and DeckCraft by the ArtCraft team, many Rust libraries, fonts),
whose licences (MIT, Apache-2.0, BSD, Zlib, SIL OFL …) ask that their notices travel with every copy.
This file collects them.

    python3 -I packaging/notices.py OUT.txt TARGET  WORKSPACE:PACKAGE [WORKSPACE:PACKAGE …]  [--fonts DIR …]

For each WORKSPACE (a Cargo workspace) it asks `cargo metadata` for the dependency graph on TARGET (a
Rust target triple), walks the normal (shipped) dependencies of PACKAGE, and copies every crate's licence
files (LICENSE*, LICENCE*, COPYING*, NOTICE*, COPYRIGHT*), falling back to the workspace's own for crates
that keep them only at the workspace root. Identical texts are printed once. `--fonts DIR` adds the
licence files of bundled fonts.
"""
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

NAMES = ("license", "licence", "copying", "notice", "copyright", "ofl", "unlicense")
MAX_FILE = 512 * 1024


def licence_files(directory):
    out = []
    try:
        entries = sorted(os.listdir(directory))
    except OSError:
        return out
    for name in entries:
        p = Path(directory) / name
        low = name.lower()
        if p.is_file() and any(low.startswith(n) for n in NAMES) and p.stat().st_size <= MAX_FILE:
            out.append(p)
    return out


def walk(workspace, package, target):
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--filter-platform", target, "--locked"],
        cwd=workspace, check=True, capture_output=True, text=True).stdout)
    pkgs = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = next((p["id"] for p in meta["packages"] if p["name"] == package and p["id"] in meta["workspace_members"]), None)
    if root is None:
        sys.exit(f"{package} is not a member of {workspace}")
    seen, todo = set(), [root]
    while todo:
        pid = todo.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for dep in nodes[pid]["deps"]:
            if any(k.get("kind") is None for k in dep["dep_kinds"]):
                todo.append(dep["pkg"])
    root_dir = Path(meta["workspace_root"])
    for pid in sorted(seen, key=lambda i: (pkgs[i]["name"], pkgs[i]["version"])):
        p = pkgs[pid]
        files = licence_files(Path(p["manifest_path"]).parent)
        if not files and pid in meta["workspace_members"]:
            files = licence_files(root_dir)
        yield p, files


def main():
    args = sys.argv[1:]
    fonts = []
    while "--fonts" in args:
        i = args.index("--fonts")
        fonts.append(args[i + 1])
        del args[i:i + 2]
    out, target, specs = args[0], args[1], args[2:]
    crates = {}  # (name, version) -> (licence expression, repository, [text hashes])
    texts = {}   # hash -> (title, text)
    def add_text(title, path):
        data = path.read_bytes().decode("utf-8", "replace").strip()
        h = hashlib.sha256(data.encode()).hexdigest()[:12]
        texts.setdefault(h, (title, data))
        return h
    for spec in specs:
        workspace, package = spec.rsplit(":", 1)
        for p, files in walk(workspace, package, target):
            key = (p["name"], p["version"])
            if p["name"] == "kruboffice" or key in crates:  # KrubOffice's own launcher: LICENSE-MIT / LICENSE-APACHE
                continue
            hashes = [add_text(f"{p['name']} {p['version']}: {f.name}", f) for f in files]
            crates[key] = (p.get("license") or "see licence text", p.get("repository") or "", hashes)
    font_entries = []
    for d in fonts:
        for root, _dirs, names in os.walk(d):
            for f in licence_files(root):
                rel = os.path.relpath(f, d)
                font_entries.append((rel, add_text(f"fonts: {rel}", f)))
    lines = [
        "KrubOffice: third-party notices",
        "",
        "KrubOffice is open source under the MIT license or the Apache License 2.0 (LICENSE-MIT.txt,",
        "LICENSE-APACHE.txt). It contains the open-source software and fonts listed below, each under its",
        "own licence. Their licence texts and copyright notices follow.",
        "",
        f"Components ({len(crates)} Rust crates):",
        "",
    ]
    for (name, version), (lic, repo, hashes) in sorted(crates.items()):
        refs = ", ".join(f"[{h}]" for h in hashes) or "(no licence file in the crate; licence: see expression)"
        lines.append(f"  {name} {version}  ({lic})  {repo}  {refs}")
    if font_entries:
        lines += ["", "Fonts:", ""] + [f"  {rel}  [{h}]" for rel, h in sorted(font_entries)]
    lines += ["", "=" * 78, "Licence texts", "=" * 78, ""]
    for h, (title, data) in texts.items():
        lines += [f"[{h}] {title}", "-" * 78, data, "", ""]
    Path(out).write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{out}: {len(crates)} crates, {len(texts)} distinct licence texts, {len(font_entries)} font licence files")


if __name__ == "__main__":
    main()
