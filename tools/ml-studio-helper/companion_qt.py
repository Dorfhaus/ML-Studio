"""Melli's portable, translucent companion surface (Windows, macOS and Linux)."""
import argparse
from collections import deque
import json
import logging
from logging.handlers import RotatingFileHandler
import math
import os
from pathlib import Path
import queue
import sys
import threading
import time
from PySide6 import QtCore as C, QtGui as G, QtWidgets as W
from memory_store import MemoryStore

HERE = Path(__file__).resolve().parent
DATA = Path(os.environ.get('ML_HELPER_DATA', HERE / 'data'))
INK, TEXT, MUTED = '#121e30', '#e8eff8', '#8acaf1'

def control_icon(kind):
    canvas=G.QPixmap(20,20);canvas.fill(C.Qt.GlobalColor.transparent)
    paint=G.QPainter(canvas);paint.setRenderHint(G.QPainter.RenderHint.Antialiasing)
    paint.setPen(G.QPen(G.QColor(MUTED),1.5,C.Qt.PenStyle.SolidLine,C.Qt.PenCapStyle.RoundCap))
    if kind=='send':
        for x,y,a,b in [(10,16,10,4),(5,9,10,4),(10,4,15,9)]:paint.drawLine(x,y,a,b)
    elif kind=='close':paint.drawLine(6,6,14,14);paint.drawLine(14,6,6,14)
    else:paint.drawLine(5,10,15,10)
    paint.end();return G.QIcon(canvas)

class Signals(C.QObject):
    ready = C.Signal(object)
    reply = C.Signal(str, object)
    failed = C.Signal(str)

