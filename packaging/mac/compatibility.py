"""Check deployment targets of the shipped executable and its archived native engines."""
import io
import mmap
import plistlib
import re
import struct
import subprocess
import tempfile
import zipfile
from pathlib import Path

MINIMUM = (12, 0, 0)
REQUIRED = {
    'ML Master Assistant.vst3/Contents/MacOS/ML Master Assistant',
    'ML Plasma.clap/Contents/MacOS/ML Plasma',
    'verovio',
}
MACH_HEADERS = {bytes.fromhex(magic) for magic in
                ('cffaedfe', 'feedfacf', 'cafebabe', 'bebafeca', 'cafebabf', 'bfbafeca')}


def version(raw):
    parts = [int(n) for n in raw.split('.')]
    return tuple(parts + [0] * (3 - len(parts)))


def minimum_os(path, arch):
    output = subprocess.check_output(['otool', '-arch', arch, '-l', path], text=True)
    for block in re.split(r'^Load command \d+\s*$', output, flags=re.M):
        if re.search(r'^\s*cmd LC_BUILD_VERSION$', block, re.M):
            if not re.search(r'^\s*platform (?:1|MACOS|macos)$', block, re.M):
                continue
            field = 'minos'
        elif re.search(r'^\s*cmd LC_VERSION_MIN_MACOSX$', block, re.M):
            field = 'version'
        else:
            continue
        match = re.search(rf'^\s*{field} ([0-9.]+)$', block, re.M)
        if match:
            return match[1]
    raise ValueError(f'{path.name} has no macOS deployment target for {arch}')


def native_members(binary):
    # Rust embeds each engine ZIP with include_bytes!. ZIP directory offsets are
    # relative to the start of that archive inside the executable.
    with binary.open('rb') as file, mmap.mmap(file.fileno(), 0, access=mmap.ACCESS_READ) as data:
        pos = 0
        while (end := data.find(b'PK\x05\x06', pos)) >= 0:
            pos = end + 4
            if end + 22 > len(data):
                continue
            disk, cd_disk, _, count, size, offset, comment = struct.unpack_from('<4H2IH', data, pos)
            start = end - size - offset
            if disk or cd_disk or not count or start < 0 or data[start:start+4] != b'PK\x03\x04':
                continue
            with zipfile.ZipFile(io.BytesIO(data[start:end+22+comment])) as archive:
                for member in archive.infolist():
                    if member.file_size < 4:
                        continue
                    with archive.open(member) as payload:
                        magic = payload.read(4)
                        if magic in MACH_HEADERS:
                            yield member.filename, magic + payload.read()


def verify(app, arch, scratch):
    app = Path(app)
    with (app/'Contents/Info.plist').open('rb') as plist:
        if version(plistlib.load(plist)['LSMinimumSystemVersion']) != MINIMUM:
            raise ValueError('App minimum OS differs from the advertised macOS 12.0')
    binary = app/'Contents/MacOS/lemonstudio'
    targets = {}

    def check(name, path):
        target = minimum_os(path, arch)
        if version(target) > MINIMUM:
            raise ValueError(f'{name} ({arch}) requires macOS {target}; promised minimum is 12.0')
        targets[name] = target

    check(binary.name, binary)
    with tempfile.TemporaryDirectory(prefix='native-check-', dir=scratch) as folder:
        extracted = Path(folder)/'image'
        for name, payload in native_members(binary):
            extracted.write_bytes(payload)
            check(name, extracted)
    missing = REQUIRED.difference(targets)
    if missing:
        raise ValueError(f'Missing packaged {arch} engines: {sorted(missing)}')
    return targets
