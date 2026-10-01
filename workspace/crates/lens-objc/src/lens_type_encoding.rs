const LENS_MAX_TYPE_DEPTH: usize = 64;

pub fn lens_decode_type(lens_enc: &str) -> String {
    let mut lens_p = LensParser::lens_new(lens_enc);
    lens_p.lens_parse_type_d(0).unwrap_or_else(|| lens_enc.to_string())
}

pub fn lens_method_signature(lens_selector: &str, lens_encoding: &str) -> String {
    let mut lens_p = LensParser::lens_new(lens_encoding);
    let lens_ret = lens_p.lens_parse_type_d(0).unwrap_or_else(|| "id".to_string());
    lens_p.lens_read_number();

    let mut lens_args = Vec::new();
    while lens_p.lens_peek().is_some() {
        match lens_p.lens_parse_type_d(0) {
            Some(lens_t) => {
                lens_p.lens_read_number();
                lens_args.push(lens_t);
            }
            None => break,
        }
    }
    let lens_params: Vec<String> = if lens_args.len() >= 2 {
        lens_args[2..].to_vec()
    } else {
        Vec::new()
    };

    let lens_colon_count = lens_selector.matches(':').count();
    if lens_colon_count == 0 {
        return format!("({}){}", lens_ret, lens_selector);
    }
    let lens_parts: Vec<&str> = lens_selector.split(':').collect();
    let mut lens_out = format!("({})", lens_ret);
    for lens_idx in 0..lens_colon_count {
        let lens_keyword = lens_parts.get(lens_idx).copied().unwrap_or("");
        let lens_ty = lens_params.get(lens_idx).cloned().unwrap_or_else(|| "id".to_string());
        lens_out.push_str(&format!("{}:({})arg{} ", lens_keyword, lens_ty, lens_idx + 1));
    }
    lens_out.trim_end().to_string()
}

struct LensParser<'a> {
    lens_b: &'a [u8],
    lens_i: usize,
}

