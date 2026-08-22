use bevy_ecs::prelude::*;
use wintf::ecs::Point;
use wintf::ecs::pointer::Phase;

use super::test_support::{
    drag_end_event_at, drag_event, drag_event_at, dragging_state, fake_handle, odd_edge_snapshot,
    position_of, rect, single_monitor_snapshot, window_pos_at, window_pos_sized,
};
use super::{
    Anchored, BalloonFollow, MonitorSnapshot, PlacementRoute, move_window_to, on_balloon_drag,
    on_char_drag, on_char_drag_end, resize_window_to,
};
use crate::placement::resolver::Anchor;
use crate::placement::resolver::PointPx;
use crate::placement::resolver::SizePx;

// -------------------------------------------------------------------------
// on_char_drag（4.2/4.3/4.4・U4）
// -------------------------------------------------------------------------

/// Tunnel フェーズは無視する（donor on_shell_drag と同じ規約）。
#[test]
fn on_char_drag_tunnel_phase_is_ignored() {
    let mut world = World::new();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
        .id();
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(50, 60),
            BalloonFollow {
                balloon,
                offset: PointPx { x: 11, y: 22 },
            },
        ))
        .id();

    let ev = Phase::Tunnel(drag_event(window));
    assert!(!on_char_drag(&mut world, window, window, &ev));
    assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
}

/// Bubble フェーズ: キャラ窓の WindowPos（wndproc 更新済み想定・物理 px）に
/// offset を加算した位置へバルーンが追従する。再スケールなしの檻として
/// 96 の倍数を避けた座標で完全一致を要求する（U4・3.3）。
#[test]
fn on_char_drag_bubble_moves_balloon_by_offset() {
    let mut world = World::new();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: 498, y: -37 };
    // wndproc がドラッグ中に更新した後のキャラ窓位置を模す
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(1207, 653),
            BalloonFollow { balloon, offset },
        ))
        .id();

    let ev = Phase::Bubble(drag_event(window));
    // donor 同様、イベントは消費しない（伝播続行＝false）
    assert!(!on_char_drag(&mut world, window, window, &ev));

    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 1207 + offset.x,
            y: 653 + offset.y
        }
    );
    // キャラ窓自体はハンドラでは動かさない（wndproc の領分）
    assert_eq!(position_of(&world, window), Point { x: 1207, y: 653 });
}

/// キャラ窓に WindowPos（position）が無ければ何もしない（false・panic なし）。
#[test]
fn on_char_drag_without_window_pos_is_noop() {
    let mut world = World::new();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
        .id();
    let window = world
        .spawn((
            fake_handle(0x1000),
            BalloonFollow {
                balloon,
                offset: PointPx { x: 11, y: 22 },
            },
        ))
        .id();

    let ev = Phase::Bubble(drag_event(window));
    assert!(!on_char_drag(&mut world, window, window, &ev));
    assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
}

/// BalloonFollow の無い entity への Bubble は no-op（false・panic なし）。
#[test]
fn on_char_drag_without_balloon_follow_is_noop() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x1000), window_pos_at(50, 60)))
        .id();

    let ev = Phase::Bubble(drag_event(window));
    assert!(!on_char_drag(&mut world, window, window, &ev));
    assert_eq!(position_of(&world, window), Point { x: 50, y: 60 });
}

