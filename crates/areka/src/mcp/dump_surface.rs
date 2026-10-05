//! `dump_surface` の処理（spec: areka-P0-mcp-dump-images）——事実を集めて判断し、絵を PNG で返す。
//!
//! 結線状態が無い → 窓が無い答え。装着の相がまだ → 答えずに [`super::later`] へ預けて毎フレーム
//! やり直す。それ以外 → 判断して、今の見た目は最後に表示した絵を、指定は単体の合成の絵を返す。
//! 判断と絵の写しは UI スレッドで、乗算の戻し・PNG・base64 は別のスレッドで行い、そのスレッドから
//! 答える（[`reply_elsewhere`]）。読むだけで、ゴーストへイベントを送らず、台本を再生せず、
//! ファイルを書かない（要件 6.1・6.2）。

use std::sync::mpsc;
use std::time::{Duration, Instant};

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

#[path = "dump_surface_judge.rs"]
pub(in crate::mcp) mod judge;

#[path = "dump_surface_image.rs"]
pub(in crate::mcp) mod image;

const TOOL: &str = "dump_surface";

/// 符号化を受け持つスレッドの名前。
const ENCODE_THREAD: &str = "mcp-encode";

/// 別のスレッドで行う残りの仕事。引数は UI スレッドの側の所要時間（成功の記録に載せる）。
pub(in crate::mcp) type Job = Box<dyn FnOnce(Duration) -> ToolOutcome + Send>;

/// UI スレッドの側の答え。
pub(in crate::mcp) enum Step {
    /// その場で答える（判断で返す失敗・写しまでで起きた想定外の失敗）。
    Now(ToolOutcome),
    /// 写しまで済んだ成功（スコープ, 残りの仕事）。乗算の戻し・重ね合わせ・PNG・base64 と成功の記録を残す。
    Encode(u32, Job),
}

impl Step {
    /// 今のスレッドで終える（装着の前に預けた組。`later` は答えをその場で受け取るので逃がせない）。
    pub(in crate::mcp) fn here(self, started: Instant) -> ToolOutcome {
        match self {
            Step::Now(outcome) => outcome,
            Step::Encode(_, job) => job(started.elapsed()),
        }
    }
}

/// 今答えられればその場で（成功は別のスレッドから）、装着の相がまだなら `later` に預けて答える（要件 6.1）。
pub(super) fn handle(world: &mut World, _ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    let started = Instant::now();
    match answer(world, &args) {
        Some(step) => reply_elsewhere(TOOL, step, started, reply),
        None => super::later(world, reply, move |w| {
            let started = Instant::now();
            answer(w, &args).map(|step| step.here(started))
        }),
    }
}

/// その場の答えはその場で送り、写しまで済んだ成功は別のスレッドで仕上げてそのスレッドから送る
/// （`ReplyTo` はスレッドをまたげる・`later` を通さない）。スレッドを起こせなければ想定外の失敗として
/// その場で答える。
pub(in crate::mcp) fn reply_elsewhere(
    tool: &'static str,
    step: Step,
    started: Instant,
    reply: ReplyTo,
) {
    let (scope, job) = match step {
        Step::Now(outcome) => return reply.send(outcome),
        Step::Encode(scope, job) => (scope, job),
    };
    let ui = started.elapsed();
    // 仕事と返事は起こせた後で渡す（起こせなければ手元に残り、その場で答えられる）。
    let (tx, rx) = mpsc::channel::<(Job, ReplyTo)>();
    let spawned = std::thread::Builder::new()
        .name(ENCODE_THREAD.to_owned())
        .spawn(move || {
            if let Ok((job, reply)) = rx.recv() {
                // panic で返事を落とすと入口が「終了中」と誤って答えるので、ここで受けて記録する。
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(ui)))
                    .unwrap_or_else(|_| fail(tool, scope, "the encoding thread panicked"));
                reply.send(outcome);
            }
        });
    match spawned {
        Ok(_) => {
            if let Err(mpsc::SendError((_, reply))) = tx.send((job, reply)) {
                reply.send(fail(tool, scope, "the encoding thread is gone"));
            }
        }
        Err(e) => reply.send(fail(tool, scope, &e.to_string())),
    }
}

