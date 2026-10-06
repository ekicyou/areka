//! 隠れているバルーンへの指令はコマを預かるだけにする（spec: areka-P0-animated-image-playback
//! task 4.2・要件 1.8・1.13・2.3・3.3・3.7・6.2・6.4・6.5）。
//!
//! 面 0 は 8×4 の全透明の枠に 2 つの動く絵（回数つき `fin.png`・終わりなし `end.png`）を横に
//! 並べたもの、面 1 は静止画 1 枚の 8×4。絵は `MemoryDecoder` へ登録して本物の焼き・分解・合成を通す。合成の回数は
//! 表示成立点の `info!`（`cache_hit=false` が合成した回）を数える。

use super::*;

use std::num::NonZeroU32;
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AnimatedImage, AnimationFrame, AnimationInfo, AnimationLimits, DecodedImage,
    LoopCount, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake_with_limits,
};
use areka_emo_compose::FilmSheet;
use areka_parsers::shell::{Collision, CollisionName, Element, ElementPath, Surface};

use super::test_support::{
    CapturedEvent, build_target_assets, capture, make_world_with_gpu, mount_entities,
    native_golden_with, shell_of, spawn_window_with_dpi,
};

const BASE: &str = "balloon";
const INFO_LINE_MESSAGE: &str = "apply(ShowSurface): 表示・マスクを更新";
const T: TargetId = TargetId(7);

fn element(layer: u32, path: &str, x: i64) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x,
        y: 0,
    }
}

/// 4×4 の不透明な単色。
fn solid(c: u8) -> DecodedImage {
    DecodedImage {
        width: 4,
        height: 4,
        stride: 16,
        bgra: [c, c / 2, 255 - c, 255].repeat(16),
        has_alpha: true,
    }
}

/// 4×4 の不透明な単色で、内側の 2×2（(1,1)〜(2,2)）だけが透明。
fn holed(c: u8) -> DecodedImage {
    let mut img = solid(c);
    for (x, y) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
        let at = (y * 16 + x * 4) as usize;
        img.bgra[at..at + 4].copy_from_slice(&[0, 0, 0, 0]);
    }
    img
}

fn film(dec: &mut MemoryDecoder, rel: &str, frames: Vec<DecodedImage>, laps: Option<u32>) {
    let loop_count = match laps {
        None => LoopCount::Infinite,
        Some(n) => LoopCount::Finite(NonZeroU32::new(n).unwrap()),
    };
    let first = frames[0].clone();
    let frames: Vec<AnimationFrame> = frames
        .into_iter()
        .map(|image| AnimationFrame {
            image,
            delay_ms: 100,
        })
        .collect();
    dec.insert_animated(
        Path::new(BASE).join(rel),
        AnimationInfo {
            width: 4,
            height: 4,
            frame_count: frames.len() as u32,
        },
        Ok(first),
        Ok(AnimatedImage { frames, loop_count }),
    );
}