/// (a)(b) 単一ライター・振動なし: 連続 DragEvent の**各適用直後**に WindowPos が
/// 「X=生ドラッグ X・Y=釘付け Y」を示し、非釘付け Y が一度も現れない
/// （v1 の事後補正振動に対する最強の檻——反映段階で既に正しい座標のみが書かれる）。
/// X はカーソル差分の素通し（物理 px・再スケールなし・4.7）。
#[test]
fn on_char_drag_writes_only_policy_applied_positions() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=1043−687=356
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687), // 釘付け済み初期位置
            Anchored(Anchor::Bottom),
            dragging_state((1207, 356), start),
        ))
        .id();

    // 上下左右へ振るカーソル列（生 Y はどれも下端から浮く／沈む値になる）
    for cursor in [(1450, 650), (1500, 300), (1290, 900), (1601, 113)] {
        let ev = Phase::Bubble(drag_event_at(window, start, cursor));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        let expected_x = 1207 + (cursor.0 - start.0);
        assert_eq!(
            position_of(&world, window),
            Point {
                x: expected_x,
                y: 356
            },
            "cursor={cursor:?}: 反映段階で既に釘付け済みの座標のみが書かれる"
        );
    }
}

/// (b') 非 Bottom アンカーの drag 配線存在チェック（Req1.6・design Integration
/// Tests #8 末尾・[[test-only-decision-branches-not-proven-wiring]] の「一度」）:
/// `Anchored(Left)` 窓のドラッグで X=`wa.left` 固定・Y 保持（縦自由）になる。
///
/// これは `on_char_drag` の drag 配線が**実 `Anchored.0`（Left）を `project_anchor`
/// へ転送している**証拠であり、`Anchor::Bottom` をハードコードしていないことを
/// 弁別する檻——もし Bottom 決め打ちなら X=raw.x（≠wa.left）・Y=wa.bottom−h
/// （≠raw.y）となって落ちる。期待 `(wa.left, raw.y)` と Bottom 誤配線の
/// `(raw.x, wa.bottom−h)` が両軸とも全く異なる座標になるよう値を選ぶ。Top/Right
/// の drag は同一配線の再確認ゆえ足さない（proven-wiring 過剰檻の回避）。
#[test]
fn on_char_drag_left_anchor_pins_left_edge_and_keeps_y() {
    let mut world = World::new();
    // 96 非倍数の left=53・bottom=1043・非零原点（dpi/96 再スケール混入の檻）
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let start = (1400, 600);
    // 初期窓位置＋カーソル差分で生ドラッグ座標 raw を復元（policy_mapped_position と同式）:
    // raw.x = 700 + (1500−1400) = 800／raw.y = 300 + (917−600) = 617
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(700, 300, 434, 687),
            Anchored(Anchor::Left),
            dragging_state((700, 300), start),
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(window, start, (1500, 917)));
    // donor 同様イベントは消費しない（伝播続行＝false）
    assert!(!on_char_drag(&mut world, window, window, &ev));

    // Left（左端固定・縦自由）: X=wa.left=53・Y=raw.y=617。もし配線が Bottom を
    // ハードコードしていたら (raw.x=800, wa.bottom−h=1043−687=356) となり、両軸とも
    // 全く異なる座標で落ちる（wa.left 53 ≠ raw.x 800／wa.bottom−h 356 ≠ raw.y 617）。
    assert_eq!(
        position_of(&world, window),
        Point { x: 53, y: 617 },
        "実 Anchored.0=Left を転送: X=wa.left 固定・Y=raw.y 保持（Bottom 決め打ちなら落ちる）"
    );
}

/// (c) モニタ跨ぎ: 生ドラッグ位置の窓中心が隣モニタへ移ったら、跨いだ先の
/// work area 下端へ再吸着し、戻れば元モニタの下端へ戻る（live 算出・4.7）。
#[test]
fn on_char_drag_resnaps_to_crossed_monitor_bottom() {
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![
            rect(0, 0, 1920, 1040),       // primary（下端 1040）
            rect(1920, -213, 4480, 1227), // 右の高解像度モニタ（下端 1227）
        ],
    });
    let start = (1600, 500);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1400, 353, 434, 687), // primary の下端に釘付け済み
            Anchored(Anchor::Bottom),
            dragging_state((1400, 353), start),
        ))
        .id();

    // カーソルを右モニタ方向へ: raw=(2700,353)・中心 x=2917 → 右モニタ帰属
    let ev = Phase::Bubble(drag_event_at(window, start, (2900, 500)));
    assert!(!on_char_drag(&mut world, window, window, &ev));
    assert_eq!(
        position_of(&world, window),
        Point {
            x: 2700,
            y: 1227 - 687
        }
    );

    // 戻す: raw=(1100,353)・中心 x=1317 → primary へ再吸着
    let ev = Phase::Bubble(drag_event_at(window, start, (1300, 500)));
    assert!(!on_char_drag(&mut world, window, window, &ev));
    assert_eq!(
        position_of(&world, window),
        Point {
            x: 1100,
            y: 1040 - 687
        }
    );
}

