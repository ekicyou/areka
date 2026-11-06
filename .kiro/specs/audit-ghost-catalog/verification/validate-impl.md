# audit-ghost-catalog の機能単位の検証（2026-11-06）

## Validation Report
- DECISION: GO
- MECHANICAL_RESULTS:
  - Tests: PASS（`pwsh -NoProfile -File tools/test-all.ps1`・終了コード 0・全段緑。最終は `0259dc69` の木で回した。1 回目は `72ea7fd6` で緑、その後の検証の直しを入れて回し直した）
  - TBD/TODO grep: CLEAN（`main` との差分の足した行に 0 件）
  - Secrets grep: CLEAN
  - Smoke boot: PASS（同じ全体テストの中の `crates/areka/tests/smoke_boot_loop_exit.rs` 5 本が緑）
- INTEGRATION:
  - Cross-task contracts: 合う。テストの道具は組み立ての単位ごとに 1 つ（`src/test_support.rs`・`tests/ghost/common.rs`・`tests/ghost/recorder.rs` の `BoxedBackend`）。名前の読みは `catalog::descript_name_in` と `config::listed_name` の 1 つずつ（`crates/areka` の `mcp/resolve.rs`・`readme/opener.rs` も後者を呼ぶ）
  - Shared state consistency: 合う。descript は `parse_kv_lowercased`、`install.txt` は後の行が勝ち空の値を残す（理由は `manifest.rs` の頭に書いた）。作業フォルダの寿命は文書と実装が合う。名前の大小の比べ方は検証で 1 つにそろえた（下の直し ⑴）
  - Boundary audit: 範囲の外で変わった 19 ファイルは、どれも設計の「範囲の外で触るファイル」の表に載る。下流の都合が上流へ漏れた形は無い（sylphya の `RemoveFlat` は要件 9.15 の汎用の口）
- COVERAGE:
  - Requirements mapped: 要件 1〜9（9.1〜9.16）を全部、完了したタスクで満たす
  - Coverage gaps: 無し。直さない #17・#41・#57、議題 ⑴、取り下げた #56 は要件 9.2・9.15・9.16 の記録どおり
- DESIGN:
  - Architecture drift: 無し（「変わること」⑴〜⑻ を抜き取りで確かめた）
  - Dependency direction: 新しい依存は無い。`Cargo.lock`・`vendors/` は `main` との差分 0
  - File Structure Plan vs actual: 合う。触った `.rs` は全部 1,000 行以下（最大 990 行）
- OWNERSHIP: LOCAL
- UPSTREAM_SPEC: N/A
- BLOCKED_TASKS: 無し（1〜18.4 がすべて `[x]`）
- REMEDIATION: 無し（検証で見つけた 2 件は下のとおり直した）

## 検証で見つけて直したもの
1. 名前の大小の比べ方が 2 通りあった。`areka-nar` の `names::fold` だけが `to_lowercase` で、語末の Σ を ς にするため、NTFS で同じ名前の `aΣ` と `aσ` を別の名前と見ていた。`catalog_roots` の `fold_key` と `areka-user-data` の写しと同じ「1 字を 1 字の大文字へ畳む」形へそろえ、テスト `the_case_fold_maps_every_sigma_to_one_name` を足した（`20ee8f9b`・要件 9.16）
2. #56 を取り下げたことが要件と設計に書かれていなかった。要件 9.16 に書き、設計の古い 2 か所を直した（`20ee8f9b`・`0259dc69`）

## 完了の棚卸へ申し送るもの（tasks.md の Implementation Notes）
- `crates/areka-mcp-bridge/src/config.rs` が空の `AREKA_PROFILE_DIR` で作業フォルダからの相対の置き場を作る（持ち主 `audit-mcp-update-crates`）
- `crates/shiori-host32-helper/src/shiori_proxy.rs` が素の `LoadLibraryW`（持ち主 `audit-shiori-host32`）
- i686 の helper が親の `PASTA_DEBUG` をそのまま受け継ぎ 127.0.0.1:9276 を取り合う（`shiori-host32-host` の `process_host.rs`）
- 新しい clippy の指摘が範囲の外の `dola`・`areka-kanade` に残る
- roadmap の `temp-path-exceptions-to-target` の行へ、本 spec が 5 行を取ったことと残りの行数を書く（設計の約束）
