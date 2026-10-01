use std::process::Command;

fn lens_minimal_macho() -> Vec<u8> {
    use lens_macho::lens_consts::*;
    let mut lens_seg = Vec::new();
    let mut lens_name = b"__TEXT".to_vec();
    lens_name.resize(16, 0);
    lens_seg.extend_from_slice(&lens_name);
    lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
    lens_seg.extend_from_slice(&0x4000u64.to_le_bytes());
    lens_seg.extend_from_slice(&0u64.to_le_bytes());
    lens_seg.extend_from_slice(&0x4000u64.to_le_bytes());
    lens_seg.extend_from_slice(&5u32.to_le_bytes());
    lens_seg.extend_from_slice(&5u32.to_le_bytes());
    lens_seg.extend_from_slice(&0u32.to_le_bytes());
    lens_seg.extend_from_slice(&0u32.to_le_bytes());
    let lens_cmdsize = 8 + lens_seg.len();
    let mut lens_v = Vec::new();
    lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
    lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
    lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
    lens_v.extend_from_slice(&2u32.to_le_bytes());
    lens_v.extend_from_slice(&1u32.to_le_bytes());
    lens_v.extend_from_slice(&(lens_cmdsize as u32).to_le_bytes());
    lens_v.extend_from_slice(&0u32.to_le_bytes());
    lens_v.extend_from_slice(&0u32.to_le_bytes());
    lens_v.extend_from_slice(&LENS_LC_SEGMENT_64.to_le_bytes());
    lens_v.extend_from_slice(&(lens_cmdsize as u32).to_le_bytes());
    lens_v.extend_from_slice(&lens_seg);
    lens_v
}

fn lens_minimal_macho_encrypted() -> Vec<u8> {
    use lens_macho::lens_consts::*;
    let mut lens_seg = Vec::new();
    let mut lens_name = b"__TEXT".to_vec();
    lens_name.resize(16, 0);
    lens_seg.extend_from_slice(&lens_name);
    lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
    lens_seg.extend_from_slice(&0x4000u64.to_le_bytes());
    lens_seg.extend_from_slice(&0u64.to_le_bytes());
    lens_seg.extend_from_slice(&0x4000u64.to_le_bytes());
    lens_seg.extend_from_slice(&5u32.to_le_bytes());
    lens_seg.extend_from_slice(&5u32.to_le_bytes());
    lens_seg.extend_from_slice(&0u32.to_le_bytes());
    lens_seg.extend_from_slice(&0u32.to_le_bytes());
    let lens_seg_cmdsize = 8 + lens_seg.len();
    let mut lens_enc = Vec::new();
    lens_enc.extend_from_slice(&0x1000u32.to_le_bytes());
    lens_enc.extend_from_slice(&0x3000u32.to_le_bytes());
    lens_enc.extend_from_slice(&1u32.to_le_bytes());
    lens_enc.extend_from_slice(&0u32.to_le_bytes());
    let lens_enc_cmdsize = 8 + lens_enc.len();

    let lens_sizeofcmds = lens_seg_cmdsize + lens_enc_cmdsize;
    let mut lens_v = Vec::new();
    lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
    lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
    lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
    lens_v.extend_from_slice(&2u32.to_le_bytes());
    lens_v.extend_from_slice(&2u32.to_le_bytes());
    lens_v.extend_from_slice(&(lens_sizeofcmds as u32).to_le_bytes());
    lens_v.extend_from_slice(&0u32.to_le_bytes());
    lens_v.extend_from_slice(&0u32.to_le_bytes());
    lens_v.extend_from_slice(&LENS_LC_SEGMENT_64.to_le_bytes());
    lens_v.extend_from_slice(&(lens_seg_cmdsize as u32).to_le_bytes());
    lens_v.extend_from_slice(&lens_seg);
    lens_v.extend_from_slice(&LENS_LC_ENCRYPTION_INFO_64.to_le_bytes());
    lens_v.extend_from_slice(&(lens_enc_cmdsize as u32).to_le_bytes());
    lens_v.extend_from_slice(&lens_enc);
    lens_v
}

