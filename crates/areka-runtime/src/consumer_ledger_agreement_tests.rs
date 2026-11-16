//! 正準台帳と `runtime` の 11 の受け口の自己選別の一致の檻（areka-P0-mcp-author-tools
//! 要件 3.2・3.11・design「`\!` の表の仕上げ」の「一致のテスト」）。
//!
//! 表は宣言するだけで、受け口は自分の名前と第 1 引数で自己選別する（両者は依存しない）。
//! `check_script` は表を引いて「知らない `\!`」を答えるので、表と受け口の選別がずれると
//! 「検査だけが通して再生で落ちる」「検査だけが落として再生で通る」が生まれる。ここで固定する。
//!
//! 11 の受け口をそれぞれ本物の送信端つきで組み、1 つの `\!` の cue を全員へ配って、どの送信端に
//! 指令が届いたかを見る（配送は全員へ同じ cue を配る broadcast なので、本番と同じ形）。
//!
//! 1. 表が担当を言う各行（受け口の既存のテストから取った引数つきの見本）→ その担当の受け口だけに
//!    届く。担当が `runtime` の外（seriko の `bind`・文字の層の `\f`・台本の組み立ての
//!    `set,choicetimeout`・プロパティの書き込みと読み）の行は、11 のどれにも届かない。
//! 2. 表に出てくる名前の全部 × 表に出てくる第 1 引数の全部（＋第 1 引数なし＋どこにも無い語）の
//!    掛け合わせのうち、表に無い組 → 11 のどれにも届かない（受け口が表に無い組を拾えば赤）。
//!    引数の列は「第 1 引数＋同じ名前の見本の第 2 引数から先（尾）」で組む。切替・書庫の受け口は
//!    選別子の後ろに引数が無いと何も送らないので、尾を付けないと選別がゆるんでも赤にならない。
//!    尾の無い形（第 1 引数だけ）も配り、第 1 引数なしは裸と空の第 1 引数（`""`＋尾）の両方を配る。
//!
//! 見本の一覧は表の行とちょうど同じ組を持つこと自体も判定する（行を足して見本を足し忘れれば赤）。

use std::collections::BTreeSet;

use super::test_support::{ledger_keys, samples};
use super::{ConsumerLedger, LedgerKey};

// ---------------------------------------------------------------- 檻

/// 見本の一覧は表の行とちょうど同じ組を持つ（行を足して見本を足し忘れても、見本だけ残っても赤）。
#[test]
fn samples_cover_exactly_the_ledger_rows() {
    let ledger = ConsumerLedger::canonical();
    let sampled: BTreeSet<LedgerKey> = samples()
        .into_iter()
        .map(|(name, selector, _)| (name.to_string(), selector.map(str::to_string)))
        .collect();
    assert_eq!(
        sampled,
        ledger_keys(&ledger),
        "見本の組と正準台帳の行の組は同じであること"
    );
    for (name, selector, args) in samples() {
        if let Some(selector) = selector {
            assert_eq!(
                args.first(),
                Some(&selector),
                "{name}: 選別子つきの行の見本は、第 1 引数がその選別子であること"
            );
        }
    }
}
