// Die schlanke GNU-Toolchain von Rust bringt nicht alle Windows-Importbibliotheken mit.
// Fehlende (bisher nur libshlwapi.a, stammt aus dem Paket winapi-x86_64-pc-windows-gnu)
// liegen im Ordner `link-gnu` und werden hier dem Linker bekannt gemacht.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("link-gnu");
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    println!("cargo:rerun-if-changed=build.rs");
}
