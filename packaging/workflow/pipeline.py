"""Resumable full or macOS-only release. `check` verifies setup; `run` publishes."""
from pathlib import Path, PurePosixPath
from concurrent.futures import ThreadPoolExecutor
import argparse, datetime, hashlib, json, mmap, os, platform, re, shutil, subprocess, sys, time, urllib.request, urllib.error, urllib.parse, zipfile
import release as r
import builders as b
import storage
import discord_changelog
import download_page
from checkpoints import Journal, atomic_json, digest, exclusive, plan_files, stage_plan

CONFIG=Path(os.environ.get('ML_STUDIO_RELEASE_CONFIG', r.PRIVATE/'pipeline.json'))
PLATFORMS=download_page.PLATFORMS
MAC_PLATFORMS=PLATFORMS[2:]
WINDOWS_LINUX_LINKS=('platforms/windows.json','platforms/linux.json','updates/stable.json','updates/linux-x64/stable.json')
REPOSITORY='Dorfhaus/ML-Studio'


def config():
    if not CONFIG.exists():raise RuntimeError('Publishing pipeline is not configured on this computer: '+str(CONFIG))
    return json.loads(CONFIG.read_text(encoding='utf-8-sig'))


def source_branch():
    branch = r.git('branch', '--show-current')
    return branch if re.fullmatch(r'release/[a-z0-9-]+', branch) else 'main'


def ensure_public_tree():
    markers = r'ML Studio 30' + r'36|neon_30' + r'36|crate::synth' + r'wave|LEMON_30' + r'36'
    result = subprocess.run(
        ['git', 'grep', '-IilE', markers, '--', 'crates', 'plugins', 'packaging', 'docs'],
        cwd=r.ROOT, capture_output=True, text=True,
    )
    if result.returncode != 1:
        raise RuntimeError('Unreleased theme source found or source scan failed: ' + result.stdout.strip())


def read_url(url):
    request=urllib.request.Request(url,headers={'User-Agent':'Lemon-Studio-release','Cache-Control':'no-cache'})
    with urllib.request.urlopen(request,timeout=120) as response:return response.read()


def live_versions():
    return {p:json.loads(read_url(r.BASE+'platforms/'+p+'.json?check='+str(time.time_ns())))['version'] for p in PLATFORMS}


def engine_id():return r.git('rev-parse','HEAD:plugins/lemon-master-assistant')


def mix_reference_pack(config):
    entry = config['mix_reference_pack']
    source = Path(entry['path'])
    if not source.is_file() or digest(source) != entry['sha256']:
        raise RuntimeError('Website Mix References pack is missing or changed')
    r.verify_mix_reference_pack(source)
    return source


def probe(mac=False):
    c=config();required=PLATFORMS[2:] if mac else PLATFORMS[:2]
    if not Path(c['juce_source'],'modules/juce_core/juce_core.h').is_file():raise RuntimeError('Matching JUCE source is missing')
    for p in required:
        e=c['engines'][p];archive=Path(e['bundle'])
        if e['source']!=engine_id():raise RuntimeError('Master Assistant source changed. Rebuild and validate the '+p+' engine before publishing; cached engines were not reused.')
        if not archive.is_file() or digest(archive)!=e['sha256']:raise RuntimeError('Engine archive changed: '+p)
        with zipfile.ZipFile(archive) as z:
            m=json.loads(z.read('engine.json'));expected=('macos','aarch64' if p=='mac-arm64' else 'x86_64') if mac else (p,'x86_64')
            if (m['os'],m['arch'])!=expected:raise RuntimeError('Wrong engine architecture: '+p)
            for f in m['files']:
                if hashlib.sha256(z.read(f['path'])).hexdigest()!=f['sha256']:raise RuntimeError('Damaged native engine archive')
        b.verify_score_bundle(c, p)
    from cryptography.hazmat.primitives.serialization import load_pem_private_key,Encoding,PublicFormat
    key=r.PRIVATE/('mac-update-signing-key.pem' if mac else 'update-signing-key.pem')
    expected=r.ROOT/'crates/os-app/src'/('update-mac-public-key.hex' if mac else 'update-public-key.hex')
    public=load_pem_private_key(key.read_bytes(),None).public_key().public_bytes(Encoding.Raw,PublicFormat.Raw).hex()
    if public!=expected.read_text().strip():raise RuntimeError('Signing key does not match application')
    subprocess.run(['cargo','--version'],check=True,stdout=subprocess.DEVNULL)
    if mac:
        subprocess.run(['xcode-select','-p'],check=True,stdout=subprocess.DEVNULL)
        targets=subprocess.check_output(['rustup','target','list','--installed'],text=True)
        if not all(t in targets for t in ['aarch64-apple-darwin','x86_64-apple-darwin']):raise RuntimeError('Both Mac Rust targets must be installed')
        r.check_connection()
    else:
        b.visualizer_inputs(c)
        mix_reference_pack(c)
        cfg=r.read_config()
        for k in ('llvm','nsis','runtime'):
            if not Path(cfg[k]).exists():raise RuntimeError('Missing build tool: '+k)
        if not (Path(c['linux_tools'])/'python/bin/cargo-zigbuild.exe').is_file():raise RuntimeError('Linux cross-compiler missing')
        if Path('Q:/').exists():raise RuntimeError('Q: is occupied; the Linux worker will not replace it')
        public=Path(c['public_checkout'])
        remote=subprocess.check_output(['git','remote','get-url','github'],cwd=public,text=True).strip()
        if remote!='git@github.com:'+REPOSITORY+'.git':raise RuntimeError('Wrong GitHub publication repository')
        subprocess.run(['git','ls-remote','github','refs/heads/michael-lane'],cwd=public,check=True,stdout=subprocess.DEVNULL)
    return {'platforms':list(required),'engine_source':engine_id(),'signing_key_matches':True}


