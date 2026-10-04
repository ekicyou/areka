//! `dump_surface` の処理（spec: areka-P0-mcp-dump-images）——事実を集めて判断し、絵を PNG で返す。
//!
//! 結線状態が無い → 窓が無い答え。装着の相がまだ → 答えずに [`super::later`] へ預けて毎フレーム
//! やり直す。それ以外 → 判断して、今の見た目は最後に表示した絵を、指定は単体の合成の絵を返す。
//! 読むだけで、ゴーストへイベントを送らず、台本を再生せず、ファイルを書かない（要件 6.2）。

use areka_emo_present::EmoPresenter;
use areka_mcp::ToolOutcome;
use areka_mcp::tools::dump_surface::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;
use tracing::{debug, error};

use super::resolve::ActiveGhost;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::target_map::shell_target;
use judge::SurfacePlan;

// `judge_balloon`・`balloon_text` の呼び手は `dump_balloon`（次の配線）。生えるまで未使用の警告を抑える。
#[allow(dead_code)]
#[path = "dump_surface_judge.rs"]
pub(in crate::mcp) mod judge;

#[path = "dump_surface_image.rs"]
pub(in crate::mcp) mod image;

const TOOL: &str = "dump_surface";

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
    let plan =
        match judge::judge_surface(Some(&PresenterFacts(presenter)), args.scope, args.surface) {
            Ok(plan) => plan,
            Err(reason) => return Some(refuse(TOOL, args.scope, reason)),
        };
    Some(match plan {
        SurfacePlan::Shown { scope, surface_id } => {
            match presenter.last_shown(shell_target(scope)) {
                Some((_, Some(pic))) => picture(
                    scope,
                    surface_id,
                    judge::shown_text(scope, surface_id),
                    pic.bytes(),
                    pic.width(),
                    pic.height(),
                ),
                _ => fail(TOOL, scope, "the last shown picture is no longer kept"),
            }
        }
        SurfacePlan::Alone { scope, surface_id } => {
            match presenter.compose_alone(shell_target(scope), surface_id) {
                Some(Ok(pic)) => picture(
                    scope,
                    surface_id,
                    judge::alone_text(scope, surface_id),
                    pic.bytes(),
                    pic.width(),
                    pic.height(),
                ),
                Some(Err(e)) => fail(TOOL, scope, &e.to_string()),
                // 判断が同じ表示の層でスコープの登録を確かめた後なので、ここへは来ない。
                None => fail(TOOL, scope, judge::NO_SUCH_SCOPE),
            }
        }
    })
}

/// 本文＋画像 1 枚の成功（要件 1.4・2.4・5.4）。大きさが合わなければ想定外の失敗。
fn picture(
    scope: u32,
    surface_id: u32,
    text: String,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> ToolOutcome {
    match encode(TOOL, scope, bytes, width, height) {
        Ok(png) => {
            debug!(
                tool = TOOL,
                scope,
                surface_id,
                width,
                height,
                base64_len = png.len(),
                "[mcp] 絵を返す"
            );
            outcome::with_image(outcome::ok(&text), png)
        }
        Err(failed) => failed,
    }
}

/// 乗算済み BGRA を PNG の base64 へ。幅か高さが 0、またはバイト数が幅×高さ×4 と合わなければ
/// 符号化を呼ばずに想定外の失敗を返す（`png_base64` の前提を守る）。
pub(in crate::mcp) fn encode(
    tool: &str,
    scope: u32,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<String, ToolOutcome> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if width == 0 || height == 0 || expected != Some(bytes.len()) {
        return Err(fail(tool, scope, "picture size mismatch"));
    }
    Ok(image::png_base64(bytes, width, height))
}

/// 判断で返す失敗（窓・スコープ・surface ID・未表示）。記録は `debug!` だけ（要件 4.6）。
pub(in crate::mcp) fn refuse(tool: &str, scope: Option<i64>, reason: &str) -> ToolOutcome {
    debug!(tool, scope = ?scope, reason, "[mcp] 撮れない");
    outcome::ng(reason)
}

/// 想定外の失敗。`error!` 1 件にツール名・スコープ・理由を載せ、画像なしの `NG:` を返す（要件 4.7・4.8）。
pub(in crate::mcp) fn fail(tool: &str, scope: u32, reason: &str) -> ToolOutcome {
    error!(tool, scope, reason, "[mcp] 絵を返せなかった");
    outcome::ng(reason)
}

/// 表示の層を読んで判断の事実に答える（`dump_balloon` も使う）。
pub(in crate::mcp) struct PresenterFacts<'a>(pub(in crate::mcp) &'a EmoPresenter);

impl judge::ShellFacts for PresenterFacts<'_> {
    /// そのスコープのシェルの target が表示の層に登録されている＝絵の資産が組まれている。
    fn scope_exists(&self, scope: u32) -> bool {
        self.0.target_visible(shell_target(scope)).is_some()
    }

    fn surface_exists(&self, scope: u32, surface_id: u32) -> bool {
        self.0.has_surface(shell_target(scope), surface_id) == Some(true)
    }

    fn last_shown(&self, scope: u32) -> Option<u32> {
        self.0.last_shown(shell_target(scope)).map(|(id, _)| id)
    }
}

#[cfg(test)]
#[path = "dump_surface_tests.rs"]
mod dump_surface_tests;
