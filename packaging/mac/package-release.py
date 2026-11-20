"""Run after the native Mac build. Creates a DMG and a Mac-only website update ZIP."""
from pathlib import Path
from compatibility import verify as verify_compatibility
from distribution_signing import signing_config, sign_app, notarize_dmg
import hashlib, json, os, platform, re, shutil, subprocess, sys, tempfile, zipfile
if platform.system() != 'Darwin': raise SystemExit('macOS is required.')
root = Path(__file__).resolve().parents[2]
app = Path(sys.argv[1]).resolve()
sys.path.insert(0,str(root/'packaging'))
from private_build import require_public_binary
require_public_binary(app/'Contents/MacOS/lemonstudio')
if any('Master Assistant' in str(p) for p in app.rglob('*')): raise SystemExit('Private assistant bundle cannot be published.')
version = re.search(r'pub const VERSION: u64 = (\d+);', (root/'crates/os-app/src/updates.rs').read_text()).group(1)
arch = os.environ.get("LEMON_MAC_ARCH",platform.machine())
actual=subprocess.check_output(["lipo","-archs",str(app/"Contents/MacOS/lemonstudio")],text=True).split()
if actual != [arch]: raise SystemExit(f"Architecture mismatch: requested {arch}, actual {actual}")
if arch not in ('arm64','x86_64'): raise SystemExit('Unsupported architecture')
out = Path(os.environ['LEMON_PACKAGE_OUT']) if os.environ.get('LEMON_PACKAGE_OUT') else Path.home()/'Desktop/Lemon Studio Mac Public Download'/version/arch
out.mkdir(parents=True, exist_ok=True)
targets = verify_compatibility(app, arch, out)
print('Packaged deployment targets:', targets)
signing = signing_config()
source = out/f'ML-Studio-Source-Mac-{arch}-{version}.zip'
subprocess.run([sys.executable, str(root/'packaging/export-source.py'), str(source), "--public"], check=True)
dmg = out/f'ML-Studio-Mac-{arch}-{version}.dmg'
if dmg.exists(): raise SystemExit('Versioned DMG already exists; preserve published artifacts and increment the build number for changes.')
with tempfile.TemporaryDirectory(prefix='mac-package-', dir=app.parent) as scratch:
    stage = Path(scratch)
    if signing:
        sign_app(app, signing[0])  # This same bundle is used to generate the signed updater.
    signing_note = ('This release is Developer ID signed and notarized by Apple.' if signing else
                    'This community build is ad-hoc signed and has no Apple notarization ticket.\n'
                    'Open ML Studio from Applications once. If macOS cannot verify the developer and you trust this download, '
                    'find ML Studio under System Settings > Privacy & Security and choose Open Anyway. '
                    'On macOS 12 Monterey, use System Preferences > Security & Privacy > General.\n'
                    'For other alerts, including damage or malware warnings, see https://support.apple.com/en-us/102445')
    subprocess.run(['ditto', str(app), str(stage/'ML Studio.app')], check=True)
    (stage/'Applications').symlink_to('/Applications')
    for name in ('LICENSE', 'THIRD-PARTY-NOTICES.txt'):
        shutil.copy2(root/name, stage/name)
    shutil.copy2(source, stage/'ML Studio Source.zip')
    (stage/'READ ME.txt').write_text(f'ML Studio - Michael Lane edition\nMac architecture: {arch}\nDrag the app to Applications. {signing_note}\nMatching editable source and notices are included. Existing projects and settings are preserved.\n')
    with tempfile.TemporaryDirectory(prefix='dmg-', dir=out) as candidate:
        image = Path(candidate)/dmg.name
        subprocess.run(['hdiutil','create','-volname','ML Studio','-srcfolder',str(stage),'-format','UDZO',str(image)], check=True)
        submission = notarize_dmg(image, *signing) if signing else None
        subprocess.run(['hdiutil','verify',str(image)], check=True)
        image.rename(dmg)
manifest = {'version':int(version),'label':f'Mac {arch}', 'url':f'mac/{version}/{dmg.name}', 'source':f'mac/{version}/{source.name}', 'sha256':hashlib.sha256(dmg.read_bytes()).hexdigest()}
manifest['signing'] = 'developer-id-notarized' if submission else 'ad-hoc'
if submission:
    (out/'notarization.json').write_text(json.dumps({'submission':submission,'sha256':manifest['sha256']},indent=2))
(out/f'mac-{arch}.json').write_text(json.dumps(manifest, indent=2))
if os.environ.get('LEMON_PIPELINE') != '1':
 with zipfile.ZipFile(out/f'Mac-{arch}-{version}-files.zip','w',zipfile.ZIP_DEFLATED) as z:
     for p in (dmg, source): z.write(p, f'lemon-studio/mac/{version}/{p.name}')
 with zipfile.ZipFile(out/f'Mac-{arch}-{version}-publish-last.zip','w',zipfile.ZIP_DEFLATED) as z:
     z.write(out/f'mac-{arch}.json', f'lemon-studio/platforms/mac-{arch}.json')
print(f'Ready for publication: {out}\nUpload/extract files first, verify HTTPS downloads, then upload/extract publish-last. See MAC_RELEASE.md.')
