//! スナップショット更新時の差分（要件 8）。
//!
//! 正典のスナップショットが新しくなったとき、何が増えて何が消えて何の本文が
//! 変わったかを id 付きで挙げる（要件 8.1）。台帳を見直す範囲を絞るための道具で
//! ある。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。受け取る
//! のは**すでに組み上がった 2 つのカタログ**と台帳で、返すのは 4 つの id の一覧
//! だけ。新しいカタログを作る側（スナップショットを読む段）は入出力層の担当で、
//! この関数はその成果を値として受け取る。
//!
//! # 常時走るテストからは呼べない（要件 8.4）
//!
//! `tests/consistency*` の一式はこの関数を呼んではならない。呼ぶには「新しい
//! カタログ」が要り、それを作るにはスナップショット（npm グローバルの JSON）が
//! 要る。手元にその JSON が無い環境では作れないので、常時走るテストが差分に
//! 依存すると、環境の違いだけでテストが赤くなってしまう。差分はスナップショットが
//! 手元にあるときの作業（実行ファイルの副手続き）として扱う。
//!
//! そこで [`diff`] と [`CatalogDiff`] は **`pub(crate)`** にしてある。
//! `tests/consistency*` はライブラリの外にある別クレートなので、これで要件 8.4 を
//! 申し合わせではなく型検査が守る。タスク 1.7 が `io::snapshot` に同じ形を採った
//! 前例に倣う（`io/mod.rs` の注記）。実行ファイル側の `cli::inspect` はライブラリの
//! 中にあるので、そのまま引ける（タスク 6.3）。
//!
//! このファイルの在中テスト（`diff_tests.rs`）は例外ではない——そちらは 2 つの
//! カタログを**値として手で組む**ので、スナップショットには 1 バイトも触らない。
//!
//! # 本文の変更はハッシュの比較だけで判じる（要件 8.2）
//!
//! カタログは本文そのものを持たない（要件 1.3・9.4）。持っているのは本文から作った
//! 16 桁の印だけで、[`diff`] が見るのもそれだけである。見出し・分類・版番号・URL は
//! **1 つも見ない**——それらが変わっても本文が同じなら、この道具は何も言わない。
//! 要件 8.1 が挙げよと言うのは「**本文**が変わった項目」であって、見出しの改称は
//! 本文の変更ではないからである（見出しの新しい値はカタログを作り直せばそのまま
//! 列に入る・要件 1.1）。
//!
//! ハッシュの算法はカタログ冒頭の `hash_algorithm` に記録してある（設計 D-1）。
//! 算法を差し替えた後のカタログを差し替え前のカタログと比べると、本文が 1 文字も
//! 変わっていない項目まで「本文が変わった」に挙がる。**多めに挙がる向きなので
//! 見落としは起きない**が、読み手を驚かせる。2 つのカタログの `hash_algorithm` が
//! 揃っているかを見るのは呼び出し側（実行ファイルの副手続き）の役目で、ここでは
//! 判じない——[`CatalogDiff`] は 4 つの一覧しか持たない形に凍結してある（設計
//! diff 節）。

use std::collections::BTreeSet;

use crate::catalog::Catalog;
use crate::ledger::Ledger;
use crate::model::EntryId;

/// 2 つのカタログの差分（要件 8.1・8.3）。
///
/// 4 つの一覧はいずれも **id の byte 昇順**に並ぶ。同じ入力なら同じ並びで返る
/// （要件 7.3 の決まり方を差分にも通す）。
///
/// [`Self::removed`] と [`Self::removed_in_ledger`] は**取り分けた 2 つではない**。
/// 台帳に現れる削除 id は両方に載る。要件 8.3 は「削除された項目のうち台帳に
/// 現れるもの」を別に明示せよと言うのであって、削除の一覧から抜けとは言わない
/// ——抜いてしまうと「消えた項目の全部」を知りたい読み手が 2 つの一覧を足し
/// 合わせる羽目になる。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CatalogDiff {
    /// 新しいカタログにだけある id（増えた項目）。
    pub added: Vec<EntryId>,
    /// 現行のカタログにだけある id（消えた項目）。
    pub removed: Vec<EntryId>,
    /// 両方にあって本文のハッシュが違う id（本文が変わった項目・要件 8.2）。
    pub changed: Vec<EntryId>,
    /// [`Self::removed`] のうち、いずれかの台帳に現れる id（要件 8.3）。
    ///
    /// ここに載った id は台帳の行の引き取り手が正典から消えたということなので、
    /// 台帳を書いた人が行の始末を決める必要がある。
    pub removed_in_ledger: Vec<EntryId>,
}

/// 現行のカタログと新しいカタログを比べる。
///
/// `ledgers` は 4 ドメインの台帳（要件 3.1）。渡された順は結果に影響しない——
/// [`CatalogDiff::removed_in_ledger`] も id の順に並ぶ。同じ id が 2 本の台帳に
/// 載っていても 1 度しか挙がらない。
///
/// 失敗しない。食い違いは所見ではなく、そのまま 4 つの一覧になる。
pub(crate) fn diff(current: &Catalog, next: &Catalog, ledgers: &[Ledger]) -> CatalogDiff {
    // どの一覧も `BTreeMap` の鍵を昇順に辿って作るので、並べ直しは要らない。
    let added = next
        .entries
        .keys()
        .filter(|id| !current.entries.contains_key(*id))
        .cloned()
        .collect();

    let removed: Vec<EntryId> = current
        .entries
        .keys()
        .filter(|id| !next.entries.contains_key(*id))
        .cloned()
        .collect();

    let changed = current
        .entries
        .iter()
        .filter(|(id, entry)| {
            // 両方にある項目だけを見る。見るのはハッシュだけ（要件 8.2）。
            next.entries
                .get(*id)
                .is_some_and(|after| after.hash != entry.hash)
        })
        .map(|(id, _)| id.clone())
        .collect();

    // 台帳 4 本の id を 1 つの集合にまとめてから絞る。台帳ごとに拾って継ぎ足すと
    // 台帳の順に並び、同じ id が 2 度載る。
    let in_ledger: BTreeSet<&EntryId> = ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.keys())
        .collect();
    let removed_in_ledger = removed
        .iter()
        .filter(|id| in_ledger.contains(id))
        .cloned()
        .collect();

    CatalogDiff {
        added,
        removed,
        changed,
        removed_in_ledger,
    }
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
