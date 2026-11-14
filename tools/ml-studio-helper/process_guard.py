"""Own the model on every platform, including abrupt UI exit."""
import os
import signal
import subprocess
import sys
import threading
from pathlib import Path

if os.name == 'nt':
    from process_guard_windows import Job
else:
    class Job:
        def __init__(self):
            self.process = self.watcher = None
            self.lock = threading.RLock()
        def add(self, process):
            self.process = process
            self.watcher = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), '--watch', str(process.pid)],
                stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                start_new_session=True)
        def close(self):
            with self.lock:self._close()
        def _close(self):
            if self.process and self.process.poll() is None:
                try: os.killpg(self.process.pid, signal.SIGTERM)
                except ProcessLookupError: pass
                except PermissionError: self.process.terminate()
                try: self.process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    try: os.killpg(self.process.pid, signal.SIGKILL)
                    except ProcessLookupError: pass
                    except PermissionError: self.process.kill()
                    self.process.wait(timeout=3)
            self.process = None
            # Reap the child before signalling the crash watcher. Otherwise both
            # can race to kill a dying process group (macOS can return EPERM).
            if self.watcher:
                self.watcher.stdin.close()
                try: self.watcher.wait(timeout=4)
                except subprocess.TimeoutExpired:
                    self.watcher.terminate();self.watcher.wait(timeout=3)
                self.watcher = None

    def watch(pid):
        import select
        import time
        while True:
            try: os.kill(pid, 0)
            except ProcessLookupError: return
            ready, _, _ = select.select([sys.stdin.buffer], [], [], .5)
            if ready and not os.read(sys.stdin.fileno(), 1): break
        try: os.killpg(pid, signal.SIGTERM)
        except ProcessLookupError: return
        except PermissionError:
            try: os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:return
        time.sleep(1)
        try: os.killpg(pid, signal.SIGKILL)
        except ProcessLookupError: pass
        except PermissionError:
            try: os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:pass

if __name__ == '__main__' and os.name != 'nt' and len(sys.argv) == 3 and sys.argv[1] == '--watch':
    watch(int(sys.argv[2]))
