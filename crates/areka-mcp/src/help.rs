//! 登録案内の HTML（日本語・5 項目）。

use crate::port::{DEFAULT_PORT, PORT_ENV};

/// 登録手順の日本語 HTML を実際の番号 `port` で組む（要件 6.1・6.2）。
///
/// 5 項目: ⑴ 待ち受けている URL、⑵ Claude Code の登録コマンド、⑶ Cursor の
/// `mcpServers` の断片、⑷ `AREKA_MCP_PORT` の説明（`0` で待ち受けない・既定の番号）、
/// ⑸ Claude Desktop は中継が要る（設定例は後続 `mcp-stdio-bridge` が足す）。
/// URL とコマンド例の番号は引数から取り、既定の番号は [`DEFAULT_PORT`] から取る。
/// 埋め込む値は数と定数だけなので HTML のエスケープは要らない。
pub fn help_html(port: u16) -> String {
    let url = format!("http://127.0.0.1:{port}/api/mcp/v1");
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

<h2>ポートの変え方</h2>
<p>環境変数 <code>{PORT_ENV}</code> にポート番号（1〜65535）を入れて areka を起動します。
<code>{PORT_ENV}=0</code> で待ち受けない設定になります。未設定なら既定は <code>{DEFAULT_PORT}</code> です。</p>

<h2>Claude Desktop</h2>
<p>Claude Desktop は HTTP の MCP サーバを設定に直接書けないため、stdio との中継が要ります。中継は今後の版で用意します。</p>
</body>
</html>
"#
    )
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod help_tests;
