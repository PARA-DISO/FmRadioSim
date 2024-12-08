#pragma once
#include <immintrin.h>
#include "rstype.h"
#define ENABLE_UPSAMPLING 0
#define TEST_CODE false
#define DISABLE_SIMD_DEMODULATE 0
typedef uint64_t usize;
typedef double f64;
typedef __m256d f64x4;
typedef f64 FilterInfo[4];
typedef struct {
  f64 c0;
  f64 c1;
  f64 c2;
  f64 c3;
  f64 c4;
} FilterCoeffs;
typedef struct {
  f64 angle[4];
  f64 delta_angle;
  f64 prev_sig[4];
  f64 prev_cos[4];
  f64 next_cos[4];
  f64 stage[16];

  f64 filter_coeff;
  // FilterCoeffs filter_coeff;
  f64 filter_info[16];
} CnvFiInfos;
typedef struct {
  f64 prev_sig[2];
  f64 prev_prev_sig[2];
  f64 prev_out[2];
  f64 prev_prev_out[2];
  f64 stage[4];
  FilterCoeffs coeff;
} FilteringInfo;
// typedef struct {
//   f64 angle;
//   f64 prev_sin;
//   f64 prev_sig[2];
//   f64 prev_internal[2];
//   FilterCoeffs filter_coeff;
//   FilterInfo filter_info[6];
// } DemodulationInfo;
typedef struct {
  f64 integral[2];
  f64 t[4];
  f64 prev_sig[8];
  f64 sample_period;
  f64 carrier_freq;
  f64 modulation_index;
  f64 prev_inter_sig[4];
} ModulationInfo;
typedef struct {
  f64 angle[4];
  f64 prev_sin[4];
  f64 prev_sig[8];
  f64 prev_internal[8];
  // f64 filter_coeff;
  FilterCoeffs filter_coeff;
  // FilterInfo filter_info[6];
  // f64 filter_info[8];
  #if !DISABLE_SIMD_DEMODULATE
  // f64 filter_coeff;
  f64 filter_info[16];
  #else
  // FilterCoeffs filter_coeff;
  FilterInfo filter_info[6];
  #endif
  
} DemodulationInfo;
// #define TAU 2.0 * M_PI
void fm_modulate(f64 output_signal[], const f64 input_signal[], usize const buf_len, ModulationInfo* info);
void fm_demodulate(f64 output_signal[], const f64 input_signal[], const f64 sample_period,f64 const carrier_freq,DemodulationInfo* const info, const usize buf_len);
void convert_intermediate_freq(
  f64 output_signal[], const f64 input_signal[],
  const f64 sample_period,
  f64 const fc, f64 const fi,
  CnvFiInfos* const info, const usize buf_len);
typedef struct {
  f64 prev;
  usize multiplier;
  usize input_len;
} ResamplerInfo;
void upsample(f64* dst, f64* input, ResamplerInfo* info);
void downsample(f64* dst, f64* input, ResamplerInfo* info);
void filtering(f64* dst, f64* input, FilteringInfo* info,u64 buf_len);