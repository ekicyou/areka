//! `app_exit` の決定論テスト（areka-P0-app-lifetime-separation）。
//!
//! 全窓破棄の私有部品 `despawn_app_windows` の 3 本は旧 `main.rs` シームの
//! smoke 用の全窓破棄関数のテストから移した（task 3.1）。判断は不変で、対象の関数名と
//! 打ち切り行の相名（`[quit_app]`）だけを追随させた。

use super::*;

/// 全窓破棄の標的は `With<GhostWindowMarker>`: ゴースト窓だけを despawn し、
/// 無印の窓（`Window` だけを持つ entity）と無関係 entity は残す。
#[test]
fn despawn_app_windows_hits_ghost_windows_only() {
    let mut world = World::new();
    let ghost = world.spawn(GhostWindowMarker).id();
    let ghost2 = world.spawn(GhostWindowMarker).id();
    let plain_window = world.spawn(wintf::ecs::Window::default()).id();
    let other = world.spawn_empty().id();

    let count = despawn_app_windows(&mut world);

    assert_eq!(count, 2, "ゴースト窓の 2 entity を despawn すべき");
    assert!(world.get_entity(ghost).is_err());
    assert!(world.get_entity(ghost2).is_err());
    assert!(
        world.get_entity(plain_window).is_ok(),
        "印の無い窓は標的にしない"
    );
    assert!(world.get_entity(other).is_ok());
}

/// 標的なしの World では 0 を返し何も壊さない（冪等・no-op 安全）。
#[test]
fn despawn_app_windows_empty_world_is_noop() {
    let mut world = World::new();
    let other = world.spawn_empty().id();
    assert_eq!(despawn_app_windows(&mut world), 0);
    assert!(world.get_entity(other).is_ok());
}

