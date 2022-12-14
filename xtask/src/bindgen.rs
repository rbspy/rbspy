use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn generate_ruby_bindings(
    ruby_source_path: std::path::PathBuf,
    version_tag: &str,
    output_filename: Option<&str>,
) -> Result<()> {
    prepare_ruby_source(&ruby_source_path, version_tag)
        .context("prepare ruby source repository")?;

    let work_path = tempdir::TempDir::new("rbspy-ruby-bindgen").context("create work directory")?;
    log::info!(
        "Working directory path is {}",
        work_path.path().to_string_lossy()
    );
    let wrapper_path = prepare_ruby_headers(work_path.path(), &ruby_source_path)
        .context("prepare ruby headers")?;

    let bindings = bindgen::builder()
        .allowlist_type("imemo_type")
        .allowlist_type("iseq_insn_info_entry")
        .allowlist_type("RArray")
        .allowlist_type("rb_control_frame_struct")
        .allowlist_type("rb_execution_context_struct")
        .allowlist_type("rb_id_serial_t")
        .allowlist_type("rb_iseq_constant_body")
        .allowlist_type("rb_iseq_location_struct")
        .allowlist_type("rb_iseq_struct")
        .allowlist_type("rb_method_entry_struct")
        .allowlist_type("rb_thread_struct")
        .allowlist_type("rb_thread_t")
        .allowlist_type("rb_thread_struct")
        .allowlist_type("RString")
        .allowlist_type("ruby_method_ids")
        .allowlist_type("ruby_fl_type")
        .allowlist_type("ruby_fl_ushift")
        .allowlist_type("ruby_id_types")
        .allowlist_type("VALUE")
        .allowlist_type("vm_svar")
        .clang_args(vec![
            "-fdeclspec".to_string(),
            format!("-I{}", work_path.path().join("include").to_string_lossy()),
            format!("-I{}", work_path.path().join("ruby").to_string_lossy()),
            format!("-I{}", work_path.path().to_string_lossy()),
        ])
        .header(wrapper_path.to_string_lossy())
        .impl_debug(true)
        .merge_extern_blocks(true)
        .layout_tests(false)
        // Skip deriving Debug as a workaround for https://github.com/rust-lang/rust-bindgen/issues/2221
        .no_debug("iseq_catch_table")
        .no_debug("rb_method_definition_struct")
        .generate_comments(false)
        .generate();
    let bindings = match bindings {
        Ok(bindings) => bindings,
        Err(_) => Err(anyhow::anyhow!("couldn't generate bindings"))?,
    };

    let version_number = version_tag.trim_start_matches("v");
    let default_filename = format!("ruby_{}.rs", version_number);
    let output_filename = output_filename.unwrap_or(default_filename.as_str());
    let bindings_path = format!("ruby-structs/src/{}", output_filename);
    bindings.write_to_file(&bindings_path)?;

    postprocess_bindings_for_windows(&PathBuf::from(bindings_path.clone()))?;

    eprintln!("Wrote bindings to {}", bindings_path);
    Ok(())
}

