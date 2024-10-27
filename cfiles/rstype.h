#pragma once
#include <immintrin.h>
#include <stdint.h>
#include "f64consts.h"
typedef uint16_t u16;
typedef uint8_t u8;
typedef uint32_t u32;
typedef uint64_t u64;
typedef u64 usize;

typedef int16_t i16;
typedef int8_t i8;
typedef int32_t i32;
typedef int64_t i64;
typedef i64 isize;

typedef double f64;
typedef float f32;
typedef __m128 f32x4;
typedef __m128d f64x2;
typedef __m256 f32x8;
typedef __m256d f64x4;