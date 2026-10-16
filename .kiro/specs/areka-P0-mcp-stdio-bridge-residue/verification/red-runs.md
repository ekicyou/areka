# 赤の実走と確かめの記録

## 確かめ C（要件 2.3・タスク 2.1）: 偽の受け口の自己確認を部品のテストの束でだけ走らせる

直す前と直した後に、ワークツリーの根で同じコマンドを取った。

```
cargo test -p areka-mcp-bridge -- --list
```

控えはワークツリーの `target\residue-2.1\list-before.txt`（前）と `target\residue-2.1\list-after.txt`（後）。

### 束ごとの数

| 束 | 前のテストの数 | 前の自己確認 | 後のテストの数 | 後の自己確認 |
|---|---|---|---|---|
| 部品のテスト（`src\lib.rs`） | 86 | 2 | 86 | 2 |
| `src\main.rs` | 0 | 0 | 0 | 0 |
| 通しのテスト（`tests\exe_test.rs`） | 10 | 2 | 8 | 0 |
| 通しのテスト（`tests\real_receiver_test.rs`） | 6 | 0 | 6 | 0 |
| 文書のテスト | 0 | 0 | 0 | 0 |
| 合計 | 102 | 4 | 100 | 2 |

- 自己確認の 2 本（`one_request_gets_the_scripted_reply_and_is_recorded`・`the_handshake_defaults_and_every_reply_mode_answer_as_named`）は、前は部品の束と `exe_test` の束に 1 回ずつ（道筋は両方とも `fake_server::tests::<名前>`）、後は部品の束に 1 回だけ（道筋は `fake_server_tests::<名前>`）、通しの束では 0 回。
- 合計が 102 から 100 へ減った 2 本は、`exe_test` の束で 2 回目に走っていた自己確認の 2 本そのもの。

### 関数の名前の集まり

道筋（`::` の前）を外した関数の名前を重ねずに並べると、前も後も 100 個で、2 つの並びに違いは無い（`diff` の出力が空）。どの関数の名前も直した後に少なくとも 1 回走る。

### テストの結果

直した後の `cargo test -p areka-mcp-bridge` は緑（部品 86・`main.rs` 0・`exe_test` 8・`real_receiver_test` 6・文書 0、失敗 0）。`cargo fmt --all -- --check` も差分なし。テストの本体は変えていない（替えたのは自己確認のファイルの頭の説明と `use`、偽の受け口の頭の説明と末尾の `mod tests` の宣言、`src\lib.rs` の兄弟の宣言だけ）。

## 確かめ B（要件 4.4・5.4・8.2〜8.4・タスク 5.1）: 件 4・5 のテストに変異を当てて赤と緑を確かめる