/// **Req 6.2/6.3（despawn の呼出点そのもの・task 7.3）**: 標的の一部が**ループ実行中に**
/// 破棄済みへ変わっても、`World::despawn` の `Could not despawn entity`（`bevy_ecs::world`
/// の `warn!`）を 1 件も出さず、正常終了系（`debug!`）として打ち切って**残りの標的を
/// 処理し切る**。
///
/// # 探針の作り方（不動点にしないために）
///
/// 「先に despawn しておく」では本条件は作れない——query は生存 entity しか返さないため
/// 標的リストにそもそも載らず、打ち切り経路へ入らない（＝不動点の檻になる）。標的が
/// **ループ中に**破棄済みへ変わる機構は bevy では連鎖 despawn ただ 1 つ（`Children` は
/// `LINKED_SPAWN` の関係対象＝親の despawn が子孫へ再帰する）ゆえ、標的同士を親子で
/// 吊るす。
///
/// ただし**2 段（親・子）では不動点になる**——`add_children` は先に子へ `ChildOf` を
/// 挿してから親へ `Children` を挿すので、子の archetype が先に生まれ、query は子を先に
/// 返す（子を先に消してから親を消す＝連鎖を踏まない）。そこで **root → mid → leaf の
/// 3 段**にする: archetype 生成順は `{marker,ChildOf}`（leaf）→ `{marker,Children}`
/// （root）→ `{marker,ChildOf,Children}`（mid）となり、処理順が **leaf → root → mid**
/// ＝ root の despawn が mid を連鎖破棄した**後**に mid が処理される。この順序前提は
/// テスト内で明示的に自己検査する（bevy 側の順序が変われば檻は緑のまま空虚化せず、
/// 前提 assert が赤くなって気づける）。
///
/// 本番ツリーの窓 entity 同士に現在この連鎖は**無い**（`spawn_ghost_windows` はキャラ窓・
/// バルーン窓を top-level で spawn し、リポジトリ内にカスタム関係型も無い）。それでも
/// 呼出点に存在確認が無いこと自体が構造的な穴であり（3.2 の消費側 4 入口は呼出点を
/// 覆っていない）、本檻はその穴を塞いだことを固定する——将来の到達に対する保険という
/// 位置づけは task 6.3 の 3 檻と同じである。
///
/// # 「警告ゼロ」を tracing 捕捉だけで主張してはならない（本檻の対照アームの理由）
///
/// `bevy_ecs` は **`log` クレート**の `warn!` を使う（`bevy_ecs-0.18.1` の
/// `src/world/mod.rs:71` が `use log::warn;`・`World::despawn` は同 :1462-1469 で
/// 失敗時に `warn!("{error}")`＋`false`）。本番プロセスでこの行が
/// `WARN bevy_ecs::world: Could not despawn entity` として見えるのは
/// `tracing_subscriber` が `log`→`tracing` ブリッジを張るからであって、
/// テストの捕捉ハーネス（[`capture_logs`]＝素の thread-local dispatcher）には
/// **原理的に 1 件も届かない**。ゆえに「捕捉イベントに warn が無い」は bevy の警告に
/// 関しては**恒真**であり、それだけを根拠にすると檻が空虚化する。
///
/// そこで**対照アーム**を置く: 同じ探針 World で存在確認**無し**のループを走らせ、
/// `World::despawn` が `mid` に対して `false` を返すことを実測する。上記の実装から
/// `false` は「`Could not despawn entity` の警告を 1 件出した」と**同値**であり、
/// これが本檻の非空虚性の証明である。捕捉側の `warn` ゼロ主張は areka 自身の出力
/// （`enqueue`/`Arrangement` 等）に対してのみ意味を持つ。
#[test]
fn despawn_app_windows_skips_cascade_despawned_target_without_warning() {
    use crate::placement::test_support::{capture_logs, expect_one};

    /// 探針 World: `root → mid → leaf` の連鎖 ＋ 連鎖に無関係な後続標的 `later`。
    /// 戻り値は `(world, root, mid, leaf, later)`。
    fn probe() -> (World, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        let root = world.spawn(GhostWindowMarker).id();
        let mid = world.spawn(GhostWindowMarker).id();
        let leaf = world.spawn(GhostWindowMarker).id();
        world.entity_mut(root).add_children(&[mid]);
        world.entity_mut(mid).add_children(&[leaf]);
        // `later` は連鎖 3 体より後に生まれる archetype（キャラ窓の印を併せ持つゴースト窓）に置き、
        // 処理順で `mid` より後になるようにする（素の `GhostWindowMarker` だと最初の archetype
        // に入り先頭で処理され、「打ち切りは後続を止めない」の主張が空虚になる）。
        let later = world
            .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
            .id();
        (world, root, mid, leaf, later)
    }

    /// 本体と**同一の query**で標的を集める（順序前提と対照アームの両方が本体と
    /// 同じ列を見ていることを構造で保証する）。
    fn targets_of(world: &mut World) -> Vec<Entity> {
        world
            .query_filtered::<Entity, With<GhostWindowMarker>>()
            .iter(world)
            .collect()
    }

    // ── 対照アーム（非空虚性の証明）: 存在確認**無し**のループは無効 entity を叩く ──
    let (mut world, root, mid, leaf, later) = probe();
    let order = targets_of(&mut world);
    let at = |e: Entity| {
        order
            .iter()
            .position(|x| *x == e)
            .expect("標的として拾われている")
    };
    // 前提（探針が不動点でないことの自己検査）: 連鎖の親 `root` が子孫 `mid` より
    // **先に**処理され、`later` が `mid` より**後**であること。前者が崩れると連鎖破棄を
    // 踏まず、後者が崩れると「打ち切りは後続を止めない」の主張が空虚になる。
    assert!(
        at(root) < at(mid) && at(mid) < at(later),
        "探針前提: 処理順は root → mid → later を満たさねばならない（order={order:?}\
         ・root={root:?} mid={mid:?} leaf={leaf:?} later={later:?}）"
    );
    let failed: Vec<Entity> = order
        .iter()
        .filter(|e| !world.despawn(**e))
        .copied()
        .collect();
    assert_eq!(
        failed,
        vec![mid],
        "対照アーム: 存在確認が無ければ `mid` へ無効 despawn が飛ぶ\
         （＝`Could not despawn entity` の警告 1 件）。ここが空なら本檻は恒真の空虚檻である"
    );

    // ── 本体アーム: 同じ探針で、警告を出さず debug 1 行で打ち切り後続も処理し切る ──
    let (mut world, root, mid, leaf, later) = probe();
    let (count, events) = capture_logs(|| despawn_app_windows(&mut world));

    assert_eq!(
        count, 4,
        "標的として拾った 4 体を報告する（掃除後は 4 体とも消える）"
    );
    assert!(world.get_entity(root).is_err());
    assert!(
        world.get_entity(mid).is_err(),
        "探針前提: root の despawn が mid へ連鎖している"
    );
    assert!(world.get_entity(leaf).is_err());
    assert!(
        world.get_entity(later).is_err(),
        "打ち切りは後続の標的を止めない（Req 6.3「他の scope の処理を継続」）"
    );
    // `tracing::Level` の Ord は ERROR < WARN < INFO < DEBUG < TRACE ゆえ
    // 「INFO より verbose」＝ debug/trace のみ、が静穏性の表現（follow.rs 3.2 檻と同型）。
    // ここが見ているのは areka 自身の出力である（bevy の `log` 経由 warn は対照アーム担当）。
    assert!(
        events.iter().all(|e| e.level > tracing::Level::INFO),
        "破棄済み標的に対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
    );
    let skipped = expect_one(&events, DESPAWNED_SKIP_TAG);
    assert_eq!(
        skipped.level,
        tracing::Level::DEBUG,
        "破棄済みの打ち切りは debug 水準（正常終了系）"
    );
    assert!(
        skipped.message().contains("[quit_app]"),
        "打ち切り行が自分の相を名乗っていない: {:?}",
        skipped.message()
    );
}

