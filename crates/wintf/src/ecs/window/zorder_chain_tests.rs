//! 鎖の差分の純判断・後押しの選定・記録の出口——決定論的テスト
//! （要件 4.1／7.1／7.2／8.2／10.1／11.1／14.4／14.5）。
//!
//! 実機も実ディスプレイも World の巡回も要らない（要件 10.1）。`HWND` は Win32 へ 1 度も
//! 渡さない偽の値であり、`Entity` は空の `World` から順に採るだけの値である。
//!
//! # 「自分の窓以外は現れない」をどう機械で固定するか
//!
//! 後押しが動かす窓は 1 つに絞ってある——**鎖の根**（初版の「先頭を 2 番目の直後へ」は
//! `research.md` §13.2 で**撤回**された）。挿入位置は錨か先頭の 2 択である。禁じ手
//! （`GW_HWNDPREV` で拾った他プロセスの窓を挿入位置に渡す形・`HWND_TOP` などの絶対帯指定）は
//! いずれも「指令に現れる窓ハンドルが `members` の外から来る」という 1 つの形で現れる。
//! よって指令に現れる**すべての窓ハンドルを列挙し、`members` の部分集合であること**を
//! 主張する。禁じ手へ差し替えた瞬間に、その集合へ外の値が 1 つ混ざって赤くなる。
//!
//! 「無い」を主張する検査は道具が壊れていても緑になるので、同じ検査が**実際に外の値を
//! 掴む**対照（他プロセスの窓を模した値・絶対帯指定の値）を必ず併置してある。
//!
//! # 記録を捕捉する檻は単一スレッドで走る
//!
//! ここに World も Schedule も無い——記録は純粋な関数呼び出しから出る。捕捉ハーネス
//! `capture_under_filter` はスレッドローカルの差し替えなので、同じスレッドで撃った記録
//! だけが拾える。Bevy の既定実行器（多スレッド）を挟むと 1 行も拾えず検査が空虚に緑に
//! なるため、**この檻は Schedule を一切使わない**。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
};
use windows::core::HRESULT;

use super::{
    ChainOp, CrossEdge, CrossOwnerLink, log_chain_absent, log_chain_link_failed, log_chain_linked,
    log_chain_settled, log_chain_skipped, log_chain_unlink_failed, log_chain_unlinked,
    nudge_command, plan_chain_ops,
};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::SetWindowPosCommand;
use crate::ecs::window::zorder_chain_diag::{
    ChainSegment, ChainSkipReason, DetachReason, chain_record_tags, log_group_rejected,
};

/// 実機サインオフが用いる `RUST_LOG` の鎖側の指定そのもの。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_chain=debug";

/// 鎖系 7 語の出力先（本モジュールの module path 既定）。
const CHAIN_TARGET: &str = "wintf::ecs::window::zorder_chain";

/// 行を組む純関数の層の出力先（保全語彙 2 語だけがここから出る）。
const DIAG_TARGET: &str = "wintf::ecs::window::zorder_chain_diag";

// ===========================================================================
// 道具立て
// ===========================================================================

/// テスト用の偽 HWND（Win32 へは渡さない・値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// テスト用の Entity を n 個採る（空の World から順に確保するだけ）。
fn entities(n: usize) -> Vec<Entity> {
    let mut world = World::new();
    (0..n).map(|_| world.spawn_empty().id()).collect()
}

/// 望む繋ぎ 1 本（区間は差分の判断に一切効かないので、ここでは通し番号で埋める）。
fn edge(owned: Entity, owner: Entity) -> CrossEdge {
    CrossEdge {
        owned,
        owner,
        segment: ChainSegment::Group(0),
    }
}

/// 区間だけが違う同じ端点の繋ぎ（差分の判断が区間を見ていないことの対照に使う）。
fn edge_in_tail(owned: Entity, owner: Entity) -> CrossEdge {
    CrossEdge {
        owned,
        owner,
        segment: ChainSegment::Tail,
    }
}

