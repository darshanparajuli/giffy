pub(crate) struct Decompressor<'a> {
    data_sub_blocks: &'a [u8],
    lzw_min_code_size: u8,
    clear_code: usize,
    code_values: Vec<usize>,
    code_table: Vec<CodeValue>,
    code_size: u8,
}

// Refer to https://www.w3.org/Graphics/GIF/spec-gif89a.txt for details.
impl<'a> Decompressor<'a> {
    pub(crate) fn new(data_sub_blocks: &'a [u8], lzw_min_code_size: u8) -> Self {
        Self {
            data_sub_blocks,
            lzw_min_code_size,
            clear_code: 1 << lzw_min_code_size,
            code_values: vec![],
            code_table: vec![],
            code_size: lzw_min_code_size + 1,
        }
    }

    fn reset(&mut self) {
        self.code_size = self.lzw_min_code_size + 1;

        self.code_table.clear();
        self.code_values.clear();

        for i in 0..self.clear_code {
            self.code_values.push(i);
            self.code_table.push(CodeValue::Range(
                self.code_values.len() - 1,
                self.code_values.len(),
            ));
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

        if let Some(CodeValue::Range(begin, end)) = &self.code_table.get(current as usize) {
            for i in &self.code_values[*begin..*end] {
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

            if (current as usize) < self.code_table.len() {
                match &self.code_table[current as usize] {
                    CodeValue::Range(begin, end) => {
                        for i in &self.code_values[*begin..*end] {
                            result.push(*i);
                        }

                        let k = self.code_values[*begin];
                        if let CodeValue::Range(begin, end) = &self.code_table[prev as usize] {
                            let new_begin = self.code_values.len();
                            for i in *begin..*end {
                                self.code_values.push(self.code_values[i]);
                            }
                            self.code_values.push(k);
                            let new_end = self.code_values.len();

                            if self.code_table.len() == (1 << self.code_size) - 1 {
                                if self.code_size == 12 {
                                    self.expect_clear_code(code_reader)?;
                                    return Ok(true);
                                } else {
                                    self.code_size += 1;
                                    self.code_table.push(CodeValue::Range(new_begin, new_end));
                                }
                            } else {
                                self.code_table.push(CodeValue::Range(new_begin, new_end));
                            }
                        } else {
                            return Err(format!("Invalid prev code type {}", prev));
                        }
                    }

                    CodeValue::Single(c) => {
                        if *c == self.clear_code {
                            return Ok(true);
                        } else if *c == self.clear_code + 1 {
                            return Ok(false);
                        } else {
                            return Err(format!("Invalid single code {}", c));
                        }
                    }
                }
            } else {
                if let CodeValue::Range(begin, end) = &self.code_table[prev as usize] {
                    let new_begin = self.code_values.len();
                    for i in *begin..*end {
                        self.code_values.push(self.code_values[i]);
                    }

                    let k = self.code_values[*begin];
                    self.code_values.push(k);
                    let new_end = self.code_values.len();

                    for i in &self.code_values[new_begin..new_end] {
                        result.push(*i);
                    }

                    if self.code_table.len() == (1 << self.code_size) - 1 {
                        if self.code_size == 12 {
                            self.expect_clear_code(code_reader)?;
                            return Ok(true);
                        } else {
                            self.code_size += 1;
                            self.code_table.push(CodeValue::Range(new_begin, new_end));
                        }
                    } else {
                        self.code_table.push(CodeValue::Range(new_begin, new_end));
                    }
                } else {
                    return Err(format!("Invalid prev code: {}", prev));
                }
            }

            prev = current;
        }
    }

    fn expect_clear_code(&self, code_reader: &mut CodeReader) -> Result<(), String> {
        if let Some(c) = code_reader.read(self.code_size) {
            if c as usize != self.clear_code {
                return Err(format!(
                    "Invalid clear code {}, expected: {}",
                    c, self.code_size
                ));
            }
        } else {
            return Err(format!("Missing clear code {}", self.clear_code));
        }

        Ok(())
    }

    pub(crate) fn decompress(&mut self) -> Result<Vec<usize>, String> {
        let mut result = vec![];

        let mut code_reader = CodeReader::new(self.data_sub_blocks);
        self.expect_clear_code(&mut code_reader)?;

        loop {
            self.reset();
            if !self.decompress_until_clear(&mut code_reader, &mut result)? {
                break;
            }
        }

        Ok(result)
    }
}

#[derive(Debug)]
enum CodeValue {
    Range(usize, usize),
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
    use crate::util::Color;

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

        let mut decompressor = Decompressor::new(&input, 2);
        let actual = decompressor
            .decompress()
            .unwrap()
            .iter()
            .map(|i| color_table[*i])
            .collect::<Vec<_>>();

        assert_eq!(expected, actual);
    }
}
