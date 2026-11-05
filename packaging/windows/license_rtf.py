"""The installer's licence page (RTF) from LICENSE-MIT and LICENSE-APACHE."""
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
def rtf(text):
    out = []
    for ch in text:
        if ch in "\\{}":
            out.append("\\" + ch)
        elif ch == "\n":
            out.append("\\par\n")
        elif ord(ch) > 127:
            out.append(f"\\u{ord(ch) if ord(ch) < 32768 else ord(ch) - 65536}?")
        else:
            out.append(ch)
    return "".join(out)
body = ("KrubOffice is licensed under the MIT license or the Apache License 2.0, at your option.\n\n"
        + (root / "LICENSE-MIT").read_text(encoding="utf-8") + "\n\n" + (root / "LICENSE-APACHE").read_text(encoding="utf-8"))
Path(sys.argv[1]).write_text("{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Segoe UI;}}\\fs18\n" + rtf(body) + "}\n", encoding="ascii")
