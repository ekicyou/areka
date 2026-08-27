//! グループ維持系（[`apply_zorder_group_maintenance`](super::apply_zorder_group_maintenance)）の
//! 決定論的テスト——実機も実ディスプレイも要らない（要件 10.1）。
//!
//! 窓の実体は空の `World` から採った `Entity` の値、ハンドルは Win32 へ渡さない偽の
//! `HWND`、前面走査は [`FrontScan`] を手で組んだ値である。指令は実行せず、キューの中身
//! （[`drain_window_pos_commands`]）をそのまま検査する——「どの窓へ何を出そうとしたか」は
//! 記録行ではなく**積まれた指令そのもの**で見る（記録は実装が忘れれば出ないが、指令は
//! 忘れれば窓が動かない）。
//!
//! # 本ファイルが固定するもの
//!
//! - **印の門**——是正が要るかもしれない印が立っていない巡は、観測すら行わない。
//! - **調停**——同じ巡に既存のペア機構が是正を出していれば、発行を見送って理由を記録し、
//!   印は保持する。
//! - **連鎖**——是正が要ると判断された**最初の 1 グループ**だけへ、各窓を直前の窓の直後へ
//!   差し込む連鎖として一括で出す。先頭の窓は動かさない。
//! - **構成窓だけ**——構成外の窓へは 1 本も指令が出ない（要件 2.5）。
//! - **位置と寸法を変えない**——指令の組み立て方そのものが保証する（要件 11.1）。
//!
//! 次巡の実測照合・連続失敗の頭打ち・印の解除・起床の印は**まだ無い**（後続タスクの担当）。
//! よって本ファイルは「発行した巡でも印は立ったまま」を正として書いてある。
//!
//! # 「起きてはならない」を片側だけの入力で主張しない
//!
//! 本 spec の先行タスクは、檻の入力がすべて同じ側に偏っていたために変異体を素通りさせて
//! いる。ここでは各主張に必ず対照を併置してある——「観測しない」には**観測する**巡を、
//! 「指令が出ない」には**出る**巡を、「構成外へ出ない」には**構成窓へは出る**ことを、
//! 「2 本目のグループは動かない」には**次巡では動く（＝まだ是正を要している）**ことを。

use std::cell::Cell;
use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
};

use super::{WorldGroupProbe, apply_zorder_group_maintenance, run_group_maintenance_pass};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_group::GroupProbe;
use crate::ecs::window::zorder_pair::{FrontScan, InsertSpec};
use crate::ecs::window::zorder_pair_maintain::HandleQuery;
use crate::ecs::window::{
    IssuedPairFix, SetWindowPosCommand, WindowHandle, ZOrderGroupSpec, ZOrderGroups,
    drain_window_pos_commands,
};

/// 実機サインオフが用いる `RUST_LOG` 相当（グループ系の出力先を点灯させる指定）。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_group=debug";

/// 既存の前面走査の出力先（本物を呼んでいれば失敗の記録はこの名前で出る）。
const SHARED_SCAN_TARGET: &str = "wintf::ecs::window::zorder_pair";

/// 既存の前面走査の記録を拾える濾過。
const SHARED_SCAN_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_pair=debug";

/// テスト用の偽 HWND（Win32 へは渡さない・値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// 窓の実体を n 個作る（値としての `Entity` が欲しいだけ）。
fn entities(n: usize) -> Vec<Entity> {
    let mut world = World::new();
    (0..n).map(|_| world.spawn_empty().id()).collect()
}

/// 前の巡の残留を捨ててから測る（キューはスレッドローカルであり、同じスレッドを別のテストが
/// 使い回す）。
fn clear_queue() {
    let _ = drain_window_pos_commands();
}

// ---------------------------------------------------------------------------
// 実測の口の差し替え（呼出回数を数える）
// ---------------------------------------------------------------------------

/// 決定論テスト用の実測の口。**呼ばれた回数を数える**。
///
/// 回数を数えるのは「印が立っていなければ観測しない」を、記録や指令の有無ではなく
/// **呼出そのもの**で見るためである。指令の有無で見ると「観測はしたが出さなかった」形が
/// 素通りする。
struct FakeProbe {
    handles: HashMap<Entity, HWND>,
    /// 走査の起点ごとの結果（未登録は「手前に可視の窓が無い・最前面まで辿れた」）
    fronts: HashMap<isize, (Vec<HWND>, bool)>,
    resolve_calls: Cell<usize>,
    scan_calls: Cell<usize>,
}

