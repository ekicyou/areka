//! アンカーの 2 イベント（`OnAnchorSelectEx`・`OnAnchorSelect`）の組み立てと、送ってよいイベントの
//! 表の 2 行のテスト（areka-P0-anchor-tag-canon 要件 4.1・4.2・4.6・4.7）。
//!
//! どちらも GET で、Reference の個数と値を列ごと突き合わせる。

use super::*;

/// GET を分解して (id の綴り, references, Status の wire 値) を取り出す（NOTIFY なら panic）。
fn expect_get(call: ShioriCall) -> (String, Vec<String>, Option<String>) {
    match call {
        ShioriCall::Get {
            id,
            references,
            status,
        } => (id.as_str().to_string(), references, status.render()),
        ShioriCall::Notify { .. } => panic!("GET のはずが NOTIFY"),
    }
}

/// 加工されたら壊れる引数の並び（前後の空白・空の要素・カンマ・バックスラッシュ）。
fn arguments() -> Vec<String> {
    vec![
        " 二番目  ".to_string(),
        String::new(),
        "a,b".to_string(),
        "\\_a[x]".to_string(),
    ]
}

#[test]
fn on_anchor_select_ex_is_get_with_text_id_then_arguments_in_written_order() {
    let (id, refs, _) = expect_get(on_anchor_select_ex(
        "くわしく",
        "Anchor詳細",
        &arguments(),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(id, "OnAnchorSelectEx");
    assert_eq!(
        refs,
        vec![
            "くわしく".to_string(),   // Reference0＝範囲の文字（要件 4.1）
            "Anchor詳細".to_string(), // Reference1＝ID（要件 4.1）
            " 二番目  ".to_string(),  // Reference2 以降＝引数を記述順のまま
            String::new(),
            "a,b".to_string(),
            "\\_a[x]".to_string(),
        ]
    );
}

#[test]
fn on_anchor_select_ex_without_arguments_stops_at_reference1() {
    let (id, refs, _) = expect_get(on_anchor_select_ex(
        "くわしく",
        "Anchor詳細",
        &[],
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(id, "OnAnchorSelectEx");
    assert_eq!(
        refs,
        vec!["くわしく".to_string(), "Anchor詳細".to_string()],
        "引数が無ければ Reference2 以降の位置を作らない（空文字で埋めない・要件 4.6）"
    );
}

#[test]
fn on_anchor_select_is_get_with_id_only() {
    let (id, refs, _) = expect_get(on_anchor_select("Anchor詳細", &ExecutionSnapshot::INACTIVE));
    assert_eq!(id, "OnAnchorSelect");
    assert_eq!(
        refs,
        vec!["Anchor詳細".to_string()],
        "Reference0＝ID の 1 個だけ（要件 4.2）"
    );
}

/// 実行状態の行は、他の組み立てと同じく渡された状態から導く（要件 4.7）。
#[test]
fn anchor_builders_derive_status_from_the_snapshot() {
    let talking = ExecutionSnapshot {
        talk_active: true,
        ..ExecutionSnapshot::INACTIVE
    };
    for call in [
        on_anchor_select_ex("くわしく", "ID", &arguments(), &talking),
        on_anchor_select("ID", &talking),
    ] {
        let (id, _, status) = expect_get(call);
        assert_eq!(status, Some("talking".to_string()), "{id} の Status");
    }
    for call in [
        on_anchor_select_ex("くわしく", "ID", &arguments(), &ExecutionSnapshot::INACTIVE),
        on_anchor_select("ID", &ExecutionSnapshot::INACTIVE),
    ] {
        let (id, _, status) = expect_get(call);
        assert_eq!(status, None, "{id} は話していなければ行を出さない");
    }
}

/// 表に正典の 2 語が載り、組み立ての綴りと一致する。
#[test]
fn the_two_anchor_events_are_in_the_allowed_table() {
    for id in ["OnAnchorSelectEx", "OnAnchorSelect"] {
        assert_eq!(allowed_static(id), Some(id), "{id} が表に無い");
    }
    let snap = ExecutionSnapshot::INACTIVE;
    for call in [
        on_anchor_select_ex("くわしく", "ID", &[], &snap),
        on_anchor_select("ID", &snap),
    ] {
        let ShioriCall::Get { id, .. } = call else {
            panic!("GET のはずが NOTIFY");
        };
        assert!(
            matches!(id, EventId::Static(name) if is_allowed_event_id(name)),
            "決まった名前のイベントとして表の綴りで送る: {}",
            id.as_str()
        );
    }
}
