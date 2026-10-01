pub const LENS_FAT_MAGIC: u32 = 0xcafebabe;
pub const LENS_FAT_MAGIC_64: u32 = 0xcafebabf;

pub const LENS_MH_MAGIC_64: u32 = 0xfeedfacf;

pub const LENS_CPU_ARCH_ABI64: u32 = 0x0100_0000;
pub const LENS_CPU_TYPE_ARM: u32 = 12;
pub const LENS_CPU_TYPE_ARM64: u32 = LENS_CPU_TYPE_ARM | LENS_CPU_ARCH_ABI64;
pub const LENS_CPU_SUBTYPE_MASK: u32 = 0xff00_0000;
pub const LENS_CPU_SUBTYPE_ARM64_ALL: u32 = 0;
pub const LENS_CPU_SUBTYPE_ARM64E: u32 = 2;

pub const LENS_LC_REQ_DYLD: u32 = 0x8000_0000;
pub const LENS_LC_SEGMENT_64: u32 = 0x19;
pub const LENS_LC_SYMTAB: u32 = 0x02;
pub const LENS_LC_UUID: u32 = 0x1b;
pub const LENS_LC_FUNCTION_STARTS: u32 = 0x26;
pub const LENS_LC_ENCRYPTION_INFO_64: u32 = 0x2c;
pub const LENS_LC_DYLD_INFO: u32 = 0x22;
pub const LENS_LC_DYLD_INFO_ONLY: u32 = 0x22 | LENS_LC_REQ_DYLD;
pub const LENS_LC_DYLD_CHAINED_FIXUPS: u32 = 0x34 | LENS_LC_REQ_DYLD;
