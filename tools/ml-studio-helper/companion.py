"""Optional Windows DAW companion: bounded memory, lazy model, owned overlay."""
import argparse
import ctypes
from ctypes import wintypes as W
from collections import deque
import logging
from logging.handlers import RotatingFileHandler
import math
import os
from pathlib import Path
import queue
import threading
import time
import tkinter as tk
from PIL import Image, ImageTk
from bubble_style import surface_image, sprite_frames, SURFACE, INPUT

HERE = Path(__file__).resolve().parent
BG, FG, ACCENT, KEY = SURFACE, '#e8eff8', '#8acaf1', '#010203'
user = ctypes.WinDLL('user32', use_last_error=True)
kernel = ctypes.WinDLL('kernel32', use_last_error=True)
user.GetForegroundWindow.restype = W.HWND
user.GetAncestor.argtypes = [W.HWND, W.UINT]
user.GetAncestor.restype = W.HWND
user.GetWindowThreadProcessId.argtypes = [W.HWND, ctypes.POINTER(W.DWORD)]
user.GetWindowRect.argtypes = [W.HWND, ctypes.POINTER(W.RECT)]
user.IsWindowVisible.argtypes = [W.HWND]
user.IsIconic.argtypes = [W.HWND]
user.SetWindowLongPtrW.argtypes = [W.HWND, ctypes.c_int, ctypes.c_ssize_t]
user.SetWindowLongPtrW.restype = ctypes.c_ssize_t
kernel.OpenProcess.argtypes = [W.DWORD, W.BOOL, W.DWORD]
kernel.OpenProcess.restype = W.HANDLE
kernel.WaitForSingleObject.argtypes = [W.HANDLE, W.DWORD]
kernel.CloseHandle.argtypes = [W.HANDLE]
kernel.CreateMutexW.argtypes = [ctypes.c_void_p, W.BOOL, W.LPCWSTR]
kernel.CreateMutexW.restype = W.HANDLE
kernel.CreateEventW.argtypes = [ctypes.c_void_p, W.BOOL, W.BOOL, W.LPCWSTR]
kernel.CreateEventW.restype = W.HANDLE

def process_id(hwnd):
    pid = W.DWORD()
    user.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
    return pid.value

def owner_window(pid):
    candidates = []
    callback_type = ctypes.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)
    def visit(hwnd, _):
        if process_id(hwnd) == pid and user.IsWindowVisible(hwnd):
            rect = W.RECT()
            user.GetWindowRect(hwnd, ctypes.byref(rect))
            area = (rect.right - rect.left) * (rect.bottom - rect.top)
            candidates.append((area, hwnd))
        return True
    user.EnumWindows(callback_type(visit), 0)
    return max(candidates)[1] if candidates else None

