//! 入れ子（element定義でサーフェスを部品として置く・areka 独自の語）の静的な事実。
//!
//! element定義のファイル名の欄が半角の数字だけのとき、その欄を画像のファイル名でなく
//! サーフェスの番号として読む（要件 1.1・1.2）。読み分けの実装は本モジュールの
//! [`element_kind`] 1 関数だけで、畳み込み（[`crate::fold`]）と焼く前の除外（下流の
//! `shell_target`）が同じ関数を呼ぶ。
//!
//! 記録は出さない。事実を値で返し、記録は fs を触る入口が読み込み 1 回につき 1 度だけ出す。

use areka_parsers::shell::ElementPath;

/// element定義が置くもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// 画像（今までどおり）。
    Image,
    /// サーフェスの番号（欄が半角の数字だけで、u32 に収まる）。
    Surface(u32),
    /// 欄は半角の数字だけだが u32 に収まらない。画像としては読まない（要件 1.9）。
    SurfaceOutOfRange,
}

/// 欄が空でなく、全部が半角の数字（0〜9）なら番号として読む。`0100` は 100。
/// 符号つき・全角の数字・拡張子つき・空は画像（要件 1.1・1.3・1.9）。
pub fn element_kind(path: &ElementPath) -> ElementKind {
    let s = path.as_str();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return ElementKind::Image;
    }
    // 数字だけが確かなので、失敗は桁あふれだけ（先頭の 0 は値に効かない）。
    match s.parse::<u32>() {
        Ok(id) => ElementKind::Surface(id),
        Err(_) => ElementKind::SurfaceOutOfRange,
    }
}

#[cfg(test)]
#[path = "nesting_kind_tests.rs"]
mod kind_tests;
