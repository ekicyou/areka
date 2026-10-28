# Brief: placement-crate-split

> 2026-10-28 `/kiro-discovery` で起票。開発者「areka 本体 crate (crates/areka) の肥大を解消するため、`placement` と `emo2_boot` を独立 crate へ分離したい。対応時期は0.1.0で、dolaとかの対応のタイミングか、ponytail関係のタイミング。」「この際、areka crateのコードベースが大きすぎる問題をサルベージすべきかもしれない」。6 本に分けた（この spec は 1 本目）。分け方・順番・共通の約束・新旧のパスの対応表は roadmap の「テーマ別の決めごと」の「本体の crate 分け」が正本。数は main `d6054e4d` の実測（非空行）＝着手のときに測り直す。

## Problem

`crates/areka` は「他の crate への橋渡しに徹する」本体のはずが、`src` だけで本番 60,532 行・テスト 146,318 行を抱えている。lib を持たない bin crate なので、次の困りごとがある。

- 境目が守られない。どのモジュールも `pub(crate)` で互いの中を触れ、モジュールどうしの参照し合いが 17 組ある。
- どこを 1 行変えても crate 全体を組み直す。テストは 1 つの実行ファイル（`cargo test -p areka --bin areka`）に全部入る。
- 見直し（ponytail の audit）の受け持ちを crate の境目で切れず、パスで割るしかない。

配置（`placement`）はこのうち一番切り出しやすい塊で、最初に出す。

## Current State

- **大きさ**: 本番 32 ファイル 12,932 行（直下 24・`follow/` 8）。テスト 107 ファイル 40,408 行、`#[cfg(test)]` だけの判定器 3 ファイル（`transition_judge.rs`・`transition_judge_offset.rs`・`transition_judge_verdict.rs`）2,467 行、本番ファイルの中のテスト 2 か所（`config.rs`・`windowposition.rs`）533 行。800 行を超える本番ファイルは `follow/window_move.rs` 1,168・`follow/drag_follow.rs` 933・`mod.rs` 929。
- **外への依存は 0**: 本番から本体の他のモジュールへの `crate::` は 0 件。`super::super` は 6 件あるが全部 placement の中で止まる。プロセスの中で共有する状態（`static`・`thread_local!`・`OnceLock`）も本番には 0。
- **外から placement への参照**: 153 ファイル・526 回。
  - emo2_boot: 本番 22 ファイル/96 回、テスト 62/218。
  - input_events: 本番 10/14、テスト 18/42。
  - menu: 本番 3/6。mcp: 本番 1/1。
  - src 直下: 本番 4/40。examples: 4/12。
