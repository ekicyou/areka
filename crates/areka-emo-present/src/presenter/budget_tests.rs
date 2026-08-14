//! `FrameBudget`（budget.rs）の単体檻。
//!
//! 本ファイルが固定するのは **確保計数シームの意味論**（Requirement 1.3・design.md
//! §FrameBudget・D6）である。すなわち
//!
//! - 確保が起きたと申告されたときにだけカウンタが増える（申告なしでは増えない）
//! - 発生点とフィールドが 1:1 で対応する（取り違えると別のフィールドが動く）
//! - 適用単位の増分は [`FrameBudget::take_delta`] の取り出しでリセットされる
//! - 累積は取り出しでリセットされず、run 全体を通して増え続ける
//!
//! の 4 点である。増分は perf サマリ行の `alloc_*` フィールドへそのまま載り（Requirement 1.3）、
//! 累積はテストが「定常状態で新規確保が起きていない」を主張するための読み取り口になる。
//!
//! # 席の再利用は本段の対象外
//!
//! 現時点の [`FrameBudget`] は**計数だけ**を持ち、再利用席（合成先・リサンプル作業領域・
//! マスク輪番）は後段が同じ器へ足す。ゆえに本檻は「確保が起きたか」を呼び手の申告として扱う。
//! 席が入った後は同じシームが「再利用が成立したか」を計数する側へ回るが、
//! ここで固定した計数の意味論（増分と累積の関係）はそのとき変わらない。
//!
//! # 実時間を合否条件に使わない（Requirement 6.2）
//!
//! 本檻は時刻にも実行速度にも一切触れない（純粋な計数の代数のみ）。

use super::*;

/// 発生点の全数（design.md §FrameBudget の席一覧と 1:1）。
///
/// 4 つ未満へ縮んだら perf サマリ行のフィールドが欠け、判定スクリプトの契約が壊れる。
#[test]
fn every_allocation_site_is_enumerated() {
    assert_eq!(
        AllocSite::ALL.len(),
        4,
        "確保発生点は 4 つ（合成先・表示バッファ・リサンプル作業領域・当たり判定マスク）"
    );
}

/// Requirement 1.3 観測完了: 新品の器は増分・累積とも全て 0。
///
/// 「初期値が 0 でない」実装は、定常状態ゼロの主張（Requirement 3.1）を最初から成立不能にする。
#[test]
fn a_fresh_budget_counts_nothing() {
    let budget = FrameBudget::new();

    assert_eq!(
        *budget.cumulative(),
        BudgetCounters::default(),
        "新品の累積は全て 0"
    );
    for site in AllocSite::ALL {
        assert_eq!(budget.cumulative().count(site), 0, "{site:?} の累積が 0 でない");
    }
}

/// Requirement 1.3 観測完了: **確保が起きたときだけ**カウンタが増える。
///
/// 申告のない適用（＝是正後の定常状態が目指す形）では、何度増分を取り出しても全フィールドが
/// 0 のままである。これは「呼ばれるたびに数える」誤実装（例: 取得シームの入口で無条件に
/// 増やす）を殺す——それでは再利用の成立・不成立が区別できず、計数が判定材料にならない。
#[test]
fn nothing_is_counted_without_an_allocation() {
    let mut budget = FrameBudget::new();

    for _ in 0..8 {
        assert_eq!(
            budget.take_delta(),
            BudgetDelta::default(),
            "確保の申告が無い適用の増分は 0（定常状態の形）"
        );
    }
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters::default(),
        "確保の申告が無ければ累積も動かない"
    );
}

/// Requirement 1.3 観測完了: 発生点とフィールドが 1:1 で対応する（取り違えを殺す）。
///
/// 各発生点を 1 回だけ申告し、⑴対応する**名前つきフィールド**が 1 になる ⑵他の 3 フィールドが
/// 0 のままである、を全発生点について確認する。フィールドの割り当てを 1 組でも入れ替えると
/// （例: `Xmap` が `alloc_mask` を増やす）、この檻は必ず赤になる。
#[test]
fn each_site_increments_only_its_own_field() {
    for site in AllocSite::ALL {
        let mut budget = FrameBudget::new();
        budget.note_alloc(site);
        let delta = budget.take_delta();

        // 名前つきフィールドで直接受ける（`count` の実装が壊れていても素通ししない）。
        let named = match site {
            AllocSite::ComposeDst => delta.alloc_compose_dst,
            AllocSite::ResampleDst => delta.alloc_resample_dst,
            AllocSite::Xmap => delta.alloc_xmap,
            AllocSite::Mask => delta.alloc_mask,
        };
        assert_eq!(named, 1, "{site:?} の申告が対応フィールドへ載っていない: {delta:?}");

        for other in AllocSite::ALL {
            if other != site {
                assert_eq!(
                    delta.count(other),
                    0,
                    "{site:?} の申告が無関係な発生点 {other:?} を動かした: {delta:?}"
                );
            }
        }
    }
}