変異は 1 つずつ当てた。当てる前に、元のファイルをワークツリーの `target\mutation-backup\`（`http.rs`・`main.rs`）へ写し、戻すときは写しを素のコピーで上書きした（`git checkout` は使っていない）。

控えの sha256（当てる前の作業ファイルと同じ）:

| ファイル | sha256 | git のオブジェクト（`git hash-object`） |
|---|---|---|
| `crates\areka-mcp-bridge\src\http.rs` | `a9f9ef33c9f375cdc3cfe50e864f79cc64b9b57d4f8f8d6c3df13119846b5584` | `d44a69effcc4339bdc3d818330913fe08bf6a6e5` |
| `crates\areka-mcp-bridge\src\main.rs` | `c7adc674fd171405261cf5dfc9dcb91bec39255a7fac0885361514ed396920bd` | `22dce89bd8da74650a5065e8e08ebaef168cbb15` |

作業ファイルは CRLF の行末で取り出されている（`core.autocrlf=true`）ので、`git show HEAD:<道筋>` の出力（LF）の sha256 とは素のままでは一致しない。行末を CR LF にした `git show HEAD:<道筋>` の出力の sha256 は控えと同じで、`git hash-object` の値は `git rev-parse HEAD:<道筋>` と同じ。どちらでも、控えは HEAD と同じ中身。

### 件 4: `id` を持つ要求に 202 が返ったら、すぐ「応答なし」のエラーにする

当てた変異（`http.rs` の `OwnHttp::post_message` だけ）: `post` の答えを受け取った後に、送ったのが要求（`ClientJsonRpcMessage::Request`）で答えが `StreamableHttpPostResponse::Accepted` なら `Err(StreamableHttpError::Client(HttpError::NoResponse))` を返す。202 を分ける `classify` の枝そのものは替えていない（通知への 202 はそのまま受け取ったで通る）。

```
cargo test -p areka-mcp-bridge --lib a_request_answered_with_202
```

- 赤: `service::failure_tests::a_request_answered_with_202_waits_for_the_bound_and_gets_the_timeout_error_once` が失敗（1 本中 1 本）。
- 落ちた判定: `service_failure_tests.rs` 132 行目の `assert_eq!(answer, failed(json!(1), message))`。
  - 実際: `"No response from server: empty body or connection closed"`（上限を待たずにすぐ返った）
  - 期待: `"No response from server: timed out after 300ms"`（上限まで待ってからの 1 回のエラー）
- 戻した後: 控えと作業ファイルの sha256 が同じ（`a9f9ef33…46b5584`）、`git hash-object` も HEAD と同じ（`d44a69ef…`）。同じコマンドで緑（1 本通過）。

### 件 5: 中継の入口の 6 秒の見切り（`EXIT_WAIT`）を 0 にする

当てた変異（`main.rs` の 1 行だけ）: `const EXIT_WAIT: Duration = Duration::from_secs(6);` を `Duration::ZERO` に替えた。

```
cargo test -p areka-mcp-bridge --test exe_test after_stdin_closed
```

- 赤: 2 本とも失敗（2 本中 2 本）。2 本とも標準エラーに `[bridge] stdout is still not read 0ns after stdin closed; quitting` が出ていて、標準入力が閉じたとたんに中継の見切りの枝で終わっている。終了コードの判定（0）はどちらも通っており、その次の判定で落ちた。
  - `a_reply_coming_more_than_5s_after_stdin_closed_is_dropped_and_the_exit_code_is_0`: `exe_test.rs` 489 行目の `assert_eq!(areka.requests_of("tools/call").len(), 1, …)` で落ちた（実際 0・期待 1）。要求を受け口へ届ける前に終わっている。この判定を越えても、続く「閉じてから 5 秒以上 12 秒未満」と「`[bridge] stdout is still not read` の行が無い」の判定も同じ理由で落ちる形（走らせて確かめてはいない）。
  - `a_reply_coming_within_5s_after_stdin_closed_is_written_before_exit_code_0`: `answer_lines`（`exe_test.rs` 157 行目）で `stdout does not end with LF: ""` の panic。1 秒後の応答を書く前に終わり、標準出力が空。
- 戻した後: 控えと作業ファイルの sha256 が同じ（`c7adc674…396920bd`）、`git hash-object` も HEAD と同じ（`22dce89b…`）。同じコマンドで緑（2 本通過）。

### 中継の側に変異を当てられない部分（5.1 の「30 秒の上限や応答を待たない」）

5.1 のうち「閉じた後に、30 秒の応答の上限や、来ない応答そのものを待たずに 5 秒で終わる」の側は、外の部品（rmcp 3.5.0）が標準入力の終わりを見てから書きかけの応答を待つ決まった 5 秒が持っている。中継の側でこの 5 秒を決めている所は無いので、当てられる変異が無い。次の初めの 2 つは `main.rs` の読みからの見立てで、変異を当てて走らせたものではない。

- `main.rs` の `serve` の `select` を外して `run` だけを待つ形にしても、rmcp は自分の 5 秒で戻るので、5.1 のテストは緑のまま（時間も標準エラーも同じ）。
- `main.rs` の `runtime.shutdown_background()` を替えても、効くのは `serve` が戻った後に残った書きを待つかどうかだけで、rmcp が 5 秒で戻ることは変わらない。
- 中継が持つ待ちは 6 秒の見切り（`EXIT_WAIT`）だけで、これは上の件 5 の変異で赤になることを確かめた。5.1 のテストはこの見切りの枝を通らなかったこと（`[bridge] finished` の行がある・`[bridge] stdout is still not read` の行が無い）と、5 秒以上 12 秒未満の幅を判定しているので、rmcp の版を上げて 5 秒が短くなっても長くなっても赤になる。

### 最後の確かめ

2 つとも戻した後に `cargo test -p areka-mcp-bridge` を 1 回回して緑（部品 87・`main.rs` 0・`exe_test` 10・`real_receiver_test` 6・文書 0、失敗 0）。`git status` で `http.rs`・`main.rs` に差分は無い。
