use super::test_support::bump_balloon_window_dpi;
use super::{
    balloon_target, capture_logs, run_attach_phase, run_dpi_phase, run_text_scale_phase, BindSet,
    PatternState, PresentCommand, SpineHarness,
};

/// balloon target の適用 k（`applied_scale`）を読む短縮（`None`＝未表示は前提違反ゆえ panic）。
fn balloon_applied_scale(harness: &SpineHarness, scope: u32) -> f32 {
    harness
        .wiring
        .presenter()
        .applied_scale(balloon_target(scope))
        .expect("attach 初回表示済みの balloon target は適用 k を持つ")
}

/// **可視バルーンの DPI 変化**（Flow 2 の正常系・R8.1）: `Changed<DPI>` → `run_dpi_phase` で
/// 適用 k が跳ねたあと、`run_text_scale_phase` が文字層 binding を新 k へ組み直すことを
/// **実 attach（`text_slot_view` が `Some`）の本番経路**で固定する。
///
/// 併せて churn ガード（R8.5）——k が動いていないフレームでは再構築が 1 件も起きない——を
/// 変化の前後**両方**で観測する。毎フレーム走査という cadence が「毎フレーム再生成」に
/// 退化していないことは、この 2 点でしか区別できない。
#[test]
fn spine_dpi_change_refreshes_balloon_text_scale_on_real_attach() {
    let mut harness = SpineHarness::boot(r"\s[0]\e");

    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("attached=2")),
        "前提: attach 完了（balloon 初回表示＝文字層 actor 登録済み）が観測できない: {logs:?}"
    );

    // 前提の非空虚性: attach で actor が登録され、balloon target は適用 k を持っている。
    let k_before = balloon_applied_scale(&harness, 0);
    // 前提: 本番経路の `Some(view)` が実際に成立している（`None` なら以降は縮退 skip の観測に
    // 退化し、再構築ゼロが「正しく no-op」なのか「view が無い」のか区別できなくなる）。
    // actor 登録そのものは `TextLayerRuntime` の読み口が無い（`is_attached` は供給面の有無＝
    // `present_frame` 後にしか立たない）ため、後段の `vec![0]` 到達がその証跡を兼ねる。
    assert!(
        harness
            .wiring
            .presenter()
            .text_slot_view(balloon_target(0))
            .is_some(),
        "前提: attach 初回表示で balloon の text_slot_view が Some になっている（本番の Some 経路）"
    );
    // churn ガード（変化前）: k が動いていないフレームでは 1 件も再構築しない。
    assert!(
        run_text_scale_phase(&mut harness.wiring).is_empty(),
        "k 不変のフレームで文字層を組み直してはならない（R8.5）"
    );

    // DPI 変化 → dpi 相（refresh_scale で再表示・窓寸 reconcile）。
    let new_dpi = bump_balloon_window_dpi(&mut harness, 0);
    run_dpi_phase(&mut harness.wiring, &mut harness.world);
    let k_after = balloon_applied_scale(&harness, 0);
    assert_ne!(
        k_after, k_before,
        "前提: DPI={new_dpi} で balloon target の適用 k が実際に変わる（変わらなければ本ケースは空虚）"
    );

    // 本題: 文字層 binding が新 k へ組み直される（当該 scope のみ・k 不変の scope1 は動かない）。
    assert_eq!(
        run_text_scale_phase(&mut harness.wiring),
        vec![0u32],
        "適用 k が変わった balloon scope の文字層 binding が新 k へ組み直される（R8.1・D11-4）"
    );
    // churn ガード（変化後）: 直後のフレームは同値 k ゆえ再構築ゼロ。
    assert!(
        run_text_scale_phase(&mut harness.wiring).is_empty(),
        "組み直した直後のフレームは同値 k ＝ no-op（毎フレーム再生成の禁止・R8.5）"
    );

    harness.shutdown_bounded();
}

