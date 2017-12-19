extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=bz2");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .whitelisted_type("rb_iseq_constant_body")
        .whitelisted_type("rb_iseq_location_struct")
        .whitelisted_type("rb_thread_struct")
        .whitelisted_type("rb_iseq_struct")
        .whitelisted_type("rb_control_frame_struct")
        .whitelisted_type("rb_thread_struct")
        .whitelisted_type("VALUE")
        .clang_arg("-I/home/bork/.rbenv/versions/2.1.6/include/ruby-2.1.0/")
        .clang_arg("-I/home/bork/.rbenv/versions/2.1.6/include/ruby-2.1.0/x86_64-linux")
        .generate_comments(false)
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