impl FakeProbe {
    fn new() -> Self {
        Self {
            handles: HashMap::new(),
            fronts: HashMap::new(),
            resolve_calls: Cell::new(0),
            scan_calls: Cell::new(0),
        }
    }

    /// 実体と偽ハンドルの対応をまとめて足す（`hwnds[i]` が `members[i]` のハンドル）。
    fn with_handles(mut self, members: &[Entity], hwnds: &[HWND]) -> Self {
        for (entity, hwnd) in members.iter().zip(hwnds.iter()) {
            self.handles.insert(*entity, *hwnd);
        }
        self
    }

    /// ある窓から手前へ辿ったときに見える可視の窓の列（近い順）を仕込む。
    fn with_front(mut self, from: HWND, windows: &[HWND], reached_top: bool) -> Self {
        self.fronts
            .insert(from.0 as isize, (windows.to_vec(), reached_top));
        self
    }
}

impl GroupProbe for FakeProbe {
    fn resolve(&self, entity: Entity) -> Option<HWND> {
        self.resolve_calls.set(self.resolve_calls.get() + 1);
        self.handles.get(&entity).copied()
    }

    fn scan_in_front(&self, hwnd: HWND) -> FrontScan {
        self.scan_calls.set(self.scan_calls.get() + 1);
        match self.fronts.get(&(hwnd.0 as isize)) {
            Some((windows, reached_top)) => FrontScan {
                windows: windows.clone(),
                reached_top: *reached_top,
            },
            None => FrontScan {
                windows: Vec::new(),
                reached_top: true,
            },
        }
    }
}

/// 受け口をグループ 1 本で組む（印は立てた状態）。
fn groups_with(id: u32, members: &[Entity]) -> ZOrderGroups {
    let mut groups = ZOrderGroups::default();
    groups.groups.push(ZOrderGroupSpec {
        id,
        members: members.to_vec(),
    });
    groups.pending = true;
    groups
}

/// 積まれた指令の「動かす窓」を積まれた順に取り出す。
fn issued_targets(cmds: &[SetWindowPosCommand]) -> Vec<HWND> {
    cmds.iter().map(|c| c.hwnd).collect()
}

/// 積まれた指令の「挿入先」を積まれた順に取り出す。
fn issued_anchors(cmds: &[SetWindowPosCommand]) -> Vec<Option<HWND>> {
    cmds.iter().map(|c| c.hwnd_insert_after).collect()
}

// ===========================================================================
// 印の門——印が立っていない巡は観測すらしない
// ===========================================================================

/// 印が立っていなければ、実測の口は 1 度も呼ばれず、指令も 1 本も積まれない。
///
/// 対照として、**同じ受け口・同じ実測の口で印だけを立てた巡**を続けて回す。こちらでは
/// 観測が走り、指令が積まれる——「そもそも壊れていて何も起きない」形と区別するための
/// 片側である。
#[test]
fn an_unset_mark_stops_the_pass_before_any_observation() {
    let members = entities(3);
    let hwnds = [fake_hwnd(0xA1), fake_hwnd(0xA2), fake_hwnd(0xA3)];
    let probe = FakeProbe::new()
        .with_handles(&members, &hwnds)
        // 末尾から手前を見ても構成窓が 1 枚も居ない＝相対順は成立していない
        .with_front(hwnds[2], &[], true);

    let mut groups = groups_with(7, &members);
    groups.pending = false;

    clear_queue();
    run_group_maintenance_pass(&mut groups, false, &probe);

    assert_eq!(
        probe.resolve_calls.get(),
        0,
        "印が立っていない巡でハンドルを引いている"
    );
    assert_eq!(
        probe.scan_calls.get(),
        0,
        "印が立っていない巡で前面走査を行っている"
    );
    assert!(
        drain_window_pos_commands().is_empty(),
        "印が立っていない巡で指令が積まれている"
    );
    assert!(
        !groups.has_verify(),
        "印が立っていない巡で検証待ちが預けられている"
    );

    // 対照: 印を立てれば同じ入力で観測が走り、指令が積まれる
    groups.pending = true;
    run_group_maintenance_pass(&mut groups, false, &probe);

    assert!(
        probe.resolve_calls.get() > 0,
        "印を立てた巡でもハンドルを引いていない（門ではなく実装そのものが死んでいる疑い）"
    );
    assert!(
        probe.scan_calls.get() > 0,
        "印を立てた巡でも前面走査を行っていない（門ではなく実装そのものが死んでいる疑い）"
    );
    assert!(
        !drain_window_pos_commands().is_empty(),
        "印を立てた巡でも指令が積まれない（門ではなく実装そのものが死んでいる疑い）"
    );
    assert!(
        groups.has_verify(),
        "印を立てた巡でも検証待ちが預けられていない"
    );
}

