#include "resampler.h"
#include "immintrin.h"
void upsample(f64* dst, f64* input, ResamplerInfo* info) {
  usize len = info->input_len;
  f64 prev = info->prev;
  usize multiplier = info->multiplier;
  
  f64x4 offset = _mm256_set1_pd(4);
  f64x4 m = _mm256_set1_pd(multiplier);
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
  for(int i = 0,j = 0; i < len; i+=multiplier,++j) {
    dst[j] = input[i];
  }
}

