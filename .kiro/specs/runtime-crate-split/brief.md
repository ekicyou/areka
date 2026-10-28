# Brief: runtime-crate-split

> 2026-10-28 `/kiro-discovery` で起票。開発者「areka 本体 crate (crates/areka) の肥大を解消するため、`placement` と `emo2_boot` を独立 crate へ分離したい」。6 本のうちの 3 本目。分け方と共通の約束は roadmap の「テーマ別の決めごと」の「本体の crate 分け」が正本。数は main `d6054e4d` の実測で、`runtime-module-untangle` の後の形は見込み＝着手のときに測り直す。

## Problem

`runtime-module-untangle` で、本体の中の `runtime` モジュールは本体の外の部分を参照しなくなる。それでもまだ本体の中にあるので、本体と一緒に組み直され、テストも本体の 1 つの実行ファイルに入ったままになる。

## Current State

- `runtime-module-untangle` の後の `crates/areka/src/runtime/` の見込みは、本番 約 12,000 行・テスト 約 45,000 行。前の spec で session と install・update・readme へ移す 10 ファイル（本番 約 3,300 行）を除いた数。
- 外の crate の使われ方（今の emo2_boot・本番 ファイル/回）は次のとおり。
  - tracing 37/102・bevy_ecs 24/42・wintf 21/32・dola 13/34
  - areka-emo-present 15/23・areka-ghost 11/23・areka-sakura 18/21・areka-parsers 12/16
  - areka-emo-text 11/13・areka-emo-compose 10/12・areka-seriko 10/11・areka-kanade 10/10
  - areka-actor 5/6・areka-emo-atlas 4/4・areka-sylphya 4/4・thiserror 4/4・windows 2/4
  - areka-mcp・areka-nar・areka-user-data・areka-talk は 0。
  - テストだけで使うのは log-capture-kit・temp-path-kit・sample-ghost-kit・areka-update（1 ファイル）・shiori-host32-host（`spine` の `ScriptedShioriBackend`）。
- **`target_map`**（scope から表示対象 `TargetId` への写像の正本・std と `areka-emo-present::TargetId`・`areka-sakura::ActorKey` だけを使う）の使い手は次のとおり。
  - 本番: emo2_boot の中 15 ファイル、input の `user_break.rs:22`、mcp の `check_script.rs`・`dump_balloon.rs`・`dump_surface.rs`。
  - テスト: mcp の GPU のテスト 2 本、placement の `transition_judge_tests`、`shell_balloon_switch_session_*_tests` 3 本。
- **crate の外のテストが使う支え**（今の数）は次のとおり。
  - `ghost_switch_test_support`: 34 ファイル・45 回。前の spec で session へ移る。
  - `spine`: 28 ファイル・46 回。`mcp/dump_surface.rs:415` は本番ファイルの中の `#[cfg(test)]` から `spin_wait_until` を呼ぶ。
  - `spine::hold_support`: 2。`sample_test_support`: 2 ファイル・8 回。
  - 逆向きに emo2_boot のテストが外の支えを使う所もある。`placement::test_support` 24 回・`shared_test_support` 2・`scope_bind_fixture_test_support` 2・`install::fetch_url_test_support` 1。
- **本体に残る `transition_judge*`**（`placement-crate-split` で placement から本体のテスト専用へ移した判定器）は、emo2_boot のテスト 2 本と自分のテストが使う。
- **境をまたぐテスト**: placement の `reset_balloon_pos_tests.rs` は `switch_assets` を使う。`placement-crate-split` で本体に残したもの。
- **examples**: `collision-probe.rs` が `#[path]` で `target_map.rs`・`hit_region.rs` を取り込んでいる。
- **ログの出所**: `runtime-module-untangle` の後は `areka::runtime::…`。移すと `areka_runtime::…` に変わる。

## Desired Outcome

- `crates/areka-runtime/`（lib・`publish = false`）が runtime の本番とテストを持ち、`areka-placement` に依存する。`crates/areka` は path 依存でそれを使う。
- `target_map` と、テスト専用の `transition_judge*` が `areka-runtime` にある。
- 本体の中で runtime を名指しする道は、全部 `areka_runtime::` になっている。
- 振る舞いは何も変わらない（「検証」）。

## Approach

- 新しい crate を作り、`git mv` で履歴を残して移す。`runtime/mod.rs` の中身は `lib.rs` になる。外から使われる `pub(crate)` だけを `pub` にする。
  - 主な面: `RuntimeWiring` とその `pub(crate) fn` 約 10 本（`runtime()`・`presenter()` ほか）・`frame` の `ZOrderAbsentReports`・`run_ghost_quit_phase`・`TalkLifecycleSignal`・`NoUserBreakSignal`・`HitRegion`・`resolve_hit_region`・`ConsumerLedger`。
  - 正確な一覧は着手のときに測る。
- **テストの支え**: `spine` などの外のテストが使う支えは、`areka-runtime` の `test-support` 機能で開ける。`placement-crate-split` と同じやり方にそろえ、写しを作らない。
- **境をまたぐテスト**: `reset_balloon_pos_tests.rs`（placement と runtime を使う）と `transition_judge*` の判定器とそのテストを、`areka-runtime` へ移す。
- **examples**: `collision-probe.rs` は `areka-runtime` の examples へ移し、`#[path]` をやめて crate の依存で読む。
- **ログの出所**: `log_history.rs` の決まり、`RUST_LOG` の文字列、`doc/ssp-mcp/log-convention.md`、実機の手順書の `areka::runtime` を `areka_runtime` に直す。

## Scope

- **In**: crate の新設と移動・公開面・本体の参照の書き換え・テストと examples と支えの移し替え・`test-support` 機能・ソースを文字列で読むテストのパス・1,000 行の番人の表（emo2_boot の 1 件ほか）・ログの出所・文書のパス（`doc/COMPAT_ARCHITECTURE.md` 38 件・`doc/ukadoc-coverage/ledger/*.toml` の根拠の場所 約 54 件ほか）・`structure.md` の新しい crate の節・`audit-emo2-boot-*` 3 本の「受け持つファイル」の書き換え。
- **Out**: 名前や依存の付け替え（前の spec で済んでいること。残っていたら前の spec の抜けとして直す）・input と session（後の 3 本）。

## Boundary Candidates

- crate の骨組みと移動。
- 公開面とテストの支え。
- `target_map`・`transition_judge*`・境をまたぐテスト・examples。
- ログの出所と文書。
- 検証。

## Out of Boundary

- 振る舞いの変更。中身の直しは audit の 3 本へ申し送る。

## Upstream / Downstream

- **Upstream**: `runtime-module-untangle`。
- **Downstream**: `input-crate-split`。`audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` はこの spec の着地を待つ。v0.2.0 の作り直しで runtime に触る spec（`dola-interrupt-ticks` → `talk-anchor-interrupt-ticks` → `instant-to-interrupt-ticks` ほか）も、新しい crate の上で行う。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: audit の 3 本（受け持つファイル）。emo2_boot に触ると書いている約 70 本は、roadmap の新旧のパスの対応表で読み替える。

## Constraints

- **検証（要件に入れる）**: roadmap の「本体の crate 分け」の「共通の約束」の検証の項を、そのまま要件に入れる。この spec で回す実機テストは、出所を名前で読む次の 2 つを 1 回ずつ。
  - `tests/smoke_boot_loop_exit.rs`
  - `tests/emo2_real_run`

  `log_history` の分け方（MCP の `get_log` の 5 種別）が前後で同じことも示す。
- **共通の約束**: roadmap の同じ節のとおり。
- **Fable**: Opus で足りる。
