use bevy_ecs::prelude::*;
use tracing::Level;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use wintf::ecs::drag::{DragConfig, DragConstraint, OnDrag, OnDragEnd};
use wintf::ecs::layout::{BoxStyle, HitTest};
use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed};
use wintf::ecs::{
    DpiSuggestedRectPolicy, KeepDirectlyAbove, Point, SizeI, Window, WindowPos, WindowStyle,
};

use super::test_support::{ghost_window_entities, titles, two_scope_placements};
use super::{
    BalloonWindowMarker, CharWindowMarker, GhostWindowMarker, GhostWindows, spawn_ghost_windows,
};
use crate::placement::diag::{ZORDER_PAIR_DECLARED_TAG, zorder_pair_declared_line};
use crate::placement::follow::{Anchored, BalloonFollow};
use crate::placement::resolver::Anchor;
use crate::placement::source::GhostTitles;
use crate::placement::test_support::capture_logs;

// -------------------------------------------------------------------------
// T-I1: spawn 組立（6.1/6.2・1.1/1.2）
// -------------------------------------------------------------------------

/// T-I1: bare World で 2 スコープぶん spawn → 窓 4 entity・markers 正値・
/// `GhostWindows` の scope×種別引き当てが `ScopePlacement` と一致する。
#[test]
fn t_i1_spawn_assembles_four_windows_with_markers_and_ghost_windows() {
    let mut world = World::new();
    let placements = two_scope_placements();

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    // 窓 4 entity（2 スコープ × キャラ/バルーン・1.1/1.2）
    assert_eq!(ghost_window_entities(&mut world).len(), 4);

    // scopes() はスコープ番号を昇順で列挙する
    assert_eq!(gw.scopes().collect::<Vec<_>>(), vec![0, 1]);

    for p in &placements {
        // GhostWindows の scope×種別引き当て（6.1/6.2）
        let char_e = gw.char_window(p.scope).expect("char window entity");
        let balloon_e = gw.balloon_window(p.scope).expect("balloon window entity");
        assert_ne!(char_e, balloon_e);

        // markers 正値（補助的な逆引き・6.2）
        assert_eq!(
            world.get::<CharWindowMarker>(char_e).map(|m| m.scope),
            Some(p.scope)
        );
        assert!(world.get::<BalloonWindowMarker>(char_e).is_none());
        assert_eq!(
            world.get::<BalloonWindowMarker>(balloon_e).map(|m| m.scope),
            Some(p.scope)
        );
        assert!(world.get::<CharWindowMarker>(balloon_e).is_none());
        assert!(world.get::<GhostWindowMarker>(char_e).is_some());
        assert!(world.get::<GhostWindowMarker>(balloon_e).is_some());

        // WindowPos が ScopePlacement と一致（物理 px 転記・U1）
        let char_pos = world.get::<WindowPos>(char_e).expect("char WindowPos");
        assert_eq!(
            char_pos.position,
            Some(Point {
                x: p.char_pos.x,
                y: p.char_pos.y
            })
        );
        assert_eq!(
            char_pos.size,
            Some(SizeI::new(p.char_size.w, p.char_size.h))
        );
        let balloon_pos = world.get::<WindowPos>(balloon_e).expect("balloon WindowPos");
        assert_eq!(
            balloon_pos.position,
            Some(Point {
                x: p.balloon_pos.x,
                y: p.balloon_pos.y
            })
        );
        assert_eq!(
            balloon_pos.size,
            Some(SizeI::new(p.balloon_size.w, p.balloon_size.h))
        );
    }

    // 未知スコープは None（panic しない）
    assert_eq!(gw.char_window(99), None);
    assert_eq!(gw.balloon_window(99), None);
}

/// T-I1 補: `GhostWindows` は Resource としても挿入され、戻り値と同じ
/// 引き当てを返す（design「Resource 挿入＋戻り値の両方で公開」）。
#[test]
fn t_i1_ghost_windows_resource_matches_return_value() {
    let mut world = World::new();
    let placements = two_scope_placements();

    let returned = spawn_ghost_windows(&mut world, &placements, &titles());

    let resource = world
        .get_resource::<GhostWindows>()
        .expect("GhostWindows Resource が挿入されているはず");
    assert_eq!(
        resource.scopes().collect::<Vec<_>>(),
        returned.scopes().collect::<Vec<_>>()
    );
    for scope in returned.scopes() {
        assert_eq!(resource.char_window(scope), returned.char_window(scope));
        assert_eq!(resource.balloon_window(scope), returned.balloon_window(scope));
    }
}