def remote(args,log,sync=False):
    import shlex
    cfg=r.read_config();repo=cfg['mac_repo'];q=shlex.quote
    prefix='export PATH="$HOME/.cargo/bin:$HOME/.lemon-studio-release/tools/bin:$PATH"; set -e; '
    branch=source_branch()
    revision=r.git('rev-parse','HEAD')
    if branch!='main':
        primary=repo
        name=branch.split('/')[-1]
        repo=primary+'-'+name
        prefix+=f'export ML_STUDIO_RELEASE_CONFIG="$HOME/.lemon-studio-release/pipeline-{name}.json"; '
        prefix+='export ML_STUDIO_FILTERED_RELEASE=1; '
        if sync:
            prefix+=f'git -C {q(primary)} fetch home {q(branch)}; '
            prefix+=f'test "$(git -C {q(primary)} rev-parse FETCH_HEAD)" = {q(revision)}; '
            prefix+=f'if test -e {q(repo)}; then test -z "$(git -C {q(repo)} status --porcelain)"; '
            prefix+=f'git -C {q(repo)} merge --ff-only {q(revision)}; '
            prefix+=f'else git -C {q(primary)} worktree add --detach {q(repo)} {q(revision)}; fi; '
    elif sync:
        prefix+=f'test -z "$(git -C {q(repo)} status --porcelain)"; git -C {q(repo)} fetch home main; git -C {q(repo)} merge --ff-only home/main; '
    prefix+=f'test -z "$(git -C {q(repo)} status --porcelain)"; test "$(git -C {q(repo)} rev-parse HEAD)" = {q(revision)}; '
    command=prefix+'exec '+shlex.join([cfg['mac_python'],repo+'/packaging/workflow/pipeline.py',*map(str,args)])
    for attempt in range(3 if args[0] in ('publish-worker','publish-files-worker') else 1):
        try:
            b.command(log,'ssh','-T',cfg['publish_via_ssh'],command);return
        except subprocess.CalledProcessError:
            if attempt==2 or args[0] not in ('publish-worker','publish-files-worker'):raise
            time.sleep(5*(attempt+1))


sys.path.insert(0, str(r.ROOT / 'packaging'))
from maintenance_release import is_maintenance_build


def release_notes():
    if is_maintenance_build(r.ROOT, r.version()):
        return {'title': 'Maintenance hotfix', 'changes': ['Maintenance hotfix.']}
    history=json.loads((r.ROOT/'packaging/changelog.json').read_text(encoding='utf-8'))
    sections=[s for s in history['sections'] if str(r.version()) in s['title']]
    if not sections or not sections[0]['changes']:raise RuntimeError('Add accurate changelog notes for this build before publishing')
    return sections[0]


