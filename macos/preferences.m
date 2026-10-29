#import <Foundation/Foundation.h>
#include <string.h>

// Use the same domain for bundled launches and local development builds.
static CFStringRef const PreferencesDomain = CFSTR("local.imageoptimizer.desktop");
static CFStringRef const LanguageKey = CFSTR("InterfaceLanguage");

int io_language_preference(void) {
    @autoreleasepool {
        id value = CFBridgingRelease(CFPreferencesCopyAppValue(LanguageKey, PreferencesDomain));
        if ([value isEqual:@"pl"]) return 1;
        if ([value isEqual:@"en"]) return 2;
        return 0;
    }
}

bool io_save_language_preference(int language) {
    CFStringRef value = language == 1 ? CFSTR("pl") : language == 2 ? CFSTR("en") : NULL;
    CFPreferencesSetAppValue(LanguageKey, value, PreferencesDomain);
    return CFPreferencesAppSynchronize(PreferencesDomain);
}

void io_system_language(char *buffer, size_t capacity) {
    @autoreleasepool {
        NSString *language = [NSLocale preferredLanguages].firstObject ?: @"en";
        if (capacity > 0) strlcpy(buffer, language.UTF8String, capacity);
    }
}

// Keep mode and last custom path together; NSData preserves filesystem path bytes.
static CFStringRef const OutputKey = CFSTR("OutputDestination");

size_t io_load_output_preference(const char *domain, int *mode, uint8_t *path, size_t capacity) {
    @autoreleasepool {
        NSString *name = [NSString stringWithUTF8String:domain];
        id value = CFBridgingRelease(CFPreferencesCopyAppValue(OutputKey, (__bridge CFStringRef)name));
        *mode = 0;
        if (![value isKindOfClass:[NSDictionary class]]) return 0;
        id storedMode = value[@"mode"];
        if ([storedMode isKindOfClass:[NSNumber class]]) *mode = [storedMode intValue];
        id data = value[@"path"];
        if (![data isKindOfClass:[NSData class]]) return 0;
        size_t length = [data length];
        if (length <= capacity) [data getBytes:path length:length];
        return length;
    }
}

bool io_save_output_preference(const char *domain, int mode, const uint8_t *path, size_t length) {
    @autoreleasepool {
        NSString *name = [NSString stringWithUTF8String:domain];
        NSDictionary *value = mode == 0 && length == 0 ? nil : @{
            @"mode": @(mode), @"path": [NSData dataWithBytes:path length:length]
        };
        CFPreferencesSetAppValue(OutputKey, (__bridge CFPropertyListRef)value, (__bridge CFStringRef)name);
        return CFPreferencesAppSynchronize((__bridge CFStringRef)name);
    }
}
