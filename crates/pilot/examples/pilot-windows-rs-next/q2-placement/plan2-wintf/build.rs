// 案 2 の wintf 役の束縛（既定の形）を OUT_DIR へ書く（コミットしない）。emo-text 役の要る物（filters-for-emo.txt）まで抱える。
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=filters-wintf.txt");
    println!("cargo:rerun-if-changed=filters-for-emo.txt");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    windows_bindgen::builder()
        .output(&out)
        .filter_files(["filters-wintf.txt", "filters-for-emo.txt"])
        .flat()
        .write();
}
