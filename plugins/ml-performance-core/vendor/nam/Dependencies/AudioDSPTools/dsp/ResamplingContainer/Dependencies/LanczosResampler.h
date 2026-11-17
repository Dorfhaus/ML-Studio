// File: LanczosResampler.h
// Created Date: Saturday December 16th 2023
// Author: Steven Atkinson (steven@atkinson.mn)


// This file originally came from the iPlug2 library and has been subsequently modified;
// the following license is copied as required from
// https://github.com/iPlug2/iPlug2/blob/40ebb560eba68f096221e99ef0ae826611fc2bda/LICENSE.txt
// -------------------------------------------------------------------------------------

/*
iPlug 2 C++ Plug-in Framework.

Copyright (C) the iPlug 2 Developers. Portions copyright other contributors, see each source file for more information.

Based on WDL-OL/iPlug by Oli Larkin (2011-2018), and the original iPlug v1 (2008) by John Schwartz / Cockos

LICENSE:

This software is provided 'as-is', without any express or implied warranty.  In no event will the authors be held liable
for any damages arising from the use of this software.

Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter it
and redistribute it freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim that you wrote the original software. If
you use this software in a product, an acknowledgment in the product documentation would be appreciated but is not
required.
1. Altered source versions must be plainly marked as such, and must not be misrepresented as being the original
software.
1. This notice may not be removed or altered from any source distribution.

iPlug 2 includes the following 3rd party libraries (see each license info):

* Cockos WDL https://www.cockos.com/wdl
* NanoVG https://github.com/memononen/nanovg
* NanoSVG https://github.com/memononen/nanosvg
* MetalNanoVG https://github.com/ollix/MetalNanoVG
* RTAudio https://www.music.mcgill.ca/~gary/rtaudio
* RTMidi https://www.music.mcgill.ca/~gary/rtmidi
*/
// -------------------------------------------------------------------------------------

/*
 This code is derived from
 https://github.com/surge-synthesizer/sst-basic-blocks/blob/main/include/sst/basic-blocks/dsp/LanczosResampler.h

 The following license info is copied from the above file:

 * sst-basic-blocks - an open source library of core audio utilities
 * built by Surge Synth Team.
 *
 * Provides a collection of tools useful on the audio thread for blocks,
 * modulation, etc... or useful for adapting code to multiple environments.
 *
 * Copyright 2023, various authors, as described in the GitHub
 * transaction log. Parts of this code are derived from similar
 * functions original in Surge or ShortCircuit.
 *
 * sst-basic-blocks is released under the GNU General Public Licence v3
 * or later (GPL-3.0-or-later). The license is found in the "LICENSE"
 * file in the root of this repository, or at
 * https://www.gnu.org/licenses/gpl-3.0.en.html
 *
 * All source in sst-basic-blocks available at
 * https://github.com/surge-synthesizer/sst-basic-blocks

 * A special note on licensing: This file (and only this file)
 * has Paul Walker (baconpaul) as the sole author to date.
 *
 * In order to make this handy small function based on public
 * information available to a set of open source projects
 * adapting hardware to software, but which are licensed under
 * MIT or BSD or similar licenses, this file and only this file
 * can be used in an MIT/BSD context as well as a GPL3 context, by
 * copying it and modifying it as you see fit.
 *
 * If you do that, you will need to replace the `sum_ps_to_float`
 * call below with either an hadd if you are SSE3 or higher or
 * an appropriate reduction operator from your toolkit.
 *
 * But basically: Need to resample 48k to variable rate with
 * a small window and want to use this? Go for it!
 *
 * For avoidance of doubt, this license exception only
 * applies to this file.
 */

#pragma once

#include <algorithm>
#include <utility>
#include <cmath>
#include <cstring>
#include <vector> // ML: per-rate antialias tables, prepared off the audio thread

#if defined AUDIODSPTOOLS_SIMDE
  #if defined(__arm64__)
    #define SIMDE_ENABLE_NATIVE_ALIASES
    #include "simde/x86/sse2.h"
  #else
    #include <emmintrin.h>
  #endif
#endif

// #include "IPlugConstants.h"

