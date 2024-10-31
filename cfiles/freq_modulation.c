#include "./freq_modulation.h"
#include <math.h>
#include <stdio.h>

// TODO: 係数がわたっているのか確認
// (結果が完全に0になる原因)
inline f64 lpf(f64 sig, FilterCoeffs* coeff,FilterInfo info) {
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
inline void differential(f64* dr, f64* di,const f64 r, const f64 i, f64* prev, const f64 sample_period) {
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
  // double s[4] = {0,0,0,*sum};

  f64 prev = *prev_sig;
  f64 current_sum = *sum;
  for (usize i = 0; i < buf_len; i+=4) {
    // s[0] = current_sum + prev + input_signal[i];
    // s[1] = s[0] + input_signal[i]   + input_signal[i+1];
    // s[2] = s[1] + input_signal[i+1] + input_signal[i+2];
    // s[3] = s[2] + input_signal[i+2] +input_signal[i+3];
    // TODO: この計算は間違っている。(そもそも正しい方法ではない)
    
    f64x4 in = _mm256_load_pd(input_signal+i);
    f64x4 sums = _mm256_add_pd(_mm256_set1_pd(current_sum + prev),in);
    in = _mm256_permute4x64_pd(in,_MM_SHUFFLE(2,1,0,3));
    sums = _mm256_fmadd_pd(in,_mm256_set_pd(2,2,2,0), sums);
    in = _mm256_permute4x64_pd(in,_MM_SHUFFLE(2,1,0,3));
    sums = _mm256_fmadd_pd(in,_mm256_set_pd(2,2,0,0), sums);
    in = _mm256_permute4x64_pd(in,_MM_SHUFFLE(2,1,0,3));
    sums = _mm256_fmadd_pd(in,_mm256_set_pd(2,0,0,0), sums);
    prev = input_signal[i+3];
    current_sum = sums.m256d_f64[3];
    // Note:　ここまでの処理は間違い。
    // f64x4 sums = _mm256_load_pd(s);
    f64x4 integral = _mm256_fmadd_pd(coeff,sums, angle);
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
void convert_intermediate_freq(
  f64 output_signal[], const f64 input_signal[],
  const f64 sample_period,
  f64 const fc, f64 const fi,
  CnvFiInfos* const info, const usize buf_len) {
    const f64 f = fc - fi;
    const f64 half_sample_period = sample_period/2.;
    f64 angle = info->angle;
    f64 prev = info->prev_sig;
    bool is_accept = true;
    for (size_t i = 0, j = 0; i < buf_len; ++i) {
      // 2倍サンプリング + 中間周波数へ落とす 
      f64 s1 = 2. * prev * sin(angle);
      f64 s2 = 2. * ((prev + input_signal[i]) / 2.) * sin(angle + TAU * f * half_sample_period);
      prev = input_signal[i];
      s1 = lpf(
        lpf(s1,&info->filter_coeff,info->filter_info[0]),
        &info->filter_coeff,info->filter_info[1]
      );
      s2 = lpf(
        lpf(s1,&info->filter_coeff,info->filter_info[0]),
        &info->filter_coeff,info->filter_info[1]
      );
      if(is_accept) {
        output_signal[j] = s1;
        ++j;
      }
      is_accept ^= true;
      angle += TAU * f * sample_period;
    }
    info->prev_sig = prev;
    info->angle = fmod(angle,TAU);
}
void fm_demodulate(f64 output_signal[], const f64 input_signal[], const f64 sample_period,f64 const fc,DemodulationInfo* const info, const usize buf_len) {
  FilterCoeffs* const coeff = &info->filter_coeff;
  FilterInfo* filter_info = info->filter_info;
  // printf("buffer len: %ld\n", buf_len);
  f64 prev_sin = info->prev_sin;
  f64 angle = info->angle;
  f64 prev_a = info->prev_internal[0];
  f64 prev_b = info->prev_internal[1];
  for (usize i = 0; i < buf_len; i++) {
    f64 sin_val = sin(angle);
    f64 current_a = lpf(-input_signal[i] * sin_val, coeff,filter_info[0]);
    f64 current_b = lpf(input_signal[i] * ((sin_val - prev_sin) / (TAU*fc*sample_period)), coeff,filter_info[2]);
    const f64 re = lpf(
      prev_a,coeff, filter_info[1]
    );
    const f64 im = lpf(
      prev_b,coeff, filter_info[3]
    );
    f64 d_re,d_im;
    differential(&d_re,&d_im,re,im,info->prev_sig,sample_period);
    f64 a = d_re * im;
    f64 b = d_im * re;
    output_signal[i] = a - b;
    prev_sin =sin_val ;
    angle += TAU * fc * sample_period;
    prev_a = current_a;
    prev_b = current_b;
  }
  info->angle = fmod(angle,TAU);
  info->prev_sin = prev_sin;
  info->prev_internal[0] = prev_a;
  info->prev_internal[1] = prev_b;
}

void upsample(f64* dst, f64* input, ResamplerInfo* info) {
  usize len = info->input_len;
  f64 prev = info->prev;
  usize multiplier = info->multiplier;
  f64x4 offset = _mm256_set1_pd(4);
  f64x4 m = _mm256_set1_pd(multiplier);
  // printf("len: %ld / multiplier: %ld\n", len,multiplier);
  for (int i=0; i<len; ++i) {
    f64x4 a = _mm256_set1_pd(prev);
    f64x4 b = _mm256_set1_pd(input[i]);
    f64x4 n = _mm256_set_pd(3,2,1,0);
    f64* d = dst + i*multiplier;
    prev = input[i];
    // upsample
    for (usize j = 0; j < multiplier; j+=4)
    {
      f64x4 coeff1 = _mm256_div_pd(n,m);
      f64x4 coeff2  = _mm256_sub_pd(_mm256_set1_pd(1),coeff1);
      f64x4 t = _mm256_fmadd_pd(a,coeff1,_mm256_mul_pd(b,coeff1));
      n = _mm256_add_pd(n,offset);
      _mm256_store_pd(d+j,t);
    }
  }
  info->prev = prev;
}

void downsample(f64* dst, f64* input, ResamplerInfo* info) {
  usize len = info->input_len;
  usize multiplier = info->multiplier;
  // printf("len: %ld / multiplier: %ld\n", len,multiplier);
  for(int i = 0,j = 0; i < len; i+=multiplier,++j) {
    dst[j] = input[i];
  }
  // printf("end down sample\n");
}