#[test]
fn lens_verify_reports_unencrypted() {
    let lens_dir = env!("CARGO_TARGET_TMPDIR");
    let lens_path = std::path::Path::new(lens_dir).join("lens_test_min.macho");
    std::fs::write(&lens_path, lens_minimal_macho()).unwrap();
    let lens_out = Command::new(env!("CARGO_BIN_EXE_archivelens"))
        .arg("verify")
        .arg(&lens_path)
        .output()
        .unwrap();
    assert!(lens_out.status.success());
    let lens_stdout = String::from_utf8_lossy(&lens_out.stdout);
    assert!(lens_stdout.contains("ENCRYPTED:  no"), "got: {lens_stdout}");
    assert!(lens_stdout.contains("arm64"));
}

#[test]
fn lens_rejects_invalid_zip() {
    let lens_dir = env!("CARGO_TARGET_TMPDIR");
    let lens_path = std::path::Path::new(lens_dir).join("lens_test_fake.ipa");
    std::fs::write(&lens_path, b"PK\x03\x04rest-of-zip").unwrap();
    let lens_out = Command::new(env!("CARGO_BIN_EXE_archivelens"))
        .arg("verify")
        .arg(&lens_path)
        .output()
        .unwrap();
    assert!(!lens_out.status.success());
    let lens_stderr = String::from_utf8_lossy(&lens_out.stderr);
    assert!(lens_stderr.contains("not a valid zip"), "got: {lens_stderr}");
}

#[test]
fn lens_accepts_ipa_archive() {
    use std::io::Write;
    let lens_dir = env!("CARGO_TARGET_TMPDIR");
    let lens_path = std::path::Path::new(lens_dir).join("lens_test.ipa");
    let mut lens_buf = Vec::new();
    {
        let mut lens_zw = zip::ZipWriter::new(std::io::Cursor::new(&mut lens_buf));
        let lens_opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        lens_zw.start_file("Payload/Foo.app/Info.plist", lens_opts).unwrap();
        lens_zw.write_all(b"plist").unwrap();
        lens_zw.start_file("Payload/Foo.app/Foo", lens_opts).unwrap();
        lens_zw.write_all(&lens_minimal_macho()).unwrap();
        lens_zw.finish().unwrap();
    }
    std::fs::write(&lens_path, &lens_buf).unwrap();
    let lens_out = Command::new(env!("CARGO_BIN_EXE_archivelens"))
        .arg("verify")
        .arg(&lens_path)
        .output()
        .unwrap();
    assert!(
        lens_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&lens_out.stderr)
    );
    let lens_stdout = String::from_utf8_lossy(&lens_out.stdout);
    assert!(lens_stdout.contains("ENCRYPTED:  no"), "got: {lens_stdout}");
    assert!(lens_stdout.contains("arm64"));
}

#[test]
fn lens_verify_reports_encrypted_with_guidance() {
    let lens_dir = env!("CARGO_TARGET_TMPDIR");
    let lens_path = std::path::Path::new(lens_dir).join("lens_test_encrypted.macho");
    std::fs::write(&lens_path, lens_minimal_macho_encrypted()).unwrap();
    let lens_out = Command::new(env!("CARGO_BIN_EXE_archivelens"))
        .arg("verify")
        .arg(&lens_path)
        .output()
        .unwrap();
    assert!(lens_out.status.success());
    let lens_stdout = String::from_utf8_lossy(&lens_out.stdout);
    assert!(lens_stdout.contains("ENCRYPTED:  YES"), "got: {lens_stdout}");
    assert!(lens_stdout.contains("FairPlay"), "got: {lens_stdout}");
}

