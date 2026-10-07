//! `dump_surface` の実際の描画を通るテスト（spec: areka-P0-mcp-dump-images・要件 1.1・1.3〜1.6・
//! 2.1〜2.3・4.1〜4.3・4.6・5.2・5.3・6.1・6.2・7.4 ⑴⑵⑹、spec: areka-P0-mcp-dump-images-residue・
//! 要件 5.1・x64 だけ接続）。土台は [`super::gpu_test_support`]。

use std::time::Duration;

use areka_kanade::RaiseOutcome;
use areka_mcp::tools::dump_surface::Args;
use areka_mcp::{ToolContent, ToolOutcome};
use bevy_ecs::world::World;
use log_capture_kit::count_levels;

use super::dump_surface_tests::is_picture;
use super::gpu_test_support::{FLUSH_EVENT, GpuRig, decode_png};
use super::{Step, WaitAnswer, finish};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::spine::RecordedCall;
use crate::emo2_boot::target_map::shell_target;

/// 検体（emo2）の `surfaces.txt` に在る、アニメーションも着せ替えも持たない surface（`element0` 1 枚）。
/// 起動のときの 0 と違う番号で、台本で出したことが ID に現れる。
const SHOWN: u32 = 1101;

/// 今の見た目（要件 1.1・1.3・1.4）: 台本 `\0\s[1101]` の後の `dump_surface`（省略）の PNG を
/// 復号した画素が、同じ surface を合成器で合成して乗算を戻した絵と一致し、本文が今の見た目の逐語。
/// 窓の `DPI` を 144（拡大率 1.5）にして、拡大を掛ける前の原寸であることも同時に固定する。
///
/// # 非空虚性
/// 乗算の戻しで色の並びを取り違えると画素が一致せず赤。拡大した絵を返すと大きさが合わず赤。
/// 本文が別の言い方（指定の文言など）だと逐語の比較で赤。
#[test]
fn shown_surface_matches_the_composed_pixels_at_native_size() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 144);
    let shown = gpu.frames_until(100, |world| {
        world
            .get_non_send::<Emo2Wiring>()
            .and_then(|w| w.presenter().last_shown(shell_target(0)))
            .is_some_and(|(id, pic)| id == SHOWN && pic.is_some())
    });
    let answer = gpu.dump_surface(None, None).wait_answer();
    let expected = gpu.composed_rgba(SHOWN);
    let down = gpu.shutdown();

    let answer = answer.expect("装着の後は上限のうちに答えが届く");
    let (text, (w, h, pixels), mime) = match answer.outcome.content.as_slice() {
        [
            ToolContent::Text(text),
            ToolContent::Image { data, mime_type },
        ] => (text.clone(), decode_png(data), mime_type.clone()),
        other => panic!("本文＋画像 1 枚の形ではない: {other:?}"),
    };
    let (ew, eh, expected_pixels) = expected;
    // 画素の列は大きいので、食い違った画素の数で比べる（大きさが違えば数えずに大きさで赤）。
    let differing = (pixels.len() == expected_pixels.len()).then(|| {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected_pixels.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count()
    });
    assert_eq!(
        (
            shown,
            down,
            answer.outcome.is_error,
            text,
            mime,
            (w, h),
            differing
        ),
        (
            true,
            true,
            false,
            "OK:scope 0, surface 1101 as currently shown (with running animations and dressups, \
             before scaling and transparency)"
                .to_owned(),
            "image/png".to_owned(),
            (ew, eh),
            Some(0),
        ),
        "（台本の surface が出た, 降ろせた, isError, 本文, MIME, 復号した（幅, 高さ）, 食い違った画素の数）"
    );
    assert!(ew > 0 && eh > 0, "比べる絵が空ではない");
}

/// 表示中（[`SHOWN`]）と別の、同じスコープ 0 用の surface（`element0` 1 枚・アニメーションも着せ替えも無い）。
const OTHER: u32 = 1100;
/// 検体のスコープ 1 用の代表の surface（`surfaces.txt` の「\1の代表サーフェス」・まばたきつき）。
const KERO: u32 = 10;
/// 検体のスコープ 1 用の表情の surface（`element0` 1 枚＋まばたき）。
const KERO_FACE: u32 = 2100;
/// 検体の `kero.surface.alias` にだけ在る番号（`100,[2100]`・生の `surface100` は無い）。
const ALIAS_ONLY: i64 = 100;
/// 読み戻しの後に回す巡の数（遅れて届く変化を待つ）。
const SETTLE_FRAMES: usize = 30;

