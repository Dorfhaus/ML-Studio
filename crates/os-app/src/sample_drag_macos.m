// SPDX-License-Identifier: GPL-3.0-or-later
#import <Cocoa/Cocoa.h>

static __weak NSView *completedHit;
static NSView *hitAtPoint(NSPoint p) {
    NSInteger number = [NSWindow windowNumberAtPoint:p belowWindowWithWindowNumber:0];
    NSWindow *window = [NSApp windowWithWindowNumber:number];
    NSView *root = window.contentView;
    if (!root) return nil;
    NSPoint point = [window convertPointFromScreen:p];
    if (root.superview) point = [root.superview convertPoint:point fromView:nil];
    return [root hitTest:point];
}
void *ml_completed_sample_drag(void) {
    NSView *hit = completedHit;
    completedHit = nil;
    return (__bridge void *)hit;
}
int ml_view_contains(NSView *area, NSView *hit) {
    return hit && (hit == area || [hit isDescendantOf:area]);
}

@interface MLBrowserDragSource : NSObject <NSDraggingSource>
@end
@implementation MLBrowserDragSource
- (NSDragOperation)draggingSession:(NSDraggingSession*)session
    sourceOperationMaskForDraggingContext:(NSDraggingContext)context {
    return NSDragOperationCopy;
}
- (void)draggingSession:(NSDraggingSession*)session endedAtPoint:(NSPoint)point operation:(NSDragOperation)operation {
    // Use the final mouse location. The drag image hotspot can be offset.
    completedHit = (operation & NSDragOperationCopy) ? hitAtPoint(NSEvent.mouseLocation) : nil;
}
@end

// Hit-test the topmost window, including native editors embedded in mixer strips.
int ml_drag_over_view(NSView *area) {
    return ml_view_contains(area, hitAtPoint(NSEvent.mouseLocation));
}

static NSEvent *lastDragEvent;
void ml_watch_sample_drags(void) {
    static id monitor;
    if (monitor) return;
    monitor = [NSEvent addLocalMonitorForEventsMatchingMask:
        NSEventMaskLeftMouseDragged | NSEventMaskLeftMouseUp | NSEventMaskLeftMouseDown
        handler:^NSEvent *(NSEvent *event) {
            lastDragEvent = event.type == NSEventTypeLeftMouseDragged ? event : nil;
            return event;
        }];
}

int ml_drag_sample_files(NSView *view, const char *const *paths, unsigned count) {
    NSEvent *event = lastDragEvent ?: NSApp.currentEvent;
    if (!(NSEvent.pressedMouseButtons & 1) || !event || event.window != view.window ||
        (event.type != NSEventTypeLeftMouseDragged && event.type != NSEventTypeLeftMouseDown)) return 0;
    NSMutableArray *items = [NSMutableArray arrayWithCapacity:count];
    NSPoint p = [view convertPoint:event.locationInWindow fromView:nil];
    for (unsigned i = 0; i < count; ++i) {
        NSURL *url = [NSURL fileURLWithFileSystemRepresentation:paths[i] isDirectory:NO relativeToURL:nil];
        NSDraggingItem *item = [[NSDraggingItem alloc] initWithPasteboardWriter:url];
        NSImage *icon = [NSWorkspace.sharedWorkspace iconForFile:url.path];
        [item setDraggingFrame:NSMakeRect(p.x - 12, p.y - 12, 24, 24) contents:icon];
        [items addObject:item];
    }
    static MLBrowserDragSource *source;
    if (!source) source = [MLBrowserDragSource new];
    return [view beginDraggingSessionWithItems:items event:event source:source] != nil;
}
