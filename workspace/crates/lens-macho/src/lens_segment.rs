use crate::lens_reader::LensReader;
use crate::LensResult;

#[derive(PartialEq, Eq)]
pub struct LensSection {
    pub lens_sectname: String,
    pub lens_segname: String,
    pub lens_addr: u64,
    pub lens_size: u64,
    pub lens_offset: u32,
    pub lens_flags: u32,
}

#[derive(PartialEq, Eq)]
pub struct LensSegment {
    pub lens_segname: String,
    pub lens_vmaddr: u64,
    pub lens_vmsize: u64,
    pub lens_fileoff: u64,
    pub lens_filesize: u64,
    pub lens_sections: Vec<LensSection>,
}

pub fn lens_parse_segment(lens_body: &[u8]) -> LensResult<LensSegment> {
    let mut lens_r = LensReader::lens_new(lens_body);
    let lens_segname = lens_r.lens_read_fixed_str(16)?;
    let lens_vmaddr = lens_r.lens_read_u64()?;
    let lens_vmsize = lens_r.lens_read_u64()?;
    let lens_fileoff = lens_r.lens_read_u64()?;
    let lens_filesize = lens_r.lens_read_u64()?;
    let lens__maxprot = lens_r.lens_read_u32()?;
    let lens__initprot = lens_r.lens_read_u32()?;
    let lens_nsects = lens_r.lens_read_u32()?;
    let lens__flags = lens_r.lens_read_u32()?;

    let mut lens_sections = Vec::new();
    for _ in 0..lens_nsects {
        let lens_sectname = lens_r.lens_read_fixed_str(16)?;
        let lens_sect_segname = lens_r.lens_read_fixed_str(16)?;
        let lens_addr = lens_r.lens_read_u64()?;
        let lens_size = lens_r.lens_read_u64()?;
        let lens_offset = lens_r.lens_read_u32()?;
        let lens__align = lens_r.lens_read_u32()?;
        let lens__reloff = lens_r.lens_read_u32()?;
        let lens__nreloc = lens_r.lens_read_u32()?;
        let lens_flags = lens_r.lens_read_u32()?;
        let lens__r1 = lens_r.lens_read_u32()?;
        let lens__r2 = lens_r.lens_read_u32()?;
        let lens__r3 = lens_r.lens_read_u32()?;
        lens_sections.push(LensSection {
            lens_sectname: lens_sectname,
            lens_segname: lens_sect_segname,
            lens_addr: lens_addr,
            lens_size: lens_size,
            lens_offset: lens_offset,
            lens_flags: lens_flags,
        });
    }
    Ok(LensSegment {
        lens_segname: lens_segname,
        lens_vmaddr: lens_vmaddr,
        lens_vmsize: lens_vmsize,
        lens_fileoff: lens_fileoff,
        lens_filesize: lens_filesize,
        lens_sections: lens_sections,
    })
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    fn lens_seg_body_one_section() -> Vec<u8> {
        let mut lens_b = Vec::new();
        let mut lens_segname = b"__TEXT".to_vec();
        lens_segname.resize(16, 0);
        lens_b.extend_from_slice(&lens_segname);
        lens_b.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_b.extend_from_slice(&0x4000u64.to_le_bytes());
        lens_b.extend_from_slice(&0u64.to_le_bytes());
        lens_b.extend_from_slice(&0x4000u64.to_le_bytes());
        lens_b.extend_from_slice(&5u32.to_le_bytes());
        lens_b.extend_from_slice(&5u32.to_le_bytes());
        lens_b.extend_from_slice(&1u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        let mut lens_sectname = b"__text".to_vec();
        lens_sectname.resize(16, 0);
        lens_b.extend_from_slice(&lens_sectname);
        lens_b.extend_from_slice(&lens_segname);
        lens_b.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_b.extend_from_slice(&0x400u64.to_le_bytes());
        lens_b.extend_from_slice(&0x1000u32.to_le_bytes());
        lens_b.extend_from_slice(&2u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b.extend_from_slice(&0x80000400u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b
    }

    #[test]
    fn lens_parses_segment_and_section() {
        let lens_body = lens_seg_body_one_section();
        let lens_seg = lens_parse_segment(&lens_body).unwrap();
        assert_eq!(lens_seg.lens_segname, "__TEXT");
        assert_eq!(lens_seg.lens_vmaddr, 0x1000);
        assert_eq!(lens_seg.lens_sections.len(), 1);
        assert_eq!(lens_seg.lens_sections[0].lens_sectname, "__text");
        assert_eq!(lens_seg.lens_sections[0].lens_offset, 0x1000);
    }

    #[test]
    fn lens_huge_nsects_with_tiny_body_errors_fast() {
        let mut lens_b = Vec::new();
        let mut lens_segname = b"__DATA".to_vec();
        lens_segname.resize(16, 0);
        lens_b.extend_from_slice(&lens_segname);
        lens_b.extend_from_slice(&0u64.to_le_bytes());
        lens_b.extend_from_slice(&0u64.to_le_bytes());
        lens_b.extend_from_slice(&0u64.to_le_bytes());
        lens_b.extend_from_slice(&0u64.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        assert!(lens_parse_segment(&lens_b).is_err());
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensSection { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("Section");lens_debug.field("sectname", &self.lens_sectname);lens_debug.field("segname", &self.lens_segname);lens_debug.field("addr", &self.lens_addr);lens_debug.field("size", &self.lens_size);lens_debug.field("offset", &self.lens_offset);lens_debug.field("flags", &self.lens_flags);lens_debug.finish() } }
impl std::fmt::Debug for LensSegment { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("Segment");lens_debug.field("segname", &self.lens_segname);lens_debug.field("vmaddr", &self.lens_vmaddr);lens_debug.field("vmsize", &self.lens_vmsize);lens_debug.field("fileoff", &self.lens_fileoff);lens_debug.field("filesize", &self.lens_filesize);lens_debug.field("sections", &self.lens_sections);lens_debug.finish() } }
