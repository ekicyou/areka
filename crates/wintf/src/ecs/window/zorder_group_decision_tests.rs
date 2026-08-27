//! `zorder_group` の受け口・観測・是正要否の純判断・記録の入口に対する決定論的テスト。
//!
//! 実機も実ディスプレイも World も使わない（要件 10.1）。窓の実体は空の `World` から
//! 採った `Entity` の値、ハンドルは Win32 へ渡さない偽の `HWND`、前面走査は
//! [`FrontScan`] を手で組んだ値である——実測の口（`GroupProbe`）を差し替えられる形に
//! してあるのは、まさにこのためである。
//!
//! # 本ファイルが固定するもの
//!
//! - **受け口**——既定は空で、検証待ちは一回限り、連続失敗はグループごとに数える。
//! - **観測**——未解決の窓は列から落ちて数だけ残り、相対順は「部分列」で判定され、
//!   構成外の窓が何枚挟まっても、最前面に居ても、判定は変わらない（要件 6.2）。
//! - **走査の共有**——実測の口の既定実装が、既存のペア機構の前面走査を**本当に呼んで
//!   いる**こと（要件 9.3）。ここだけは偽の走査では見えないので、無効なハンドルを渡して
//!   本物だけが残す痕跡を観測する（窓は 1 枚も作らない）。
//! - **判断**——3 択に閉じ、先頭の窓は決して連鎖に載らず、連鎖に載るのは構成窓だけ。
//! - **既定状態＝非強制**——受け口が空のとき、実測の口が**一度も呼ばれず**、判断も
//!   **一度も走らない**（要件 6.1／6.4）。
//! - **記録**——見送りは必ず理由つきの記録になり適用対象としては返らない（要件 8.3）。
//!   未出現の窓は記録に残るが維持は止まらない（要件 8.4）。
//!
//! # 「起きてはならない」を片側だけの入力で主張しない
//!
//! 本 spec の先行タスク（1.2）は、檻の入力がすべて同じ側に偏っていたために
//! 「1 枚だけの要素を末尾へ動かす」変異体を 21 本全部が素通りさせた。ここでは
//! 「呼ばれない」「動かさない」「含まれない」の各主張に、**呼ばれる側・動く側・含まれる側**
//! の対照を必ず併置してある（実測の口の呼出回数は 0 と非 0 の両方を、先頭不動は 2 枚と
//! 4 枚の両方を、構成外の窓は前・間・後ろの 3 位置を見る）。
//!
//! # 記録捕捉で「出ないこと」を主張するときの作法
//!
//! 「この水準では出ない」は捕捉そのものが死んでいても成立する。よって出ないことを見る
//! テストには、**同じ捕捉窓の中で確かに拾える記録**を併置してある（既存ペア機構の
//! `zorder_pair_record_tests` と同じ作法）。

use std::cell::Cell;
use std::collections::HashMap;

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;

use super::{
    GroupFixDecision, GroupObservation, GroupProbe, GroupSkipReason, GroupVerify,
    GroupVerifyOutcome, ZOrderGroupSpec, ZOrderGroups, decide_group_fix, log_group_applied,
    observe_group, order_holds, plan_group_fixes, record_group_decision, record_group_skip,
    record_group_verification,
};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_pair::FrontScan;

/// 実機サインオフが用いる `RUST_LOG` 相当（グループ系の出力先を点灯させる指定）。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_group=debug";

/// 既定水準（診断手順を有効化していない通常運転）。
const DEFAULT_DIRECTIVES: &str = "info";

/// 記録の出力先。マクロを 1 ファイルに閉じている限りこの 1 本に保たれる。
const LOG_TARGET: &str = "wintf::ecs::window::zorder_group";

/// テスト用の偽 HWND（Win32 へは渡さない・値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// 窓の実体を n 個作る（値としての `Entity` が欲しいだけ）。
fn entities(n: usize) -> Vec<Entity> {
    let mut world = World::new();
    (0..n).map(|_| world.spawn_empty().id()).collect()
}

// ---------------------------------------------------------------------------
// 実測の口の差し替え（呼出回数を数える）
// ---------------------------------------------------------------------------

/// 決定論テスト用の実測の口。**呼ばれた回数を数える**。
///
/// 回数を数えるのは「受け口が空なら観測しない」を、記録の有無ではなく**呼出そのもの**で
/// 見るためである。記録で見ると「記録が出なかっただけ」と区別が付かない。
struct CountingProbe {
    handles: HashMap<Entity, HWND>,
    /// 走査の起点ごとの結果（未登録は「手前に可視の窓が無い・最前面まで辿れた」）
    fronts: HashMap<isize, (Vec<HWND>, bool)>,
    resolve_calls: Cell<usize>,
    scan_calls: Cell<usize>,
}

