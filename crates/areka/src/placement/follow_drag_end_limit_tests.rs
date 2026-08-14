use bevy_ecs::prelude::*;
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{SizeI, WindowPos};

use super::super::test_support::{LogEvent, capture_logs, expect_one};
use super::test_support::{
    drag_end_event_at, drag_event_at, fake_handle, point_of, rect, window_pos_sized,
};
use super::{
    BalloonFollow, MonitorSnapshot, PlacementRoute, on_balloon_drag, on_balloon_drag_end,
};
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};

// -------------------------------------------------------------------------
// 解放時補正の檻（task 3.4・design「C8: 解放時補正」・Testing Strategy Integration 2・
// 要件 2.5／5.4）
//
// 何を固定するか:
//   (1) ドラッグ**中**は 1 bit も介入しない（要件 2.5 前段）——`on_balloon_drag` は
//       カーソル追随の結果をそのまま offset へ記憶するだけで、窓を書かない
//   (2) 解放時は「解放位置の取得 → 生 offset の永続 write-through → 補正書込」の
//       **順序が固定**であること（要件 5.4・DD6）。順序が逆転すると補正値が保存値へ
//       焼き付くので、保存値・`BalloonFollow.offset` が生値のままであることと、
//       ログの**出現順**の両方で噛む
//   (3) 作業領域内で解放したときは補正の書き込みが 1 件も起きないこと
//   (4) 解放時の新規書込は `BalloonLimitRelease` を名乗ること（DD7）
//   (5) `BalloonLimit(false)` の scope では解放時補正が起きないこと（要件 2.7）
//
// World は実 spawn（`spawn_ghost_windows`）で組む——`BalloonLimit`・`BalloonFollow`・
// `Anchored`・`GhostWindows` がすべて本番の構築経路そのものになり、「spawn が付け
// 忘れても緑」を避けられる（`follow_balloon_limit_tests.rs` と同じ流儀）。
// -------------------------------------------------------------------------

/// 補正ログの判定語（本体定数とは独立に檻側へ literal で置く）。
const LIMIT_CLAMP_TAG: &str = "[balloon-limit] Clamp";
/// 解決不能（縮退）の判定語。
const LIMIT_UNRESOLVED_TAG: &str = "[balloon-limit] Unresolved";
/// 2 段 grep 第 1 段の接頭辞（「関門が何かを言った」ことの一括検出）。
const LIMIT_TAG_PREFIX: &str = "[balloon-limit]";
/// 3 関門を弁別する契機ラベル（起動時 `boot-gate`・runtime `runtime-gate` と対）。
const RELEASE_CONTEXT: &str = "release-gate";
/// 窓移動レコードの判定語（`placement::diag` の手順書語彙）。
const WINDOW_MOVE_TAG: &str = "[diag.window_move]";
/// 永続 write-through 直前に出る保存ログの判定語（順序固定の前段マーカー）。
const PERSIST_SAVE_TAG: &str = "balloon DragEnd 保存";

/// 単一モニタの作業領域（全 4 辺が 96 の非倍数・原点非 (0,0)＝隠れた再スケール檻）。
fn wa() -> RectPx {
    rect(53, 37, 1877, 1043)
}

fn single_monitor() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![wa()],
    }
}

fn balloon_size() -> SizePx {
    SizePx { w: 223, h: 158 }
}

