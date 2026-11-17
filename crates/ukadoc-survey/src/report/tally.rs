//! 台帳の集計——状態の分布（全体・ページ別）・世代別・テーマ別（要件 7.1・7.8）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。台帳 1 本を
//! 受け取り、件数だけを返す。**本文は作らない**——見出しや表への組み立ては
//! `report::domain`（要件 7.1）と `report::summary`（要件 7.2）の担当で、ここは数を
//! 数えるところまでを受け持つ。
//!
//! # 入力はその台帳 1 本だけ（設計 D-11）
//!
//! カタログも証拠も要らない。ページ名は項目 id の 2 番目の区切りから取れるので
//! （[`crate::model::EntryId::page`]）、ページ別の分布にカタログは不要である。4 本の台帳を独立して
//! 編集できること（要件 3.4）は、ドメイン別報告がその台帳だけから決まることに載って
//! いる。
//!
//! # 並びは 2 通りに決め打つ（要件 7.3）
//!
//! 同じ入力なら 2 回続けて同じ答えになるよう、表はすべて [`BTreeMap`] で持ち、
//! 順序の定まらない入れ物は 1 つも使わない。
//!
//! - **状態**は要件 2.2 の語彙の順（[`Status::ALL`] の並び）。
//! - **ページ・世代・テーマ**は名前順（設計 report 節の事後条件）。
//!
//! # 0 件の欄を消さない
//!
//! 状態の分布は 7 語彙すべてを、件数 0 のものも欠かさず持つ。欄ごと消すと「0 件
//! だった」と「そもそも数えていない」が読み手には区別できない。ページ別の未分類 0 件が
//! 見えることは要件 6.9 が要る形でもある（未分類の件数は台帳側に宣言値を持たせず、
//! この分布を正とする）。テーマ別は台帳が使った名前だけを持つ。1 件も使われていない
//! テーマの 0 件の行は、報告の側が引数のテーマ名から並べる（[`theme_names`]）。
//!
//! # 呼び名は 1 か所でしか綴らない
//!
//! 報告に出す平易な日本語（要件 7.8）は [`Status::as_japanese`] が正本で、ここでは
//! 綴り直さない（[`StatusCounts::japanese_rows`] がそこから引く）。同じ語彙を 2 か所に
//! 綴ると、片方を並べ替える誤りが往復のテストを素通りする。
//!
//! # 表の見出しと行の道具もここに 1 つ
//!
//! ドメイン別報告と全体報告は同じ 7 語彙の列を持つ表を書く。見出しと行の書き方は
//! [`push_status_header`]・[`push_status_row`] の 1 か所に置き、並べるテーマ名の決め方
//! ([`theme_names`]) も同じ。

use std::collections::{BTreeMap, BTreeSet};

use crate::ledger::{Ledger, LedgerEntry};
use crate::model::{PageName, Status};

/// `introduced` が空のときの世代の名前（設計 report 節）。
///
/// 空文字は「最古」ではなく「分からない」である（要件 4.2）。だから最古の世代へ
/// 混ぜず、専用の欄に集める。
pub const UNKNOWN_GENERATION: &str = "世代不明";

/// 状態の分布。要件 2.2 の 7 語彙を、件数 0 のものも欠かさず同じ並びで持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCounts {
    /// 常に [`Status::ALL`] と同じ並び・同じ長さ。
    rows: Vec<(Status, usize)>,
}

impl Default for StatusCounts {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusCounts {
    /// 7 語彙すべてを 0 件で並べた分布を作る。
    ///
    /// 並びの正本は [`Status::ALL`] ただ 1 つ。ここで語彙を並べ直さない。
    pub fn new() -> Self {
        Self {
            rows: Status::ALL.into_iter().map(|status| (status, 0)).collect(),
        }
    }

    /// 1 件数える。
    pub fn add(&mut self, status: Status) {
        for row in &mut self.rows {
            if row.0 == status {
                row.1 += 1;
                return;
            }
        }
    }

    /// ある状態の件数。
    pub fn get(&self, status: Status) -> usize {
        self.rows
            .iter()
            .find(|row| row.0 == status)
            .map(|row| row.1)
            .unwrap_or_default()
    }

    /// 全状態の合計。
    pub fn total(&self) -> usize {
        self.rows.iter().map(|row| row.1).sum()
    }

    /// 別の分布の件数を状態ごとに足す。
    pub fn merge(&mut self, other: &StatusCounts) {
        for (row, (_, count)) in self.rows.iter_mut().zip(&other.rows) {
            row.1 += count;
        }
    }

    /// 要件 2.2 の順に並んだ「状態・件数」。
    pub fn rows(&self) -> &[(Status, usize)] {
        &self.rows
    }

    /// 要件 2.2 の順に並んだ「平易な日本語の呼び名・件数」（要件 7.8）。
    ///
    /// 呼び名は [`Status::as_japanese`] から引く。報告の本文はこれをそのまま使えば
    /// よく、呼び名を 2 か所で綴らずに済む。
    pub fn japanese_rows(&self) -> Vec<(&'static str, usize)> {
        self.rows
            .iter()
            .map(|(status, count)| (status.as_japanese(), *count))
            .collect()
    }
}

/// 台帳 1 本の集計（要件 7.1 の ⑴ ⑵ ⑷）。
///
/// 残る ⑶ 別名の一覧と ⑸ 束の一覧はここに含めない。前者は台帳の行をそのまま並べる
/// だけで数えるものが無く、後者は [`super::bundle`] の担当である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tally {
    /// ドメイン全体の状態の分布。
    pub overall: StatusCounts,
    /// ページ別の状態の分布（ページ名の順）。項目を 1 つも持たないページは現れない。
    pub by_page: BTreeMap<PageName, StatusCounts>,
    /// 世代別の状態の分布（世代名の順）。世代は [`generation_of`] が決める。
    pub by_generation: BTreeMap<String, StatusCounts>,
    /// テーマ別の状態の分布（テーマ名の順）。台帳が 1 件でも使ったテーマだけが現れる。
    pub by_theme: BTreeMap<String, StatusCounts>,
}

