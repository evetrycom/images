fn main() {
    // Link libvips menggunakan pkg-config
    // Ini memberi tahu rustc di mana mencari libvips.so
    println!("cargo:rerun-if-changed=build.rs");

    let _lib = pkg_config::Config::new()
        .atleast_version("8.0")
        .probe("vips")
        .expect("libvips tidak ditemukan. Pastikan libvips sudah terinstall dan pkg-config bisa menemukannya.");

    // Karena kode Rust memanggil g_object_unref dan g_free secara langsung,
    // kita perlu melink glib dan gobject secara eksplisit.
    pkg_config::Config::new().probe("glib-2.0").expect("glib-2.0 tidak ditemukan");
    pkg_config::Config::new().probe("gobject-2.0").expect("gobject-2.0 tidak ditemukan");

    println!("cargo:rerun-if-changed=build.rs");
}