impl CountingProbe {
    fn new() -> Self {
        Self {
            handles: HashMap::new(),
            fronts: HashMap::new(),
            resolve_calls: Cell::new(0),
            scan_calls: Cell::new(0),
        }
    }

    /// 実体とハンドルの対応を足す。
    fn with_handle(mut self, entity: Entity, hwnd: HWND) -> Self {
        self.handles.insert(entity, hwnd);
        self
    }

    /// ある窓から手前へ辿ったときに見える可視の窓の列（近い順）を仕込む。
    fn with_front(mut self, from: HWND, windows: &[HWND], reached_top: bool) -> Self {
        self.fronts
            .insert(from.0 as isize, (windows.to_vec(), reached_top));
        self
    }
}

impl GroupProbe for CountingProbe {
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

/// 走査結果を手で組む（`order_holds` の直接検査用）。
fn scan(windows: &[HWND], reached_top: bool) -> FrontScan {
    FrontScan {
        windows: windows.to_vec(),
        reached_top,
    }
}

/// 観測結果を手で組む（`decide_group_fix` の直接検査用）。
///
/// 実測列は宣言列と同じにしてある——判断（`decide_group_fix`）は実測列を見ないので
/// ここでは差が意味を持たない。宣言列と実測列が食い違う入力での書式の固定は、
/// 記録の層の兄弟テスト（`zorder_group_diag_tests.rs`）が持つ。
fn observation(id: u32, hwnds: &[HWND], missing: usize, order_ok: bool) -> GroupObservation {
    GroupObservation {
        id,
        hwnds: hwnds.to_vec(),
        measured_front: hwnds.to_vec(),
        missing,
        order_ok,
        // 走査を行った巡（＝検証の証跡になり得る巡）として組む。走査を行わなかった巡
        // （番兵）との差は兄弟の `zorder_group_verify_tests.rs` が固定している。
        scan_complete: Some(true),
    }
}

// ===========================================================================
// 受け口（ZOrderGroups）
// ===========================================================================

/// 既定の受け口は空で、印も検証待ちも連続失敗も立っていない（既定状態＝非強制の土台）。
#[test]
fn default_receiver_declares_nothing_and_waits_for_nothing() {
    let groups = ZOrderGroups::default();

    assert!(groups.groups.is_empty(), "既定でグループが宣言されている");
    assert!(!groups.pending, "既定で是正の印が立っている");
    assert!(!groups.has_verify(), "既定で検証待ちがある");
    assert_eq!(groups.fail_streak(7), 0, "既定で連続失敗が数えられている");
}

/// 検証待ちは一回限り——引き取れば預かりは消える。
#[test]
fn armed_verification_is_taken_exactly_once() {
    let mut groups = ZOrderGroups::default();
    let verify = GroupVerify {
        id: 3,
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0)],
    };
    groups.arm_verify(verify.clone());

    assert!(groups.has_verify(), "預けた検証待ちが見えない");
    assert_eq!(groups.take_verify(), Some(verify), "預けた内容が戻らない");
    assert!(!groups.has_verify(), "引き取った後も預かりが残っている");
    assert_eq!(groups.take_verify(), None, "二度目の引き取りで値が出た");
}

/// 連続失敗はグループごとに独立に数え、成立した巡に 0 へ戻る。
#[test]
fn fail_streaks_are_counted_per_group_and_cleared_on_success() {
    let mut groups = ZOrderGroups::default();

    assert_eq!(groups.note_verify_failure(1), 1);
    assert_eq!(groups.note_verify_failure(1), 2);
    // 対照: 別のグループの失敗は混ざらない（グループごとに数える＝要件 8.2 の頭打ちの前提）
    assert_eq!(groups.note_verify_failure(2), 1, "他グループの数が混ざった");
    assert_eq!(groups.fail_streak(1), 2);

    groups.clear_fail_streak(1);
    assert_eq!(groups.fail_streak(1), 0, "成立後も失敗が残っている");
    assert_eq!(groups.fail_streak(2), 1, "無関係なグループまで消された");
}

/// 連続失敗の数は頭打ちになり、一周して 0 に見えることはない。
#[test]
fn fail_streak_saturates_instead_of_wrapping_to_zero() {
    let mut groups = ZOrderGroups::default();
    for _ in 0..300 {
        groups.note_verify_failure(1);
    }
    assert_eq!(groups.fail_streak(1), u8::MAX, "数が一周して小さくなった");
}

// ===========================================================================
// 既定状態＝非強制（要件 6.1／6.2／6.4）
// ===========================================================================

