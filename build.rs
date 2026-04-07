fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Try pkg-config first (works locally via Homebrew/system packages).
    // In Docker, PKG_CONFIG_PATH=/vips/lib/pkgconfig is set so this also works there.
    let vips_lib_dir = if let Ok(lib) = pkg_config::Config::new()
        .atleast_version("8.0")
        .probe("vips")
    {
        // pkg-config found vips - get its lib dir for rpath
        lib.link_paths
            .first()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/vips/lib".to_string())
    } else {
        // Fallback: manually specify the path.
        // Can be overridden via VIPS_LIB_DIR for non-standard installations.
        let dir = std::env::var("VIPS_LIB_DIR").unwrap_or_else(|_| "/vips/lib".to_string());
        println!("cargo:rustc-link-search=native={}", dir);
        println!("cargo:rustc-link-lib=dylib=vips");
        dir
    };

    // Embed rpath so the binary finds libvips at runtime without LD_LIBRARY_PATH.
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", vips_lib_dir);

    // Always explicitly link glib/gobject since we call g_object_unref / g_free directly.
    let glib_lib_dir = if let Ok(lib) = pkg_config::Config::new().probe("glib-2.0") {
        lib.link_paths
            .first()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        println!("cargo:rustc-link-lib=dylib=glib-2.0");
        String::new()
    };

    if let Err(_) = pkg_config::Config::new().probe("gobject-2.0") {
        println!("cargo:rustc-link-lib=dylib=gobject-2.0");
    }

    // Embed rpath for glib too if we found it
    if !glib_lib_dir.is_empty() {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", glib_lib_dir);
    }
}
