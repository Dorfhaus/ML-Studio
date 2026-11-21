"""Isolated public build workers. Credentials never enter these environments."""
from pathlib import Path
import json, os, shutil, struct, subprocess, tarfile, tempfile, time, zipfile
from contextlib import contextmanager
import release as r
import storage
from checkpoints import atomic_json, digest, plan_files


def environment(config,platform):
    env=dict(os.environ,OPENSTUDIO_NO_PLUGIN_BUILD='1',LEMON_RELEASE='1',LEMON_WAVE_LIBRARY='',CARGO_BUILD_JOBS=str(config.get('jobs_per_build',4)),PYTHONUTF8='1')
    for key in ('CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_BUILD_RUSTFLAGS'):env.pop(key,None)
    env['CARGO_TARGET_DIR']=str(Path(config['cache'])/platform)
    env['LEMON_MASTER_BUNDLE']=config['engines'][platform]['bundle']
    env['LEMON_JUCE_SOURCE']=config['juce_source']
    for key in ('LEMON_VISUALIZER_WORKER', 'LEMON_VISUALIZER_SOURCE'):
        env.pop(key, None)
    if platform == 'windows':
        env['LEMON_VISUALIZER_WORKER'], env['LEMON_VISUALIZER_SOURCE'] = map(str, visualizer_inputs(config))
    env.pop('LEMON_SCORE_BUNDLE', None)
    if entry := config.get('score_bundles', {}).get(platform):
        env['LEMON_SCORE_BUNDLE'] = entry['bundle']
    if bundle := config.get('plasma_bundles', {}).get(platform):
        target = {'windows': 'WINDOWS_X86_64', 'linux': 'LINUX_X86_64',
                  'mac-arm64': 'MACOS_AARCH64', 'mac-x86_64': 'MACOS_X86_64'}[platform]
        env['ML_PLASMA_BUNDLE_' + target] = bundle
    if platform.startswith('mac-'):
        if 'mac_distribution' in config:
            settings = config['mac_distribution']
            if not settings.get('identity') or not settings.get('keychain_profile'):
                raise ValueError('mac_distribution requires identity and keychain_profile')
            env['ML_MAC_SIGNING_IDENTITY'] = settings['identity']
            env['ML_MAC_NOTARY_PROFILE'] = settings['keychain_profile']
        env.update(LEMON_MAC_ARCH=platform[4:],LEMON_PIPELINE='1')
        env['PATH']=str(Path.home()/'.cargo/bin')+os.pathsep+str(Path.home()/'.lemon-studio-release/tools/bin')+os.pathsep+env.get('PATH','')
    else:env['LIBCLANG_PATH']=r.read_config()['llvm']
    return env


def visualizer_inputs(config):
    """Fail public builds before compiling if the pinned renderer or its source is missing."""
    entry = config.get('visualizer_worker', {})
    catalog = json.loads((r.ROOT/'packaging/visualizer/video-tools.json').read_text(encoding='utf-8'))
    worker = Path(entry.get('binary', ''))
    source = Path(entry.get('source', ''))
    if (not worker.is_file() or worker.stat().st_size != catalog['renderer_size']
            or digest(worker) != catalog['renderer_sha256']):
        raise RuntimeError('Configure the pinned Windows Visualizer renderer in pipeline.json (visualizer_worker.binary)')
    if not source.is_file() or digest(source) != catalog['renderer_source_sha256']:
        raise RuntimeError('Configure the matching Visualizer worker source archive in pipeline.json (visualizer_worker.source)')
    return worker, source