/// (d) Free 窓（`Anchored(Free)`＝move_window=true）は wndproc 委譲のまま:
/// ハンドラはキャラ窓を書かず、DraggingState があってもポリシー写像を使わない
/// （wndproc 更新済み WindowPos 基準でバルーン追従のみ・挙動不変・4.7/Req1.6）。
#[test]
fn on_char_drag_free_window_stays_wndproc_delegated() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: 498, y: -37 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 217, 434, 687), // wndproc がドラッグ中に更新した位置
            Anchored(Anchor::Free),
            BalloonFollow { balloon, offset },
            // DraggingState が居ても free 経路は写像を使わない檻（実 flow でも挿入される）
            dragging_state((999, 888), (0, 0)),
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(window, (0, 0), (10, 10)));
    assert!(!on_char_drag(&mut world, window, window, &ev));

    // キャラ窓は不動（wndproc の領分）・バルーンは WindowPos 基準で追従
    assert_eq!(position_of(&world, window), Point { x: 1207, y: 217 });
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 1207 + offset.x,
            y: 217 + offset.y
        }
    );
}

/// (d') `Anchored` 不在（安全側フォールバック・task 2.7 の新規判断分岐）: marker が
/// 一切無い窓は Free と同じく wndproc 委譲へ倒す——DraggingState が居ても単一ライター
/// 写像を走らせず、キャラ窓を書かない（旧「marker 無し＝Free」意味論の保存・Req1.6）。
#[test]
fn on_char_drag_without_anchored_stays_wndproc_delegated() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: 498, y: -37 };
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 217, 434, 687), // wndproc がドラッグ中に更新した位置
            // Anchored は付けない（None）——DraggingState は実 flow 同様に挿入される
            BalloonFollow { balloon, offset },
            dragging_state((999, 888), start),
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(window, start, (1601, 113)));
    assert!(!on_char_drag(&mut world, window, window, &ev));

    // 単一ライター写像は走らず、キャラ窓は wndproc 更新位置のまま不動
    assert_eq!(position_of(&world, window), Point { x: 1207, y: 217 });
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 1207 + offset.x,
            y: 217 + offset.y
        }
    );
}

/// (e) バルーン追従はポリシー**適用後**座標＋offset 基準
/// （生ドラッグ座標基準だと Y がずれる檻・4.2/4.7）。
#[test]
fn on_char_drag_balloon_follows_policy_applied_position() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: -400, y: 25 };
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
            dragging_state((1207, 356), start),
        ))
        .id();

    // カーソルが上へ 250px: raw Y=106 だが適用後 Y=356
    let ev = Phase::Bubble(drag_event_at(window, start, (1450, 350)));
    assert!(!on_char_drag(&mut world, window, window, &ev));

    let char_pos = position_of(&world, window);
    assert_eq!(char_pos, Point { x: 1257, y: 356 });
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: char_pos.x + offset.x,
            y: char_pos.y + offset.y
        }
    );
}

