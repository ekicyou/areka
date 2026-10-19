//! 鎖の適用系の 1 巡の**組の外しと結び直し**——替え玉で窓口の呼び出し列を固定する
//! （`areka-P0-zorder-scope-inversion` 要件 1.5・6.1・6.2・6.4・6.5・7.1・7.2・7.3）。
//!
//! 窓口の替え玉は [`double`](super::double) をそのまま使う（中身は変えない）。檻は兄弟の
//! `zorder_chain_apply_tests.rs` と同じく単一スレッドの実行器を明示する——記録の捕捉も
//! 台本もスレッドローカルであり、多スレッドの実行器では検査が空虚に緑になる。
//!
//! 組が 1 つも外れず結ばれもしない巡の呼び出し列が今と同じであることは、既存の替え玉の
//! 試験（`zorder_chain_apply_tests.rs` ほか）が書き換えなしで緑であることが示す。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};

use super::apply_zorder_chain;
use super::double::{Call, Script, with_script};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{
    ChainPlan, ChainSegment, CrossEdge, CrossOwnerLink, KeepDirectlyAbove, OwnerLink, PairReleased,
    WindowHandle, ZOrderChainPlan,
};

/// 鎖の記録と、組の見送りの記録（`[zorder-pair] skip`）の両方を拾う。
const DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_chain=debug,wintf::ecs::window::zorder_pair=debug";

const PAIR_RELEASED: &str = "[zorder-chain] pair-released";
const PAIR_RESTORED: &str = "[zorder-chain] pair-restored";
const LINK_FAILED: &str = "[zorder-chain] link-failed";
const PAIR_SKIP: &str = "[zorder-pair] skip";

fn h(v: usize) -> HWND {
    HWND(v as *mut _)
}

fn handle(v: usize) -> WindowHandle {
    WindowHandle {
        hwnd: h(v),
        instance: HINSTANCE::default(),
    }
}

/// 1 巡回し、捕捉した記録と窓口の呼び出しの記録を返す。
fn run_once(world: &mut World, script: Script) -> (String, Script) {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(apply_zorder_chain);
    with_script(script, || {
        capture_under_filter(DIRECTIVES, || schedule.run(world))
    })
}

fn publish(world: &mut World, chain: Option<ChainPlan>) {
    world.insert_resource(ZOrderChainPlan { chain, dirty: true });
}

fn chain(members: Vec<Entity>, cross_edges: Vec<CrossEdge>) -> Option<ChainPlan> {
    Some(ChainPlan {
        members,
        cross_edges,
        absent: Vec::new(),
    })
}

fn edge(owned: Entity, owner: Entity) -> CrossEdge {
    CrossEdge {
        owned,
        owner,
        segment: ChainSegment::Group(0),
    }
}

fn lines<'a>(out: &'a str, tag: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(tag)).collect()
}

/// 組（バルーン ← キャラクター）が結ばれた 2 枚を作る。
fn linked_pair(world: &mut World, balloon_hwnd: usize, peer_hwnd: usize) -> (Entity, Entity) {
    let peer = world.spawn(handle(peer_hwnd)).id();
    let balloon = world
        .spawn((
            handle(balloon_hwnd),
            KeepDirectlyAbove { peer },
            OwnerLink {
                owner_hwnd: h(peer_hwnd),
            },
        ))
        .id();
    (balloon, peer)
}

// ===========================================================================
// 組 → 逆転（s0,b0,b1,s1）
// ===========================================================================