def verify_score_bundle(config, platform):
    entry = config.get('score_bundles', {}).get(platform)
    if not entry:
        raise RuntimeError('Missing Score Editor bundle configuration: ' + platform)
    archive = Path(entry['bundle'])
    if not archive.is_file() or digest(archive) != entry['sha256']:
        raise RuntimeError('Missing or changed Score Editor bundle: ' + platform)
    with zipfile.ZipFile(archive) as bundle:
        binary = bundle.read('verovio.exe' if platform == 'windows' else 'verovio')
        if platform == 'windows':
            pe_offset = struct.unpack_from('<I', binary, 60)[0]
            valid = binary[:2] == b'MZ' and binary[pe_offset:pe_offset + 6] == b'PE\0\0\x64\x86'
        elif platform == 'linux':
            valid = binary[:6] == b'\x7fELF\x02\x01' and binary[18:20] == b'\x3e\x00'
        else:
            expected_cpu = 0x0100000c if platform == 'mac-arm64' else 0x01000007
            valid = binary[:4] == b'\xcf\xfa\xed\xfe' and struct.unpack_from('<I', binary, 4)[0] == expected_cpu
        if not valid or not bundle.read('data/Bravura.xml') or bundle.testzip() is not None:
            raise RuntimeError('Invalid Score Editor bundle for ' + platform)


def command(log,*args,env=None,cwd=r.ROOT,timeout=None):
    Path(log).parent.mkdir(parents=True,exist_ok=True)
    with Path(log).open('a',encoding='utf-8') as f:
        f.write('\n> '+str(args[0])+' '+str(args[1] if len(args)>1 else '')+'\n');f.flush()
        subprocess.run(list(map(str,args)),cwd=cwd,env=env,stdout=f,stderr=subprocess.STDOUT,check=True,timeout=timeout)


def windows(config,out,log):
    storage.build_preflight(Path(config['cache'])/'windows', out)
    out=Path(out);out.mkdir(parents=True);env=environment(config,'windows');cfg=r.read_config()
    command(log,'cargo','build','--locked','--release','-p','os-app',env=env)
    binary=Path(env['CARGO_TARGET_DIR'])/'release/lemonstudio.exe'
    command(log,r.sys.executable,r.ROOT/'packaging/windows/prepare-update.py','--output',out,'--binary',binary,env=env)
    site=out/'lemon-studio';v=str(r.version());payload=out/'payload';payload.mkdir()
    if os.environ.get('ML_STUDIO_FILTERED_RELEASE') == '1':
        profile=out/'startup-profile'
        profile.mkdir()
        report=out/'startup-report.txt'
        clean=dict(env, OPENSTUDIO_USER_DIR=str(profile), GRIDLINE_CONFIG_HOME=str(profile))
        command(log,site/'updates'/v/'lemonstudio.exe','--startup-report',report,env=clean,timeout=140)
        if 'mixer_tracks=127' not in report.read_text(encoding='utf8'):
            raise RuntimeError('Windows release failed clean-profile startup')
    for name in ('lemonstudio.exe','ML Studio Source.zip','LICENSE','THIRD-PARTY-NOTICES.txt'):shutil.copy2(site/'updates'/v/name,payload/name)
    shutil.copytree(cfg['runtime'],payload/'runtime');shutil.copy2(r.ROOT/'crates/os-app/assets/templates/ML Mix Template.flp',payload/'ML Mix Template.lmn')
    (payload/'READ ME.txt').write_text('ML Studio — Michael Lane Edition\n'+r.BASE,encoding='utf-8')
    label=f'{v[:4]}.{v[4:6]}.{v[6:8]}.{int(v[8:])}'
    installer=site/'downloads'/v/f'ML-Studio-Windows-Setup-{label}.exe';installer.parent.mkdir(parents=True)
    command(log,cfg['nsis'],f'/DPAYLOAD={payload}',f'/DOUTPUT={installer}',r.ROOT/'packaging/windows/LemonStudio.nsi',env=env)
    (site/'platforms').mkdir(exist_ok=True)
    atomic_json(site/'platforms/windows.json',dict(version=r.version(),label='Windows',url=installer.relative_to(site).as_posix(),source=f'updates/{v}/ML Studio Source.zip',sha256=digest(installer)))
    return r.write_plan(site,out,'windows-linux')


