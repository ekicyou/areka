// 案 1 の emo-text 役の束縛（既定の形）を OUT_DIR へ書く（コミットしない）。フィルターは emo-text 役の物だけ。
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=filters-emo.txt");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    windows_bindgen::builder()
        .output(&out)
        .filter_files(["filters-emo.txt"])
        .flat()
        .write();
}