/// スコープ `scope` に `id` の表示が成立し、その絵が合成メモに在る。
fn is_shown(world: &World, scope: u32, id: u32) -> bool {
    world
        .get_non_send::<Emo2Wiring>()
        .and_then(|w| w.presenter().last_shown(shell_target(scope)))
        .is_some_and(|(shown, pic)| shown == id && pic.is_some())
}

/// 画面の状態＝スコープ 0・1 の（今の surface, 可視）。読み戻しの前後で比べる。
fn screen(world: &World) -> [(Option<u32>, Option<bool>); 2] {
    let presenter = world.non_send::<Emo2Wiring>().presenter();
    [0, 1].map(|scope| {
        (
            presenter.current_surface_id(shell_target(scope)),
            presenter.target_visible(shell_target(scope)),
        )
    })
}

/// 答えを照合する形（isError, 本文, MIME, 復号した（幅, 高さ）, 比べる絵と食い違った画素の数）。
type PictureFacts = (bool, String, String, (u32, u32), Option<usize>);

/// 本文＋画像 1 枚の答えを [`PictureFacts`] に読む。大きさが違えば画素は数えず `None`。形が違えば panic。
fn read_picture(outcome: &ToolOutcome, expected: &(u32, u32, Vec<u8>)) -> PictureFacts {
    let [
        ToolContent::Text(text),
        ToolContent::Image { data, mime_type },
    ] = outcome.content.as_slice()
    else {
        panic!("本文＋画像 1 枚の形ではない: {:?}", outcome.content);
    };
    let (w, h, pixels) = decode_png(data);
    let differing = (pixels.len() == expected.2.len()).then(|| {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.2.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count()
    });
    (
        outcome.is_error,
        text.clone(),
        mime_type.clone(),
        (w, h),
        differing,
    )
}

/// [`read_picture`] の期待（成功・PNG・`expected` と同じ大きさ・食い違い 0）。
fn picture_of(text: &str, expected: &(u32, u32, Vec<u8>)) -> PictureFacts {
    assert!(expected.0 > 0 && expected.1 > 0, "比べる絵が空ではない");
    (
        false,
        text.to_owned(),
        "image/png".to_owned(),
        (expected.0, expected.1),
        Some(0),
    )
}

/// 画像の無い失敗（本文 1 つだけ・`isError: true`）。
fn refusal(reason: &str) -> ToolOutcome {
    ToolOutcome {
        content: vec![ToolContent::Text(format!("NG:{reason}"))],
        is_error: true,
    }
}

/// 指定した surface（要件 2.1・2.3・2.4）: 表示中の 1101 と別の 1100 を指定すると、1100 を合成した
/// 絵の画素で返り、呼ぶ前と、呼んだ直後と、その後に巡を回した後とで、スコープ 0・1 の今の surface と
/// 可視が同じ。
///
/// # 非空虚性
/// 今の見た目（1101）を返すと画素が食い違って赤。指定した surface を表示に回すと、スコープ 0 の
/// 今の surface が 1100 に変わって赤（前の状態が 1101 の可視であることも確かめる）。
#[test]
fn specified_surface_returns_its_pixels_without_changing_the_screen() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 96);
    let shown = gpu.frames_until(100, |world| is_shown(world, 0, SHOWN));
    let before = screen(&gpu.rig.world);
    let answer = gpu.dump_surface(None, Some(i64::from(OTHER))).wait_answer();
    let right_after = screen(&gpu.rig.world);
    gpu.frames(SETTLE_FRAMES);
    let settled = screen(&gpu.rig.world);
    let expected = gpu.composed_rgba(OTHER);
    let down = gpu.shutdown();

    let answer = answer.expect("装着の後は上限のうちに答えが届く");
    assert_eq!(
        (
            shown,
            down,
            before[0],
            right_after,
            settled,
            read_picture(&answer.outcome, &expected)
        ),
        (
            true,
            true,
            (Some(SHOWN), Some(true)),
            before,
            before,
            picture_of(
                "OK:scope 0, surface 1100 rendered alone in its initial state (not what is on the \
                 screen now)",
                &expected
            )
        ),
        "（台本の surface が出た, 降ろせた, 前のスコープ 0, 直後の画面, 巡の後の画面, 答え）"
    );
}

