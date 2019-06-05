use crate::util::Color;

use std::io::Read;
use std::mem;

#[derive(Debug)]
pub(crate) struct Header {
    pub(crate) sig: String,
    pub(crate) version: String,
}

#[derive(Debug)]
pub(crate) struct LogicalScreenDescriptor {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) global_color_table_flag: bool,
    pub(crate) color_resolution: u8,
    pub(crate) sort_flag: bool,
    pub(crate) global_color_table_size: u8,
    pub(crate) background_color_index: u8,
    pub(crate) pixel_aspect_ratio: f32,
    pub(crate) global_color_table: Option<Vec<Color>>,
}

#[derive(Debug)]
pub(crate) enum BlockType {
    TableBasedImage,
    Extension(ExtensionType),
    Trailer,
    Unknown(u8),
}

#[derive(Debug)]
pub(crate) enum ExtensionType {
    ApplicationExtension,
    CommentExtension,
    GraphicControlExtension,
    PlainTextExtension,
    Unknown(u8),
}

#[derive(Debug)]
pub(crate) enum DataType {
    ApplicationExtensionType(ApplicationExtension),
    CommentExtensionType(CommentExtension),
    GraphicControlExtensionType(GraphicControlExtension),
    PlainTextExtensionType(PlainTextExtension),
    TableBasedImageType(TableBasedImage),
}

#[derive(Debug)]
pub(crate) struct GraphicControlExtension {
    pub(crate) disposal_method: DisposalMethod,
    pub(crate) user_input_expected: bool,
    pub(crate) transparent_color_index_available: bool,
    pub(crate) delay_time: u16,
    pub(crate) transparent_color_index: u8,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum DisposalMethod {
    Unspecified,
    DoNotDispose,
    RestoreToBackgroundColor,
    RestoreToPrevious,
    Undefined,
}

#[derive(Debug)]
pub(crate) struct TableBasedImage {
    pub(crate) image_descriptor: ImageDescriptor,
    pub(crate) local_color_table: Option<Vec<Color>>,
    pub(crate) image_data: ImageData,
}

#[derive(Debug)]
pub(crate) struct ImageDescriptor {
    pub(crate) left: u16,
    pub(crate) top: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) local_color_table_flag: bool,
    pub(crate) interlace_flag: bool,
    pub(crate) sort_flag: bool,
    pub(crate) local_color_table_size: u8,
}

#[derive(Debug)]
pub(crate) struct ImageData {
    pub(crate) lzw_min_code_size: u8,
    pub(crate) data_sub_blocks: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct PlainTextExtension {
    pub(crate) text_grid_left_pos: u16,
    pub(crate) text_grid_top_pos: u16,
    pub(crate) text_grid_width: u16,
    pub(crate) text_grid_height: u16,
    pub(crate) char_cell_width: u8,
    pub(crate) char_cell_height: u8,
    pub(crate) text_fg_color_index: u8,
    pub(crate) text_bg_color_index: u8,
    pub(crate) plain_text_data: String,
}

#[derive(Debug)]
pub(crate) struct ApplicationExtension {
    pub(crate) id: String,
    pub(crate) auth_code: String,
    pub(crate) data_sub_blocks: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct CommentExtension {
    pub(crate) text: String,
}

#[derive(Debug)]
pub(crate) struct ParseResult {
    pub(crate) header: Header,
    pub(crate) logical_screen_descriptor: LogicalScreenDescriptor,
    pub(crate) data_blocks: Vec<DataType>,
}

#[derive(Debug)]
pub(crate) struct Parser<'a, T: Read> {
    src: &'a mut T,
}

impl<'a, T: Read> Parser<'a, T> {
    pub(crate) fn new(src: &'a mut T) -> Self {
        Self { src }
    }

