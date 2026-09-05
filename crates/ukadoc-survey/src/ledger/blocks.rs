//! 台帳本文を「前置き」と「項目ごとの塊」に切り分ける（要件 2.1・3.3a・設計 D-12）。
//!
//! 台帳は**人が手で書く文書**である。欠けた id を差し込むとき（要件 3.3a）、値を読んで
//! 組み立て直すと備考の書き方も空行も変わってしまう。既存の本文をそのまま残すには
//! 本文を切り貼りするしかないので、その切れ目をここが与える（設計 D-12）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない。文字列を受け取り、
//! その中のバイト位置を返すだけである（要件 6.2）。
//!
//! # 何を返すか
//!
//! [`split`] は前置きの終端のバイト位置と、塊の一覧を返す。塊は本文を隙間なく覆う——
//! 前置きに続けて塊を順に繋ぐと、元の本文に 1 バイトも違わず戻る。差し込みはこの
//! バイト列をそのまま写すので、位置が 1 つずれると手書きの台帳が黙って壊れる。
//!
//! # 切れ目の決め方
//!
//! 塊の始まりは**行頭**の `[entry.` で、終わりは次の塊の始まりの直前（または本文の
//! 終端）である（設計 `ledger` 節の実装上の注意）。判定は行単位で行うので、備考の
//! 複数行文字列の中に字下げして書かれた見出しらしき行は塊を始めない。
//!
//! 見出し行そのものの読み取りは `toml` に任せる。逃がしを自前で解くと、付録 A.3 が
//! 定める逆斜線 2 つ重ねの綴り（`\\![get,property,ID]`）を `toml` と違う形にほどく
//! 危険がある。行を 1 つの TOML 文書として読ませれば、鍵の綴りは必ず `toml` と揃う。
//!
//! # 較正——自前の走査が壊れていないことを別の道具で示す
//!
//! 備考の複数行文字列の中に**行頭**の見出しらしき行があると、行だけを見る走査は
//! そこで塊を割ってしまう。これを見抜くために、[`split`] は同じ本文を `toml` で読んだ
//! `entry` の鍵の集合と、自分が切り出した id の集合を突き合わせる。食い違えば
//! [`SurveyError::LedgerSplitMismatch`] で落ちる（設計 `ledger` 節の事後条件・
//! Error Handling の「契約違反（構造）」）。
//!
//! **この較正には盲点がある**（設計 D-12）。備考の中の見出しが既にある id と同じ綴り
//! だった場合、集合としては一致してしまうので [`split`] は素通りする。塞ぐのはここ
//! ではなく、台帳の並びを**厳密な昇順**で確かめる側（[`SurveyError::LedgerOutOfOrder`]・
//! `ledger::read` と整合検査）で、同じ id が 2 度現れれば重複として落ちる。
//!
//! # 呼ぶ側が負う前提
//!
//! 本文は**復帰文字を落としたもの**であること（設計 `ledger` 節の事前条件・D-6。
//! `io::files::read_normalized` がそれを行う）。ここで黙って落とすことはしない——
//! 落とすと返すバイト位置が呼び出し側の持つ本文と食い違い、差し込みが本文を壊す。
//! 見出し行に復帰文字が残っていれば `toml` として読めず、その行を挙げて落ちる。
//! 備考の中だけに残っている場合は素通りする——返すバイト位置は渡された本文の
//! とおりなので切り貼りは壊れないが、落とすのはあくまで呼ぶ側の仕事である。

use std::collections::BTreeSet;

use crate::error::SurveyError;
use crate::model::EntryId;

/// 失敗の本文に添える台帳の置き場。
///
/// 設計の [`split`] は本文だけを受け取り、どのドメインの台帳を読んでいるかを知らない
/// （`catalog::read` が同じ形をしている）。ドメインの部分は伏せたまま、置き場の形を
/// 添える。実際の綴りは `io::paths::ledger_path` が決める。
const LEDGER_FILE: &str = "doc/ukadoc-coverage/ledger/<ドメイン>.toml";

/// 塊の始まりの印。**行頭**でのみ塊を始める。
const BLOCK_HEAD: &str = "[entry.";

/// 項目を置く表の名前。
const ENTRY_TABLE: &str = "entry";

/// 台帳本文の中の項目 1 つ分の塊。
///
/// `start` と `end` は**本文中のバイト位置**で、`text[start..end]` がその項目の
/// 見出し行から次の塊の直前までを丸ごと指す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// 見出し行に書かれていた項目 id（逃がしを解いた綴り）。
    pub id: EntryId,
    /// 塊の始まり（見出し行の先頭）。
    pub start: usize,
    /// 塊の終わり（次の塊の始まり、または本文の終端）。
    pub end: usize,
}

