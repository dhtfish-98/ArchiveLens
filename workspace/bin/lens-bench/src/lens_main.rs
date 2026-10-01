use std::io::Write;
use std::time::Instant;

fn main() {
    let lens_args: Vec<String> = std::env::args().collect();
    let lens_usage = "usage: lens-bench <throughput|metadata> <binary> [--dump <csv>]";
    let lens_mode = match lens_args.get(1) {
        Some(lens_m) => lens_m.as_str(),
        None => {
            eprintln!("{lens_usage}");
            std::process::exit(2);
        }
    };
    let lens_path = match lens_args.get(2) {
        Some(lens_p) => lens_p.clone(),
        None => {
            eprintln!("{lens_usage}");
            std::process::exit(2);
        }
    };
    let lens_dump = lens_args
        .iter()
        .position(|lens_a| lens_a == "--dump")
        .and_then(|lens_i| lens_args.get(lens_i + 1))
        .cloned();
    let lens_emit = lens_args
        .iter()
        .position(|lens_a| lens_a == "--emit")
        .and_then(|lens_i| lens_args.get(lens_i + 1))
        .cloned();

    let lens_code = match lens_mode {
        "throughput" => lens_throughput(&lens_path, lens_dump.as_deref(), lens_emit.as_deref()),
        "metadata" => lens_metadata(&lens_path),
        lens_other => {
            eprintln!("unknown mode {lens_other}\n{lens_usage}");
            2
        }
    };
    std::process::exit(lens_code);
}

fn lens_load(lens_path: &str) -> Result<(Vec<u8>, lens_macho::LensMachOImage), String> {
    let lens_bytes = std::fs::read(lens_path).map_err(|lens_e| format!("cannot read {lens_path}: {lens_e}"))?;
    let lens_image = lens_macho::LensMachOImage::lens_parse(&lens_bytes).map_err(|lens_e| lens_e.to_string())?;
    Ok((lens_bytes, lens_image))
}

fn lens_throughput(lens_path: &str, lens_dump: Option<&str>, lens_emit: Option<&str>) -> i32 {
    let (lens_bytes, lens_image) = match lens_load(lens_path) {
        Ok(lens_v) => lens_v,
        Err(lens_e) => {
            eprintln!("{lens_e}");
            return 1;
        }
    };
    let lens_slice = match lens_macho::lens_fat::lens_select_arm64_slice(&lens_bytes) {
        Ok(lens_s) => lens_s,
        Err(lens_e) => {
            eprintln!("{lens_e}");
            return 1;
        }
    };
    let lens_sdata = lens_slice.lens_data;
    let lens_text = match lens_image.lens_section_by_name("__text") {
        Some(lens_t) => lens_t,
        None => {
            eprintln!("no __text section");
            return 1;
        }
    };
    let lens_off = lens_text.lens_offset as usize;
    let lens_size = lens_text.lens_size as usize;
    let lens_end = lens_off.saturating_add(lens_size).min(lens_sdata.len());
    if lens_off >= lens_end {
        eprintln!(
            "__text out of range (off={lens_off} size={lens_size} slice={})",
            lens_sdata.len()
        );
        return 1;
    }
    let lens_code = &lens_sdata[lens_off..lens_end];
    let lens_n_words = lens_code.len() / 4;
    let lens_base = lens_text.lens_addr;

    let mut lens_dumpf = lens_dump.map(|lens_p| {
        let lens_f = std::fs::File::create(lens_p).expect("create dump file");
        std::io::BufWriter::new(lens_f)
    });
    let mut lens_emitf = lens_emit.map(|lens_p| {
        let lens_f = std::fs::File::create(lens_p).expect("create emit file");
        std::io::BufWriter::new(lens_f)
    });

    let lens_start = Instant::now();
    let mut lens_decoded: u64 = 0;
    let mut lens_sink: u64 = 0;
    for lens_i in 0..lens_n_words {
        let lens_b = lens_i * 4;
        let lens_word = u32::from_le_bytes([lens_code[lens_b], lens_code[lens_b + 1], lens_code[lens_b + 2], lens_code[lens_b + 3]]);
        let lens_addr = lens_base.wrapping_add(lens_b as u64);
        let lens_insn = lens_arm64::lens_decode(lens_word, lens_addr);
        lens_sink = lens_sink.wrapping_add(lens_insn.lens_text.len() as u64);
        lens_decoded += 1;
        if let Some(lens_f) = lens_dumpf.as_mut() {
            let lens_mnem = lens_insn.lens_text.split_whitespace().next().unwrap_or("");
            let _ = writeln!(lens_f, "{lens_addr:x},{lens_word:08x},{lens_mnem}");
        }
        if let Some(lens_f) = lens_emitf.as_mut() {
            let _ = writeln!(lens_f, "{lens_addr:x}: {lens_word:08x}  {}", lens_insn.lens_text);
        }
    }
    if let Some(lens_f) = lens_dumpf.as_mut() {
        let _ = lens_f.flush();
    }
    if let Some(lens_f) = lens_emitf.as_mut() {
        let _ = lens_f.flush();
    }
    let lens_elapsed = lens_start.elapsed();
    let lens_secs = lens_elapsed.as_secs_f64();
    let lens_minsn_s = (lens_decoded as f64 / lens_secs) / 1.0e6;

    eprintln!(
        "decoded {lens_decoded} insns of {} __text bytes in {:.1} ms  ({:.2} Minsn/s)  [checksum {lens_sink}]",
        lens_code.len(),
        lens_secs * 1000.0,
        lens_minsn_s
    );
    let lens_ms = lens_secs * 1000.0;
    let lens_text_bytes = lens_code.len();
    let lens_bin = lens_base_name(lens_path);
    println!(
        "RESULT tool=archivelens mode=throughput bin={lens_bin} insns={lens_decoded} text_bytes={lens_text_bytes} ms={lens_ms:.1} minsn_s={lens_minsn_s:.2}"
    );
    0
}

