#include "./freq_modulation.h"
#include <math.h>
#include <stdio.h>
#define ROTATE_RIGHT _MM_SHUFFLE(2, 1, 0, 3)
#define ROTATE_LEFT _MM_SHUFFLE(0, 3, 2, 1)
#define _mm256_ror_pd(a) _mm256_permute4x64_pd(a, _MM_SHUFFLE(2, 1, 0, 3))
#define _mm256_rol_pd(a) _mm256_permute4x64_pd(a, _MM_SHUFFLE(0, 3, 2, 1))
// DEBUG CODE
#define ptr_print(in,out) printf("C-Func::%s: read @ 0x%p / write @ 0x%p\n",__func__,in,out);fflush(stdout);
void print_sd(f64 x) { printf("%g\n", x); }
void _mm256_print_pd(f64x4 v) {
  printf("[%g, %g, %g, %g]\n", v.m256d_f64[0], v.m256d_f64[1], v.m256d_f64[2],
         v.m256d_f64[3]);
}
void _mm256_fprint_pd(FILE* fd, f64x4 v) {
  fprintf(fd,"%f\n%f\n%f\n%f\n", v.m256d_f64[0], v.m256d_f64[1], v.m256d_f64[2],
         v.m256d_f64[3]);
}
void _mm_print_pd(f64x2 v) {
  printf("[%f, %f]\n", v.m128d_f64[0], v.m128d_f64[1]);
}
//===========================
void print_filter_info(FilteringInfo* info) {
  fflush(stdout);
  printf(
    "BPF-Info {\n"
    "  prev-sig: [%g, %g]\n"
    "  prev_prev_sig: [%g, %g]\n"
    "  prev_out: [%g, %g]\n"
    "  prev_prev_out: [%g, %g]\n"
    "  stage_lo: [%g, %g]\n"
    "  stage_hi: [%g, %g]\n"
    "}\n",info->prev_sig[0],info->prev_sig[1],
    info->prev_prev_sig[0],info->prev_prev_sig[1],
    info->prev_out[0],info->prev_out[1],
    info->prev_prev_out[0],info->prev_prev_out[1],
    info->stage[0],info->stage[1],
    info->stage[2], info->stage[3]);
  fflush(stdout);
}
void print_demodulate_info(DemodulationInfo* info) {
  fflush(stdout);
  printf(
    "DemodulationInfo{\n"
    "  angle: [%g, %g, %g, %g]\n"
    "  prev_sin: [%g, %g, %g, %g]\n"
    "  prev_sig_lo: [%g, %g, %g, %g]\n"
    "  prev_sig_hi: [%g, %g, %g, %g]\n"
    "  prev_sig_internal_lo: [%g, %g, %g, %g]\n"
    "  prev_sig_internal_hi: [%g, %g, %g, %g]\n"
    "  prev_sig: [%g, %g, %g, %g]\n"
    "  prev_prev_sig: [%g, %g, %g, %g]\n"
    "  prev_out: [%g, %g, %g, %g]\n"
    "  prev_prev_out: [%g, %g, %g, %g]\n"
    "}\n",
    info->angle[0],info->angle[1],info->angle[2],info->angle[3],
    info->prev_sin[0],info->prev_sin[1],info->prev_sin[2],info->prev_sin[3],
    info->prev_sig[0],info->prev_sig[1],info->prev_sig[2],info->prev_sig[3],
    info->prev_sig[4],info->prev_sig[5],info->prev_sig[6],info->prev_sig[7],
    info->prev_internal[0],info->prev_internal[1],info->prev_internal[2],info->prev_internal[3],
    info->prev_internal[4],info->prev_internal[5],info->prev_internal[6],info->prev_internal[7],
    info->filter_info[0],info->filter_info[1],info->filter_info[2],info->filter_info[3],
    info->filter_info[4],info->filter_info[5],info->filter_info[6],info->filter_info[7],
    info->filter_info[8],info->filter_info[9],info->filter_info[10],info->filter_info[11],
    info->filter_info[12],info->filter_info[13],info->filter_info[14],info->filter_info[15]
  );
  fflush(stdout);
}
void print_resampler_info(ResamplerInfo* info) {
  printf(
    "ResamplerInfo {\n"
    "  prev: %g\n"
    "  multiplier: %lld\n"
    "  input_len: %lld\n"
    "}\n",
    info->prev,
    info->multiplier,
    info->input_len
  );
}
//----------------------------
static const unsigned int crc32tab[256] = { 
	0x00000000, 0x77073096, 0xee0e612c, 0x990951ba,
	0x076dc419, 0x706af48f, 0xe963a535, 0x9e6495a3,
	0x0edb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988,
	0x09b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91,
	0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de,
	0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
	0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec,
	0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5,
	0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172,
	0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
	0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940,
	0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
	0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116,
	0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
	0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924,
	0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d,
	0x76dc4190, 0x01db7106, 0x98d220bc, 0xefd5102a,
	0x71b18589, 0x06b6b51f, 0x9fbfe4a5, 0xe8b8d433,
	0x7807c9a2, 0x0f00f934, 0x9609a88e, 0xe10e9818,
	0x7f6a0dbb, 0x086d3d2d, 0x91646c97, 0xe6635c01,
	0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e,
	0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457,
	0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea, 0xfcb9887c,
	0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
	0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2,
	0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb,
	0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0,
	0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
	0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086,
	0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
	0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4,
	0x59b33d17, 0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad,
	0xedb88320, 0x9abfb3b6, 0x03b6e20c, 0x74b1d29a,
	0xead54739, 0x9dd277af, 0x04db2615, 0x73dc1683,
	0xe3630b12, 0x94643b84, 0x0d6d6a3e, 0x7a6a5aa8,
	0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1,
	0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe,
	0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7,
	0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc,
	0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
	0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252,
	0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
	0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60,
	0xdf60efc3, 0xa867df55, 0x316e8eef, 0x4669be79,
	0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236,
	0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f,
	0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04,
	0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
	0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x026d930a,
	0x9c0906a9, 0xeb0e363f, 0x72076785, 0x05005713,
	0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38,
	0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21,
	0x86d3d2d4, 0xf1d4e242, 0x68ddb3f8, 0x1fda836e,
	0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
	0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c,
	0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
	0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2,
	0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db,
	0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0,
	0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
	0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6,
	0xbad03605, 0xcdd70693, 0x54de5729, 0x23d967bf,
	0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94,
	0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d,
};