def linux_compile(config,log):
    storage.build_preflight(Path(config['cache'])/'linux', Path(log).parent)
    env=environment(config,'linux');tools=Path(config['linux_tools'])
    # Existing cross-tool wrappers require Q:. Hold the coordinator lock, never replace another mapping.
    if Path('Q:/').exists():raise RuntimeError('Q: is occupied; preserve the existing drive mapping.')
    command(log,'subst','Q:',r.ROOT.parent)
    try:
        qtools=Path('Q:/')/tools.relative_to(r.ROOT.parent)
        env.update(CARGO_ZIGBUILD_ZIG_PATH=str(qtools/'python/ziglang/zig.exe'),CARGO_ZIGBUILD_CACHE_DIR=str(qtools/'zig-cache'),PKG_CONFIG=str(qtools/'pkg-config.cmd'),PKG_CONFIG_ALLOW_CROSS='1',PKG_CONFIG_LIBDIR=str(qtools/'sysroot/usr/lib/x86_64-linux-gnu/pkgconfig'))
        command(log,qtools/'python/bin/cargo-zigbuild.exe','zigbuild','--locked','--release','-p','os-app','--target','x86_64-unknown-linux-gnu.2.36',env=env)
    finally:command(log,'subst','Q:','/D')
    return Path(env['CARGO_TARGET_DIR'])/'x86_64-unknown-linux-gnu/release/lemonstudio'


def linux_package(config,binary,win_plan,out,log):
    out=Path(out);out.mkdir(parents=True);site=out/'lemon-studio';v=str(r.version());env=environment(config,'linux')
    win_site=Path(win_plan).parent/'lemon-studio';source=site/'updates'/v/'Lemon Studio Source.zip';source.parent.mkdir(parents=True);shutil.copy2(win_site/'updates'/v/source.name,source)
    archive=site/'linux'/v/f'ML-Studio-Linux-x86_64-{v}.tar.gz';archive.parent.mkdir(parents=True)
    with tarfile.open(archive,'w:gz') as t:
        for src,name,mode in [(Path(binary),'lemonstudio',0o755),(r.ROOT/'packaging/linux/README.txt','README.txt',0o644),(r.ROOT/'LICENSE','LICENSE',0o644),(r.ROOT/'THIRD-PARTY-NOTICES.txt','THIRD-PARTY-NOTICES.txt',0o644), *r.LINUX_LAUNCHER_FILES]:
            info=t.gettarinfo(str(src),'ML Studio/'+name);info.mode=mode;info.uid=info.gid=0;info.uname=info.gname=''
            with src.open('rb') as f:t.addfile(info,f)
    (site/'platforms').mkdir()
    # The combined Windows plan already provides the branded source download.
    atomic_json(site/'platforms/linux.json',dict(version=r.version(),label='Linux',url=archive.relative_to(site).as_posix(),source=f'updates/{v}/ML Studio Source.zip',sha256=digest(archive)))
    command(log,r.sys.executable,r.ROOT/'packaging/prepare-unix-update.py','--platform','linux-x64','--site',site,'--app',binary,env=env)
    return r.write_plan(site,out,'windows-linux')


@contextmanager
def mounted_mac_installer(dmg,out,log):
    scratch = Path(tempfile.mkdtemp(prefix='mac-smoke-',dir=out))
    mount = scratch/'installer'
    mount.mkdir()
    try:
        command(log,'hdiutil','attach','-nobrowse','-readonly','-mountpoint',mount,dmg)
        yield scratch,mount
    finally:
        for attempt in range(4):
            if not mount.is_mount():
                break
            try:
                command(log,'hdiutil','detach',mount,timeout=60)
            except (subprocess.CalledProcessError,subprocess.TimeoutExpired):
                if attempt == 3 and mount.is_mount():
                    raise
                time.sleep(2 ** attempt)
        if mount.is_mount():
            raise RuntimeError('Installer image is still mounted at '+str(mount))
        shutil.rmtree(scratch)


