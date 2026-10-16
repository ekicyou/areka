# 設計の検証: areka-P0-mcp-stdio-bridge-residue

> 2026-10-16・`/kiro-validate-design`（対話なしで判定まで）。読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）。設計の主張は今のコードと rmcp 3.5.0 の元のファイル（`c:\rust\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\rmcp-3.5.0\`）で引き直した。ビルドとテストは回していない。

## まとめ

設計は今ある形の延長だけで組まれていて、触る所・触らない所の線（`config.rs` に触らない・`package.ps1` は PE を読む所と項目 5 だけ・新しいクレートなし・`service_tests.rs` へ足さない・exe を起こすテストは `tests/exe_test.rs` で番号と `target\` の下の置き場を渡す・件 7・8 は差の記録に限界として書くだけ・件 9 は外）はどれも守られている。引いた行番号・関数の名前・テストの名前・rmcp の行は、ほぼすべて今のコードと合っていた。直すべき所は 2 つで、どちらも確かめの手順とテストの判定の書き足しで済み、形を作り直す必要は無い。

## 引き直した事実（合っていたもの）

- `crates/areka-mcp-bridge/src/main.rs` 1 行目が `windows_subsystem` の行・`EXIT_WAIT`＝6 秒・`serve` が `run` と「閉じてから 6 秒」を `select` で競わせる。
- `scan.rs` 98〜105 行目の `open` と、`http.rs` 125〜130 行目の `OwnHttp::post` の頭が同じ形。`scan_tests.rs` は `open` を直接呼ばない（消しても今のテストは変わらない）。
- `http.rs` の `classify`（269 行目〜）は 202 を `StreamableHttpPostResponse::Accepted` にする。
- `tests/common/fake_server.rs` の末尾 519〜521 行目に `mod tests`。`fake_server_tests.rs` が使う偽の受け口の項目は公開のもの（`FakeServer`・`Script`・`Reply` とその公開の関数）と `MCP_PATH`・`SERVER_NAME` だけで、非公開の定数や関数・非公開の欄は使っていない（`use crate::fake_server::*;` へ替えても通る見込み）。`tests/exe_test.rs` 側の `mod fake_server` には `#[allow(dead_code)]` が付いているので、自己確認を外しても警告は増えない。
- `service_failure_tests.rs`（229 行）の `failed`・`patient`・`a_request_not_answered_within_the_bound_gets_the_timeout_error_once`、`service_tests.rs` の `Desk::start_with`・`ask`・`close`・`handshakes`・`result`・`WAIT`（どれも `pub(super)`）、`exe_test.rs`（568 行）の `run_bridge`・`call`・`called`（`content[0].text` が `done`）・`answer_lines`・`answer_to`・`Finished::waited`・`TempPath::under_target("bridge-exe")`。
- `crates/areka-mcp/src/port.rs` 46・67 行目の `warn!` の文、`port_tests.rs` 82 行目のコメント・97・113 行目の期待。リポジトリの中でこの文を読む所はほかに無い（完了した spec の文書だけ）。
- `tools/package.ps1`: `Read-PeInfo`（351 行目）・`$opt = $pe + 24`・項目 5 の繰り返し（517〜528 行目）・`5 cannot read as PE: ...`・`content checks 1-8 all passed`（591 行目）・`SCRIPT_VERSION = '2.1.0'`。Subsystem はオプションヘッダーの先頭から 68 バイト目で、PE32 と PE32+ で同じ位置（設計どおり）。
- `doc/ssp-mcp/bridge-diff-areka.md`: 9 行目「22 行（4 節）」は 4 節の表の行の数と一致。42・90・95・96・102 行目の中身も設計の言うとおり。件 8 の行に書く「受け口が使っていない接続を 30 秒で閉じる」は、`areka-mcp` の `server.rs` が hyper の `http1::Builder` に `TokioTimer` を付けている（頭を読む上限の既定 30 秒が効く）ことと合う。
- rmcp 3.5.0: `transport/streamable_http_client.rs` 1628〜1638 行目で `Accepted` を受けた `id` は待ちの表から外されない（1437 行目で取り消しのときに外す）。`service.rs` 1762 行目〜で `QuitReason::Closed` のとき途中の応答を 5 秒まで待って書く。

## 直すべき所（2 件）