/// T-I1 補: 窓タイトルは `GhostTitles` 由来（欠落スコープは既定 "areka"）。
#[test]
fn t_i1_window_titles_come_from_ghost_titles() {
    let mut world = World::new();
    let placements = two_scope_placements();

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    let char0 = gw.char_window(0).unwrap();
    let balloon1 = gw.balloon_window(1).unwrap();
    assert_eq!(world.get::<Window>(char0).unwrap().title, "むらさき");
    assert_eq!(world.get::<Window>(balloon1).unwrap().title, "エモ");

    // タイトル欠落スコープは既定文字列（GhostTitles::title の契約）
    let mut world2 = World::new();
    let gw2 = spawn_ghost_windows(
        &mut world2,
        &placements,
        &GhostTitles::from_scope_titles([]),
    );
    let char0 = gw2.char_window(0).unwrap();
    assert_eq!(world2.get::<Window>(char0).unwrap().title, "areka");
}

/// T-I1 補: キャラ窓は `BalloonFollow`（balloon 引き当て＋offset 転写）と
/// `OnDrag`（追従）を持ち、バルーン窓は `OnDrag`（相対位置記憶・4.8/DD16）を
/// 持つが `BalloonFollow` は持たない（4.2 結線・design 同型 bullet。
/// バルーン側ハンドラの実挙動は
/// `t_i4_char_move_follows_adjusted_offset_after_balloon_solo_drag` が檻）。
#[test]
fn t_i1_char_window_has_follow_and_on_drag_balloon_has_on_drag_only() {
    let mut world = World::new();
    let placements = two_scope_placements();

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    for p in &placements {
        let char_e = gw.char_window(p.scope).unwrap();
        let balloon_e = gw.balloon_window(p.scope).unwrap();

        // BalloonFollow: 対応バルーン entity＋ScopePlacement.balloon_offset の転写
        let follow = world
            .get::<BalloonFollow>(char_e)
            .expect("char window BalloonFollow");
        assert_eq!(follow.balloon, balloon_e);
        assert_eq!(follow.offset, p.balloon_offset);
        assert!(world.get::<OnDrag>(char_e).is_some());

        // バルーン窓: 相対位置記憶ハンドラあり（4.8）・BalloonFollow なし
        assert!(world.get::<BalloonFollow>(balloon_e).is_none());
        assert!(world.get::<OnDrag>(balloon_e).is_some());
    }
}

/// T-I1 補: 全窓が `HitTest::none()`（全面ヒットで透過を殺さない）を持つ。
/// `spawn_ghost_windows` 自体はポインタハンドラを付けない（依存方向
/// input_events→placement・placement は `crate::` パスを持てないため）。
/// spawn 直後に `input_events::attach_char_pointer_handlers` を呼ぶと、キャラ窓は
/// 正規ポインタハンドラ（`OnPointerMoved`＋`OnPointerPressed`＝stand-in
/// `on_ghost_pressed` を退役して差し替え）を持ち、バルーン窓はポインタハンドラを
/// 一切持たない（M1 はバルーンにマウス送出なし・DD-IE-12）。ハンドラ実挙動の檻は
/// input_events の task 2.7 檻＝proven-wiring ゆえ存在の有無だけ固定する
/// （[[test-only-decision-branches-not-proven-wiring]]）。本テストは `#[cfg(test)]`
/// ゆえ `crate::` パス使用可（example の `#[path]` include 不変条件は非テストコード限定）。
#[test]
fn t_i1_all_windows_have_hit_test_none_and_char_has_pointer_handlers() {
    let mut world = World::new();
    let placements = two_scope_placements();

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    // spawn は crate::-free ゆえハンドラを付けない。装着は input_events が担う。
    crate::input_events::attach_char_pointer_handlers(&mut world);

    for e in ghost_window_entities(&mut world) {
        assert_eq!(world.get::<HitTest>(e).copied(), Some(HitTest::none()));
    }

    for p in &placements {
        let char_e = gw.char_window(p.scope).unwrap();
        let balloon_e = gw.balloon_window(p.scope).unwrap();

        // キャラ窓は正規の移動／押下ハンドラを両方持つ（task 3.2 差し替え）
        assert!(
            world.get::<OnPointerMoved>(char_e).is_some(),
            "scope{}: キャラ窓は OnPointerMoved を持つ",
            p.scope
        );
        assert!(
            world.get::<OnPointerPressed>(char_e).is_some(),
            "scope{}: キャラ窓は OnPointerPressed を持つ",
            p.scope
        );

        // バルーン窓はポインタハンドラを一切持たない（DD-IE-12・stand-in 撤去）
        assert!(
            world.get::<OnPointerMoved>(balloon_e).is_none(),
            "scope{}: バルーン窓に OnPointerMoved を付けない（DD-IE-12）",
            p.scope
        );
        assert!(
            world.get::<OnPointerPressed>(balloon_e).is_none(),
            "scope{}: バルーン窓に OnPointerPressed を付けない（DD-IE-12）",
            p.scope
        );
    }
}

