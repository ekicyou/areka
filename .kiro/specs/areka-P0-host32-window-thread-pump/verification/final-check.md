# 最終確認（タスク 6.1）: 常設テスト全体・編集集合・規模

- 日時: 2026-09-17
- コミット: `cca0b618`（作業ツリーに未コミットの変更なし）
- 比較の起点: `git merge-base HEAD origin/main` = `a726174ade19712599b605131c66be8621ebc350`
- 対象要件: 2.5, 2.6, 2.7, 2.8, 4.9, 4.11, 7.2, 7.3, 7.4, 7.5

## 1. 前提の成果物

| コマンド | 終了コード |
|---|---|
| `cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc` | 0 |
| `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` | 0 |
| `cargo test --workspace --no-run`（テストの実行ファイルを先に作る） | 0 |

`--no-run` のビルドで `target/debug/shiori-host32-helper.exe` が x64 版（317,952 バイト）に上書きされていたため、
`target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe`（273,408 バイト）を `target/debug/` へコピーしてから全体テストを走らせた。
コピー後の大きさは 273,408 バイトで、32bit 版に置き換わったことを確かめた。

## 2. ワークスペース全体のテスト

コマンド: `cargo test --workspace`（出力はすべてファイルへ保存し、途中で切っていない）

| 項目 | 値 |
|---|---|
| 終了コード | 0 |
| 所要（壁時計） | 3 分 17.6 秒 |
| `Running` 行（テストの実行ファイル） | 81 本 |
| `Doc-tests` 行 | 22 本 |
| `test result:` 行 | 103 本（81 + 22 と一致＝全スイートが最後まで走った） |
| `test result: FAILED` の行 | 0 本 |
| passed 合計 | 7,602 |
| failed 合計 | **0** |
| ignored 合計 | 40（本仕様で増やしたものは 0。新しい 6 本はいずれも `ok` で走っている） |

失敗が 0 件だったため、単独での再実行は行っていない（再実行の対象が 0 件）。

### 新しい 6 本（無条件に含まれていること・4.11）

| テスト | 置き場 | 結果 |
|---|---|---|
| `shiori::real::idle_tests::on_idle_is_called_while_idle` | `crates/areka-kanade/src/shiori/real_idle_tests.rs` | ok |
| `shiori::real::idle_tests::helper_exit_during_idle_reports_shiori_down_once` | 同上 | ok |
| `shiori::real::idle_tests::requests_after_idle_are_served_in_order` | 同上 | ok |
| `shiori::real::idle_tests::no_liveness_report_after_clean_unload_even_when_idle` | 同上 | ok |
| `idle_pump_test::sync_send_to_idle_window_returns_within_bound` | `crates/areka-kanade/tests/kanade/idle_pump_test.rs` | ok |
| `idle_pump_test::abortifhung_send_reaches_window_idle_for_twenty_seconds` | 同上 | ok |

2 つのファイルに `#[ignore]`・環境変数による分岐・feature による除外は 0 か所（`#[test]` の直前・本文を検索して確認）。
`real.rs` 側の接続宣言は `#[cfg(test)]` のみ、`tests/kanade.rs` 側は無条件の `mod idle_pump_test;` である。

### 関係するスイートの内訳

| スイート | passed | failed | ignored | 所要 |
|---|---|---|---|---|
| `areka-kanade` のライブラリテスト（速い 4 本を含む） | 290 | 0 | 0 | 1.54 秒 |
| `areka-kanade` の統合テスト `tests/kanade.rs`（窓を作る 2 本を含む） | 52 | 0 | 0 | 40.76 秒 |
| `shiori-host32-helper`（既存の遅い hung テストを含む） | 21 | 0 | 3 | 40.01 秒 |
| `shiori-host32-host` のライブラリテスト | 126 | 0 | 0 | 0.22 秒 |
| `shiori-host32-ipc` のライブラリテスト | 20 | 0 | 0 | 0.00 秒 |

`areka-kanade` のライブラリテスト全体が 1.54 秒で終わっており、速いテストは 5 秒以内（4.9）。

## 3. 編集集合（7.3）

`git diff --name-only <起点> HEAD` から本仕様の文書（`.kiro/specs/areka-P0-host32-window-thread-pump/`）を除いた変更ファイルは 10 本で、
すべて設計の「File Structure Plan」に載っている。計画に無いファイルの変更は 0 本。

| ファイル | 計画上の扱い |
|---|---|
| `crates/areka-kanade/src/shiori/real.rs` | 修正 |
| `crates/areka-kanade/src/shiori/real_idle_tests.rs` | 新規 |
| `crates/areka-kanade/tests/kanade.rs` | 接続宣言の追加（差分は 2 行） |
| `crates/areka-kanade/tests/kanade/common/mod.rs` | 接続宣言の追加 |
| `crates/areka-kanade/tests/kanade/common/common_window_actor.rs` | 新規 |
| `crates/areka-kanade/tests/kanade/idle_pump_test.rs` | 新規 |
| `crates/areka-kanade/tests/kanade/real_helper_test.rs` | 窓の生成をロックで囲む（rustfmt による折り返しを含む） |
| `crates/shiori-host32-host/src/parent_window.rs` | 修正（コードの差分は import と `pump_pending_messages` の追加のみ） |
| `crates/shiori-host32-ipc/src/lib.rs` | コメントのみ |
| `crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs` | コメントのみ |

