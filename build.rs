fn main() {
    println!("cargo:rustc-link-lib=cups");

    let bindings = bindgen::Builder::default()
        .header("/Library/Developer/CommandLineTools/SDKs/MacOSX14.4.sdk/usr/include/cups/cups.h")
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file("src/cups_bindings.rs")
        .expect("Couldn't write bindings!");
}