/// 帳簿の 1 件（窓ハンドルの値は照合に使わないので通し番号で埋める）。
fn ledger(owned: Entity, owner: Entity, seq: usize) -> (Entity, CrossOwnerLink) {
    (
        owned,
        CrossOwnerLink {
            owner,
            owned_hwnd: fake_hwnd(0x1000 + seq),
            owner_hwnd: fake_hwnd(0x2000 + seq),
            segment: ChainSegment::Group(0),
        },
    )
}

/// テスト用の失敗値（実際の Win32 呼び出しは行わない）。
fn fake_error() -> windows::core::Error {
    windows::core::Error::from(HRESULT(0x8007_0005u32 as i32))
}

/// 捕捉した出力から、指定タグを含む行をちょうど 1 本取り出す。
fn only_line_with<'a>(out: &'a str, tag: &str) -> &'a str {
    let found: Vec<&str> = out.lines().filter(|l| l.contains(tag)).collect();
    assert_eq!(found.len(), 1, "`{tag}` の行がちょうど 1 本ではない: {out}");
    found[0]
}

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 指令に現れる窓ハンドルを 1 つ残らず列挙する。
///
/// `SetWindowPos` へ渡る窓は 2 つ——動かす窓（`hwnd`）と挿入位置（`hwnd_insert_after`）
/// ——であり、他の 5 欄は整数とフラグである。欄が増えたらここへ足すこと（足し忘れれば
/// 「外の窓が現れない」の主張がその欄の分だけ効かなくなる）。
fn hwnds_named_by(cmd: &SetWindowPosCommand) -> Vec<HWND> {
    let mut named = vec![cmd.hwnd];
    named.extend(cmd.hwnd_insert_after);
    named
}

/// 撤去がすべて付与より前に並んでいること（§12.4 の実測手順そのもの）。
fn assert_every_detach_precedes_every_attach(ops: &[ChainOp]) {
    let last_detach = ops
        .iter()
        .rposition(|op| matches!(op, ChainOp::Detach { .. }));
    let first_attach = ops
        .iter()
        .position(|op| matches!(op, ChainOp::Attach { .. }));
    if let (Some(last), Some(first)) = (last_detach, first_attach) {
        assert!(
            last < first,
            "付与のあとに撤去が並んでいる（途中状態が壊れる手順）: {ops:?}"
        );
    }
}

// ===========================================================================
// 差分の 4 形——追加のみ・撤去のみ・張り替え・変化なし
// ===========================================================================

/// ① 追加のみ——帳簿が空なら、望む順のまま付与だけが並ぶ。
#[test]
fn adding_edges_to_an_empty_ledger_yields_attaches_in_the_declared_order() {
    let e = entities(3);
    let desired = [edge(e[0], e[1]), edge(e[1], e[2])];

    let ops = plan_chain_ops(&desired, &[]);

    assert_eq!(
        ops,
        vec![
            ChainOp::Attach {
                owned: e[0],
                owner: e[1]
            },
            ChainOp::Attach {
                owned: e[1],
                owner: e[2]
            },
        ]
    );
    assert_every_detach_precedes_every_attach(&ops);
}

/// ② 撤去のみ——望む鎖が空になれば、帳簿の全件が解除として外れる（要件 4.1／15.4）。
#[test]
fn dropping_the_whole_chain_tears_down_every_recorded_edge() {
    let e = entities(3);
    let current = [ledger(e[0], e[1], 0), ledger(e[1], e[2], 1)];

    let ops = plan_chain_ops(&[], &current);

    assert_eq!(
        ops,
        vec![
            ChainOp::Detach {
                owned: e[0],
                reason: DetachReason::Teardown
            },
            ChainOp::Detach {
                owned: e[1],
                reason: DetachReason::Teardown
            },
        ]
    );
    assert_every_detach_precedes_every_attach(&ops);
}