def prepare(notes, no_changelog=False, mac_only=False):
    live=live_versions();v=r.version()
    if no_changelog:
        v=max(v,max(live.values())+1,int(datetime.date.today().strftime('%Y%m%d')+'01'))
        version_path=r.ROOT/'crates/os-app/src/updates.rs'
        version_path.write_text(re.sub(r'pub const VERSION: u64 = \d+;',f'pub const VERSION: u64 = {v};',version_path.read_text(encoding='utf-8')),encoding='utf-8')
        label=f'{str(v)[:4]}.{str(v)[4:6]}.{str(v)[6:8]}.{int(str(v)[8:])}'
        installers=[]
        if not mac_only:
            installer=r.ROOT/'packaging/windows/LemonStudio.nsi'
            installer.write_text(re.sub(r'("DisplayVersion" ")[^"]+',lambda m:m[1]+label,installer.read_text(encoding='utf-8')),encoding='utf-8')
            installers.append(installer)
        marker=r.ROOT/'packaging/maintenance-build.txt'
        marker.write_text(f'{v}\n',encoding='utf-8')
        r.run('git','add',version_path,marker,*installers)
        r.run('git','commit','-m',f'Prepare maintenance release {v}')
    elif notes:
        n=json.loads(Path(notes).read_text(encoding='utf-8-sig'))
        if not isinstance(n.get('title'),str) or not n['title'].strip() or not n.get('changes') or not all(isinstance(x,str) and x.strip() for x in n['changes']):raise RuntimeError('Notes need a title and a list of accurate user-facing changes')
        v=max(v,max(live.values())+1,int(datetime.date.today().strftime('%Y%m%d')+'01'))
        history_path=r.ROOT/'packaging/changelog.json';history=json.loads(history_path.read_text(encoding='utf-8'))
        if any(str(v) in section['title'] for section in history['sections']):raise RuntimeError('This build already has release notes; run without --notes')
        p=r.ROOT/'crates/os-app/src/updates.rs';p.write_text(re.sub(r'pub const VERSION: u64 = \d+;',f'pub const VERSION: u64 = {v};',p.read_text(encoding='utf-8')),encoding='utf-8')
        label=f'{str(v)[:4]}.{str(v)[4:6]}.{str(v)[6:8]}.{int(str(v)[8:])}'
        installers=[]
        if not mac_only:
            p=r.ROOT/'packaging/windows/LemonStudio.nsi';p.write_text(re.sub(r'("DisplayVersion" ")[^"]+',lambda m:m[1]+label,p.read_text(encoding='utf-8')),encoding='utf-8')
            installers.append(p)
        p=r.ROOT/'packaging/changelog.json';history=json.loads(p.read_text(encoding='utf-8'))
        if any(str(v) in s['title'] for s in history['sections']):raise RuntimeError('This build already has release notes; preserve them and run without --notes')
        history['sections'].insert(0,dict(date=datetime.date.today().isoformat(),title=n['title']+' · public build '+str(v),changes=n['changes']))
        atomic_json(p,history)
        r.run(sys.executable,r.ROOT/'packaging/render-changelog.py')
        r.run('git','add','crates/os-app/src/updates.rs','packaging/changelog.json','packaging/download-page.html',*installers)
        r.run('git','commit','-m',f'Prepare {"macOS" if mac_only else "complete"} release {v}')
    elif v<=max(live.values()):raise RuntimeError('This build is already published. Supply --notes with the new changes to prepare a new release.')
    release_notes()
    r.run('git','push','--receive-pack=git -c receive.autogc=false receive-pack','home','HEAD:refs/heads/'+source_branch())


def identity():
    c=config()
    return {'version':r.version(),'commit':r.git('rev-parse','HEAD'),'engine_source':engine_id(),'configuration':hashlib.sha256(json.dumps(c,sort_keys=True).encode()).hexdigest()}


def output_root():return storage.app_root()/'Lemon Studio Public Download'/str(r.version())/'Pipeline'


def privacy(plan):
    if os.environ.get('ML_STUDIO_FILTERED_RELEASE') == '1':
        ensure_public_tree()
        markers = (b'ML Studio '+b'3036', b'neon_'+b'3036', b'appearance_ml_studio_'+b'3036')
        for path in plan_files(plan)[1:]:
            if path.name in ('lemonstudio', 'lemonstudio.exe'):
                with path.open('rb') as f, mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as data:
                    if any(data.find(marker)>=0 for marker in markers):
                        raise RuntimeError('Unreleased theme marker in '+path.name)
            elif path.name.endswith('Studio Source.zip'):
                with zipfile.ZipFile(path) as source:
                    for member in source.infolist():
                        if member.filename.endswith(('.rs', '.md', '.json', '.cpp', '.h')):
                            data=source.read(member)
                            if any(marker in data for marker in markers):
                                raise RuntimeError('Unreleased theme marker in '+member.filename)
    pack=Path(os.environ.get('LOCALAPPDATA',''))/'Lemon Studio/Lemon Wave/Included Library.lwpack'
    if pack.is_file():
        data=pack.read_bytes()
        for p in plan_files(plan)[1:]:
            if p.name in ('lemonstudio','lemonstudio.exe'):
                with p.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as m:
                    if m.find(data)>=0:raise RuntimeError('Personal Wave library found in public binary; publication stopped')


def shared_run():return r.shared_folder()/'pipeline'/str(r.version())/r.git('rev-parse','HEAD')[:12]


def stage(path,name):
    privacy(path)
    dest, verified = stage_plan(path,shared_run()/name,with_proof=True)
    return str(dest),verified


