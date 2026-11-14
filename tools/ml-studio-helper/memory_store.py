"""One bounded, replace-in-place user memory file. Never stores chat transcripts."""
import json
import os
import re
import threading
from pathlib import Path

MAX_BYTES = 16384
MAX_FACTS = 24
MAX_FACT_CHARS = 220
SENSITIVE = re.compile(r'password|passphrase|api.?key|access.?token|secret.?key|credit.?card|social.?security|private.?key', re.I)

class MemoryStore:
    def __init__(self, directory):
        self.path = Path(directory) / 'memory.json'
        self.lock = threading.RLock()
        self.enabled, self.facts = True, []
        self.generation = 0
        try:
            if self.path.stat().st_size <= MAX_BYTES:
                data = json.loads(self.path.read_text(encoding='utf-8'))
                self.enabled = data.get('enabled', True) is True
                self.facts = self.normalize(data.get('facts', []))
        except (OSError, ValueError, TypeError, AttributeError):
            pass

    @staticmethod
    def normalize(items):
        if not isinstance(items, list): return []
        result = []
        for item in items:
            if not isinstance(item, dict): continue
            key, text = item.get('key'), item.get('text')
            if not isinstance(key, str) or not isinstance(text, str): continue
            key = re.sub(r'[^a-z0-9_-]', '', key.lower())[:32]
            text = ' '.join(text.split())
            if not key or not text or len(text) > MAX_FACT_CHARS or SENSITIVE.search(text): continue
            result = [f for f in result if f['key'] != key and f['text'].casefold() != text.casefold()]
            result.append({'key': key, 'text': text})
        # Keep identity and communication/accessibility needs ahead of lower-value
        # preferences when full; within a tier the most recent statement wins.
        def priority(fact):
            key=fact['key']
            if re.search(r'name|language|accessib|communication',key):return 3
            if re.search(r'project|goal|genre|prefer',key):return 2
            return 1
        return sorted(result,key=priority)[-MAX_FACTS:]

    def save(self):
        with self.lock:
            self.facts = self.normalize(self.facts)
            data = json.dumps({'version': 1, 'enabled': self.enabled, 'facts': self.facts}, ensure_ascii=False).encode('utf-8')
            while len(data) > MAX_BYTES and self.facts:
                self.facts.pop(0)
                data = json.dumps({'version': 1, 'enabled': self.enabled, 'facts': self.facts}, ensure_ascii=False).encode('utf-8')
            self.path.parent.mkdir(parents=True, exist_ok=True)
            temporary = self.path.with_suffix('.tmp')
            try:
                with temporary.open('wb') as stream:
                    stream.write(data)
                os.replace(temporary, self.path)
            finally:
                temporary.unlink(missing_ok=True)

    def context(self):
        with self.lock:
            return json.dumps(self.facts, ensure_ascii=False) if self.enabled else '[]'

    def replace(self, facts, enabled=None):
        with self.lock:
            self.generation += 1
            self.facts = self.normalize(facts)
            if enabled is not None: self.enabled = bool(enabled)
            self.save()

    def clear(self):
        self.replace([])

    def learn(self, candidates, messages, generation):
        with self.lock:
            if not self.enabled or generation != self.generation: return
            # Persist only short verbatim user statements, never the model's
            # inferred summary or the assistant's own replies.
            evidence = [' '.join(m.split()).casefold() for m in messages]
            accepted = [f for f in self.normalize(candidates) if any(f['text'].casefold() in text for text in evidence)]
            if accepted:
                self.facts = self.normalize([*self.facts, *accepted])
                self.save()
