import importlib.util,json,os,sys,tempfile,unittest,zipfile
from pathlib import Path
from unittest.mock import patch
sys.path.insert(0,str(Path(__file__).parent))
import checkpoints as c
import pipeline as p
import builders as b

class PipelineTests(unittest.TestCase):
    def test_superseded_release_cannot_resume_publication(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'state.json'
            c.atomic_json(path,{'identity':{'version':7},'steps':{},'superseded':'Model replaced before DAW publication; use build 8'})
            with self.assertRaisesRegex(RuntimeError,'superseded'):c.Journal(path,{'version':7})
    def test_resume_skips_success_and_retries_only_failed_step(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);artifact=root/'app.exe';artifact.write_bytes(b'compiled');calls=[]
            j=c.Journal(root/'state.json',{'version':7,'commit':'abc'})
            def build(n):calls.append(('build',n));return str(artifact),[artifact]
            j.step('build',build)
            with self.assertRaises(RuntimeError):j.step('upload',lambda n:(_ for _ in ()).throw(RuntimeError('connection lost')))
            j=c.Journal(root/'state.json',{'version':7,'commit':'abc'})
            j.step('build',build)
            j.step('upload',lambda n:(calls.append(('upload',n)) or 'done',[artifact]))
            self.assertEqual(calls,[('build',1),('upload',2)])
            self.assertIn('upload: complete',(root/'Progress.txt').read_text())
    def test_revision_and_artifact_changes_stop_without_rerunning(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);file=root/'artifact';file.write_bytes(b'original');j=c.Journal(root/'state.json',{'commit':'a'})
            j.step('compile',lambda n:('ready',[file]));file.write_bytes(b'changed')
            with self.assertRaisesRegex(RuntimeError,'changed'):j.step('compile',lambda n:self.fail('must not rebuild'))
            with self.assertRaisesRegex(RuntimeError,'identity'):c.Journal(root/'state.json',{'commit':'b'})
    def test_process_lock_rejects_second_publisher_and_releases(self):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'run.lock'
            with c.exclusive(path):
                with self.assertRaisesRegex(RuntimeError,'publisher'): 
                    with c.exclusive(path):pass
            with c.exclusive(path):pass
    def test_staging_deduplicates_and_excludes_unplanned_files(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'a.zip').write_bytes(b'archive');(root/'private.txt').write_text('do not copy')
            plan=root/'plan.json';c.atomic_json(plan,dict(version=7,commit='abc',scope='windows-linux',files=[dict(path='updates/7/a.zip',local='a.zip',sha256=c.digest(root/'a.zip')),dict(path='updates/linux-x64/7/a.zip',local='a.zip',sha256=c.digest(root/'a.zip'))]))
            staged=c.stage_plan(plan,root/'shared');data=json.loads(staged.read_text());self.assertEqual(data['files'][0]['local'],data['files'][1]['local'])
            self.assertFalse((staged.parent/'private.txt').exists());c.stage_plan(plan,root/'shared')
            (staged.parent/data['files'][0]['local']).write_bytes(b'tampered')
            with self.assertRaisesRegex(RuntimeError,'changed'):c.stage_plan(plan,root/'shared')
    def test_public_build_overrides_personal_environment_and_isolates_caches(self):
        cfg={'cache':'public-cache','juce_source':'juce','engines':{x:{'bundle':x+'.zip'} for x in p.PLATFORMS},
             'score_bundles':{x:{'bundle':'score-'+x+'.zip'} for x in p.PLATFORMS}}
        with patch.dict(os.environ,{'LEMON_WAVE_LIBRARY':'personal.lwpack','CARGO_TARGET_DIR':'personal-cache','LEMON_SCORE_BUNDLE':'personal.zip'}),patch.object(b.r,'read_config',return_value={'llvm':'llvm'}), patch.object(b, 'visualizer_inputs', return_value=(Path('renderer.exe'), Path('renderer-source.zip'))):
            environments=[b.environment(cfg,x) for x in p.PLATFORMS]
        self.assertTrue(all(e['LEMON_WAVE_LIBRARY']=='' and e['LEMON_RELEASE']=='1' for e in environments))
        self.assertEqual(len({e['CARGO_TARGET_DIR'] for e in environments}),4)
        self.assertEqual([e['LEMON_SCORE_BUNDLE'] for e in environments],['score-'+x+'.zip' for x in p.PLATFORMS])
        self.assertEqual(environments[0]['LEMON_VISUALIZER_WORKER'], 'renderer.exe')
        self.assertEqual(environments[0]['LEMON_VISUALIZER_SOURCE'], 'renderer-source.zip')
        self.assertTrue(all('LEMON_VISUALIZER_WORKER' not in e and 'LEMON_VISUALIZER_SOURCE' not in e for e in environments[1:]))

    def test_direct_downloads_follow_mixed_platform_versions(self):
        template = (p.r.ROOT/'packaging/download-page.html').read_bytes()
        manifests = {}
        for platform in p.PLATFORMS:
            mac = platform.startswith('mac-')
            build = 45 if mac else 44
            folder = 'mac' if mac else 'linux' if platform == 'linux' else 'downloads'
            manifests[platform] = dict(version=build, url=f'{folder}/{build}/{platform}.bin',
                                       source=f'{"mac" if mac else "updates"}/{build}/{platform} Source.zip')
        mixed = p.download_page.render(template, manifests, p.r.DOWNLOAD_PAGE)
        for name, manifest in manifests.items():
            for field in ('url', 'source'):
                expected = ('href="' + p.r.DOWNLOAD_PAGE + p.urllib.parse.quote(manifest[field]) + '"').encode()
                self.assertEqual(mixed.count(expected), 2 if (name, field) == ('windows', 'source') else 1)
        self.assertEqual(mixed, p.download_page.render(mixed, manifests, p.r.DOWNLOAD_PAGE))
        incomplete = mixed.replace(b'data-platform="linux"', b'data-platform="missing"')
        with self.assertRaisesRegex(ValueError, 'Expected one matching download link'):
            p.download_page.render(incomplete, manifests, p.r.DOWNLOAD_PAGE)

    def test_mix_reference_download_is_website_only(self):
        self.assertTrue(p.r.permitted(p.r.MIX_REFERENCE_PATH,'website'))
        for scope in (*p.PLATFORMS,'windows-linux','helper'):
            self.assertFalse(p.r.permitted(p.r.MIX_REFERENCE_PATH,scope))
        self.assertFalse(p.r.permitted('another-pack.mlmixref','website'))
    def test_changed_engine_rejected_before_native_build(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'modules/juce_core').mkdir(parents=True);(root/'modules/juce_core/juce_core.h').touch()
            cfg={'juce_source':str(root),'engines':{'windows':{'source':'old','bundle':'missing.zip'}}}
            with patch.object(p,'config',return_value=cfg),patch.object(p,'engine_id',return_value='new'):
                with self.assertRaisesRegex(RuntimeError,'source changed'):p.probe()
    def test_packaging_retry_verifies_existing_dmg(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            out = root/'attempt-1'
            manifest = out/'lemon-studio/platforms/mac-arm64.json'
            c.atomic_json(manifest,{'url':'app.dmg'})
            plan = out/'publish-plan.json'
            c.atomic_json(plan,{'files':[]})
            journal = c.Journal(root/'state.json',{'commit':'a'})
            with patch.object(b,'package_mac',return_value=plan) as make, patch.object(b,'verify_mac_package',side_effect=[RuntimeError('busy mount'),None]):
                with self.assertRaisesRegex(RuntimeError,'busy mount'):
                    b.mac({},'arm64',out,root/'log',journal=journal)
                result = b.mac({},'arm64',root/'attempt-2',root/'log',journal=journal)
                self.assertEqual(result,plan)
                make.assert_called_once()

    def test_stage_rejects_path_escape(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);folder=root/'release';folder.mkdir();secret=root/'private';secret.write_bytes(b'no')
            plan=folder/'plan.json';c.atomic_json(plan,dict(files=[dict(local='../private',sha256=c.digest(secret))]))
            with self.assertRaisesRegex(RuntimeError,'Invalid'):c.plan_files(plan)
    def test_complete_step_without_expected_artifact_is_not_recorded(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);j=c.Journal(root/'state.json',{'version':1})
            with self.assertRaises(FileNotFoundError):j.step('build',lambda n:('done',[root/'missing']))
            self.assertEqual(j.data['steps']['build']['status'],'failed')

    def test_full_coordinator_restarts_failed_publish_without_rebuilding(self):
        self.coordinator_restarts_failed_publish(mac_only=False)

    def test_mac_coordinator_preserves_scope_when_resumed(self):
        self.coordinator_restarts_failed_publish(mac_only=True)

    def coordinator_restarts_failed_publish(self,mac_only):
        from contextlib import ExitStack
        import threading
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);out=root/'Desktop';share=root/'share';share.mkdir();out.mkdir();source=root/'source';(source/'packaging').mkdir(parents=True);(source/'packaging/download-page.html').write_text('page')
            reference=root/'reference.mlmixref';reference.write_bytes(b'fixture')
            calls=[];barrier=threading.Barrier(2 if mac_only else 4,timeout=10);failed=[False]
            def makeplan(folder,scope):
                folder=Path(folder);folder.mkdir(parents=True,exist_ok=True);file=folder/'test.bin';file.write_bytes(scope.encode())
                dest=folder/'publish-plan.json';c.atomic_json(dest,dict(version=7,commit='abc',scope=scope,files=[dict(path='mac/7/'+scope+'.bin',local='test.bin',sha256=c.digest(file))]));return dest
            def win(cfg,folder,log):calls.append('windows');barrier.wait();return makeplan(folder,'windows-linux')
            def lin(cfg,log):calls.append('linux');barrier.wait();f=out/'linux-binary';f.write_bytes(b'linux');return f
            def remote(args,log,sync=False):
                if args[0]=='worker':
                    self.assertEqual(args[2:],['--mac-only'] if mac_only else [])
                    calls.append(args[1]);barrier.wait();makeplan(share/('mac-'+args[1]),'mac-'+args[1])
                elif args[0]=='publish-files-worker':
                    c.atomic_json(share/args[1]/'FILES-VERIFIED.json',{'scope':args[1]})
                elif args[0]=='publish-worker':
                    calls.append('publish-'+args[1])
                    if args[1]=='mac-arm64' and not failed[0]:failed[0]=True;raise RuntimeError('simulated lost connection')
                    c.atomic_json(share/args[1]/'PUBLISHED.json',{'verified':True})
            def stage(plan,name):dest=c.stage_plan(plan,share/name);return str(dest),c.plan_files(dest)
            protected={p: 'fixture' for p in p.WINDOWS_LINUX_LINKS}
            def verify(plans,folder,website,before=None):
                self.assertNotIn(website, plans)
                scopes={json.loads(Path(plan).read_text())['scope'] for plan in plans}
                self.assertEqual(scopes, set(p.MAC_PLATFORMS) if mac_only else set(p.MAC_PLATFORMS)|{'windows-linux','helper'})
                self.assertEqual(before, protected if mac_only else None)
                f=folder/'github-release.json';c.atomic_json(f,{'tag':'fixture'});return str(f),[f]
            def ghverify(plan,folder,commit=None):f=folder/'GITHUB-VERIFIED.json';c.atomic_json(f,{'url':'fixture'});return 'fixture',[f]
            def backup(*args,**kw):
                f=share/'snapshot.zip';f.write_bytes(b'snapshot');c.atomic_json(share/'LATEST.json',{'source':f.name,'sha256':c.digest(f)})
            with ExitStack() as stack:
                changes=[(p.platform,'system',lambda:'Windows'),(p,'config',lambda:{'source_sync':'fixture','juce_source':'fixture'}),(p,'probe',lambda:None),(p,'remote',remote),(p,'prepare',lambda n:None),(p,'release_notes',lambda:{}),(p,'identity',lambda:{'version':7,'commit':'abc'}),(p,'output_root',lambda:out),(p,'shared_run',lambda:share),(p,'stage',stage),(p,'verify_live',verify),(p,'github_stage',lambda *a:({'commit':'abc','tag':'fixture'},[out/'github-release.json'])),(p,'github_verify',ghverify),(p.r,'ROOT',source),(p.r,'PRIVATE',root/'private'),(p.r,'git',lambda *a:'' if a[0]=='status' else 'abc'),(p.r,'run',lambda *a:None),(p.r,'shared_folder',lambda:share),(b,'windows',win),(b,'linux_compile',lin),(b,'linux_package',lambda cfg,binary,wp,folder,log:makeplan(folder,'linux-package')),(b,'merged_plan',lambda plans,folder:makeplan(folder/'combined','windows-linux')),(b,'command',backup),(p.r,'write_plan',lambda site,folder,scope:makeplan(folder,scope))]
                for obj,name,value in changes:stack.enter_context(patch.object(obj,name,value))
                stack.enter_context(patch.object(p.r, 'version', return_value=7))
                stack.enter_context(patch.object(p, 'render_website', return_value=b'page'))
                stack.enter_context(patch.object(p, 'mix_reference_pack', return_value=reference))
                stack.enter_context(patch.object(p.discord_changelog, 'bot_api', return_value=object()))
                post = stack.enter_context(patch.object(p.discord_changelog, 'publish', return_value={'verified': True}))
                stack.enter_context(patch.object(p,'helper_stage',lambda folder:stage(makeplan(folder/'helper','helper'),'helper')))
                stack.enter_context(patch.object(p,'prepare',lambda *a,**kw:None))
                capture=stack.enter_context(patch.object(p,'download_digests',return_value=protected))
                with self.assertRaisesRegex(RuntimeError,'lost connection'):p.coordinator(mac_only=mac_only)
                p.coordinator()
            for name in ['arm64','x86_64']:self.assertEqual(calls.count(name),1,name)
            for name in ['windows','linux']:self.assertEqual(calls.count(name),0 if mac_only else 1,name)
            post.assert_called_once()
            self.assertTrue(json.loads((out/'DISCORD-VERIFIED.json').read_text())['verified'])
            self.assertEqual(calls.count('publish-mac-arm64'),2)
            self.assertEqual(calls.count('publish-mac-x86_64'),1)
            if mac_only:
                self.assertNotIn('publish-helper',calls)
                self.assertNotIn('publish-windows-linux',calls)
                capture.assert_called_once_with(p.WINDOWS_LINUX_LINKS)
            else:
                self.assertLess(calls.index('publish-helper'),calls.index('publish-windows-linux'))
                capture.assert_not_called()
            self.assertEqual(json.loads((out/'state.json').read_text())['steps']['Back up source']['status'],'complete')

if __name__=='__main__':unittest.main()
