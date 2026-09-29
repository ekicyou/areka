//! 台本 `\![updatebymyself]`・`\![update,…]`・`\![updateother,…]` の受け口と引数の解析
//! （design「areka / 入口 / 台本の受け口」・要件 1.5〜1.8・10.8）。
//!
//! 骨格は `install_cue.rs` の受け口と同型。台本の演出は全員へ配られるので、ここで行うのは次の 2 つに
//! 限る。
//!
//! 1. **自己選別**——コマンド名が `updatebymyself`・`update`・`updateother` のどれか（選別子なし）の
//!    ものだけを受理する。他の名前は担当外として `debug!` で読み飛ばす。
//! 2. **送り出し**——引数を純粋な [`parse_update_command`] で生の要求に組み、通れば窓口へ 1 件送る。
//!    対象を解く（フォルダ・名前引き）のは窓口の取り出し（UI スレッド）の仕事である。
//!
//! # 黙って諦めない
//!
//! 開けない自分宛の荷物・断り・読み飛ばした指定・送出の失敗のいずれも `warn!` で記録する。
//! 受信端が閉じていても台本は殺さない（記録して継続する）。
// 受け口の列への登記（タスク 7.1）がまだ無い。7.1 で外す。
#![allow(dead_code)]

use std::sync::mpsc::Sender;

use dola::cue::{CueCommand, TalkCue};
use tracing::{debug, warn};

use crate::update::{RawUpdateRequest, TargetKind};

/// 今の 3 つを更新する（`all` と同じ）。
const NAME_UPDATE_BY_MYSELF: &str = "updatebymyself";
/// 今の対象を `+` で並べて更新する。
const NAME_UPDATE: &str = "update";
/// 名前で引くシェル・バルーンを更新する。
const NAME_UPDATE_OTHER: &str = "updateother";

/// 更新オプション（α では受けない・要件 1.8・10.8）。`--option=…` は接頭辞で判定する。
const UPDATE_OPTIONS: [&str; 3] = ["checkonly", "testonly", "recovery"];
const OPTION_PREFIX: &str = "--option=";
const SHELL_PREFIX: &str = "--shell=";
const BALLOON_PREFIX: &str = "--balloon=";

/// 今の 3 つ（要件 1.4 と同じ並び）。
const CURRENT_ALL: [TargetKind; 3] = [TargetKind::Ghost, TargetKind::Shell, TargetKind::Balloon];

fn is_own(name: &str) -> bool {
    [NAME_UPDATE_BY_MYSELF, NAME_UPDATE, NAME_UPDATE_OTHER].contains(&name)
}

fn is_update_option(param: &str) -> bool {
    UPDATE_OPTIONS.contains(&param) || param.starts_with(OPTION_PREFIX)
}

/// 台本の更新の受け口（design「台本の受け口」）。
///
/// 配送は台本ごとに受け口を複製するため [`Clone`] が要る（どの複製も窓口の単一の受信端へ届く）。
#[derive(Clone)]
pub(crate) struct UpdateCueSink {
    /// 窓口の生の要求の送出端（`update::desk::raw_sender` から借りる）。
    tx: Sender<RawUpdateRequest>,
}

impl UpdateCueSink {
    pub(crate) fn new(tx: Sender<RawUpdateRequest>) -> Self {
        Self { tx }
    }
}

impl dola::cue::CueSink for UpdateCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) 開封。開封できない荷物は宛名で水準を分ける（自分宛の壊れ物は警告・他人宛は良性）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            match &cue.command {
                CueCommand::Custom { command, .. } if is_own(command) => warn!(
                    event = "update_cue_unopenable",
                    command = ?cue.command,
                    "[UpdateCueSink] 自分宛（更新）の開けない荷物を読み飛ばす"
                ),
                _ => debug!(
                    event = "update_cue_skip",
                    command = ?cue.command,
                    "[UpdateCueSink] 担当外の cue を読み飛ばす"
                ),
            }
            return;
        };

        // 2) 自己選別。担当は 3 つのコマンド名だけ（選別子なし）。
        if !is_own(name) {
            debug!(
                event = "update_cue_skip",
                name, "[UpdateCueSink] 担当外のコマンドを読み飛ばす（自己選別）"
            );
            return;
        }

        // 3) 解析。断りは要求 0。
        let parsed = match parse_update_command(name, &params) {
            Ok(parsed) => parsed,
            Err(refusal) => {
                warn!(
                    event = "update_cue_refused",
                    name,
                    refusal = ?refusal,
                    "[UpdateCueSink] 更新の要求を作らない"
                );
                return;
            }
        };
        for ignored in &parsed.ignored {
            warn!(
                event = "update_cue_selector_ignored",
                name,
                ignored = %ignored,
                "[UpdateCueSink] 知らない指定を読み飛ばす"
            );
        }

        // 4) 送り出し。受信端が閉じていても台本は殺さない。
        if let Err(err) = self.tx.send(parsed.request) {
            warn!(
                event = "update_cue_send_failed",
                request = ?err.0,
                "[UpdateCueSink] 更新の要求を送り出せなかった（窓口が無い）"
            );
        }
    }
}

