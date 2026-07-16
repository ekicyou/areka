//! CueSheet と compile_sheet のユニットテスト
//!
//! Task 6.3: 昇順ソート、0 ベース正規化、into_entry 3 種変換の検証。

use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet, CueTarget,
    RoutingCommand, compile_sheet,
};

// ============================================================================
// CueSheet 構築テスト
// ============================================================================

#[test]
fn cue_sheet_sorts_by_start_time() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 3.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 1.0,
            payload: CueCommand::Text("first".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 2.0,
            payload: CueCommand::Text("second".into()).into(),
            duration: 0.0,
        },
    ]);

    let times: Vec<f64> = sheet.cues().iter().map(|c| c.start_time).collect();
    assert_eq!(times, vec![1.0, 2.0, 3.0]);
}

#[test]
fn cue_sheet_empty() {
    let sheet = CueSheet::new(vec![]);
    assert!(sheet.is_empty());
    assert_eq!(sheet.len(), 0);
}

#[test]
fn cue_sheet_len() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("b"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);
    assert_eq!(sheet.len(), 2);
    assert!(!sheet.is_empty());
}

#[test]
fn cue_sheet_stable_sort_preserves_equal_start_time_order() {
    // D3-T 特性化: CueSheet::new は安定ソート — 同時刻 Cue は記述順を保持する
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 1.0,
            payload: CueCommand::Text("first".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 1.0,
            payload: CueCommand::Text("second".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.5,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    // 0.5 が先頭、同時刻 1.0 の 2 件は記述順
    assert!(matches!(
        sheet.cues()[0].payload,
        CuePayload::Command(CueCommand::Clear)
    ));
    match (&sheet.cues()[1].payload, &sheet.cues()[2].payload) {
        (
            CuePayload::Command(CueCommand::Text(t1)),
            CuePayload::Command(CueCommand::Text(t2)),
        ) => {
            assert_eq!(t1, "first");
            assert_eq!(t2, "second");
        }
        other => panic!("Expected two Text commands, got {other:?}"),
    }
}

// ============================================================================
// Actor フィルタリングテスト
// ============================================================================

#[test]
fn filter_by_actor() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("kero"),
            start_time: 0.5,
            payload: CueCommand::Emote { key: "grumble".into() }.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let sakura_cues = sheet.filter_by_actor(&ActorKey::from("sakura"));
    assert_eq!(sakura_cues.len(), 2);

    let kero_cues = sheet.filter_by_actor(&ActorKey::from("kero"));
    assert_eq!(kero_cues.len(), 1);

    let none_cues = sheet.filter_by_actor(&ActorKey::from("nobody"));
    assert!(none_cues.is_empty());
}

#[test]
fn actors_unique_list() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("kero"),
            start_time: 0.5,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let actors = sheet.actors();
    assert_eq!(actors.len(), 2);
    assert!(actors.contains(&&ActorKey::from("sakura")));
    assert!(actors.contains(&&ActorKey::from("kero")));
}

// ============================================================================
// compile_sheet テスト
// ============================================================================

#[test]
fn compile_sheet_normalizes_to_zero_base() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 5.0,
            payload: CueCommand::Text("first".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 7.0,
            payload: CueCommand::Text("second".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 10.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let compiled = compile_sheet(&sheet);
    assert_eq!(compiled.len(), 3);

    // 最小 start_time=5.0 を 0 に正規化
    let offsets: Vec<f64> = compiled.iter().map(|c| c.offset).collect();
    assert_eq!(offsets, vec![0.0, 2.0, 5.0]);
}

#[test]
fn compile_sheet_empty() {
    let sheet = CueSheet::new(vec![]);
    let compiled = compile_sheet(&sheet);
    assert!(compiled.is_empty());
}

#[test]
fn compile_sheet_preserves_actor() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("kero"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let compiled = compile_sheet(&sheet);
    assert_eq!(compiled[0].actor.as_str(), "sakura");
    assert_eq!(compiled[1].actor.as_str(), "kero");
}

#[test]
fn compile_sheet_normalizes_negative_start_times() {
    // D3-T: 負の start_time も最小値を 0 基準に正規化する
    // （TimedSchedule::insert は非負オフセットを debug_assert で要求するため、
    //  この正規化が負時刻台本を安全に接続する境界となる）
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: -2.0,
            payload: CueCommand::Text("first".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let compiled = compile_sheet(&sheet);
    let offsets: Vec<f64> = compiled.iter().map(|c| c.offset).collect();
    assert_eq!(offsets, vec![0.0, 3.0]);
}

#[test]
fn compile_sheet_preserves_barrier_and_routing_kinds() {
    // D3-T: コマンド/バリア/ルーティング混在台本が Entry 3 種へ正しく振り分けられる
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 1.0,
            payload: CueCommand::Text("cmd".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 2.0,
            payload: CuePayload::Barrier(BarrierKind::WaitForInput { timeout: None }),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 3.0,
            payload: CuePayload::Routing(RoutingCommand::RouteRemove {
                target: CueTarget::Shell,
            }),
            duration: 0.0,
        },
    ]);

    let compiled = compile_sheet(&sheet);
    assert_eq!(compiled.len(), 3);
    assert!(
        matches!(&compiled[0].entry, dola::cue::Entry::Payload(t, CueCommand::Text(_)) if *t == 0.0)
    );
    assert!(
        matches!(&compiled[1].entry, dola::cue::Entry::Barrier(t, BarrierKind::WaitForInput { .. }) if *t == 1.0)
    );
    assert!(
        matches!(&compiled[2].entry, dola::cue::Entry::Routing(t, RoutingCommand::RouteRemove { .. }) if *t == 2.0)
    );
}

