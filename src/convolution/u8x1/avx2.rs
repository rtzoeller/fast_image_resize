use std::arch::x86_64::*;

use crate::convolution::{optimisations, Coefficients};
use crate::image_view::{FourRows, FourRowsMut, TypedImageView, TypedImageViewMut};
use crate::pixels::U8;
use crate::simd_utils;

#[inline]
pub(crate) fn horiz_convolution(
    src_image: TypedImageView<U8>,
    mut dst_image: TypedImageViewMut<U8>,
    offset: u32,
    coeffs: Coefficients,
) {
    let (values, window_size, bounds_per_pixel) =
        (coeffs.values, coeffs.window_size, coeffs.bounds);

    let normalizer_guard = optimisations::NormalizerGuard16::new(values);
    let coefficients_chunks = normalizer_guard.normalized_chunks(window_size, &bounds_per_pixel);
    let dst_height = dst_image.height().get();

    let src_iter = src_image.iter_4_rows(offset, dst_height + offset);
    let dst_iter = dst_image.iter_4_rows_mut();
    for (src_rows, dst_rows) in src_iter.zip(dst_iter) {
        unsafe {
            horiz_convolution_8u4x(src_rows, dst_rows, &coefficients_chunks, &normalizer_guard);
        }
    }

    let mut yy = dst_height - dst_height % 4;
    while yy < dst_height {
        unsafe {
            horiz_convolution_8u(
                src_image.get_row(yy + offset).unwrap(),
                dst_image.get_row_mut(yy).unwrap(),
                &coefficients_chunks,
                &normalizer_guard,
            );
        }
        yy += 1;
    }
}

/// For safety, it is necessary to ensure the following conditions:
/// - length of all rows in src_rows must be equal
/// - length of all rows in dst_rows must be equal
/// - coefficients_chunks.len() == dst_rows.0.len()
/// - max(chunk.start + chunk.values.len() for chunk in coefficients_chunks) <= src_row.0.len()
/// - precision <= MAX_COEFS_PRECISION
#[inline]
#[target_feature(enable = "avx2")]
unsafe fn horiz_convolution_8u4x(
    src_rows: FourRows<U8>,
    dst_rows: FourRowsMut<U8>,
    coefficients_chunks: &[optimisations::CoefficientsI16Chunk],
    normalizer_guard: &optimisations::NormalizerGuard16,
) {
    let s_rows = [src_rows.0, src_rows.1, src_rows.2, src_rows.3];
    let d_rows = [dst_rows.0, dst_rows.1, dst_rows.2, dst_rows.3];
    let zero = _mm_setzero_si128();
    // 8 components will be added, use only 1/8 of the error
    let initial = _mm256_set1_epi32(1 << (normalizer_guard.precision() - 4));

    for (dst_x, coeffs_chunk) in coefficients_chunks.iter().enumerate() {
        let coeffs = coeffs_chunk.values;
        let mut x = coeffs_chunk.start as usize;
        let mut result_i32x8 = [initial, initial, initial, initial];

        let coeffs_by_16 = coeffs.chunks_exact(16);
        let reminder16 = coeffs_by_16.remainder();
        for k in coeffs_by_16 {
            let coeffs_i16x16 = _mm256_loadu_si256(k.as_ptr() as *const __m256i);
            for i in 0..4 {
                let pixels_u8x16 = simd_utils::loadu_si128(s_rows[i], x);
                let pixels_i16x16 = _mm256_cvtepu8_epi16(pixels_u8x16);
                result_i32x8[i] = _mm256_add_epi32(
                    result_i32x8[i],
                    _mm256_madd_epi16(pixels_i16x16, coeffs_i16x16),
                );
            }
            x += 16;
        }

        let mut coeffs_by_8 = reminder16.chunks_exact(8);
        let reminder8 = coeffs_by_8.remainder();
        if let Some(k) = coeffs_by_8.next() {
            let coeffs_i16x8 = _mm_loadu_si128(k.as_ptr() as *const __m128i);
            for i in 0..4 {
                let pixels_u8x8 = simd_utils::loadl_epi64(s_rows[i], x);
                let pixels_i16x8 = _mm_cvtepu8_epi16(pixels_u8x8);
                result_i32x8[i] = _mm256_add_epi32(
                    result_i32x8[i],
                    _mm256_set_m128i(zero, _mm_madd_epi16(pixels_i16x8, coeffs_i16x8)),
                );
            }
            x += 8;
        }

        let mut result_i32x4 = result_i32x8.map(|v| hsum_i32x8_avx2(v));

        for &coeff in reminder8 {
            let coeff_i32 = coeff as i32;
            for i in 0..4 {
                result_i32x4[i] += s_rows[i].get_unchecked(x).0.to_owned() as i32 * coeff_i32;
            }
            x += 1;
        }

        let result_u8x4 = result_i32x4.map(|v| normalizer_guard.clip(v));
        for i in 0..4 {
            d_rows[i].get_unchecked_mut(dst_x).0 = result_u8x4[i];
        }
    }
}

