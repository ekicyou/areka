//! pattern定義の描画メソッドの 8 語の見分けと、描画メソッドの語の綴りの比べ方（seriko-pattern-calls
//! 要件 1.1・1.2）。
//!
//! 8 語（`start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・
//! `parallelstop`・`insert`・`move`）の表と綴りの規則はツリーでここだけが持つ唯一の正本である。
//! emo-compose（合成メソッドの名前引き・再公開）と emo-atlas（マニフェスト）はここを引く。
//! 照会だけで、記録は出さない。

use std::borrow::Cow;

/// pattern定義の描画メソッドの語の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternWord {
    /// 8 語のどれでもない（重ね方の語・未知の語）。
    Draw,
    Start,
    Stop,
    AlternativeStart,
    AlternativeStop,
    ParallelStart,
    ParallelStop,
    Insert,
    Move,
}

/// 描画メソッドの語の正規化（前後の空白を落とし・小文字にし・`-` と `_` を除く）。
///
/// [`pattern_word`] と emo-compose の合成メソッドの名前引き（`ComposeMethod::from_name`・
/// `is_known_name`）が**同じ規則**で語を比べるための唯一の定義である。合成のたびに引く側は
/// 確保しない [`same_canonical`] で同じ規則を当てる。
///
/// もう正規化済みの語（ふつうの `overlay` など）は写さずに借りて返す。
pub fn canonical_method_name(name: &str) -> Cow<'_, str> {
    let name = name.trim();
    if name
        .bytes()
        .any(|b| b.is_ascii_uppercase() || b == b'-' || b == b'_')
    {
        Cow::Owned(name.to_ascii_lowercase().replace(['-', '_'], ""))
    } else {
        Cow::Borrowed(name)
    }
}

/// `name` を [`canonical_method_name`] と同じ規則で正規化した語が、正規化済みの語 `canon` と同じか
/// （確保しない・`canonical_method_name(name) == canon` と同じ答え）。
pub fn same_canonical(name: &str, canon: &str) -> bool {
    name.trim()
        .bytes()
        .filter(|&b| b != b'-' && b != b'_')
        .map(|b| b.to_ascii_lowercase())
        .eq(canon.bytes())
}

/// 描画メソッドの語が 8 語のどれかを答える（綴りの比べ方は [`canonical_method_name`]・記録は出さない）。
pub fn pattern_word(method: &str) -> PatternWord {
    match &*canonical_method_name(method) {
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#start_2cID:1
        "start" => PatternWord::Start,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#stop_2cID:1
        "stop" => PatternWord::Stop,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#alternativestart_2c_28ID1_2cID2..._29:1
        "alternativestart" => PatternWord::AlternativeStart,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#alternativestop_2c_28ID1_2cID2..._29:1
        "alternativestop" => PatternWord::AlternativeStop,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#parallelstart_2c_28ID1_2cID2..._29:1
        "parallelstart" => PatternWord::ParallelStart,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#parallelstop_2c_28ID1_2cID2..._29:1
        "parallelstop" => PatternWord::ParallelStop,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#insert_2cID:1
        "insert" => PatternWord::Insert,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#move:1
        "move" => PatternWord::Move,
        _ => PatternWord::Draw,
    }
}