/// 組の重なり（`b0,s0,b1,s1`・鎖の繋ぎは `s0 ← b1`）から逆転 `s0,b0,b1,s1` へ。
/// 呼び出し列は「鎖の撤去 → 組の読み取り・外し → 鎖の付与 → 後押し → 前面走査」で、
/// 結び直しも輪の確かめの読み取りも出ない。
#[test]
fn pair_to_inversion_releases_between_detach_and_attach() {
    let mut world = World::new();
    let (b0, s0) = linked_pair(&mut world, 10, 11);
    let (b1, s1) = linked_pair(&mut world, 20, 21);
    world.entity_mut(s0).insert(CrossOwnerLink {
        owner: b1,
        owned_hwnd: h(11),
        owner_hwnd: h(20),
        segment: ChainSegment::Group(0),
    });
    publish(
        &mut world,
        chain(vec![s0, b0, b1, s1], vec![edge(s0, b0), edge(b0, b1)]),
    );
    let mut script = Script::default();
    script.owner_of.extend([(10, 11), (20, 21), (11, 20)]);

    let (out, used) = run_once(&mut world, script);

    assert_eq!(
        used.calls[..6],
        [
            Call::ReadOwner(11),
            Call::ClearOwner(11),
            Call::ReadOwner(10),
            Call::ClearOwner(10),
            Call::SetOwner(11, 10),
            Call::SetOwner(10, 20),
        ],
        "撤去 → 組の外し → 付与の順でない: {:?}",
        used.calls
    );
    assert!(
        matches!(
            used.calls[6..],
            [Call::RawNext(_), Call::Nudge(..), Call::MeasureFront(_)]
        ),
        "付与の後に後押しと前面走査が 1 回ずつ続かない: {:?}",
        used.calls
    );
    // 一直線の持ち主の関係（s0 → b0 → b1 → s1）。
    assert_eq!(used.owner_of.get(&11), Some(&10));
    assert_eq!(used.owner_of.get(&10), Some(&20));
    assert_eq!(used.owner_of.get(&20), Some(&21));

    assert!(
        world.get::<PairReleased>(b0).is_some(),
        "外した組に印が無い"
    );
    assert!(world.get::<OwnerLink>(b0).is_none(), "外した組に帳簿が残る");
    assert!(
        world.get::<PairReleased>(b1).is_none(),
        "連続する組まで外した"
    );
    assert!(world.get::<OwnerLink>(b1).is_some());

    let released = lines(&out, PAIR_RELEASED);
    assert_eq!(released.len(), 1, "{out}");
    assert!(released[0].contains(&format!("entity={b0:?}")), "{out}");
    assert!(released[0].contains("unlinked=true"), "{out}");
    assert!(lines(&out, PAIR_RESTORED).is_empty(), "{out}");
}

// ===========================================================================
// 結び直しの 5 通り（鎖を解く公開で印の付いた組を結び直す）
// ===========================================================================

/// 印の付いた組（結び目なし）を作る。
fn released_pair(world: &mut World, balloon_hwnd: usize, peer: Entity) -> Entity {
    world
        .spawn((
            handle(balloon_hwnd),
            KeepDirectlyAbove { peer },
            PairReleased,
        ))
        .id()
}

/// ⑴ 相手の実体が無い: 窓口を呼ばず `PeerMissing` を出し、印だけ外す。
#[test]
fn restore_with_missing_peer_only_drops_the_mark() {
    let mut world = World::new();
    let s0 = world.spawn(handle(11)).id();
    let b0 = released_pair(&mut world, 10, s0);
    world.despawn(s0);
    publish(&mut world, None);

    let (out, used) = run_once(&mut world, Script::default());

    assert!(used.calls.is_empty(), "窓口を呼んだ: {:?}", used.calls);
    let skip = lines(&out, PAIR_SKIP);
    assert_eq!(skip.len(), 1, "{out}");
    assert!(skip[0].contains("reason=PeerMissing"), "{out}");
    assert!(world.get::<PairReleased>(b0).is_none());
    assert!(world.get::<OwnerLink>(b0).is_none());
}

/// ⑵ どちらかのハンドルが無い: 窓口を呼ばず `HandleMissing` を出し、印だけ外す
/// （ハンドルが付いた巡に確立系が張る）。
#[test]
fn restore_with_missing_handle_only_drops_the_mark() {
    let mut world = World::new();
    let s0 = world.spawn_empty().id();
    let b0 = released_pair(&mut world, 10, s0);
    publish(&mut world, None);

    let (out, used) = run_once(&mut world, Script::default());

    assert!(used.calls.is_empty(), "窓口を呼んだ: {:?}", used.calls);
    let skip = lines(&out, PAIR_SKIP);
    assert_eq!(skip.len(), 1, "{out}");
    assert!(skip[0].contains("reason=HandleMissing"), "{out}");
    assert!(world.get::<PairReleased>(b0).is_none());
    assert!(world.get::<OwnerLink>(b0).is_none());
}

