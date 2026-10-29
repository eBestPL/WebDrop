// Pixel-only AVIF / JPEG XL bridge. No source metadata is passed to encoders.
#include <avif/avif.h>
#include <jxl/encode.h>
#include <jxl/decode.h>
#include <jxl/cms.h>
#include <jxl/thread_parallel_runner.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// All returned allocations use malloc and are owned by Rust after success.
typedef struct {
    uint8_t *pixels;
    uint8_t *icc;
    size_t icc_len;
    uint32_t width, height;
    uint32_t primaries, transfer, rotation, mirror;
    uint32_t crop_x, crop_y, crop_w, crop_h;
} WDImage;

void wd_free(void *ptr) { free(ptr); }
static int valid_size(uint32_t w, uint32_t h) {
    return w && h && w <= 65535 && h <= 65535 && (uint64_t)w * h <= 64000000;
}

int wd_avif_encode(const uint8_t *pixels, uint32_t w, uint32_t h,
                   int quality, int lossless, int thorough, uint8_t **out, size_t *len) {
    int error = 6;
    *out = NULL; *len = 0;
    avifImage *image = avifImageCreate(w, h, 8, AVIF_PIXEL_FORMAT_YUV444);
    avifEncoder *enc = avifEncoderCreate();
    avifRWData data = AVIF_DATA_EMPTY;
    if (!image || !enc) { error = 3; goto cleanup; }
    // Identity GBRA transform preserves RGB exactly in lossless mode.
    image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_IDENTITY;
    image->yuvRange = AVIF_RANGE_FULL;
    image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT709;
    image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_SRGB;
    avifRGBImage rgb;
    avifRGBImageSetDefaults(&rgb, image);
    rgb.format = AVIF_RGB_FORMAT_RGBA;
    rgb.pixels = (uint8_t *)pixels;
    rgb.rowBytes = w * 4;
    if (avifImageRGBToYUV(image, &rgb) != AVIF_RESULT_OK) goto cleanup;
    enc->codecChoice = AVIF_CODEC_CHOICE_AOM;
    enc->maxThreads = 4;
    enc->quality = lossless ? AVIF_QUALITY_LOSSLESS : quality;
    enc->qualityAlpha = AVIF_QUALITY_LOSSLESS;
    enc->speed = thorough ? 4 : 6;
    if (avifEncoderWrite(enc, image, &data) != AVIF_RESULT_OK) goto cleanup;
    *out = malloc(data.size);
    if (!*out) { error = 3; goto cleanup; }
    memcpy(*out, data.data, data.size);
    *len = data.size;
    error = 0;
cleanup:
    avifRWDataFree(&data);
    if (enc) avifEncoderDestroy(enc);
    if (image) avifImageDestroy(image);
    return error;
}

