// Synthetic fixtures only. Build with the same libavif/libjxl headers as WebDrop.
#include <avif/avif.h>
#include <jxl/encode.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
static unsigned char pixels[48 * 24 * 4];
static const unsigned char exif[] = {'I','I',42,0,8,0,0,0,0,0,0,0,0,0};
static void save(const char *dir, const char *name, const void *data, size_t len) {
    char path[4096]; snprintf(path,sizeof(path),"%s/%s",dir,name);
    FILE *f=fopen(path,"wb"); assert(f); assert(fwrite(data,1,len,f)==len); fclose(f);
}
int main(int argc, char **argv) {
    assert(argc == 2);
    for(int y=0;y<24;y++) for(int x=0;x<48;x++) {
        int i=(y*48+x)*4;
        pixels[i]=x<24?220:20; pixels[i+1]=y<12?180:40; pixels[i+2]=70; pixels[i+3]=255;
    }
    for(int kind=0;kind<3;kind++) {
        avifImage *im=avifImageCreate(48,24,8,AVIF_PIXEL_FORMAT_YUV444);
        im->matrixCoefficients=AVIF_MATRIX_COEFFICIENTS_IDENTITY;
        im->colorPrimaries=AVIF_COLOR_PRIMARIES_BT709;
        im->transferCharacteristics=kind==2?AVIF_TRANSFER_CHARACTERISTICS_SMPTE2084:AVIF_TRANSFER_CHARACTERISTICS_SRGB;
        im->yuvRange=AVIF_RANGE_FULL;
        avifRGBImage rgb; avifRGBImageSetDefaults(&rgb,im); rgb.format=AVIF_RGB_FORMAT_RGBA;
        rgb.pixels=pixels; rgb.rowBytes=48*4;
        assert(avifImageRGBToYUV(im,&rgb)==AVIF_RESULT_OK);
        assert(avifImageSetMetadataExif(im,exif,sizeof(exif))==AVIF_RESULT_OK);
        const char *xmp="<x:xmpmeta>PRIVATE-LOCATION</x:xmpmeta>";
        assert(avifImageSetMetadataXMP(im,(const uint8_t*)xmp,strlen(xmp))==AVIF_RESULT_OK);
        if(kind==0) { im->transformFlags=AVIF_TRANSFORM_IROT; im->irot.angle=3; }
        avifEncoder *enc=avifEncoderCreate(); enc->quality=100; enc->qualityAlpha=100; enc->speed=8; enc->maxThreads=2;
        avifRWData out=AVIF_DATA_EMPTY;
        if(kind==1) {
            assert(avifEncoderAddImage(enc,im,1,AVIF_ADD_IMAGE_FLAG_NONE)==AVIF_RESULT_OK);
            assert(avifEncoderAddImage(enc,im,1,AVIF_ADD_IMAGE_FLAG_NONE)==AVIF_RESULT_OK);
            assert(avifEncoderFinish(enc,&out)==AVIF_RESULT_OK);
        } else assert(avifEncoderWrite(enc,im,&out)==AVIF_RESULT_OK);
        save(argv[1],kind==0?"modern-rotated.avif":kind==1?"modern-animated.avif":"modern-hdr.avif",out.data,out.size);
        avifRWDataFree(&out); avifEncoderDestroy(enc); avifImageDestroy(im);
    }
    for(int kind=0;kind<3;kind++) {
        JxlEncoder *enc=JxlEncoderCreate(NULL);
        JxlBasicInfo info; JxlEncoderInitBasicInfo(&info);
        info.xsize=48; info.ysize=24; info.bits_per_sample=8; info.num_color_channels=3;
        info.num_extra_channels=1;info.alpha_bits=8;info.uses_original_profile=JXL_TRUE;
        if(kind==0)info.orientation=JXL_ORIENT_ROTATE_90_CW;
        if(kind==1) {info.have_animation=JXL_TRUE;info.animation.tps_numerator=10;info.animation.tps_denominator=1;}
        assert(JxlEncoderSetBasicInfo(enc,&info)==JXL_ENC_SUCCESS);
        JxlColorEncoding color;JxlColorEncodingSetToSRGB(&color,JXL_FALSE);
        if(kind==2)color.transfer_function=JXL_TRANSFER_FUNCTION_PQ;
        assert(JxlEncoderSetColorEncoding(enc,&color)==JXL_ENC_SUCCESS);
        assert(JxlEncoderUseContainer(enc,JXL_TRUE)==JXL_ENC_SUCCESS);
        assert(JxlEncoderUseBoxes(enc)==JXL_ENC_SUCCESS);
        unsigned char box[4+sizeof(exif)]={0};memcpy(box+4,exif,sizeof(exif));
        assert(JxlEncoderAddBox(enc,"Exif",box,sizeof(box),JXL_FALSE)==JXL_ENC_SUCCESS);
        const char *xmp="<x:xmpmeta>PRIVATE-LOCATION</x:xmpmeta>";
        assert(JxlEncoderAddBox(enc,"xml ",(const uint8_t*)xmp,strlen(xmp),JXL_FALSE)==JXL_ENC_SUCCESS);
        JxlEncoderFrameSettings *frame=JxlEncoderFrameSettingsCreate(enc,NULL);
        assert(JxlEncoderSetFrameLossless(frame,JXL_TRUE)==JXL_ENC_SUCCESS);
        assert(JxlEncoderFrameSettingsSetOption(frame,JXL_ENC_FRAME_SETTING_EFFORT,3)==JXL_ENC_SUCCESS);
        JxlPixelFormat format={4,JXL_TYPE_UINT8,JXL_NATIVE_ENDIAN,0};
        for(int f=0;f<(kind==1?2:1);f++) {
            if(kind==1) { JxlFrameHeader header;JxlEncoderInitFrameHeader(&header);header.duration=1;assert(JxlEncoderSetFrameHeader(frame,&header)==JXL_ENC_SUCCESS); }
            assert(JxlEncoderAddImageFrame(frame,&format,pixels,sizeof(pixels))==JXL_ENC_SUCCESS);
        }
        JxlEncoderCloseInput(enc);
        unsigned char data[65536],*next=data;size_t available=sizeof(data);
        assert(JxlEncoderProcessOutput(enc,&next,&available)==JXL_ENC_SUCCESS);
        save(argv[1],kind==0?"modern-rotated.jxl":kind==1?"modern-animated.jxl":"modern-hdr.jxl",data,sizeof(data)-available);
        JxlEncoderDestroy(enc);
    }
}
