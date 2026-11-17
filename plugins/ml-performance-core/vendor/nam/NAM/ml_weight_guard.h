// ML addition: reject truncated user models before reading beyond their weights.
#pragma once
#include <vector>
#include <stdexcept>
namespace nam {
inline thread_local std::vector<float>::iterator ml_weight_end;
inline thread_local bool ml_weight_active=false;
struct MLWeightGuard {
 std::vector<float>::iterator previous; bool active;
 explicit MLWeightGuard(std::vector<float>& w):previous(ml_weight_end),active(ml_weight_active){ml_weight_end=w.end();ml_weight_active=true;}
 ~MLWeightGuard(){ml_weight_end=previous;ml_weight_active=active;}
};
inline float ml_read_weight(std::vector<float>::iterator& it) {
 if(ml_weight_active && it==ml_weight_end)throw std::runtime_error("NAM model has truncated weights");
 return *it++;
}
}
