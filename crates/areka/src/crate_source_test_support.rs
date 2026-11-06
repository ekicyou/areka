//! 本体のテストが別の crate のファイルを読むときの、たった 1 つの読み口
//! （spec: placement-crate-split 要件 5.4・5.5）。
//!
//! 読んでよいのは、本体が依存する crate（依存の向きに下の crate）だけ。`krate` は `crates/` の下の
//! フォルダ名（`wintf`・`areka-placement` など）で書く。本体自身のファイルはここを通さず、相対の
//! `include_str!`／`CARGO_MANIFEST_DIR` で読む（規則は「自分の crate は相対・別の crate は読み口」の 1 つ）。
//! 例外は名前を付けたワークスペース全体を歩く見張りの 3 本（`session_end_sync_send_tests.rs`・
//! `emo2_boot/frame_attach_tests.rs`・`readonly_tests.rs`）だけ。
//!
//! どちらの口も読み先が無ければ黙って通らない: `crate_source!` はコンパイルが止まり、
//! `crate_path` は panic する。

use std::path::PathBuf;

/// `crates/<krate>/<path>` の中身を、コンパイルの時に読む（無ければコンパイルが止まる）。
macro_rules! crate_source {
    ($krate:literal, $path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../",
            $krate,
            "/",
            $path
        ))
    };
}
pub(crate) use crate_source;

/// `crates/<krate>/<path>` の道を返す。在ることを確かめ、無ければ panic する（要件 5.5）。
pub(crate) fn crate_path(krate: &str, path: &str) -> PathBuf {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(krate)
        .join(path);
    assert!(
        full.exists(),
        "下の crate の読み先が無い（置き場所が変わったか）: {}",
        full.display()
    );
    full
}

mod tests {
    use super::*;

    /// 在る読み先は道を返し、マクロは中身を読む。
    #[test]
    fn the_reader_reads_a_lower_crate() {
        assert!(crate_path("wintf", "src/lib.rs").is_file());
        assert!(crate_source!("wintf", "Cargo.toml").contains("name = \"wintf\""));
    }

    /// 無い読み先は黙って通らない（panic で止まる）。
    #[test]
    #[should_panic(expected = "下の crate の読み先が無い")]
    fn a_missing_path_stops_with_a_panic() {
        crate_path("wintf", "src/no_such_file.rs");
    }
}