/// **本タスクの完了条件**——受け口が空のとき、観測も判断も一度も呼ばれない。
///
/// 実測の口の呼出回数が 0 であることが「観測しない」の実体であり、返る計画が 0 本である
/// ことが「判断しない」の実体である（判断は 1 グループにつきちょうど 1 回走るため、
/// 計画の本数がそのまま判断の回数になる）。
#[test]
fn an_empty_receiver_never_observes_and_never_decides() {
    let probe = CountingProbe::new();

    let plans = plan_group_fixes(&[], &probe);

    assert!(plans.is_empty(), "グループが無いのに判断が走った");
    assert_eq!(probe.resolve_calls.get(), 0, "グループが無いのに窓を引いた");
    assert_eq!(
        probe.scan_calls.get(),
        0,
        "グループが無いのに前面走査を行った"
    );
}

/// 対照——グループが 1 本でもあれば、観測も判断も確かに走る。
///
/// これが無いと上の主張は「実測の口が壊れていて何も呼ばれない」でも成立してしまう。
#[test]
fn a_declared_group_does_observe_and_decide() {
    let e = entities(3);
    let probe = CountingProbe::new()
        .with_handle(e[0], fake_hwnd(0xA0))
        .with_handle(e[1], fake_hwnd(0xB0))
        .with_handle(e[2], fake_hwnd(0xC0));
    let groups = vec![ZOrderGroupSpec {
        id: 1,
        members: e.clone(),
    }];

    let plans = plan_group_fixes(&groups, &probe);

    assert_eq!(plans.len(), 1, "宣言したグループの判断が出ていない");
    assert_eq!(probe.resolve_calls.get(), 3, "メンバー全員を引いていない");
    assert_eq!(
        probe.scan_calls.get(),
        1,
        "前面走査は 1 グループ 1 本のはず"
    );
}

/// 活性化された窓（最前面に来た構成外の窓）があっても、他スコープの相対順は変わらない。
///
/// グループの構成窓が宣言どおりに並んでいる限り、構成外の窓が最前面へ来ても
/// 判断は「指令 0 本」である——だからこそ、グループに属さない窓どうしの前後は
/// 利用者の操作のまま残る（要件 6.2）。
#[test]
fn activating_a_non_member_window_still_yields_no_command() {
    let e = entities(2);
    let (front, back) = (fake_hwnd(0xA0), fake_hwnd(0xB0));
    let activated = fake_hwnd(0xF1);
    let other_scope = fake_hwnd(0xF2);
    let probe = CountingProbe::new()
        .with_handle(e[0], front)
        .with_handle(e[1], back)
        // 末尾から手前へ: 活性化された窓・他スコープの窓・グループ先頭、の順に見える
        .with_front(back, &[activated, other_scope, front], true);
    let groups = vec![ZOrderGroupSpec {
        id: 1,
        members: e.clone(),
    }];

    let plans = plan_group_fixes(&groups, &probe);

    assert!(plans[0].observation.order_ok, "構成外の窓が判定を壊した");
    assert_eq!(
        plans[0].decision,
        GroupFixDecision::Skip(GroupSkipReason::AlreadyOrdered),
        "構成外の窓を並べ替えようとしている"
    );
}

/// 別のグループの中身を入れ替えても、当該グループの観測と判断は一切変わらない（要件 3.6）。
#[test]
fn a_group_is_judged_independently_of_the_other_groups() {
    let e = entities(4);
    let (a_front, a_back) = (fake_hwnd(0xA0), fake_hwnd(0xA1));
    let (b_front, b_back) = (fake_hwnd(0xB0), fake_hwnd(0xB1));
    let probe = CountingProbe::new()
        .with_handle(e[0], a_front)
        .with_handle(e[1], a_back)
        .with_handle(e[2], b_front)
        .with_handle(e[3], b_back)
        // A は成立、B は崩れている（B の窓が A の間に挟まっていても A の判定は動かない）
        .with_front(a_back, &[b_back, a_front, b_front], true)
        .with_front(b_back, &[a_back, a_front], true);

    let group_a = ZOrderGroupSpec {
        id: 1,
        members: vec![e[0], e[1]],
    };
    let group_b = ZOrderGroupSpec {
        id: 2,
        members: vec![e[2], e[3]],
    };

    let alone = plan_group_fixes(std::slice::from_ref(&group_a), &probe);
    let together = plan_group_fixes(&[group_a.clone(), group_b.clone()], &probe);
    let reordered = plan_group_fixes(&[group_b, group_a], &probe);

    assert_eq!(
        alone[0].observation, together[0].observation,
        "隣にグループが増えると観測が変わった"
    );
    assert_eq!(
        alone[0].decision, together[0].decision,
        "隣にグループが増えると判断が変わった"
    );
    assert_eq!(
        alone[0].observation, reordered[1].observation,
        "グループの並び順が観測に影響した"
    );
    // 対照: B は確かに崩れており（A と同じ扱いではない）、独立性の主張が
    // 「どちらも同じ結果」で恒真になっていない。
    assert!(
        !together[1].observation.order_ok,
        "B が崩れていない前提が壊れた"
    );
}

// ===========================================================================
// 観測（observe_group / order_holds）
// ===========================================================================

