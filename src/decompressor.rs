use crate::util::Color;

pub(crate) struct Decompressor<'a> {
    data_sub_blocks: &'a [u8],
    color_table: &'a [Color],
    lzw_min_code_size: u8,
    clear_code: usize,
    code_table: Vec<CodeValue>,
    code_size: u8,
}

impl<'a> Decompressor<'a> {
    pub(crate) fn new(
        data_sub_blocks: &'a [u8],
        color_table: &'a [Color],
        lzw_min_code_size: u8,
    ) -> Self {
        Self {
            data_sub_blocks,
            color_table,
            lzw_min_code_size,
            clear_code: color_table.len(),
            code_table: Vec::new(),
            code_size: lzw_min_code_size + 1,
        }
    }

    fn reset(&mut self) {
        self.code_size = self.lzw_min_code_size + 1;

        self.code_table.clear();
        for i in 0..self.color_table.len() {
            self.code_table.push(CodeValue::Indices(vec![i]));
        }

        self.code_table.push(CodeValue::Single(self.clear_code));
        self.code_table.push(CodeValue::Single(self.clear_code + 1));
    }

    fn decompress_until_clear(
        &mut self,
        code_reader: &mut CodeReader,
        result: &mut Vec<usize>,
    ) -> Result<bool, String> {
        let current;
        if let Some(c) = code_reader.read(self.code_size) {
            current = c;
        } else {
            return Ok(false);
        }

        if let Some(CodeValue::Indices(indices)) = &self.code_table.get(current as usize) {
            for i in indices {
                result.push(*i);
            }
        } else {
            return Err(format!("Invalid code: {}", current));
        }

        let mut prev = current;

        loop {
            let current;
            if let Some(c) = code_reader.read(self.code_size) {
                current = c;
            } else {
                return Ok(false);
            }

            // println!(
            //     "clear_code: {}, code_size: {}, prev: {:?}, current: {}, table size: {}",
            //     self.clear_code,
            //     self.code_size,
            //     prev,
            //     current,
            //     self.code_table.len()
            // );

            if (current as usize) < self.code_table.len() {
                match &self.code_table[current as usize] {
                    CodeValue::Indices(indices) => {
                        for i in indices.iter() {
                            result.push(*i);
                        }

                        let k = indices[0];
                        if let CodeValue::Indices(prev_indices) = &self.code_table[prev as usize] {
                            let mut new_indices = vec![];
                            for i in prev_indices.iter() {
                                new_indices.push(*i);
                            }
                            new_indices.push(k);

                            // println!(
                            //     "code size: {}, table size: {}",
                            //     self.code_size,
                            //     self.code_table.len()
                            // );

                            if self.code_table.len() == (1 << self.code_size) - 1 {
                                if self.code_size == 12 {
                                    self.expect_clear_code(code_reader)?;
                                    return Ok(true);
                                } else {
                                    self.code_size += 1;
                                    self.code_table.push(CodeValue::Indices(new_indices));
                                }
                            } else {
                                self.code_table.push(CodeValue::Indices(new_indices));
                            }
                        } else {
                            return Err(format!("Invalid prev code type {}", prev));
                        }
                    }
                    CodeValue::Single(c) => {
                        if *c == self.clear_code {
                            return Ok(true);
                        } else if *c == self.clear_code + 1 {
                            break;
                        } else {
                            return Err(format!("Invalid single code {}", c));
                        }
                    }
                }
            } else {
                if let CodeValue::Indices(indices) = &self.code_table[prev as usize] {
                    let mut output = vec![];
                    for i in indices.iter() {
                        output.push(*i);
                    }

                    let k = indices[0];
                    output.push(k);

                    for i in output.iter() {
                        result.push(*i);
                    }

                    // println!(
                    //     "code size: {}, table size: {}",
                    //     self.code_size,
                    //     self.code_table.len()
                    // );

                    if self.code_table.len() == (1 << self.code_size) - 1 {
                        if self.code_size == 12 {
                            self.expect_clear_code(code_reader)?;
                            return Ok(true);
                        } else {
                            self.code_size += 1;
                            self.code_table.push(CodeValue::Indices(output));
                        }
                    } else {
                        self.code_table.push(CodeValue::Indices(output));
                    }
                } else {
                    return Err(format!("Invalid prev code: {}", prev));
                }
            }

            prev = current;
        }

        Ok(false)
    }