class Companion:
    def __init__(self, guide, parent_pid=0, smoke=False, stop_event=None):
        self.guide, self.parent_pid, self.smoke = guide, parent_pid, smoke
        self.stop_event = stop_event
        self.parent_handle = kernel.OpenProcess(0x100000, False, parent_pid) if parent_pid else None
        if parent_pid and not self.parent_handle:
            raise RuntimeError('ML Studio is no longer running.')
        self.owner = owner_window(parent_pid) if parent_pid else None
        self.root = tk.Tk()
        self.root.title('ML Studio Helper')
        self.root.overrideredirect(True)
        self.root.configure(bg=KEY)
        self.root.wm_attributes('-transparentcolor', KEY)
        self.root.geometry('150x274')
        self.root.update_idletasks()
        self.hwnd = user.GetAncestor(self.root.winfo_id(), 2)
        if self.owner:
            user.SetWindowLongPtrW(self.hwnd, -8, self.owner)  # owned, not global topmost
        self.canvas = tk.Canvas(self.root, width=150, height=274, bg=KEY, highlightthickness=0)
        self.canvas.pack()
        self.frames = [ImageTk.PhotoImage(frame) for frame in sprite_frames(HERE / 'assets' / 'companion-polished.png')]
        self.sprite = self.canvas.create_image(75, 252, image=self.frames[0], anchor='s')
        self.canvas.tag_bind(self.sprite, '<Button-1>', lambda _: self.toggle())
        self.canvas.configure(cursor='hand2')
        self.close_button = tk.Button(self.root, text='\u00d7', command=self.close, bg=BG, fg=FG,
                                      relief='flat', bd=0, font=('Segoe UI', 12), cursor='hand2')
        self.close_button.place(x=121, y=1, width=26, height=25)
        self.root.bind('<Escape>', lambda _: self.close())
        self.root.protocol('WM_DELETE_WINDOW', self.close)
        self.cancel = threading.Event()
        self.commands = queue.Queue(maxsize=2)
        self.events = queue.Queue(maxsize=4)
        self.thread = None
        self.chat = None
        self.bubble = None
        self.bubble_height = 220
        self.show_history = False
        self.viewing_source = False
        self.busy = False
        self.ready = False
        self.closed = False
        self.visible = True
        self.phase = 'idle'
        self.talking_until = 0
        self.history = deque(maxlen=12)
        self.history.append(('Helper', 'Hey! What are we making today?'))
        self.source = ''
        self.started = time.monotonic()
        self.last_frame = None
        self.place()
        self.root.after(150, self.poll)

    def place(self):
        if self.owner:
            rect = W.RECT()
            if not user.GetWindowRect(self.owner, ctypes.byref(rect)):
                return
            x, y = rect.right - 168, rect.bottom - 305
        else:
            x, y = self.root.winfo_screenwidth() - 168, self.root.winfo_screenheight() - 330
        x, y = max(0, x), max(0, y)
        self.root.geometry(f'150x274+{x}+{y}')
        if self.bubble:
            self.bubble.geometry(f'400x{self.bubble_height}+{max(0,x-330)}+{max(0,y-72)}')

    def toggle(self):
        if self.bubble:
            self.close_bubble()
            return
        self.bubble = tk.Toplevel(self.root)
        self.bubble.overrideredirect(True)
        self.bubble.configure(bg=KEY)
        self.bubble.wm_attributes('-transparentcolor', KEY)
        self.shell = tk.Canvas(self.bubble, bg=KEY, highlightthickness=0, bd=0)
        self.shell.pack(fill='both', expand=True)
        self.shell.bind('<Configure>', self.paint_bubble)
        self.panel = panel = tk.Frame(self.shell, bg=BG)
        panel.columnconfigure(0, weight=1)
        panel.rowconfigure(1, weight=1, minsize=55)
        self.panel_window = self.shell.create_window(22, 16, window=panel, anchor='nw', width=306, height=180)
        header = tk.Frame(panel, bg=BG)
        header.grid(row=0, column=0, sticky='ew', pady=(0, 8))
        self.status = tk.Label(header, text='Waking up\u2026', bg=BG, fg='#8ecaff', font=('Segoe UI', 9), anchor='w')
        self.status.pack(side='left', fill='x', expand=True)
        def small_button(parent, text, command):
            return tk.Button(parent, text=text, command=command, bg=BG, fg='#9cc8ff',
                             activebackground='#1c3470', activeforeground=FG, bd=0,
                             highlightthickness=0, relief='flat', cursor='hand2', font=('Segoe UI', 10))
        small_button(header, '\u00d7', self.close).pack(side='right')
        small_button(header, '\u2014', self.close_bubble).pack(side='right', padx=4)
        small_button(header, '...', self.options).pack(side='right', padx=4)
        self.transcript = tk.Text(panel, bg=BG, fg=FG, insertbackground=FG, wrap='word', bd=0,
                                 highlightthickness=0, font=('Segoe UI', 11), height=1,
                                 padx=0, pady=3, spacing1=2, spacing3=6, cursor='arrow')
        self.transcript.grid(row=1, column=0, sticky='nsew')
        self.transcript.tag_configure('speaker', foreground='#88a8ba', font=('Segoe UI', 9))
        self.controls = tk.Frame(panel, bg=BG)
        # Secondary actions live in the header menu, not the conversation.
        self.reset_button = small_button(self.controls, 'New chat', self.reset)
        self.reset_button.pack(side='left')
        self.history_button = small_button(self.controls, 'History', self.toggle_history)
        self.history_button.pack(side='left', padx=12)
        self.sources_button = small_button(self.controls, 'Source', self.show_source)
        self.sources_button.pack(side='right')
        self.form = form = tk.Canvas(panel, bg=BG, height=46, highlightthickness=0, bd=0)
        form.grid(row=4, column=0, sticky='ew', pady=(12,0))
        self.draft = tk.StringVar()
        self.entry = tk.Entry(form, bg=INPUT, fg=FG, insertbackground=ACCENT, relief='flat',
                              bd=0, font=('Segoe UI', 11), width=1, insertwidth=2,
                              insertontime=600, insertofftime=400, textvariable=self.draft)
        self.entry_window = form.create_window(15,23,window=self.entry,anchor='w',width=238)
        self.hint_label = tk.Label(self.entry, text='Message\u2026', bg=INPUT, fg='#8c9eb5',
                                   font=('Segoe UI', 10), bd=0, cursor='xterm')
        self.hint_label.place(x=8,rely=0.5,anchor='w')
        self.hint_label.bind('<Button-1>', lambda _: self.entry.focus_force())
        self.draft.trace_add('write', self.draft_changed)
        self.entry.bind('<FocusIn>', self.paint_input)
        self.entry.bind('<FocusOut>', self.paint_input)
        self.entry.bind('<Return>', self.send)
        self.send_button = tk.Button(form, text='\u2191', command=self.send, bg=INPUT, fg=ACCENT,
                                     activebackground=INPUT, activeforeground=FG,
                                     disabledforeground='#526680', relief='flat', bd=0,
                                     cursor='hand2', font=('Segoe UI', 15))
        self.send_window = form.create_window(279,23,window=self.send_button)
        form.bind('<Configure>', self.paint_input)
        self.surface_key = None
        self.input_key = None
        self.bubble.bind('<Escape>', lambda _: self.close_bubble())
        self.render()
        self.place()
        if self.thread is None:
            self.cancel.clear()
            self.thread = threading.Thread(target=self.worker, name='ML Helper model', daemon=False)
            self.thread.start()
        self.set_controls()
        self.bubble.after_idle(self.focus_input)

    def focus_input(self):
        if self.bubble and not self.closed:
            self.entry.focus_force()

    def options(self):
        menu = tk.Menu(self.bubble, tearoff=False, bg=BG, fg=FG,
                       activebackground='#293e5a', activeforeground=FG, bd=0)
        menu.add_command(label='New chat', command=self.reset, state='normal' if self.ready and not self.busy else 'disabled')
        menu.add_command(label='Latest reply' if self.show_history else 'Conversation history', command=self.toggle_history)
        menu.add_command(label='Back to chat' if self.viewing_source else 'Guide source',
                         command=self.back_to_chat if self.viewing_source else self.show_source,
                         state='normal' if self.source else 'disabled')
        try:
            menu.tk_popup(self.bubble.winfo_pointerx(), self.bubble.winfo_pointery())
        finally:
            menu.grab_release()
            menu.destroy()

    def draft_changed(self, *_):
        if self.draft.get():
            self.hint_label.place_forget()
        else:
            self.hint_label.place(x=8,rely=0.5,anchor='w')

    def paint_input(self, _event=None):
        if not self.bubble:
            return
        height = max(46, self.entry.winfo_reqheight()+20, self.send_button.winfo_reqheight()+12)
        self.form.configure(height=height)
        width = max(306,self.form.winfo_width())
        focused = self.entry.focus_get() == self.entry
        key = (width,height,focused)
        if key != self.input_key:
            self.input_key = key
            self.input_texture = ImageTk.PhotoImage(surface_image(height,width,True,focused))
            self.form.delete('surface')
            self.form.create_image(0,0,image=self.input_texture,anchor='nw',tags='surface')
            self.form.tag_lower('surface')
        self.form.coords(self.entry_window,15,height/2)
        self.form.itemconfigure(self.entry_window,width=width-65)
        self.form.coords(self.send_window,width-27,height/2)

    def minimum_bubble_height(self):
        self.paint_input()
        self.panel.update_idletasks()
        requested = self.status.master.winfo_reqheight()+self.form.winfo_reqheight()
        return max(200, requested + 20 + 55 + 36)

    def close_bubble(self):
        if self.bubble:
            self.bubble.destroy()
            self.bubble = None
        # Collapsing only hides the panel; the visible character's X fully exits.

    def paint_bubble(self, _event=None):
        if not self.bubble:
            return
        if self.surface_key != self.bubble_height:
            self.surface_key = self.bubble_height
            self.surface_texture = ImageTk.PhotoImage(surface_image(self.bubble_height))
            self.shell.delete('shell')
            self.shell.create_image(0,0,image=self.surface_texture,anchor='nw',tags='shell')
            self.shell.tag_lower('shell')
        self.shell.itemconfigure(self.panel_window, height=self.bubble_height-36)

    def toggle_history(self):
        self.show_history = not self.show_history
        self.render()

    def render(self):
        if not self.bubble:
            return
        self.viewing_source = False
        entries = list(self.history) if self.show_history else [self.history[-1]]
        content = '\n'.join(text for _, text in entries)
        lines = sum(max(1, math.ceil(len(line)/34)) for line in content.splitlines())
        self.bubble_height = max(self.minimum_bubble_height(), min(390, 145 + lines*21))
        self.place()
        self.paint_bubble()
        self.history_button.configure(text='Latest' if self.show_history else 'History')
        self.transcript.configure(state='normal')
        self.transcript.delete('1.0', 'end')
        for speaker, content in entries:
            if self.show_history or speaker == 'You':
                self.transcript.insert('end', speaker + '\n', 'speaker')
            self.transcript.insert('end', content.replace('**', '') + '\n\n')
        self.transcript.configure(state='disabled')
        self.transcript.see('end' if self.show_history else '1.0')
        self.sources_button.configure(text='Source', command=self.show_source)

    def set_controls(self):
        if not self.bubble:
            return
        state = 'normal' if self.ready and not self.busy else 'disabled'
        # Drafting is always possible, including during loading or generation.
        self.entry.configure(state='normal')
        for control in (self.send_button, self.reset_button):
            control.configure(state=state)
        self.sources_button.configure(state='normal' if self.source else 'disabled')
        self.status.configure(text='Thinking\u2026' if self.busy else ('ML Studio' if self.ready else 'Waking up\u2026'))

    def send(self, _event=None):
        if not self.ready or self.busy:
            return
        query = self.entry.get().strip()
        if not query:
            return
        if len(query) > 700 or '<|' in query or '|>' in query:
            self.status.configure(text='Use plain text, up to 700 characters.')
            return
        self.entry.delete(0, 'end')
        self.history.append(('You', query))
        self.busy = True
        self.phase = 'thinking'
        self.set_controls()
        self.render()
        self.commands.put_nowait(query)

    def reset(self):
        if self.busy or not self.ready:
            return
        self.chat.history.clear()
        self.history.clear()
        self.history.append(('Helper', 'Fresh start! What are we working on?'))
        self.source = ''
        self.render()
        self.set_controls()

    def show_source(self):
        self.viewing_source = True
        self.bubble_height = max(350, self.minimum_bubble_height())
        self.place()
        self.paint_bubble()
        self.transcript.configure(state='normal')
        self.transcript.delete('1.0', 'end')
        self.transcript.insert('end', self.source)
        self.transcript.configure(state='disabled')
        self.sources_button.configure(text='Back to chat', command=self.back_to_chat)

    def back_to_chat(self):
        self.sources_button.configure(text='Guide source', command=self.show_source)
        self.render()

    def worker(self):
        try:
            from local_chat import open_chat
            with open_chat(self.guide, HERE / 'logs' / 'unused.log', cancel=self.cancel) as chat:
                self.chat = chat
                self.events.put(('ready', None))
                while not self.cancel.is_set():
                    try:
                        query = self.commands.get(timeout=.1)
                    except queue.Empty:
                        continue
                    try:
                        self.events.put(('reply', chat.answer(query)))
                    except Exception as error:
                        if not self.cancel.is_set():
                            logging.error('Answer failed: %s', type(error).__name__)
                            self.events.put(('reply', ("I couldn't finish that answer. Please try again.", [])))
        except Exception as error:
            if not self.cancel.is_set():
                logging.error('Model startup failed: %s', type(error).__name__)
                self.events.put(('failed', None))
        finally:
            self.chat = None

    def poll(self):
        if self.closed:
            return
        if self.stop_event and kernel.WaitForSingleObject(self.stop_event, 0) == 0:
            self.close()
            return
        if self.parent_handle and kernel.WaitForSingleObject(self.parent_handle, 0) == 0:
            self.close()
            return
        if self.parent_pid and not self.owner:
            self.owner = owner_window(self.parent_pid)
            if self.owner:
                user.SetWindowLongPtrW(self.hwnd, -8, self.owner)
        active_pid = process_id(user.GetForegroundWindow())
        visible = not self.parent_pid or (self.owner and not user.IsIconic(self.owner) and active_pid in (self.parent_pid, os.getpid()))
        if visible != self.visible:
            self.visible = visible
            (self.root.deiconify if visible else self.root.withdraw)()
            if self.bubble:
                (self.bubble.deiconify if visible else self.bubble.withdraw)()
        if visible:
            self.place()
        while not self.events.empty():
            kind, value = self.events.get_nowait()
            if kind == 'ready':
                self.ready = True
            elif kind == 'reply':
                reply, sources = value
                self.history.append(('Helper', reply[:2400]))
                self.source = '\n\n'.join(title+'\n'+body for title, body in sources)[:8000]
                self.busy = False
                self.phase = 'talking'
                self.talking_until = time.monotonic() + min(6, max(2, len(reply)/65))
                self.render()
            elif kind == 'failed':
                self.history.append(('Helper', "The offline model couldn't start. Close and reopen me to retry."))
                self.render()
            self.set_controls()
        now = time.monotonic()
        if self.phase == 'talking' and now > self.talking_until:
            self.phase = 'idle'
        if visible:
            elapsed = now-self.started
            frame = 3 if self.phase == 'thinking' else (2 if self.phase == 'talking' and int(elapsed*4)%2 else (1 if elapsed%4.6 < .18 else 0))
            if frame != self.last_frame:
                self.canvas.itemconfigure(self.sprite, image=self.frames[frame])
                self.last_frame = frame
            self.canvas.coords(self.sprite, 75, 252 + round(math.sin(elapsed*1.7)))
        self.root.after(150 if visible else 500, self.poll)

    def close(self):
        if self.closed:
            return
        self.closed = True
        self.cancel.set()
        self.root.destroy()

    def run(self):
        try:
            if self.smoke:
                self.toggle()
                self.root.after(1200, self.close)
            self.root.mainloop()
        finally:
            self.cancel.set()
            if self.thread:
                self.thread.join(timeout=15)
            if self.parent_handle:
                kernel.CloseHandle(self.parent_handle)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--guide', type=Path, default=HERE / 'user-guide.md')
    parser.add_argument('--parent-pid', type=int, default=0)
    parser.add_argument('--smoke', action='store_true')
    args = parser.parse_args()
    mutex = kernel.CreateMutexW(None, False, f'Local\\MLStudioHelper-{args.parent_pid}')
    if not mutex or ctypes.get_last_error() == 183:
        return
    logs = HERE / 'logs'
    logs.mkdir(exist_ok=True)
    handler = RotatingFileHandler(logs / 'companion.log', maxBytes=65536, backupCount=2, encoding='utf-8')
    logging.basicConfig(handlers=[handler], level=logging.WARNING)
    stop_event = kernel.CreateEventW(None, False, False, f'Local\\MLStudioHelperStop-{args.parent_pid}')
    if not stop_event:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        Companion(args.guide, args.parent_pid, args.smoke, stop_event).run()
    except Exception:
        logging.exception('Companion failed')
        raise
    finally:
        kernel.CloseHandle(stop_event)
        kernel.CloseHandle(mutex)

if __name__ == '__main__':
    main()