/// 今答えられるなら `Some`。装着の相がまだなら `None`（`later` が次のフレームでもう 1 度呼ぶ）。
fn answer(world: &World, args: &Args) -> Option<Step> {
    let Some(wiring) = world.get_non_send::<Emo2Wiring>() else {
        return Some(Step::Now(refuse(TOOL, args.scope, judge::NO_WINDOW)));
    };
    if !wiring.attached() {
        return None;
    }
    let presenter = wiring.presenter();
    let plan =
        match judge::judge_surface(Some(&PresenterFacts(presenter)), args.scope, args.surface) {
            Ok(plan) => plan,
            Err(reason) => return Some(Step::Now(refuse(TOOL, args.scope, reason))),
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
                _ => Step::Now(fail(
                    TOOL,
                    scope,
                    "the last shown picture is no longer kept",
                )),
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
                Some(Err(e)) => Step::Now(fail(TOOL, scope, &e.to_string())),
                // 判断が同じ表示の層でスコープの登録を確かめた後なので、ここへは来ない。
                None => Step::Now(fail(TOOL, scope, judge::NO_SUCH_SCOPE)),
            }
        }
    })
}

/// 絵を写して、本文＋画像 1 枚の成功を別のスレッドで仕上げる（要件 1.4・2.4・5.4）。大きさが合わなければ
/// その場で想定外の失敗。
fn picture(
    scope: u32,
    surface_id: u32,
    text: String,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Step {
    if let Err(failed) = check_size(TOOL, scope, bytes, width, height) {
        return Step::Now(failed);
    }
    let bytes = bytes.to_vec();
    Step::Encode(
        scope,
        Box::new(move |ui| {
            let started = Instant::now();
            let png = image::png_base64(&bytes, width, height);
            debug!(
                tool = TOOL,
                scope,
                surface_id,
                width,
                height,
                base64_len = png.len(),
                ui_us = ui.as_micros() as u64,
                encode_us = started.elapsed().as_micros() as u64,
                "[mcp] 絵を返す"
            );
            outcome::with_image(outcome::ok(&text), png)
        }),
    )
}

/// 乗算済み BGRA を符号化できる大きさか。幅か高さが 0、またはバイト数が幅×高さ×4 と合わなければ
/// 想定外の失敗を返す（`png_base64` の前提を守る）。
pub(in crate::mcp) fn check_size(
    tool: &str,
    scope: u32,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<(), ToolOutcome> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if width == 0 || height == 0 || expected != Some(bytes.len()) {
        return Err(fail(tool, scope, "picture size mismatch"));
    }
    Ok(())
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

/// 答えを上限つきで待つ（成功は別のスレッドから届く・テスト用）。
#[cfg(test)]
pub(in crate::mcp) trait WaitAnswer {
    /// 届いた答え。期限切れ・答えずに手放したら `None`。
    fn wait_answer(&self) -> Option<areka_mcp::tools::Answer>;
}

#[cfg(test)]
impl WaitAnswer for areka_mcp::tools::Pending {
    fn wait_answer(&self) -> Option<areka_mcp::tools::Answer> {
        let mut got = None;
        crate::emo2_boot::spine::spin_wait_until(|| match self.try_answer() {
            Ok(Some(answer)) => {
                got = Some(answer);
                true
            }
            Ok(None) => false,
            Err(_) => true,
        });
        got
    }
}

#[cfg(test)]
#[path = "dump_surface_tests.rs"]
mod dump_surface_tests;

#[cfg(all(test, target_pointer_width = "64"))]
#[path = "dump_surface_gpu_test_support.rs"]
pub(in crate::mcp) mod gpu_test_support;

#[cfg(all(test, target_pointer_width = "64"))]
#[path = "dump_surface_gpu_tests.rs"]
mod dump_surface_gpu_tests;
