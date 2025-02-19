use std::num::NonZeroU32;

use fast_image_resize::pixels::*;
use fast_image_resize::{
    CpuExtensions, DifferentTypesOfPixelsError, FilterType, Image, ImageView, PixelType, ResizeAlg,
    Resizer,
};
use utils::{cpu_ext_into_str, PixelExt};

mod utils;

fn get_new_height(src_image: &ImageView, new_width: u32) -> u32 {
    let scale = new_width as f32 / src_image.width().get() as f32;
    (src_image.height().get() as f32 * scale).round() as u32
}

const NEW_WIDTH: u32 = 255;
const NEW_BIG_WIDTH: u32 = 5016;

#[test]
fn try_resize_to_other_pixel_type() {
    let src_image = U8x4::load_big_src_image();
    let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Lanczos3));
    let mut dst_image = Image::new(
        NonZeroU32::new(1024).unwrap(),
        NonZeroU32::new(256).unwrap(),
        PixelType::U8,
    );
    assert!(matches!(
        resizer.resize(&src_image.view(), &mut dst_image.view_mut()),
        Err(DifferentTypesOfPixelsError)
    ));
}

fn downscale_test<P: PixelExt>(resize_alg: ResizeAlg, cpu_extensions: CpuExtensions) -> Vec<u8> {
    let image = P::load_big_src_image();
    assert_eq!(image.pixel_type(), P::pixel_type());

    let mut resizer = Resizer::new(resize_alg);
    unsafe {
        resizer.set_cpu_extensions(cpu_extensions);
    }
    let new_height = get_new_height(&image.view(), NEW_WIDTH);
    let mut result = Image::new(
        NonZeroU32::new(NEW_WIDTH).unwrap(),
        NonZeroU32::new(new_height).unwrap(),
        image.pixel_type(),
    );
    assert!(resizer
        .resize(&image.view(), &mut result.view_mut())
        .is_ok());

    let alg_name = match resize_alg {
        ResizeAlg::Nearest => "nearest",
        ResizeAlg::Convolution(filter) => match filter {
            FilterType::Box => "box",
            FilterType::Bilinear => "bilinear",
            FilterType::Hamming => "hamming",
            FilterType::Mitchell => "mitchell",
            FilterType::CatmullRom => "catmullrom",
            FilterType::Lanczos3 => "lanczos3",
            _ => "unknown",
        },
        ResizeAlg::SuperSampling(_, _) => "supersampling",
        _ => "unknown",
    };

    let name = format!(
        "downscale-{}-{}-{}",
        P::pixel_type_str(),
        alg_name,
        cpu_ext_into_str(cpu_extensions),
    );
    utils::save_result(&result, &name);
    result.buffer().to_owned()
}

fn upscale_test<P: PixelExt>(resize_alg: ResizeAlg, cpu_extensions: CpuExtensions) -> Vec<u8> {
    let image = P::load_small_src_image();
    assert_eq!(image.pixel_type(), P::pixel_type());

    let mut resizer = Resizer::new(resize_alg);
    unsafe {
        resizer.set_cpu_extensions(cpu_extensions);
    }
    let new_height = get_new_height(&image.view(), NEW_BIG_WIDTH);
    let mut result = Image::new(
        NonZeroU32::new(NEW_BIG_WIDTH).unwrap(),
        NonZeroU32::new(new_height).unwrap(),
        image.pixel_type(),
    );
    assert!(resizer
        .resize(&image.view(), &mut result.view_mut())
        .is_ok());

    let alg_name = match resize_alg {
        ResizeAlg::Nearest => "nearest",
        ResizeAlg::Convolution(filter) => match filter {
            FilterType::Box => "box",
            FilterType::Bilinear => "bilinear",
            FilterType::Hamming => "hamming",
            FilterType::Mitchell => "mitchell",
            FilterType::CatmullRom => "catmullrom",
            FilterType::Lanczos3 => "lanczos3",
            _ => "unknown",
        },
        ResizeAlg::SuperSampling(_, _) => "supersampling",
        _ => "unknown",
    };

    let name = format!(
        "upscale-{}-{}-{}",
        P::pixel_type_str(),
        alg_name,
        cpu_ext_into_str(cpu_extensions),
    );
    utils::save_result(&result, &name);
    result.buffer().to_owned()
}

#[test]
fn downscale_u8() {
    type P = U8;
    let buffer = downscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    assert_eq!(utils::image_checksum::<1>(&buffer), [2920348]);

    let mut cpu_extensions_vec = vec![CpuExtensions::None];
    #[cfg(target_arch = "x86_64")]
    {
        cpu_extensions_vec.push(CpuExtensions::Avx2);
    }
    for cpu_extensions in cpu_extensions_vec {
        let buffer =
            downscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
        assert_eq!(utils::image_checksum::<1>(&buffer), [2923557]);
    }
}

#[test]
fn downscale_u8_1() {
    let src_vec = vec![1u8; 1280 * 720];
    let src_image = Image::from_vec_u8(
        NonZeroU32::new(1280).unwrap(),
        NonZeroU32::new(720).unwrap(),
        src_vec,
        PixelType::U8,
    )
    .unwrap();
    let mut dst_iamge = Image::new(
        NonZeroU32::new(64).unwrap(),
        NonZeroU32::new(64).unwrap(),
        PixelType::U8,
    );
    let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Lanczos3));
    unsafe { resizer.set_cpu_extensions(CpuExtensions::Avx2) };
    resizer
        .resize(&src_image.view(), &mut dst_iamge.view_mut())
        .unwrap();
}

