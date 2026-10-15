// Compiles the C++17 kernels in cpp/ with the platform compiler (MSVC, GCC or
// Clang) found by the `cc` crate. Compiler warnings are errors. Floating-point
// contraction is disabled so results match the pure-Rust reference kernels.
//
// If no C++ compiler is available, the crate still builds: `cfg(apsi_native)`
// is left unset, `apsi_native::available()` reports false with a reason, and
// every call uses the Rust reference implementation.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(apsi_native)");
    println!("cargo::rerun-if-changed=cpp/apsi_kernels.cpp");
    println!("cargo::rerun-if-changed=cpp/apsi_kernels.hpp");
    println!("cargo::rerun-if-env-changed=APSI_NATIVE_DISABLE");

    if std::env::var_os("APSI_NATIVE_DISABLE").is_some() {
        println!("cargo::rustc-env=APSI_NATIVE_REASON=disabled by APSI_NATIVE_DISABLE at build time");
        return;
    }

    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").file("cpp/apsi_kernels.cpp").include("cpp").warnings(true).extra_warnings(true).warnings_into_errors(true);
    if build.get_compiler().is_like_msvc() {
        build.flag("/EHsc").flag("/fp:precise");
    } else {
        build.flag("-ffp-contract=off").flag("-pthread");
    }

    match build.try_compile("apsi_kernels") {
        Ok(()) => {
            println!("cargo::rustc-cfg=apsi_native");
            let compiler = build.get_compiler();
            let name = compiler.path().file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            println!("cargo::rustc-env=APSI_NATIVE_REASON=compiled with {name}");
        }
        Err(e) => {
            let msg = e.to_string().replace(['\n', '\r'], " ");
            println!("cargo::warning=apsi-native: C++ kernels not built ({msg}); using the Rust reference kernels");
            println!("cargo::rustc-env=APSI_NATIVE_REASON=no usable C++17 compiler at build time");
        }
    }
}