fn lens_objc_macho() -> Vec<u8> {
    use lens_macho::lens_consts::*;
    let lens_methname = b"init\0dealloc\0";
    let lens_classname = b"Foo\0Bar\0";
    let lens_methtype = b"v16@0:8\0";
    fn lens_sect_bytes(lens_name: &str, lens_addr: u64, lens_size: u64, lens_offset: u32) -> Vec<u8> {
        let mut lens_s = Vec::new();
        let mut lens_sn = lens_name.as_bytes().to_vec();
        lens_sn.resize(16, 0);
        let mut lens_sg = b"__TEXT".to_vec();
        lens_sg.resize(16, 0);
        lens_s.extend_from_slice(&lens_sn);
        lens_s.extend_from_slice(&lens_sg);
        lens_s.extend_from_slice(&lens_addr.to_le_bytes());
        lens_s.extend_from_slice(&lens_size.to_le_bytes());
        lens_s.extend_from_slice(&lens_offset.to_le_bytes());
        for _ in 0..7 {
            lens_s.extend_from_slice(&0u32.to_le_bytes());
        }
        lens_s
    }
    let lens_nsects = 3u32;
    let lens_seg_body_len = 64 + (lens_nsects as usize) * 80;
    let lens_cmdsize = 8 + lens_seg_body_len;
    let lens_data_start = 32 + lens_cmdsize;
    let lens_off_methname = lens_data_start as u32;
    let lens_off_classname = lens_off_methname + lens_methname.len() as u32;
    let lens_off_methtype = lens_off_classname + lens_classname.len() as u32;
    let mut lens_seg = Vec::new();
    let mut lens_segn = b"__TEXT".to_vec();
    lens_segn.resize(16, 0);
    lens_seg.extend_from_slice(&lens_segn);
    lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
    lens_seg.extend_from_slice(&0x4000u64.to_le_bytes());
    lens_seg.extend_from_slice(&0u64.to_le_bytes());
    lens_seg.extend_from_slice(&0x4000u64.to_le_bytes());
    lens_seg.extend_from_slice(&5u32.to_le_bytes());
    lens_seg.extend_from_slice(&5u32.to_le_bytes());
    lens_seg.extend_from_slice(&lens_nsects.to_le_bytes());
    lens_seg.extend_from_slice(&0u32.to_le_bytes());
    lens_seg.extend_from_slice(&lens_sect_bytes(
        "__objc_methname",
        0x2000,
        lens_methname.len() as u64,
        lens_off_methname,
    ));
    lens_seg.extend_from_slice(&lens_sect_bytes(
        "__objc_classname",
        0x3000,
        lens_classname.len() as u64,
        lens_off_classname,
    ));
    lens_seg.extend_from_slice(&lens_sect_bytes(
        "__objc_methtype",
        0x3100,
        lens_methtype.len() as u64,
        lens_off_methtype,
    ));
    let mut lens_v = Vec::new();
    lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
    lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
    lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
    lens_v.extend_from_slice(&2u32.to_le_bytes());
    lens_v.extend_from_slice(&1u32.to_le_bytes());
    lens_v.extend_from_slice(&(lens_cmdsize as u32).to_le_bytes());
    lens_v.extend_from_slice(&0u32.to_le_bytes());
    lens_v.extend_from_slice(&0u32.to_le_bytes());
    lens_v.extend_from_slice(&LENS_LC_SEGMENT_64.to_le_bytes());
    lens_v.extend_from_slice(&(lens_cmdsize as u32).to_le_bytes());
    lens_v.extend_from_slice(&lens_seg);
    lens_v.extend_from_slice(lens_methname);
    lens_v.extend_from_slice(lens_classname);
    lens_v.extend_from_slice(lens_methtype);
    lens_v
}

#[test]
fn lens_objc_lists_selectors_and_classnames() {
    let lens_dir = env!("CARGO_TARGET_TMPDIR");
    let lens_path = std::path::Path::new(lens_dir).join("lens_test_objc.macho");
    std::fs::write(&lens_path, lens_objc_macho()).unwrap();
    let lens_out = Command::new(env!("CARGO_BIN_EXE_archivelens"))
        .arg("objc")
        .arg(&lens_path)
        .output()
        .unwrap();
    assert!(lens_out.status.success());
    let lens_stdout = String::from_utf8_lossy(&lens_out.stdout);
    assert!(lens_stdout.contains("selectors:    2"), "got: {lens_stdout}");
    assert!(lens_stdout.contains("dealloc"), "got: {lens_stdout}");
    assert!(lens_stdout.contains("Foo"), "got: {lens_stdout}");
    assert!(lens_stdout.contains("v16@0:8"), "got: {lens_stdout}");
}
