// ===================== 本タスク（4.1）の証明テスト =====================
//
// 「台本通りの応答・終了結果・死活遷移を任意に再現できることを確認できる」
// （tasks.md 4.1 の観測可能な完了条件）を、6 シナリオで直接固定する。

use super::*;

/// シナリオ1: GET 応答（`Ok(Some)`）が台本どおり返り、呼出が記録されること。
#[test]
fn scripted_get_ok_some_returns_exact_value_and_is_recorded() {
    let (mut backend, handle) = ScriptedShioriBackend::builder()
        .get("OnBoot", Ok(Some("\\h\\s0hello\\e".to_string())))
        .build();

    let result = backend.get("OnBoot", &[], None);

    // `RequestError` は `PartialEq` を実装しないため（凍結面の消費のみ・機械的写像の
    // 都合）、`Result` 全体の `assert_eq!` はできない——`Ok` の中身を直接照合する。
    match result {
        Ok(Some(script)) => assert_eq!(script, "\\h\\s0hello\\e"),
        other => panic!("expected Ok(Some(..)), got {other:?}"),
    }

    let calls = handle.calls();
    let calls = calls.lock().expect("calls mutex poisoned");
    assert_eq!(
        &*calls,
        &vec![RecordedCall::Get {
            id: "OnBoot".to_string(),
            references: vec![],
        }]
    );
}

/// シナリオ2: GET 応答として台本化した失敗（`Err(RequestError::Timeout)`）が
/// そのまま variant 一致で返ること。
#[test]
fn scripted_get_err_returns_exact_error_variant() {
    let (mut backend, _handle) = ScriptedShioriBackend::builder()
        .get("OnSecondChange", Err(RequestError::Timeout))
        .build();

    let result = backend.get("OnSecondChange", &[], None);

    match result {
        Err(RequestError::Timeout) => {}
        other => panic!("expected Err(RequestError::Timeout), got {other:?}"),
    }
}

/// シナリオ3: NOTIFY 応答が台本どおり返り、呼出が記録されること。
#[test]
fn scripted_notify_returns_exact_value_and_is_recorded() {
    let (mut backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnCloseAll", Ok(()))
        .build();

    let references = vec!["reason".to_string()];
    let result = backend.notify("OnCloseAll", &references, None);

    assert!(result.is_ok(), "expected Ok(()), got {result:?}");

    let calls = handle.calls();
    let calls = calls.lock().expect("calls mutex poisoned");
    assert_eq!(
        &*calls,
        &vec![RecordedCall::Notify {
            id: "OnCloseAll".to_string(),
            references,
        }]
    );
}

/// シナリオ4: `unload()` の結果（`Ok(ExitKind::Clean)`）が台本どおり返ること。
#[test]
fn scripted_unload_returns_exact_exit_kind() {
    let (mut backend, _handle) = ScriptedShioriBackend::builder()
        .unload(Ok(ExitKind::Clean))
        .build();

    let result = backend.unload();

    assert_eq!(
        result.expect("scripted unload should be Ok"),
        ExitKind::Clean
    );
}

/// シナリオ5: 死活状態の遷移。初期 `status()` は台本どおり `Running` を返し、その後
/// テストのスレッドから `handle.set_status` で `Exited(Abnormal(1))` へ差し替えると、
/// 以降の `status()` 呼出はその新しい値を返す（helper がシナリオ途中で死ぬ様子を
/// 「backend の外側・テスト自身」から駆動できることの直接証跡・要件 7.1）。
#[test]
fn status_transitions_from_running_to_exited_when_mutated_externally_mid_scenario() {
    let (mut backend, handle) = ScriptedShioriBackend::builder()
        .status(HelperStatus::Running)
        .build();

    assert_eq!(backend.status(), HelperStatus::Running);

    // シミュレート: helper がシナリオ途中で異常終了する（テスト自身の駆動）。
    handle.set_status(HelperStatus::Exited(ExitKind::Abnormal(1)));

    assert_eq!(
        backend.status(),
        HelperStatus::Exited(ExitKind::Abnormal(1)),
        "status() 呼出は途中差し替え後の値を反映しなければならない"
    );
}

/// シナリオ6: `RecordingSink` の clone 共有蓄積。2 つの clone それぞれから単一出力契約
/// [`CueSink`] 経由で 1 件ずつ emit すると、同一の共有蓄積へ FIFO で積まれること
/// （dispatcher が broadcast で全 sink（各 talk へ clone した surface/text スロット）へ
/// 同一 cue を配る使い方を裏付ける）。
#[test]
fn recording_sink_clones_share_storage_in_fifo_order() {
    let sink = RecordingSink::new();
    let records = sink.records();

    let mut clone_a = sink.clone();
    let mut clone_b = sink.clone();

    let cue_a = TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("via clone a".to_string()),
        duration: 0.0,
    };
    let cue_b = TalkCue {
        at: 1.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("via clone b".to_string()),
        duration: 0.0,
    };

    CueSink::emit(&mut clone_a, cue_a.clone());
    CueSink::emit(&mut clone_b, cue_b.clone());

    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(&*recorded, &vec![cue_a, cue_b]);
}
