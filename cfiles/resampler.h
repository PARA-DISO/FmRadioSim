#pragma once
#include "rstype.h"
typedef struct {
  f64 prev;
  usize multiplier;
  usize input_len;
} ResamplerInfo;
void upsample(f64* dst, f64* input, ResamplerInfo* info);
void downsample(f64* dst, f64* input, ResamplerInfo* info);