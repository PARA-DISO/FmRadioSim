#include "./freq_modulation.h"
#include <math.h>
#include <stdio.h>
#define ROTATE_RIGHT _MM_SHUFFLE(2,1,0,3)
#define ROTATE_LEFT _MM_SHUFFLE(0,3,2,1)
#define  _mm256_ror_pd(a) _mm256_permute4x64_pd(a, _MM_SHUFFLE(2,1,0,3))
#define  _mm256_rol_pd(a) _mm256_permute4x64_pd(a, _MM_SHUFFLE(0,3,2,1))
// DEBUG CODE
void _mmm256_print_pd(f64x4 v) {
  printf("[%f, %f, %f, %f]\n", v.m256d_f64[0], v.m256d_f64[1], v.m256d_f64[2], v.m256d_f64[3]);
}


inline f64 fast_lpf(f64 sig, f64 coeff, f64* prev) {
  f64 s = *prev + coeff * (sig - *prev);
  *prev = s;
  return s;
}
inline f64x4 fast_lpfx4(f64x4 sig, f64x4 coeff, f64x4* prev) {
 f64x4 s = _mm256_sub_pd(sig,*prev);
 s = _mm256_fmadd_pd(s,coeff,*prev);
  *prev = s;
  return s;
}
inline f64x4 _mm256_lpf_pd(f64x4 sig, f64x4 prev_sig,f64x4 prev_out, f64x4 coeff_a, f64x4 coeff_b) {
  // convert correct signal
  f64x4 sig_tmp_a = _mm256_ror_pd(_mm256_blend_pd(sig, prev_sig,0b1000));
  f64x4 sig_tmp_b = _mm256_permute2f128_pd (sig, prev_sig,0x03);
  f64x4 sig_tmp_c = _mm256_rol_pd(_mm256_blend_pd(prev_sig,sig,0b0001));
  // inner product
  f64x4 a3 = _mm256_mul_pd(sig,coeff_a);      // Target for S3
  f64x4 a2 = _mm256_mul_pd(sig_tmp_a,coeff_b);// Target for S2
  f64x4 a1 = _mm256_mul_pd(sig_tmp_b,coeff_b);// Target for S1
  f64x4 a0 = _mm256_mul_pd(sig_tmp_c,coeff_b);// Target for S0
  f64x4 a3_2 = _mm256_hadd_pd(a3,a2);
  f64x4 a3_1 = _mm256_hadd_pd(a1,a0);
  f64x4 a_lo = _mm256_permute2f128_pd(a3_2,a3_1,0x20);
  f64x4 a_hi = _mm256_permute2f128_pd(a3_2,a3_1,0x31);
  f64x4 a = _mm256_add_pd(a_lo,a_hi);
  // filter process
  return _mm256_fmadd_pd(coeff_b,prev_out, a);
}
// inline f64 fast_3xlpf(f64 sig, f64 coeff, f64* prev) {
//   f64 s1 = coeff * (sig - prev[0]) + prev[0];
//   f64 s2 = coeff * (s1  - prev[1]) + prev[1];
//   f64 s3 = coeff * (s2  - prev[2]) + prev[2];
//   prev[0] = s1;
//   prev[1] = s2;
//   prev[2] = s3;
//   return s3;
// }
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
    // 積分
    f64x4 in = _mm256_load_pd(input_signal+i);
    f64x4 sums = _mm256_add_pd(_mm256_set1_pd(current_sum + prev),in);
    in = _mm256_ror_pd(in);
    sums = _mm256_fmadd_pd(in,_mm256_set_pd(2,2,2,0), sums);
    in = _mm256_ror_pd(in);
    sums = _mm256_fmadd_pd(in,_mm256_set_pd(2,2,0,0), sums);
    in = _mm256_ror_pd(in);
    sums = _mm256_fmadd_pd(in,_mm256_set_pd(2,0,0,0), sums);
    prev = input_signal[i+3];
    // current_sum = sums.m256d_f64[3];
    current_sum = _mm256_cvtsd_f64(_mm256_permute_pd(sums, _MM_SHUFFLE(3,1,2,3)));
    f64x4 integral = _mm256_fmadd_pd(coeff,sums, angle);
    // 変調
    f64x4 sigs = _mm256_cos_pd(integral);
    _mm256_store_pd(output_signal + i, sigs);
    angle = _mm256_add_pd(angle, phi);
  }
  _mm256_store_pd(_angle,_mm256_fmod_pd(angle,_mm256_set1_pd(TAU)));
  *sum = current_sum;
  *prev_sig = prev;
}
#define FIRST_POS 1
void convert_intermediate_freq(
  f64 output_signal[], const f64 input_signal[],
  const f64 sample_period,
  f64 const fc, f64 const fi,
  CnvFiInfos* const info, const usize buf_len) {
    const f64 f = fc - fi;
    printf("f: %f Hz\n",f);
    const f64 half_sample_period = sample_period/2.;
    // f64 angle = info->angle;
    f64x4 prev = _mm256_load_pd(info->prev_sig);
    // stage: stage num - sig num
    // f64x4 stage1 = _mm256_load_pd(info->stage);
    // f64x4 stage2 = _mm256_load_pd(&info->stage[4]);
    const f64x4 coeff  = _mm256_set1_pd(info->filter_coeff);
    // f64x4 filter_info1 = _mm256_load_pd(info->filter_info);
    // f64x4 filter_info2 = _mm256_load_pd(info->filter_info + 4);
    f64* filter_info = info->filter_info;
    f64 t0 = info->stage[0];
    f64 t1 = info->stage[1];
    f64x4 prev_cos = _mm256_load_pd(info->prev_cos);
    f64x4 angle = _mm256_load_pd(info->angle);
    f64x4 full_delta_angle = _mm256_set1_pd(info->delta_angle*4);
    f64x4 half_angle = _mm256_set1_pd(info->delta_angle/2);
    // f64x4 delta_angle = _mm256_set1_pd(info->delta_angle);/
    for (usize i = 0, j = 0; i < buf_len; i+=4) {
      // 2倍サンプリング + 中間周波数へ落とす 
      f64x4 sig = _mm256_load_pd(input_signal+i);
      prev = _mm256_blend_pd(prev, sig, 0b0111);
      prev = _mm256_ror_pd(prev);
      f64x4 h_angle = _mm256_add_pd(angle,half_angle);
      f64x4 cos_b = _mm256_cos_pd(h_angle);
      prev_cos = _mm256_blend_pd(prev_cos, cos_b, 0b0111);
      prev_cos = _mm256_ror_pd(prev_cos);
      f64x4 cos_a = _mm256_add_pd(prev_cos,cos_b);
     
      angle = _mm256_add_pd(angle,full_delta_angle);
      f64x4 sig_a = _mm256_mul_pd(prev, cos_a);
      f64x4 sig_b = _mm256_mul_pd((_mm256_add_pd(prev, sig)),cos_b);
      prev = sig;
      prev_cos = cos_b;
      // 高周波成分の除去
      for (int j = 0; j < 4; j++)
      {
        // stage1 = _mm256_blend_pd(stage1,sig_a,FIRST_POS);
        // stage2 = _mm256_blend_pd(stage2,sig_b,FIRST_POS);
        // // LPF
        // // filter infoが怪しい(共通ではないのでは?)
        // f64x4 t1 = _mm256_sub_pd(stage1,filter_info1);
        // f64x4 filter_info1 = _mm256_fmadd_pd(t1,coeff,filter_info1);
        // f64x4 t2 = _mm256_sub_pd(stage2,filter_info1);
        // t2 = _mm256_fmadd_pd(t2,coeff,filter_info1);
        // // rotate
        // stage1 = _mm256_permute4x64_pd(filter_info1, ROTATE_LEFT);
        // stage2 = _mm256_permute4x64_pd(t2, ROTATE_LEFT);
        // sig_a = _mm256_permute4x64_pd(sig_a, ROTATE_RIGHT);
        // sig_b = _mm256_permute4x64_pd(sig_b, ROTATE_RIGHT);
        // filter_info1 = t2;

        f64 s1 = _mm256_cvtsd_f64(sig_a);
        f64 s2 = _mm256_cvtsd_f64(sig_b);
        t0 = fast_lpf(fast_lpf(s1,info->filter_coeff,&filter_info[0]),info->filter_coeff,&filter_info[1]);
        t1 = fast_lpf(fast_lpf(s2,info->filter_coeff,&filter_info[0]),info->filter_coeff,&filter_info[1]);
        sig_a = _mm256_permute4x64_pd(sig_a, ROTATE_LEFT);
        sig_b = _mm256_permute4x64_pd(sig_b, ROTATE_LEFT);
      }
      // ダウンサンプリング
      // output_signal[i >> 2] = 2 * _mm256_cvtsd_f64(stage1) ;
      output_signal[i >> 2] = 4 * t0;
      // output_signal[i >> 2] =  _mm256_cvtsd_f64(sig_a);
    }
    _mm256_store_pd(info->prev_sig, prev);
    _mm256_store_pd(info->angle,_mm256_fmod_pd(angle,_mm256_set1_pd(TAU)));
    _mm256_store_pd(info->prev_cos,prev_cos);
    info->stage[0] = t0;
    info->stage[1] = t1;
    // _mm256_store_pd(info->stage,stage1);
    // _mm256_store_pd(&info->stage[4],stage2);
    // _mm256_store_pd(info->filter_info,filter_info1);
    // _mm256_store_pd(info->filter_info +4 ,filter_info2);
}
void fm_demodulate(f64 output_signal[], const f64 input_signal[], const f64 sample_period,f64 const fc,DemodulationInfo* const info, const usize buf_len) {
  // FilterCoeffs* const coeff = &info->filter_coeff;
  // FilterInfo* filter_info = info->filter_info;

  // printf("buffer len: %ld\n", buf_len);
  // f64 prev_sin = info->prev_sin;
  // f64 angle = info->angle;
  // f64 prev_a = info->prev_internal[0];
  // f64 prev_b = info->prev_internal[1];
  // for (usize i = 0; i < buf_len; i++) {
  //   const f64 sin_val = sin(angle);
  //   const f64 cos_val = ((sin_val - prev_sin) / (TAU*fc*sample_period));
  //   const f64 current_a = lpf(-2 * input_signal[i] * sin_val, coeff,filter_info[0]);
  //   const f64 current_b = lpf(2 * input_signal[i] * cos_val, coeff,filter_info[2]);
  //   const f64 re = lpf(
  //     prev_a,coeff, filter_info[1]
  //   );
  //   const f64 im = lpf(
  //     prev_b,coeff, filter_info[3]
  //   );
  //   f64 d_re,d_im;
  //   differential(&d_re,&d_im,re,im,info->prev_sig,sample_period);
  //   f64 a = d_re * im;
  //   f64 b = d_im * re;
  //   output_signal[i] = a - b;
  //   // output_signal[i] = re;
  //   prev_sin =sin_val ;
  //   angle += TAU * fc * sample_period;
  //   prev_a = current_a;
  //   prev_b = current_b;
  // }
  // info->angle = fmod(angle,TAU);
  // info->prev_sin = prev_sin;
  // info->prev_internal[0] = prev_a;
  // info->prev_internal[1] = prev_b;
  f64x4 delta_angle = _mm256_set1_pd(TAU * fc * sample_period * 4);
  _mmm256_print_pd(delta_angle);
  
  f64x4 angle = _mm256_load_pd(info->angle);
  _mmm256_print_pd(angle);
  f64x4 prev_sin = _mm256_load_pd(info->prev_sin);
  f64x4 differential_coeff = _mm256_set1_pd(1 / (TAU*fc*sample_period));
  f64x4 prev_sig_lo = _mm256_load_pd(info->prev_sig);
  f64x4 prev_sig_hi = _mm256_load_pd(info->prev_sig + 4);
  f64x4 prev_sig_internal_lo = _mm256_load_pd(info->prev_internal); // 0 0 2 2
  f64x4 prev_sig_internal_hi = _mm256_load_pd(info->prev_internal+4); // 1 1 3 3
  f64x4 filter_prev = _mm256_load_pd(info->filter_info);
  f64x4 filter_coeff = _mm256_set1_pd(info->filter_coeff);
  f64x4 d_coeff = _mm256_set1_pd(1/sample_period);
  printf("demodulate start\n");
  for (usize i = 0; i < buf_len; i+=4) {
    // Removing Carrier
    f64x4 sin_val = _mm256_sin_pd(angle);
    prev_sin = _mm256_blend_pd(sin_val,prev_sin,0b1000);
    prev_sin = _mm256_ror_pd(prev_sin);
    f64x4 cos_val = _mm256_mul_pd(_mm256_sub_pd(sin_val,prev_sin),differential_coeff);
    f64x4 sig = _mm256_load_pd(input_signal+i);
    f64x4 sig1 = _mm256_mul_pd(_mm256_set1_pd(-1),_mm256_mul_pd(sig,sin_val));
    f64x4 sig2 = _mm256_mul_pd(_mm256_set1_pd(1),_mm256_mul_pd(sig,cos_val));
    angle = _mm256_add_pd(angle,delta_angle);
    angle = _mm256_fmod_pd(angle,_mm256_set1_pd(TAU));
    f64x4 prev_sin_tmp = prev_sin;
    prev_sin = sin_val;
    // Signal Interleaving
    // [s1 s2, s1, s2]
    f64x4 sig_lo = _mm256_unpacklo_pd(sig1,sig2); // 0 0 2 2
    f64x4 sig_hi = _mm256_unpackhi_pd(sig1,sig2); // 1 1 3 3
    // [s1, s2, p1, p2]
    f64x4 s0 = _mm256_permute2f128_pd(sig_lo,prev_sig_lo,0x20);
    f64x4 s2 = _mm256_permute2f128_pd(sig_lo,prev_sig_lo,0x31);
    f64x4 s1 = _mm256_permute2f128_pd(sig_hi,prev_sig_hi,0x20);
    f64x4 s3 = _mm256_permute2f128_pd(sig_hi,prev_sig_hi,0x31);
    // LPF Process
    // y[i] = c*(x[i] -y[i-1]) + y[i-1]
    f64x4 t0 = _mm256_sub_pd(s0,filter_prev);
    f64x4 o0 = _mm256_fmadd_pd(t0,filter_coeff,filter_prev);
    f64x4 t1 = _mm256_sub_pd(s1,o0);
    f64x4 o1 = _mm256_fmadd_pd(t1,filter_coeff,o0);
    f64x4 t2 = _mm256_sub_pd(s2,o1);
    f64x4 o2 = _mm256_fmadd_pd(t2,filter_coeff,o1);
    f64x4 t3 = _mm256_sub_pd(s3,o2);
    filter_prev = _mm256_fmadd_pd(t3,filter_coeff,o2);
    // DeInterleaving
    f64x4 s_lo = _mm256_permute2f128_pd(o0,o2,0x31); // 0 0 2 2
    f64x4 s_hi =  _mm256_permute2f128_pd(o1,filter_prev,0x31); // 1 1 3 3
    // TEST AFTER LPF SIGNAL
    f64x4 test_point1 = _mm256_unpacklo_pd(s_lo,s_hi);
    f64x4 test_point2 = _mm256_unpackhi_pd(s_lo,s_hi);
    // differential
    // TODO: アルゴリズムが異なる。
    // x[i] - x[i-1]であるべきが、x[i] -x[i-4]となっている。
    // 以下のコメントアウトコードは正しくはない=>単純な時系列データではないためこの方法は使えない。
    // prev_sig_internal_lo = _mm256_blend_pd(prev_sig_internal_lo,sig_lo,0b0001);
    // prev_sig_internal_lo = _mm256_rol_pd(prev_sig_internal_lo);
    // prev_sig_internal_hi = _mm256_blend_pd(prev_sig_internal_hi,sig_hi,0b0001);
    // prev_sig_internal_hi = _mm256_rol_pd(prev_sig_internal_hi);
    prev_sig_internal_lo = _mm256_blend_pd(s_hi,prev_sig_internal_hi,0b1100);
    prev_sig_internal_lo = _mm256_permute2f128_pd(prev_sig_internal_lo,prev_sig_internal_lo,0x01);
    f64x4 dsig_l = _mm256_mul_pd(_mm256_sub_pd(s_lo,prev_sig_internal_lo),d_coeff); // 0 0 2 2
    f64x4 dsig_h = _mm256_mul_pd(_mm256_sub_pd(s_hi,s_lo),d_coeff); // 1 1 3 3 
    f64x4 test_point3 = _mm256_unpacklo_pd(dsig_l,dsig_h);
    f64x4 test_point4 = _mm256_unpackhi_pd(dsig_l,dsig_h);
    // _mm256_store_pd(output_signal+i,dsig_l);
    // たすき掛け 
    dsig_l = _mm256_permute_pd(dsig_l,0b0101);
    dsig_h = _mm256_permute_pd(dsig_h,0b0101);
    f64x4 ta = _mm256_mul_pd(dsig_l,s_lo);
    f64x4 tb = _mm256_mul_pd(dsig_h,s_hi);
    f64x4 sig_out =  _mm256_hsub_pd(ta,tb);
    // _mm256_store_pd(output_signal+i,test_point4);
    _mm256_store_pd(output_signal+i,sig_out);
    // _mmm256_print_pd(prev_sin_tmp);
    // move value for next loop
    prev_sig_lo = _mm256_permute2f128_pd(o0,o2,0x20);
    prev_sig_hi = _mm256_permute2f128_pd(o1,filter_prev,0x20);
    prev_sig_internal_lo = s_lo;
    prev_sig_internal_hi = s_hi;
  }
  _mm256_store_pd(info->angle,_mm256_fmod_pd(angle,_mm256_set1_pd(TAU)));
  _mm256_store_pd(info->prev_sin,prev_sin);
  _mm256_store_pd(info->prev_sig,prev_sig_lo);
  _mm256_store_pd(info->prev_sig+4,prev_sig_hi);
  _mm256_store_pd(info->prev_internal,prev_sig_internal_lo);
  _mm256_store_pd(info->prev_internal+4,prev_sig_internal_hi);
  _mm256_store_pd(info->filter_info,filter_prev);
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