// ============================================================================
// 非有限 start_time の境界（D3-V 特性化）
// ============================================================================

#[test]
fn cue_sheet_new_with_nan_start_time_does_not_panic() {
    // 特性化: NaN の start_time はソート比較で Equal 扱い（unwrap_or）となるため
    // CueSheet::new は panic せず、全 Cue が保持される（位置は規定されない）。
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 2.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: f64::NAN,
            payload: CueCommand::Text("nan".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);
    assert_eq!(sheet.len(), 3);
}

#[test]
fn compile_sheet_nan_start_time_excluded_from_min_yields_nan_offset() {
    // 特性化: NaN の start_time は f64::min が NaN を無視するため最小値計算から
    // 脱落し、当該 Cue のオフセットのみ NaN となる（有限 Cue の正規化は正しいまま）。
    // NaN オフセットは TimedSchedule::insert の debug_assert 対象（P25 参照）。
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: f64::NAN,
            payload: CueCommand::Text("nan".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 5.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 7.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let compiled = compile_sheet(&sheet);
    assert_eq!(compiled.len(), 3);
    let nan_offsets = compiled.iter().filter(|c| c.offset.is_nan()).count();
    assert_eq!(nan_offsets, 1, "NaN start_time の Cue のみ NaN オフセットとなる");
    let finite: Vec<f64> = compiled
        .iter()
        .filter(|c| c.offset.is_finite())
        .map(|c| c.offset)
        .collect();
    assert_eq!(finite, vec![0.0, 2.0], "有限 Cue は min=5.0 基準で正規化される");
}

#[test]
fn compile_sheet_all_infinite_start_times_yield_nan_offsets() {
    // 特性化: 全 Cue が +inf の場合 min = +inf となり、inf - inf = NaN オフセット
    // が生成される（P25 参照）。
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: f64::INFINITY,
            payload: CueCommand::Text("x".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: f64::INFINITY,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let compiled = compile_sheet(&sheet);
    assert_eq!(compiled.len(), 2);
    assert!(compiled.iter().all(|c| c.offset.is_nan()));
}

#[test]
fn compile_sheet_partial_infinite_start_time_is_never_delivered() {
    // 特性化: 一部の Cue のみ +inf の場合、そのオフセットは +inf となり
    // TimedSchedule 上で永遠に配信されない（is_completed() が true にならない
    // liveness 喪失 — P25 参照）。+inf は insert の非負 debug_assert は通過する。
    use dola::cue::TimedSchedule;

    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.0,
            payload: CueCommand::Text("now".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: f64::INFINITY,
            payload: CueCommand::Text("never".into()).into(),
            duration: 0.0,
        },
    ]);

    let compiled = compile_sheet(&sheet);
    assert_eq!(compiled[1].offset, f64::INFINITY);

    let mut sched = TimedSchedule::<CueCommand>::new(0.0);
    for cc in &compiled {
        sched.insert(cc.entry.clone());
    }

    sched.tick(f64::MAX); // 表現可能な最大の有限時刻まで進めても
    assert_eq!(sched.ready().len(), 1); // "now" のみ配信
    assert_eq!(sched.remaining(), 1); // "never" は残置
    assert!(!sched.is_completed()); // スケジュールは完了しない
}

// ============================================================================
// into_entry 3 種変換テスト
// ============================================================================

