use bevy_ecs::prelude::*;
use tracing::Level;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_ACCEPTFILES, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use wintf::ecs::drag::{DragConfig, DragConstraint, OnDrag, OnDragEnd};
use wintf::ecs::layout::{BoxStyle, HitTest};
use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed};
use wintf::ecs::{
    DpiSuggestedRectPolicy, KeepDirectlyAbove, Point, SizeI, Window, WindowPos, WindowStyle,
};

use areka_placement::diag::{ZORDER_PAIR_DECLARED_TAG, zorder_pair_declared_line};
use areka_placement::follow::{AnchorSource, Anchored, BalloonFollow};
use areka_placement::resolver::Anchor;
use areka_placement::source::GhostTitles;
use areka_placement::spawn::test_support::{ghost_window_entities, titles, two_scope_placements};
use areka_placement::spawn::{
    BalloonLimit, BalloonWindowMarker, CharWindowMarker, GhostWindowMarker, GhostWindows,
    spawn_ghost_windows,
};
use log_capture_kit::capture;

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
        let balloon_pos = world
            .get::<WindowPos>(balloon_e)
            .expect("balloon WindowPos");
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
        assert_eq!(
            resource.balloon_window(scope),
            returned.balloon_window(scope)
        );
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
        assert_eq!(follow.offset(), p.balloon_offset);
        assert!(world.get::<OnDrag>(char_e).is_some());

        // バルーン窓: 相対位置記憶ハンドラあり（4.8）・BalloonFollow なし
        assert!(world.get::<BalloonFollow>(balloon_e).is_none());
        assert!(world.get::<OnDrag>(balloon_e).is_some());
    }
}

/// T-I1 補: 全窓が `HitTest::none()`（全面ヒットで透過を殺さない）を持つ。
/// `spawn_ghost_windows` 自体はポインタハンドラを付けない（依存方向
/// input_events→placement）。
/// spawn 直後に `areka_input::attach_char_pointer_handlers` を呼ぶと、キャラ窓は
/// 正規ポインタハンドラ（`OnPointerMoved`＋`OnPointerPressed`＝stand-in
/// `on_ghost_pressed` を退役して差し替え）を持ち、キャラ窓のハンドラ装着はバルーン窓に
/// 何も付けない（バルーン窓のハンドラは `areka_input::balloon::attach_balloon_pointer_handlers`
/// が別に付ける）。ハンドラ実挙動の檻は input_events の檻が持つので、ここは存在の有無
/// だけを固定する。装着の関数は areka_input の側にあり
/// areka_placement からは見えない（依存の向き）ので、本テストは areka_input の側に置く。
#[test]
fn t_i1_all_windows_have_hit_test_none_and_char_has_pointer_handlers() {
    let mut world = World::new();
    let placements = two_scope_placements();

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    // spawn は入力の crate を見られないのでハンドラを付けない。装着は areka_input が担う。
    crate::attach_char_pointer_handlers(&mut world);

    for e in ghost_window_entities(&mut world) {
        assert_eq!(world.get::<HitTest>(e).copied(), Some(HitTest::none()));
    }

    for p in &placements {
        let char_e = gw.char_window(p.scope).unwrap();
        let balloon_e = gw.balloon_window(p.scope).unwrap();

        // キャラ窓は正規の移動／押下ハンドラを両方持つ
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

        // キャラ窓のハンドラ装着はバルーン窓にポインタハンドラを付けない（バルーン窓の分は別の関数）
        assert!(
            world.get::<OnPointerMoved>(balloon_e).is_none(),
            "scope{}: キャラ窓のハンドラ装着がバルーン窓に OnPointerMoved を付けた",
            p.scope
        );
        assert!(
            world.get::<OnPointerPressed>(balloon_e).is_none(),
            "scope{}: キャラ窓のハンドラ装着がバルーン窓に OnPointerPressed を付けた",
            p.scope
        );
    }
}

// -------------------------------------------------------------------------
// anchor 伝搬（4.2・DD15 基盤）
// -------------------------------------------------------------------------

/// 要件 4.2・1.6: 生成直後の**全 char 窓**が対応する 5 値アンカーの
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