/// ③ 張り替え——同じ窓の相手が変わるとき、撤去は必ず付与より前に出る（要件 7.1）。
#[test]
fn rechaining_one_window_detaches_before_it_attaches() {
    let e = entities(3);
    let desired = [edge(e[0], e[2])];
    let current = [ledger(e[0], e[1], 0)];

    let ops = plan_chain_ops(&desired, &current);

    assert_eq!(
        ops,
        vec![
            ChainOp::Detach {
                owned: e[0],
                reason: DetachReason::Rechain
            },
            ChainOp::Attach {
                owned: e[0],
                owner: e[2]
            },
        ]
    );
    assert_every_detach_precedes_every_attach(&ops);
}

/// ④ 変化なし——望みと現況が同じなら操作は 1 つも出ない（要件 6.4・空振りの巡を作らない）。
#[test]
fn a_chain_that_already_matches_yields_no_operation_at_all() {
    let e = entities(3);
    let desired = [edge(e[0], e[1]), edge(e[1], e[2])];
    let current = [ledger(e[0], e[1], 0), ledger(e[1], e[2], 1)];

    let ops = plan_chain_ops(&desired, &current);

    assert!(ops.is_empty(), "変化が無いのに操作が出ている: {ops:?}");
}

/// 区間だけが変わっても操作は 1 つも出ない——差分は**所有関係**の差だけを見る。
///
/// 区間は記録のための欄であり、実行環境への指令とは無関係である。ここを見て操作を
/// 作ると、同じ並びのまま帰属が変わっただけの巡で鎖を外して張り直すことになる
/// （途中状態で鎖が切れる・後押しも無駄に出る）。
#[test]
fn a_segment_only_change_yields_no_operation_at_all() {
    let e = entities(3);
    // 現況の帳簿は `Group(0)`（`ledger` の既定）。望みは同じ端点で区間だけ `Tail`。
    let desired = [edge_in_tail(e[0], e[1]), edge_in_tail(e[1], e[2])];
    let current = [ledger(e[0], e[1], 0), ledger(e[1], e[2], 1)];

    assert_ne!(
        desired[0],
        edge(e[0], e[1]),
        "対照が成立していない（区間が同じなら何も主張できない）"
    );

    let ops = plan_chain_ops(&desired, &current);

    assert!(ops.is_empty(), "区間の差で操作を出している: {ops:?}");
}

/// スプライス（切る 1 本・張る 2 本）でも、撤去の塊が付与の塊より前にまとまる。
///
/// 4 形を 1 つずつ確かめても「混ざったときの並び」は言えない——撤去と付与が交互に出る
/// 実装でも 4 形の各テストは緑のままになりうる。ここで混在させて塊の順を固定する。
#[test]
fn a_splice_keeps_all_detaches_ahead_of_all_attaches() {
    let e = entities(4);
    // 現況: e0←e1（張り替え対象）と e2←e3（解除対象）。
    let current = [ledger(e[0], e[1], 0), ledger(e[2], e[3], 1)];
    // 望み: e0 の相手が e2 へ変わり、e1←e3 が新しく要る。
    let desired = [edge(e[0], e[2]), edge(e[1], e[3])];

    let ops = plan_chain_ops(&desired, &current);

    assert_eq!(
        ops,
        vec![
            ChainOp::Detach {
                owned: e[0],
                reason: DetachReason::Rechain
            },
            ChainOp::Detach {
                owned: e[2],
                reason: DetachReason::Teardown
            },
            ChainOp::Attach {
                owned: e[0],
                owner: e[2]
            },
            ChainOp::Attach {
                owned: e[1],
                owner: e[3]
            },
        ]
    );
    assert_every_detach_precedes_every_attach(&ops);
}

/// 望みの一部だけが変わっても、変わっていない繋ぎには操作が出ない（無駄な張り直しをしない）。
#[test]
fn an_unchanged_edge_is_left_alone_while_its_neighbour_is_rechained() {
    let e = entities(4);
    let current = [ledger(e[0], e[1], 0), ledger(e[1], e[2], 1)];
    let desired = [edge(e[0], e[1]), edge(e[1], e[3])];

    let ops = plan_chain_ops(&desired, &current);

    assert_eq!(
        ops,
        vec![
            ChainOp::Detach {
                owned: e[1],
                reason: DetachReason::Rechain
            },
            ChainOp::Attach {
                owned: e[1],
                owner: e[3]
            },
        ]
    );
}