/// グループが 1 本も宣言されていなければ、印が立っていても観測も指令も記録も無い（要件 6.1）。
///
/// 「記録が出ない」は捕捉そのものが死んでいても成立するため、**同じ捕捉窓の中で**
/// 確かに拾える巡（既に整列済みのグループ＝見送りが 1 行出る）を併置してある。
#[test]
fn an_empty_roster_observes_nothing_and_issues_nothing_even_while_marked() {
    let members = entities(2);
    let hwnds = [fake_hwnd(0xB1), fake_hwnd(0xB2)];
    let probe = FakeProbe::new()
        .with_handles(&members, &hwnds)
        // 末尾の手前に先頭が居る＝宣言どおり
        .with_front(hwnds[1], &[hwnds[0]], true);

    let mut empty = ZOrderGroups::default();
    empty.pending = true;
    let mut ordered = groups_with(9, &members);

    clear_queue();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        run_group_maintenance_pass(&mut empty, false, &probe);
        run_group_maintenance_pass(&mut ordered, false, &probe);
    });

    assert!(
        drain_window_pos_commands().is_empty(),
        "空の受け口・整列済みのいずれでも指令は出ないはずだが積まれている"
    );

    let skip_lines: Vec<&str> = out
        .lines()
        .filter(|line| line.contains("[zorder-group] skip"))
        .collect();
    assert_eq!(
        skip_lines.len(),
        1,
        "見送りの記録がちょうど 1 行でない（空の受け口が記録を出している／対照が拾えていない）: {out}"
    );
    assert!(
        skip_lines[0].contains("group_id=9") && skip_lines[0].contains("reason=AlreadyOrdered"),
        "対照の見送りが整列済みのグループのものになっていない: {}",
        skip_lines[0]
    );
}

// ===========================================================================
// 調停——同じ巡にペア機構が是正を出していれば見送る
// ===========================================================================

/// ペア機構が同じ巡に是正を出していれば、発行を見送り、理由を記録し、印は保持する。
///
/// 対照は**同じ入力でペア機構の是正が無い巡**である。そちらでは指令が積まれ、検証待ちが
/// 預けられ、調停の記録は出ない。印がどちらの巡でも立ったままであることも併せて見る
/// ——印の解除は後続タスクの担当であり、この段で消える実装は誤りである。
#[test]
fn a_pair_fix_in_the_same_pass_defers_the_issue_and_keeps_the_mark() {
    let members = entities(3);
    let hwnds = [fake_hwnd(0xC1), fake_hwnd(0xC2), fake_hwnd(0xC3)];
    let probe = FakeProbe::new()
        .with_handles(&members, &hwnds)
        .with_front(hwnds[2], &[], true);

    let mut groups = groups_with(4, &members);

    clear_queue();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        run_group_maintenance_pass(&mut groups, true, &probe);
    });
    let deferred = drain_window_pos_commands();

    assert!(
        deferred.is_empty(),
        "ペア機構が是正を出した巡に、グループ側も指令を積んでいる"
    );
    assert!(
        !groups.has_verify(),
        "出していない指令の検証待ちが預けられている"
    );
    assert!(groups.pending, "調停で見送った巡に印が落ちている");
    let arbitration = out
        .lines()
        .find(|line| line.contains("reason=PairFixThisPass"))
        .unwrap_or_else(|| panic!("調停の見送りが記録されていない（黙って見送っている）: {out}"));
    assert!(
        arbitration.contains("group_id=-"),
        "調停の見送りが特定のグループのものとして記録されている（巡そのものの見送りである）: {arbitration}"
    );

    // 対照: ペア機構の是正が無い巡では出る
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        run_group_maintenance_pass(&mut groups, false, &probe);
    });
    let issued = drain_window_pos_commands();

    assert_eq!(
        issued.len(),
        2,
        "ペア機構の是正が無い巡でも指令が積まれない（調停ではなく実装そのものが死んでいる疑い）"
    );
    assert!(
        groups.has_verify(),
        "発行した巡に検証待ちが預けられていない"
    );
    assert!(
        groups.pending,
        "発行した巡で印が落ちている（印の解除は後続タスクの担当）"
    );
    assert!(
        !out.contains("reason=PairFixThisPass"),
        "ペア機構の是正が無い巡に調停の見送りが記録されている: {out}"
    );
}