// -------------------------------------------------------------------------
// anchor 伝搬（4.2・DD15 基盤・task 3.1）
//
// 旧 `bottom_snap_marker_attached_to_snapping_char_windows_only`（二値吸着 marker
// の有無検証）を、解決済み 5 値アンカーの entity 表現（`Anchored`）付与検証へ
// 意味を保って差し替え（marker 有無→アンカー種別付与・単一真実源＝`Anchored`・Req1.6）。
// -------------------------------------------------------------------------

/// 3.1（4.2/1.6）: 生成直後の**全 char 窓**が対応する 5 値アンカーの
/// `Anchored(anchor)` を保持する（非 Free 窓は非 Free アンカー・Free 窓は
/// `Anchored(Anchor::Free)`＝無条件付与）。バルーン窓には `Anchored` を付けない
/// （吸着／リサイズ対象はキャラ窓のみ・DD15）。
#[test]
fn anchored_attached_to_all_char_windows_with_resolved_anchor() {
    let mut world = World::new();
    let mut placements = two_scope_placements(); // 両方 Anchor::Bottom（emo2＝bottom）
    placements[1].anchor = Anchor::Free; // scope1 を非吸着（Free）へ

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    let char0 = gw.char_window(0).unwrap();
    let char1 = gw.char_window(1).unwrap();
    // 各 char 窓は自スコープの解決済みアンカーを Anchored として保持する（4.2）
    assert_eq!(
        world.get::<Anchored>(char0).copied(),
        Some(Anchored(Anchor::Bottom)),
        "非 Free スコープの char 窓は Anchored(Bottom) を保持"
    );
    assert_eq!(
        world.get::<Anchored>(char1).copied(),
        Some(Anchored(Anchor::Free)),
        "Free スコープの char 窓も無条件で Anchored(Free) を保持（resize identity 射影が読む）"
    );
    // 転写元の placement と一致すること（値の取り違え封じ）
    for p in &placements {
        assert_eq!(
            world.get::<Anchored>(gw.char_window(p.scope).unwrap()).copied(),
            Some(Anchored(p.anchor)),
            "scope{}: Anchored は ScopePlacement.anchor の転写",
            p.scope
        );
    }
    // バルーン窓には anchor 値によらず Anchored を付けない（char 窓のみ）
    for scope in [0usize, 1] {
        assert!(
            world
                .get::<Anchored>(gw.balloon_window(scope).unwrap())
                .is_none(),
            "scope{scope}: バルーン窓には Anchored を付けない"
        );
    }
}

// -------------------------------------------------------------------------
// T-I2: z-order 既定（5.1・DD13）
// -------------------------------------------------------------------------

/// T-I2: 全窓の `WindowStyle.ex_style` に `WS_EX_TOPMOST` が含まれない
/// （既定 z-order 非 topmost・5.1／DD13。style/ex_style の正値も固定する）。
#[test]
fn t_i2_no_window_has_ws_ex_topmost() {
    let mut world = World::new();
    let placements = two_scope_placements();

    spawn_ghost_windows(&mut world, &placements, &titles());

    let entities = ghost_window_entities(&mut world);
    assert_eq!(entities.len(), 4);
    for e in entities {
        let style = world.get::<WindowStyle>(e).expect("WindowStyle");
        assert!(
            !style.ex_style.contains(WS_EX_TOPMOST),
            "WS_EX_TOPMOST が含まれてはならない（5.1／DD13）: {:?}",
            style.ex_style
        );
        assert_eq!(style.ex_style, WS_EX_LAYERED | WS_EX_TOOLWINDOW);
        assert_eq!(style.style, WS_POPUP | WS_VISIBLE);
    }
}