/// 出力に、同じ被所有側への付与が 2 本現れない（要件 14.4——星形・分岐を作らない）。
///
/// 望む列に同じ窓が 2 度現れたら、鎖の**先頭に近い方**だけを採る。2 本出せば同じ窓に
/// 2 つの所有者を主張することになり、鎖が一直線でなくなる。
#[test]
fn no_window_is_ever_attached_twice_in_one_plan() {
    let e = entities(3);
    let desired = [edge(e[0], e[1]), edge(e[0], e[2])];

    let ops = plan_chain_ops(&desired, &[]);

    assert_eq!(
        ops,
        vec![ChainOp::Attach {
            owned: e[0],
            owner: e[1]
        }]
    );

    // 一般形でも数える（この 1 例だけを特別扱いする実装で緑にならないように）。
    let mut attached: Vec<Entity> = Vec::new();
    for op in &ops {
        if let ChainOp::Attach { owned, .. } = op {
            assert!(
                !attached.contains(owned),
                "同じ窓へ 2 本の付与が出ている: {ops:?}"
            );
            attached.push(*owned);
        }
    }
}

// ===========================================================================
// 後押しの選定——自分の窓 2 枚だけ
// ===========================================================================

/// 錨の直後が根でない巡は、**根を錨（1 つ手前の窓）の直後へ**差し直す（§13 実測 10）。
///
/// 動かすのが根であることが要点である——鎖の中で他の窓を所有しているのは根であり、
/// 所有側が動いたときにだけ Windows は被所有側を引き連れて並べ直す。先頭は誰も
/// 所有していないので、先頭を動かしても鎖は並ばない（実測 24 通り中 0 通り）。
#[test]
fn the_nudge_reinserts_the_root_right_behind_its_anchor() {
    let members = [fake_hwnd(0x10), fake_hwnd(0x20), fake_hwnd(0x30)];

    // 錨（`members[1]`）の直後は根ではない＝素直な形が本物の位置変更になる巡。
    let cmd =
        nudge_command(&members, Some(fake_hwnd(0xAA))).expect("窓が 2 枚以上あれば後押しが出る");

    assert_eq!(cmd.hwnd, members[2], "動かす窓が鎖の根でない");
    assert_eq!(
        cmd.hwnd_insert_after,
        Some(members[1]),
        "挿入位置が根の 1 つ手前の窓（錨）でない"
    );
}

/// 錨の直後が**まだ何も無い**巡（`None`）も、素直な形を採る。
#[test]
fn a_missing_raw_neighbour_still_takes_the_plain_shape() {
    let members = [fake_hwnd(0x10), fake_hwnd(0x20), fake_hwnd(0x30)];

    let cmd = nudge_command(&members, None).expect("窓が 2 枚以上あれば後押しが出る");

    assert_eq!(cmd.hwnd, members[2], "動かす窓が鎖の根でない");
    assert_eq!(
        cmd.hwnd_insert_after,
        Some(members[1]),
        "隣が無い巡で挿入位置が錨から外れている"
    );
}

/// 錨の直後が既に根なら、**挿入位置を先頭へ**切り替える（空振りを塞ぐ・要件 1.1／1.2）。
///
/// 素直な形は「根を錨の直後へ」だが、根が既に錨の生の直後に居ると、その要求は現在位置と
/// 同じであり `SetWindowPos` は重なりを 1 ミリも動かさない＝完全な空振りになる。適用系は
/// 後押しの前に印を降ろし再試行経路を持たないので、1 度空振りすると鎖は収まらないまま
/// 固まる（2026-08-30 に実窓検証で掘り当てた本番欠陥）。
#[test]
fn a_redundant_insert_position_switches_to_the_head_of_the_chain() {
    let members = [fake_hwnd(0x10), fake_hwnd(0x20), fake_hwnd(0x30)];

    // 錨（`members[1]`）の生の直後が根（`members[2]`）＝素直な形が空振りする巡。
    let cmd = nudge_command(&members, Some(members[2])).expect("窓が 2 枚以上あれば後押しが出る");

    assert_eq!(cmd.hwnd, members[2], "動かす窓が鎖の根でない");
    assert_eq!(
        cmd.hwnd_insert_after,
        Some(members[0]),
        "空振りする巡で挿入位置が先頭へ切り替わっていない"
    );
}

