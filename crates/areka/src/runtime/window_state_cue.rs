//! `\v`／`\![set,windowstate,…]` の受け口——**自己選別と第 2 引数の読み分けと送り出し**だけを
//! 行う層（design「WindowStateCueSink」・要件 1.1〜1.3／1.5〜1.7／7.1）。
//!
//! 骨格は重なりの受け口 `ZOrderCueSink`（`zorder_cue.rs`）と同型で、相乗りはしない。
//!
//! 1. **自己選別**——手前へ出す指令の運搬名（[`RAISE_TAG_CARRIER`]）と、名前＋第 1 引数が
//!    `("set", "windowstate")` の組だけを拾い、他は担当外として `debug!` で読み飛ばす。
//! 2. **読み分け**——純関数 [`parse_window_state`] が第 2 引数を**小文字ちょうど**で読む
//!    （`stayontop`→オン・`!stayontop`→オフ・それ以外は読めない＝状態を変えず `warn!`）。
//!    第 3 引数以降は第 2 引数で決めた上で余りとして `debug!` に残す。`\v` は引数の有無に
//!    関わらず手前へ出す。
//! 3. **送り出し**——指令を送り、重なりの仕事が増えた旗（兄弟と共用の `ZORDER`）を立てる。
//!
//! # 書いたスコープを読まない（裁定 1）
//!
//! `\1\v` も `\0\v` も、そのゴーストの窓の全部に効く。ゆえに `cue` の演者の欄は読まない。
//!
//! # 宛名の規律
//!
//! 開けない荷物の `set` を警告する持ち主は `ZOrderCueSink` のままとし、ここは `debug!` だけ
//! に留める（二重の警告を出さない）。
//!
//! # 黙って諦めない
//!
//! 読み飛ばし・読めない値・余り・送出の失敗のいずれも記録してから抜ける。受信端が閉じて
//! いても台本は殺さない。

use std::sync::mpsc::Sender;

use areka_parsers::sakura::RAISE_TAG_CARRIER;
use dola::cue::TalkCue;
use tracing::{debug, warn};
use wintf::ecs::world::tick_wake;

/// 常に手前の指令（取り出しの相が状態へ当てる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStateDirective {
    /// `\![set,windowstate,stayontop]`（on = true）・`\![set,windowstate,!stayontop]`（on = false）
    StayOnTop { on: bool },
    /// `\v`：今いる帯の最上へ 1 回出す（状態は変えない）
    Raise,
}

/// [`parse_window_state`] の読み分けの結果。
#[derive(Debug, PartialEq, Eq)]
pub enum ParsedWindowState<'a> {
    /// 指令が決まった。`extra` は第 2 引数より後ろの余り（`\v` では引数の全部）。
    Directive {
        directive: WindowStateDirective,
        extra: &'a [&'a str],
    },
    /// 第 2 引数が `stayontop`・`!stayontop` 以外（`minimize`・空・大文字の混じった綴り・無い）
    Unrecognized { value: Option<&'a str> },
    /// 自分宛でない
    NotMine,
}

/// コマンド名（窓の状態の指定）。
const NAME_SET: &str = "set";
/// 第 1 引数（選別子）——`set` のうち本 sink が担当するのはこの選別子を伴う組だけである。
const SELECTOR_WINDOWSTATE: &str = "windowstate";

/// 汎用キャリアの名前と引数の列から指令を読み分ける（純関数）。
///
/// `set` では `params[0]` が選別子 `windowstate`・`params[1]` が第 2 引数・`params[2..]` が
/// 余り。`\v`（[`RAISE_TAG_CARRIER`]）では引数の全部が余り。
pub fn parse_window_state<'a>(name: &str, params: &'a [&'a str]) -> ParsedWindowState<'a> {
    if name == RAISE_TAG_CARRIER {
        return ParsedWindowState::Directive {
            directive: WindowStateDirective::Raise,
            extra: params,
        };
    }
    if name != NAME_SET || params.first() != Some(&SELECTOR_WINDOWSTATE) {
        return ParsedWindowState::NotMine;
    }
    // 小文字ちょうどで比べる（大文字の混じった綴りは読めない値・要件 1.7）。
    let on = match params.get(1).copied() {
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cwindowstate_2cstayontop_5d:1
        Some("stayontop") => true,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1
        Some("!stayontop") => false,
        value => return ParsedWindowState::Unrecognized { value },
    };
    ParsedWindowState::Directive {
        directive: WindowStateDirective::StayOnTop { on },
        extra: &params[2..],
    }
}

/// `\v`／`\![set,windowstate,…]` の受け口（design「WindowStateCueSink」）。
#[derive(Clone)]
pub struct WindowStateCueSink {
    /// 取り出しの相への送出端。
    tx: Sender<WindowStateDirective>,
}

impl WindowStateCueSink {
    /// 送信端（受信端は結線が持つ）から受け口を組む。
    pub fn new(tx: Sender<WindowStateDirective>) -> Self {
        Self { tx }
    }
}

impl dola::cue::CueSink for WindowStateCueSink {
    fn emit(&mut self, cue: TalkCue) {
        // 1) 開封。開けない荷物は宛名に関わらず `debug!` だけ（警告は `ZOrderCueSink` の持ち場）。
        let Some((name, params)) = cue.command.as_command_carrier() else {
            debug!(
                command = ?cue.command,
                "WindowStateCueSink: 開けない荷物・キャリアでない cue を良性に読み飛ばす（担当外・宛名の規律）"
            );
            return;
        };

        // 2) 自己選別と読み分け。
        let directive = match parse_window_state(name, &params) {
            ParsedWindowState::Directive { directive, extra } => {
                if !extra.is_empty() {
                    debug!(
                        ?directive,
                        ?extra,
                        "WindowStateCueSink: 第 2 引数で決め、余りの引数は使わない"
                    );
                }
                directive
            }
            ParsedWindowState::Unrecognized { value } => {
                warn!(
                    ?value,
                    "WindowStateCueSink: windowstate の第 2 引数が読めないので常に手前の状態を変えない"
                );
                return;
            }
            ParsedWindowState::NotMine => {
                debug!(
                    name,
                    selector = params.first().copied().unwrap_or_default(),
                    "WindowStateCueSink: 担当外のコマンドを良性に読み飛ばす（自己選別）"
                );
                return;
            }
        };

        // 3) 送り出し。受信端が閉じていても台本は殺さない（記録して継続）。
        if self.tx.send(directive).is_err() {
            warn!(
                ?directive,
                "WindowStateCueSink: 常に手前の指令を送り出せなかった（受信端が閉じている）"
            );
        }

        // 4) 重なりの仕事が増えた旗（兄弟の重なりの受け口と共用）を送り出しの後に立てる。
        tick_wake::mark(tick_wake::ZORDER);
    }
}

#[cfg(test)]
#[path = "window_state_cue_tests.rs"]
mod tests;
