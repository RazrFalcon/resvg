#[cfg(target_os = "windows")]
fn main() {
    use std::env;
    use std::path::Path;

    let skia_dir = env::var("SKIA_DIR").expect("SKIA_DIR is not set");
    let skia_path = Path::new(&skia_dir);

    let mut build = cc::Build::new();
    let tool = build.get_compiler();

    build.cpp(true);
    build.file("cpp/skia_capi.cpp").include("cpp");

    if env::var("SKIA_VER_M58").is_ok() {
        build.define("SKIA_VER_M58", None);
        build.include(skia_path.join("include").join("core"));
        build.include(skia_path.join("include").join("config"));
        build.include(skia_path.join("include").join("effects"));
    } else {
        build.include(skia_path);
    }

    if tool.is_like_msvc() {
        build.compile("libskiac.lib");
    } else {
        build.flag("-std=c++14");
        build.compile("libskiac.a");
    }

    let skia_lib_dir = env::var("SKIA_LIB_DIR").expect("SKIA_LIB_DIR is not set");
    let skia_lib_path = Path::new(&skia_lib_dir);

    println!("cargo:rustc-link-search={}", skia_lib_path.display()); // for MSVC
    println!("cargo:rustc-link-lib=skia.dll");
}

#[cfg(target_os = "linux")]
fn main() {
    use std::env;
    use std::path::Path;

    let skia_dir = env::var("SKIA_DIR").expect("SKIA_DIR is not set");
    let skia_path = Path::new(&skia_dir);

    let mut build = cc::Build::new();
    build.cpp(true);
    build.file("cpp/skia_capi.cpp").include("cpp");

    if env::var("SKIA_VER_M58").is_ok() {
        build.define("SKIA_VER_M58", None);
        build.include(skia_path.join("include").join("core"));
        build.include(skia_path.join("include").join("config"));
        build.include(skia_path.join("include").join("effects"));
    } else {
        build.include(skia_path);
    }

    build.flag("-std=c++14");
    build.compile("libskiac.a");

    let skia_lib_dir = env::var("SKIA_LIB_DIR").expect("SKIA_LIB_DIR is not set");
    let skia_lib_path = Path::new(&skia_lib_dir);

    println!("cargo:rustc-link-search={}", skia_lib_path.display());
    println!("cargo:rustc-link-lib=skia");
}
