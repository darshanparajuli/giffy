#![allow(unused_doc_comments)]

mod decompressor;
mod parser;
mod util;

use decompressor::Decompressor;
pub use util::Color;

use parser::*;

use std::fs::File;

pub fn load(file_name: &str) -> Result<Gif, String> {
    let mut file = File::open(file_name).map_err(|e| format!("Error: {}", e))?;

    let mut parser = Parser::new(&mut file);
    let result = parser.parse()?;

    println!("{:?}", result.logical_screen_descriptor);

    let decoder = Decoder::new(&result);
    let frames = decoder.decode()?;

    Ok(Gif {
        image_frames: frames,
        width: result.logical_screen_descriptor.width as u32,
        height: result.logical_screen_descriptor.height as u32,
    })
}

#[derive(Debug)]
pub struct Gif {
    pub width: u32,
    pub height: u32,
    pub image_frames: Vec<ImageFrame>,
}

#[derive(Debug, Clone)]
pub struct ImageFrame {
    pub color_values: Box<[Color]>,
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
            match block {
                DataType::ApplicationExtensionType(_) => {}
                DataType::CommentExtensionType(_) => {}
                DataType::GraphicControlExtensionType(_) => {}
                DataType::PlainTextExtensionType(_) => {}
                DataType::TableBasedImageType(image) => {
                    let table = {
                        if image.local_color_table.is_some() {
                            image.local_color_table.as_ref()
                        } else {
                            self.data
                                .logical_screen_descriptor
                                .global_color_table
                                .as_ref()
                        }
                    };

                    // if color_table.is_none() {
                    //     color_table.replace(table.expect("color table is missing"));
                    // } else {
                    //     if table.is_some() {
                    //         color_table.replace(table.expect("color table is missing"));
                    //     }
                    // }

                    let color_table = table.unwrap();
                    let mut decompressor = Decompressor::new(
                        &image.image_data.data_sub_blocks,
                        &color_table,
                        image.image_data.lzw_min_code_size,
                    );

                    let result = decompressor.decompress()?;
                    println!("result len: {}", result.len());

                    if frames.is_empty() {
                        frames.push(ImageFrame {
                            color_values: result.into_boxed_slice(),
                        });
                    } else {
                        let top = image.image_descriptor.top as usize;
                        let left = image.image_descriptor.left as usize;
                        let image_width = self.data.logical_screen_descriptor.width as usize;

                        let mut new_frame = frames.last().unwrap().clone();
                        println!("last len: {}", new_frame.color_values.len());

                        let offset = top * image_width + left;
                        for i in 0..result.len() {
                            new_frame.color_values[offset + i] = result[i];
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
    use std::io::prelude::*;

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
