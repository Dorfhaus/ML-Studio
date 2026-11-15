
"""Bundle a pinned Verovio CLI installation for a single ML Studio target."""

import argparse
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

REVISION = "425dd7b6ba26edcecbe345c10b15586d095743d9"

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path, help=f"Verovio source at {REVISION}")
parser.add_argument("install", type=Path, help="Verovio CMake install prefix")
parser.add_argument("output", type=Path)
args = parser.parse_args()

program = args.install / "bin" / "verovio.exe"
if not program.is_file():
    program = args.install / "bin" / "verovio"
data = args.install / "share" / "verovio"
licenses = Path(__file__).with_name("licenses")
if not program.is_file() or not (data / "Bravura.xml").is_file():
    parser.error("expected a Verovio CMake install with bin and share/verovio")

with ZipFile(args.output, "w", ZIP_DEFLATED, compresslevel=6) as archive:
    archive.write(program, program.name)
    for file in sorted(data.rglob("*")):
        relative = file.relative_to(data)
        if file.is_file() and relative.parts[0] in {
            "Bravura", "Bravura.xml", "Bravura.css", "Leipzig", "Leipzig.xml", "Leipzig.css",
            "text", "Liberation.css", "tuning-glyphnames.json",
        }:
            archive.write(file, "data/" + relative.as_posix())
    for file in sorted(licenses.iterdir()):
        archive.write(file, "licenses/" + file.name)
    for filename in ("COPYING", "COPYING.LESSER", "include/json/LICENSE", "include/tuning-library/LICENSE.md"):
        archive.write(args.source / filename, "licenses/verovio/" + filename)
    archive.writestr("SOURCE.txt", f"Verovio 6.3.0\nhttps://github.com/rism-digital/verovio/tree/{REVISION}\n")
