import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from local_chat import Chat, bounded_history, open_chat, MAX_HISTORY_CHARS
from process_guard import Job

class HelperTests(unittest.TestCase):
    def test_history_has_both_count_and_size_limits(self):
        messages = [{'role': 'user' if i % 2 == 0 else 'assistant', 'content': 'x'*7000} for i in range(2000)]
        result = bounded_history(messages)
        self.assertLessEqual(len(result), 12)
        self.assertLessEqual(sum(len(m['content']) for m in result), MAX_HISTORY_CHARS)
        self.assertEqual(result[0]['role'], 'user')

    def test_greeting_and_guide_remain_distinct(self):
        guide = Path(os.environ['ML_HELPER_GUIDE'])
        chat = Chat(1, 'test', guide)
        self.assertGreaterEqual(len(chat.index[0]), 196)
        self.assertIsNone(chat.route('Hi!', []))
        self.assertIn('hover', str(chat.sources('open menus on hover')).lower())
        with self.assertRaises(ValueError):
            chat.answer('x'*701)

    def test_job_terminates_owned_process(self):
        job = Job()
        process = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)'], **({'creationflags':subprocess.CREATE_NO_WINDOW} if os.name=='nt' else {'start_new_session':True}))
        try:
            job.add(process)
            job.close()
            process.wait(timeout=3)
            self.assertIsNotNone(process.returncode)
        finally:
            job.close()
            if process.poll() is None:
                process.kill()

    def test_model_close_during_startup(self):
        cancel = threading.Event()
        start = time.monotonic()
        original = subprocess.Popen
        def spawn(*args, **kwargs):
            process=original(*args, **kwargs)
            cancel.set()
            return process
        with patch('local_chat.subprocess.Popen',side_effect=spawn):
            with self.assertRaises(RuntimeError):
                with open_chat(Path(os.environ['ML_HELPER_GUIDE']), Path('unused'), cancel=cancel):
                    self.fail('Expected startup cancellation')
        self.assertLess(time.monotonic()-start, 8)

    @unittest.skipIf(os.name=='nt','POSIX crash watcher; Windows uses kill-on-close jobs')
    def test_crash_watcher_stops_model_when_ui_dies(self):
        code="import os,subprocess,sys;from process_guard import Job;p=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)'],start_new_session=True,stdout=subprocess.DEVNULL);j=Job();j.add(p);print(p.pid,flush=True);os._exit(0)"
        with subprocess.Popen([sys.executable,'-c',code],cwd=Path(__file__).parent,stdout=subprocess.PIPE,text=True) as launcher:
            pid=int(launcher.stdout.readline().strip());launcher.wait(timeout=3)
        deadline=time.monotonic()+6
        while time.monotonic()<deadline:
            try:os.kill(pid,0)
            except ProcessLookupError:return
            time.sleep(.1)
        self.fail('Model survived its owning UI crash')

    def test_model_reply_and_cancel_in_flight(self):
        cancel = threading.Event()
        with open_chat(Path(os.environ['ML_HELPER_GUIDE']), Path('unused'), cancel=cancel) as chat:
            reply, source = chat.answer('Hi!')
            self.assertTrue(reply)
            self.assertEqual(source, [])
            finished = threading.Event()
            def generate():
                try:
                    chat.answer('Explain how to arrange a complete house track, with examples.')
                except Exception:
                    pass
                finally:
                    finished.set()
            worker = threading.Thread(target=generate)
            worker.start()
            time.sleep(.2)
            cancel.set()
            self.assertTrue(finished.wait(5), 'In-flight request should stop promptly')
            worker.join(timeout=1)

if __name__ == '__main__':
    unittest.main(verbosity=2)