// ===========================================================================
// 連鎖の発行
// ===========================================================================

/// 連鎖は各窓を直前の窓の直後へ差し込み、**先頭の窓は動かさない**。
///
/// 枚数を 2 枚と 4 枚の両方で見る——先頭が連鎖に載らないことを 1 通りの長さだけで主張
/// すると、「末尾を落とす」たぐいの変異が片方の長さでしか赤くならない。
#[test]
fn the_chain_inserts_each_window_behind_the_previous_one_and_never_moves_the_head() {
    for count in [2usize, 4usize] {
        let members = entities(count);
        let hwnds: Vec<HWND> = (0..count).map(|i| fake_hwnd(0xD00 + i)).collect();
        let probe =
            FakeProbe::new()
                .with_handles(&members, &hwnds)
                .with_front(hwnds[count - 1], &[], true);
        let mut groups = groups_with(1, &members);

        clear_queue();
        run_group_maintenance_pass(&mut groups, false, &probe);
        let issued = drain_window_pos_commands();

        assert_eq!(
            issued_targets(&issued),
            hwnds[1..].to_vec(),
            "連鎖が動かす窓の並びが宣言の 2 枚目以降と一致しない（枚数 {count}）"
        );
        assert_eq!(
            issued_anchors(&issued),
            hwnds[..count - 1]
                .iter()
                .map(|h| Some(*h))
                .collect::<Vec<_>>(),
            "各段の挿入先が直前の窓になっていない（枚数 {count}）"
        );
        assert!(
            !issued_targets(&issued).contains(&hwnds[0]),
            "先頭の窓へ指令が出ている（枚数 {count}）"
        );
    }
}

/// 積まれた指令は**表示順だけ**を動かす（要件 11.1）。
///
/// 位置・寸法を伴わないことは、フラグと 4 つの座標欄の両方で見る。あわせて
/// 「重なりは動く側」（`SWP_NOZORDER` が立っていない）も見る——3 つのフラグを全部立てて
/// 何も変えない指令にする変異が、緑のまま通るのを防ぐためである。
#[test]
fn every_issued_command_changes_only_the_stacking_order() {
    let members = entities(3);
    let hwnds = [fake_hwnd(0xE1), fake_hwnd(0xE2), fake_hwnd(0xE3)];
    let probe = FakeProbe::new()
        .with_handles(&members, &hwnds)
        .with_front(hwnds[2], &[], true);
    let mut groups = groups_with(2, &members);

    clear_queue();
    run_group_maintenance_pass(&mut groups, false, &probe);
    let issued = drain_window_pos_commands();

    assert_eq!(issued.len(), 2, "連鎖の段数が構成窓の枚数と合わない");
    for cmd in &issued {
        assert!(
            cmd.flags & SWP_NOMOVE == SWP_NOMOVE,
            "位置を動かし得る指令になっている: {:?}",
            cmd.flags
        );
        assert!(
            cmd.flags & SWP_NOSIZE == SWP_NOSIZE,
            "寸法を変え得る指令になっている: {:?}",
            cmd.flags
        );
        assert!(
            cmd.flags & SWP_NOACTIVATE == SWP_NOACTIVATE,
            "活性化を伴う指令になっている: {:?}",
            cmd.flags
        );
        assert!(
            cmd.flags & SWP_NOZORDER != SWP_NOZORDER,
            "重なりを動かさない指令になっている（是正にならない）: {:?}",
            cmd.flags
        );
        assert_eq!(
            (cmd.x, cmd.y, cmd.width, cmd.height),
            (0, 0, 0, 0),
            "指令が座標・寸法の値を運んでいる"
        );
    }
}

