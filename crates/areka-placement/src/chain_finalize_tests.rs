// =============================================================================
// 実表示寸での連鎖再解決の檻（areka-P0-scope-chain-gap 要件 7.1/7.2/7.3・design C6）
//
// 判定は純関数ゆえ GPU も World も要さない。物理 px を直接与える。規則の網羅
// （組ごとの決め・基準・非正寸・空入力・極端な値）は chain_finalize_resolve_tests.rs が持ち、
// ここは実機の再現・欠陥の式の否定・2 度目は何もしない・停滞の診断だけを置く。
// =============================================================================

use super::*;

/// 未接触スコープを組む補助（現在位置＝既定位置）。
fn untouched(scope: usize, x: i32, w: i32) -> ScopeChainState {
    ScopeChainState {
        scope,
        current_x: x,
        width: w,
        default_x: Some(x),
    }
}

/// 明示的に再配置されたスコープを組む補助（現在位置≠既定位置）。
fn repositioned(scope: usize, current_x: i32, w: i32, default_x: i32) -> ScopeChainState {
    ScopeChainState {
        scope,
        current_x,
        width: w,
        default_x: Some(default_x),
    }
}

/// 起動の並べ直しを印なしで通す（台本の印・組・覚えた印が無い形の
/// `finalize_chain_once_with` と同じ順: [`boot_pair_decision`] → [`resolve_chain`]）。
fn boot_chain(states: &[ScopeChainState]) -> Vec<ChainMove> {
    let slots: Vec<ChainSlot> = states
        .iter()
        .map(|s| ChainSlot {
            scope: s.scope,
            current_x: s.current_x,
            width: s.width,
            decision: boot_pair_decision(s, false, false, None),
        })
        .collect();
    resolve_chain(&slots)
}

/// 指示を適用した後の X 列を作る（事後条件の検証用）。
fn apply(states: &[ScopeChainState], moves: &[ChainMove]) -> Vec<i32> {
    states
        .iter()
        .map(|s| {
            moves
                .iter()
                .find(|m| m.scope == s.scope)
                .map_or(s.current_x, |m| m.new_x)
        })
        .collect()
}

// -----------------------------------------------------------------------------
// 本丸: 実機で観測した機序をそのまま檻へ入れる
// -----------------------------------------------------------------------------

/// 実機再現（emo2・拡大率 200%）: scope0 が起動面 868 で配置された後、実表示面 764 へ
/// 縮んで下端中央固定で再アンカーされ左端が 2012→2064 へ 52 右寄りする。連鎖が
/// 再計算されないと scope1 との間に 52px の隙間が残る——本檻はその隙間を 0 へ戻す
/// 指示が出ることを固定する。
///
/// scope0 は連鎖の起点ゆえ**動かさない**（接地点は正しい・7.2）。動くのは scope1 だけ。
#[test]
fn emo2_surface_swap_gap_is_closed_by_moving_the_follower_only() {
    let states = [
        // 再アンカー後の実位置 2064（既定は 2012）・実表示幅 764
        repositioned(0, 2064, 764, 2012),
        // 未接触のまま（配置時の 1340）・実表示幅 672
        untouched(1, 1340, 672),
    ];

    let moves = boot_chain(&states);

    assert_eq!(
        moves.len(),
        1,
        "動かすのは後続スコープだけ（起点は不動・7.2）"
    );
    assert_eq!(
        moves[0],
        ChainMove {
            scope: 1,
            new_x: 1392
        },
        "new_x = 2064 − 672 = 1392（前スコープ左端 − 自スコープ幅。据え置きの 1340 だと 52px の隙間が残る）"
    );
}

/// 欠陥式（前スコープの幅を引く）へ戻ってはならない。不等幅でのみ判別できる。
#[test]
fn previous_width_subtraction_is_rejected() {
    let states = [untouched(0, 1000, 400), untouched(1, 0, 320)];

    let moves = boot_chain(&states);
    let xs = apply(&states, &moves);

    assert_eq!(
        xs[1], 680,
        "自スコープの幅を引く（1000 − 320）。前スコープの幅を引く旧式（1000 − 400 = 600）へ戻ってはならない"
    );
}