/// 解決できない窓は列から落ちて数だけ残り、残ったメンバーの相対順は保たれる（要件 1.4／8.4）。
#[test]
fn unresolved_members_drop_out_but_keep_the_declared_order() {
    let e = entities(4);
    let (first, third) = (fake_hwnd(0xA0), fake_hwnd(0xC0));
    let probe = CountingProbe::new()
        .with_handle(e[0], first)
        // e[1] と e[3] はまだ窓が無い
        .with_handle(e[2], third)
        .with_front(third, &[first], true);

    let obs = observe_group(
        &ZOrderGroupSpec {
            id: 5,
            members: e.clone(),
        },
        &probe,
    );

    assert_eq!(obs.id, 5);
    assert_eq!(obs.hwnds, vec![first, third], "順序が保存されていない");
    assert_eq!(obs.missing, 2, "未解決の数が合わない");
    assert!(obs.order_ok, "存在する窓だけでの相対順が成立していない");
}

/// 解決できた窓が 2 枚未満なら、前面走査そのものを行わない（実測列も空のまま）。
#[test]
fn a_group_with_fewer_than_two_windows_is_never_scanned() {
    let e = entities(2);
    let probe = CountingProbe::new().with_handle(e[0], fake_hwnd(0xA0));

    let obs = observe_group(&ZOrderGroupSpec { id: 1, members: e }, &probe);

    assert_eq!(obs.hwnds.len(), 1);
    assert_eq!(obs.missing, 1);
    assert_eq!(probe.scan_calls.get(), 0, "比べる相手が居ないのに走査した");
    // 測っていないのだから実測列は空——宣言列を写しておくと「測った」と読める行が出る
    assert!(
        obs.measured_front.is_empty(),
        "走査していないのに実測列が埋まっている: {:?}",
        obs.measured_front
    );
}

/// 実測列は**走査が実際に出会った並び**であって、宣言された並びではない（要件 9.1／9.2）。
///
/// 宣言は手前から `[m0, m1, m2]`、実際の重なりは手前から `[m1, m0, m2]` である。
/// 走査は末尾 `m2` から手前へ辿るので `m0`・`m1` の順に出会い、実測列はそれを手前から
/// 並べ直した `[m1, m0, m2]` になる。ここが宣言の写しなら、**まったく別の Z 形が同じ
/// 記録行を出す**ことになり、`fix` 行が「どの窓がどの窓のすぐ手前に着いたか」に答えられない。
#[test]
fn the_measured_column_comes_from_the_scan_not_from_the_declaration() {
    let e = entities(3);
    let (m0, m1, m2) = (fake_hwnd(0x10), fake_hwnd(0x11), fake_hwnd(0x12));
    let (x, y) = (fake_hwnd(0xF0), fake_hwnd(0xF1));
    let probe = CountingProbe::new()
        .with_handle(e[0], m0)
        .with_handle(e[1], m1)
        .with_handle(e[2], m2)
        // m2 から手前へ: 構成外の窓を挟みつつ m0 → m1 の順で出会う（＝宣言の逆）
        .with_front(m2, &[x, m0, y, m1], true);

    let obs = observe_group(
        &ZOrderGroupSpec {
            id: 8,
            members: e.clone(),
        },
        &probe,
    );

    assert!(!obs.order_ok, "崩れた重なりが成立と判定された");
    // 宣言列は宣言のまま（task 2.1 の意味を変えない＝足すのであって置き換えない）
    assert_eq!(obs.hwnds, vec![m0, m1, m2], "宣言列が書き換えられた");
    // 実測列は走査が出会った順（手前から）
    assert_eq!(
        obs.measured_front,
        vec![m1, m0, m2],
        "実測列が走査を映していない"
    );
    // グループの外側は数も位置も持ち込まない（要件 3.6／6.1——構成外の窓は落ちる）
    for foreign in [x, y] {
        assert!(
            !obs.measured_front.contains(&foreign),
            "構成外の窓 {foreign:?} が実測列に載っている"
        );
    }

    // 対照: 同じメンバーで実際の重なりが宣言どおりなら、実測列は宣言列と一致する
    let ordered = CountingProbe::new()
        .with_handle(e[0], m0)
        .with_handle(e[1], m1)
        .with_handle(e[2], m2)
        .with_front(m2, &[m1, x, m0], true);
    let obs_ok = observe_group(&ZOrderGroupSpec { id: 8, members: e }, &ordered);
    assert!(obs_ok.order_ok, "宣言どおりの重なりが不成立と判定された");
    assert_eq!(
        obs_ok.measured_front, obs_ok.hwnds,
        "成立した巡で実測列と宣言列が食い違った"
    );
}

