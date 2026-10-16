//! areka.exe（bin）にだけ、標準の部品（comctl32）の版 6 を使う申告のマニフェスト
//! `areka.manifest` を埋める（areka-P0-wintf-tooltip 要件 3.15）。
//!
//! 指示は `rustc-link-arg-bins` で出すので、サンプルと結合テスト（`tests/`）の exe には届かない。
//! ただし Cargo は bin の指示を bin のユニットテストの exe にも当てるので、そちらにも入る
//! （build.rs からは見分けられない。areka のユニットテストは版 6 の下で走る）。
//! リンカが MSVC のときだけ出す。
//!
//! アプリのアイコンと通知領域用のアイコンも同じ道で入れる（areka-P0-app-icon 要件 3.1・3.2）。
//! 作り直しの道具 `tools/app-icon.ps1` が作ってコミットした資源のファイル `icon/icons.res` を
//! そのまま bin のリンクに渡すだけで、ここでは資源を組まない。資源の番号の表は `icon/README.md` にある。

fn main() {
    println!("cargo:rerun-if-changed=areka.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=icon/icons.res");
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
    let icons = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("icon")
        .join("icons.res");
    println!("cargo:rustc-link-arg-bins={}", icons.display());
}
