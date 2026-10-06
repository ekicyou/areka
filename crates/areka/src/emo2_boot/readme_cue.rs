//! 開く系のタグの受け口（design「ReadmeCueSink」・areka-P0-open-external-tags 要件 1.5・
//! 1.7・2.6・3.2・4.7・6.3・7.5・8.6／説明書は要件 4.5／4.6／8.4）。
//!
//! 台本の演出は演者ごとに振り分けられずに全員へ配られるので、本受け口にも文字も演技も
//! 他人宛のコマンドも届く。ここで行うのは次の 2 つに限る。
//!
//! 1. **自己選別**——`("open", "readme")` の組と、行き先の規則
//!    （[`crate::readme::destination::classify`]）が開く系とみなすもの
//!    （`\![open,file|browser|explorer|editor|mailer,…]` と運搬名 `\j` の `\j[ID]`）だけを
//!    受理し、それ以外（他の名前・`\![open,help]` など他の選別子・裸の `\![open]`・
//!    キャリアでない cue）は担当外として読み飛ばす。
//! 2. **送り出し**——引数なしの `\![open,readme]` は説明書の要求、開く系は規則が分類した
//!    行き先を 1 件ずつ UI へ送る。引数付きの `\![open,readme,種類,名前]` は列挙が要るため
//!    α では語彙だけを持ち、警告して何もしない（要件 4.6）。規則が断る入力（引数が無い・
//!    `\j` の ID が知らない形・扱えない種類）は `open_external_rejected` を 1 行残して送らない。
//!
//! 行き先の規則は台本からの取り出し（`link_destinations`）と同じ 1 つの関数で、両者が
//! 食い違わない（要件 8.6）。行き先を解決するのも開くのも開く専用のスレッドの仕事で、
//! 本受け口は OS・fs・World に触れない（台本の再生を止めない・要件 7.5）。
//!
//! 骨格は `zorder_cue.rs`／`move_cue.rs` の受け口と同型で、開封できない荷物を宛名で
//! 分ける規律もそのまま踏襲する（自分宛の壊れ物は警告・他人宛は良性の読み飛ばし）。
//! 開封できない荷物は第 1 引数を読めないので、名前 `open` と運搬名 `\j` を自分宛とみなす。
//! `\![open,help]` など `open` の他の選別子に別の担当が付いた日には、この述語を消費者台帳の
//! 登記と突き合わせて見直すこと（他人宛の壊れ物に本受け口が警告を出してしまう）。
//!
//! # 黙って諦めない（要件 8.4）
//!
//! 読み飛ばし・断り・送出の失敗のいずれの経路も、必ず理由を記録してから抜ける。受け口が
//! 閉じていても台本は殺さない（記録して継続する）。

use std::sync::mpsc::Sender;

use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};

use areka_parsers::sakura::JUMP_TAG_CARRIER;

use crate::readme::ReadmeRequest;
use crate::readme::destination::classify;

/// コマンド名（作り付けの窓や外部を開く汎用の名前）。
const NAME_OPEN: &str = "open";
/// 第 1 引数（選別子）——本受け口が担当するのはこの選別子を伴う組だけである。
const SELECTOR_README: &str = "readme";

/// 開く系のタグ（`\![open,readme]` を含む）の受け口（design「ReadmeCueSink」）。
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
                CueCommand::Custom { command, .. }
                    if command == NAME_OPEN || command == JUMP_TAG_CARRIER =>
                {
                    warn!(
                        command = ?cue.command,
                        "ReadmeCueSink: 自分宛（open・\\j）の開けない荷物を良性に読み飛ばす（宛名の規律）"
                    )
                }
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

        let selector = params.first().copied().unwrap_or_default();
        let request = if (name, selector) == (NAME_OPEN, SELECTOR_README) {
            // 2) 説明書は今のまま。引数付き（`\![open,readme,種類,名前]`）は列挙が要るため
            //    α では語彙だけを持ち、警告して何もしない（要件 4.6）。
            // ukadoc の正典 URL は担当の定義箇所（`CommandConsumer::ReadmeSink` の doc）に 1 行置く。
            if params.len() > 1 {
                warn!(
                    arguments = ?&params[1..],
                    "ReadmeCueSink: 引数付きの説明書タグは語彙のみのため何もしない（要件 4.6）"
                );
                return;
            }
            ReadmeRequest::Readme
        } else {
            // 3) 開く系は行き先の規則で分類する（取り出しと同じ関数・要件 8.6）。規則が対象外
            //    とするもの（他の名前・`\![open,help]`・裸の `\![open]`）は担当外である
            //    （報せる責任はその担当者にある）。
            match classify(name, &params) {
                None => {
                    debug!(
                        name,
                        selector, "ReadmeCueSink: 担当外のコマンドを良性に読み飛ばす（自己選別）"
                    );
                    return;
                }
                Some(Err(rejected)) => {
                    warn!(
                        event = "open_external_rejected",
                        tag = %rejected.tag,
                        reason = ?rejected.reason,
                        "ReadmeCueSink: 開く系のタグを断った（開かない）"
                    );
                    return;
                }
                Some(Ok(dest)) => ReadmeRequest::Open(dest),
            }
        };

        // 4) 送り出し。受信端が閉じていても台本は殺さない（記録して継続・非 panic）。
        if self.tx.send(request).is_err() {
            warn!("ReadmeCueSink: 開く系の要求を送り出せなかった（受信端が閉じている）");
        }
    }
}

#[cfg(test)]
#[path = "readme_cue_tests.rs"]
mod tests;
