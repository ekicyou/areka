//! `\![execute,install,path,パス]` の受け口（design「areka / 入口 / install_cue」・要件 1.5／1.6／1.7）。
//!
//! 骨格は `change_cue.rs` の受け口と同型。台本の演出は全員へ配られるので、ここで行うのは次の 2 つに
//! 限る。
//!
//! 1. **自己選別**——コマンド名と第 1 引数の組が `("execute", "install")` のものだけを受理する。
//!    `execute` の他の第 1 引数・裸の `\![execute]`・他の名前は担当外として `debug!` で読み飛ばす。
//! 2. **送り出し**——2 番目以降の引数を `install::judge::script_request` へ渡し、通れば出どころ
//!    「台本」の生の要求 1 件を窓口へ送る。依頼にするのは窓口の取り出し（UI スレッド）の仕事である。
//!
//! # 黙って諦めない
//!
//! `path` 以外（`url` を含む）・相対パスと空・パスの後ろの引数・送出の失敗のいずれも記録する。
//! 受信端が閉じていても台本は殺さない（記録して継続する）。

use std::sync::mpsc::Sender;

use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};

use crate::install::judge::{ScriptRefusal, script_request};
use crate::install::{InstallOrigin, RawInstallRequest};

/// コマンド名（外部の仕事を始めさせる汎用の名前）。
const NAME_EXECUTE: &str = "execute";
/// 第 1 引数（選別子）——本受け口が担当するのはこの選別子を伴う組だけである。
const SELECTOR_INSTALL: &str = "install";

/// `\![execute,install,…]` の受け口（design「install_cue」）。
///
/// 配送は台本ごとに受け口を複製するため [`Clone`] が要る（どの複製も窓口の単一の受信端へ届く）。
#[derive(Clone)]
pub(crate) struct InstallCueSink {
    /// 窓口の生の要求の送出端（`install::desk::raw_sender` から借りる）。
    tx: Sender<RawInstallRequest>,
}

impl InstallCueSink {
    pub(crate) fn new(tx: Sender<RawInstallRequest>) -> Self {
        Self { tx }
    }
}

impl dola::cue::CueSink for InstallCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) 開封。開封できない荷物は宛名で水準を分ける（自分宛の壊れ物は警告・他人宛は良性）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. } if command == NAME_EXECUTE => warn!(
                    event = "install_cue_unopenable",
                    command = ?cue.command,
                    "[InstallCueSink] 自分宛（execute）の開けない荷物を読み飛ばす"
                ),
                _ => debug!(
                    event = "install_cue_skip",
                    command = ?cue.command,
                    "[InstallCueSink] 担当外の cue を読み飛ばす"
                ),
            }
            return;
        };

        // 2) 自己選別。担当は `("execute", "install")` の 1 組だけ。
        let selector = params.first().copied().unwrap_or_default();
        if (name, selector) != (NAME_EXECUTE, SELECTOR_INSTALL) {
            debug!(
                event = "install_cue_skip",
                name, selector, "[InstallCueSink] 担当外のコマンドを読み飛ばす（自己選別）"
            );
            return;
        }

        // 3) 引数の検査。`path` 以外と、相対パス・空は送らない。
        let arguments = &params[1..];
        let path = match script_request(arguments) {
            Ok(path) => path,
            Err(ScriptRefusal::NotPath { found }) => {
                warn!(
                    event = "install_cue_unsupported",
                    found = %found,
                    "[InstallCueSink] \\![execute,install] の 2 番目の引数が path でないので何もしない"
                );
                return;
            }
            Err(refusal @ (ScriptRefusal::Empty | ScriptRefusal::Relative { .. })) => {
                warn!(
                    event = "install_cue_bad_path",
                    refusal = ?refusal,
                    "[InstallCueSink] 書庫のパスが空か相対パスなので、インストールを始めない"
                );
                return;
            }
        };

        // 4) パスの後ろの引数は読まない（要求は出す）。
        if let Some(ignored) = arguments.get(2..).filter(|rest| !rest.is_empty()) {
            warn!(
                event = "install_cue_extra_ignored",
                ignored = ?ignored,
                "[InstallCueSink] パスより後ろの引数は読まない"
            );
        }

        // 5) 送り出し。受信端が閉じていても台本は殺さない。
        let request = RawInstallRequest {
            path,
            origin: InstallOrigin::Script,
        };
        if let Err(err) = self.tx.send(request) {
            warn!(
                event = "install_cue_send_failed",
                path = %err.0.path.display(),
                "[InstallCueSink] インストールの要求を送り出せなかった（窓口が無い）"
            );
        }
    }
}

#[cfg(test)]
#[path = "install_cue_tests.rs"]
mod tests;