/// (f) DragEnd: 最終カーソル位置へ同写像を適用する（accumulator の
/// `current_dragging_entity` 先行クリアで最終 DragEvent が欠落する穴の埋め・
/// DD15 v2 (3)）。バルーンも適用後座標基準で追従する。
#[test]
fn on_char_drag_end_applies_policy_at_final_cursor() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: -400, y: 25 };
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            // 「最後に配送された DragEvent 時点」の位置を模す（最終位置とはずれている）
            window_pos_sized(1250, 356, 434, 687),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
            // OnDragEnd 配送時点では DraggingState はまだ生きている（dispatch.rs は
            // ハンドラ配送**後**に remove する）——実 flow 準拠
            dragging_state((1207, 356), start),
        ))
        .id();

    let ev = Phase::Bubble(drag_end_event_at(window, (1601, 113)));
    assert!(!on_char_drag_end(&mut world, window, window, &ev));

    // raw=(1207+201, 356−487)=(1408, −131) → 適用後 (1408, 356)
    assert_eq!(position_of(&world, window), Point { x: 1408, y: 356 });
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 1408 + offset.x,
            y: 356 + offset.y
        }
    );
}

/// (f) 補: DragEnd の Tunnel フェーズは無視する（他ハンドラと同じ規約）。
#[test]
fn on_char_drag_end_tunnel_phase_is_ignored() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1250, 356, 434, 687),
            Anchored(Anchor::Bottom),
            dragging_state((1207, 356), start),
        ))
        .id();

    let ev = Phase::Tunnel(drag_end_event_at(window, (1601, 113)));
    assert!(!on_char_drag_end(&mut world, window, window, &ev));
    assert_eq!(position_of(&world, window), Point { x: 1250, y: 356 });
}

/// Task 2.2 保存フック（Req1.1/1.9・design C2）: 非 Free アンカーのキャラ窓の
/// DragEnd で、確定位置 `mapped` が当該スコープの `WindowPos` entries として Ghost
/// 永続スコープへ write-through される。`barrier` 後に別ハンドルの `load_scope` で
/// 読み戻し、保存 x/y が `mapped` 位置に等しいことを固定する（persist.rs の
/// `persist_entries_with_wiring_write_through_to_ghost_scope` と同流儀の実 publisher 檻）。
#[test]
fn on_char_drag_end_persists_char_pos_for_scope() {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use areka_sylphya::persist::{FakePersistIo, PersistIo};
    use areka_sylphya::{
        Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
    };

    use super::super::persist::PersistWiring;
    use crate::placement::spawn::CharWindowMarker;

    // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す・persist.rs と同流儀）。
    struct SharedFakeIo(Arc<FakePersistIo>);
    impl PersistIo for SharedFakeIo {
        fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
            self.0.read(path)
        }
        fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
            self.0.commit(path, content)
        }
    }

    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(SharedFakeIo(shared.clone())),
        runtime_sink: None,
    });

    let mut world = World::new();
    // UI スレッド常駐の保存投函口を挿入（persist_entries が引く NonSend リソース）。
    world.insert_non_send(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=1043−687=356

    let start = (1400, 600);
    // scope=1 の非 Free（Bottom）キャラ窓。値は (f) 檻と同一＝mapped=(1408, 356) が既知。
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1250, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 1 },
            dragging_state((1207, 356), start),
        ))
        .id();

    // 最終カーソル (1601, 113) → raw=(1408, −131) → 適用後 mapped=(1408, 356)
    let ev = Phase::Bubble(drag_end_event_at(window, (1601, 113)));
    assert!(!on_char_drag_end(&mut world, window, window, &ev));

    // 確定点: mapped が WindowPos へ反映されている
    assert_eq!(position_of(&world, window), Point { x: 1408, y: 356 });

    // barrier 復帰＝上記 put の write-through 保存まで完了（同一送信端 FIFO）。
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    // 別ハンドルの load_scope で scope1 の WindowPos を観測（実 IO 通過＝投函の証明）。
    // 保存 x は**原点＝下端中央**基準（左上 1408 ＋ w/2=217 → 1625）。
    let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 1,
                axis: Axis::X
            },
            "1625".to_string()
        )),
        "DragEnd 確定位置 X=1408 が scope1 の WindowPos として保存されていない: {loaded:?}"
    );
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 1,
                axis: Axis::Y
            },
            "356".to_string()
        )),
        "DragEnd 確定位置 Y=356 が scope1 の WindowPos として保存されていない: {loaded:?}"
    );

    // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
    parts.publisher.close();
    let _ = parts.handle.join();
}

