#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>
#include <math.h>
#include <stdint.h>
#include <stdlib.h>

// Limited text extraction detects Illustrator's PDF placeholder page, which is
// present in some AI files saved with PDF compatibility disabled.
typedef struct { char text[65536]; size_t length; } IOText;
static void appendString(CGPDFStringRef string, IOText *text) {
    const unsigned char *bytes = CGPDFStringGetBytePtr(string);
    size_t count = CGPDFStringGetLength(string);
    for (size_t i = 0; i < count && text->length + 1 < sizeof(text->text); i++) {
        unsigned char c = bytes[i];
        if (c >= 'A' && c <= 'Z') c += 'a' - 'A';
        if (c >= 'a' && c <= 'z') text->text[text->length++] = c;
    }
    text->text[text->length] = 0;
}
static void scanString(CGPDFScannerRef scanner, void *info) {
    CGPDFStringRef string;
    if (CGPDFScannerPopString(scanner, &string)) appendString(string, info);
}
static void scanArray(CGPDFScannerRef scanner, void *info) {
    CGPDFArrayRef array;
    if (!CGPDFScannerPopArray(scanner, &array)) return;
    for (size_t i = 0; i < CGPDFArrayGetCount(array); i++) {
        CGPDFStringRef string;
        if (CGPDFArrayGetString(array, i, &string)) appendString(string, info);
    }
}
static bool isPlaceholder(CGPDFPageRef page) {
    IOText text = { .length = 0 };
    CGPDFOperatorTableRef table = CGPDFOperatorTableCreate();
    CGPDFOperatorTableSetCallback(table, "Tj", scanString);
    CGPDFOperatorTableSetCallback(table, "TJ", scanArray);
    CGPDFOperatorTableSetCallback(table, "'", scanString);
    CGPDFOperatorTableSetCallback(table, "\"", scanString);
    CGPDFContentStreamRef stream = CGPDFContentStreamCreateWithPage(page);
    CGPDFScannerRef scanner = CGPDFScannerCreate(stream, table, &text);
    CGPDFScannerScan(scanner);
    CGPDFScannerRelease(scanner);
    CGPDFContentStreamRelease(stream);
    CGPDFOperatorTableRelease(table);
    return strstr(text.text, "withoutpdfcontent") != NULL;
}

