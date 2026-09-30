//! `\![execute,install,path,パス]`・`\![execute,install,url,URL,nar]` の受け口（design「areka / 入口 /
//! install_cue」・「`\![execute,install,url]`」・要件 1.5／1.6／1.7・6.1〜6.3）。
//!
//! 骨格は `change_cue.rs` の受け口と同型。台本の演出は全員へ配られるので、ここで行うのは次の 2 つに
//! 限る。
//!
//! 1. **自己選別**——コマンド名と第 1 引数の組が `("execute", "install")` のものだけを受理する。
//!    `execute` の他の第 1 引数・裸の `\![execute]`・他の名前は担当外として `debug!` で読み飛ばす。
//! 2. **送り出し**——2 番目以降の引数を `install::judge::script_request` へ渡し、`path` の腕は出どころ
//!    「台本」の生の要求 1 件を窓口へ送る。`url` の腕は短命の取得のスレッド（`install::fetch_url`）を
//!    起こし、落とし終えたらそのスレッドが同じ要求を送る。依頼にするのは窓口の取り出し（UI スレッド）の
//!    仕事である。
//!
//! # 黙って諦めない
//!
//! `path`／`url` 以外・相対パスと空・形の悪い URL・`nar` 以外の種別・読まない後ろの引数・送出の失敗の
//! いずれも記録する。受信端が閉じていても台本は殺さない（記録して継続する）。

use std::sync::Arc;
use std::sync::mpsc::Sender;

use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};

use crate::install::judge::{ScriptRefusal, ScriptRequest, script_request};
use crate::install::{InstallOrigin, RawInstallRequest, fetch_url};

/// コマンド名（外部の仕事を始めさせる汎用の名前）。
const NAME_EXECUTE: &str = "execute";
/// 第 1 引数（選別子）——本受け口が担当するのはこの選別子を伴う組だけである。
const SELECTOR_INSTALL: &str = "install";

/// `url` の腕で取得を起こす口（本番は `fetch_url::spawn_download`・テストは偽の取得口を差す）。
pub(crate) type StartFetch = Arc<dyn Fn(String, Sender<RawInstallRequest>) + Send + Sync>;

/// `\![execute,install,…]` の受け口（design「install_cue」）。
///
/// 配送は台本ごとに受け口を複製するため [`Clone`] が要る（どの複製も窓口の単一の受信端へ届く）。
#[derive(Clone)]
pub(crate) struct InstallCueSink {
    /// 窓口の生の要求の送出端（`install::desk::raw_sender` から借りる）。
    tx: Sender<RawInstallRequest>,
    start_fetch: StartFetch,
}

impl InstallCueSink {
    pub(crate) fn new(tx: Sender<RawInstallRequest>) -> Self {
        Self::with_fetch(tx, Arc::new(fetch_url::spawn_download))
    }

    /// 取得の起こし方を差し替える口（テストが偽の取得口を差す）。
    pub(crate) fn with_fetch(tx: Sender<RawInstallRequest>, start_fetch: StartFetch) -> Self {
        Self { tx, start_fetch }
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

        // 3) 引数の検査。断りはどれも `warn!` 1 件で何もしない。
        let arguments = &params[1..];
        let request = match script_request(arguments) {
            Ok(request) => request,
            Err(ScriptRefusal::NotPath { found }) => {
                warn!(
                    event = "install_cue_unsupported",
                    found = %found,
                    "[InstallCueSink] \\![execute,install] の 2 番目の引数が path でも url でもないので何もしない"
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
            Err(ScriptRefusal::BadUrl { found }) => {
                warn!(
                    event = "install_cue_bad_url",
                    found = %found,
                    "[InstallCueSink] URL が空か http:// / https:// で始まらないので、取得しない"
                );
                return;
            }
            Err(ScriptRefusal::UnsupportedKind { found }) => {
                warn!(
                    event = "install_cue_unsupported_kind",
                    found = %found,
                    "[InstallCueSink] \\![execute,install,url] の種別が nar でも省略でもないので、取得しない"
                );
                return;
            }
        };

        // 4) 読まない後ろの引数（`path` はパス、`url` は種別より後ろ）を記録する（要求は出す）。
        let read = match request {
            ScriptRequest::Path(_) => 2,
            ScriptRequest::Url(_) => 3,
        };
        if let Some(ignored) = arguments.get(read..).filter(|rest| !rest.is_empty()) {
            warn!(
                event = "install_cue_extra_ignored",
                ignored = ?ignored,
                "[InstallCueSink] パス／種別より後ろの引数は読まない"
            );
        }

        // 5) 送り出し。`url` は取得のスレッドが落とし終えてから送る。受信端が閉じていても台本は殺さない。
        let path = match request {
            ScriptRequest::Path(path) => path,
            ScriptRequest::Url(url) => return (self.start_fetch)(url, self.tx.clone()),
        };
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