int wd_jxl_encode(const uint8_t *pixels, uint32_t w, uint32_t h,
                  int quality, int lossless, int thorough, uint8_t **out, size_t *len) {
    int error = 6;
    *out = NULL; *len = 0;
    JxlEncoder *enc = JxlEncoderCreate(NULL);
    void *runner = JxlThreadParallelRunnerCreate(NULL, 4);
    uint8_t *buffer = NULL;
    if (!enc || !runner) { error = 3; goto cleanup; }
    if (JxlEncoderSetParallelRunner(enc, JxlThreadParallelRunner, runner) != JXL_ENC_SUCCESS) goto cleanup;
    JxlBasicInfo info;
    JxlEncoderInitBasicInfo(&info);
    info.xsize = w; info.ysize = h;
    info.bits_per_sample = 8;
    info.num_color_channels = 3;
    info.num_extra_channels = 1;
    info.alpha_bits = 8;
    info.uses_original_profile = lossless ? JXL_TRUE : JXL_FALSE;
    if (JxlEncoderSetBasicInfo(enc, &info) != JXL_ENC_SUCCESS) goto cleanup;
    JxlColorEncoding color;
    JxlColorEncodingSetToSRGB(&color, JXL_FALSE);
    if (JxlEncoderSetColorEncoding(enc, &color) != JXL_ENC_SUCCESS) goto cleanup;
    JxlEncoderFrameSettings *frame = JxlEncoderFrameSettingsCreate(enc, NULL);
    if (!frame ||
        JxlEncoderSetFrameLossless(frame, lossless) != JXL_ENC_SUCCESS ||
        JxlEncoderSetFrameDistance(frame, lossless ? 0 : JxlEncoderDistanceFromQuality((float)quality)) != JXL_ENC_SUCCESS ||
        JxlEncoderSetExtraChannelDistance(frame, 0, 0) != JXL_ENC_SUCCESS ||
        JxlEncoderFrameSettingsSetOption(frame, JXL_ENC_FRAME_SETTING_EFFORT, thorough ? 9 : 7) != JXL_ENC_SUCCESS ||
        JxlEncoderFrameSettingsSetOption(frame, JXL_ENC_FRAME_SETTING_KEEP_INVISIBLE, 1) != JXL_ENC_SUCCESS) goto cleanup;
    JxlPixelFormat format = {4, JXL_TYPE_UINT8, JXL_NATIVE_ENDIAN, 0};
    if (JxlEncoderAddImageFrame(frame, &format, pixels, (size_t)w * h * 4) != JXL_ENC_SUCCESS) goto cleanup;
    JxlEncoderCloseInput(enc);
    size_t capacity = 65536, used = 0;
    buffer = malloc(capacity);
    if (!buffer) { error = 3; goto cleanup; }
    for (;;) {
        uint8_t *next = buffer + used;
        size_t available = capacity - used;
        JxlEncoderStatus status = JxlEncoderProcessOutput(enc, &next, &available);
        used = capacity - available;
        if (status == JXL_ENC_SUCCESS) break;
        if (status != JXL_ENC_NEED_MORE_OUTPUT) goto cleanup;
        if (capacity >= 512 * 1024 * 1024) { error = 2; goto cleanup; }
        capacity *= 2;
        uint8_t *grown = realloc(buffer, capacity);
        if (!grown) { error = 3; goto cleanup; }
        buffer = grown;
    }
    *out = buffer; buffer = NULL; *len = used; error = 0;
cleanup:
    free(buffer);
    if (enc) JxlEncoderDestroy(enc);
    if (runner) JxlThreadParallelRunnerDestroy(runner);
    return error;
}

int wd_avif_decode(const uint8_t *data, size_t len, WDImage *out) {
    memset(out, 0, sizeof(*out));
    int error = 1;
    avifDecoder *dec = avifDecoderCreate();
    if (!dec) return 3;
    dec->maxThreads = 4;
    dec->imageSizeLimit = 64000000;
    dec->imageDimensionLimit = 65535;
    dec->imageCountLimit = 2;
    dec->ignoreXMP = AVIF_TRUE;
    if (avifDecoderSetIOMemory(dec, data, len) != AVIF_RESULT_OK ||
        avifDecoderParse(dec) != AVIF_RESULT_OK) goto cleanup;
    if (dec->imageCount != 1) { error = 4; goto cleanup; }
    if (avifDecoderNextImage(dec) != AVIF_RESULT_OK) goto cleanup;
    avifImage *im = dec->image;
    if (!valid_size(im->width, im->height)) { error = 2; goto cleanup; }
    // HDR/gain maps require a deliberate tone-mapping workflow, not silent clipping.
    if (im->gainMap || im->transferCharacteristics == AVIF_TRANSFER_CHARACTERISTICS_SMPTE2084 ||
        im->transferCharacteristics == AVIF_TRANSFER_CHARACTERISTICS_HLG) { error = 5; goto cleanup; }
    out->width = im->width; out->height = im->height;
    out->primaries = im->colorPrimaries; out->transfer = im->transferCharacteristics;
    out->crop_w = im->width; out->crop_h = im->height;
    if (im->transformFlags & AVIF_TRANSFORM_CLAP) {
        avifCropRect crop;
        if (!avifCropRectFromCleanApertureBox(&crop, &im->clap, im->width, im->height, &dec->diag)) goto cleanup;
        out->crop_x = crop.x; out->crop_y = crop.y; out->crop_w = crop.width; out->crop_h = crop.height;
    }
    if (im->transformFlags & AVIF_TRANSFORM_IROT) out->rotation = im->irot.angle;
    if (im->transformFlags & AVIF_TRANSFORM_IMIR) out->mirror = im->imir.axis + 1;
    if (im->icc.size) {
        if (im->icc.size > 4 * 1024 * 1024) { error = 2; goto cleanup; }
        out->icc = malloc(im->icc.size);
        if (!out->icc) { error = 3; goto cleanup; }
        memcpy(out->icc, im->icc.data, im->icc.size);
        out->icc_len = im->icc.size;
    }
    out->pixels = malloc((size_t)im->width * im->height * 4);
    if (!out->pixels) { error = 3; goto cleanup; }
    avifRGBImage rgb;
    avifRGBImageSetDefaults(&rgb, im);
    rgb.format = AVIF_RGB_FORMAT_RGBA;
    rgb.depth = 8;
    rgb.alphaPremultiplied = AVIF_FALSE;
    rgb.pixels = out->pixels;
    rgb.rowBytes = im->width * 4;
    if (avifImageYUVToRGB(im, &rgb) != AVIF_RESULT_OK) goto cleanup;
    error = 0;
cleanup:
    avifDecoderDestroy(dec);
    if (error) { free(out->pixels); free(out->icc); memset(out, 0, sizeof(*out)); }
    return error;
}

