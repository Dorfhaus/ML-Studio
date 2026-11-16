// SPDX-License-Identifier: GPL-3.0-or-later
// Native macOS AUv2 calls. The Rust host serializes access to each instance.
#import <AppKit/AppKit.h>
#import <AudioToolbox/AudioToolbox.h>
#import <AudioUnit/AUCocoaUIView.h>
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

enum { BlockFrames = 4096 };

typedef struct {
    double beat, tempo, bar, frame, loopStart, loopEnd;
    uint32_t nextBeat, numerator, denominator, playing, cycling;
} Transport;

typedef struct {
    AudioUnit unit;
    BOOL instrument;
    BOOL initialized;
    UInt32 channels;
    UInt32 frames;
    Float64 sampleTime;
    Transport transport;
    Float64 expectedFrame;
    BOOL transportChanged;
    float input[2][BlockFrames];
    float output[2][BlockFrames];
    void *editor;
} Host;

typedef struct {
    uint32_t id;
    float minimum, maximum, initial;
    uint32_t steps, bits;
    char name[256];
} Parameter;

static OSStatus beat_and_tempo(void *context, Float64 *beat, Float64 *tempo) {
    Host *host = context;
    if (beat) *beat = host->transport.beat;
    if (tempo) *tempo = host->transport.tempo;
    return noErr;
}

static OSStatus musical_time(void *context, UInt32 *nextBeat, Float32 *numerator,
                             UInt32 *denominator, Float64 *downbeat) {
    Host *host = context;
    if (nextBeat) *nextBeat = host->transport.nextBeat;
    if (numerator) *numerator = host->transport.numerator;
    if (denominator) *denominator = host->transport.denominator;
    if (downbeat) *downbeat = host->transport.bar;
    return noErr;
}

static OSStatus transport_state(void *context, Boolean *playing, Boolean *changed,
                               Float64 *frame, Boolean *cycling, Float64 *start, Float64 *end) {
    Host *host = context;
    if (playing) *playing = (Boolean)host->transport.playing;
    if (changed) *changed = (Boolean)host->transportChanged;
    if (frame) *frame = host->transport.frame;
    if (cycling) *cycling = (Boolean)host->transport.cycling;
    if (start) *start = host->transport.loopStart;
    if (end) *end = host->transport.loopEnd;
    return noErr;
}

typedef void (*ComponentVisitor)(void *, uint32_t, uint32_t, uint32_t, const char *, uint32_t, bool);

void lm_au_components(ComponentVisitor visitor, void *data) {
    @autoreleasepool {
        const OSType types[] = {kAudioUnitType_MusicDevice, kAudioUnitType_Effect, kAudioUnitType_MusicEffect};
        for (unsigned i = 0; i < sizeof(types) / sizeof(types[0]); ++i) {
            AudioComponentDescription search = { .componentType = types[i] };
            AudioComponent component = NULL;
            while ((component = AudioComponentFindNext(component, &search))) {
                AudioComponentDescription desc = {0};
                if (AudioComponentGetDescription(component, &desc) ||
                    (desc.componentFlags & kAudioComponentFlag_IsV3AudioUnit)) continue;
                CFStringRef ref = NULL;
                if (AudioComponentCopyName(component, &ref) || !ref) continue;
                NSString *name = CFBridgingRelease(ref);
                UInt32 version = 0;
                AudioComponentGetVersion(component, &version);
                visitor(data, desc.componentType, desc.componentSubType, desc.componentManufacturer,
                        name.UTF8String, version, i == 0);
            }
        }
    }
}