#[test]
fn upscale_u8() {
    type P = U8;
    let buffer = upscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    assert_eq!(utils::image_checksum::<1>(&buffer), [1148754010]);

    let mut cpu_extensions_vec = vec![CpuExtensions::None];
    #[cfg(target_arch = "x86_64")]
    {
        cpu_extensions_vec.push(CpuExtensions::Avx2);
    }
    for cpu_extensions in cpu_extensions_vec {
        let buffer =
            upscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
        assert_eq!(utils::image_checksum::<1>(&buffer), [1148811406]);
    }
}

#[test]
fn downscale_u8x3() {
    type P = U8x3;
    let buffer = downscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    assert_eq!(
        utils::image_checksum::<3>(&buffer),
        [2937940, 2945380, 2882679]
    );

    let mut cpu_extensions_vec = vec![CpuExtensions::None];
    #[cfg(target_arch = "x86_64")]
    {
        cpu_extensions_vec.push(CpuExtensions::Sse4_1);
        cpu_extensions_vec.push(CpuExtensions::Avx2);
    }
    for cpu_extensions in cpu_extensions_vec {
        let buffer =
            downscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
        assert_eq!(
            utils::image_checksum::<3>(&buffer),
            [2942479, 2947850, 2885072]
        );
    }
}

#[test]
fn upscale_u8x3() {
    type P = U8x3;
    let buffer = upscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    assert_eq!(
        utils::image_checksum::<3>(&buffer),
        [1156008260, 1158417906, 1135087540]
    );

    let mut cpu_extensions_vec = vec![CpuExtensions::None];
    #[cfg(target_arch = "x86_64")]
    {
        cpu_extensions_vec.push(CpuExtensions::Sse4_1);
        cpu_extensions_vec.push(CpuExtensions::Avx2);
    }
    for cpu_extensions in cpu_extensions_vec {
        let buffer =
            upscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
        assert_eq!(
            utils::image_checksum::<3>(&buffer),
            [1156107005, 1158443335, 1135101759]
        );
    }
}

#[test]
fn downscale_u16x3() {
    type P = U16x3;
    let buffer = downscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    assert_eq!(
        utils::image_u16_checksum::<3>(&buffer),
        [755050580, 756962660, 740848503]
    );

    let mut cpu_extensions_vec = vec![CpuExtensions::None];
    #[cfg(target_arch = "x86_64")]
    {
        cpu_extensions_vec.push(CpuExtensions::Sse4_1);
        cpu_extensions_vec.push(CpuExtensions::Avx2);
    }
    for cpu_extensions in cpu_extensions_vec {
        let buffer =
            downscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
        assert_eq!(
            utils::image_u16_checksum::<3>(&buffer),
            [756269847, 757632467, 741478612]
        );
    }
}

#[test]
fn upscale_u16x3() {
    type P = U16x3;
    let buffer = upscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    assert_eq!(
        utils::image_u16_checksum::<3>(&buffer),
        [297094122820, 297713401842, 291717497780]
    );

    let mut cpu_extensions_vec = vec![CpuExtensions::None];
    #[cfg(target_arch = "x86_64")]
    {
        cpu_extensions_vec.push(CpuExtensions::Sse4_1);
        cpu_extensions_vec.push(CpuExtensions::Avx2);
    }
    for cpu_extensions in cpu_extensions_vec {
        let buffer =
            upscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
        assert_eq!(
            utils::image_u16_checksum::<3>(&buffer),
            [297122154090, 297723994984, 291725294637]
        );
    }
}

#[test]
fn downscale_u8x4() {
    type P = U8x4;
    let buffer = downscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    assert_eq!(
        utils::image_checksum::<4>(&buffer),
        [2937940, 2945380, 2882679, 11054250]
    );

    let mut cpu_extensions_vec = vec![CpuExtensions::None];
    #[cfg(target_arch = "x86_64")]
    {
        cpu_extensions_vec.push(CpuExtensions::Sse4_1);
        cpu_extensions_vec.push(CpuExtensions::Avx2);
    }
    for cpu_extensions in cpu_extensions_vec {
        let buffer =
            downscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
        assert_eq!(
            utils::image_checksum::<4>(&buffer),
            [2942479, 2947850, 2885072, 11054250]
        );

        downscale_test::<P>(
            ResizeAlg::SuperSampling(FilterType::Lanczos3, 2),
            cpu_extensions,
        );
    }
}

#[test]
fn upscale_u8x4() {
    type P = U8x4;
    let buffer = upscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    assert_eq!(
        utils::image_checksum::<4>(&buffer),
        [1156008260, 1158417906, 1135087540, 4269569040]
    );

    let mut cpu_extensions_vec = vec![CpuExtensions::None];
    #[cfg(target_arch = "x86_64")]
    {
        cpu_extensions_vec.push(CpuExtensions::Sse4_1);
        cpu_extensions_vec.push(CpuExtensions::Avx2);
    }
    for cpu_extensions in cpu_extensions_vec {
        let buffer =
            upscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
        assert_eq!(
            utils::image_checksum::<4>(&buffer),
            [1156107005, 1158443335, 1135101759, 4269569040]
        );
    }
}

// #[test]
fn _resize_i32() {
    type P = I32;
    downscale_test::<P>(ResizeAlg::Nearest, CpuExtensions::None);
    for cpu_extensions in [CpuExtensions::None] {
        downscale_test::<P>(ResizeAlg::Convolution(FilterType::Lanczos3), cpu_extensions);
    }
}