// -------------------------------------------------------------------------
// 外部権威宣言（areka-P0-dpi-window-vanish 要件 4.3・D3・task 5.1）
//
// 付与漏れが S1（OS 提案位置の素通し）再発の穴になる。wintf 側は
// 「component 未付与＝従来どおり提案位置を適用」ゆえ、**落とした窓だけが静かに
// OS 直書きへ戻る**——落ちたことはログにも型にも現れない。よって
// 「全 scope × 窓 2 種」を檻で数え上げて固定する。
// -------------------------------------------------------------------------

/// 5.1（4.3・D3）: spawn 直後の**全**ゴースト窓（全 scope × キャラ/バルーン）が
/// `DpiSuggestedRectPolicy::ExternalAuthority` を保持する。
///
/// 空虚性回避（[[2.2 の教訓]]・tasks.md Implementation Notes 2.2/3.2/4.4/4.6）:
/// ①探針の自己検査——**既定値が `ExternalAuthority` ではない**ことを先に固定する
/// （既定が外部権威なら「付与されている」の主張は component 未付与でも成り立ち、
/// 檻が不動点に落ちる）。②**数え上げで固定する**——`ghost_window_entities` の全数
/// （2 scope × 2 種＝4）を走査し、scope×種別を名指しで再確認する（キャラ窓にだけ
/// 付ける部分是正が「4 件中 2 件」で赤になる）。③非ゴースト窓には付かないことも
/// 主張する（World 全体への無差別挿入で緑にならない）。
#[test]
fn external_authority_attached_to_every_ghost_window_of_every_scope() {
    // ① 探針の自己検査: 既定値は従来挙動側（＝未付与と外部権威が区別できる）
    assert_ne!(
        DpiSuggestedRectPolicy::default(),
        DpiSuggestedRectPolicy::ExternalAuthority,
        "既定値が ExternalAuthority なら本檻は不動点に落ちて何も検出しない"
    );

    let mut world = World::new();
    let mut placements = two_scope_placements();
    placements[1].anchor = Anchor::Free; // アンカー種別に依存しないことも同時に固定
    // 非ゴースト窓（無差別挿入の検出用の対照）
    let stranger = world.spawn(Window::default()).id();

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    // ② 全数走査（4 窓すべてに付いている）
    let entities = ghost_window_entities(&mut world);
    assert_eq!(entities.len(), 4, "2 scope × キャラ/バルーン＝4 窓のはず");
    for e in &entities {
        assert_eq!(
            world.get::<DpiSuggestedRectPolicy>(*e).copied(),
            Some(DpiSuggestedRectPolicy::ExternalAuthority),
            "ゴースト窓 {e:?} に外部権威宣言が無い（OS 提案位置が直書きされる＝S1 再発）"
        );
    }

    // ② scope×種別の名指し（部分是正が「どちらを落としたか」で赤になる）
    for p in &placements {
        assert_eq!(
            world
                .get::<DpiSuggestedRectPolicy>(gw.char_window(p.scope).unwrap())
                .copied(),
            Some(DpiSuggestedRectPolicy::ExternalAuthority),
            "scope{}: キャラ窓に外部権威宣言が無い",
            p.scope
        );
        assert_eq!(
            world
                .get::<DpiSuggestedRectPolicy>(gw.balloon_window(p.scope).unwrap())
                .copied(),
            Some(DpiSuggestedRectPolicy::ExternalAuthority),
            "scope{}: バルーン窓に外部権威宣言が無い（バルーンも OS 直書きから外す・D3。\
             落とすと balloon_pos − char_pos ≡ offset が DPI 跨ぎで崩れる）",
            p.scope
        );
    }

    // ③ 非ゴースト窓には付かない（World 全体への無差別挿入では緑にならない）
    assert!(
        world.get::<DpiSuggestedRectPolicy>(stranger).is_none(),
        "spawn_ghost_windows が自分の生成窓以外へ政策を挿入している"
    );
}

// -------------------------------------------------------------------------
// T-I3: 単位契約（U2・DD8・4.1/4.5）
// -------------------------------------------------------------------------

