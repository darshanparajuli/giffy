//! giffy is a simple GIF decoder.
//!
//! # Example
//!
//! ```no_run
//! use giffy;
//! use std::fs::File;
//!
//! let mut src = File::open("<gif path>").expect("File not found");
//! match giffy::load(&mut src) {
//!     Ok(gif) => {
//!         for frame in gif.image_frames {
//!             // do something with the frame
//!         }
//!     }
//!
//!     Err(e) => {
//!         eprintln!("Error: {}", e);
//!     }
//! }
//! ```

mod decompressor;
mod parser;
mod util;

use decompressor::Decompressor;
use parser::*;
use std::io::Read;

pub use util::Color;

/// This struct holds the width, height and the image frames of the GIF media.
#[derive(Debug, Clone)]
pub struct Gif {
    /// The width of the GIF media.
    pub width: u32,
    /// The height of the GIF media.
    pub height: u32,
    /// Individual image frames.
    pub image_frames: Vec<ImageFrame>,
}

/// This struct is used to hold the color information and the delay time of a frame.
#[derive(Debug, Clone)]
pub struct ImageFrame {
    /// The colors that make up the image frame. This is used for drawing the image frame.
    pub colors: Box<[Color]>,
    /// The amount of time this image frame should stay on screen before moving
    /// on to the next image frame.
    pub delay_time: u16,
}

/// Attempt to load a GIF from a given `src`.
///
/// # Errors
///
/// This function will return an error if the GIF src is not in a valid GIF format.
pub fn load<R>(src: &mut R) -> Result<Gif, String>
where
    R: Read,
{
    let mut parser = Parser::new(src);
    let result = parser.parse()?;

    let decoder = Decoder::new(&result);
    let frames = decoder.decode()?;

    Ok(Gif {
        image_frames: frames,
        width: result.logical_screen_descriptor.width as u32,
        height: result.logical_screen_descriptor.height as u32,
    })
}

struct Decoder<'a> {
    data: &'a ParseResult,
}

impl<'a> Decoder<'a> {
    fn new(input: &'a ParseResult) -> Self {
        Self { data: input }
    }

    fn decode(&self) -> Result<Vec<ImageFrame>, String> {
        let mut frames = vec![];

        for block in self.data.data_blocks.iter() {
            if let DataType::TableBasedImageType(image) = block {
                let color_table = {
                    if image.local_color_table.is_some() {
                        image.local_color_table.as_ref().unwrap()
                    } else {
                        self.data
                            .logical_screen_descriptor
                            .global_color_table
                            .as_ref()
                            .ok_or("Global color table is missing!")?
                    }
                };

                let (transparent_flag, transparent_color_index, disposal_method, delay_time) =
                    match image.graphic_control_extension {
                        Some(ref ext) => (
                            ext.transparent_color_index_available,
                            ext.transparent_color_index,
                            ext.disposal_method,
                            ext.delay_time,
                        ),
                        None => (false, 0, DisposalMethod::Unspecified, 0),
                    };

                let mut decompressor = Decompressor::new(
                    &image.image_data.data_sub_blocks,
                    image.image_data.lzw_min_code_size,
                );

                let index_table = decompressor.decompress()?;

                if frames.is_empty() {
                    frames.push(self.create_first_frame(
                        &index_table,
                        &color_table,
                        image.image_descriptor.interlace_flag,
                        delay_time,
                    )?);
                } else {
                    frames.push(self.create_frame(
                        &frames,
                        &image,
                        &index_table,
                        &color_table,
                        disposal_method,
                        transparent_flag,
                        transparent_color_index,
                        delay_time,
                    )?);
                }
            }
        }

        Ok(frames)
    }

    fn create_first_frame(
        &self,
        index_table: &[usize],
        color_table: &[Color],
        interlace_flag: bool,
        delay_time: u16,
    ) -> Result<ImageFrame, String> {
        let result = index_table
            .iter()
            .map(|i| Some(color_table[*i]))
            .collect::<Vec<_>>();

        let result = if interlace_flag {
            Self::deinterlace(
                result,
                self.data.logical_screen_descriptor.width as usize,
                self.data.logical_screen_descriptor.height as usize,
            )
        } else {
            result
        };

        let result = result
            .into_iter()
            .collect::<Option<Vec<Color>>>()
            .ok_or("Missing color value")?
            .into_boxed_slice();

        Ok(ImageFrame {
            delay_time,
            colors: result,
        })
    }

