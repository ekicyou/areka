# 実装の記録: runtime-crate-split

## 1. 着手の基準（段 0）

対応する要件: 7.2, 6.6（「前」の側）

### 1.1 テスト一覧（前）

- 取った場所: ブランチの先頭 `6a0e44737637841d0a590cadb0e56842e731a11c`（コードを変える前）
- コマンド: `cargo test --workspace --no-fail-fast -- --list`（標準出力を保存）
- 保存先: `target/runtime-crate-split/tests-before.txt`
- 行末が `: test` の行の数（`grep -c ': test$'`）: **14143**

### 1.2 ログ履歴の分類（前）: 実行系（`areka::runtime`）の行はどの種別にも入らない

現行の `log_history` の分類（`crates/areka/src/log_history.rs` の RULES）では、target が `areka::runtime` の行はどの種別にも当たらない。

- 既存のテスト `status_rows_and_areka_is_exact_only`（`crates/areka/src/log_history_tests.rs` 100 行目〜）の 124 行目:
  `assert_eq!(classify(Level::INFO, "areka::runtime"), None);`
  （コメント 123 行目: `areka` は完全一致だけで、下のモジュールの info は外れる）
- 実行: `cargo test -p areka --bin areka log_history` → 43 passed; 0 failed（上のテストを含む。緑）

分割の後はこの行が `areka_runtime` 側の target になるので、後のタスクで「後」の側を同じ形で示し、前後を比べる。