/// ⑶ 同じ巡に確立系が既に張っている: 二重に張らず、印だけ外す。
#[test]
fn restore_after_same_round_establishment_only_drops_the_mark() {
    let mut world = World::new();
    let s0 = world.spawn(handle(11)).id();
    let b0 = released_pair(&mut world, 10, s0);
    world.entity_mut(b0).insert(OwnerLink { owner_hwnd: h(11) });
    publish(&mut world, None);

    let (out, used) = run_once(&mut world, Script::default());

    assert!(used.calls.is_empty(), "二重に張った: {:?}", used.calls);
    assert!(lines(&out, PAIR_RESTORED).is_empty(), "{out}");
    assert!(world.get::<PairReleased>(b0).is_none());
    assert_eq!(
        world.get::<OwnerLink>(b0),
        Some(&OwnerLink { owner_hwnd: h(11) })
    );
}

/// ⑷ 張れた: `バルーン ← キャラクター` を張り、帳簿を付けて `pair-restored`。
#[test]
fn restore_links_the_pair_and_records_it() {
    let mut world = World::new();
    let s0 = world.spawn(handle(11)).id();
    let b0 = released_pair(&mut world, 10, s0);
    publish(&mut world, None);

    let (out, used) = run_once(&mut world, Script::default());

    assert_eq!(used.calls.first(), Some(&Call::SetOwner(10, 11)));
    assert_eq!(
        used.calls
            .iter()
            .filter(|c| matches!(c, Call::SetOwner(..)))
            .count(),
        1
    );
    assert_eq!(used.owner_of.get(&10), Some(&11));
    assert!(world.get::<PairReleased>(b0).is_none());
    assert_eq!(
        world.get::<OwnerLink>(b0),
        Some(&OwnerLink { owner_hwnd: h(11) })
    );
    let restored = lines(&out, PAIR_RESTORED);
    assert_eq!(restored.len(), 1, "{out}");
    assert!(
        restored[0].contains(&format!("entity={b0:?} peer={s0:?}")),
        "{out}"
    );
}

/// ⑸ 張れない: `link-failed segment=-` を記録して印を外し、残りの組は続ける。
#[test]
fn restore_failure_is_recorded_and_the_rest_continue() {
    let mut world = World::new();
    let s0 = world.spawn(handle(11)).id();
    let s1 = world.spawn(handle(21)).id();
    // 操作列はバルーンの `Entity` の昇順——作った順と限らないので並べ替えてから割り当てる。
    let mut balloons = [world.spawn_empty().id(), world.spawn_empty().id()];
    balloons.sort();
    let [first, second] = balloons;
    world
        .entity_mut(first)
        .insert((handle(10), KeepDirectlyAbove { peer: s0 }, PairReleased));
    world
        .entity_mut(second)
        .insert((handle(20), KeepDirectlyAbove { peer: s1 }, PairReleased));
    publish(&mut world, None);
    let mut script = Script::default();
    script.set_fails.insert(10);

    let (out, used) = run_once(&mut world, script);

    let sets: Vec<&Call> = used
        .calls
        .iter()
        .filter(|c| matches!(c, Call::SetOwner(..)))
        .collect();
    assert_eq!(sets, [&Call::SetOwner(10, 11), &Call::SetOwner(20, 21)]);
    let failed = lines(&out, LINK_FAILED);
    assert_eq!(failed.len(), 1, "{out}");
    assert!(failed[0].contains("segment=-"), "{out}");
    assert_eq!(lines(&out, PAIR_RESTORED).len(), 1, "{out}");
    assert!(world.get::<PairReleased>(first).is_none());
    assert!(world.get::<OwnerLink>(first).is_none());
    assert!(world.get::<PairReleased>(second).is_none());
    assert!(world.get::<OwnerLink>(second).is_some());
}

// ===========================================================================
// ハンドル待ちの持ち越し
// ===========================================================================

