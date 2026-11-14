"""Refresh source-backed reference pages without replacing authored workflows."""
from pathlib import Path
import re,json,collections,sys
ROOT=Path(__file__).resolve().parents[1]
GUIDE=ROOT/'docs/user-guide.md'
MARK='<!-- source-reference-start -->'
STR=r'"(?:\\.|[^"\\])*"'
def decode(s):
 s=s[1:-1]
 s=re.sub(r'\\u\{([0-9a-fA-F]+)\}',lambda m:chr(int(m[1],16)),s)
 return s.replace('\\"','"').replace('\\n',' ').replace('\\t',' ').replace('\\\\','\\')
def clean(s):
 # Keep strings intact (including URLs); remove comments and unit-test bodies.
 s=re.split(r'#\[cfg\(test\)\]\s*mod\s+\w+',s)[0]
 return re.sub(STR+r'|//[^\n]*',lambda m:m[0] if m[0].startswith('"') else '',s)

def call_args(s,start):
 # Balanced Rust call reader, ignoring strings and line comments.
 stack=['('];parts=[];begin=start;i=start
 while i<len(s):
  if s.startswith('//',i):
   i=s.find('\n',i)
   if i<0:return None
   continue
  if s[i]=='"':
   m=re.match(STR,s[i:])
   if not m:return None
   i+=m.end();continue
  c=s[i]
  if c in '([{':stack.append(c)
  elif c in ')]}':
   stack.pop()
   if not stack:
    parts.append(s[begin:i].strip());return parts
  elif c==',' and len(stack)==1:parts.append(s[begin:i].strip());begin=i+1
  i+=1
 return None

def literal(s):
 s=s.strip()
 return decode(s) if re.fullmatch(STR,s) else None

def readable(name):
 aliases={'3x osc':'ML Trio','parametric eq 2':'ML EQ','sampler':'ML Sampler','delay':'ML Delay','reverb':'ML Reverb','limiter':'ML Limiter','wave':'ML Wave','chop':'ML Chop','tesla':'ML Tesla','rack mixer':'Channel Strip mixer','mixer':'Mixer','piano':'Piano Roll','channel':'Channel settings','channel rack':'Channel Rack'}
 name=name.replace('lemon-','').replace('_',' ').replace('-',' ')
 return aliases.get(name,name.title())

