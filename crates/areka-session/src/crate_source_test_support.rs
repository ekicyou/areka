//! テストが別の crate のファイルを読むときの読み口（`areka-session` と本体のテストが使う・中身は `areka-runtime` の `crate_path` 1 つ）。
//!
//! 読んでよいのは、読む側の crate が依存する crate（依存の向きに下の crate）だけ。`krate` は `crates/` の下の
//! フォルダ名（`wintf`・`areka-placement` など）で書く。読む側の crate 自身のファイルはここを通さず、相対の
//! `include_str!`／`CARGO_MANIFEST_DIR` で読む（規則は「自分の crate は相対・別の crate は読み口」の 1 つ）。
//! 例外は名前を付けたワークスペース全体を歩く見張りの 1 本（`areka-runtime` の `src/frame_attach_tests.rs`）だけ。
//!
//! どちらの口も読み先が無ければ黙って通らない: `crate_source!` はコンパイルが止まり、
//! `crate_path` は panic する。

/// `crates/<krate>/<path>` の中身を、コンパイルの時に読む（無ければコンパイルが止まる）。
#[macro_export]
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

/// 読み口の中身は `areka-runtime` の `crate_path`（ワークスペースのテストで 1 つ）。
pub use areka_runtime::source_text_test_support::crate_path;

#[cfg(test)]
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
