#include <immintrin.h>
#include "rstype.h"
#define EXTRA_PRECISION

f64x4 __vectorcall _mm256_fcos_pd(f64x4 angle) {
  // define abs function
  #define _mm256_abs_pd(x) _mm256_andnot_pd(sign_bit, x)

  // set consts
  const f64x4 sign_bit = _mm256_set1_pd(-0.0);
  const f64x4 tp = _mm256_set1_pd(1./(TAU));
  const f64x4 imm_0_25 = _mm256_set1_pd(0.25);
  const f64x4 imm_16 = _mm256_set1_pd(16.);
  const f64x4 imm_0_5 = _mm256_set1_pd(0.5);
  const f64x4 imm_0_225 = _mm256_set1_pd(0.225);
  // calculate cos
  f64x4 x0 = _mm256_mul_pd(tp,angle);
  f64x4 tmp1 = _mm256_floor_pd(_mm256_add_pd(x0,imm_0_25));
  f64x4 x2 = _mm256_sub_pd(x0,_mm256_add_pd(imm_0_25,tmp1));
  f64x4 tmp2 = _mm256_mul_pd(imm_16,_mm256_sub_pd(_mm256_abs_pd(x2), imm_0_5));
  f64x4 x3 = _mm256_mul_pd(x2,tmp2);
  #ifdef EXTRA_PRECISION
  f64x4 tmp3 = _mm256_mul_pd(imm_0_225,x3);
  f64x4 tmp4 = _mm256_abs_pd(x3);
  f64x4 x4 = _mm256_fmsub_pd(tmp3,tmp4,tmp3);
  return _mm256_add_pd(x3,x4);
  #else
  return x3;
  #endif
  
};