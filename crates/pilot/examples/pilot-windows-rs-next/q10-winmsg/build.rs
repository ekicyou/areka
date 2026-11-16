fn main() {
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    windows_bindgen::builder()
        .output(&out)
        .filter_file("filters.txt")
        .sys()
        .flat()
        .write();
}