/// Task 8.1 偽 Free アンカー DragEnd→保存値等価の檻（Req1.1・design C2/C3・
/// Testing Strategy Unit §7）: `Anchored(Anchor::Free)` のキャラ窓を headless World に
/// 合成し、DragEnd 駆動→`project_anchor` の Free identity 腕を素通しした確定位置が、
/// **アンカー種別を問わず**そのまま `WindowPos` entries 化されて Ghost 永続スコープへ
/// write-through されることを決定論固定する。
///
/// なぜこの檻が正本か: 保存はドラッグ中の吸着制約（Bottom 等）ではなく DragEnd の
/// 確定位置を書く（Req1.1）。Free は wndproc（move_window=true）が動かし切った位置を
/// `project_anchor` が identity で無害通過させ、本ハンドラが**保存専用アーム**として
/// 働く——実 emo2 は全スコープ Bottom（実機で Free の保存経路を一度も踏まない）ゆえ、
/// この偽 Free 檻だけがその等価性の source of truth となる。
///
/// 檻の噛み方（射影が Free 位置を改変したら落ちる）: snapshot を挿入し、確定 raw の
/// Y=883 は同モニタの bottom 吸着値（1043−687=356）と**異なる**値を選ぶ。もし Free が
/// identity でなく（誤って Bottom 等へ）射影されれば mapped.y と保存 Y が 356 へ変わり、
/// position_of・load_scope の双方が落ちる。座標は 96 の非倍数（1531・883）で隠れた
/// dpi/96 再スケールの檻も兼ね、既定値（0・96 系）と重ならない。
#[test]
fn on_char_drag_end_persists_free_anchor_raw_position_for_scope() {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use areka_sylphya::persist::{FakePersistIo, PersistIo};
    use areka_sylphya::{
        Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
    };

    use super::super::persist::PersistWiring;
    use crate::placement::spawn::CharWindowMarker;

    // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す・上の Bottom 檻と同流儀）。
    struct SharedFakeIo(Arc<FakePersistIo>);
    impl PersistIo for SharedFakeIo {
        fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
            self.0.read(path)
        }
        fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
            self.0.commit(path, content)
        }
    }

    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(SharedFakeIo(shared.clone())),
        runtime_sink: None,
    });

    let mut world = World::new();
    world.insert_non_send(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    // snapshot 挿入（bottom=1043）。Free identity なら未使用だが、誤射影時に
    // Bottom 吸着 Y=1043−687=356 が現れる差分検出のため意図的に居させる。
    world.insert_resource(single_monitor_snapshot());

    // scope=2 の Free キャラ窓。DraggingState は実 flow（dispatch_drag_events 挿入）を模す。
    // initial_inset=(1250,356)＝ドラッグ開始時窓位置・drag_start=(1400,600)＝開始カーソル。
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1250, 356, 434, 687),
            Anchored(Anchor::Free),
            CharWindowMarker { scope: 2 },
            dragging_state((1250, 356), start),
        ))
        .id();

    // 最終カーソル (1681, 1127) → raw = (1250+(1681−1400), 356+(1127−600)) = (1531, 883)。
    // Free identity ゆえ mapped = raw = wndproc 確定位置（射影で改変されない）。
    let ev = Phase::Bubble(drag_end_event_at(window, (1681, 1127)));
    assert!(!on_char_drag_end(&mut world, window, window, &ev));

    // 確定点: mapped=(1531,883) が WindowPos へ反映（Bottom 吸着 356 ではなく生確定 883）。
    assert_eq!(
        position_of(&world, window),
        Point { x: 1531, y: 883 },
        "Free は identity 射影＝確定 raw をそのまま反映（Bottom 誤射影なら Y=356 で落ちる）"
    );

    // barrier 復帰＝上記 put の write-through 保存まで完了（同一送信端 FIFO）。
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");

    // 別ハンドルの load_scope で scope2 の WindowPos を読み戻す（実 IO 通過＝投函の証明）。
    // Free アンカーでも保存値は確定 raw と value-equal（アンカー種別を問わない・Req1.1）。
    let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 2,
                axis: Axis::X
            },
            "1531".to_string()
        )),
        "Free DragEnd 確定 X=1531 が scope2 の WindowPos として保存されていない: {loaded:?}"
    );
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 2,
                axis: Axis::Y
            },
            "883".to_string()
        )),
        "Free DragEnd 確定 Y=883 が scope2 の WindowPos として保存されていない\
         （Bottom 誤射影なら 356・保存脱落なら空）: {loaded:?}"
    );

    // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
    parts.publisher.close();
    let _ = parts.handle.join();
}