#[test]
fn into_entry_command_becomes_payload() {
    let payload = CuePayload::Command(CueCommand::Text("test".into()));
    let entry = payload.into_entry(1.5);
    match entry {
        dola::cue::Entry::Payload(t, cmd) => {
            assert_eq!(t, 1.5);
            assert!(matches!(cmd, CueCommand::Text(_)));
        }
        _ => panic!("Expected Entry::Payload"),
    }
}

#[test]
fn into_entry_barrier_becomes_barrier() {
    let payload = CuePayload::Barrier(BarrierKind::WaitForInput { timeout: None });
    let entry = payload.into_entry(2.0);
    match entry {
        dola::cue::Entry::Barrier(t, kind) => {
            assert_eq!(t, 2.0);
            assert!(matches!(kind, BarrierKind::WaitForInput { .. }));
        }
        _ => panic!("Expected Entry::Barrier"),
    }
}

#[test]
fn into_entry_routing_becomes_routing() {
    let routing = RoutingCommand::RouteRemove {
        target: CueTarget::Shell,
    };
    let payload = CuePayload::Routing(routing);
    let entry = payload.into_entry(0.0);
    match entry {
        dola::cue::Entry::Routing(t, r) => {
            assert_eq!(t, 0.0);
            assert!(matches!(r, RoutingCommand::RouteRemove { .. }));
        }
        _ => panic!("Expected Entry::Routing"),
    }
}

// ============================================================================
// compile_sheet → TimedSchedule 統合テスト
// ============================================================================

#[test]
fn compile_sheet_to_timed_schedule_integration() {
    use dola::cue::TimedSchedule;

    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 2.0,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 2.5,
            payload: CuePayload::Barrier(BarrierKind::WaitForInput { timeout: None }),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 3.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let compiled = compile_sheet(&sheet);
    let mut sched = TimedSchedule::<CueCommand>::new(10.0); // 絶対時刻 10.0 開始

    for cc in &compiled {
        sched.insert(cc.entry.clone());
    }

    // offset=0.0 → Text
    sched.tick(10.0);
    assert_eq!(sched.ready().len(), 1);
    assert!(matches!(sched.ready()[0], CueCommand::Text(_)));

    // offset=0.5 → Barrier 到達
    sched.tick(10.5);
    assert!(sched.current_barrier().is_some());

    // バリア解除後に Clear 取得
    sched.notify_barrier_resolved(None);
    sched.tick(11.0);
    assert!(matches!(sched.ready()[0], CueCommand::Clear));
}

// ============================================================================
// 自己完結した絶対時刻台本（absolute_start_time）テスト — R1.7
// ============================================================================

/// 絶対開始時刻を刻印していない台本のアンカーは 0.0（既定）。
#[test]
fn cue_sheet_absolute_start_time_defaults_to_zero() {
    let sheet = CueSheet::new(vec![Cue {
        actor: ActorKey::from("a"),
        start_time: 1.0,
        payload: CueCommand::Text("hi".into()).into(),
        duration: 0.25,
    }]);

    assert_eq!(sheet.absolute_start_time(), 0.0);
}

/// R1.7: 各 cue の絶対発火時刻は台本のみから `absolute_start_time + start_time` で導ける。
#[test]
fn absolute_fire_time_of_every_cue_derives_from_sheet_alone() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.0,
            payload: CueCommand::ClearAll.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.5,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.25,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 2.25,
            payload: CueCommand::Emote { key: "3".into() }.into(),
            duration: 0.0,
        },
    ])
    .with_absolute_start_time(100.0);

    let fired: Vec<f64> = sheet
        .cues()
        .iter()
        .map(|cue| sheet.absolute_fire_time(cue))
        .collect();

    // アンカー 100.0 ＋ 相対 start_time（相対値は変換されない）
    assert_eq!(fired, vec![100.0, 100.5, 102.25]);
}

/// R1.7: talk の絶対終了時刻は `absolute_start_time + max(start_time + duration)`。
///
/// 「最後に発火する cue」でも「最大 start_time」でもなく**占有区間の最大端**であることを
/// 固定する（最長 duration を持つ cue が末尾でない配置で検証）。
#[test]
fn absolute_end_time_is_max_of_start_plus_duration_not_last_cue_start() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            // 1.0 + 5.0 = 6.0 が最大端（末尾 cue ではない）
            start_time: 1.0,
            payload: CueCommand::Text("long".into()).into(),
            duration: 5.0,
        },
        Cue {
            actor: ActorKey::from("a"),
            // 3.0 + 0.5 = 3.5（最大 start_time だが最大端ではない）
            start_time: 3.0,
            payload: CueCommand::Wait.into(),
            duration: 0.5,
        },
    ])
    .with_absolute_start_time(10.0);

    assert_eq!(sheet.absolute_end_time(), 16.0);
    // 最大 start_time（10.0+3.0）にも、最後の cue の発火時刻にも縮退しない
    assert_ne!(sheet.absolute_end_time(), 13.0);
    assert_ne!(sheet.absolute_end_time(), 13.5);
}