def generate():
 original=GUIDE.read_text(encoding='utf-8-sig').split(MARK)[0].rstrip()
 pages=[];records=[];skipped=collections.Counter()
 def article(title,body,src):
  if not body:return
  body='\n'.join(line.rstrip() for line in body.splitlines() if line.strip()!='-')
  pages.append('### '+title+'\nKeywords: '+title.lower()+' reference controls options settings\n'+body+'\n')
  records.append({'title':title,'source':src})
 # Each setting has its own retrievable article and exact allowed values.
 for path in [ROOT/'crates/os-app/src/prefs.rs',ROOT/'crates/os-app/src/shortcuts.rs']:
  s=path.read_text(encoding='utf-8')
  arrays={m[1]:[decode(q) for q in re.findall(STR,m[2])] for m in re.finditer(r'const\s+(\w+):[^=]+?=\s*&?\[(.*?)\];',s,re.S)}
  for m in re.finditer(r'^pub const (\w+): (Check|Combo|Number|Slider)\s*=\s*\w+\s*\{(.*?)\};',s,re.S|re.M):
   name,kind,fields=m.groups();label=re.search(r'label:\s*('+STR+')',fields)
   if name=='TEST_CHECK':continue
   if not label:continue
   label=decode(label[1]);default=re.search(r'default:\s*(\w+)',fields)
   if not default:continue
   before=s[:m.start()].rstrip().splitlines();comment=[]
   while before and before[-1].lstrip().startswith('///'):comment.insert(0,before.pop().strip()[3:].strip())
   description=' '.join(comment)
   # Source comments containing old availability claims are not user instructions.
   stale=bool(re.search(r'not .*yet|no .*yet|stored and shown only|no typing|FL.?s range',description,re.I))
   # Comments mix implementation details and historical availability claims.
   # Export only a location hint, never unverified comment prose as instructions.
   location=re.match(r'(General|Audio|MIDI|File|Project)\s*[>\u25b8][^:.(]+',description)
   if stale:skipped['settings_comments_needing_reverification']+=1
   body=('Location: Settings > '+location[0].replace('\u25b8','>').strip()+'.\n\n' if location else '')
   if kind=='Combo':
    values=re.search(r'choices:\s*&\[(.*?)\]',fields,re.S)
    if values:choices=[decode(q) for q in re.findall(STR,values[1])]
    else:
     ref=re.search(r'choices:\s*(\w+)',fields);choices=arrays.get(ref[1],[]) if ref else []
    body+='Available choices: '+('; '.join(choices) if choices else 'Use the current drop-down list; choices are supplied by this control.')+'.\n'
    if choices and default[1].isdigit() and int(default[1])<len(choices):body+='Default: '+choices[int(default[1])]+'.\n'
   elif kind=='Check':body+='Checkbox; default: '+('on' if default[1]=='true' else 'off')+'.\n'
   else:
    bounds={key:re.search(r'\b'+key+r':\s*(\d+)',fields) for key in ('min','max','step')}
    body+='Default: '+default[1]+'. '
    if bounds['max']:body+='Allowed control values: '+(bounds['min'][1] if bounds['min'] else '0')+' to '+bounds['max'][1]+'. '
    if bounds['step']:body+='Wheel increment: '+bounds['step'][1]+'.'
    body+='\n'
   body+='This is a per-profile preference. Existing saved choices override the default. A hidden preference can be revealed through Settings > Visibility; a disabled control may require another setting or a supported context.'
   article('Setting: '+label,body,path.relative_to(ROOT).as_posix())
 # Static tooltips are the actual instructions users see when hovering controls.
 paths=sorted((ROOT/'crates/os-app/src').rglob('*.rs'))+sorted((ROOT/'plugins').rglob('*.rs'))
 for path in paths:
  if any(x in path.parts for x in ('tests','examples')):continue
  s=clean(path.read_text(encoding='utf-8'))
  hints=[];menus=[];unavailable=[]
  for m in re.finditer(r'\bui\.hit\(',s):
   args=call_args(s,m.end())
   text=literal(args[-1]) if args else None
   if text and len(text)>8 and '{' not in text:hints.append(text)
   else:skipped['dynamic_or_nonliteral_hints']+=1
  for m in re.finditer(r'\bPItem::(new|sub|not_yet)\(',s):
   args=call_args(s,m.end());text=literal(args[0]) if args else None
   if text and text not in ('','(none)','Default'):
    (unavailable if m[1]=='not_yet' else menus).append(text+(' [submenu]' if m[1]=='sub' else ''))
   else:skipped['dynamic_menu_labels']+=1
  # Toolbar tuples supply hints through variables rather than direct ui.hit calls.
  hints.extend(decode(m[1]) for m in re.finditer('('+STR+r')\s*,\s*Some\(Action::',s))
  hints=list(dict.fromkeys(hints));menus=list(dict.fromkeys(menus))
  unavailable=[v+' [not implemented]' for v in dict.fromkeys(unavailable) if v not in {x.removesuffix(' [submenu]') for x in menus}]
  if not hints and not menus and not unavailable:continue
  relative=path.relative_to(ROOT)
  if 'plugins' in path.parts:
   group=readable(path.parts[path.parts.index('plugins')+1])+' / '+readable(path.stem)
  else:group=readable(path.stem)+(' view' if 'views' in path.parts else '')
  parts=[('Control hints',hints),('Menu labels',menus),('Unavailable menu placeholders',unavailable)]
  for category,items in parts:
   for start in range(0,len(items),16):
    block=items[start:start+16]
    if not block:continue
    title=group+' - '+category.lower()+' '+str(start//16+1)
    intro='These are the control hints shown by this editor. Read modifier instructions as applying to that specific control.' if category=='Control hints' else 'These are menu labels declared for this area, not a sequence of steps. Some commands appear only in the appropriate selection/context. Entries marked not implemented cannot be used. Dynamic plugin/device/preset lists vary with the current project and installation.'
    article(title,intro+'\n\n'+'\n'.join('- '+v for v in block),relative.as_posix())
 # Complete declared configurable transport/window shortcuts, including empty bindings.
 s=(ROOT/'crates/os-app/src/shortcuts.rs').read_text(encoding='utf-8')
 section=s.split('pub const COMMANDS:')[1].split('];')[0]
 rows=re.findall(r'\('+STR+r'\s*,\s*('+STR+r')\s*,\s*('+STR+r')\)',section)
 article('Configurable command shortcuts - exact defaults','Settings > General > Input > Manage shortcuts changes these defaults. Apply keeps the changes; clearing a command disables its binding. A reassigned default does not trigger the old action. Ctrl means Command on macOS.\n\n'+'\n'.join('- '+decode(a)+': '+decode(b) for a,b in rows),'crates/os-app/src/shortcuts.rs')
 # Plugin parameter labels actually declared in plugin constructors/setting registries.
 for directory in sorted((ROOT/'plugins').iterdir()):
  if not directory.is_dir():continue
  groups={}
  for path in (directory/'src').glob('*.rs'):
   s=clean(path.read_text(encoding='utf-8'))
   spec=re.search(r'\bname:\s*('+STR+')',s) if directory.name=='lemon-small-fx' else None
   group=decode(spec[1]) if spec and decode(spec[1]).startswith(('ML ','Lemon ')) else readable(directory.name)
   labels=groups.setdefault(group,[])
   for pattern in [r'(?:FloatParam|IntParam|BoolParam)::new\(\s*('+STR+')',r'\bsetting\(\s*'+STR+r'\s*,\s*('+STR+')',r'Def\s*\{\s*id:\s*'+STR+r',\s*name:\s*('+STR+')']:
    labels.extend(decode(m[1]) for m in re.finditer(pattern,s))
  for group,labels in groups.items():
   labels=list(dict.fromkeys(labels))
   for start in range(0,len(labels),20):
    article(group+' - parameter names '+str(start//20+1),'These are host-visible parameter names declared by the plugin. Use the plugin editor or Browse parameters to locate them. Names alone do not imply a particular unit or audible result; consult the instrument/effect workflow articles and the displayed value.\n\n'+'\n'.join('- '+x for x in labels[start:start+20]),directory.relative_to(ROOT).as_posix())
 text=original+'\n\n'+MARK+'\n'+ '\n'.join(pages)+'\n<!-- source-reference-end -->\n'
 if '--check' not in sys.argv:GUIDE.write_text(text,encoding='utf-8')
 report={'generated_articles':len(pages),'guide_articles':len(re.findall(r'^### ',text,re.M)),'records':records,'explicit_exclusions':dict(skipped),'scope':'Static preference declarations, direct editor hints, literal menu labels, configurable command shortcuts and constructor parameter names. Dynamic controls and behavioral completeness require manual review; catalog presence is not a claim that every feature is fully documented.'}
 audit=ROOT/'docs/guide-source-audit.json'
 serialized=json.dumps(report,indent=2)+'\n'
 if '--check' in sys.argv:
  if GUIDE.read_text(encoding='utf-8')!=text or not audit.exists() or audit.read_text(encoding='utf-8')!=serialized:
   raise SystemExit('Guide references are stale: run packaging/expand-guide-reference.py')
 else:audit.write_text(serialized,encoding='utf-8')
 print(json.dumps({k:v for k,v in report.items() if k!='records'},indent=2))
if __name__=='__main__':generate()