    pub(crate) fn parse(&mut self) -> Result<ParseResult, String> {
        let header = self.read_header()?;
        if header.sig != "GIF" {
            return Err("Error: file is not a GIF".into());
        }

        let logical_screen_descriptor = self.read_logical_screen_descriptor()?;

        let mut data_blocks = Vec::new();
        loop {
            match self.read_block_type()? {
                BlockType::TableBasedImage => {
                    let table_based_image = self.read_table_based_image()?;
                    data_blocks.push(DataType::TableBasedImageType(table_based_image));
                }

                BlockType::Extension(extension_type) => match extension_type {
                    ExtensionType::ApplicationExtension => {
                        let ext = self.read_application_extension()?;
                        data_blocks.push(DataType::ApplicationExtensionType(ext));
                    }

                    ExtensionType::CommentExtension => {
                        let ext = self.read_comment_extension()?;
                        data_blocks.push(DataType::CommentExtensionType(ext));
                    }

                    ExtensionType::GraphicControlExtension => {
                        let ext = self.read_graphic_control_extension()?;
                        data_blocks.push(DataType::GraphicControlExtensionType(ext));
                    }

                    ExtensionType::PlainTextExtension => {
                        let ext = self.read_plain_text_extension()?;
                        data_blocks.push(DataType::PlainTextExtensionType(ext));
                    }

                    ExtensionType::Unknown(x) => {
                        return Err(format!("Error: unknown extension type: {:x}", x));
                    }
                },

                BlockType::Trailer => break,

                BlockType::Unknown(x) => {
                    return Err(format!("Error: unknown block type: {:x}", x));
                }
            }
        }

        Ok(ParseResult {
            header,
            logical_screen_descriptor,
            data_blocks,
        })
    }

    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<(), String> {
        self.src
            .read_exact(buffer)
            .map_err(|e| format!("Error: {}", e))
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        let mut buffer = [0u8; 1];
        self.read_bytes(&mut buffer)?;
        Ok(buffer[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let mut buffer = [0u8; 2];
        self.read_bytes(&mut buffer)?;
        Ok(unsafe { mem::transmute(buffer) })
    }

    fn read_block_type(&mut self) -> Result<BlockType, String> {
        match self.read_u8()? {
            0x2c => Ok(BlockType::TableBasedImage),
            0x21 => {
                let extension_type = match self.read_u8()? {
                    0xf9 => ExtensionType::GraphicControlExtension,
                    0xfe => ExtensionType::CommentExtension,
                    0x01 => ExtensionType::PlainTextExtension,
                    0xff => ExtensionType::ApplicationExtension,
                    x => ExtensionType::Unknown(x),
                };
                Ok(BlockType::Extension(extension_type))
            }
            0x3b => Ok(BlockType::Trailer),
            x => Ok(BlockType::Unknown(x)),
        }
    }

    fn read_header(&mut self) -> Result<Header, String> {
        let mut buffer = [0u8; 6];
        self.read_bytes(&mut buffer)?;

        let sig = std::str::from_utf8(&buffer[0..3])
            .map(|s| s.into())
            .map_err(|e| format!("Error: {}", e))?;

        let version = std::str::from_utf8(&buffer[3..])
            .map(|s| s.into())
            .map_err(|e| format!("Error: {}", e))?;

        Ok(Header { sig, version })
    }

    fn read_logical_screen_descriptor(&mut self) -> Result<LogicalScreenDescriptor, String> {
        let mut lsd = LogicalScreenDescriptor {
            width: 0,
            height: 0,
            global_color_table_flag: false,
            color_resolution: 0,
            sort_flag: false,
            global_color_table_size: 0,
            background_color_index: 0,
            pixel_aspect_ratio: 0f32,
            global_color_table: None,
        };

        lsd.width = self.read_u16()?;
        lsd.height = self.read_u16()?;

        /**
         * Global Color Table Flag       1 Bit
         * Color Resolution              3 Bits
         * Sort Flag                     1 Bit
         * Size of Global Color Table    3 Bits
         */
        let packed_fields = self.read_u8()?;
        lsd.global_color_table_flag = (packed_fields >> 7) == 1;
        lsd.color_resolution = (packed_fields & 0b0111_0000) >> 4;
        lsd.sort_flag = ((packed_fields & 0b0000_1000) >> 3) == 1;
        lsd.global_color_table_size = packed_fields & 0b0000_0111;

        lsd.background_color_index = self.read_u8()?;
        lsd.pixel_aspect_ratio = {
            let val = self.read_u8()?;
            if val == 0 {
                val as f32
            } else {
                (val as f32 + 15.0f32) / 64.0f32
            }
        };

        if lsd.global_color_table_flag {
            let size = 3 * (1 << (lsd.global_color_table_size + 1));
            let mut table = vec![0u8; size];
            self.read_bytes(&mut table)?;

            let global_color_table = table
                .chunks_exact(3)
                .map(|a| Color(a[0], a[1], a[2]))
                .collect::<Vec<_>>();
            lsd.global_color_table = Some(global_color_table);
        }

        Ok(lsd)
    }

    fn read_image_descriptor(&mut self) -> Result<ImageDescriptor, String> {
        let mut image_desc = ImageDescriptor {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            local_color_table_flag: false,
            interlace_flag: false,
            sort_flag: false,
            local_color_table_size: 0,
        };

        image_desc.left = self.read_u16()?;
        image_desc.top = self.read_u16()?;
        image_desc.width = self.read_u16()?;
        image_desc.height = self.read_u16()?;

        let packed_fields = self.read_u8()?;
        image_desc.local_color_table_flag = (packed_fields >> 7) == 1;
        image_desc.interlace_flag = ((packed_fields & 0b0100_0000) >> 6) == 1;
        image_desc.sort_flag = ((packed_fields & 0b0010_0000) >> 5) == 1;
        image_desc.local_color_table_size = packed_fields & 0b0000_0111;

        Ok(image_desc)
    }

    fn read_table_based_image(&mut self) -> Result<TableBasedImage, String> {
        let image_descriptor = self.read_image_descriptor()?;
        let local_color_table = if image_descriptor.local_color_table_flag {
            let size = 3 * (1 << (image_descriptor.local_color_table_size + 1));
            let mut table = vec![0u8; size];
            self.read_bytes(&mut table)?;
            let table = table
                .chunks_exact(3)
                .map(|a| Color(a[0], a[1], a[2]))
                .collect::<Vec<_>>();
            Some(table)
        } else {
            None
        };

        let lzw_min_code_size = self.read_u8()?;
        let data_sub_blocks = self.read_data_sub_blocks()?;

        Ok(TableBasedImage {
            image_descriptor,
            local_color_table,
            image_data: ImageData {
                lzw_min_code_size,
                data_sub_blocks,
            },
        })
    }

    fn read_data_sub_blocks(&mut self) -> Result<Vec<u8>, String> {
        let mut sub_blocks = Vec::new();

        loop {
            let block_size = self.read_u8()?;

            // Block terminator value is 0x00
            if block_size == 0 {
                break;
            }

            let mut data = vec![0u8; block_size as usize];
            self.read_bytes(&mut data)?;

            sub_blocks.append(&mut data);
        }

        Ok(sub_blocks)
    }

    fn read_application_extension(&mut self) -> Result<ApplicationExtension, String> {
        let block_size = self.read_u8()?;
        if block_size != 11 {
            return Err(format!(
                "Error: invalid Application Extension block size: {}",
                block_size
            ));
        }

        let id = {
            let mut buffer = [0u8; 8];
            self.read_bytes(&mut buffer)?;
            std::str::from_utf8(&buffer).unwrap().into()
        };

        let auth_code = {
            let mut buffer = [0u8; 3];
            self.read_bytes(&mut buffer)?;
            std::str::from_utf8(&buffer).unwrap().into()
        };

        let data_sub_blocks = self.read_data_sub_blocks()?;

        Ok(ApplicationExtension {
            id,
            auth_code,
            data_sub_blocks,
        })
    }

    fn read_comment_extension(&mut self) -> Result<CommentExtension, String> {
        let data = self.read_data_sub_blocks()?;
        let text = String::from_utf8(data).map_err(|e| format!("Error: {}", e))?;
        Ok(CommentExtension { text })
    }

    fn read_graphic_control_extension(&mut self) -> Result<GraphicControlExtension, String> {
        let block_size = self.read_u8()?;
        if block_size != 4 {
            return Err(format!(
                "Error: invalid Graphic Control Extension block size: {}",
                block_size
            ));
        }

        let packed_fields = self.read_u8()?;
        let disposal_method = match (packed_fields & 0b0001_1100) >> 2 {
            0 => DisposalMethod::Unspecified,
            1 => DisposalMethod::DoNotDispose,
            2 => DisposalMethod::RestoreToBackgroundColor,
            3 => DisposalMethod::RestoreToPrevious,
            4...7 => DisposalMethod::Undefined,
            x => {
                return Err(format!("Error: invalid disposal method: {}", x));
            }
        };

        let user_input_expected = ((packed_fields & 0b0000_0010) >> 1) == 1;

        let transparent_color_index_available = (packed_fields & 0b0000_0001) == 1;
        let delay_time = self.read_u16()?;
        let transparent_color_index = self.read_u8()?;

        if self.read_u8()? != 0 {
            return Err("Error: block terminator not found for Graphic Control Extension".into());
        }

        Ok(GraphicControlExtension {
            disposal_method,
            user_input_expected,
            transparent_color_index_available,
            delay_time,
            transparent_color_index,
        })
    }

    fn read_plain_text_extension(&mut self) -> Result<PlainTextExtension, String> {
        let block_size = self.read_u8()?;
        if block_size != 12 {
            return Err(format!(
                "Error: invalid Plain Text Extension block size: {}",
                block_size
            ));
        }

        let text_grid_left_pos = self.read_u16()?;
        let text_grid_top_pos = self.read_u16()?;
        let text_grid_width = self.read_u16()?;
        let text_grid_height = self.read_u16()?;

        let char_cell_width = self.read_u8()?;
        let char_cell_height = self.read_u8()?;
        let text_fg_color_index = self.read_u8()?;
        let text_bg_color_index = self.read_u8()?;

        let data = self.read_data_sub_blocks()?;
        let plain_text_data = String::from_utf8(data).map_err(|e| format!("Error: {}", e))?;

        return Ok(PlainTextExtension {
            text_grid_left_pos,
            text_grid_top_pos,
            text_grid_width,
            text_grid_height,
            char_cell_width,
            char_cell_height,
            text_fg_color_index,
            text_bg_color_index,
            plain_text_data,
        });
    }
}
