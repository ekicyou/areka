//! `dump_balloon` の処理（spec: areka-P0-mcp-dump-images）——バルーンの背景と文字を重ねて PNG で返す。
//!
//! 入口は `dump_surface` と同じ（結線状態が無い → 窓が無い答え・装着の相がまだ → [`super::later`]）。
//! 背景はバルーンの対象の最後に表示した絵（隠していても残る）、文字は普通のバルーンの文字の面の写し、
//! その上に矢印が出ていれば印の面の写し（spec: balloon-scroll-arrows・要件 3.6）。
//! 背景の写しと文字の面の写しの積み込みは UI スレッドで、写しの読み出しは後のフレームで待たずに覗き
//! （GPU の終わりを待たない・要件 1.2）、重ね合わせ・PNG・base64 は別のスレッドで行う。
//! シェルの絵の中の箱の文字の面は読まない（要件 3.6）。読むだけで、表示・隠れている状態・台詞を
//! 変えない（要件 3.8・6.1・6.2）。

use std::time::Instant;

use areka_emo_text::surface::PendingReadBack;
use areka_mcp::ToolOutcome;
use areka_mcp::tools::dump_balloon::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use areka_sakura::ActorKey;
use bevy_ecs::world::World;
use tracing::debug;
use wintf::ecs::Arrangement;

use super::dump_surface::{
    Job, PresenterFacts, Reader, Step, check_size, fail, image, judge, refuse, start,
};
use super::resolve::ActiveGhost;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::target_map::balloon_target;

#[path = "dump_balloon_overlay.rs"]
mod overlay;

const TOOL: &str = "dump_balloon";

/// 今答えられればその場で（成功は別のスレッドから）、装着の相がまだなら `later` に預けて答える（要件 6.1）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    start(world, TOOL, ghost, reply, move |w| answer(w, &args));
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
    let scope = match judge::judge_balloon(Some(&PresenterFacts(presenter)), args.scope) {
        Ok(scope) => scope,
        Err(reason) => return Some(Step::Now(refuse(TOOL, args.scope, reason))),
    };
    let failed = |reason: &str| Some(Step::Now(fail(TOOL, scope, reason)));
    let target = balloon_target(scope);
    let pic = match presenter.last_shown(target) {
        Some((_, Some(pic))) => pic,
        Some((_, None)) => return failed("the last shown picture is no longer kept"),
        None => return failed("the balloon of this scope is not ready"),
    };
    let (width, height) = (pic.width(), pic.height());
    if let Err(e) = check_size(TOOL, scope, pic.bytes(), width, height) {
        return Some(Step::Now(e));
    }
    let canvas = pic.bytes().to_vec();

    // 文字の面が在るときだけ重ねる。無ければ背景だけで成功する（要件 3.9）。
    let runtime = wiring.runtime().borrow();
    let text = presenter
        .text_slot_view(target)
        .zip(runtime.surface(&ActorKey::from(scope.to_string())));
    let Some((view, surface)) = text else {
        return Some(Step::Encode(
            scope,
            balloon_job(scope, canvas, (width, height), None, None),
        ));
    };
    let Some(arrangement) = world.get::<Arrangement>(view.slot()) else {
        return failed("the text slot has no arrangement");
    };
    if view.surface_size() != (width, height) {
        return failed("picture size mismatch");
    }
    // 呼び出しを受けた時点の文字の面の写しを積む（要件 1.3）。読み出しは後のフレームで覗く。
    let pending = match surface.begin_read_back() {
        Ok(pending) => pending,
        Err(e) => return failed(&e.to_string()),
    };
    // 矢印が出ていれば印の面（バルーンの絵全体の物理寸・窓の原点）の写しも同じ時点で積む（要件 3.6）。
    let marker = match runtime
        .marker_surface(&ActorKey::from(scope.to_string()))
        .map(|m| m.begin_read_back().map(|p| (p, m.size())))
        .transpose()
    {
        Ok(marker) => marker,
        Err(e) => return failed(&e.to_string()),
    };
    let offset = (arrangement.offset.x, arrangement.offset.y);
    Some(Step::Read(
        scope,
        reader(
            scope,
            (pending, surface.size()),
            marker,
            canvas,
            (width, height),
            view.physical_size(),
            offset,
        ),
    ))
}