Host *lm_au_new(uint32_t type, uint32_t subtype, uint32_t manufacturer, int32_t *error) {
    @autoreleasepool {
        AudioComponentDescription desc = { .componentType = type, .componentSubType = subtype,
                                            .componentManufacturer = manufacturer,
                                            .componentFlagsMask = kAudioComponentFlag_IsV3AudioUnit };
        AudioComponent component = AudioComponentFindNext(NULL, &desc);
        if (!component) { *error = kAudio_ParamError; return NULL; }
        AudioUnit unit = NULL;
        *error = AudioComponentInstanceNew(component, &unit);
        if (*error || !unit) return NULL;
        Host *host = calloc(1, sizeof(*host));
        if (!host) { AudioComponentInstanceDispose(unit); return NULL; }
        host->unit = unit;
        host->instrument = type == kAudioUnitType_MusicDevice;
        host->transport = (Transport){ .tempo = 120, .numerator = 4, .denominator = 4 };
        HostCallbackInfo callbacks = {host, beat_and_tempo, musical_time, transport_state, NULL};
        AudioUnitSetProperty(unit, kAudioUnitProperty_HostCallbacks, kAudioUnitScope_Global,
                             0, &callbacks, sizeof(callbacks));
        return host;
    }
}

void lm_au_close_editor(Host *host) {
    if (!host->editor) return;
    NSView *view = CFBridgingRelease(host->editor);
    host->editor = NULL;
    if (NSThread.isMainThread) [view removeFromSuperview];
    else dispatch_async(dispatch_get_main_queue(), ^{ [view removeFromSuperview]; });
}

void lm_au_destroy(Host *host) {
    if (!host) return;
    if (host->editor && !NSThread.isMainThread) {
        dispatch_async(dispatch_get_main_queue(), ^{ lm_au_destroy(host); });
        return;
    }
    @autoreleasepool {
        lm_au_close_editor(host);
        if (host->initialized) AudioUnitUninitialize(host->unit);
        AudioComponentInstanceDispose(host->unit);
        free(host);
    }
}

static OSStatus pull_input(void *context, AudioUnitRenderActionFlags *flags,
                           const AudioTimeStamp *time, UInt32 bus, UInt32 frames,
                           AudioBufferList *list) {
    Host *host = context;
    if (list->mNumberBuffers != host->channels || frames > BlockFrames) return kAudio_ParamError;
    size_t offset = 0;
    if ((time->mFlags & kAudioTimeStampSampleTimeValid) && time->mSampleTime > host->sampleTime) {
        double delta = time->mSampleTime - host->sampleTime;
        if (delta <= BlockFrames) offset = (size_t)delta;
        else offset = BlockFrames;
    }
    UInt32 bytes = frames * sizeof(float);
    for (UInt32 channel = 0; channel < host->channels; ++channel) {
        AudioBuffer *buffer = &list->mBuffers[channel];
        if (buffer->mData && buffer->mDataByteSize < bytes) return kAudio_ParamError;
        if (bus == 0 && offset + frames <= host->frames) {
            float *input = host->input[channel] + offset;
            if (buffer->mData) memcpy(buffer->mData, input, bytes);
            else buffer->mData = input;
        } else {
            static float silence[BlockFrames];
            if (buffer->mData) memset(buffer->mData, 0, bytes);
            else buffer->mData = silence;
            *flags |= kAudioUnitRenderAction_OutputIsSilence;
        }
        buffer->mDataByteSize = bytes;
    }
    return noErr;
}

static OSStatus configure(Host *host, double rate, UInt32 channels) {
    AudioStreamBasicDescription format = {
        .mSampleRate = rate, .mFormatID = kAudioFormatLinearPCM,
        .mFormatFlags = kAudioFormatFlagsNativeFloatPacked | kAudioFormatFlagIsNonInterleaved,
        .mBytesPerPacket = sizeof(float), .mFramesPerPacket = 1, .mBytesPerFrame = sizeof(float),
        .mChannelsPerFrame = channels, .mBitsPerChannel = 32
    };
    OSStatus status = AudioUnitSetProperty(host->unit, kAudioUnitProperty_StreamFormat,
                                           kAudioUnitScope_Output, 0, &format, sizeof(format));
    if (status) return status;
    if (!host->instrument) {
        status = AudioUnitSetProperty(host->unit, kAudioUnitProperty_StreamFormat,
                                      kAudioUnitScope_Input, 0, &format, sizeof(format));
        if (status) return status;
        AURenderCallbackStruct callback = {pull_input, host};
        status = AudioUnitSetProperty(host->unit, kAudioUnitProperty_SetRenderCallback,
                                      kAudioUnitScope_Input, 0, &callback, sizeof(callback));
        if (status) return status;
    }
    UInt32 limit = BlockFrames;
    status = AudioUnitSetProperty(host->unit, kAudioUnitProperty_MaximumFramesPerSlice,
                                  kAudioUnitScope_Global, 0, &limit, sizeof(limit));
    if (status) {
        UInt32 size = sizeof(limit);
        if (AudioUnitGetProperty(host->unit, kAudioUnitProperty_MaximumFramesPerSlice,
                                kAudioUnitScope_Global, 0, &limit, &size) || limit < BlockFrames) return status;
    }
    host->channels = channels;
    host->sampleTime = 0;
    return AudioUnitInitialize(host->unit);
}

