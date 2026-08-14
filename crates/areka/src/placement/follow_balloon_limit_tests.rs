use bevy_ecs::prelude::*;
use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;
use wintf::ecs::{Point, SizeI, WindowPos, WindowStyle};

use super::super::test_support::{LogEvent, capture_logs, expect_one};
use super::test_support::{CLAMP_TAG as GUARD_CLAMP_TAG, fake_handle, point_of, rect};
use super::{
    BalloonFollowTrigger, MonitorSnapshot, PlacementRoute, enqueue_window_set_pos, follow_balloon,
    move_window_to, resize_window_keep_position,
};
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{BalloonLimit, GhostWindows, spawn_ghost_windows};

// -------------------------------------------------------------------------
// runtime 関門の wiring 檻（task 3.3・design「C7: runtime 関門」・
// Testing Strategy Integration 1・要件 2.1/2.2/2.6/2.7/2.8/2.9/5.5/6.1/6.3）
//
// 何を固定するか: 純関数のクランプ式（`balloon_limit_tests.rs` が全網羅）ではなく、
// **単一ライター `enqueue_window_set_pos` の内側で実際に走るか・誰に対して走るか・
// 何を入力にしているか**である。
//
// 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
//   (1) 探針の自己検査——補正前の提案位置が本当に作業領域からはみ出していること
//       （はみ出していなければ `limit_correction` は `None` を返し、緑が何も
//       意味しなくなる）
//   (2) **キャラ窓は動かない**こと——バルーン側の成果とキャラ窓側の挙動を
//       読み違えない（2.8 は構造保証だが、位置 assert でも二重に固定する）
//   (3) 判定語のリテラルを檻側にも持つ（本体定数と独立に固定する既存規約）
//   (4) 縮退（解決不能）では**書込が通る**ことまで見る——「補正しない」だけでは
//       素通しの証明にならない（要件 6.3 の log-first と対）
//
// World は実 spawn（`spawn_ghost_windows`）で組む。`BalloonLimit` の付与も
// `BalloonWindowMarker` も `GhostWindows` も本番の構築経路そのものであり、
// 手で組んだ World では「spawn が付け忘れても緑」になってしまうため。
// -------------------------------------------------------------------------

/// 補正ログの判定語（本体定数とは独立に檻側へ literal で置く）。
const LIMIT_CLAMP_TAG: &str = "[balloon-limit] Clamp";
/// 解決不能（縮退）の判定語。
const LIMIT_UNRESOLVED_TAG: &str = "[balloon-limit] Unresolved";
/// 2 段 grep 第 1 段の接頭辞（「関門が何かを言った」ことの一括検出）。
const LIMIT_TAG_PREFIX: &str = "[balloon-limit]";
/// 3 関門を弁別する契機ラベル（起動時関門の `boot-gate` と対）。
const RUNTIME_CONTEXT: &str = "runtime-gate";
/// 窓移動レコードの判定語（`placement::diag` の手順書語彙）。
const WINDOW_MOVE_TAG: &str = "[diag.window_move]";

/// 単一モニタの作業領域（全 4 辺が 96 の非倍数・原点非 (0,0)＝隠れた再スケール檻）。
fn wa() -> RectPx {
    rect(53, 37, 1877, 1043)
}

/// 左隣モニタ（負座標）。5.5「基準はキャラ窓の帰属モニタ」の弁別に使う。
fn left_wa() -> RectPx {
    rect(-1920, -40, 0, 1000)
}

fn single_monitor() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![wa()],
    }
}

fn balloon_size() -> SizePx {
    SizePx { w: 223, h: 158 }
}

/// 作業領域内へ収めたときの位置上限（右下側）。檻側の独立計算（実装を呼ばない）。
fn max_pos(area: RectPx, size: SizePx) -> PointPx {
    PointPx {
        x: area.right - size.w,
        y: area.bottom - size.h,
    }
}

/// 4 辺内包しているか（檻側の独立実装）。
fn contained(pos: PointPx, size: SizePx, area: RectPx) -> bool {
    pos.x >= area.left
        && pos.y >= area.top
        && pos.x + size.w <= area.right
        && pos.y + size.h <= area.bottom
}

