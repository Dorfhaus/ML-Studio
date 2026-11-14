"""Build relocatable optional Melli packages; dependencies are pinned by SHA-256."""
from pathlib import Path, PurePosixPath
import argparse, hashlib, json, os, posixpath, shutil, stat, tarfile, urllib.request, zipfile
ROOT=Path(__file__).resolve().parents[2]
LOCK=Path(__file__).with_name('sources.json')
PLATFORMS=['windows','linux','mac-arm64','mac-x86_64']
VERSION=2

def sha(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  while b:=f.read(1024*1024):h.update(b)
 return h.hexdigest()

def get_json(url):
 with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'ML-Studio-Melli-build'}),timeout=120) as r:return json.load(r)

def lock_sources():
 py=get_json('https://api.github.com/repos/astral-sh/python-build-standalone/releases/tags/20261110')
 ll=get_json('https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/b12378')
 qt={name:get_json('https://pypi.org/pypi/'+name+'/6.12.0/json') for name in ['PySide6-Essentials','shiboken6']}
 def asset(data,name):
  a=next(x for x in data['assets'] if x['name']==name)
  digest=a.get('digest','').removeprefix('sha256:')
  if len(digest)!=64:raise RuntimeError('Missing upstream digest: '+name)
  return dict(name=name,url=a['browser_download_url'],sha256=digest,size=a['size'])
 targets={'windows':'x86_64-pc-windows-msvc','linux':'x86_64-unknown-linux-gnu','mac-arm64':'aarch64-apple-darwin','mac-x86_64':'x86_64-apple-darwin'}
 runtimes={'windows':['win-vulkan-x64.zip','win-cpu-x64.zip'],'linux':['ubuntu-vulkan-x64.tar.gz','ubuntu-x64.tar.gz'],'mac-arm64':['macos-arm64.tar.gz'],'mac-x86_64':['macos-x64.tar.gz']}
 data={'version':VERSION,'platforms':{}}
 for p in PLATFORMS:
  tag='win_amd64' if p=='windows' else 'manylinux_2_34_x86_64' if p=='linux' else 'macosx_14_0_universal2'
  wheels=[]
  for name,d in qt.items():
   f=next(x for x in d['urls'] if tag in x['filename'])
   wheels.append(dict(name=f['filename'],url=f['url'],sha256=f['digests']['sha256'],size=f['size']))
  data['platforms'][p]={'python':asset(py,'cpython-3.12.16+20261110-'+targets[p]+'-install_only_stripped.tar.gz'),
    'runtimes':[asset(ll,'llama-b12378-bin-'+x) for x in runtimes[p]],'wheels':wheels}
 data['model']={'name': 'Qwen3.5-0.8B-UD-Q3_K_XL.gguf', 'url': 'https://huggingface.co/unsloth/Qwen3.5-0.8B-GGUF/resolve/6ab461498e2023f6e3c1baea90a8f0fe38ab64d0/Qwen3.5-0.8B-UD-Q3_K_XL.gguf', 'sha256': '30a1353e67c9795e375dac8a19d25429ace0ec2b6ca55436406ee02d2603a218', 'size': 492216576}
 LOCK.write_text(json.dumps(data,indent=2)+'\n',encoding='utf-8')

def download(entry,cache):
 p=cache/entry['name']
 if p.is_file() and p.stat().st_size==entry['size'] and sha(p)==entry['sha256']:return p
 temp=p.with_suffix(p.suffix+'.part')
 with urllib.request.urlopen(urllib.request.Request(entry['url'],headers={'User-Agent':'ML-Studio-Melli-build'}),timeout=120) as r,temp.open('wb') as f:shutil.copyfileobj(r,f,1024*1024)
 if temp.stat().st_size!=entry['size'] or sha(temp)!=entry['sha256']:raise RuntimeError('Download checksum failed: '+entry['name'])
 os.replace(temp,p);return p

def safe_name(name):
 p=PurePosixPath(name)
 if p.is_absolute() or '..' in p.parts or '\\' in name or ':' in name:raise RuntimeError('Unsafe archive path: '+name)
 return p

def unpack(archive,dest):
 dest.mkdir(parents=True,exist_ok=True)
 if archive.suffix in ('.zip','.whl'):
  with zipfile.ZipFile(archive) as z:
   for item in z.infolist():
    p=dest/str(safe_name(item.filename))
    if item.is_dir():continue
    content=z.read(item)
    mode=(item.external_attr>>16)&0xffff
    if stat.S_ISLNK(mode):
     target=posixpath.normpath(posixpath.join(posixpath.dirname(item.filename),content.decode()))
     safe_name(target);content=z.read(target);mode=(z.getinfo(target).external_attr>>16)&0xffff
    p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(content)
    p.chmod(0o755 if mode&0o111 else 0o644)
 else:
  with tarfile.open(archive) as t:
   members={m.name:m for m in t.getmembers()}
   def resolve(m,seen=()):
    if m.name in seen:raise RuntimeError('Cyclic archive link')
    if m.issym() or m.islnk():
     target=posixpath.normpath(posixpath.join(posixpath.dirname(m.name),m.linkname) if m.issym() else m.linkname)
     safe_name(target);return resolve(members[target],(*seen,m.name))
    if not m.isfile():raise RuntimeError('Unsupported archive member')
    return m
   for m in members.values():
    safe_name(m.name)
    if m.isdir():continue
    original=resolve(m);p=dest/m.name;p.parent.mkdir(parents=True,exist_ok=True)
    with t.extractfile(original) as stream,p.open('wb') as out:shutil.copyfileobj(stream,out)
    p.chmod(0o755 if original.mode&0o111 else 0o644)

