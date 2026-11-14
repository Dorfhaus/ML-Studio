"""Small offline index; no training framework is imported by the companion."""
import re
from collections import Counter

STOP = set('how do i can my me to the a an it of on in when for with from and or where does is what please you could would want'.split())
ALIASES = {'size': 'length', 'crackling': 'crackles', 'plugins': 'plugin', 'tracks': 'track',
           'channels': 'channel', 'inserts': 'insert', 'settings': 'setting',
           'selecting': 'select', 'selected': 'select', 'selection': 'select',
           'several': 'multiple', 'multi': 'multiple', 'multiselect': 'select',
           'routing': 'route', 'routed': 'route', 'menus': 'menu', 'shortcuts': 'shortcut',
           'keys': 'key', 'effects': 'effect', 'notes': 'note', 'clips': 'clip',
           'hovering': 'hover', 'opening': 'open', 'recordings': 'recording'}

def words(value):
    return [ALIASES.get(word, word) for word in re.findall(r'[a-z0-9]+', value.casefold()) if word not in STOP]

def index_guide(path):
    entries = []
    for match in re.finditer(r'(?ms)^### ([^\n]+)\n(.*?)(?=^#{2,3} |\Z)', path.read_text(encoding='utf-8-sig')):
        title, body = match.groups()
        keywords = ' '.join(line.removeprefix('Keywords:') for line in body.splitlines() if line.startswith('Keywords:'))
        body = '\n'.join(line for line in body.splitlines() if not line.startswith(('Guide:', 'Related:', 'Keywords:', '![', '<!--'))).strip()
        # Index the entire article. Keep paragraphs together where possible, with
        # bounded overlapping chunks for unusually long paragraphs/registries.
        chunks, current = [], ''
        for paragraph in re.split(r'\n\s*\n', body):
            while len(paragraph) > 2200:
                if current:
                    chunks.append(current); current = ''
                cut = paragraph.rfind(' ', 1800, 2200)
                if cut < 0: cut = 2200
                chunks.append(paragraph[:cut])
                paragraph = paragraph[max(0, cut - 300):].lstrip()
            if current and len(current) + len(paragraph) > 2200:
                chunks.append(current); current = ''
            current = (current + '\n\n' + paragraph).strip()
        if current: chunks.append(current)
        for chunk in chunks:
            entries.append((title, chunk, Counter(words(title) * 4 + words(keywords) * 2 + words(chunk))))
    return entries, Counter(word for _, _, terms in entries for word in terms)