/// areka-P0-window-link-directives 1.1（要件 6.1／6.2）: 作った全 char 窓に、`Anchored` と同じ
/// descript の向き・台本の印なしの `AnchorSource` が付く。バルーン窓には付けない。
#[test]
fn anchor_source_attached_to_all_char_windows_with_descript_anchor() {
    let mut world = World::new();
    let mut placements = two_scope_placements();
    placements[1].anchor = Anchor::Free;

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    for p in &placements {
        assert_eq!(
            world
                .get::<AnchorSource>(gw.char_window(p.scope).unwrap())
                .copied(),
            Some(AnchorSource {
                descript: p.anchor,
                by_script: false,
            }),
            "scope{}: AnchorSource は descript の向き・印なし",
            p.scope
        );
        assert!(
            world
                .get::<AnchorSource>(gw.balloon_window(p.scope).unwrap())
                .is_none(),
            "scope{}: バルーン窓には AnchorSource を付けない",
            p.scope
        );
    }
}

// -------------------------------------------------------------------------
// balloon limit 伝搬（areka-P0-windowposition-limit 2.7/2.8・design C9/DD4）
//
// `ScopeConfig.balloon_limit` → `ScopePlacement.balloon_limit` → `BalloonLimit`
// Component の一方向転写（DD4）の最終段を固定する。`Anchored` 同型だが**付く側が
// 逆**——`Anchored` はキャラ窓のみ、`BalloonLimit` はバルーン窓のみ。
// -------------------------------------------------------------------------

/// 要件 2.7: 生成直後の各バルーン窓が自スコープの解決済み limit 値を
/// `BalloonLimit(bool)` として保持する（`true`/`false` の両値を固定＝
/// 「常に true を入れている」実装では緑にならない）。
#[test]
fn balloon_limit_attached_to_every_balloon_window_with_resolved_value() {
    let mut world = World::new();
    let mut placements = two_scope_placements(); // 両方 balloon_limit: true
    placements[1].balloon_limit = false; // scope1 を limit 無効へ

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    assert_eq!(
        world
            .get::<BalloonLimit>(gw.balloon_window(0).unwrap())
            .copied(),
        Some(BalloonLimit(true)),
        "scope0: limit 有効スコープのバルーン窓は BalloonLimit(true) を保持"
    );
    assert_eq!(
        world
            .get::<BalloonLimit>(gw.balloon_window(1).unwrap())
            .copied(),
        Some(BalloonLimit(false)),
        "scope1: limit 無効スコープのバルーン窓は BalloonLimit(false) を保持"
    );
}

/// 要件 2.8 の構造保証: キャラ窓には limit 値によらず `BalloonLimit` を付けない。
///
/// runtime 関門は `BalloonLimit` を持つ entity にのみ作用するため、
/// 「キャラ窓に付いていない」ことが「limit 補正がキャラ窓の位置を変更しない」
/// （要件 2.8）の証明そのものになる。両値（有効／無効）で固定する。
#[test]
fn balloon_limit_never_attached_to_char_windows() {
    let mut world = World::new();
    let mut placements = two_scope_placements();
    placements[1].balloon_limit = false;

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());

    // `BalloonLimit` 保持者は World 全体でバルーン窓に限られる（キャラ窓への付け過ぎ・付け忘れの両封じ）。
    let holders: Vec<Entity> = world
        .query_filtered::<Entity, With<BalloonLimit>>()
        .iter(&world)
        .collect();
    let mut expected: Vec<Entity> = placements
        .iter()
        .map(|p| gw.balloon_window(p.scope).unwrap())
        .collect();
    let mut holders_sorted = holders;
    holders_sorted.sort();
    expected.sort();
    assert_eq!(
        holders_sorted, expected,
        "BalloonLimit を持つ entity はバルーン窓 2 枚だけ"
    );
}

// -------------------------------------------------------------------------
// T-I2: z-order 既定（5.1・DD13）
// -------------------------------------------------------------------------