unsigned int crc32(char *p, int len)
{
	unsigned int crcinit = 0;
	unsigned int crc = 0;

	crc = crcinit ^ 0xFFFFFFFF;
	for (; len--; p++) {
		crc = ((crc >> 8) & 0x00FFFFFF) ^ crc32tab[(crc ^ (*p)) & 0xFF];
	}
	return crc ^ 0xFFFFFFFF;
}
//----------------------------
inline f64 fast_lpf(f64 sig, f64 coeff, f64 *prev) {
  f64 s = *prev + coeff * (sig - *prev);
  *prev = s;
  return s;
}

// (結果が完全に0になる原因)
inline f64 lpf(f64 sig, FilterCoeffs *coeff, FilterInfo info) {
  const f64 in1 = info[0];
  const f64 in2 = info[1];
  const f64 out1 = info[2];
  const f64 out2 = info[3];
  const f64 buf = coeff->c0 * sig + coeff->c1 * in1 + coeff->c2 * in2 -
                  coeff->c3 * out1 - coeff->c4 * out2;
  info[0] = sig;
  info[1] = in1;
  info[2] = buf;
  info[3] = out1;
  return buf;
}
inline void differential(f64 *dr, f64 *di, const f64 r, const f64 i, f64 *prev,
                         const f64 sample_period) {
  *dr = (r - prev[0]) / sample_period;
  *di = (i - prev[1]) / sample_period;
  prev[0] = r;
  prev[1] = i;
}

