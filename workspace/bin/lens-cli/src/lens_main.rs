use clap::{Parser, Subcommand};
use lens_image::LensImage;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "archivelens", about = "ArchiveLens Mach-O loader")]
struct LensCli {
    #[command(subcommand)]
    lens_command: LensCommand,
}

#[derive(Subcommand)]
enum LensCommand {
    /// Report encryption status and basic sanity checks (run this first).
    #[command(name = "verify")]
    LensVerify { #[arg(value_name = "PATH")] lens_path: PathBuf },
    /// Header, segments, UUID, symbol/string counts.
    #[command(name = "info")]
    LensInfo { #[arg(value_name = "PATH")] lens_path: PathBuf },
    /// List symbols with addresses.
    #[command(name = "symbols")]
    LensSymbols { #[arg(value_name = "PATH")] lens_path: PathBuf },
    /// List __cstring strings with addresses.
    #[command(name = "strings")]
    LensStrings { #[arg(value_name = "PATH")] lens_path: PathBuf },
    /// List Objective-C string pools: selectors, class names, method types.
    #[command(name = "objc")]
    LensObjc { #[arg(value_name = "PATH")] lens_path: PathBuf },
    /// Dump Objective-C classes recovered from __objc_classlist.
    #[command(name = "classdump")]
    LensClassdump { #[arg(value_name = "PATH")] lens_path: PathBuf },
    /// List Swift nominal types from __swift5_types (classes, structs, enums).
    #[command(name = "swift-types")]
    LensSwiftTypes { #[arg(value_name = "PATH")] lens_path: PathBuf },
    /// Disassemble arm64 code. With an address, decode one function; otherwise dump all of __text.
    #[command(name = "disasm")]
    LensDisasm {
        #[arg(value_name = "PATH")]
        lens_path: PathBuf,
        /// Start virtual address, e.g. 0x100abcdef. Omit to dump all of __text.
        #[arg(value_name = "ADDR")]
        lens_addr: Option<String>,
        /// Max instructions when an address is given (default 256).
        #[arg(long = "count", value_name = "COUNT", default_value_t = 256)]
        lens_count: usize,
    },
    /// First-cut decompile: CFG-structured pseudocode for a function.
    #[command(name = "decompile")]
    LensDecompile {
        #[arg(value_name = "PATH")]
        lens_path: PathBuf,
        /// Function start virtual address. Omit and pass --all to decompile everything.
        #[arg(value_name = "ADDR")]
        lens_addr: Option<String>,
        #[arg(long = "count", value_name = "COUNT", default_value_t = 512)]
        lens_count: usize,
        /// Decompile every function (from LC_FUNCTION_STARTS, or ObjC method IMPs
        /// when the binary has no function-starts table).
        #[arg(long = "all")]
        lens_all: bool,
        /// Decompile the whole binary into a structured multi-folder project at
        /// this directory (classes/, categories/, functions/, manifest.csv).
        #[arg(long = "project", value_name = "DIR")]
        lens_project: Option<PathBuf>,
    },
}

fn lens_read_input(lens_path: &PathBuf) -> Result<Vec<u8>, String> {
    let lens_bytes = std::fs::read(lens_path).map_err(|lens_e| format!("cannot read {}: {lens_e}", lens_path.display()))?;
    if lens_bytes.starts_with(b"PK\x03\x04") {
        return lens_extract_ipa_executable(&lens_bytes);
    }
    Ok(lens_bytes)
}

fn lens_extract_ipa_executable(lens_bytes: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut lens_zip = zip::ZipArchive::new(std::io::Cursor::new(lens_bytes))
        .map_err(|lens_e| format!("input looks like a .ipa but is not a valid zip: {lens_e}"))?;

    let mut lens_exact: Option<String> = None;
    let mut lens_fallback: Option<(String, u64)> = None;
    for lens_i in 0..lens_zip.len() {
        let lens_f = lens_zip.by_index(lens_i).map_err(|lens_e| lens_e.to_string())?;
        if lens_f.is_dir() {
            continue;
        }
        let lens_name = lens_f.name().replace('\\', "/");
        let lens_parts: Vec<&str> = lens_name.split('/').collect();
        if lens_parts.len() == 3 && lens_parts[0] == "Payload" && lens_parts[1].ends_with(".app") {
            let lens_app = lens_parts[1].trim_end_matches(".app");
            if lens_parts[2] == lens_app {
                lens_exact = Some(lens_name);
                break;
            }
            if !lens_parts[2].contains('.') {
                let lens_sz = lens_f.size();
                if lens_fallback.as_ref().is_none_or(|(_, lens_s)| lens_sz > *lens_s) {
                    lens_fallback = Some((lens_name, lens_sz));
                }
            }
        }
    }
    let lens_target = lens_exact.or_else(|| lens_fallback.map(|(lens_n, _)| lens_n)).ok_or_else(|| {
        "no Payload/<App>.app/<Executable> found in .ipa (is this an iOS app archive?)".to_string()
    })?;

    let mut lens_entry = lens_zip.by_name(&lens_target).map_err(|lens_e| lens_e.to_string())?;
    let mut lens_out = Vec::with_capacity(lens_entry.size() as usize);
    lens_entry.read_to_end(&mut lens_out).map_err(|lens_e| lens_e.to_string())?;
    Ok(lens_out)
}

fn lens_uuid_string(lens_uuid: &[u8; 16]) -> String {
    lens_uuid.iter()
        .map(|lens_b| format!("{lens_b:02X}"))
        .collect::<Vec<_>>()
        .join("")
}

fn lens_dm(lens_name: &str) -> String {
    lens_swift::lens_demangle(lens_name)
}

fn lens_imp_note(lens_imp: u64) -> String {
    if lens_imp == 0 {
        String::new()
    } else {
        format!("  // 0x{lens_imp:x}")
    }
}

fn lens_render_method(lens_selector: &str, lens_types: &str) -> String {
    if lens_types.is_empty() {
        lens_selector.to_string()
    } else {
        lens_objc::lens_type_encoding::lens_method_signature(lens_selector, lens_types)
    }
}

fn lens_run() -> Result<(), String> {
    let lens_cli = LensCli::parse();
    match lens_cli.lens_command {
        LensCommand::LensVerify { lens_path: lens_path } => {
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_img = LensImage::lens_load(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            let lens_arch =
                if lens_img.lens_macho.lens_cpusubtype & 0x00ff_ffff == lens_macho::lens_consts::LENS_CPU_SUBTYPE_ARM64E {
                    "arm64e"
                } else {
                    "arm64"
                };
            println!("arch:       {lens_arch}");
            println!("filetype:   0x{:x}", lens_img.lens_macho.lens_filetype);
            println!("segments:   {}", lens_img.lens_macho.lens_segments.len());
            if lens_img.lens_macho.lens_is_encrypted() {
                println!("ENCRYPTED:  YES (cryptid=1) — FairPlay-protected.");
                println!("            __TEXT is ciphertext; decompilation is impossible");
                println!("            until decrypted. Obtain a decrypted dump (e.g.");
                println!("            frida-ios-dump / bagbak) and retry.");
            } else {
                println!("ENCRYPTED:  no — ready for analysis.");
            }
            Ok(())
        }
        LensCommand::LensInfo { lens_path: lens_path } => {
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_img = LensImage::lens_load(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            println!("cputype:    0x{:x}", lens_img.lens_macho.lens_cputype);
            println!("cpusubtype: 0x{:x}", lens_img.lens_macho.lens_cpusubtype);
            println!("filetype:   0x{:x}", lens_img.lens_macho.lens_filetype);
            match &lens_img.lens_macho.lens_uuid {
                Some(lens_u) => println!("uuid:       {}", lens_uuid_string(lens_u)),
                None => println!("uuid:       (none)"),
            }
            println!("segments:   {}", lens_img.lens_macho.lens_segments.len());
            for lens_s in &lens_img.lens_macho.lens_segments {
                println!(
                    "  {} vm=0x{:x} size=0x{:x} sections={}",
                    lens_s.lens_segname,
                    lens_s.lens_vmaddr,
                    lens_s.lens_vmsize,
                    lens_s.lens_sections.len()
                );
            }
            println!("symbols:    {}", lens_img.lens_macho.lens_symbols.len());
            println!("strings:    {}", lens_img.lens_strings.len());
            println!("func starts:{}", lens_img.lens_macho.lens_function_starts.len());
            if let Some(lens_f0) = lens_img.lens_macho.lens_function_starts.first() {
                let lens_sample: Vec<String> = lens_img
                    .lens_macho
                    .lens_function_starts
                    .iter()
                    .take(3)
                    .map(|lens_a| format!("0x{lens_a:x}"))
                    .collect();
                println!("  first funcs: {} ...", lens_sample.join(", "));
                let _ = lens_f0;
            }
            println!("chained fixups: {}", lens_img.lens_macho.lens_has_chained_fixups);
            Ok(())
        }
        LensCommand::LensSymbols { lens_path: lens_path } => {
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_img = LensImage::lens_load(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            for lens_s in &lens_img.lens_macho.lens_symbols {
                if !lens_s.lens_name.is_empty() {
                    println!("0x{:016x} {}", lens_s.lens_value, lens_dm(&lens_s.lens_name));
                }
            }
            Ok(())
        }
        LensCommand::LensStrings { lens_path: lens_path } => {
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_img = LensImage::lens_load(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            for lens_s in &lens_img.lens_strings {
                println!("0x{:016x} {}", lens_s.lens_addr, lens_s.lens_value);
            }
            Ok(())
        }
        LensCommand::LensObjc { lens_path: lens_path } => {
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_objc = lens_objc::lens_parse_objc_strings(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            println!("selectors:    {}", lens_objc.lens_selectors.len());
            println!("class names:  {}", lens_objc.lens_class_names.len());
            println!("method types: {}", lens_objc.lens_method_types.len());
            println!("--- selectors ---");
            for lens_s in &lens_objc.lens_selectors {
                println!("0x{:016x} {}", lens_s.lens_addr, lens_s.lens_value);
            }
            println!("--- class names ---");
            for lens_s in &lens_objc.lens_class_names {
                println!("0x{:016x} {}", lens_s.lens_addr, lens_s.lens_value);
            }
            println!("--- method types ---");
            for lens_s in &lens_objc.lens_method_types {
                println!("0x{:016x} {}", lens_s.lens_addr, lens_s.lens_value);
            }
            Ok(())
        }
        LensCommand::LensClassdump { lens_path: lens_path } => {
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_classes = lens_objc::lens_parse_objc_classes(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            let lens_total_methods: usize = lens_classes.iter().map(|lens_c| lens_c.lens_instance_methods.len()).sum();
            println!(
                "// {} classes, {} instance methods\n",
                lens_classes.len(),
                lens_total_methods
            );
            for lens_c in &lens_classes {
                let lens_protos = if lens_c.lens_protocols.is_empty() {
                    String::new()
                } else {
                    let lens_ps: Vec<String> = lens_c.lens_protocols.iter().map(|lens_p| lens_dm(lens_p)).collect();
                    format!(" <{}>", lens_ps.join(", "))
                };
                match &lens_c.lens_superclass {
                    Some(lens_sup) => println!("@interface {} : {}{}", lens_dm(&lens_c.lens_name), lens_dm(lens_sup), lens_protos),
                    None => println!("@interface {}{}", lens_dm(&lens_c.lens_name), lens_protos),
                }
                if !lens_c.lens_ivars.is_empty() {
                    println!("{{");
                    for lens_iv in &lens_c.lens_ivars {
                        let lens_ty = lens_objc::lens_type_encoding::lens_decode_type(&lens_iv.lens_type_enc);
                        println!("    {} {}; // +0x{:x}", lens_ty, lens_iv.lens_name, lens_iv.lens_offset);
                    }
                    println!("}}");
                }
                for lens_m in &lens_c.lens_class_methods {
                    println!("+ {};{}", lens_render_method(&lens_m.lens_name, &lens_m.lens_types), lens_imp_note(lens_m.lens_imp));
                }
                for lens_m in &lens_c.lens_instance_methods {
                    println!("- {};{}", lens_render_method(&lens_m.lens_name, &lens_m.lens_types), lens_imp_note(lens_m.lens_imp));
                }
                println!("@end\n");
            }

            let lens_categories =
                lens_objc::lens_parse_objc_categories(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            if !lens_categories.is_empty() {
                println!("// {} categories\n", lens_categories.len());
                for lens_cat in &lens_categories {
                    let lens_cls = lens_cat
                        .lens_class_name
                        .as_deref()
                        .map(lens_dm)
                        .unwrap_or_else(|| "?".to_string());
                    let lens_protos = if lens_cat.lens_protocols.is_empty() {
                        String::new()
                    } else {
                        let lens_ps: Vec<String> = lens_cat.lens_protocols.iter().map(|lens_p| lens_dm(lens_p)).collect();
                        format!(" <{}>", lens_ps.join(", "))
                    };
                    println!("@interface {} ({}){}", lens_cls, lens_cat.lens_name, lens_protos);
                    for lens_m in &lens_cat.lens_class_methods {
                        println!("+ {};", lens_render_method(&lens_m.lens_name, &lens_m.lens_types));
                    }
                    for lens_m in &lens_cat.lens_instance_methods {
                        println!("- {};", lens_render_method(&lens_m.lens_name, &lens_m.lens_types));
                    }
                    println!("@end\n");
                }
            }
            Ok(())
        }
        LensCommand::LensSwiftTypes { lens_path: lens_path } => {
            use lens_swift::lens_metadata::LensSwiftKind;
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_types =
                lens_swift::lens_metadata::lens_parse_swift_types(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            println!("// {} Swift types", lens_types.len());
            for lens_t in &lens_types {
                let lens_kw = match lens_t.lens_kind {
                    LensSwiftKind::LensClass => "class",
                    LensSwiftKind::LensStruct => "struct",
                    LensSwiftKind::LensEnum => "enum",
                    LensSwiftKind::LensOther => "type",
                };
                println!("{lens_kw} {}", lens_t.lens_name);
            }
            Ok(())
        }
        LensCommand::LensDisasm { lens_path: lens_path, lens_addr: lens_addr, lens_count: lens_count } => {
            use std::io::Write;
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_macho = lens_macho::LensMachOImage::lens_parse(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            let lens_slice = lens_macho::lens_fat::lens_select_arm64_slice(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            let lens_sdata = lens_slice.lens_data;
            match lens_addr {
                Some(lens_addr) => {
                    let lens_start = lens_addr.strip_prefix("0x").unwrap_or(&lens_addr);
                    let mut lens_cur = u64::from_str_radix(lens_start, 16)
                        .map_err(|_| format!("bad address: {lens_addr}"))?;
                    for _ in 0..lens_count {
                        let lens_off = match lens_macho.lens_vmaddr_to_offset(lens_cur) {
                            Some(lens_o) => lens_o,
                            None => {
                                eprintln!("(address 0x{lens_cur:x} not in a mapped segment)");
                                break;
                            }
                        };
                        let lens_word = match lens_macho::lens_reader::LensReader::lens_at(lens_sdata, lens_off)
                            .ok()
                            .and_then(|mut lens_r| lens_r.lens_read_u32().ok())
                        {
                            Some(lens_w) => lens_w,
                            None => break,
                        };
                        let lens_insn = lens_arm64::lens_decode(lens_word, lens_cur);
                        println!("0x{:x}:  {:08x}  {}", lens_cur, lens_word, lens_insn.lens_text);
                        if lens_insn.lens_flow == lens_arm64::LensFlow::LensReturn {
                            break;
                        }
                        lens_cur = lens_cur.wrapping_add(4);
                    }
                    Ok(())
                }
                None => {
                    let lens_text = lens_macho
                        .lens_section_by_name("__text")
                        .ok_or_else(|| "no __text section".to_string())?;
                    let lens_off = lens_text.lens_offset as usize;
                    let lens_size = lens_text.lens_size as usize;
                    let lens_end = lens_off
                        .checked_add(lens_size)
                        .filter(|lens_e| *lens_e <= lens_sdata.len())
                        .ok_or_else(|| "__text out of range".to_string())?;
                    let lens_code = &lens_sdata[lens_off..lens_end];
                    let lens_base = lens_text.lens_addr;
                    let lens_stdout = std::io::stdout();
                    let mut lens_w = std::io::BufWriter::new(lens_stdout.lock());
                    let mut lens_i = 0;
                    while lens_i + 4 <= lens_code.len() {
                        let lens_word =
                            u32::from_le_bytes([lens_code[lens_i], lens_code[lens_i + 1], lens_code[lens_i + 2], lens_code[lens_i + 3]]);
                        let lens_a = lens_base + lens_i as u64;
                        let lens_insn = lens_arm64::lens_decode(lens_word, lens_a);
                        let _ = writeln!(lens_w, "0x{:x}:  {:08x}  {}", lens_a, lens_word, lens_insn.lens_text);
                        lens_i += 4;
                    }
                    Ok(())
                }
            }
        }
        LensCommand::LensDecompile {
            lens_path: lens_path,
            lens_addr: lens_addr,
            lens_count: lens_count,
            lens_all: lens_all,
            lens_project: lens_project,
        } => {
            use std::io::Write;
            let lens_bytes = lens_read_input(&lens_path)?;
            let lens_macho = lens_macho::LensMachOImage::lens_parse(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            let lens_slice = lens_macho::lens_fat::lens_select_arm64_slice(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
            let lens_sdata = lens_slice.lens_data;
            let (lens_names, lens_sigs) = lens_build_names(&lens_bytes);

            if let Some(lens_dir) = lens_project {
                let lens_summary =
                    lens_export_project(&lens_macho, lens_sdata, &lens_bytes, lens_count, &lens_dir, &lens_names, &lens_sigs)?;
                println!("{lens_summary}");
                return Ok(());
            }

            if lens_all {
                // Every function we can name: function-starts table first, and if
                // the binary lacks one, fall back to the Objective-C method IMPs.
                let mut lens_starts: Vec<u64> = lens_macho.lens_function_starts.clone();
                if lens_starts.is_empty() {
                    lens_starts = lens_names.keys().copied().collect();
                }
                lens_starts.sort_unstable();
                lens_starts.dedup();
                let lens_stdout = std::io::stdout();
                let mut lens_w = std::io::BufWriter::new(lens_stdout.lock());
                let _ = writeln!(lens_w, "// full decompilation: {} functions\n", lens_starts.len());
                for lens_i in 0..lens_starts.len() {
                    // starts is sorted, so the next function is the following entry.
                    // Passing it in keeps the per-function cost O(1), not O(n).
                    let lens_next_start = lens_starts.get(lens_i + 1).copied();
                    let lens_code =
                        lens_decompile_one(&lens_macho, lens_sdata, lens_starts[lens_i], lens_next_start, lens_count, &lens_names, &lens_sigs);
                    let _ = write!(lens_w, "{lens_code}");
                    let _ = writeln!(lens_w);
                }
                return Ok(());
            }

            let lens_addr = lens_addr.ok_or_else(|| {
                "decompile needs a function address (or pass --all to decompile everything)"
                    .to_string()
            })?;
            let lens_start = lens_parse_addr(&lens_addr)?;
            let lens_next_start = lens_macho
                .lens_function_starts
                .iter()
                .copied()
                .filter(|&lens_a| lens_a > lens_start)
                .min();
            print!(
                "{}",
                lens_decompile_one(&lens_macho, lens_sdata, lens_start, lens_next_start, lens_count, &lens_names, &lens_sigs)
            );
            Ok(())
        }
    }
}

/// Decompile a single function starting at `start`, returning the rendered
/// pseudocode (header comment + body) as a string.
fn lens_decompile_one(
    lens_macho: &lens_macho::LensMachOImage,
    lens_sdata: &[u8],
    lens_start: u64,
    lens_next_start: Option<u64>,
    lens_count: usize,
    lens_names: &LensNameMap,
    lens_sigs: &LensNameMap,
) -> String {
    use std::fmt::Write;

    let mut lens_insns = Vec::new();
    let mut lens_furthest = lens_start;
    let mut lens_cur = lens_start;
    for _ in 0..lens_count {
        if let Some(lens_ns) = lens_next_start {
            if lens_cur >= lens_ns {
                break;
            }
        }
        let lens_off = match lens_macho.lens_vmaddr_to_offset(lens_cur) {
            Some(lens_o) => lens_o,
            None => break,
        };
        let lens_word = match lens_macho::lens_reader::LensReader::lens_at(lens_sdata, lens_off)
            .ok()
            .and_then(|mut lens_r| lens_r.lens_read_u32().ok())
        {
            Some(lens_w) => lens_w,
            None => break,
        };
        let lens_insn = lens_arm64::lens_decode(lens_word, lens_cur);
        if let lens_arm64::LensFlow::LensBranch(lens_t) | lens_arm64::LensFlow::LensCondBranch(lens_t) = lens_insn.lens_flow {
            if lens_t > lens_furthest && lens_next_start.is_none_or(|lens_ns| lens_t < lens_ns) {
                lens_furthest = lens_t;
            }
        }
        let lens_is_ret = lens_insn.lens_flow == lens_arm64::LensFlow::LensReturn;
        lens_insns.push(lens_insn);
        if lens_is_ret && lens_cur >= lens_furthest {
            break;
        }
        lens_cur = lens_cur.wrapping_add(4);
    }

    let lens_fname = lens_names
        .get(&lens_start)
        .cloned()
        .unwrap_or_else(|| format!("sub_{lens_start:x}"));
    let lens_blocks = lens_arm64::lens_cfg::lens_build_blocks(&lens_insns);
    let lens_rblocks: Vec<LensRBlock> = lens_blocks.iter().map(|lens_b| lens_render_block(lens_b, lens_names)).collect();
    let lens_by_addr: std::collections::HashMap<u64, usize> = lens_rblocks
        .iter()
        .enumerate()
        .map(|(lens_i, lens_rb)| (lens_rb.lens_start, lens_i))
        .collect();
    let lens_order: Vec<u64> = lens_rblocks.iter().map(|lens_rb| lens_rb.lens_start).collect();

    let mut lens_s = String::new();
    let _ = writeln!(
        lens_s,
        "// {lens_fname}  @0x{lens_start:x}  ({} blocks, {} instructions)",
        lens_blocks.len(),
        lens_insns.len()
    );
    if let Some(lens_sig) = lens_sigs.get(&lens_start) {
        let _ = writeln!(lens_s, "// signature: {lens_sig}   [x0=self, x1=_cmd, x2..=args]");
    }
    let _ = writeln!(lens_s, "{lens_fname}() {{");
    let mut lens_out = Vec::new();
    lens_structure_emit(
        &lens_rblocks,
        &lens_by_addr,
        &lens_order,
        lens_names,
        0,
        lens_order.len(),
        u64::MAX,
        1,
        &mut lens_out,
    );
    for lens_line in lens_out {
        let _ = writeln!(lens_s, "{lens_line}");
    }
    let _ = writeln!(lens_s, "}}");
    lens_s
}

/// Keep only characters that are safe in a single path segment on every OS,
/// mapping everything else (dots, spaces, `<>`, commas from Swift generics) to
/// `_`, and cap the length so deeply generic names don't blow the path limit.
fn lens_sanitize_seg(lens_s: &str) -> String {
    let mut lens_out: String = lens_s
        .chars()
        .map(|lens_c| {
            if lens_c.is_ascii_alphanumeric() || matches!(lens_c, '_' | '+' | '-') {
                lens_c
            } else {
                '_'
            }
        })
        .collect();
    if lens_out.is_empty() {
        lens_out.push('_');
    }
    if lens_out.len() > 80 {
        lens_out.truncate(80);
    }
    lens_out
}

/// Relative path (under the project root) for a class or category's methods.
/// Swift `Module.Type` names nest under a per-module folder; plain Objective-C
/// class names sit directly under `classes/`.
fn lens_class_relpath(lens_cls: &str, lens_cat: Option<&str>) -> String {
    if let Some(lens_c) = lens_cat {
        return format!(
            "categories/{}+{}.c",
            lens_sanitize_seg(&lens_cls.replace('.', "_")),
            lens_sanitize_seg(lens_c)
        );
    }
    match lens_cls.split_once('.') {
        Some((lens_module, lens_rest)) => {
            format!("classes/{}/{}.c", lens_sanitize_seg(lens_module), lens_sanitize_seg(lens_rest))
        }
        None => format!("classes/{}.c", lens_sanitize_seg(lens_cls)),
    }
}

/// Map every Objective-C method IMP address to the project-relative file its
/// class (or category) should live in.
fn lens_build_groups(lens_bytes: &[u8]) -> LensNameMap {
    let mut lens_g = std::collections::HashMap::new();
    if let Ok(lens_classes) = lens_objc::lens_parse_objc_classes(lens_bytes) {
        for lens_c in &lens_classes {
            let lens_rel = lens_class_relpath(&lens_dm(&lens_c.lens_name), None);
            for lens_m in lens_c.lens_instance_methods.iter().chain(lens_c.lens_class_methods.iter()) {
                if lens_m.lens_imp != 0 {
                    lens_g.entry(lens_m.lens_imp).or_insert_with(|| lens_rel.clone());
                }
            }
        }
    }
    if let Ok(lens_cats) = lens_objc::lens_parse_objc_categories(lens_bytes) {
        for lens_cat in &lens_cats {
            let lens_cls = lens_cat
                .lens_class_name
                .as_deref()
                .map(lens_dm)
                .unwrap_or_else(|| "Unknown".to_string());
            let lens_rel = lens_class_relpath(&lens_cls, Some(&lens_cat.lens_name));
            for lens_m in lens_cat.lens_instance_methods.iter().chain(lens_cat.lens_class_methods.iter()) {
                if lens_m.lens_imp != 0 {
                    lens_g.entry(lens_m.lens_imp).or_insert_with(|| lens_rel.clone());
                }
            }
        }
    }
    lens_g
}

fn lens_csv_field(lens_s: &str) -> String {
    if lens_s.contains([',', '"', '\n']) {
        format!("\"{}\"", lens_s.replace('"', "\"\""))
    } else {
        lens_s.to_string()
    }
}

fn lens_write_rel(lens_dir: &std::path::Path, lens_rel: &str, lens_content: &str) -> Result<(), String> {
    let lens_path = lens_dir.join(lens_rel);
    if let Some(lens_parent) = lens_path.parent() {
        std::fs::create_dir_all(lens_parent).map_err(|lens_e| lens_e.to_string())?;
    }
    std::fs::write(&lens_path, lens_content).map_err(|lens_e| lens_e.to_string())
}

/// Decompile the whole binary into a structured project directory. Named
/// (class/category) methods are grouped one file per class; unnamed functions
/// are streamed in address order into chunked files so we never hold the whole
/// (potentially multi-gigabyte) output in memory at once.
fn lens_export_project(
    lens_macho: &lens_macho::LensMachOImage,
    lens_sdata: &[u8],
    lens_bytes: &[u8],
    lens_count: usize,
    lens_dir: &std::path::Path,
    lens_names: &LensNameMap,
    lens_sigs: &LensNameMap,
) -> Result<String, String> {
    use std::io::Write;
    const LENS_CHUNK: usize = 1000; // unnamed functions per file

    let lens_groups = lens_build_groups(lens_bytes);
    let mut lens_starts: Vec<u64> = lens_macho.lens_function_starts.clone();
    if lens_starts.is_empty() {
        lens_starts = lens_names.keys().copied().collect();
    }
    lens_starts.sort_unstable();
    lens_starts.dedup();

    std::fs::create_dir_all(lens_dir).map_err(|lens_e| lens_e.to_string())?;
    let lens_mf = std::fs::File::create(lens_dir.join("manifest.csv")).map_err(|lens_e| lens_e.to_string())?;
    let mut lens_manifest = std::io::BufWriter::new(lens_mf);
    let _ = writeln!(lens_manifest, "address,name,file");

    // Named-method files are bounded by method count, so buffering them is fine.
    let mut lens_class_files: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut lens_named = 0usize;
    let mut lens_unnamed = 0usize;

    let mut lens_chunk = String::new();
    let mut lens_chunk_first = 0u64;
    let mut lens_chunk_n = 0usize;
    let mut lens_chunk_idx = 0usize;

    for lens_i in 0..lens_starts.len() {
        let lens_start = lens_starts[lens_i];
        let lens_next = lens_starts.get(lens_i + 1).copied();
        let lens_code = lens_decompile_one(lens_macho, lens_sdata, lens_start, lens_next, lens_count, lens_names, lens_sigs);
        let lens_fname = lens_names
            .get(&lens_start)
            .cloned()
            .unwrap_or_else(|| format!("sub_{lens_start:x}"));

        if let Some(lens_rel) = lens_groups.get(&lens_start) {
            let lens_buf = lens_class_files.entry(lens_rel.clone()).or_default();
            if lens_buf.is_empty() {
                lens_buf.push_str(&format!("// {lens_rel}\n// Decompiled by ArchiveLens\n\n"));
            }
            lens_buf.push_str(&lens_code);
            lens_buf.push('\n');
            let _ = writeln!(lens_manifest, "0x{lens_start:x},{},{}", lens_csv_field(&lens_fname), lens_rel);
            lens_named += 1;
        } else {
            if lens_chunk_n == 0 {
                lens_chunk_first = lens_start;
            }
            let lens_rel = format!(
                "functions/{:03x}/funcs_{:x}.c",
                lens_chunk_idx / 256,
                lens_chunk_first
            );
            lens_chunk.push_str(&lens_code);
            lens_chunk.push('\n');
            let _ = writeln!(lens_manifest, "0x{lens_start:x},{},{}", lens_csv_field(&lens_fname), lens_rel);
            lens_chunk_n += 1;
            lens_unnamed += 1;
            if lens_chunk_n >= LENS_CHUNK {
                lens_write_rel(lens_dir, &lens_rel, &lens_chunk)?;
                lens_chunk.clear();
                lens_chunk_n = 0;
                lens_chunk_idx += 1;
            }
        }
    }
    if lens_chunk_n > 0 {
        let lens_rel = format!(
            "functions/{:03x}/funcs_{:x}.c",
            lens_chunk_idx / 256,
            lens_chunk_first
        );
        lens_write_rel(lens_dir, &lens_rel, &lens_chunk)?;
    }

    let lens_class_count = lens_class_files.len();
    for (lens_rel, lens_content) in &lens_class_files {
        lens_write_rel(lens_dir, lens_rel, lens_content)?;
    }
    lens_manifest.flush().map_err(|lens_e| lens_e.to_string())?;

    let lens_total = lens_named + lens_unnamed;
    let lens_readme = format!(
        "# Decompiled project\n\n\
         Generated by ArchiveLens. This is best-effort pseudocode for static analysis, \
         **not compilable C** — register-level operands, unresolved types, and \
         `goto` remain where the decompiler cannot prove more structure.\n\n\
         ## Layout\n\n\
         - `classes/` — one file per Objective-C / Swift class (Swift types are \
         nested under their module folder).\n\
         - `categories/` — Objective-C categories, `Class+Category.c`.\n\
         - `functions/` — unnamed functions (`sub_*`) in address order, chunked \
         {LENS_CHUNK} per file.\n\
         - `manifest.csv` — every function: address, name, and its file.\n\n\
         ## Summary\n\n\
         - Total functions: {lens_total}\n\
         - Named (class/category) methods: {lens_named} across {lens_class_count} class files\n\
         - Unnamed functions: {lens_unnamed}\n"
    );
    std::fs::write(lens_dir.join("README.md"), lens_readme).map_err(|lens_e| lens_e.to_string())?;

    Ok(format!(
        "wrote {lens_total} functions to {} ({lens_named} named across {lens_class_count} class files, {lens_unnamed} unnamed)",
        lens_dir.display()
    ))
}

fn lens_is_reg_tok(lens_s: &str) -> bool {
    lens_s == "sp"
        || ((lens_s.starts_with('x') || lens_s.starts_with('w'))
            && lens_s.len() >= 2
            && lens_s[1..].chars().all(|lens_c| lens_c.is_ascii_digit()))
}

fn lens_is_hex_const(lens_s: &str) -> bool {
    lens_s.strip_prefix("0x").is_some_and(|lens_h| {
        !lens_h.is_empty()
            && lens_h.chars()
                .all(|lens_c| lens_c.is_ascii_digit() || ('a'..='f').contains(&lens_c))
    })
}

fn lens_fold_arith(lens_expr: &str) -> String {
    if let Some((lens_a, lens_b)) = lens_expr.split_once(" + ") {
        if lens_is_hex_const(lens_a) && lens_is_hex_const(lens_b) {
            if let (Ok(lens_x), Ok(lens_y)) = (
                u64::from_str_radix(&lens_a[2..], 16),
                u64::from_str_radix(&lens_b[2..], 16),
            ) {
                return format!("0x{:x}", lens_x.wrapping_add(lens_y));
            }
        }
    }
    if let Some((lens_a, lens_n)) = lens_expr.split_once(" << ") {
        if lens_is_hex_const(lens_a) {
            if let (Ok(lens_x), Ok(lens_sh)) = (u64::from_str_radix(&lens_a[2..], 16), lens_n.parse::<u32>()) {
                return format!("0x{:x}", lens_x.wrapping_shl(lens_sh));
            }
        }
    }
    lens_expr.to_string()
}

fn lens_subst_consts(lens_expr: &str, lens_env: &std::collections::HashMap<String, String>) -> String {
    let lens_subbed: Vec<String> = lens_expr
        .split(' ')
        .map(|lens_tok| {
            let core = lens_tok.trim_matches(|lens_c| "*()[],".contains(lens_c));
            if let Some(lens_v) = lens_env.get(core) {
                lens_tok.replace(core, lens_v)
            } else {
                lens_tok.to_string()
            }
        })
        .collect();
    let lens_joined = lens_subbed.join(" ");
    if let Some(lens_inner) = lens_joined.strip_prefix("*(").and_then(|lens_s| lens_s.strip_suffix(')')) {
        format!("*({})", lens_fold_arith(lens_inner))
    } else {
        lens_fold_arith(&lens_joined)
    }
}

fn lens_propagate(lens_stmts: &[String]) -> Vec<String> {
    let mut lens_env: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut lens_rendered: Vec<(Option<String>, String)> = Vec::new();
    for lens_s in lens_stmts {
        if let Some((lens_lhs, lens_rhs)) = lens_s.split_once(" = ") {
            if lens_is_reg_tok(lens_lhs) && lens_rhs.ends_with(';') {
                let lens_val = lens_subst_consts(&lens_rhs[..lens_rhs.len() - 1], &lens_env);
                if lens_is_hex_const(&lens_val) {
                    lens_env.insert(lens_lhs.to_string(), lens_val.clone());
                } else {
                    lens_env.remove(lens_lhs);
                }
                lens_rendered.push((Some(lens_lhs.to_string()), format!("{lens_lhs} = {lens_val};")));
                continue;
            }
        }
        let lens_body = lens_s.strip_suffix(';').unwrap_or(lens_s);
        let lens_sub = lens_subst_consts(lens_body, &lens_env);
        lens_rendered.push((
            None,
            if lens_s.ends_with(';') {
                format!("{lens_sub};")
            } else {
                lens_sub
            },
        ));
    }
    let lens_regs_of = |lens_s: &str| -> Vec<String> {
        lens_s.split(|lens_c: char| !(lens_c.is_alphanumeric() || lens_c == '_'))
            .filter(|lens_t| lens_is_reg_tok(lens_t))
            .map(|lens_t| lens_t.to_string())
            .collect()
    };
    let mut lens_live: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut lens_keep = vec![true; lens_rendered.len()];
    for lens_idx in (0..lens_rendered.len()).rev() {
        let (lens_dest, lens_text) = &lens_rendered[lens_idx];
        if lens_text.contains("()") {
            for lens_n in 0..=7 {
                lens_live.insert(format!("x{lens_n}"));
                lens_live.insert(format!("w{lens_n}"));
            }
        }
        match lens_dest {
            Some(lens_d) => {
                let lens_rhs = lens_text.split_once(" = ").map(|(_, lens_r)| lens_r).unwrap_or("");
                let lens_is_const_def = lens_is_hex_const(lens_rhs.trim_end_matches(';'));
                if lens_is_const_def && !lens_live.contains(lens_d) {
                    lens_keep[lens_idx] = false;
                } else {
                    lens_live.remove(lens_d);
                    for lens_u in lens_regs_of(lens_rhs) {
                        lens_live.insert(lens_u);
                    }
                }
            }
            None => {
                for lens_u in lens_regs_of(lens_text) {
                    lens_live.insert(lens_u);
                }
            }
        }
    }
    lens_rendered
        .into_iter()
        .zip(lens_keep)
        .filter(|(_, lens_k)| *lens_k)
        .map(|((_, lens_t), _)| lens_t)
        .collect()
}

struct LensRBlock {
    lens_start: u64,
    lens_stmts: Vec<String>,
    lens_term: LensTerm,
    lens_succ: Vec<u64>,
}

fn lens_call_name(lens_t: u64, lens_names: &std::collections::HashMap<u64, String>) -> String {
    lens_names
        .get(&lens_t)
        .cloned()
        .unwrap_or_else(|| format!("sub_{lens_t:x}"))
}

enum LensTerm {
    LensRet,
    LensIndirectRet(String),
    LensGoto(u64),
    LensIf { lens_cond: String, lens_taken: u64 },
    LensFall,
}

fn lens_render_block(
    lens_b: &lens_arm64::lens_cfg::LensBlock,
    lens_names: &std::collections::HashMap<u64, String>,
) -> LensRBlock {
    use lens_arm64::LensFlow;
    let mut lens_stmts = Vec::new();
    let mut lens_last_flags: Option<&lens_arm64::LensFlagOp> = None;
    let mut lens_term = LensTerm::LensFall;
    for lens_ins in &lens_b.lens_insns {
        match &lens_ins.lens_flow {
            LensFlow::LensCall(lens_t) => {
                lens_stmts.push(format!("{}();", lens_call_name(*lens_t, lens_names)));
            }
            LensFlow::LensIndirectCall => lens_stmts.push(lens_pseudo(lens_ins)),
            LensFlow::LensReturn => lens_term = LensTerm::LensRet,
            LensFlow::LensIndirect => lens_term = LensTerm::LensIndirectRet(lens_ins.lens_text.clone()),
            LensFlow::LensBranch(lens_t) => lens_term = LensTerm::LensGoto(*lens_t),
            LensFlow::LensCondBranch(lens_t) => {
                let lens_cond = match lens_ins.lens_cond {
                    Some(lens_code) => lens_resolve_cond(lens_code, lens_last_flags),
                    None => lens_cond_of(&lens_ins.lens_text),
                };
                lens_term = LensTerm::LensIf { lens_cond: lens_cond, lens_taken: *lens_t };
            }
            LensFlow::LensFallthrough => {
                if lens_ins.lens_flags.is_some() {
                    lens_last_flags = lens_ins.lens_flags.as_ref();
                } else {
                    lens_stmts.push(lens_pseudo(lens_ins));
                }
            }
        }
    }
    LensRBlock {
        lens_start: lens_b.lens_start,
        lens_stmts: lens_propagate(&lens_stmts),
        lens_term: lens_term,
        lens_succ: lens_b.lens_succ.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn lens_structure_emit(
    lens_rb: &[LensRBlock],
    lens_by: &std::collections::HashMap<u64, usize>,
    lens_order: &[u64],
    lens_names: &std::collections::HashMap<u64, String>,
    lens_lo: usize,
    lens_hi: usize,
    lens_follow: u64,
    lens_ind: usize,
    lens_out: &mut Vec<String>,
) {
    let lens_pad = "    ".repeat(lens_ind);
    let mut lens_i = lens_lo;
    while lens_i < lens_hi {
        if let Some((lens_e, lens_lcond, lens_exit)) = lens_detect_do_while(lens_rb, lens_by, lens_order, lens_i, lens_hi) {
            lens_out.push(format!("{lens_pad}do {{"));
            lens_structure_emit(lens_rb, lens_by, lens_order, lens_names, lens_i, lens_e, lens_exit, lens_ind + 1, lens_out);
            let lens_lpad = "    ".repeat(lens_ind + 1);
            for lens_s in &lens_rb[lens_by[&lens_order[lens_e]]].lens_stmts {
                lens_out.push(format!("{lens_lpad}{lens_s}"));
            }
            lens_out.push(format!("{lens_pad}}} while ({lens_lcond});"));
            lens_i = lens_by.get(&lens_exit).copied().unwrap_or(lens_hi);
            continue;
        }
        if let Some((lens_e, lens_wcond, lens_exit)) = lens_detect_while(lens_rb, lens_by, lens_order, lens_i, lens_hi) {
            lens_out.push(format!("{lens_pad}while ({lens_wcond}) {{"));
            lens_structure_emit(lens_rb, lens_by, lens_order, lens_names, lens_i + 1, lens_e + 1, lens_order[lens_i], lens_ind + 1, lens_out);
            lens_out.push(format!("{lens_pad}}}"));
            lens_i = lens_by.get(&lens_exit).copied().unwrap_or(lens_hi);
            continue;
        }
        let lens_block = &lens_rb[lens_by[&lens_order[lens_i]]];
        if lens_is_jump_target(lens_rb, lens_order[lens_i]) {
            lens_out.push(format!(
                "{}L_{:x}:",
                "    ".repeat(lens_ind.saturating_sub(1)),
                lens_order[lens_i]
            ));
        }
        for lens_s in &lens_block.lens_stmts {
            lens_out.push(format!("{lens_pad}{lens_s}"));
        }
        match &lens_block.lens_term {
            LensTerm::LensRet => {
                lens_out.push(format!("{lens_pad}return;"));
                lens_i += 1;
            }
            LensTerm::LensIndirectRet(lens_t) => {
                lens_out.push(format!("{lens_pad}return; // {lens_t}"));
                lens_i += 1;
            }
            LensTerm::LensFall => lens_i += 1,
            LensTerm::LensGoto(lens_t) => {
                if !lens_by.contains_key(lens_t) {
                    lens_out.push(format!("{lens_pad}return {}();", lens_call_name(*lens_t, lens_names)));
                } else if *lens_t != lens_follow || lens_i + 1 != lens_hi {
                    lens_out.push(format!("{lens_pad}goto L_{lens_t:x};"));
                }
                lens_i += 1;
            }
            LensTerm::LensIf { lens_cond: lens_cond, lens_taken: lens_taken } => {
                if !lens_by.contains_key(lens_taken) {
                    lens_out.push(format!(
                        "{lens_pad}if ({lens_cond}) return {}();",
                        lens_call_name(*lens_taken, lens_names)
                    ));
                    lens_i += 1;
                    continue;
                }
                if let Some(lens_plan) = lens_plan_if(lens_rb, lens_by, lens_order, lens_i, lens_hi, *lens_taken, lens_cond) {
                    let lens_inv = lens_invert_cond(lens_cond);
                    match lens_plan {
                        LensIfPlan::LensSimple { lens_tidx: lens_tidx } => {
                            lens_out.push(format!("{lens_pad}if ({lens_inv}) {{"));
                            lens_structure_emit(lens_rb, lens_by, lens_order, lens_names, lens_i + 1, lens_tidx, *lens_taken, lens_ind + 1, lens_out);
                            lens_out.push(format!("{lens_pad}}}"));
                            lens_i = lens_tidx;
                        }
                        LensIfPlan::LensElse { lens_tidx: lens_tidx, lens_midx: lens_midx, lens_merge: lens_merge } => {
                            lens_out.push(format!("{lens_pad}if ({lens_inv}) {{"));
                            lens_structure_emit(lens_rb, lens_by, lens_order, lens_names, lens_i + 1, lens_tidx, lens_merge, lens_ind + 1, lens_out);
                            lens_out.push(format!("{lens_pad}}} else {{"));
                            lens_structure_emit(lens_rb, lens_by, lens_order, lens_names, lens_tidx, lens_midx, lens_merge, lens_ind + 1, lens_out);
                            lens_out.push(format!("{lens_pad}}}"));
                            lens_i = lens_midx;
                        }
                    }
                    continue;
                }
                lens_out.push(format!("{lens_pad}if ({lens_cond}) goto L_{lens_taken:x};"));
                lens_i += 1;
            }
        }
    }
}

fn lens_is_invertible(lens_c: &str) -> bool {
    [" == ", " != ", " <= ", " >= ", " < ", " > "]
        .iter()
        .any(|lens_op| lens_c.contains(lens_op))
}

fn lens_invert_cond(lens_c: &str) -> String {
    for (lens_op, lens_inv) in [
        (" == ", " != "),
        (" != ", " == "),
        (" <= ", " > "),
        (" >= ", " < "),
        (" < ", " >= "),
        (" > ", " <= "),
    ] {
        if let Some(lens_pos) = lens_c.find(lens_op) {
            return format!("{}{}{}", &lens_c[..lens_pos], lens_inv, &lens_c[lens_pos + lens_op.len()..]);
        }
    }
    format!("!({lens_c})")
}

fn lens_is_jump_target(lens_rb: &[LensRBlock], lens_addr: u64) -> bool {
    lens_rb.iter().any(|lens_b| {
        matches!(lens_b.lens_term, LensTerm::LensGoto(t) if t == lens_addr)
            || matches!(&lens_b.lens_term, LensTerm::LensIf { lens_taken, .. } if *lens_taken == lens_addr)
    })
}

enum LensIfPlan {
    LensSimple {
        lens_tidx: usize,
    },
    LensElse {
        lens_tidx: usize,
        lens_midx: usize,
        lens_merge: u64,
    },
}

fn lens_detect_do_while(
    lens_rb: &[LensRBlock],
    lens_by: &std::collections::HashMap<u64, usize>,
    lens_order: &[u64],
    lens_i: usize,
    lens_hi: usize,
) -> Option<(usize, String, u64)> {
    let lens_header = lens_order[lens_i];
    for lens_e in (lens_i..lens_hi).rev() {
        if let LensTerm::LensIf { lens_cond: lens_cond, lens_taken: lens_taken } = &lens_rb[lens_by[&lens_order[lens_e]]].lens_term {
            if *lens_taken == lens_header {
                let lens_exit = *lens_order.get(lens_e + 1)?;
                if lens_by.contains_key(&lens_exit)
                    && lens_loop_ok(lens_rb, lens_by, lens_order, lens_i, lens_e, lens_exit)
                    && lens_is_invertible(lens_cond)
                {
                    return Some((lens_e, lens_cond.clone(), lens_exit));
                }
            }
        }
    }
    None
}

fn lens_detect_while(
    lens_rb: &[LensRBlock],
    lens_by: &std::collections::HashMap<u64, usize>,
    lens_order: &[u64],
    lens_i: usize,
    lens_hi: usize,
) -> Option<(usize, String, u64)> {
    let lens_header = lens_order[lens_i];
    let (lens_hcond, lens_exit) = match &lens_rb[lens_by[&lens_header]].lens_term {
        LensTerm::LensIf { lens_cond: lens_cond, lens_taken: lens_taken } if lens_is_invertible(lens_cond) => (lens_cond.clone(), *lens_taken),
        _ => return None,
    };
    let &lens_eidx = lens_by.get(&lens_exit)?;
    if lens_eidx <= lens_i {
        return None;
    }
    for lens_e in (lens_i + 1..lens_hi.min(lens_eidx)).rev() {
        if !matches!(&lens_rb[lens_by[&lens_order[lens_e]]].lens_term, LensTerm::LensGoto(t) if *t == lens_header) {
            continue;
        }
        let lens_early = (lens_i + 1..lens_e).any(|lens_k| lens_rb[lens_by[&lens_order[lens_k]]].lens_succ.contains(&lens_header));
        if !lens_early && lens_loop_ok(lens_rb, lens_by, lens_order, lens_i, lens_e, lens_exit) {
            return Some((lens_e, lens_invert_cond(&lens_hcond), lens_exit));
        }
    }
    None
}

fn lens_loop_ok(
    lens_rb: &[LensRBlock],
    lens_by: &std::collections::HashMap<u64, usize>,
    lens_order: &[u64],
    lens_i: usize,
    lens_e: usize,
    lens_exit: u64,
) -> bool {
    for lens_k in lens_i..=lens_e {
        for &lens_s in &lens_rb[lens_by[&lens_order[lens_k]]].lens_succ {
            let lens_internal = lens_by.get(&lens_s).is_some_and(|&lens_si| lens_si >= lens_i && lens_si <= lens_e);
            if !lens_internal && lens_s != lens_exit {
                return false;
            }
        }
    }
    for (lens_idx, lens_addr) in lens_order.iter().enumerate() {
        if lens_idx >= lens_i && lens_idx <= lens_e {
            continue;
        }
        for &lens_s in &lens_rb[lens_by[lens_addr]].lens_succ {
            if lens_by.get(&lens_s).is_some_and(|&lens_si| lens_si > lens_i && lens_si <= lens_e) {
                return false;
            }
        }
    }
    true
}

fn lens_plan_if(
    lens_rb: &[LensRBlock],
    lens_by: &std::collections::HashMap<u64, usize>,
    lens_order: &[u64],
    lens_i: usize,
    lens_hi: usize,
    lens_taken: u64,
    lens_cond: &str,
) -> Option<LensIfPlan> {
    let &lens_tidx = lens_by.get(&lens_taken)?;
    if !(lens_tidx > lens_i + 1 && lens_tidx <= lens_hi && lens_is_invertible(lens_cond)) {
        return None;
    }
    let lens_guard = lens_order[lens_i];
    if let LensTerm::LensGoto(lens_merge) = lens_rb[lens_by[&lens_order[lens_tidx - 1]]].lens_term {
        if let Some(&lens_midx) = lens_by.get(&lens_merge) {
            if lens_midx > lens_tidx
                && lens_midx <= lens_hi
                && lens_region_ok(lens_rb, lens_by, lens_order, lens_i + 1, lens_tidx, lens_merge, lens_guard)
                && lens_region_ok(lens_rb, lens_by, lens_order, lens_tidx, lens_midx, lens_merge, lens_guard)
            {
                return Some(LensIfPlan::LensElse { lens_tidx: lens_tidx, lens_midx: lens_midx, lens_merge: lens_merge });
            }
        }
    }
    if lens_region_ok(lens_rb, lens_by, lens_order, lens_i + 1, lens_tidx, lens_taken, lens_guard) {
        return Some(LensIfPlan::LensSimple { lens_tidx: lens_tidx });
    }
    None
}

fn lens_region_ok(
    lens_rb: &[LensRBlock],
    lens_by: &std::collections::HashMap<u64, usize>,
    lens_order: &[u64],
    lens_lo: usize,
    lens_hi: usize,
    lens_exit_addr: u64,
    lens_guard_addr: u64,
) -> bool {
    if lens_lo >= lens_hi || lens_hi > lens_order.len() {
        return false;
    }
    for lens_k in lens_lo..lens_hi {
        for &lens_s in &lens_rb[lens_by[&lens_order[lens_k]]].lens_succ {
            let lens_internal = lens_by.get(&lens_s).is_some_and(|&lens_si| lens_si >= lens_lo && lens_si < lens_hi);
            if !lens_internal && lens_s != lens_exit_addr {
                return false;
            }
        }
    }
    for (lens_idx, lens_addr) in lens_order.iter().enumerate() {
        if lens_idx >= lens_lo && lens_idx < lens_hi {
            continue;
        }
        for &lens_s in &lens_rb[lens_by[lens_addr]].lens_succ {
            if let Some(&lens_si) = lens_by.get(&lens_s) {
                if lens_si > lens_lo && lens_si < lens_hi {
                    return false;
                }
                if lens_si == lens_lo && *lens_addr != lens_guard_addr {
                    return false;
                }
            }
        }
    }
    true
}

const LENS_CC: [&str; 16] = [
    "eq", "ne", "cs", "cc", "mi", "pl", "vs", "vc", "hi", "ls", "ge", "lt", "gt", "le", "al", "nv",
];

fn lens_resolve_cond(lens_code: u8, lens_flags: Option<&lens_arm64::LensFlagOp>) -> String {
    use lens_arm64::LensFlagKind;
    let lens_f = match lens_flags {
        Some(lens_f) => lens_f,
        None => return format!("cond_{}", LENS_CC.get(lens_code as usize).unwrap_or(&"?")),
    };
    match lens_f.lens_kind {
        LensFlagKind::LensCmp => lens_cmp_expr(lens_code, &lens_f.lens_a, &lens_f.lens_b),
        LensFlagKind::LensCmn => lens_cmp_expr(lens_code, &format!("({} + {})", lens_f.lens_a, lens_f.lens_b), "0"),
        LensFlagKind::LensTst => {
            let lens_inner = format!("({} & {})", lens_f.lens_a, lens_f.lens_b);
            match lens_code {
                0 => format!("{lens_inner} == 0"),
                1 => format!("{lens_inner} != 0"),
                _ => format!(
                    "cond_{} /* {lens_inner} */",
                    LENS_CC.get(lens_code as usize).unwrap_or(&"?")
                ),
            }
        }
    }
}

fn lens_cmp_expr(lens_code: u8, lens_a: &str, lens_b: &str) -> String {
    let lens_op = match lens_code {
        0 => "==",
        1 => "!=",
        2 => ">=",
        3 | 4 => "<",
        5 | 10 => ">=",
        8 | 12 => ">",
        9 | 13 => "<=",
        11 => "<",
        6 | 7 => return "/* overflow */".to_string(),
        14 => return "1".to_string(),
        15 => return "0".to_string(),
        _ => return format!("cond_{}", LENS_CC.get(lens_code as usize).unwrap_or(&"?")),
    };
    format!("{lens_a} {lens_op} {lens_b}")
}

fn lens_parse_addr(lens_addr: &str) -> Result<u64, String> {
    let lens_s = lens_addr.strip_prefix("0x").unwrap_or(lens_addr);
    u64::from_str_radix(lens_s, 16).map_err(|_| format!("bad address: {lens_addr}"))
}

type LensNameMap = std::collections::HashMap<u64, String>;

fn lens_build_names(lens_bytes: &[u8]) -> (LensNameMap, LensNameMap) {
    let mut lens_names = std::collections::HashMap::new();
    let mut lens_sigs = std::collections::HashMap::new();
    let mut lens_add = |lens_imp: u64, lens_sign: char, lens_cls: &str, lens_cat: Option<&str>, lens_sel: &str, lens_types: &str| {
        if lens_imp == 0 {
            return;
        }
        let lens_disp = match lens_cat {
            Some(lens_c) => format!("{lens_sign}[{lens_cls}({lens_c}) {lens_sel}]"),
            None => format!("{lens_sign}[{lens_cls} {lens_sel}]"),
        };
        lens_names.entry(lens_imp).or_insert(lens_disp);
        lens_sigs.entry(lens_imp)
            .or_insert(format!("{lens_sign} {}", lens_render_method(lens_sel, lens_types)));
    };
    if let Ok(lens_classes) = lens_objc::lens_parse_objc_classes(lens_bytes) {
        for lens_c in &lens_classes {
            let lens_cls = lens_dm(&lens_c.lens_name);
            for lens_m in &lens_c.lens_instance_methods {
                lens_add(lens_m.lens_imp, '-', &lens_cls, None, &lens_m.lens_name, &lens_m.lens_types);
            }
            for lens_m in &lens_c.lens_class_methods {
                lens_add(lens_m.lens_imp, '+', &lens_cls, None, &lens_m.lens_name, &lens_m.lens_types);
            }
        }
    }
    if let Ok(lens_cats) = lens_objc::lens_parse_objc_categories(lens_bytes) {
        for lens_cat in &lens_cats {
            let lens_cls = lens_cat
                .lens_class_name
                .as_deref()
                .map(lens_dm)
                .unwrap_or_else(|| "?".to_string());
            for lens_m in &lens_cat.lens_instance_methods {
                lens_add(lens_m.lens_imp, '-', &lens_cls, Some(&lens_cat.lens_name), &lens_m.lens_name, &lens_m.lens_types);
            }
            for lens_m in &lens_cat.lens_class_methods {
                lens_add(lens_m.lens_imp, '+', &lens_cls, Some(&lens_cat.lens_name), &lens_m.lens_name, &lens_m.lens_types);
            }
        }
    }
    (lens_names, lens_sigs)
}

fn lens_pseudo(lens_ins: &lens_arm64::LensInsn) -> String {
    use lens_arm64::LensFlow;
    match &lens_ins.lens_flow {
        LensFlow::LensReturn => return "return;".to_string(),
        LensFlow::LensBranch(lens_t) => return format!("goto L_{lens_t:x};"),
        LensFlow::LensCondBranch(lens_t) => return format!("if ({}) goto L_{lens_t:x};", lens_cond_of(&lens_ins.lens_text)),
        LensFlow::LensIndirectCall => {
            let lens_r = lens_ins.lens_text.trim_start_matches("blr ").trim();
            return format!("(*{lens_r})();");
        }
        LensFlow::LensIndirect => return format!("goto *; // {}", lens_ins.lens_text),
        LensFlow::LensCall(lens_t) => return format!("sub_{lens_t:x}();"),
        LensFlow::LensFallthrough => {}
    }
    let (lens_mn, lens_rest) = match lens_ins.lens_text.split_once(' ') {
        Some((lens_m, lens_r)) => (lens_m, lens_r),
        None => return format!("// {}", lens_ins.lens_text),
    };
    let lens_ops: Vec<&str> = lens_rest.split(", ").collect();
    let lens_noh = |lens_s: &str| lens_s.trim_start_matches('#').to_string();
    let lens_binop = |lens_sym: &str| -> Option<String> {
        if lens_ops.len() >= 3 {
            Some(format!(
                "{} = {} {lens_sym} {};",
                lens_ops[0],
                lens_noh(lens_ops[1]),
                lens_noh(&lens_ops[2..].join(", "))
            ))
        } else {
            None
        }
    };
    let lens_out = match lens_mn {
        "mov" if lens_ops.len() == 2 => format!("{} = {};", lens_ops[0], lens_noh(lens_ops[1])),
        "add" => lens_binop("+").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "sub" => lens_binop("-").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "orr" => lens_binop("|").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "and" => lens_binop("&").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "eor" => lens_binop("^").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "mul" => lens_binop("*").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "udiv" | "sdiv" => lens_binop("/").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "lsl" => lens_binop("<<").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "lsr" | "asr" => lens_binop(">>").unwrap_or_else(|| lens_fallback(&lens_ins.lens_text)),
        "adrp" | "adr" if lens_ops.len() == 2 => format!("{} = {};", lens_ops[0], lens_ops[1]),
        "ldr" | "ldur" | "ldrb" | "ldrh" if lens_ops.len() >= 2 => {
            format!("{} = *({});", lens_ops[0], lens_mem_inner(&lens_ops[1..].join(", ")))
        }
        "str" | "stur" | "strb" | "strh" if lens_ops.len() >= 2 => {
            format!("*({}) = {};", lens_mem_inner(&lens_ops[1..].join(", ")), lens_ops[0])
        }
        "uxtb" | "uxth" | "sxtb" | "sxth" | "sxtw" if lens_ops.len() == 2 => {
            format!("{} = {};", lens_ops[0], lens_ops[1])
        }
        "csel" if lens_ops.len() == 4 => format!("{} = {} ? {} : {};", lens_ops[0], lens_ops[3], lens_ops[1], lens_ops[2]),
        "cset" if lens_ops.len() == 2 => format!("{} = ({});", lens_ops[0], lens_ops[1]),
        _ => lens_fallback(&lens_ins.lens_text),
    };
    lens_out
}

fn lens_fallback(lens_text: &str) -> String {
    format!("// {lens_text}")
}

fn lens_mem_inner(lens_op: &str) -> String {
    let lens_inner = lens_op.trim_start_matches('[').trim_end_matches(']');
    match lens_inner.split_once(", #") {
        Some((lens_base, lens_off)) => format!("{lens_base} + {lens_off}"),
        None => lens_inner.to_string(),
    }
}

fn lens_cond_of(lens_text: &str) -> String {
    if let Some(lens_rest) = lens_text.strip_prefix("cbz ") {
        let lens_r = lens_rest.split(',').next().unwrap_or("?").trim();
        return format!("{lens_r} == 0");
    }
    if let Some(lens_rest) = lens_text.strip_prefix("cbnz ") {
        let lens_r = lens_rest.split(',').next().unwrap_or("?").trim();
        return format!("{lens_r} != 0");
    }
    if let Some(lens_rest) = lens_text.strip_prefix("tbz ") {
        let mut lens_it = lens_rest.split(',');
        let lens_r = lens_it.next().unwrap_or("?").trim();
        let lens_b = lens_it.next().unwrap_or("#?").trim().trim_start_matches('#');
        return format!("({lens_r} & (1 << {lens_b})) == 0");
    }
    if let Some(lens_rest) = lens_text.strip_prefix("tbnz ") {
        let mut lens_it = lens_rest.split(',');
        let lens_r = lens_it.next().unwrap_or("?").trim();
        let lens_b = lens_it.next().unwrap_or("#?").trim().trim_start_matches('#');
        return format!("({lens_r} & (1 << {lens_b})) != 0");
    }
    if let Some(lens_rest) = lens_text.strip_prefix("b.") {
        let lens_cc = lens_rest.split(' ').next().unwrap_or("?");
        return format!("cond_{lens_cc}");
    }
    "cond".to_string()
}

fn main() -> ExitCode {
    match lens_run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(lens_msg) => {
            eprintln!("error: {lens_msg}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod lens_tests {
    use super::*;
    use std::collections::HashMap;

    fn lens_build_ipa(lens_files: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::{Cursor, Write};
        let mut lens_buf = Vec::new();
        {
            let mut lens_zw = zip::ZipWriter::new(Cursor::new(&mut lens_buf));
            let lens_opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for (lens_name, lens_data) in lens_files {
                lens_zw.start_file(*lens_name, lens_opts).unwrap();
                lens_zw.write_all(lens_data).unwrap();
            }
            lens_zw.finish().unwrap();
        }
        lens_buf
    }

    #[test]
    fn lens_ipa_extract_picks_named_executable() {
        let lens_ipa = lens_build_ipa(&[
            ("Payload/Foo.app/Info.plist", b"plist"),
            ("Payload/Foo.app/icon.png", b"png"),
            ("Payload/Foo.app/Foo", b"\xcf\xfa\xed\xfe MACHO"),
        ]);
        assert_eq!(
            lens_extract_ipa_executable(&lens_ipa).unwrap(),
            b"\xcf\xfa\xed\xfe MACHO"
        );
    }

    #[test]
    fn lens_ipa_extract_falls_back_to_largest_extensionless() {
        let lens_ipa = lens_build_ipa(&[
            ("Payload/Bar.app/small", b"aa"),
            ("Payload/Bar.app/BigExec", b"AAAAAAAAAAAAAAAA"),
        ]);
        assert_eq!(lens_extract_ipa_executable(&lens_ipa).unwrap(), b"AAAAAAAAAAAAAAAA");
    }

    #[test]
    fn lens_ipa_extract_rejects_non_ipa_zip() {
        let lens_zip = lens_build_ipa(&[("notes.txt", b"hello")]);
        assert!(lens_extract_ipa_executable(&lens_zip).is_err());
    }

    fn lens_rb(lens_start: u64, lens_stmts: &[&str], lens_term: LensTerm, lens_succ: &[u64]) -> LensRBlock {
        LensRBlock {
            lens_start: lens_start,
            lens_stmts: lens_stmts.iter().map(|lens_s| lens_s.to_string()).collect(),
            lens_term: lens_term,
            lens_succ: lens_succ.to_vec(),
        }
    }

    #[test]
    fn lens_if_else_recovered_without_fallthrough() {
        let lens_blocks = vec![
            lens_rb(
                0x00,
                &[],
                LensTerm::LensIf {
                    lens_cond: "a == 0".into(),
                    lens_taken: 0x40,
                },
                &[0x40, 0x10],
            ),
            lens_rb(
                0x10,
                &[],
                LensTerm::LensIf {
                    lens_cond: "b == 0".into(),
                    lens_taken: 0x30,
                },
                &[0x30, 0x20],
            ),
            lens_rb(0x20, &["r = 1;"], LensTerm::LensGoto(0x40), &[0x40]),
            lens_rb(0x30, &["r = 2;"], LensTerm::LensFall, &[0x40]),
            lens_rb(0x40, &["x0 = r;"], LensTerm::LensRet, &[]),
        ];
        let lens_order: Vec<u64> = lens_blocks.iter().map(|lens_b| lens_b.lens_start).collect();
        let lens_by: HashMap<u64, usize> = lens_order.iter().enumerate().map(|(lens_i, lens_a)| (*lens_a, lens_i)).collect();
        let mut lens_out = Vec::new();
        let lens_names: HashMap<u64, String> = HashMap::new();
        lens_structure_emit(
            &lens_blocks,
            &lens_by,
            &lens_order,
            &lens_names,
            0,
            lens_order.len(),
            u64::MAX,
            1,
            &mut lens_out,
        );
        let lens_text = lens_out.join("\n");
        assert!(lens_text.contains("if (a != 0) {"), "outer if missing:\n{lens_text}");
        assert!(lens_text.contains("} else {"), "if/else not recovered:\n{lens_text}");
        let lens_p1 = lens_text.find("r = 1;").expect("r=1 missing");
        let lens_pe = lens_text.find("} else {").expect("else missing");
        let lens_p2 = lens_text.find("r = 2;").expect("r=2 missing");
        assert!(lens_p1 < lens_pe && lens_pe < lens_p2, "then/else out of order:\n{lens_text}");
        assert!(
            !lens_text.contains("r = 1;\n        r = 2;"),
            "then fell into else:\n{lens_text}"
        );
    }

    #[test]
    fn lens_do_while_loop_recovered() {
        let lens_blocks = vec![
            lens_rb(0x00, &["i = 0;"], LensTerm::LensFall, &[0x10]),
            lens_rb(
                0x10,
                &["sum += i;", "i++;"],
                LensTerm::LensIf {
                    lens_cond: "i < 10".into(),
                    lens_taken: 0x10,
                },
                &[0x10, 0x20],
            ),
            lens_rb(0x20, &["x0 = sum;"], LensTerm::LensRet, &[]),
        ];
        let lens_order: Vec<u64> = lens_blocks.iter().map(|lens_b| lens_b.lens_start).collect();
        let lens_by: HashMap<u64, usize> = lens_order.iter().enumerate().map(|(lens_i, lens_a)| (*lens_a, lens_i)).collect();
        let mut lens_out = Vec::new();
        let lens_names: HashMap<u64, String> = HashMap::new();
        lens_structure_emit(
            &lens_blocks,
            &lens_by,
            &lens_order,
            &lens_names,
            0,
            lens_order.len(),
            u64::MAX,
            1,
            &mut lens_out,
        );
        let lens_text = lens_out.join("\n");
        assert!(lens_text.contains("do {"), "no do-while:\n{lens_text}");
        assert!(
            lens_text.contains("} while (i < 10);"),
            "wrong loop cond:\n{lens_text}"
        );
        let lens_dopos = lens_text.find("do {").unwrap();
        let lens_whpos = lens_text.find("} while").unwrap();
        let lens_bodypos = lens_text.find("sum += i;").unwrap();
        let lens_exitpos = lens_text.find("x0 = sum;").unwrap();
        assert!(
            lens_dopos < lens_bodypos && lens_bodypos < lens_whpos && lens_whpos < lens_exitpos,
            "loop layout wrong:\n{lens_text}"
        );
    }

    #[test]
    fn lens_while_loop_recovered() {
        let lens_blocks = vec![
            lens_rb(
                0x00,
                &[],
                LensTerm::LensIf {
                    lens_cond: "i >= 10".into(),
                    lens_taken: 0x30,
                },
                &[0x30, 0x10],
            ),
            lens_rb(0x10, &["sum += i;", "i++;"], LensTerm::LensGoto(0x00), &[0x00]),
            lens_rb(0x30, &["x0 = sum;"], LensTerm::LensRet, &[]),
        ];
        let lens_order: Vec<u64> = lens_blocks.iter().map(|lens_b| lens_b.lens_start).collect();
        let lens_by: HashMap<u64, usize> = lens_order.iter().enumerate().map(|(lens_i, lens_a)| (*lens_a, lens_i)).collect();
        let mut lens_out = Vec::new();
        let lens_names: HashMap<u64, String> = HashMap::new();
        lens_structure_emit(
            &lens_blocks,
            &lens_by,
            &lens_order,
            &lens_names,
            0,
            lens_order.len(),
            u64::MAX,
            1,
            &mut lens_out,
        );
        let lens_text = lens_out.join("\n");
        assert!(lens_text.contains("while (i < 10) {"), "no while loop:\n{lens_text}");
        assert!(!lens_text.contains("goto L_0;"), "back-edge not elided:\n{lens_text}");
        let lens_wpos = lens_text.find("while (i < 10)").unwrap();
        let lens_bodypos = lens_text.find("sum += i;").unwrap();
        let lens_exitpos = lens_text.find("x0 = sum;").unwrap();
        assert!(
            lens_wpos < lens_bodypos && lens_bodypos < lens_exitpos,
            "while layout wrong:\n{lens_text}"
        );
    }

    #[test]
    fn lens_propagate_folds_adrp_address() {
        let lens_stmts = vec![
            "x8 = 0x10ce86000;".to_string(),
            "x0 = *(x8 + 0x790);".to_string(),
        ];
        let lens_out = lens_propagate(&lens_stmts);
        assert_eq!(lens_out, vec!["x0 = *(0x10ce86790);".to_string()]);
    }

    #[test]
    fn lens_propagate_folds_add_chain_into_use() {
        let lens_stmts = vec![
            "x2 = 0x10ccca000;".to_string(),
            "x2 = x2 + 0x268;".to_string(),
            "*(x2) = x0;".to_string(),
        ];
        let lens_out = lens_propagate(&lens_stmts);
        assert_eq!(lens_out, vec!["*(0x10ccca268) = x0;".to_string()]);
    }

    #[test]
    fn lens_propagate_keeps_arg_setup_before_call() {
        let lens_stmts = vec!["x0 = 0x1234;".to_string(), "foo();".to_string()];
        let lens_out = lens_propagate(&lens_stmts);
        assert_eq!(lens_out, vec!["x0 = 0x1234;".to_string(), "foo();".to_string()]);
    }

    #[test]
    fn lens_class_relpath_layout() {
        // Plain Objective-C class sits directly under classes/.
        assert_eq!(lens_class_relpath("NSObject", None), "classes/NSObject.c");
        // Swift Module.Type nests under a per-module folder.
        assert_eq!(
            lens_class_relpath("FBSDKCoreKit.AEMNetworker", None),
            "classes/FBSDKCoreKit/AEMNetworker.c"
        );
        // Categories go under categories/ as Class+Category.c, dots flattened.
        assert_eq!(
            lens_class_relpath("FBSDKCoreKit.BridgeAPI", Some("FBSDKCoreKit")),
            "categories/FBSDKCoreKit_BridgeAPI+FBSDKCoreKit.c"
        );
    }

    #[test]
    fn lens_sanitize_seg_is_path_safe() {
        assert_eq!(lens_sanitize_seg("Foo<Bar, Baz>"), "Foo_Bar__Baz_");
        assert_eq!(lens_sanitize_seg(""), "_");
        assert!(lens_sanitize_seg(&"x".repeat(200)).len() <= 80);
    }

    #[test]
    fn lens_csv_field_quotes_when_needed() {
        assert_eq!(lens_csv_field("sub_100"), "sub_100");
        assert_eq!(lens_csv_field("a,b"), "\"a,b\"");
        assert_eq!(lens_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn lens_invert_cond_flips_operators() {
        assert_eq!(lens_invert_cond("x21 == 0"), "x21 != 0");
        assert_eq!(lens_invert_cond("w9 != 0x305"), "w9 == 0x305");
        assert_eq!(lens_invert_cond("a < b"), "a >= b");
        assert_eq!(lens_invert_cond("cond_ne"), "!(cond_ne)");
        assert!(!lens_is_invertible("cond_ne"));
    }
}
