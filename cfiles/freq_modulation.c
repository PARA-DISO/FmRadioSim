#include "./freq_modulation.h"
#include <math.h>
#include <stdio.h>
#include "consts.h"
// TODO: 係数がわたっているのか確認
// (結果が完全に0になる原因)
f64 lpf(f64 sig, FilterCoeffs* coeff,FilterInfo info) {
  const f64 in1 = info[0];
  const f64 in2 = info[1];
  const f64 out1 = info[2];
  const f64 out2 = info[3];
  const f64 buf = coeff->c0 * sig + coeff->c1 * in1 + coeff->c2 * in2
            - coeff->c3 * out1
            - coeff->c4 * out2;
  info[0] = sig;
  info[1] = in1;
  info[2] = buf;
  info[3] = out1;
  return buf;
}
void differential(f64* dr, f64* di,const f64 r, const f64 i, f64* prev, const f64 sample_period) {
  *dr = (r - prev[0]) / sample_period;
  *di = (i - prev[1]) / sample_period;
  prev[0] = r;
  prev[1]  = i;
}
void fm_modulate(f64 output_signal[], const f64 input_signal[],f64* const prev_sig, f64* const sum, const f64 sample_period, f64* const _angle, f64 modulate_index,const f64 fc, const usize buf_len) {
  // static f64x4 t = _mm256_load_pd();
  f64x4 angle = _mm256_load_pd(_angle);
  f64x4 phi = _mm256_set1_pd(TAU*fc*sample_period*4.);
  f64x4 coeff = _mm256_set1_pd(modulate_index * sample_period/2.);
  double s[4] = {0};
  f64 prev = *prev_sig;
  f64 current_sum = *sum;
  for (usize i = 0; i < buf_len; i+=4) {
    s[0] = prev + input_signal[i];
    s[1] = s[0] + input_signal[i+1];
    s[2] = s[1] + input_signal[i+2];
    s[3] = s[2] + input_signal[i+3];
    current_sum = s[3];
    prev = input_signal[i+3];
    f64x4 integral = _mm256_fmadd_pd(coeff,_mm256_load_pd(s),angle);
    f64x4 sigs = _mm256_cos_pd(integral);
    _mm256_store_pd(output_signal + i, sigs);
    angle = _mm256_add_pd(angle, phi);
    // output_signal[i] = cos(*angle + (modulate_index * sample_period/2. * *sum));
    // *angle += TAU * fc * sample_period;
    // *prev_sig = input_signal[i];
    
  }
  _mm256_store_pd(_angle,_mm256_fmod_pd(angle,_mm256_set1_pd(TAU)));
  *sum = current_sum;
  *prev_sig = prev;
  // for (usize i = 0; i < buf_len; i++) {
  //   *sum += *prev_sig + input_signal[i];
  //   output_signal[i] = cos(*angle + (modulate_index * sample_period/2. * *sum));
  //   *angle += TAU * fc * sample_period;
  //   *prev_sig = input_signal[i];
  // }
}
void fm_demodulate(f64 output_signal[], const f64 input_signal[], const f64 sample_period,void* const filter_coeff, FilterInfo filter_info[],f64* prev, f64* const angle,f64 const carrier_freq, const usize buf_len) {
  FilterCoeffs* const coeff = (FilterCoeffs*) filter_coeff;
  // printf("buffer len: %ld\n", buf_len);
  for (usize i = 0; i < buf_len; i++) {
    const f64 re = lpf(lpf(-input_signal[i] * sin(*angle),coeff,filter_info[0]),coeff,filter_info[1]);
    const f64 im = lpf(lpf(input_signal[i] * cos(*angle),coeff,filter_info[2]),coeff,filter_info[3]);
    f64 d_re,d_im;
    differential(&d_re,&d_im,re,im,prev,sample_period);
    f64 a = d_re * im;
    f64 b = d_im * re;
    output_signal[i] = a - b;
    *angle += TAU * carrier_freq * sample_period;
  }
}