void fm_modulate(f64 output_signal[], const f64 input_signal[],
                 const usize buf_len, ModulationInfo *info) {
#if TEST_CODE
  f64x4 angle = _mm256_load_pd(info->t);
  f64x4 phi =
      _mm256_set1_pd(TAU * info->carrier_freq * info->sample_period * 4.);
  f64x4 coeff =
      _mm256_set1_pd(info->modulation_index * info->sample_period / 2.);

  f64 prev = info->prev_sig[0];
  f64 current_sum = info->integral[0];
  for (usize i = 0; i < buf_len; i += 4) {
    // 積分
    f64x4 in = _mm256_load_pd(input_signal + i);
    f64x4 sums = _mm256_add_pd(_mm256_set1_pd(current_sum + prev), in);
    in = _mm256_ror_pd(in);
    sums = _mm256_fmadd_pd(in, _mm256_set_pd(2, 2, 2, 0), sums);
    in = _mm256_ror_pd(in);
    sums = _mm256_fmadd_pd(in, _mm256_set_pd(2, 2, 0, 0), sums);
    in = _mm256_ror_pd(in);
    sums = _mm256_fmadd_pd(in, _mm256_set_pd(2, 0, 0, 0), sums);
    prev = input_signal[i + 3];
    // current_sum = sums.m256d_f64[3];
    current_sum =
        _mm256_cvtsd_f64(_mm256_permute_pd(sums, _MM_SHUFFLE(3, 1, 2, 3)));
    f64x4 integral = _mm256_fmadd_pd(coeff, sums, angle);
    // 変調
    f64x4 sigs = _mm256_cos_pd(integral);
    _mm256_store_pd(output_signal + i, sigs);
    angle = _mm256_add_pd(angle, phi);
  }
  _mm256_store_pd(info->t, _mm256_fmod_pd(angle, _mm256_set1_pd(TAU)));
  info->integral[0] = current_sum;
  info->prev_sig[0] = prev;
#else
  // ptr_print(input_signal,output_signal);
  f64x4 angle = _mm256_load_pd(info->t);
  f64x4 phi =
      _mm256_set1_pd(TAU * info->carrier_freq * info->sample_period * 4.);
  f64x4 coeff = _mm256_set1_pd(info->modulation_index * info->sample_period);
  
  f64x2 prev_sum = _mm_load_pd(info->integral);
  f64x4 s1_sums_prev = _mm256_load_pd(info->prev_sig);
  f64x4 s2_sums_prev = _mm256_load_pd(info->prev_sig+4);
  //
  // FILE* test_log = fopen("test_log.txt", "w");
  #ifdef INTERPOLATION
  f64x4 c1 = _mm256_set_pd(1, 0.75, 0.5, 0.25);
  f64x4 c2 = _mm256_set_pd(0, 0.25, 0.5, 0.75);
  f64x4 prev_inter_sig = _mm256_load_pd(info->prev_inter_sig);
  #endif
  // _mm256_print_pd(coeff);
  for (usize i = 0; i < buf_len; i += 8) {
    // integral
     f64x4 s1 = _mm256_load_pd(input_signal + i);            // 0 1 2 3
  f64x4 s2 = _mm256_load_pd(input_signal + i + 4);        // 4 5 6 7
  f64x4 slo = _mm256_shuffle_pd(s1, s2, 0b0000);          // 0 4 2 6
  f64x4 shi = _mm256_shuffle_pd(s1, s2, 0b1111);          // 1 5 3 7
  f64x4 tmp_sig1 = _mm256_add_pd(slo, shi);               // 0+1 4+5 2+3 6+7
  f64x4 s1_tmp = _mm256_shuffle_pd(tmp_sig1, s1, 0b0000); // 0+1 0 2+3 2
  f64x4 s2_tmp = _mm256_shuffle_pd(tmp_sig1, s2, 0b0101); // 4+5 4 6+7 6
  
  f64x2 s1_lo = _mm256_extractf128_pd(s1_tmp, 0);         // 0+1 0
  f64x2 s1_hi = _mm256_extractf128_pd(s1_tmp, 1);         // 2+3 2
  f64x2 s2_lo = _mm256_extractf128_pd(s2_tmp, 0);         // 4+5 4
  f64x2 s2_hi = _mm256_extractf128_pd(s2_tmp, 1);         // 6+7 6
  /*res*/ f64x2 s1_lo_sums = _mm_add_pd(s1_lo, prev_sum); // s+0+1 s+0
  f64x2 s1_lo_hi = _mm_broadcastsd_pd(s1_lo_sums);             // 0+1
  f64x2 s2_lo_hi = _mm_broadcastsd_pd(s2_lo);             // 4+5
  
  /*res*/ f64x2 s1_hi_sums = _mm_add_pd(s1_hi, s1_lo_hi); // s+0+1+2+3 s+0+2
  f64x2 s2_hi_sums_tmp = _mm_add_pd(s2_hi, s2_lo_hi);     // 6+7+5+4 6+5+4
  f64x2 s1_hi_hi = _mm_broadcastsd_pd(s1_hi_sums);
  /*res*/ f64x2 s2_lo_sums =
      _mm_add_pd(s2_lo, s1_hi_hi); // s+0+1+2+3+4+5 s+0+1+2+3+4
  /*res*/ f64x2 s2_hi_sums =
      _mm_add_pd(s2_hi_sums_tmp, s1_hi_hi); // 4+5+6+7 6+5+4
  
  f64x2 s2_hi_hi = _mm_broadcastsd_pd(s2_hi_sums);
  prev_sum = s2_hi_hi;
  
#ifdef INTERPOLATION
    // interpolation x4
    f64x2 s1_lo_lo = _mm_permute_pd(s1_lo_sums, 0b11);
    f64x2 s1_hi_lo = _mm_permute_pd(s1_hi_sums, 0b11);
    f64x2 s2_lo_lo = _mm_permute_pd(s2_lo_sums, 0b11);
    f64x2 s2_hi_lo = _mm_permute_pd(s2_hi_sums, 0b11);
    f64x4 s_a = _mm256_broadcastsd_pd(s1_lo_lo);
    f64x4 s_b = _mm256_broadcastsd_pd(s1_lo_hi);
    f64x4 s_c = _mm256_broadcastsd_pd(s1_hi_lo);
    f64x4 s_d = _mm256_broadcastsd_pd(s1_hi_hi);
    f64x4 s_e = _mm256_broadcastsd_pd(s2_lo_lo);
    f64x4 s_f = _mm256_broadcastsd_pd(s2_lo_hi);
    f64x4 s_g = _mm256_broadcastsd_pd(s2_hi_lo);
    f64x4 s_h = _mm256_broadcastsd_pd(s2_hi_hi);
    f64x4 inter_sig[8] = {0};
    inter_sig[0] = _mm256_fmadd_pd(c1, s_a, _mm256_mul_pd(prev_inter_sig, c2));
    inter_sig[1] = _mm256_fmadd_pd(c1, s_b, _mm256_mul_pd(s_a, c2));
    inter_sig[2] = _mm256_fmadd_pd(c1, s_c, _mm256_mul_pd(s_b, c2));
    inter_sig[3] = _mm256_fmadd_pd(c1, s_d, _mm256_mul_pd(s_c, c2));
    inter_sig[4] = _mm256_fmadd_pd(c1, s_e, _mm256_mul_pd(s_d, c2));
    inter_sig[5] = _mm256_fmadd_pd(c1, s_f, _mm256_mul_pd(s_e, c2));
    inter_sig[6] = _mm256_fmadd_pd(c1, s_g, _mm256_mul_pd(s_f, c2));
    inter_sig[7] = _mm256_fmadd_pd(c1, s_h, _mm256_mul_pd(s_g, c2));
    // <next loop data>
    prev_sum = s2_hi_hi;
    prev_inter_sig = s_h;
    // modulation
    f64 *pos = output_signal + (i << 2);
    for (usize xx = 0; xx < 8; xx++) {
      f64x4 modulated_angle1 = _mm256_fmadd_pd(coeff, inter_sig[xx], angle);
      f64x4 cos_value = _mm256_cos_pd(modulated_angle1);
      angle = _mm256_add_pd(angle, phi);
      _mm256_store_pd(pos + xx, cos_value);
    }
#else
    f64x2 s1_lo_sorted = _mm_permute_pd(s1_lo_sums, 0b01);
    f64x2 s1_hi_sorted = _mm_permute_pd(s1_hi_sums, 0b01);
    f64x2 s2_lo_sorted = _mm_permute_pd(s2_lo_sums, 0b01);
    f64x2 s2_hi_sorted = _mm_permute_pd(s2_hi_sums, 0b01);
    f64x4 s1_sums = _mm256_set_m128d(s1_hi_sorted, s1_lo_sorted);
    f64x4 s2_sums = _mm256_set_m128d(s2_hi_sorted, s2_lo_sorted);
    // modulation
    f64x4 angle2 = _mm256_add_pd(angle, phi);
    f64x4 angle_tmp = _mm256_add_pd(angle2, phi);
    f64x4 modulated_angle1 = _mm256_fmadd_pd(coeff, s1_sums_prev, angle);
    f64x4 modulated_angle2 = _mm256_fmadd_pd(coeff, s2_sums_prev, angle2);
    
    f64x4 cos_value1 = _mm256_cos_pd(modulated_angle1);
    f64x4 cos_value2 = _mm256_cos_pd(modulated_angle2);
    // f64x4 cos_value1 = _mm256_cos_pd(angle);
    // f64x4 cos_value2 = _mm256_cos_pd(angle2);
    // _mm256_fprint_pd(test_log,s1_sums);
    // _mm256_fprint_pd(test_log,s2_sums);
    _mm256_store_pd(output_signal + i, cos_value1);
    _mm256_store_pd(output_signal + i + 4, cos_value2);
    // <for next loop data>
    angle = angle_tmp;
    s1_sums_prev = s1_sums;
    s2_sums_prev = s2_sums;
#endif
  }
  // fclose(test_log);
  _mm256_store_pd(info->t, _mm256_fmod_pd(angle, _mm256_set1_pd(TAU)));
  _mm256_store_pd(info->prev_sig,s1_sums_prev);
  _mm256_store_pd(info->prev_sig +4,s2_sums_prev);
  _mm_store_pd(info->integral, prev_sum);
  #if INTERPOLATION
  _mm256_store_pd(info->prev_inter_sig, prev_inter_sig);
  #endif
#endif
}
#define FIRST_POS 1
void convert_intermediate_freq(f64 output_signal[], const f64 input_signal[],
                               const f64 sample_period, f64 const fc,
                               f64 const fi, CnvFiInfos *const info,
                               const usize buf_len) {

#if ENABLE_UPSAMPLING
  // f64x4 prev = _mm256_load_pd(info->prev_sig);
  // stage: stage num - sig num
  // const f64x4 coeff  = _mm256_set1_pd(info->filter_coeff);
  f64x4 prev_cos = _mm256_load_pd(info->prev_cos); // -0.5 0.5 1.5 2.5
  f64x4 next_cos = _mm256_load_pd(info->next_cos); // 3.5 4.5 5.5 6.5
  f64x4 angle = _mm256_load_pd(info->angle);
  f64x4 full_delta_angle = _mm256_set1_pd(info->delta_angle * 4);
  // Signals
  f64x2 before_sig_lo = _mm_load_pd(info->prev_sig); // -2 -1
  f64x2 stage2_a_lo = _mm_load_pd(info->stage);      // -0.5 0.5
  f64x2 stage2_a_hi = _mm_load_pd(info->stage + 2);  // 1.5 2.5
  f64x2 stage2_b_lo = _mm_load_pd(info->stage + 4);  // 0 1
  f64x2 stage2_b_hi = _mm_load_pd(info->stage + 6);  // 2 3
  f64x2 stage1_a_lo = _mm_load_pd(info->stage + 8);  // -0.5 0.5
  f64x2 stage1_b_lo = _mm_load_pd(info->stage + 10); // 1.5 2.5
  f64x2 stage1_a_hi = _mm_load_pd(info->stage + 12); // 0 1
  f64x2 stage1_b_hi = _mm_load_pd(info->stage + 14); // 2 3
  // LPF INFOS
  f64x2 prev_out = _mm_load_pd(info->filter_info + 8);
  // _mm_print_pd(prev_out);
  // _mm_print_pd(stage1_a_lo);
  // print_sd(info->filter_coeff);
  // filter coefficients
  f64x2 coeff_a = _mm_set1_pd(info->filter_coeff);
  f64x2 coeff_b = _mm_set1_pd(1 - info->filter_coeff);
  // _mm_print_pd(coeff_a);
  for (usize i = 0, j = 0; i < buf_len; i += 4) {
    f64x4 signal = _mm256_load_pd(input_signal + i); // 0 1 2 3
    // shift cosine value 1
    f64x2 prev_cos_lo = _mm256_extractf128_pd(prev_cos, 0);       // -0.5 0.5
    f64x2 prev_cos_hi = _mm256_extractf128_pd(prev_cos, 1);       // 1.5 2.5
    f64x2 next_cos_value_lo = _mm256_extractf128_pd(next_cos, 0); // 3.5 4.5
    // load signal
    f64x2 slo = _mm256_extractf128_pd(signal, 0); // 0 1
    f64x2 shi = _mm256_extractf128_pd(signal, 1); // 2 3
    // shift cosine value 2
    f64x2 current_cos_lo_hi =
        _mm_shuffle_pd(prev_cos_lo, prev_cos_hi, 0b01); // 0.5 1.5
    f64x2 current_cos_val_lo =
        _mm_add_pd(prev_cos_lo, current_cos_lo_hi); // 0 1
    f64x2 current_cos_hi_hi =
        _mm_shuffle_pd(prev_cos_hi, next_cos_value_lo, 0b01); // 2.5 3.5
    // generate intermediate signal1
    f64x2 sig_prev_lo_tmp = _mm_shuffle_pd(before_sig_lo, slo, 0b01); //-1 0
    f64x2 sig_prev_hi_tmp = _mm_shuffle_pd(slo, shi, 0b01);           // 1 2
    // generate intermediate cos
    f64x2 current_cos_val_hi =
        _mm_add_pd(prev_cos_hi, current_cos_hi_hi); // 2 3
    // generate intermediate signal2
    f64x2 prev_lo = _mm_add_pd(sig_prev_lo_tmp, slo); // -0.5 0.5
    f64x2 prev_hi = _mm_add_pd(sig_prev_hi_tmp, shi); // 1.5 2.5
    // shift carrier freq
    f64x2 sig_a_lo = _mm_mul_pd(prev_lo, prev_cos_lo);    // -0.5 0.5
    f64x2 sig_a_hi = _mm_mul_pd(prev_hi, prev_cos_hi);    // 1.5 2.5
    f64x2 sig_b_lo = _mm_mul_pd(slo, current_cos_val_lo); // 0 1
    f64x2 sig_b_hi = _mm_mul_pd(shi, current_cos_val_hi); // 2 3
    angle = _mm256_add_pd(angle, full_delta_angle);
    prev_cos = next_cos;
    before_sig_lo = shi;
    //// calculate 2 stage First IIR LPF with pipline process
    //// IIR LPF: y[i] = a(x[i]-y[i-1])+y[i-1]
    // interleaving1
    f64x2 x0 = _mm_unpacklo_pd(stage2_a_lo, stage1_a_lo); // -0.5
    f64x2 x1 = _mm_unpacklo_pd(stage2_b_lo, stage1_b_lo); // 0
    // interleaving2
    f64x2 x2 = _mm_unpackhi_pd(stage2_a_lo, stage1_a_lo); // 0.5
    f64x2 x3 = _mm_unpackhi_pd(stage2_b_lo, stage1_b_lo); // 1
    // interleaving3
    f64x2 x4 = _mm_unpacklo_pd(stage2_a_hi, stage1_a_hi); // 1.5
    f64x2 x5 = _mm_unpacklo_pd(stage2_b_hi, stage1_b_hi); // 2
    // interleaving4
    f64x2 x6 = _mm_unpackhi_pd(stage2_a_hi, stage1_a_hi); // 2.5
    f64x2 x7 = _mm_unpackhi_pd(stage2_b_hi, stage1_b_hi); // 3

    f64x2 o0 = _mm_fmadd_pd(coeff_b, prev_out, x0);
    f64x2 o1 = _mm_fmadd_pd(coeff_b, o0, x1);
    f64x2 o2 = _mm_fmadd_pd(coeff_b, o1, x2);
    f64x2 o3 = _mm_fmadd_pd(coeff_b, o2, x3);
    f64x2 o4 = _mm_fmadd_pd(coeff_b, o3, x4);
    f64x2 o5 = _mm_fmadd_pd(coeff_b, o4, x5);
    f64x2 o6 = _mm_fmadd_pd(coeff_b, o5, x6);
    f64x2 o7 = _mm_fmadd_pd(coeff_b, o6, x7);

    // set next stage parameters
    // prev_sig = x7;
    prev_out = o7;
    f64x2 stage2_a_lo_tmp = _mm_unpackhi_pd(o0, o2);
    f64x2 stage2_b_lo_tmp = _mm_unpackhi_pd(o1, o3);
    f64x2 stage2_a_hi_tmp = _mm_unpackhi_pd(o4, o6);
    f64x2 stage2_b_hi_tmp = _mm_unpackhi_pd(o5, o7);
    // f64x2 out_a_lo_tmp = _mm_unpackhi_pd(o0, o2);
    f64x2 out_b_lo_tmp = _mm_unpacklo_pd(o1, o3);
    // f64x2 out_a_hi_tmp = _mm_unpackhi_pd(o4, o6);
    f64x2 out_b_hi_tmp = _mm_unpacklo_pd(o5, o7);
    stage1_a_lo = _mm_mul_pd(sig_a_lo, coeff_a);
    stage1_b_lo = _mm_mul_pd(sig_a_hi, coeff_a);
    stage1_a_hi = _mm_mul_pd(sig_b_lo, coeff_a);
    stage1_b_hi = _mm_mul_pd(sig_b_hi, coeff_a);
    stage2_a_lo = _mm_mul_pd(stage2_a_lo_tmp, coeff_a);
    stage2_b_lo = _mm_mul_pd(stage2_b_lo_tmp, coeff_a);
    stage2_a_hi = _mm_mul_pd(stage2_a_hi_tmp, coeff_a);
    stage2_b_hi = _mm_mul_pd(stage2_b_hi_tmp, coeff_a);
    _mm_store_pd(output_signal + i, out_b_lo_tmp);
    _mm_store_pd(output_signal + i + 2, out_b_hi_tmp);

    // output_signal[i >> 2] = 4*_mm_cvtsd_f64(o0);
    // output_signal[i >> 2] = 1*_mm_cvtsd_f64(slo);
    next_cos = _mm256_cos_pd(angle);
  }
  _mm_store_pd(info->prev_sig, before_sig_lo);
  _mm256_store_pd(info->angle, _mm256_fmod_pd(angle, _mm256_set1_pd(TAU)));
  _mm256_store_pd(info->next_cos, next_cos);
  _mm256_store_pd(info->prev_cos, prev_cos);
  _mm_store_pd(info->stage, stage2_a_lo);
  _mm_store_pd(info->stage + 2, stage2_a_hi);
  _mm_store_pd(info->stage + 4, stage2_b_lo);
  _mm_store_pd(info->stage + 6, stage2_b_hi);
  _mm_store_pd(info->stage + 8, stage1_a_lo);
  _mm_store_pd(info->stage + 10, stage1_b_lo);
  _mm_store_pd(info->stage + 12, stage1_a_hi);
  _mm_store_pd(info->stage + 14, stage1_b_hi);
  _mm_store_pd(info->filter_info + 8, prev_out);
#else
  // f64x4 prev_cos = _mm256_load_pd(info->prev_cos); // -0.5 0.5 1.5 2.5
  // f64x4 next_cos = _mm256_load_pd(info->next_cos); // 3.5 4.5 5.5 6.5
  // ptr_print(input_signal,output_signal);
  f64x4 angle = _mm256_load_pd(info->angle);
  f64x4 full_delta_angle = _mm256_set1_pd(info->delta_angle * 4);
  for (usize i = 0, j = 0; i < buf_len; i += 8) {
    f64x4 cos_value1 = _mm256_cos_pd(angle);
    angle = _mm256_add_pd(angle, full_delta_angle);
    f64x4 cos_value2 = _mm256_cos_pd(angle);
    angle = _mm256_add_pd(angle, full_delta_angle);
    f64x4 signal1 = _mm256_load_pd(input_signal + i);     // 0 1 2 3
    f64x4 signal2 = _mm256_load_pd(input_signal + i + 4); // 0 1 2 3
    f64x4 sig1 = _mm256_mul_pd(signal1, cos_value1);
    f64x4 sig2 = _mm256_mul_pd(signal2, cos_value2);
    _mm256_store_pd(output_signal + i, sig1);
    _mm256_store_pd(output_signal + i + 4, sig2);
  }
  // printf("cvt-freq-crc: %lu\n",crc32((char*) output_signal, buf_len * 8));
  // fflush(stdout);
  // _mm256_store_pd(info->next_cos,next_cos);
  _mm256_store_pd(info->angle, _mm256_fmod_pd(angle, _mm256_set1_pd(TAU)));
#endif
}
void fm_demodulate(f64 output_signal[], const f64 input_signal[],
                   const f64 sample_period, f64 const fc,
                   DemodulationInfo *const info, const usize buf_len) {
#if DISABLE_SIMD_DEMODULATE
  FilterCoeffs *const coeff = &info->filter_coeff;
  FilterInfo *filter_info = info->filter_info;
  // f64 const coeff = info->filter_coeff;
  // f64* filter_info = info->filter_info;
  // printf("buffer len: %ld\n", buf_len);
  f64 prev_sin = info->prev_sin[0];
  f64 angle = info->angle[0];
  f64 prev_a = info->prev_internal[0];
  f64 prev_b = info->prev_internal[1];
  for (usize i = 0; i < buf_len; i++) {
    const f64 sin_val = sin(angle);
    // const f64 cos_val = cos(angle); //((sin_val - prev_sin) /
    // (TAU*fc*sample_period))
    const f64 cos_val = ((sin_val - prev_sin) / (TAU * fc * sample_period));
    const f64 current_a =
        lpf(-2 * input_signal[i] * sin_val, coeff, &filter_info[0]);
    const f64 current_b =
        lpf(2 * input_signal[i] * cos_val, coeff, &filter_info[2]);
    const f64 re = lpf(prev_a, coeff, &filter_info[1]);
    const f64 im = lpf(prev_b, coeff, &filter_info[3]);
    f64 d_re, d_im;
    differential(&d_re, &d_im, re, im, info->prev_sig, sample_period);
    f64 a = d_re * im;
    f64 b = d_im * re;
    output_signal[i] = 2 * (a - b);
    // output_signal[i] = re;
    prev_sin = sin_val;
    angle += TAU * fc * sample_period;
    prev_a = current_a;
    prev_b = current_b;
  }
  info->angle[0] = fmod(angle, TAU);
  info->prev_sin[0] = prev_sin;
  info->prev_internal[0] = prev_a;
  info->prev_internal[1] = prev_b;
#else
  // printf("Demodulate:: Before-Proc\n");
  // print_demodulate_info(info);
  // printf("buffer size: %lld (%g)\n", buf_len,buf_len/4.);
  // Angles 
  // print_sd(fc);
  f64x4 delta_angle = _mm256_set1_pd(TAU * fc * sample_period * 4);
  f64x4 angle = _mm256_load_pd(info->angle);
  // Prev Signals
  f64x4 prev_sin = _mm256_load_pd(info->prev_sin);
  f64x4 differential_coeff = _mm256_set1_pd(1 / (TAU * fc * sample_period));
  f64x4 prev_sig_lo = _mm256_load_pd(info->prev_sig);
  f64x4 prev_sig_hi = _mm256_load_pd(info->prev_sig + 4);
  f64x4 prev_sig_internal_lo = _mm256_load_pd(info->prev_internal); // 0 0 2 2
  f64x4 prev_sig_internal_hi =
      _mm256_load_pd(info->prev_internal + 4); // 1 1 3 3
  // LPF INFOS
  f64x4 prev_sig = _mm256_load_pd(info->filter_info);
  f64x4 prev_prev_sig = _mm256_load_pd(info->filter_info + 4);
  f64x4 prev_out = _mm256_load_pd(info->filter_info + 8);
  f64x4 prev_prev_out = _mm256_load_pd(info->filter_info + 12);
  // filter coefficients
  f64x4 c0 = _mm256_set1_pd(info->filter_coeff.c0);
  f64x4 c1 = _mm256_set1_pd(info->filter_coeff.c1);
  f64x4 c2 = _mm256_set1_pd(info->filter_coeff.c2);
  f64x4 d0 = _mm256_set1_pd(info->filter_coeff.c3);
  f64x4 d1 = _mm256_set1_pd(info->filter_coeff.c4);
  // printf("lpf coeff: %g,%g,%g,%g,%g\n",info->filter_coeff.c0,info->filter_coeff.c1,info->filter_coeff.c2,info->filter_coeff.c3,info->filter_coeff.c4);
  // ptr_print(input_signal,output_signal);
  // printf("demodulate-crc-before: %lu\n",crc32((char*) input_signal, buf_len * 8));
  // fflush(stdout);
  // //
  // printf("prev sig | prev prev sig @ demodulate\n");
  f64x4 d_coeff = _mm256_set1_pd(1 / sample_period);
  for (usize i = 0; i < buf_len; i += 4) {
    // Removing Carrier
    f64x4 sin_val = _mm256_sin_pd(angle);
    prev_sin = _mm256_blend_pd(sin_val, prev_sin, 0b1000);
    prev_sin = _mm256_ror_pd(prev_sin);
    f64x4 cos_val =
        _mm256_mul_pd(_mm256_sub_pd(sin_val, prev_sin), differential_coeff);
    f64x4 sig = _mm256_load_pd(input_signal + i);
    f64x4 sig1 = _mm256_mul_pd(_mm256_set1_pd(-1), _mm256_mul_pd(sig, sin_val));
    f64x4 sig2 = _mm256_mul_pd(sig, cos_val);
    angle = _mm256_add_pd(angle, delta_angle);
    angle = _mm256_fmod_pd(angle, _mm256_set1_pd(TAU));
    f64x4 prev_sin_tmp = prev_sin;
    prev_sin = sin_val;
    // Signal Interleaving
    // [s1 s2, s1, s2]
    f64x4 sig_lo = _mm256_unpacklo_pd(sig1, sig2); // 0 0 2 2
    f64x4 sig_hi = _mm256_unpackhi_pd(sig1, sig2); // 1 1 3 3
    // [s1, s2, p1, p2]
    f64x4 s0 = _mm256_permute2f128_pd(sig_lo, prev_sig_lo, 0x20);
    f64x4 s2 = _mm256_permute2f128_pd(sig_lo, prev_sig_lo, 0x31);
    f64x4 s1 = _mm256_permute2f128_pd(sig_hi, prev_sig_hi, 0x20);
    f64x4 s3 = _mm256_permute2f128_pd(sig_hi, prev_sig_hi, 0x31);
    // _mm256_print_pd(s3);

    // LPF Process
    // c0 * x[i] + c1 * x[i-1] + c2 * x[i-2] - d0 * y[i-1] - d1 * y[i-2]
    f64x4 o0 = _mm256_fmadd_pd(
        c0, s0,
        _mm256_fmadd_pd(
            c1, prev_sig,
            _mm256_fmadd_pd(
                c2, prev_prev_sig,
                _mm256_fnmsub_pd(d0, prev_out,
                                 _mm256_mul_pd(d1, prev_prev_out)))));
    f64x4 o1 = _mm256_fmadd_pd(
        c0, s1,
        _mm256_fmadd_pd(
            c1, s0,
            _mm256_fmadd_pd(
                c2, prev_sig,
                _mm256_fnmsub_pd(d0, o0, _mm256_mul_pd(d1, prev_out)))));
    f64x4 o2 = _mm256_fmadd_pd(
        c0, s2,
        _mm256_fmadd_pd(
            c1, s1,
            _mm256_fmadd_pd(c2, s0,
                            _mm256_fnmsub_pd(d0, o1, _mm256_mul_pd(d1, o0)))));
    f64x4 o3 = _mm256_fmadd_pd(
        c0, s3,
        _mm256_fmadd_pd(
            c1, s2,
            _mm256_fmadd_pd(c2, s1,
                            _mm256_fnmsub_pd(d0, o2, _mm256_mul_pd(d1, o1)))));
    // move value
    prev_prev_sig = s2;
    prev_sig = s3;
    prev_out = o3;
    prev_prev_out = o2;
    // DeInterleaving
    f64x4 s_lo = _mm256_permute2f128_pd(o0, o2, 0x31); // 0 0 2 2
    f64x4 s_hi = _mm256_permute2f128_pd(o1, o3, 0x31); // 1 1 3 3
    // TEST AFTER LPF SIGNAL
    f64x4 test_point1 = _mm256_unpacklo_pd(s_lo, s_hi); // REAL
    f64x4 test_point2 = _mm256_unpackhi_pd(s_lo, s_hi); // IMAGINARY
    // differential
    prev_sig_internal_lo = _mm256_blend_pd(s_hi, prev_sig_internal_hi, 0b1100);
    prev_sig_internal_lo = _mm256_permute2f128_pd(prev_sig_internal_lo,
                                                  prev_sig_internal_lo, 0x01);
    f64x4 dsig_l = _mm256_mul_pd(_mm256_sub_pd(s_lo, prev_sig_internal_lo),
                                 d_coeff);                            // 0 0 2 2
    f64x4 dsig_h = _mm256_mul_pd(_mm256_sub_pd(s_hi, s_lo), d_coeff); // 1 1 3 3
    f64x4 test_point3 = _mm256_unpacklo_pd(dsig_l, dsig_h);           // REAL'
    f64x4 test_point4 = _mm256_unpackhi_pd(dsig_l, dsig_h); // IMAGINARY'
    // たすき掛け
    dsig_l = _mm256_permute_pd(dsig_l, 0b0101);
    dsig_h = _mm256_permute_pd(dsig_h, 0b0101);
    f64x4 ta = _mm256_mul_pd(dsig_l, s_lo);
    f64x4 tb = _mm256_mul_pd(dsig_h, s_hi);
    f64x4 sig_out = _mm256_hsub_pd(ta, tb);
    // _mm256_store_pd(output_signal+i,test_point2);
    _mm256_store_pd(output_signal + i,
                    _mm256_mul_pd(_mm256_set1_pd(8), sig_out));
    // move value for next loop
    prev_sig_lo = _mm256_permute2f128_pd(o0, o2, 0x20);
    prev_sig_hi = _mm256_permute2f128_pd(o1, o3, 0x20);
    prev_sig_internal_lo = s_lo;
    prev_sig_internal_hi = s_hi;
    // _mm256_print_pd(prev_sig);
    // _mm256_print_pd(prev_prev_sig);
  }
  // printf("demodulate-crc-after: %lu\n",crc32((char*) input_signal, buf_len * 8));
  // fflush(stdout);
  // printf("demodulate-crc-write: %lu\n",crc32((char*) output_signal, buf_len * 8));
  // fflush(stdout);
  
  _mm256_store_pd(info->angle, _mm256_fmod_pd(angle, _mm256_set1_pd(TAU)));
  _mm256_store_pd(info->prev_sin, prev_sin);
  _mm256_store_pd(info->prev_sig, prev_sig_lo);
  _mm256_store_pd(info->prev_sig + 4, prev_sig_hi);
  _mm256_store_pd(info->prev_internal, prev_sig_internal_lo);
  _mm256_store_pd(info->prev_internal + 4, prev_sig_internal_hi);
  _mm256_store_pd(info->filter_info, prev_sig);
  _mm256_store_pd(info->filter_info + 4, prev_prev_sig);
  _mm256_store_pd(info->filter_info + 8, prev_out);
  _mm256_store_pd(info->filter_info + 12, prev_prev_out);
  // printf("Demodulate:: After-Proc\n");
  // print_demodulate_info(info);
#endif
}

