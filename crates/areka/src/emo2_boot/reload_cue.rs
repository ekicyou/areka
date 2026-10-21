//! `\![reload,…]`・`\![reloadsurface]` の受け口（design「ReloadCueSink」・要件 1.4・1.5・10.3）。
//!
//! 骨格は `change_cue.rs` と同型。台本の演出は全員へ配られるので、本受け口にも他人宛のコマンドが
//! 届く。ここで行うのは次の 2 つに限る。
//!
//! 1. **自己選別**——`reload_args::parse_script` が `Some` を返す綴り（`\![reload,ghost|shiori|
//!    shell|balloon|descript(,パラメータ)]`・`\![reloadsurface]`）だけを受理する。`\![reload,makoto]`・
//!    `\![reload,aigraph]`・引数なしの `\![reload]`・他の名前は担当外として `debug!` で読み飛ばす
//!    （要件 1.5・10.3）。
//! 2. **送り出し**——`parse_script` の写し（対象と descript の語の仕分け）をそのまま UI へ送る。
//!    断る・始めるの判定は UI 側の窓口（`reload::request_reload`）の仕事である。
//!
//! 受信端が閉じていても（ゴーストが居ない場面）台本は殺さない（`warn!` を残して継続する）。
//!
//! 受信端 [`ReloadRx`] とその据え付け [`wire_reload_rx`] もここに置く。受け口の dispatcher への登録と
//! 受信端の据え付けは `runtime_boot` が行う。毎フレームの取り出し（`try_iter` の呼び手）は `reload::drain`。

use std::sync::mpsc::{Receiver, Sender, TryIter};

use bevy_ecs::world::World;
use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};

use super::reload_args::{ScriptReload, parse_script};

/// `\![reload,…]` の受け口（design「ReloadCueSink」）。
///
/// 配送は台本ごとに受け口を複製するため [`Clone`] が要る（どの複製も単一の受信端へ届く）。
#[derive(Clone)]
pub(crate) struct ReloadCueSink {
    /// UI スレッド（読み直しの要求の取り出しの系）への送出端。
    tx: Sender<ScriptReload>,
}

impl ReloadCueSink {
    pub(crate) fn new(tx: Sender<ScriptReload>) -> Self {
        Self { tx }
    }
}

impl dola::cue::CueSink for ReloadCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) 開封。開封できない荷物は宛名で水準を分ける（自分宛の壊れ物は警告・他人宛は良性）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. }
                    if command == "reload" || command == "reloadsurface" =>
                {
                    warn!(
                        event = "reload_cue_unopenable",
                        command = ?cue.command,
                        "[ReloadCueSink] 自分宛（reload）の開けない荷物を読み飛ばす"
                    )
                }
                _ => debug!(
                    event = "reload_cue_skip",
                    command = ?cue.command,
                    "[ReloadCueSink] 担当外の cue を読み飛ばす"
                ),
            }
            return;
        };

        // 2) 自己選別。拾う綴りは `parse_script` が決める（makoto・aigraph ほかは `None`）。
        let params: Vec<String> = params.into_iter().map(str::to_owned).collect();
        let Some(reload) = parse_script(name, &params) else {
            debug!(
                event = "reload_cue_skip",
                name,
                ?params,
                "[ReloadCueSink] 担当外のコマンドを読み飛ばす（自己選別）"
            );
            return;
        };

        // 3) 送り出し。受信端が閉じていても台本は殺さない。
        let target = format!("{:?}", reload.target);
        if self.tx.send(reload).is_err() {
            warn!(
                event = "reload_cue_send_failed",
                target, "[ReloadCueSink] 読み直しの要求を送り出せなかった（受信端が閉じている）"
            );
        }
    }
}

/// 台本からの読み直しの要求の受信端（World の NonSend・ゴーストごとに新品）。
pub(crate) struct ReloadRx(Receiver<ScriptReload>);

impl ReloadRx {
    /// 溜まった要求を待たずに取り出す（取り出しの系が全件を読む）。
    pub(crate) fn try_iter(&self) -> TryIter<'_, ScriptReload> {
        self.0.try_iter()
    }
}

/// 受信端を World へ据える（ゴーストごと・系は登録しない）。
///
/// 前のゴーストの受信端は置き換わって落ちる。
pub(crate) fn wire_reload_rx(world: &mut World, rx: Receiver<ScriptReload>) {
    world.insert_non_send(ReloadRx(rx));
}

#[cfg(test)]
#[path = "reload_cue_tests.rs"]
mod tests;