/// T-I3（3.1 改訂）: 窓 entity に `BoxStyle` 不在（U2・論理 DIP を持ち込まない）・
/// `DragConstraint` 不在（DD8・全モニタドラッグ可・4.5）。`DragConfig.move_window`
/// は「非 Free アンカーのキャラ窓のみ false（単一ライター・DD15 v2・4.7）、Free
/// キャラ窓とバルーン窓は true（wndproc 委譲・4.1/4.5）」。DragEnd ハンドラ
/// （`OnDragEnd`）は Free 含む**全**キャラ窓に付く（非 Free は最終再固定・Free は
/// 保存専用アーム・1.1・task 2.2）。バルーン窓にも `OnDragEnd`（`on_balloon_drag_end`）が
/// 付く（単独ドラッグ確定 offset の永続 write-through・2.1・task 2.3）。`Anchored` は
/// char 窓のみ（Free/非 Free とも無条件付与）・バルーン窓には付かない（4.2/1.6）。
#[test]
fn t_i3_no_box_style_no_drag_constraint_and_move_window_contract() {
    let mut world = World::new();
    let mut placements = two_scope_placements();
    placements[1].anchor = Anchor::Free; // scope1 を Free へ（両変種を 1 テストで檻化）

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    let entities = ghost_window_entities(&mut world);
    assert_eq!(entities.len(), 4);
    for e in entities {
        assert!(
            world.get::<BoxStyle>(e).is_none(),
            "窓 entity に BoxStyle を付けてはならない（U2）"
        );
        assert!(
            world.get::<DragConstraint>(e).is_none(),
            "窓 entity に DragConstraint を付けてはならない（DD8・4.5）"
        );
        let drag = world.get::<DragConfig>(e).expect("DragConfig");
        assert_eq!(drag.threshold, 5, "threshold は既定値を保つ");
        assert!(drag.enabled, "ドラッグは有効");
    }

    let snap_char = gw.char_window(0).unwrap();
    let free_char = gw.char_window(1).unwrap();
    assert!(
        !world.get::<DragConfig>(snap_char).unwrap().move_window,
        "非 Free アンカーのキャラ窓は move_window=false（単一ライター・DD15 v2・4.7）"
    );
    assert!(
        world.get::<OnDragEnd>(snap_char).is_some(),
        "非 Free アンカーのキャラ窓に DragEnd 最終適用ハンドラが付く（DD15 v2 (3)）"
    );
    assert!(
        world.get::<DragConfig>(free_char).unwrap().move_window,
        "Free キャラ窓は move_window=true（wndproc 委譲・4.1）"
    );
    assert!(
        world.get::<OnDragEnd>(free_char).is_some(),
        "Free キャラ窓にも OnDragEnd が付く（全アンカー結線・保存専用アーム・1.1・task 2.2）"
    );
    // Anchored は char 窓のみ（Free/非 Free とも付与）・バルーン窓には付かない（4.2/1.6）
    assert_eq!(
        world.get::<Anchored>(snap_char).copied(),
        Some(Anchored(Anchor::Bottom)),
        "非 Free キャラ窓は Anchored(Bottom) を保持"
    );
    assert_eq!(
        world.get::<Anchored>(free_char).copied(),
        Some(Anchored(Anchor::Free)),
        "Free キャラ窓も Anchored(Free) を保持"
    );
    for scope in [0usize, 1] {
        let balloon = gw.balloon_window(scope).unwrap();
        assert!(
            world.get::<DragConfig>(balloon).unwrap().move_window,
            "scope{scope}: バルーン窓は move_window=true（単独ドラッグ・4.5）"
        );
        assert!(
            world.get::<OnDragEnd>(balloon).is_some(),
            "scope{scope}: バルーン窓に OnDragEnd（on_balloon_drag_end）が付く（単独ドラッグ確定 offset 保存・task 2.3）"
        );
        assert!(
            world.get::<Anchored>(balloon).is_none(),
            "scope{scope}: バルーン窓に Anchored は付けない（char 窓のみ）"
        );
    }
}

// -------------------------------------------------------------------------
// ペア宣言（areka-P0-ghost-window-zorder 要件 1.1／6.1・design「spawn ペア宣言」）
//
// 宣言は「バルーン窓はキャラ窓のすぐ手前に居るべき」を表す永続 component で、
// 付けるのは scope を知る唯一の層＝本 spawn である（wintf は scope を知れない）。
// 付け漏れは型にも実行時エラーにも現れず、実機で「バルーンが埋もれる」形でしか
// 出てこないため、scope×窓種別を名指しで数え上げて固定する。
// -------------------------------------------------------------------------