/// **要件 3.9・3.10（OS の閉鎖要求・ゴースト窓）**: 送り口付きの World でスコープ 1 の
/// キャラ窓へ受け手を呼ぶと、メニューの「終了」と同じ `CloseReason::User { scope: 1 }` の
/// 終了要求が kanade へちょうど 1 件届き、窓は消えない（閉じるのは終了の握手の完了後）。
/// 対照アーム: 送り口の無い World（kanade 未結線の起動）では窓を閉じて終了を指示する。
///
/// 送り口は `pub(crate)` の構築関数でここに直接組む（`input_events_tests.rs` の
/// `world_with_wiring` は私有で流用できない）。当たり判定は受け手が読まないので代役で足りる。
#[test]
fn ghost_os_close_sends_close_request_with_the_window_scope() {
    use crate::emo2_boot::hit_region::HitRegion;
    use crate::input_events::{MouseWiring, RegionSource};
    use crate::placement::spawn::CharWindowMarker;
    use areka_kanade::{CloseReason, KanadeMsg};
    use std::sync::mpsc;

    fn no_hit(scope: u32, _x: i64, _y: i64) -> HitRegion {
        HitRegion {
            scope,
            region: None,
            surface_point: (0, 0),
        }
    }

    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::new(tx, RegionSource::Mock(no_hit)));
    world.insert_non_send(AppExit::new());
    let window = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 1 }))
        .id();

    on_ghost_os_close(&mut world, window);

    match rx.try_recv().expect("終了要求が 1 件届く") {
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope },
        } => assert_eq!(scope, 1, "窓のスコープがそのまま載る"),
        _ => panic!("CloseRequest{{User}} を期待"),
    }
    assert!(rx.try_recv().is_err(), "終了要求はちょうど 1 件");
    assert!(
        world.get_entity(window).is_ok(),
        "窓は消さない（閉じるのは終了の握手の完了後）"
    );

    assert!(
        !world.non_send::<AppExit>().is_requested(),
        "結線済みでは受け手は終了を指示しない（指示は終了系列の完了後）"
    );

    // 対照アーム（要件 3.10）: 送り口の無い World（kanade 未結線の起動）では、別れの台詞を
    // 流す相手がいないので全窓を閉じて終了を指示する——標準の道具で閉じられるアプリのまま。
    let mut bare = World::new();
    bare.insert_non_send(AppExit::new());
    let window = bare
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 1 }))
        .id();
    on_ghost_os_close(&mut bare, window);
    assert!(bare.get_entity(window).is_err(), "未結線では窓を閉じる");
    assert!(
        bare.non_send::<AppExit>().is_requested(),
        "未結線では終了を指示する"
    );
}
