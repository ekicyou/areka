//! `help` の決定論テスト（要件 6.1・6.2 の 5 項目と番号）。

use super::help_html;
use crate::port::{DEFAULT_PORTS, FALLBACK_STEPS, PORT_ENV};

/// (要件 6.1, 6.2, 2.7) 既定でない番号で組むと 5 項目が実番号で載り、
/// 既定の番号（9801・9821）の URL は無く、⑷ は既定の順（先頭 2 つ → 隣 … 末尾 2 つ）を説明する。
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
    // ⑷ ポートの決まり方と変え方（既定の順・早い者勝ち・名前・0 で待ち受けない）
    let [first, second] = DEFAULT_PORTS;
    assert!(
        html.contains(&format!("<code>{first}</code> → <code>{second}</code>")),
        "既定の先頭 2 つの順: {html}"
    );
    assert!(html.contains("早い者勝ち"), "{html}");
    assert!(
        html.contains(&format!(
            "<code>{}</code>・<code>{}</code> … <code>{}</code>・<code>{}</code>",
            first + 1,
            second + 1,
            first + FALLBACK_STEPS,
            second + FALLBACK_STEPS
        )),
        "隣の候補（先頭と末尾）: {html}"
    );
    assert!(html.contains(PORT_ENV), "{html}");
    assert!(html.contains("その番号だけで待ち受けます"), "{html}");
    assert!(html.contains("0</code> で待ち受けない"), "{html}");
    // ⑸ Claude Desktop は中継が要る（設定例なし）
    assert!(html.contains("Claude Desktop"), "{html}");
    assert!(html.contains("中継"), "{html}");
    assert!(!html.contains("claude_desktop_config"), "{html}");

    // 番号は引数から: 既定の番号の URL は無い（URL・コマンド・断片は実番号だけ）
    for port in DEFAULT_PORTS {
        assert!(!html.contains(&format!("127.0.0.1:{port}")), "{html}");
    }
}
