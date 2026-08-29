//! 鎖の適用系——**偽ハンドルで 1 巡を丸ごと踏む**決定論的テスト
//! （要件 6.3／6.4／7.1／7.2／8.2／8.3／9.1／9.2／9.3／11.1／14.1／14.2／14.5）。
//!
//! 実機も実ディスプレイも要らない。`HWND` は Win32 へ 1 度も渡さない偽の値であり、
//! 実行環境への 5 つの窓口（所有者の読み取り・撤去・付与・後押し・前面走査）は
//! [`double`](super::double) の台本つき替え玉が受け止める。よって
//! 「**何を・どの順で**実行環境へ頼んだか」をそのまま主張できる。
//!
//! # 檻は単一スレッドの実行器を明示する
//!
//! 記録の捕捉ハーネス `capture_under_filter` はスレッドローカルの差し替えであり、
//! 替え玉の台本も同じくスレッドローカルである。Bevy の**既定の実行器は多スレッド**なので、
//! そのまま使うと system が別スレッドで走り、記録が 1 行も拾えないうえ台本にも当たらず、
//! 検査が空虚に緑になる（既知の盲点＝`zorder_pair_establish_tests.rs:142-152`）。
//! よって [`chain_schedule`] は `SingleThreadedExecutor` を明示する。
//! 「出ないこと」を主張する窓には、同じ窓に**確かに出る記録**を併置して本数で固定する。
//!
//! # 「無い」の主張には必ず対照を置く
//!
//! 促しの呼び出しの不在（要件 14.2）はソースの字面で固定するが、走査が壊れていれば
//! 「無い」は恒真になる。よって⑴同じ針が**実際に促している既存経路**では当たること、
//! ⑵**呼ぶ側の実装へ差し替えた本文**では当たること、の 2 つを併置する。

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};

use super::apply_zorder_chain;
use super::double::{self, Call, Script, with_script};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{
    ChainPlan, ChainSegment, CrossEdge, CrossOwnerLink, WindowHandle, ZOrderChainPlan,
};

/// 実機サインオフが用いる `RUST_LOG` の鎖側の指定そのもの。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_chain=debug";

const LINKED: &str = "[zorder-chain] linked";
const UNLINKED: &str = "[zorder-chain] unlinked";
const SETTLED: &str = "[zorder-chain] settled";
const SKIPPED: &str = "[zorder-chain] skipped";
const LINK_FAILED: &str = "[zorder-chain] link-failed";

// ===========================================================================
// 道具立て
// ===========================================================================

/// Win32 へは渡さない偽ハンドル（値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// 適用系だけを載せた 1 巡分の schedule（**単一スレッドの実行器を明示**）。
fn chain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(apply_zorder_chain);
    schedule
}

/// 窓ハンドル付きの entity を作る。
fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

/// 望む繋ぎ 1 本（区間つき）。
fn edge(owned: Entity, owner: Entity, segment: ChainSegment) -> CrossEdge {
    CrossEdge {
        owned,
        owner,
        segment,
    }
}

/// 帳簿（本 spec が張った繋ぎ）を被所有側へ据える。区間は既定でグループ 0。
fn record_link(
    world: &mut World,
    owned: Entity,
    owner: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
) {
    record_link_in(
        world,
        owned,
        owner,
        owned_hwnd,
        owner_hwnd,
        ChainSegment::Group(0),
    );
}

/// 区間を明示して帳簿を据える。
fn record_link_in(
    world: &mut World,
    owned: Entity,
    owner: Entity,
    owned_hwnd: HWND,
    owner_hwnd: HWND,
    segment: ChainSegment,
) {
    world.entity_mut(owned).insert(CrossOwnerLink {
        owner,
        owned_hwnd,
        owner_hwnd,
        segment,
    });
}

/// 望む鎖を公開する（`dirty` を立てる＝内容が変わった巡）。
fn publish(world: &mut World, members: Vec<Entity>, cross_edges: Vec<CrossEdge>) {
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members,
            cross_edges,
            absent: Vec::new(),
        }),
        dirty: true,
    });
}

