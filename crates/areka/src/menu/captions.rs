//! メニューの項目名と表示可否の照会（areka-P0-popup-menu-minimal）。
//!
//! 枠ごとの SHIORI リソース名の表、1 回の表示で問い合わせる名前の列挙、kanade への
//! 照会の送出、返り値から項目名への写し、`popupmenu.visible` による表示可否の判定を置く。

use std::collections::HashMap;

/// 照会で決まった項目名の表。非空の文言だけを持つ（空・値なし・失敗は既定名に落ちるので
/// この表に入らない＝引けなかった名前はそのまま「既定名を使う」を意味する）。
#[derive(Default)]
pub(crate) struct CaptionMap(HashMap<&'static str, String>);

impl CaptionMap {
    /// リソース名に対応する文言を足す。空文字列は入れない（要件 3.3）。
    pub(crate) fn insert(&mut self, id: &'static str, caption: String) {
        if !caption.is_empty() {
            self.0.insert(id, caption);
        }
    }

    /// リソース名に対応する文言。無ければ `None`（呼び手は既定名を使う）。
    pub(crate) fn get(&self, id: &str) -> Option<&str> {
        self.0.get(id).map(String::as_str)
    }
}
