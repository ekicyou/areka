# Brief: areka-P0-clippy-199-lints

> 2026-10-03 `/kiro-discovery` で起票（`areka-P0-mcp-server-core` の `/kiro-complete` の棚卸で回収。セッションのチップ「Fix clippy 1.99 lints across workspace」を spec 化）。file:line は起票時（`claude/areka-p0-mcp-server-core-b1d479` の HEAD）＝着手時に引き直す。

## Problem

開発者と各 spec の実装・レビュー係が `cargo clippy -- -D warnings` を門として使えない。ツールチェーンの clippy が 1.99 に上がって新しい lint が既存のコードに出るようになり、`cargo clippy --workspace --all-targets -- -D warnings` が赤のまま。そのせいで、各 spec の変更が新しい警告を足していないかを clippy で確かめられない（`areka-P0-mcp-server-core` のレビューは `-p areka-mcp` に絞り、`-p areka` は「既存の指摘で赤」と断って通した）。

## Current State

- 実測（2026-10-03・`cargo clippy --workspace --all-targets --keep-going -- -D warnings`）の指摘の場所:
  - `dola`: `validate/rules.rs` 11・`validate/mod.rs` 3・`runtime/timeline_manager/mod.rs` 2・`runtime/loop_controller/tests.rs` 2・`runtime/{subscription_manager,instance_manager}/mod.rs`・`runtime/{conflict_resolver,facade}.rs`・`compile/resolve.rs:284`（`too_many_arguments`）各 1
  - `areka-emo-compose`: `plan.rs:363`（`collapsible_if`）・`composer_tests.rs` 2・`golden_tests_test_support.rs`・`golden_tests_surface1000_bind_tests.rs`・`blit_transparent_alpha_tests.rs`
  - `areka-kanade`: `shiori/real.rs`・`schedule/user_break_tests.rs`・`actor_raise_reply_tests.rs`
  - `shiori-host32-host`: `tests/lifecycle_cyclic_e2e.rs` 4・`tests/lifecycle_kill_e2e.rs` 2
  - `shiori-abi`: `ergonomic.rs`
  - `areka`（上流が赤で止まるため上の実測には出ない。`--no-deps` の別の実測で約 19）: `app_exit.rs`・`boot_config.rs`・`emo2_boot/*`（`frame/wiring.rs:168` ほか）・`ghost_session.rs`・`input_events/balloon.rs`・`placement/*`（`config.rs:165`・`follow/window_move.rs:452`・`spawn.rs:719/732`・`persist.rs:287`・`transition_diag.rs:768`）・`update/procedure.rs`
- `tools/test-all.ps1` に clippy の段は無い（全体の門は fmt と全テスト）。だから赤でも誰も止まらない。
- 新しいクレート `areka-mcp` は `-D warnings` で緑（指摘 0）。

## Desired Outcome

- `cargo clippy --workspace --all-targets -- -D warnings` が緑。
- 振る舞いは変わらない（全体テストが緑のまま・テストの期待値の書き換え 0）。
- `too_many_arguments` のように意図して残す形は、`#[allow]` ではなく `#[expect(clippy::…, reason = "…")]` で理由を書く（使われなくなれば警告で外し忘れに気付ける）。

## Approach

lint ごとに clippy の提案どおりの機械的な直しを当てる（`collapsible_if` は条件の合流・`needless_*` は削る、など）。直すと読みにくくなる・API の形が変わるものだけ `#[expect(reason)]` で残す。クレートごとにまとめて 1 コミットずつ。

## Scope

- **In**: 上の場所の lint の解消（本番コードとテスト）。`#[expect]` を置く場合の理由の文。
- **Out**: 振る舞いの変更・リファクタ（lint を消す以上の書き換え）・公開 API の形の変更・依存の追加・ツールチェーンの版の固定や上げ下げ。

## Boundary Candidates

- クレート単位（dola／areka-emo-compose／areka-kanade／shiori 系／areka）。並走中の spec と触るファイルが重なるクレートは後回しにして、重ならないクレートから順に直せる。

## Out of Boundary

- `tools/test-all.ps1` へ clippy の段を足すこと（下の議題。足すなら別の判断で）。
- `vendors/` 配下（別リポジトリ）。

## Upstream / Downstream

- **Upstream**: なし（ツールチェーンの clippy 1.99）。
- **Downstream**: 以後のすべての spec の実装・レビューが `-D warnings` を門に使える。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka/src/placement/*`・`emo2_boot/*`・`ghost_session.rs`・`areka-kanade` に触る並走中の spec（roadmap の「直列の列」）と重なりうる。着手のときに台帳の「並び」で空き席を選び、重なるクレートは列が空くまで待つ。直接修正候補 2（`placement/` の `#[allow(dead_code)]` の掃除）と同じファイルに触るので、まとめて片付けてよいかは着手時に決める。

## Constraints

- 1 ファイル 1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）を超えない。
- 検証は `pwsh -NoProfile -File tools/test-all.ps1`（fmt＋全テスト）と `cargo clippy --workspace --all-targets -- -D warnings`。
- **議題（要件の段）**: `tools/test-all.ps1` に clippy の段（`-D warnings`）を足すか。足せば今後の退行を止められるが、ツールチェーンが上がるたびに既存のコードで赤になり、関係の無い spec の完了が止まる。