namespace dsp
{
/* LanczosResampler
 *
 * A class that implement Lanczos resampling, optionally using SIMD instructions.
 * Define AUDIODSPTOOLS_SIMDE at project level in order to use SIMD and if on non-x86_64
 * include the SIMDE library in your search paths in order to translate intel
 * intrinsics to e.g. arm64
 *
 * See https://en.wikipedia.org/wiki/Lanczos_resampling
 *
 * @tparam T the sampletype
 * @tparam NCHANS the number of channels
 * @tparam A The Lanczos filter size. A higher value makes the filter closer to an
   ideal stop-band that rejects high-frequency content (anti-aliasing),
   but at the expense of higher latency
 */
template <typename T = double, int NCHANS = 2, size_t A = 12>
class LanczosResampler
{
private:
#if AUDIODSPTOOLS_SIMDE
  static_assert(std::is_same<T, float>::value, "LanczosResampler requires T to be float when using SIMD instructions");
  static_assert(false, "SIMD version has not been checked! You need to remove this to use it at your own risk!");
#endif

  // The buffer size. This needs to be at least as large as the largest block of samples
  // that the input side will see.
  // WARNING: hard-coded to accommodate 8192 samples, from 44.1 to 192k!
  // If someone is past this, then maybe they know what they're doing ;)
  // (size_t)(8192 * 3 * 192000 / 44100)+1 = 106998
  static constexpr size_t kBufferSize = 131072; // Round up to pow2. I don't know why, but it doesn't work otherwise.
  // The filter width. 2x because the filter goes from -A to A
  size_t mRadius = A;
  size_t mWidth = 2*A;
  // The discretization resolution for the filter table.
  static constexpr size_t kTablePoints = 1024;
  static constexpr double kDeltaX = 1.0 / (kTablePoints);

public:
  /** Constructor
   * @param inputRate The input sample rate
   * @param outputRate The output sample rate
   */
  LanczosResampler(float inputRate, float outputRate)
  : mInputSampleRate(inputRate)
  , mOutputSampleRate(outputRate)
  {
    SetPhases();
    ClearBuffer();


    // ML: widen the window and lower the cutoff for downsampling; interpolation
    // alone would fold model harmonics above the destination Nyquist into audio.
    const double cutoff=std::min(1.0,double(outputRate)/inputRate);
    mRadius=size_t(std::ceil(A/cutoff));mWidth=2*mRadius;
    sTable.assign(kTablePoints+1,std::vector<T>(mWidth));
    sDeltaTable.assign(kTablePoints+1,std::vector<T>(mWidth));
    auto kernel = [this,cutoff](double x) {
      if (std::fabs(x) < 1e-7)
        return T(cutoff);

      const auto pi = 3.14159265358979323846; // ML: standalone constant; no iPlug runtime dependency.
      return T(mRadius * std::sin(pi * x * cutoff) * std::sin(pi * x / mRadius) / (pi * pi * x * x));
    };

    if (true)
    {
      for (auto t = 0; t < kTablePoints + 1; ++t)
      {
        const double x0 = kDeltaX * t;

        for (auto i = 0; i < mWidth; ++i)
        {
          const double x = x0 + i - double(mRadius);
          sTable[t][i] = kernel(x);
        }
      }

      for (auto t = 0; t < kTablePoints; ++t)
      {
        for (auto i = 0; i < mWidth; ++i)
        {
          sDeltaTable[t][i] = sTable[t + 1][i] - sTable[t][i];
        }
      }

      for (auto i = 0; i < mWidth; ++i)
      {
        // Wrap at the end - delta is the same
        sDeltaTable[kTablePoints][i] = sDeltaTable[0][i];
      }

    }
  }

  inline size_t GetNumSamplesRequiredFor(size_t nOutputSamples) const
  {
    /*
     * So (mPhaseIn + mPhaseInIncr * res - mPhaseOut - mPhaseOutIncr * nOutputSamples) * sri > A + 1
     *
     * Use the fact that mPhaseInIncr = mInputSampleRate and find
     * res > (mRadius+1) - (mPhaseIn - mPhaseOut + mPhaseOutIncr * desiredOutputs) * sri
     */
    double res = mRadius + 1.0
                 - ((double)(mPhaseInNumerator - mPhaseOutNumerator - mPhaseOutIncrNumerator * (long)nOutputSamples))
                     / ((double)mPhaseDenominator);

    return static_cast<size_t>(std::max(res + 1.0, 0.0));
  }