/// 組の外しが窓口を呼んでも、鎖の付与が 1 本も書けなければ持ち越しを続ける。
/// 印は持ち越しの巡でも先に付き、ハンドルが揃った巡で付与が張られる（要件 6.1）。
#[test]
fn handle_wait_carries_over_even_when_the_pair_release_called_the_os() {
    let mut world = World::new();
    // キャラクターの窓は作り直しの最中でハンドルが無い（帳簿は前の窓を控えている）。
    let s0 = world.spawn_empty().id();
    let b0 = world
        .spawn((
            handle(10),
            KeepDirectlyAbove { peer: s0 },
            OwnerLink { owner_hwnd: h(11) },
        ))
        .id();
    publish(&mut world, chain(vec![s0, b0], vec![edge(s0, b0)]));
    let mut script = Script::default();
    script.owner_of.insert(10, 11);

    let (_, used) = run_once(&mut world, script);

    assert_eq!(used.calls, [Call::ReadOwner(10), Call::ClearOwner(10)]);
    assert!(
        world.get::<PairReleased>(b0).is_some(),
        "印が先に付いていない"
    );
    assert!(
        world.resource::<ZOrderChainPlan>().dirty,
        "組の操作の分で持ち越しが解けた"
    );

    // ハンドルが付いた巡: 組の操作は無く、鎖の付与だけが張られる。
    world.entity_mut(s0).insert(handle(11));
    let (_, used) = run_once(&mut world, Script::default());

    assert_eq!(used.calls.first(), Some(&Call::SetOwner(11, 10)));
    assert!(
        !used
            .calls
            .iter()
            .any(|c| matches!(c, Call::ReadOwner(10) | Call::ClearOwner(10))),
        "外した組を 2 度外した: {:?}",
        used.calls
    );
    assert!(!world.resource::<ZOrderChainPlan>().dirty);
    assert!(world.get::<PairReleased>(b0).is_some());
}

// ===========================================================================
// 解除の後押しの出し分け（要件 4.5・7.3）
// ===========================================================================

const SETTLED: &str = "[zorder-chain] settled";

/// 非隣接 `b1,s0,s1,b0`（組は 2 つとも外れ、鎖の繋ぎは `s0 ← b1`・`s1 ← s0`・`b0 ← s1`）から
/// 解除（鎖 `None`）。呼び出し列は「鎖の撤去 → 組の張り直し → 組ごとの後押し → 前面走査」で、
/// 後押しは結び直した組ごとに `[バルーン, キャラクター]` へ 1 回ずつ、`settled` も 1 行ずつ。
#[test]
fn reset_to_none_nudges_each_restored_pair() {
    let mut world = World::new();
    let s0 = world.spawn(handle(11)).id();
    let s1 = world.spawn(handle(21)).id();
    let mut balloons = [world.spawn_empty().id(), world.spawn_empty().id()];
    balloons.sort();
    let [b0, b1] = balloons;
    world
        .entity_mut(b0)
        .insert((handle(10), KeepDirectlyAbove { peer: s0 }, PairReleased));
    world
        .entity_mut(b1)
        .insert((handle(20), KeepDirectlyAbove { peer: s1 }, PairReleased));
    for (owned, owner, owned_hwnd, owner_hwnd) in
        [(s0, b1, 11, 20), (s1, s0, 21, 11), (b0, s1, 10, 21)]
    {
        world.entity_mut(owned).insert(CrossOwnerLink {
            owner,
            owned_hwnd: h(owned_hwnd),
            owner_hwnd: h(owner_hwnd),
            segment: ChainSegment::Group(0),
        });
    }
    publish(&mut world, None);
    let mut script = Script::default();
    script.owner_of.extend([(11, 20), (21, 11), (10, 21)]);

    let (out, used) = run_once(&mut world, script);

    assert!(
        used.calls[..6]
            .iter()
            .all(|c| matches!(c, Call::ReadOwner(_) | Call::ClearOwner(_))),
        "鎖の撤去が先に済んでいない: {:?}",
        used.calls
    );
    assert_eq!(
        used.calls[6..],
        [
            Call::SetOwner(10, 11),
            Call::SetOwner(20, 21),
            Call::RawNext(10),
            Call::Nudge(11, 10),
            Call::MeasureFront(11),
            Call::RawNext(20),
            Call::Nudge(21, 20),
            Call::MeasureFront(21),
        ],
        "組の張り直し → 組ごとの後押し → 前面走査の順でない: {:?}",
        used.calls
    );
    let settled = lines(&out, SETTLED);
    assert_eq!(settled.len(), 2, "{out}");
    assert!(settled[0].contains("declared=0xA,0xB"), "{out}");
    assert!(settled[1].contains("declared=0x14,0x15"), "{out}");
}