🔴 **直すべき所 1**: 件 4 の変異（確かめ B）が狙いの枝より広く壊し、赤の理由の見立ても外れている
**中身**: 設計は「`http.rs` の `classify` の 202 の枝を `Err(HttpError::NoResponse)` に替えると、件 4 のテストが `No response from server: empty body or connection closed` で赤」と書く。ところが `classify` は要求と通知を区別しないので、この変異は握手の `notifications/initialized`（偽の受け口は `id` の無いものに既定で 202 を返す＝`fake_server.rs` 376〜377 行目）も壊す。rmcp のクライアントが立たず、探索は「見つからない」になり、件 4 のテストは別の文で赤になる（ほかの中継のテストもまとめて赤になる）。
**影響**: 要件 8.2・8.4 の記録に「赤になった判定」を書くとき、見立てと違う結果になり、実装係が迷う。また、新しいテストが「`id` を持つ要求の 202」という枝そのものを守っていることを示せない。
**直し方**: 変異を `post_message` の側へ寄せる。たとえば「要求（`ClientJsonRpcMessage::Request`）で、`post` の答えが `Accepted` なら `Err(HttpError::NoResponse)` を返す」に替える。通知の 202 は今のまま通るので、握手は立ち、件 4 のテストだけが `No response from server: empty body or connection closed` で赤になる（書いた後の失敗として 1 回答える）。設計の「確かめの手順」の確かめ B の件 4 の行をこの形に直す。
**つながる要件**: 4.1・8.2・8.4
**設計の場所**: 「確かめの手順」確かめ B（件 4）

🔴 **直すべき所 2**: 件 5.1 のテストが「rmcp の 5 秒で終わった」のか「中継の 6 秒の見切りで終わった」のかを見分けない
**中身**: 5.1 の判定は「終了コード 0・標準出力が空・閉じてから 5 秒以上 12 秒未満」。中継には別に「閉じてから 6 秒で見切る」枝（`main.rs` の `select` の右）があるので、rmcp を上げて途中の応答を 5 秒で切らなくなっても（もっと長く待つ・最後まで待つ）、6 秒の見切りで同じ結果になり、5.1 は緑のままになる。ところが設計は Revalidation Triggers と差の記録の「残る縛り」の書き直しで、「件 5 のテストが rmcp の 5 秒を固定しているので、版を上げて変われば赤になる」と書く。この主張は 5 秒より短くなる向きにしか当たらない。
**影響**: 差の記録に事実と違う約束が載る。rmcp を上げたときに、中継が 6 秒の見切り（警告の記録つき）で終わるようになっても気付けない。
**直し方**: 5.1 と 5.2 の判定に、中継が自分で終わった枝を通ったことを足す: 標準エラーに `[bridge] finished` の行があり、`[bridge] stdout is still not read` の行が無い（今ある `record` の道具で書ける・決定論のまま・時間の幅は今のまま）。これで中継の判断の枝（`select` のどちらで終わったか）を固定することになり、rmcp の 5 秒が長くなる向きでも赤になる。確かめ B の `EXIT_WAIT` を 0 にする変異は、このままでも赤になる。
**つながる要件**: 5.1・5.3・7.4
**設計の場所**: 「件 5 のテスト」の 5.1・5.2 の判定、Revalidation Triggers の rmcp の行、「差の記録」の 102 行目の書き直し

## 細かい所（議題にしなくてよい・実装のときに直せばよい）

- `scan.rs` から `open` を消すと、`io`・`Ipv4Addr`・`TcpStream` に加えて `std::time::Duration` も使わなくなる（設計の並びに `Duration` が無い。「残らないもの」の言い方で拾える）。
- 「This Spec Owns」は `port_tests.rs` の「期待 3 か所」と書くが、「File Structure Plan」は「82 行目のコメントと 97・113 行目の期待」（期待は 2 か所で、それぞれの判定が 2 つに分かれる）。中身は同じ。
- `Reply::Delayed` は `Box<Reply>` を取るので、台本は `Reply::Delayed(Duration::from_secs(12), Box::new(called()))` の形になる。

## よい所

- **線の引き方がはっきりしている**: 触るファイルと触らないファイルを、理由（並走の spec・約束・要件）つきで列挙し、`config.rs` を通らないために記録の文を共有の側で中立の言い方へ直す（案 (a)）など、約束の線の中で筋の通る形を選んでいる。どの部品も互いに独立で、順も並べ方も自由に実装できる。
- **事実で裏打ちされている**: rmcp 3.5.0 の決まり（202 の待ち・閉じた後の 5 秒）を元のファイルの行で押さえ、変異の当たらない所（5.1 の「30 秒を待たない」側）は「当てられる変異が無い」と理由を記録する道まで決めてある。配布の検査も、今の項目 5 の否の文の形と読めないときの赤の経路にそのまま乗せていて、新しい終了コードや総括の行の変化を生まない。

## 判定

**GO**

**理由**: 約束の線（`config.rs`・`package.ps1` の範囲・新しいクレートなし・`service_tests.rs` へ足さない・exe のテストの番号と置き場・件 7・8・9 の扱い）はすべて守られ、形を作り直す必要のある食い違いは無い。直すべき 2 件は、確かめ B の変異の場所と件 5 のテストの判定の書き足しで、設計の討議で直せる。

**次の段**:
1. `/kiro-design-discussion areka-P0-mcp-stdio-bridge-residue` で上の 2 件を議題にし、設計の確かめ B（件 4）と件 5 のテストの判定・差の記録の書き方を直す。
2. その後 `/kiro-spec-tasks areka-P0-mcp-stdio-bridge-residue` でタスクを作る。