void upsample(f64 *dst, f64 *input, ResamplerInfo *info) {
  usize len = info->input_len;
  f64 prev = info->prev;
  usize multiplier = info->multiplier;
  f64x4 offset = _mm256_set1_pd(4);
  f64x4 m = _mm256_set1_pd(1./multiplier);
  // printf("len: %ld / multiplier: %ld\n", len,multiplier);
  for (int i = 0; i < len; ++i) {
    f64x4 a = _mm256_set1_pd(prev);
    f64x4 b = _mm256_set1_pd(input[i]);
    f64x4 n = _mm256_set_pd(3, 2, 1, 0);
    f64 *d = dst + i * multiplier;
    prev = input[i];
    // upsample
    for (usize j = 0; j < multiplier; j += 4) {
      f64x4 coeff1 = _mm256_mul_pd(n, m);
      f64x4 coeff2 = _mm256_sub_pd(_mm256_set1_pd(1), coeff1);
      f64x4 t = _mm256_fmadd_pd(a, coeff1, _mm256_mul_pd(b, coeff1));
      n = _mm256_add_pd(n, offset);
      _mm256_store_pd(d + j, t);
    }
  }
  info->prev = prev;
}

void downsample(f64 *dst, f64 *input, ResamplerInfo *info) {
  usize len = info->input_len;
  // print_resampler_info(info);
  // fflush(stdout);
  // printf("downsample-crc-before: %lu\n",crc32((char*) input, len * 8));
  // fflush(stdout);
  
  usize multiplier = info->multiplier;
  // printf("len: %lld / multiplier: %lld\n", len,multiplier);
  for (usize i = 0, j = 0; i < len; i += multiplier, ++j) {
    dst[j] = input[i];
  }
  // printf("downsample-crc-after: %lu\n",crc32((char*) input, len * 8));
  // fflush(stdout);
  // print_resampler_info(info);
  // fflush(stdout);
  // printf("end down sample\n");
}