/// 逆転 `s0,b0,b1,s1`（`b0` の組は外れ、繋ぎは `s0 ← b0`・`b0 ← b1`）を作り、descript の
/// 基底 `b0,s0,b1,s1`（繋ぎ `s0 ← b1`）へ戻す公開を出す。`fail_restore` なら結び直しを失敗させる。
fn inversion_back_to_base(fail_restore: bool) -> (String, Script) {
    let mut world = World::new();
    let s0 = world.spawn(handle(11)).id();
    let b0 = released_pair(&mut world, 10, s0);
    let (b1, s1) = linked_pair(&mut world, 20, 21);
    for (owned, owner, owned_hwnd, owner_hwnd) in [(s0, b0, 11, 10), (b0, b1, 10, 20)] {
        world.entity_mut(owned).insert(CrossOwnerLink {
            owner,
            owned_hwnd: h(owned_hwnd),
            owner_hwnd: h(owner_hwnd),
            segment: ChainSegment::Group(0),
        });
    }
    publish(&mut world, chain(vec![b0, s0, b1, s1], vec![edge(s0, b1)]));
    let mut script = Script::default();
    script.owner_of.extend([(11, 10), (10, 20), (20, 21)]);
    if fail_restore {
        script.set_fails.insert(10);
    }
    run_once(&mut world, script)
}

/// 鎖が残る解除: 組の張り直しの後に鎖の付与、そして鎖全体の後押しが 1 回だけ。
#[test]
fn reset_to_descript_base_nudges_the_whole_chain_once() {
    let (out, used) = inversion_back_to_base(false);

    let tail: Vec<&Call> = used
        .calls
        .iter()
        .filter(|c| !matches!(c, Call::ReadOwner(_) | Call::ClearOwner(_)))
        .collect();
    assert!(
        matches!(
            tail[..],
            [
                Call::SetOwner(10, 11),
                Call::SetOwner(11, 20),
                Call::RawNext(_),
                Call::Nudge(..),
                Call::MeasureFront(_),
            ]
        ),
        "組の張り直し → 鎖の付与 → 鎖全体の後押し 1 回の順でない: {:?}",
        used.calls
    );
    assert_eq!(lines(&out, SETTLED).len(), 1, "{out}");
    assert!(
        lines(&out, SETTLED)[0].contains("declared=0xA,0xB,0x14,0x15"),
        "{out}"
    );
}

/// 結び直しの 5 通りの ⑸ の続き: 組が張れなくても、鎖の付与と鎖全体の後押しは続く。
#[test]
fn a_failed_restore_still_lets_the_chain_attach_and_nudge() {
    let (out, used) = inversion_back_to_base(true);

    assert!(
        used.calls.contains(&Call::SetOwner(11, 20)),
        "{:?}",
        used.calls
    );
    assert_eq!(
        used.calls
            .iter()
            .filter(|c| matches!(c, Call::Nudge(..)))
            .count(),
        1,
        "{:?}",
        used.calls
    );
    let failed = lines(&out, LINK_FAILED);
    assert_eq!(failed.len(), 1, "{out}");
    assert!(failed[0].contains("segment=-"), "{out}");
    assert!(lines(&out, PAIR_RESTORED).is_empty(), "{out}");
    assert_eq!(lines(&out, SETTLED).len(), 1, "{out}");
}

// ===========================================================================
// 外し損ねの巡だけの輪の確かめ（要件 7.1・7.2）
// ===========================================================================

const LINK_REFUSED: &str = "[zorder-chain] link-refused";

fn reads(used: &Script) -> Vec<Call> {
    used.calls
        .iter()
        .copied()
        .filter(|c| matches!(c, Call::ReadOwner(_)))
        .collect()
}

