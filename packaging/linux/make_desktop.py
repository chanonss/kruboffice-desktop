"""Write kruboffice.desktop from packaging/types.txt (one "App<TAB>ext,ext,..." line per program).

    python3 -I packaging/linux/make_desktop.py packaging/types.txt OUT.desktop

The MIME types are the usual ones for each extension. Extensions with no registered type are left out
(they still open from the Start Center and with "Open with kruboffice").
"""
import sys

MIME = {
    "docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "docm": "application/vnd.ms-word.document.macroenabled.12",
    "dotx": "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
    "dotm": "application/vnd.ms-word.template.macroenabled.12",
    "doc": "application/msword", "dot": "application/msword",
    "rtf": "application/rtf", "odt": "application/vnd.oasis.opendocument.text",
    "txt": "text/plain", "md": "text/markdown", "markdown": "text/markdown",
    "html": "text/html", "htm": "text/html",
    "tex": "text/x-tex", "latex": "text/x-tex", "ltx": "text/x-tex",
    "xlsx": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "xlsm": "application/vnd.ms-excel.sheet.macroenabled.12",
    "xltx": "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
    "xltm": "application/vnd.ms-excel.template.macroenabled.12",
    "xls": "application/vnd.ms-excel", "xlt": "application/vnd.ms-excel",
    "xlsb": "application/vnd.ms-excel.sheet.binary.macroenabled.12",
    "ods": "application/vnd.oasis.opendocument.spreadsheet",
    "csv": "text/csv", "tsv": "text/tab-separated-values",
    "pptx": "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "potx": "application/vnd.openxmlformats-officedocument.presentationml.template",
    "ppsx": "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
}

mimes = []
for line in open(sys.argv[1], encoding="utf-8"):
    if not line.strip():
        continue
    _app, exts = line.rstrip("\n").split("\t")
    for e in exts.split(","):
        m = MIME.get(e)
        if m and m not in mimes:
            mimes.append(m)

with open(sys.argv[2], "w", encoding="utf-8") as f:
    f.write(f"""[Desktop Entry]
Type=Application
Name=KrubOffice
Name[th]=ครับออฟฟิศ
GenericName=Office suite
Comment=Documents, spreadsheets and presentations (KhianKrub, KhitKrub, PhutKrub)
Comment[th]=เขียนครับ คิดครับ พูดครับ ในแอปเดียว
Exec=kruboffice %F
Icon=kruboffice
Terminal=false
Categories=Office;WordProcessor;Spreadsheet;Presentation;
MimeType={";".join(mimes)};
StartupWMClass=app.kruboffice.desktop
StartupNotify=true
""")

# Hidden entries for the three programs' own windows (each is its own process with its own window class), so the
# panel shows their name and icon instead of a generic one. They stay out of the menu; files open through KrubOffice.
HIDDEN = [
    ("khiankrub", "KhianKrub", "เขียนครับ", "ai.storyteller.wordcraft"),
    ("khitkrub", "KhitKrub", "คิดครับ", "ai.storyteller.gridcraft"),
    ("phutkrub", "PhutKrub", "พูดครับ", "ai.storyteller.deckcraft"),
]
out_dir = sys.argv[2].rsplit("/", 1)[0]
for exe, name, thai, wm_class in HIDDEN:
    with open(f"{out_dir}/kruboffice-{exe}.desktop", "w", encoding="utf-8") as f:
        f.write(f"""[Desktop Entry]
Type=Application
Name={name}
Name[th]={thai}
Exec=/usr/lib/kruboffice/{exe} %F
Icon=kruboffice-{exe}
Terminal=false
NoDisplay=true
StartupWMClass={wm_class}
""")
