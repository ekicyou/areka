use super::*;
use crate::asker::AskerId;
use crate::keytable::Refusal;
use crate::persist::{Axis, PersistKey, PersistScope};
use crate::test_log_capture::{assert_logged, capture};
use std::path::PathBuf;

fn asker() -> AskerId {
    AskerId::new("ghost/test")
}

// === SET 分類 3 分岐（決定論檻・全分岐可達）===

#[test]
fn classify_set_effective_key_is_runtime_command() {
    assert_eq!(classify_set("surface.num"), SetClass::RuntimeCommand);
    assert_eq!(classify_set("menu"), SetClass::RuntimeCommand);
    assert_eq!(
        classify_set("seriko.defaultsurface"),
        SetClass::RuntimeCommand
    );
}

#[test]
fn classify_set_free_dotted_key_is_store_write() {
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
fn classify_set_unparseable_key_is_store_write() {
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
    let effects = apply(&msg);
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
    let effects = apply(&msg);
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
    let effects = apply(&msg);
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
        apply(&msg),
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
    let effects = apply(&msg);
    // 204/失敗 → 不在の観測記録のみ。既定値は sakura 所有ゆえ鏡像へ書かない。
    assert_eq!(
        effects,
        vec![Effect::RecordAbsentFlat {
            asker: asker(),
            name: "username".into(),
        }]
    );
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::SetFlatPerAsker { .. }))
    );
}

#[test]
fn apply_remove_flat_emits_one_removal_per_named_flat_only() {
    let msg = SylphyaMsg::RemoveFlat {
        asker: asker(),
        names: vec!["selfname2".into(), "keroname".into()],
    };
    // 名指しの名前だけを消す（`username` など名指しでない名前の効果は出ない・書込も無い）。
    assert_eq!(
        apply(&msg),
        vec![
            Effect::RemoveFlatPerAsker {
                asker: asker(),
                name: "selfname2".into(),
            },
            Effect::RemoveFlatPerAsker {
                asker: asker(),
                name: "keroname".into(),
            },
        ]
    );
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
    let effects = apply(&msg);
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
fn apply_persist_put_only_saves_without_snapshot_effects() {
    let entries = vec![
        (
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X,
            },
            "10".to_string(),
        ),
        (PersistKey::BootCount, "3".to_string()),
    ];
    let msg = SylphyaMsg::PersistPut {
        scope: PersistScope::Ghost,
        entries: entries.clone(),
    };
    assert_eq!(
        apply(&msg),
        vec![Effect::PersistSave {
            scope: PersistScope::Ghost,
            entries,
        }]
    );
}

#[test]
fn apply_close_emits_stop() {
    assert_eq!(apply(&SylphyaMsg::Close), vec![Effect::Stop]);
}

#[test]
fn apply_barrier_emits_barrier() {
    let (tx, _rx) = areka_actor::reply_channel::<()>();
    let msg = SylphyaMsg::Barrier { reply: tx };
    assert_eq!(apply(&msg), vec![Effect::Barrier]);
}

// === アクター適用記録の固定ログ debug!===