/// 二度目の適用は空になる（一度きりの確定という結線側の契約を、判定側でも壊さない）。
#[test]
fn applying_twice_is_a_no_op_the_second_time() {
    let states = [untouched(0, 1000, 400), untouched(1, 0, 320)];
    let moves = boot_chain(&states);
    assert!(!moves.is_empty(), "一度目は指示が出る");

    let xs = apply(&states, &moves);
    let settled = [untouched(0, xs[0], 400), untouched(1, xs[1], 320)];
    assert!(
        boot_chain(&settled).is_empty(),
        "確定後の状態を再投入しても指示は出ない"
    );
}

// -----------------------------------------------------------------------------
// 確定が見送られ続けたときの一発診断（areka-P0-scope-chain-gap 6.5）
// -----------------------------------------------------------------------------

/// 有界の待ちを超えた**ちょうど 1 回**だけ報告し、それ以前も以後も黙る。
///
/// 「毎フレームの見送りは無音・停滞し続けても出力は 1 行」という 6.5 の核をここで固定する
/// （結線側の檻は同じ性質を実ログ捕捉で確かめる）。
#[test]
fn chain_deferral_reports_exactly_once_at_the_bounded_wait() {
    let mut stall = ChainFinalizeStall::default();

    // 閾値の手前までは 1 度も報告しない。
    for frame in 1..CHAIN_FINALIZE_STALL_FRAMES {
        assert!(
            !note_chain_deferral(&mut stall),
            "閾値未満（{frame} フレーム目）で報告してはならない"
        );
    }
    assert!(!stall.reported, "まだ報告していない");

    // 閾値に到達したフレームで 1 度だけ報告する。
    assert!(
        note_chain_deferral(&mut stall),
        "閾値に到達したフレームで報告する"
    );
    assert!(stall.reported, "報告済みの標識が立つ");
    assert_eq!(
        stall.deferrals, CHAIN_FINALIZE_STALL_FRAMES,
        "報告時点の見送り数＝閾値"
    );

    // 以後は停滞が続いても黙り、計数も進めない（同じ停滞で溢れさせない）。
    for _ in 0..(CHAIN_FINALIZE_STALL_FRAMES * 2) {
        assert!(!note_chain_deferral(&mut stall), "二度目以降は報告しない");
    }
    assert_eq!(
        stall.deferrals, CHAIN_FINALIZE_STALL_FRAMES,
        "報告後は数えもしない"
    );
}

/// 診断の本文が「どのスコープが」「どの条件で」を名指しする（無内容な一行にしない・6.5）。
#[test]
fn defer_reason_names_the_scope_and_the_condition() {
    let landing = ChainDeferReason::ResnapNotLanded {
        scope: 1,
        shown: (764, 1094),
        window: (868, 1094),
    };
    assert_eq!(landing.scope(), Some(1), "障害の在り処はスコープ 1");
    let text = landing.to_string();
    assert!(text.contains("scope 1"), "スコープを名指しする: {text}");
    assert!(text.contains("764"), "実表示寸を載せる: {text}");
    assert!(text.contains("868"), "窓寸を載せて食い違いを示す: {text}");

    // 全理由が本文を持つ（Display の取りこぼしを塞ぐ）。
    for reason in ALL_REASONS {
        assert!(!reason.to_string().is_empty(), "理由 {reason:?} の本文が空");
    }
}

/// 見送り理由ごとの、[`ALL_REASONS`] での番号と固定語（観測レコードの `reason=`）と、
/// スコープを持つか（世界全体に関わる理由は持たない＝偽のスコープ番号を作らない）。
///
/// 網羅の `match` ゆえ、理由を 1 つ足すとここが組めなくなる。番号を振る人は [`ALL_REASONS`]
/// へも同じ位置に足す（番号が列の位置と合わなければ下のテストが赤になる）。
fn expected(reason: ChainDeferReason) -> (usize, &'static str, bool) {
    match reason {
        ChainDeferReason::NoGhostWindows => (0, "no-ghost-windows", false),
        ChainDeferReason::NoScopes => (1, "no-scopes", false),
        ChainDeferReason::NoCharWindow { .. } => (2, "no-char-window", true),
        ChainDeferReason::NotShownYet { .. } => (3, "not-shown-yet", true),
        ChainDeferReason::UnusableShownSize { .. } => (4, "unusable-shown-size", true),
        ChainDeferReason::NoWindowPos { .. } => (5, "no-window-pos", true),
        ChainDeferReason::IncompleteWindowPos { .. } => (6, "incomplete-window-pos", true),
        ChainDeferReason::ResnapNotLanded { .. } => (7, "resnap-not-landed", true),
        ChainDeferReason::DpiSyncHeld { .. } => (8, "dpi-sync-held", true),
    }
}

