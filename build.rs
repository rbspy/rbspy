extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn get_bindings<'a>(version: &'a str) -> bindgen::Bindings {
    let underscore_version = version.replace(".", "_");
    bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .whitelist_type("rb_iseq_constant_body")
        .whitelist_type("rb_iseq_location_struct")
        .whitelist_type("rb_thread_struct")
        .whitelist_type("rb_iseq_struct")
        .whitelist_type("rb_control_frame_struct")
        .whitelist_type("rb_thread_struct")
        .whitelist_type("RString")
        .whitelist_type("VALUE")
        .impl_debug(true)
        .clang_arg(format!("-I/home/bork/scratch/ruby-header-files/{}/include", underscore_version))
        .clang_arg(format!("-I/home/bork/scratch/ruby-header-files/{}", underscore_version))
        .clang_arg("-I/home/bork/scratch/ruby-header-files/general")
        .generate_comments(false)
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings")
}

fn main() {
    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=bz2");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = get_bindings("2.1.6");
    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
