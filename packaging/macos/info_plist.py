"""KrubOffice.app's Info.plist. Reads `kruboffice --list-types` on stdin (one line per program:
name TAB comma-separated extensions) and writes the plist, declaring each program's file types
as a document type of KrubOffice. Rank "Alternate": KrubOffice appears under Open With and can be
made the default, but doesn't take a file type over from an app the person already uses."""
import plistlib
import sys

version, khian, khit, phut = sys.argv[1:5]

# Uniform type identifiers macOS knows for the common extensions; other extensions are matched
# by extension alone.
UTI = {
    "docx": "org.openxmlformats.wordprocessingml.document",
    "docm": "org.openxmlformats.wordprocessingml.document.macroenabled",
    "dotx": "org.openxmlformats.wordprocessingml.template",
    "doc": "com.microsoft.word.doc",
    "rtf": "public.rtf",
    "odt": "org.oasis-open.opendocument.text",
    "txt": "public.plain-text",
    "md": "net.daringfireball.markdown",
    "html": "public.html",
    "xlsx": "org.openxmlformats.spreadsheetml.sheet",
    "xlsm": "org.openxmlformats.spreadsheetml.sheet.macroenabled",
    "xls": "com.microsoft.excel.xls",
    "ods": "org.oasis-open.opendocument.spreadsheet",
    "csv": "public.comma-separated-values-text",
    "tsv": "public.tab-separated-values-text",
    "pptx": "org.openxmlformats.presentationml.presentation",
    "ppsx": "org.openxmlformats.presentationml.slideshow",
    "potx": "org.openxmlformats.presentationml.template",
}
KIND = {"KhianKrub": "เอกสาร (KhianKrub)", "KhitKrub": "ตารางคำนวณ (KhitKrub)", "PhutKrub": "งานนำเสนอ (PhutKrub)"}

types = []
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    name, exts = line.split("\t")
    exts = [e for e in exts.split(",") if e]
    utis = sorted({UTI[e] for e in exts if e in UTI})
    entry = {
        "CFBundleTypeName": KIND.get(name, name),
        "CFBundleTypeRole": "Editor",
        "LSHandlerRank": "Alternate",
        "CFBundleTypeExtensions": exts,
        "CFBundleTypeIconFile": name,
    }
    if utis:
        entry["LSItemContentTypes"] = utis
    types.append(entry)
if len(types) != 3:
    sys.exit(f"expected 3 programs from --list-types, got {len(types)}")

plist = {
    "CFBundleName": "KrubOffice",
    "CFBundleDisplayName": "KrubOffice",
    "CFBundleExecutable": "kruboffice",
    "CFBundleIdentifier": "app.kruboffice.desktop",
    "CFBundleIconFile": "KrubOffice",
    "CFBundlePackageType": "APPL",
    "CFBundleShortVersionString": version,
    "CFBundleVersion": version,
    "CFBundleDevelopmentRegion": "th",
    "CFBundleLocalizations": ["th", "en"],
    "LSMinimumSystemVersion": "11.0",
    "LSApplicationCategoryType": "public.app-category.productivity",
    "NSHighResolutionCapable": True,
    "NSHumanReadableCopyright": f"KrubOffice {version}: KhianKrub {khian}, KhitKrub {khit}, PhutKrub {phut}. MIT OR Apache-2.0.",
    "CFBundleDocumentTypes": types,
}
sys.stdout.buffer.write(plistlib.dumps(plist))
