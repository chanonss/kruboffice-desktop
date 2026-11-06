"""Pack a Windows .ico (16-256 px, PNG entries) from one square PNG. Uses macOS `sips` for the sizes.

    python3 -I packaging/windows/make_ico.py SRC.png OUT.ico
"""
import os
import struct
import subprocess
import sys
import tempfile

src, out = sys.argv[1], sys.argv[2]
sizes = [16, 24, 32, 48, 64, 128, 256]
blobs = []
with tempfile.TemporaryDirectory() as d:
    for s in sizes:
        p = os.path.join(d, f"{s}.png")
        subprocess.run(["sips", "-z", str(s), str(s), src, "--out", p], check=True, capture_output=True)
        blobs.append(open(p, "rb").read())
header = struct.pack("<HHH", 0, 1, len(sizes))
offset = 6 + 16 * len(sizes)
entries = b""
for s, b in zip(sizes, blobs):
    entries += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(b), offset)
    offset += len(b)
open(out, "wb").write(header + entries + b"".join(blobs))
print(out, os.path.getsize(out))