    fn create_frame(
        &self,
        frames: &[ImageFrame],
        image: &TableBasedImage,
        index_table: &[usize],
        color_table: &[Color],
        disposal_method: DisposalMethod,
        transparent_flag: bool,
        transparent_color_index: u8,
        delay_time: u16,
    ) -> Result<ImageFrame, String> {
        let top = image.image_descriptor.top as usize;
        let height = image.image_descriptor.height as usize;
        let left = image.image_descriptor.left as usize;
        let width = image.image_descriptor.width as usize;
        let image_width = self.data.logical_screen_descriptor.width as usize;

        let result = if transparent_flag {
            index_table
                .iter()
                .map(|i| {
                    if *i == transparent_color_index as usize {
                        None
                    } else {
                        Some(color_table[*i])
                    }
                })
                .collect::<Vec<_>>()
        } else {
            index_table
                .iter()
                .map(|i| Some(color_table[*i]))
                .collect::<Vec<_>>()
        };

        let mut new_frame = match disposal_method {
            DisposalMethod::RestoreToBackgroundColor => ImageFrame {
                delay_time,
                colors: vec![
                    color_table[self.data.logical_screen_descriptor.background_color_index
                        as usize];
                    frames.last().unwrap().colors.len()
                ]
                .into_boxed_slice(),
            },
            DisposalMethod::DoNotDispose | DisposalMethod::Unspecified => {
                frames.last().unwrap().clone()
            }
            d => return Err(format!("Dispose method {:?} not supported", d)),
        };

        let result = if image.image_descriptor.interlace_flag {
            Self::deinterlace(result, width, height)
        } else {
            result
        };

        for y in 0..height {
            let offset = (top + y) * image_width + left;
            for x in 0..width {
                let c = result[y * width + x];
                if let Some(c) = c {
                    new_frame.colors[offset + x] = c;
                }
            }
        }

        Ok(new_frame)
    }

    // Refer to https://www.w3.org/Graphics/GIF/spec-gif89a.txt for details.
    fn deinterlace(input: Vec<Option<Color>>, width: usize, height: usize) -> Vec<Option<Color>> {
        let mut result = vec![None; width * height];

        let mut index = 0;
        let passes = [(0, 8), (4, 8), (2, 4), (1, 2)];

        for (start, step) in passes.iter() {
            'l: for y in (*start..height as usize).step_by(*step) {
                for x in 0..width as usize {
                    let index_dst = y * width as usize + x;
                    if index_dst >= result.len() {
                        break 'l;
                    }

                    result[index_dst] = input[index];
                    index += 1;
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    struct MockReader<'a> {
        data: &'a [u8],
        remaining: usize,
    }

    impl<'a> Read for MockReader<'a> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut count = 0;

            if self.remaining > 0 {
                let offset = self.data.len() - self.remaining;

                for i in 0..buf.len() {
                    buf[i] = self.data[offset + i];
                }

                self.remaining -= buf.len();
                count += buf.len();
            }

            Ok(count)
        }
    }

    #[test]
    fn test_sample_gif() {
        let input = vec![
            71, 73, 70, 56, 57, 97, 10, 0, 10, 0, 145, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 255,
            0, 0, 0, 33, 249, 4, 0, 0, 0, 0, 0, 44, 0, 0, 0, 0, 10, 0, 10, 0, 0, 2, 22, 140, 45,
            153, 135, 42, 28, 220, 51, 160, 2, 117, 236, 149, 250, 168, 222, 96, 140, 4, 145, 76,
            1, 0, 59,
        ];

        let mut reader = MockReader {
            data: &input,
            remaining: input.len(),
        };

        let expected = vec![vec![
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 255, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(0, 0, 255),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
            Color(255, 0, 0),
        ]
        .into_boxed_slice()];

        let mut parser = Parser::new(&mut reader);
        let result = parser.parse().unwrap();

        let decoder = Decoder::new(&result);
        let actual = decoder.decode().unwrap();

        let mut v = vec![];
        for i in actual.iter() {
            v.push(i.colors.clone());
        }

        assert_eq!(expected, v);
    }
}