/// 台帳本文を前置きと項目ごとの塊に切り分ける（要件 2.1・設計 D-12）。
///
/// 返るのは前置きの終端のバイト位置と、本文に現れた順の塊の一覧である。項目が 1 つも
/// 無ければ前置きは本文の全部（`text.len()`）で塊は空、本文が見出し行で始まれば
/// 前置きは 0 になる。
///
/// # 落ちる場合
///
/// - 見出し行が TOML として読めない（[`SurveyError::TomlParse`]）
/// - 見出しの鍵が項目 id の 2 形のどちらでもない（[`SurveyError::BadEntryId`]）
/// - 本文全体が TOML として読めず、較正の相手を作れない（[`SurveyError::TomlParse`]）
/// - 切り分けた id の集合が `toml` の読んだ鍵の集合と食い違う
///   （[`SurveyError::LedgerSplitMismatch`]）
pub fn split(text: &str) -> Result<(usize, Vec<Block>), SurveyError> {
    let heads = scan_heads(text)?;
    let prologue = heads.first().map_or(text.len(), |(start, _)| *start);

    let mut blocks = Vec::with_capacity(heads.len());
    for (index, (start, id)) in heads.iter().enumerate() {
        let end = heads.get(index + 1).map_or(text.len(), |(next, _)| *next);
        blocks.push(Block {
            id: id.clone(),
            start: *start,
            end,
        });
    }

    calibrate(text, &blocks)?;
    Ok((prologue, blocks))
}

/// 行単位に走って、塊の始まりのバイト位置と項目 id を集める。
///
/// 改行で割った各行の長さを足し上げるので、位置は本文のバイト位置そのものになる
/// （末尾に改行が無い本文でも最後の行が正しく数えられる）。
fn scan_heads(text: &str) -> Result<Vec<(usize, EntryId)>, SurveyError> {
    let mut heads = Vec::new();
    let mut offset = 0usize;
    for line in text.split('\n') {
        if line.starts_with(BLOCK_HEAD) {
            heads.push((offset, header_id(line)?));
        }
        // 割った改行 1 バイトを足し戻す。最後の行の後ろには改行が無いが、その足し過ぎた
        // 分は誰も読まない（走査はここで終わる）。
        offset += line.len() + 1;
    }
    Ok(heads)
}

/// 見出し行 1 本を 1 つの TOML 文書として読み、項目 id を取り出す。
///
/// 逃がしの解き方を `toml` に委ねるのが要点である（付録 A.3 の逆斜線 2 つ重ね）。
/// 自前でほどくと、較正の相手である `toml` と綴りが割れる余地が残る。
fn header_id(line: &str) -> Result<EntryId, SurveyError> {
    let table: toml::Table = line
        .parse()
        .map_err(|err| unreadable(format!("塊の見出しが読めない: {line}（{err}）")))?;
    let entry = table
        .get(ENTRY_TABLE)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| unreadable(format!("塊の見出しに [entry.…] が無い: {line}")))?;
    let key = entry
        .keys()
        .next()
        .ok_or_else(|| unreadable(format!("塊の見出しに項目 id が無い: {line}")))?;
    // 2 形のどちらでもなければ、ここで `BadEntryId` になる（要件 1.9）。
    EntryId::parse(key)
}

/// 自前の走査を `toml` の読み取りで較正する（設計 `ledger` 節の事後条件）。
///
/// 集合として比べる。順序も重複も見ない——重複を見抜くのは並びを厳密な昇順で確かめる
/// 側の仕事である（設計 D-12）。
fn calibrate(text: &str, blocks: &[Block]) -> Result<(), SurveyError> {
    let root: toml::Table = text
        .parse()
        .map_err(|err| unreadable(format!("TOML として読めない: {err}")))?;
    let read: BTreeSet<&str> = match root.get(ENTRY_TABLE) {
        // `[entry]` が 1 つも無い台帳は項目 0 件。塊も 0 個なら一致する。
        None => BTreeSet::new(),
        Some(value) => value
            .as_table()
            .ok_or_else(|| unreadable("[entry] が表でない".to_owned()))?
            .keys()
            .map(String::as_str)
            .collect(),
    };
    let cut: BTreeSet<&str> = blocks.iter().map(|block| block.id.as_str()).collect();

    if cut == read {
        return Ok(());
    }
    Err(SurveyError::LedgerSplitMismatch {
        detail: format!(
            "切り分けだけにある id: [{}]／読み取りだけにある id: [{}]",
            join(cut.difference(&read)),
            join(read.difference(&cut)),
        ),
    })
}

/// 食い違った id を読める形に並べる。
fn join<'a>(ids: impl Iterator<Item = &'a &'a str>) -> String {
    ids.copied().collect::<Vec<&str>>().join(", ")
}

/// 台帳の本文が読めないことを告げる失敗。置き場を必ず添える（要件 6.12）。
fn unreadable(reason: String) -> SurveyError {
    SurveyError::TomlParse {
        path: LEDGER_FILE.to_owned(),
        reason,
    }
}

#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;
