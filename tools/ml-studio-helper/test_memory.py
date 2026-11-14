import json
from pathlib import Path
import tempfile
import unittest
from memory_store import MemoryStore, MAX_BYTES

class MemoryTests(unittest.TestCase):
    def test_many_conversations_remain_bounded_and_reopen(self):
        with tempfile.TemporaryDirectory() as tmp:
            memory=MemoryStore(tmp)
            memory.learn([{'key':'name','text':'My name is Pat'}],['My name is Pat'],0)
            for batch in range(40):
                memory.learn([{'key':str(batch*30+i),'text':'I like '+str(batch*30+i)+' '+('music '*20)} for i in range(30)], [' '.join('I like '+str(batch*30+i)+' '+('music '*20) for i in range(30))], memory.generation)
            self.assertLessEqual(memory.path.stat().st_size, MAX_BYTES)
            self.assertLessEqual(len(memory.facts),24)
            self.assertIn({'key':'name','text':'My name is Pat'},memory.facts)
            self.assertEqual(MemoryStore(tmp).facts,memory.facts)
            self.assertEqual([p.name for p in Path(tmp).iterdir()],['memory.json'])

    def test_forget_rejects_pending_extraction_and_disabled_learning(self):
        with tempfile.TemporaryDirectory() as tmp:
            memory=MemoryStore(tmp);generation=memory.generation
            memory.clear();memory.learn([{'key':'name','text':'My name is Pat'}],['My name is Pat'],generation)
            self.assertEqual(memory.facts,[])
            memory.replace([],False);memory.learn([{'key':'name','text':'My name is Pat'}],['My name is Pat'],memory.generation)
            self.assertEqual(memory.context(),'[]');self.assertFalse(MemoryStore(tmp).enabled)

    def test_only_verbatim_non_secret_facts_and_corrections(self):
        with tempfile.TemporaryDirectory() as tmp:
            memory=MemoryStore(tmp)
            memory.learn([{'key':'genre','text':'I like house'},{'key':'age','text':'I am 30'},{'key':'secret','text':'my password is abc'}],['I like house, my password is abc'],0)
            self.assertEqual(memory.facts,[{'key':'genre','text':'I like house'}])
            memory.learn([{'key':'genre','text':'I prefer techno now'}],['I prefer techno now'],0)
            self.assertEqual(memory.facts,[{'key':'genre','text':'I prefer techno now'}])
            memory.clear();self.assertEqual(MemoryStore(tmp).facts,[])

if __name__=='__main__':unittest.main()