/// For safety, it is necessary to ensure the following conditions:
/// - bounds.len() == dst_row.len()
/// - coeffs.len() == dst_rows.0.len() * window_size
/// - max(bound.start + bound.size for bound in bounds) <= src_row.len()
/// - precision <= MAX_COEFS_PRECISION
#[inline]
#[target_feature(enable = "avx2")]
unsafe fn horiz_convolution_8u(
    src_row: &[U8],
    dst_row: &mut [U8],
    coefficients_chunks: &[optimisations::CoefficientsI16Chunk],
    normalizer_guard: &optimisations::NormalizerGuard16,
) {
    let zero = _mm_setzero_si128();
    // 8 components will be added, use only 1/8 of the error
    let initial = _mm256_set1_epi32(1 << (normalizer_guard.precision() - 4));

    for (dst_x, &coeffs_chunk) in coefficients_chunks.iter().enumerate() {
        let coeffs = coeffs_chunk.values;
        let mut x = coeffs_chunk.start as usize;
        let mut result_i32x8 = initial;

        let coeffs_by_16 = coeffs.chunks_exact(16);
        let reminder16 = coeffs_by_16.remainder();
        for k in coeffs_by_16 {
            let coeffs_i16x16 = _mm256_loadu_si256(k.as_ptr() as *const __m256i);
            let pixels_u8x16 = simd_utils::loadu_si128(src_row, x);
            let pixels_i16x16 = _mm256_cvtepu8_epi16(pixels_u8x16);
            result_i32x8 = _mm256_add_epi32(
                result_i32x8,
                _mm256_madd_epi16(pixels_i16x16, coeffs_i16x16),
            );
            x += 16;
        }

        let mut coeffs_by_8 = reminder16.chunks_exact(8);
        let reminder8 = coeffs_by_8.remainder();
        if let Some(k) = coeffs_by_8.next() {
            let coeffs_i16x8 = _mm_loadu_si128(k.as_ptr() as *const __m128i);
            let pixels_u8x8 = simd_utils::loadl_epi64(src_row, x);
            let pixels_i16x8 = _mm_cvtepu8_epi16(pixels_u8x8);
            result_i32x8 = _mm256_add_epi32(
                result_i32x8,
                _mm256_set_m128i(zero, _mm_madd_epi16(pixels_i16x8, coeffs_i16x8)),
            );

            x += 8;
        }

        let mut result_i32 = hsum_i32x8_avx2(result_i32x8);

        for &coeff in reminder8 {
            let coeff_i32 = coeff as i32;
            result_i32 += src_row.get_unchecked(x).0 as i32 * coeff_i32;
            x += 1;
        }

        dst_row.get_unchecked_mut(dst_x).0 = normalizer_guard.clip(result_i32);
    }
}

// only needs AVX2
#[inline]
#[target_feature(enable = "avx2")]
unsafe fn hsum_i32x8_avx2(v: __m256i) -> i32 {
    let sum128 = _mm_add_epi32(_mm256_castsi256_si128(v), _mm256_extracti128_si256::<1>(v));
    hsum_epi32_avx(sum128)
}

#[inline(always)]
unsafe fn hsum_epi32_avx(x: __m128i) -> i32 {
    // 3-operand non-destructive AVX lets us save a byte without needing a movdqa
    let hi64 = _mm_unpackhi_epi64(x, x);
    let sum64 = _mm_add_epi32(hi64, x);
    const I: i32 = ((2 << 6) | (3 << 4) | 1) as i32;
    let hi32 = _mm_shuffle_epi32::<I>(sum64); // Swap the low two elements
    let sum32 = _mm_add_epi32(sum64, hi32);
    _mm_cvtsi128_si32(sum32) // movd
}