int32_t lm_au_activate(Host *host, double rate) {
    @autoreleasepool {
        if (host->initialized) AudioUnitUninitialize(host->unit);
        host->initialized = NO;
        OSStatus status = configure(host, rate, 2);
        if (status) status = configure(host, rate, 1);
        host->initialized = !status;
        return status;
    }
}

void lm_au_reset(Host *host) {
    AudioUnitReset(host->unit, kAudioUnitScope_Global, 0);
    host->sampleTime = 0;
}

void lm_au_midi(Host *host, uint32_t offset, uint32_t status, uint32_t a, uint32_t b) {
    MusicDeviceMIDIEvent(host->unit, status, a, b, offset);
}

void lm_au_transport(Host *host, const Transport *time) {
    host->transportChanged = host->transport.playing != time->playing ||
        host->transport.cycling != time->cycling ||
        (time->playing && fabs(host->expectedFrame - time->frame) > 1);
    host->transport = *time;
}

int32_t lm_au_render(Host *host, uint32_t frames, float *left, float *right) {
    if (!host->initialized || frames > BlockFrames) return kAudio_ParamError;
    UInt32 bytes = frames * sizeof(float);
    host->frames = frames;
    if (!host->instrument) {
        if (host->channels == 2) {
            memcpy(host->input[0], left, bytes);
            memcpy(host->input[1], right, bytes);
        } else {
            for (UInt32 i = 0; i < frames; ++i) host->input[0][i] = (left[i] + right[i]) * 0.5f;
        }
    }
    struct { UInt32 count; AudioBuffer buffers[2]; } buffers = { .count = host->channels };
    for (UInt32 channel = 0; channel < host->channels; ++channel) {
        memset(host->output[channel], 0, bytes);
        buffers.buffers[channel] = (AudioBuffer){1, bytes, host->output[channel]};
    }
    AudioTimeStamp time = { .mSampleTime = host->sampleTime, .mFlags = kAudioTimeStampSampleTimeValid };
    AudioUnitRenderActionFlags flags = 0;
    OSStatus status = AudioUnitRender(host->unit, &flags, &time, 0, frames, (AudioBufferList *)&buffers);
    host->sampleTime += frames;
    host->expectedFrame = host->transport.frame + (host->transport.playing ? frames : 0);
    if (status) return status;
    if (buffers.count != host->channels ||
        buffers.buffers[0].mDataByteSize < bytes ||
        buffers.buffers[host->channels - 1].mDataByteSize < bytes) return kAudio_ParamError;
    float *outLeft = buffers.buffers[0].mData;
    float *outRight = buffers.buffers[host->channels - 1].mData;
    if (outLeft) memcpy(left, outLeft, bytes); else memset(left, 0, bytes);
    if (outRight) memcpy(right, outRight, bytes); else memset(right, 0, bytes);
    return noErr;
}

uint32_t lm_au_param_ids(Host *host, uint32_t *ids, uint32_t capacity) {
    UInt32 size = 0;
    if (AudioUnitGetPropertyInfo(host->unit, kAudioUnitProperty_ParameterList,
                                 kAudioUnitScope_Global, 0, &size, NULL)) return 0;
    if (!ids) return size / sizeof(AudioUnitParameterID);
    size = MIN(size, capacity * sizeof(AudioUnitParameterID));
    if (AudioUnitGetProperty(host->unit, kAudioUnitProperty_ParameterList, kAudioUnitScope_Global,
                             0, ids, &size)) return 0;
    return size / sizeof(AudioUnitParameterID);
}

