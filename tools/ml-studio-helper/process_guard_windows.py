"""Windows job ownership: closing/crashing the UI also terminates its model."""
import ctypes
import threading
from ctypes import wintypes as W

kernel = ctypes.WinDLL('kernel32', use_last_error=True)
kernel.CreateJobObjectW.argtypes = [ctypes.c_void_p, W.LPCWSTR]
kernel.CreateJobObjectW.restype = W.HANDLE
kernel.SetInformationJobObject.argtypes = [W.HANDLE, ctypes.c_int, ctypes.c_void_p, W.DWORD]
kernel.AssignProcessToJobObject.argtypes = [W.HANDLE, W.HANDLE]
kernel.CloseHandle.argtypes = [W.HANDLE]

class BASIC(ctypes.Structure):
    _fields_ = [('per_process', ctypes.c_longlong), ('per_job', ctypes.c_longlong),
                ('flags', W.DWORD), ('min_ws', ctypes.c_size_t), ('max_ws', ctypes.c_size_t),
                ('active', W.DWORD), ('affinity', ctypes.c_size_t), ('priority', W.DWORD), ('scheduling', W.DWORD)]

class IO(ctypes.Structure):
    _fields_ = [(name, ctypes.c_ulonglong) for name in ('read', 'write', 'other', 'read_bytes', 'write_bytes', 'other_bytes')]

class EXTENDED(ctypes.Structure):
    _fields_ = [('basic', BASIC), ('io', IO), ('process_memory', ctypes.c_size_t),
                ('job_memory', ctypes.c_size_t), ('peak_process', ctypes.c_size_t), ('peak_job', ctypes.c_size_t)]

class Job:
    def __init__(self):
        self.lock = threading.RLock()
        self.handle = kernel.CreateJobObjectW(None, None)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        limits = EXTENDED()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        if not kernel.SetInformationJobObject(self.handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            self.close()
            raise ctypes.WinError(ctypes.get_last_error())

    def add(self, process):
        if not kernel.AssignProcessToJobObject(self.handle, W.HANDLE(int(process._handle))):
            raise ctypes.WinError(ctypes.get_last_error())

    def close(self):
        with self.lock:
            if self.handle:
                kernel.CloseHandle(self.handle)
                self.handle = None
