use std::env;
use std::path::PathBuf;
extern crate windows_exe_info;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    

    println!("cargo:rustc-link-search=native=./");
    println!("cargo:rustc-link-lib=pdfium");
    // println!("cargo:rustc-link-lib=dylib=pdfium");
    println!("cargo:rustc-link-lib=static=pdfium");
    println!("cargo:rerun-if-changed=wrapper_pdfium.h");
    println!("cargo:rerun-if-changed=wrapper_cups.h");


    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("Out path: {:?}", out_path);

        let bindings_pdfium = bindgen::Builder::default()
            .header("wrapper_pdfium.h")
            .clang_arg("-I./include")
            .clang_arg(r"-IC:\Users\User\Downloads\clang+llvm-20.1.5-x86_64-pc-windows-msvc\lib\clang\20\include")
            .clang_arg(r"-IC:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um")
            .clang_arg(r"-IC:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared")
            .clang_arg(r"-IC:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\ucrt")
            .clang_arg(r"-IC:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\winrt")
            .clang_arg(r"-IC:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\include")
            .generate()
            .expect("Failed to bind to pdfium");

        bindings_pdfium.write_to_file(out_path.join("bindings_pdfium.rs"))
        .expect("Couldn't write pdfium bindings");



    if target_os == "macos" {
        println!("Building for macOS");

        println!("cargo:rustc-link-lib=cups");

        let sdk_path = std::process::Command::new("xcrun")
            .args(&["--sdk", "macosx", "--show-sdk-path"])
            .output()
            .expect("xcrun failed")
            .stdout;

        let sdk_root = String::from_utf8(sdk_path).expect("Invalid UTF-8 in SDK path");
        let sdk_root = sdk_root.trim();


        let bindings = bindgen::Builder::default()
            .header("wrapper_cups.h")
            .clang_arg("-I./include")
            .clang_arg(format!("-isysroot{}", sdk_root))
            // .clang_arg("-I/usr/include")
            .generate()
            .expect("Unable to generate bindings");

        
        bindings
            .write_to_file(out_path.join("bindings_cups.rs"))
            .expect("Couldn't write cups bindings!");
    }

    if target_os == "windows" {
        println!("Building for Windows");

        windows_exe_info::icon::icon_ico("./assets/app_icon.ico");


        
    }
}