/// T-I2（守り G3）: 全窓の `WindowStyle.ex_style` に `WS_EX_TOPMOST` が含まれない
/// （既定はオフ・5.1／DD13。style/ex_style の正値も固定する）。常に手前のオンは、ここの既定の
/// スタイルではなく、印 `StayOnTop` と窓の作り（印のある窓だけ `WS_EX_TOPMOST` を足す）で足す
/// （stay-on-top-directives 要件 2.3・6.3）。
/// 全窓が投げ込みの受け入れの宣言 `WS_EX_ACCEPTFILES` を含む（areka-P0-file-drop 要件 1.1・9.9）。
#[test]
fn t_i2_no_window_has_ws_ex_topmost() {
    let mut world = World::new();
    let placements = two_scope_placements();

    spawn_ghost_windows(&mut world, &placements, &titles());

    let entities = ghost_window_entities(&mut world);
    assert_eq!(entities.len(), 4);
    for e in entities {
        let style = world.get::<WindowStyle>(e).expect("WindowStyle");
        // 等値ゆえ `WS_EX_TOPMOST` が無いこと（5.1／DD13）と `WS_EX_ACCEPTFILES` が在ること
        // （file-drop 要件 1.1）を共に押さえる。
        assert_eq!(
            style.ex_style,
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_ACCEPTFILES
        );
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

/// 要件 4.3・D3: spawn 直後の**全**ゴースト窓（全 scope × キャラ/バルーン）が
/// `DpiSuggestedRectPolicy::ExternalAuthority` を保持する。
///
/// 空虚性回避:
/// ①探針の自己検査——**既定値が `ExternalAuthority` ではない**ことを先に固定する
/// （既定が外部権威なら「付与されている」の主張は component 未付与でも成り立ち、
/// 檻が不動点に落ちる）。②**数え上げで固定する**——`ghost_window_entities` の全数
/// （2 scope × 2 種＝4）を走査する（キャラ窓にだけ付ける部分是正が「4 件中 2 件」で
/// 赤になる）。③非ゴースト窓には付かないことも主張する（World 全体への無差別挿入で
/// 緑にならない）。
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

    spawn_ghost_windows(&mut world, &placements, &titles());

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

    // ③ 非ゴースト窓には付かない（World 全体への無差別挿入では緑にならない）
    assert!(
        world.get::<DpiSuggestedRectPolicy>(stranger).is_none(),
        "spawn_ghost_windows が自分の生成窓以外へ政策を挿入している"
    );
}

// -------------------------------------------------------------------------
// T-I3: 単位契約（U2・DD8・4.1/4.5）
// -------------------------------------------------------------------------

/// T-I3: 窓 entity に `BoxStyle` 不在（U2・論理 DIP を持ち込まない）・
/// `DragConstraint` 不在（DD8・全モニタドラッグ可・4.5）。`DragConfig.move_window`
/// は「非 Free アンカーのキャラ窓のみ false（単一ライター・DD15 v2・4.7）、Free
/// キャラ窓とバルーン窓は true（wndproc 委譲・4.1/4.5）」。DragEnd ハンドラ
/// （`OnDragEnd`）は Free 含む**全**キャラ窓に付く（非 Free は最終再固定・Free は
/// 保存専用アーム・1.1）。バルーン窓にも `OnDragEnd`（`on_balloon_drag_end`）が
/// 付く（単独ドラッグ確定 offset の永続 write-through・2.1）。`Anchored` の
/// 付き方は別のテスト（`Anchored(anchor)` の保持）が固定する。
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
        assert_eq!(
            drag.threshold,
            DragConfig::default().threshold,
            "threshold は既定値を保つ"
        );
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
        "Free キャラ窓にも OnDragEnd が付く（全アンカー結線・保存専用アーム・1.1）"
    );
    for scope in [0usize, 1] {
        let balloon = gw.balloon_window(scope).unwrap();
        assert!(
            world.get::<DragConfig>(balloon).unwrap().move_window,
            "scope{scope}: バルーン窓は move_window=true（単独ドラッグ・4.5）"
        );
        assert!(
            world.get::<OnDragEnd>(balloon).is_some(),
            "scope{scope}: バルーン窓に OnDragEnd（on_balloon_drag_end）が付く（単独ドラッグ確定 offset 保存）"
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
/// peer は自スコープのキャラ窓と等値で押さえるので、スコープを跨がない。宣言を持つ窓が
/// バルーン窓ちょうど 2 個であることで、キャラ窓（向きの逆転）・非ゴースト窓（World 全体への
/// 無差別挿入）に付いていないことも押さえる。
#[test]
fn keep_directly_above_attached_to_every_balloon_window_pointing_at_its_char_window() {
    let mut world = World::new();
    let mut placements = two_scope_placements();
    placements[1].anchor = Anchor::Free; // アンカー種別に依存しないことも同時に固定
    // 非ゴースト窓（無差別挿入の検出用の対照）
    world.spawn(Window::default());

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
    }

    // 宣言を持つ窓はバルーン窓ちょうど 2 個（生成窓 4 個と非ゴースト窓 1 個のうち）
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

    let (gw, events) = capture(|| spawn_ghost_windows(&mut world, &placements, &titles()));

    let declared: Vec<&log_capture_kit::CapturedEvent> = events
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
}
