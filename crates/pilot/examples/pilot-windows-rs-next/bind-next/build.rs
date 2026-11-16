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
        // 問い 6 の下線の描き手（6.2）。名前空間は `Windows.Win32` の 1 段（DirectWrite の段は無い）。合わない名前は黙って無視される。
        .implements([
            "Windows.Win32.IDWriteTextRenderer1",
            "Windows.Win32.IDWriteTextRenderer",
            "Windows.Win32.IDWritePixelSnapping",
        ])
        .write();
}