- **公開し直す面**: 外から使われている `pub(crate)` は、モジュール 11（`balloon_limit`・`balloon_lock`・`descript_window_keys`・`dpi_sync`・`reseed`・`reset_balloon_pos`・`reset_pos`・`script_alignment`・`sticky_group_ledger`・`zorder_chain_compose`・`zorder_group_ledger`）、項目約 28、`follow` の再公開 4、メソッド 1（`GhostTitles::from_scope_titles`）。外から一度も使われない `pub(crate)` は約 31 件。
- **使う外の crate**: wintf・bevy_ecs・tracing・areka-parsers・areka-emo-compose・areka-emo-present・areka-sylphya・areka-emo-atlas・areka-ghost・windows・thiserror。windows の機能は本番で `Win32_Foundation`・`Win32_UI_WindowsAndMessaging`、テストで `Win32_System_Com`・`Win32_System_LibraryLoader`。
- **移すと壊れる所**:
  - 自分を指すパス `crate::placement::` が本番 2 件（`follow/offset_space.rs`）、テスト 68 ファイル・202 件。
  - placement のテストのうち 3 本が外を使う。
    - `reset_balloon_pos_tests.rs` → `emo2_boot::switch_assets`
    - `spawn_assembly_tests.rs` → `input_events::attach_char_pointer_handlers`
    - `transition_judge_tests.rs` → `emo2_boot::target_map`
  - ソースを文字列で読むテストが 6 本ある（`include_str!`・`CARGO_MANIFEST_DIR`）。`measure_template_tests`・`zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`・`spawn_zorder_pair_deferred_tests`・`transition_signoff_procedure_tests`・`zorder_property_deferral_tests`。
  - examples の 2 本（`window-placement.rs`・`collision-probe.rs`）が `#[path = "../src/placement/mod.rs"]` で中身を取り込んでいる。
  - 1,000 行の番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`）に placement のパスが 3 つ、`temp_path_guard_test.rs` に `measure_tests.rs` が 1 つ。
  - ログの出所: 本番のログは既定の出所（モジュールのパス `areka::placement::…`）を使う。直書きは `diag.rs:69` の `"areka::placement::diag"`。名前で読んでいる所は次のとおり。
    - `diag_tests.rs`・`follow_window_move_diag_tests.rs`
    - `crates/areka/tests/window_link_real_run.rs:49`（`RUST_LOG`）
    - `crates/wintf` の `window_pos_tests.rs:14`
    - 語彙として直書きした `"areka::persist::*"` は、モジュールのパスに頼らない。
  - 文書のパス: `doc/` 4 ファイル 53 件（`COMPAT_ARCHITECTURE.md` 19・`doc/ukadoc-coverage/` 3 ファイル）。`tools/perf/fixtures/*/run.log` に `INFO areka::placement:` の行が 42。
- **`transition_judge*`**: `mod.rs` で `#[cfg(test)]` を付けて宣言し、本番からの呼び出しは 0。外で使うのは emo2_boot のテスト 2 本（`frame_transition_atomicity_tests.rs`・`frame_transition_branch_tests.rs`）。自分のテストは `target_map` を使う。
- **テストの支え**: `placement::test_support`（`capture_logs` 38 回・`LogEvent` 28・`ExpectField` 8・`expect_one`）を placement の外の 194 ファイルが使う。中身は `log-capture-kit` の薄い包み。`placement_shared_test_support`（`balloon_root`・`emo2_root`・`emily_shaped_scopes`・`with_com_initialized`・`WA`）も外で使う。
- **依存の向きの規約**（`mod.rs`）: 「`resolver`（純粋・std のみ）← `config`（areka-parsers のみ）← `measure`（emo-atlas/compose）← `spawn`／`follow`（wintf/bevy_ecs）← main.rs シーム。左のモジュールは右へ import しない」「diag は最下流」。実際の参照と食い違う所が 5 件ある。
  - `resolver.rs` が `tracing`・`wintf::ecs::DPI`・`config`・`follow::OffsetBase` を引く。
  - `diag.rs` が `bevy_ecs::Entity`・`wintf::ecs::ZOrderPairStrategy` を引く。
  - `config.rs` が `tracing` を引く。
  - `measure.rs` が `super::PlacementError` を引き、`mod.rs` と参照し合う。
  - `spawn` と `follow` は互いに引き合う（同じ段なので規約の違反ではない）。

## Desired Outcome

- `crates/areka-placement/`（lib・`publish = false`）が placement の本番とテストを持ち、`crates/areka` は path 依存でそれを使う。
- 振る舞いは何も変わらない（下の「検証」がそれを示す）。
- placement の公開面が、外から使われる分だけ `pub` になり、外から使われない分は crate の中に閉じている。
- 依存の向きの規約の文が、新しい crate の中で実際の参照と一致している。

## Approach

- 新しい crate を作り、`git mv` で履歴を残して移す。`mod.rs` の中身は crate の根（`lib.rs`）になる。
- 中の `crate::placement::` は `crate::` へ、本体の `crate::placement::` は `areka_placement::` へ書き換える。
- 外から使われる `pub(crate)` だけを `pub` にする。外から使われない約 31 件は `pub(crate)` のまま残す（消すかどうかは `audit-placement-core` の持ち物）。
- **wintf へは寄せない**。wintf は crates.io へ出している汎用の crate で、placement は ukagaka の意味（descript のキー・スコープ・バルーンの追従・sylphya）を持つ。寄せれば wintf に areka 系の依存が入る。wintf と重なる所（`MonitorSnapshot` が包む `Monitor`・`diag::MonitorRecord` と wintf の `transition_diag::MonitorRecord`・`RectPx`）は `audit-placement-core` で片付ける。
- **`transition_judge*` の置き場**: placement と emo2_boot をまたいで判定するテスト専用の部品なので、上の crate に置く。この spec のあいだは本体に残し（placement のフォルダから本体のテスト専用のモジュールへ移す）、`runtime-crate-split` で `areka-runtime` のテスト専用へ移す。
- **境をまたぐテスト 3 本**（上の 3 本）は、本体に残す。上の crate の側に置くのが筋。
- **テストの支え**: crate の外のテストが使う支えは、`areka-placement` の `test-support` 機能で開ける。本体は dev-dependency でその機能を付ける。写しは作らない。`capture_logs` の汎用の部分は `log-capture-kit` へ下ろしてもよい。どちらか 1 つに決め、二重にしない。
- **examples**: placement だけを使う `window-placement.rs` は `areka-placement` の examples へ移す。`collision-probe.rs` は emo2_boot のファイルも `#[path]` で読むので本体に残し、placement は crate の依存として使う。
- **依存の向きの規約の食い違い 5 件**: 要件の段で 1 件ずつ決める。推奨は「規約に合わせて直す」。直すのが移すだけで済まない件は、規約の文を実際に合わせて書き直す。どちらにしても、文と参照が一致して終わる。

## Scope

- **In**: crate の新設と移動・公開面の付け替え・本体の参照の書き換え・テストと examples の移し替え・`test-support` 機能・ソースを文字列で読むテストのパス・1,000 行の番人の表・ログの出所の名前で読む所・文書のパス（`doc/`・`doc/ukadoc-coverage/ledger/*.toml` の根拠の場所・`.kiro/steering/structure.md`）・`audit-placement-core`・`audit-placement-follow` の brief の「受け持つファイル」の書き換え・依存の向きの規約。
- **Out**: placement の中身の直し（不具合・要らない項目の削除・wintf との重複の整理は `audit-placement-core`・`audit-placement-follow` へ申し送る）・emo2_boot や本体のほかの部分の分け方（後の 5 本）。

## Boundary Candidates

- crate の骨組みと移動（ビルドが通るまで）。
- 公開面とテストの支え。
- テスト・examples・番人の表・文字列で読むテスト。
- ログの出所と文書のパス。
- 検証（テストの名前の一覧・警告・全体テスト・実機）。

## Out of Boundary

- placement の振る舞いの変更。直したくなった不具合は、その場で直さず audit の 2 本へ申し送る。
- `pub(crate)` のまま残す項目の削除。

## Upstream / Downstream

- **Upstream**: v0.1.0 の版の門を越えていること（「本体の crate 分け」の時期の決め）。`areka-test-binary-stability`（v0.1.0・`cargo test -p areka --bin areka` の落ち方を測ることが本体）は分ける前の形で測り終える。テストが crate ごとの実行ファイルに分かれると、落ちる組み合わせが変わるため。
- **Downstream**: `runtime-module-untangle`（次の 1 本）。`audit-placement-core`・`audit-placement-follow` はこの spec の着地を待つ。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `audit-placement-core`・`audit-placement-follow`: 受け持つファイルのパスをこの spec の完了で書き換える。
  - placement に触ると書いている約 19 本（`window-snap`・`stay-on-top-directives`・`shell-scale-linkage`・`tasktray-minimize`・`placement-measure-bake-once`・`window-work-area-clamp` ほか）: brief は書き換えず、roadmap の新旧のパスの対応表で読み替える。
  - `multi-ghost-runtime`: `placement/{spawn,persist}.rs` を触る。同じウェーブに置かない。

## Constraints

- **検証（要件に入れる）**: roadmap の「本体の crate 分け」の「共通の約束」の検証の項を、この spec の要件にそのまま入れる。この spec で回す実機テストは次の 2 つ。
  - `crates/areka/tests/window_link_real_run.rs`（`RUST_LOG` で出所を名指しする）を 1 回。
  - `tools/perf` の判定（`judge-perf.py`）が出所の名前に頼っていないことの確認。頼っていれば新しい名前へ直し、自分の確かめ（SelfTest）を回す。
- **ログの出所の名前**: モジュールのパスから付く出所は、移すと `areka_placement::…` に変わる。そのまま変え、名前で読んでいる所を全部直す。直書きの `"areka::placement::diag"` も新しい名前に合わせる（`RUST_LOG=areka_placement=debug` で診断が開くように）。語彙として付けている `"areka::persist::*"` は変えない。
- **共通の約束**（roadmap の同じ節）: ルートの `Cargo.toml` は触らない（`members = ["crates/*"]`）。新しい外の依存を足さない。`THIRD-PARTY-NOTICES.md` は `cargo about generate` で作り直す。`Cargo.lock` が動くので `/release` の版上げと同じウェーブに置かない。`crates/areka` を触るほかの spec と同じウェーブに置かない。1,000 行の上限は新しい crate でも同じ。
- **Fable**: Opus で足りる（新しい境目は今のモジュールの境目そのまま）。
