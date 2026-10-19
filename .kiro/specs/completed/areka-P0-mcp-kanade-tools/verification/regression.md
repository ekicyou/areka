# 回帰の記録（タスク 6.2）

- 日付: 2026-10-19
- コミット: `8b3c5f247e79c2e08a9ebbeb58ef951e9b0cc07c`（作業ツリーは未変更の状態で回した）
- 分かれた元（merge base）: `e2370df8a225c93ec2ec94ac77d76d330dca3464`

## 回したコマンドと結果（この順に 1 本ずつ・並べて回していない）

| # | コマンド | 結果 |
|---|---|---|
| 1 | `cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` | 成功 |
| 2 | `cargo test -p areka-kanade -j 4 --no-fail-fast` | 単体 647 通過・`tests\kanade.rs` 85 通過・失敗 0 |
| 3 | `cargo test -p areka-ghost -j 4 --no-fail-fast` | 単体 186 通過・`tests\ghost.rs` 40 通過・`tests\real_pasta_online.rs` 6 通過・失敗 0 |
| 4 | `cargo test -p areka -j 4 --no-fail-fast` | 単体 3,158 通過（2 件は元から `ignore`）・`embedded_icon_test` 2・`emo2_real_run` 1・`mcp_get_log_real_run` 1・`smoke_boot_loop_exit` 5 通過・失敗 0 |
| 5 | `cargo test -p log-capture-kit --test file_length_guard_test` | 6 通過・失敗 0（1,000 行の番人は緑） |
| 6 | `cargo fmt --all -- --check` | 差分なし（終了コード 0） |

- 揺れ（回し直しで緑になったもの）: なし。今回は 1 回目ですべて緑だったので、回し直しはしていない。
  5.2 の記録にある `mcp::check_script::check_script_tests::omitted_or_matching_name_answers_with_the_running_ghost` も今回は 1 回目で通った。
- 4 の `KanadeMsg` の網羅の `match`（MCP の外）も含めて areka 全体がビルドでき、赤は出ていない。
- 注: `emo2_real_run_boots_talks_and_exits_zero` は環境変数で開く実走で、今回は変数を立てていないため中で何もせず通過している（このテストのいつもの扱い）。

## 触らない約束のファイルの差分

merge base との差を次のコマンドで見た。

```
git diff $(git merge-base HEAD main) --stat -- crates/areka-kanade/src/change.rs crates/areka-kanade/src/schedule/boot.rs crates/areka-kanade/src/schedule/steady.rs crates/areka-kanade/src/schedule/change.rs crates/areka-ghost/src/runtime.rs crates/areka-ghost/src/dispatcher.rs crates/areka-mcp crates/areka/src/main.rs crates/areka/src/mcp/mod.rs crates/areka/src/mcp/resolve.rs crates/areka/src/mcp/get_status.rs crates/areka-talk doc/ssp-mcp/areka-tools.md crates/areka/tests
```

- 結果: **差分 0 件**（出力なし）。
- 実走の 3 本: `crates/areka/tests` の実走のテスト（`emo2_real_run.rs` の `emo2_real_run_boots_talks_and_exits_zero`、`mcp_get_log_real_run.rs` の `get_log_status_over_real_socket_keeps_lines_hidden_from_stdout_and_bodies_intact`、`smoke_boot_loop_exit.rs` の実ゴースト起動のテスト）は、`crates/areka/tests` をまるごと上のコマンドに含めて **差分 0 件**。
  名前に `real_run` を含むテストの関数は `emo2_real_run_boots_talks_and_exits_zero` の 1 本だけで、ほかの 2 本はファイル名で数えている。

### `schedule/close.rs` だけは合意済みのテスト用の変更あり

- `crates/areka-kanade/src/schedule/close.rs` は 2 か所・各 1 行の追加（`..State::initial()`）だけ。
- 2 か所とも 189 行目から始まる `#[cfg(test)] mod tests` の中（差分の位置は 223 行目と 249 行目付近）で、テスト用に `State` を組み立てる補助の中。本番のコードの行は 1 行も変わっていない。
- この変更は 3.3 で `State` に欄を足した際に、持ち場の mcp-reload のセッションと合意して入れたもの（tasks.md の Implementation Notes 3.3）。
