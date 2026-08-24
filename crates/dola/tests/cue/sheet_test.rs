//! CueSheet 構築と canonical 変換 to_talk_schedule のユニットテスト。
//!
//! 昇順ソート・絶対アンカー保存・先頭待ち保存・duration clamp・占有 horizon を検証する。

use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet, CueTarget, RoutingCommand,
    to_talk_schedule,
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
        (CuePayload::Command(CueCommand::Text(t1)), CuePayload::Command(CueCommand::Text(t2))) => {
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
            payload: CueCommand::Emote {
                key: "grumble".into(),
            }
            .into(),
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

// ============================================================================
// canonical 変換 to_talk_schedule — R11.1 / R1.3 / R1.6 / R1.8 / R2.5
//
// 台本→時刻スケジュールへの**唯一の変換**。旧 2 実装（min 正規化で先頭待ちを食う版と
// sakura 独自版・task 8.2 で撤去）を廃した dola ingress の単一権威。
// 絶対アンカー（absolute_start_time）＋相対 start_time を保存し、同一 at は記述順（FIFO）、
// duration を有限・非負へ clamp（envelope と horizon の両方に適用）、占有 horizon を保持して
// is_completed を占有終了で判定する。
// ============================================================================

/// R11.1: 先頭に待ちを持つ台本を変換しても待ちが消えない（相対 start_time を保存）。
///
/// canonical 変換は相対時刻をそのまま保つため、先頭 Wait(at=0.5) は 0.5 で due になる
/// （min 正規化で 0 へ食う旧実装と異なり、食われない）。
#[test]
fn to_talk_schedule_preserves_leading_wait() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.5,
            payload: CueCommand::Wait.into(),
            duration: 0.5,
        },
        Cue {
            actor: ActorKey::from("0"),
            start_time: 1.0,
            payload: CueCommand::Text("hi".into()).into(),
            duration: 0.0,
        },
    ]);

    // canonical 変換は相対 start_time を保存する（先頭待ちを食わない）。
    let mut schedule = to_talk_schedule(&sheet);
    schedule.tick(0.4);
    assert!(
        schedule.ready().is_empty(),
        "0.4 では先頭 Wait(at=0.5) は未 due（食われていない）"
    );
    schedule.tick(0.5);
    assert_eq!(schedule.ready().len(), 1, "0.5 で先頭 Wait が due");
    assert_eq!(schedule.ready()[0].at, 0.5, "相対 start_time が保存される");
    assert!(matches!(schedule.ready()[0].command, CueCommand::Wait));
}

/// R1.8: 非有限（NaN/±inf）・負の duration は 0 へ clamp され、envelope と horizon の
/// **両方**へ clamp 後の値が反映される（dola ingress の単一権威）。
#[test]
fn to_talk_schedule_clamps_non_finite_and_negative_duration_into_envelope_and_horizon() {
    let bad = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -3.0];
    let cues: Vec<Cue> = bad
        .iter()
        .enumerate()
        .map(|(i, &d)| Cue {
            actor: ActorKey::from("0"),
            start_time: i as f64, // 0,1,2,3
            payload: CueCommand::Text(format!("t{i}")).into(),
            duration: d,
        })
        .collect();
    let sheet = CueSheet::new(cues);

    let mut schedule = to_talk_schedule(&sheet);
    schedule.tick(100.0); // 全 due
    let ready = schedule.ready();
    assert_eq!(ready.len(), 4);
    for cue in ready {
        assert_eq!(
            cue.duration, 0.0,
            "非有限/負の duration は envelope で 0 へ clamp される"
        );
    }

    // horizon も clamp 後（各 start_time+0）の max=3.0。clamp が horizon にも適用されて
    // いなければ +inf の cue で horizon=∞ となり tick(100) では完了しない（load-bearing）。
    assert!(
        schedule.is_completed(),
        "horizon も clamp 後の値ゆえ tick(100) で占有終了に達し完了する"
    );
}