impl<'a> LensParser<'a> {
    fn lens_new(lens_s: &'a str) -> LensParser<'a> {
        LensParser {
            lens_b: lens_s.as_bytes(),
            lens_i: 0,
        }
    }

    fn lens_peek(&self) -> Option<u8> {
        self.lens_b.get(self.lens_i).copied()
    }

    fn lens_read_number(&mut self) -> String {
        let lens_start = self.lens_i;
        while matches!(self.lens_peek(), Some(c) if c.is_ascii_digit()) {
            self.lens_i += 1;
        }
        String::from_utf8_lossy(&self.lens_b[lens_start..self.lens_i]).into_owned()
    }

    fn lens_read_quoted(&mut self) -> String {
        self.lens_i += 1;
        let lens_start = self.lens_i;
        while matches!(self.lens_peek(), Some(c) if c != b'"') {
            self.lens_i += 1;
        }
        let lens_s = String::from_utf8_lossy(&self.lens_b[lens_start..self.lens_i]).into_owned();
        if self.lens_peek() == Some(b'"') {
            self.lens_i += 1;
        }
        lens_s
    }

    fn lens_read_balanced(&mut self, lens_open: u8, lens_close: u8) -> String {
        self.lens_i += 1;
        let lens_start = self.lens_i;
        let mut lens_depth = 1usize;
        while let Some(lens_c) = self.lens_peek() {
            if lens_c == lens_open {
                lens_depth += 1;
            } else if lens_c == lens_close {
                lens_depth -= 1;
                if lens_depth == 0 {
                    break;
                }
            }
            self.lens_i += 1;
        }
        let lens_inner = String::from_utf8_lossy(&self.lens_b[lens_start..self.lens_i]).into_owned();
        if self.lens_peek() == Some(lens_close) {
            self.lens_i += 1;
        }
        lens_inner
    }

    fn lens_skip_qualifiers(&mut self) -> bool {
        let mut lens_is_const = false;
        while let Some(lens_c) = self.lens_peek() {
            match lens_c {
                b'r' => {
                    lens_is_const = true;
                    self.lens_i += 1;
                }
                b'n' | b'N' | b'o' | b'O' | b'R' | b'V' => self.lens_i += 1,
                _ => break,
            }
        }
        lens_is_const
    }

    fn lens_parse_type_d(&mut self, lens_depth: usize) -> Option<String> {
        if lens_depth > LENS_MAX_TYPE_DEPTH {
            return Some("?".to_string());
        }
        let lens_is_const = self.lens_skip_qualifiers();
        let lens_c = self.lens_peek()?;
        let lens_ty = match lens_c {
            b'v' => self.lens_take("void"),
            b'c' => self.lens_take("char"),
            b'i' => self.lens_take("int"),
            b's' => self.lens_take("short"),
            b'l' => self.lens_take("long"),
            b'q' => self.lens_take("long long"),
            b'C' => self.lens_take("unsigned char"),
            b'I' => self.lens_take("unsigned int"),
            b'S' => self.lens_take("unsigned short"),
            b'L' => self.lens_take("unsigned long"),
            b'Q' => self.lens_take("unsigned long long"),
            b'f' => self.lens_take("float"),
            b'd' => self.lens_take("double"),
            b'D' => self.lens_take("long double"),
            b'B' => self.lens_take("BOOL"),
            b'*' => self.lens_take("char *"),
            b'#' => self.lens_take("Class"),
            b':' => self.lens_take("SEL"),
            b'@' => {
                self.lens_i += 1;
                match self.lens_peek() {
                    Some(b'"') => {
                        let lens_name = self.lens_read_quoted();
                        if lens_name.is_empty() {
                            "id".to_string()
                        } else if lens_name.starts_with('<') {
                            format!("id {}", lens_name)
                        } else {
                            format!("{} *", lens_name)
                        }
                    }
                    Some(b'?') => {
                        self.lens_i += 1;
                        "id /* block */".to_string()
                    }
                    _ => "id".to_string(),
                }
            }
            b'?' => self.lens_take("void *"),
            b'^' => {
                self.lens_i += 1;
                let lens_inner = self
                    .lens_parse_type_d(lens_depth + 1)
                    .unwrap_or_else(|| "void".to_string());
                format!("{} *", lens_inner)
            }
            b'{' => {
                let lens_inner = self.lens_read_balanced(b'{', b'}');
                lens_named_aggregate(&lens_inner, "struct")
            }
            b'(' => {
                let lens_inner = self.lens_read_balanced(b'(', b')');
                lens_named_aggregate(&lens_inner, "union")
            }
            b'[' => {
                let lens_inner = self.lens_read_balanced(b'[', b']');
                let lens_digits: String = lens_inner.chars().take_while(|lens_c| lens_c.is_ascii_digit()).collect();
                let lens_elem = LensParser::lens_new(&lens_inner[lens_digits.len()..])
                    .lens_parse_type_d(lens_depth + 1)
                    .unwrap_or_else(|| "void".to_string());
                format!("{}[{}]", lens_elem, lens_digits)
            }
            b'b' => {
                self.lens_i += 1;
                let lens_n = self.lens_read_number();
                format!("int : {}", lens_n)
            }
            _ => {
                self.lens_i += 1;
                (lens_c as char).to_string()
            }
        };
        Some(if lens_is_const {
            format!("const {}", lens_ty)
        } else {
            lens_ty
        })
    }

    fn lens_take(&mut self, lens_s: &str) -> String {
        self.lens_i += 1;
        lens_s.to_string()
    }
}

fn lens_named_aggregate(lens_inner: &str, lens_anon: &str) -> String {
    let lens_name = lens_inner.split('=').next().unwrap_or("");
    if lens_name.is_empty() || lens_name == "?" {
        lens_anon.to_string()
    } else {
        lens_name.to_string()
    }
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    #[test]
    fn lens_primitives() {
        assert_eq!(lens_decode_type("v"), "void");
        assert_eq!(lens_decode_type("B"), "BOOL");
        assert_eq!(lens_decode_type("Q"), "unsigned long long");
        assert_eq!(lens_decode_type("d"), "double");
        assert_eq!(lens_decode_type("q"), "long long");
        assert_eq!(lens_decode_type("#"), "Class");
        assert_eq!(lens_decode_type(":"), "SEL");
        assert_eq!(lens_decode_type("*"), "char *");
    }

    #[test]
    fn lens_objects_and_blocks() {
        assert_eq!(lens_decode_type("@"), "id");
        assert_eq!(lens_decode_type("@\"NSString\""), "NSString *");
        assert_eq!(lens_decode_type("@\"<MTLTexture>\""), "id <MTLTexture>");
        assert_eq!(lens_decode_type("@?"), "id /* block */");
    }

    #[test]
    fn lens_pointers_structs_arrays() {
        assert_eq!(lens_decode_type("^i"), "int *");
        assert_eq!(lens_decode_type("^v"), "void *");
        assert_eq!(lens_decode_type("^{__CVBuffer=}"), "__CVBuffer *");
        assert_eq!(lens_decode_type("{CGRect={CGPoint=dd}{CGSize=dd}}"), "CGRect");
        assert_eq!(lens_decode_type("{?=ii}"), "struct");
        assert_eq!(lens_decode_type("[10i]"), "int[10]");
        assert_eq!(lens_decode_type("r^v"), "const void *");
    }

    #[test]
    fn lens_no_arg_method() {
        assert_eq!(
            lens_method_signature("startUpdating", "v16@0:8"),
            "(void)startUpdating"
        );
        assert_eq!(
            lens_method_signature("isRecording", "B16@0:8"),
            "(BOOL)isRecording"
        );
    }

    #[test]
    fn lens_one_arg_method() {
        assert_eq!(
            lens_method_signature("enableHookPresent:", "v20@0:8B16"),
            "(void)enableHookPresent:(BOOL)arg1"
        );
    }

    #[test]
    fn lens_multi_arg_method_with_block() {
        assert_eq!(
            lens_method_signature("startRecordingWithConfig:completion:", "v32@0:8@16@?24"),
            "(void)startRecordingWithConfig:(id)arg1 completion:(id /* block */)arg2"
        );
    }

    #[test]
    fn lens_method_returning_object() {
        assert_eq!(
            lens_method_signature("converToCodecConfig:", "@24@0:8@16"),
            "(id)converToCodecConfig:(id)arg1"
        );
    }

    #[test]
    fn lens_unparseable_encoding_falls_back() {
        assert_eq!(lens_method_signature("foo", ""), "(id)foo");
    }

    #[test]
    fn lens_deeply_nested_pointers_do_not_overflow() {
        let lens_enc = format!("{}v", "^".repeat(100_000));
        let lens_out = lens_decode_type(&lens_enc);
        assert!(lens_out.ends_with('*') || lens_out.contains('?'), "got: {lens_out}");
    }

    #[test]
    fn lens_deeply_nested_arrays_do_not_overflow() {
        let lens_enc = format!("{}{}i{}", "[1".repeat(50_000), "", "]".repeat(50_000));
        let _ = lens_decode_type(&lens_enc);
    }
}