/// **構成外の窓へは 1 本も指令が出ない**（要件 2.5）。
///
/// 構成外の窓を手前・間・奥の 3 位置に置く——1 位置だけでは、たまたまその位置を読み飛ばす
/// 実装が素通りする。対照として、構成窓の側には**確かに出ている**ことを同じテストで見る。
#[test]
fn no_command_reaches_a_window_outside_the_group() {
    let members = entities(3);
    let inside = [fake_hwnd(0xF1), fake_hwnd(0xF2), fake_hwnd(0xF3)];
    let outside = [fake_hwnd(0x901), fake_hwnd(0x902), fake_hwnd(0x903)];
    let probe = FakeProbe::new()
        .with_handles(&members, &inside)
        // 末尾から手前へ: 構成外・構成窓・構成外・構成窓・構成外（＝手前・間・奥の 3 位置）。
        // 現れる構成窓の順が宣言の逆順（inside[1] のあと inside[0]）ではないので是正が要る。
        .with_front(
            inside[2],
            &[outside[0], inside[0], outside[1], inside[1], outside[2]],
            true,
        );
    let mut groups = groups_with(3, &members);

    clear_queue();
    run_group_maintenance_pass(&mut groups, false, &probe);
    let issued = drain_window_pos_commands();

    for cmd in &issued {
        assert!(
            inside.contains(&cmd.hwnd),
            "構成外の窓へ指令が出ている: {:?}",
            cmd.hwnd
        );
        let anchor = cmd.hwnd_insert_after.expect("連鎖の挿入先は必ず在る");
        assert!(
            inside.contains(&anchor),
            "構成外の窓を挿入先に指している: {anchor:?}"
        );
    }
    for foreign in outside {
        assert!(
            !issued_targets(&issued).contains(&foreign),
            "構成外の窓 {foreign:?} が動かされている"
        );
    }

    // 対照: 構成窓の側へは確かに出ている（何も出さない実装で緑にならない）
    assert_eq!(
        issued_targets(&issued),
        inside[1..].to_vec(),
        "構成窓へ連鎖が出ていない（構成外を避けているのではなく何もしていない疑い）"
    );
}

/// 是正を出すのは**最初の 1 グループ**だけであり、2 本目は動かない。
///
/// 「動かない」だけでは「2 本目は諦めた」形と区別が付かない。よって 1 本目が整列した
/// 次の巡を続けて回し、2 本目が**まだ是正を要している**（そこで初めて連鎖が出る）ことまで
/// 見る。あわせて、発行したグループより後ろに居る整列済みのグループの見送りが、
/// 発行によって握り潰されていないことも見る。
#[test]
fn only_the_first_group_that_needs_a_fix_is_issued_this_pass() {
    let members = entities(6);
    let first = [fake_hwnd(0x1101), fake_hwnd(0x1102)];
    let second = [fake_hwnd(0x2201), fake_hwnd(0x2202)];
    let settled = [fake_hwnd(0x3301), fake_hwnd(0x3302)];

    let mut groups = ZOrderGroups::default();
    groups.pending = true;
    groups.groups.push(ZOrderGroupSpec {
        id: 1,
        members: members[0..2].to_vec(),
    });
    groups.groups.push(ZOrderGroupSpec {
        id: 2,
        members: members[2..4].to_vec(),
    });
    groups.groups.push(ZOrderGroupSpec {
        id: 3,
        members: members[4..6].to_vec(),
    });

    let broken = FakeProbe::new()
        .with_handles(&members[0..2], &first)
        .with_handles(&members[2..4], &second)
        .with_handles(&members[4..6], &settled)
        .with_front(first[1], &[], true)
        .with_front(second[1], &[], true)
        .with_front(settled[1], &[settled[0]], true);

    clear_queue();
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        run_group_maintenance_pass(&mut groups, false, &broken);
    });
    let issued = drain_window_pos_commands();

    assert_eq!(
        issued_targets(&issued),
        vec![first[1]],
        "1 巡で出す連鎖が最初の 1 グループに閉じていない"
    );
    assert!(
        !issued_targets(&issued).contains(&second[1]),
        "2 本目のグループにも指令が出ている"
    );
    assert!(
        out.contains("group_id=3") && out.contains("reason=AlreadyOrdered"),
        "発行したグループより後ろの整列済みグループの見送りが記録されていない: {out}"
    );

    // 続く巡: 1 本目は整列した。2 本目は**まだ是正を要している**ので今度はそちらが出る。
    let settled_first = FakeProbe::new()
        .with_handles(&members[0..2], &first)
        .with_handles(&members[2..4], &second)
        .with_handles(&members[4..6], &settled)
        .with_front(first[1], &[first[0]], true)
        .with_front(second[1], &[], true)
        .with_front(settled[1], &[settled[0]], true);

    run_group_maintenance_pass(&mut groups, false, &settled_first);
    let issued = drain_window_pos_commands();

    assert_eq!(
        issued_targets(&issued),
        vec![second[1]],
        "見送られた 2 本目が次の巡でも是正されない（黙って諦めている）"
    );
}