fn char_pos() -> PointPx {
    PointPx { x: 1301, y: 411 }
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

/// ユーザーがバルーンを引きずって離した位置（右下 2 辺を同時にはみ出す）。
fn release_outside() -> PointPx {
    PointPx { x: 1800, y: 1100 }
}

/// [`release_outside`] を作業領域内へ収めた位置（檻側の独立計算）。
fn release_outside_corrected() -> PointPx {
    max_pos(wa(), balloon_size())
}

/// 作業領域内で離した位置（補正不要）。
fn release_inside() -> PointPx {
    PointPx { x: 900, y: 500 }
}

/// 生 offset（解放位置 − キャラ窓左上）＝保存されるべき値。
fn raw_offset(release: PointPx) -> PointPx {
    PointPx {
        x: release.x - char_pos().x,
        y: release.y - char_pos().y,
    }
}

/// 基準となる 1 scope ぶんの配置（キャラ窓・バルーンとも作業領域に内包）。
fn base_placement(balloon_limit: bool) -> ScopePlacement {
    ScopePlacement {
        scope: 0,
        char_pos: char_pos(),
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

/// wndproc（`move_window=true`）がドラッグでバルーン窓を動かし切った状態を作る。
///
/// 実 flow では ECS はここに一切関与しない——だからこそ解放時が唯一の観測点になる。
fn wndproc_moved_balloon_to(world: &mut World, balloon: Entity, pos: PointPx) {
    world.entity_mut(balloon).insert(window_pos_sized(
        pos.x,
        pos.y,
        balloon_size().w,
        balloon_size().h,
    ));
}

/// in-session offset へ「解放位置由来でも補正位置由来でもない」値を仕込む。
///
/// 解放時補正が `BalloonFollow.offset` を書き換えたら（＝焼き付けたら）この値が消える。
fn stale_offset() -> PointPx {
    PointPx { x: 999, y: 888 }
}

fn set_stale_offset(world: &mut World, char_window: Entity) {
    let mut follow = *world
        .get::<BalloonFollow>(char_window)
        .expect("spawn が BalloonFollow を付ける");
    follow.offset = stale_offset();
    world.entity_mut(char_window).insert(follow);
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

/// `needle` を含むイベントが**ちょうど 1 件**在ることを主張し、その添字を返す。
///
/// 添字は捕捉順＝発火順そのものであり、これが「順序固定」を直接測る唯一の手段である
/// （`capture_logs` は `Vec` へ push するだけで並べ替えない）。
fn index_of(events: &[LogEvent], needle: &str) -> usize {
    let hits: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.message().contains(needle))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "`{needle}` を含むログがちょうど 1 件ではない: {events:?}"
    );
    hits[0]
}

/// 探針の自己検査: 解放位置が本当にはみ出していて、補正後は本当に収まっていること。
///
/// これが崩れると `limit_correction` が `None` を返し、以降の全檻が「補正が起きなかった
/// こと」を「補正が正しかったこと」と取り違える。
#[test]
fn probe_self_check_release_positions_are_really_outside_and_inside() {
    assert!(
        !contained(release_outside(), balloon_size(), wa()),
        "探針: 画面外解放の位置が作業領域内に収まっている（檻が空虚になる）"
    );
    assert!(
        contained(release_outside_corrected(), balloon_size(), wa()),
        "探針: 補正後の期待値が作業領域内に収まっていない（期待値の算出が誤り）"
    );
    assert!(
        contained(release_inside(), balloon_size(), wa()),
        "探針: 域内解放の位置がはみ出している（無補正の檻が空虚になる）"
    );
    // 順序固定の弁別力: 生 offset と「補正位置から導いた offset」が別物であること。
    assert_ne!(
        raw_offset(release_outside()),
        raw_offset(release_outside_corrected()),
        "探針: 生 offset と補正由来 offset が同値（焼き付きを検出できない）"
    );
}

// -------------------------------------------------------------------------
// (1) ドラッグ中は一切介入しない（要件 2.5 前段）
// -------------------------------------------------------------------------

/// ドラッグ**中**は limit 関門が働かない——窓は wndproc が置いた画面外位置のまま動かず、
/// `BalloonFollow.offset` は生の差分へ更新され、補正ログも窓移動レコードも出ない。
///
/// 解放時補正の檻と対を成す: 「解放でだけ引き戻る」ことは、ドラッグ中に引き戻らない
/// ことの証明とセットでなければ「常に引き戻っている」と区別できない。
#[test]
fn the_drag_itself_is_never_intercepted_by_the_limit_gate() {
    let (mut world, char_window, balloon) = base_world(true);
    let dragged_to = release_outside();
    wndproc_moved_balloon_to(&mut world, balloon, dragged_to);

    let (consumed, events) = capture_logs(|| {
        on_balloon_drag(
            &mut world,
            balloon,
            balloon,
            &Phase::Bubble(drag_event_at(balloon, (0, 0), (10, 10))),
        )
    });

    assert!(!consumed, "ドラッグイベントは消費しない（伝播続行の規約）");
    assert_eq!(
        point_of(&world, balloon),
        dragged_to,
        "ドラッグ中に窓位置が引き戻された（要件 2.5 前段違反）"
    );
    assert_eq!(
        world
            .get::<BalloonFollow>(char_window)
            .expect("spawn が BalloonFollow を付ける")
            .offset,
        raw_offset(dragged_to),
        "ドラッグ中の offset 記憶が生の差分でない"
    );
    assert_no_event(&events, LIMIT_TAG_PREFIX);
    assert_no_event(&events, WINDOW_MOVE_TAG);
}

// -------------------------------------------------------------------------
// (2)(4) 画面外解放——保存は生値・表示だけ補正・順序固定・route は BalloonLimitRelease
// -------------------------------------------------------------------------

/// 画面外で解放すると、**永続へ書かれるのは生 offset**・表示位置だけが補正される
/// （要件 2.5／5.4・DD6）。実 publisher（`spawn_sylphya` ＋ `SharedFakeIo`）で
/// barrier→`load_scope` し、保存値が生値であることを実 IO 越しに固定する。
///
/// 順序（永続 write-through → 表示補正）は 3 通りで噛む:
///   - 保存された `BalloonOffset` が生 offset（補正由来 offset とは別値）
///   - `BalloonFollow.offset` が仕込んだ stale 値のまま（補正が焼き付いていない）
///   - 捕捉ログの**出現順**が「保存 → 補正」（`index_of` は捕捉順の添字を返す）
#[test]
fn releasing_outside_the_work_area_persists_the_raw_offset_and_corrects_only_the_display() {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use areka_sylphya::persist::{FakePersistIo, PersistIo};
    use areka_sylphya::{
        Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, load_scope, spawn_sylphya,
    };

    use super::super::persist::PersistWiring;

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

    let (mut world, char_window, balloon) = base_world(true);
    world.insert_non_send_resource(PersistWiring {
        publisher: parts.publisher.clone(),
    });
    let released_at = release_outside();
    wndproc_moved_balloon_to(&mut world, balloon, released_at);
    set_stale_offset(&mut world, char_window);

    let expected_raw = raw_offset(released_at);
    let corrected = release_outside_corrected();

    let (consumed, events) = capture_logs(|| {
        on_balloon_drag_end(
            &mut world,
            balloon,
            balloon,
            &Phase::Bubble(drag_end_event_at(balloon, (0, 0))),
        )
    });

    assert!(!consumed, "DragEnd イベントは消費しない（伝播続行の規約）");

    // --- 表示位置だけが補正される ---
    assert_eq!(
        point_of(&world, balloon),
        corrected,
        "解放位置が作業領域内へ補正されていない（要件 2.5 後段）"
    );
    assert!(contained(point_of(&world, balloon), balloon_size(), wa()));

    // --- in-session の相対位置には焼き付かない（DD6） ---
    assert_eq!(
        world
            .get::<BalloonFollow>(char_window)
            .expect("spawn が BalloonFollow を付ける")
            .offset,
        stale_offset(),
        "解放時補正が BalloonFollow.offset を書き換えた（補正の焼き付き・DD6 違反）"
    );

    // --- キャラ窓は動かない（要件 2.8） ---
    assert_eq!(point_of(&world, char_window), char_pos());

    // --- 補正ログ（要件 6.1・DD7） ---
    let clamp = expect_one(&events, LIMIT_CLAMP_TAG);
    assert_eq!(clamp.level, tracing::Level::INFO, "補正の記録は info 水準");
    assert_eq!(clamp.field("scope"), "0");
    assert_eq!(
        clamp.field("context"),
        format!("{RELEASE_CONTEXT:?}"),
        "3 関門を弁別する契機ラベルが解放時補正のものでない"
    );
    assert_eq!(
        clamp.field("route"),
        format!("{:?}", PlacementRoute::BalloonLimitRelease),
        "解放時の補正が専用 route を名乗っていない（DD7）"
    );
    assert_eq!(clamp.field("from"), format!("{released_at:?}"));
    assert_eq!(clamp.field("to"), format!("{corrected:?}"));

    // --- 窓移動レコードは補正後位置・route は BalloonLimitRelease（DD7） ---
    let record = expect_one(&events, WINDOW_MOVE_TAG);
    let line = record.message();
    assert!(
        line.contains(&format!("x={} y={}", corrected.x, corrected.y)),
        "窓移動レコードが補正後の最終位置を記録していない: {line}"
    );
    assert!(
        line.contains(&format!(
            "route={}",
            PlacementRoute::BalloonLimitRelease.as_str()
        )),
        "解放時の新規書込が BalloonLimitRelease を名乗っていない: {line}"
    );

    // --- 順序固定（要件 5.4・DD6）: 永続 write-through が補正書込より先 ---
    assert!(
        index_of(&events, PERSIST_SAVE_TAG) < index_of(&events, LIMIT_CLAMP_TAG),
        "永続 write-through より先に補正が走っている（順序が逆転すると保存値へ焼き付く）"
    );
    assert!(
        index_of(&events, PERSIST_SAVE_TAG) < index_of(&events, WINDOW_MOVE_TAG),
        "補正の窓書込が永続 write-through より先に走っている（順序固定の違反）"
    );

    // --- 保存ログが記録した値も生値（実 IO 観測の前段確認） ---
    let save = expect_one(&events, PERSIST_SAVE_TAG);
    assert_eq!(save.field("persist_x"), expected_raw.x.to_string());
    assert_eq!(save.field("persist_y"), expected_raw.y.to_string());

    // --- 実 IO 越しの保存値が生 offset であること ---
    parts
        .publisher
        .barrier()
        .expect("barrier should resolve while actor is alive");
    let loaded = load_scope(PersistScope::Ghost, &roots, &SharedFakeIo(shared.clone()));
    for (axis, value) in [
        (Axis::X, expected_raw.x),
        (Axis::Y, expected_raw.y),
    ] {
        assert!(
            loaded.contains(&(
                PersistKey::BalloonOffset { scope: 0, axis },
                value.to_string()
            )),
            "保存値が解放位置由来の生 offset でない（{axis:?}={value} が無い）: {loaded:?}"
        );
    }
    // 補正由来 offset が保存されていないこと（焼き付きの直接否定）。
    let baked = raw_offset(corrected);
    for (axis, value) in [(Axis::X, baked.x), (Axis::Y, baked.y)] {
        assert!(
            !loaded.contains(&(
                PersistKey::BalloonOffset { scope: 0, axis },
                value.to_string()
            )),
            "補正後位置から導いた offset が保存されている（DD6 違反・{axis:?}={value}）: {loaded:?}"
        );
    }

    // 正典終了（アクター join）——テスト後始末（リーク回避・非本質）。
    parts.publisher.close();
    let _ = parts.handle.join();
}

// -------------------------------------------------------------------------
// (3) 作業領域内で解放したら補正の書き込みは 1 件も起きない
// -------------------------------------------------------------------------

/// 作業領域内で解放したときは補正の書き込みが発生しない（enqueue 記録 0 件）。
///
/// 「位置が変わらないこと」だけでは、補正が走って同じ値を書き直した場合と区別できない
/// ——窓移動レコード（`enqueue_window_set_pos` が書込成功時に出す）が 1 件も無いことで
/// 「そもそも書いていない」を固定する。
#[test]
fn releasing_inside_the_work_area_writes_nothing() {
    let (mut world, char_window, balloon) = base_world(true);
    let released_at = release_inside();
    wndproc_moved_balloon_to(&mut world, balloon, released_at);
    set_stale_offset(&mut world, char_window);

    let (_, events) = capture_logs(|| {
        on_balloon_drag_end(
            &mut world,
            balloon,
            balloon,
            &Phase::Bubble(drag_end_event_at(balloon, (0, 0))),
        )
    });

    // 先に捕捉が成立していることを確かめてから「無いこと」を主張する。
    let save = expect_one(&events, PERSIST_SAVE_TAG);
    assert_eq!(
        save.field("persist_x"),
        raw_offset(released_at).x.to_string(),
        "域内解放でも保存は走る（保存フックは limit と無関係）"
    );
    assert_no_event(&events, LIMIT_TAG_PREFIX);
    assert_no_event(&events, WINDOW_MOVE_TAG);
    assert_eq!(
        point_of(&world, balloon),
        released_at,
        "補正不要のはずが位置が動いている"
    );
}

// -------------------------------------------------------------------------
// (5) limit 無効の scope では解放時補正が起きない（要件 2.7）
// -------------------------------------------------------------------------

/// `BalloonLimit(false)` の scope は画面外で解放してもそのまま（現行挙動の維持）。
#[test]
fn releasing_outside_does_not_correct_when_balloon_limit_is_disabled() {
    let (mut world, char_window, balloon) = base_world(false);
    let released_at = release_outside();
    wndproc_moved_balloon_to(&mut world, balloon, released_at);
    set_stale_offset(&mut world, char_window);

    let (_, events) = capture_logs(|| {
        on_balloon_drag_end(
            &mut world,
            balloon,
            balloon,
            &Phase::Bubble(drag_end_event_at(balloon, (0, 0))),
        )
    });

    expect_one(&events, PERSIST_SAVE_TAG);
    assert_no_event(&events, LIMIT_TAG_PREFIX);
    assert_no_event(&events, WINDOW_MOVE_TAG);
    assert_eq!(
        point_of(&world, balloon),
        released_at,
        "limit=0 のバルーンが解放時に引き戻された（現行挙動が変わっている）"
    );
}

// -------------------------------------------------------------------------
// 縮退（log-first・要件 6.3）——警告を残して補正を見送る
// -------------------------------------------------------------------------

/// 基準・判定寸が解決できない各経路で、警告のうえ補正を見送り**保存は通る**こと。
///
/// 「補正しない」だけでは素通しの証明にならない——保存フック（順序の前段）が最後まで
/// 走り切っていることまで見る。
#[test]
fn unresolvable_reference_warns_and_skips_the_release_correction() {
    let released_at = release_outside();
    let cases: [(&str, fn(&mut World, Entity, Entity)); 4] = [
        ("MonitorSnapshot 不在", |world, _c, _b| {
            let _ = world.remove_resource::<MonitorSnapshot>();
        }),
        ("モニタ 0 台", |world, _c, _b| {
            world.insert_resource(MonitorSnapshot {
                work_areas: Vec::new(),
            });
        }),
        // `WindowPos::default()` の寸センチネル。`Option::Some` ゆえ上流の逆引きは
        // 通り抜け、矩形を組む段で初めて弾かれる（`Option::None` だけを未確定と見る
        // 実装だと逆転矩形のままモニタ帰属が意味を失う）。
        ("キャラ窓の寸がセンチネル", |world, char_window, _b| {
            let mut wp = *world.get::<WindowPos>(char_window).expect("WindowPos");
            wp.size = Some(SizeI::new(CW_USEDEFAULT, CW_USEDEFAULT));
            world.entity_mut(char_window).insert(wp);
        }),
        ("バルーンの寸が未確定", |world, _c, balloon| {
            let mut wp = *world.get::<WindowPos>(balloon).expect("WindowPos");
            wp.size = None;
            world.entity_mut(balloon).insert(wp);
        }),
    ];

    for (label, break_it) in cases {
        let (mut world, char_window, balloon) = base_world(true);
        wndproc_moved_balloon_to(&mut world, balloon, released_at);
        break_it(&mut world, char_window, balloon);

        let (_, events) = capture_logs(|| {
            on_balloon_drag_end(
                &mut world,
                balloon,
                balloon,
                &Phase::Bubble(drag_end_event_at(balloon, (0, 0))),
            )
        });

        // 先に捕捉が成立していることを確かめてから「無いこと」を主張する。
        expect_one(&events, PERSIST_SAVE_TAG);
        let warn = expect_one(&events, LIMIT_UNRESOLVED_TAG);
        assert_eq!(warn.level, tracing::Level::WARN, "{label}: 縮退は warn 水準");
        assert_eq!(
            warn.field("context"),
            format!("{RELEASE_CONTEXT:?}"),
            "{label}: 契機ラベルが解放時補正のものでない"
        );
        assert_no_event(&events, LIMIT_CLAMP_TAG);
        assert_no_event(&events, WINDOW_MOVE_TAG);
        assert_eq!(
            point_of(&world, balloon),
            released_at,
            "{label}: 補正を見送るはずが位置が変わっている"
        );
    }
}

// -------------------------------------------------------------------------
// 基準はキャラ窓の帰属モニタ（要件 5.5）
// -------------------------------------------------------------------------

/// 複数モニタで、キャラ窓が属する側の作業領域が基準になること。
///
/// 解放位置の中心は**右**モニタに属する一方、キャラ窓は左モニタに居る。どちらを基準に
/// したかが最終位置 1 つで判別できる配置にしてある。
#[test]
fn the_release_correction_uses_the_work_area_of_the_char_window_monitor() {
    let left_wa = rect(-1920, -40, 0, 1000);
    let mut placement = base_placement(true);
    placement.char_pos = PointPx { x: -800, y: 300 };
    placement.balloon_pos = PointPx { x: -500, y: 700 };
    placement.balloon_offset = PointPx { x: 300, y: 400 };
    let (mut world, _char_window, balloon) = world_with(
        placement,
        MonitorSnapshot {
            work_areas: vec![left_wa, wa()],
        },
    );
    let released_at = PointPx { x: 100, y: 950 };
    wndproc_moved_balloon_to(&mut world, balloon, released_at);

    // 探針: 解放位置の中心は右モニタに属する（基準を取り違えれば結果が変わる配置）。
    let center = PointPx {
        x: released_at.x + balloon_size().w / 2,
        y: released_at.y + balloon_size().h / 2,
    };
    let right = wa();
    assert!(
        center.x >= right.left
            && center.x < right.right
            && center.y >= right.top
            && center.y < right.bottom,
        "探針: 解放位置の中心が右モニタに属していない（檻が弁別力を失う）"
    );

    on_balloon_drag_end(
        &mut world,
        balloon,
        balloon,
        &Phase::Bubble(drag_end_event_at(balloon, (0, 0))),
    );

    assert_eq!(
        point_of(&world, balloon),
        max_pos(left_wa, balloon_size()),
        "キャラ窓が属する左モニタではなくバルーン側のモニタを基準にしている（要件 5.5 違反）"
    );
}

// -------------------------------------------------------------------------
// runtime 関門との二重適用が無害であること（DD6 の「クランプは冪等」）
// -------------------------------------------------------------------------

/// 解放時補正が渡した補正後位置は、単一ライター内の runtime 関門でもう一度クランプ
/// されるが、クランプは冪等ゆえ結果が変わらず補正ログも二重にならない。
///
/// `expect_one`（ちょうど 1 件）が二重ログを直接禁じ、`GhostWindows` が居る実 World で
/// あることが「runtime 関門が実際に働ける状態で通した」ことを担保する。
#[test]
fn the_release_correction_is_idempotent_through_the_runtime_gate() {
    let (mut world, _char_window, balloon) = base_world(true);
    assert!(
        world.get_resource::<GhostWindows>().is_some(),
        "探針: runtime 関門が基準を引ける状態でなければ二重適用を検査できない"
    );
    wndproc_moved_balloon_to(&mut world, balloon, release_outside());

    let (_, events) = capture_logs(|| {
        on_balloon_drag_end(
            &mut world,
            balloon,
            balloon,
            &Phase::Bubble(drag_end_event_at(balloon, (0, 0))),
        )
    });

    let clamp = expect_one(&events, LIMIT_CLAMP_TAG);
    assert_eq!(
        clamp.field("context"),
        format!("{RELEASE_CONTEXT:?}"),
        "runtime 関門が二度目の補正ログを出している（冪等でない）"
    );
    assert_eq!(point_of(&world, balloon), release_outside_corrected());
}
