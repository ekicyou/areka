//! 名前の畳みのアクターの檻（spec areka-P0-property-name-case-fold タスク 1.2・要件 5.2）。
//!
//! 書き込みの仕分けは半角の英字の大小で変わらない。効果と `warn!` が運ぶ名前は書かれた綴りの
//! まま、自由な名前の書き込みの鍵は畳んだ正準の綴りになる。

use super::*;
use crate::asker::{AskerContext, AskerId};
use crate::persist::{FakePersistIo, PersistKey, PersistScope, ScopeRoots};
use crate::test_log_capture::{assert_logged, capture};
use crate::value::DottedResolution;
use std::path::PathBuf;

fn asker() -> AskerId {
    AskerId::new("ghost/test")
}

fn ctx() -> AskerContext {
    AskerContext { asker: asker() }
}

fn spawn_empty() -> SylphyaParts {
    spawn_sylphya(SylphyaInit {
        roots: ScopeRoots::default(),
        io: Box::new(FakePersistIo::new()),
        runtime_sink: None,
    })
}

fn read(parts: &SylphyaParts, key: &str) -> DottedResolution {
    parts.reader.resolve_dotted_str(&ctx(), key)
}

fn value(v: &str) -> DottedResolution {
    DottedResolution::Value(v.into())
}

fn set(key: &str, value: &str) -> Vec<Effect> {
    SylphyaCore::new().apply(&SylphyaMsg::Set {
        asker: asker(),
        key: key.into(),
        value: value.into(),
    })
}

#[test]
fn non_settable_names_in_any_case_stay_not_settable() {
    // 1. 書けない名前の大小違い（要件 3.1）。
    assert_eq!(classify_set("CURRENTGHOST.NAME"), SetClass::NotSettable);
    assert_eq!(classify_set("Baseware.Name"), SetClass::NotSettable);

    let events = capture(|| {
        assert_eq!(
            set("CURRENTGHOST.NAME", "x"),
            vec![Effect::NotSettable {
                asker: asker(),
                key: "CURRENTGHOST.NAME".into(),
                value: "x".into(),
            }]
        );
    });
    assert_logged(
        &events,
        tracing::Level::WARN,
        LOG_TARGET,
        "SET on non-settable canonical vocab",
    );
}

#[test]
fn runtime_command_names_in_any_case_stay_runtime_command() {
    // 2. 書ける名前の大小違い（要件 3.2）。1 段目を畳まずに比べると MENU が自由な名前へ落ちる。
    assert_eq!(classify_set("MENU"), SetClass::RuntimeCommand);
    assert_eq!(classify_set("Surface.Num"), SetClass::RuntimeCommand);

    assert_eq!(
        set("MENU", "1"),
        vec![Effect::RuntimeCommandReserved {
            asker: asker(),
            key: "MENU".into(),
            value: "1".into(),
        }]
    );
    assert_eq!(
        set("Surface.Num", "5"),
        vec![Effect::RuntimeCommandReserved {
            asker: asker(),
            key: "Surface.Num".into(),
            value: "5".into(),
        }]
    );
}

#[test]
fn free_names_in_any_case_write_the_folded_key() {
    // 3. 自由な名前の大小違い 2 綴り（要件 3.3・3.4）。
    for key in ["My.Flag", "MY.FLAG"] {
        assert_eq!(classify_set(key), SetClass::StoreWrite);
        assert_eq!(
            set(key, "1"),
            vec![Effect::SetDottedPerAsker {
                asker: asker(),
                key: "my.flag".into(),
                value: "1".into(),
            }]
        );
    }
}

// === アクターを立てて書いて読む段（タスク 1.3・barrier で反映を待つ）===