def verify_mac_package(dmg, update, arch, out, log):
    with mounted_mac_installer(dmg,out,log) as (scratch,mount):
        app=mount/'ML Studio.app'
        command(log,'codesign','--verify','--deep','--strict',app)
        feed=json.loads((update/'signed.json').read_text())
        files=json.loads(feed['payload'])['files']
        for entry in files:
            if any(digest(root/entry['name'])!=entry['sha256'] for root in (app,update)):
                raise RuntimeError('Installer and updater differ: '+entry['name'])
        profile=scratch/'profile'
        profile.mkdir()
        env=dict(os.environ,OPENSTUDIO_USER_DIR=str(profile),GRIDLINE_CONFIG_HOME=str(profile),OPENSTUDIO_NO_PLUGIN_BUILD='1')
        binary=app/'Contents/MacOS/lemonstudio'
        report=out/'startup-report.txt'
        command(log,'arch','-'+arch,binary,'--startup-report',report,env=env,timeout=180)
        if str(profile) not in report.read_text():
            raise RuntimeError('Mac startup did not use the clean profile')
        result=subprocess.check_output(['arch','-'+arch,binary,'--scan-plugin','au','macOS Audio Units'],env=env,text=True,timeout=30)
        scan=json.loads(next(line[7:] for line in result.splitlines() if line.startswith('OSSCAN ')))
        ids={plugin['id'] for plugin in scan.get('plugins',[])}
        if not scan['ok'] or not {'61756D75646C73206170706C','617566786C7061736170706C'}<=ids:
            raise RuntimeError('Packaged Mac app did not discover the expected Audio Units')
        atomic_json(out/'audio-unit-smoke.json',{'architecture':arch,'registered_plugins':len(ids),'installer_matches_updater':True})



def package_mac(config,arch,out,log,source):
    storage.build_preflight(Path(config['cache'])/('mac-'+arch), out)
    env=environment(config,'mac-'+arch);out=Path(out);out.mkdir(parents=True);env['LEMON_PACKAGE_OUT']=str(out)
    if source is None:
        command(log,'bash',r.ROOT/'packaging/mac/build-release.command',env=env)
    else:
        app = Path(env['CARGO_TARGET_DIR'])/'bundled/ML Studio.app'
        if env.get('ML_MAC_SIGNING_IDENTITY'):
            # Developer ID signing modifies the bundle. Preserve the checkpoint for retry.
            candidate = out/'ML Studio.app'
            command(log,'ditto',app,candidate)
            app = candidate
        command(log,'python3',r.ROOT/'packaging/mac/package-release.py',app,source,env=env)
    manifest=json.loads((out/f'mac-{arch}.json').read_text());site=out/'lemon-studio';(site/'platforms').mkdir(parents=True)
    for rel in (manifest['url'],manifest['source']):
        dest=site/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(out/Path(rel).name,dest)
    shutil.copy2(out/f'mac-{arch}.json',site/'platforms'/f'mac-{arch}.json')
    command(log,r.sys.executable,r.ROOT/'packaging/prepare-unix-update.py','--platform','mac-'+arch,'--site',site,'--app',app if source else Path(env['CARGO_TARGET_DIR'])/'bundled/ML Studio.app',env=env)
    return r.write_plan(site,out,'mac-'+arch)


def mac(config,arch,out,log,source=None,journal=None):
    def package(_):
        plan = package_mac(config,arch,out,log,source)
        return str(plan),plan_files(plan)
    if journal:
        plan = Path(journal.step('Package '+arch,package))
    else:
        plan = Path(package(1)[0])
    folder = plan.parent
    site = folder/'lemon-studio'
    manifest = json.loads((site/'platforms'/f'mac-{arch}.json').read_text())
    verify_mac_package(site/manifest['url'],site/'mac/updates'/arch/str(r.version()),arch,folder,log)
    return plan


def merged_plan(plans,out):
    out=Path(out);entries={}
    for path in plans:
        path=Path(path);p=json.loads(path.read_text())
        for e in p['files']:
            if e['path'] in entries:
                if entries[e['path']]['sha256']!=e['sha256']:raise RuntimeError('Conflicting platform files')
                continue
            local=(path.parent/e['local']).resolve().relative_to(out.resolve()).as_posix()
            entries[e['path']]=dict(e,local=local)
    p=dict(scope='windows-linux',version=r.version(),commit=r.git('rev-parse','HEAD'),files=list(entries.values()))
    dest=out/'publish-plan.json';atomic_json(dest,p);return dest
