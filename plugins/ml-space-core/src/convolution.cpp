#include "TwoStageFFTConvolver.h"
#include <new>
#include <memory>
extern "C" {
void* ml_conv_new(const float* ir,size_t n) noexcept {
 try { auto c=std::unique_ptr<fftconvolver::TwoStageFFTConvolver>(new fftconvolver::TwoStageFFTConvolver()); if(!c->init(128,2048,ir,n)) return nullptr; return c.release(); } catch(...) { return nullptr; }
}
void ml_conv_process(void* p,const float* in,float* out,size_t n) { static_cast<fftconvolver::TwoStageFFTConvolver*>(p)->process(in,out,n); }
void ml_conv_clear(void* p) { static_cast<fftconvolver::TwoStageFFTConvolver*>(p)->clearHistory(); }
void ml_conv_free(void* p) { delete static_cast<fftconvolver::TwoStageFFTConvolver*>(p); }
}