/// スコープ 0 でスコープ 1 用の ID（要件 2.1「どのスコープ用の絵かは問わない」）: スコープを省いて
/// 10 を指定すると、10 を合成した絵の画素で成功する。
///
/// # 非空虚性
/// 判断がスコープの用途で断ると本文 1 片の失敗になり、形の照合で赤。
#[test]
fn scope_zero_renders_a_surface_meant_for_scope_one() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 96);
    let shown = gpu.frames_until(100, |world| is_shown(world, 0, SHOWN));
    let answer = gpu.dump_surface(None, Some(i64::from(KERO))).wait_answer();
    let expected = gpu.composed_rgba(KERO);
    let down = gpu.shutdown();

    let answer = answer.expect("装着の後は上限のうちに答えが届く");
    assert_eq!(
        (shown, down, read_picture(&answer.outcome, &expected)),
        (
            true,
            true,
            picture_of(
                "OK:scope 0, surface 10 rendered alone in its initial state (not what is on the \
                 screen now)",
                &expected
            )
        ),
        "（台本の surface が出た, 降ろせた, 答え）"
    );
}

/// 別名の表にだけ在る番号（要件 2.1「別名は解かない」・4.2）: 本物のシェルで 100 を指定すると
/// `NG:No such surface ID. Check get_expression_table tool`（画像なし）。
///
/// # 非空虚性
/// 100 が検体の別名の表に在り、生の surface としては無いことを `surfaces.txt` から確かめる。
/// 別名を解くと 2100 の絵で成功して赤。
#[test]
fn number_only_in_the_alias_table_is_no_such_surface() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 96);
    let shown = gpu.frames_until(100, |world| is_shown(world, 0, SHOWN));
    let surfaces = std::fs::read_to_string(
        gpu.rig
            .root
            .ghost_dir("A")
            .join("shell")
            .join("master")
            .join("surfaces.txt"),
    )
    .expect("検体の surfaces.txt を読める");
    let lines: Vec<&str> = surfaces.lines().map(str::trim).collect();
    let in_alias = lines.contains(&"kero.surface.alias") && lines.contains(&"100,[2100]");
    let raw = lines.contains(&"surface100");
    let answer = gpu.dump_surface(None, Some(ALIAS_ONLY)).wait_answer();
    let down = gpu.shutdown();

    assert_eq!(
        (shown, down, in_alias, raw, answer.map(|a| a.outcome)),
        (
            true,
            true,
            true,
            false,
            Some(refusal(
                "No such surface ID. Check get_expression_table tool"
            ))
        ),
        "（台本の surface が出た, 降ろせた, 別名の表に在る, 生の surface に在る, 答え）"
    );
}

/// 隠したキャラクター（要件 1.6）: `\s[-1]` で隠した後の `dump_surface`（省略）が、隠す直前の 1101 の
/// 絵と ID で、今の見た目の本文の成功を返す。
///
/// # 非空虚性
/// 隠れたこと（スコープ 0 が不可視）を待ってから呼ぶ。隠しているときに断る・空の絵を返すと赤。
#[test]
fn hidden_character_returns_the_picture_before_hiding() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\w2\s[-1]\e", 96);
    let hidden = gpu.frames_until(100, |world| {
        is_shown(world, 0, SHOWN)
            && world
                .non_send::<Emo2Wiring>()
                .presenter()
                .target_visible(shell_target(0))
                == Some(false)
    });
    let answer = gpu.dump_surface(None, None).wait_answer();
    let expected = gpu.composed_rgba(SHOWN);
    let down = gpu.shutdown();

    let answer = answer.expect("装着の後は上限のうちに答えが届く");
    assert_eq!(
        (hidden, down, read_picture(&answer.outcome, &expected)),
        (
            true,
            true,
            picture_of(
                "OK:scope 0, surface 1101 as currently shown (with running animations and \
                 dressups, before scaling and transparency)",
                &expected
            )
        ),
        "（1101 を出して隠した, 降ろせた, 答え）"
    );
}

