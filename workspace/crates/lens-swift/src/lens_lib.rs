pub mod lens_metadata;

pub fn lens_demangle(lens_sym: &str) -> String {
    if let Some(lens_rest) = lens_sym.strip_prefix("_Tt") {
        let mut lens_p = LensDemangler {
            lens_b: lens_rest.as_bytes(),
            lens_i: 0,
            lens_depth: 0,
        };
        if let Some(lens_s) = lens_p.lens_parse_type() {
            if lens_p.lens_i == lens_p.lens_b.len() {
                return lens_s;
            }
        }
    }
    for lens_pre in ["_$s", "$s", "_$S", "$S"] {
        if let Some(lens_rest) = lens_sym.strip_prefix(lens_pre) {
            if let Some(lens_s) = lens_demangle_modern(lens_rest) {
                return lens_s;
            }
        }
    }
    lens_sym.to_string()
}

fn lens_demangle_modern(lens_rest: &str) -> Option<String> {
    let mut lens_p = LensDemangler {
        lens_b: lens_rest.as_bytes(),
        lens_i: 0,
        lens_depth: 0,
    };
    let lens_module = lens_p.lens_parse_identifier()?;
    let mut lens_path = vec![lens_module];
    while let Some(lens_id) = lens_p.lens_parse_identifier() {
        match lens_p.lens_peek() {
            Some(b'C') | Some(b'V') | Some(b'O') | Some(b'P') => {
                lens_p.lens_i += 1;
                lens_path.push(lens_id);
            }
            _ => {
                lens_path.push(lens_id);
                break;
            }
        }
    }
    if lens_path.len() < 2 {
        return None;
    }
    Some(lens_path.join("."))
}

const LENS_MAX_DEPTH: usize = 64;

struct LensDemangler<'a> {
    lens_b: &'a [u8],
    lens_i: usize,
    lens_depth: usize,
}

impl<'a> LensDemangler<'a> {
    fn lens_peek(&self) -> Option<u8> {
        self.lens_b.get(self.lens_i).copied()
    }

    fn lens_parse_type(&mut self) -> Option<String> {
        if self.lens_depth > LENS_MAX_DEPTH {
            return None;
        }
        self.lens_depth += 1;
        let lens_r = self.lens_parse_type_inner();
        self.lens_depth -= 1;
        lens_r
    }

    fn lens_parse_type_inner(&mut self) -> Option<String> {
        match self.lens_peek()? {
            b'C' | b'V' | b'O' | b'P' => {
                self.lens_i += 1;
                let lens_context = self.lens_parse_context()?;
                let lens_name = self.lens_parse_name()?;
                Some(format!("{lens_context}.{lens_name}"))
            }
            b'G' => self.lens_parse_generic(),
            b'S' => self.lens_parse_stdlib_shortcut(),
            _ => None,
        }
    }

    fn lens_parse_context(&mut self) -> Option<String> {
        match self.lens_peek()? {
            b'C' | b'V' | b'O' | b'P' | b'G' | b'S' => self.lens_parse_type(),
            b's' => {
                self.lens_i += 1;
                Some("Swift".to_string())
            }
            lens_c if lens_c.is_ascii_digit() => self.lens_parse_identifier(),
            _ => None,
        }
    }

    fn lens_parse_name(&mut self) -> Option<String> {
        if self.lens_peek() == Some(b'P') {
            self.lens_i += 1;
            let lens__disc = self.lens_parse_identifier()?;
            self.lens_parse_identifier()
        } else {
            self.lens_parse_identifier()
        }
    }

    fn lens_parse_identifier(&mut self) -> Option<String> {
        let lens_start = self.lens_i;
        while matches!(self.lens_peek(), Some(c) if c.is_ascii_digit()) {
            self.lens_i += 1;
        }
        if self.lens_i == lens_start {
            return None;
        }
        let lens_len: usize = core::str::from_utf8(&self.lens_b[lens_start..self.lens_i])
            .ok()?
            .parse()
            .ok()?;
        if lens_len == 0 {
            return None;
        }
        let lens_end = self.lens_i.checked_add(lens_len)?;
        if lens_end > self.lens_b.len() {
            return None;
        }
        let lens_s = core::str::from_utf8(&self.lens_b[self.lens_i..lens_end]).ok()?.to_string();
        self.lens_i = lens_end;
        Some(lens_s)
    }