// ===========================================================================
// system としての結線（World 側の 2 つのクエリ）
// ===========================================================================

/// 受け口が挿さっていない巡は丸ごと何も起きない（結線前の状態でも異常終了しない）。
#[test]
fn a_missing_resource_leaves_the_pass_a_total_no_op() {
    let mut world = World::new();
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(apply_zorder_group_maintenance);

    clear_queue();
    schedule.run(&mut world);

    assert!(
        drain_window_pos_commands().is_empty(),
        "受け口の無い巡で指令が積まれている"
    );
}

/// system は、同じ巡にペア機構が置いた [`IssuedPairFix`] を見て発行を見送る。
///
/// 実測の口は本番のもの（[`WorldGroupProbe`]）であり、偽ハンドルの前面走査は必ず失敗する
/// ——つまり「相対順は成立していない」と判断されるので、調停が無ければ連鎖が出る。
/// 2 つの巡の差そのものを見るため、どちらの巡も同じ受け口・同じ窓で組む。
#[test]
fn the_system_defers_when_the_pair_mechanism_issued_a_fix_in_the_same_pass() {
    fn build(with_pair_fix: bool) -> (World, Schedule) {
        let mut world = World::new();
        let a = spawn_handle(&mut world, fake_hwnd(0x5001));
        let b = spawn_handle(&mut world, fake_hwnd(0x5002));
        if with_pair_fix {
            // ペア機構が是正を出した巡の写し。この巡に付いたこと自体が調停の入力である。
            world.spawn(IssuedPairFix {
                insert_after: InsertSpec::TopEdge,
            });
        }
        let mut groups = ZOrderGroups::default();
        groups.pending = true;
        groups.groups.push(ZOrderGroupSpec {
            id: 5,
            members: vec![a, b],
        });
        world.insert_resource(groups);
        let mut schedule = Schedule::default();
        schedule.set_executor(SingleThreadedExecutor::new());
        schedule.add_systems(apply_zorder_group_maintenance);
        (world, schedule)
    }

    // 調停あり: 1 本も出ない
    let (mut world, mut schedule) = build(true);
    clear_queue();
    schedule.run(&mut world);
    assert!(
        drain_window_pos_commands().is_empty(),
        "ペア機構の是正が在る巡に system が指令を積んでいる"
    );

    // 対照: 調停なし: 連鎖が出る
    let (mut world, mut schedule) = build(false);
    clear_queue();
    schedule.run(&mut world);
    assert_eq!(
        drain_window_pos_commands().len(),
        1,
        "ペア機構の是正が無い巡でも system が指令を積まない（調停ではなく結線が死んでいる疑い）"
    );
}

