# Brief: temp-path-exceptions-to-target

> 2026-11-01 `test-roots-under-target` の完了時の棚卸で起票（同 spec の要件 Boundary Context の Adjacent・要件 5.4・Known Gaps 3「例外表の 18 ファイルのテストは OS の一時フォルダを使い続ける」）。

## Problem

開発の決まりは「実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ」である。`test-roots-under-target` で窓口 `TempPath`（`crates/temp-path-kit`）の 2 つの入口がどちらも `<ワークツリー>\target\test-roots\` に作るようになったが、迂回の見張り `crates/log-capture-kit/tests/temp_path_guard_test.rs` の例外表 `ALLOWED_ENTRY_POINT_USES` に載るファイルは、今も `std::env::temp_dir()` を直に使って OS の一時フォルダ（`%TEMP%`）に根を作っている。そのため「ワークスペースのテストが作る一時の根はすべて `target\` の下」は窓口が作る分についてしか成り立たず、残りは `cargo clean` でもワークツリーの削除でも消えない。

## Current State

例外表は 19 行（2026-11-01 の数え）。理由の区分は 3 つ:

- ⒜ プロセス識別子で一意化済み（16 件）: `areka-emo-present/src/balloon_test_support.rs`・`areka-ghost/src/{shiori_inproc_adapter_tests,shiori_inproc_tests}.rs`・`areka-ghost/tests/ghost/{inproc_e2e_test,spine_e2e_test_s7_second_boot_record_present}.rs`・`areka/src/placement/measure_tests.rs`・`shiori-host32-helper/src/{main_loopback_tests,shiori_proxy,shiori_proxy_loadu_tests}.rs`・`shiori-host32-host/src/process_host.rs`・`shiori-host32-host/tests/{lifecycle_cyclic_e2e,lifecycle_kill_e2e,shiori_load_e2e,shiori_request_e2e}.rs`・`shiori-host32-testdll-loadu/src/lib.rs`・`areka/src/install/fetch_url.rs`
- ⒝ 読み出しだけ（2 件）: `areka/src/placement/placement_monitor_tests.rs`・`shiori-host32-host/tests/error_paths.rs`
- ⒞ 固定名であることが仕様（1 件）: `areka-ghost/tests/ghost/inproc_fixture.rs`

このうち `shiori_proxy.rs`・`process_host.rs`・`fetch_url.rs`・`shiori-host32-testdll-loadu/src/lib.rs` は本番のファイル（入口の呼び出しは `#[cfg(test)]` の内側）で、dev-dependency の `temp-path-kit` をそのまま引けない。i686 でだけ組まれるファイルもある。

## Desired Outcome

1. ⒜ の各ファイルのテストが作る一時の根が `<ワークツリー>\target\test-roots\` の下に入る（OS の一時フォルダに入らない）。
2. 例外表から移したファイルの行が消え、見張りの件数の主張が新しい数に揃う。残す行（⒝ の読み出しだけ・⒞ の固定名など）は、残す訳が表に書いてある。
3. テストの主張・期待値・本数は変わらない。本番コードの振る舞いは 1 行も変わらない。

## Approach

要件で決める。候補は 2 つ:

- 案 1: テストの側のファイルは `TempPath`（`new` か `under_target`）へ置き換え、本番のファイルの `#[cfg(test)]` の部分は兄弟のテストのファイルへ出してから置き換える。
- 案 2: 置き換えずに、各ファイルの自前の一時パスの組み立ての根だけを `target\test-roots` に向ける（`CARGO_MANIFEST_DIR` から求める）。

窓口を 1 か所に集める方針（`temp-path-kit` の目的）から、案 1 が本筋の見込み。

## Scope
- **In**: 例外表の ⒜（必要なら ⒝・⒞）のファイルの一時パスの置き場の移動・例外表と件数の主張の更新・本番のファイルからテストの部分を兄弟のファイルへ出すこと。
- **Out**: 窓口 `temp-path-kit` の入口の形の変更・`TempPath::new` の呼び出し 240 か所の書き換え（`test-roots-under-target` で済み）・残ったフォルダの掃除の仕組み・`CARGO_TARGET_DIR` への追随。

## Boundary Candidates
- テストだけのファイル（置き換え）
- 本番のファイルの `#[cfg(test)]` の部分（兄弟のファイルへ出す）
- 見張りの例外表と件数の主張

## Out of Boundary
- lint の解消（`clippy-199-lints` の持ち物）
- host-32 の e2e の振る舞いや時間の変更

## Upstream / Downstream
- **Upstream**: ✅ `test-roots-under-target`（窓口の 2 つの入口が `target\test-roots` に作る・見張りが窓口の中も見る）・✅ `ghost-standard-balloon`（`TempPath::under_target`）
- **Downstream**: なし（例外表が空に近づくほど、新しいテストが `%TEMP%` に入る道が細る）

## Existing Spec Touchpoints
- **Extends**: なし（`test-roots-under-target` は完了・本 spec はその Known Gaps の追跡）
- **Adjacent**: `clippy-199-lints`（例外表のファイルの一部、とくに `shiori-host32-*` と `areka-ghost` のテストと触るファイルが重なる＝起票の理由。直列にする）・`areka-test-binary-stability`（静かな机で測っているあいだは着地させない）

## Constraints
- 1 ファイル 1,000 行（`file_length_guard_test`）。`measure_tests.rs` は 993 行なので、置き換えで行が増えるなら兄弟のファイルへ。
- パスの長さ: `target\test-roots\` の下はワークツリーの根の分だけ長くなる。host-32 の e2e が `LoadLibrary` に渡すパスは 260 字の内に収まるかを静的に数える（`test-roots-under-target` の Implementation Notes のタスク 3.2 の数え方）。
- i686 でだけ組まれるファイルは `tools/test-all.ps1` の i686 の段で確かめる。

## 起票の理由と区分
- **起票の理由**: 進行中の `clippy-199-lints` と触るファイルが重なる（`kiro-complete` 冒頭ステップ手順 3 の 2 番目の条件）。規模も十数ファイル・本番のファイルの分割を含み、このブランチの数時間では終わらない。
- **優先度の区分**: C（開発の決まり「一時フォルダは `target\` の下だけ」による持ち越し）。
- **要件定義のモデル**: Opus。
- **着手の目安**: `clippy-199-lints` の着地の後。