def worker(arch,mac_only=False):
    if platform.system()!='Darwin':raise RuntimeError('Mac worker requires macOS')
    root=output_root()/('mac-'+arch);root.mkdir(parents=True,exist_ok=True)
    with exclusive(r.PRIVATE/('pipeline-mac-'+arch+'.lock')), storage.active_release(root.parent.parent):
        j=Journal(root/'state.json',identity())
        def build(attempt):
            env = b.environment(config(), 'mac-'+arch)
            target = Path(env['CARGO_TARGET_DIR'])
            storage.build_preflight(target, root)
            triple = 'aarch64' if arch == 'arm64' else arch
            binary = target/(triple+'-apple-darwin')/'release/lemonstudio'
            app = target/'bundled/ML Studio.app'
            def compile_app(_):
                b.command(root/'build.log','bash',r.ROOT/'packaging/mac/build-release.command','compile',env=env)
                return str(binary), [binary]
            def bundle_app(_):
                b.command(root/'build.log','bash',r.ROOT/'packaging/mac/build-release.command','bundle',env=env)
                return str(app), [p for p in app.rglob('*') if p.is_file()]
            j.step('Compile '+arch,compile_app)
            j.step('Bundle '+arch,bundle_app)
            # Both architectures ship the same corresponding Mac source.
            folder = root.parent/'mac-source'
            folder.mkdir(exist_ok=True)
            import fcntl
            with (folder/'export.lock').open('ab') as lock:
                fcntl.flock(lock,fcntl.LOCK_EX)
                source_journal = Journal(folder/'state.json',identity())
                def export(_):
                    archive = folder/'source.zip'
                    b.command(root/'build.log',sys.executable,r.ROOT/'packaging/export-source.py',archive,'--public',env=env)
                    return str(archive), [archive]
                source = source_journal.step('Export public Mac source',export)
            plan = b.mac(config(), arch, root/f'build-{attempt}', root/'build.log',source,j)
            return str(plan), plan_files(plan)
        plan=j.step('Build mac-'+arch,build)
        if r.git('status','--porcelain') or r.git('rev-parse','HEAD')!=j.data['identity']['commit']:raise RuntimeError('Mac source changed during build; publication stopped')
        j.step('Stage mac-'+arch,lambda _:stage(plan,'mac-'+arch))


def publish_shared(name, immutable_only=False):
    path=shared_run()/name/'publish-plan.json';r.publish(path,immutable_only)


def artifacts(plans,platforms=PLATFORMS):
    index={}
    for path in plans:
        p=json.loads(Path(path).read_text())
        for e in p['files']:index[e['path']]=(Path(path).parent/e['local'],e['sha256'])
    assets=[]
    for name in platforms:
        manifest=json.loads(index['platforms/'+name+'.json'][0].read_text())
        for rel in (manifest['url'],manifest['source']):
            f,h=index[rel]
            if any(a['path']==rel for a in assets):continue
            dest = f'ML-Studio-Source-{r.version()}.zip' if f.name in ('ML Studio Source.zip','Lemon Studio Source.zip') else f.name
            if dest.startswith('Lemon-Studio-'):
                dest = 'ML-Studio-' + dest.removeprefix('Lemon-Studio-')
            assets.append(dict(name=dest,url=r.BASE+urllib.parse.quote(rel),sha256=h,path=rel,local=str(f)))
    for rel,(f,h) in index.items():
        if rel.startswith('helper/') and rel.endswith('.zip'):
            assets.append(dict(name=f.name,url=r.BASE+urllib.parse.quote(rel),sha256=h,path=rel,local=str(f)))
    return index,assets


def render_website(plans, mac_only):
    platforms = MAC_PLATFORMS if mac_only else PLATFORMS
    index, _ = artifacts(plans, platforms)
    manifests = {}
    for name in PLATFORMS:
        rel = 'platforms/' + name + '.json'
        raw = index[rel][0].read_bytes() if name in platforms else read_url(r.BASE + rel + '?verify=' + str(time.time_ns()))
        manifests[name] = json.loads(raw)
    template = (r.ROOT/'packaging/download-page.html').read_bytes()
    return download_page.render(template, manifests, r.DOWNLOAD_PAGE)


def verify_existing_helper(entry, previous):
    url, expected = entry['url'], entry['sha256']
    headers = {'Cache-Control':'no-cache'}
    etag = previous.get('etag') if previous.get('sha256') == expected else None
    if etag:
        headers['If-None-Match'] = etag
    try:
        with urllib.request.urlopen(urllib.request.Request(url,headers=headers),timeout=120) as response:
            if hashlib.file_digest(response,'sha256').hexdigest() != expected:
                return None
            tag = response.headers.get('ETag','')
            return {'sha256':expected,'etag':tag if tag.startswith('"') else None}
    except urllib.error.HTTPError as error:
        if error.code == 304 and etag:
            return previous
        if error.code == 404:
            return None
        raise


def helper_stage(out):
    """Require the exact optional packages pinned in the signed DAW source."""
    catalog=json.loads((r.ROOT/'packaging/helper/catalog.json').read_text())
    folder=out/'helper';site=folder/'lemon-studio'
    entries = []
    cache_file = r.PRIVATE/'helper-http-validators.json'
    previous = json.loads(cache_file.read_text()) if cache_file.exists() else {}
    for platform in PLATFORMS:
        spec=catalog[platform];entry=spec['archive'];source=storage.app_root()/'Helper Release'/entry['name']
        storage.free_space(source)
        if not source.is_file() or source.stat().st_size!=entry['size'] or digest(source)!=entry['sha256']:
            raise RuntimeError('Missing or changed Melli package: '+str(source))
        rel=f"helper/{spec['version']}/{entry['name']}"
        if entry['url']!=r.BASE+rel:raise RuntimeError('Melli package URL differs from publication target')
        target=site/rel;target.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(source,target)
        entries.append(entry)
    plan = r.write_plan(site,folder,'helper')
    with ThreadPoolExecutor(max_workers=4) as pool:
        checked = list(pool.map(lambda e:verify_existing_helper(e,previous.get(e['url'],{})),entries))
    if all(checked):
        atomic_json(cache_file,{e['url']:v for e,v in zip(entries,checked)})
        metadata = json.loads(plan.read_text())
        atomic_json(folder/'PUBLISHED.json',dict(metadata,verified_existing=True))
        return str(plan), [*plan_files(plan),folder/'PUBLISHED.json']
    return stage(plan,'helper')