/// 2 つの形が同時に空振りすることはない（＝どちらか一方は必ず本物の位置変更である）。
///
/// これが本 task の是正の要である。「Windows は隣接なら省略する」という**未文書の性質**に
/// 賭けるのではなく、「要求位置が現在位置と違えば Z は動く」という `SetWindowPos` の基本の
/// 性質だけに乗る。根の生の 1 つ手前は高々 1 枚なので、錨と先頭が別の窓である限り
/// 2 つの要求が同時に「現在位置と同じ」にはならない。
#[test]
fn the_two_insert_positions_can_never_both_be_redundant() {
    let members = [
        fake_hwnd(0x10),
        fake_hwnd(0x20),
        fake_hwnd(0x30),
        fake_hwnd(0x40),
    ];
    let root = members[3];
    let anchor = members[2];
    let head = members[0];

    // ⑴ 錨の直後が根＝素直な形が空振りする巡 → 挿入位置は先頭（根の直後ではない）。
    let redundant = nudge_command(&members, Some(root)).expect("後押しが出る");
    assert_eq!(redundant.hwnd_insert_after, Some(head));
    assert_ne!(
        redundant.hwnd_insert_after,
        Some(anchor),
        "空振りする挿入位置をそのまま採っている"
    );

    // ⑵ 錨の直後が根でない巡 → 挿入位置は錨（こちらが本物の位置変更）。
    let plain = nudge_command(&members, Some(head)).expect("後押しが出る");
    assert_eq!(plain.hwnd_insert_after, Some(anchor));

    // 2 つの形は必ず別の挿入位置を名指しする（3 枚以上のとき錨と先頭は別の窓）。
    assert_ne!(
        redundant.hwnd_insert_after, plain.hwnd_insert_after,
        "2 つの形が同じ挿入位置に畳まれている（空振りを塞げない）"
    );
}

/// 窓が 2 枚のときは錨と先頭が同じ窓なので、2 択は 1 つに畳まれる。
///
/// 畳まれても困らない——そのとき空振りするのは「根が先頭の直後に居る」＝**既に望む
/// 並びである**場合だけであり、収めるものが無いからである。
#[test]
fn a_two_window_chain_collapses_both_insert_positions_into_one() {
    let members = [fake_hwnd(0x10), fake_hwnd(0x20)];

    let plain = nudge_command(&members, Some(fake_hwnd(0xAA))).expect("後押しが出る");
    let redundant = nudge_command(&members, Some(members[1])).expect("後押しが出る");

    assert_eq!(plain.hwnd, members[1], "動かす窓が根でない");
    assert_eq!(redundant.hwnd, members[1], "動かす窓が根でない");
    assert_eq!(plain.hwnd_insert_after, Some(members[0]));
    assert_eq!(
        redundant.hwnd_insert_after,
        Some(members[0]),
        "2 枚のときに存在しない 3 枚目を挿入位置にしている"
    );
}

/// 窓が 2 枚未満なら後押しを出さない（張るべき繋ぎも 1 本も無い）。
#[test]
fn fewer_than_two_windows_produce_no_nudge_at_all() {
    assert!(
        nudge_command(&[], None).is_none(),
        "窓 0 枚で後押しが出ている"
    );
    assert!(
        nudge_command(&[fake_hwnd(0x10)], None).is_none(),
        "窓 1 枚で後押しが出ている"
    );
    assert!(
        nudge_command(&[fake_hwnd(0x10), fake_hwnd(0x20)], None).is_some(),
        "境界の反対側（窓 2 枚）で後押しが消えている"
    );
}

