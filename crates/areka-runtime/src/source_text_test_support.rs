//! `areka-runtime` と本体のテストが使う本文の下拵えとファイルの読み口（写しを作らない・spec: runtime-crate-split 設計「実装の中の討議の結果」1）。

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
pub fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 字面の現れる位置（無ければその場で落として、檻の前提が崩れたことを名指しで告げる）。
///
/// 檻が「在る・無い」だけでなく**前後関係**を主張するための道具である。
pub fn index_of(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("檻の錨が本文に見当たらない（前提が崩れている）: {needle}"))
}

/// `crates/<krate>/<path>` の道を返す。在ることを確かめ、無ければ panic する（要件 5.5）。
///
/// 本体と `areka-runtime` のテストが別の crate のファイルを読むときの、たった 1 つの読み口
/// （`CARGO_MANIFEST_DIR` は両方で `crates/<自分>` なので、`..` の先は同じフォルダ）。
pub fn crate_path(krate: &str, path: &str) -> std::path::PathBuf {
    let full = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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

/// 空白の連なりを 1 つに詰める（改行や字下げの入り方で檻が壊れないようにする）。
pub fn squeeze(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}
