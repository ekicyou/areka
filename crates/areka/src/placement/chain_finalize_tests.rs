// =============================================================================
// 実表示寸での連鎖再解決の檻（scg 要件 7.1/7.2/7.3・design C6）
//
// 判定は純関数ゆえ GPU も World も要さない。物理 px を直接与えるため DPI 水準は
// 「幅の組」を変えることで表現する（resolver 檻の DPIS ループと同じ意図で、
// 等幅・不等幅の双方を同一経路で検定する）。
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

/// **そもそも既定配置ではない**スコープを組む補助（保存位置が復元された scope・scg 7.3）。
fn restored(scope: usize, current_x: i32, w: i32) -> ScopeChainState {
    ScopeChainState {
        scope,
        current_x,
        width: w,
        default_x: None,
    }
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

/// 全隣接ペアの隙間 0 を検定する（未接触ペアのみが対象）。
fn assert_all_pairs_flush(states: &[ScopeChainState], xs: &[i32], label: &str) {
    for n in 1..states.len() {
        assert_eq!(
            xs[n - 1] - (xs[n] + states[n].width),
            0,
            "{label}: scope{}/scope{n} の隙間は 0（隣接・scg 7.1）",
            n - 1
        );
    }
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

    let moves = finalize_chain(&states);

    assert_eq!(moves.len(), 1, "動かすのは後続スコープだけ（起点は不動・7.2）");
    assert_eq!(
        moves[0],
        ChainMove {
            scope: 1,
            new_x: 1392
        },
        "new_x = 2064 − 672 = 1392（前スコープ左端 − 自スコープ幅）"
    );

    let xs = apply(&states, &moves);
    assert_eq!(xs[0], 2064, "起点スコープは動かない");
    assert_all_pairs_flush(&states, &xs, "emo2 200%");
    // 是正前の見た目（52px の隙間）へ戻ってはならない。
    assert_ne!(
        xs[1], 1340,
        "再解決せず据え置くと 52px の隙間が残る（退行）"
    );
}

// -----------------------------------------------------------------------------
// 規則の全網羅（不等幅・等幅を同一経路で）
// -----------------------------------------------------------------------------

/// 不等幅 3 スコープ: 全隣接ペアの隙間が 0 になる指示が出る。幅差が隙間へ漏れない。
#[test]
fn unequal_widths_are_all_made_flush() {
    let widths = [400, 320, 200];
    // 配置時から全スコープの幅が変わり、素朴には隙間だらけの状態を作る。
    let states = [
        untouched(0, 1000, widths[0]),
        untouched(1, 500, widths[1]),
        untouched(2, 100, widths[2]),
    ];

    let moves = finalize_chain(&states);
    let xs = apply(&states, &moves);

    assert_eq!(xs[0], 1000, "起点は不動");
    assert_eq!(xs[1], 680, "1000 − 320");
    assert_eq!(xs[2], 480, "680 − 200");
    assert_all_pairs_flush(&states, &xs, "不等幅 400/320/200");
}

/// 等幅でも同一の式で処理される（等幅を特殊扱いしない・scg 2.5 と同旨）。
#[test]
fn equal_widths_use_the_same_rule() {
    let states = [
        untouched(0, 1000, 320),
        untouched(1, 500, 320),
        untouched(2, 0, 320),
    ];

    let moves = finalize_chain(&states);
    let xs = apply(&states, &moves);

    assert_eq!(xs, vec![1000, 680, 360], "等幅でも前スコープ左端 − 自幅");
    assert_all_pairs_flush(&states, &xs, "等幅 320×3");
}

/// 欠陥式（前スコープの幅を引く）へ戻ってはならない。不等幅でのみ判別できる。
#[test]
fn previous_width_subtraction_is_rejected() {
    let states = [untouched(0, 1000, 400), untouched(1, 0, 320)];

    let moves = finalize_chain(&states);
    let xs = apply(&states, &moves);

    assert_eq!(xs[1], 680, "自スコープの幅を引く（1000 − 320）");
    assert_ne!(
        xs[1], 600,
        "前スコープの幅を引く旧式（1000 − 400）へ戻ってはならない"
    );
}

// -----------------------------------------------------------------------------
// 明示的な再配置の尊重（7.3）
// -----------------------------------------------------------------------------

/// 明示的に再配置されたスコープは動かさず、以後の連鎖はその**実位置**を基準にする。
#[test]
fn repositioned_scope_is_not_pulled_back_and_becomes_the_next_basis() {
    let states = [
        untouched(0, 1000, 400),
        // 台本の移動指令で 680（既定）から 900 へ動かされている
        repositioned(1, 900, 320, 680),
        untouched(2, 0, 200),
    ];

    let moves = finalize_chain(&states);

    assert!(
        moves.iter().all(|m| m.scope != 1),
        "明示的に再配置されたスコープへ指示を出さない（7.3）"
    );
    let xs = apply(&states, &moves);
    assert_eq!(xs[1], 900, "実位置のまま据え置く");
    assert_eq!(xs[2], 700, "以後の連鎖は実位置 900 を基準にする（900 − 200）");
}

/// **保存位置が復元されたスコープは、現在位置に関わらず常に対象外**（scg 7.3）。
///
/// 前回セッションで利用者がドラッグした位置は「既定配置」ではない。既定位置と比較する形の
/// 判定では、復元位置が既定位置として台帳に載ってしまうと `current_x == default_x` が成立し
/// 未接触と誤判定される——**セッションを跨ぐと利用者のドラッグが隣接位置へ引き戻される**。
/// 起動シーム（`main.rs`）が復元済みスコープの既定位置を落とし、ここが `None` を対象外として
/// 扱うことで防ぐ。
#[test]
fn restored_scope_is_never_pulled_back_even_when_it_sits_still() {
    let states = [
        untouched(0, 1000, 400),
        // 復元位置 900。既定位置は台帳から落とされている（None）。
        restored(1, 900, 320),
        untouched(2, 0, 200),
    ];

    let moves = finalize_chain(&states);

    assert!(
        moves.iter().all(|m| m.scope != 1),
        "復元されたスコープへ指示を出さない（7.3・セッションを跨ぐドラッグの保護）"
    );
    let xs = apply(&states, &moves);
    assert_eq!(xs[1], 900, "復元位置のまま据え置く");
    assert_eq!(xs[2], 700, "以後の連鎖は復元位置 900 を基準にする（900 − 200）");
    // 既定位置を持っていたら引き戻されていた値。ここへ動いてはならない。
    assert_ne!(
        xs[1], 680,
        "既定位置として扱うと 1000 − 320 = 680 へ引き戻される（是正前の挙動）"
    );
}

/// 復元スコープが**たまたま隣接位置に居ても**指示は出ない（`None` は位置に依らず対象外）。
#[test]
fn restored_scope_is_excluded_regardless_of_where_it_sits() {
    let states = [untouched(0, 1000, 400), restored(1, 680, 320)];

    assert!(
        finalize_chain(&states).is_empty(),
        "復元スコープは既に隣接していても対象外（判定が位置ではなく由来で決まる）"
    );
}

/// 起点スコープが動かされていても、後続は起点の実位置へ隣接する。
#[test]
fn follower_chains_from_the_actual_position_of_a_moved_origin() {
    let states = [
        repositioned(0, 1500, 400, 1000),
        untouched(1, 680, 320),
    ];

    let moves = finalize_chain(&states);
    let xs = apply(&states, &moves);

    assert_eq!(xs[0], 1500, "起点は動かさない");
    assert_eq!(xs[1], 1180, "1500 − 320（既定値 1000 ではなく実位置基準）");
    assert_all_pairs_flush(&states, &xs, "起点が移動済み");
}

// -----------------------------------------------------------------------------
// 冗長駆動の回避・縮退入力
// -----------------------------------------------------------------------------

/// 既に隣接している列には指示を出さない（べき等・冗長な書き込みを作らない）。
#[test]
fn already_flush_chain_emits_no_moves() {
    let states = [
        untouched(0, 1000, 400),
        untouched(1, 680, 320),
        untouched(2, 480, 200),
    ];

    assert!(
        finalize_chain(&states).is_empty(),
        "既に隣接なら指示は空（べき等）"
    );
}

/// 二度目の適用は空になる（一度きりの確定という結線側の契約を、判定側でも壊さない）。
#[test]
fn applying_twice_is_a_no_op_the_second_time() {
    let states = [untouched(0, 1000, 400), untouched(1, 0, 320)];
    let moves = finalize_chain(&states);
    assert!(!moves.is_empty(), "一度目は指示が出る");

    let xs = apply(&states, &moves);
    let settled = [
        untouched(0, xs[0], 400),
        untouched(1, xs[1], 320),
    ];
    assert!(
        finalize_chain(&settled).is_empty(),
        "確定後の状態を再投入しても指示は出ない"
    );
}

/// 非正寸は動かさない（縮退入力で暴走座標を作らない）。実位置は次の基準になる。
#[test]
fn non_positive_width_is_skipped_and_keeps_its_place() {
    let states = [
        untouched(0, 1000, 400),
        untouched(1, 700, 0),
        untouched(2, 400, 200),
    ];

    let moves = finalize_chain(&states);

    assert!(
        moves.iter().all(|m| m.scope != 1),
        "非正寸のスコープへ指示を出さない"
    );
    let xs = apply(&states, &moves);
    assert_eq!(xs[1], 700, "実位置のまま");
    assert_eq!(xs[2], 500, "700 − 200（非正寸スコープの実位置を基準にする）");
}

/// 空入力・単一スコープは常に空（動かす相手が居ない）。
#[test]
fn empty_and_single_scope_yield_no_moves() {
    assert!(finalize_chain(&[]).is_empty(), "空入力は空");
    assert!(
        finalize_chain(&[untouched(0, 1000, 400)]).is_empty(),
        "単一スコープは連鎖の相手が居ない"
    );
}

/// 極端入力でも panic しない（飽和演算）。
#[test]
fn saturating_arithmetic_does_not_panic_on_extremes() {
    let states = [
        untouched(0, i32::MIN, 400),
        untouched(1, 0, i32::MAX),
    ];
    let moves = finalize_chain(&states);
    assert_eq!(moves.len(), 1, "指示は出る（値は飽和）");
    assert_eq!(moves[0].new_x, i32::MIN, "飽和して下限に張り付く");
}

// -----------------------------------------------------------------------------
// 補助
// -----------------------------------------------------------------------------

/// `moved_default_pos` は X だけを差し替え Y を保存する（Y は再解決の対象外・7.2）。
#[test]
fn moved_default_pos_replaces_x_and_preserves_y() {
    let current = PointPx { x: 1340, y: 904 };
    assert_eq!(
        moved_default_pos(current, 1392),
        PointPx { x: 1392, y: 904 },
        "X のみ差し替え・Y は保存"
    );
}

// -----------------------------------------------------------------------------
// 確定が見送られ続けたときの一発診断（scg 6.5）
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

    // 世界全体に関わる理由はスコープを持たない（偽のスコープ番号を作らない）。
    assert_eq!(ChainDeferReason::NoGhostWindows.scope(), None);
    assert_eq!(ChainDeferReason::NoScopes.scope(), None);

    // 全経路が本文を持つ（Display の取りこぼしを塞ぐ）。
    for reason in [
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
        ChainDeferReason::DpiSyncHeld { scope: 0 },
        landing,
    ] {
        assert!(!reason.to_string().is_empty(), "理由 {reason:?} の本文が空");
    }
}