/// 台本つきで 1 巡回し、捕捉した記録と実行環境の呼び出しの記録を返す。
fn run_once(world: &mut World, script: Script) -> (String, Script) {
    let mut schedule = chain_schedule();
    let (out, used) = with_script(script, || {
        capture_under_filter(SIGNOFF_DIRECTIVES, || {
            schedule.run(world);
        })
    });
    (out, used)
}

/// 指定タグの行をすべて取り出す。
fn lines_with<'a>(out: &'a str, tag: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(tag)).collect()
}

/// 指定タグの行がちょうど `expected` 本であることを固定する。
fn assert_line_count(out: &str, tag: &str, expected: usize, what: &str) {
    let found = lines_with(out, tag);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{tag}` は {expected} 本のはずが {} 本\n---捕捉全文---\n{out}",
        found.len()
    );
}

/// 1 行から `key=value` を切り出す。
fn field<'a>(line: &'a str, key: &str) -> &'a str {
    let needle = format!("{key}=");
    let start = line
        .find(&needle)
        .unwrap_or_else(|| panic!("`{key}=` が行に無い: {line}"))
        + needle.len();
    line[start..].split_whitespace().next().unwrap_or("")
}

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 実行環境の窓口を 1 度でも呼んだか。
fn touched_runtime(calls: &[Call]) -> bool {
    !calls.is_empty()
}

// ===========================================================================
// 変化が無ければ 1 命令も出さない（要件 6.4／14.2）
// ===========================================================================

/// 印が立っていない巡は、実行環境を 1 度も呼ばず、記録も 1 行も出さない。
///
/// 捕捉が死んでいれば「出ない」は恒真なので、**同じ捕捉窓で確かに出る記録**を併置し、
/// 鎖の記録だけがゼロであることを本数で固定する。
#[test]
fn an_unchanged_plan_issues_nothing_at_all() {
    let mut world = World::new();
    let front = spawn_window(&mut world, fake_hwnd(0x10));
    let back = spawn_window(&mut world, fake_hwnd(0x20));
    record_link(&mut world, front, back, fake_hwnd(0x10), fake_hwnd(0x20));
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members: vec![front, back],
            cross_edges: vec![edge(front, back, ChainSegment::Group(0))],
            absent: Vec::new(),
        }),
        dirty: false,
    });

    let mut schedule = chain_schedule();
    let (out, used) = with_script(Script::default(), || {
        capture_under_filter(SIGNOFF_DIRECTIVES, || {
            // 対照: 同じ捕捉窓に確かに現れる 1 行（捕捉そのものが生きている証拠）。
            tracing::debug!("{}", "[zorder-chain] canary from the same capture window");
            schedule.run(&mut world);
        })
    });

    assert!(
        out.contains("canary from the same capture window"),
        "捕捉窓が死んでいる（対照の 1 行すら拾えていない）: {out}"
    );
    assert!(
        !touched_runtime(&used.calls),
        "印が立っていない巡で実行環境を呼んでいる: {:?}",
        used.calls
    );
    for tag in [LINKED, UNLINKED, SETTLED, SKIPPED, LINK_FAILED] {
        assert_line_count(&out, tag, 0, "印が立っていない巡");
    }
}

/// 印は立っているが出す操作が無い巡は、理由つきの見送りを残して実行環境を呼ばない。
///
/// 黙って諦めない（要件 8.3）——「同じ内容の再公開」と「記録が出ていない」を、
/// 事後に区別できる形にしておく。
#[test]
fn a_republished_but_identical_plan_records_a_reason_and_touches_nothing() {
    let mut world = World::new();
    let front = spawn_window(&mut world, fake_hwnd(0x10));
    let back = spawn_window(&mut world, fake_hwnd(0x20));
    record_link(&mut world, front, back, fake_hwnd(0x10), fake_hwnd(0x20));
    publish(
        &mut world,
        vec![front, back],
        vec![edge(front, back, ChainSegment::Group(0))],
    );

    let (out, used) = run_once(&mut world, Script::default());

    assert!(
        !touched_runtime(&used.calls),
        "出す操作が無いのに実行環境を呼んでいる: {:?}",
        used.calls
    );
    assert_line_count(&out, SKIPPED, 1, "同じ内容の再公開");
    assert_eq!(
        field(lines_with(&out, SKIPPED)[0], "reason"),
        "NoChange",
        "見送りの理由が読めない"
    );
    assert_line_count(&out, SETTLED, 0, "操作が走っていない巡に後押しは出ない");
}

// ===========================================================================
// 撤去が先・付与が後（§12.4 の実測手順）
// ===========================================================================

/// 張り替えの巡では、**すべての撤去がすべての付与より前に**実行環境へ届く。
///
/// 混ぜると、外す前に張った窓が一時的に 2 つの所有者を主張する形になり、途中状態で
/// 鎖が分岐する。ここでは替え玉が積んだ呼び出しの列そのものを見て順序を固定する。
#[test]
fn every_detach_reaches_the_runtime_before_every_attach() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    let c = spawn_window(&mut world, fake_hwnd(0x30));
    // 現況: a は b に所有されている。望む鎖: a → c → b（a の相手が入れ替わる）。
    record_link(&mut world, a, b, fake_hwnd(0x10), fake_hwnd(0x20));
    publish(
        &mut world,
        vec![a, c, b],
        vec![
            edge(a, c, ChainSegment::Group(0)),
            edge(c, b, ChainSegment::Group(0)),
        ],
    );

    let mut script = Script::default();
    script.owner_of.insert(0x10, 0x20);

    let (out, used) = run_once(&mut world, script);

    let last_detach = used
        .calls
        .iter()
        .rposition(|c| matches!(c, Call::ClearOwner(_)))
        .expect("撤去が 1 度も走っていない");
    let first_attach = used
        .calls
        .iter()
        .position(|c| matches!(c, Call::SetOwner(_, _)))
        .expect("付与が 1 度も走っていない");
    assert!(
        last_detach < first_attach,
        "付与のあとに撤去が並んでいる（途中状態が壊れる手順）: {:?}",
        used.calls
    );
    // 撤去の前に必ず現況を読んでいる（照合を省いていない）。
    let read = used
        .calls
        .iter()
        .position(|c| matches!(c, Call::ReadOwner(0x10)))
        .expect("外す前に現況を読んでいない");
    assert!(read < last_detach, "読む前に外している: {:?}", used.calls);

    assert_line_count(&out, UNLINKED, 1, "張り替え");
    assert_line_count(&out, LINKED, 2, "張り替え");
}

// ===========================================================================
// 帳簿と現況が食い違えば実行環境を呼ばない（§12.6・要件 8.3）
// ===========================================================================

/// 現況が帳簿と違う繋ぎは、**実行環境を呼ばず**に帳簿だけを落とし、理由を記録する。
///
/// 照合を省くと、既存のペア機構が張り替えた繋ぎを誤って外し、バルーンがキャラ窓の
/// 直上という不変条件（要件 6.3）を壊す。
#[test]
fn a_diverged_ledger_entry_is_dropped_without_calling_the_runtime() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    record_link(&mut world, a, b, fake_hwnd(0x10), fake_hwnd(0x20));
    // 解除（望む繋ぎはゼロ）。帳簿の 1 件は撤去の対象になる。
    world.insert_resource(ZOrderChainPlan {
        chain: None,
        dirty: true,
    });

    let mut script = Script::default();
    // 現況の所有者は帳簿の控え（0x20）と違う——他機構が張り替えた姿。
    script.owner_of.insert(0x10, 0x99);

    let (out, used) = run_once(&mut world, script);

    assert!(
        used.calls.contains(&Call::ReadOwner(0x10)),
        "外す前に現況を読んでいない: {:?}",
        used.calls
    );
    assert!(
        !used.calls.iter().any(|c| matches!(c, Call::ClearOwner(_))),
        "食い違っているのに実行環境へ撤去を頼んでいる: {:?}",
        used.calls
    );
    assert_line_count(&out, UNLINKED, 1, "食い違い");
    assert_eq!(
        field(lines_with(&out, UNLINKED)[0], "reason"),
        "Diverged",
        "食い違いの理由が読めない"
    );
    // 帳簿は落ちる（同じ判断を毎巡繰り返さない）。
    world.flush();
    assert!(
        world.entity(a).get::<CrossOwnerLink>().is_none(),
        "食い違った帳簿が残っている"
    );
    // 実行環境を 1 度も書いていないので後押しも出ない。
    assert_line_count(&out, SETTLED, 0, "書いていない巡");
}

// ===========================================================================
// 失敗した 1 本だけを飛ばす（要件 8.2）
// ===========================================================================

/// 付与に失敗した繋ぎは**その 1 本だけ**飛ばし、残りは張る。同じ巡で再試行しない。
#[test]
fn a_failed_attach_skips_only_that_edge_and_the_rest_are_linked() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    let c = spawn_window(&mut world, fake_hwnd(0x30));
    publish(
        &mut world,
        vec![a, b, c],
        vec![
            edge(a, b, ChainSegment::Group(0)),
            edge(b, c, ChainSegment::Group(0)),
        ],
    );

    let mut script = Script::default();
    script.set_fails = HashSet::from([0x10usize]);

    let (out, used) = run_once(&mut world, script);

    let attempts: Vec<&Call> = used
        .calls
        .iter()
        .filter(|c| matches!(c, Call::SetOwner(_, _)))
        .collect();
    assert_eq!(
        attempts.len(),
        2,
        "同じ巡で再試行している、ないし残りを張っていない: {:?}",
        used.calls
    );
    assert_line_count(&out, LINK_FAILED, 1, "1 本だけ失敗");
    assert_line_count(&out, LINKED, 1, "残りは張る");
    assert_eq!(
        field(lines_with(&out, LINKED)[0], "owned_hwnd"),
        "0x20",
        "失敗した側を成功として記録している"
    );

    // 失敗した繋ぎは帳簿に載らない（載せると次巡の照合が嘘の一致を出す）。
    world.flush();
    assert!(
        world.entity(a).get::<CrossOwnerLink>().is_none(),
        "張れなかった繋ぎが帳簿に載っている"
    );
    assert!(
        world.entity(b).get::<CrossOwnerLink>().is_some(),
        "張れた繋ぎが帳簿に載っていない"
    );
}

/// 窓ハンドルがまだ取れていない繋ぎは、理由つきの見送りとして 1 本だけ飛ばす。
#[test]
fn an_edge_without_handles_is_skipped_with_a_reason_and_the_rest_are_linked() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    // ハンドル未取得の窓（実体はあるが OS のハンドルがまだ無い）。
    let pending = world.spawn_empty().id();
    publish(
        &mut world,
        vec![a, b, pending],
        vec![
            edge(a, b, ChainSegment::Group(0)),
            edge(b, pending, ChainSegment::Group(0)),
        ],
    );

    let (out, used) = run_once(&mut world, Script::default());

    assert_line_count(&out, SKIPPED, 1, "ハンドル未取得");
    assert_eq!(
        field(lines_with(&out, SKIPPED)[0], "reason"),
        "HandleMissing",
        "見送りの理由が読めない"
    );
    assert_line_count(&out, LINKED, 1, "残りは張る");
    assert_eq!(
        used.calls
            .iter()
            .filter(|c| matches!(c, Call::SetOwner(_, _)))
            .count(),
        1,
        "ハンドルの無い窓へ書き込もうとしている: {:?}",
        used.calls
    );
}

// ===========================================================================
// 後押し 1 回と、その直後の実測（要件 9.2／9.3／11.1）
// ===========================================================================

/// 操作が走った巡は、鎖全体へ後押しが**ちょうど 1 回**出て、その直後に実測される。
///
/// 宣言と実測は同じ 1 行に載る（分けると「指令は出したが効かなかった」の判定が
/// 2 行の突合になる＝要件 9.2 が同一行を求める理由）。
#[test]
fn the_nudge_runs_once_for_the_whole_chain_and_the_measurement_follows_it_immediately() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    let c = spawn_window(&mut world, fake_hwnd(0x30));
    publish(
        &mut world,
        vec![a, b, c],
        vec![
            edge(a, b, ChainSegment::Group(0)),
            edge(b, c, ChainSegment::Group(0)),
        ],
    );

    let mut script = Script::default();
    // 最も奥（0x30）から手前へ辿ると、間に部外者（0x99・不可視の隣は走査が既に
    // 読み飛ばしている）を挟んで 0x20・0x10 が現れる。
    script
        .front_of
        .insert(0x30, vec![0x20usize, 0x99usize, 0x10usize]);

    let (out, used) = run_once(&mut world, script);

    let nudges: Vec<&Call> = used
        .calls
        .iter()
        .filter(|c| matches!(c, Call::Nudge(_, _)))
        .collect();
    assert_eq!(
        nudges.len(),
        1,
        "後押しが鎖全体につき 1 回になっていない: {:?}",
        used.calls
    );
    assert_eq!(
        *nudges[0],
        Call::Nudge(0x10, 0x20),
        "後押しの形が「先頭を 2 番目の直後へ差し直す」になっていない"
    );

    // 実測は**後押しの直後**である（間に他の窓口を挟まない）。
    let nudge_at = used
        .calls
        .iter()
        .position(|c| matches!(c, Call::Nudge(_, _)))
        .expect("後押しが無い");
    assert_eq!(
        used.calls.get(nudge_at + 1),
        Some(&Call::MeasureFront(0x30)),
        "後押しの直後に実測していない: {:?}",
        used.calls
    );

    assert_line_count(&out, SETTLED, 1, "鎖全体につき 1 行");
    let settled = lines_with(&out, SETTLED)[0];
    assert_eq!(field(settled, "nudged_hwnd"), "0x10", "{settled}");
    assert_eq!(field(settled, "insert_after"), "0x20", "{settled}");
    assert_eq!(field(settled, "declared"), "0x10,0x20,0x30", "{settled}");
    assert_eq!(field(settled, "measured"), "0x10,0x20,0x30", "{settled}");
    assert_eq!(field(settled, "nudge_ok"), "true", "{settled}");
}

/// 後押しが失敗しても記録して続行する（黙って消えない・要件 8.2／8.3）。
#[test]
fn a_failed_nudge_is_recorded_on_the_settled_line_rather_than_swallowed() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let mut script = Script::default();
    script.nudge_fails = true;

    let (out, _used) = run_once(&mut world, script);

    assert_line_count(&out, SETTLED, 1, "後押しの失敗");
    assert_eq!(
        field(lines_with(&out, SETTLED)[0], "nudge_ok"),
        "false",
        "後押しの失敗が行から読めない"
    );
}

/// 実測は宣言に無い窓を拾わない（鎖の外の窓の前後は主張しない・DD-3b）。
#[test]
fn the_measurement_reports_only_the_windows_the_chain_declared() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(0))],
    );

    let mut script = Script::default();
    // 部外者（0xAA・0xBB）が鎖の窓の間にも手前にも居る。
    script
        .front_of
        .insert(0x20, vec![0xAAusize, 0x10usize, 0xBBusize]);

    let (out, _used) = run_once(&mut world, script);

    assert_eq!(
        field(lines_with(&out, SETTLED)[0], "measured"),
        "0x10,0x20",
        "鎖の外の窓を実測へ混ぜている"
    );
}

// ===========================================================================
// 破棄に先立って外す（要件 7.2）
// ===========================================================================

/// 所有側の窓が去った繋ぎは、望む鎖の変化を待たずに外れる。
///
/// 所有関係を張ったまま所有側を壊すと OS の破棄カスケードが被所有側を巻き込む。
/// 外す契機は「望む鎖が変わったこと」ではなく「窓が去ること」なので、印を待たない。
#[test]
fn a_link_whose_owner_is_leaving_is_detached_before_the_window_is_destroyed() {
    let mut world = World::new();
    let owned = spawn_window(&mut world, fake_hwnd(0x10));
    let owner = spawn_window(&mut world, fake_hwnd(0x20));
    record_link(&mut world, owned, owner, fake_hwnd(0x10), fake_hwnd(0x20));
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members: vec![owned, owner],
            cross_edges: vec![edge(owned, owner, ChainSegment::Group(0))],
            absent: Vec::new(),
        }),
        // 印は立っていない——それでも去る窓の切離しは走る。
        dirty: false,
    });
    // 所有側の窓が去る（実体ごと消える）。
    world.despawn(owner);

    let mut script = Script::default();
    script.owner_of.insert(0x10, 0x20);

    let (out, used) = run_once(&mut world, script);

    assert!(
        used.calls.contains(&Call::ClearOwner(0x10)),
        "去る窓の繋ぎを外していない: {:?}",
        used.calls
    );
    assert_line_count(&out, UNLINKED, 1, "去る窓");
    assert_eq!(
        field(lines_with(&out, UNLINKED)[0], "reason"),
        "Departing",
        "去る窓の理由が読めない"
    );
    world.flush();
    assert!(
        world.entity(owned).get::<CrossOwnerLink>().is_none(),
        "去った相手の帳簿が残っている"
    );
}

/// 相手が健在なら切離しは走らない（生きている窓の関係には触れない）。
#[test]
fn a_link_whose_owner_is_alive_is_left_untouched() {
    let mut world = World::new();
    let owned = spawn_window(&mut world, fake_hwnd(0x10));
    let owner = spawn_window(&mut world, fake_hwnd(0x20));
    record_link(&mut world, owned, owner, fake_hwnd(0x10), fake_hwnd(0x20));
    world.insert_resource(ZOrderChainPlan {
        chain: None,
        dirty: false,
    });

    let (_out, used) = run_once(&mut world, Script::default());

    assert!(
        !touched_runtime(&used.calls),
        "健在な相手の繋ぎに触っている: {:?}",
        used.calls
    );
}

// ===========================================================================
// 区間の帰属（要件 9.1「どのグループの」）
// ===========================================================================

/// **同じ 1 巡**の中で、グループの繋ぎと後方配置の繋ぎが別々の `segment=` で出る。
///
/// 要件 9.1 は「どのグループの窓を、どの窓のすぐ手前に位置づけたか」を求める。区間は
/// 連結された `members` からは復元できない（グループの境目が列に残らない）ので、
/// 望む鎖が [`CrossEdge::segment`] として運んでくる。ここでは**その値が記録の字面へ
/// 素通しで届いていること**を、区間の異なる 2 種類の繋ぎを 1 巡に混ぜて固定する。
/// 帰属を落とす（全部を番兵や 1 種類へ潰す）変異は、この本数と字面の両方で赤くなる。
#[test]
fn one_pass_records_a_group_edge_and_a_tail_edge_under_different_segments() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    let c = spawn_window(&mut world, fake_hwnd(0x30));
    let d = spawn_window(&mut world, fake_hwnd(0x40));
    // 手前 2 枚がグループ 0、奥 2 枚が後方配置。継ぎ目の繋ぎは手前側の区間を名乗る。
    publish(
        &mut world,
        vec![a, b, c, d],
        vec![
            edge(a, b, ChainSegment::Group(0)),
            edge(b, c, ChainSegment::Group(0)),
            edge(c, d, ChainSegment::Tail),
        ],
    );

    let (out, _used) = run_once(&mut world, Script::default());

    assert_line_count(&out, LINKED, 3, "区間の混在した 1 巡");
    let by_owned = |hwnd: &str| -> &str {
        let found: Vec<&str> = lines_with(&out, LINKED)
            .into_iter()
            .filter(|l| field(l, "owned_hwnd") == hwnd)
            .collect();
        assert_eq!(
            found.len(),
            1,
            "owned_hwnd={hwnd} の繋いだ行が 1 本でない: {out}"
        );
        found[0]
    };

    assert_eq!(field(by_owned("0x10"), "segment"), "g0", "{out}");
    assert_eq!(field(by_owned("0x20"), "segment"), "g0", "{out}");
    assert_eq!(field(by_owned("0x30"), "segment"), "tail", "{out}");

    // 番兵へ潰れていないこと（区間を落とす変異は必ずここに現れる）。
    assert!(
        !lines_with(&out, LINKED)
            .iter()
            .any(|l| field(l, "segment") == "-"),
        "区間が番兵へ潰れている: {out}"
    );
    // 2 種類が実際に共存している（1 種類へ潰す変異も赤になる）。
    let mut kinds: Vec<&str> = lines_with(&out, LINKED)
        .iter()
        .map(|l| field(l, "segment"))
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(
        kinds,
        vec!["g0", "tail"],
        "区間が 1 種類へ潰れている: {out}"
    );

    // 欄の並びは字面のまま（間に別の欄が割り込まない＝初版 6.3 の罠）。
    assert!(
        by_owned("0x10").contains(&format!("segment=g0 owned={a:?}")),
        "{out}"
    );
    assert!(
        by_owned("0x30").contains(&format!("segment=tail owned={c:?}")),
        "{out}"
    );
}

/// 外した行は、**張った時点で帳簿が控えた区間**を名乗る。
///
/// 撤去が起きる局面（解除・窓の退去）では望む鎖から区間を引けない。控えが無ければ
/// ここは必ず番兵になり、「どのグループの繋ぎが解けたか」が記録から読めなくなる。
#[test]
fn the_unlinked_line_names_the_segment_the_ledger_recorded_when_it_was_linked() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    let c = spawn_window(&mut world, fake_hwnd(0x30));
    let d = spawn_window(&mut world, fake_hwnd(0x40));
    record_link_in(
        &mut world,
        a,
        b,
        fake_hwnd(0x10),
        fake_hwnd(0x20),
        ChainSegment::Group(2),
    );
    record_link_in(
        &mut world,
        c,
        d,
        fake_hwnd(0x30),
        fake_hwnd(0x40),
        ChainSegment::Tail,
    );
    // 全解除（望む繋ぎはゼロ）。
    world.insert_resource(ZOrderChainPlan {
        chain: None,
        dirty: true,
    });

    let mut script = Script::default();
    script.owner_of.insert(0x10, 0x20);
    script.owner_of.insert(0x30, 0x40);

    let (out, _used) = run_once(&mut world, script);

    assert_line_count(&out, UNLINKED, 2, "全解除");
    let segments: Vec<(&str, &str)> = lines_with(&out, UNLINKED)
        .into_iter()
        .map(|l| (field(l, "owned_hwnd"), field(l, "segment")))
        .collect();
    assert!(
        segments.contains(&("0x10", "g2")) && segments.contains(&("0x30", "tail")),
        "撤去の区間が帳簿の控えを名乗っていない: {segments:?}
{out}"
    );
}

/// 端点が同じまま区間だけが変わった繋ぎは、実行環境を呼ばずに帳簿の控えだけが変わる。
///
/// 同じ隣り合わせが、後方配置からグループへ（あるいはその逆へ）付け替わることは実際に
/// 起こる。所有関係は変わらないので指令は出さない——それでも控えを放置すると、次に
/// 外したときの記録が古い区間を名乗る。
#[test]
fn a_reattributed_edge_updates_the_ledger_without_calling_the_runtime() {
    let mut world = World::new();
    let a = spawn_window(&mut world, fake_hwnd(0x10));
    let b = spawn_window(&mut world, fake_hwnd(0x20));
    record_link_in(
        &mut world,
        a,
        b,
        fake_hwnd(0x10),
        fake_hwnd(0x20),
        ChainSegment::Tail,
    );
    publish(
        &mut world,
        vec![a, b],
        vec![edge(a, b, ChainSegment::Group(1))],
    );

    let (out, used) = run_once(&mut world, Script::default());

    assert!(
        !touched_runtime(&used.calls),
        "帰属が変わっただけで実行環境を呼んでいる: {:?}",
        used.calls
    );
    assert_line_count(&out, SKIPPED, 1, "帰属だけの変化");
    assert_eq!(field(lines_with(&out, SKIPPED)[0], "reason"), "NoChange");

    world.flush();
    assert_eq!(
        world
            .entity(a)
            .get::<CrossOwnerLink>()
            .expect("帳簿は残る")
            .segment,
        ChainSegment::Group(1),
        "帳簿の区間が古いまま残っている"
    );
}

// ===========================================================================
// 促しの呼び出しを持たない（要件 14.2・14.5）
// ===========================================================================

/// 適用系のソースには、処理の実行を促す呼び出しも遅延キューへの積込も現れない。
///
/// 「無い」の主張は走査が壊れていても緑になるので、⑴同じ針が実際に促している既存経路で
/// 当たること、⑵**呼ぶ側の実装へ差し替えた本文**では当たること、の 2 つを併置する。
#[test]
fn the_apply_system_never_asks_the_runtime_to_keep_ticking() {
    let here = code_only(include_str!("zorder_chain_apply.rs"));

    assert!(
        !here.contains("tick_wake"),
        "適用の側から起床を促している（要件 14.2 が禁じた形）"
    );
    assert!(
        !here.contains("SetWindowPosCommand::enqueue"),
        "後押しが遅延キューへ積まれている（同じ巡で実測できない・要件 9.2）"
    );

    // 対照⑴: 同じ針が、実際に促す／積む既存経路では必ず当たる（走査の空振り検出）。
    assert!(
        code_only(include_str!("command.rs")).contains("tick_wake"),
        "走査そのものが壊れている（起床を促す既存経路を見つけられない）"
    );
    assert!(
        code_only(include_str!("zorder_pair_maintain.rs")).contains("SetWindowPosCommand::enqueue"),
        "走査そのものが壊れている（積む既存経路を見つけられない）"
    );

    // 対照⑵: 呼ぶ側の実装へ差し替えると、同じ走査が確かに赤へ倒れる。
    let swapped = here.replace(
        "unsafe {\n        guarded_set_window_pos(",
        "SetWindowPosCommand::enqueue(cmd.clone());\n    unsafe {\n        guarded_set_window_pos(",
    );
    assert_ne!(swapped, here, "差し替えの当て先が本文から消えている");
    assert!(
        swapped.contains("SetWindowPosCommand::enqueue"),
        "呼ぶ側の実装へ差し替えても走査が当たらない（この検査は空虚に緑である）"
    );
}

// ===========================================================================
// 替え玉そのものの較正（台本が本当に効いているか）
// ===========================================================================

/// 替え玉は据えたときだけ効き、外れれば本番の窓口へ戻る。
///
/// 台本が据わっていない走行で呼び出しが積まれるなら、上の各テストの「呼んでいない」は
/// 何も意味しない。ここで札の生死そのものを固定する。
#[test]
fn the_runtime_double_is_only_in_effect_while_its_script_is_installed() {
    let (seen, used) = with_script(Script::default(), || {
        double::read_owner(fake_hwnd(0x10)).is_some()
    });
    assert!(seen, "台本を据えても替え玉が効いていない");
    assert_eq!(
        used.calls,
        vec![Call::ReadOwner(0x10)],
        "呼び出しが記録されていない"
    );

    assert!(
        double::read_owner(fake_hwnd(0x10)).is_none(),
        "台本を外しても替え玉が居座っている（本番の窓口へ戻っていない）"
    );
}

/// 台本の失敗指定が、それぞれの窓口で確かに失敗として現れる。
#[test]
fn the_scripted_failures_actually_fail() {
    let script = Script {
        read_fails: HashSet::from([0x10usize]),
        clear_fails: HashSet::from([0x20usize]),
        set_fails: HashSet::from([0x30usize]),
        owner_of: HashMap::new(),
        ..Script::default()
    };
    let (results, _used) = with_script(script, || {
        (
            double::read_owner(fake_hwnd(0x10))
                .expect("据わっている")
                .is_err(),
            double::clear_owner(fake_hwnd(0x20))
                .expect("据わっている")
                .is_err(),
            double::set_owner(fake_hwnd(0x30), fake_hwnd(0x40))
                .expect("据わっている")
                .is_err(),
            double::set_owner(fake_hwnd(0x50), fake_hwnd(0x60))
                .expect("据わっている")
                .is_ok(),
        )
    });
    assert_eq!(
        results,
        (true, true, true, true),
        "台本の失敗指定が窓口へ届いていない"
    );
}