/// Task 8.3 発火規律の統合檻（Req1.9・8.4・design C2/C3・Testing Strategy Integration §2）:
/// 永続の窓位置・バルーン相対オフセットを書くのは **DragEnd の観測点のみ**であり、
/// 自動再射影（`resize_window_to`）・`\![move]` 消費経路（`move_window_to`）・復元時
/// 再射影（`apply_restored_placements`・純関数）・**連続ドラッグ**（`on_char_drag`）は
/// 永続ストアを一切書き換えないことを、ストア内容のバイト等価で決定論固定する。
///
/// # 檻の噛み方（意味のある不変チェックにするための seed）
///
/// まず 1 回の正当な書込（char の `on_char_drag_end`）で **ストアに内容を与えて**から
/// スナップショットを捕捉する（空ストア同士の比較では「何も書かない」ことが自明に
/// 成立してしまい檻が噛まないため）。その後に非 DragEnd 操作群を駆動し、`barrier` を
/// 挟んで（保留 put があれば flush される）ストア内容を再捕捉し、seed 時点と **完全一致**
/// することを assert する。`load_scope` は決定論順（sylphya 契約）ゆえ Vec を直接比較できる。
///
/// もし駆動した操作のいずれかが `persist_put` を投函すれば、scope1 の WindowPos／
/// BalloonOffset entries が変化して seed スナップショットと乖離し、本 assert が落ちる
/// （RED 検証: 駆動ブロックへ一時的に `persist_entries` を差し込むと本檻が実際に落ちることを
/// 確認済み——発火規律が破れれば必ず検出する）。emo2 は全スコープ Bottom＝実機で Free の
/// 保存経路を踏まないのと同様、自動再射影が永続へ漏れないことは決定論檻でのみ観測できる。
#[test]
fn non_dragend_operations_leave_persist_store_byte_invariant() {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use areka_sylphya::persist::{FakePersistIo, PersistIo};
    use areka_sylphya::{PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya};

    use super::super::persist::{PersistWiring, apply_restored_placements};
    use crate::placement::resolver::ScopePlacement;
    use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

    // 共有 fake IO（アクター Box 移送用と観測用で同一ストアを指す・上の DragEnd 檻と同流儀）。
    struct SharedFakeIo(Arc<FakePersistIo>);
    impl PersistIo for SharedFakeIo {
        fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
            self.0.read(path)
        }
        fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
            self.0.commit(path, content)
        }
    }

    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let parts = spawn_sylphya(SylphyaInit {
        roots: roots.clone(),
        io: Box::new(SharedFakeIo(shared.clone())),
        runtime_sink: None,
    });

    let mut world = World::new();
    world.insert_non_send(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=1043−687=356

    // char 窓 scope=1（Bottom）＋ balloon 窓 scope=1（BalloonFollow で連結）。
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(701, 383),
            BalloonWindowMarker { scope: 1 },
        ))
        .id();
    let start = (1400, 600);
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1250, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 1 },
            BalloonFollow {
                balloon,
                offset: PointPx { x: -549, y: 27 },
            },
            dragging_state((1207, 356), start),
        ))
        .id();

    // --- SEED: 1 回の正当な書込（char DragEnd）でストアに内容を与える（不変チェックを
    //     意味あるものにするため）。最終カーソル (1601,113) → mapped=(1408, 356)。
    let ev = Phase::Bubble(drag_end_event_at(char_window, (1601, 113)));
    assert!(!on_char_drag_end(&mut world, char_window, char_window, &ev));
    parts
        .publisher
        .barrier()
        .expect("seed barrier should resolve while actor is alive");

    // seed 書込後のストア内容を正準スナップショットとして捕捉（load_scope は決定論順）。
    let before = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
    assert!(
        !before.is_empty(),
        "seed の DragEnd 書込がストアを満たしていない＝不変チェックが無意味になる: {before:?}"
    );

    // --- DRIVE: 書いてはならない非 DragEnd 操作群を駆動する ---------------------------
    // 1) `\![move]` 消費経路（apply_move_directive が唯一呼ぶ位置ライター）。
    assert!(
        move_window_to(&mut world, char_window, 999, 777),
        "move_window_to は成立するはず（char に WindowHandle あり）"
    );
    // 2) 自動再射影（re-snap）経路。Bottom → project_anchor で y=1043−700=343 へ再固定。
    assert!(
        resize_window_to(
            &mut world,
            char_window,
            SizePx { w: 500, h: 700 },
            PlacementRoute::Resnap
        ),
        "resize_window_to は成立するはず（Anchored/正寸/WindowHandle あり）"
    );
    // 3) 復元時再射影（純関数・World も永続も触れない・返り値は捨てる）。
    let snap = single_monitor_snapshot();
    let placements = vec![ScopePlacement {
        scope: 1,
        char_pos: PointPx { x: 1250, y: 356 },
        char_size: SizePx { w: 434, h: 687 },
        balloon_pos: PointPx { x: 701, y: 383 },
        balloon_size: SizePx { w: 200, h: 300 },
        balloon_offset: PointPx { x: -549, y: 27 },
        // windowposition-limit: 正典既定（有効）。本檻は limit の判定を対象にしない。
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }];
    let _restored = apply_restored_placements(placements, &before, &snap);
    // 4) 連続ドラッグ（DragEnd ではない・書込トリガにしない確定点規律）。
    let drag = Phase::Bubble(drag_event_at(char_window, start, (1450, 350)));
    assert!(!on_char_drag(&mut world, char_window, char_window, &drag));

    // 保留 put（存在しないはず）があれば flush する越境フェンス。
    parts
        .publisher
        .barrier()
        .expect("post-drive barrier should resolve while actor is alive");

    // --- ASSERT: ストア内容が seed 時点と完全一致（非 DragEnd 操作は何も書いていない）---
    let after = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
    assert_eq!(
        before, after,
        "非 DragEnd 操作（move_window_to / resize_window_to / apply_restored_placements / \
         連続 on_char_drag）が永続ストアを書き換えた（Req1.9/8.4 発火規律違反）: \
         before={before:?} after={after:?}"
    );

    // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
    parts.publisher.close();
    let _ = parts.handle.join();
}

