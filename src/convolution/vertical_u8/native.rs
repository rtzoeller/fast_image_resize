use crate::convolution::optimisations::NormalizerGuard16;
use crate::convolution::{optimisations, Coefficients};
use crate::image_view::{TypedImageView, TypedImageViewMut};
use crate::pixels::Pixel;

#[inline(always)]
pub(crate) fn vert_convolution<T>(
    src_image: TypedImageView<T>,
    mut dst_image: TypedImageViewMut<T>,
    coeffs: Coefficients,
) where
    T: Pixel<Component = u8>,
{
    // Check safety conditions
    debug_assert_eq!(src_image.width(), dst_image.width());
    debug_assert_eq!(coeffs.bounds.len(), dst_image.height().get() as usize);

    let (values, window_size, bounds) = (coeffs.values, coeffs.window_size, coeffs.bounds);
    let normalizer_guard = optimisations::NormalizerGuard16::new(values);
    let coefficients_chunks = normalizer_guard.normalized_chunks(window_size, &bounds);
    let precision = normalizer_guard.precision();
    let initial = 1 << (precision - 1);

    let dst_rows = dst_image.iter_rows_mut();
    let coeffs_chunks_iter = coefficients_chunks.into_iter();
    for (coeffs_chunk, dst_row) in coeffs_chunks_iter.zip(dst_rows) {
        let first_y_src = coeffs_chunk.start;
        let ks = coeffs_chunk.values;
        let mut x_src: usize = 0;
        let dst_components = T::components_mut(dst_row);
        let (head, dst_chunks, tail) = unsafe { dst_components.align_to_mut::<u32>() };

        if !head.is_empty() {
            x_src = convolution_by_u8(
                &src_image,
                &normalizer_guard,
                initial,
                head,
                x_src,
                first_y_src,
                ks,
            );
        }

        // Convolution by u8x4
        for dst_chunk in dst_chunks {
            let mut ss = [initial; 4];
            let src_rows = src_image.iter_rows(first_y_src);
            for (&k, src_row) in ks.iter().zip(src_rows) {
                let src_ptr = src_row.as_ptr() as *const u8;
                let src_chunk = unsafe {
                    let ptr = src_ptr.add(x_src) as *const u32;
                    *ptr
                };

                let components: [u8; 4] = src_chunk.to_le_bytes();
                for (s, c) in ss.iter_mut().zip(components) {
                    *s += c as i32 * (k as i32);
                }
            }
            *dst_chunk = u32::from_le_bytes(ss.map(|v| unsafe { normalizer_guard.clip(v) }));
            x_src += 4;
        }

        if !tail.is_empty() {
            convolution_by_u8(
                &src_image,
                &normalizer_guard,
                initial,
                tail,
                x_src,
                first_y_src,
                ks,
            );
        }
    }
}

#[inline(always)]
fn convolution_by_u8<T>(
    src_image: &TypedImageView<T>,
    normalizer_guard: &NormalizerGuard16,
    initial: i32,
    dst_components: &mut [u8],
    mut x_src: usize,
    first_y_src: u32,
    ks: &[i16],
) -> usize
where
    T: Pixel<Component = u8>,
{
    for dst_component in dst_components {
        let mut ss = initial;
        let src_rows = src_image.iter_rows(first_y_src);
        for (&k, src_row) in ks.iter().zip(src_rows) {
            let src_ptr = src_row.as_ptr() as *const u8;
            let src_component = unsafe { *src_ptr.add(x_src as usize) };
            ss += src_component as i32 * (k as i32);
        }
        *dst_component = unsafe { normalizer_guard.clip(ss) };
        x_src += 1
    }
    x_src
}