int wd_jxl_decode(const uint8_t *data, size_t len, WDImage *out) {
    memset(out, 0, sizeof(*out));
    int error = 1, complete = 0;
    JxlDecoder *dec = JxlDecoderCreate(NULL);
    void *runner = JxlThreadParallelRunnerCreate(NULL, 4);
    if (!dec || !runner) { error = 3; goto cleanup; }
    if (JxlDecoderSetParallelRunner(dec, JxlThreadParallelRunner, runner) != JXL_DEC_SUCCESS ||
        JxlDecoderSetCms(dec, *JxlGetDefaultCms()) != JXL_DEC_SUCCESS ||
        JxlDecoderSetUnpremultiplyAlpha(dec, JXL_TRUE) != JXL_DEC_SUCCESS ||
        JxlDecoderSubscribeEvents(dec, JXL_DEC_BASIC_INFO | JXL_DEC_COLOR_ENCODING | JXL_DEC_FULL_IMAGE) != JXL_DEC_SUCCESS ||
        JxlDecoderSetInput(dec, data, len) != JXL_DEC_SUCCESS) goto cleanup;
    JxlDecoderCloseInput(dec);
    JxlPixelFormat format = {4, JXL_TYPE_UINT8, JXL_NATIVE_ENDIAN, 0};
    for (;;) {
        JxlDecoderStatus status = JxlDecoderProcessInput(dec);
        if (status == JXL_DEC_BASIC_INFO) {
            JxlBasicInfo info;
            if (JxlDecoderGetBasicInfo(dec, &info) != JXL_DEC_SUCCESS) goto cleanup;
            if (!valid_size(info.xsize, info.ysize)) { error = 2; goto cleanup; }
            if (info.have_animation) { error = 4; goto cleanup; }
            if (info.intensity_target > 255.0f) { error = 5; goto cleanup; }
            out->width = info.xsize; out->height = info.ysize;
            out->crop_w = info.xsize; out->crop_h = info.ysize;
        } else if (status == JXL_DEC_COLOR_ENCODING) {
            JxlColorEncoding original;
            if (JxlDecoderGetColorAsEncodedProfile(dec, JXL_COLOR_PROFILE_TARGET_ORIGINAL, &original) == JXL_DEC_SUCCESS &&
                (original.transfer_function == JXL_TRANSFER_FUNCTION_PQ || original.transfer_function == JXL_TRANSFER_FUNCTION_HLG)) {
                error = 5; goto cleanup;
            }
            JxlColorEncoding srgb;
            JxlColorEncodingSetToSRGB(&srgb, JXL_FALSE);
            if (JxlDecoderSetOutputColorProfile(dec, &srgb, NULL, 0) != JXL_DEC_SUCCESS) { error = 5; goto cleanup; }
        } else if (status == JXL_DEC_NEED_IMAGE_OUT_BUFFER) {
            size_t size;
            if (JxlDecoderImageOutBufferSize(dec, &format, &size) != JXL_DEC_SUCCESS ||
                size != (size_t)out->width * out->height * 4 || out->pixels) goto cleanup;
            out->pixels = malloc(size);
            if (!out->pixels) { error = 3; goto cleanup; }
            if (JxlDecoderSetImageOutBuffer(dec, &format, out->pixels, size) != JXL_DEC_SUCCESS) goto cleanup;
        } else if (status == JXL_DEC_FULL_IMAGE) {
            complete = 1;
        } else if (status == JXL_DEC_SUCCESS) {
            if (complete && out->pixels) error = 0;
            break;
        } else goto cleanup;
    }
cleanup:
    if (dec) JxlDecoderDestroy(dec);
    if (runner) JxlThreadParallelRunnerDestroy(runner);
    if (error) { free(out->pixels); memset(out, 0, sizeof(*out)); }
    return error;
}
