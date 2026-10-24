#![allow(deref_nullptr)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unnecessary_transmutes)]
pub mod ruby_1_9_1_0;
pub mod ruby_1_9_2_0;
pub mod ruby_1_9_3_0;
pub mod ruby_2_0_0_0;
pub mod ruby_2_1_0;
pub mod ruby_2_1_1;
pub mod ruby_2_1_10;
pub mod ruby_2_1_2;
pub mod ruby_2_1_3;
pub mod ruby_2_1_4;
pub mod ruby_2_1_5;
pub mod ruby_2_1_6;
pub mod ruby_2_1_7;
pub mod ruby_2_1_8;
pub mod ruby_2_1_9;
pub mod ruby_2_2_0;
pub mod ruby_2_2_1;
pub mod ruby_2_2_10;
pub mod ruby_2_2_2;
pub mod ruby_2_2_3;
pub mod ruby_2_2_4;
pub mod ruby_2_2_5;
pub mod ruby_2_2_6;
pub mod ruby_2_2_7;
pub mod ruby_2_2_8;
pub mod ruby_2_2_9;
pub mod ruby_2_3_0;
pub mod ruby_2_3_1;
pub mod ruby_2_3_2;
pub mod ruby_2_3_3;
pub mod ruby_2_3_4;
pub mod ruby_2_3_5;
pub mod ruby_2_3_6;
pub mod ruby_2_3_7;
pub mod ruby_2_3_8;
pub mod ruby_2_4_0;
pub mod ruby_2_4_1;
pub mod ruby_2_4_10;
pub mod ruby_2_4_2;
pub mod ruby_2_4_3;
pub mod ruby_2_4_4;
pub mod ruby_2_4_5;
pub mod ruby_2_4_6;
pub mod ruby_2_4_7;
pub mod ruby_2_4_8;
pub mod ruby_2_4_9;
pub mod ruby_2_5_0;
pub mod ruby_2_5_1;
pub mod ruby_2_5_2;
pub mod ruby_2_5_3;
pub mod ruby_2_5_4;
pub mod ruby_2_5_5;
pub mod ruby_2_5_6;
pub mod ruby_2_5_7;
pub mod ruby_2_5_8;
pub mod ruby_2_5_9;
pub mod ruby_2_6_0;
pub mod ruby_2_6_1;
pub mod ruby_2_6_10;
pub mod ruby_2_6_2;
pub mod ruby_2_6_3;
pub mod ruby_2_6_4;
pub mod ruby_2_6_5;
pub mod ruby_2_6_6;
pub mod ruby_2_6_7;
pub mod ruby_2_6_8;
pub mod ruby_2_6_9;
pub mod ruby_2_7_0;
pub mod ruby_2_7_1;
pub mod ruby_2_7_2;
pub mod ruby_2_7_3;
pub mod ruby_2_7_4;
pub mod ruby_2_7_5;
pub mod ruby_2_7_6;
pub mod ruby_2_7_7;
pub mod ruby_2_7_8;
pub mod ruby_3_0_0;
pub mod ruby_3_0_1;
pub mod ruby_3_0_2;
pub mod ruby_3_0_3;
pub mod ruby_3_0_4;
pub mod ruby_3_0_5;
pub mod ruby_3_0_6;
pub mod ruby_3_0_7;
pub mod ruby_3_1_0;
pub mod ruby_3_1_1;
pub mod ruby_3_1_2;
pub mod ruby_3_1_3;
pub mod ruby_3_1_4;
pub mod ruby_3_1_5;
pub mod ruby_3_1_6;
pub mod ruby_3_1_7;
pub mod ruby_3_2_0;
pub mod ruby_3_2_1;
pub mod ruby_3_2_10;
pub mod ruby_3_2_11;
pub mod ruby_3_2_2;
pub mod ruby_3_2_3;
pub mod ruby_3_2_4;
pub mod ruby_3_2_5;
pub mod ruby_3_2_6;
pub mod ruby_3_2_7;
pub mod ruby_3_2_8;
pub mod ruby_3_2_9;
pub mod ruby_3_3_0;
pub mod ruby_3_3_1;
pub mod ruby_3_3_10;
pub mod ruby_3_3_11;
pub mod ruby_3_3_12;
pub mod ruby_3_3_13;
pub mod ruby_3_3_2;
pub mod ruby_3_3_3;
pub mod ruby_3_3_4;
pub mod ruby_3_3_5;
pub mod ruby_3_3_6;
pub mod ruby_3_3_7;
pub mod ruby_3_3_8;
pub mod ruby_3_3_9;
pub mod ruby_3_4_0;
pub mod ruby_3_4_1;
pub mod ruby_3_4_10;
pub mod ruby_3_4_11;
pub mod ruby_3_4_12;
pub mod ruby_3_4_2;
pub mod ruby_3_4_3;
pub mod ruby_3_4_4;
pub mod ruby_3_4_5;
pub mod ruby_3_4_6;
pub mod ruby_3_4_7;
pub mod ruby_3_4_8;
pub mod ruby_3_4_9;
pub mod ruby_4_0_0;
pub mod ruby_4_0_1;
pub mod ruby_4_0_2;
pub mod ruby_4_0_3;
pub mod ruby_4_0_4;
pub mod ruby_4_0_5;
pub mod ruby_4_0_6;
pub mod ruby_4_0_7;
pub mod ruby_4_0_8;
pub mod ruby_4_0_9;