    fn expect_clear_code(&self, code_reader: &mut CodeReader) -> Result<(), String> {
        if let Some(c) = code_reader.read(self.code_size) {
            if c as usize != self.clear_code {
                return Err(format!(
                    "Invalid clear code {}, expected: {}",
                    c, self.clear_code
                ));
            }
        } else {
            return Err(format!("Missing clear code {}", self.clear_code));
        }

        Ok(())
    }

    pub(crate) fn decompress(&mut self) -> Result<Vec<Color>, String> {
        let mut result = vec![];

        self.reset();

        let mut code_reader = CodeReader::new(self.data_sub_blocks);
        self.expect_clear_code(&mut code_reader)?;

        while self.decompress_until_clear(&mut code_reader, &mut result)? {
            self.reset();
        }

        // println!("result count: {}", result.len());

        Ok(result
            .iter()
            .map(|i| self.color_table[*i])
            .collect::<Vec<_>>())
    }
}

#[derive(Debug)]
enum CodeValue {
    Indices(Vec<usize>),
    Single(usize),
}

struct CodeReader<'a> {
    data: &'a [u8],
    index: usize,
    remaining_bits: u8,
}

impl<'a> CodeReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            index: 0,
            remaining_bits: 8,
        }
    }

    fn read(&mut self, mut bits: u8) -> Option<u16> {
        if self.index >= self.data.len() {
            return None;
        }

        let mut result = 0u16;
        let mut acc = 0;
        let mut byte: u8 = self.data[self.index] >> (8 - self.remaining_bits);

        loop {
            if bits >= self.remaining_bits {
                let mask = if self.remaining_bits == 8 {
                    !0
                } else {
                    !(!0u8 << self.remaining_bits)
                };

                result |= ((byte & mask) as u16) << acc;

                acc += self.remaining_bits;
                bits -= self.remaining_bits;

                self.remaining_bits = 8;
                self.index += 1;

                if self.index < self.data.len() {
                    byte = self.data[self.index];
                } else {
                    if bits > 0 {
                        return None;
                    }
                }
            } else {
                if bits != 0 {
                    result |= ((byte & !(!0u8 << bits)) as u16) << acc;
                    self.remaining_bits -= bits;
                }

                break;
            }
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_reader_read() {
        let data = vec![
            0b01011101, 0b01011101, 0b01011101, 0b01011101, 0b01011101, 0b11110101, 0b10110110,
            0b01100110, 0b10110110, 0b01100110, 0b01010100,
        ];

        let mut cr = CodeReader::new(&data);

        assert_eq!(Some(0b101), cr.read(3));
        assert_eq!(Some(0b011), cr.read(3));
        assert_eq!(Some(0b101), cr.read(3));
        assert_eq!(Some(0b1110), cr.read(4));
        assert_eq!(Some(0b1010), cr.read(4));
        assert_eq!(Some(0b0101110), cr.read(7));
        assert_eq!(Some(0b01011101), cr.read(8));
        assert_eq!(Some(0b01011101), cr.read(8));
        assert_eq!(Some(0b11110101), cr.read(8));
        assert_eq!(Some(0b0110011010110110), cr.read(16));
        assert_eq!(Some(0b110), cr.read(3));
        assert_eq!(Some(0b011010110), cr.read(9));
        assert_eq!(Some(0b010101000110), cr.read(12));
    }

    #[test]
    fn test_decompressor_decompress() {
        let input = vec![
            140, 45, 153, 135, 42, 28, 220, 51, 160, 2, 117, 236, 149, 250, 168, 222, 96, 140, 4,
            145, 76, 1,
        ];

        let expected = vec![
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
        ];

        let color_table = vec![
            Color(255, 255, 255),
            Color(255, 0, 0),
            Color(0, 0, 255),
            Color(0, 0, 0),
        ];
        let mut decompressor = Decompressor::new(&input, &color_table, 2);
        let actual = decompressor.decompress().unwrap();
        assert_eq!(expected, actual);
    }
}