/// 見送り理由の**固定語**（観測レコードの `reason=`）が全経路そろい、互いに異なる
/// （`areka-P0-dpi-transition-atomicity` 設計 Data Models の `chain` 行）。
///
/// 本文（[`std::fmt::Display`]）は値を含むので機械判定に使えない。判定側が辞書引きするのは
/// こちらの語である——重複すると 2 つの原因が 1 語に潰れて切り分けが効かなくなる。
#[test]
fn every_defer_reason_has_a_distinct_machine_word() {
    let all = [
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
    let mut words: Vec<&str> = all.iter().map(|reason| reason.as_str()).collect();
    let count = words.len();
    words.sort_unstable();
    words.dedup();
    assert_eq!(words.len(), count, "見送り理由の語が重複している: {words:?}");
    for word in &words {
        assert!(!word.is_empty(), "空の理由語がある");
        assert!(
            !word.contains(' '),
            "理由語に空白がある（1 行 1 レコードの分解が壊れる）: {word}"
        );
    }
    // 遷移後の解き直し専用の理由（起動時確定では起こらない）。
    assert_eq!(
        ChainDeferReason::DpiSyncHeld { scope: 3 }.as_str(),
        "dpi-sync-held"
    );
    assert_eq!(ChainDeferReason::DpiSyncHeld { scope: 3 }.scope(), Some(3));
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

    // 初期化しない限り 2 度目は永久に黙る（初期化が必要であることの対照）。
    for _ in 0..CHAIN_FINALIZE_STALL_FRAMES {
        assert!(!note_chain_deferral(&mut stall), "初期化前は黙ったまま");
    }

    stall.reset();
    assert_eq!(
        stall,
        ChainFinalizeStall::default(),
        "初期化で計数も一発フラグも消える"
    );
    for _ in 1..CHAIN_FINALIZE_STALL_FRAMES {
        assert!(!note_chain_deferral(&mut stall), "2 度目も閾値までは無音");
    }
    assert!(
        note_chain_deferral(&mut stall),
        "2 度目の待ちでも閾値でちょうど 1 度報告する（要件 6.3）"
    );
}
