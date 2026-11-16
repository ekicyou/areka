// 既定の形の束縛を OUT_DIR へ書く（コミットしない）。filters.txt は 1 行 1 つの名指し。
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=filters.txt");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    windows_bindgen::builder()
        .output(&out)
        .filter_file("filters.txt")
        .flat()
        .write();
}