/// 1.1: spawn 直後、**全 scope の**バルーン窓が同一スコープのキャラ窓を指す
/// `KeepDirectlyAbove` を持つ。
///
/// 対照を 3 つ置く: ①キャラ窓側には付かない（宣言は片側のみ・向きが逆転していない）
/// ②非ゴースト窓には付かない（World 全体への無差別挿入では緑にならない）
/// ③peer はスコープを跨がない（scope0 の宣言が scope1 のキャラ窓を指していない）。
#[test]
fn keep_directly_above_attached_to_every_balloon_window_pointing_at_its_char_window() {
    let mut world = World::new();
    let mut placements = two_scope_placements();
    placements[1].anchor = Anchor::Free; // アンカー種別に依存しないことも同時に固定
    // 非ゴースト窓（無差別挿入の検出用の対照）
    let stranger = world.spawn(Window::default()).id();

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    for p in &placements {
        let char_e = gw.char_window(p.scope).unwrap();
        let balloon_e = gw.balloon_window(p.scope).unwrap();

        assert_eq!(
            world.get::<KeepDirectlyAbove>(balloon_e).copied(),
            Some(KeepDirectlyAbove { peer: char_e }),
            "scope{}: バルーン窓が同一スコープのキャラ窓を指す宣言を持たない",
            p.scope
        );
        // ① 宣言は手前に居るべき側（バルーン窓）にのみ付く
        assert!(
            world.get::<KeepDirectlyAbove>(char_e).is_none(),
            "scope{}: キャラ窓に宣言が付いている（向きが逆）",
            p.scope
        );
    }

    // ② 非ゴースト窓には付かない
    assert!(
        world.get::<KeepDirectlyAbove>(stranger).is_none(),
        "spawn_ghost_windows が自分の生成窓以外へ宣言を挿入している"
    );

    // ③ peer はスコープを跨がない
    assert_ne!(
        world
            .get::<KeepDirectlyAbove>(gw.balloon_window(0).unwrap())
            .unwrap()
            .peer,
        gw.char_window(1).unwrap(),
        "scope0 の宣言が scope1 のキャラ窓を指している（スコープ結合の取り違え）"
    );

    // 宣言を持つ窓はバルーン窓ちょうど 2 個（生成窓 4 個のうち半分）
    let declared: Vec<Entity> = world
        .query_filtered::<Entity, With<KeepDirectlyAbove>>()
        .iter(&world)
        .collect();
    assert_eq!(
        declared.len(),
        2,
        "宣言を持つ窓は 2 スコープぶんのバルーン窓ちょうど 2 個: {declared:?}"
    );
}

/// 6.1: spawn は scope とペア両窓を載せた `declared` レコードを scope ごとに
/// **ちょうど 1 本**出す（wintf 側レコードとの 2 段 grep の結合キー供給）。
///
/// 本文は純関数 `zorder_pair_declared_line` の組立結果と一致させる（組立の二重実装を
/// 許さない）。捕捉窓の中に確実に拾える記録が併置されているので、件数の主張は
/// 「捕捉が働いていないから 0 本」では成立しない——2 本を名指しで突き合わせる。
#[test]
fn spawn_emits_one_declared_record_per_scope_with_scope_and_both_entities() {
    let mut world = World::new();
    let placements = two_scope_placements();

    let (gw, events) = capture_logs(|| spawn_ghost_windows(&mut world, &placements, &titles()));

    let declared: Vec<&crate::placement::test_support::LogEvent> = events
        .iter()
        .filter(|e| e.message().contains(ZORDER_PAIR_DECLARED_TAG))
        .collect();
    assert_eq!(
        declared.len(),
        2,
        "2 スコープぶんの宣言レコードがちょうど 2 本ではない: {events:?}"
    );

    for (index, p) in placements.iter().enumerate() {
        let char_e = gw.char_window(p.scope).unwrap();
        let balloon_e = gw.balloon_window(p.scope).unwrap();
        assert_eq!(
            declared[index].message(),
            zorder_pair_declared_line(p.scope, char_e, balloon_e),
            "scope{} の宣言レコードが scope・キャラ窓・バルーン窓と一致しない",
            p.scope
        );
        assert_eq!(
            declared[index].level,
            Level::DEBUG,
            "宣言レコードは debug 水準（design 診断ログ語彙表）"
        );
    }

    // 2 スコープのレコードは互いに異なる（1 スコープぶんを 2 回出していない）
    assert_ne!(
        declared[0].message(),
        declared[1].message(),
        "同一内容の宣言レコードが 2 本出ている: {events:?}"
    );
}