/// 相対順は「部分列」で見る——構成外の窓が前・間・後ろのどこに何枚挟まっても成立する。
#[test]
fn relative_order_holds_even_with_foreign_windows_interleaved() {
    let (m0, m1, m2) = (fake_hwnd(0x10), fake_hwnd(0x11), fake_hwnd(0x12));
    let (x, y, z) = (fake_hwnd(0xF0), fake_hwnd(0xF1), fake_hwnd(0xF2));
    let members = [m0, m1, m2];

    // 末尾 m2 から手前へ: m1・m0 がこの順に現れれば成立（間に何が挟まっても同じ）
    assert!(order_holds(&members, &scan(&[m1, m0], true)));
    assert!(order_holds(&members, &scan(&[x, m1, y, m0, z], true)));
    assert!(order_holds(&members, &scan(&[m1, m0, x, y, z], true)));
}

/// 相対順が崩れていれば成立しない（対照——上の主張が恒真でないことの裏）。
#[test]
fn relative_order_fails_when_a_member_is_out_of_place() {
    let (m0, m1, m2) = (fake_hwnd(0x10), fake_hwnd(0x11), fake_hwnd(0x12));
    let members = [m0, m1, m2];

    // m0 が m1 より奥に居る（順序が逆）
    assert!(!order_holds(&members, &scan(&[m0, m1], true)));
    // m1 がそもそも m2 より手前に居ない
    assert!(!order_holds(&members, &scan(&[m0], true)));
    // 誰も手前に居ない
    assert!(!order_holds(&members, &scan(&[], true)));
}

/// 窓が 2 枚未満のときは、走査結果が何であれ「崩れている」とは言わない。
#[test]
fn relative_order_is_vacuously_true_for_fewer_than_two_windows() {
    let m0 = fake_hwnd(0x10);
    assert!(order_holds(&[], &scan(&[], false)));
    assert!(order_holds(&[m0], &scan(&[], false)));
}

/// 走査が最後まで辿れなかったときは、是正が要る側へ倒す。
#[test]
fn an_incomplete_scan_is_treated_as_needing_a_fix() {
    let (m0, m1) = (fake_hwnd(0x10), fake_hwnd(0x11));

    // 打切りで m0 を見つけられなかった → 成立とは言わない
    assert!(!order_holds(&[m0, m1], &scan(&[fake_hwnd(0xF0)], false)));
    // 対照: 打切りでも見つかっていれば成立（打切りを一律に失敗へ倒しているわけではない）
    assert!(order_holds(&[m0, m1], &scan(&[m0], false)));
}

// ===========================================================================
// 既定実装が本物の前面走査へ繋がっていること（要件 9.3）
// ===========================================================================

/// 既存の前面走査の出力先。本物を呼んでいれば、失敗の記録はこの名前で出る。
const SHARED_SCAN_TARGET: &str = "wintf::ecs::window::zorder_pair";

/// 既存の前面走査の記録を拾える濾過（警告は `info` でも通るが、意図を明示しておく）。
const SHARED_SCAN_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_pair=debug";

/// `resolve` だけを書いた実装——`scan_in_front` は**既定のまま**にしてある。
///
/// 数え上げ用の [`CountingProbe`] は走査を差し替えてしまうので、「既定実装が本当に既存の
/// 前面走査を呼んでいるか」はそちらの檻の外に落ちる。この型はその一点だけを見るために居る。
struct DefaultScanProbe;

impl GroupProbe for DefaultScanProbe {
    fn resolve(&self, _entity: Entity) -> Option<HWND> {
        None
    }
}

/// 既定の `scan_in_front` は、既存のペア機構の前面走査をそのまま呼ぶ（要件 9.3）。
///
/// # なぜ無効なハンドルで確かめるのか
///
/// 本物の走査（`measure_windows_in_front`）は無効なハンドルに対して 2 つの痕跡を残す
/// ——⑴ 走査の失敗を **`wintf::ecs::window::zorder_pair` の出力先**へ警告として記録し、
/// ⑵ 列を不完全（`reached_top` が偽）として返す。**どちらも本物の走査が走らない限り
/// 観測できない**ので、この 2 つを併せて主張すれば「共有している」が檻の内側に入る。
///
/// 実窓も実ディスプレイも要らない——無効なハンドルを渡すだけであり、窓は 1 枚も作らない。
///
/// # この 1 本が無いと何が静かに壊れるか
///
/// `scan_in_front` は**既定メソッド**である。後続タスクが本番の `GroupProbe` を書くときに
/// うっかり override して走査を自前で書き直しても、他の檻は偽の走査を渡す前提で組んで
/// あるため 1 本も赤くならない。実際、既定実装の本体を「空の `FrontScan` を返すだけ」に
/// 差し替える変異は、本テストを足すまで 27 本すべてを素通りした。
#[test]
fn the_default_scan_delegates_to_the_shared_front_scan() {
    let probe = DefaultScanProbe;
    let mut scanned = None;

    let out = capture_under_filter(SHARED_SCAN_DIRECTIVES, || {
        scanned = Some(probe.scan_in_front(HWND::default()));
    });

    let scanned = scanned.expect("走査は必ず値を返す");
    assert!(
        !scanned.reached_top,
        "無効なハンドルの走査を「最前面まで辿った」と偽っている（本物の走査を呼んでいない疑い）"
    );
    assert!(
        scanned.windows.is_empty(),
        "無効なハンドルから窓が採れている（本物の走査を呼んでいない疑い）"
    );

    let failure_line = out
        .lines()
        .find(|line| line.contains("手前側の走査に失敗しました"))
        .unwrap_or_else(|| {
            panic!("既存の前面走査の失敗記録が出ていない（走査を自前で書き直した疑い）: {out}")
        });
    assert!(
        failure_line.contains(SHARED_SCAN_TARGET),
        "失敗記録が既存の前面走査の出力先から出ていない: {failure_line}"
    );
}

