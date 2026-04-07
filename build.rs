fn main() {
    // Link libvips menggunakan pkg-config
    // Ini memberi tahu rustc di mana mencari libvips.so
    println!("cargo:rerun-if-changed=build.rs");

    let lib = pkg_config::Config::new()
        .atleast_version("8.0")
        .probe("vips")
        .expect("libvips tidak ditemukan. Pastikan libvips sudah terinstall dan pkg-config bisa menemukannya.");

    // Tambahan: ekspor include paths agar bisa digunakan jika perlu di masa depan
    for path in &lib.include_paths {
        println!("cargo:include={}", path.display());
    }
}
