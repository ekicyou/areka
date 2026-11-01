# Brief: clippy-199-lints

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

## `areka-P0-animated-image-playback` からの申し送り（2026-10-07・完了時の棚卸）

- 実装中（2026-10-06）に `cargo clippy --all-targets -- -D warnings` で、上の列挙の外の赤を 2 件見た: `areka-seriko` の `actor.rs` の `large_enum_variant` と `looper.rs` の `collapsible_if`（どちらも本 spec の前から）。
- 列挙済みの赤も 10-06 時点で残っている: `areka-emo-compose` のテスト 5 か所の `chunks_exact`（`blit_transparent_alpha_tests.rs`・`golden_tests_surface1000_bind_tests.rs`・`golden_tests_test_support.rs`・`composer_tests.rs` ×2）と `dola` の 21 件。
- 本 spec は `areka-seriko` の `table.rs`・`looper.rs`・`parts.rs`・`timeline.rs`・`state.rs`・`actor.rs` を触ったので、着手のときに行の位置を引き直す。

## 2026-10-23 `shiori4-api` からの申し送り

- 開発者の決定（10-23「COM-SHIORI インターフェースの作りかけは排除し、shiori4 に完全移行」）で、`crates/shiori-abi/`（COM の面）と `crates/shiori4-testdll/` は `shiori4-api` が消す。この spec が着手するときに、まだ残っていれば受け持ちから外してよい（直しても消えるファイル）。

## 件 1（2026-10-31 `animated-surface-log-flood` の申し送り）: emo-compose のテストの `chunks_exact` が今も残り、emo-present のテストにも 7 件ある

> 出どころ: `animated-surface-log-flood` の実装中の clippy（rustc・clippy 1.99.1）。どれも同 spec の前からの指摘で、同 spec は自分が変えた所の指摘だけを直した。2026-10-31 に `cargo clippy -p <クレート> --no-deps --all-targets` で測り直した（worktree の HEAD `c7b1fdb47`）。

- **`areka-emo-compose` のテスト 5 か所（列挙済み・今も残る）**: `chunks_exact` を定数の幅で使う（`chunks_exact_to_as_chunks`）。`blit_transparent_alpha_tests.rs:60`・`golden_tests_surface1000_bind_tests.rs:89`・`golden_tests_test_support.rs:168`・`composer_tests.rs:841`・`composer_tests.rs:893`。上の 10-07 の申し送りの 5 か所と同じ（行は動いた）。`-p areka-emo-compose --no-deps --all-targets` の指摘はこの 5 件だけ（`plan.rs` の `collapsible_if` は出ない）。
- **`areka-emo-present` のテスト 7 件（列挙の外・新しく見つけた）**: `--no-deps --all-targets` の指摘は全部テストのファイルで、本番のコードは 0 件。
  - `chunks_exact_to_as_chunks` 4 件: `shell_target_template_tests.rs:153`・`presenter_budget_steady_state_tests.rs:230`・`presenter_budget_equivalence_tests.rs:228`・`presenter_budget_equivalence_tests.rs:341`。
  - `type_complexity` 2 件: `display_tests.rs:63`・`balloon_target_tests.rs:378`。
  - `assertions_on_constants` 1 件: `presenter_budget_equivalence_tests.rs:533`。
  - 実装中に見た `hub.rs:269` の `collapsible_if` は、同 spec のタスク 3.6 の後は出ない（10-31 の測りで 0）。
- **`--no-deps` を付けないと**、`-p areka-emo-present -- -D warnings` は依存の `dola` の既存の 21 件（`collapsible_if` ほか）で止まる。上の列挙の段 1（`dola` の `validate/`・`runtime/`・`compile/`）と同じもの＝新しい起票はしない。
- **触るファイル**: 本 spec の段 2 に `crates/areka-emo-present/src/` のテスト 5 本（`balloon_target_tests.rs`・`display_tests.rs`・`presenter_budget_steady_state_tests.rs`・`presenter_budget_equivalence_tests.rs`・`shell_target_template_tests.rs`）が加わる。クレートの数は 9（`areka-emo-present` を足す）。emo-present の `presenter/` を触るほかの spec とは、テストのファイルが重ならないか着手のときに照らす。
- **規模**: +1 タスク（emo-present のテスト 5 本・機械的な直し）。

