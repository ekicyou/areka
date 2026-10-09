# Brief: areka-P0-actor-thread-log-capture

> 起票: 2026-10-10（`areka-P0-ghost-session-test-load-flake` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票。`completed/areka-P0-ghost-session-test-load-flake/tasks.md` の Implementation Notes の 5.6 と「範囲外の申し送り」⑵）。コードの読みで確かめた。

## Problem

テストの記録の捕捉 `log_capture_kit::capture` は、呼んだスレッドで出た記録しか集めない。`areka-actor` の `spawn_actor` で起こしたスレッド（インストールの背景の `install`・更新の `update`・ゴーストの `kanade`・SHIORI の `shiori` など）が出す記録は、そのスレッドを動かしているテストの捕捉に入らない。そのため、テストが赤になったとき、背景のスレッドが書いた失敗の詳しい記録（どのパスで、OS のどのエラーか）を赤の文言に添えられず、原因の読みが遠回りになる。

## Current State

- `log_capture_kit::capture`（`crates/log-capture-kit/src/capture.rs`）は `tracing::subscriber::with_default` で、呼んだスレッドの既定の受け手だけを窓の間差し替える。crate の説明（`lib.rs`）も「呼出スレッド局所」と書き、別のスレッドの記録は入らない。
- 全スレッドの捕捉 `install_global_capture_all`（`global.rs`）は、プロセスの既定を 1 度だけ据え付けて取り消せず、以後そのテストの実行ファイルの全スレッドで `tracing::enabled!` を真にする。areka のテストの実行ファイルでは使わない決まりにしている（`session_end_deadline_tests.rs` の冒頭の説明）。
- `spawn_actor`（`crates/areka-actor/src/spawn.rs`）は `std::thread::Builder` で素のスレッドを起こし、中で `info_span!("actor")` に入って body を呼ぶだけ。呼んだスレッドの既定の受け手を子へ渡す手は無い。
- 走り始めのフック `install_thread_start_hook` はある。ただし型 `ThreadStartHook` は `fn(&str)`（アクター名だけを受ける・状態を持てない）で、プロセスに 1 度だけ・最初の導入が勝つ。中身は実行体 `areka` の `thread_roles`（スレッド名簿へ役割名を載せる）で、本番は `main` が導入する。areka のテストの実行ファイルでも `thread_roles_tests.rs` が同じ関数を導入するので、テストの捕捉に使える枠は空いていない。テストの捕捉には今は使われていない。
- 起きた所（`ghost-session-test-load-flake`）:
  - タスク 5.6: 上書きの失敗の赤を読むとき、`areka_nar` の確定の失敗（パスと os error 5）は背景のスレッド `install` で出るので、テストの手がかり（`desk_overwrite_tests.rs` の `clues`）に載せられず、`OnInstallFailure` の Reference の語で代わりにした（`clues` の説明に同じことが書いてある）。
  - タスク 3.3: kanade のスレッドで出る締切の error（`change_deadline_exceeded`）が捕捉に映らず、問い合わせの往復で「締切が切れていない」を見る形で代わりに判定した。
  - `session_end_deadline_tests.rs` も、見張りのスレッドの `warn!` を捕まえられないので、解く手の回数で代わりに判定している。

## Desired Outcome

- テストが捕捉の窓の中で動かしたアクターのスレッドの記録が、そのテストの捕捉に入る。赤の文言に背景のスレッドの失敗の記録（パス・OS のエラー）を添えられる。
- 本番の記録（出る行・水準・出先）は変わらない。
- 並べて回るほかのテストの記録は混ざらない。

## Approach

候補を要件で決める。

- `spawn_actor` が、起こす時点の呼んだスレッドの既定の受け手（`tracing` の `dispatcher::get_default`）を取り、子のスレッドの中で既定にする。本番は既定の受け手がプロセスの全体の設定なので出る記録は同じで、テストでは窓の受け手へ届く。確かめること: 窓が閉じた後に子が出した記録の扱い・テストより長く生きるアクター・窓の外で起こされたスレッドは捕まらないこと（`install` のスレッドは窓口が最初の依頼で 1 度だけ起こす＝窓の中で依頼を出すテストでないと引き継げない）・log-capture-kit が説明する発行点の interest の焼き付きと番兵の扱い。
- フックを状態を持てる形へ広げる案は、本番の `thread_roles` と同じ枠を分け合うので、取るなら両方が並べる形にする。
- 引き継ぎをテストだけに効かせるか、本番も同じ道を通すかは要件で決める（本番の出る記録が変わらないことはテストで固定する）。

## Scope

- **In**: 子のスレッドへの受け手の引き継ぎ（または同じ働きの仕組み）・それを固める檻（窓の中で起こしたアクターの記録が入る・窓の外の記録は入らない・本番の形では出る記録が変わらない）・効いていることの対照として `desk_overwrite_tests.rs` の手がかりに背景のスレッドの warn 以上を足す。
- **Out**: 既存のテストの代わりの判定の書き換え（残してよい）・全スレッドの捕捉の窓口の変更・本番のログの出先と文言。

## Boundary Candidates

- `areka-actor` の `spawn_actor`（とフックの形）
- log-capture-kit 側に要る口（あれば）

## Out of Boundary

- 記録の文言・水準の変更
- `thread_roles` の役割名の宣言

## Upstream / Downstream

- **Upstream**: なし（`areka-actor`・`log-capture-kit` は在る）。
- **Downstream**: 背景のスレッドの記録を見たいテストすべて（install・update・kanade・SHIORI の結線のテスト）。`test-wait-marker-gaps` の赤の読み分けも楽になる。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-test-cage-determinism`（log-capture-kit）・完了 `areka-P0-draw-load-parity`（走り始めのフック）。
- **Adjacent**: `areka-actor` に触る spec とは同時に走らせない（2026-10-10 の時点で、brief に `areka-actor` を挙げる起票済みの spec は無い）。`test-wait-marker-gaps`（同じテストを読む）。

## Constraints

- 本番の振る舞いと出る記録を変えない。
- 1 ファイル 1,000 行の番人を守る。
- テストの実行ファイルの全スレッドで `tracing::enabled!` を真にする形（全体の設定）は取らない。
- 規模の見立て: S（3〜6）。
