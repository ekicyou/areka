//! `help` の決定論テスト（要件 6.1・6.2 の 5 項目と番号・Claude Desktop の節の設定例）。

use std::path::Path;

use serde_json::{Value, json};

use super::{BRIDGE_EXE_FILE_NAME, help_html};
use crate::port::{DEFAULT_PORTS, FALLBACK_STEPS, PORT_ENV, candidates_from_env_value};
use crate::tools::{OWN_TABLE, spec_from_definition};

/// (要件 6.1, 6.2, 2.7) 既定でない番号で組むと 5 項目が実番号で載り、
/// 既定の番号（9801・9821）の URL は無く、⑷ は既定の順（先頭 2 つ → 隣 … 末尾 2 つ）を説明する。
#[test]
fn help_html_lists_five_items_with_actual_port() {
    let html = help_html(12345, None);

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
    // ⑸ Claude Desktop は中継が要る（設定例は下の Desktop の節のテストで確かめる）
    assert!(html.contains("Claude Desktop"), "{html}");
    assert!(html.contains("中継"), "{html}");

    // 番号は引数から: 既定の番号の URL は無い（URL・コマンド・断片は実番号だけ）
    for port in DEFAULT_PORTS {
        assert!(!html.contains(&format!("127.0.0.1:{port}")), "{html}");
    }
}

/// (要件 4.2, 4.3) 「areka 独自のツール」の節があり、独自のツールの表の各行の名前（登録する定義から読む）と
/// 日本語の 1 行が `<code>名前</code> — 1 行` の形で節の中に出る（spec: areka-P0-mcp-author-tools）。
#[test]
fn help_html_lists_own_tools_from_table() {
    let html = help_html(12345, None);

    let section = html
        .split_once("<h2>areka 独自のツール</h2>")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("節が無い: {html}"));
    assert!(!OWN_TABLE.is_empty());
    for (definition, _, summary) in OWN_TABLE {
        let name = spec_from_definition(definition).expect("定義が読める").name;
        assert!(
            section.contains(&format!("<li><code>{name}</code> — {summary}</li>")),
            "{name}: {html}"
        );
    }
    assert!(section.contains("<code>check_script</code>"), "{html}");
}

/// Claude Desktop の節（見出しの次から、次の見出しの手前まで）。
fn desktop_section(html: &str) -> &str {
    html.split_once("<h2>Claude Desktop</h2>")
        .and_then(|(_, rest)| rest.split_once("<h2>"))
        .map(|(section, _)| section)
        .unwrap_or_else(|| panic!("節が無い: {html}"))
}

/// 節の設定例（`<pre><code>…</code></pre>` の中の、頁に書かれたままの文字）。
fn desktop_example_source(html: &str) -> &str {
    desktop_section(html)
        .split_once("<pre><code>")
        .and_then(|(_, rest)| rest.split_once("</code></pre>"))
        .map(|(code, _)| code)
        .unwrap_or_else(|| panic!("設定例が無い: {html}"))
}

/// 設定例を、ブラウザが表示する文字（HTML の置き換えを戻したもの）にして JSON として読む。
/// 受け口ごしの help のテスト（`server_gate_help_tests.rs`）も使う。
pub(crate) fn desktop_example(html: &str) -> Value {
    let shown = desktop_example_source(html)
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    serde_json::from_str(&shown).unwrap_or_else(|err| panic!("JSON でない（{err}）: {shown}"))
}

/// `\`・空白・`&` 入りの置き場（要件 6.2）。
const BRIDGE_PATH: &str = r"C:\Program Files\R&D tools\areka-mcp-bridge.exe";

/// (要件 6.1, 6.2, 6.4) 既定の 20 候補の外の番号では、設定例が JSON として読めて、`command` が渡した
/// パスと同じ・`env` に `AREKA_MCP_PORT` がその番号で載る。頁の上では `&` が置き換えてある。
#[test]
fn desktop_example_carries_the_path_and_the_port_outside_the_defaults() {
    let html = help_html(12345, Some(Path::new(BRIDGE_PATH)));

    assert_eq!(
        desktop_example(&html),
        json!({
            "mcpServers": {
                "areka": {
                    "command": BRIDGE_PATH,
                    "env": { PORT_ENV: "12345" }
                }
            }
        }),
        "{html}"
    );
    let source = desktop_example_source(&html);
    assert!(source.contains("R&amp;D tools"), "{html}");
    assert!(!source.contains("R&D"), "{html}");
}

/// (要件 6.3) 既定の 20 候補のどれで待ち受けていても、設定例に `env` を載せない。
#[test]
fn desktop_example_has_no_env_on_every_default_candidate() {
    let defaults = candidates_from_env_value(None);
    assert_eq!(defaults.len(), 20);
    for port in defaults {
        let html = help_html(port, Some(Path::new(BRIDGE_PATH)));
        assert_eq!(
            desktop_example(&html),
            json!({ "mcpServers": { "areka": { "command": BRIDGE_PATH } } }),
            "{port}: {html}"
        );
    }
}

/// (要件 6.5) 節に、設定ファイルの名前・Desktop の設定の画面から開くこと・書いた後に起動し直すこと
/// （窓を閉じるだけでは終了しない旨の断り付き）・Desktop を areka より先に起動していてもツールが並ぶこと
/// （控えか、控えが無ければ中継が自分で持っている一覧で。「一度つないだ後は」の条件は付かない）・
/// ツールを使うには areka の起動が要り、起動すれば Desktop を起動し直さなくてよいことが載り、
/// 「今後の版」の文は無い。置き場を渡したときは置き換えの文も無い。
#[test]
fn desktop_section_tells_where_to_write_and_to_restart() {
    let html = help_html(12345, Some(Path::new(BRIDGE_PATH)));
    let section = desktop_section(&html);

    assert!(
        section.contains("<code>claude_desktop_config.json</code>"),
        "{html}"
    );
    assert!(section.contains("設定の画面から開"), "{html}");
    assert!(section.contains("起動し直"), "{html}");
    assert!(
        section.contains("窓を閉じるだけでは Claude Desktop は終了しません"),
        "{html}"
    );
    assert!(section.contains("控え"), "{html}");
    assert!(!html.contains("一度つないだ後は"), "{html}");
    assert!(
        section.contains("areka より先に起動していても、areka のツールが並びます"),
        "{html}"
    );
    assert!(section.contains("自分で持っている一覧"), "{html}");
    assert!(
        section.contains("ツールを使うには areka を起動してください"),
        "{html}"
    );
    assert!(
        section.contains("Claude Desktop を起動し直さなくても使えます"),
        "{html}"
    );
    assert!(!html.contains("今後の版"), "{html}");
    assert!(!section.contains("置き換えてください"), "{html}");
}

/// (要件 6.1) 置き場が取れないときは、設定例の `command` をファイル名だけにして、
/// 絶対パスに置き換える旨の 1 文を添える。
#[test]
fn desktop_example_without_a_path_shows_the_file_name_and_asks_to_replace_it() {
    let html = help_html(12345, None);

    assert_eq!(
        desktop_example(&html)["mcpServers"]["areka"]["command"],
        json!(BRIDGE_EXE_FILE_NAME),
        "{html}"
    );
    assert!(
        desktop_section(&html)
            .contains("areka.exe と同じフォルダにある中継の絶対パスに置き換えてください"),
        "{html}"
    );
}