/// R1.3: 有限・非負の duration は clamp 恒等ゆえ envelope へ**ビット等価**で運ばれる
/// （無変形）。同一 at は記述順（FIFO）で `ready()` に並ぶ。
///
/// これは旧 command.rs の自己充足檻（テスト本体で TalkCue を組んで自身に主張）を、
/// 実 canonical 変換を通す load-bearing な形へ移設したもの（3.2 申し送り）。
#[test]
fn to_talk_schedule_carries_finite_duration_untransformed() {
    let durations = [
        0.0_f64,
        0.05,
        0.25,
        1.0 / 3.0,
        12.345_678_9,
        f64::MIN_POSITIVE,
    ];
    let cues: Vec<Cue> = durations
        .iter()
        .enumerate()
        .map(|(i, &d)| Cue {
            actor: ActorKey::from("0"),
            start_time: 0.0, // 同一 at=0.0 → FIFO 記述順
            payload: CueCommand::Text(format!("t{i}")).into(),
            duration: d,
        })
        .collect();
    let sheet = CueSheet::new(cues);

    let mut schedule = to_talk_schedule(&sheet);
    schedule.tick(0.0);
    let ready = schedule.ready();
    assert_eq!(ready.len(), durations.len(), "全 cue が due（同一 at=0.0）");
    for (i, &d) in durations.iter().enumerate() {
        assert_eq!(
            ready[i].command,
            CueCommand::Text(format!("t{i}")),
            "同一 at 群は記述順（FIFO）で並ぶ"
        );
        assert_eq!(
            ready[i].duration.to_bits(),
            d.to_bits(),
            "duration={d} は envelope へビット等価で運ばれる（無変形）"
        );
        assert_eq!(ready[i].at, 0.0, "発火時刻も無変形で運ぶ");
        assert_eq!(ready[i].actor, ActorKey::from("0"), "演者も無変形で運ぶ");
        // 搬送体は broadcast で複製されて各 CueSink へ届く（Clone/PartialEq 導出の確認）。
        assert_eq!(ready[i].clone(), ready[i]);
    }
}

/// R1.1/R1.2: 搬送体の duration は**コマンド種別を問わない一律フィールド**。全 10 variant が
/// 瞬時（明示的 0）・時間占有の双方を canonical 変換で運べる（旧 command.rs 檻の移設）。
#[test]
fn to_talk_schedule_duration_uniform_across_every_command_variant() {
    use dola::DynamicValue;

    let commands = vec![
        CueCommand::Text("hello".into()),
        CueCommand::Clear,
        CueCommand::Emote {
            key: "smile".into(),
        },
        CueCommand::Choice {
            id: "yes".into(),
            text: "はい".into(),
            references: vec![],
        },
        CueCommand::EntityRef(42),
        CueCommand::Custom {
            command: "fade".into(),
            params: DynamicValue::Null,
        },
        CueCommand::NewLine { ratio: 1.0 },
        CueCommand::BalloonSurface { key: "2".into() },
        CueCommand::Wait,
        CueCommand::ClearAll,
    ];
    assert_eq!(commands.len(), 10, "presentation コマンドは 10 種");

    for command in commands {
        for duration in [0.0_f64, 1.25] {
            let sheet = CueSheet::new(vec![Cue {
                actor: ActorKey::from("0"),
                start_time: 0.0,
                payload: CuePayload::Command(command.clone()),
                duration,
            }]);
            let mut schedule = to_talk_schedule(&sheet);
            schedule.tick(0.0);
            let ready = schedule.ready();
            assert_eq!(ready.len(), 1);
            assert_eq!(
                ready[0].duration.to_bits(),
                duration.to_bits(),
                "{command:?} の搬送体も duration を一律に保持する"
            );
            assert_eq!(ready[0].command, command, "command は無変形で運ばれる");
        }
    }
}

/// R5.4: 純粋な待ち（`CueCommand::Wait`）の時間も搬送体の duration が運ぶ（action は空）。
#[test]
fn to_talk_schedule_carries_wait_duration_without_action() {
    let sheet = CueSheet::new(vec![Cue {
        actor: ActorKey::from("0"),
        start_time: 0.0,
        payload: CueCommand::Wait.into(),
        duration: 0.5,
    }]);

    let mut schedule = to_talk_schedule(&sheet);
    schedule.tick(0.0);
    let ready = schedule.ready();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].command, CueCommand::Wait, "action は持たない");
    assert_eq!(
        ready[0].duration, 0.5,
        "Wait の時間は搬送体の duration が運ぶ（コマンド埋め込みでない）"
    );
}

/// R1.6: Barrier / Routing ペイロードは skip されず Entry::Barrier / Entry::Routing へ
/// **経路づけ**られる（旧 sakura to_schedule は skip していた・canonical は routing する）。
#[test]
fn to_talk_schedule_routes_barrier_and_routing_not_skipping() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.0,
            payload: CueCommand::Text("keep".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.1,
            payload: CuePayload::Routing(RoutingCommand::RouteRemove {
                target: CueTarget::Shell,
            }),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.2,
            payload: CuePayload::Barrier(BarrierKind::WaitForInput { timeout: None }),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.3,
            payload: CueCommand::Text("after".into()).into(),
            duration: 0.0,
        },
    ]);

    let mut schedule = to_talk_schedule(&sheet);
    schedule.tick(1.0);

    // Barrier 手前の Command のみ配信・Routing は next_routing へ・Barrier で停止。
    let ready = schedule.ready();
    assert_eq!(ready.len(), 1, "Barrier 手前の Command のみ配信");
    assert_eq!(ready[0].command, CueCommand::Text("keep".into()));
    assert!(
        schedule.next_routing().is_some(),
        "Routing は skip されず Entry::Routing へ経路づけられる"
    );
    assert!(
        schedule.current_barrier().is_some(),
        "Barrier は skip されず Entry::Barrier で停止する"
    );
    assert_eq!(schedule.remaining(), 1, "Barrier 後の after は未配信");
}

