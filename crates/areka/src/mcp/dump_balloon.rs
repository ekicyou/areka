//! `dump_balloon` の処理（spec: areka-P0-mcp-dump-images）——バルーンの背景と文字を重ねて PNG で返す。
//!
//! 入口は `dump_surface` と同じ（結線状態が無い → 窓が無い答え・装着の相がまだ → [`super::later`]）。
//! 背景はバルーンの対象の最後に表示した絵（隠していても残る）、文字は普通のバルーンの文字の面の読み戻し。
//! シェルの絵の中の箱の文字の面は読まない（要件 3.6）。読むだけで、表示・隠れている状態・台詞を
//! 変えない（要件 3.8・6.2）。

use areka_mcp::ToolOutcome;
use areka_mcp::tools::dump_balloon::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use areka_sakura::ActorKey;
use bevy_ecs::world::World;
use tracing::debug;
use wintf::ecs::Arrangement;

use super::dump_surface::{PresenterFacts, encode, fail, judge, refuse};
use super::resolve::ActiveGhost;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::target_map::balloon_target;

#[path = "dump_balloon_overlay.rs"]
mod overlay;

const TOOL: &str = "dump_balloon";

/// 今答えられればその場で、装着の相がまだなら `later` に預けて答える（要件 6.1）。
pub(super) fn handle(world: &mut World, _ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    match answer(world, &args) {
        Some(result) => reply.send(result),
        None => super::later(world, reply, move |w| answer(w, &args)),
    }
}

/// 今答えられるなら `Some`。装着の相がまだなら `None`（`later` が次のフレームでもう 1 度呼ぶ）。
fn answer(world: &World, args: &Args) -> Option<ToolOutcome> {
    let Some(wiring) = world.get_non_send::<Emo2Wiring>() else {
        return Some(refuse(TOOL, args.scope, judge::NO_WINDOW));
    };
    if !wiring.attached() {
        return None;
    }
    let presenter = wiring.presenter();
    let scope = match judge::judge_balloon(Some(&PresenterFacts(presenter)), args.scope) {
        Ok(scope) => scope,
        Err(reason) => return Some(refuse(TOOL, args.scope, reason)),
    };
    let target = balloon_target(scope);
    let pic = match presenter.last_shown(target) {
        Some((_, Some(pic))) => pic,
        Some((_, None)) => {
            return Some(fail(
                TOOL,
                scope,
                "the last shown picture is no longer kept",
            ));
        }
        None => return Some(fail(TOOL, scope, "the balloon of this scope is not ready")),
    };
    let (width, height) = (pic.width(), pic.height());
    let mut canvas = pic.bytes().to_vec();
    if canvas.len() != width as usize * height as usize * 4 {
        return Some(fail(TOOL, scope, "picture size mismatch"));
    }

    // 文字の面が在るときだけ重ねる。無ければ背景だけで成功する（要件 3.9）。
    let runtime = wiring.runtime().borrow();
    let text = presenter
        .text_slot_view(target)
        .zip(runtime.surface(&ActorKey::from(scope.to_string())));
    if let Some((view, surface)) = text {
        let Some(arrangement) = world.get::<Arrangement>(view.slot()) else {
            return Some(fail(TOOL, scope, "the text slot has no arrangement"));
        };
        let bytes = match surface.read_back() {
            Ok(bytes) => bytes,
            Err(e) => return Some(fail(TOOL, scope, &e.to_string())),
        };
        let (tw, th) = surface.size();
        if view.surface_size() != (width, height) || bytes.len() != tw as usize * th as usize * 4 {
            return Some(fail(TOOL, scope, "picture size mismatch"));
        }
        overlay::overlay_text(
            &mut canvas,
            (width, height),
            view.physical_size(),
            &bytes,
            (tw, th),
            (arrangement.offset.x, arrangement.offset.y),
        );
    }

    Some(match encode(TOOL, scope, &canvas, width, height) {
        Ok(png) => {
            debug!(
                tool = TOOL,
                scope,
                width,
                height,
                base64_len = png.len(),
                "[mcp] 絵を返す"
            );
            outcome::with_image(outcome::ok(&judge::balloon_text(scope)), png)
        }
        Err(failed) => failed,
    })
}

#[cfg(test)]
#[path = "dump_balloon_tests.rs"]
mod dump_balloon_tests;
