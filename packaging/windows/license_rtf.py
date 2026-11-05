"""The installer's licence page (RTF) from LICENSE.txt, KrubOffice's freeware licence (Thai and English)."""
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
body = (root / "LICENSE.txt").read_text(encoding="utf-8") + "\nThe licences of the open-source parts are in THIRD-PARTY-NOTICES.txt, installed with KrubOffice.\n"
# Leelawadee UI: Windows' interface font with Thai letters.
Path(sys.argv[1]).write_text("{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Leelawadee UI;}}\\fs18\n" + rtf(body) + "}\n", encoding="ascii")