/// 面 0（回数つき＋終わりなし・当たり判定 1 つ）と面 1（静止画）の面の表。
fn build_balloon() -> (EmoWorld, AtlasTable) {
    let surfaces = vec![
        Surface {
            id: 0,
            targets: vec![areka_parsers::shell::AppendTarget::Single(0)],
            elements: vec![
                element(0, "frame.png", 0),
                element(1, "fin.png", 0),
                element(2, "end.png", 4),
            ],
            collisions: vec![Collision {
                index: 0,
                left: 4,
                top: 0,
                right: 8,
                bottom: 4,
                name: CollisionName::new("Body".to_string()),
            }],
            animations: Vec::new(),
        },
        Surface {
            id: 1,
            targets: vec![areka_parsers::shell::AppendTarget::Single(1)],
            elements: vec![element(0, "s1.png", 0)],
            collisions: Vec::new(),
            animations: Vec::new(),
        },
    ];
    let mut dec = MemoryDecoder::new();
    // 外形は element の原寸で決まる（置いた X は足さない）ので、8×4 の全透明の枠を敷いて横に並べる。
    dec.insert(
        Path::new(BASE).join("frame.png"),
        8,
        4,
        32,
        vec![0; 128],
        true,
    );
    film(
        &mut dec,
        "fin.png",
        vec![solid(20), solid(90), solid(160)],
        Some(1),
    );
    film(&mut dec, "end.png", vec![solid(200), holed(240)], None);
    // 8×4 の不透明な単色（色が 1 つなので 4×4 の 2 枚ぶんをそのまま並べてよい）。
    let still = solid(60).bgra.repeat(2);
    dec.insert(Path::new(BASE).join("s1.png"), 8, 4, 32, still, true);
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: Path::new(BASE),
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake_with_limits(
        &[set],
        &dec,
        PackConfig::default(),
        AnimationLimits::default(),
    );
    assert!(baked.errors.is_empty(), "{:?}", baked.errors);
    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// (回数つきの子, 終わりなしの子)。
fn sheets(world: &EmoWorld) -> (FilmSheet, FilmSheet) {
    let all: Vec<&FilmSheet> = world.film_sheets().collect();
    assert_eq!(
        all.len(),
        2,
        "前提: 面 0 の動く絵が 2 つとも子へ分解されている"
    );
    let fin = all.iter().find(|s| s.laps.is_some()).expect("回数つき");
    let end = all.iter().find(|s| s.laps.is_none()).expect("終わりなし");
    ((*fin).clone(), (*end).clone())
}

/// 回数つきのコマ `fin`・終わりなしのコマ `end` を欄に載せたコマ（`None` は載せない＝経過 0）。
fn frames(world: &EmoWorld, fin: Option<usize>, end: Option<usize>) -> PatternState {
    let (f, e) = sheets(world);
    let mut p = PatternState::default();
    if let Some(i) = fin {
        p.set_film(f.id, f.frames[i]);
    }
    if let Some(i) = end {
        p.set_film(e.id, e.frames[i]);
    }
    p
}

fn golden(surface_id: u32, binds: &BindSet, pattern: &PatternState) -> Vec<u8> {
    let (world, atlas) = build_balloon();
    native_golden_with(&world, &atlas, surface_id, binds, pattern).0
}

fn attach(presenter: &mut EmoPresenter, world: &mut World, ownership: VisibilityOwnership) {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas) = build_balloon();
    presenter
        .attach_target(world, T, window, emo_world, atlas, 96)
        .expect("attach_target");
    presenter
        .set_visibility_ownership(T, ownership)
        .expect("set_visibility_ownership");
}

/// 指令を 1 つ適用し、応答がちょうど 1 回 `Ok` で返ることを確かめる。
fn send(
    presenter: &mut EmoPresenter,
    world: &mut World,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: T,
            surface_id,
            binds,
            pattern,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.try_recv(), Ok(Some(Ok(())))),
        "応答が 1 回 Ok で返っていない"
    );
    assert!(
        matches!(rx.try_recv(), Err(areka_actor::ReplyError::Dropped)),
        "応答が 2 回以上返る（または送り手が残っている）"
    );
}

/// (表示が成立した回数, そのうち合成した回数)。
fn applies(events: &[CapturedEvent]) -> (usize, usize) {
    let infos: Vec<_> = events
        .iter()
        .filter(|e| e.message() == INFO_LINE_MESSAGE)
        .collect();
    let composed = infos
        .iter()
        .filter(|e| e.field("cache_hit").map(|v| v.to_string()) == Some("false".to_string()))
        .count();
    (infos.len(), composed)
}

/// 外から所有されるバルーンを面 0（経過 0）で確立する（見えないまま）。
fn established(world: &mut World) -> EmoPresenter {
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, world, VisibilityOwnership::External);
    send(
        &mut presenter,
        world,
        0,
        BindSet::default(),
        PatternState::default(),
    );
    assert_eq!(presenter.target_visible(T), Some(false));
    presenter
}

/// 外から所有されるバルーンを面 0（経過 0）で出す。
fn shown(world: &mut World) -> EmoPresenter {
    let mut presenter = established(world);
    presenter.show_target(world, T).expect("show_target");
    assert_eq!(presenter.target_visible(T), Some(true));
    presenter
}

fn hide(presenter: &mut EmoPresenter, world: &mut World) {
    presenter.apply(
        world,
        PresentCommand::Hide {
            target: T,
            reply: None,
        },
    );
}

