#include <algorithm>
#include <array>
#include <cmath>
#include <cstring>
#include <mutex>
#include <stdexcept>
#include "NAM/get_dsp.h"
#include "NAM/lstm.h"
#include "NAM/linear.h"
#include "NAM/convnet.h"
#include "NAM/wavenet/model.h"
#include "Dependencies/AudioDSPTools/dsp/ResamplingContainer/ResamplingContainer.h"
namespace {
std::mutex load_mutex;
constexpr int DELAY=256, BLOCK=256;
// Validate structural sizes before the upstream factory allocates user-supplied shapes.
void validate_shape(const nlohmann::json& j,const std::string& key="",int depth=0) {
 if(depth>16)throw std::runtime_error("NAM configuration is too deeply nested");
 if((key=="condition_dsp" || key=="slimmable") && !j.is_null() && j!=false)
  throw std::runtime_error("Nested and slimmable NAM models are not supported yet");
 if(j.is_object()){if(j.size()>64)throw std::runtime_error("Oversized NAM configuration");for(auto i=j.begin();i!=j.end();++i)validate_shape(i.value(),i.key(),depth+1);}
 if(j.is_array()){if(j.empty() || j.size()>64)throw std::runtime_error("Invalid NAM layer list");long sum=0;for(const auto& v:j){validate_shape(v,key,depth+1);if(key=="dilations")sum+=v.get<int>();}if(sum>65536)throw std::runtime_error("NAM receptive field is too large");}
 if(j.is_number()){
  double v=j.get<double>();if(!std::isfinite(v)||std::abs(v)>65536)throw std::runtime_error("Invalid NAM configuration value");
  if(key=="in_channels" || key=="out_channels" || key=="condition_size") {if(v!=1)throw std::runtime_error("Choose a mono NAM model");}
  if(key.find("size")!=std::string::npos || key.find("channels")!=std::string::npos || key=="groups" || key=="groups_input_mixin" || key=="bottleneck" || key=="num_layers" || key=="dilations" || key=="receptive_field") {
   const int max=key=="dilations"?16384:key=="receptive_field"?4096:key.find("kernel")!=std::string::npos?32:key=="num_layers"?16:256;
   if(v<1 || v>max || std::floor(v)!=v)throw std::runtime_error("Unsupported NAM layer dimensions");
  }
 }
}
struct Model {
 std::unique_ptr<nam::DSP> dsp;
 std::unique_ptr<dsp::ResamplingContainer<float,1,12>> resampler;
 std::array<float,DELAY> padding{};int pos=0,delay=DELAY;
 nam::lstm::LSTM* lstm=nullptr;nam::wavenet::WaveNet* wave=nullptr;
 nam::convnet::ConvNet* conv=nullptr;nam::Linear* linear=nullptr;
 explicit Model(nlohmann::json json,double host) {
  const auto arch=json.at("architecture").get<std::string>();
  if(arch!="LSTM" && arch!="WaveNet" && arch!="ConvNet" && arch!="Linear")
   throw std::runtime_error("This model architecture is not supported yet (use LSTM, WaveNet, ConvNet or Linear)");
  validate_shape(json.at("config"));
  if(arch=="LSTM" && json["config"].at("input_size")!=1)throw std::runtime_error("Choose an unconditioned mono LSTM model");
  if(arch=="Linear") {json["config"]["implementation"]="direct";if(json["config"].value("receptive_field",1)>4096)throw std::runtime_error("Use ML Convolver for long impulse responses");}
  nam::dspData config;nam::DspLoadOptions options;options.prewarm=false;
  dsp=nam::get_dsp(json,config,options);
  if(dsp->NumInputChannels()!=1 || dsp->NumOutputChannels()!=1)throw std::runtime_error("Choose a mono NAM model; ML Amp runs two independent copies for stereo");
  double rate=dsp->GetExpectedSampleRate();if(rate==NAM_UNKNOWN_EXPECTED_SAMPLE_RATE)rate=host;
  else if(!std::isfinite(rate)||rate<32000.||rate>192000.)throw std::runtime_error("Model training rate must be 32–192 kHz");
  const int maxFrames=std::max(512,int(std::ceil(BLOCK*rate/host))+32);
  dsp->SetPrewarmOnReset(false);dsp->Reset(rate,maxFrames);dsp->prewarm();
  lstm=dynamic_cast<nam::lstm::LSTM*>(dsp.get());wave=dynamic_cast<nam::wavenet::WaveNet*>(dsp.get());conv=dynamic_cast<nam::convnet::ConvNet*>(dsp.get());linear=dynamic_cast<nam::Linear*>(dsp.get());
  if(lstm)for(auto& l:lstm->_layers)l.MLCache();
  if(wave && !wave->MLCanRestore())throw std::runtime_error("Conditioned or nested WaveNet models are not supported yet");
  if(conv)conv->MLCache();
  if(!lstm&&!wave&&!conv&&!linear)throw std::runtime_error("Unsupported NAM model variant");
  if(rate!=host){resampler=std::make_unique<dsp::ResamplingContainer<float,1,12>>(rate);resampler->Reset(host,BLOCK);delay=DELAY-resampler->GetLatency();if(delay<0)throw std::runtime_error("Model sample-rate conversion exceeds supported latency");}
 }
 void clear() {
  if(lstm)for(auto& l:lstm->_layers)l.MLRestore();if(wave)wave->MLRestore();if(conv)conv->MLRestore();if(linear)linear->MLClear();
  if(resampler)resampler->MLClear();padding.fill(0.f);pos=0;
 }
 void process(const float* input,float* output,int n){
  float* in=const_cast<float*>(input);float* out=output;
  if(resampler)resampler->ProcessBlock(&in,&out,n,[this](float** a,float** b,int frames){dsp->process(a,b,frames);});else dsp->process(&in,&out,n);
  for(int i=0;i<n;++i){if(!std::isfinite(output[i]))throw std::runtime_error("Non-finite model output");if(delay){float x=output[i];output[i]=padding[pos];padding[pos]=x;pos=(pos+1)%delay;}}
 }
};
}
extern "C" {
void* ml_amp_new(const char* data,size_t size,double rate,char* error,size_t capacity) noexcept {
 try {std::lock_guard<std::mutex> guard(load_mutex);auto json=nlohmann::json::parse(data,data+size);return new Model(std::move(json),rate);}
 catch(const std::exception& e){if(capacity){std::strncpy(error,e.what(),capacity-1);error[capacity-1]=0;}return nullptr;}
 catch(...){if(capacity){std::strncpy(error,"Could not prepare model",capacity-1);error[capacity-1]=0;}return nullptr;}
}
bool ml_amp_process(void* p,const float* in,float* out,int n) noexcept {try{static_cast<Model*>(p)->process(in,out,n);return true;}catch(...){std::fill_n(out,n,0.f);return false;}}
void ml_amp_clear(void* p) noexcept {static_cast<Model*>(p)->clear();}
void ml_amp_free(void* p) noexcept {delete static_cast<Model*>(p);}
}