bool lm_au_param_info(Host *host, uint32_t id, Parameter *out) {
    @autoreleasepool {
        AudioUnitParameterInfo info = {0};
        UInt32 size = sizeof(info);
        if (AudioUnitGetProperty(host->unit, kAudioUnitProperty_ParameterInfo, kAudioUnitScope_Global,
                                 id, &info, &size)) return false;
        out->id = id;
        out->minimum = info.minValue;
        out->maximum = info.maxValue;
        out->initial = info.defaultValue;
        out->bits = (!!(info.flags & kAudioUnitParameterFlag_IsWritable))
                    | ((!!(info.flags & kAudioUnitParameterFlag_IsReadable)) << 1)
                    | ((!!(info.flags & kAudioUnitParameterFlag_NonRealTime)) << 2);
        if (info.unit == kAudioUnitParameterUnit_Boolean ||
            info.unit == kAudioUnitParameterUnit_Indexed) {
            float range = info.maxValue - info.minValue;
            if (range >= 1 && range <= 65535) out->steps = (uint32_t)range;
        }
        memcpy(out->name, info.name, sizeof(info.name));
        out->name[sizeof(info.name)] = 0;
        if ((info.flags & kAudioUnitParameterFlag_HasCFNameString) && info.cfNameString) {
            char name[256] = {0};
            if (CFStringGetCString(info.cfNameString, name, sizeof(name), kCFStringEncodingUTF8))
                memcpy(out->name, name, sizeof(name));
            if (info.flags & kAudioUnitParameterFlag_CFNameRelease) CFRelease(info.cfNameString);
        }
        return true;
    }
}

bool lm_au_get(Host *host, uint32_t id, float *value) {
    return !AudioUnitGetParameter(host->unit, id, kAudioUnitScope_Global, 0, value);
}

bool lm_au_set(Host *host, uint32_t id, float value, uint32_t offset) {
    if (offset) {
        AudioUnitParameterEvent event = {
            .scope = kAudioUnitScope_Global, .element = 0, .parameter = id,
            .eventType = kParameterEvent_Immediate,
            .eventValues.immediate = { .bufferOffset = offset, .value = value }
        };
        if (!AudioUnitScheduleParameters(host->unit, &event, 1)) return true;
    }
    return !AudioUnitSetParameter(host->unit, id, kAudioUnitScope_Global, 0, value, offset);
}

double lm_au_latency(Host *host) {
    Float64 latency = 0;
    UInt32 size = sizeof(latency);
    AudioUnitGetProperty(host->unit, kAudioUnitProperty_Latency, kAudioUnitScope_Global, 0, &latency, &size);
    return latency;
}

void lm_au_offline(Host *host, uint32_t offline) {
    AudioUnitSetProperty(host->unit, kAudioUnitProperty_OfflineRender, kAudioUnitScope_Global,
                         0, &offline, sizeof(offline));
}

const uint8_t *lm_au_save(Host *host, size_t *length, void **owner) {
    CFPropertyListRef state = NULL;
    UInt32 size = sizeof(state);
    if (AudioUnitGetProperty(host->unit, kAudioUnitProperty_ClassInfo, kAudioUnitScope_Global,
                             0, &state, &size) || !state) return NULL;
    CFDataRef data = CFPropertyListCreateData(NULL, state, kCFPropertyListBinaryFormat_v1_0, 0, NULL);
    CFRelease(state);
    if (!data) return NULL;
    *owner = (void *)data;
    *length = CFDataGetLength(data);
    return CFDataGetBytePtr(data);
}

void lm_au_release_data(void *data) { if (data) CFRelease(data); }