  inline void PushBlock(T** inputs, size_t nFrames)
  {
    for (auto s = 0; s < nFrames; s++)
    {
      for (auto c = 0; c < NCHANS; c++)
      {
        mInputBuffer[c][mWritePos] = inputs[c][s];
        mInputBuffer[c][mWritePos + kBufferSize] = inputs[c][s]; // this way we can always wrap
      }

      mWritePos = (mWritePos + 1) & (kBufferSize - 1);
      mPhaseInNumerator += mPhaseInIncrNumerator;
    }
  }

  size_t PopBlock(T** outputs, size_t max)
  {
    int populated = 0;
    while (populated < max && (mPhaseInNumerator - mPhaseOutNumerator) > mPhaseDenominator * (mRadius + 1))
    {
      ReadSamples(((double)(mPhaseInNumerator - mPhaseOutNumerator)) / ((double)mPhaseDenominator), outputs, populated);
      mPhaseOutNumerator += mPhaseOutIncrNumerator;
      populated++;
    }
    return populated;
  }

  inline void RenormalizePhases()
  {
    mPhaseInNumerator -= mPhaseOutNumerator;
    mPhaseOutNumerator = 0;
  }

  void Reset() { ClearBuffer(); }

  // ML modification: clear phase as well as audio, for repeatable host resets.
  void ClearBuffer() { memset(mInputBuffer, 0, NCHANS * kBufferSize * 2 * sizeof(T)); mWritePos=0;mPhaseInNumerator=0;mPhaseOutNumerator=0; }

private:
  // ML scalar, ratio-aware form of the upstream table-interpolated Lanczos reader.
  inline void ReadSamples(double xBack,T** outputs,int sample) const {
    const double position=mWritePos-xBack+1.0;
    int index=int(std::floor(position));const double frac=1.0-(position-index);
    index=(index+int(kBufferSize))&int(kBufferSize-1);
    if(index<=int(mRadius))index+=int(kBufferSize);
    const double table=frac*kTablePoints;const int row=int(table);const double f=table-row;
    for(int ch=0;ch<NCHANS;++ch){double sum=0.;for(size_t i=0;i<mWidth;++i){const double weight=sTable[row][i]+sDeltaTable[row][i]*f;sum+=weight*mInputBuffer[ch][index-int(mRadius)+int(i)];}outputs[ch][sample]=T(sum);}
  }
  void SetPhases()
  {
    // This is going to assume I can treat the sample rates as longs...
    // But if they're not, then things will sound just a little wrong and honestly I'm fine with that.
    // It's your fault for not using something normal like 44.1k, 48k, or their multiples.
    // (Looking at you, VST3PluginTestHost!)
    auto AssertLongLikeSampleRate = [](double x) {
      if ((double)((long)x) != x)
      {
        std::cerr << "Expected long-like sample rate; got " << x << " instead! Truncating..." << std::endl;
      }
      return (long)x;
    };

    // Greatest common denominator
    auto gcd = [](long a, long b) -> long {
      while (b != 0)
      {
        long temp = b;
        b = a % b;
        a = temp;
      }
      return a;
    };

    const long inputSampleRate = AssertLongLikeSampleRate(mInputSampleRate);
    const long outputSampleRate = AssertLongLikeSampleRate(mOutputSampleRate);
    const long g = gcd(inputSampleRate, outputSampleRate);

    // mPhaseInIncr = 1.0
    // mPhaseOutIncr = mInputSampleRate / mOutputSampleRate
    mPhaseInIncrNumerator = outputSampleRate / g;
    mPhaseDenominator = mPhaseInIncrNumerator; // val / val = 1
    mPhaseOutIncrNumerator = inputSampleRate / g;
  };

  std::vector<std::vector<T>> sTable,sDeltaTable;

  T mInputBuffer[NCHANS][kBufferSize * 2];
  int mWritePos = 0;
  const float mInputSampleRate;
  const float mOutputSampleRate;
  // Phase is treated as rational numbers to ensure floating point errors don't accumulate and we stay exactly on.
  // (Issue 15)
  long mPhaseInNumerator = 0;
  long mPhaseOutNumerator = 0;
  long mPhaseInIncrNumerator = 1;
  long mPhaseOutIncrNumerator = 1;
  long mPhaseDenominator = 1;
};

} // namespace dsp