class Melli(W.QWidget):
    def __init__(self, guide, session=None, parent_pid=0, smoke=False):
        super().__init__()
        self.guide, self.session, self.parent_pid = guide, session, parent_pid
        self.memory = MemoryStore(DATA)
        self.cancel = threading.Event()
        self.commands = queue.Queue(maxsize=1)
        self.worker = self.chat = None
        self.busy, self.expanded, self.show_history = False, False, False
        self.messages = deque([('Melli', "Hey, I'm Melli. Beats, big ideas, or whatever's on your mind. I'm in.")], maxlen=12)
        self.sources = []
        self.phase, self.started, self.talking_until = 'idle', time.monotonic(), 0
        self.last_host_seen = time.monotonic()
        self.host_state, self.state_stamp = {}, None
        self.signals = Signals()
        self.signals.ready.connect(self.ready)
        self.signals.reply.connect(self.receive)
        self.signals.failed.connect(self.failed)
        self.setWindowTitle('ML Studio Helper (Melli)')
        self.setWindowFlags(C.Qt.WindowType.Tool | C.Qt.WindowType.FramelessWindowHint | C.Qt.WindowType.WindowStaysOnTopHint)
        self.setAttribute(C.Qt.WidgetAttribute.WA_TranslucentBackground)
        self.setStyleSheet('QWidget { color: '+TEXT+'; font: 15px "Barlow Semi Condensed"; } '
            'QPushButton { background: transparent; border: none; padding: 5px; color: '+MUTED+'; } '
            'QPushButton:hover { color: white; background: #263c58; border-radius: 7px; } '
            'QLineEdit { background: #1e304a; border: 1px solid #426080; border-radius: 19px; padding: 11px 44px 11px 14px; color: '+TEXT+'; } '
            'QLineEdit:focus { border: 1px solid #83c6f2; } '
            'QTextBrowser { background: transparent; border: none; color: '+TEXT+'; } '
            'QScrollBar:vertical { background: transparent; width: 5px; } '
            'QScrollBar::handle:vertical { background: #425c79; border-radius: 2px; min-height: 20px; } '
            'QScrollBar::add-line:vertical,QScrollBar::sub-line:vertical { height: 0; }')
        sheet = G.QImage(str(HERE/'assets/companion-polished.png'))
        if sheet.isNull(): raise RuntimeError('Melli artwork is missing')
        self.frames = [G.QPixmap.fromImage(sheet.copy(i*sheet.width()//4,0,sheet.width()//4,sheet.height())).scaled(140,270,C.Qt.AspectRatioMode.KeepAspectRatio,C.Qt.TransformationMode.SmoothTransformation) for i in range(4)]
        self.portrait = W.QPushButton(self)
        self.portrait.setStyleSheet('background: transparent; border: none; padding: 0;')
        self.portrait.setIconSize(C.QSize(140,270))
        self.portrait.setToolTip('Chat with Melli')
        self.portrait.clicked.connect(self.toggle)
        self.exit_button = W.QPushButton(self);self.exit_button.setIcon(control_icon('close'))
        self.exit_button.setToolTip('Close Melli and unload the model')
        self.exit_button.clicked.connect(self.close)
        self.panel = W.QWidget(self)
        self.panel.setGeometry(22,30,365,303)
        layout = W.QVBoxLayout(self.panel); layout.setContentsMargins(0,0,0,0); layout.setSpacing(12)
        header = W.QHBoxLayout()
        self.status = W.QLabel('Melli', self.panel); self.status.setStyleSheet('color: '+MUTED+'; font-size: 12px;')
        header.addWidget(self.status,1)
        for label, tip, handler in [('...', 'Conversation and memory options', self.options),('\u2013','Collapse bubble',self.toggle)]:
            button=W.QPushButton(label);button.setToolTip(tip);button.clicked.connect(handler);header.addWidget(button)
            if label=='\u2013':button.setText('');button.setIcon(control_icon('minus'))
        layout.addLayout(header)
        self.transcript = W.QTextBrowser(); self.transcript.setOpenExternalLinks(False)
        layout.addWidget(self.transcript,1)
        self.entry = W.QLineEdit(); self.entry.setPlaceholderText('Talk to Melli...');self.entry.setMaxLength(700)
        self.entry.returnPressed.connect(self.send);layout.addWidget(self.entry)
        self.send_button=W.QPushButton(self.entry);self.send_button.setIcon(control_icon('send'));self.send_button.setGeometry(322,6,32,32)
        self.send_button.setToolTip('Send message');self.send_button.clicked.connect(self.send)
        self.panel.hide()
        self.portrait.setIcon(G.QIcon(self.frames[0]))
        self.render();self.arrange()
        self.timer=C.QTimer(self);self.timer.timeout.connect(self.tick);self.timer.start(150)
        if smoke: C.QTimer.singleShot(100,self.toggle);C.QTimer.singleShot(1500,self.close)

    def arrange(self):
        self.resize(600 if self.expanded else 170,440 if self.expanded else 290)
        x,y=(435,157) if self.expanded else (10,14)
        self.portrait.setGeometry(x,y,140,270)
        self.exit_button.setGeometry(self.width()-34,136 if self.expanded else 0,28,26)
        self.panel.setVisible(self.expanded)
        screen=self.screen() or W.QApplication.primaryScreen(); bounds=screen.availableGeometry()
        state=self.host_state
        rect=state.get('bounds')
        if isinstance(rect,list) and len(rect)==4 and all(isinstance(v,(int,float)) for v in rect):
            host=C.QRect(*map(int,rect)).intersected(bounds)
            if host.width()>200 and host.height()>200: bounds=host
        self.move(max(bounds.left(),bounds.right()-self.width()-16),max(bounds.top(),bounds.bottom()-self.height()-16))
        mask=G.QRegion(self.exit_button.geometry())
        for frame in self.frames:
            mask=mask.united(G.QRegion(frame.mask()).translated(x+(140-frame.width())//2,y+(270-frame.height())//2))
        if self.expanded:
            mask=mask.united(G.QRegion(self.bubble_path().toFillPolygon().toPolygon()))
        self.setMask(mask)
        self.update()

    def bubble_path(self):
        path=G.QPainterPath();path.addRoundedRect(C.QRectF(4,12,401,339),26,26)
        tail=G.QPainterPath(C.QPointF(402,174));tail.quadTo(420,195,451,195);tail.quadTo(420,200,402,195)
        return path.united(tail)

    def paintEvent(self,event):
        if not self.expanded:return
        p=G.QPainter(self);p.setRenderHint(G.QPainter.RenderHint.Antialiasing)
        p.setPen(G.QPen(G.QColor('#3b536e'),1));p.setBrush(G.QColor(INK));p.drawPath(self.bubble_path())

    def toggle(self):
        self.expanded=not self.expanded;self.arrange()
        if self.expanded:
            self.show();self.raise_();self.activateWindow();self.entry.setFocus()
            if not self.worker:
                self.status.setText('Melli is waking up...')
                self.worker=threading.Thread(target=self.run_model,name='Melli local model',daemon=False);self.worker.start()
        self.controls()

    def controls(self):
        self.send_button.setEnabled(self.chat is not None and not self.busy)
        if self.busy:self.status.setText('Melli is thinking...')
        elif self.chat:self.status.setText('Melli')

    def render(self):
        messages=list(self.messages) if self.show_history else [self.messages[-1]]
        self.transcript.setPlainText('\n\n'.join((speaker+'\n' if self.show_history else '')+text for speaker,text in messages))
        if self.show_history:self.transcript.verticalScrollBar().setValue(self.transcript.verticalScrollBar().maximum())

    def send(self):
        if not self.chat or self.busy:return
        text=self.entry.text().strip()
        if not text:return
        self.entry.clear();self.messages.append(('You',text));self.busy=True;self.phase='thinking'
        self.render();self.controls();self.commands.put_nowait(text)

    def ready(self,chat):self.chat=chat;self.controls()
    def receive(self,text,sources):
        self.messages.append(('Melli',text[:2400]));self.sources=sources;self.busy=False
        self.phase='talking';self.talking_until=time.monotonic()+min(5,max(1.5,len(text)/70))
        self.render();self.controls()
    def failed(self,message):
        self.messages.append(('Melli',message));self.busy=False;self.render();self.status.setText('Melli needs attention')

    def new_chat(self):
        if self.busy:return
        if self.chat:self.chat.history.clear()
        self.messages.clear();self.messages.append(('Melli','Fresh page. What are we getting into?'));self.sources=[];self.render()

    def options(self):
        menu=W.QMenu(self);menu.setStyleSheet('QMenu { background: #18283e; color: #e8eff8; padding: 6px; } QMenu::item:selected { background: #304d70; }')
        new=menu.addAction('New chat');new.setEnabled(not self.busy);new.triggered.connect(self.new_chat)
        menu.addAction('Latest reply' if self.show_history else 'Conversation history',self.history_toggle)
        source=menu.addAction('Guide source',lambda:self.transcript.setPlainText('\n\n'.join(t+'\n'+b for t,b in self.sources)))
        source.setEnabled(bool(self.sources))
        menu.addSeparator();menu.addAction('Memory: view, edit or forget',self.edit_memory)
        if self.session:menu.addAction('Remove downloaded Helper files...',self.request_remove)
        menu.addAction('Close Melli and unload model',self.close)
        menu.exec(G.QCursor.pos())
    def request_remove(self):
        try:
            (DATA/'remove-request').write_text(str(self.parent_pid),encoding='ascii')
        except OSError:
            self.failed('Could not request removal. Check your Helper storage permissions.')
            return
        self.close()
    def history_toggle(self):self.show_history=not self.show_history;self.render()

    def edit_memory(self):
        dialog=W.QDialog(self);dialog.setWindowTitle("Melli's memory");dialog.setStyleSheet('background: #18283e; color: #e8eff8;')
        layout=W.QVBoxLayout(dialog)
        layout.addWidget(W.QLabel('Useful details stay here across sessions. Chats are not saved.\nAt most 24 short notes; the memory file never exceeds 16 KiB.'))
        enabled=W.QCheckBox('Remember useful details');enabled.setChecked(self.memory.enabled);layout.addWidget(enabled)
        editor=W.QPlainTextEdit('\n'.join(f['text'] for f in self.memory.facts));editor.setMinimumSize(440,220);layout.addWidget(editor)
        note=W.QLabel('One note per line, up to 220 characters. Turning memory off stops learning and use.');note.setWordWrap(True);layout.addWidget(note)
        buttons=W.QDialogButtonBox(W.QDialogButtonBox.StandardButton.Save | W.QDialogButtonBox.StandardButton.Cancel)
        forget=buttons.addButton('Forget everything',W.QDialogButtonBox.ButtonRole.ResetRole)
        def forget_all():
            try:self.memory.clear();editor.clear()
            except OSError:note.setText('Memory could not be saved. Check folder permissions and free space.')
        forget.clicked.connect(forget_all)
        def save():
            lines=[x.strip() for x in editor.toPlainText().splitlines() if x.strip()]
            if len(lines)>24 or any(len(x)>220 for x in lines):note.setText('Keep at most 24 notes, each no longer than 220 characters.');return
            try:self.memory.replace([{'key':'note_'+str(i),'text':text} for i,text in enumerate(lines)],enabled.isChecked())
            except OSError:note.setText('Memory could not be saved. Check folder permissions and free space.');return
            dialog.accept()
        buttons.accepted.connect(save);buttons.rejected.connect(dialog.reject);layout.addWidget(buttons);dialog.exec()

    def run_model(self):
        try:
            from local_chat import open_chat
            with open_chat(self.guide, DATA/'logs/model.log',cancel=self.cancel,memory=self.memory) as chat:
                self.signals.ready.emit(chat)
                while not self.cancel.is_set():
                    try:query=self.commands.get(timeout=.1)
                    except queue.Empty:continue
                    try:self.signals.reply.emit(*chat.answer(query))
                    except Exception as error:
                        if not self.cancel.is_set():logging.warning('Answer failed: %s',type(error).__name__);self.signals.reply.emit("That reply hit a wall. Try me again.",[])
        except Exception as error:
            if not self.cancel.is_set():
                logging.warning('Model failed: %s',type(error).__name__)
                self.signals.failed.emit("I couldn't start my local model. Close and reopen me; check the Helper installation if it happens again.")

    def tick(self):
        if self.session:
            try:
                stamp=self.session.stat().st_mtime_ns
                if stamp!=self.state_stamp:
                    self.host_state=json.loads(self.session.read_text(encoding='utf-8'));self.state_stamp=stamp;self.arrange()
                    self.last_host_seen=time.monotonic()
                if self.host_state.get('close'):self.close();return
            except (OSError,ValueError):pass
            if time.monotonic()-self.last_host_seen>8:self.close();return
            visible=self.host_state.get('visible',True) and (self.host_state.get('focused',True) or self.isActiveWindow() or bool(W.QApplication.activeModalWidget()))
            if visible and not self.isVisible():self.show()
            elif not visible and self.isVisible():self.hide()
        now=time.monotonic();elapsed=now-self.started
        if self.phase=='talking' and now>self.talking_until:self.phase='idle'
        if self.isVisible():
            frame=3 if self.phase=='thinking' else (2 if self.phase=='talking' and int(elapsed*4)%2 else (1 if elapsed%4.6<.18 else 0))
            self.portrait.setIcon(G.QIcon(self.frames[frame]))
        self.timer.setInterval(150 if self.isVisible() else 500)

    def closeEvent(self,event):
        self.cancel.set();self.timer.stop();event.accept()
        W.QApplication.instance().quit()


def main():
    p=argparse.ArgumentParser();p.add_argument('--guide',type=Path,default=HERE/'user-guide.md');p.add_argument('--session',type=Path);p.add_argument('--parent-pid',type=int,default=0);p.add_argument('--smoke',action='store_true');p.add_argument('--render',type=Path)
    args=p.parse_args()
    DATA.mkdir(parents=True,exist_ok=True)
    lock=C.QLockFile(str(DATA/'companion.lock'));lock.setStaleLockTime(10000)
    if not lock.tryLock(0):return
    logs=DATA/'logs';logs.mkdir(exist_ok=True)
    logging.basicConfig(handlers=[RotatingFileHandler(logs/'companion.log',maxBytes=65536,backupCount=2,encoding='utf-8')],level=logging.WARNING)
    app=W.QApplication(sys.argv[:1]);app.setApplicationName('Melli');app.setQuitOnLastWindowClosed(True)
    font=HERE/'assets/BarlowSemiCondensed-Regular.ttf'
    if not font.exists():font=HERE.parents[1]/'crates/os-ui/fonts/Barlow/BarlowSemiCondensed-Regular.ttf'
    G.QFontDatabase.addApplicationFont(str(font))
    window=Melli(args.guide,args.session,args.parent_pid,args.smoke)
    if args.render:
        window.expanded=True;window.arrange();window.show();app.processEvents();window.grab().save(str(args.render));window.close()
    else:
        window.show()
        try:app.exec()
        finally:
            window.cancel.set()
            if window.worker:window.worker.join(timeout=15)
    lock.unlock()

if __name__=='__main__':main()