/// 隠れている対象へコマだけ違う指令を 3 回送ると合成 0 回で、出した絵は最後の指令のコマ。
/// その間も `last_shown`・`read_back` は成立済みの絵を返す（要件 6.4・3.7・3.3）。
#[test]
fn hidden_frame_changes_are_held_and_the_last_one_is_shown() {
    let mut world = make_world_with_gpu();
    let mut presenter = established(&mut world);
    let (w, _) = build_balloon();
    let established_picture = golden(0, &BindSet::default(), &PatternState::default());

    let ((), events) = capture(|| {
        for p in [
            frames(&w, Some(1), None),
            frames(&w, Some(2), Some(0)),
            frames(&w, None, Some(1)),
        ] {
            send(&mut presenter, &mut world, 0, BindSet::default(), p);
        }
    });
    assert_eq!(applies(&events), (0, 0), "隠れている間に合成した");
    assert_eq!(presenter.target_visible(T), Some(false));

    let (id, picture) = presenter.last_shown(T).expect("last_shown");
    assert_eq!(id, 0);
    assert_eq!(
        picture.expect("成立済みの絵がメモに在る").bytes(),
        &established_picture[..]
    );
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        established_picture
    );

    let (result, events) = capture(|| presenter.show_target(&mut world, T));
    result.expect("show_target");
    assert_eq!(applies(&events), (1, 1), "出すときに 1 回だけ合成する");
    assert_eq!(presenter.target_visible(T), Some(true));
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &frames(&w, None, Some(1))),
        "出した絵が最後の指令のコマでない"
    );
}

/// 回数つきが止まった状態で隠し、戻しの指令なしで出すと最初の絵は経過 0。終わりなしの欄は
/// 外れない（要件 2.3・3.3）。
#[test]
fn a_stopped_finite_film_restarts_from_rest_when_shown_again() {
    let mut world = make_world_with_gpu();
    let mut presenter = shown(&mut world);
    let (w, _) = build_balloon();
    let stopped = frames(&w, Some(2), Some(1));
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        stopped.clone(),
    );
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &stopped),
        "前提: 見えている間はそのまま合成される"
    );

    hide(&mut presenter, &mut world);
    presenter.show_target(&mut world, T).expect("show_target");
    let restarted = golden(0, &BindSet::default(), &frames(&w, None, Some(1)));
    assert_ne!(restarted, golden(0, &BindSet::default(), &stopped));
    assert_eq!(presenter.read_back(T).expect("read_back"), restarted);
}

/// 見えている対象へ `show_target` を重ねても、回数つきの欄は外さない（続けて見えていれば続き・
/// 要件 2.8）。
#[test]
fn show_target_on_a_visible_target_keeps_the_finite_frame() {
    let mut world = make_world_with_gpu();
    let mut presenter = shown(&mut world);
    let (w, _) = build_balloon();
    let running = frames(&w, Some(1), None);
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        running.clone(),
    );
    presenter.show_target(&mut world, T).expect("show_target");
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &running)
    );
}

/// 本番の隠し方（`PresentCommand::Hide`＝今の面を消す）の直後でも、コマだけが違う指令は預かる
/// だけで合成 0 回。出すと最後に預かったコマ（要件 6.4・3.7）。
#[test]
fn frame_changes_right_after_the_real_hide_command_are_held() {
    let mut world = make_world_with_gpu();
    let mut presenter = shown(&mut world);
    let (w, _) = build_balloon();
    hide(&mut presenter, &mut world);
    assert_eq!(
        presenter.current_surface_id(T),
        None,
        "前提: Hide は今の面を消す"
    );

    let ((), events) = capture(|| {
        for p in [frames(&w, None, Some(1)), frames(&w, Some(1), Some(1))] {
            send(&mut presenter, &mut world, 0, BindSet::default(), p);
        }
    });
    assert_eq!(
        applies(&events),
        (0, 0),
        "Hide の直後のコマだけの指令で合成した"
    );
    assert_eq!(presenter.target_visible(T), Some(false));

    presenter.show_target(&mut world, T).expect("show_target");
    assert_eq!(presenter.current_surface_id(T), Some(0));
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &frames(&w, None, Some(1))),
        "出した絵が最後に預かったコマ（回数つきは経過 0）でない"
    );
}

