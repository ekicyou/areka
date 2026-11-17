//! 字面の檻（`zorder_wiring_tests.rs`・`frame_schedule_tests.rs`）が `areka-runtime` と本体の両方で使う本文の下拵え（写しを作らない・spec: runtime-crate-split 設計「実装の中の討議の結果」1）。

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

/// 空白の連なりを 1 つに詰める（改行や字下げの入り方で檻が壊れないようにする）。
pub fn squeeze(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}
