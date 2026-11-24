//! areka-session のテストの exe にだけ、標準の部品（comctl32）の版 6 を使う申告として本体の
//! `../areka/areka.manifest` を埋める。
//!
//! 告知（`alert.rs`）が版 6 にしか無い `TaskDialogIndirect` を取り込むので、申告の無い exe は
//! 起動の時点で 0xc0000139（入口が見つからない）で落ちる。申告の正本は本体の `areka.manifest` で、写しは持たない。
//! この crate は lib だけなので、`rustc-link-arg` はこの crate のテストの exe にしか届かず、使う側の exe には
//! 届かない（`rustc-link-arg-tests` は結合テストが無い crate では Cargo が受け付けない）。
//! リンカが MSVC のときだけ出す。

fn main() {
    println!("cargo:rerun-if-changed=../areka/areka.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../areka/areka.manifest");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}