// ===========================================================================
// 判断（decide_group_fix）
// ===========================================================================

/// 相対順が既に成立していれば指令 0 本（同値ガード）。
#[test]
fn an_already_ordered_group_yields_no_command() {
    let obs = observation(1, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, true);

    assert_eq!(
        decide_group_fix(&obs),
        GroupFixDecision::Skip(GroupSkipReason::AlreadyOrdered)
    );
}

/// 解決できた窓が 2 枚未満なら見送り——順序の成否より先に効く。
///
/// `order_ok` の値を両方入れてあるのは、閾値を 1 枚へ緩めた実装が
/// 「1 枚でも `AlreadyOrdered` になるだけ」で素通りしないようにするためである。
#[test]
fn a_group_with_fewer_than_two_resolved_windows_is_skipped_with_a_reason() {
    for order_ok in [true, false] {
        for hwnds in [vec![], vec![fake_hwnd(0xA0)]] {
            let obs = observation(1, &hwnds, 2, order_ok);
            assert_eq!(
                decide_group_fix(&obs),
                GroupFixDecision::Skip(GroupSkipReason::TooFewResolved),
                "{hwnds:?} / order_ok={order_ok}: 枚数の歯止めが効いていない"
            );
        }
    }
    // 対照: 2 枚あれば枚数の歯止めは効かない（閾値が 3 枚へずれていないことの裏）
    let two = observation(1, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, false);
    assert!(
        matches!(decide_group_fix(&two), GroupFixDecision::Chain { .. }),
        "2 枚あるのに枚数不足として見送られた"
    );
}

/// 崩れていれば連鎖を計画する——先頭は動かさず、残りを宣言順のまま後ろへ繋ぐ。
#[test]
fn a_broken_group_is_planned_as_a_chain_that_never_moves_the_head() {
    let windows = [
        fake_hwnd(0xA0),
        fake_hwnd(0xB0),
        fake_hwnd(0xC0),
        fake_hwnd(0xD0),
    ];

    // 2 枚と 4 枚の両方を見る（先頭不動を片側の長さだけで主張しない）
    for len in [2usize, 4] {
        let members = &windows[..len];
        let obs = observation(1, members, 0, false);
        let GroupFixDecision::Chain { head, chain } = decide_group_fix(&obs) else {
            panic!("{len} 枚: 是正が要るのに連鎖が計画されていない");
        };

        assert_eq!(head, members[0], "{len} 枚: 先頭が最も手前の窓でない");
        assert_eq!(chain, members[1..].to_vec(), "{len} 枚: 連鎖の並びが違う");
        assert!(
            !chain.contains(&head),
            "{len} 枚: 動かさないはずの先頭が連鎖に載っている"
        );
    }
}

/// 連鎖に載るのは構成窓だけ——観測が集めた窓以外は 1 枚も現れない（要件 2.5）。
#[test]
fn a_planned_chain_contains_only_the_group_members() {
    let e = entities(3);
    let members = [fake_hwnd(0xA0), fake_hwnd(0xB0), fake_hwnd(0xC0)];
    let foreign = [fake_hwnd(0xF0), fake_hwnd(0xF1), fake_hwnd(0xF2)];
    let probe = CountingProbe::new()
        .with_handle(e[0], members[0])
        .with_handle(e[1], members[1])
        .with_handle(e[2], members[2])
        // 崩れた並び。構成外の窓を前・間・後ろの 3 位置へ置く。
        .with_front(
            members[2],
            &[foreign[0], members[1], foreign[1], foreign[2]],
            true,
        );

    let plans = plan_group_fixes(
        &[ZOrderGroupSpec {
            id: 1,
            members: e.clone(),
        }],
        &probe,
    );

    let GroupFixDecision::Chain { head, chain } = &plans[0].decision else {
        panic!("崩れた並びなのに連鎖が計画されていない");
    };
    assert_eq!(*head, members[0]);
    for window in chain {
        assert!(
            members.contains(window),
            "構成外の窓 {window:?} へ指令を出そうとしている"
        );
    }
    for window in foreign {
        assert!(
            !chain.contains(&window) && *head != window,
            "構成外の窓 {window:?} が計画に載っている"
        );
    }
}