#[test]
fn reads_in_any_case_return_the_same_value() {
    // 4. 読みの大小違い・括弧の中の名前の大小違い（要件 1.1・1.2）。
    let parts = spawn_empty();
    parts.publisher.publish_static(
        asker(),
        vec![],
        vec![("baseware.name".into(), "areka".into())],
    );
    parts
        .publisher
        .set(asker(), "test(ABC).key".into(), "v".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(read(&parts, "BASEWARE.NAME"), value("areka"));
    assert_eq!(read(&parts, "Baseware.Name"), value("areka"));
    assert_eq!(read(&parts, "test(abc).key"), value("v"));
}

#[test]
fn fullwidth_letters_and_kana_are_different_names() {
    // 5. 畳まない文字（要件 1.3）。
    let parts = spawn_empty();
    parts
        .publisher
        .set(asker(), "ｔｅｓｔ.ｋｅｙ".into(), "w".into());
    parts.publisher.set(asker(), "test.あ".into(), "h".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(read(&parts, "ｔｅｓｔ.ｋｅｙ"), value("w"));
    assert_eq!(read(&parts, "ＴＥＳＴ.ＫＥＹ"), DottedResolution::NotFound);
    assert_eq!(read(&parts, "test.あ"), value("h"));
    assert_eq!(read(&parts, "test.ア"), DottedResolution::NotFound);
}

#[test]
fn surrounding_spaces_are_not_trimmed() {
    // 6. 前後の空白（要件 1.4）。
    let parts = spawn_empty();
    parts.publisher.publish_static(
        asker(),
        vec![],
        vec![("baseware.name".into(), "areka".into())],
    );
    parts.publisher.barrier().expect("barrier");
    assert_eq!(read(&parts, " BASEWARE.NAME"), DottedResolution::NotFound);
    assert_eq!(read(&parts, "BASEWARE.NAME "), DottedResolution::NotFound);
}

#[test]
fn values_keep_their_spelling() {
    // 7. 値の綴りは畳まない（要件 1.5）。
    let parts = spawn_empty();
    parts
        .publisher
        .set(asker(), "test.key".into(), "MiXeD値".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(read(&parts, "TEST.KEY"), value("MiXeD値"));
}

#[test]
fn per_asker_value_wins_in_any_case() {
    // 8. ゴーストごとの値が全体の値に先立つ（要件 1.7）。
    let parts = spawn_empty();
    parts
        .publisher
        .publish_static(asker(), vec![], vec![("test.key".into(), "global".into())]);
    parts
        .publisher
        .set(asker(), "test.key".into(), "mine".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(read(&parts, "TEST.KEY"), value("mine"));
}

#[test]
fn malformed_names_are_not_found_in_any_case() {
    // 9. 書式の誤り: 生の綴りで保存されても、どの綴りの問いでも当たらない（要件 1.6）。
    let parts = spawn_empty();
    parts
        .publisher
        .set(asker(), "INDEX(abc)".into(), "x".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(read(&parts, "INDEX(abc)"), DottedResolution::NotFound);
    assert_eq!(read(&parts, "index(abc)"), DottedResolution::NotFound);
}

#[test]
fn free_name_spellings_share_one_name_last_write_wins() {
    // 10. 自由な名前の書きと読み・大小だけ違う 2 綴りへの続けての書き込み（要件 3.3・3.4）。
    let parts = spawn_empty();
    parts.publisher.set(asker(), "My.Flag".into(), "1".into());
    parts.publisher.set(asker(), "MY.FLAG".into(), "2".into());
    parts.publisher.barrier().expect("barrier");
    for key in ["my.flag", "My.Flag", "MY.FLAG"] {
        assert_eq!(read(&parts, key), value("2"), "{key}");
    }
}

#[test]
fn upper_case_writes_do_not_hide_real_values() {
    // 11. 書けない名前を大文字で書いても本物の値が残る（要件 3.5）。
    let parts = spawn_empty();
    parts.publisher.publish_static(
        asker(),
        vec![],
        vec![("baseware.name".into(), "areka".into())],
    );
    parts
        .publisher
        .set(asker(), "BASEWARE.NAME".into(), "evil".into());
    parts
        .publisher
        .set(asker(), "CURRENTGHOST.NAME".into(), "evil".into());
    parts.publisher.barrier().expect("barrier");
    assert_eq!(read(&parts, "baseware.name"), value("areka"));
    assert_eq!(
        read(&parts, "currentghost.name"),
        DottedResolution::NotFound
    );
}

#[test]
fn persisted_values_read_in_any_case() {
    // 12. 保存された値を大小違いの名前で引く（要件 4.1・4.2）。
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "7".into())],
    );
    let parts = spawn_sylphya(SylphyaInit {
        roots,
        io: Box::new(io),
        runtime_sink: None,
    });
    assert_eq!(read(&parts, "areka.boot.count"), value("7"));
    assert_eq!(read(&parts, "AREKA.BOOT.COUNT"), value("7"));
}