/// `show_target` が成立したら預かったコマは使い切る（後の通し直しは `last_show` を使う）。
/// 通し直しが失敗したら預かったコマは残り、次の `show_target` がそれで出す。
#[test]
fn show_target_clears_the_held_frame_on_success_and_keeps_it_on_failure() {
    use crate::display::{DisplayFault, arm_display_fault, clear_display_fault};

    let mut world = make_world_with_gpu();
    let (w, _) = build_balloon();

    // 成立: 回数つきの欄つきで預け、出す（欄は外れる）。見えている対象への重ねがけは外さないので、
    // 使い切っていない古い預かりが残っていると回数つきのコマが戻ってくる。
    let mut presenter = established(&mut world);
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        frames(&w, Some(2), Some(1)),
    );
    presenter.show_target(&mut world, T).expect("show_target");
    let shown_picture = golden(0, &BindSet::default(), &frames(&w, None, Some(1)));
    assert_eq!(presenter.read_back(T).expect("read_back"), shown_picture);
    presenter
        .show_target(&mut world, T)
        .expect("show_target（重ねがけ）");
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        shown_picture,
        "成立した後も預かったコマが残っている"
    );

    // 失敗: 預けたコマの通し直しの表示の記録を 1 回だけ失敗させる。
    struct Disarm;
    impl Drop for Disarm {
        fn drop(&mut self) {
            clear_display_fault();
        }
    }
    let mut presenter = established(&mut world);
    let held = frames(&w, None, Some(1));
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        held.clone(),
    );
    {
        let _disarm = Disarm;
        arm_display_fault(DisplayFault::CreateBitmap);
        assert!(
            presenter.show_target(&mut world, T).is_err(),
            "前提: 通し直しが失敗する"
        );
    }
    assert_eq!(presenter.target_visible(T), Some(false));
    presenter
        .show_target(&mut world, T)
        .expect("show_target（やり直し）");
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &held),
        "失敗した後に預かったコマが消えた"
    );
}

/// 先送りの縁: 面の番号が違う・着せ替えが違う・見えている・シェルは今までどおり合成される。
/// 面の番号が違う指令（`Hide` の後も）・成立済みのコマへ戻った指令が通った後は、前に預かった
/// コマを使わない。
#[test]
fn commands_outside_the_deferral_conditions_compose_as_before() {
    let mut world = make_world_with_gpu();
    let (w, _) = build_balloon();

    // 面の番号が違う（預かっていたコマは捨てる）。
    let mut presenter = established(&mut world);
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        frames(&w, None, Some(1)),
    );
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            1,
            BindSet::default(),
            PatternState::default(),
        )
    });
    assert_eq!(applies(&events), (1, 1), "面の番号が違う指令が合成されない");
    presenter.show_target(&mut world, T).expect("show_target");
    assert_eq!(presenter.current_surface_id(T), Some(1));
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(1, &BindSet::default(), &PatternState::default())
    );

    // `Hide` の後に別の面を確立する指令は合成し、それより前に預かったコマを使わない。
    let mut presenter = shown(&mut world);
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        frames(&w, None, Some(1)),
    );
    hide(&mut presenter, &mut world);
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        frames(&w, Some(1), Some(1)),
    );
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            1,
            BindSet::default(),
            PatternState::default(),
        )
    });
    assert_eq!(
        applies(&events),
        (1, 1),
        "Hide の後の別の面の確立が合成されない"
    );
    assert_eq!(presenter.current_surface_id(T), Some(1));
    presenter.show_target(&mut world, T).expect("show_target");
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(1, &BindSet::default(), &PatternState::default()),
        "Hide の後に別の面を確立したのに、前に預かったコマで出した"
    );

    // 成立済みのコマへ戻った指令は今までどおり通り（引き当てが当たる）、預かったコマを捨てる。
    let mut presenter = established(&mut world);
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        frames(&w, None, Some(1)),
    );
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            0,
            BindSet::default(),
            PatternState::default(),
        )
    });
    assert_eq!(
        applies(&events),
        (1, 0),
        "成立済みのコマと同じ指令が通らない"
    );
    presenter.show_target(&mut world, T).expect("show_target");
    assert_eq!(
        presenter.read_back(T).expect("read_back"),
        golden(0, &BindSet::default(), &PatternState::default()),
        "成立済みのコマへ戻った後に、前に預かったコマで出した"
    );

    // 着せ替えが違う。
    let mut presenter = established(&mut world);
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            0,
            BindSet::from_ids([3]),
            frames(&w, None, Some(1)),
        )
    });
    assert_eq!(applies(&events), (1, 1), "着せ替えが違う指令が合成されない");

    // 見えている。
    let mut presenter = shown(&mut world);
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            0,
            BindSet::default(),
            frames(&w, None, Some(1)),
        )
    });
    assert_eq!(
        applies(&events),
        (1, 1),
        "見えている対象への指令が合成されない"
    );

    // シェル（命令で見える対象）は隠した後の指令で合成して見えるようになる。
    let mut presenter = EmoPresenter::new();
    attach(
        &mut presenter,
        &mut world,
        VisibilityOwnership::CommandDriven,
    );
    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        PatternState::default(),
    );
    hide(&mut presenter, &mut world);
    let ((), events) = capture(|| {
        send(
            &mut presenter,
            &mut world,
            0,
            BindSet::default(),
            frames(&w, None, Some(1)),
        )
    });
    assert_eq!(applies(&events), (1, 1), "シェルへの指令が合成されない");
    assert_eq!(presenter.target_visible(T), Some(true));
}

