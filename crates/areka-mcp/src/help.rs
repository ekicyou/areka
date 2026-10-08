//! 登録案内の HTML（日本語・5 項目）。

use crate::port::{DEFAULT_PORTS, FALLBACK_STEPS, PORT_ENV};
use crate::tools::{OWN_TABLE, spec_from_definition};

/// 登録手順の日本語 HTML を実際の番号 `port` で組む（要件 6.1・6.2）。
///
/// 5 項目: ⑴ 待ち受けている URL、⑵ Claude Code の登録コマンド、⑶ Cursor の
/// `mcpServers` の断片、⑷ ポートの決まり方と変え方（既定の順・早い者勝ち・隣への退避・
/// `AREKA_MCP_PORT` で 1 つを指定・`0` で待ち受けない）、⑸ Claude Desktop は中継が要る
/// （設定例は後続 `mcp-stdio-bridge` が足す）。
/// URL とコマンド例の番号は引数から取り、既定の順の説明は [`DEFAULT_PORTS`]・[`FALLBACK_STEPS`]
/// から組む（数を手で書かない）。
/// 続けて「areka 独自のツール」の節に [`OWN_TABLE`] の各行を `<code>名前</code> — 1 行` で並べる
/// （名前は登録と同じ定義から読む・spec: areka-P0-mcp-author-tools）。
/// 埋め込む値は数と定数（名前と 1 行は `<`・`>`・`&` を含まない）なので HTML のエスケープは要らない。
pub fn help_html(port: u16) -> String {
    // 読めない定義は登録の側（`register_row`）が `error!` で記録して登録しないので、ここでも載せない。
    let own_tools: String = OWN_TABLE
        .iter()
        .filter_map(|&(definition, _, summary)| {
            let name = spec_from_definition(definition).ok()?.name;
            Some(format!("<li><code>{name}</code> — {summary}</li>\n"))
        })
        .collect();
    let url = format!("http://127.0.0.1:{port}/api/mcp/v1");
    let [first, second] = DEFAULT_PORTS;
    let (first_next, second_next) = (first + 1, second + 1);
    let (first_last, second_last) = (first + FALLBACK_STEPS, second + FALLBACK_STEPS);
    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<title>areka MCP サーバの登録</title>
</head>
<body>
<h1>areka MCP サーバの登録</h1>

<h2>接続先の URL</h2>
<p>areka はいま次の URL で MCP を待ち受けています。</p>
<pre><code>{url}</code></pre>

<h2>Claude Code</h2>
<p>次のコマンドで登録します。</p>
<pre><code>claude mcp add --transport http areka {url}</code></pre>

<h2>Cursor</h2>
<p>設定ファイル（<code>mcp.json</code>）の <code>mcpServers</code> に次を加えます。</p>
<pre><code>{{
  "mcpServers": {{
    "areka": {{
      "url": "{url}"
    }}
  }}
}}</code></pre>

<h2>ポートの決まり方と変え方</h2>
<p>既定では <code>{first}</code> → <code>{second}</code> の順に試し、先に空いていた 1 つで待ち受けます（同じ番号を使う SSP などとは早い者勝ちです）。
どちらも使用中なら隣の番号（<code>{first_next}</code>・<code>{second_next}</code> … <code>{first_last}</code>・<code>{second_last}</code>）へ順に移ります。
実際に待ち受けている番号は上の URL のとおりです。</p>
<p>環境変数 <code>{PORT_ENV}</code> にポート番号（1〜65535）を入れて areka を起動すると、その番号だけで待ち受けます（使用中なら待ち受けません）。
<code>{PORT_ENV}=0</code> で待ち受けない設定になります。</p>

<h2>Claude Desktop</h2>
<p>Claude Desktop は HTTP の MCP サーバを設定に直接書けないため、stdio との中継が要ります。中継は今後の版で用意します。</p>

<h2>areka 独自のツール</h2>
<p>SSP と同じツールに加えて、SSP に無い次のツールを出しています。</p>
<ul>
{own_tools}</ul>
</body>
</html>
"#
    )
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod help_tests;
