//! 件 26（`animated-surface-log-flood` 要件 9.1・9.3）: `-1` で止まったまばたきの重ね絵が、止まった後の
//! 表示に残らないこと（表示の側の境目の判定）。
//!
//! 見本はぱすたの animation1400 と同じ形（`bind+random`・pattern0 が閉じた目 1410・pattern1 が半目
//! 1411・pattern2 が `-1`）。seriko は止まった後に 1400 の欄を外した指令を出す（seriko の側は
//! `looper_tests.rs` の `blink_ending_in_minus_one_leaves_no_frame_in_the_command_after_stop`）。
//! その指令で成立した絵が、まばたきの重ねの無い絵と同じことを判定する。
//!
//! - 動く絵の欄が残らない止まり方（指令の欄が空）は、いちからの合成（`compose_into`）の道。
//! - 同じ面で動く絵（純 `random` の animation1500）がコマを替えながら残る止まり方は、半目の絵を
//!   下敷きにした矩形の重ね直し（`compose_into_reusing`・タスク 3.3）の道。こちらは控えに外れる回と
//!   控えに当たる回の両方を見る。

use super::*;

use std::path::Path;
use std::time::Duration;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{ComposeMethod, PatternFrame};

use super::test_support::{
    capture, make_world_with_gpu, native_golden_with, pattern_overlay_at, spawn_window_with_dpi,
};

const TARGET: TargetId = TargetId(0);
const BASE_ID: u32 = 1000;
const BLINK: u32 = 1400;
const MOVER: u32 = 1500;
const RANGE_LINE_MESSAGE: &str = "apply(ShowSurface): 重ね直しの範囲";

const SHELL: &str = "surface1000\n{\nelement0,overlay,p.png,0,0\n\
animation1400.interval,bind+random,4\n\
animation1400.pattern0,overlay,1410,0,4,2\n\
animation1400.pattern1,overlay,1411,120,4,2\n\
animation1400.pattern2,overlay,-1,50,0,0\n\
animation1500.interval,random,2\n\
animation1500.pattern0,overlay,1500,0,1,8\n}\n\
surface1500\n{\nelement0,overlay,move.png,0,0\n}\n\
surface1410\n{\nelement0,overlay,closed.png,0,0\n}\n\
surface1411\n{\nelement0,overlay,half.png,0,0\n}\n";

fn build_assets() -> (EmoWorld, AtlasTable) {
    let shell = areka_parsers::shell::parse(SHELL);
    let base = Path::new("shell/master");
    let solid = |(w, h): (u32, u32), bgr: [u8; 3]| -> Vec<u8> {
        (0..w * h)
            .flat_map(|_| [bgr[0], bgr[1], bgr[2], 0xFF])
            .collect()
    };
    let mut dec = MemoryDecoder::new();
    for (name, size, bgr) in [
        ("p.png", (16, 12), [0x20, 0x40, 0x60]),
        ("closed.png", (4, 3), [0x00, 0x00, 0xF0]),
        ("half.png", (4, 3), [0x00, 0xF0, 0x00]),
        ("move.png", (3, 2), [0xF0, 0x00, 0x00]),
    ] {
        dec.insert(
            base.join(name),
            size.0,
            size.1,
            size.0 * 4,
            solid(size, bgr),
            true,
        );
    }
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "atlas bake: {:?}", baked.errors);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

fn binds() -> BindSet {
    BindSet::from_ids([BLINK])
}

fn show(presenter: &mut EmoPresenter, world: &mut World, pattern: PatternState) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: TARGET,
            surface_id: BASE_ID,
            binds: binds(),
            pattern,
            reply: Some(tx),
        },
    );
    assert!(matches!(
        rx.recv_timeout(Duration::from_secs(10)),
        Ok(Ok(()))
    ));
}

