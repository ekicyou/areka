//! `\![change,shell,名(,--option=raise-event)]`・`\![change,balloon,名]` の受け口
//! （design「SwitchCueSink」・要件 1.2／1.3／1.4／1.15）。
//!
//! 骨格は `change_cue.rs` の `ChangeCueSink` と同じ 2 段である。
//!
//! 1. **自己選別**——コマンド名と第 1 引数の組が `("change", "shell")`・`("change", "balloon")` の
//!    ものだけを受理する。`\![change,ghost,…]`（`ChangeCueSink` の持ち場）・第 1 引数の無い裸の
//!    `\![change]`・他の名前は担当外として `debug!` で読み飛ばす。
//! 2. **送り出し**——第 2 引数の名前を**無変形**で、種別と `--option=raise-event` の有無とともに
//!    切替要求 1 件として UI へ送る。名前の突き合わせは UI 側の入口（`shell_balloon_switch`）の
//!    仕事である。`raise-event` はシェルでだけ効き、バルーンでは知らない option と同じく
//!    `warn!` を残して無視する（バルーンに「切り替え前」のイベントは無い・要件 1.4）。
//!
//! 依存は要求の型（`shell_balloon_switch::SkinRequestRaw`）だけに閉じる。
//!
//! # 黙って諦めない
//!
//! 名前なし・知らない option・送出の失敗のいずれも記録してから抜ける。受信端が閉じていても台本は
//! 殺さない（記録して継続する）。

use std::sync::mpsc::Sender;

use dola::cue::TalkCue;
use tracing::{debug, warn};

use super::shell_balloon_switch::{SkinKind, SkinRequestRaw};

/// コマンド名（ゴースト・シェル・バルーンの切替の汎用の名前）。
const NAME_CHANGE: &str = "change";
/// 第 1 引数（選別子）——シェルの切替。
const SELECTOR_SHELL: &str = "shell";
/// 第 1 引数（選別子）——バルーンの切替。
const SELECTOR_BALLOON: &str = "balloon";
/// `OnShellChanging` を送らせる option（シェルでだけ効く）。
const OPTION_RAISE_EVENT: &str = "--option=raise-event";

/// `\![change,shell|balloon,…]` の受け口（design「SwitchCueSink」）。
///
/// 配送は台本ごとに受け口を複製するため [`Clone`] が要る（どの複製も単一の受信端へ届く）。
#[derive(Clone)]
pub(crate) struct SwitchCueSink {
    /// UI スレッド（切替要求の取り出しの系）への送出端。
    tx: Sender<SkinRequestRaw>,
}

impl SwitchCueSink {
    /// 送信端から受け口を組む。
    pub(crate) fn new(tx: Sender<SkinRequestRaw>) -> Self {
        Self { tx }
    }
}

impl dola::cue::CueSink for SwitchCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) 開封。開封できない `change` の荷物は第 1 引数が読めず自分宛か決まらないので、ここでは
        //    `debug!` で見送る（同じ名前の `ChangeCueSink` が警告を 1 件残す——2 重に警告しない）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            debug!(
                event = "switch_cue_skip",
                command = ?cue.command,
                "[SwitchCueSink] 担当外の cue を読み飛ばす"
            );
            return;
        };

        // 2) 自己選別。担当は `("change", "shell")`・`("change", "balloon")` の 2 組だけ。
        let selector = params.first().copied().unwrap_or_default();
        let kind = match (name, selector) {
            (NAME_CHANGE, SELECTOR_SHELL) => SkinKind::Shell,
            (NAME_CHANGE, SELECTOR_BALLOON) => SkinKind::Balloon,
            _ => {
                debug!(
                    event = "switch_cue_skip",
                    name, selector, "[SwitchCueSink] 担当外のコマンドを読み飛ばす（自己選別）"
                );
                return;
            }
        };

        // 3) 名前なし（空も含む）は切替先が無いので捨てる。
        let Some(target) = params.get(1).copied().filter(|n| !n.is_empty()) else {
            warn!(
                event = "switch_cue_no_name",
                selector, "[SwitchCueSink] 切替先の名前が無い——切替要求を出さない"
            );
            return;
        };

        // 4) option。`raise-event` はシェルでだけ効き、他は警告して無視する（切替は続ける）。
        let mut raise_event = false;
        for option in &params[2..] {
            if kind == SkinKind::Shell && *option == OPTION_RAISE_EVENT {
                raise_event = true;
            } else {
                warn!(
                    event = "switch_cue_unknown_option",
                    selector,
                    option,
                    "[SwitchCueSink] 知らない option を無視する（切替要求は出す）"
                );
            }
        }

        // 5) 送り出し。名前は無変形。受信端が閉じていても台本は殺さない。
        let request = SkinRequestRaw {
            kind,
            name: target.to_owned(),
            raise_event,
        };
        if self.tx.send(request).is_err() {
            warn!(
                event = "switch_cue_send_failed",
                selector,
                target,
                "[SwitchCueSink] 切替要求を送り出せなかった（受信端が閉じている）"
            );
        }
    }
}

#[cfg(test)]
#[path = "switch_cue_tests.rs"]
mod tests;
