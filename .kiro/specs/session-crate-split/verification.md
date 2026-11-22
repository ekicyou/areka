# 検証の記録（session-crate-split）

## 着手のときの証跡

### テストの一覧（1.1）

- コマンド: `cargo test --workspace --no-fail-fast -- --list`（標準出力を `target/session-crate-split/tests-before.txt` に残した。14755 行・終了コード 0）。
- 基点のコミット: `ba887213c`（枝 `claude/kiro-start-session-crate-split-56c35d`・コードの変更なし）。
- `: test$` の行は 14443 行。`: test$` の行から `: test` と先頭の `パス::` を落として名前の末尾の多重集合を作れる形である（異なる末尾 14330 種・合計 14443。同名が 113 件重なる）。段 6 では同じ手順で作って比べる。

### 兄弟との照らし（1.1）

兄弟 2 本の作業の木（読むだけ）で、触るファイルを設計の「同じウェーブとの重なり」と照らした。

- `test-name-work-tags`（木 `kiro-start-pilot-windows-rs-next-86646c`・tasks.md あり）: 名前の付け替えが中心。本体の `runtime_tests/zorder_wiring_tests.rs` は範囲から外れており、この仕様の `include_str!` と `t_zwi05` の字面の書き換えと重ならない（⑴）。名前の行を直す `doc/COMPAT_ARCHITECTURE.md` §8 はこの仕様がパスの行だけを直す許された重なり（⑷）で、後から着地する側が手で解く。`.kiro/specs/audit-areka-app/brief.md` は向こうが 1 項目を足し、こちらも書き換えるので、同じファイルの別の箇所として競合しうる。`crates/areka-runtime/src/consumer_ledger_test_support.rs` の 28 行・`window_state_cue_tests.rs` の 324 行は名のコメントだけで、この仕様は触らない。
- `dola-cue-test-builder`（木 `kiro-start-pilot-windows-rs-next-86646c`・brief.md のみで tasks.md は無い）: brief の触るファイルは `crates/dola/src/cue/command.rs`・`crates/dola/Cargo.toml`・`areka-ghost` の `prop_sink.rs`・`prop_get_sink_tests.rs`、`areka-seriko` の `actor_test_support.rs`、`areka-runtime` の `consumer_ledger_test_support.rs`・`frame_window_link_drain_tests.rs`・使い手の `Cargo.toml`。呼び手の名（`carrier_cue` ほか）を残す形なら、この仕様が移す・書き換えるファイルに触らない（⑵）。
- 結果: 設計が足したファイル（`tools/package.ps1`・`tools/perf/judge-perf.py`・`tools/load-flake.ps1`・`doc/ukadoc-coverage/`・`doc/ideas/`・`session_tests/` ほか）は兄弟 2 本の触るファイルと重ならない。重なりうるのは上の `doc/COMPAT_ARCHITECTURE.md` と `audit-areka-app/brief.md` の 2 件だけで、前者は設計が許した重なり、後者は後から着地する側が両方を残して解く。
