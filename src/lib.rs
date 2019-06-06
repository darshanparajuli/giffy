#![allow(unused_doc_comments)]

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
    pub width: u32,
    pub height: u32,
    pub image_frames: Vec<ImageFrame>,
}

/// This struct holds the color values of the image frame.
#[derive(Debug, Clone)]
pub struct ImageFrame {
    pub color_values: Box<[Color]>,
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

        let mut graphic_control_ext = None;

        for block in self.data.data_blocks.iter() {
            match block {
                DataType::ApplicationExtensionType(_) => {}
                DataType::CommentExtensionType(_) => {}
                DataType::GraphicControlExtensionType(ext) => {
                    graphic_control_ext.replace(ext);
                }
                DataType::PlainTextExtensionType(_) => {}
                DataType::TableBasedImageType(image) => {
                    let color_table = {
                        if image.local_color_table.is_some() {
                            image.local_color_table.as_ref().unwrap()
                        } else {
                            self.data
                                .logical_screen_descriptor
                                .global_color_table
                                .as_ref()
                                .expect("Global color table is missing!")
                        }
                    };

                    let (transparent_flag, transparent_color_index, disposal_method) =
                        match graphic_control_ext {
                            Some(ext) => (
                                ext.transparent_color_index_available,
                                ext.transparent_color_index,
                                ext.disposal_method,
                            ),
                            None => (false, 0, DisposalMethod::Unspecified),
                        };

                    graphic_control_ext = None;

                    let mut decompressor = Decompressor::new(
                        &image.image_data.data_sub_blocks,
                        image.image_data.lzw_min_code_size,
                    );

                    let result = decompressor.decompress()?;

                    if frames.is_empty() {
                        let result = result.iter().map(|i| color_table[*i]).collect::<Vec<_>>();
                        frames.push(ImageFrame {
                            color_values: result.into_boxed_slice(),
                        });
                    } else {
                        let top = image.image_descriptor.top as usize;
                        let height = image.image_descriptor.height as usize;
                        let left = image.image_descriptor.left as usize;
                        let width = image.image_descriptor.width as usize;
                        let image_width = self.data.logical_screen_descriptor.width as usize;

                        let result = if transparent_flag {
                            result
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
                            result
                                .iter()
                                .map(|i| Some(color_table[*i]))
                                .collect::<Vec<_>>()
                        };

                        let mut new_frame = match disposal_method {
                            DisposalMethod::RestoreToBackgroundColor => ImageFrame {
                                color_values: vec![
                                    color_table[self
                                        .data
                                        .logical_screen_descriptor
                                        .background_color_index
                                        as usize];
                                    frames.last().unwrap().color_values.len()
                                ]
                                .into_boxed_slice(),
                            },
                            _ => frames.last().unwrap().clone(),
                        };

                        for y in 0..height {
                            let offset = (top + y) * image_width + left;
                            for x in 0..width {
                                let c = result[y * width + x];
                                if let Some(c) = c {
                                    new_frame.color_values[offset + x] = c;
                                }
                            }
                        }

                        frames.push(new_frame);
                    }
                }
            }
        }

        Ok(frames)
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
            v.push(i.color_values.clone());
        }

        assert_eq!(expected, v);
    }
}