// Returns owned, premultiplied RGBA8 in sRGB, in top-to-bottom row order.
// Error codes are translated to user-facing Polish in native_decode.rs.
int io_decode(const uint8_t *bytes, size_t length, int pdf, uint32_t dpi,
              uint32_t selectedPage, uint8_t **output, uint32_t *width,
              uint32_t *height, uint32_t *orientation, uint32_t *pageCount) {
    @autoreleasepool {
        NSData *data = [NSData dataWithBytesNoCopy:(void *)bytes length:length freeWhenDone:NO];
        CGPDFDocumentRef document = NULL;
        CGImageSourceRef source = NULL;
        CGImageRef image = NULL;
        CGPDFPageRef page = NULL;
        CGContextRef context = NULL;
        uint8_t *pixels = NULL;
        int error = 1;
        size_t w = 0, h = 0;
        double pdfWidth = 0, pdfHeight = 0;
        *output = NULL;
        *orientation = 1;
        *pageCount = 1;
        if (pdf) {
            CGDataProviderRef provider = CGDataProviderCreateWithCFData((__bridge CFDataRef)data);
            document = CGPDFDocumentCreateWithProvider(provider);
            CGDataProviderRelease(provider);
            if (!document) goto cleanup;
            if (!CGPDFDocumentIsUnlocked(document)) { error = 4; goto cleanup; }
            *pageCount = (uint32_t)CGPDFDocumentGetNumberOfPages(document);
            if (selectedPage < 1 || selectedPage > *pageCount) { error = 5; goto cleanup; }
            page = CGPDFDocumentGetPage(document, selectedPage);
            if (!page) goto cleanup;
            if (isPlaceholder(page)) { error = 6; goto cleanup; }
            CGRect box = CGPDFPageGetBoxRect(page, kCGPDFCropBox);
            double pw = box.size.width, ph = box.size.height;
            int rotation = ((CGPDFPageGetRotationAngle(page) % 360) + 360) % 360;
            if (rotation == 90 || rotation == 270) { double swap = pw; pw = ph; ph = swap; }
            pdfWidth = pw; pdfHeight = ph;
            double dw = ceil(pw * dpi / 72.0), dh = ceil(ph * dpi / 72.0);
            if (!isfinite(dw) || !isfinite(dh) || dw < 1 || dh < 1 || dw > 65535 || dh > 65535 || dw * dh > 64000000) { error = 2; goto cleanup; }
            w = (size_t)dw; h = (size_t)dh;
        } else {
            source = CGImageSourceCreateWithData((__bridge CFDataRef)data, (__bridge CFDictionaryRef)@{(id)kCGImageSourceShouldCache: @NO});
            if (!source) goto cleanup;
            *pageCount = (uint32_t)CGImageSourceGetCount(source);
            // HEIC auxiliary/depth images are not pages: select its primary image.
            CFStringRef type = CGImageSourceGetType(source);
            BOOL heif = type && ([(__bridge NSString *)type containsString:@"heic"] || [(__bridge NSString *)type containsString:@"heif"]);
            size_t index = heif ? CGImageSourceGetPrimaryImageIndex(source) : selectedPage - 1;
            if ((heif && selectedPage != 1) || selectedPage < 1 || index >= *pageCount) { error = 5; goto cleanup; }
            if (heif) *pageCount = 1;
            NSDictionary *properties = CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(source, index, NULL));
            double dw = [properties[(id)kCGImagePropertyPixelWidth] doubleValue];
            double dh = [properties[(id)kCGImagePropertyPixelHeight] doubleValue];
            if (!isfinite(dw) || !isfinite(dh) || dw < 1 || dh < 1 || dw > 65535 || dh > 65535 || dw * dh > 64000000) { error = 2; goto cleanup; }
            image = CGImageSourceCreateImageAtIndex(source, index, (__bridge CFDictionaryRef)@{(id)kCGImageSourceShouldCache: @YES, (id)kCGImageSourceShouldCacheImmediately: @YES});
            if (!image) goto cleanup;
            w = CGImageGetWidth(image); h = CGImageGetHeight(image);
            NSNumber *exif = properties[(id)kCGImagePropertyOrientation];
            if (exif) *orientation = exif.unsignedIntValue;
        }
        if (w == 0 || h == 0 || w > 65535 || h > 65535 || (uint64_t)w * h > 64000000) { error = 2; goto cleanup; }
        pixels = calloc(w * h, 4);
        if (!pixels) { error = 3; goto cleanup; }
        {
            CGColorSpaceRef srgb = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
            context = CGBitmapContextCreate(pixels, w, h, 8, w * 4, srgb, kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big);
            CGColorSpaceRelease(srgb);
        }
        if (!context) { error = 3; goto cleanup; }
        CGContextSetInterpolationQuality(context, kCGInterpolationHigh);
        if (pdf) {
            // CGContext's standard PDF transform handles page rotation and CropBox origin.
            CGContextScaleCTM(context, dpi / 72.0, dpi / 72.0);
            CGContextConcatCTM(context, CGPDFPageGetDrawingTransform(page, kCGPDFCropBox, CGRectMake(0, 0, pdfWidth, pdfHeight), 0, true));
            CGContextDrawPDFPage(context, page);
        } else {
            CGContextDrawImage(context, CGRectMake(0, 0, w, h), image);
        }
        *width = (uint32_t)w; *height = (uint32_t)h;
        *output = pixels;
        pixels = NULL;
        error = 0;
    cleanup:
        if (context) CGContextRelease(context);
        if (image) CGImageRelease(image);
        if (source) CFRelease(source);
        if (document) CGPDFDocumentRelease(document);
        free(pixels);
        return error;
    }
}

void io_free(uint8_t *pixels) { free(pixels); }
