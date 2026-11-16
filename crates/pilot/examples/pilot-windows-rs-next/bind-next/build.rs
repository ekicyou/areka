// 共有の既定の形の束縛。filters/*.txt を全部読み、OUT_DIR へ書く（コミットしない）。
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let mut files: Vec<_> = std::fs::read_dir("filters")
        .expect("filters/")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
    }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    windows_bindgen::builder()
        .output(&out)
        .filter_files(&files)
        .write();
}
