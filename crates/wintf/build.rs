//! wintf のサンプル（examples）の exe にだけ、標準の部品（comctl32）の版 6 を使う申告の
//! マニフェスト `examples.manifest` を埋める（areka-P0-wintf-tooltip 要件 3.15）。
//!
//! 指示は `rustc-link-arg-examples` で出すので、ライブラリ・テストの exe・wintf を使う側の exe には
//! 届かない。リンカが MSVC のときだけ出す。
//!
//! テストの `wic_test.rs` が読む areka-emo-atlas のパレットの見本が在るとき（ワークスペースの中）だけ
//! `cfg(areka_palette_samples)` を立てる。無いとき（crates.io から取った wintf など）は 6 本を「無視」にする。

fn main() {
    println!("cargo:rerun-if-changed=examples.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(areka_palette_samples)");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    if std::path::Path::new(&manifest_dir)
        .join("../areka-emo-atlas/src/testdata/palette")
        .is_dir()
    {
        println!("cargo:rustc-cfg=areka_palette_samples");
    }
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let manifest = std::path::Path::new(&manifest_dir).join("examples.manifest");
    println!("cargo:rustc-link-arg-examples=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-examples=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
