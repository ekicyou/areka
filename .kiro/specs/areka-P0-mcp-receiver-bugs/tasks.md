# Implementation Plan

- [x] 1. 受け口が形の違う `params` に `-32602` で答える（件 1）
- [x] 1.1 型のない要求への答えを決める純粋な関数と、rmcp の拡張の口の上書きを足す
  - `params` が欠かせない 3 つのメソッド（`tools/call`・`initialize`・`completion/complete`）の表を rmcp の名前の定数で持ち、表の中なら `-32602`（文は `invalid params for <メソッドの名前>`・`data` なし）と `debug` の記録 1 件（欄 `method`・文 `MCP: params の形がメソッドに合わない`）、表の外なら rmcp の既定と同じ `-32601`（文はメソッドの名前・`data` なし・記録なし）を返す
  - 上書きは常にその関数の答えを `Err` で返すだけにし、`params` の中身は読まず記録にも載せない。HTTP の状態の決め方・`dispatch.rs`・中継のコードは変えない
  - 頭の説明に「形の合わない 3 つは `-32602`、ほかの型のない要求は `-32601`」の 1 文を足し、純粋な関数のテストを兄弟の新しいテストのファイルへ接続する
  - 兄弟のテストで、表の 3 つそれぞれが `{-32602, "invalid params for <m>", data なし}` と `DEBUG` ちょうど 1 件（欄 `method` が `m`）、表の外（`no/such`・`ping`・`tools/list`・`prompts/get`）が `{-32601, <名前>, data なし}` と記録 0 件になることを、テストのスレッドで呼んで `capture` で数えて確かめる（`install_global_capture_all` は使わない）
  - 完了の状態: `cargo test -p areka-mcp` の新しいテストが緑で、`handler.rs` が 1,000 行の内
  - _Boundary: ParamsAnswer, HandlerTests_
  - _Requirements: 1.1, 1.3, 1.4, 1.5, 1.6_

- [x] 1.2 5 つの版の実ソケットで新しい答えと変わらない答えを固定する
  - 版ごとの話し方で要求を組む小さな部品を 1 つ足す（旧式の 4 版は `MCP-Protocol-Version` の札・`initialize` だけは札なし、`2026-07-28` は `MCP-Protocol-Version`・`Mcp-Method`・`params` の `_meta`・`name` が文字列なら `Mcp-Name`）。今の版の並びと `_meta` の部品を使い回す
  - テスト 1: 5 つの版それぞれで、比べる相手として未登録の名前の `tools/call` の HTTP の状態を控え、`tools/call` の `{"name":7}`・`{}`・`{"name":"get_status","arguments":7}`・`params` なし（旧式の 4 版だけ）、`initialize` の `{"protocolVersion":7}`、`completion/complete` の `{"ref":7}` が、本文の `error` が `{"code":-32602,"message":"invalid params for <m>"}` に等しく（`data`・`result` なし）、状態が比べる相手と同じ（旧式 200・`2026-07-28` 400 を値でも書く）ことを確かめる
  - テスト 2: 5 つの版それぞれで、`nope/unknown` の `{"name":7}` が `{-32601, "nope/unknown"}`、`2026-07-28` だけ `ping` が `{-32601, "ping"}`・404、`tools/list` の `{"cursor":7}` が成功のままであることを確かめる
  - 完了の状態: 追記した 2 本が緑で、`server_protocol_tests.rs` が 1,000 行の内（560 行ほど）
  - _Boundary: ReceiverTests_
  - _Depends: 1.1_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6, 2.1, 2.3_

- [x] 2. (P) 中継の比較テストの 1 行と差の記録の 1 行を新しい答えに替える
  - 中継の比較テストの `id` が 8 の行（`tools/call` の `{"name":7}`）の期待を `-32602` に替え、その上の説明を「形の違う `params` の `tools/call` は受け口が `-32602` で答え、中継も型のない要求として届けるので同じ答えになる」に替える。ほかの行の期待・比べ方（直接と中継ごしの全体が等しい・5 つの版）・中継のコードは変えない
  - 差の記録の「形の違う `params` の `tools/call`」の行の areka の欄を `-32602`（`message` は `invalid params for tools/call`・`data` なし）・中継ごしでも同じ答えに替え、「本 spec で直す」の 2 文を外す。テストの欄はそのまま
  - 完了の状態: `cargo test -p areka-mcp-bridge --test real_receiver_test` が 5 つの版で緑、差の記録の行が事実と一致する
  - 1.2 とは触るファイルが重ならないので並べて進められる。1.1 の答えが入っていないと期待の替わった行が赤になる
  - _Boundary: BridgeCompare, DiffRecord_
  - _Depends: 1.1_
  - _Requirements: 2.2, 2.4, 2.5, 2.6_