fn lens_metadata(lens_path: &str) -> i32 {
    let lens_bytes = match std::fs::read(lens_path) {
        Ok(lens_b) => lens_b,
        Err(lens_e) => {
            eprintln!("cannot read {lens_path}: {lens_e}");
            return 1;
        }
    };
    let lens_start = Instant::now();
    let lens_classes = match lens_objc::lens_parse_objc_classes(&lens_bytes) {
        Ok(lens_c) => lens_c,
        Err(lens_e) => {
            eprintln!("{lens_e}");
            return 1;
        }
    };
    let lens_cats = lens_objc::lens_parse_objc_categories(&lens_bytes).unwrap_or_default();
    let lens_elapsed = lens_start.elapsed();

    let lens_n_classes = lens_classes.len();
    let lens_n_cats = lens_cats.len();
    let mut lens_n_methods = 0usize;
    let mut lens_n_ivars = 0usize;
    let mut lens_n_named_super = 0usize;
    for lens_c in &lens_classes {
        lens_n_methods += lens_c.lens_instance_methods.len() + lens_c.lens_class_methods.len();
        lens_n_ivars += lens_c.lens_ivars.len();
        if lens_c.lens_superclass.is_some() {
            lens_n_named_super += 1;
        }
    }
    for lens_c in &lens_cats {
        lens_n_methods += lens_c.lens_instance_methods.len() + lens_c.lens_class_methods.len();
    }
    let lens_secs = lens_elapsed.as_secs_f64();
    eprintln!(
        "recovered {lens_n_classes} classes / {lens_n_cats} categories / {lens_n_methods} methods / {lens_n_ivars} ivars in {:.1} ms",
        lens_secs * 1000.0
    );
    println!(
        "RESULT tool=archivelens mode=metadata bin={} classes={lens_n_classes} categories={lens_n_cats} methods={lens_n_methods} ivars={lens_n_ivars} named_super={lens_n_named_super} ms={:.1}",
        lens_base_name(lens_path),
        lens_secs * 1000.0
    );
    0
}

fn lens_base_name(lens_path: &str) -> &str {
    lens_path.rsplit(['/', '\\']).next().unwrap_or(lens_path)
}