/// 一度も表示していないスコープ（要件 4.3・2.3）: 台本がスコープ 1 に触れないとき、スコープ 1 の
/// 省略は `NG:No surface has been shown in this scope yet`、指定（2100）はその絵で成功する。
///
/// # 非空虚性
/// スコープ 1 の表示が一度も成立していないことを表示の層から確かめる。未表示で指定まで断ると赤。
#[test]
fn never_shown_scope_refuses_the_omitted_but_renders_the_specified() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 96);
    let shown = gpu.frames_until(100, |world| is_shown(world, 0, SHOWN));
    let never = gpu
        .rig
        .world
        .non_send::<Emo2Wiring>()
        .presenter()
        .last_shown(shell_target(1))
        .is_none();
    let omitted = gpu.dump_surface(Some(1), None).wait_answer();
    let specified = gpu
        .dump_surface(Some(1), Some(i64::from(KERO_FACE)))
        .wait_answer();
    let expected = gpu.composed_rgba(KERO_FACE);
    let down = gpu.shutdown();

    let specified = specified.expect("装着の後は上限のうちに答えが届く");
    assert_eq!(
        (
            shown,
            down,
            never,
            omitted.map(|a| a.outcome),
            read_picture(&specified.outcome, &expected)
        ),
        (
            true,
            true,
            true,
            Some(refusal("No surface has been shown in this scope yet")),
            picture_of(
                "OK:scope 1, surface 2100 rendered alone in its initial state (not what is on the \
                 screen now)",
                &expected
            )
        ),
        "（台本の surface が出た, 降ろせた, スコープ 1 は未表示, 省略の答え, 指定の答え）"
    );
}

/// 絵の資産の無いスコープ（要件 4.1）: 検体はスコープ 0・1 だけなので、スコープ 2 は省略も指定も
/// `NG:No such scope in this ghost`。
///
/// # 非空虚性
/// 指定（1101・スコープ 0 には在る）でも断ることで、surface ID より先にスコープを確かめることを見る。
#[test]
fn scope_two_is_no_such_scope() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 96);
    let shown = gpu.frames_until(100, |world| is_shown(world, 0, SHOWN));
    let omitted = gpu.dump_surface(Some(2), None).wait_answer();
    let specified = gpu
        .dump_surface(Some(2), Some(i64::from(SHOWN)))
        .wait_answer();
    let down = gpu.shutdown();

    let no_scope = Some(refusal("No such scope in this ghost"));
    assert_eq!(
        (
            shown,
            down,
            omitted.map(|a| a.outcome),
            specified.map(|a| a.outcome)
        ),
        (true, true, no_scope.clone(), no_scope),
        "（台本の surface が出た, 降ろせた, 省略の答え, 指定の答え）"
    );
}

/// 装着の前（要件 6.1）: 最初の巡を回す前（装着の相がまだ）に呼ぶとその場では答えが無く、巡を回すと
/// 後から答えが届く（指定した 1100 の絵）。
///
/// # 非空虚性
/// 呼ぶ時点で結線が在り、装着がまだであることを確かめる（結線が無いと「窓が無い」でその場で答える）。
/// 後から答える置き場へ預けないと、届かずに期限切れで赤。
#[test]
fn before_attachment_answers_after_the_frames_run() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 96);
    let attached = gpu
        .rig
        .world
        .get_non_send::<Emo2Wiring>()
        .map(|w| w.attached());
    let pending = gpu.dump_surface(None, Some(i64::from(OTHER)));
    let at_once = pending.try_answer().ok().flatten().map(|a| a.outcome);
    let mut later = None;
    let arrived = gpu.frames_until(100, |_| {
        later = pending.try_answer().ok().flatten();
        later.is_some()
    });
    let expected = gpu.composed_rgba(OTHER);
    let down = gpu.shutdown();

    let later = later.expect("巡を回した後は答えが届く");
    assert_eq!(
        (
            attached,
            at_once,
            arrived,
            down,
            read_picture(&later.outcome, &expected)
        ),
        (
            Some(false),
            None,
            true,
            true,
            picture_of(
                "OK:scope 0, surface 1100 rendered alone in its initial state (not what is on the \
                 screen now)",
                &expected
            )
        ),
        "（呼ぶ時点の装着, その場の答え, 届いた, 降ろせた, 届いた答え）"
    );
}

