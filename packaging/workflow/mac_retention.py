"""Keep verified Mac installers for the current release and one rollback."""
from pathlib import Path
import json
import os
import re
import shutil
import sys

import release as r
import storage
from checkpoints import atomic_json, digest, exclusive


def retain(version, commit):
    if sys.platform != 'darwin':
        raise RuntimeError('Mac retention requires macOS')
    root = storage.plain_root(storage.app_root())
    if not root.is_relative_to(Path.home()) or root == Path.home():
        raise RuntimeError('Mac build output must be in the local profile')
    shared = r.shared_folder()
    receipts = shared/'published'
    versions = sorted((p.name for p in receipts.iterdir()
                       if re.fullmatch(r'[0-9]{10}',p.name) and int(p.name) <= version
                       and all((p/('mac-'+a)/'PUBLISHED.json').is_file() for a in ('arm64','x86_64'))),
                      reverse=True)
    if not versions or versions[0] != str(version):
        raise RuntimeError('Mac publication receipts are incomplete')
    archive = root/'Mac Rollbacks'
    if storage.linked(archive):
        raise RuntimeError('Unexpected linked rollback folder')
    for v in versions[:2]:
        dst = archive/v
        if dst.resolve() != dst:
            raise RuntimeError('Unexpected linked archive path')
        dst.mkdir(parents=True,exist_ok=True)
        proof = {}
        for arch in ('arm64','x86_64'):
            scope = 'mac-'+arch
            receipt = json.loads((receipts/v/scope/'PUBLISHED.json').read_text())
            revision = receipt['commit']
            if (str(receipt['version']) != v or receipt['scope'] != scope
                or not re.fullmatch('[0-9a-f]{40}',revision) or (v == str(version) and revision != commit)):
                raise RuntimeError('Incorrect Mac publication receipt')
            name = f'ML-Studio-Mac-{arch}-{v}.dmg'
            entry, = (e for e in receipt['files'] if e['path'] == f'mac/{v}/{name}')
            h = entry['sha256']
            if not re.fullmatch('[a-f0-9]{64}',h):
                raise RuntimeError('Invalid installer digest')
            target = dst/name
            if storage.linked(target):
                raise RuntimeError('Unexpected linked installer')
            if not target.is_file() or digest(target) != h:
                src = shared/'pipeline'/v/revision[:12]/scope/'objects'/h/name
                storage.free_space(dst)
                tmp = dst/(name+'.pending')
                if storage.linked(tmp):
                    raise RuntimeError('Unexpected linked pending installer')
                shutil.copyfile(src,tmp)
                if digest(tmp) != h:
                    raise RuntimeError('Mac rollback checksum mismatch')
                tmp.replace(target)
            proof[name] = h
        atomic_json(dst/'installers.json',proof)
    for dst in archive.iterdir():
        if (dst.name in versions[2:] and dst.resolve() == dst and (dst/'installers.json').is_file()
            and {p.name for p in dst.iterdir()} == {'installers.json',*(f'ML-Studio-Mac-{a}-{dst.name}.dmg' for a in ('arm64','x86_64'))}):
            shutil.rmtree(dst)
    # This dedicated output root contains the workers' disposable build and smoke-test files.
    builds = root/'Lemon Studio Public Download'
    for v in versions:
        dst = builds/v
        if dst.resolve() != dst or not dst.is_dir() or storage.release_active(dst):
            continue
        pipe = dst/'Pipeline'
        if {p.name for p in dst.iterdir()} != {'Pipeline'} or not pipe.is_dir() or storage.linked(pipe):
            continue
        if {p.name for p in pipe.iterdir()} - {'mac-arm64','mac-x86_64','mac-source'}:
            continue
        for a in ('arm64','x86_64'):
            statefile = pipe/('mac-'+a)/'state.json'
            if not statefile.is_file():
                break
            state = json.loads(statefile.read_text())
            published = json.loads((receipts/v/('mac-'+a)/'PUBLISHED.json').read_text())
            if (state['steps'].get('Stage mac-'+a,{}).get('status') != 'complete'
                or str(state['identity']['version']) != v or state['identity']['commit'] != published['commit']):
                break
        else:
            for parent, dirs, _ in os.walk(dst, followlinks=False):
                if any((Path(parent)/name).is_mount() for name in dirs):
                    raise RuntimeError('Mounted volume in completed workspace: '+parent)
            shutil.rmtree(dst)
    return versions[:2]


def finish():
    with exclusive(r.PRIVATE/'pipeline-mac-arm64.lock'), exclusive(r.PRIVATE/'pipeline-mac-x86_64.lock'):
        versions = retain(r.version(),r.git('rev-parse','HEAD'))
        print('Verified Mac rollback versions:', ', '.join(versions),flush=True)
