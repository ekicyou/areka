//! SHIORI の `GetProperty` に英字の大小違いの名前を渡す檻（`areka-P0-property-name-case-fold`
//! 要件 2.2・2.3・2.4・5.4）。
//!
//! テスト用の sylphya（[`super::spawn_test_sylphya_sink`]・偽 IO）に `baseware.name = areka`（静的構成の
//! 点付き＝大域区画）と自由な名前 `path.to.key = some-value`（問い手の区画）を載せ、`barrier` の後に
//! COM ポインタ経由の `get_property` で聞く。受け口は名前を手直しせず読み手へ渡すので、大小の畳みは
//! sylphya の鍵の綴り（`parse_dotted`）だけが担う。前に空白のある ` baseware.name` は空白を削らず
//! 「無い名前」のまま（要件 2.3）。

use super::*;
use shiori_abi::error::ShioriError;
use shiori_abi::interface::IShioriHost;

#[test]
fn get_property_folds_ascii_letter_case_in_names() {
    let t = spawn_test_sylphya_sink();
    t.publisher.publish_static(
        t.asker.clone(),
        vec![],
        vec![("baseware.name".into(), "areka".into())],
    );
    t.publisher
        .set(t.asker.clone(), "path.to.key".into(), "some-value".into());
    t.publisher.barrier().expect("載せた値が反映される");
    let host: IShioriHost = t.sink.into();

    let ask = |name: &str| match host.get_property(&HSTRING::from(name)) {
        Ok(v) => Ok(v.to_string()),
        Err(ShioriError::PropertyNotFound) => Err("PropertyNotFound".to_string()),
        Err(e) => Err(format!("{e:?}")),
    };
    let names = [
        "BASEWARE.NAME",
        "Baseware.Name",
        "PATH.TO.KEY",
        " baseware.name",
        "NO.SUCH.KEY",
    ];
    let answers: Vec<_> = names.iter().map(|n| (*n, ask(n))).collect();

    let not_found = || Err("PropertyNotFound".to_string());
    assert_eq!(
        answers,
        vec![
            ("BASEWARE.NAME", Ok("areka".to_string())),
            ("Baseware.Name", Ok("areka".to_string())),
            ("PATH.TO.KEY", Ok("some-value".to_string())),
            (" baseware.name", not_found()),
            ("NO.SUCH.KEY", not_found()),
        ]
    );
}
