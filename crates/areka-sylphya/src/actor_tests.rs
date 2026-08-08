use super::*;
use crate::asker::AskerId;
use crate::persist::{Axis, PersistKey, PersistScope};

fn core() -> SylphyaCore {
    SylphyaCore::new()
}

fn asker() -> AskerId {
    AskerId::new("ghost/test")
}

// === SET 分類 3 分岐（决定论檻・全分岐可達）===

#[test]
fn classify_set_effective_key_is_runtime_command() {
    // SET 有効群の正準語彙 → RuntimeCommand。
    assert_eq!(classify_set("surface.num"), SetClass::RuntimeCommand);
    assert_eq!(classify_set("menu"), SetClass::RuntimeCommand);
    assert_eq!(classify_set("seriko.defaultsurface"), SetClass::RuntimeCommand);
}

#[test]
fn classify_set_free_dotted_key_is_store_write() {
    // 正準語彙外の自由 dotted key → StoreWrite（host 区画の受け皿）。
    assert_eq!(classify_set("myplugin.customstate"), SetClass::StoreWrite);
    assert_eq!(classify_set("foo.bar.baz"), SetClass::StoreWrite);
}

#[test]
fn classify_set_canonical_non_effective_is_not_settable() {
    // 正準語彙だが SET 無効 → NotSettable（正典沈黙の areka 裁量）。
    assert_eq!(classify_set("baseware.name"), SetClass::NotSettable);
    assert_eq!(classify_set("system.foo"), SetClass::NotSettable);
    // 汎用名 leaf も正準語彙（username は ShioriQuery・SET 不可）。
    assert_eq!(classify_set("username"), SetClass::NotSettable);
}

#[test]
fn classify_set_three_branches_all_reachable() {
    // 3 分岐が全て到達可能（互いに素）。
    let rc = classify_set("surface.num");
    let sw = classify_set("myplugin.customstate");
    let ns = classify_set("baseware.name");
    assert_ne!(rc, sw);
    assert_ne!(sw, ns);
    assert_ne!(rc, ns);
}

#[test]
fn classify_set_unparseable_key_is_store_write() {
    // parse 不能 key（正準語彙ではない）→ StoreWrite 分岐（決定論・無音失敗なし）。
    assert_eq!(classify_set("a..b"), SetClass::StoreWrite);
    assert_eq!(classify_set(""), SetClass::StoreWrite);
}

// === apply の効果列（純関数決定論）===

#[test]
fn apply_set_effective_emits_runtime_command_reserved() {
    let msg = SylphyaMsg::Set {
        asker: asker(),
        key: "surface.num".into(),
        value: "5".into(),
    };
    let effects = core().apply(&msg);
    assert_eq!(
        effects,
        vec![Effect::RuntimeCommandReserved {
            asker: asker(),
            key: "surface.num".into(),
            value: "5".into(),
        }]
    );
    // RuntimeCommand は鏡像へ書込まない（reserved seam）。
    assert!(!effects.iter().any(|e| matches!(
        e,
        Effect::SetDottedPerAsker { .. } | Effect::SetDottedGlobal { .. }
    )));
}

#[test]
fn apply_set_free_emits_host_store_write() {
    let msg = SylphyaMsg::Set {
        asker: asker(),
        key: "myplugin.customstate".into(),
        value: "on".into(),
    };
    let effects = core().apply(&msg);
    // 自由 key は asker 別 host（dotted per-asker）区画へ正準形で反映。
    assert_eq!(
        effects,
        vec![Effect::SetDottedPerAsker {
            asker: asker(),
            key: "myplugin.customstate".into(),
            value: "on".into(),
        }]
    );
}

#[test]
fn apply_set_not_settable_emits_no_write() {
    let msg = SylphyaMsg::Set {
        asker: asker(),
        key: "baseware.name".into(),
        value: "x".into(),
    };
    let effects = core().apply(&msg);
    assert_eq!(
        effects,
        vec![Effect::NotSettable {
            asker: asker(),
            key: "baseware.name".into(),
            value: "x".into(),
        }]
    );
    // 書込効果は一切出ない（呼出は Ok だが非反映）。
    assert!(!effects.iter().any(|e| matches!(
        e,
        Effect::SetDottedPerAsker { .. }
            | Effect::SetDottedGlobal { .. }
            | Effect::SetFlatPerAsker { .. }
    )));
}

#[test]
fn apply_publish_shiori_some_sets_flat_per_asker() {
    let msg = SylphyaMsg::PublishShiori {
        asker: asker(),
        name: "username".into(),
        value: Some("Alice".into()),
    };
    assert_eq!(
        core().apply(&msg),
        vec![Effect::SetFlatPerAsker {
            asker: asker(),
            name: "username".into(),
            value: "Alice".into(),
        }]
    );
}

#[test]
fn apply_publish_shiori_none_records_absent_no_default() {
    let msg = SylphyaMsg::PublishShiori {
        asker: asker(),
        name: "username".into(),
        value: None,
    };
    let effects = core().apply(&msg);
    // 204/失敗 → 不在の観測記録のみ。既定値は sakura 所有ゆえ鏡像へ書かない。
    assert_eq!(
        effects,
        vec![Effect::RecordAbsentFlat {
            asker: asker(),
            name: "username".into(),
        }]
    );
    assert!(!effects.iter().any(|e| matches!(e, Effect::SetFlatPerAsker { .. })));
}

