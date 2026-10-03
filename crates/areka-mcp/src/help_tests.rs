//! `help` の決定論テスト（要件 6.1・6.2 の 5 項目と番号）。

use super::help_html;
use crate::port::{DEFAULT_PORT, PORT_ENV};

/// (要件 6.1, 6.2, 2.7) 既定でない番号で組むと 5 項目が実番号で載り、
/// `127.0.0.1:9821` は無く、`9821` は ⑷ の既定の説明に 1 回だけ現れる。
#[test]
fn help_html_lists_five_items_with_actual_port() {
    let html = help_html(12345);

    // UTF-8 の日本語ページ
    assert!(html.contains(r#"<meta charset="utf-8">"#), "{html}");
    assert!(html.contains(r#"<html lang="ja">"#), "{html}");

    // ⑴ 待ち受けている URL（⑵⑶ にも同じ URL が入るので、⑴ だけの形で確かめる）
    assert!(
        html.contains("<pre><code>http://127.0.0.1:12345/api/mcp/v1</code></pre>"),
        "{html}"
    );
    // URL は ⑴⑵⑶ の 3 か所
    assert_eq!(
        html.matches("http://127.0.0.1:12345/api/mcp/v1").count(),
        3,
        "{html}"
    );
    // ⑵ Claude Code の登録コマンド
    assert!(
        html.contains("claude mcp add --transport http areka http://127.0.0.1:12345/api/mcp/v1"),
        "{html}"
    );
    // ⑶ Cursor の mcpServers の断片
    assert!(html.contains(r#""mcpServers""#), "{html}");
    assert!(
        html.contains(r#""url": "http://127.0.0.1:12345/api/mcp/v1""#),
        "{html}"
    );
    // ⑷ ポートの変え方（名前・0 で待ち受けない・既定）
    assert!(html.contains(PORT_ENV), "{html}");
    assert!(html.contains("0</code> で待ち受けない"), "{html}");
    assert!(
        html.contains(&format!("既定は <code>{DEFAULT_PORT}</code>")),
        "{html}"
    );
    // ⑸ Claude Desktop は中継が要る（設定例なし）
    assert!(html.contains("Claude Desktop"), "{html}");
    assert!(html.contains("中継"), "{html}");
    assert!(!html.contains("claude_desktop_config"), "{html}");

    // 番号は引数から: 固定の 9821 は URL 側に無く、既定の説明の 1 回だけ
    assert!(!html.contains("127.0.0.1:9821"), "{html}");
    assert_eq!(html.matches("9821").count(), 1, "{html}");
}