/// 基準となる 1 scope ぶんの配置（恒等式 `balloon_offset ≡ balloon_pos − char_pos` 成立）。
///
/// - キャラ窓 (1301,411) 434×600 → 矩形は作業領域に完全内包・中心も内包（帰属が最近傍
///   フォールバックへ落ちない＝基準モニタの解決が正常腕で決まる）
/// - バルーン (1601,811) 223×158 → 同じく内包（＝**書込前は補正不要**の状態から始める）
fn base_placement(balloon_limit: bool) -> ScopePlacement {
    ScopePlacement {
        scope: 0,
        char_pos: PointPx { x: 1301, y: 411 },
        char_size: SizePx { w: 434, h: 600 },
        balloon_pos: PointPx { x: 1601, y: 811 },
        balloon_size: balloon_size(),
        balloon_offset: PointPx { x: 300, y: 400 },
        balloon_limit,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 実 spawn で World を組み、偽 `WindowHandle` を付けて単一ライターを通れるようにする。
fn world_with(placement: ScopePlacement, snapshot: MonitorSnapshot) -> (World, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(snapshot);
    let titles = GhostTitles::from_scope_titles([(0, "むらさき".to_string())]);
    let ghost_windows = spawn_ghost_windows(&mut world, &[placement], &titles);
    let char_window = ghost_windows.char_window(0).expect("キャラ窓が spawn される");
    let balloon = ghost_windows
        .balloon_window(0)
        .expect("バルーン窓が spawn される");
    world.entity_mut(char_window).insert(fake_handle(0x1000));
    world.entity_mut(balloon).insert(fake_handle(0x2000));
    (world, char_window, balloon)
}

fn base_world(balloon_limit: bool) -> (World, Entity, Entity) {
    world_with(base_placement(balloon_limit), single_monitor())
}

/// 作業領域の右下からはみ出す提案位置（両軸ともはみ出す＝4 辺のうち 2 辺を同時に検査）。
fn out_of_area() -> PointPx {
    PointPx { x: 1800, y: 1100 }
}

/// [`out_of_area`] を作業領域内へ収めた位置（檻側の独立計算）。
fn out_of_area_corrected() -> PointPx {
    max_pos(wa(), balloon_size())
}

fn assert_no_event(events: &[LogEvent], needle: &str) {
    let hits: Vec<&str> = events
        .iter()
        .map(LogEvent::message)
        .filter(|m| m.contains(needle))
        .collect();
    assert!(
        hits.is_empty(),
        "`{needle}` を含むログが出ている（出てはならない）: {hits:?}"
    );
}

/// 探針の自己検査: 提案位置が本当にはみ出していて、補正後は本当に収まっていること。
///
/// これが崩れると `limit_correction` が `None` を返し、以降の全檻が
/// 「補正が起きなかったこと」を「補正が正しかったこと」と取り違える。
#[test]
fn probe_self_check_out_of_area_is_really_outside_and_corrected_is_inside() {
    assert!(
        !contained(out_of_area(), balloon_size(), wa()),
        "探針が作業領域内に収まっている（檻が空虚になる）"
    );
    assert!(
        contained(out_of_area_corrected(), balloon_size(), wa()),
        "補正後の期待値が作業領域内に収まっていない（期待値の算出が誤り）"
    );
    assert_ne!(
        out_of_area(),
        out_of_area_corrected(),
        "補正前後が同値（クランプが観測できない）"
    );
}

// -------------------------------------------------------------------------
// (a) 発動条件はデータ駆動（2.7/2.8・DD1）
// -------------------------------------------------------------------------

/// `BalloonLimit(true)` の窓への書込は作業領域内へ補正され、scope・補正前後・契機が
/// 記録される（2.1/2.2/6.1）。
#[test]
fn enqueue_clamps_the_write_when_the_target_carries_balloon_limit_true() {
    let (mut world, _char_window, balloon) = base_world(true);
    let proposed = out_of_area();

    let (ok, events) = capture_logs(|| {
        enqueue_window_set_pos(
            &mut world,
            balloon,
            proposed.x,
            proposed.y,
            None,
            Some(PlacementRoute::BalloonFollow),
        )
    });

    assert!(ok, "補正しても書込自体は成功する");
    assert_eq!(
        point_of(&world, balloon),
        out_of_area_corrected(),
        "limit 有効なバルーンの書込位置が作業領域内へ補正されていない"
    );

    let clamp = expect_one(&events, LIMIT_CLAMP_TAG);
    assert_eq!(clamp.level, tracing::Level::INFO, "補正の記録は info 水準");
    assert_eq!(clamp.field("scope"), "0", "補正ログに scope が載っていない");
    assert_eq!(
        clamp.field("context"),
        format!("{RUNTIME_CONTEXT:?}"),
        "3 関門を弁別する契機ラベルが runtime 関門のものでない"
    );
    assert_eq!(
        clamp.field("route"),
        format!("{:?}", Some(PlacementRoute::BalloonFollow)),
        "補正ログの契機 route が元書込のものでない（DD7）"
    );
    assert_eq!(clamp.field("from"), format!("{proposed:?}"));
    assert_eq!(clamp.field("to"), format!("{:?}", out_of_area_corrected()));
}

/// `BalloonLimit(false)` の scope は補正しない（2.7＝現行挙動の維持）。
#[test]
fn enqueue_passes_through_when_balloon_limit_is_disabled() {
    let (mut world, _char_window, balloon) = base_world(false);
    let proposed = out_of_area();

    let (ok, events) = capture_logs(|| {
        enqueue_window_set_pos(
            &mut world,
            balloon,
            proposed.x,
            proposed.y,
            None,
            Some(PlacementRoute::BalloonFollow),
        )
    });

    assert!(ok);
    // 先に捕捉が成立していることを確かめてから「無いこと」を主張する。
    expect_one(&events, WINDOW_MOVE_TAG);
    assert_no_event(&events, LIMIT_TAG_PREFIX);
    assert_eq!(
        point_of(&world, balloon),
        proposed,
        "limit=0 のバルーンが補正されている（現行挙動が変わっている）"
    );
}

/// `BalloonLimit` を持たない窓（キャラ窓・placement 外の窓）は素通し（2.8）。
#[test]
fn enqueue_passes_through_for_windows_without_the_balloon_limit_component() {
    let (mut world, char_window, balloon) = base_world(true);
    // 前提の構造保証: キャラ窓には spawn が limit を焼き込まない。
    assert!(
        world.get::<BalloonLimit>(char_window).is_none(),
        "キャラ窓に BalloonLimit が付いている（2.8 の構造保証が崩れている）"
    );
    assert!(
        world.get::<BalloonLimit>(balloon).is_some(),
        "バルーン窓に BalloonLimit が付いていない（前提が崩れている）"
    );
    let proposed = out_of_area();

    let (ok, events) = capture_logs(|| {
        enqueue_window_set_pos(
            &mut world,
            char_window,
            proposed.x,
            proposed.y,
            None,
            Some(PlacementRoute::Restore),
        )
    });

    assert!(ok);
    expect_one(&events, WINDOW_MOVE_TAG);
    assert_no_event(&events, LIMIT_TAG_PREFIX);
    assert_eq!(
        point_of(&world, char_window),
        proposed,
        "キャラ窓の書込位置が limit で補正されている（2.8 違反）"
    );
}

// -------------------------------------------------------------------------
// (b) 各書込経路での補正（2.2 経路③④⑤⑦）
// -------------------------------------------------------------------------

/// 経路③: `\![move]`／連鎖確定の随伴——キャラ窓移動に随伴するバルーン書込が補正され、
/// **キャラ窓自身は 1 px も動かない**（2.8）。
#[test]
fn move_window_to_clamps_the_trailing_balloon_but_never_the_char_window() {
    let (mut world, char_window, balloon) = base_world(true);
    let char_target = PointPx { x: 1500, y: 700 };

    let (ok, events) = capture_logs(|| {
        move_window_to(&mut world, char_window, char_target.x, char_target.y)
    });

    assert!(ok);
    assert_eq!(
        point_of(&world, char_window),
        char_target,
        "limit の補正がキャラ窓の位置を動かした（2.8 違反）"
    );
    // 随伴の提案位置＝キャラ確定位置＋offset（檻側で独立に組む）。
    let proposed = PointPx {
        x: char_target.x + 300,
        y: char_target.y + 400,
    };
    assert!(
        !contained(proposed, balloon_size(), wa()),
        "探針: 随伴の提案位置がはみ出していない（檻が空虚）"
    );
    assert_eq!(point_of(&world, balloon), out_of_area_corrected());
    let clamp = expect_one(&events, LIMIT_CLAMP_TAG);
    assert_eq!(clamp.field("from"), format!("{proposed:?}"));
}

/// 経路④⑤: `follow_balloon` の随伴——**引き金によらず**同一に補正される。
///
/// ドラッグ随伴（可視性ガード適用外）と配置系随伴（ガード適用）の双方で最終位置が
/// 一致することが、「limit の補正が引き金・route に依存しない常時不変量である」
/// （2.1）ことの檻である。
#[test]
fn follow_balloon_clamps_for_every_trigger() {
    let char_target = Point { x: 1500, y: 700 };
    for trigger in [
        BalloonFollowTrigger::Drag,
        BalloonFollowTrigger::Placement(PlacementRoute::Resnap),
        BalloonFollowTrigger::Placement(PlacementRoute::MoveCue),
    ] {
        let (mut world, char_window, balloon) = base_world(true);
        follow_balloon(&mut world, char_window, char_target, trigger);
        assert_eq!(
            point_of(&world, balloon),
            out_of_area_corrected(),
            "引き金 {trigger:?} の随伴で補正されていない"
        );
    }
}

/// 経路⑦: 位置据え置きのリサイズ——**位置は同じまま寸だけが変わる**書込でも、
/// 新しい寸が作る矩形ではみ出すなら補正される（判定対象は位置と寸が作る矩形）。
#[test]
fn resize_window_keep_position_clamps_when_the_new_size_overflows_the_work_area() {
    let (mut world, _char_window, balloon) = base_world(true);
    let before = point_of(&world, balloon);
    let new_size = SizePx { w: 400, h: 300 };
    // 探針: 位置は動かさないのに、新寸だと右辺・下辺の両方がはみ出す。
    assert!(
        contained(before, balloon_size(), wa()),
        "探針: リサイズ前は収まっている前提"
    );
    assert!(
        !contained(before, new_size, wa()),
        "探針: 新寸でもはみ出さない（檻が空虚）"
    );

    let (ok, events) = capture_logs(|| resize_window_keep_position(&mut world, balloon, new_size));

    assert!(ok);
    assert_eq!(
        point_of(&world, balloon),
        max_pos(wa(), new_size),
        "寸法変更だけの書込が補正されていない（経路⑦が素通り）"
    );
    // 2.9: 位置だけを補正し、寸はそのまま書かれる。
    assert_eq!(
        world
            .get::<WindowPos>(balloon)
            .and_then(|wp| wp.size)
            .expect("size があるはず"),
        SizeI::new(new_size.w, new_size.h),
        "関門が寸を書き換えた（位置のみ補正の契約違反）"
    );
    let clamp = expect_one(&events, LIMIT_CLAMP_TAG);
    assert_eq!(clamp.field("from"), format!("{before:?}"));
}

// -------------------------------------------------------------------------
// (e) 可視性ガードとの順序（DD5・limit が最後の語）
// -------------------------------------------------------------------------

/// 可視性ガード（先）→ limit 補正（後）の順序が固定であること。
///
/// 決め手は Y である——ガードは **Y を一切変えない**契約なので、最終位置の Y が
/// 作業領域内へ収まっていれば、ガードの後段で limit が働いたことになる。さらに
/// 補正ログの `from` がガードの `clamped`（＝ガード出力）と一致することで、
/// 「関門の入力＝ガードの出力」という順序そのものを固定する。
#[test]
fn visibility_guard_runs_first_and_the_limit_correction_has_the_last_word() {
    let (mut world, char_window, balloon) = base_world(true);
    let char_target = Point { x: 1500, y: 700 };
    let proposed = out_of_area();

    let (_, events) = capture_logs(|| {
        follow_balloon(
            &mut world,
            char_window,
            char_target,
            // ガードが発火する引き金（非ドラッグの配置系）。
            BalloonFollowTrigger::Placement(PlacementRoute::Resnap),
        )
    });

    // ガードが実際に走り、素の提案位置を見て X を引き戻したこと。
    let guard = expect_one(&events, GUARD_CLAMP_TAG);
    assert_eq!(
        guard.field("proposed"),
        format!("{proposed:?}"),
        "ガードが見た位置が素の提案位置でない（関門がガードより前に動いている）"
    );
    let guard_output = PointPx {
        x: out_of_area_corrected().x,
        y: proposed.y,
    };
    assert_eq!(
        guard.field("clamped"),
        format!("{guard_output:?}"),
        "ガードの出力が想定と違う（檻の前提が崩れている）"
    );

    // 関門の入力がガードの出力そのものであること＝順序の固定。
    let clamp = expect_one(&events, LIMIT_CLAMP_TAG);
    assert_eq!(clamp.field("from"), format!("{guard_output:?}"));
    assert_eq!(clamp.field("to"), format!("{:?}", out_of_area_corrected()));

    // 最終位置は limit のもの（ガードが触れない Y が収まっている）。
    assert_eq!(point_of(&world, balloon), out_of_area_corrected());
    assert!(contained(point_of(&world, balloon), balloon_size(), wa()));
}

// -------------------------------------------------------------------------
// (f) 可視性状態を入力に持たない（2.6）
// -------------------------------------------------------------------------

/// 非表示のバルーン窓でも可視のときと **bit 同一**に補正されること。
///
/// バルーンの可視性は表示層（`emo2_boot::balloon_visibility`）の所有で placement の
/// World には無いが、World 上で可視性にもっとも近い表現である `WindowStyle` の
/// `WS_VISIBLE` を落とす／`WindowStyle` ごと外す、の 2 変種でも結果が変わらないことを
/// 固定する。関門が可視性らしき状態で分岐し始めたらここが割れる。
#[test]
fn runtime_gate_result_is_identical_for_hidden_and_visible_balloons() {
    let proposed = out_of_area();
    let mut results = Vec::new();
    for variant in ["visible", "hidden", "no-style"] {
        let (mut world, _char_window, balloon) = base_world(true);
        match variant {
            "hidden" => {
                let mut style = *world
                    .get::<WindowStyle>(balloon)
                    .expect("spawn が WindowStyle を付ける");
                assert!(
                    (style.style & WS_VISIBLE) == WS_VISIBLE,
                    "探針: 既定の spawn が WS_VISIBLE を持たない（変種が対照になっていない）"
                );
                style.style = style.style & !WS_VISIBLE;
                world.entity_mut(balloon).insert(style);
            }
            "no-style" => {
                world.entity_mut(balloon).remove::<WindowStyle>();
            }
            _ => {}
        }
        let (_, events) = capture_logs(|| {
            enqueue_window_set_pos(
                &mut world,
                balloon,
                proposed.x,
                proposed.y,
                None,
                Some(PlacementRoute::BalloonFollow),
            )
        });
        let clamp = expect_one(&events, LIMIT_CLAMP_TAG);
        results.push((
            variant,
            point_of(&world, balloon),
            clamp.field("from").to_string(),
            clamp.field("to").to_string(),
        ));
    }
    let (_, first_pos, first_from, first_to) = results[0].clone();
    for (variant, pos, from, to) in &results {
        assert_eq!(
            *pos, first_pos,
            "変種 {variant} で最終位置が変わった（関門が可視性状態を見ている）"
        );
        assert_eq!(*from, first_from, "変種 {variant} で補正前の記録が変わった");
        assert_eq!(*to, first_to, "変種 {variant} で補正後の記録が変わった");
    }
    assert_eq!(first_pos, out_of_area_corrected());
}

// -------------------------------------------------------------------------
// (5.5) 基準はキャラ窓の帰属モニタ
// -------------------------------------------------------------------------

/// 複数モニタで、キャラ窓が属する側の作業領域が基準になること（5.5）。
///
/// バルーンの提案位置は**右モニタの中**に中心を持つ一方、キャラ窓は左モニタに居る。
/// 左右で補正結果が完全に食い違う配置にしてあるので、どちらのモニタを基準にしたかが
/// 位置 1 つで判別できる。
#[test]
fn the_reference_work_area_is_the_one_the_char_window_belongs_to() {
    let mut placement = base_placement(true);
    placement.char_pos = PointPx { x: -800, y: 300 };
    placement.balloon_pos = PointPx { x: -500, y: 700 };
    placement.balloon_offset = PointPx { x: 300, y: 400 };
    let snapshot = MonitorSnapshot {
        work_areas: vec![left_wa(), wa()],
    };
    let (mut world, _char_window, balloon) = world_with(placement, snapshot);
    let proposed = PointPx { x: 100, y: 950 };

    // 探針: 提案位置の中心は**右**モニタに属する（基準を取り違えれば結果が変わる配置）。
    let center = PointPx {
        x: proposed.x + balloon_size().w / 2,
        y: proposed.y + balloon_size().h / 2,
    };
    let right = wa();
    assert!(
        center.x >= right.left
            && center.x < right.right
            && center.y >= right.top
            && center.y < right.bottom,
        "探針: 提案位置の中心が右モニタに属していない（檻が弁別力を失う）"
    );

    enqueue_window_set_pos(
        &mut world,
        balloon,
        proposed.x,
        proposed.y,
        None,
        Some(PlacementRoute::BalloonFollow),
    );

    assert_eq!(
        point_of(&world, balloon),
        max_pos(left_wa(), balloon_size()),
        "キャラ窓が属する左モニタではなくバルーン側のモニタを基準にしている（5.5 違反）"
    );
}

// -------------------------------------------------------------------------
// (c) 縮退（解決不能）——warn を残して素通し・書込は阻害しない（6.3）
// -------------------------------------------------------------------------

/// 基準が解決できない 4 経路すべてで、警告のうえ**書込が通る**こと。
///
/// 「補正しない」だけでは素通しの証明にならない——位置が実際に書かれ、戻り値が
/// `true` であることまで見る（要件 2.2 の書込を関門が落とさない）。
#[test]
fn unresolvable_reference_warns_and_lets_the_write_through() {
    let proposed = out_of_area();
    let cases: [(&str, fn(&mut World, Entity, Entity)); 4] = [
        ("MonitorSnapshot 不在", |world, _c, _b| {
            let _ = world.remove_resource::<MonitorSnapshot>();
        }),
        ("モニタ 0 台", |world, _c, _b| {
            world.insert_resource(MonitorSnapshot {
                work_areas: Vec::new(),
            });
        }),
        ("GhostWindows 不在", |world, _c, _b| {
            let _ = world.remove_resource::<GhostWindows>();
        }),
        ("キャラ窓の矩形が未確定", |world, char_window, _b| {
            world.entity_mut(char_window).remove::<WindowPos>();
        }),
    ];

    for (label, break_it) in cases {
        let (mut world, char_window, balloon) = base_world(true);
        break_it(&mut world, char_window, balloon);

        let (ok, events) = capture_logs(|| {
            enqueue_window_set_pos(
                &mut world,
                balloon,
                proposed.x,
                proposed.y,
                None,
                Some(PlacementRoute::BalloonFollow),
            )
        });

        let warn = expect_one(&events, LIMIT_UNRESOLVED_TAG);
        assert_eq!(warn.level, tracing::Level::WARN, "{label}: 縮退は warn 水準");
        assert_eq!(
            warn.field("context"),
            format!("{RUNTIME_CONTEXT:?}"),
            "{label}: 契機ラベルが runtime 関門のものでない"
        );
        assert_no_event(&events, LIMIT_CLAMP_TAG);
        assert!(ok, "{label}: 補正できないことを理由に書込が落ちている");
        assert_eq!(
            point_of(&world, balloon),
            proposed,
            "{label}: 素通しのはずが位置が変わっている"
        );
    }
}

/// 対象バルーンの現寸が未確定（窓生成直後）でも、警告のうえ書込が通ること。
///
/// `size=None`（移動専用の書込）は対象の現 `WindowPos.size` を判定寸に使うため、
/// それが未確定だと矩形を組めない。ここだけは寸の**明示指定**があれば解決するので、
/// 同じ World で `Some(寸)` を渡すと補正が復活することも併せて固定する。
#[test]
fn undetermined_balloon_size_warns_but_an_explicit_size_still_clamps() {
    let proposed = out_of_area();
    let (mut world, _char_window, balloon) = base_world(true);
    world.entity_mut(balloon).insert(WindowPos {
        position: Some(Point { x: 1601, y: 811 }),
        size: None,
        ..Default::default()
    });

    let (ok, events) = capture_logs(|| {
        enqueue_window_set_pos(
            &mut world,
            balloon,
            proposed.x,
            proposed.y,
            None,
            Some(PlacementRoute::BalloonFollow),
        )
    });
    expect_one(&events, LIMIT_UNRESOLVED_TAG);
    assert_no_event(&events, LIMIT_CLAMP_TAG);
    assert!(ok);
    assert_eq!(point_of(&world, balloon), proposed);

    // 明示寸があれば現寸を読む必要がない＝同じ World でも補正が成立する。
    let (ok, events) = capture_logs(|| {
        enqueue_window_set_pos(
            &mut world,
            balloon,
            proposed.x,
            proposed.y,
            Some(balloon_size()),
            Some(PlacementRoute::KeepPositionResize),
        )
    });
    expect_one(&events, LIMIT_CLAMP_TAG);
    assert!(ok);
    assert_eq!(point_of(&world, balloon), out_of_area_corrected());
}

// -------------------------------------------------------------------------
// (g) 位置ログは補正後の最終位置・route は元書込のもの（DD7・6.1）
// -------------------------------------------------------------------------

/// `[diag.window_move]` レコードが**補正後**の位置を記録し、route は元書込のままで
/// あること（補正は同一書込の一部であって別書込ではない）。
#[test]
fn the_window_move_record_carries_the_corrected_position_and_the_original_route() {
    let (mut world, _char_window, balloon) = base_world(true);
    let proposed = out_of_area();
    let corrected = out_of_area_corrected();

    let (_, events) = capture_logs(|| {
        enqueue_window_set_pos(
            &mut world,
            balloon,
            proposed.x,
            proposed.y,
            None,
            Some(PlacementRoute::BalloonFollow),
        )
    });

    let record = expect_one(&events, WINDOW_MOVE_TAG);
    let line = record.message();
    assert!(
        line.contains(&format!("x={} y={}", corrected.x, corrected.y)),
        "窓移動レコードが補正後の最終位置を記録していない: {line}"
    );
    assert!(
        !line.contains(&format!("x={} y={}", proposed.x, proposed.y)),
        "窓移動レコードに補正前の位置が残っている: {line}"
    );
    assert!(
        line.contains(&format!("route={}", PlacementRoute::BalloonFollow.as_str())),
        "route が元書込のものでない（関門が別書込を名乗っている）: {line}"
    );
}

/// 収まっている書込では**何も起きない**（無補正が観測できること・6.1 の「実際に
/// 動かしたとき」に限る規約）。
#[test]
fn a_write_that_already_fits_produces_no_correction_and_no_log() {
    let (mut world, _char_window, balloon) = base_world(true);
    let inside = PointPx { x: 900, y: 500 };
    assert!(
        contained(inside, balloon_size(), wa()),
        "探針: 検査対象の位置が収まっていない"
    );

    let (ok, events) = capture_logs(|| {
        enqueue_window_set_pos(
            &mut world,
            balloon,
            inside.x,
            inside.y,
            None,
            Some(PlacementRoute::BalloonFollow),
        )
    });

    assert!(ok);
    expect_one(&events, WINDOW_MOVE_TAG);
    assert_no_event(&events, LIMIT_TAG_PREFIX);
    assert_eq!(point_of(&world, balloon), inside);
}
