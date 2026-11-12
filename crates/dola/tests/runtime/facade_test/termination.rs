//! 終了フロー（research.md §R3: termination シーム）
//!
//! Conclude / Cancel / Finish の各終了フローと境界条件を束ねる。

// =========================================================================
// 8.5 Conclude/Cancel/Finish フローテスト
// =========================================================================
mod conclude_cancel_finish {
    use super::super::*;

    #[test]
    fn conclude_delivers_final_values() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        // t=0.0: 初回
        let _ = rt.update(0.0);

        // t=0.3 で Conclude
        rt.conclude(group_id).unwrap();

        // 次の update で最終値 (1.0) が配信される
        let diff = rt.update(0.3).changes;
        let opacity = diff.iter().find(|(id, _)| *id == opacity_id);
        match opacity {
            Some((_, EvaluatedValue::Float(v))) => {
                assert!((*v - 1.0).abs() < 1e-9, "conclude final value: {v}")
            }
            other => panic!("expected final opacity Float(1.0), got {other:?}"),
        }

        // 以降は差分なし
        let diff = rt.update(0.5).changes;
        assert!(
            diff.is_empty(),
            "expected empty diff after conclude, got {diff:?}"
        );
    }

    #[test]
    fn cancel_freezes_current_value() {
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        // t=0.0: 初回
        let _ = rt.update(0.0);
        // t=0.5: 中間値取得
        let _ = rt.update(0.5);

        // Cancel — 現在値凍結
        rt.cancel(group_id).unwrap();

        // 以降は差分なし（タイムテーブル削除済み、値は前回 update 値で凍結）
        let diff = rt.update(0.6).changes;
        assert!(diff.is_empty(), "expected empty diff after cancel");

        let diff = rt.update(1.0).changes;
        assert!(diff.is_empty(), "expected still empty after cancel");
    }

    #[test]
    fn finish_delayed_conclude() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        // t=0.0
        let _ = rt.update(0.0);

        // finish: deadline = 0.5（t=0.5 で Conclude 相当）
        rt.finish(group_id, 0.5).unwrap();

        // t=0.3: deadline 未到達 → まだ再生中
        let diff = rt.update(0.3).changes;
        let (_, opacity) = diff
            .iter()
            .find(|(id, _)| *id == opacity_id)
            .expect("opacity is delivered while playing at 0.3");
        match opacity {
            EvaluatedValue::Float(v) => assert!((*v - 0.3).abs() < 1e-9, "playing at 0.3: {v}"),
            other => panic!("expected Float, got {other:?}"),
        }

        // t=0.5: deadline 到達 → Conclude 相当で最終値が飛ぶ
        let diff = rt.update(0.5).changes;
        let (_, opacity) = diff
            .iter()
            .find(|(id, _)| *id == opacity_id)
            .expect("finish delivers the final opacity at the deadline");
        assert_eq!(
            *opacity,
            EvaluatedValue::Float(1.0),
            "finish conclude final"
        );

        // 以降は差分なし
        let diff = rt.update(0.8).changes;
        assert!(
            diff.is_empty(),
            "expected empty after finish-conclude, got {diff:?}"
        );
    }

    #[test]
    fn conclude_on_invalid_group_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.conclude(9999).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(9999)));
    }

    #[test]
    fn cancel_on_invalid_group_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.cancel(9999).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(9999)));
    }

    #[test]
    fn double_conclude_fails() {
        let mut rt = DolaRuntime::new();
        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        rt.conclude(group_id).unwrap();
        // Concluded でインスタンス削除済み → 2 回目は InvalidGroupId
        let err = rt.conclude(group_id).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
    }

    #[test]
    fn cancel_after_conclude_fails() {
        let mut rt = DolaRuntime::new();
        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        rt.conclude(group_id).unwrap();
        // Conclude 済み（インスタンス削除済み）の Cancel は InvalidGroupId
        let err = rt.cancel(group_id).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
    }

    #[test]
    fn finish_on_invalid_group_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.finish(9999, 1.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(9999)));
    }

    #[test]
    fn finish_after_conclude_fails() {
        let mut rt = DolaRuntime::new();
        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        rt.conclude(group_id).unwrap();
        let err = rt.finish(group_id, 1.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
    }
}