/// 逆転 `s0,b0,s1,b1`（組は 2 つとも外れ、帳簿に `s0 ← b0`）を解除する巡。
/// `stuck` なら `s0 ← b0` の撤去を外せなくする。戻り値の末尾は `[b0, b1]`。
fn reset_after_inversion(stuck: bool) -> (String, Script, World, [Entity; 2]) {
    let mut world = World::new();
    let s0 = world.spawn(handle(11)).id();
    let s1 = world.spawn(handle(21)).id();
    let mut balloons = [world.spawn_empty().id(), world.spawn_empty().id()];
    balloons.sort();
    let [b0, b1] = balloons;
    world
        .entity_mut(b0)
        .insert((handle(10), KeepDirectlyAbove { peer: s0 }, PairReleased));
    world
        .entity_mut(b1)
        .insert((handle(20), KeepDirectlyAbove { peer: s1 }, PairReleased));
    world.entity_mut(s0).insert(CrossOwnerLink {
        owner: b0,
        owned_hwnd: h(11),
        owner_hwnd: h(10),
        segment: ChainSegment::Group(0),
    });
    publish(&mut world, None);
    let mut script = Script::default();
    script.owner_of.insert(11, 10);
    if stuck {
        script.clear_fails.insert(11);
    }
    let (out, used) = run_once(&mut world, script);
    (out, used, world, balloons)
}

/// 正常な解除の巡では、確かめの読み取りが 1 回も入らない（読むのは撤去の照合だけ）。
#[test]
fn a_clean_reset_reads_no_owner_before_restoring() {
    let (out, used, _, _) = reset_after_inversion(false);

    assert_eq!(reads(&used), [Call::ReadOwner(11)], "{:?}", used.calls);
    assert!(used.calls.contains(&Call::SetOwner(10, 11)));
    assert!(lines(&out, LINK_REFUSED).is_empty(), "{out}");
}

/// 鎖の撤去が外せずに `s0 ← b0` が残った巡の解除: `b0 ← s0` は輪になるので張らずに
/// `link-refused reason=Loop` を記録して印を外す。残りの組（`b1 ← s1`）は確かめて張る。
#[test]
fn a_stuck_detach_refuses_the_restore_that_would_loop() {
    let (out, used, world, [b0, b1]) = reset_after_inversion(true);

    assert_eq!(
        reads(&used),
        [
            Call::ReadOwner(11),
            Call::ReadOwner(11),
            Call::ReadOwner(21)
        ],
        "{:?}",
        used.calls
    );
    assert!(
        !used.calls.contains(&Call::SetOwner(10, 11)),
        "輪を張った: {:?}",
        used.calls
    );
    assert!(used.calls.contains(&Call::SetOwner(20, 21)));
    let refused = lines(&out, LINK_REFUSED);
    assert_eq!(refused.len(), 1, "{out}");
    assert!(refused[0].contains(&format!("owned={b0:?}")), "{out}");
    assert!(refused[0].contains("reason=Loop"), "{out}");
    assert!(world.get::<PairReleased>(b0).is_none(), "印が残った");
    assert!(
        world.get::<OwnerLink>(b0).is_none(),
        "見送った組に帳簿が付いた"
    );
    assert!(world.get::<OwnerLink>(b1).is_some());
}

/// 鎖 `a,b,c,d`（繋ぎ `a ← b`・`b ← c`・`c ← d`）への組み替えで、帳簿の `a ← c` を外す巡。
/// `diverged` なら `a` の今の持ち主を帳簿と食い違わせる。`c` の持ち主は読めない。
fn relink_chain(diverged: bool) -> (String, Script) {
    let mut world = World::new();
    let [a, b, c, d] = [1, 2, 3, 4].map(|v| world.spawn(handle(v)).id());
    world.entity_mut(a).insert(CrossOwnerLink {
        owner: c,
        owned_hwnd: h(1),
        owner_hwnd: h(3),
        segment: ChainSegment::Group(0),
    });
    publish(
        &mut world,
        chain(vec![a, b, c, d], vec![edge(a, b), edge(b, c), edge(c, d)]),
    );
    let mut script = Script::default();
    script.owner_of.insert(1, if diverged { 99 } else { 3 });
    script.read_fails.insert(3);
    run_once(&mut world, script)
}