    fn lens_parse_generic(&mut self) -> Option<String> {
        self.lens_i += 1;
        let lens_base = self.lens_parse_type()?;
        let mut lens_args = Vec::new();
        loop {
            match self.lens_peek()? {
                b'_' => {
                    self.lens_i += 1;
                    break;
                }
                _ => lens_args.push(self.lens_parse_type()?),
            }
            if lens_args.len() > 64 {
                return None;
            }
        }
        Some(format!("{}<{}>", lens_base, lens_args.join(", ")))
    }

    fn lens_parse_stdlib_shortcut(&mut self) -> Option<String> {
        self.lens_i += 1;
        let lens_name = match self.lens_peek()? {
            b'S' => "Swift.String",
            b'i' => "Swift.Int",
            b'u' => "Swift.UInt",
            b'd' => "Swift.Double",
            b'f' => "Swift.Float",
            b'b' => "Swift.Bool",
            b'c' => "Swift.UnicodeScalar",
            b'a' => "Swift.Array",
            b'q' => "Swift.Optional",
            _ => return None,
        };
        self.lens_i += 1;
        Some(lens_name.to_string())
    }
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    #[test]
    fn lens_simple_class() {
        assert_eq!(lens_demangle("_TtC6TWorld11Application"), "TWorld.Application");
        assert_eq!(
            lens_demangle("_TtC10ADPService14ADPServiceImpl"),
            "ADPService.ADPServiceImpl"
        );
    }

    #[test]
    fn lens_swift_stdlib_module() {
        assert_eq!(lens_demangle("_TtCs12_SwiftObject"), "Swift._SwiftObject");
    }

    #[test]
    fn lens_nested_class() {
        assert_eq!(
            lens_demangle("_TtCC10DDCommonUI26CameraPickerViewController18CenterFocusOverlay"),
            "DDCommonUI.CameraPickerViewController.CenterFocusOverlay"
        );
    }

    #[test]
    fn lens_private_discriminator_dropped() {
        assert_eq!(
            lens_demangle("_TtCC10MapboxMaps16MapboxObservableP33_D7FE31DC82D97CBC9D480B03D975E3C813BlockObserver"),
            "MapboxMaps.MapboxObservable.BlockObserver"
        );
    }

    #[test]
    fn lens_struct_and_enum_kinds() {
        assert_eq!(lens_demangle("_TtV4Core5Thing"), "Core.Thing");
        assert_eq!(lens_demangle("_TtO4Core4Kind"), "Core.Kind");
    }

    #[test]
    fn lens_non_swift_returned_unchanged() {
        assert_eq!(lens_demangle("NSObject"), "NSObject");
        assert_eq!(lens_demangle("UIViewController"), "UIViewController");
    }

    #[test]
    fn lens_modern_mangling_nominal_path() {
        assert_eq!(
            lens_demangle("_$s10Foundation10CocoaErrorV4CodeVMa"),
            "Foundation.CocoaError.Code"
        );
        assert_eq!(
            lens_demangle("_$s10Foundation10URLRequestV10httpMethodSSSgvg"),
            "Foundation.URLRequest.httpMethod"
        );
        assert_eq!(lens_demangle("$s6TWorld11ApplicationCMa"), "TWorld.Application");
    }

    #[test]
    fn lens_unparseable_falls_back_to_raw() {
        let lens_raw = "_TtCFC8BrazeKitP33_9A88XYZ22NotificationClientLive3fooFT_T_L_9Inner";
        assert_eq!(lens_demangle(lens_raw), lens_raw);
    }

    #[test]
    fn lens_deeply_nested_does_not_overflow() {
        let lens_s = format!("_Tt{}1A1B", "C".repeat(100_000));
        let _ = lens_demangle(&lens_s);
    }
}