#[test]
fn apply_publish_static_flat_per_asker_and_dotted_global() {
    let msg = SylphyaMsg::PublishStatic {
        asker: asker(),
        flat: vec![
            ("selfname".into(), "さくら".into()),
            ("keroname".into(), "うにゅう".into()),
        ],
        dotted: vec![
            ("baseware.name".into(), "areka".into()),
            ("baseware.version".into(), "1.0".into()),
        ],
    };
    let effects = core().apply(&msg);
    assert_eq!(
        effects,
        vec![
            Effect::SetFlatPerAsker {
                asker: asker(),
                name: "selfname".into(),
                value: "さくら".into(),
            },
            Effect::SetFlatPerAsker {
                asker: asker(),
                name: "keroname".into(),
                value: "うにゅう".into(),
            },
            Effect::SetDottedGlobal {
                key: "baseware.name".into(),
                value: "areka".into(),
            },
            Effect::SetDottedGlobal {
                key: "baseware.version".into(),
                value: "1.0".into(),
            },
        ]
    );
}

#[test]
fn apply_persist_put_projects_to_dotted_global_and_saves() {
    let entries = vec![
        (PersistKey::WindowPos { scope: 0, axis: Axis::X }, "10".to_string()),
        (PersistKey::BootCount, "3".to_string()),
    ];
    let msg = SylphyaMsg::PersistPut {
        scope: PersistScope::Ghost,
        entries: entries.clone(),
        reply: None,
    };
    let effects = core().apply(&msg);
    assert_eq!(
        effects,
        vec![
            Effect::SetDottedGlobal {
                key: "areka.window.scope(0).x".into(),
                value: "10".into(),
            },
            Effect::SetDottedGlobal {
                key: "areka.boot.count".into(),
                value: "3".into(),
            },
            Effect::PersistSave {
                scope: PersistScope::Ghost,
                entries,
            },
        ]
    );
}

#[test]
fn apply_close_emits_stop() {
    assert_eq!(core().apply(&SylphyaMsg::Close), vec![Effect::Stop]);
}

#[test]
fn apply_barrier_emits_barrier() {
    let (tx, _rx) = areka_actor::reply_channel::<()>();
    let msg = SylphyaMsg::Barrier { reply: tx };
    assert_eq!(core().apply(&msg), vec![Effect::Barrier]);
}

// === design Monitoring 固定ログ: アクター適用記録 debug!（R9.3 サインオフ証跡・Task 10.1）===

/// SET store-write の適用時に、design Monitoring の固定ログ
/// `debug!(target: "areka_sylphya::actor", ...)`（publish/SET/persist の適用記録）が発火する
/// （R9.3 grep 証跡・無音でない適用記録・R8.1）。`apply` は純関数ゆえテストスレッド上で駆動でき、
/// interest-keeper 経由 [`crate::test_log_capture::capture`] で並列負荷下でも決定論捕捉する
/// （bare `with_default` 不使用）。ログが削除・target/レベル変更されると本檻が落ちる。
#[test]
fn apply_store_write_emits_actor_debug_log() {
    use crate::test_log_capture::{assert_logged, capture};

    let events = capture(|| {
        let _ = core().apply(&SylphyaMsg::Set {
            asker: asker(),
            key: "myplugin.customstate".into(),
            value: "on".into(),
        });
    });
    assert_logged(
        &events,
        tracing::Level::DEBUG,
        LOG_TARGET,
        "SET store-write to host dotted region",
    );
}

/// PublishShiori{None}（204/失敗＝不在記録）の適用時にも同 target の固定 debug! が発火する
/// （SHIORI 照会系の適用記録・無音でない不在縮退・R8.1）。
#[test]
fn apply_publish_shiori_absent_emits_actor_debug_log() {
    use crate::test_log_capture::{assert_logged, capture};

    let events = capture(|| {
        let _ = core().apply(&SylphyaMsg::PublishShiori {
            asker: asker(),
            name: "username".into(),
            value: None,
        });
    });
    assert_logged(
        &events,
        tracing::Level::DEBUG,
        LOG_TARGET,
        "shiori resource absent",
    );
}

// === 決定論（同一入力 → 同一効果列・I/O なし）===

#[test]
fn apply_is_deterministic_across_variants() {
    let c = core();
    let msgs = vec![
        SylphyaMsg::PublishStatic {
            asker: asker(),
            flat: vec![("selfname".into(), "x".into())],
            dotted: vec![("baseware.name".into(), "areka".into())],
        },
        SylphyaMsg::PublishShiori {
            asker: asker(),
            name: "username".into(),
            value: None,
        },
        SylphyaMsg::Set {
            asker: asker(),
            key: "myplugin.k".into(),
            value: "v".into(),
        },
        SylphyaMsg::Set {
            asker: asker(),
            key: "surface.num".into(),
            value: "1".into(),
        },
        SylphyaMsg::Close,
    ];
    // 10 周: 同一入力 → 同一効果列（決定論檻）。
    for _ in 0..10 {
        for m in &msgs {
            assert_eq!(c.apply(m), c.apply(m));
        }
    }
}

#[test]
fn apply_persist_put_deterministic_same_ref() {
    // reply を含む msg は Clone 不能ゆえ同一参照で 2 回呼んで決定論を確認。
    let msg = SylphyaMsg::PersistPut {
        scope: PersistScope::Ghost,
        entries: vec![(PersistKey::VanishCount, "0".to_string())],
        reply: None,
    };
    let c = core();
    assert_eq!(c.apply(&msg), c.apply(&msg));
}

// RuntimeCommandSink は型予約のみ（M1 未配線）。トレイトが Send で dispatch を持つことを型で確認。
struct NoopSink;
impl RuntimeCommandSink for NoopSink {
    fn dispatch(&self, _asker: &AskerId, _key: &str, _value: &str) {}
}

#[test]
fn runtime_command_sink_trait_is_reserved() {
    fn assert_send<T: Send>() {}
    assert_send::<NoopSink>();
    let sink: Box<dyn RuntimeCommandSink> = Box::new(NoopSink);
    sink.dispatch(&asker(), "surface.num", "1");
}