## 2026-11-01 棚卸㉝の再測定（main 13b80ab1）
`cargo clippy` は走らせていない（共通の決まり）。git の履歴からの数え。
- **前提の変化**: 段 1 のファイルと `areka-sakura` の 2 ファイル（`src/drive.rs` 655・`src/compile_arm_tests.rs` 928）は棚卸㉜から動いていない（d374f901 からの差分で 0）。✅ `shell-scale-linkage` が着地し、v0.1.0 の残りのウェーブの spec は `balloon-communicatebox` になった。✅ `test-roots-under-target`（11-01）は入口の差し替えだけで着地し、一時の根の例外表の直しは新しい `temp-path-exceptions-to-target`（11-01 起票・「`clippy-199-lints` の着地の後」）へ回った。
- **触るファイル**: 段 1＝`crates/dola/src/validate/{mod,rules}.rs`・`compile/resolve.rs`・`runtime/{conflict_resolver,facade}.rs`・`runtime/{timeline_manager,subscription_manager,instance_manager}/mod.rs`・`runtime/loop_controller/tests.rs`・`crates/shiori-host32-host/tests/lifecycle_{cyclic,kill}_e2e.rs`（段 1 に入れるなら `areka-sakura` の上の 2 ファイル）。段 2＝棚卸㉜のまま（`areka-emo-compose`・`areka-emo-present` のテスト 5 本・`areka-seriko`・`areka-emo-text`・`areka-kanade`・`areka-sakura`・`areka` と `shiori-host32-host/src/`）。
- **規模**: 丸ごと 1 本なら 14〜20、段 1 だけなら 3〜5、`areka-sakura` を入れて 4〜6。最初のタスクで測り直す。
- **先に要るもの**: 働きの上では無し。人や外の待ち・静かな机・長い走行・前後の全体テストの比べ・依存の追加・1 本だけの席は要らない。
- **`balloon-communicatebox` との重なり**: 丸ごと 1 本は重なりうる（段 2 の `areka` の `emo2_boot/*`・`consumer_ledger*.rs` は誰も workspace の clippy に掛けていない）。段 1（`areka-sakura` を入れても）は 0。
- **`temp-path-exceptions-to-target` との重なり**: 段 1 の `shiori-host32-host/tests/lifecycle_{cyclic,kill}_e2e.rs` は向こうの例外表のファイル＝段 1 を切っても同じウェーブに置けない。向こうの待ちはファイルの重なりだけで、働きの上の依存ではない（順はどちらが先でもよい・向こうの後に続く spec は無い）。
- **優先度の区分**: C のまま。
- **要件定義のモデル**: Opus のまま。
- **分割の案**: 段 1 を別の 1 本（たとえば `clippy-dola-host32-tests`）に切れば、v0.1.0 のウェーブの空き席に入れられる（棚卸㉛で開発者に尋ねた・答え待ち）。`areka-sakura` の 2 ファイルは `range-choice-tag`（`compile.rs`）・`talk-fast-forward`（`drive.rs`）と同じウェーブでなければ段 1 に入れてよい。残りは v0.1.0 の後。
- **見つけた穴・古くなった記述**: 本文の「`areka` 約 19」は古い（実数は数倍の見込み・棚卸㉓）。`shiori-abi` は `shiori4-api`（未完了）が消すので受け持たない。v0.1.0 の後の見直し `audit-dola`・`audit-shiori-host32`・`audit-parsers-sakura` が段 1 と `areka-sakura` の分を持つ＝先に済ませれば見直しの差分が小さくなる。