void filtering_with_resample(f64 dst[], const f64 input[], FilteringInfo *info, usize buf_len) {
  // printf("BPF:: Before-Proc\n");
  // print_filter_info(info);
  // prev sigs
  f64x2 prev_sig = _mm_load_pd(info->prev_sig);
  f64x2 prev_prev_sig = _mm_load_pd(info->prev_prev_sig);
  f64x2 prev_out = _mm_load_pd(info->prev_out);
  f64x2 prev_prev_out = _mm_load_pd(info->prev_prev_out);
  //
  f64x2 stage_lo = _mm_load_pd(info->stage);
  f64x2 stage_hi = _mm_load_pd(info->stage + 2);
  // filter coeffs
  f64x2 c0 = _mm_set1_pd(info->coeff.c0);
  // f64x2 c1 = _mm_set1_pd(info->coeff.c1);
  // f64x2 c2 = _mm_set1_pd(info->coeff.c2);
  f64x2 d0 = _mm_set1_pd(info->coeff.c3);
  f64x2 d1 = _mm_set1_pd(info->coeff.c4);
  for (usize i = 0; i < buf_len; i += 4) {
    #ifndef BPF_BYPASS
    f64x2 sig_lo = _mm_load_pd(input + i);
    f64x2 sig_hi = _mm_load_pd(input + i + 2);
    //
    f64x2 s0 = _mm_shuffle_pd(stage_lo, sig_lo, 0b00); // 0' 0
    f64x2 s1 = _mm_shuffle_pd(stage_lo, sig_lo, 0b11); // 1' 1
    f64x2 s2 = _mm_shuffle_pd(stage_hi, sig_hi, 0b00); // 2' 2
    f64x2 s3 = _mm_shuffle_pd(stage_hi, sig_hi, 0b11); // 3' 3
    // filter process
    f64x2 v0 = _mm_mul_pd(prev_prev_out, d1);
    f64x2 w0 = _mm_fnmsub_pd(prev_out, d0, v0);
    f64x2 x0 = _mm_sub_pd(s0, prev_prev_sig);
    f64x2 y0 = _mm_fmadd_pd(c0, x0, w0);
    // for s1
    f64x2 v1 = _mm_mul_pd(prev_out, d1);
    f64x2 w1 = _mm_fnmsub_pd(y0, d0, v1);
    f64x2 x1 = _mm_sub_pd(s1, prev_sig);
    f64x2 y1 = _mm_fmadd_pd(c0, x1, w1);
    // for s2
    f64x2 v2 = _mm_mul_pd(y0, d1);
    f64x2 w2 = _mm_fnmsub_pd(y1, d0, v2);
    f64x2 x2 = _mm_sub_pd(s2, s0);
    f64x2 y2 = _mm_fmadd_pd(c0, x2, w2);
    // for s3
    f64x2 v3 = _mm_mul_pd(y1, d1);
    f64x2 w3 = _mm_fnmsub_pd(y2, d0, v3);
    f64x2 x3 = _mm_sub_pd(s3, s0);
    f64x2 y3 = _mm_fmadd_pd(c0, x3, w3);
    // set next stage
    stage_lo = _mm_shuffle_pd(y0, y1, 0b11); // 0' 1'
    stage_hi = _mm_shuffle_pd(y2, y3, 0b11); // 2' 3'
    prev_out = y3;
    prev_prev_out = y2;
    prev_sig = s3;
    prev_prev_sig = s2;
    dst[i >> 2] = 2. * _mm_cvtsd_f64(y3);
    #else
      dst[i >> 2] = input[i];
    #endif
  }
  _mm_store_pd(info->prev_sig, prev_sig);
  _mm_store_pd(info->prev_prev_sig, prev_prev_sig);
  _mm_store_pd(info->prev_out, prev_out);
  _mm_store_pd(info->prev_prev_out, prev_prev_out);
  _mm_store_pd(info->stage, stage_lo);
  _mm_store_pd(info->stage + 2, stage_hi);
}

