//! 翻訳（`OnTranslate`）のために外から渡す 2 つの関数の型。
//!
//! 運行は台詞の環境変数の値も MAKOTO の DLL も知らない。値の源（ghost）と MAKOTO の鎖
//! （後半の spec `areka-P0-makoto-dll-host`）は、それぞれ素のクロージャとして渡される
//! （[`crate::resources::ResourceSink`] と同じ疎結合の口）。口の引数は文字列だけで、
//! DLL・プロセス・文字コードの型を持たない（要件 7.4）。
//!
//! 運行表の側の翻訳（`schedule::translate`）とは別のモジュールで、`crate::change` と
//! `schedule::change` と同じ分け方をとる。

/// 台詞の環境変数を展開する（値の無い名前は綴りのまま返す）。
///
/// kanade のスレッドで同期に呼ばれ、返るまで運行は進まない。
pub type ScriptExpander = Box<dyn Fn(&str) -> String + Send>;

/// MAKOTO の鎖を差し込む口。引数は（台詞, 元のイベントの ID）。返すのは台詞（要件 7.1）。
///
/// kanade のスレッドで同期に呼ばれ、返るまで運行は進まない。DLL の鎖を差し込む実装は、
/// SHIORI の往復と同じく期限つきで返す責任を持つ。
pub type MakotoChain = Box<dyn Fn(&str, &str) -> String + Send>;

/// 翻訳のために外から渡す 2 つの関数（展開と MAKOTO の口）の束。
pub struct TranslateSeams {
    /// 台詞の環境変数の展開（`OnTranslate` の手前で呼ぶ）。
    pub expand: ScriptExpander,
    /// MAKOTO の鎖（`OnTranslate` の後・再生の前で呼ぶ）。
    pub makoto: MakotoChain,
}

impl TranslateSeams {
    /// 展開も MAKOTO も台詞をそのまま返す（口を渡さない構成・テストの既定・要件 7.2）。
    ///
    /// 本 spec が持つ MAKOTO の実装はこの素通し 1 つだけ（要件 7.4）。
    pub fn passthrough() -> Self {
        Self {
            expand: Box::new(str::to_owned),
            makoto: Box::new(|script, _source_id| script.to_owned()),
        }
    }
}

#[cfg(test)]
#[path = "translate_tests.rs"]
mod tests;