/// 見送り理由の全部（[`expected`] の番号の順）。
const ALL_REASONS: [ChainDeferReason; 9] = [
    ChainDeferReason::NoGhostWindows,
    ChainDeferReason::NoScopes,
    ChainDeferReason::NoCharWindow { scope: 0 },
    ChainDeferReason::NotShownYet { scope: 0 },
    ChainDeferReason::UnusableShownSize {
        scope: 0,
        w: 0,
        h: 0,
    },
    ChainDeferReason::NoWindowPos { scope: 0 },
    ChainDeferReason::IncompleteWindowPos { scope: 0 },
    ChainDeferReason::ResnapNotLanded {
        scope: 0,
        shown: (1, 1),
        window: (2, 2),
    },
    ChainDeferReason::DpiSyncHeld { scope: 0 },
];

/// 見送り理由の**固定語**（観測レコードの `reason=`）が全理由そろい、互いに異なる
/// （`areka-P0-dpi-transition-atomicity` 設計 Data Models の `chain` 行）。
///
/// [`ALL_REASONS`] の番号が 0〜N−1 をちょうど 1 度ずつ埋め（列の位置＝[`expected`] の番号）、
/// 各理由の語とスコープの有無が表と等しいことを確かめる。本文（[`std::fmt::Display`]）は
/// 値を含むので機械判定に使えない。判定側が辞書引きするのはこちらの語である——重複すると 2 つの原因が 1 語に潰れて
/// 切り分けが効かなくなる。
#[test]
fn every_defer_reason_has_a_distinct_machine_word() {
    for (i, reason) in ALL_REASONS.into_iter().enumerate() {
        let (n, word, has_scope) = expected(reason);
        assert_eq!(n, i, "ALL_REASONS の {i} 番目の {reason:?} は表では {n} 番");
        assert_eq!(reason.as_str(), word, "{reason:?} の語が表と違う");
        // ALL_REASONS のスコープ番号はどれも 0。
        assert_eq!(
            reason.scope(),
            has_scope.then_some(0),
            "{reason:?} のスコープの有無が表と違う"
        );
    }
    let mut words: Vec<&str> = ALL_REASONS.iter().map(|reason| reason.as_str()).collect();
    words.sort_unstable();
    words.dedup();
    assert_eq!(
        words.len(),
        ALL_REASONS.len(),
        "見送り理由の語が重複している: {words:?}"
    );
    for word in &words {
        assert!(!word.is_empty(), "空の理由語がある");
        assert!(
            !word.contains(' '),
            "理由語に空白がある（1 行 1 レコードの分解が壊れる）: {word}"
        );
    }
}

/// 停滞診断の初期化で、**2 度目の待ちでも警告が一度は出る**ようになる
/// （`areka-P0-dpi-transition-atomicity` 要件 6.3）。
///
/// 初期化しないと `reported` が立ったままで、2 度目以降の遷移で見送りが続いても無音になる
/// ——「見送り続けている」という一番知りたい事実がログから消える。
#[test]
fn resetting_the_stall_lets_a_second_wait_report_once_again() {
    let mut stall = ChainFinalizeStall::default();
    for _ in 1..CHAIN_FINALIZE_STALL_FRAMES {
        assert!(!note_chain_deferral(&mut stall));
    }
    assert!(note_chain_deferral(&mut stall), "1 度目の待ちで報告する");

    stall.reset();
    for _ in 1..CHAIN_FINALIZE_STALL_FRAMES {
        assert!(!note_chain_deferral(&mut stall), "2 度目も閾値までは無音");
    }
    assert!(
        note_chain_deferral(&mut stall),
        "2 度目の待ちでも閾値でちょうど 1 度報告する（要件 6.3）"
    );
}
