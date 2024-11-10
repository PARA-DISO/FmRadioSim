#pragma once
#include <immintrin.h>
#include "rstype.h"

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
  f64 stage[8];
  f64 filter_coeff;
  f64 filter_info[8];
} CnvFiInfos;
typedef struct {
  f64 angle;
  f64 prev_sin;
  f64 prev_sig[2];
  f64 prev_internal[2];
  FilterCoeffs filter_coeff;
  FilterInfo filter_info[6];
} DemodulationInfo;
// #define TAU 2.0 * M_PI
void fm_modulate(f64 output_signal[], const f64 input_signal[],f64* const prev_sig,f64* const sum, const f64 sample_periodic, f64* const _angle, const f64 modulate_index, const f64 fc, usize const buf_len);
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