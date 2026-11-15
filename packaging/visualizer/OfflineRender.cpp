// SPDX-License-Identifier: AGPL-3.0-or-later
// Offline entry point for the AudioVisualizer renderer, based on v0.77 by mtyas.
#include "render/Renderer.h"
#include <juce_audio_formats/juce_audio_formats.h>
#include <windows.h>
#include <algorithm>
#include <fstream>
#include <iostream>
#include <fcntl.h>
#include <io.h>
#include <stdexcept>
#include <vector>

namespace {
using namespace juce::gl;

class WindowContext {
public:
    WindowContext() {
        window = CreateWindowExW(0, L"STATIC", L"ML Visualizer Export", WS_POPUP,
                                 0, 0, 8, 8, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
        if (!window) throw std::runtime_error("Could not create the offscreen rendering window");
        dc = GetDC(window);
        PIXELFORMATDESCRIPTOR format{};
        format.nSize = sizeof(format);
        format.nVersion = 1;
        format.dwFlags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL;
        format.iPixelType = PFD_TYPE_RGBA;
        format.cColorBits = 32;
        format.cDepthBits = 24;
        const int index = ChoosePixelFormat(dc, &format);
        if (!index || !SetPixelFormat(dc, index, &format))
            throw std::runtime_error("Could not choose an OpenGL pixel format");
        legacy = wglCreateContext(dc);
        if (!legacy || !wglMakeCurrent(dc, legacy))
            throw std::runtime_error("Could not start OpenGL");
        using CreateContext = HGLRC (WINAPI*)(HDC, HGLRC, const int*);
        auto create = reinterpret_cast<CreateContext>(wglGetProcAddress("wglCreateContextAttribsARB"));
        const int attributes[]{0x2091, 3, 0x2092, 2, 0x9126, 1, 0};
        if (!create || !(context = create(dc, nullptr, attributes)) || !wglMakeCurrent(dc, context))
            throw std::runtime_error("OpenGL 3.2 is required for visualizer export");
        juce::gl::loadFunctions();
    }

    ~WindowContext() {
        wglMakeCurrent(nullptr, nullptr);
        if (context) wglDeleteContext(context);
        if (legacy) wglDeleteContext(legacy);
        if (dc) ReleaseDC(window, dc);
        if (window) DestroyWindow(window);
    }

private:
    HWND window{};
    HDC dc{};
    HGLRC legacy{}, context{};
};

void render(const juce::File& scene, const juce::File& wav,
            const juce::File& progress, int width, int height, int fps, double tempo, double startBeat) {
    juce::WavAudioFormat format;
    auto input = wav.createInputStream();
    if (!input) throw std::runtime_error("Could not read rendered audio");
    std::unique_ptr<juce::AudioFormatReader> reader(format.createReaderFor(input.release(), true));
    if (!reader || reader->sampleRate <= 0 || reader->lengthInSamples <= 0)
        throw std::runtime_error("The rendered mix contains no audio frames");
    const auto json = juce::JSON::parse(scene.loadFileAsString());
    if (!json.isObject() || !json.hasProperty("visual"))
        throw std::runtime_error("Could not read the saved visualizer scene");
    const auto media = json.getProperty("videoFile", "").toString();
    if (media.isNotEmpty()) {
        const juce::File file(media);
        if (!file.hasFileExtension("png;jpg;jpeg;gif;bmp") || !file.existsAsFile())
            throw std::runtime_error("Choose an existing still image for the background in this export preview");
    }

    WindowContext gl;
    juce::OpenGLContext shaderContext;
    shaderContext.extensions.initialise();
    Engine engine;
    engine.loadFullState(json);
    engine.analyzer.prepare(reader->sampleRate);
    const auto layers = engine.visual.getLayerStack();
    for (int i = 0; i < layers.count; ++i) {
        const auto id = layers.ids[(size_t) i];
        if (engine.visual.getLayerType(id) == LayerType::WebcamFX
            && engine.visual.getBase(engine.visual.getLayerPrefix(id) + ".enabled") > 0.5f)
            throw std::runtime_error("Disable webcam layers before exporting");
    }
    if (media.isNotEmpty()) {
        for (int attempt = 0; attempt < 300 && !engine.videoDecoder.isLoaded(); ++attempt)
            juce::Thread::sleep(10);
        if (!engine.videoDecoder.isLoaded())
            throw std::runtime_error("The visualizer could not load its background image");
    }
    engine.bus.set(engine.sBpm, (float) tempo);
    engine.bus.set(engine.sPlaying, 1.0f);
    if (engine.analyzer.source >= Analyzer::Sidechain1)
        throw std::runtime_error("Choose a main audio source before exporting this scene");
    Renderer renderer(engine, shaderContext);
    renderer.newOpenGLContextCreated();
    if (!renderer.ready())
        throw std::runtime_error("Could not compile the visualizer shaders on this GPU");

    GLuint framebuffer{}, texture{};
    glGenFramebuffers(1, &framebuffer);
    glBindFramebuffer(GL_FRAMEBUFFER, framebuffer);
    glGenTextures(1, &texture);
    glBindTexture(GL_TEXTURE_2D, texture);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, width, height, 0, GL_RGBA, GL_UNSIGNED_BYTE, nullptr);
    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, texture, 0);
    if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE)
        throw std::runtime_error("Could not create the export framebuffer");

    if (_setmode(_fileno(stdout), _O_BINARY) == -1)
        throw std::runtime_error("Could not open the video frame pipe");
    std::vector<unsigned char> rgba(size_t(width) * height * 4);
    juce::AudioBuffer<float> samples(2, 512);
    const auto sampleRate = (int64_t) reader->sampleRate;
    const int64_t count = (reader->lengthInSamples * fps + sampleRate - 1) / sampleRate;
    int64_t sample = 0;
    for (int64_t frame = 0; frame < count; ++frame) {
        const auto until = std::min<int64_t>(reader->lengthInSamples, (frame + 1) * sampleRate / fps);
        while (sample < until) {
            const auto n = (int) std::min<int64_t>(512, until - sample);
            if (!reader->read(&samples, 0, n, sample, true, true))
                throw std::runtime_error("Could not read a block of rendered audio");
            engine.analyzer.process(samples.getReadPointer(0), samples.getReadPointer(1), nullptr, nullptr, nullptr, nullptr, n);
            sample += n;
        }
        const double beat = startBeat + (double)frame / fps * tempo / 60.0;
        engine.modulation.hostPpq = beat;
        engine.bus.set(engine.sBeat, (float)(beat - std::floor(beat)));
        renderer.renderFrame(width, height, 1.0 / fps, framebuffer);
        glReadPixels(0, 0, width, height, GL_RGBA, GL_UNSIGNED_BYTE, rgba.data());
        if (glGetError() != GL_NO_ERROR)
            throw std::runtime_error("The GPU could not render a video frame");
        if (std::fwrite(rgba.data(), 1, rgba.size(), stdout) != rgba.size())
            throw std::runtime_error("Could not write the temporary video frames");
        if (frame % 5 == 0 || frame + 1 == count)
            progress.replaceWithText(juce::String(frame + 1) + "/" + juce::String(count));
    }
    if (std::fflush(stdout) != 0) throw std::runtime_error("Could not flush video frames");
    renderer.openGLContextClosing();
    glDeleteTextures(1, &texture);
    glDeleteFramebuffers(1, &framebuffer);
}
}

int wmain(int argc, wchar_t** argv) {
    try {
        juce::ScopedJuceInitialiser_GUI init;
        if (argc != 9)
            throw std::runtime_error("Expected scene, WAV, progress, width, height, fps, tempo, start beat");
        const int width = juce::String(argv[4]).getIntValue(), height = juce::String(argv[5]).getIntValue(), fps = juce::String(argv[6]).getIntValue();
        if (width < 2 || height < 2 || width > 1920 || height > 1920 || fps != 30)
            throw std::runtime_error("Unsupported video dimensions or frame rate");
        render(juce::File(argv[1]), juce::File(argv[2]), juce::File(argv[3]), width, height, fps, juce::String(argv[7]).getDoubleValue(), juce::String(argv[8]).getDoubleValue());
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
