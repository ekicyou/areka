// 案 3 の共有の束縛（既定の形）を OUT_DIR へ書く（コミットしない）。
// フィルターは両役の和: wintf 役の filters-wintf.txt と emo-text 役の filters-emo.txt を両方読む。
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=filters-wintf.txt");
    println!("cargo:rerun-if-changed=filters-emo.txt");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    windows_bindgen::builder()
        .output(&out)
        .filter_files(["filters-wintf.txt", "filters-emo.txt"])
        .flat()
        .write();
}
