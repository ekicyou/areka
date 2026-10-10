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

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: フルテストの速度調整（PR#258）は、helper の約 42 秒の檻を i686 の段だけで走らせる形にした（`crates/shiori-host32-helper/src/main.rs` の檻の `cfg`）。上書きそのものは直っていない＝全体テストの x64 の段は今も `cargo test --workspace`（`tools/test-all.ps1`）で、helper の実行ファイルを x64 で作る。ただしこの変更で、x64 の段が helper を作る理由はほぼ無くなった（残る単体テストは i686 の段と同じもの）＝案 ⑴ が「x64 の段から helper を外す」の 1 行で取れる見込みになった。外すと `target\debug\` に helper が無くなるので、案 ⑵（i686 版を隣へ置く）と組にする。
- **触るファイル**: 案 ⑴⑵ なら `tools/test-all.ps1`（70 行）だけ。案 ⑶（向きを確かめて記録を出す）なら、helper を起こす所 `crates/shiori-host32-host/src/process_host.rs`（632 行）か、helper の置き場を決める関数 `crates/areka/src/boot_config.rs`（503 行）と兄弟のテスト。新しいファイルは無い見込み。
- **規模**: S（3〜5）。
- **先に要るもの**: なし。ファイルの重なり: `boot_config.rs` は `shell-companion-balloon`・`ghost-inner-balloon`・`baseware-root-list`・`makoto-dll-host`・`shiori4-api` の brief が挙げる＝案 ⑶ の記録は `process_host.rs` の側に置けば重なりが `makoto-dll-host` だけになる。`tools/test-all.ps1` は `clippy-199-lints`（clippy の段を足すと決めたとき）と同じファイル。
- **測定の仕事ではない**。確かめはビルドと、手で 1 回起こすことだけ（重い回 0）。
- **優先度の区分**: C（開発の手順の罠・製品は壊れない）。
- **要件定義のモデル**: Opus。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - brief の「helper を起こす所（`areka-kanade` の host-32 の起動）」は場所が違う。置き場を決めるのは `areka` の `boot_config.rs`、起こすのは `shiori-host32-host` の `process_host.rs`。
  - 煙テスト（`crates/areka/tests/smoke_boot_loop_exit.rs` の、i686 の成果物を揃える関数）は、全体テストの x64 の段の中で i686 版を `areka.exe` の隣へ写す。だから「全体テストの後は x64 版」になる道筋は自明でない（写した版の更新時刻が古く、次の x64 のビルドが作り直す、という読みは確かめていない）。最初のタスクで、どの操作の後に x64 版へ戻るかをビルドだけで確かめる。
  - 本番の `areka` には、helper の場所を外から指す口が無い（テストの側だけ `HOST32_HELPER_EXE` を読む）。