/// **後押しの指令に、自分の鎖の窓以外の窓ハンドルが 1 つも現れない**。
///
/// 禁じ手（他プロセスの窓を挿入位置に渡す形・絶対帯の指定）はどれも
/// 「`members` の外の値が指令に載る」形で現れるので、指令が名指しする窓の集合を丸ごと
/// 調べる。**2 択のどちらの枝でも**調べる——切り替えた先が鎖の外を指していては意味が無い。
#[test]
fn the_nudge_command_only_ever_names_the_chains_own_windows() {
    let members = [
        fake_hwnd(0x10),
        fake_hwnd(0x20),
        fake_hwnd(0x30),
        fake_hwnd(0x40),
    ];

    for (label, anchor_next) in [
        ("素直な枝", Some(fake_hwnd(0xAA))),
        ("切り替えた枝", Some(members[3])),
    ] {
        let cmd = nudge_command(&members, anchor_next).expect("窓が 2 枚以上あれば後押しが出る");
        let named = hwnds_named_by(&cmd);

        assert!(
            !named.is_empty(),
            "{label}: 指令が窓を 1 つも名指ししていない（検査が空振りしている）"
        );
        for hwnd in &named {
            assert!(
                members.contains(hwnd),
                "{label}: 鎖の外の窓ハンドルが後押しの指令に現れている: {hwnd:?} / members={members:?}"
            );
        }
        // 参照するのは根と挿入位置の 2 枚だけ（鎖の他の窓は引数に取らない）。
        assert_eq!(
            named.len(),
            2,
            "{label}: 指令が名指しする窓が 2 枚でない: {named:?}"
        );
        assert!(
            !named.contains(&members[1]),
            "{label}: 根と挿入位置以外の窓を参照している: {named:?}"
        );
    }
}

/// 上の検査そのものが、外から来た窓を実際に掴めることを示す（道具の較正）。
///
/// 「無い」の主張は道具が壊れていても緑になる。禁じ手 2 種を実際に組み立て、同じ検査が
/// **必ず赤にする**ことをここで確かめる。
#[test]
fn the_same_check_actually_catches_the_forbidden_nudge_shapes() {
    let members = [fake_hwnd(0x10), fake_hwnd(0x20)];
    let cmd = nudge_command(&members, None).expect("窓が 2 枚以上あれば後押しが出る");

    // 禁じ手⑴: 他プロセスの窓（`GW_HWNDPREV` で拾ったもの）を挿入位置に渡す形。
    let foreign = SetWindowPosCommand::new(
        members[0],
        0,
        0,
        0,
        0,
        cmd.flags,
        Some(fake_hwnd(0xDEAD_BEEF)),
    );
    assert!(
        !hwnds_named_by(&foreign).iter().all(|h| members.contains(h)),
        "他プロセスの窓を渡した指令を検査が素通りさせている"
    );

    // 禁じ手⑵: 絶対帯の指定（`HWND_TOP` は数値 0 の擬似ハンドル）。
    let absolute_band = SetWindowPosCommand::new(
        members[0],
        0,
        0,
        0,
        0,
        cmd.flags,
        Some(HWND(std::ptr::null_mut())),
    );
    assert!(
        !hwnds_named_by(&absolute_band)
            .iter()
            .all(|h| members.contains(h)),
        "絶対帯の指定を検査が素通りさせている"
    );
}

