# 検証の記録（audit-ukadoc-survey）

## 指摘 35: カタログの読み直しの 1 バイトの安定（タスク 1.1）

- 整合テスト `the_catalog_round_trips_byte_for_byte`（`crates/ukadoc-survey/tests/consistency/checks.rs`）を置き、最初にこの 1 本だけを回した。
- 結果: 緑。今の `catalog.toml` は `write(read(t)) == t` を満たす。差は 0 バイトで、6.5 の議題は起きない。

## 所要時間（Performance の「前」）

- `cargo test -p ukadoc-survey`（ビルド済み・排他テストの机を持って 1 回）: 各テスト実行ファイルの所要の合計 約 40.9 秒（単体 601 本 2.56 秒・cli_streams 6 本 2.12 秒・consistency 114 本 36.11 秒・doc-test 5 本 0.06 秒）。全部緑。壁時計の通し値は取り損ねた（計時の道具が無かった）