bool lm_au_load(Host *host, const uint8_t *bytes, size_t length) {
    CFDataRef data = CFDataCreate(NULL, bytes, length);
    if (!data) return false;
    CFPropertyListRef state = CFPropertyListCreateWithData(NULL, data, kCFPropertyListImmutable, NULL, NULL);
    CFRelease(data);
    if (!state) return false;
    bool ok = CFGetTypeID(state) == CFDictionaryGetTypeID() &&
        !AudioUnitSetProperty(host->unit, kAudioUnitProperty_ClassInfo, kAudioUnitScope_Global,
                              0, &state, sizeof(state));
    CFRelease(state);
    return ok;
}

bool lm_au_has_editor(Host *host) {
    UInt32 size = 0;
    return !AudioUnitGetPropertyInfo(host->unit, kAudioUnitProperty_CocoaUI, kAudioUnitScope_Global,
                                    0, &size, NULL) && size >= sizeof(AudioUnitCocoaViewInfo);
}

bool lm_au_editor_size(Host *host, uint32_t *width, uint32_t *height) {
    if (!host->editor || !NSThread.isMainThread) return false;
    NSSize size = [(__bridge NSView *)host->editor frame].size;
    if (!isfinite(size.width) || !isfinite(size.height) || size.width < 1 || size.height < 1) return false;
    *width = (uint32_t)ceil(MIN(size.width, 16384));
    *height = (uint32_t)ceil(MIN(size.height, 16384));
    return true;
}

bool lm_au_open_editor(Host *host, void *parent, uint32_t *width, uint32_t *height) {
    if (!parent || !NSThread.isMainThread) return false;
    @autoreleasepool {
        if (host->editor) return lm_au_editor_size(host, width, height);
        UInt32 size = 0;
        if (AudioUnitGetPropertyInfo(host->unit, kAudioUnitProperty_CocoaUI, kAudioUnitScope_Global,
                                     0, &size, NULL) || size < sizeof(AudioUnitCocoaViewInfo) || size > 65536) return false;
        AudioUnitCocoaViewInfo *info = calloc(1, size);
        if (!info) return false;
        UInt32 capacity = size;
        OSStatus status = AudioUnitGetProperty(host->unit, kAudioUnitProperty_CocoaUI,
                                               kAudioUnitScope_Global, 0, info, &size);
        @try {
            if (status || size < sizeof(AudioUnitCocoaViewInfo) || size > capacity || !info->mCocoaAUViewBundleLocation) return false;
            NSBundle *bundle = [NSBundle bundleWithURL:(__bridge NSURL *)info->mCocoaAUViewBundleLocation];
            size_t count = (size - offsetof(AudioUnitCocoaViewInfo, mCocoaAUViewClass)) / sizeof(CFStringRef);
            for (size_t i = 0; i < count; ++i) {
                if (!info->mCocoaAUViewClass[i]) continue;
                Class cls = [bundle classNamed:(__bridge NSString *)info->mCocoaAUViewClass[i]];
                if (![cls conformsToProtocol:@protocol(AUCocoaUIBase)]) continue;
                id<AUCocoaUIBase> factory = [[cls alloc] init];
                NSView *view = [factory uiViewForAudioUnit:host->unit withSize:NSZeroSize];
                if (!view) continue;
                [view setFrameOrigin:NSZeroPoint];
                [view setAutoresizingMask:NSViewNotSizable];
                [(__bridge NSView *)parent addSubview:view];
                host->editor = (__bridge_retained void *)view;
                if (lm_au_editor_size(host, width, height)) return true;
                lm_au_close_editor(host);
            }
        } @catch (NSException *exception) {
            (void)exception;
            lm_au_close_editor(host);
        } @finally {
            if (!status && size >= sizeof(AudioUnitCocoaViewInfo) && size <= capacity) {
                if (info->mCocoaAUViewBundleLocation) CFRelease(info->mCocoaAUViewBundleLocation);
                size_t count = (size - offsetof(AudioUnitCocoaViewInfo, mCocoaAUViewClass)) / sizeof(CFStringRef);
                for (size_t i = 0; i < count; ++i)
                    if (info->mCocoaAUViewClass[i]) CFRelease(info->mCocoaAUViewClass[i]);
            }
            free(info);
        }
    }
    return false;
}