/// **不可視中の DPI 変化が `Show` で着地する**（R8.1・6.5 一次実走の欠陥の本丸）:
/// `\b[-1]`→（DPI 変化）→`\b[0]` の順で、文字層が最終的に**新 k** へ着地することを固定する。
///
/// # なぜこの順序が本質なのか
///
/// `EmoPresenter::refresh_scale` は**不可視の target を再表示で蘇らせない**（`Hide` 済みなら
/// `applied` を更新せず `None`）。ゆえに「`refresh_scale` が `Some` を返した窓へ伝搬する」型の
/// 結線は、この順序で**一度も発火しない**——`Changed<DPI>` のエッジは不可視のフレームで消費
/// されて二度と来ず、適用 k は後続の `Show`（`apply_show`＝drain 相）で跳ぶからである。
/// バルーンは大半の時間が不可視であり、`\b[-1]`→`\b[0]` は
/// `spine_s3_balloon_face_cue_delivers_hide_then_show_in_order` が本番の通常列として既に
/// 固定している。よって「報告駆動」は実運用でほぼ常に取りこぼす。
///
/// 本ケースは `run_text_scale_phase` を `refresh_scale` の戻り値から**完全に独立**させた設計
/// （引数に報告を取らない＝構造的に参照できない）の到達判定である。同じ独立性が
/// 「k は変わったが丸め後の物理寸が同じ」（`refresh_scale` が成功しても `None` を返す documented
/// なケース）も同時に閉塞する——どちらも「報告 `None` だが `applied` は新 k」という同一の形だからである。
#[test]
fn spine_dpi_change_while_balloon_hidden_lands_on_next_show() {
    let mut harness = SpineHarness::boot(r"\s[0]\e");

    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter().any(|l| l.contains("attached=2")),
        "前提: attach 完了が観測できない: {logs:?}"
    );
    let k_attached = balloon_applied_scale(&harness, 0);
    assert!(
        run_text_scale_phase(&mut harness.wiring).is_empty(),
        "前提: 装着直後は binding と適用 k が一致（再構築ゼロ）"
    );

    // (1) `\b[-1]` 相当（本番 adapter が DisplayCommand::HideBalloon から組む指令と同型）。
    harness.wiring.apply_present(
        &mut harness.world,
        PresentCommand::Hide {
            target: balloon_target(0),
            reply: None,
        },
    );

    // (2) 不可視のまま DPI 変化 → dpi 相は再表示せず適用 k も据え置き（＝報告駆動が落ちる条件）。
    let new_dpi = bump_balloon_window_dpi(&mut harness, 0);
    run_dpi_phase(&mut harness.wiring, &mut harness.world);
    assert_eq!(
        balloon_applied_scale(&harness, 0),
        k_attached,
        "前提: 不可視の target は再表示されず適用 k も更新されない（refresh_scale の可視ゲート）"
    );
    assert!(
        run_text_scale_phase(&mut harness.wiring).is_empty(),
        "不可視の間は文字層も動かない（適用 k がまだ旧値＝同値 k の no-op・R8.5）"
    );

    // (3) `\b[0]` 相当の再表示: ここで `apply_show` が**新 DPI**で k を導出し適用する。
    harness.wiring.apply_present(
        &mut harness.world,
        PresentCommand::ShowSurface {
            target: balloon_target(0),
            surface_id: 0,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: None,
        },
    );
    let k_shown = balloon_applied_scale(&harness, 0);
    assert_ne!(
        k_shown, k_attached,
        "前提: 再表示で DPI={new_dpi} 由来の新 k が適用される（変わらなければ本ケースは空虚）"
    );

    // (4) 本題: `Changed<DPI>` のエッジは既に消費済みでも、文字層は新 k へ着地する。
    assert_eq!(
        run_text_scale_phase(&mut harness.wiring),
        vec![0u32],
        "不可視中の DPI 変化でも、再表示後のフレームで文字層 binding が新 k へ着地する（R8.1）"
    );
    assert!(
        run_text_scale_phase(&mut harness.wiring).is_empty(),
        "着地後は同値 k ＝ no-op（R8.5）"
    );

    harness.shutdown_bounded();
}
