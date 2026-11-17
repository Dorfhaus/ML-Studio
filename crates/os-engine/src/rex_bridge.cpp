// SPDX-License-Identifier: GPL-3.0-or-later
// One TU keeps the native implementation private and allows exception-safe ownership.
#include "velociloops.cpp"

struct MLREX {
    std::unique_ptr<VLFile_s> file;
    VLFileInfo info{};
    int32_t frames = 0;
};
static constexpr int32_t ml_rex_max_frames = 16 * 1024 * 1024;
static int64_t ml_rex_frame(const MLREX& f, int32_t ppq) {
    return std::llround(double(ppq) * f.info.sample_rate * 60000. / (15360. * f.info.tempo));
}
extern "C" void* ml_rex_open(const uint8_t* data, size_t size, int32_t* info) noexcept {
    if (!data || !info || size == 0 || size > 64u * 1024u * 1024u) return nullptr;
    try {
        auto result = std::make_unique<MLREX>();
        result->file = std::make_unique<VLFile_s>();
        if (!result->file->impl.loadFromBuffer(reinterpret_cast<const char*>(data), size)) return nullptr;
        if (vl_get_info(result->file.get(), &result->info) != VL_OK) return nullptr;
        auto& v=result->info;
        if (v.sample_rate < 8000 || v.sample_rate > 192000 || v.tempo <= 0 || v.ppq_length <= 0 ||
            v.slice_count <= 0 || v.slice_count > 65536 || v.channels < 1 || v.channels > 2) return nullptr;
        auto frames=ml_rex_frame(*result, v.ppq_length);
        if (frames <= 0 || frames > ml_rex_max_frames) return nullptr;
        result->frames=static_cast<int32_t>(frames);
        info[0]=v.sample_rate; info[1]=result->frames; info[2]=v.tempo; info[3]=v.ppq_length; info[4]=v.slice_count;
        return result.release();
    } catch (...) {return nullptr;}
}
extern "C" void ml_rex_close(void* handle) noexcept {delete static_cast<MLREX*>(handle);}
extern "C" bool ml_rex_render(void* handle, float* left, float* right, int32_t* cues) noexcept {
    if (!handle || !left || !right || !cues) return false;
    try {
        auto& f=*static_cast<MLREX*>(handle);
        std::fill_n(left,f.frames,0.f); std::fill_n(right,f.frames,0.f);
        std::vector<float> l,r;
        for (int32_t i=0;i<f.info.slice_count;++i) {
            VLSliceInfo slice{};
            if (vl_get_slice_info(f.file.get(),i,&slice)!=VL_OK || slice.ppq_pos<0) return false;
            auto start=ml_rex_frame(f,slice.ppq_pos);
            if (start<0 || start>f.frames) return false;
            cues[i]=static_cast<int32_t>(start);
            const int32_t total=vl_get_slice_frame_count(f.file.get(),i);
            if (total<0 || total>ml_rex_max_frames) return false;
            int32_t count=std::min<int32_t>(total,f.frames-static_cast<int32_t>(start));
            if (!count) continue;
            l.resize(total); r.resize(total); int32_t written=0;
            if (vl_decode_slice(f.file.get(),i,l.data(),r.data(),0,total,&written)!=VL_OK || written<0 || written>total) return false;
            for (int32_t j=0;j<std::min(written,count);++j) {left[start+j]+=l[j];right[start+j]+=r[j];}
        }
        return true;
    } catch (...) {return false;}
}