// ===========================================================================
// 記録（唯一の入口）
// ===========================================================================

/// 見送りは必ず理由つきの記録になり、適用対象としては返らない（要件 8.3）。
#[test]
fn every_skip_is_recorded_with_its_reason_and_yields_no_fix() {
    let obs = observation(9, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, true);

    for reason in [
        GroupSkipReason::AlreadyOrdered,
        GroupSkipReason::TooFewResolved,
        GroupSkipReason::MemberMissing,
        GroupSkipReason::PairFixThisPass,
        GroupSkipReason::GaveUpAfterFailures,
    ] {
        let mut taken = Some(GroupFixDecision::Skip(reason));
        let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
            taken = record_group_decision(&obs, GroupFixDecision::Skip(reason));
        });

        assert_eq!(taken, None, "{reason:?}: 見送りが適用対象として返った");
        let line = only_line_with(&out, "[zorder-group] skip");
        assert!(
            line.contains(&format!("reason={reason:?}")),
            "{reason:?}: 理由の無い見送りになっている: {line}"
        );
        assert!(
            line.contains("group_id=9"),
            "{reason:?}: 対象が読めない: {line}"
        );
    }
}

/// 是正の腕は記録の入口をそのまま通り抜け、見送りの記録は出ない（対照）。
#[test]
fn a_chain_passes_through_the_record_gate_untouched() {
    let obs = observation(9, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, false);
    let chain = GroupFixDecision::Chain {
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0)],
    };

    let mut taken = None;
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        taken = record_group_decision(&obs, chain.clone());
    });

    assert_eq!(taken, Some(chain), "是正の腕が入口で失われた");
    assert!(
        !out.contains("[zorder-group] skip"),
        "是正なのに見送りが記録された: {out}"
    );
}

/// まだ現れていない窓があれば記録に残るが、維持は止まらない（要件 8.4）。
#[test]
fn missing_members_are_recorded_while_maintenance_continues() {
    let obs = observation(4, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 3, false);
    let chain = GroupFixDecision::Chain {
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0)],
    };

    let mut taken = None;
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        taken = record_group_decision(&obs, chain.clone());
    });

    assert_eq!(taken, Some(chain), "未出現の窓があると是正が止まっている");
    let line = only_line_with(&out, "[zorder-group] skip");
    assert!(
        line.contains("reason=MemberMissing") && line.contains("missing=3"),
        "未出現の窓が記録から読めない: {line}"
    );

    // 対照: 未出現が無ければこの記録は出ない（恒真の記録になっていない）
    let complete = observation(4, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, false);
    let quiet = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        record_group_decision(
            &complete,
            GroupFixDecision::Chain {
                head: fake_hwnd(0xA0),
                chain: vec![fake_hwnd(0xB0)],
            },
        );
    });
    assert!(
        !quiet.contains("MemberMissing"),
        "未出現が無いのに記録が出た: {quiet}"
    );
}

/// 観測より前の見送り（巡の調停）は、フィールドを落とさず番兵で記録する。
#[test]
fn a_pass_level_skip_is_recorded_with_sentinels_not_missing_fields() {
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        record_group_skip(None, GroupSkipReason::PairFixThisPass, None);
    });

    let line = only_line_with(&out, "[zorder-group] skip");
    assert!(line.contains("group_id=-"), "{line}");
    assert!(line.contains("reason=PairFixThisPass"), "{line}");
    assert!(line.contains("resolved=-"), "{line}");
    assert!(line.contains("order_ok=-"), "{line}");
}

/// 検証は、出した指令と実測を同じ 1 行に載せる（要件 9.1／9.2）。
#[test]
fn verification_puts_the_command_and_the_measurement_on_one_line() {
    let verify = GroupVerify {
        id: 2,
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0), fake_hwnd(0xC0)],
    };
    let measured = observation(
        2,
        &[fake_hwnd(0xA0), fake_hwnd(0xB0), fake_hwnd(0xC0)],
        0,
        true,
    );

    let mut outcome = GroupVerifyOutcome::NotMeasured;
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        outcome = record_group_verification(&verify, &measured);
    });

    assert_eq!(
        outcome,
        GroupVerifyOutcome::Matched,
        "成立した検証が成立として返っていない"
    );
    let line = only_line_with(&out, "[zorder-group] fix");
    assert!(line.contains("group_id=2"), "{line}");
    assert!(line.contains("head=0xA0"), "{line}");
    // 連鎖の各段が「動かした窓@挿入先」として読める
    assert!(line.contains("moves=0xB0@0xA0,0xC0@0xB0"), "{line}");
    assert!(line.contains("measured=0xA0,0xB0,0xC0"), "{line}");
}

