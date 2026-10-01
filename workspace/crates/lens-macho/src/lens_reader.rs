use crate::{LensError, LensResult};

pub struct LensReader<'a> {
    lens_buf: &'a [u8],
    lens_pos: usize,
}

impl<'a> LensReader<'a> {
    pub fn lens_new(lens_buf: &'a [u8]) -> LensReader<'a> {
        LensReader { lens_buf: lens_buf, lens_pos: 0 }
    }

    pub fn lens_at(lens_buf: &'a [u8], lens_pos: usize) -> LensResult<LensReader<'a>> {
        if lens_pos > lens_buf.len() {
            return Err(LensError::LensEof(lens_pos));
        }
        Ok(LensReader { lens_buf: lens_buf, lens_pos: lens_pos })
    }

    pub fn lens_pos(&self) -> usize {
        self.lens_pos
    }

    pub fn lens_seek(&mut self, lens_pos: usize) -> LensResult<()> {
        if lens_pos > self.lens_buf.len() {
            return Err(LensError::LensEof(lens_pos));
        }
        self.lens_pos = lens_pos;
        Ok(())
    }

    pub fn lens_read_bytes(&mut self, lens_n: usize) -> LensResult<&'a [u8]> {
        let lens_start = self.lens_pos;
        let lens_end = lens_start.checked_add(lens_n).ok_or(LensError::LensEof(lens_start))?;
        if lens_end > self.lens_buf.len() {
            return Err(LensError::LensEof(lens_start));
        }
        self.lens_pos = lens_end;
        Ok(&self.lens_buf[lens_start..lens_end])
    }

    pub fn lens_read_u8(&mut self) -> LensResult<u8> {
        Ok(self.lens_read_bytes(1)?[0])
    }

    pub fn lens_read_u16(&mut self) -> LensResult<u16> {
        let lens_b = self.lens_read_bytes(2)?;
        Ok(u16::from_le_bytes([lens_b[0], lens_b[1]]))
    }

    pub fn lens_read_u32(&mut self) -> LensResult<u32> {
        let lens_b = self.lens_read_bytes(4)?;
        Ok(u32::from_le_bytes([lens_b[0], lens_b[1], lens_b[2], lens_b[3]]))
    }

    pub fn lens_read_u64(&mut self) -> LensResult<u64> {
        let lens_b = self.lens_read_bytes(8)?;
        Ok(u64::from_le_bytes([
            lens_b[0], lens_b[1], lens_b[2], lens_b[3], lens_b[4], lens_b[5], lens_b[6], lens_b[7],
        ]))
    }

    pub fn lens_read_u32_be(&mut self) -> LensResult<u32> {
        let lens_b = self.lens_read_bytes(4)?;
        Ok(u32::from_be_bytes([lens_b[0], lens_b[1], lens_b[2], lens_b[3]]))
    }

    pub fn lens_read_fixed_str(&mut self, lens_n: usize) -> LensResult<String> {
        let lens_b = self.lens_read_bytes(lens_n)?;
        let lens_end = lens_b.iter().position(|&lens_c| lens_c == 0).unwrap_or(lens_b.len());
        Ok(String::from_utf8_lossy(&lens_b[..lens_end]).into_owned())
    }

    pub fn lens_read_uleb128(&mut self) -> LensResult<u64> {
        let mut lens_result: u64 = 0;
        let mut lens_shift = 0u32;
        loop {
            let lens_byte = self.lens_read_u8()?;
            if lens_shift < 64 {
                lens_result |= ((lens_byte & 0x7f) as u64) << lens_shift;
            }
            if lens_byte & 0x80 == 0 {
                break;
            }
            lens_shift += 7;
            if lens_shift > 63 {
                return Err(LensError::LensMalformed("uleb128 too long"));
            }
        }
        Ok(lens_result)
    }
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    #[test]
    fn lens_reads_little_endian_ints() {
        let mut lens_r = LensReader::lens_new(&[0x01, 0x00, 0x00, 0x00, 0xff, 0xff]);
        assert_eq!(lens_r.lens_read_u32().unwrap(), 1);
        assert_eq!(lens_r.lens_read_u16().unwrap(), 0xffff);
    }

    #[test]
    fn lens_reads_big_endian_u32() {
        let mut lens_r = LensReader::lens_new(&[0xca, 0xfe, 0xba, 0xbe]);
        assert_eq!(lens_r.lens_read_u32_be().unwrap(), 0xcafebabe);
    }

    #[test]
    fn lens_oob_read_returns_eof_not_panic() {
        let mut lens_r = LensReader::lens_new(&[0x00, 0x01]);
        assert_eq!(lens_r.lens_read_u32(), Err(LensError::LensEof(0)));
    }

    #[test]
    fn lens_fixed_str_trims_nul() {
        let mut lens_r = LensReader::lens_new(b"__TEXT\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(lens_r.lens_read_fixed_str(16).unwrap(), "__TEXT");
    }

    #[test]
    fn lens_uleb128_multibyte() {
        let mut lens_r = LensReader::lens_new(&[0xE5, 0x8E, 0x26]);
        assert_eq!(lens_r.lens_read_uleb128().unwrap(), 624485);
    }
}
