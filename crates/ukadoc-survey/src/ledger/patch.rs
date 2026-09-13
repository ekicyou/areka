//! 既存の塊のバイト列を保ったまま `priority` の 1 行だけを置き換える（要件 7.1・12.2）。
//!
//! ここは純粋層で、ファイルには触らない。本文の文字列を受け取り、置き換えた本文を
//! 文字列で返すだけである（要件 6.2）。どこへ書くかは入出力層（`io::files::write_lf`）
//! の仕事である。
//!
//! # なぜ組み立て直さないのか
//!
//! 台帳は**人が手で書く文書**である。値を読んで TOML として書き出し直すと、備考の
//! 書き方も区切りの空白も空行も持ち主の書いたものと変わってしまう（`ledger::write`
//! と同じ理由・設計 D-12）。書き戻すのは `priority` だけなので、本文はバイト列のまま
//! 写し、当たった 1 行だけを差し替える。
//!
//! 塊の切れ目は [`crate::ledger::blocks::split`] に委ねる。切り分けは本文を隙間なく
//! 覆うので、前置きと塊を順に写せば触っていない部分は 1 バイトも動かない。
//!
//! # 行頭でなければ触らない
//!
//! 備考は複数行の文字列で、その中に `priority = ` で始まる行が書かれていることがある。
//! 置き換えの相手は**行頭**が `priority = ` で始まる行だけである。字下げされていれば
//! 備考の一部として写す。
//!
//! 逆に、備考の中の**行頭**に `priority = ` があると、塊の中で当たる行が 2 本になる。
//! どちらを直すべきかを機械が決めることはできないので、id を挙げて落ちる（黙って
//! 片方を選ぶと、人の書いた備考が書き換わるか、確定値が入らないまま緑になる）。
//!
//! # 呼ぶ側が負う前提
//!
//! 本文は**復帰文字を落としたもの**であること（[`crate::ledger::blocks::split`] の
//! 事前条件・設計 D-6）。ここで黙って落とすことはしない——落とすと返る本文が呼び出し
//! 側の持つ本文と行末で食い違い、差分が濁る。

use std::collections::BTreeMap;
use std::ops::Range;

use crate::error::SurveyError;
use crate::ledger::blocks;
use crate::model::EntryId;
use crate::tomlout::basic_string;

/// 失敗の本文に添える台帳の置き場。
///
/// ここは本文だけを受け取り、どのドメインの台帳を読んでいるかを知らない
/// （`blocks::split` が同じ形をしている）。ドメインの部分は伏せたまま置き場の形を
/// 添える。実際の綴りは `io::paths::ledger_path` が決める。
const LEDGER_FILE: &str = "doc/ukadoc-coverage/ledger/<ドメイン>.toml";

/// 置き換えの相手になる行の始まり。**行頭**でのみ当たる。
///
/// 綴りは要件付録 A.2 が凍結した書き方（欄名・空白・等号・空白）そのままである。
const PRIORITY_HEAD: &str = "priority = ";

/// 各塊の `priority = "…"` 行だけを `wanted` の値へ置き換えた本文を返す（要件 7.1）。
///
/// `wanted` に無い id の塊は 1 バイトも変えない。置換の必要が 0 件なら、返る本文は
/// 入力と 1 バイトも違わない（冪等）。
///
/// # 落ちる場合
///
/// - 本文を塊へ切り分けられない（[`crate::ledger::blocks::split`] の失敗をそのまま返す）
/// - 塊の中に行頭の `priority = ` が無い、または 2 本以上ある
///   （[`SurveyError::DeriveMismatch`]。本文に id を挙げる）
pub fn replace_priority(
    text: &str,
    wanted: &BTreeMap<EntryId, String>,
) -> Result<String, SurveyError> {
    let (prologue, blocks) = blocks::split(text)?;

    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..prologue]);
    for block in &blocks {
        let body = &text[block.start..block.end];
        // `wanted` に無い塊も行を数える。欄が欠けている（または備考の行頭に紛れている）
        // 台帳は次の書き戻しで黙って壊れるので、写す前に告げる。
        let line = priority_line(body, &block.id)?;
        match wanted.get(&block.id) {
            None => out.push_str(body),
            Some(value) => {
                out.push_str(&body[..line.start]);
                out.push_str(PRIORITY_HEAD);
                out.push_str(&basic_string(value));
                out.push_str(&body[line.end..]);
            }
        }
    }
    Ok(out)
}

/// 塊の中の `priority` 行の範囲（塊の先頭からのバイト位置・改行は含まない）を返す。
///
/// 塊の先頭は見出し行（`[entry.` で始まる）なので、見出しがここに当たることはない。
/// 当たる行がちょうど 1 本でなければ、どちらを直すべきかを決められないので落ちる。
fn priority_line(body: &str, id: &EntryId) -> Result<Range<usize>, SurveyError> {
    let mut found: Option<Range<usize>> = None;
    let mut offset = 0usize;
    for line in body.split('\n') {
        if line.starts_with(PRIORITY_HEAD) {
            if found.is_some() {
                return Err(undecidable(id, "priority の行が 2 本以上ある"));
            }
            found = Some(offset..offset + line.len());
        }
        // 割った改行 1 バイトを足し戻す（`blocks::scan_heads` と同じ数え方）。
        offset += line.len() + 1;
    }
    found.ok_or_else(|| undecidable(id, "priority の行が無い"))
}

/// 置き換える行を決められないことを告げる失敗。id を必ず添える（要件 12.6・6.12）。
fn undecidable(id: &EntryId, reason: &str) -> SurveyError {
    SurveyError::DeriveMismatch {
        file: LEDGER_FILE.to_owned(),
        reason: format!("項目 {} の {reason}", id.as_str()),
    }
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
