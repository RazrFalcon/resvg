#[cfg(all(unix, not(target_os = "macos")))]
fn main() {
    let mut build = cc::Build::new();
    build.cpp(true);
    build.flag("-std=c++11");
    build.file("qt-bindings/qt_capi.cpp").include("qt-bindings");

    let lib = pkg_config::find_library("Qt5Gui").expect("Unable to find Qt5Gui");
    for path in lib.include_paths {
        build.include(path.to_str().expect("Failed to convert include path to str"));
    }

    build.compile("libqtc.a");
}

#[cfg(target_os = "windows")]
fn main() {
    let qt_dir = std::env::var("QT_DIR").expect("QT_DIR is not set");
    let qt_path = std::path::Path::new(&qt_dir);

    let mut build = cc::Build::new();
    let tool = build.get_compiler();

    build.cpp(true);
    build.file("qt-bindings/qt_capi.cpp").include("cpp");

    build.include(qt_path.join("include"));
    build.include(qt_path.join("include").join("QtCore"));
    build.include(qt_path.join("include").join("QtGui"));

    if tool.is_like_msvc() {
        build.compile("libqtc.lib");
    } else {
        build.flag("-std=c++11");
        build.compile("libqtc.a");
    }

    println!("cargo:rustc-link-search={}", qt_path.join("bin").display()); // for MinGW
    println!("cargo:rustc-link-search={}", qt_path.join("lib").display()); // for MSVC

    println!("cargo:rustc-link-lib=Qt5Core");
    println!("cargo:rustc-link-lib=Qt5Gui");
}

#[cfg(target_os = "macos")]
fn main() {
    let qt_dir = std::env::var("QT_DIR").expect("QT_DIR is not set");
    let qt_path = std::path::Path::new(&qt_dir);

    let mut build = cc::Build::new();
    build.cpp(true);
    build.flag("-std=c++11");
    build.flag(&format!("-F{}/lib", qt_dir));
    build.file("qt-bindings/qt_capi.cpp").include("cpp");

    build.include(qt_path.join("lib/QtGui.framework/Headers"));
    build.include(qt_path.join("lib/QtCore.framework/Headers"));

    build.compile("libqtc.a");

    println!("cargo:rustc-link-search=framework={}/lib", qt_dir);
    println!("cargo:rustc-link-lib=framework=QtCore");
    println!("cargo:rustc-link-lib=framework=QtGui");
}