#[cfg(test)]
mod tests {
    use syn::{spanned::Spanned, visit::Visit};

    #[derive(Default)]
    struct BoolFields {
        lines: Vec<usize>,
    }

    impl<'ast> Visit<'ast> for BoolFields {
        fn visit_field(&mut self, field: &'ast syn::Field) {
            if let syn::Type::Path(ty) = &field.ty {
                if ty.qself.is_none() && ty.path.is_ident("bool") {
                    self.lines.push(field.span().start().line);
                }
            }
            syn::visit::visit_field(self, field);
        }
    }

    fn bool_field_lines(source: &str) -> syn::Result<Vec<usize>> {
        let syntax = syn::parse_file(source)?;
        let mut fields = BoolFields::default();
        fields.visit_file(&syntax);
        Ok(fields.lines)
    }

    #[test]
    fn detects_bool_fields_regardless_of_formatting() {
        let source = concat!(
            "struct Example {\n",
            "\tpub\tr#type: bool ,\n",
            "    pub Flag:\n",
            "        /* comment */ bool,\n",
            "}\n",
            "union Either { pub enabled: bool, pub byte: u8 }\n",
            "struct Tuple(pub bool);\n",
        );
        assert_eq!(bool_field_lines(source).unwrap(), vec![2, 3, 6, 7]);
    }

    #[test]
    fn ignores_bool_parameters_returns_and_comments() {
        let source = r#"
// pub misleading: bool,
struct Example {
    pub enabled: u8,
    pub callback: unsafe extern "C" fn(bool) -> bool,
}
impl Example {
    pub fn set_bit(&mut self, val: bool) {}
    pub fn enabled(&self) -> bool { true }
}
"#;
        assert!(bool_field_lines(source).unwrap().is_empty());
    }

    #[test]
    fn bindings_have_no_bool_fields() -> std::io::Result<()> {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut violations = Vec::new();
        let mut files_checked = 0;
        for entry in std::fs::read_dir(source)? {
            let path = entry?.path();
            let name = path.file_name().unwrap().to_string_lossy();
            if !name.starts_with("ruby_") || !name.ends_with(".rs") {
                continue;
            }
            files_checked += 1;
            let contents = std::fs::read_to_string(&path)?;
            let lines = bool_field_lines(&contents)
                .unwrap_or_else(|error| panic!("Failed to parse {}: {error}", path.display()));
            for line in lines {
                violations.push(format!("{}:{line}", path.display()));
            }
        }
        assert!(files_checked > 0, "No Ruby binding files were checked");
        violations.sort();
        assert!(
            violations.is_empty(),
            "Ruby bindings must use u8 instead of bool fields so every byte value is valid. \
             Regenerate with the current xtask or replace these fields with u8:\n{}",
            violations.join("\n")
        );
        Ok(())
    }
}