/// 覗かれるたびに待たずに読む口（要件 1.2・1.5）。文字の面と（あれば）印の面が読めたら大きさを
/// 確かめ、重ねる仕事を返す。先に読めた方は持っておき、もう片方を待つ。
fn reader(
    scope: u32,
    text: (PendingReadBack, (u32, u32)),
    marker: Option<(PendingReadBack, (u32, u32))>,
    canvas: Vec<u8>,
    size: (u32, u32),
    physical: (u32, u32),
    offset: (f32, f32),
) -> Reader {
    let mut canvas = Some(canvas);
    let mut text_bytes = None;
    let mut marker_bytes = None;
    Box::new(move || {
        if !read_into(&mut text_bytes, &text, scope)? {
            return Ok(None);
        }
        if let Some(m) = &marker
            && !read_into(&mut marker_bytes, m, scope)?
        {
            return Ok(None);
        }
        // 読めた後は覗かれない（覗く関数が符号化待ちへ移る）。届いたら黙らせず想定外の失敗。
        let (Some(canvas), Some(bytes)) = (canvas.take(), text_bytes.take()) else {
            return Err(fail(TOOL, scope, "the text read was already taken"));
        };
        let layer = (physical, bytes, text.1, offset);
        let marker_layer = marker_bytes.take().zip(marker.as_ref().map(|m| m.1));
        Ok(Some(balloon_job(
            scope,
            canvas,
            size,
            Some(layer),
            marker_layer,
        )))
    })
}

/// 面の写しを待たずに 1 度覗き、読めたら大きさを確かめて `slot` に持つ。持っていれば `true`。
fn read_into(
    slot: &mut Option<Vec<u8>>,
    (pending, (w, h)): &(PendingReadBack, (u32, u32)),
    scope: u32,
) -> Result<bool, ToolOutcome> {
    if slot.is_none() {
        let bytes = match pending.try_finish() {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(false),
            Err(e) => return Err(fail(TOOL, scope, &e.to_string())),
        };
        if bytes.len() != *w as usize * *h as usize * 4 {
            return Err(fail(TOOL, scope, "picture size mismatch"));
        }
        *slot = Some(bytes);
    }
    Ok(true)
}

/// 文字の面の読み出し（物理の大きさ, 密な BGRA, 面の大きさ, 文字の領域の原点）。
type TextLayer = ((u32, u32), Vec<u8>, (u32, u32), (f32, f32));

/// 印の面の読み出し（密な BGRA, 面の大きさ）。面はバルーンの絵全体の物理寸で窓の原点に在る。
type MarkerLayer = (Vec<u8>, (u32, u32));

/// 背景（乗算済み BGRA の写し）に文字を重ね、PNG・base64 にして成功で答える仕事。文字の面が
/// ある・ないの両方の腕でこれを使う。
fn balloon_job(
    scope: u32,
    mut canvas: Vec<u8>,
    size: (u32, u32),
    layer: Option<TextLayer>,
    marker: Option<MarkerLayer>,
) -> Job {
    let (width, height) = size;
    Box::new(move |ui| {
        let started = Instant::now();
        if let Some((physical, bytes, text_size, offset)) = layer {
            overlay::overlay_text(&mut canvas, size, physical, &bytes, text_size, offset);
            // 印の面は文字より手前（画面と同じ重なり）。ずらしは無い（要件 3.6）。
            if let Some((bytes, marker_size)) = marker {
                overlay::overlay_text(&mut canvas, size, physical, &bytes, marker_size, (0.0, 0.0));
            }
        }
        let png = image::png_base64(&canvas, width, height);
        debug!(
            tool = TOOL,
            scope,
            width,
            height,
            base64_len = png.len(),
            ui_us = ui.as_micros() as u64,
            encode_us = started.elapsed().as_micros() as u64,
            "[mcp] 絵を返す"
        );
        outcome::with_image(outcome::ok(&judge::balloon_text(scope)), png)
    })
}

#[cfg(test)]
#[path = "dump_balloon_tests.rs"]
mod dump_balloon_tests;

#[cfg(all(test, target_pointer_width = "64"))]
#[path = "dump_balloon_gpu_tests.rs"]
mod dump_balloon_gpu_tests;