/// Downloads the ruby source code, configures it, and builds the headers we need
fn prepare_ruby_source(path: &Path, version_tag: &str) -> Result<()> {
    if std::fs::create_dir(&path).is_ok() {
        if Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .status()
            .is_ok()
        {
            let status = Command::new("git")
                .args(vec![
                    "clone",
                    "https://github.com/ruby/ruby.git",
                    &path.to_string_lossy(),
                ])
                .status()?;
            if !status.success() {
                return Err(anyhow!("git clone failed"));
            }
        } else {
            return Err(anyhow!(
                "git does not appear to be installed, so can't clone ruby source"
            ));
        }
    }

    let status = Command::new("git")
        .args(vec!["checkout", "-f", version_tag])
        .current_dir(path)
        .status()
        .context("check out ruby repository")?;
    if !status.success() {
        return Err(anyhow!("failed to check out ruby repository ({})", status));
    }

    let status = Command::new("git")
        .args(vec!["clean", "-dfx"])
        .current_dir(path)
        .status()
        .context("clean ruby working copy")?;
    if !status.success() {
        return Err(anyhow!("failed to clean ruby working copy ({})", status));
    }

    let status = Command::new("autoreconf")
        .arg("--install")
        .current_dir(path)
        .status()
        .context("generate ruby configure script")?;
    if !status.success() {
        return Err(anyhow!(
            "failed to generate ruby configure script ({})",
            status
        ));
    }

    let status = Command::new("sh")
        .args(vec!["./configure", "--disable-install-doc"])
        .current_dir(path)
        .status()
        .context("configure ruby build")?;
    if !status.success() {
        return Err(anyhow!("failed to configure ruby build ({})", status));
    }

    // Build only the headers ("includes") to save time
    let status = Command::new("make")
        .args(vec!["-j4", "incs"])
        .current_dir(path)
        .status()
        .context("build ruby includes")?;
    if !status.success() {
        return Err(anyhow!("failed to build ruby includes ({})", status));
    }

    Ok(())
}

/// Copies any ruby headers we need to the given path
fn prepare_ruby_headers(path: &Path, ruby_source_path: &Path) -> Result<PathBuf> {
    copy_dir_recursive(ruby_source_path.join("include"), path.join("include"))?;
    let _ = copy_dir_recursive(ruby_source_path.join("internal"), path.join("internal"));
    let _ = copy_dir_recursive(ruby_source_path.join("ccan"), path.join("ccan"));

    let config_path = find_file(ruby_source_path.join(".ext"), &PathBuf::from("config.h"))?;
    log::info!("Found config.h at {}", config_path.to_string_lossy());
    std::fs::create_dir(path.join("ruby"))?;
    std::fs::copy(config_path, path.join("ruby").join("config.h"))?;

    for entry in std::fs::read_dir(ruby_source_path)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_file() && entry.file_name().to_string_lossy().ends_with(".h") {
            std::fs::copy(entry.path(), path.join(entry.file_name()))?;
        }
    }

    let wrapper_path = path.join("wrapper.h");
    let mut wrapper = std::fs::File::create(&wrapper_path)?;
    writeln!(wrapper, "#define RUBY_JMP_BUF sigjmp_buf")?;
    writeln!(wrapper, "#include \"{}/vm_core.h\"", path.to_string_lossy())?;
    writeln!(wrapper, "#include \"{}/iseq.h\"", path.to_string_lossy())?;

    return Ok(wrapper_path);
}

/// Workaround for types that won't compile on Windows targets
fn postprocess_bindings_for_windows(path: &Path) -> Result<()> {
    fn regex_replace(expr: &str, path: &Path) -> Result<()> {
        let status = Command::new("perl")
            .args(vec!["-pi", "-e", expr, &path.to_string_lossy()])
            .status()
            .context("postprocess ruby bindings")?;
        if !status.success() {
            return Err(anyhow!("failed to postprocess ruby bindings ({})", status));
        }

        Ok(())
    }

    regex_replace("s/::std::os::raw::c_ulong;/usize;/g", path)?;
    regex_replace("s/63u8\\) as u64/63u8\\) as usize/g", path)?;
    regex_replace("s/let val: u64 =/let val: usize =/g", path)?;
    regex_replace("s/let num_entries: u64 =/let num_entries: usize =/g", path)?;

    Ok(())
}

// Helper functions

fn find_file(haystack: impl AsRef<Path>, needle: &Path) -> Result<PathBuf> {
    for entry in std::fs::read_dir(haystack)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        log::debug!("find_file: considering {}", entry.path().to_string_lossy());
        if ty.is_dir() {
            return find_file(entry.path(), needle);
        } else if ty.is_file() && entry.file_name() == needle {
            return Ok(entry.path());
        }
    }

    Err(anyhow!("couldn't locate file"))
}

// https://stackoverflow.com/a/65192210
fn copy_dir_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