/// R1.7: 末尾の純粋 Wait cue の duration は絶対終了時刻へ算入される（早期終了しない・R2.5 の土台）。
#[test]
fn absolute_end_time_includes_trailing_wait_duration() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.0,
            payload: CueCommand::Text("bye".into()).into(),
            duration: 0.15,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.15,
            payload: CueCommand::Wait.into(),
            duration: 2.0,
        },
    ])
    .with_absolute_start_time(50.0);

    // 末尾 Wait の発火時刻（50.15）でなく、その duration の終端（52.15）
    assert_eq!(sheet.absolute_end_time(), 52.15);
}

/// 空台本は時間を占有しない＝絶対終了時刻はアンカーそのもの。
#[test]
fn absolute_end_time_of_empty_sheet_is_the_anchor() {
    let sheet = CueSheet::new(vec![]).with_absolute_start_time(7.5);

    assert_eq!(sheet.absolute_end_time(), 7.5);
}

/// 刻印は相対時刻を書き換えない（cue の相対 start_time / duration は不変）。
#[test]
fn stamping_absolute_start_time_leaves_relative_times_untouched() {
    let cues = vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.0,
            payload: CueCommand::Text("x".into()).into(),
            duration: 0.5,
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.5,
            payload: CueCommand::NewLine { ratio: 1.0 }.into(),
            duration: 0.0,
        },
    ];
    let unstamped = CueSheet::new(cues.clone());
    let stamped = CueSheet::new(cues).with_absolute_start_time(1234.5);

    let rel = |s: &CueSheet| -> Vec<(f64, f64)> {
        s.cues()
            .iter()
            .map(|c| (c.start_time, c.duration))
            .collect()
    };
    assert_eq!(rel(&stamped), rel(&unstamped));
    assert_eq!(stamped.absolute_start_time(), 1234.5);
    // 同一台本でもアンカーが違えば絶対時刻だけがずれる（相対は共有）
    assert_eq!(
        stamped.absolute_end_time() - unstamped.absolute_end_time(),
        1234.5
    );
}

// ============================================================================
// serde テスト
// ============================================================================

/// R9.3: `absolute_start_time` は `#[serde(default)]`＝アンカーを持たない JSON は 0.0。
#[test]
fn cue_sheet_deserializes_without_absolute_start_time_as_zero() {
    let json = r#"{"cues":[
        {"actor":"sakura","start_time":0.0,"payload":{"Command":{"Text":"hi"}}}
    ]}"#;
    let sheet: CueSheet = serde_json::from_str(json).unwrap();

    assert_eq!(sheet.absolute_start_time(), 0.0);
    assert_eq!(sheet.len(), 1);
    assert_eq!(sheet.absolute_end_time(), 0.0);
}

/// 刻印済みアンカーは roundtrip を跨いで保存される（自己完結台本の搬送）。
#[test]
fn cue_sheet_serde_roundtrip_preserves_absolute_start_time() {
    let sheet = CueSheet::new(vec![Cue {
        actor: ActorKey::from("sakura"),
        start_time: 0.5,
        payload: CueCommand::Text("hello".into()).into(),
        duration: 0.25,
    }])
    .with_absolute_start_time(42.0);

    let json = serde_json::to_string(&sheet).unwrap();
    let parsed: CueSheet = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.absolute_start_time(), 42.0);
    assert_eq!(parsed.absolute_fire_time(&parsed.cues()[0]), 42.5);
    assert_eq!(parsed.absolute_end_time(), 42.75);
}

#[test]
fn cue_sheet_serde_roundtrip() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("kero"),
            start_time: 1.0,
            payload: CuePayload::Barrier(BarrierKind::Timeout { duration: 3.0 }),
            duration: 0.0,
        },
    ]);

    let json = serde_json::to_string(&sheet).unwrap();
    let parsed: CueSheet = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed.cues()[0].actor.as_str(), "sakura");
    assert_eq!(parsed.cues()[1].actor.as_str(), "kero");
}