/// 台帳 1 本を集計する（要件 7.1・設計 D-11）。
pub fn tally(ledger: &Ledger) -> Tally {
    let overall = status_counts(ledger.entries.values());

    let mut by_page: BTreeMap<PageName, StatusCounts> = BTreeMap::new();
    let mut by_generation: BTreeMap<String, StatusCounts> = BTreeMap::new();
    let mut by_theme: BTreeMap<String, StatusCounts> = BTreeMap::new();

    for entry in ledger.entries.values() {
        // ページは id から取る。前置きの `pages` は見ない（設計 D-11。前置きと id の
        // 食い違いは整合検査の担当で、ここが黙って埋めると食い違いが見えなくなる）。
        by_page
            .entry(entry.id.page())
            .or_default()
            .add(entry.status);

        by_generation
            .entry(generation_of(&entry.introduced))
            .or_default()
            .add(entry.status);

        // 台帳の `values` は `ledger::read` が 8 テーマに限っている（要件 6.10）。
        // 知らない綴りも黙って捨てず、自分の欄を持たせる——捨てると手で組んだ台帳で
        // 件数だけが合わなくなる。
        for value in &entry.values {
            by_theme.entry(value.clone()).or_default().add(entry.status);
        }
    }

    Tally {
        overall,
        by_page,
        by_generation,
        by_theme,
    }
}

/// 項目の列の状態の分布を数える。
///
/// 台帳 1 本より細かい単位（全体報告のドメイン別・要件 7.2 など）でも使えるよう、
/// [`Ledger`] ではなく項目の列を受け取る。
pub fn status_counts<'a>(entries: impl Iterator<Item = &'a LedgerEntry>) -> StatusCounts {
    let mut counts = StatusCounts::new();
    for entry in entries {
        counts.add(entry.status);
    }
    counts
}

/// 7 語彙を列に並べた表の見出しを書く。
///
/// 列の名前は [`Status::as_japanese`] から引き、[`Status::ALL`] の順に並べる
/// （要件 2.2 の語彙の順・要件 7.8 の呼び名）。ここで綴り直さない。
pub(super) fn push_status_header(out: &mut String, first: &str) {
    out.push_str(&format!("| {first}"));
    for status in Status::ALL {
        out.push_str(&format!(" | {}", status.as_japanese()));
    }
    out.push_str(" | 合計 |\n| ---");
    // 1 列目を除く残り（7 語彙＋合計）はすべて右寄せ。
    for _ in 0..Status::ALL.len() + 1 {
        out.push_str(" | ---:");
    }
    out.push_str(" |\n");
}

/// 7 語彙を列に並べた表の 1 行を書く。
///
/// [`StatusCounts`] は 7 語彙すべてを持つので、0 件の列も欠けない。
pub(super) fn push_status_row(out: &mut String, label: &str, counts: &StatusCounts) {
    out.push_str(&format!("| {label}"));
    for (_, count) in counts.rows() {
        out.push_str(&format!(" | {count}"));
    }
    out.push_str(&format!(" | {} |\n", counts.total()));
}

/// 並べるテーマ名を名前順で決める。
///
/// 引数のテーマ名と、台帳が実際に使った名前の**両方**を並べる。引数だけを見ると台帳に
/// 書かれた見知らぬ名前の件数が消え、台帳だけを見ると 1 件も使われていないテーマの
/// 欄が消える。
pub(super) fn theme_names<'a>(
    by_theme: &'a BTreeMap<String, StatusCounts>,
    themes: &[&'a str],
) -> Vec<&'a str> {
    let mut names: BTreeSet<&str> = by_theme.keys().map(String::as_str).collect();
    names.extend(themes.iter().copied());
    names.into_iter().collect()
}

/// 登場した版から世代を取る（設計 report 節）。
///
/// 先頭 2 節を点で繋いだものが世代である（`2.3.53` → `2.3`）。節が 1 つしか無ければ
/// それをそのまま使い、空文字なら [`UNKNOWN_GENERATION`]。版番号として正しいかは
/// ここでは見ない——カタログの版番号との突き合わせは整合検査（要件 6.7）の担当で、
/// ここで独自に弾くと同じ判定が 2 か所に散る。
pub fn generation_of(introduced: &str) -> String {
    if introduced.is_empty() {
        return UNKNOWN_GENERATION.to_owned();
    }
    let mut segments = introduced.split('.');
    match (segments.next(), segments.next()) {
        (Some(major), Some(minor)) => format!("{major}.{minor}"),
        (Some(major), None) => major.to_owned(),
        // `split` は必ず 1 つ以上返すのでここには来ない。
        (None, _) => UNKNOWN_GENERATION.to_owned(),
    }
}

#[cfg(test)]
#[path = "tally_tests.rs"]
mod tests;
