use super::*;

#[test]
fn barrier_kind_clone_debug_partial_eq() {
    let barrier = BarrierKind::WaitForInput { timeout: Some(5.0) };
    let clone = barrier.clone();
    assert_eq!(barrier, clone);
    let debug = format!("{:?}", barrier);
    assert!(debug.contains("WaitForInput"));
}

#[test]
fn barrier_kind_three_variants() {
    let _ = BarrierKind::WaitForInput { timeout: None };
    let _ = BarrierKind::WaitForChoice {
        timeout: Some(30.0),
    };
    let _ = BarrierKind::Timeout { duration: 5.0 };
}

#[test]
fn routing_command_three_variants() {
    let actor = ActorKey::from("sakura");
    let _ = RoutingCommand::RouteAdd {
        target: CueTarget::Balloon,
        to: EntityKey::Actor(actor.clone(), CueTarget::Balloon),
    };
    let _ = RoutingCommand::RouteSwitch {
        target: CueTarget::Shell,
        to: EntityKey::Spot("spot1".into()),
    };
    let _ = RoutingCommand::RouteRemove {
        target: CueTarget::Balloon,
    };
}

#[test]
fn cue_command_ten_variants() {
    let cmds = vec![
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
    assert_eq!(cmds.len(), 10);

    // Clone + Debug + PartialEq
    for cmd in &cmds {
        let clone = cmd.clone();
        assert_eq!(*cmd, clone);
        let _ = format!("{:?}", cmd);
    }
}

#[test]
fn cue_payload_from_conversions() {
    let cmd = CueCommand::Text("test".into());
    let payload: CuePayload = cmd.into();
    assert!(matches!(payload, CuePayload::Command(CueCommand::Text(_))));

    let barrier = BarrierKind::WaitForInput { timeout: None };
    let payload: CuePayload = barrier.into();
    assert!(matches!(payload, CuePayload::Barrier(_)));

    let routing = RoutingCommand::RouteRemove {
        target: CueTarget::Shell,
    };
    let payload: CuePayload = routing.into();
    assert!(matches!(payload, CuePayload::Routing(_)));
}

#[test]
fn domain_types_serde_roundtrip() {
    let actor = ActorKey::from("sakura");
    let json = serde_json::to_string(&actor).unwrap();
    let parsed: ActorKey = serde_json::from_str(&json).unwrap();
    assert_eq!(actor, parsed);

    let target = CueTarget::Balloon;
    let json = serde_json::to_string(&target).unwrap();
    let parsed: CueTarget = serde_json::from_str(&json).unwrap();
    assert_eq!(target, parsed);

    let cmd = CueCommand::Text("hello".into());
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: CueCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(cmd, parsed);

    let barrier = BarrierKind::Timeout { duration: 3.0 };
    let json = serde_json::to_string(&barrier).unwrap();
    let parsed: BarrierKind = serde_json::from_str(&json).unwrap();
    assert_eq!(barrier, parsed);
}

#[test]
fn cue_command_newline_serde_roundtrip() {
    let cmd = CueCommand::NewLine { ratio: 1.5 };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: CueCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(cmd, parsed);
}

#[test]
fn cue_command_balloon_surface_serde_roundtrip() {
    let cmd = CueCommand::BalloonSurface { key: "2".into() };
    let json = serde_json::to_string(&cmd).unwrap();
    // externally tagged で additive（既存 variant のワイヤ形は不変）。
    assert_eq!(json, r#"{"BalloonSurface":{"key":"2"}}"#);
    let parsed: CueCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(cmd, parsed);

    // 名前形・非表示センチネルも不透明のまま忠実にラウンドトリップする。
    for key in ["バルーン１", "-1", "10"] {
        let cmd = CueCommand::BalloonSurface { key: key.into() };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: CueCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed, "BalloonSurface(key={key:?}) must roundtrip");
    }
}

/// `Cursor`（カーソル位置指定 `\_l` の不透明転写）の additive 追加檻（3.1・3.2・8.1）。
///
/// - x・y は**記述通りの不透明 String**であり、単位付き（`5em`/`2lh`/`50%`）・裸数値・
///   相対（`@` 前置）・空（省略）の区別を失わず往復する（dola は換算/解決しない・3.2/3.3）。
/// - externally tagged で additive（既存 variant のワイヤ形は不変・8.1）。
#[test]
fn cue_command_cursor_serde_roundtrip() {
    // 代表形: 単位付き。設計「Data Models / ワイヤ形」表の Cursor 行と**バイト同一**。
    let cmd = CueCommand::Cursor {
        x: "5em".into(),
        y: "2lh".into(),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    assert_eq!(json, r#"{"Cursor":{"x":"5em","y":"2lh"}}"#);
    let parsed: CueCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(cmd, parsed);

    // 双方空（省略）の区別を保つ——空でも欠落させず忠実に往復する（3.2・3.5）。
    let empty = CueCommand::Cursor {
        x: "".into(),
        y: "".into(),
    };
    let json = serde_json::to_string(&empty).unwrap();
    assert_eq!(json, r#"{"Cursor":{"x":"","y":""}}"#);
    let parsed: CueCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(empty, parsed);
    // 空は代表形と別物（区別保持）。
    assert_ne!(empty, cmd);

    // 裸数値・相対（`@` 前置）・%・片側空のいずれも不透明のまま忠実に往復する（3.2）。
    for (x, y) in [
        ("50", "100"),  // 裸数値
        ("@10", "@-5"), // 相対（`@` 前置）
        ("50%", "25%"), // パーセント
        ("5em", ""),    // 片側のみ空（区別保持）
        ("", "2lh"),    // もう片側のみ空
    ] {
        let cmd = CueCommand::Cursor {
            x: x.into(),
            y: y.into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: CueCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(
            cmd, parsed,
            "Cursor(x={x:?}, y={y:?}) must roundtrip opaquely"
        );
    }
}

/// `Wait`（純粋な待ち）・`ClearAll`（全スコープ消去）の additive 追加檻。
///
/// 両者は unit variant ゆえ externally tagged のワイヤ形は裸の文字列
/// （`"Wait"` / `"ClearAll"`）になる（5.2・6.3）。
#[test]
fn cue_command_wait_and_clear_all_serde_roundtrip() {
    for (cmd, wire) in [
        (CueCommand::Wait, r#""Wait""#),
        (CueCommand::ClearAll, r#""ClearAll""#),
    ] {
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, wire, "{cmd:?} は unit variant のワイヤ形を持つ");
        let parsed: CueCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed, "{cmd:?} は往復で同一へ復元される");
    }
}

/// `Clear`（対象スコープのみ消去）と `ClearAll`（全スコープ消去）は峻別される
/// 別コマンドであり、ワイヤ上でも取り違えられない（6.3）。
#[test]
fn clear_and_clear_all_are_distinct_commands() {
    assert_ne!(CueCommand::Clear, CueCommand::ClearAll);
    assert_ne!(
        serde_json::to_string(&CueCommand::Clear).unwrap(),
        serde_json::to_string(&CueCommand::ClearAll).unwrap()
    );

    // 一方のワイヤ形が他方へ deserialize されることはない。
    let parsed: CueCommand = serde_json::from_str(r#""Clear""#).unwrap();
    assert_eq!(parsed, CueCommand::Clear);
    let parsed: CueCommand = serde_json::from_str(r#""ClearAll""#).unwrap();
    assert_eq!(parsed, CueCommand::ClearAll);
}

/// `Wait` は action を持たない（unit variant）——時間は envelope `duration` が担う
/// （D2/D3: payload 埋め込みでなく envelope）。ワイヤ形にも duration は現れない（5.1/5.2）。
#[test]
fn wait_carries_no_action_and_time_lives_in_envelope_duration() {
    // Wait 単体のワイヤ形に時間値は含まれない（payload は裸の "Wait" のみ）。
    assert_eq!(
        serde_json::to_string(&CueCommand::Wait).unwrap(),
        r#""Wait""#
    );

    // 待ち時間は envelope 側 `Cue.duration` が保持する。
    let cue = Cue {
        actor: ActorKey::from("0"),
        start_time: 1.0,
        payload: CueCommand::Wait.into(),
        duration: 0.5,
    };
    let parsed: Cue = serde_json::from_str(&serde_json::to_string(&cue).unwrap()).unwrap();
    assert_eq!(parsed.payload, CueCommand::Wait.into());
    assert_eq!(parsed.start_time, 1.0);
    assert_eq!(
        parsed.duration, 0.5,
        "Wait の時間は envelope duration が担う"
    );
}

// ── R8.1 ワイヤ互換回帰 カバレッジマトリクス（Task 10.5）──
//
// R8.1「既存 cue の外部表現（シリアライズ形）を変えずに additive 拡張し、既存台本データの
// 読み込み互換を保つ」の回帰は、本 spec の 4 つの additive デルタ
// （references skip_serializing・Cursor variant・`\!` キャリア形・CueTarget::Window）が
// 既存ワイヤ形のバイト列を一切変えないことを、以下の既存檻が**期待 JSON リテラル**で
// 網羅固定している（設計 Data Models「ワイヤ形」表・Testing Strategy Integration#5 の
// 「ワイヤ檻〔既存 8 variant 無改変緑＋Cursor/references/キャリア形の追加檻〕」に一致）。
// Task 10.5 監査結論: 下記で完全網羅ゆえ、同一リテラルを再主張する檻の新設は
// 重複（[[test-only-decision-branches-not-proven-wiring]]）——追加せず、この対応表を正典とする。
//
//   R8.1 デルタ／義務                     | 固定する檻（本モジュール）
//   ------------------------------------ | ------------------------------------------------
//   既存 8 variant がバイト同一（現行spec）| existing_eight_variants_wire_forms_are_unchanged_by_additive_extension
//     └ Choice(references:vec![]) 込み    |   （8th variant の期待 JSON に references キー無し＝skip 後を固定）
//   references 空＝現行とバイト同一        | choice_references_additive_wire_form (a)
//   references あり＝キー付き記述順        | choice_references_additive_wire_form (b)
//   references 欠落の旧 Choice を default 読 | choice_references_additive_wire_form (c)
//   Cursor（additive・空/単位/相対/裸/%）  | cue_command_cursor_serde_roundtrip
//   `\!` キャリア（Custom params 正準形）   | command_carrier_wire_form_is_canonical
//   キャリア非正準は None（良性スキップ）    | as_command_carrier_returns_none_for_non_canonical
//   CueTarget::Window（additive unit・既存不変）| cue_target_window_is_additive_unit_variant
//   Cue.duration 欠落の旧資産を 0 で読む     | cue_duration_defaults_to_zero_for_legacy_serialized_data
//   Wait/ClearAll unit variant のワイヤ形    | cue_command_wait_and_clear_all_serde_roundtrip
//
// 注記: Cursor/キャリア(Custom)/Window は externally-tagged enum への variant 追加ゆえ、
// 既存 variant のバイト列は構造的に不変（新タグは既存タグの表現へ干渉しない）。上記 8-variant
// 檻はその不変を期待リテラルで固定し、万一の破壊を回帰として捕捉する load-bearing な檻である。

/// 既存 8 variant のワイヤ形が Wait/ClearAll の additive 追加後も**完全に不変**である
/// ことを、期待 JSON リテラルで固定する（5.2・6.3・9.3）。
///
/// Task 10.5 監査: Choice の期待 JSON は `references` キーを持たない（`references: vec![]`
/// が `skip_serializing_if = "Vec::is_empty"` で消える形）ため、本檻は references 追加**後**の
/// 8-variant バイト同一を固定する consolidated 回帰檻でもある（R8.1・上の対応表参照）。
#[test]
fn existing_eight_variants_wire_forms_are_unchanged_by_additive_extension() {
    let expected: Vec<(CueCommand, &str)> = vec![
        (CueCommand::Text("hello".into()), r#"{"Text":"hello"}"#),
        (CueCommand::Clear, r#""Clear""#),
        (
            CueCommand::Emote {
                key: "smile".into(),
            },
            r#"{"Emote":{"key":"smile"}}"#,
        ),
        (
            CueCommand::Choice {
                id: "yes".into(),
                text: "はい".into(),
                references: vec![],
            },
            r#"{"Choice":{"id":"yes","text":"はい"}}"#,
        ),
        (CueCommand::EntityRef(42), r#"{"EntityRef":42}"#),
        (
            CueCommand::Custom {
                command: "fade".into(),
                params: DynamicValue::Null,
            },
            r#"{"Custom":{"command":"fade","params":null}}"#,
        ),
        (
            CueCommand::NewLine { ratio: 1.0 },
            r#"{"NewLine":{"ratio":1.0}}"#,
        ),
        (
            CueCommand::BalloonSurface { key: "2".into() },
            r#"{"BalloonSurface":{"key":"2"}}"#,
        ),
    ];
    assert_eq!(expected.len(), 8, "既存 variant は 8 種");

    for (cmd, wire) in expected {
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(
            json, wire,
            "{cmd:?} のワイヤ形は additive 拡張後も不変でなければならない"
        );
        let parsed: CueCommand = serde_json::from_str(wire).unwrap();
        assert_eq!(parsed, cmd, "既存ワイヤ形の資産は従来どおり読める");
    }
}

/// `Choice.references` の additive 拡張檻（1.3・8.1）。
///
/// - (a) references 空のシリアライズ形は現行と**バイト同一**（`references` キーが現れない
///   ＝`skip_serializing_if = "Vec::is_empty"`）。既存ワイヤ形不変の直接証跡。
/// - (b) references ありは `references` キー付きで往復する（欠落させず記述順を保つ・1.3）。
/// - (c) `references` を持たない旧 JSON 資産は `default` により空 vec へ復元される（後方互換）。
#[test]
fn choice_references_additive_wire_form() {
    // (a) 空 references は現行とバイト同一（キーが現れない）。
    let empty = CueCommand::Choice {
        id: "OnYes".into(),
        text: "はい".into(),
        references: vec![],
    };
    assert_eq!(
        serde_json::to_string(&empty).unwrap(),
        r#"{"Choice":{"id":"OnYes","text":"はい"}}"#,
        "references 空のシリアライズ形は現行とバイト同一（references キーは skip される）"
    );

    // (b) references ありはキー付きで往復し、記述順を保つ。
    let with_refs = CueCommand::Choice {
        id: "OnYes".into(),
        text: "はい".into(),
        references: vec!["r0".into(), "r1".into()],
    };
    let json = serde_json::to_string(&with_refs).unwrap();
    assert_eq!(
        json, r#"{"Choice":{"id":"OnYes","text":"はい","references":["r0","r1"]}}"#,
        "references ありは references キー付きで記述順を保ってシリアライズされる"
    );
    let parsed: CueCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, with_refs, "references ありは往復で同一へ復元される");

    // (c) references を持たない旧 JSON は default で空 vec へ復元される。
    let legacy = r#"{"Choice":{"id":"OnYes","text":"はい"}}"#;
    let parsed: CueCommand = serde_json::from_str(legacy).unwrap();
    assert_eq!(
        parsed, empty,
        "references 欠落の旧資産は default で空 vec へ復元される（後方互換）"
    );
}

#[test]
fn actor_key_hash_eq() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ActorKey::from("sakura"));
    set.insert(ActorKey::from("sakura"));
    assert_eq!(set.len(), 1);
    set.insert(ActorKey::from("kero"));
    assert_eq!(set.len(), 2);
}

// ── D3-T 追加ギャップテスト ──

#[test]
fn actor_key_new_display_as_str() {
    let key = ActorKey::new("sakura");
    assert_eq!(key.as_str(), "sakura");
    assert_eq!(key.to_string(), "sakura"); // Display 実装

    // From<String> / From<&str> の等価性
    assert_eq!(ActorKey::from("kero"), ActorKey::from("kero".to_string()));
}

#[test]
fn entity_key_namespaces_are_distinct() {
    use std::collections::HashSet;
    // 同名でも名前空間（バリアント）が違えば別キー
    let spot = EntityKey::Spot("x".to_string());
    let balloon = EntityKey::Balloon("x".to_string());
    let actor = EntityKey::Actor(ActorKey::from("x"), CueTarget::Shell);
    assert_ne!(spot, balloon);
    assert_ne!(spot, actor);

    let mut set = HashSet::new();
    set.insert(spot);
    set.insert(balloon);
    set.insert(actor);
    assert_eq!(set.len(), 3);

    // Actor の CueTarget スロット違いも別キー
    let mut slots = HashSet::new();
    slots.insert(EntityKey::Actor(ActorKey::from("a"), CueTarget::Shell));
    slots.insert(EntityKey::Actor(ActorKey::from("a"), CueTarget::Balloon));
    assert_eq!(slots.len(), 2);
}

#[test]
fn entity_key_serde_roundtrip() {
    let keys = vec![
        EntityKey::Actor(ActorKey::from("sakura"), CueTarget::Balloon),
        EntityKey::Spot("spot1".to_string()),
        EntityKey::Balloon("balloon1".to_string()),
    ];
    for key in keys {
        let json = serde_json::to_string(&key).unwrap();
        let parsed: EntityKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, parsed);
    }
}

#[test]
fn routing_command_serde_roundtrip() {
    let cmds = vec![
        RoutingCommand::RouteAdd {
            target: CueTarget::Balloon,
            to: EntityKey::Actor(ActorKey::from("sakura"), CueTarget::Balloon),
        },
        RoutingCommand::RouteSwitch {
            target: CueTarget::Shell,
            to: EntityKey::Spot("spot1".to_string()),
        },
        RoutingCommand::RouteRemove {
            target: CueTarget::Shell,
        },
    ];
    for cmd in cmds {
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: RoutingCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }
}

#[test]
fn cue_payload_and_cue_serde_roundtrip() {
    // CuePayload 3 種のラウンドトリップ
    let payloads = vec![
        CuePayload::Command(CueCommand::Custom {
            command: "fade".to_string(),
            params: DynamicValue::Null,
        }),
        CuePayload::Barrier(BarrierKind::WaitForChoice { timeout: Some(3.0) }),
        CuePayload::Routing(RoutingCommand::RouteRemove {
            target: CueTarget::Balloon,
        }),
    ];
    for payload in payloads {
        let json = serde_json::to_string(&payload).unwrap();
        let parsed: CuePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, parsed);
    }

    // Cue 全体（PartialEq 非導出のためフィールドごとに検証）
    let cue = Cue {
        actor: ActorKey::from("sakura"),
        start_time: 1.5,
        payload: CueCommand::Text("hello".into()).into(),
        duration: 0.0,
    };
    let json = serde_json::to_string(&cue).unwrap();
    let parsed: Cue = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.actor, cue.actor);
    assert_eq!(parsed.start_time, cue.start_time);
    assert_eq!(parsed.payload, cue.payload);
    assert_eq!(parsed.duration, cue.duration);
}

// ── D3-V 境界特性化テスト ──

#[test]
fn entity_ref_u64_boundary_serde_roundtrip() {
    // EntityRef は u64 全域（Entity::to_bits() 変換値）を保持する。
    // i64::MAX 超の値（上位ビットが立つ Entity generation 等）も JSON 経由で
    // 欠損なくラウンドトリップすることを固定する（serde_json は u64 を直接扱う。
    // なお TOML の整数は i64 のため、TOML 直列化では i64::MAX 超は表現できない —
    // 現行ワークスペースに CueCommand の TOML 直列化経路はない）。
    for bits in [0u64, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX] {
        let cmd = CueCommand::EntityRef(bits);
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: CueCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed, "EntityRef({bits}) must roundtrip losslessly");
    }
}

#[test]
fn cue_construction() {
    let cue = Cue {
        actor: ActorKey::from("sakura"),
        start_time: 1.5,
        payload: CueCommand::Text("hello".into()).into(),
        duration: 0.0,
    };
    assert_eq!(cue.actor.as_str(), "sakura");
    assert_eq!(cue.start_time, 1.5);
    assert!(matches!(
        cue.payload,
        CuePayload::Command(CueCommand::Text(_))
    ));
}

// ── duration（再生時間）envelope 檻 ──

#[test]
fn cue_duration_defaults_to_zero_for_legacy_serialized_data() {
    // 再生時間フィールドを持たない旧シリアライズ資産（3 フィールド）を読み込むと
    // duration=0（瞬時）として従来どおり解釈できる（後方互換）。
    let legacy = r#"{"actor":"0","start_time":0.0,"payload":{"Command":{"Text":"hi"}}}"#;
    let parsed: Cue = serde_json::from_str(legacy).unwrap();
    assert_eq!(parsed.actor, ActorKey::from("0"));
    assert_eq!(parsed.start_time, 0.0);
    assert_eq!(parsed.payload, CueCommand::Text("hi".into()).into());
    assert_eq!(
        parsed.duration, 0.0,
        "duration 欠落の旧資産は 0（瞬時）として復元されねばならない"
    );

    // Barrier / Routing ペイロード（duration 非該当）の旧資産も同様に読める。
    let legacy_barrier =
        r#"{"actor":"0","start_time":1.0,"payload":{"Barrier":{"WaitForInput":{"timeout":null}}}}"#;
    let parsed: Cue = serde_json::from_str(legacy_barrier).unwrap();
    assert_eq!(parsed.duration, 0.0);
}

#[test]
fn cue_duration_roundtrip_preserves_value() {
    // 新規往復では duration の値が保たれ、既存ペイロードのワイヤ形は変わらない。
    let cue = Cue {
        actor: ActorKey::from("sakura"),
        start_time: 1.5,
        payload: CueCommand::Text("hello".into()).into(),
        duration: 0.25,
    };
    let json = serde_json::to_string(&cue).unwrap();
    assert!(
        json.contains(r#""payload":{"Command":{"Text":"hello"}}"#),
        "既存 variant のワイヤ形は envelope 拡張後も不変: {json}"
    );

    let parsed: Cue = serde_json::from_str(&json).unwrap();
    // Cue は PartialEq 非導出のためフィールドごとに検証する。
    assert_eq!(parsed.actor, cue.actor);
    assert_eq!(parsed.start_time, cue.start_time);
    assert_eq!(parsed.payload, cue.payload);
    assert_eq!(parsed.duration, 0.25);
}

#[test]
fn duration_is_uniform_envelope_field_across_all_payloads() {
    // 「再生時間フィールドを持たない cue」という概念を作らない——
    // 全 presentation cue（CueCommand 全 variant）が envelope duration を保持し、
    // 瞬時コマンドは明示的 0 を持つ（欠落でない）。
    let payloads: Vec<CuePayload> = vec![
        CueCommand::Text("hello".into()).into(),
        CueCommand::Clear.into(),
        CueCommand::Emote {
            key: "smile".into(),
        }
        .into(),
        CueCommand::Choice {
            id: "yes".into(),
            text: "はい".into(),
            references: vec![],
        }
        .into(),
        CueCommand::EntityRef(42).into(),
        CueCommand::Custom {
            command: "fade".into(),
            params: DynamicValue::Null,
        }
        .into(),
        CueCommand::NewLine { ratio: 1.0 }.into(),
        CueCommand::BalloonSurface { key: "2".into() }.into(),
        CueCommand::Wait.into(),
        CueCommand::ClearAll.into(),
        // duration 非該当ペイロード（Barrier / Routing）も envelope としては
        // 一律にフィールドを持つ（値は 0・静的 duration タイムラインの外）。
        BarrierKind::Timeout { duration: 5.0 }.into(),
        RoutingCommand::RouteRemove {
            target: CueTarget::Shell,
        }
        .into(),
    ];

    for payload in payloads {
        // 瞬時（明示的 0）
        let instant = Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: payload.clone(),
            duration: 0.0,
        };
        let parsed: Cue = serde_json::from_str(&serde_json::to_string(&instant).unwrap()).unwrap();
        assert_eq!(parsed.duration, 0.0, "瞬時 cue は明示的 0 を保持する");
        assert_eq!(parsed.payload, instant.payload);

        // 時間占有（不透明秒数）
        let timed = Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload,
            duration: 1.25,
        };
        let parsed: Cue = serde_json::from_str(&serde_json::to_string(&timed).unwrap()).unwrap();
        assert_eq!(parsed.duration, 1.25);
        assert_eq!(parsed.payload, timed.payload);
    }
}

// ── `\!` 汎用キャリア正準形（command_carrier / as_command_carrier）檻（4.1・4.2・8.1）──

/// 往復同一（4.1・4.2）: `as_command_carrier(&command_carrier(n, t)) == Some((n, t))`。
/// 空トークン（省略スロット）を欠落させず保持する（fixture `\![move,-353,,,0,base,base]`
/// は空トークン 2 個を保った 6 トークン列・4.2）。
#[test]
fn command_carrier_round_trip_preserves_empty_tokens() {
    let tokens = vec![
        "-353".to_string(),
        "".to_string(),
        "".to_string(),
        "0".to_string(),
        "base".to_string(),
        "base".to_string(),
    ];
    let cmd = CueCommand::command_carrier("move", tokens.clone());
    let expected: Vec<&str> = tokens.iter().map(String::as_str).collect();
    assert_eq!(
        cmd.as_command_carrier(),
        Some(("move", expected)),
        "往復同一・空トークンを保持する"
    );
}

/// キャリアのワイヤ形（不変条件・8.1）: 正準コンストラクタは `Custom` の
/// `params: Array<String>` 正準形を単一箇所で構築し、設計「Data Models / ワイヤ形」表の
/// `\!` キャリア行と**バイト同一**の JSON になる。
#[test]
fn command_carrier_wire_form_is_canonical() {
    let cmd = CueCommand::command_carrier(
        "move",
        vec![
            "-353".into(),
            "".into(),
            "".into(),
            "0".into(),
            "base".into(),
            "base".into(),
        ],
    );
    let json = serde_json::to_string(&cmd).unwrap();
    assert_eq!(
        json,
        r#"{"Custom":{"command":"move","params":["-353","","","0","base","base"]}}"#
    );
    let parsed: CueCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, cmd);
}

/// 非正準は `None`（消費側は記録付き良性スキップへ縮退）: `Custom` 以外／`params` が
/// Array でない／Array 要素に非 String が混じる、のいずれも `None`。
#[test]
fn as_command_carrier_returns_none_for_non_canonical() {
    // (a) `Custom` 以外の variant は非キャリア → None。
    assert_eq!(CueCommand::Text("hi".into()).as_command_carrier(), None);
    assert_eq!(CueCommand::Wait.as_command_carrier(), None);
    assert_eq!(
        CueCommand::BalloonSurface { key: "2".into() }.as_command_carrier(),
        None
    );

    // (b) `Custom` だが params が Array でない → None。
    assert_eq!(
        CueCommand::Custom {
            command: "move".into(),
            params: DynamicValue::Null,
        }
        .as_command_carrier(),
        None
    );

    // (c) `Custom` かつ Array だが非 String 要素が混じる → None（all-String でない）。
    assert_eq!(
        CueCommand::Custom {
            command: "move".into(),
            params: DynamicValue::Array(vec![
                DynamicValue::String("a".into()),
                DynamicValue::Integer(3),
            ]),
        }
        .as_command_carrier(),
        None
    );

    // 空 Array（トークン 0 個）は正準（all-String を空虚に満たす）→ Some(空列)。
    assert_eq!(
        CueCommand::command_carrier("raise", vec![]).as_command_carrier(),
        Some(("raise", Vec::<&str>::new()))
    );
}

/// `CueTarget::Window` は additive unit variant（8.1）。既存 variant のワイヤ形は不変で、
/// `EntityKey` 参照は非破壊（Window を含むスロットも構築可）。
#[test]
fn cue_target_window_is_additive_unit_variant() {
    let target = CueTarget::Window;
    let json = serde_json::to_string(&target).unwrap();
    assert_eq!(json, r#""Window""#);
    let parsed: CueTarget = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, target);

    // 既存 variant のワイヤ形は additive 追加後も不変。
    assert_eq!(
        serde_json::to_string(&CueTarget::Shell).unwrap(),
        r#""Shell""#
    );
    assert_eq!(
        serde_json::to_string(&CueTarget::Balloon).unwrap(),
        r#""Balloon""#
    );

    // `EntityKey` 参照非破壊（Window を含む Actor スロットも構築可）。
    let key = EntityKey::Actor(ActorKey::from("0"), CueTarget::Window);
    assert_eq!(
        key,
        EntityKey::Actor(ActorKey::from("0"), CueTarget::Window)
    );
}

// ── 配送エンベロープ（TalkCue）檻 ──
//
// NOTE(Task 3.2): 旧 `talk_cue_envelope_carries_cue_duration_untransformed` と
// `talk_cue_duration_is_uniform_across_every_command_variant` は、テスト本体で
// `TalkCue { .., duration: cue.duration }` を組んでから自身に等価を主張する**自己充足檻**
// だった（Rust の構造体代入が自分自身を主張しているだけ・変異テストで素通しを実測）。
// 「無変形の carry」の behavioral な証明は、実 canonical 変換 `dola::cue::to_talk_schedule` を
// 通す `crates/dola/tests/cue/sheet_test.rs` の
// `to_talk_schedule_carries_finite_duration_untransformed` /
// `to_talk_schedule_duration_uniform_across_every_command_variant` が load-bearing に担う
// ため、こちらの自己充足檻は撤去した。