設計の計画にある `verification/real-machine.md` はタスク 6.2 の成果物で、本確認の時点では未作成。

## 4. 併走 spec などが所有するファイルに差分が無いこと（2.5〜2.8・3.3・7.3）

実在しないパスを指定すると差分は空になり終了コード 0 になるため、先に `git ls-files` で追跡ファイル数を数え、1 以上であることを確かめてから
`git diff --quiet <起点> HEAD -- <パス>` の終了コードを見た（0 = 差分なし）。

| パス | 追跡ファイル数 | `git diff --quiet` の終了コード |
|---|---|---|
| `crates/areka-kanade/src/schedule/`（起動・終了の配線） | 18 | 0 |
| `crates/areka-kanade/src/actor.rs` | 1 | 0 |
| `crates/areka-kanade/src/msg.rs` | 1 | 0 |
| `crates/areka-kanade/src/shiori/mod.rs` | 1 | 0 |
| `crates/areka-kanade/src/shiori/real_tests.rs` | 1 | 0 |
| `crates/areka-kanade/Cargo.toml` | 1 | 0 |
| `crates/shiori-host32-host/src/shiori3.rs` | 1 | 0 |
| `crates/shiori-host32-host/src/client.rs` | 1 | 0 |
| `crates/shiori-host32-host/src/lifecycle.rs` | 1 | 0 |
| `crates/shiori-host32-host/src/process_host.rs` | 1 | 0 |
| `crates/shiori-host32-host/Cargo.toml` | 1 | 0 |
| `crates/shiori-host32-ipc/Cargo.toml` | 1 | 0 |
| `crates/shiori-host32-helper/Cargo.toml` | 1 | 0 |
| `crates/areka-ghost/` | 41 | 0 |
| `crates/areka-actor/` | 9 | 0 |
| `Cargo.toml`（ワークスペース） | 1 | 0 |

crate 単位の変更ファイル数:

| crate | 追跡ファイル数 | 変更ファイル数 |
|---|---|---|
| `crates/shiori-host32-host/` | 18 | 1（`parent_window.rs`） |
| `crates/shiori-host32-ipc/` | 2 | 1（`src/lib.rs`・コメントのみ） |
| `crates/shiori-host32-helper/` | 8 | 1（`main_response_flavor_hung_cage_tests.rs`・コメントのみ） |

IPC と helper の変更がコメントだけであることは、`git diff -U0 <起点> HEAD -- <ファイル>` の追加・削除行から
`//` で始まる行を除いた残りを数えて確かめた。

| ファイル | 追加・削除行のうちコメント以外 |
|---|---|
| `crates/shiori-host32-ipc/src/lib.rs` | 0 行 |
| `crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs` | 0 行 |

## 5. 完了 spec のアーカイブと互換アーキテクチャ文書（7.2・7.5）

| パス | 追跡ファイル数 | `git diff --quiet` の終了コード |
|---|---|---|
| `.kiro/specs/completed/`（`areka-P0-emo2-conformance-e2e/` を含む全体） | 1,381 | 0 |
| `doc/COMPAT_ARCHITECTURE.md` | 1 | 0 |

`doc/COMPAT_ARCHITECTURE.md` への追記は 0 行（設計の「§8 の扱い」どおり追記しない）。

## 6. 規模（7.4）

変更した `crates/` 配下のファイル 10 本の行数（`wc -l`）。1,000 行を超えるファイルは 0 本。

| ファイル | 行数 |
|---|---|
| `crates/shiori-host32-host/src/parent_window.rs` | 694 |
| `crates/shiori-host32-ipc/src/lib.rs` | 637 |
| `crates/areka-kanade/src/shiori/real.rs` | 360 |
| `crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs` | 292 |
| `crates/areka-kanade/src/shiori/real_idle_tests.rs` | 260 |
| `crates/areka-kanade/tests/kanade/real_helper_test.rs` | 260 |
| `crates/areka-kanade/tests/kanade/idle_pump_test.rs` | 169 |
| `crates/areka-kanade/tests/kanade/common/mod.rs` | 96 |
| `crates/areka-kanade/tests/kanade/common/common_window_actor.rs` | 89 |
| `crates/areka-kanade/tests/kanade.rs` | 27 |

## 結論

全体テストは 103 スイートすべてが完走し、失敗 0 件。新しい 6 本は無条件に含まれて `ok`。
編集集合は設計の計画の範囲に収まり、併走 spec などが所有するファイル・完了 spec のアーカイブ・互換アーキテクチャ文書の差分はすべて 0。
変更したファイルはすべて 1,000 行以下。
