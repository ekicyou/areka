//! `real.rs` の兄弟のテスト: 接続の 4 つの手が共通の client の組み立てを呼び、その組み立てが版を
//! client へ渡し、資源の印は GET の 2 つの手から呼んだときだけ・資源の一覧の判定の後ろにだけ付く
//! ことを構造で固定する（shiori2-protocol 要件 2.2・3.3・3.7）。
//!
//! 接続（[`super::ShioriConnection`]）は窓と中継が無いと組めないので、往復を回して確かめる代わりに
//! 実装の本文を読む（先例: host32 の `the_one_way_event_body_contains_no_adoption_path`）。

/// 改行の綴り（CRLF／LF）に依存しないよう CR を落とした `real.rs` の本文。
fn source() -> String {
    include_str!("real.rs").replace('\r', "")
}

/// 塊 `{imp}`（例: `impl ShioriConnection {`）の中の手 `fn {name}(` の本体を切り出す
/// （trait の宣言にも同じ名前があるので、先に impl の塊へ絞る）。
fn body_of(source: &str, imp: &str, name: &str) -> String {
    let imp = source
        .split_once(imp)
        .unwrap_or_else(|| panic!("塊 `{imp}` が見つからない"))
        .1
        .split_once("\n}\n")
        .expect("impl の閉じ括弧が見つからない")
        .0;
    imp.split_once(&format!("    fn {name}("))
        .unwrap_or_else(|| panic!("手 `{name}` が見つからない"))
        .1
        // 塊の最後の手は塊の閉じ括弧の手前で切れるので、後ろの改行は求めない。
        .split_once("\n    }")
        .unwrap_or_else(|| panic!("手 `{name}` の閉じ括弧が見つからない"))
        .0
        .to_string()
}

const HANDS: &str = "impl ShioriBackend for ShioriConnection {";

#[test]
fn the_four_hands_build_the_client_in_one_place_with_version_and_resource_mark() {
    let source = source();
    // 版と印は組み立ての 1 か所だけにある（手ごとに書き写すと片方だけ直して食い違う）。
    assert_eq!(
        source.matches("with_version(self.version)").count(),
        1,
        "接続の版を渡すのは client の組み立て 1 か所だけのはず（要件 2.2）"
    );
    assert_eq!(
        source.matches("for_resource(").count(),
        1,
        "資源の印を付けるのは client の組み立て 1 か所だけのはず（要件 3.3・3.7）"
    );
    let client = body_of(&source, "impl ShioriConnection {", "client");
    assert!(
        client.contains("with_version(self.version)"),
        "client の組み立ては接続の版を client へ渡すはず（要件 2.2）"
    );
    let check = client
        .find("resource && is_allowed_resource_id(id)")
        .expect("client の組み立てに「GET の手から・資源の一覧にある」の判定が無い");
    let mark = client
        .find("for_resource(")
        .expect("client の組み立てに資源の印が無い");
    assert!(check < mark, "資源の印は資源の一覧の判定の後ろにあるはず");
    for (name, call) in [
        ("get", "self.client(id, true)"),
        ("get_with_headers", "self.client(id, true)"),
        ("notify", "self.client(id, false)"),
        ("notify_with_headers", "self.client(id, false)"),
    ] {
        let body = body_of(&source, HANDS, name);
        assert!(
            body.contains(call),
            "手 `{name}` は `{call}` で client を組むはず（GET は印あり・NOTIFY は印なし）"
        );
        assert!(
            !body.contains("Shiori3Client::new"),
            "手 `{name}` が client を自前で組んでいる（組み立ては 1 か所に）"
        );
    }
}