/// 正常な巡の鎖の付与には確かめの読み取りが入らない（読めない窓が在っても全部張る）。
#[test]
fn a_clean_round_attaches_without_owner_reads() {
    let (out, used) = relink_chain(false);

    assert_eq!(reads(&used), [Call::ReadOwner(1)], "{:?}", used.calls);
    for set in [
        Call::SetOwner(1, 2),
        Call::SetOwner(2, 3),
        Call::SetOwner(3, 4),
    ] {
        assert!(used.calls.contains(&set), "{set:?}: {:?}", used.calls);
    }
    assert!(lines(&out, LINK_REFUSED).is_empty(), "{out}");
}

/// 食い違いの巡は各付与の直前に確かめる。辿れない `b ← c` だけを `reason=Unreadable` で
/// 見送り、残りは張る。
#[test]
fn a_diverged_round_checks_each_attach_and_refuses_the_unreadable_one() {
    let (out, used) = relink_chain(true);

    let checks = reads(&used);
    for read in [Call::ReadOwner(2), Call::ReadOwner(3), Call::ReadOwner(4)] {
        assert!(checks.contains(&read), "{read:?}: {:?}", used.calls);
    }
    assert!(used.calls.contains(&Call::SetOwner(1, 2)));
    assert!(used.calls.contains(&Call::SetOwner(3, 4)));
    assert!(
        !used.calls.contains(&Call::SetOwner(2, 3)),
        "辿れない付与を張った: {:?}",
        used.calls
    );
    let refused = lines(&out, LINK_REFUSED);
    assert_eq!(refused.len(), 1, "{out}");
    assert!(refused[0].contains("reason=Unreadable"), "{out}");
}

// 外し損ねの 3 つの源（去る窓の切離し・鎖の撤去・組の外し）は、どれか 1 つだけでも確かめを
// 入れる。鎖の撤去の源は上の 2 項が受け持ち、残りの 2 つの源を 1 項ずつ固定する。

/// 去る窓の切離しだけが外せなかった巡: 鎖の付与 `a ← b` の直前に確かめの読み取りが入る。
#[test]
fn a_failed_departing_detach_alone_checks_the_attach() {
    let mut world = World::new();
    let gone = world.spawn_empty().id();
    world.spawn((
        handle(5),
        CrossOwnerLink {
            owner: gone,
            owned_hwnd: h(5),
            owner_hwnd: h(6),
            segment: ChainSegment::Group(0),
        },
    ));
    let [a, b] = [1, 2].map(|v| world.spawn(handle(v)).id());
    publish(&mut world, chain(vec![a, b], vec![edge(a, b)]));
    let mut script = Script::default();
    script.owner_of.insert(5, 6);
    script.clear_fails.insert(5);

    let (_, used) = run_once(&mut world, script);

    assert_eq!(
        reads(&used),
        [Call::ReadOwner(5), Call::ReadOwner(2)],
        "{:?}",
        used.calls
    );
    let at = |c: Call| used.calls.iter().position(|x| *x == c);
    assert!(
        at(Call::ReadOwner(2)) < at(Call::SetOwner(1, 2)),
        "{:?}",
        used.calls
    );
}

/// 組の外しだけが食い違った巡: 鎖の付与 `s0 ← b0` の直前に確かめの読み取りが入る。
#[test]
fn a_diverged_pair_release_alone_checks_the_attach() {
    let mut world = World::new();
    let (b0, s0) = linked_pair(&mut world, 10, 11);
    publish(&mut world, chain(vec![s0, b0], vec![edge(s0, b0)]));
    let mut script = Script::default();
    script.owner_of.insert(10, 99);

    let (_, used) = run_once(&mut world, script);

    assert_eq!(
        reads(&used),
        [
            Call::ReadOwner(10),
            Call::ReadOwner(10),
            Call::ReadOwner(99)
        ],
        "{:?}",
        used.calls
    );
    let at = |c: Call| used.calls.iter().position(|x| *x == c);
    assert!(
        at(Call::ReadOwner(99)) < at(Call::SetOwner(11, 10)),
        "{:?}",
        used.calls
    );
    assert!(world.get::<PairReleased>(b0).is_some());
}
