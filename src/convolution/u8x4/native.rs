use crate::convolution::{optimisations, Coefficients};
use crate::image_view::{TypedImageView, TypedImageViewMut};
use crate::pixels::U8x4;

pub(crate) fn horiz_convolution(
    src_image: TypedImageView<U8x4>,
    mut dst_image: TypedImageViewMut<U8x4>,
    offset: u32,
    coeffs: Coefficients,
) {
    let (values, window_size, bounds) = (coeffs.values, coeffs.window_size, coeffs.bounds);

    let normalizer_guard = optimisations::NormalizerGuard16::new(values);
    let precision = normalizer_guard.precision();
    let coefficients_chunks = normalizer_guard.normalized_chunks(window_size, &bounds);
    let initial = 1 << (precision - 1);

    let src_rows = src_image.iter_rows(offset);
    let dst_rows = dst_image.iter_rows_mut();
    for (dst_row, src_row) in dst_rows.zip(src_rows) {
        for (&coeffs_chunk, dst_pixel) in coefficients_chunks.iter().zip(dst_row.iter_mut()) {
            let first_x_src = coeffs_chunk.start as usize;
            let ks = coeffs_chunk.values;
            let mut ss = [initial; 4];
            let src_pixels = unsafe { src_row.get_unchecked(first_x_src..) };
            for (&k, &src_pixel) in ks.iter().zip(src_pixels) {
                let components: [u8; 4] = src_pixel.0.to_le_bytes();
                for (i, s) in ss.iter_mut().enumerate() {
                    *s += components[i] as i32 * (k as i32);
                }
            }
            dst_pixel.0 = u32::from_le_bytes(ss.map(|v| unsafe { normalizer_guard.clip(v) }));
        }
    }
}