/// 邪魔をしない（要件 6.2・4.6）: 成功と判断の失敗を混ぜた一連の呼び出しの前後で、偽の SHIORI の
/// 呼び出しの列が、後の関所の印 1 件のほかに増えず、ERROR の記録が 0 件。
///
/// # 非空虚性
/// 前後とも関所（[`GpuRig::flush_to_shiori`]）を通してから列を読むので、呼び出しが kanade へ送った
/// イベントは後の印より前に列へ載る（届く前に読んで見逃さない）。ゴーストへイベントを送ると列が
/// 伸びて赤。判断の失敗の記録を `error!` にすると件数で赤。呼び出しは全部上限のうちに答えが届いたことと、
/// 前の列に `OnBoot` が載っている（観測口が生きている）ことも確かめる。
#[test]
fn calls_do_not_disturb_the_ghost() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 96);
    let shown = gpu.frames_until(100, |world| is_shown(world, 0, SHOWN));
    let first = gpu.flush_to_shiori();
    let before = gpu.calls();
    let (answered, levels) = count_levels(|| {
        [
            (None, None),
            (None, Some(i64::from(OTHER))),
            (None, Some(i64::from(KERO))),
            (None, Some(ALIAS_ONLY)),
            (Some(1), None),
            (Some(1), Some(i64::from(KERO_FACE))),
            (Some(2), None),
            (Some(-1), None),
        ]
        .map(|(scope, surface)| gpu.dump_surface(scope, surface).wait_answer().is_some())
    });
    let second = gpu.flush_to_shiori();
    let after = gpu.calls();
    let down = gpu.shutdown();

    let booted = before
        .iter()
        .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnBoot"));
    let mut expected = before.clone();
    expected.push(RecordedCall::Notify {
        id: FLUSH_EVENT.to_owned(),
        references: Vec::new(),
    });
    let sent = Some(RaiseOutcome::NoReply);
    assert_eq!(
        (
            shown,
            first,
            second,
            down,
            booted,
            answered,
            levels.error,
            after
        ),
        (true, sent, sent, true, true, [true; 8], 0, expected),
        "（台本の surface が出た, 前の関所, 後の関所, 降ろせた, 前の列に OnBoot, 答えが届いた, \
         ERROR の件数, 後の呼び出しの列）"
    );
}

/// 符号化のスレッドの記録（spec: areka-P0-mcp-dump-images-residue・要件 5.1）: 台本の surface が
/// 出た後、本物の `answer` が返した写しまで済んだ仕事を、記録を数える中で符号化のスレッドの体
/// （`finish`）に通すと、ERROR が 0 件で画像つきの成功。
///
/// # 非空虚性
/// 種が写しまで済んだ成功（`Step::Encode`）であることを確かめる（その場の失敗なら赤）。判定の関数
/// `is_picture` と `finish` の ERROR の数え方は `dump_surface_tests.rs` で較正済み。仕事の中で
/// `fail` を呼ぶと件数と判定の両方で赤。
#[test]
fn surface_job_runs_on_the_encoding_body_without_error() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 96);
    let shown = gpu.frames_until(100, |world| is_shown(world, 0, SHOWN));
    let args = Args {
        scope: None,
        surface: None,
        ghost_name: None,
    };
    let step = super::answer(&gpu.rig.world, &args);
    let down = gpu.shutdown();

    let Some(Step::Encode(scope, job)) = step else {
        panic!("台本の surface が出た後の今の見た目は、写しまで済んだ成功の種");
    };
    let (answered, levels) = count_levels(|| finish(super::TOOL, scope, job, Duration::ZERO));
    assert_eq!(
        (shown, down, scope, is_picture(&answered), levels.error),
        (true, true, 0, true, 0),
        "（台本の surface が出た, 降ろせた, スコープ, 画像つきの成功, ERROR の件数）"
    );
}
