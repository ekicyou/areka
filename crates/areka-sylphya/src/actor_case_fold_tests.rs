//! 名前の畳みのアクターの檻（spec areka-P0-property-name-case-fold タスク 1.2・要件 5.2）。
//!
//! 書き込みの仕分けは半角の英字の大小で変わらない。効果と `warn!` が運ぶ名前は書かれた綴りの
//! まま、自由な名前の書き込みの鍵は畳んだ正準の綴りになる。

use super::*;
use crate::asker::AskerId;
use crate::test_log_capture::{assert_logged, capture};

fn asker() -> AskerId {
    AskerId::new("ghost/test")
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