def build(platform,out):
 lock=json.loads(LOCK.read_text());config=lock['platforms'][platform]
 cache=out/'downloads';cache.mkdir(parents=True,exist_ok=True)
 dest=out/'packages'/('v'+str(VERSION))/platform;dest.mkdir(parents=True,exist_ok=True)
 if shutil.disk_usage(out).free<20*1024**3:raise RuntimeError('Need 20 GiB free build headroom')
 unpack(download(config['python'],cache),dest)
 site=dest/('python/Lib/site-packages' if platform=='windows' else 'python/lib/python3.12/site-packages')
 for wheel in config['wheels']:unpack(download(wheel,cache),site)
 for i,entry in enumerate(config['runtimes']):
  scratch=out/'runtime-extracted'/entry['name'].replace('.tar.gz','').replace('.zip','')
  unpack(download(entry,cache),scratch)
  executable='llama-server.exe' if platform=='windows' else 'llama-server'
  server=next(scratch.rglob(executable));target=dest/('runtime-gpu' if i==0 else 'runtime-cpu')
  shutil.copytree(server.parent,target,dirs_exist_ok=True)
  if platform.startswith('mac'):
   # One Metal-capable runtime also supplies the CPU fallback without duplication.
   (dest/'runtime-cpu').mkdir(exist_ok=True)
 for name in ['companion_qt.py','local_chat.py','guide_index.py','memory_store.py','process_guard.py','process_guard_windows.py','README.txt']:
  shutil.copy2(ROOT/'tools/ml-studio-helper'/name,dest/name)
 (dest/'assets').mkdir(exist_ok=True);shutil.copy2(ROOT/'tools/ml-studio-helper/assets/companion-polished.png',dest/'assets/companion-polished.png')
 for name in ['BarlowSemiCondensed-Regular.ttf','OFL.txt']:shutil.copy2(ROOT/'crates/os-ui/fonts/Barlow'/name,dest/'assets'/name)
 for license in (ROOT/'packaging/helper/licenses').glob('*'):shutil.copy2(license,dest/license.name)
 shutil.copy2(ROOT/'LICENSE',dest/'LICENSE-ML-Studio.txt')
 (dest/'THIRD-PARTY.txt').write_text('Melli includes CPython 3.12.16 (PSF), PySide6/Qt 6.12.0 and Shiboken6 (LGPL/GPL, dynamic libraries), and llama.cpp b12378 (MIT).\nOriginal licence files are retained in their distributions. Qt source: https://download.qt.io/official_releases/qt/6.12/\nPySide source: https://code.qt.io/pyside/pyside-setup.git/\nPython source: https://www.python.org/downloads/source/\nllama.cpp source: https://github.com/ggml-org/llama.cpp/tree/b12378\nThe separately downloaded Qwen3.5-0.8B model is Apache-2.0, quantized by Unsloth. Source: https://huggingface.co/unsloth/Qwen3.5-0.8B-GGUF/tree/6ab461498e2023f6e3c1baea90a8f0fe38ab64d0\nQt libraries may be replaced/relinked under their licence; no restriction is imposed on doing so.\n',encoding='utf-8')
 # Capture complete file ownership for safe, bounded update/uninstall cleanup.
 paths=sorted(p.relative_to(dest).as_posix() for p in dest.rglob('*') if p.is_file() and p.name!='melli-files.json')
 (dest/'melli-files.json').write_text(json.dumps(paths),encoding='utf-8')
 archive=out/f'melli-{platform}-v{VERSION}.zip'
 with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
  for p in sorted(dest.rglob('*')):
   if p.is_file():
    info=zipfile.ZipInfo(p.relative_to(dest).as_posix());info.compress_type=zipfile.ZIP_DEFLATED
    mode=0o755 if p.stat().st_mode&0o111 or p.name in ('python3','python3.12','llama-server') else 0o644
    info.external_attr=(stat.S_IFREG|mode)<<16
    z.writestr(info,p.read_bytes())
 receipt={'platform':platform,'version':VERSION,'archive':str(archive),'sha256':sha(archive),'size':archive.stat().st_size,'unpacked_size':sum(p.stat().st_size for p in dest.rglob('*') if p.is_file()),'model':lock['model']}
 (out/f'{platform}.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8')
 print(json.dumps(receipt),flush=True)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--lock',action='store_true');p.add_argument('--platform',choices=PLATFORMS);p.add_argument('--output',type=Path);a=p.parse_args()
 if a.lock:lock_sources()
 else:build(a.platform,a.output)