/// Requirement 1.3 観測完了: 増分は取り出しでリセットされ、累積はリセットされない。
///
/// 適用 3 回分を演じる:
/// - 適用 1: 合成先 1 回・マスク 2 回の確保（同一発生点の複数回申告が足し合わされる）
/// - 適用 2: 確保なし（定常状態）→ 増分は全 0・累積は据え置き
/// - 適用 3: 表示バッファ 1 回 → 増分に現れ、累積は 1・2 を保ったまま伸びる
///
/// # 殺す誤実装
///
/// - `take_delta` がリセットしない → 適用 2 の増分に適用 1 の値が残り赤
/// - `take_delta` が累積まで巻き戻す → 適用 2 の累積 assert で赤
/// - 増分と累積が同一の器を共有している → どちらかの assert で必ず赤
#[test]
fn take_delta_resets_the_increment_while_the_cumulative_keeps_growing() {
    let mut budget = FrameBudget::new();

    // 適用 1: 初回確保（寸法確定前の一度きりの形）。
    budget.note_alloc(AllocSite::ComposeDst);
    budget.note_alloc(AllocSite::Mask);
    budget.note_alloc(AllocSite::Mask);
    let first = budget.take_delta();
    assert_eq!(
        first,
        BudgetDelta {
            alloc_compose_dst: 1,
            alloc_resample_dst: 0,
            alloc_xmap: 0,
            alloc_mask: 2,
        },
        "同一発生点の複数回申告は足し合わされること"
    );
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 1,
            alloc_resample_dst: 0,
            alloc_xmap: 0,
            alloc_mask: 2,
        },
        "累積は増分と同じ値から始まる"
    );

    // 適用 2: 確保なし（定常状態）。増分 0・累積据え置き。
    assert_eq!(
        budget.take_delta(),
        BudgetDelta::default(),
        "取り出し済みの増分が次の適用へ持ち越されている（リセット漏れ）"
    );
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 1,
            alloc_resample_dst: 0,
            alloc_xmap: 0,
            alloc_mask: 2,
        },
        "累積は増分の取り出しでリセットされないこと"
    );

    // 適用 3: 表示バッファの再確保（寸法変化の形）。
    budget.note_alloc(AllocSite::ResampleDst);
    let third = budget.take_delta();
    assert_eq!(third.alloc_resample_dst, 1, "適用 3 の増分");
    assert_eq!(third.alloc_compose_dst, 0, "適用 3 で申告していない発生点は 0");
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 1,
            alloc_resample_dst: 1,
            alloc_xmap: 0,
            alloc_mask: 2,
        },
        "累積は run 全体で積み上がること"
    );
}

/// 増分と累積の整合: 適用ごとに取り出した増分の総和が、累積と一致する。
///
/// これは「累積だけ・増分だけ」を数える片肺実装を殺す（片方が動かなければ総和が食い違う）。
/// 判定スクリプトは行ごとの増分を、テストは累積を根拠にするため、両者が同じ事実を指していない
/// 状態は静かな誤診断を生む。
#[test]
fn the_sum_of_per_apply_deltas_equals_the_cumulative() {
    let mut budget = FrameBudget::new();
    let program: [&[AllocSite]; 4] = [
        &[AllocSite::ComposeDst, AllocSite::Xmap],
        &[],
        &[AllocSite::Mask, AllocSite::Mask, AllocSite::ResampleDst],
        &[AllocSite::Xmap],
    ];

    let mut summed = [0_u64; 4];
    for apply in program {
        for site in apply {
            budget.note_alloc(*site);
        }
        let delta = budget.take_delta();
        for site in AllocSite::ALL {
            summed[site as usize] += u64::from(delta.count(site));
        }
    }

    for site in AllocSite::ALL {
        assert_eq!(
            budget.cumulative().count(site),
            summed[site as usize],
            "{site:?}: 増分の総和と累積が食い違っている"
        );
    }
    // 台本の実数（発生点ごとの申告回数）。総和どうしの空虚な一致を防ぐ。
    assert_eq!(budget.cumulative().count(AllocSite::ComposeDst), 1);
    assert_eq!(budget.cumulative().count(AllocSite::ResampleDst), 1);
    assert_eq!(budget.cumulative().count(AllocSite::Xmap), 2);
    assert_eq!(budget.cumulative().count(AllocSite::Mask), 2);
}