/// 検証が不一致なら、是正の記録ではなく検証不一致の記録を error 水準で出す。
#[test]
fn a_failed_verification_is_recorded_as_a_mismatch_at_error_level() {
    let verify = GroupVerify {
        id: 2,
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0)],
    };
    let measured = observation(2, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 1, false);

    let mut outcome = GroupVerifyOutcome::Matched;
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        outcome = record_group_verification(&verify, &measured);
    });

    assert_eq!(
        outcome,
        GroupVerifyOutcome::Mismatched,
        "不一致の検証が成立として返った"
    );
    assert!(
        !out.contains("[zorder-group] fix"),
        "不一致なのに是正が記録された: {out}"
    );
    let line = only_line_with(&out, "[zorder-group] verify-failed");
    assert!(
        line.contains("ERROR"),
        "検証不一致が error 水準でない: {line}"
    );
    assert!(line.contains("missing=1"), "{line}");
}

/// 4 種の記録タグはすべて、サインオフの水準で同じ 1 本の出力先へ出る。
#[test]
fn every_group_record_is_visible_under_the_signoff_directive() {
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, emit_every_record);

    for tag in [
        "[zorder-group] applied",
        "[zorder-group] fix",
        "[zorder-group] skip",
        "[zorder-group] verify-failed",
    ] {
        assert!(
            out.contains(tag),
            "サインオフの水準で `{tag}` が観測できない: {out}"
        );
    }
    assert!(
        out.contains(LOG_TARGET),
        "grep 対象の出力先が module path 既定でない: {out}"
    );
    // 既存ペア機構の語彙を横取りしていない（要件 9.5——あちらの 6 タグは無編集で残る）
    assert!(
        !out.contains("[zorder-pair]"),
        "グループ系の記録がペア機構の語彙を名乗っている: {out}"
    );
}

/// 既定水準では診断専用の 3 種が無音で、検証不一致だけが残る。
///
/// 「出ない」の主張が捕捉の死で恒真にならないよう、同じ捕捉窓から確かに拾える記録
/// （検証不一致）を併置してある。
#[test]
fn diagnostic_records_are_silent_at_default_level_while_mismatch_still_speaks() {
    let out = capture_under_filter(DEFAULT_DIRECTIVES, emit_every_record);

    for tag in [
        "[zorder-group] applied",
        "[zorder-group] fix",
        "[zorder-group] skip",
    ] {
        assert!(
            !out.contains(tag),
            "診断専用の `{tag}` が既定水準へ漏れている: {out}"
        );
    }
    assert!(
        out.contains("[zorder-group] verify-failed"),
        "既定水準で残るべき検証不一致が出ていない（捕捉が死んでいる疑い）: {out}"
    );
}

/// 見送りの理由 5 種は互いに異なる語で記録される（1 語へ潰れると理由が読めない）。
#[test]
fn the_five_skip_reasons_are_recorded_as_distinct_words() {
    let obs = observation(1, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, true);
    let mut seen: Vec<String> = Vec::new();

    for reason in [
        GroupSkipReason::AlreadyOrdered,
        GroupSkipReason::TooFewResolved,
        GroupSkipReason::MemberMissing,
        GroupSkipReason::PairFixThisPass,
        GroupSkipReason::GaveUpAfterFailures,
    ] {
        let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
            record_group_skip(Some(1), reason, Some(&obs));
        });
        let line = only_line_with(&out, "[zorder-group] skip");
        let word = line
            .split("reason=")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or_default()
            .to_string();
        assert!(!word.is_empty(), "理由語が読めない: {line}");
        assert!(!seen.contains(&word), "理由語 `{word}` が他と重なっている");
        seen.push(word);
    }
    assert_eq!(seen.len(), 5);
}

// ---------------------------------------------------------------------------
// 記録の水準・出力先を見るための共通の出し口
// ---------------------------------------------------------------------------

/// 4 種の記録をすべて 1 回ずつ出す（水準・出力先の確認用）。
fn emit_every_record() {
    let obs_ok = observation(1, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, true);
    let obs_broken = observation(1, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, false);
    let verify = GroupVerify {
        id: 1,
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0)],
    };

    log_group_applied("group_id=1 members=2");
    record_group_skip(Some(1), GroupSkipReason::AlreadyOrdered, Some(&obs_ok));
    record_group_verification(&verify, &obs_ok);
    record_group_verification(&verify, &obs_broken);
}

/// 捕捉した出力から、指定タグを含む行をちょうど 1 本取り出す。
fn only_line_with<'a>(out: &'a str, tag: &str) -> &'a str {
    let found: Vec<&str> = out.lines().filter(|l| l.contains(tag)).collect();
    assert_eq!(found.len(), 1, "`{tag}` の行がちょうど 1 本ではない: {out}");
    found[0]
}