/// 後押しは重なりだけを動かす——位置・寸法・活性化を伴わない（要件 11.1）。
///
/// **2 択のどちらの枝でも**同じであること（切り替えた先が窓を動かす形になっていない）。
#[test]
fn the_nudge_moves_only_the_zorder() {
    let members = [fake_hwnd(0x10), fake_hwnd(0x20), fake_hwnd(0x30)];

    for (label, anchor_next) in [("素直な枝", None), ("切り替えた枝", Some(members[2]))] {
        let cmd = nudge_command(&members, anchor_next).expect("窓が 2 枚以上あれば後押しが出る");

        assert_eq!(
            cmd.flags,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            "{label}: 重なり以外を動かすフラグが立っている: {:?}",
            cmd.flags
        );
        assert_eq!(
            (cmd.x, cmd.y, cmd.width, cmd.height),
            (0, 0, 0, 0),
            "{label}: 位置・寸法の欄に値が入っている"
        );
        // Z を触らない指令（`SWP_NOZORDER`）では収まらない＝§12.2 実測 7。
        assert_eq!(
            cmd.flags & SET_WINDOW_POS_FLAGS(0x0004),
            SET_WINDOW_POS_FLAGS(0),
            "{label}: Z を触らない指令になっている（後押しとして効かない形）"
        );
    }
}

/// 後押しは遅延キューへ積まず、追加の起床も要求しない（要件 14.5／14.2）。
///
/// 積む経路（`SetWindowPosCommand::enqueue`）は内部で起床の印を立てる。後押しをそこへ
/// 通すと「書いたあとに次の巡を促す」形になり、退役した反復是正の機構が裏口から戻る。
#[test]
fn the_nudge_is_built_without_the_deferred_queue_or_a_wake_request() {
    let here = code_only(include_str!("zorder_chain.rs"));

    assert!(
        !here.contains("SetWindowPosCommand::enqueue"),
        "後押しが遅延キューへ積まれている（同じ巡で実測できない・要件 9.2）"
    );
    assert!(
        !here.contains("tick_wake"),
        "適用の側から起床を促している（要件 14.2 が禁じた形）"
    );

    // 対照: 同じ針が、実際に積む／促す既存経路では必ず当たる（走査の空振り検出）。
    assert!(
        code_only(include_str!("zorder_pair_maintain.rs")).contains("SetWindowPosCommand::enqueue"),
        "走査そのものが壊れている（積む既存経路を見つけられない）"
    );
    assert!(
        code_only(include_str!("command.rs")).contains("tick_wake"),
        "走査そのものが壊れている（起床を促す既存経路を見つけられない）"
    );
}

// ===========================================================================
// 記録の出口——鎖系 7 語はこのモジュールから出る
// ===========================================================================

/// 鎖系 7 語はすべて本モジュールの出力先から出る（grep の対象が割れない）。
#[test]
fn every_chain_record_leaves_from_this_module_not_from_the_line_builders() {
    let e = entities(2);
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        log_chain_linked(
            Some(ChainSegment::Group(1)),
            e[0],
            e[1],
            Some(fake_hwnd(0x10)),
            Some(fake_hwnd(0x20)),
            1,
            2,
        );
        log_chain_unlinked(
            Some(ChainSegment::Tail),
            e[0],
            Some(fake_hwnd(0x10)),
            Some(fake_hwnd(0x20)),
            DetachReason::Teardown,
        );
        log_chain_settled(
            Some(fake_hwnd(0x10)),
            Some(fake_hwnd(0x20)),
            &[fake_hwnd(0x10), fake_hwnd(0x20)],
            &[fake_hwnd(0x10), fake_hwnd(0x20)],
            Some(true),
        );
        log_chain_absent(3, "b0");
        log_chain_skipped(ChainSkipReason::NoChange);
        log_chain_link_failed(
            Some(ChainSegment::Group(0)),
            Some(fake_hwnd(0x10)),
            Some(fake_hwnd(0x20)),
            &fake_error(),
        );
        log_chain_unlink_failed(Some(fake_hwnd(0x10)), &fake_error());
    });

    for tag in chain_record_tags() {
        let line = only_line_with(&out, tag);
        assert!(
            line.contains(CHAIN_TARGET),
            "`{tag}` の出力先が鎖のモジュールでない: {line}"
        );
        assert!(
            !line.contains(DIAG_TARGET),
            "`{tag}` が行組立の層から出ている（記録の出口が割れている）: {line}"
        );
    }
}

