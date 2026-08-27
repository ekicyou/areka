//! `\![set,zorder,...]`／`\![reset,zorder]` の受け口——**自己選別と送り出しだけ**を行う層
//! （design「ZOrderCueSink」・要件 1.7／4.4／8.3／11.2）。
//!
//! 台本の演出は演者ごとに振り分けられずに全員へ配られるので、本 sink にも文字も演技も
//! 他人宛のコマンドも届く。ここで行うのは次の 2 つに限る。
//!
//! 1. **自己選別**——コマンド名と第 1 引数の組が `("set", "zorder")` か `("reset", "zorder")`
//!    のものだけを受理し、それ以外は担当外として読み飛ばす（読み飛ばした側で重なりの
//!    状態には一切触れない＝要件 4.4／11.2）。
//! 2. **送り出し**——受理したものは[`ZOrderDirective`]として送り出し、画面更新を促す旗を
//!    立てる。**この層はトークンを解釈しない**（解釈には台帳の状態が要るので、後段の
//!    取り出しの相の担当である）。
//!
//! # `MoveCueSink` との意図的な差＝実行スコープを読まない（要件 1.7）
//!
//! 本 sink の骨格は `move_cue.rs` の `MoveCueSink::emit` と同型だが、**1 点だけ違う**。
//! `MoveCueSink` は移動対象を `cue.actor`（タグを実行したスコープ）から取るのに対し、
//! 本 sink は `cue.actor` を**読まない**。重なりの意味を決めるのはタグに書かれたスコープ
//! 番号だけであり、どのスコープがタグを実行したかは意味に影響しない（要件 1.7）。
//! `\0` が書いても `\1` が書いても `\![set,zorder,1,0]` は同じ指令になる。
//! この差は兄弟のテストが両側から固定している——実行スコープを振っても指令が変わらない
//! ことと、本ファイルのコードにその欄を読む綴りが無いことの 2 つである。
//!
//! # 宛名の規律（開封できない荷物の水準の分け方）
//!
//! 汎用キャリアの正準形は「文字列の配列」であり、そうでない `Custom` は開封できない。
//! 開封できない荷物は**宛名**で水準を分ける（`move_cue.rs:489-505` の先例）——自分宛の
//! 壊れ物は警告、他人宛は良性の読み飛ばしとして記録する（報せる責任は宛名の担当者にある）。
//! ただし開封できない形では**第 1 引数が読めない**ので、宛名は名前だけで判ずるほかない。
//! `set`／`reset` の名前を今 担当しているのは本 sink だけなので、この 2 つを自分宛とする。
//! 将来 `\![set,他]` に別の担当が付いたときは、この述語を担当の登記と突き合わせて
//! 見直すこと（要件 11.3 が残している余地）。
//!
//! # 黙って諦めない（要件 8.3）
//!
//! 読み飛ばし・送出の失敗のいずれの経路も、必ず理由を記録してから抜ける。受け口が
//! 閉じていても台本は殺さない（記録して継続する）。

// 本 sink を配送へ登録するのは結線の task（6.2）であり、それまでは本番の実行体から
// 参照されない。段階実装の想定内であり、結線が着地したらこの許可は撤去する。
#![allow(dead_code)]

use std::sync::mpsc::Sender;

use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};
use wintf::ecs::world::tick_wake;

/// 重なりの指令（この層は解釈しないので、受け取ったトークンをそのまま運ぶ）。
///
/// 解釈（数値モード／明示モードの読み分け・拒否判定・正規化）は台帳の状態を要するため、
/// 取り出しの相が [`parse_zorder_tokens`](crate::placement::zorder_group_ledger::parse_zorder_tokens)
/// を通して行う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZOrderDirective {
    /// `\![set,zorder,...]`——選別子 `zorder` より後ろのトークンを記述順のまま運ぶ。
    Set {
        /// 要素のトークン列（無変形・空トークンも保持する）。
        tokens: Vec<String>,
    },
    /// `\![reset,zorder]`——タグ由来のグループを落として shell 設定の基底へ戻す。
    Reset,
}

