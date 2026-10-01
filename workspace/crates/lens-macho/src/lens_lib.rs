pub mod lens_chained_fixups;
pub mod lens_consts;
pub mod lens_dyld_info;
pub mod lens_fat;
pub mod lens_header;
pub mod lens_image;
pub mod lens_linkedit;
pub mod lens_reader;
pub mod lens_segment;
pub mod lens_symtab;

pub use lens_image::LensMachOImage;

#[derive(thiserror::Error, PartialEq, Eq)]
pub enum LensError {
    #[error("unexpected end of data at offset {0}")]
    LensEof(usize),
    #[error("bad magic: 0x{0:08x}")]
    LensBadMagic(u32),
    #[error("no arm64/arm64e slice found")]
    LensNoArm64Slice,
    #[error("malformed structure: {0}")]
    LensMalformed(&'static str),
}

pub type LensResult<LENS_T> = core::result::Result<LENS_T, LensError>;

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensError { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self {Self::LensEof(lens_slot0) => { let mut lens_debug = lens_formatter.debug_tuple("Eof");lens_debug.field(lens_slot0);lens_debug.finish() },Self::LensBadMagic(lens_slot0) => { let mut lens_debug = lens_formatter.debug_tuple("BadMagic");lens_debug.field(lens_slot0);lens_debug.finish() },Self::LensNoArm64Slice => { lens_formatter.write_str("NoArm64Slice") },Self::LensMalformed(lens_slot0) => { let mut lens_debug = lens_formatter.debug_tuple("Malformed");lens_debug.field(lens_slot0);lens_debug.finish() }} } }