/// 解析の断り（要件 1.8・10.8）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// 更新オプション（`checkonly`・`testonly`・`recovery`・`--option=…`）か、読めない引数。
    Option { found: String },
    /// `update` の対象に `ghost`・`shell`・`balloon`・`all` 以外の語がある（`platform` を含む）。
    UnknownTarget { found: String },
    /// `updateother` に `--shell=`／`--balloon=` が 1 つも無い。
    NoTargets,
}

/// 解析の結果（要求と、読み飛ばした指定の列＝呼び手が 1 件ずつ `warn!` にする）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Parsed {
    pub request: RawUpdateRequest,
    pub ignored: Vec<String>,
}

/// 純粋: コマンド名と引数から要求を組む（要件 1.5〜1.8）。`name` は 3 つのどれかである前提
/// （呼び手が自己選別する）で、それ以外は `update` と同じに読む。
pub(crate) fn parse_update_command(name: &str, params: &[&str]) -> Result<Parsed, Refusal> {
    let current = |kinds: Vec<TargetKind>| Parsed {
        request: RawUpdateRequest::Current(kinds),
        ignored: Vec::new(),
    };
    match name {
        NAME_UPDATE_BY_MYSELF => match params.first() {
            Some(found) => Err(Refusal::Option {
                found: found.to_string(),
            }),
            None => Ok(current(CURRENT_ALL.to_vec())),
        },
        NAME_UPDATE_OTHER => parse_other(params),
        _ => {
            let first = params.first().copied().unwrap_or_default();
            let mut kinds = Vec::new();
            for word in first.split('+') {
                match word {
                    "all" => kinds.extend(CURRENT_ALL),
                    "ghost" => kinds.push(TargetKind::Ghost),
                    "shell" => kinds.push(TargetKind::Shell),
                    "balloon" => kinds.push(TargetKind::Balloon),
                    _ => {
                        return Err(Refusal::UnknownTarget {
                            found: word.to_string(),
                        });
                    }
                }
            }
            if let Some(found) = params.get(1) {
                return Err(Refusal::Option {
                    found: found.to_string(),
                });
            }
            Ok(current(kinds))
        }
    }
}

/// `updateother` の引数: `--shell=名前`／`--balloon=名前` を並んだ順に。更新オプションか `--` で
/// 始まらない語が 1 つでもあれば要求ごと断り、知らない `--名前=` は読み飛ばす。
fn parse_other(params: &[&str]) -> Result<Parsed, Refusal> {
    // 更新オプションは位置に依らず要求ごと断る（読み替えて普通の更新をしない・裁定 8）。
    if let Some(found) = params
        .iter()
        .find(|p| is_update_option(p) || !p.starts_with("--"))
    {
        return Err(Refusal::Option {
            found: found.to_string(),
        });
    }
    let mut targets = Vec::new();
    let mut ignored = Vec::new();
    for param in params {
        if let Some(name) = param.strip_prefix(SHELL_PREFIX) {
            targets.push((TargetKind::Shell, name.to_string()));
        } else if let Some(name) = param.strip_prefix(BALLOON_PREFIX) {
            targets.push((TargetKind::Balloon, name.to_string()));
        } else {
            ignored.push(param.to_string());
        }
    }
    if targets.is_empty() {
        return Err(Refusal::NoTargets);
    }
    Ok(Parsed {
        request: RawUpdateRequest::Other(targets),
        ignored,
    })
}

#[cfg(test)]
#[path = "update_cue_tests.rs"]
mod tests;
