import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import mac_retention as m
from checkpoints import atomic_json, digest


class MacRetentionTests(unittest.TestCase):
    def test_keep_two_verified_installers_and_preserve_unfinished_work(self):
        with tempfile.TemporaryDirectory() as temp:
            home = Path(temp).resolve()
            root = home/'Builds'
            share = home/'Share'
            rev = 'a'*40
            versions = ['2026010101','2026010201','2026010301']
            def published(v):
                for arch in ('arm64','x86_64'):
                    scope = 'mac-'+arch
                    name = f'ML-Studio-Mac-{arch}-{v}.dmg'
                    candidate = home/'payload'
                    candidate.write_bytes((v+arch).encode())
                    h = digest(candidate)
                    obj = share/'pipeline'/v/rev[:12]/scope/'objects'/h/name
                    obj.parent.mkdir(parents=True)
                    candidate.replace(obj)
                    atomic_json(share/'published'/v/scope/'PUBLISHED.json',
                                dict(version=int(v),scope=scope,commit=rev,files=[dict(path=f'mac/{v}/{name}',sha256=h)]))
            def worker(v, status):
                folder = root/'Lemon Studio Public Download'/v
                for a in ('arm64','x86_64'):
                    name='mac-'+a
                    atomic_json(folder/'Pipeline'/name/'state.json',
                                dict(identity=dict(version=int(v),commit=rev),steps={'Stage '+name:dict(status=status)}))
                return folder
            with patch.object(m.sys,'platform','darwin'), patch.object(Path,'home',return_value=home), patch.object(m.storage,'app_root',return_value=root), patch.object(m.r,'shared_folder',return_value=share), patch.object(m.storage,'free_space'):
                for v in versions[:2]:
                    published(v)
                pending = worker(versions[0],'failed')
                ready = worker(versions[1],'complete')
                m.retain(int(versions[1]),rev)
                self.assertTrue(pending.exists())
                self.assertFalse(ready.exists())
                published(versions[2])
                old = root/'Mac Rollbacks'/versions[0]
                newest = worker(versions[2],'complete')
                data = next((share/'pipeline'/versions[2]).rglob('*.dmg'))
                original = data.read_bytes()
                data.write_bytes(b'bad image')
                with self.assertRaisesRegex(RuntimeError,'checksum'):
                    m.retain(int(versions[2]),rev)
                self.assertTrue(old.exists())
                self.assertTrue(newest.exists())
                data.write_bytes(original)
                m.retain(int(versions[2]),rev)
                self.assertFalse(old.exists())
                self.assertFalse(newest.exists())
                self.assertTrue(pending.exists())
                self.assertEqual(sorted(p.name for p in old.parent.iterdir()),versions[1:])