/// 見えているバルーンでコマを替えても、文字のスロットと対象の寸法は同じ（要件 6.2）。透ける形は
/// 表示中のコマに従い、作者の当たり判定の矩形は同じ（要件 1.13・1.8）。
#[test]
fn frame_changes_keep_the_slot_and_size_and_follow_the_transparency() {
    let mut world = make_world_with_gpu();
    let mut presenter = shown(&mut world);
    let (w, _) = build_balloon();
    let mask_hit = |presenter: &EmoPresenter, world: &World, x: u32, y: u32| {
        let surface = mount_entities(presenter, T).0;
        world
            .get::<AlphaMaskResource>(surface)
            .expect("AlphaMaskResource")
            .mask()
            .expect("マスク")
            .is_hit(x, y)
    };

    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        frames(&w, None, Some(0)),
    );
    let slot = presenter.text_slot_view(T).expect("text_slot_view");
    let size = presenter
        .target_physical_size(T)
        .expect("target_physical_size");
    assert!(mask_hit(&presenter, &world, 5, 1), "コマ 0 は不透明");
    assert_eq!(presenter.hit_region(T, 5, 1), Some("Body"));

    send(
        &mut presenter,
        &mut world,
        0,
        BindSet::default(),
        frames(&w, None, Some(1)),
    );
    assert_eq!(presenter.text_slot_view(T), Some(slot));
    assert_eq!(presenter.target_physical_size(T), Some(size));
    assert!(!mask_hit(&presenter, &world, 5, 1), "コマ 1 の穴が透けない");
    assert!(mask_hit(&presenter, &world, 4, 0), "穴の外は不透明のまま");
    assert_eq!(presenter.hit_region(T, 5, 1), Some("Body"));
}

/// 静止画だけのバルーンは、出す・隠す・同じ指令を何度送っても合成は最初の 1 回だけ（要件 6.5）。
#[test]
fn a_still_only_balloon_composes_once_as_before() {
    let mut world = make_world_with_gpu();
    let (emo_world, atlas, golden_bytes) = build_target_assets(8, 4, 0x33);
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    presenter
        .attach_target(&mut world, T, window, emo_world, atlas, 96)
        .expect("attach_target");
    presenter
        .set_visibility_ownership(T, VisibilityOwnership::External)
        .expect("set_visibility_ownership");

    let ((), events) = capture(|| {
        for _ in 0..2 {
            send(
                &mut presenter,
                &mut world,
                1000,
                BindSet::default(),
                PatternState::default(),
            );
        }
        presenter.show_target(&mut world, T).expect("show_target");
        hide(&mut presenter, &mut world);
        for _ in 0..3 {
            send(
                &mut presenter,
                &mut world,
                1000,
                BindSet::default(),
                PatternState::default(),
            );
        }
        presenter.show_target(&mut world, T).expect("show_target");
    });
    let (_, composed) = applies(&events);
    assert_eq!(composed, 1, "静止画のバルーンの合成の回数が変わった");
    assert_eq!(presenter.read_back(T).expect("read_back"), golden_bytes);
}
