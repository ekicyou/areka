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


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

`cargo clippy` は走らせていない（共通の決まり）。下は git の履歴からの推定で、着手の最初のタスクで測り直す。

- 規模: S〜M（8〜12 タスク）。切る: なし（20 を超えない）。ただし着手の順を 2 段に分ける案（下）。
- 前提の状態: 上流なし。brief の実測は `mcp-server-core` の枝（main の `55067b63`＝PR#219 の時点）＝`emo-text-file-split`（PR#217）の分割の**後**なので、分割で動いたファイルは brief の列挙に影響しない。
- 崩れた前提／古くなった位置（`55067b63..634032f6` の差分から）:
  - **列挙のファイルはすべて今も在る**（移動・改名 0）。
  - 列挙のうち、その後に書き換わったもの: `areka-kanade` の `shiori/real.rs`・`actor_raise_reply_tests.rs`（どちらも `translate-pipeline`）、`areka` の `emo2_boot/frame/wiring.rs`（`status-execution-states`）・`ghost_session.rs`（`mcp-tool-entrances`・`translate-pipeline`・`shell-balloon`）・`input_events/balloon.rs`（`shell-balloon`）。指摘の行は動いている見込み。
  - 列挙のうち動いていないもの: `dola` の全件（`dola` の変更は `cue/sink.rs` と README だけ）・`areka-emo-compose` の `plan.rs` とテスト 4 本・`shiori-host32-host` の `tests/` 2 本・`shiori-abi` の `ergonomic.rs`・`areka` の `app_exit.rs`・`boot_config.rs`・`placement/*`・`update/procedure.rs`。
  - **新しい指摘が増えている見込み**: C2 で約 2.3 万行が入った（新しいファイル: kanade の `translate.rs`・`schedule/translate.rs`・`online.rs`・`actor_translate.rs`、`areka-emo-compose` の `boxes.rs`、`areka-emo-text` の `actor_box.rs`・`place.rs`・`state_route.rs`・`surface_window_child.rs`（brief の列挙に `areka-emo-text` は無かった）、`areka-parsers` の `shell/boxes.rs`、`areka-ghost` の `translate_wiring.rs`、areka の `emo2_boot/shell_box_assets.rs`・`balloon_visibility_*`・`input_events/shell_box*.rs`・`src/mcp/`、`areka-mcp` の `tools/`）。各 spec のレビューは clippy を門にしていないので、0 とは言えない。
- 触るファイル（並走の照合用・クレート単位）:
  - 段 1（いつでも空き席に入れられる）: `crates/dola/src/{validate,runtime,compile}/**`・`crates/shiori-abi/src/ergonomic.rs`・`crates/shiori-host32-host/tests/lifecycle_{cyclic,kill}_e2e.rs`・`crates/areka-mcp/`（指摘が出れば）。今の列と C3 の 9 本のどれも触らない。`dola` は crates.io へ出すクレートだが、`release-cycle` は版の行しか触らないので重ならない（`too_many_arguments` を `#[expect]` で残す `compile/resolve.rs` は公開の API の形を変えない）。
  - 段 2（列が空いてから）: `areka-emo-compose`（C3-③ `surface-element-nesting` が `plan.rs` を触る＝brief の `plan.rs` の `collapsible_if` と同じファイル）・`areka-kanade`（C2-⑦ `mouse-drag-events`・C3-④ `balloon-lifecycle-events` は `schedule/{steady,events}.rs`・`msg.rs` で、列挙の `shiori/real.rs` とは別のファイルだが、新しい指摘がどこに出るかは測るまで分からない）・`areka`（`emo2_boot/`・`input_events/` は C2-⑦・C3-②④ が、`boot_config.rs` は C3-⑥ `ghost-standard-balloon` が触る。`placement/*` は今どの列も持っていない）。
- 議題（答えで作業が変わるものだけ）:
  1. （既存）`tools/test-all.ps1` に clippy の段を足すか。
  2. **2 段に分けて出すか**。⒜ 段 1 だけを 1 本目の PR として C3 の空き席で出し、段 2 は C3 の着地の後に同じ spec の 2 本目の PR で出す（1 spec 1 PR の決まりから外れる）⒝ spec 全体を C3 の着地の後まで待たせる（`areka` の `placement/*` と段 1 だけなら今でも重ならない）。1 本で出すなら ⒝。
- 見つけた穴: なし。