/// コマンド名（重なり指定）。
const NAME_SET: &str = "set";
/// コマンド名（重なり解除）。
const NAME_RESET: &str = "reset";
/// 第 1 引数（選別子）——本 sink が担当するのはこの選別子を伴う組だけである。
const SELECTOR_ZORDER: &str = "zorder";

/// 開封できない荷物の宛名が本 sink のものか（水準の分け方はモジュール doc「宛名の規律」）。
fn is_own_address(command: &str) -> bool {
    command == NAME_SET || command == NAME_RESET
}

/// `\![set,zorder,...]`／`\![reset,zorder]` の受け口（design「ZOrderCueSink」）。
///
/// 配送は台本ごとに受け口を複製するため [`Clone`] が要る。内側の送信端は常に複製でき、
/// どの複製も単一の受信端（取り出しの相）へ届くので、複製しても配送の意味は変わらない。
#[derive(Clone)]
pub struct ZOrderCueSink {
    /// 取り出しの相への送出端。
    tx: Sender<ZOrderDirective>,
}

impl ZOrderCueSink {
    /// 送信端（受信端は結線の task が持つ）から受け口を組む。
    pub fn new(tx: Sender<ZOrderDirective>) -> Self {
        Self { tx }
    }
}

/// 演者非依存の単一出力契約を実装する（配送への登録が要求する形）。
///
/// 全ての cue が届くので、担当外は記録付きの良性な読み飛ばしへ落とす。cue の占有時間には
/// 一切触れない（観測するだけで、待ちの契約に影響を与えない）。
impl dola::cue::CueSink for ZOrderCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) 開封。開封できない荷物は宛名で水準を分ける（モジュール doc「宛名の規律」）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. } if is_own_address(command) => warn!(
                    command = ?cue.command,
                    "ZOrderCueSink: 自分宛（set/reset）の開けない荷物を良性に読み飛ばす（宛名の規律）"
                ),
                CueCommand::Custom { .. } => debug!(
                    command = ?cue.command,
                    "ZOrderCueSink: 他人宛の開けない荷物を良性に読み飛ばす（担当外・宛名の規律）"
                ),
                _ => debug!(
                    command = ?cue.command,
                    "ZOrderCueSink: キャリアでない cue を良性に読み飛ばす（担当外）"
                ),
            }
            return;
        };

        // 2) 自己選別。担当は「名前＋第 1 引数」の 2 組だけで、他は全て担当外である
        //    （`\![set,他]`／`\![reset,他]` を含む＝読み飛ばした側で重なりを変えない・
        //    要件 4.4／11.2）。第 1 引数が無い場合も担当外へ落ちる。
        let selector = params.first().copied().unwrap_or_default();
        let directive = match (name, selector) {
            (NAME_SET, SELECTOR_ZORDER) => ZOrderDirective::Set {
                // 解釈しない——選別子より後ろを記述順のまま運ぶだけ（解釈は台帳の状態が
                // 要るので取り出しの相の担当）。
                tokens: params.iter().skip(1).map(|t| (*t).to_owned()).collect(),
            },
            (NAME_RESET, SELECTOR_ZORDER) => {
                // 正典の解除は引数を取らない。余分が書かれていても解除として受理し、
                // 運ばなかったことを記録する（黙って捨てない・要件 8.3）。
                if params.len() > 1 {
                    debug!(
                        extra = ?&params[1..],
                        "ZOrderCueSink: 重なり解除は引数を取らないため余分なトークンを運ばない"
                    );
                }
                ZOrderDirective::Reset
            }
            _ => {
                debug!(
                    name,
                    selector,
                    "ZOrderCueSink: 担当外のコマンドを良性に読み飛ばす（自己選別・要件 11.2）"
                );
                return;
            }
        };

        // 3) 送り出し。受信端が閉じていても台本は殺さない（記録して継続・非 panic）。
        if self.tx.send(directive).is_err() {
            warn!("ZOrderCueSink: 重なりの指令を送り出せなかった（受信端が閉じている）");
        }

        // 4) 重なりに仕事が増えた＝次の画面更新で処理を実行してほしい旨の旗を立てる。
        //    送り出しの後に立てるのは `MoveCueSink` と同じ順序である。
        tick_wake::mark(tick_wake::ZORDER);
    }
}

#[cfg(test)]
#[path = "zorder_cue_tests.rs"]
mod tests;
