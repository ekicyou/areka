# Brief: areka-P0-emo2-real-run-wrap-timeout

> 起票: 2026-10-08（`areka-P0-wintf-tooltip` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。実機で赤を見た。原因の見立ては記録の時刻から立てたもので、まだ確かめていない。

## Problem

明示したときだけ走る結合テスト `crates/areka/tests/emo2_real_run.rs`（`AREKA_EMO2_REAL_RUN=1`）が、折り返しの解決の証跡 `wrap=BudouxWordWrap` を待つ所で赤になる。areka.exe を emo2 で実際に起動して確かめる唯一の自動の通しが、手元で使えない。

## Current State

2026-10-06〜08（`wintf-tooltip` のブランチ・タスク 1.1 の確かめ）:

- テストは areka.exe を `AREKA_APP_SMOKE_EXIT_MS=3000`（3 秒で自動終了）で起動し、ログに wire の成立・取り付けの完了・`wrap=BudouxWordWrap` が出ることを確かめる。
- 192 DPI（200%）の画面で debug ビルドを走らせると、文字の取り付けまでに起動から約 2.5 秒かかり、3 秒の自動終了に間に合わず `wrap=BudouxWordWrap` が出る前に終わる見込み。
- `wintf-tooltip` が areka.exe に足したマニフェスト（comctl32 の版 6）を外しても同じく赤＝この spec より前からの問題。
- 常時のテスト（`tools/test-all.ps1`）には入っていないので、全体テストは緑のまま。

## Desired Outcome

`AREKA_EMO2_REAL_RUN=1` の通しが、手元の 200% の画面の debug ビルドでも安定して緑になる。待つ時間を「自動終了までの残り」に頼らず、観測（ログに印が出たこと）で待つ。

## Approach

まず起動から取り付けまでの時間を測り、遅いのが取り付けそのものか、テストの待ち方かを切り分ける。自動終了を延ばすだけで済ませず、印が出てから閉じる形（または印が出るまで待つ有界の待ち）にする。

## Scope

- **In**: 赤の再現と時間の測り・テストの待ち方の直し（または取り付けの遅さの原因の直し）・直した後に 200% と 100% 相当で緑の確かめ。
- **Out**: 折り返しそのものの振る舞い（`budoux-reveal-reflow` 系）・常時のテストへの組み込み（要るなら議題）。

## Boundary Candidates

- テストの待ち方（`emo2_real_run.rs`）／起動から文字の取り付けまでの時間（`emo2_boot` の装着の段）。

## Out of Boundary

- emo2 の台本・pasta の SHIORI の振る舞い。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: emo2 で通しを確かめる実機の確認すべて。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-shiori4-test-ghost`（emo2 の検体と `emo2_real_run.rs`）。
- **Adjacent**: `ghost-session-test-load-flake`（壁時計の締切を観測の待ちへ置き換える方針が同じ）。

## Constraints

- 実機の一時フォルダはワークツリーの `target\` の下だけ。emo2 の実走は絶対パスかつ短いパス。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 起票（10-08）の後、関係のファイルは変わっていない。検査 `crates/areka/tests/emo2_real_run.rs`（247 行）は自動終了 3,000 ミリ秒・見張りの締切 120 秒のまま。待っている証跡 `wrap=BudouxWordWrap` は、文字の面を初めて装着したときの記録（`crates/areka-emo-text/src/actor_present.rs` の「テキスト供給面を予約スロットへ装着した」）に今も出る。`budoux-reveal-reflow` はこの記録を変えていない。
- **触るファイル**:
  - `crates/areka/tests/emo2_real_run.rs` だけで済ませるのが第一。
  - アプリの側で「印が出てから閉じる」を作るなら、自動終了の仕掛け（`crates/areka/src/main.rs`・950 行で上限の近く）に触る。`main.rs` は `mcp-kanade-tools`・`mcp-stdio-bridge` なども触る。
  - 遅さの原因が装着の側にあった場合だけ `crates/areka/src/emo2_boot/` の装着の段。
- **規模**: S（3〜6 タスク）。
- **先に要るもの**: 無い。
- **優先度の区分**: B（バグ。areka.exe を emo2 で起動して確かめる唯一の自動の通しが、手元で赤）。
- **要件定義のモデル**: Opus。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - 検査の側だけで直すなら、ほかの未完了の spec と触るファイルの重なりは 0（このファイルに触ると書く brief は本 spec だけ）＝どのウェーブにも置ける。`anchor-tag-canon` とも並べられる。
  - 実機の窓が出る検査なので、負荷を掛ける検査（`areka-test-threads-av`・`test-wait-marker-gaps`）と同じ時間には回さない。
  - 原因の見立て（起動から装着まで約 2.5 秒）はまだ測っていない＝最初のタスクで測る。自動終了を延ばすだけの直しは採らない。