/// 保全語彙 2 語だけは行組立の層から出る——そしてサインオフの 1 本の指定が両方を点灯させる。
///
/// 直上のテストは「行組立の層から出ていない」を主張するので、`DIAG_TARGET` の針が
/// そもそも当たらない綴りでも緑になる。ここで**実際に当たる**側を併置して較正する。
/// あわせて、`…::zorder_chain=debug` という 1 本の指定が前方一致で
/// `…::zorder_chain_diag` も点けることを固定する（要件 9.5 の保全が割れない根拠）。
#[test]
fn the_preserved_group_records_still_leave_from_the_line_builder_module() {
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        log_group_rejected("ModeMixed", "b1,0");
        log_chain_skipped(ChainSkipReason::NoChange);
    });

    let preserved = only_line_with(&out, "[zorder-group] rejected");
    assert!(
        preserved.contains(DIAG_TARGET),
        "保全語彙の住処が行組立の層から動いている: {preserved}"
    );

    let chain = only_line_with(&out, "[zorder-chain] skipped");
    assert!(
        chain.contains(CHAIN_TARGET) && !chain.contains(DIAG_TARGET),
        "鎖の記録が行組立の層から出ている: {chain}"
    );
}

/// 鎖系のマクロ呼出は本モジュールにしかない（行組立の層には保全語彙 2 語ぶんだけが残る）。
///
/// 出力先の主張は「今この瞬間どこから出たか」しか言わない。**別の場所からも出せる**形が
/// 残っていれば、後から 2 つ目の出口が生えたときに誰も赤くならない。ここではマクロ呼出の
/// 数そのものを両側で数え、出口が増えた瞬間に赤くする。
#[test]
fn the_tracing_macros_for_the_chain_have_exactly_one_home() {
    let here = code_only(include_str!("zorder_chain.rs"));
    let diag = code_only(include_str!("zorder_chain_diag.rs"));

    // こちら: debug 5 本（linked・unlinked・settled・absent・skipped）＋ error 2 本。
    assert_eq!(
        here.matches("debug!(").count(),
        5,
        "鎖の debug 記録の本数が 5 本でない"
    );
    assert_eq!(
        here.matches("error!(").count(),
        2,
        "鎖の error 記録の本数が 2 本でない"
    );

    // あちら: 保全語彙 2 語ぶん（debug 1・warn 1）だけ。鎖の記録は 1 本も出さない。
    assert_eq!(
        diag.matches("debug!(").count(),
        1,
        "行組立の層の debug 記録が保全語彙 1 語ぶんでない"
    );
    assert_eq!(
        diag.matches("warn!(").count(),
        1,
        "行組立の層の warn 記録が保全語彙 1 語ぶんでない"
    );
    assert_eq!(
        diag.matches("error!(").count(),
        0,
        "行組立の層が error 記録を出している（鎖の失敗語が二重の住処を持つ）"
    );

    // 対照: 数える針が本当に当たっている（0 を数えて緑になる走査ではない）。
    assert!(
        here.contains("debug!(") && here.contains("error!("),
        "走査そのものが空振りしている"
    );
}

/// 鎖系のタグ定数は行組立の層にしかない——本モジュールは字面を持たない。
///
/// タグを 2 箇所に持つと、片方だけを直したときに実機の grep が静かに片肺になる。
#[test]
fn the_chain_tag_spellings_stay_in_the_line_builder_module() {
    let here = code_only(include_str!("zorder_chain.rs"));

    assert!(
        !here.contains("[zorder-chain]"),
        "鎖のタグの字面が記録の出口にも書かれている（住処が 2 つに割れる）"
    );
    assert!(
        !here.contains("[zorder-group]"),
        "保全語彙のタグの字面が鎖のモジュールに書かれている"
    );

    // 対照: 同じ針が、字面を持つ層では必ず当たる。
    let diag = code_only(include_str!("zorder_chain_diag.rs"));
    for tag in chain_record_tags() {
        assert!(
            diag.contains(&format!("\"{tag}\"")),
            "行組立の層に `{tag}` の字面が無い（走査が壊れている）"
        );
    }
}
