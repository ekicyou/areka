//! # lookahead — 合図 1 つで状態を進める唯一の規則（純粋層）
//!
//! 本番の合図の適用（[`crate::actor::TextLayerRuntime::apply_cue`]）と、トークの先渡しを
//! 空回しで流す側は、どちらも [`advance_state`] を呼んで状態を進める。行き先（`\s` の解決を
//! 含む）を求める規則を 2 つに分けて持たないためである（要件 2.2）。
//!
//! **層規律**: 純粋層。時計も窓も `World` も見ない（`PURE_SOURCES` に載せて走査する）。

use areka_sakura::contract::{CueCommand, TalkCue};
use tracing::debug;

use crate::state::{SurfaceKeyOutcome, TextLayerState};

/// 合図 1 つで状態を進める。`\s` は `resolve` で番号に解いて行き先へ渡し（閉包が無ければ読まない）、
/// そのあと `state.apply_cue(cue)` を呼ぶ。
///
/// 同じ状態・同じ閉包・同じ合図なら同じ結果になる。描画の全消し要求や選択肢の写しの後始末は
/// 状態の外の話なので、ここでは扱わない（呼ぶ側の結線が持つ）。
pub(crate) fn advance_state(
    state: &mut TextLayerState,
    resolve: Option<&dyn Fn(&str) -> SurfaceKeyOutcome>,
    cue: &TalkCue,
) {
    // `\s` は解決の閉包で番号に解いて行き先へ渡す（要件 4.1）。閉包が無いあいだは
    // 読まない＝行き先は普通のバルーンのまま（要件 5.4）。
    if let CueCommand::Emote { key } = &cue.command {
        match resolve {
            Some(resolve) => state.route_surface(&cue.actor, resolve(key)),
            None => {
                debug!(actor = %cue.actor, key, "解決の閉包が無い——\\s を読まない（行き先は普通のバルーン）")
            }
        }
    }
    state.apply_cue(cue);
}
