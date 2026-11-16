// 既定の形と .minimal() の形の束縛を別々の OUT_DIR ファイルへ書く（コミットしない）。
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let filter = ["HWND", "GetDesktopWindow", "IsWindow"];
    windows_bindgen::builder()
        .output(out.join("hwnd_default.rs"))
        .filters(filter)
        .flat()
        .write();
    windows_bindgen::builder()
        .output(out.join("hwnd_minimal.rs"))
        .filters(filter)
        .flat()
        .minimal()
        .write();
}
