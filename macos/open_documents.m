#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>

typedef void (*IOOpenPathCallback)(const char *path);
static IOOpenPathCallback openPath;

@interface IOOpenDocuments : NSObject
@end

@implementation IOOpenDocuments
- (void)installHandler:(NSNotification *)notification {
    (void)notification;
    [[NSAppleEventManager sharedAppleEventManager]
        setEventHandler:self
        andSelector:@selector(openDocuments:reply:)
        forEventClass:kCoreEventClass
        andEventID:kAEOpenDocuments];
}

- (void)openDocuments:(NSAppleEventDescriptor *)event
               reply:(NSAppleEventDescriptor *)reply {
    (void)reply;
    NSAppleEventDescriptor *files = [event paramDescriptorForKeyword:keyDirectObject];
    for (NSInteger i = 1; i <= files.numberOfItems; i++) {
        NSURL *url = [files descriptorAtIndex:i].fileURLValue;
        if (url.isFileURL && openPath) {
            const char *path = url.fileSystemRepresentation;
            if (path) openPath(path);
        }
    }
}
@end

// Called on the main thread before eframe starts AppKit. Install before the
// initial Finder event. winit owns NSApplication initialization/lifecycle.
void io_install_open_documents(IOOpenPathCallback callback) {
    static IOOpenDocuments *handler;
    openPath = callback;
    if (handler) return;
    handler = [IOOpenDocuments new];
    [[NSNotificationCenter defaultCenter]
        addObserver:handler
        selector:@selector(installHandler:)
        name:NSApplicationWillFinishLaunchingNotification
        object:nil];
}