/// 閉じる → 半目 → 外す を 2 周（2 周目は控えに当たる）。外した後の絵は、まばたきの重ねの無い絵
/// （着せ替えも欄も無い同じ面）と同じ。
#[test]
fn stopped_blink_leaves_no_closed_eye_in_the_picture() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas) = build_assets();
    presenter
        .attach_target(&mut world, TARGET, window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    let (emo_world, atlas) = build_assets();
    let (open_eye, _) = native_golden_with(
        &emo_world,
        &atlas,
        BASE_ID,
        &BindSet::default(),
        &PatternState::default(),
    );

    show(&mut presenter, &mut world, PatternState::default());
    for lap in 0..2 {
        show(
            &mut presenter,
            &mut world,
            pattern_overlay_at(BLINK, 1410, 4, 2),
        );
        show(
            &mut presenter,
            &mut world,
            pattern_overlay_at(BLINK, 1411, 4, 2),
        );
        show(&mut presenter, &mut world, PatternState::default());
        assert_eq!(
            presenter.read_back(TARGET).expect("read_back"),
            open_eye,
            "{lap} 周目: `-1` で外した後の絵に閉じた目が残る"
        );
    }
}

/// 欄を並べた現在コマ集合（どれも overlay）。
fn frames(cells: &[(u32, u32, i64, i64)]) -> PatternState {
    let mut p = PatternState::default();
    for &(anim, surface_id, x, y) in cells {
        p.set(
            anim,
            PatternFrame {
                surface_id,
                method: ComposeMethod::Overlay,
                x,
                y,
            },
        );
    }
    p
}

/// 同じ面で動く絵（1500）がコマを替えながら残るあいだに、まばたきが 閉じる → 半目 → 外す と進む。
/// 外した後の指令は 1500 の欄だけを運ぶので、その絵は半目の絵を下敷きにした矩形の重ね直しで組まれる。
/// 1 周目（控えに外れる）も 2 周目（控えに当たる）も、その絵は「1500 のそのコマだけ・まばたきの重ね
/// 無し」の絵と同じ。
#[test]
fn stopped_blink_beside_a_moving_picture_leaves_no_closed_eye_on_the_rect_path() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas) = build_assets();
    presenter
        .attach_target(&mut world, TARGET, window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    let (emo_world, atlas) = build_assets();
    let stopped = frames(&[(MOVER, 1500, 11, 8)]);
    let (golden, _) =
        native_golden_with(&emo_world, &atlas, BASE_ID, &BindSet::default(), &stopped);
    assert!(
        golden.chunks(4).any(|px| px == [0xF0, 0x00, 0x00, 0xFF]),
        "見本の前提: 止まった絵に 1500 のコマが載る"
    );

    for lap in 0..2 {
        show(
            &mut presenter,
            &mut world,
            frames(&[(MOVER, 1500, 1, 8), (BLINK, 1410, 4, 2)]),
        );
        show(
            &mut presenter,
            &mut world,
            frames(&[(MOVER, 1500, 6, 8), (BLINK, 1411, 4, 2)]),
        );
        let ((), events) = capture(|| show(&mut presenter, &mut world, stopped.clone()));
        assert_eq!(
            presenter.read_back(TARGET).expect("read_back"),
            golden,
            "{lap} 周目: `-1` で外した後の絵に閉じた目が残る"
        );
        // 空の道で緑にしない: 1 周目は矩形で重ね直し、2 周目は控えに当たる（範囲の行が出ない）。
        let partial = events
            .iter()
            .filter(|e| {
                e.message() == RANGE_LINE_MESSAGE && e.field_str("recompose") == Some("partial")
            })
            .count();
        let ranges = events
            .iter()
            .filter(|e| e.message() == RANGE_LINE_MESSAGE)
            .count();
        let (want_partial, want_ranges) = if lap == 0 { (1, 1) } else { (0, 0) };
        assert_eq!(
            (partial, ranges),
            (want_partial, want_ranges),
            "{lap} 周目の止まった絵の組み方"
        );
    }
}
