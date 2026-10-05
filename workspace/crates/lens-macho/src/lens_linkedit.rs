use crate::lens_reader::LensReader;
use crate::LensResult;

#[derive(PartialEq, Eq)]
pub struct LensEncryption {
    pub lens_cryptid: u32,
    pub lens_cryptoff: u32,
    pub lens_cryptsize: u32,
}

pub fn lens_parse_encryption(lens_body: &[u8]) -> LensResult<LensEncryption> {
    let mut lens_r = LensReader::lens_new(lens_body);
    let lens_cryptoff = lens_r.lens_read_u32()?;
    let lens_cryptsize = lens_r.lens_read_u32()?;
    let lens_cryptid = lens_r.lens_read_u32()?;
    Ok(LensEncryption {
        lens_cryptid: lens_cryptid,
        lens_cryptoff: lens_cryptoff,
        lens_cryptsize: lens_cryptsize,
    })
}

pub fn lens_parse_uuid(lens_body: &[u8]) -> LensResult<[u8; 16]> {
    let mut lens_r = LensReader::lens_new(lens_body);
    let lens_b = lens_r.lens_read_bytes(16)?;
    let mut lens_uuid = [0u8; 16];
    lens_uuid.copy_from_slice(lens_b);
    Ok(lens_uuid)
}

pub fn lens_parse_function_starts(lens_file: &[u8], lens_body: &[u8], lens_text_vmaddr: u64) -> LensResult<Vec<u64>> {
    let mut lens_r = LensReader::lens_new(lens_body);
    let lens_dataoff = lens_r.lens_read_u32()? as usize;
    let lens_datasize = lens_r.lens_read_u32()? as usize;
    let lens_end = lens_dataoff
        .checked_add(lens_datasize)
        .ok_or(crate::LensError::LensEof(lens_dataoff))?;
    if lens_end > lens_file.len() {
        return Err(crate::LensError::LensEof(lens_dataoff));
    }
    let mut lens_dr = LensReader::lens_new(&lens_file[lens_dataoff..lens_end]);
    let mut lens_addr = lens_text_vmaddr;
    let mut lens_out = Vec::new();
    while lens_dr.lens_pos() < lens_datasize {
        let lens_delta = lens_dr.lens_read_uleb128()?;
        if lens_delta == 0 {
            break;
        }
        lens_addr = lens_addr.wrapping_add(lens_delta);
        lens_out.push(lens_addr);
    }
    Ok(lens_out)
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    fn lens_linkedit_body(lens_dataoff: u32, lens_datasize: u32) -> Vec<u8> {
        let mut lens_b = Vec::new();
        lens_b.extend_from_slice(&lens_dataoff.to_le_bytes());
        lens_b.extend_from_slice(&lens_datasize.to_le_bytes());
        lens_b
    }

    #[test]
    fn lens_function_starts_accumulates_deltas_and_stops_at_zero() {
        let mut lens_file = vec![0u8; 8];
        let lens_dataoff = lens_file.len() as u32;
        lens_file.extend_from_slice(&[0x10, 0x20, 0x00]);
        let lens_body = lens_linkedit_body(lens_dataoff, 3);
        let lens_starts = lens_parse_function_starts(&lens_file, &lens_body, 0x4000).unwrap();
        assert_eq!(lens_starts, vec![0x4010, 0x4030]);
    }

    #[test]
    fn lens_function_starts_decodes_multibyte_uleb128() {
        let mut lens_file = vec![0u8; 4];
        let lens_dataoff = lens_file.len() as u32;
        lens_file.extend_from_slice(&[0x80, 0x01, 0x00]);
        let lens_body = lens_linkedit_body(lens_dataoff, 3);
        let lens_starts = lens_parse_function_starts(&lens_file, &lens_body, 0x4000).unwrap();
        assert_eq!(lens_starts, vec![0x4080]);
    }

    #[test]
    fn lens_function_starts_out_of_bounds_dataoff_errors() {
        let lens_file = vec![0u8; 8];
        let lens_body = lens_linkedit_body(1000, 10);
        assert!(lens_parse_function_starts(&lens_file, &lens_body, 0x4000).is_err());
    }

    #[test]
    fn lens_function_starts_cannot_decode_past_declared_size() {
        let lens_file = [0x80, 0x01, 0x00];
        let lens_body = lens_linkedit_body(0, 1);
        assert!(lens_parse_function_starts(&lens_file, &lens_body, 0x1000).is_err());
        let lens_body = lens_linkedit_body(0, 3);
        assert_eq!(lens_parse_function_starts(&lens_file, &lens_body, 0x1000).unwrap(), vec![0x1080]);
    }

    #[test]
    fn lens_encryption_and_uuid_parse_fields() {
        let mut lens_enc = Vec::new();
        lens_enc.extend_from_slice(&0x4000u32.to_le_bytes());
        lens_enc.extend_from_slice(&0x1000u32.to_le_bytes());
        lens_enc.extend_from_slice(&1u32.to_le_bytes());
        lens_enc.extend_from_slice(&0u32.to_le_bytes());
        let lens_e = lens_parse_encryption(&lens_enc).unwrap();
        assert_eq!(lens_e.lens_cryptid, 1);
        assert_eq!(lens_e.lens_cryptoff, 0x4000);
        assert_eq!(lens_e.lens_cryptsize, 0x1000);

        let lens_uuid = lens_parse_uuid(&[0xAB; 16]).unwrap();
        assert_eq!(lens_uuid, [0xAB; 16]);
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensEncryption { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("Encryption");lens_debug.field("cryptid", &self.lens_cryptid);lens_debug.field("cryptoff", &self.lens_cryptoff);lens_debug.field("cryptsize", &self.lens_cryptsize);lens_debug.finish() } }
