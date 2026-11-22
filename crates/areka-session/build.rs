//! areka-session のテストの exe にだけ、標準の部品（comctl32）の版 6 を使う申告のマニフェスト
//! `tests.manifest` を埋める。
//!
//! 告知（`alert.rs`）が版 6 にしか無い `TaskDialogIndirect` を取り込むので、申告の無い exe は
//! 起動の時点で 0xc0000139（入口が見つからない）で落ちる。本番の申告は本体の exe が持つ。
//! この crate は lib だけなので、`rustc-link-arg` はこの crate のテストの exe にしか届かず、使う側の exe には
//! 届かない（`rustc-link-arg-tests` は結合テストが無い crate では Cargo が受け付けない）。
//! リンカが MSVC のときだけ出す。

fn main() {
    println!("cargo:rerun-if-changed=tests.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let manifest =
        std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("tests.manifest");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}
