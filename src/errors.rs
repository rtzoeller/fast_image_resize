use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, PartialEq)]
pub enum ImageRowsError {
    #[error("Count of rows don't match to image height")]
    InvalidRowsCount,
    #[error("Size of row don't match to image width")]
    InvalidRowSize,
}

#[derive(Error, Debug, Clone, Copy, PartialEq)]
pub enum ImageBufferError {
    #[error("Size of buffer is smaller than required.")]
    InvalidBufferSize,
    #[error("Alignment of buffer don't match to alignment of u32")]
    InvalidBufferAlignment,
}

#[derive(Error, Debug, Clone, Copy, PartialEq)]
pub enum CropBoxError {
    #[error("Position of the crop box is out of the image boundaries")]
    PositionIsOutOfImageBoundaries,
    #[error("Size of the crop box is out of the image boundaries")]
    SizeIsOutOfImageBoundaries,
}

#[derive(Error, Debug, Clone, Copy)]
#[error("Type of pixels of the source image is not equal to pixel type of the destination image.")]
pub struct DifferentTypesOfPixelsError;