def download_digests(paths):
    return {rel:hashlib.sha256(read_url(r.BASE+rel+'?verify='+str(time.time_ns()))).hexdigest() for rel in paths}


def verify_live(plans,out,website_plan,protected=None):
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
    mac_only=protected is not None
    platforms=MAC_PLATFORMS if mac_only else PLATFORMS
    index,assets=artifacts(plans,platforms)
    feeds=['mac/updates/arm64/stable.json','mac/updates/x86_64/stable.json']
    if not mac_only: feeds+=['updates/stable.json','updates/linux-x64/stable.json']
    for rel in ['platforms/'+p+'.json' for p in platforms]+feeds:
        data=read_url(r.BASE+rel+'?verify='+str(time.time_ns()))
        if hashlib.sha256(data).hexdigest()!=index[rel][1]:raise RuntimeError('Live feed or manifest differs: '+rel)
        value=json.loads(data)
        if 'payload' in value:
            key=r.ROOT/'crates/os-app/src'/('update-mac-public-key.hex' if rel.startswith('mac/') else 'update-public-key.hex')
            Ed25519PublicKey.from_public_bytes(bytes.fromhex(key.read_text().strip())).verify(bytes.fromhex(value['signature']),value['payload'].encode())
            value=json.loads(value['payload'])
        if value['version']!=r.version():raise RuntimeError('Wrong live release version')
    live_page = read_url(r.DOWNLOAD_PAGE+'?verify='+str(time.time_ns()))
    live_manifests = {name: json.loads(read_url(r.BASE+'platforms/'+name+'.json?verify='+str(time.time_ns()))) for name in PLATFORMS}
    if download_page.render(live_page, live_manifests, r.DOWNLOAD_PAGE) != live_page:
        raise RuntimeError('Live download buttons differ from the published manifests')
    website = json.loads(Path(website_plan).read_text())
    if hashlib.sha256(live_page).hexdigest() != next(e['sha256'] for e in website['files'] if e['path']=='index.html'):
        raise RuntimeError('Live download page differs from the staged website')
    reference_hash = None
    if mac_only:
        if protected != download_digests(WINDOWS_LINUX_LINKS):
            raise RuntimeError('Windows or Linux public feed or manifest changed during the Mac release')
    else:
        reference_hash = next(entry['sha256'] for entry in website['files'] if entry['path'] == r.MIX_REFERENCE_PATH)
        reference = read_url(r.BASE+urllib.parse.quote(r.MIX_REFERENCE_PATH)+'?verify='+str(time.time_ns()))
        if hashlib.sha256(reference).hexdigest() != reference_hash:
            raise RuntimeError('Website Mix References download differs')
    # Artifact bytes were verified over HTTPS by each publisher before its manifest became live.
    for path in [*plans, website_plan]:
        p=json.loads(Path(path).read_text());receipt=json.loads(Path(path).with_name('PUBLISHED.json').read_text())
        if any(receipt[k]!=p[k] for k in ('version','commit','scope','files')):raise RuntimeError('Publication receipt does not match prepared artifacts')
    for a in assets:
        dest=out/'Downloads'/a['name'];dest.parent.mkdir(exist_ok=True)
        if not dest.exists():shutil.copyfile(a['local'],dest)
        if digest(dest)!=a['sha256']:raise RuntimeError('Desktop artifact hash differs')
    notes=release_notes();v=str(r.version());label=f'{v[:4]}.{v[4:6]}.{v[6:8]}.{int(v[8:])}'
    plan=dict(tag='v'+label,title='ML Studio '+label,notes='\n'.join('- '+x for x in notes['changes'])+'\n\n'+r.DOWNLOAD_PAGE,assets=[{k:a[k] for k in ('name','url','sha256')} for a in assets])
    if mac_only:
        plan.update(title=plan['title']+' for macOS', latest=False)
    atomic_json(out/'github-release.json',plan);atomic_json(out/'VERIFIED.json',{'version':r.version(),'platforms':list(platforms),'signatures':len(feeds),'download_hashes_verified_by_publishers':True,'website_exact':True,'website_mix_references_sha256':reference_hash,'protected_platform_manifests_and_feeds':protected})
    return str(out/'github-release.json'),[out/'github-release.json',out/'VERIFIED.json']+[out/'Downloads'/a['name'] for a in assets]