/// SET store-write の適用時に、固定ログ
/// `debug!(target: "areka_sylphya::actor", ...)`（publish/SET/persist の適用記録）が発火する
/// （無音でない適用記録）。`apply` は純関数ゆえテストスレッド上で駆動でき、
/// 共有機構経由 [`crate::test_log_capture::capture`] で並列負荷下でも決定論捕捉する
/// （bare `with_default` 不使用）。ログが削除・target/レベル変更されると本檻が落ちる。
#[test]
fn apply_store_write_emits_actor_debug_log() {
    let events = capture(|| {
        let _ = apply(&SylphyaMsg::Set {
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
/// （SHIORI 照会系の適用記録・無音でない不在縮退）。
#[test]
fn apply_publish_shiori_absent_emits_actor_debug_log() {
    let events = capture(|| {
        let _ = apply(&SylphyaMsg::PublishShiori {
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

#[test]
fn apply_persist_remove_only_removes_from_memory_without_snapshot_effects() {
    let keys = vec![
        PersistKey::BalloonOffset {
            scope: 0,
            axis: Axis::X,
        },
        PersistKey::BalloonOffset {
            scope: 0,
            axis: Axis::Y,
        },
    ];
    let msg = SylphyaMsg::PersistRemove {
        scope: PersistScope::Ghost,
        keys: keys.clone(),
    };
    assert_eq!(
        apply(&msg),
        vec![Effect::PersistRemove {
            scope: PersistScope::Ghost,
            keys,
        }]
    );
}

#[test]
fn apply_set_on_a_persisted_name_is_not_settable() {
    let msg = SylphyaMsg::Set {
        asker: asker(),
        key: "areka.boot.count".into(),
        value: "9".into(),
    };
    assert_eq!(
        apply(&msg),
        vec![Effect::NotSettable {
            asker: asker(),
            key: "areka.boot.count".into(),
            value: "9".into(),
        }]
    );
}

/// 断る名前ごとに warn を 1 件残す（運行の命令・書けない正準の語彙・決まった書き手だけの鍵）。
#[test]
fn apply_set_refusals_each_warn() {
    for (key, text) in [
        ("surface.num", "SET on runtime-command vocab"),
        ("baseware.name", "SET on non-settable canonical vocab"),
        ("areka.boot.count", "only fixed writers"),
    ] {
        let events = capture(|| {
            let _ = apply(&SylphyaMsg::Set {
                asker: asker(),
                key: key.into(),
                value: "1".into(),
            });
        });
        assert_logged(&events, tracing::Level::WARN, LOG_TARGET, text);
    }
}

fn write(writer: Writer, puts: &[(&str, &str)], removes: &[&str]) -> Vec<Effect> {
    let (reply, _rx) = areka_actor::reply_channel();
    apply(&SylphyaMsg::Write {
        writer,
        asker: asker(),
        layer: PersistScope::Shell,
        puts: puts
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        removes: removes.iter().map(|k| k.to_string()).collect(),
        reply,
    })
}

#[test]
fn apply_write_by_name_of_a_free_key_writes_the_folded_key() {
    assert_eq!(
        write(Writer::ByName, &[("MyPlugin.State", "on")], &[]),
        vec![Effect::Write {
            asker: asker(),
            layer: PersistScope::Shell,
            puts: vec![("myplugin.state".into(), "on".into())],
            removes: vec![],
        }]
    );
}

#[test]
fn apply_write_by_name_of_refused_names_writes_nothing_and_warns() {
    for (key, refusal, text) in [
        (
            "surface.num",
            Refusal::RuntimeCommand,
            "SET on runtime-command vocab",
        ),
        (
            "baseware.name",
            Refusal::NotSettable,
            "SET on non-settable canonical vocab",
        ),
        (
            "areka.boot.count",
            Refusal::FixedWriterOnly,
            "only fixed writers",
        ),
    ] {
        let mut effects = Vec::new();
        let events = capture(|| effects = write(Writer::ByName, &[("free.key", "1")], &[key]));
        assert_eq!(
            effects,
            vec![Effect::Refused {
                asker: asker(),
                key: key.into(),
                refusal,
            }],
            "{key}"
        );
        assert_logged(&events, tracing::Level::WARN, LOG_TARGET, text);
    }
}

#[test]
fn apply_write_by_fixed_writer_skips_the_name_rule() {
    assert_eq!(
        write(
            Writer::Fixed,
            &[("areka.boot.count", "4")],
            &["areka.vanish.count"]
        ),
        vec![Effect::Write {
            asker: asker(),
            layer: PersistScope::Shell,
            puts: vec![("areka.boot.count".into(), "4".into())],
            removes: vec!["areka.vanish.count".into()],
        }]
    );
}

#[test]
fn apply_reads_emit_no_effects() {
    let (reply, _rx) = areka_actor::reply_channel();
    assert_eq!(
        apply(&SylphyaMsg::Get {
            asker: asker(),
            key: "areka.boot.count".into(),
            layer: None,
            home: crate::keytable::Home::File,
            reply,
        }),
        vec![]
    );
    let (reply, _rx) = areka_actor::reply_channel();
    assert_eq!(
        apply(&SylphyaMsg::GetFlat {
            asker: asker(),
            name: "username".into(),
            reply,
        }),
        vec![]
    );
    let (reply, _rx) = areka_actor::reply_channel();
    assert_eq!(
        apply(&SylphyaMsg::TalkSnapshot {
            asker: asker(),
            reply,
        }),
        vec![]
    );
}

#[test]
fn apply_set_layer_root_emits_the_re_root() {
    assert_eq!(
        apply(&SylphyaMsg::SetLayerRoot {
            layer: PersistScope::Balloon,
            root: Some(PathBuf::from("/b")),
        }),
        vec![Effect::SetLayerRoot {
            layer: PersistScope::Balloon,
            root: Some(PathBuf::from("/b")),
        }]
    );
}
