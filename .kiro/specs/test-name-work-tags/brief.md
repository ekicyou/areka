# Brief: test-name-work-tags

> 2026-11-22 `audit-areka-input-menu` の完了の棚卸（`/kiro-complete`）で起票。出どころ＝同 spec のかけ直しの audit の指摘（research.md 12.3 の 3 行目「`spawn_assembly_tests.rs` が `t_i4_…` のテストの名を指す」）。その場で直さなかった訳＝規模が spec 1 本に当たる（3 つの crate の 29 ファイル・284 本のテストの名と、それを名指す文書と brief に及ぶ）。

## Problem

開発者と、テストを読み書きするセッションが困っている。テストの関数の名の頭に、設計の作業文書のテストの番号の札（`t_i4_`・`t_c3_`・`t_l2_`・`t_rs01_` など）が残っている。札の意味は spec の設計にしか無く、spec は完了でアーカイブされ、文書は流れていくので、名を読んでも札の意味は分からない。CLAUDE.md の `## Comments` は、spec・要件・タスクの番号をコードに書かない決まりで、名にも同じ理由が当たる。新しく足すテストが周りの名に合わせて札を写し、札は増え続けている。

## Current State

- 数え（2026-11-22・main 取り込み後・`git grep -E "fn t_[a-z]+[0-9]+[a-z]?_" -- crates`）: **29 ファイル・284 本**。
  - `areka-placement` 20 ファイル: `zorder_group_ledger_tests.rs` 23・`zorder_group_ledger_state_tests.rs` 21・`config.rs` 20・`zorder_chain_compose_tests.rs` 17・`resolver_resolve_tests.rs` 15・`reseed_tests.rs` 14・`resolver_balloon_keyword_tests.rs` 12・`zorder_group_ledger_written_order_tests.rs` 10・`zorder_group_ledger_rebase_tests.rs` 9・`balloon_limit_tests.rs` 9・`zorder_property_deferral_tests.rs` 8・`spawn_cleanup_tests.rs` 6・`resolver_resolve_free_tests.rs` 6・`resolver_resolve_balloon_tests.rs` 6・`zorder_chain_compose_written_order_tests.rs` 5・`spawn_follow_pipeline_tests.rs` 3・`spawn_clickthrough_tests.rs` 3・`resolver_union_tests.rs` 3・`source_tests.rs` 2・`config_limit_tests.rs` 2
  - `areka-runtime` 8 ファイル: `frame/zorder_descript_tests.rs` 19・`zorder_cue_tests.rs` 16・`frame/zorder_drain_projection_tests.rs` 14・`window_state_cue_tests.rs` 13・`frame/zorder_drain_tests.rs` 11・`frame/zorder_drain_written_order_tests.rs` 8・`zorder_wiring_tests.rs` 5・`frame_schedule_tests.rs` 2
  - `crates/areka` 1 ファイル: `runtime_tests/zorder_wiring_tests.rs` 2
- 名を名指す側（コードの外）: `doc/COMPAT_ARCHITECTURE.md` §8 の判定の欄（`t_c3_char_n_prefix_does_not_add_scope_n`・`t_l2_build_placement_config_keeps_canonical_defaults_for_all_scopes`・`t_rs01_new_offset_and_alignment_reach_the_char_window_parts` ほか）、未完了の spec の brief（`extra-character-windows/brief.md:133`・`zorder-property/brief.md`）。コードの中: `areka-placement/src/spawn_assembly_tests.rs:154` の doc コメント（`t_i4_char_move_follows_adjusted_offset_after_balloon_solo_drag`）ほか。着手のときに `\bt_[a-z]+[0-9]+[a-z]?_[a-z]` で数え直す。
- 札を名に付ける決まりはどこにも書かれていない（steering・CLAUDE.md・スキルに無い）。

## Desired Outcome

- `crates/` のテストの関数の名に作業番号の札が 0 本（上の grep で 0）。
- 名を変えたテストの名指し（コードのコメント・`doc/`・steering・未完了の spec の文書）が新しい名を指す。古い名が残るのは歴史の場所（`.kiro/specs/completed/`・`doc/archive/`・`.kiro/steering/roadmap-history.md`）だけ。
- テストの本文・期待値・数は変わらない（全体テストが緑のまま）。

## Approach

札を外すだけにする（`t_c3_char_n_prefix_does_not_add_scope_n` → `char_n_prefix_does_not_add_scope_n`）。同じモジュールの中で札を外すと名がぶつかるときだけ、名の後ろに中身を表す語を足す。ファイルごとに名を変え、同じコミットでそのファイルのテストの名指しを直す。最後に札の grep と、古い名の grep（歴史の場所だけが当たる）で確かめる。

