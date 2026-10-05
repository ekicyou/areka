//! areka.exe（bin）にだけ、標準の部品（comctl32）の版 6 を使う申告のマニフェスト
//! `areka.manifest` を埋める（areka-P0-wintf-tooltip 要件 3.15）。
//!
//! 指示は `rustc-link-arg-bins` で出すので、サンプルと結合テスト（`tests/`）の exe には届かない。
//! ただし Cargo は bin の指示を bin のユニットテストの exe にも当てるので、そちらにも入る
//! （build.rs からは見分けられない。areka のユニットテストは版 6 の下で走る）。
//! リンカが MSVC のときだけ出す。

fn main() {
    println!("cargo:rerun-if-changed=areka.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let manifest =
        std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("areka.manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