/// (g) target==自 entity ガード: 他 entity 宛イベントの Bubble を受けても
/// on_char_drag／on_char_drag_end／on_balloon_drag はすべて no-op。
#[test]
fn drag_handlers_ignore_events_targeting_other_entities() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let other = world.spawn_empty().id();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(701, 383)))
        .id();
    let initial = PointPx { x: 11, y: 22 };
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            Anchored(Anchor::Bottom),
            BalloonFollow {
                balloon,
                offset: initial,
            },
            dragging_state((1207, 356), start),
        ))
        .id();

    // on_char_drag: target=other → 窓もバルーンも不動
    let ev = Phase::Bubble(drag_event_at(other, start, (1601, 113)));
    assert!(!on_char_drag(&mut world, other, window, &ev));
    assert_eq!(position_of(&world, window), Point { x: 1207, y: 356 });
    assert_eq!(position_of(&world, balloon), Point { x: 701, y: 383 });

    // on_char_drag_end: target=other → 不動
    let ev = Phase::Bubble(drag_end_event_at(other, (1601, 113)));
    assert!(!on_char_drag_end(&mut world, other, window, &ev));
    assert_eq!(position_of(&world, window), Point { x: 1207, y: 356 });

    // on_balloon_drag: target=other → offset 不変
    let ev = Phase::Bubble(drag_event_at(other, start, (10, 10)));
    assert!(!on_balloon_drag(&mut world, other, balloon, &ev));
    assert_eq!(world.get::<BalloonFollow>(window).unwrap().offset, initial);
}

