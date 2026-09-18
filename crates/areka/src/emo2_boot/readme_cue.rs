//! `\![open,readme]` の受け口（design「ReadmeCueSink」・要件 4.5／4.6／8.4）。
//!
//! 台本の演出は演者ごとに振り分けられずに全員へ配られるので、本受け口にも文字も演技も
//! 他人宛のコマンドも届く。ここで行うのは次の 2 つに限る。
//!
//! 1. **自己選別**——コマンド名と第 1 引数の組が `("open", "readme")` のものだけを受理し、
//!    それ以外（他の名前・`\![open,他]`・キャリアでない cue）は担当外として読み飛ばす。
//! 2. **送り出し**——引数なしの `\![open,readme]` だけを説明書の要求 1 件として送り出す。
//!    引数付き（`\![open,readme,種類,名前]`）は列挙が要るため α では語彙だけを持ち、
//!    警告して何もしない（要件 4.6）。
//!
//! 開くファイルを決めるのも開くのも UI 側（`crate::readme`）の仕事で、本受け口は
//! 「開いて」の要求を運ぶだけである。依存は要求の型 1 つだけに閉じる。
//!
//! 骨格は `zorder_cue.rs`／`move_cue.rs` の受け口と同型で、開封できない荷物を宛名で
//! 分ける規律もそのまま踏襲する（自分宛の壊れ物は警告・他人宛は良性の読み飛ばし）。
//! ただし `open` は名前まるごとが本受け口の担当ではない（担当は `("open", "readme")` の
//! 1 組だけ）。開封できない荷物は第 1 引数を読めないので、今は名前 `open` だけで自分宛と
//! みなして警告している。`\![open,他]` に別の担当が付いた日には、この述語を消費者台帳の
//! 登記と突き合わせて見直すこと（他人宛の壊れ物に本受け口が警告を出してしまう）。
//!
//! # 黙って諦めない（要件 8.4）
//!
//! 読み飛ばし・送出の失敗のいずれの経路も、必ず理由を記録してから抜ける。受け口が
//! 閉じていても台本は殺さない（記録して継続する）。

// 結線（`wire_emo2_boot` の `sinks` へ本受け口を足す・task 4.3）が入るまで、本 module の中身は
// 本番のビルドから一度も呼ばれない（テストだけが組み立てる）。許可を module 単位にしているのは、
// 本 module の中身が受け口 1 つとその名札だけで、結線が入れば全部が一度に生きるからである
// （`readme.rs`・`move_cue.rs` と同じ流儀）。結線が入った時点でこの 1 行を外す。
#![allow(dead_code)]

use std::sync::mpsc::Sender;

use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};

use crate::readme::ReadmeRequest;

/// コマンド名（作り付けの窓や外部を開く汎用の名前）。
const NAME_OPEN: &str = "open";
/// 第 1 引数（選別子）——本受け口が担当するのはこの選別子を伴う組だけである。
const SELECTOR_README: &str = "readme";

/// `\![open,readme]` の受け口（design「ReadmeCueSink」）。
///
/// 配送は台本ごとに受け口を複製するため [`Clone`] が要る。内側の送信端は常に複製でき、
/// どの複製も単一の受信端（UI の `ReadmeWiring`）へ届くので、複製しても配送の意味は変わらない。
#[derive(Clone)]
pub(crate) struct ReadmeCueSink {
    /// UI スレッド（説明書の取り出しの段）への送出端。
    tx: Sender<ReadmeRequest>,
}

impl ReadmeCueSink {
    /// 送信端（受信端は結線の task が `wire_readme` へ渡す）から受け口を組む。
    pub(crate) fn new(tx: Sender<ReadmeRequest>) -> Self {
        Self { tx }
    }
}

/// 演者非依存の単一出力契約を実装する（配送への登録が要求する形）。
///
/// 全ての cue が届くので、担当外は記録付きの良性な読み飛ばしへ落とす。cue の占有時間には
/// 一切触れない（観測するだけで、待ちの契約に影響を与えない）。
impl dola::cue::CueSink for ReadmeCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) 開封。開封できない荷物は宛名で水準を分ける（モジュール doc の規律）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. } if command == NAME_OPEN => warn!(
                    command = ?cue.command,
                    "ReadmeCueSink: 自分宛（open）の開けない荷物を良性に読み飛ばす（宛名の規律）"
                ),
                CueCommand::Custom { .. } => debug!(
                    command = ?cue.command,
                    "ReadmeCueSink: 他人宛の開けない荷物を良性に読み飛ばす（担当外・宛名の規律）"
                ),
                _ => debug!(
                    command = ?cue.command,
                    "ReadmeCueSink: キャリアでない cue を良性に読み飛ばす（担当外）"
                ),
            }
            return;
        };

        // 2) 自己選別。担当は「名前＋第 1 引数」の 1 組だけで、`\![open,他]`（作り付けの窓など）も
        //    第 1 引数の無い裸の `\![open]` も担当外である（報せる責任はその担当者にある）。
        let selector = params.first().copied().unwrap_or_default();
        if (name, selector) != (NAME_OPEN, SELECTOR_README) {
            debug!(
                name,
                selector, "ReadmeCueSink: 担当外のコマンドを良性に読み飛ばす（自己選別）"
            );
            return;
        }

        // 3) 引数付き（`\![open,readme,種類,名前]`）は列挙が要るため α では語彙だけを持ち、
        //    警告して何もしない（要件 4.6）。
        // ukadoc の正典 URL は担当の定義箇所（`CommandConsumer::ReadmeSink` の doc）に 1 行置く。
        if params.len() > 1 {
            warn!(
                arguments = ?&params[1..],
                "ReadmeCueSink: 引数付きの説明書タグは語彙のみのため何もしない（要件 4.6）"
            );
            return;
        }

        // 4) 送り出し。受信端が閉じていても台本は殺さない（記録して継続・非 panic）。
        if self.tx.send(ReadmeRequest).is_err() {
            warn!("ReadmeCueSink: 説明書の要求を送り出せなかった（受信端が閉じている）");
        }
    }
}

#[cfg(test)]
#[path = "readme_cue_tests.rs"]
mod tests;
