#pragma once
#include <immintrin.h>
#include <stdint.h>

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
// #define TAU 2.0 * M_PI
void fm_modulate(f64 output_signal[], const f64 input_signal[],f64* const prev_sig,f64* const sum, const f64 sample_periodic, f64* const _angle, const f64 modulate_index, const f64 fc, usize const buf_len);
void fm_demodulate(f64 output_signal[], const f64 input_signal[], const f64 sample_period,void* const filter_coeff, FilterInfo filter_info[],f64* prev, f64* const angle,f64 const carrier_freq, const usize buf_len);
