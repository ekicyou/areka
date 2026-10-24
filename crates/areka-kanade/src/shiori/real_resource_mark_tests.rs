//! `real.rs` の兄弟のテスト: 接続の 4 つの手が版を client へ渡し、資源の印は GET の 2 つの手の
//! 中だけ・資源の一覧の判定の後ろにだけあることを構造で固定する（shiori2-protocol 要件 2.2・3.3・3.7）。
//!
//! 接続（[`super::ShioriConnection`]）は窓と中継が無いと組めないので、往復を回して確かめる代わりに
//! 実装の本文を読む（先例: host32 の `the_one_way_event_body_contains_no_adoption_path`）。

/// 改行の綴り（CRLF／LF）に依存しないよう CR を落とした `real.rs` の本文。
fn source() -> String {
    include_str!("real.rs").replace('\r', "")
}

/// `impl ShioriBackend for ShioriConnection` の中の手 `fn {name}(` の本体を切り出す
/// （trait の宣言にも同じ名前があるので、先に impl の塊へ絞る）。
fn body_of(source: &str, name: &str) -> String {
    let imp = source
        .split_once("impl ShioriBackend for ShioriConnection {")
        .expect("接続の impl の塊が見つからない")
        .1
        .split_once("\n}\n")
        .expect("接続の impl の閉じ括弧が見つからない")
        .0;
    imp.split_once(&format!("    fn {name}("))
        .unwrap_or_else(|| panic!("手 `{name}` が見つからない"))
        .1
        .split_once("\n    }\n")
        .unwrap_or_else(|| panic!("手 `{name}` の閉じ括弧が見つからない"))
        .0
        .to_string()
}

#[test]
fn the_resource_mark_is_only_in_the_two_get_hands_after_the_list_check() {
    let source = source();
    assert_eq!(
        source.matches("for_resource(").count(),
        2,
        "資源の印を付ける呼び出しは GET の 2 つの手に 1 つずつだけのはず（要件 3.3・3.7）"
    );
    for name in ["get", "get_with_headers"] {
        let body = body_of(&source, name);
        let check = body
            .find("is_allowed_resource_id(")
            .unwrap_or_else(|| panic!("`{name}` に資源の一覧の判定が無い"));
        let mark = body
            .find("for_resource(")
            .unwrap_or_else(|| panic!("`{name}` に資源の印が無い"));
        assert!(
            check < mark,
            "`{name}` の資源の印は資源の一覧の判定の後ろにあるはず"
        );
        assert!(
            body.contains("with_version(self.version)"),
            "`{name}` は接続の版を client へ渡すはず（要件 2.2）"
        );
    }
    for name in ["notify", "notify_with_headers"] {
        let body = body_of(&source, name);
        assert!(
            !body.contains("for_resource"),
            "NOTIFY の手 `{name}` に資源の印がある（NOTIFY には付けない）"
        );
        assert!(
            body.contains("with_version(self.version)"),
            "`{name}` は接続の版を client へ渡すはず（要件 2.2）"
        );
    }
}
