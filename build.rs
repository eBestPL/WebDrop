fn main() {
    println!("cargo:rerun-if-changed=native/modern_codecs.c");
    println!("cargo:rerun-if-env-changed=WEBDROP_BREW_PREFIX");
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    println!("cargo:rerun-if-env-changed=WEBDROP_NATIVE_PREFIX");
    println!("cargo:rerun-if-env-changed=WEBDROP_NATIVE_LIB_DIR");
    if target == "macos" {
        let prefix = std::env::var("WEBDROP_BREW_PREFIX").unwrap_or_else(|_| {
            if std::path::Path::new("/opt/homebrew").exists() {
                "/opt/homebrew".into()
            } else {
                "/usr/local".into()
            }
        });
        cc::Build::new()
            .file("native/modern_codecs.c")
            .include(format!("{prefix}/opt/libavif/include"))
            .include(format!("{prefix}/opt/jpeg-xl/include"))
            .compile("modern_codecs");
        for package in ["libavif", "jpeg-xl"] {
            println!("cargo:rustc-link-search=native={prefix}/opt/{package}/lib");
        }
        println!("cargo:rustc-link-arg=-Wl,-rpath,{prefix}/opt/jpeg-xl/lib");
    } else {
        let prefix = std::env::var("WEBDROP_NATIVE_PREFIX").unwrap_or_else(|_| "/usr".into());
        let library_dir =
            std::env::var("WEBDROP_NATIVE_LIB_DIR").unwrap_or_else(|_| format!("{prefix}/lib"));
        cc::Build::new()
            .file("native/modern_codecs.c")
            .include(format!("{prefix}/include"))
            .compile("modern_codecs");
        println!("cargo:rustc-link-search=native={library_dir}");
        if target == "linux" {
            println!("cargo:rustc-link-arg=-Wl,--disable-new-dtags,-rpath,$ORIGIN/lib");
        }
    }
    for library in ["avif", "jxl", "jxl_cms", "jxl_threads"] {
        println!("cargo:rustc-link-lib=dylib={library}");
    }
    println!("cargo:rerun-if-changed=macos/decode.m");
    println!("cargo:rerun-if-changed=macos/open_documents.m");
    println!("cargo:rerun-if-changed=macos/preferences.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("macos/decode.m")
            .file("macos/open_documents.m")
            .file("macos/preferences.m")
            .flag("-fobjc-arc")
            .compile("native_decode");
        for framework in ["Foundation", "AppKit", "CoreGraphics", "ImageIO"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}
