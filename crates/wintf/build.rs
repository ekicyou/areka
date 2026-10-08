//! wintf のサンプル（examples）の exe にだけ、標準の部品（comctl32）の版 6 を使う申告の
//! マニフェスト `examples.manifest` を埋める（areka-P0-wintf-tooltip 要件 3.15）。
//!
//! 指示は `rustc-link-arg-examples` で出すので、ライブラリ・テストの exe・wintf を使う側の exe には
//! 届かない。リンカが MSVC のときだけ出す。

fn main() {
    println!("cargo:rerun-if-changed=examples.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("examples.manifest");
    println!("cargo:rustc-link-arg-examples=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-examples=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