- [x] 3. 受け口と中継の既存のテストが期待を変えずに緑であることを確かめる
  - `cargo test -p areka-mcp` と `cargo test -p areka-mcp-bridge` を通し、`tools_call_any_name_is_invalid_params`・`unknown_method_is_jsonrpc_error_from_rmcp`・`unknown_protocol_version_header_is_rejected` を含む既存のテストが 2 の 1 行のほかは期待を変えずに緑であることを確かめる
  - 完了の状態: 2 つのクレートのテストがすべて緑で、`git diff` の期待の変更が 2 の 1 行だけ
  - 確かめだけの段で、ファイルは変えない
  - _Depends: 1.2, 2_
  - _Requirements: 1.6, 2.6_

- [x] 4. debug 版の `dump_balloon` を実機で確かめる（件 2）
- [x] 4.1 (P) 確かめの台本を書く
  - spec の `verification/` の下に PowerShell の台本を置く（areka のコードでもテストでもなく、全体テスト・配布に入らない）。絶対パスで `pwsh -File` から起こせ、ワークツリーの根は台本の場所から決める。引数は `-Ghost`（既定 claudia,emo2）・`-Calls`（既定 5）・`-Tool`（既定 dump_balloon・3.6 のときだけ dump_surface）
  - `-Ghost` と `-Tool` の値は建てるより前に確かめ、知らない値なら理由を 1 行出して終了コード 2 で終わる
  - 建てる（debug の areka と中継、続けて i686 の helper）→ i686 の helper を `areka.exe` の隣へ最後に写し PE の機械種別 `0x014c` を確かめる → 検体の置き場を `nar-sample-path` で取る（バルーンは claudia が `claudia`、emo2 が `emo2-kakukaku`）→ 作業の置き場 `target\mcp-receiver-bugs\<ゴースト>\` を毎回作り直す
  - areka を空きポート・`AREKA_PROFILE_DIR`（作業の置き場の `profile`）・`RUST_LOG=warn,areka::mcp=debug,areka_mcp=debug`・`NO_COLOR=1`・`AREKA_NO_ALERT=1`・`AREKA_APP_SMOKE_EXIT_MS=360000` で起こし（`AREKA_ROOT`・`AREKA_USER_ROOT` は消す）、中継を同じポート・別の `AREKA_PROFILE_DIR` で標準入出力をつないで起こす。標準エラーは作業の置き場へ写す
  - 話す順: `initialize`（`2025-06-18`）と `notifications/initialized` → 要件 2.7 の `tools/call` `{"name":7}` を 1 回 → `dump_balloon`（`scope` 0）を 1 秒おきに絵が返るか 60 秒まで → 測る回を `-Calls` 回。答えの行は 40 秒まで待つ
  - 各回を `results.json` に書き（回の番号・段・所要のミリ秒・分類 ⑴ 絵〔データの長さも〕⑵ 断り〔理由〕⑶ 上限の答え ⑷ 中継の時間切れ ⑸ 返らなかった）、端末には 1 回 1 行の ASCII の要約と測る回の絵の数を出す
  - 終了コード: 測る回がすべて ⑴ か ⑵ で絵が 1 回以上なら 0、⑶〜⑸ が 1 回でもあれば 1、条件を作れない（60 秒で絵が出ない・建てる／検体／起動の失敗・測る回の絵が 0 回）なら 2。畳むときは中継の標準入力を閉じ、areka は自動終了を待ち、締切（420 秒）を過ぎたら台本が起こした子だけを止める。ゴーストは 1 体ずつ順に起こす
  - 完了の状態: 台本が構文エラーなく読め、`-Ghost` に存在しない名前を渡すと理由を 1 行出して終了コード 2 で終わる
  - _Boundary: ManualRun_
  - _Requirements: 2.7, 3.2, 3.6, 3.8_

- [x] 4.2 claudia と emo2 で台本を回し、確かめの記録を書く
  - 1 と 2 の直しが入った後のコードで、回す前に所要時間（2 体で約 13 分＋初回の建てる時間）を伝えてから、PowerShell から台本を claudia・emo2 の順に回す（Claude アプリの Bash からは起こさない）
  - 記録に、areka の版と元のコミット・`git merge-base HEAD origin/main` とその間の `git diff --stat`（`crates/areka/src/mcp/` に触れていないこと）・debug 版であること・ゴーストとバルーン・呼び方（中継ごし・`scope` 0・`2025-06-18`）・上限の値（受け口 10 秒・中継 30 秒・台本の読み 40 秒）・置き場がすべて `target\` の下で i686 の helper が隣にあること・2.7 の答え・各回の分類と所要時間（待ちの回も）・ゴーストごとの測る回の絵の数・結論を書き、台本を添える
  - 終了コード 2 の走行は「条件を満たせなかった」として数えず、手順を見直して回し直す
  - 完了の状態: 記録に 10 回（各 5 回）の結果があり、どれも ⑴ か ⑵ でゴーストごとに絵が 1 回以上なら「件 2 は再現しない」と書いて閉じ、areka のコードは変えていない。そうでなければ結論を「再現した」とし 5 へ進む
  - _Boundary: RunRecord_
  - _Depends: 3, 4.1_
  - _Requirements: 2.7, 3.1, 3.2, 3.3, 3.8_

- [x] 5. 件 2 が再現したときだけ、待ちが止まる根本を直す（4.2 が「再現しない」で閉じたら、記録の結論を示してこの 2 つを完了とし、コードは触らない）
- [x] 5.1 止まった場所を突き止めて根本を直し、決定論のテストを 1 本足す
  - 再現した走行の `debug` の記録で、段の機械の `Attaching` のまま・`Reading` のまま・`poll_later` が回らない、のどこで止まったかを読み分ける。足りなければ段の移りに `trace!` を足し、`RUST_LOG` の `areka::mcp` を `trace` に開けて同じ条件で起こし直す
  - 状態の持ち方を直す（1 フレーム遅らせる解は取らない）。受け口の 10 秒・中継の 30 秒・バルーンの描き方・装着の待ちの段数は変えない。触るのは `dump_balloon`・`dump_surface`・`mod.rs` のうち止まった場所を持つファイルだけ
  - 直したファイルの兄弟のテストのファイルに 1 本足し（GPU を通すなら本物の GPU のテスト）、直す前の形で赤・直した後で緑になることを記録に残す
  - 完了の状態: 足したテストが直す前は赤・直した後は緑で、`cargo test -p areka` の該当のテストが緑
  - _Boundary: ConditionalFix_
  - _Depends: 4.2_
  - _Requirements: 3.4, 3.5, 3.6, 3.7_

- [x] 5.2 直した後に同じ条件の 10 回をやり直して記録に足す
  - 4.1 の台本で claudia と emo2 を回し直し、直した後の 10 回を記録に足す。原因が `dump_surface` と分け合う段の機械にあったときは `-Tool dump_surface` でも同じ条件の 10 回を回して記録する
  - 完了の状態: 直した後の 10 回（分け合う所なら `dump_surface` の 10 回も）がどれも ⑴ か ⑵ で、台本の終了コードが 0、記録の結論が「直した」で閉じている
  - _Boundary: RunRecord_
  - _Depends: 5.1_
  - _Requirements: 3.5, 3.6_

## Implementation Notes

- 4.2（2026-10-18）: 件 2 は再現しなかった（claudia 5/5・emo2 5/5 が絵）。記録は `verification/debug-dump-balloon.md`。5.1・5.2 は 5 の但し書きどおり、記録の結論をもって完了とし、コードは触っていない。
- PowerShell で `[Environment]::SetEnvironmentVariable($k, $null)` は変数を消さず空の値を置く（`$null` が `""` になる）。消すときは `[NullString]::Value` を渡す。
