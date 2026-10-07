# Brief: areka-P0-dev-helper-x64-clobber

> 2026-10-07 `ghost-standard-balloon` の完了時の棚卸で起票（`completed/areka-P0-ghost-standard-balloon/tasks.md` の Implementation Notes の 6.2・最終検証 ⑶）。

## Problem

全体テスト（`tools/test-all.ps1`）を回した後に `target\debug\areka.exe` で 32bit の SHIORI のゴースト（例: claudia の `yaya.dll`）を起こすと、SHIORI を読めずに落ちる（`0x800700C1`＝形式が違う）。開発者や実機の確かめをするセッションが毎回この罠を踏む。

## Current State

- areka は `areka.exe` の隣の `shiori-host32-helper.exe` を 32bit の SHIORI の受け口として起こす。
- 全体テストの x64 の段（`cargo test --workspace`）は、ワークスペースの全部の bin を x64 で作るので、`target\debug\shiori-host32-helper.exe` が **x64 版**になる。i686 版は `target\i686-pc-windows-msvc\debug\` にしか無い。
- 煙テストなどは自分で i686 の helper を `areka.exe` の隣へ揃える（`crates/areka/tests/smoke_boot_loop_exit.rs` の `ensure_i686_artifact`・`mcp_get_log_real_run.rs`）。手で起こす実機には揃える仕組みが無い。
- 2026-10-06 の実機の確かめでは、`areka.exe` と i686 の helper を同じフォルダへ写して避けた。

## Desired Outcome

- 全体テストの後でも、開発者が `target\debug\areka.exe`（または決まった 1 本の手順）で 32bit の SHIORI のゴーストを起こせる。
- helper の向きが違うときは、落ちる前に「helper が x64 版で 32bit の SHIORI を読めない」と分かる記録が 1 行出る（ログ無しの失敗にしない）。

## Approach

要件で次のどれかを選ぶ。⑴ x64 のワークスペースのビルドが `target\debug\shiori-host32-helper.exe` を作らないようにする（bin の置き場や名前を分ける）。⑵ 全体テストの最後の段で i686 の helper を `target\debug\` の隣へ戻す。⑶ areka が helper の向きを起動前に確かめ、違えば `target\i686-pc-windows-msvc\<profile>\` を探すか、はっきりした記録を出す（配布物では隣が常に i686 なので、開発時だけの探し方にする）。

## Scope
- **In**: 開発時の helper の置き場の罠の解消・向きが違うときの記録。
- **Out**: 配布物の helper の置き方（`tools/package.ps1`・すでに i686）・arm64 の helper。

## Boundary Candidates
- 全体テストのスクリプト（`tools/test-all.ps1`）
- helper を起こす所（`areka-kanade` の host-32 の起動）

## Out of Boundary
- 32bit の SHIORI の受け口の仕組みそのもの

## Upstream / Downstream
- **Upstream**: フルテストの速度調整（PR#258・spec なし・helper の檻を i686 の段だけで走らせた）
- **Downstream**: 実機の確かめをするすべての spec

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `dump-balloon-debug-timeout`（実機で 32bit のゴーストを起こす）・`release-code-signing`（配布物の helper）

## Constraints
- shiori-host32 は後付け・kanade の改変は疎結合化の方向でのみ（host32 のための特別扱いは最後の手段）。
- 規模の見立て: S（3〜6）。
