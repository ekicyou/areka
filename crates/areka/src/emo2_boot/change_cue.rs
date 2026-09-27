//! `\![change,ghost,名(,--option=raise-event)]` の受け口（design「ChangeCueSink」・要件 1.2／1.3／
//! 1.10／8.5）。
//!
//! 台本の演出は全員へ配られるので、本受け口にも文字も演技も他人宛のコマンドも届く。ここで行うのは
//! 次の 2 つに限る（骨格は `readme_cue.rs` の受け口と同型）。
//!
//! 1. **自己選別**——コマンド名と第 1 引数の組が `("change", "ghost")` のものだけを受理する。
//!    `\![change,shell,…]`／`\![change,balloon,…]`・第 1 引数の無い裸の `\![change]` は担当外
//!    として `debug!` で読み飛ばす（シェル・バルーンの切替は本受け口の持ち場ではない・要件 8.5）。
//! 2. **送り出し**——第 2 引数の名前を**無変形**で、`--option=raise-event` の有無とともに
//!    切替要求 1 件として UI へ送る。名前の突き合わせは UI 側の入口（`ghost_switch`）の仕事である。
//!
//! 依存は要求の型 1 つ（`ghost_switch::ChangeRequestRaw`）だけに閉じる。
//!
//! # 黙って諦めない
//!
//! 名前なし・知らない option・送出の失敗のいずれも記録してから抜ける。受信端が閉じていても台本は
//! 殺さない（記録して継続する）。

use std::sync::mpsc::Sender;

use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};

use super::ghost_switch::ChangeRequestRaw;

/// コマンド名（ゴースト・シェル・バルーンの切替の汎用の名前）。
const NAME_CHANGE: &str = "change";
/// 第 1 引数（選別子）——本受け口が担当するのはこの選別子を伴う組だけである。
const SELECTOR_GHOST: &str = "ghost";
/// `OnGhostChanging` を送らせる option。
const OPTION_RAISE_EVENT: &str = "--option=raise-event";

/// `\![change,ghost,…]` の受け口（design「ChangeCueSink」）。
///
/// 配送は台本ごとに受け口を複製するため [`Clone`] が要る（どの複製も単一の受信端へ届く）。
#[derive(Clone)]
pub(crate) struct ChangeCueSink {
    /// UI スレッド（切替要求の取り出しの系）への送出端。
    tx: Sender<ChangeRequestRaw>,
}

impl ChangeCueSink {
    /// 送信端（受信端は結線が `ghost_switch::wire_change_rx` へ渡す）から受け口を組む。
    pub(crate) fn new(tx: Sender<ChangeRequestRaw>) -> Self {
        Self { tx }
    }
}

impl dola::cue::CueSink for ChangeCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) 開封。開封できない荷物は宛名で水準を分ける（自分宛の壊れ物は警告・他人宛は良性）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. } if command == NAME_CHANGE => warn!(
                    event = "change_cue_unopenable",
                    command = ?cue.command,
                    "[ChangeCueSink] 自分宛（change）の開けない荷物を読み飛ばす"
                ),
                _ => debug!(
                    event = "change_cue_skip",
                    command = ?cue.command,
                    "[ChangeCueSink] 担当外の cue を読み飛ばす"
                ),
            }
            return;
        };

        // 2) 自己選別。担当は `("change", "ghost")` の 1 組だけ（shell／balloon・裸の change は担当外）。
        let selector = params.first().copied().unwrap_or_default();
        if (name, selector) != (NAME_CHANGE, SELECTOR_GHOST) {
            debug!(
                event = "change_cue_skip",
                name, selector, "[ChangeCueSink] 担当外のコマンドを読み飛ばす（自己選別）"
            );
            return;
        }

        // 3) 名前なし（空も含む）は切替先が無いので捨てる。
        let Some(ghost) = params.get(1).copied().filter(|n| !n.is_empty()) else {
            warn!(
                event = "change_ghost_no_name",
                "[ChangeCueSink] \\![change,ghost] に切替先の名前が無い——切替要求を出さない"
            );
            return;
        };

        // 4) option。知らないものは警告して無視する（切替は続ける）。
        let mut raise_event = false;
        for option in &params[2..] {
            if *option == OPTION_RAISE_EVENT {
                raise_event = true;
            } else {
                warn!(
                    event = "change_cue_unknown_option",
                    option, "[ChangeCueSink] 知らない option を無視する（切替要求は出す）"
                );
            }
        }

        // 5) 送り出し。名前は無変形。受信端が閉じていても台本は殺さない。
        let request = ChangeRequestRaw {
            name: ghost.to_owned(),
            raise_event,
        };
        if self.tx.send(request).is_err() {
            warn!(
                event = "change_cue_send_failed",
                ghost, "[ChangeCueSink] 切替要求を送り出せなかった（受信端が閉じている）"
            );
        }
    }
}

#[cfg(test)]
#[path = "change_cue_tests.rs"]
mod tests;