void filtering(f64 dst[], const f64 input[], FilteringInfo *info, usize buf_len) {
  // printf("BPF:: Before-Proc\n");
  // print_filter_info(info);
  // prev sigs
  f64x2 prev_sig = _mm_load_pd(info->prev_sig);
  f64x2 prev_prev_sig = _mm_load_pd(info->prev_prev_sig);
  f64x2 prev_out = _mm_load_pd(info->prev_out);
  f64x2 prev_prev_out = _mm_load_pd(info->prev_prev_out);
  //
  f64x2 stage_lo = _mm_load_pd(info->stage);
  f64x2 stage_hi = _mm_load_pd(info->stage + 2);
  // filter coeffs
  f64x2 c0 = _mm_set1_pd(info->coeff.c0);
  // f64x2 c1 = _mm_set1_pd(info->coeff.c1);
  // f64x2 c2 = _mm_set1_pd(info->coeff.c2);
  f64x2 d0 = _mm_set1_pd(info->coeff.c3);
  f64x2 d1 = _mm_set1_pd(info->coeff.c4);
  for (usize i = 0; i < buf_len; i += 4) {
    #ifndef BPF_BYPASS
    f64x2 sig_lo = _mm_load_pd(input + i);
    f64x2 sig_hi = _mm_load_pd(input + i + 2);
    //
    f64x2 s0 = _mm_shuffle_pd(stage_lo, sig_lo, 0b00); // 0' 0
    f64x2 s1 = _mm_shuffle_pd(stage_lo, sig_lo, 0b11); // 1' 1
    f64x2 s2 = _mm_shuffle_pd(stage_hi, sig_hi, 0b00); // 2' 2
    f64x2 s3 = _mm_shuffle_pd(stage_hi, sig_hi, 0b11); // 3' 3
    // filter process
    f64x2 v0 = _mm_mul_pd(prev_prev_out, d1);
    f64x2 w0 = _mm_fnmsub_pd(prev_out, d0, v0);
    f64x2 x0 = _mm_sub_pd(s0, prev_prev_sig);
    f64x2 y0 = _mm_fmadd_pd(c0, x0, w0);
    // for s1
    f64x2 v1 = _mm_mul_pd(prev_out, d1);
    f64x2 w1 = _mm_fnmsub_pd(y0, d0, v1);
    f64x2 x1 = _mm_sub_pd(s1, prev_sig);
    f64x2 y1 = _mm_fmadd_pd(c0, x1, w1);
    // for s2
    f64x2 v2 = _mm_mul_pd(y0, d1);
    f64x2 w2 = _mm_fnmsub_pd(y1, d0, v2);
    f64x2 x2 = _mm_sub_pd(s2, s0);
    f64x2 y2 = _mm_fmadd_pd(c0, x2, w2);
    // for s3
    f64x2 v3 = _mm_mul_pd(y1, d1);
    f64x2 w3 = _mm_fnmsub_pd(y2, d0, v3);
    f64x2 x3 = _mm_sub_pd(s3, s0);
    f64x2 y3 = _mm_fmadd_pd(c0, x3, w3);
    // set next stage
    stage_lo = _mm_shuffle_pd(y0, y1, 0b11); // 0' 1'
    stage_hi = _mm_shuffle_pd(y2, y3, 0b11); // 2' 3'
    prev_out = y3;
    prev_prev_out = y2;
    prev_sig = s3;
    prev_prev_sig = s2;
    f64x2 out_lo = _mm_shuffle_pd(y0, y1, 0b00); // 0' 1'
    f64x2 out_hi = _mm_shuffle_pd(y2, y3, 0b00); // 2' 3'
    _mm_store_pd(dst + i, out_lo);
    _mm_store_pd(dst + i + 2, out_hi);
    // dst[i >> 2] = 2. * _mm_cvtsd_f64(y3);
    #else
    _mm_store_pd(dst + i, input);
    _mm_store_pd(dst + i + 2, input);
    #endif
  }
  _mm_store_pd(info->prev_sig, prev_sig);
  _mm_store_pd(info->prev_prev_sig, prev_prev_sig);
  _mm_store_pd(info->prev_out, prev_out);
  _mm_store_pd(info->prev_prev_out, prev_prev_out);
  _mm_store_pd(info->stage, stage_lo);
  _mm_store_pd(info->stage + 2, stage_hi);
}