## Scope
- **In**: 上の 29 ファイルのテストの名・その名指しの直し（コード・`doc/`・steering・未完了の spec の文書）・最後の検索。
- **Out**: テストの中身や置き場の直し・テストのファイル名の変更・札でない頭（`spine_`・`real_` など意味のある語）・歴史の場所の書き換え。

## Boundary Candidates
- `areka-placement` のテスト（20 ファイル）
- `areka-runtime` のテスト（8 ファイル）と本体の 1 ファイル
- コードの外の名指し（`doc/COMPAT_ARCHITECTURE.md`・brief）

## Out of Boundary
- 札をまた足さない仕組み（照合テストなど）は持たない（要るかは要件の段で決める）。
- `doc/COMPAT_ARCHITECTURE.md` の改名（`doc-compat-architecture-rename` の持ち物）。

## Upstream / Downstream
- **Upstream**: なし。
- **Downstream**: なし（名を名指す後続の spec は、新しい名を読む）。

## Existing Spec Touchpoints
- **Extends**: なし。
- **Adjacent**: `zorder-property`・`zorder-chain-residue`（同じ zorder のテストのファイルに触りうる・着手のときにどちらかが走っていれば後に回す）、`extra-character-windows`（brief が `t_c3_`・`t_l2_`・`t_rs01_` の名を名指す）、`session-module-gather`（本体の `runtime_tests/` を動かす）、`doc-compat-architecture-rename`（同じ `doc/COMPAT_ARCHITECTURE.md` を触る）。

## Constraints
- 振る舞いは変えない（名だけ）。1 つのファイルが 1,000 行を越えないのは今のまま。
- 名の変更は機械的なので、実装は sonnet の初回で足りる見込み。
- **優先度の区分**: A（ponytail の見直しの指摘＝開発者「とにかく指摘事項は全部直す方針です」）。版の段は v0.1.0 の後の隙間。規模は M（ファイルの数で決まる・8〜12 タスクの見込み）。

## 2026-11-22 棚卸㊷の再測定（main `d4db9eb0`）
- **前提の変化**: 無し（`fn t_…` は 29 ファイル・284 本のまま）。`session-module-gather` は本体の `runtime_tests/zorder_wiring_tests.rs` を動かしていない。
- **触るファイル（穴）**: テストの名を文字列で並べる本体の 2 ファイルが起票の一覧に無かった＝`crates/areka/src/placement_tests/zorder_group_branch_coverage_tests.rs`（379 行・50 行が名を持つ）・`zorder_inversion_branch_coverage_tests.rs`（393 行・19 行）。名を変えたら同じコミットで直す。コメントの名指しは `spawn_assembly_tests.rs:154` のほかに `resolver_balloon_keyword_tests.rs:54`・`zorder_group_ledger_state_tests.rs:20`・`zorder_group_ledger_tests.rs:357`・`zorder_property_deferral_tests.rs`（6・20〜29・77・123・126 行）・`crates/areka-runtime/src/frame/zorder_drain_projection_tests.rs:531`。`doc/COMPAT_ARCHITECTURE.md` は 226・354・412 行（§8 の別々の行＝許す重なり）。
- **規模**: 8〜12 タスク（前のまま）。
- **先に要るもの**: 無い。
- **同じウェーブに置けない相手**: `session-crate-split`（本体の `runtime_tests/zorder_wiring_tests.rs`＝向こうは `include_str!` のパスを直し、こちらは 2 本の名を変える）。`dola-cue-test-builder`（向こうが runtime の呼び手まで付け替えるときだけ `crates/areka-runtime/src/{window_state_cue_tests,zorder_cue_tests,zorder_wiring_tests}.rs`）。`dola-interrupt-ticks`（同じ 3 ファイルが `TalkCue` を組む）。C26 の見直し 3 本とは、`runtime_tests/zorder_wiring_tests.rs` が `audit-areka-app` の受け持つ範囲（本体の残り）に入るほかは 0。
- **優先度の区分**: A（見直しの指摘＝開発者「とにかく指摘事項は全部直す方針です」）。
- **要件定義のモデル**: Opus（機械的な名の変更・実装は sonnet の初回で足りる見込み）。
- **分割の案**: 無し。本体の `runtime_tests/zorder_wiring_tests.rs` の 2 本を外して後の spec に渡すかは、ウェーブを組むときの決め。
- **棚卸㊷の決め**: 本体の `runtime_tests/zorder_wiring_tests.rs` の 2 本は範囲から外し、`audit-areka-app` へ渡した（同じウェーブ C25 の `session-crate-split` がこのファイルを直すため）。本体で触るのは `placement_tests/` の 2 ファイルだけ。