/// 窓の実体を 1 枚（ハンドル付き）作る。
fn spawn_handle(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

/// 本番の実測の口が返した走査結果の写し（system の外へ持ち出すための置き場）。
#[derive(Resource, Default)]
struct Scanned {
    reached_top: bool,
    windows: usize,
    ran: bool,
}

/// 本番の実測の口（[`WorldGroupProbe`]）を World の中で組み、無効なハンドルで走査させる。
///
/// 本番の維持系と**同じ形**（名前付き system・`HandleQuery` を引数に取る）で組むのが要点で
/// ある——ここだけ別の作り方をすると、本番が通す経路とは違うものを測ることになる。
fn scan_through_the_production_probe(handles: HandleQuery, mut out: ResMut<Scanned>) {
    let probe = WorldGroupProbe::new(&handles);
    let scan = probe.scan_in_front(HWND::default());
    out.reached_top = scan.reached_top;
    out.windows = scan.windows.len();
    out.ran = true;
}

/// 本番の実測の口は前面走査を**自前で書き直していない**（要件 9.3・task 2.1 からの申し送り）。
///
/// [`GroupProbe::scan_in_front`] は既定メソッドであり、既定実装が既存の走査を呼ぶことは
/// 兄弟の檻に入っている。ただし**本番の実装が override していないこと**はその外側に落ちる
/// ——ここで本番の型そのものへ同じ主張を当てて閉じる。
///
/// 無効なハンドルに対して本物の走査が残す痕跡は 2 つ——⑴ 走査の失敗を
/// `wintf::ecs::window::zorder_pair` の出力先へ警告として記録し、⑵ 列を不完全
/// （`reached_top` が偽）として返す。どちらも本物が走らない限り観測できない。
#[test]
fn the_production_probe_does_not_reimplement_the_shared_front_scan() {
    let mut world = World::new();
    world.init_resource::<Scanned>();
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(scan_through_the_production_probe);

    let captured = capture_under_filter(SHARED_SCAN_DIRECTIVES, || {
        schedule.run(&mut world);
    });

    let scanned = world.resource::<Scanned>();
    assert!(scanned.ran, "走査を回す system が動いていない");
    assert!(
        !scanned.reached_top,
        "無効なハンドルの走査を「最前面まで辿った」と偽っている（本物の走査を呼んでいない疑い）"
    );
    assert_eq!(
        scanned.windows, 0,
        "無効なハンドルから窓が採れている（本物の走査を呼んでいない疑い）"
    );

    let failure_line = captured
        .lines()
        .find(|line| line.contains("手前側の走査に失敗しました"))
        .unwrap_or_else(|| {
            panic!("既存の前面走査の失敗記録が出ていない（走査を自前で書き直した疑い）: {captured}")
        });
    assert!(
        failure_line.contains(SHARED_SCAN_TARGET),
        "失敗記録が既存の前面走査の出力先から出ていない: {failure_line}"
    );
}

// ===========================================================================
// 記録の出口は 1 本のまま（マクロをこのモジュールへ増やさない）
// ===========================================================================

/// 維持系のモジュールには `tracing` のマクロが 1 つも無い。
///
/// 出力先は呼び出し元の module path が既定であり、ここでマクロを呼ぶとサインオフの
/// grep 対象（`wintf::ecs::window::zorder_group`）が分裂する。不在だけを見ると走査が
/// 壊れていても緑になるので、**マクロを持つ兄弟**では同じ走査が必ず何かを見つけること、
/// および説明文を落とす処理が本文まで落としていないことを併置してある。
#[test]
fn no_tracing_macro_lives_in_the_maintenance_module() {
    const MACRO_NEEDLES: [&str; 6] = [
        "trace!(",
        "debug!(",
        "info!(",
        "warn!(",
        "error!(",
        "tracing::",
    ];

    let here = code_only(include_str!("zorder_group_maintain.rs"));
    let sibling = code_only(include_str!("zorder_group.rs"));

    for needle in MACRO_NEEDLES {
        assert!(
            !here.contains(needle),
            "維持系に `{needle}` が現れた（記録の出力先が 2 本に分裂する）"
        );
    }

    let found = MACRO_NEEDLES
        .iter()
        .filter(|needle| sibling.contains(**needle))
        .count();
    assert!(
        found >= 2,
        "走査がマクロを持つ兄弟でも何も見つけない（走査そのものが壊れている疑い）"
    );
    assert!(
        here.contains("pub fn apply_zorder_group_maintenance("),
        "説明文を落とす処理が本文まで落としている"
    );
}

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}