/// 変換は `absolute_start_time` をスケジュールのアンカー（絶対時刻）に用いる。
/// 相対 start_time は不変で運ばれる（アンカー＋相対＝絶対発火時刻）。
#[test]
fn to_talk_schedule_anchors_schedule_at_absolute_start_time() {
    let sheet = CueSheet::new(vec![Cue {
        actor: ActorKey::from("0"),
        start_time: 0.5,
        payload: CueCommand::Text("hi".into()).into(),
        duration: 0.0,
    }])
    .with_absolute_start_time(100.0);

    let mut schedule = to_talk_schedule(&sheet);
    // アンカー未満は noop（絶対時刻駆動）。
    schedule.tick(50.0);
    assert!(schedule.ready().is_empty());
    // アンカー到達直後（offset 0）でも cue は at=0.5 ゆえ未 due。
    schedule.tick(100.0);
    assert!(schedule.ready().is_empty());
    // 絶対 100.5（offset 0.5）で due。
    schedule.tick(100.5);
    assert_eq!(schedule.ready().len(), 1);
    assert_eq!(
        schedule.ready()[0].at,
        0.5,
        "相対 start_time は不変で運ばれる"
    );
}

/// R2.5: 末尾に待ちを持つ台本を注入時刻で駆動したとき、全 cue の**配送完了時点**では
/// まだ未完了と判定され、**占有終了 horizon** 到達で初めて完了と判定される（早期終了しない）。
#[test]
fn to_talk_schedule_not_completed_until_occupancy_horizon_reached() {
    // Text@0(dur D) → Wait@D(dur 0.8)。占有 horizon = D+0.8。
    let d = 0.15_f64;
    let wait_dur = 0.8_f64;
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.0,
            payload: CueCommand::Text("bye".into()).into(),
            duration: d,
        },
        Cue {
            actor: ActorKey::from("0"),
            start_time: d,
            payload: CueCommand::Wait.into(),
            duration: wait_dur,
        },
    ]);

    let mut schedule = to_talk_schedule(&sheet);
    // 全 entry を配り終える時刻（末尾 Wait の発火時刻＝D）まで進める。
    schedule.tick(d);
    assert_eq!(schedule.remaining(), 0, "全 entry を配り終えた");
    assert!(
        !schedule.is_completed(),
        "配送完了時点では占有 horizon（D+0.8）未到達ゆえ未完了（早期終了しない）"
    );
    // horizon 直前でもまだ未完了。
    schedule.tick(d + wait_dur - 0.01);
    assert!(!schedule.is_completed(), "占有 horizon 直前は未完了");
    // horizon 到達で初めて完了。
    schedule.tick(d + wait_dur);
    assert!(schedule.is_completed(), "占有 horizon 到達で完了");
}

/// 変換したスケジュールの完了時刻は、台本のみから導ける `absolute_end_time()` に一致する
/// （相対 horizon ＝ `absolute_end_time - absolute_start_time`・3.1 と 3.2 の相互検証）。
#[test]
fn to_talk_schedule_completes_exactly_at_sheet_absolute_end_time() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.0,
            payload: CueCommand::Text("hi".into()).into(),
            duration: 0.25,
        },
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.25,
            payload: CueCommand::Wait.into(),
            duration: 0.5,
        },
    ])
    .with_absolute_start_time(10.0);

    // 台本のみから導ける絶対終了時刻＝10.0 + max(0.25, 0.75) = 10.75。
    let end = sheet.absolute_end_time();
    assert_eq!(end, 10.75);

    let mut schedule = to_talk_schedule(&sheet);
    // 絶対終了時刻の直前は未完了（entry は配り終えていても占有中）。
    schedule.tick(end - 0.001);
    assert!(!schedule.is_completed());
    // 絶対終了時刻到達で完了（horizon = absolute_end_time - absolute_start_time）。
    schedule.tick(end);
    assert!(schedule.is_completed());
}

/// 空台本は時間を占有しない＝どの時刻でも即完了（horizon 0.0・アンカー既定 0.0）。
#[test]
fn to_talk_schedule_of_empty_sheet_is_immediately_completed() {
    let sheet = CueSheet::new(vec![]);
    let mut schedule = to_talk_schedule(&sheet);
    schedule.tick(0.0);
    assert!(schedule.is_completed(), "空台本は占有 horizon 0.0 で即完了");
    assert_eq!(schedule.remaining(), 0);
}