def github_stage(plan_path,plans,out,attempt,force=False):
    plan=json.loads(Path(plan_path).read_text());c=config();public=Path(c['public_checkout'])
    b.command(out/'github.log','git','fetch','github','michael-lane',cwd=public)
    previous=subprocess.check_output(['git','show','FETCH_HEAD:packaging/github-release.json'],cwd=public,text=True)
    previous=json.loads(previous);previous.pop('publication_attempt',None)
    head=subprocess.check_output(['git','rev-parse','FETCH_HEAD'],cwd=public,text=True).strip()
    if previous==plan and not force:return {'tag':plan['tag'],'commit':head},[Path(plan_path)]
    if force and previous!=plan:raise RuntimeError('Another release changed the GitHub branch; refusing to replace it')
    work=Path(c['cache'])/('github-'+str(r.version())+'-'+str(attempt))
    if work.exists():raise RuntimeError('Preserving existing GitHub worktree; retry creates a fresh attempt')
    b.command(out/'github.log','git','worktree','add','--detach',work,'FETCH_HEAD',cwd=public)
    if plan.get('latest') is False:
        index,_=artifacts(plans,MAC_PLATFORMS)
        manifest=json.loads(index['platforms/mac-arm64.json'][0].read_text())
        source=index[manifest['source']][0]
    else:
        index,_=artifacts(plans)
        manifest=json.loads(index['platforms/windows.json'][0].read_text())
        source=index[manifest['source']][0]
    expected=set()
    with zipfile.ZipFile(source) as z:
        for entry in z.infolist():
            if entry.is_dir():continue
            if not entry.filename.startswith('Source/'):raise RuntimeError('Unexpected source archive layout')
            name=entry.filename[7:];p=(work/name).resolve()
            if not p.is_relative_to(work.resolve()) or '.git' in Path(name).parts:raise RuntimeError('Unsafe source archive path')
            expected.add(name);p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(z.read(entry))
    tracked=subprocess.check_output(['git','ls-files','-z'],cwd=work).decode().split('\0')
    for name in tracked:
        if name and name not in expected:(work/name).unlink(missing_ok=True)
    atomic_json(work/'packaging/github-release.json',dict(plan,publication_attempt=attempt) if force else plan)
    b.command(out/'github.log','git','add','-A',cwd=work)
    b.command(out/'github.log',sys.executable,'packaging/private_build.py','--check-source',cwd=work)
    b.command(out/'github.log','git','commit','-m','Publish '+plan['title'],cwd=work)
    b.command(out/'github.log','git','push','github','HEAD:michael-lane',cwd=work)
    head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=work,text=True).strip()
    return {'tag':plan['tag'],'commit':head},[Path(plan_path)]


class GitHubJobFailed(RuntimeError):pass

def github_verify(plan_path,out,commit=None):
    plan=json.loads(Path(plan_path).read_text());url='https://api.github.com/repos/'+REPOSITORY+'/releases/tags/'+plan['tag']
    for attempt in range(30):
        try:
            release=json.loads(read_url(url))
            if not release['draft']:break
        except urllib.error.HTTPError as e:
            if e.code not in (404,429,502,503):raise
        if commit and attempt%4==0:
            runs=json.loads(read_url('https://api.github.com/repos/'+REPOSITORY+'/actions/workflows/publish-edition.yml/runs?branch=michael-lane&per_page=10'))
            matching=[x for x in runs.get('workflow_runs',[]) if x['head_sha']==commit]
            if matching and matching[0]['conclusion'] in ('failure','timed_out'):
                raise GitHubJobFailed('GitHub publication workflow failed; prepared downloads are preserved')
            if matching and matching[0]['conclusion']=='cancelled':raise RuntimeError('GitHub publication was cancelled; inspect it before resuming')
        time.sleep(min(10+attempt*5,45))
    else:raise RuntimeError('GitHub release is not complete. Check Actions, then resume this run.')
    expected={a['name']:'sha256:'+a['sha256'] for a in plan['assets']}
    expected['SHA256SUMS.txt']='sha256:'+hashlib.sha256(''.join(a['sha256']+'  '+a['name']+'\n' for a in plan['assets']).encode()).hexdigest()
    actual={a['name']:a.get('digest') for a in release['assets']}
    if expected!=actual:raise RuntimeError('GitHub assets differ from verified website downloads')
    dest=out/'GITHUB-VERIFIED.json';atomic_json(dest,dict(url=release['html_url'],assets=actual,tag=plan['tag']))
    return release['html_url'],[dest]