/// (+) MonitorSnapshot 不在（main.rs フォールバック経路）: ポリシーは identity
/// へ縮退し、窓は生ドラッグ座標のまま単一ライターで移動する（move_window=false
/// でもドラッグ追従が生きる縮退・吸着なし・panic なし）。
#[test]
fn on_char_drag_without_snapshot_moves_to_raw_position() {
    let mut world = World::new(); // Resource 未挿入
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let offset = PointPx { x: 11, y: 22 };
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
            dragging_state((1207, 356), start),
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(window, start, (1450, 350)));
    assert!(!on_char_drag(&mut world, window, window, &ev));

    // raw=(1257, 106) そのまま・バルーンは raw 基準で追従
    assert_eq!(position_of(&world, window), Point { x: 1257, y: 106 });
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 1257 + offset.x,
            y: 106 + offset.y
        }
    );
}

/// (+) WindowPos.size 不在／`WindowPos::default()` の CW_USEDEFAULT センチネル:
/// 非正寸法として identity 縮退＝生ドラッグ座標のまま移動（暴走・panic なし）。
#[test]
fn on_char_drag_with_invalid_size_degrades_to_identity() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let start = (1400, 600);

    // size=None
    let mut wp = window_pos_at(1207, 356);
    wp.size = None;
    let no_size = world
        .spawn((
            fake_handle(0x1000),
            wp,
            Anchored(Anchor::Bottom),
            dragging_state((1207, 356), start),
        ))
        .id();
    let ev = Phase::Bubble(drag_event_at(no_size, start, (1450, 350)));
    assert!(!on_char_drag(&mut world, no_size, no_size, &ev));
    assert_eq!(position_of(&world, no_size), Point { x: 1257, y: 106 });

    // size=CW_USEDEFAULT センチネル（window_pos_at は ..Default::default()）
    let sentinel = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(1207, 356),
            Anchored(Anchor::Bottom),
            dragging_state((1207, 356), start),
        ))
        .id();
    let ev = Phase::Bubble(drag_event_at(sentinel, start, (1450, 350)));
    assert!(!on_char_drag(&mut world, sentinel, sentinel, &ev));
    assert_eq!(position_of(&world, sentinel), Point { x: 1257, y: 106 });
}

/// (+) DraggingState 不在の BottomSnap 窓（実 flow では dispatch が DragEvent
/// より先に挿入する）: 生座標を復元できないため書き込みなし（panic なし・
/// バルーンも不動）。
#[test]
fn on_char_drag_without_dragging_state_is_noop_for_snap_window() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
        .id();
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            Anchored(Anchor::Bottom),
            BalloonFollow {
                balloon,
                offset: PointPx { x: 11, y: 22 },
            },
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(window, (1400, 600), (1450, 350)));
    assert!(!on_char_drag(&mut world, window, window, &ev));
    assert_eq!(position_of(&world, window), Point { x: 1207, y: 356 });
    assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
}