def coordinator(notes=None,check=False,no_changelog=False,mac_only=False):
    if platform.system()!='Windows':raise RuntimeError('Run the complete release from the Windows coordinator; this Mac provides native workers.')
    with exclusive(r.PRIVATE/'pipeline.lock'):
        if r.git('status','--porcelain'):raise RuntimeError('Commit intended source changes first; nothing was stashed or reset')
        branch=source_branch()
        if branch=='main':
            r.run('git','fetch','home','main')
            r.run('git','merge','--ff-only','home/main')
        else:
            ensure_public_tree()
            os.environ['ML_STUDIO_FILTERED_RELEASE']='1'
        r.run('git','push','--receive-pack=git -c receive.autogc=false receive-pack','home','HEAD:refs/heads/'+branch)
        c=config();prelog=r.PRIVATE/'pipeline-preflight.log'
        probe();remote(['probe'],prelog,sync=True)
        if check:
            print('Preflight passed: four architectures, engine fingerprints, build tools, signing keys, GitHub and hosting access. Nothing published.');return
        out=output_root();state=out/'state.json'
        previous=json.loads(state.read_text()) if state.exists() else None
        if previous and previous['identity']['commit']==r.git('rev-parse','HEAD'):
            if notes or no_changelog:raise RuntimeError('Resume this reserved run with plain run')
            pinned_mac=previous['identity'].get('mac_only',False)
            if mac_only and not pinned_mac:raise RuntimeError('This run already has the full release scope')
            mac_only=pinned_mac
        else:
            if previous and previous['steps'].get('Back up source',{}).get('status')!='complete':raise RuntimeError('An unfinished release belongs to another revision; resume that revision first')
            if mac_only:
                prepare(notes,no_changelog=no_changelog,mac_only=True)
            elif no_changelog:
                prepare(None, no_changelog=True)
            else:
                prepare(notes)
            out=output_root();state=out/'state.json'
        release_notes()
        run_identity=identity()
        if mac_only:run_identity['mac_only']=True
        out.mkdir(parents=True,exist_ok=True);j=Journal(state,run_identity)
        remote(['probe'],prelog,sync=True)
        def win(attempt):
            p=b.windows(c,out/f'windows-{attempt}',out/'windows.log');return str(p),plan_files(p)
        def lin(_):
            p=b.linux_compile(c,out/'linux.log');return str(p),[p]
        def preupload(plan):
            scope = json.loads(Path(plan).read_text())['scope']
            def files(_):
                if r.git('status','--porcelain') or r.git('rev-parse','HEAD') != j.data['identity']['commit']:
                    raise RuntimeError('Source changed before uploading versioned files')
                remote(['publish-files-worker',scope],out/('files-'+scope+'.log'))
                proof = Path(plan).with_name('FILES-VERIFIED.json')
                return str(proof), [proof]
            j.step('Upload versioned files '+scope,files)
        def mac(arch):
            def action(_):
                args=['worker',arch]+(['--mac-only'] if mac_only else [])
                remote(args,out/('mac-'+arch+'.log'));p=shared_run()/('mac-'+arch)/'publish-plan.json';return str(p),plan_files(p)
            plan = j.step('Build mac-'+arch,action)
            preupload(plan)
            return plan
        helper = None
        with storage.active_release(out.parent), ThreadPoolExecutor(max_workers=5) as pool:
            if not mac_only:
                winjob = pool.submit(j.step,'Build Windows',win)
                linjob = pool.submit(j.step,'Build Linux',lin)
                helperjob = pool.submit(j.step,'Stage optional Melli',lambda _:helper_stage(out))
            macjobs = [pool.submit(mac,a) for a in ('arm64','x86_64')]
            if not mac_only:
                wp, binary = winjob.result(), linjob.result()
                def package(attempt):
                    p=b.linux_package(c,binary,wp,out/f'linux-{attempt}',out/'linux.log');return str(p),plan_files(p)
                lp=j.step('Package Linux',package)
                def merged(_):
                    p=b.merged_plan([wp,lp],out);return str(p),plan_files(p)
                combined=j.step('Combine Windows and Linux',merged)
                win_shared=j.step('Stage Windows and Linux',lambda _:stage(combined,'windows-linux'))
                preupload(win_shared)
                helper = helperjob.result()
            macplans = [job.result() for job in macjobs]
        plans = macplans if mac_only else [win_shared,*macplans]
        def website(_):
            folder=out/'website';site=folder/'lemon-studio';site.mkdir(parents=True,exist_ok=True);(site/'index.html').write_bytes(render_website(plans, mac_only))
            if not mac_only:
                target = site / r.MIX_REFERENCE_PATH
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(mix_reference_pack(c), target)
            p=r.write_plan(site,folder,'website');return stage(p,'website')
        webplan=j.step('Stage website',website)
        protected=None
        if mac_only:
            def capture(_):
                before=download_digests(WINDOWS_LINUX_LINKS)
                dest=out/'windows-linux-before.json';atomic_json(dest,before)
                return before,[dest]
            protected=j.step('Record Windows and Linux feeds',capture)
        if r.git('status','--porcelain') or r.git('rev-parse','HEAD')!=j.data['identity']['commit']:raise RuntimeError('Source changed during the release; no packages will be published')
        def publish(p):
            name=json.loads(Path(p).read_text())['scope']
            def action(_):
                receipt=Path(p).with_name('PUBLISHED.json')
                if name != 'helper' or Path(p).parent != out/'helper':
                    remote(['publish-worker',name],out/('publish-'+name+'.log'))
                return str(receipt),[receipt]
            return j.step('Publish '+name,action)
        # All optional files must be live before any DAW feed advertises this build.
        if helper:publish(helper)
        with ThreadPoolExecutor(max_workers=3) as pool:list(pool.map(publish,plans))
        publish(webplan)
        if helper:plans=[*plans,helper]
        def verify(_):
            if mac_only:return verify_live(plans,out,webplan,protected)
            return verify_live(plans,out,webplan)
        gp=j.step('Verify live downloads and feeds',verify)
        publication=j.step('Publish GitHub',lambda attempt:github_stage(gp,plans,out,attempt))
        def finish_github(attempt):
            commit=publication['commit']
            for retry in range(3):
                try:return github_verify(gp,out,commit)
                except GitHubJobFailed:
                    if retry==2:raise
                    result,_=github_stage(gp,plans,out,1000+attempt*10+retry,force=True)
                    commit=result['commit']
        j.step('Verify GitHub',finish_github)
        def post_discord(_):
            result = discord_changelog.publish(
                discord_changelog.bot_api(c['source_sync']),
                r.version(), release_notes(), r.DOWNLOAD_PAGE,
            )
            receipt = out / 'DISCORD-VERIFIED.json'
            atomic_json(receipt, result)
            return result, [receipt]
        maintenance = is_maintenance_build(r.ROOT, r.version())
        if not maintenance:
            j.step('Post Discord changelog', post_discord)
        def backup(_):
            if branch!='main':
                index,_=artifacts(plans, MAC_PLATFORMS if mac_only else PLATFORMS)
                key='mac-arm64' if mac_only else 'windows'
                manifest=json.loads(index[f'platforms/{key}.json'][0].read_text())
                source=index[manifest['source']][0]
                relative=Path('releases')/(str(r.version())+'-public')/'ML Studio Source.zip'
                dest=r.shared_folder()/relative
                dest.parent.mkdir(parents=True,exist_ok=True)
                if dest.exists():
                    if digest(source)!=digest(dest):raise RuntimeError('Public backup already exists with different content')
                else:
                    pending=dest.with_suffix('.pending')
                    shutil.copyfile(source,pending)
                    if digest(source)!=digest(pending):raise RuntimeError('Public backup checksum differs')
                    os.replace(pending,dest)
                latest=dict(version=r.version(),source=relative.as_posix(),sha256=digest(dest),
                    commit=r.git('rev-parse','HEAD'),github=json.loads((out/'GITHUB-VERIFIED.json').read_text())['url'])
                atomic_json(r.shared_folder()/'LATEST-PUBLIC.json',latest)
                receipt=out/'BACKUP-VERIFIED.json';atomic_json(receipt,latest)
                return str(dest),[dest,receipt]
            env=dict(os.environ,LEMON_JUCE_SOURCE=c['juce_source'],PYTHONUTF8='1')
            b.command(out/'backup.log','powershell','-NoProfile','-ExecutionPolicy','Bypass','-File',c['source_sync'],'-Source',r.ROOT,env=env)
            latest=json.loads((r.shared_folder()/'LATEST.json').read_text(encoding='utf-8-sig'));p=r.shared_folder()/latest['source']
            if digest(p)!=latest['sha256']:raise RuntimeError('Source backup hash differs')
            latest.update(mac_status='Both Mac architectures published and verified',commit=j.data['identity']['commit'],github=json.loads((out/'GITHUB-VERIFIED.json').read_text())['url'])
            atomic_json(r.shared_folder()/'LATEST.json',latest)
            receipt=out/'BACKUP-VERIFIED.json';atomic_json(receipt,latest);return str(p),[p,receipt]
        j.step('Back up source',backup)
        def clean_mac(_):
            remote(['finish-mac'],out/'mac-retention.log')
            return 'Mac installers retained', [out/'BACKUP-VERIFIED.json']
        j.step('Clean published Mac workspaces', clean_mac)
        discord = '' if maintenance else ', Discord changelog'
        scope = 'both Mac architectures' if mac_only else 'Windows, Linux, both Mac architectures'
        print('COMPLETE: website, '+scope+', GitHub, signed feeds'+discord+' and backup verified. '+str(out),flush=True)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('action',choices=['run','check','status','probe','worker','publish-worker','publish-files-worker','finish-mac']);parser.add_argument('worker',nargs='?');options=parser.add_mutually_exclusive_group();options.add_argument('--notes',type=Path);options.add_argument('--no-changelog',action='store_true');parser.add_argument('--mac-only',action='store_true');a=parser.parse_args()
    if a.no_changelog and a.action!='run':parser.error('--no-changelog requires run')
    if a.mac_only and a.action not in ('run','worker'):parser.error('--mac-only requires run')
    if a.action=='probe':print(json.dumps(probe(mac=platform.system()=='Darwin')))
    elif a.action=='worker':
        if a.worker not in ('arm64','x86_64'):parser.error('Choose arm64 or x86_64')
        worker(a.worker,a.mac_only)
    elif a.action in ('publish-worker','publish-files-worker'):
        if a.worker not in ('windows-linux','mac-arm64','mac-x86_64','website','helper'):parser.error('Invalid scope')
        publish_shared(a.worker,a.action=='publish-files-worker')
    elif a.action=='finish-mac':
        from mac_retention import finish
        finish()
    elif a.action=='status':
        p=output_root()/'Progress.txt';print(p.read_text(encoding='utf-8') if p.exists() else 'No pipeline run for the current source version.')
    else:coordinator(a.notes,a.action=='check',a.no_changelog,a.mac_only)

if __name__=='__main__':
    try:main()
    except Exception as error:print('STOP: '+str(error),file=sys.stderr);sys.exit(1)
