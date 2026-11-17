# 実装の記録: runtime-crate-split

## 1. 着手の基準（段 0）

対応する要件: 7.2, 6.6（「前」の側）

### 1.1 テスト一覧（前）

- 取った場所: ブランチの先頭 `6a0e44737637841d0a590cadb0e56842e731a11c`（コードを変える前）
- コマンド: `cargo test --workspace --no-fail-fast -- --list`（標準出力を保存）
- 保存先: `target/runtime-crate-split/tests-before.txt`
- 行末が `: test` の行の数（`grep -c ': test$'`）: **14143**

### 1.2 ログ履歴の分類（前）: 実行系（`areka::runtime`）の行はどの種別にも入らない

現行の `log_history` の分類（`crates/areka/src/log_history.rs` の RULES）では、target が `areka::runtime` の行はどの種別にも当たらない。

- 既存のテスト `status_rows_and_areka_is_exact_only`（`crates/areka/src/log_history_tests.rs` 100 行目〜）の 124 行目:
  `assert_eq!(classify(Level::INFO, "areka::runtime"), None);`
  （コメント 123 行目: `areka` は完全一致だけで、下のモジュールの info は外れる）
- 実行: `cargo test -p areka --bin areka log_history` → 43 passed; 0 failed（上のテストを含む。緑）

分割の後はこの行が `areka_runtime` 側の target になるので、後のタスクで「後」の側を同じ形で示し、前後を比べる。

## 2. 段 1（新しい crate へ移し、公開面を開ける）

対応する要件: 1.1〜1.8, 2.1〜2.6, 3.1, 3.5, 3.7, 4.1, 4.2, 4.4, 4.5, 5.1, 5.4, 6.1

### 2.1 骨組みと依存（タスク 2.1）

- `crates/areka-runtime/Cargo.toml` を新しく書いた。lib・`publish = false` と理由を同じ行・`version`／`edition`／`authors`／`license`／`repository` は `.workspace = true`・`description` つき。
- 本番の依存は設計の 18 個（外は `workspace = true`・中は `{ path = "../…" }`）。開発用の依存は設計の 8 個（番人の 3 つ・`shiori-host32-host`・`image`・`areka-placement` の `test-support`・`tracing-subscriber`・`async-io`）。機能と optional の依存は 3.1 で足す。
- 本体の `Cargo.toml` の `[dependencies]` に `areka-runtime = { path = "../areka-runtime" }` を 1 行足した（本体が直に使わなくなる依存の行は消していない＝`audit-areka-app` へ申し送り）。
- 根の `Cargo.toml` は変えていない（`members = ["crates/*"]` が新しい crate を拾う）。`git diff --stat Cargo.toml` は空。
- `Cargo.lock` の差分は `areka-runtime` の 1 項目と、本体の依存の 1 行（`"areka-runtime"`）だけ。外の crate は増えていない。

### 2.2 移動（タスク 2.2）

- `git mv` で 230 ファイルを移した（`git status` はどれも rename）。
  - 旧 `crates/areka/src/runtime/` の 211 ファイル: `crates/areka-runtime/src/` へ 151（旧 `mod.rs` → `lib.rs` を含む）、本体の `crates/areka/src/runtime_tests/` へ 58、本体の `session/` へ e2e の 2 本。
  - 本体の `placement_tests/` から `crates/areka-runtime/src/placement_tests/` へ 19（判定器の一族 16・`reset_balloon_pos_tests`・`measure_bake_log_tests`・`spawn_zorder_pair_deferred_tests`）。
- 本体に `src/runtime/` は無い。`crates/areka-runtime` の中の `crate::runtime` の grep は 0 件（`crate::runtime::` → `crate::`、`pub(in crate::runtime)` → `pub(crate)` を機械で置き換えた）。
- `crates/areka-runtime/src` の `#[path]`・`include_str!` は、3.2 が外す `frame_attach_tests.rs` の `frame_attach_carry_tests` の 1 件を除き、どれも実在するファイルを指す（スクリプトで全部の宣言を解いて確かめた）。
- 本物の親から外した宣言: `lib.rs`（`zorder_wiring_tests`・`frame_schedule_tests`・`film_playback_e2e_tests`）・`assets.rs`（2）・`balloon_visibility.rs`（1）・`balloon_visibility_phase.rs`（4）・`consumer_ledger.rs`（1）・`frame.rs`（12）・`frame/status_report.rs`（2）・`frame/switch.rs`（2）・`frame/window_state.rs`（1）・`shell_carry.rs`・`shell_balloon_resolve.rs`・`strict_judges.rs`・`switch_cue.rs`（各 1）・`spine.rs`（13）。本体の `placement_tests/mod.rs` から移した 5 つの子の宣言を外した。
- `session/runtime_boot_tests.rs` の `#[path = "../runtime/…"]` 2 行を兄弟のファイルへ書き換えた。
- 設計の表と実物が違った所（どれも設計の D1・D8 の規則に従って決めた）:
  - 判定器の一族は実物では 16 ファイル（設計は 15）。`transition_judge*.rs` 14 と `transition_signoff{,_procedure}_tests.rs` 2。移したのは 19（設計は 18）。
  - 設計の表に無いが、本体へ移すテストの中で宣言される子なので一緒に本体へ移した: `frame_shell_box_align_tests.rs`（親 `frame_shell_box_integration_tests`）・`frame_balloon_timeout_notice_e2e_tests.rs`（親 `frame_visibility_integration_tests`）・`frame/switch_stay_on_top_tests.rs`（親 `frame/switch_reload_tests`）。
  - 設計の表に無いが、本体へ移す支え `frame_chain_finalize_persist_test_support`（本体の `crate::insert_persist_wiring` を呼ぶ）を使うので D8 の定義で「本体を引く」テストとして本体へ移した: `frame_chain_finalize_align_tests.rs`・`frame_chain_finalize_clamp_tests.rs`・`frame_chain_rule_tests.rs`。

### 2.3 ファイルの一部を本体へ出す（タスク 2.3）

- `spine` の ⑶: 旧 `spine.rs` の **374〜894 行**（521 行・`SETTLE_MIN` の説明の頭から、子のテストの宣言の前の説明 3 行まで）を、本体の `runtime_tests/spine.rs` へ 1 文字も変えずに出した。設計の「386〜894 行」は実物の行と合わない（実物では 364〜372 行が ⑴ の再公開、374 行から ⑶ の `SETTLE_MIN`）。ファイルは 715f5819 と同じ 938 行。
  - 確かめ: `git show HEAD:crates/areka/src/runtime/spine.rs | sed -n 374,894p` と `crates/areka/src/runtime_tests/spine.rs` が `diff -q` で同じ。
  - `areka-runtime` の `spine.rs` は 378 行になり、`SpineHarness`・`settle_bounded`・`make_world_with_gpu`・`capture_logs` は無い。⑴ の再公開・⑵ の台本の SHIORI と、子 `hold_support`・`wait_tests` の宣言だけが残る。
- `wire_tests`: 旧 `mod.rs` の 755 行（`#[cfg(test)]`）〜 851 行（`}`）の中身 757〜849 行（93 行）を、本体の `runtime_tests/wire_tests.rs` へ出した。ファイルのモジュールにするため、外の `mod wire_tests { … }` の 2 行と閉じ括弧を外し、各行の頭の 4 文字の空白だけを落とした（中身の文字は同じ）。
  - 確かめ: 旧 757〜849 行の各行から頭の 4 文字の空白を落とした列と `wire_tests.rs` が一致（スクリプトで比べた）。
- `git diff --color-moved=plain --color-moved-ws=allow-indentation-change`（`git add -N` で新しい 2 ファイルを差分に載せて）で、受けた側の追加の行 614 行（521＋93）がどれも「移した行」として色分けされた。

### 2.4 番人の表と読み先（タスク 2.4）

- `log-capture-kit/tests/file_length_guard_test.rs`（項目の数は 4 のまま・理由の文は変えていない＝主語に crate の名前が無いため）:
  - `crates/areka/src/runtime/frame_transition_branch_tests.rs` → `crates/areka-runtime/src/frame_transition_branch_tests.rs`（例外表と較正の `CALIBRATION_DROPPED_ENTRY` の 2 か所）
  - `crates/areka/src/placement_tests/transition_judge_verdict_tests.rs` → `crates/areka-runtime/src/placement_tests/transition_judge_verdict_tests.rs`
- `log-capture-kit/tests/workspace_scan_test.rs` 320 行: `crates/areka/src/runtime/spine_display_tests.rs` → `crates/areka/src/runtime_tests/spine_display_tests.rs`（項目の数は変えていない）。
- 文字で読むテストの一時のつなぎ直し（相対の道のまま・最終形は段 5）:
  - `runtime_tests/frame_move_order_tests.rs`・`zorder_wiring_tests.rs` の `include_str!("frame.rs")` → `"../../../areka-runtime/src/frame.rs"`、`frame_schedule_tests.rs`・`zorder_wiring_tests.rs` の `include_str!("mod.rs")` → `"../../../areka-runtime/src/lib.rs"`。
  - `placement_tests/measure_template_tests.rs`: `../runtime/frame_test_support.rs`・`../runtime/assets.rs` → `../../../areka-runtime/src/…`、`../runtime/assets_tests.rs` → `../runtime_tests/assets_tests.rs`（`examples/collision-probe/setup.rs` はそのまま）。
  - `placement_tests/zorder_{group,inversion}_branch_coverage_tests.rs` の `include_str!("../runtime/…")` → `"../../../areka-runtime/src/…"`（表の見出しの文字は変えていない）。
  - 設計の表に無かった文字で読むテスト: 本体の `tick_gate_config_producers_tests.rs` の `include_str!("runtime/…")` 9 か所 → `"../../areka-runtime/src/…"`（見出しの文字は変えていない）。
  - `areka-runtime` の `frame_attach_tests.rs` の `ACTOR_SCAN_SITES`（ワークスペースの根からの道）2 行 → `crates/areka-runtime/src/{frame/scale_text,hover_inject}.rs`。
  - `areka-runtime` の `placement_tests/spawn_zorder_pair_deferred_tests.rs` の名簿（`CARGO_MANIFEST_DIR` からの道）`src/runtime/…` → `src/…`（設計の段 5 の最終形と同じ形）。
- 本体の examples `collision-probe.rs` の `#[path = "../src/runtime/{target_map,hit_region}.rs"]` → `../../areka-runtime/src/{target_map,hit_region}.rs`（最終形は 6.1）。
- 本体と `areka-runtime` の全部の `include_str!`・`#[path]` を解いて、実在しない読み先は `frame_attach_carry_tests`（3.2 の持ち物）の 1 件だけ。
- `cargo test -p log-capture-kit`: 緑（9 の実行ファイル・全部 ok）。

### 2.5 公開面（タスク 2.5）

- 本体の `main.rs` の `mod runtime;` を外し、一時の別名 `use areka_runtime as runtime;`（段 4 で外す）を置いた。
- 開ける前の `cargo check -p areka --bin areka` は 89 件のエラー（E0603・E0624・E0616 ほか）。コンパイラが外から使うと言った項目だけを 1 件ずつ `pub(crate)` → `pub` にした（数回の組み立てで収束）。続いて `private_interfaces`（公開の項目が名指す型）と「型が非公開」のエラーが出た 11 の型を `pub` にし、項目の上に「要件 2.5: 公開の `<名前>` が名指すので開ける」の 1 行を書いた。
- 使われない `pub(crate)` は残した。開けた後に死んだコードの警告は 0 件（消した項目は無い）。
- 説明の中の本体へのリンク（2.6）を、本体を名指す地の文へ書き換えた: `consumer_ledger.rs` 6 か所（`ReadmeCueSink`・`InstallCueSink`・`UpdateCueSink`）・`lib.rs` 4 か所（`is_benign_boot_error` 2・`wire_kanade_stop` 2）・`special_name.rs` 1・`hit_region.rs` 1（`is_benign_boot_error`）。
- `windows` の機能の上乗せは要らなかった（ワークスペースの表のまま）。
- `cargo check -p areka-runtime --lib`・`cargo check -p areka --bin areka` は緑・警告 0。テストの組み立ては段 2・3 で緑に戻す。

開けた項目（`crates/areka-runtime/src` からの道・112 件・8.6 の申し送りに使う）:

- `balloon_hover.rs`: `pub struct BalloonHover(HashSet<usize>);`
- `balloon_hover.rs`: `pub fn set_balloon_hover(...)`
- `balloon_hover.rs`: `pub fn clear_balloon_hover(...)`
- `change_cue.rs`: `pub struct ChangeRequestRaw`
- `change_cue.rs`: `pub struct ChangeCueSink`
- `change_cue.rs`: `pub fn new(...)`
- `frame.rs`: `pub use self::window_state::{DiscardReason, log_stay_on_top_discard};`
- `frame.rs`: `pub use self::zorder_drain::ZOrderAbsentReports;`
- `frame/window_state.rs`: `pub enum DiscardReason`
- `frame/window_state.rs`: `pub fn log_stay_on_top_discard(...)`
- `frame/wiring.rs`: `pub fn scope_scale_table(...)`
- `frame/wiring.rs`: `pub fn presenter(...)`
- `frame/wiring.rs`: `pub fn shown_talk_id(...)`
- `frame/wiring.rs`: `pub fn attached(...)`
- `frame/wiring.rs`: `pub fn runtime(...)`
- `frame/zorder_drain.rs`: `pub struct ZOrderAbsentReports`
- `lib.rs`: `pub mod balloon_hover;`
- `lib.rs`: `pub mod change_cue;`
- `lib.rs`: `pub mod reload_args;`
- `lib.rs`: `pub mod reload_cue;`
- `lib.rs`: `pub mod scope_scale;`
- `lib.rs`: `pub mod session_hooks;`
- `lib.rs`: `pub mod shell_balloon_resolve;`
- `lib.rs`: `pub mod shell_box_hover;`
- `lib.rs`: `pub mod shell_carry;`
- `lib.rs`: `pub mod shell_cursor_assets;`
- `lib.rs`: `pub mod shell_cursor_table;`
- `lib.rs`: `pub mod skin_facts_copy;`
- `lib.rs`: `pub mod skin_switch;`
- `lib.rs`: `pub mod special_name;`
- `lib.rs`: `pub mod strict_judges;`
- `lib.rs`: `pub mod switch_assets;`
- `lib.rs`: `pub mod switch_cue;`
- `lib.rs`: `pub mod user_break_cue;`
- `lib.rs`: `pub mod window_state_cue;`
- `lib.rs`: `pub struct CueSinkSources`
- `lib.rs`: `pub enum RuntimeBootError`
- `lib.rs`: `pub sources: Option<CueSinkSources>,`
- `reload_args.rs`: `pub enum ReloadTarget`
- `reload_args.rs`: `pub struct DescriptParts`（2.5 の注つき）
- `reload_args.rs`: `pub enum McpTarget`
- `reload_args.rs`: `pub fn parse(...)`
- `reload_args.rs`: `pub struct ScriptReload`
- `reload_args.rs`: `pub struct DescriptNotes`
- `reload_cue.rs`: `pub struct ReloadCueSink`
- `reload_cue.rs`: `pub fn new(...)`
- `reload_cue.rs`: `pub struct ReloadRx(Receiver<ScriptReload>);`
- `reload_cue.rs`: `pub fn try_iter(...)`
- `reload_cue.rs`: `pub fn wire_reload_rx(...)`
- `scope_scale.rs`: `pub type SharedScopeScaleTable = Arc<RwLock<ScopeScaleTable>>;`
- `scope_scale.rs`: `pub struct ScopeScaleTable(BTreeMap<u32, ScopeScales>);`（2.5 の注つき）
- `scope_scale.rs`: `pub fn get(...)`
- `scope_scale.rs`: `pub fn scaling_text(...)`
- `session_hooks.rs`: `pub struct SessionHooks`
- `shell_balloon_resolve.rs`: `pub struct SkinCandidate`
- `shell_balloon_resolve.rs`: `pub enum NotFoundReason`（2.5 の注つき）
- `shell_balloon_resolve.rs`: `pub fn record_installed_shell(...)`
- `shell_balloon_resolve.rs`: `pub fn record_installed_balloon(...)`
- `shell_balloon_resolve.rs`: `pub fn shell_candidates(...)`
- `shell_balloon_resolve.rs`: `pub fn balloon_candidates(...)`
- `shell_balloon_resolve.rs`: `pub fn installed_for(...)`
- `shell_balloon_resolve.rs`: `pub fn resolve_skin_target(...)`
- `shell_balloon_resolve.rs`: `pub fn find_by_name_or_folder<'a>(...)`
- `shell_box_assets.rs`: `pub struct ShellBoxAssets`（2.5 の注つき）
- `shell_box_hover.rs`: `pub struct ShellBoxHover(HashMap<usize, (BoxName, Option<usize>)>);`
- `shell_box_hover.rs`: `pub fn get(...)`
- `shell_box_hover.rs`: `pub fn set(...)`
- `shell_box_hover.rs`: `pub fn injected(...)`
- `shell_box_hover.rs`: `pub fn set_injected(...)`
- `shell_carry.rs`: `pub struct ShellCarry`（2.5 の注つき）
- `shell_carry.rs`: `pub fn capture_for_reload(...)`
- `shell_carry.rs`: `pub fn hand_over(...)`
- `shell_cursor_assets.rs`: `pub struct ShellCursorAssets`
- `shell_cursor_assets.rs`: `pub enum CursorFile`（2.5 の注つき）
- `shell_cursor_assets.rs`: `pub fn cursor_for(...)`
- `shell_cursor_table.rs`: `pub enum CursorScene`
- `shell_cursor_table.rs`: `pub enum SystemCursor`
- `shell_cursor_table.rs`: `pub enum Shape<F>`
- `shell_cursor_table.rs`: `pub struct CursorRows<F>`
- `shell_cursor_table.rs`: `pub struct GhostCursorKeys<F>`
- `skin_facts_copy.rs`: `pub fn copy(...)`
- `skin_switch.rs`: `pub enum SkinKind`
- `skin_switch.rs`: `pub struct SkinRequestRaw`
- `skin_switch.rs`: `pub enum SkinSpec`
- `skin_switch.rs`: `pub enum SkinOrigin`
- `skin_switch.rs`: `pub struct SkinRequest`
- `skin_switch.rs`: `pub enum SkinVerdict`
- `skin_switch.rs`: `pub struct SkinSwitchInFlight`
- `skin_switch.rs`: `pub enum SkinSwitchStage`
- `skin_switch.rs`: `pub enum SwapFinish`（2.5 の注つき）
- `skin_switch.rs`: `pub fn skin_ref_name(...)`
- `skin_switch.rs`: `pub fn skin_ref_path(...)`
- `special_name.rs`: `pub enum NameResolution`
- `special_name.rs`: `pub enum UnresolvedReason`（2.5 の注つき）
- `special_name.rs`: `pub fn as_ref_str(...)`
- `special_name.rs`: `pub fn describe(...)`
- `special_name.rs`: `pub fn resolve_special_name(...)`
- `special_name.rs`: `pub struct LastInstalledGhost(pub String);`
- `strict_judges.rs`: `pub fn production(...)`
- `switch_assets.rs`: `pub enum SwitchBuildRequest`
- `switch_assets.rs`: `pub enum SwapBuilt`（2.5 の注つき）
- `switch_assets.rs`: `pub enum SwitchBuildError`（2.5 の注つき）
- `switch_assets.rs`: `pub fn spawn_switch_build(...)`
- `switch_cue.rs`: `pub struct SwitchCueSink`
- `switch_cue.rs`: `pub fn new(...)`
- `talk_lifecycle.rs`: `pub enum TalkLifecycleSignal`
- `talk_lifecycle.rs`: `pub enum TalkTimeout`（2.5 の注つき）
- `talk_lifecycle.rs`: `pub struct BalloonLifecycleSink`
- `talk_lifecycle.rs`: `pub fn new(...)`
- `user_break_cue.rs`: `pub enum NoUserBreakSignal`
- `user_break_cue.rs`: `pub struct NoUserBreakCueSink`
- `user_break_cue.rs`: `pub fn new(...)`

## 3. 段 2（テストの支えと機能を整え、`areka-runtime` のテストを緑にする）

対応する要件: 1.9, 3.1, 3.2, 3.3, 3.4, 3.6, 3.8, 3.9, 4.2, 4.3, 4.4, 4.5, 6.4

### 3.1 機能と optional の依存（タスク 3.1）

- `crates/areka-runtime/Cargo.toml` に `[features]` の `test-support = ["dep:shiori-host32-host", "areka-placement/test-support"]`（説明の 1 行つき）と、`[dependencies]` の `shiori-host32-host = { path = "../shiori-host32-host", optional = true }`（説明の 1 行つき）を足した。
- 連鎖の `"areka-placement/test-support"` は、機能だけが立つビルドで `frame_test_support.rs` の `GhostTitles::from_scope_titles`（`areka-placement` の `test-support` の項目）が E0599 になったので足した（設計 D11 の「コンパイラが要ると言ったときだけ」）。
- 本体の `crates/areka/Cargo.toml` の `[dev-dependencies]` に `areka-runtime = { path = "../areka-runtime", features = ["test-support"] }`（説明の 1 行つき）を足した。
- 番人の 3 つ（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`）は `areka-runtime` と本体のどちらの `[dependencies]` にも無い（全 crate の `Cargo.toml` を toml として読んで確かめた。当てはまったのは前からある `real-run-kit` の 2 つだけで、この spec の外）。
- `cargo tree` の証跡:
  - `cargo tree -p areka-runtime -e normal` にも `-F test-support` にも本体（`areka v…`）は 0 行。
  - 直の依存（`--depth 1`）を比べると、`-F test-support` で増えるのは `shiori-host32-host` の 1 行だけ（19 行 → 20 行）。
  - 木の全体（`--prefix none` で重複を除いた一覧）は機能の有無で同じ 140 行。`shiori-host32-host` は機能が無くても `areka-ghost`・`areka-kanade` を経て推移で入っている（`cargo tree -i shiori-host32-host` で確かめた・dev-dependency の注が書くとおり）。設計の「`shiori-host32-host` も出ない」は直の依存のこととして読んだ（3.7）。
- `Cargo.lock` は変わらない（`shiori-host32-host` は dev-dependency として既に `areka-runtime` の項目にある）。

### 3.2 `spine` と `frame` の支えの印（タスク 3.2）

- `lib.rs`: `spine` を `#[cfg(any(test, feature = "test-support"))] pub mod spine;` にした（上の注も今の形に書き直した）。
- `spine.rs`（⑴ ⑵）:
  - 頭の `//!` を ⑴ ⑵ だけを説明する形にした。⑴ が持つ待ちの形（純粋ポーリング）と「sleep 不使用」の射程はそのまま残し、⑶ の説明（ハイブリッド・回収・土台・構造上の逸脱）は本体の `runtime_tests/spine.rs` の頭へ移した（文は変えていない）。依存の申し送りは `areka/Cargo.toml` の dev-dependency → `areka-runtime/Cargo.toml` の 2 行（optional と dev）へ書き直した。
  - 頭の `use` は ⑵ が使う 5 行だけにした。⑶ と子のテストが `super::` で引いていた 49 行（`crate::session::ghost_quit::KanadeNoticeRx`・`log_capture_kit`・`super::sample_test_support::acquire_emo2` を含む）は、本体の `runtime_tests/spine.rs` の頭へそのまま移した（道と要らない名前は段 3 の代わりの親で直す）。子の `wait_tests` が `super::{Duration, Instant}` で引くので、`#[cfg(test)] use std::time::{Duration, Instant};` を 1 行残した。
  - ⑴ ⑵ の `pub(crate)` を `pub` にした（`spine.rs`・`spine_hold_support.rs`・`spine_wait.rs`）。
- `frame.rs`: `test_support` と `balloon_runtime_test_support` を `#[cfg(any(test, feature = "test-support"))] pub mod` にした。`frame_test_support.rs` の `pub(super)` を全部 `pub` に、`frame_balloon_runtime_test_support.rs` の `pub(crate)` を `pub` にした。
- `frame_test_support.rs` の `#[cfg(test)]`: `capture_logs`・`window_move_lines`・`window_move_routes_of` と `use log_capture_kit::…` の 2 行（設計どおり）。
- `frame_attach_tests.rs` の `gpu_attach_world` を説明ごと `frame_test_support.rs` の末尾へ `pub fn` で移した（COM の初期化の `use` 1 行を一緒に移した）。子 `frame_attach_carry_tests` の宣言（注 1 行・`#[path]`・`mod`）を外した。
- `assets::empty_definitions` を `#[cfg(any(test, feature = "test-support"))] pub fn` にした。`sample_test_support`・`frame::zorder_drain_test_support` は `#[cfg(test)]` のまま。
- 同じ働きの支えが 2 つ無いこと: 進行状態の記録の型と本体（下の外れ）は `areka-runtime` の `spine.rs` の 1 か所だけ。`gpu_attach_world` は `frame_test_support.rs` の 1 か所だけ。

コンパイラが出したので設計の規則で付け替えた項目（設計の表に無かったもの）:

| 項目 | 出たもの | 付け替え |
|---|---|---|
| `spine.rs` の `use self::wait::{BACKOFF_SLEEP, SPIN_WAIT, join_bounded}` | 機能だけが立つビルドで `unused_imports` | 本体の ⑶ が使う（`runtime_tests/spine.rs` の `SPIN_WAIT`・`BACKOFF_SLEEP`・`join_bounded`）ので `pub use` にし、`spine_wait.rs` の 3 つを `pub(super)` → `pub` |
| `ScriptedShioriHandle::status_calls` | 機能だけが立つビルドで `dead_code`（欄とメソッド） | 本体のテスト（`spine_conformance_lap_tests` ほか）が使うので `pub fn` |
| `spine_wait.rs` の `GpuSlots::try_take` | 機能だけが立つビルドで `dead_code` | 使い手は自分のテスト（`spine_wait_tests`）だけなので `#[cfg(test)]` |
| `crash_report_test_support.rs` の `RaiseException`（kernel32 の宣言） | 機能だけが立つビルドで `dead_code` | 使い手は自分の較正のテストだけなので宣言に `#[cfg(test)]` |
| `frame_test_support.rs` の `use areka_placement::diag::WINDOW_MOVE_RECORD_TAG` | 機能だけが立つビルドで `unused_imports` | 使い手は `#[cfg(test)]` の `window_move_lines` だけなので `#[cfg(test)]`（設計の「`use` 2 行」に足して 3 行） |
| `frame/work_area_sync.rs` の `SnapshotChange`・`frame/dpi.rs` の `ScaleReportSource`・`frame/drain_resnap.rs` の `PhysicalSizeSource` | `FrameHarness` の `pub` のメソッドが名指して `private_interfaces` | 設計 PublicSurface の規則（2.5）で `pub(super)` → `pub` にし、上に「要件 2.5: 公開の `<名前>` が名指すので開ける」の 1 行。モジュールは私有のままなので外からの届き方は変わらない |
| `frame.rs` の `all_shown` の再公開 | `--all-targets` で `unused_imports` | 使い手は本体の ⑶ だけになったので `#[cfg(any(test, feature = "test-support"))] pub use` |
| `frame/wiring.rs` の `RuntimeWiring::{read_back_target, apply_present, show_balloon_target}` | `--all-targets` で `dead_code` | 使い手は本体の spine のテストだけなので `#[cfg(any(test, feature = "test-support"))] pub fn` |

設計からの外れ（3.2）:

- 設計 D4 は `spine_conformance_support.rs` を丸ごと本体へ移すとしていたが、⑵（`ScriptedShioriBackend`・`ScriptedShioriHandle`）が同じファイルの進行状態の記録の型と関数（`RecordedStatus`・`StatusLedger`・`record_status`・`snapshot_status_calls`）を使っている。本体に置いたままでは `areka-runtime` の ⑵ が組めない（下の crate から上の crate は引けない）＝設計の中の食い違い。
- 直し: その 4 つ（旧 `spine_conformance_support.rs` の 30〜62 行）だけを `areka-runtime` の `spine.rs` の ⑵ の隣へ下ろし、`pub` にした（中身と説明は同じ・説明の中の「`spine.rs` の」だけを外した）。本体の `spine_conformance_support.rs` は定義を消し、`pub(super) use areka_runtime::spine::RecordedStatus;` で出し直す（兄弟の `spine_conformance_script.rs` が `conformance_support::RecordedStatus` で引くため。ほかの 3 つは本体では spine の代わりの親から届く）。定義は 1 つだけ。使わなくなった `use std::sync::{Arc, Mutex};` を外し、頭の説明の 1 文と、行番号で指していた 2 か所の説明（`spine_conformance_judge.rs`・`spine_conformance_support_stage_tests.rs`）を `areka-runtime` の `spine.rs` を名前で指す形にした。
- 理由: 定義を 2 つにしない（3.3）・⑵ を `areka-runtime` に残す D4 の決めを変えない・本番のコードに触らない、の 3 つを同時に満たす最小の直し。

### 3.3 落ちの報告（タスク 3.3）

- 宣言を `#[cfg(any(test, feature = "test-support"))] pub mod crash_report_test_support;` にした。`install`（`pub extern "C" fn`）・持ち手の表（`hold`・`release`・`HOLDER_ROWS`・`NO_ROW`）を `pub` にし、`INSTALLED` を読むだけの `pub fn installed() -> bool` を足した。頭の「写しの相手」の行は変えていない。
- grep の証跡（`crates/areka-runtime` と `crates/areka` で `link_section = ".CRT$XCU"` を前 3 行つきで grep）: 当たりは `crates/areka-runtime/src/crash_report_test_support.rs` の 1 か所だけで、すぐ上が `#[cfg(test)]`・`#[used]`。本体の側は段 3（4.4）で足す。

### 3.4 口の倒れ先（タスク 3.4）

- `session_hooks.rs` の `FALLBACK` の 2 つの `const` と説明を消し、`of` は口が無いとき `NO_BODY` を返す。`#[cfg(not(test))]` の `error!(event = "session_hooks_missing")` はそのまま（文は変えていない）。
- `session_hooks.rs` の中に本体を名指す道（`crate::session`・`crate::ghost_session`）は 0 件。口を読む相を回す `frame_attach_tests` ほかは緑（3.6）。

### 3.5 `placement_tests/`（タスク 3.5）

- `crates/areka-runtime/src/placement_tests/mod.rs` を新しく書き（置いた理由と子の宣言だけ）、`lib.rs` が `#[cfg(test)] mod placement_tests;` で宣言する。判定器は `pub(crate) mod transition_judge;`・`pub(crate) mod transition_judge_offset;`、テストは `mod …;`。判定器の一族の残り 14 ファイルは、判定器の 2 ファイルの中の兄弟を指す `#[path]` から宣言される（変えていない）。
- 判定器を使う 2 本（`frame_transition_{atomicity,branch}_tests.rs`）は `crate::placement_tests::transition_judge…` の同じ道で通った（23 本緑）。
- 判定器の一族は `#[cfg(test)]` の下だけなので `cargo check -p areka-runtime --lib` に入らない。

置いた理由の表（D8 の定義＝本体の項目を 1 つでも使えば「本体を引く」を 1 本ずつ当てた・段 1 で数えた実物の数）:

| テスト | 本体の項目を使うか | 置き場 |
|---|---|---|
| 判定器の一族 16（`transition_judge*.rs` 14・`transition_signoff{,_procedure}_tests.rs` 2） | 使わない（`crate::target_map` だけ） | `areka-runtime`（4.2） |
| `reset_balloon_pos_tests` | 使わない（`switch_assets`・`sample_test_support`・配置の `test_support`） | `areka-runtime`（4.4） |
| `measure_bake_log_tests` | 使わない（`assets`） | `areka-runtime` |
| `spawn_zorder_pair_deferred_tests` | 使わない（配置の `test_support::zorder_pair_deferred` と自分の `src`） | `areka-runtime` |
| `measure_bake_count_tests` | 使う（本体の読み口 `crate_path("areka-emo-compose", …)`） | 本体 |
| `zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`・`zorder_branch_coverage_test_support` | 使う（本体の読み口 26 か所） | 本体 |
| `measure_template_tests` | 使う（本体の examples と読み口） | 本体 |
| `spawn_assembly_tests` | 使う（`input_events`） | 本体 |

### 3.6 段 2 を緑にする（タスク 3.6）

- 出所の名前で選ぶテスト: `balloon_background_tests.rs` 245 行の `target=areka::runtime::balloon_background ` → `target=areka_runtime::balloon_background `、`shell_cursor_assets_tests.rs` 21 行の `TARGET` → `"areka_runtime::shell_cursor_assets"`。
- `cargo check -p areka-runtime --lib`・`--features test-support --lib`（機能だけが立つビルド）・`--all-targets`・`--all-targets --features test-support`: どれも緑・警告 0。
- `cargo test -p areka-runtime`: lib のテスト 915 本緑・3 本は前からの `#[ignore]`・赤 0（1 回で緑・回し直しなし）。
  - 落ちの報告の 5 本: `crash_filter_is_installed_before_the_tests_start`・`initializer_keeps_a_process_lifetime_mta`・`first_chance_writes_only_access_violations`・`line_cuts_an_overlong_line_at_a_char_boundary_with_a_mark` が緑、`crash_report_names_the_running_test` は前から `#[ignore]`（わざとプロセスを落とす較正）。
  - COM・GPU を使う `frame::attach_tests` 18 本・`spine::wait_tests` 15 本・`spine::hold_support` 8 本が緑（プロセスの寿命の MTA の下）。`placement_tests` 186 本緑（2 本は前からの `#[ignore]`）。
- `cargo test --doc -p areka-runtime`: `running 0 tests`（0 件）。
- `cargo check -p areka --bin areka`: 緑・警告 0。本体のテストの組み立ては段 3 で緑に戻す。
- `cargo fmt --all -- --check`: 段 1 の道の書き換えで短くなった行の折り返し（`areka-runtime` の 8 ファイル）が残っていたので `cargo fmt -p areka-runtime` を当てた（空白だけ）。いまは差分 0。本体の `runtime_tests/` は `main.rs` から宣言されていないので rustfmt の木に入らない（段 3 で入る）。
- `cargo test -p log-capture-kit`: 緑（1,000 行の番人を含む）。

### 3.7 設計と違った所

- 3.2 の外れ（上）。
- `cargo tree` の「`shiori-host32-host` も出ない」は、推移の依存（`areka-ghost`・`areka-kanade`）で機能が無くても木に出る。直の依存では機能の有無で `shiori-host32-host` の 1 行だけが変わる（3.1）。本番のビルドに新しい crate は入らない（木の全体が同じ）。

## 4. 段 3（本体へ移したテストを代わりの親で組む）

対応する要件: 2.1, 2.5, 3.3, 3.4, 3.5, 3.6, 3.8, 3.9, 6.1, 6.4, 6.8, 7.4

段 3 の途中で D3 ⑷ の渡し口が 20 を超えたので止め、2026-11-17 の討議で案 C（テストを `#[test]` 関数の単位で割る・設計「実装の中の討議の結果」）に決まった。この節は案 C の後の最終の形を記す。

### 4.1 代わりの親と見本の入口（タスク 4.1）

- `main.rs` の `#[cfg(test)] mod runtime_tests;` と `mod sample_test_support;`、`runtime_tests/` の代わりの親 13（`mod.rs`・`assets.rs`・`balloon_visibility.rs`・`balloon_visibility_phase.rs`・`consumer_ledger.rs`・`frame.rs`・`frame/{status_report,switch,window_state}.rs`・`shell_carry.rs`・`shell_balloon_resolve.rs`・`strict_judges.rs`・`switch_cue.rs`）と `spine.rs`（⑶ の足場を兼ねる）は WIP（`d328ed62a`）のまま使った。
- 代わりの親の写した `use` の行は、割った後に子が使わなくなった分を `cargo fix`（使われていない `use` の除去だけ）で落とした。本体のテストの組み立ての警告は 0。
- 代わりの親に足した `use` の行:
  - `frame.rs`: 子の `carry_tests`（本物の親は `areka-runtime` の `frame_attach_tests`）が `super::*` で使う 6 行（`build_boot_assets`・`emo2_root`・`emo2_balloon_root`・`TextLayerConfig`・`gpu_attach_world`・`zero_clock`・`TalkLifecycleSignal`・`mpsc`）。
  - `balloon_visibility.rs`: 本物の親の `pub(super) use phase::run_balloon_visibility_phase;` に当たる `use areka_runtime::balloon_visibility::phase::run_balloon_visibility_phase;`（私有のモジュールを道で指していた `decision::decide`・`wait::configured_timeout_secs` の 2 行は外した。どちらも親の `pub use` で glob に入る）。
  - `balloon_visibility_phase.rs`: `TalkLifecycleSignal` を `areka_runtime::talk_lifecycle` から引く形に直した（`balloon_visibility` の中の取り込みは私有）。
- `strict_judges.rs` の `use areka_runtime::strict_judges::*;` は、本体に残る子（`doc_tables`）が項目を道で引くので使われず、外した（説明の 1 文を足した）。
- 代わりの親が本物のモジュールと同じ名前で影になる道（子の `super::super::switch_cue::SwitchCueSink`・`super::frame::frame_system`）は `areka_runtime::switch_cue::SwitchCueSink`・`areka_runtime::frame::frame_system` へ書き換えた（`crate::runtime::` の書き換えと同じ扱い）。
- `runtime_tests/` の中の `crate::runtime::` の grep は 0 件。`mod spine;` を `pub(crate) mod spine;` にした（4.3 の `ghost_session_{reload,restart}_tests` が使う）。

### 4.2 関数の単位で割る（案 C）と私有の項目の開け方（タスク 4.2）

#### 割り方の規則

- D8 の「本体を引く」を関数の単位で読んだ。`#[test]` 関数が（同じファイルの補助・親のテストのファイルの補助を辿って）次のどれかを使えば本体に残す: 本体の crate の項目（`crate::ghost_session`・`crate::session`・`crate::input_events`・`crate::boot_config`・`main.rs` の `restore_merged_placements`・`insert_persist_wiring` ほか）・本体の読み口（`crate_source_test_support`）と支え（`scope_bind_fixture_test_support`）・本体の文字を読む `include_str!`（`../session/runtime_boot.rs`・`../ghost_session.rs`・`../main.rs`・`../session/ghost_quit.rs`）・本体に置いた支え（spine の ⑶ と `spine_test_support`・`spine_conformance_{judge,script,support}`・`frame_chain_finalize_persist_test_support`＝どれも本体の項目を呼ぶので D4・D8 で本体に置いたもの）。見本の入口（`crate::sample_test_support`）は crate ごとに 1 つあるので本体を引くに数えない。
- どれも使わない関数は `areka-runtime` の元のテストのファイル（同じ名前・同じ道）へ戻し、本物の親に元と同じ印・同じ名前の宣言を戻した。テストの関数の名前は変えていない。
- 両方が使う補助は `areka-runtime` の側に 1 つだけ置き（`#[cfg(any(test, feature = "test-support"))] pub`）、両方が同じ `use super::<支え>::{…};` で引く。
- 例外（番人の crate）: 両方が使う補助が番人の crate（`log_capture_kit`・`sample_ghost_kit`・`temp_path_kit`）を引くときは `test-support` に置けない（要件 3.4・D4）。写しも作らない。そこで D4 の「引くなら本体へ移すテストと一緒に本体へ移す」に従い、その補助を使う関数も本体に残した（下の表の「番人の補助で本体」）。

#### 関数の割り方の表

本体に残った関数を名前で、`areka-runtime` へ戻った関数を本数で示す（全部の関数が本体を引くファイルと spine の子は割っていない）。

| ファイル | `areka-runtime` へ戻った | 本体に残った | 両方が使う補助の置き場 |
|---|---|---|---|
| `assets_tests.rs` | 13 | `screen_binds_split_per_scope_on_shared_scope_fixture` | ― |
| `assets_shell_tests.rs` | 2 | `shell_assets_take_initial_surfaces_from_descript_defaults` | ― |
| `balloon_visibility_phase_tests.rs` | 1（`dragging_requires_both_balloon_window_and_drag_marker`） | 残りの 14（うち番人の補助 `visibility_lines` で本体: `decision_events_are_written_with_the_agreed_level_and_fields`・`transition_of_a_show_that_did_not_take_is_not_written`） | ― |
| `balloon_visibility_phase_box_tests.rs` | 6 | `box_observation_reads_the_shown_boxes_and_the_settled_hover`・`missing_box_hover_record_means_no_hover_and_is_logged_once`・`collection_settles_the_box_hover_before_observing`・`shell_window_drag_without_box_text_is_not_dragging` | `runtime`・`world_at` → `balloon_visibility_phase_test_support.rs`（既存） |
| `consumer_ledger_agreement_tests.rs` | 1（`samples_cover_exactly_the_ledger_rows`） | 残りの 4 | `ABSOLUTE`・`ledger_keys`・`samples` → 新 `consumer_ledger_test_support.rs` |
| `frame/status_report_tests.rs` | 6 | 送り・届けの相の 8 | `binding`・`obs`・`obs_box` → 新 `frame/status_report_test_support.rs` |
| `frame/status_report_stage_tests.rs` | 4 | `empty_slot_in_the_phase_keeps_the_ledger`・`phase_sends_the_stage_even_while_the_runtime_is_busy`・`unattached_scope_is_not_noted` | ― |
| `frame/window_state_tests.rs` | 11 | `despawned_registry_window_is_skipped_with_a_debug_record`・`log_fields_for_change_unchanged_and_raise`・`discard_log_reports_reason_while_on`・`discard_log_is_silent_while_off_or_unwired`・`log_ghost_falls_back_to_dash_and_raise_reports_normal_band`（番人の補助 `window_state_events` で本体） | `ON`・`OFF`・`marked`・`queue_in`・`queue_text`・`state_with_channel`・`world_with_windows` → 新 `frame/window_state_test_support.rs` |
| `frame_move_order_tests.rs` | 5 | `a_script_reset_sends_nothing_to_kanade` | `steady_harness`・`wiring` → 新 `frame_shared_test_support.rs` |
| `frame_schedule_tests.rs` | 3 | `ghost_quit_system_without_the_frame_system_quits_on_one_notice` | `code_only`・`squeeze` → 新 `source_text_test_support.rs` |
| `zorder_wiring_tests.rs` | 7 | `t_zwi05_the_boot_wires_the_channel_the_sink_and_the_handoff`・`t_zwi08_the_entry_point_carries_the_shell_setting_into_the_wiring`（本体の文字を読む） | `code_only`・`squeeze` → 同じ `source_text_test_support.rs`（2 ファイルの同じ写しが 1 つになった） |
| `shell_balloon_resolve_tests.rs` | 5 | `shell_lastinstalled_resolves_only_within_the_current_ghost`・番人の補助 `plant_ghost` で本体の 3（`hidden_shell_by_name_and_the_current_one_are_candidates`・`balloon_lastinstalled_resolves_only_when_in_the_catalog`・`balloon_candidates_put_the_ghost_inner_first`） | `NO_MEMO`・`by_name`・`folder`・`never` → 新 `shell_balloon_resolve_test_support.rs` |
| `shell_carry_tests.rs` | 4 | `nothing_is_carried_without_a_ghost_reload`・`replay_shows_the_decided_surfaces_and_binds_through_seriko`・番人の補助 `names_of` で本体の `decide_binds_*` 6 | `set` → 新 `shell_carry_test_support.rs` |
| `wire_tests.rs` | 2（新 `areka-runtime/src/wire_tests.rs`・`lib.rs` が宣言） | `wire_runtime_falls_back_to_unwired_on_missing_ghost_root` | ― |
| `strict_judges_tests.rs` | 4（先頭の 3 と `long_script` の 1） | `doc_tables` の 4（`crate::log_history`・`crate::mcp`） | ― |

- 割らなかったもの: `switch_cue_tests.rs` は 10 本とも番人の補助（`emit_one`・`warns`）か本体の `SkinKind` を使うので全部本体。`frame_chain_rule_tests.rs` は 15 本とも本体の `frame_chain_finalize_align_tests` の `spawn_resolved`（`frame_chain_finalize_persist_test_support::spawn_shown` を呼ぶ）か `PersistStore` を使うので全部本体（D8 の段 1 の注のまま）。`frame_shell_box_align_tests.rs` は親の `Cage`（本体の `gpu_frame_world`・`seat_ghost`）を使うので全部本体。ほかの本体のファイル（`frame/switch_*`・`frame_chain_finalize_*`・`frame_visibility_integration` の一族・`balloon_visibility_phase_{notice,reappear}_tests`・`balloon_visibility_lifecycle_e2e_tests`・`film_playback_e2e_tests`・`frame_attach_carry_tests`・`frame_stay_on_top_lifecycle_tests`・spine の子）は全部の関数が本体を引く。
- 数: `areka-runtime` へ戻った関数は 74 本（15 ファイルを割り、`areka-runtime` に同じ名前のテストのファイルを 15 足した）。本体の `runtime_tests/` に残ったテストは 241 本（落ちの報告の確かめ 1 本を含む）。
- `frame_chain_realign_adjacency_tests` の `pub(super)` の補助 5 つ（`GAP_WITHOUT_REALIGN`・`low_sizes`・`boot_remembered_at_high`・`lower_the_scale`・`gap`）は、それが使う私有の 11（`sizes`・`LOW_SIZE_{0,1}`・`HIGH_SIZE_{0,1}`・`HIGH_DPI`・`LOW_DPI`・`boot_remembered_at`・`change_scale`・`char_pos`・`char_width`）と一緒に新 `frame_shared_test_support.rs`（`frame.rs` が `#[cfg(any(test, feature = "test-support"))] #[path = "frame_shared_test_support.rs"] pub mod shared_test_support;` で宣言）へ `pub` で出した。`frame_test_support.rs`（955 行）には足していない。`frame_chain_realign_adjacency_tests`・`frame_chain_realign_exclusion_tests`・本体の `frame_chain_finalize_remembered_tests` は `super::shared_test_support::{…}` で引く。例外表には触れていない。
- 新しい支えのファイルは 7（`consumer_ledger_test_support.rs`・`frame/status_report_test_support.rs`・`frame/window_state_test_support.rs`・`frame_shared_test_support.rs`・`source_text_test_support.rs`・`shell_balloon_resolve_test_support.rs`・`shell_carry_test_support.rs`）。どれも番人の crate を引かず、`cargo check -p areka-runtime --lib --features test-support`（機能だけが立つビルド）で警告 0。

#### 開けた項目（⑴〜⑷）

⑴ `pub(crate)` → `pub`（本体のテストが使うもの・2.5 の注つきを含む）:

- WIP で開けて残したもの: `assets::{build_shell_assets_with_limits, build_balloon_assets_with_limits}`・`BalloonHover::is_balloon_hovered`・`balloon_visibility` の `decide`／`configured_timeout_secs` の再公開と定義・`VisibilityTrigger`・`SuppressionKinds`（2.5）・`MeasurementDiscardReason`（2.5）・`VisibilityLogEvent`・`MeasurementOrigin`・`VisibilityAction`・`VisibilityDecision`（2.5）・`ScopeObservation`（2.5）・`VisibilityObservations`・`BalloonVisibilityState`・`balloon_visibility::test_support`（`any(test, feature)`）と中の関数と `Frame`・`run_balloon_visibility_phase`・`balloon_visibility_phase::test_support` と `attach_headless`・`frame::status_report`（モジュール）・`BalloonStatusLedger`・`StageObservation`・`BalloonObservation`・`run_status_report_phase`・`report_stages`・`report_balloons`・`StayOnTopState`・`run_window_state_phase`・`run_move_drain_with`（`any(test, feature)`）・`ShownRatioSource`（2.5）・`ScaleEventSeam`・`RuntimeWiring` の欄 11（`presenter`・`lifecycle_rx`・`balloon_visibility`・`balloon_status`・`balloon_models`・`balloon_definitions`・`balloon_faces`・`clock`・`swap_slot`・`shell_carry`・`static_binds`）とメソッド 4（`attach_window_state_rx`・`drain_lifecycle_signals`・`drain_received`・`balloon_model_scopes`。後の 3 つは `any(test, feature)`）・`lib.rs` の `shell_box_assets`・`DescriptParts::ALL`・`ScopeScaleTable::set`・`percent_text`・`NO_BODY`・`LastInstalledShell`・`LastInstalledBalloon`・`ShellBoxAssets::hand_to`・`settle_box_hover`・`CarriedShow`・`BindReplay`・`CarriedBinds`・`shell_carry::{decide, decide_binds, snapshot, replay}`・`ShellCursorAssets::hand_to`・`ArekaStrictJudges`・`SwapPayload`・`SwapSlot`。
- この段で足したもの: `CarriedBinds::{replays, dropped}`・`BindReplay::{category, part, on}`・`VisibilityDecision::{actions, logs, timeout_notice}`・`VisibilityObservations::{scopes, lifecycle, dragging}`・`ScopeObservation::{visible_glyphs, visible, hover, box_showing}`・`ScopeVisibility` と欄 3・`GlyphObservation`（2.5: 公開の `ScopeObservation::visible_glyphs` が名指す）と欄 2・`SuppressionKinds::{dragging, hover}`・`ScaleEventSeam` の欄・`BalloonVisibilityState::forget_scope`・`ArekaStrictJudges::canonical`・`RuntimeWiring::drain_lifecycle_signals`（`#[cfg(test)]` → `any(test, feature)`）。
- 案 C で要らなくなって戻したもの: `status_report::collect_bindings`・`window_state::{Transition, decide_transition}`・`RuntimeWiring::drain_move_directives`（`pub(crate)` へ）。

⑵ 家の型 `test_open`（中で tracing を呼ばない項目）: `balloon_visibility_phase.rs` の `IssueOutcome`（欄も `pub`）・`issue_actions`・`clear_hover_residency`、`frame/switch.rs` の `run_switch_phase`（討議の結果 3）・`split`、`frame/drain_resnap.rs` の `realign_chain_once_with_source`、`frame/work_area_sync.rs` の `resnap_for_work_area_change`。説明の 1 行は `areka-placement` と同じ文型。`mod test_open` の中の `trace!`〜`error!`・`#[instrument]` の grep は 4 ファイルとも 0 件。

⑶ 宣言の印の切り替え: `balloon_visibility::phase`・`frame::{drain_resnap, scale_notice, switch, wiring, window_state, work_area_sync}`（WIP のまま）。`frame::scale_text` の切り替えは本体が使わなくなったので外した。

⑷ 渡し口（`#[cfg(feature = "test-support")]` の `pub fn`・使う所の呼び方を書き換えた）:

| 型・モジュール | 開けた私有の項目 | 渡し口 |
|---|---|---|
| `balloon_visibility_phase`（中で記録を出す関数） | `collect_observations`・`observe_scope_boxes`・`notify_timeout`・`emit_visibility_logs` | 同名 4（`balloon_visibility_phase_test_support.rs`・本体の代わりの親が同名で取り込む） |
| `frame::drain_resnap`（同） | `finalize_chain_once_with`・`resnap_with` | 同名 2（`drain_resnap::test_support`・同） |
| `frame::work_area_sync`（同） | `sync_monitor_snapshot_with` | 同名 1（`work_area_sync::test_support`・使う 2 か所を書き換え） |
| `frame::status_report`（同） | `report_observed` | 同名 1（`status_report_test_support.rs`・代わりの親が同名で取り込む） |
| `BalloonStatusLedger` の欄 | `last_sent`・`last_stage`・`runtime_busy_logged` | `last_sent`（`&mut`）・`last_stage`・`runtime_busy_logged` |
| `BalloonVisibilityState` の欄 | `per_scope`・`display_end`・`observation_failure_logged.runtime` | `per_scope`（`&mut`）・`display_end`・`observation_failure_logged_runtime` |
| `RuntimeWiring` の欄 | `zorder_ledger`・`window_state`・`last_drain` | `zorder_ledger`・`zorder_ledger_mut`・`window_state`・`window_state_mut`・`last_drain_mut`（`frame::wiring::test_support`） |
| `StayOnTopState` の欄 | `on` | `on`・`on_mut`（`window_state_test_support.rs`） |
| `ScriptedShioriHandle` の欄 | `status_calls` | `status_calls`（`spine::test_support`） |

- ⑷ の数: 開けた私有の項目は 19（tracing を呼ぶ関数 8・欄 11）、渡し口の関数は 22。案 C の前に数えた 28（欄 19・関数 9）から、`StayOnTopState::rx`・`ConsumerLedger::table`・`report_stage_observed`・`reconcile_reported_sizes` が要らなくなり、`RuntimeWiring::{runtime, attached}` は既にある公開の読み口（`runtime()`・`attached()`）へ書き換えた（渡し口は作らない）。足したのは `sync_monitor_snapshot_with` と `ScriptedShioriHandle::status_calls`（どちらも案 C の前の数に入っていなかった）。
- ⑷ を使う所の書き換えのうち、欄の構造体の式 1 つ（`status_report_tests.rs` の `BalloonStatusLedger { last_sent: …, ..Default::default() }`）は既定値を作ってから渡し口で書く 2 行にした。
- 完了の状態: `cargo check -p areka --bin areka --tests` の可視性のエラーは 0 件（警告も 0）。

### 4.3 ログの捕まえ方と口の据え付け（タスク 4.3）

- `capture_logs` → `capture_lines(LineFormat::LevelTargetFields, …).1`（`use log_capture_kit::{LineFormat, capture_lines};` を 1 行足す）: `frame_move_order_tests`（本体に残った 1 本の 1 か所）・`frame_chain_finalize_restart_tests`（8）・`frame_chain_finalize_remembered_tests`（1）・`frame_chain_finalize_clamp_tests`（1）・`frame_chain_rule_tests`（2）。後の 2 つは D8 で本体に置いたファイル（設計の 4 本に足した）。`spine_third_scope_talk_tests` は本体の `runtime_tests/spine.rs`（⑶）が持つ自分の `capture_logs` を使うので書き換えていない（`frame::test_support::capture_logs` ではない）。新しい包みは作っていない。
- 本体の `src` で `frame::test_support` の `capture_logs` を使う所は 0 件（使い手は `areka-runtime` の `#[cfg(test)]` の中だけ）。
- 口の据え付け（`crate::session::hooks::install(&mut …);`）: 据えた所と数。

| ファイル | 数 | 据えた所 |
|---|---|---|
| `balloon_visibility_phase_notice_tests.rs` | 3 | 補助 `world_with_ghost` と、置き場の無い・空の World を組む 2 か所 |
| `frame/status_report_tests.rs` | 2 | 補助 `world_with_ghost`・`empty_slot_skips_with_debug_and_keeps_the_ledger` の World |
| `frame/status_report_stage_tests.rs` | 2 | 補助 `world_with_seriko`・`empty_slot_in_the_phase_keeps_the_ledger` の World |
| `frame/switch_finish_tests.rs` | 1 | `rig_on`（子の `cursor`・`face`・`reload`・`stay_on_top`・`box` もここを通る） |
| `frame/switch_tests.rs` | 1 | 組み立ての補助の World |
| `frame/window_state_tests.rs` | 4 | 補助 `world_with_wiring` と、`areka-runtime` の支え `world_with_windows` で組む 3 本（支えの中に本体の口は書けない） |
| `frame_attach_carry_tests.rs` | 1 | `gpu_attach_world()` の直後 |
| `frame_move_order_tests.rs` | 1 | 本体に残った `a_script_reset_sends_nothing_to_kanade`（`steady_harness()` の直後） |
| `frame_visibility_integration_tests.rs` | 1 | 補助 `gpu_frame_world`（子の `shell_box_integration`・`align`・`arrow`・`balloon_timeout_notice_e2e`・`scale_notice_integration` もここを通る） |
| `shell_carry_tests.rs` | 1 | `replay_shows_the_decided_surfaces_and_binds_through_seriko` の World |
| `session/readme_script_e2e_tests.rs` | 1 | `run` の World |
| `runtime_tests/spine.rs`（`SpineHarness::boot_with`） | 1 | `make_world_with_gpu()` と窓の一式の直後 |

- 設計の 15 本のうち据えなかったもの: `frame_stay_on_top_lifecycle_tests`（`SwitchRig::new` が本番の `register_systems` で口を据える＝組み立てが本体の関数の中で 1 回）・`shell_balloon_resolve_tests`（本体に残った 4 本の相は口を読まない）・`frame_scale_notice_integration_tests`・`frame/switch_reload_tests`（親の補助を通る）。1 ファイル 1 回にならなかったのは、案 C で World の組み立ての補助が `areka-runtime` の支えへ移ったファイルと、置き場の無い World を自分で組むテストがあるファイル。
- `ghost_session_{reload,restart}_tests` は `crate::runtime_tests::spine::SpineHarness` を使う（ほかの spine の項目は今の道のまま・段 4 で書き換える）。
- 本体の部品を偽物に替えていない。

### 4.4 落ちの報告（タスク 4.4）

- `runtime_tests/mod.rs`（`main.rs` の `#[cfg(test)] mod runtime_tests;` の下）に `#[used]`・`#[unsafe(link_section = ".CRT$XCU")]`・`static INSTALL: extern "C" fn() = areka_runtime::crash_report_test_support::install;` の 3 行と `#[test] fn crash_filter_is_installed_before_the_tests_start()`（`installed()` が真）の 1 本（WIP のまま）。
- grep（`crates/areka-runtime` と `crates/areka` で `link_section = ".CRT$XCU"`）: `crates/areka-runtime/src/crash_report_test_support.rs:130`（すぐ上が `#[cfg(test)]`・`#[used]`）と `crates/areka/src/runtime_tests/mod.rs:18`（モジュールが `#[cfg(test)]`）の 2 か所だけ。

### 4.5 段 3 を緑にする（タスク 4.5）

- 出所の名前で選ぶテスト: `assets_tests` の `target=areka::runtime::balloon_background ` は、関数が `areka-runtime` へ戻ったので `areka-runtime/src/assets_tests.rs` の側で `target=areka_runtime::balloon_background ` にした。ほかに `areka::runtime` の綴りは 0 件。
- 割ったことで読み先が動いた文字で読むテスト: 本体の `placement_tests/measure_template_tests.rs` の較正（外れ）が読む `boot_read_honours_the_files_own_charset_declaration` の説明文が `areka-runtime` へ戻ったので、`include_str!("../runtime_tests/assets_tests.rs")` を `include_str!("../../../areka-runtime/src/assets_tests.rs")` へ一時につなぎ直した（最終形は段 5 の 6.2）。`include_str!` の `../../../areka-runtime/src/` は `areka-runtime` へ戻したファイルの側で同じファイルの名前へ直した（`zorder_wiring_tests`・`frame_schedule_tests`・`frame_move_order_tests`）。
- 1,000 行: `frame/switch_finish_tests.rs` は `.runtime` → `.runtime()` の書き換えで rustfmt が長い鎖を折り、1,010 行になった。D9 の形で、箱の束の 2 本（`shell_switch_hands_the_new_box_bundle_to_the_text_layer`・`shell_switch_falls_the_text_layer_back_to_the_bundle_candidates`）と補助（`OLD_SHELL`・`NEW_SHELL`・`OLD_SHELL_WITH_20`・`box_bundle`・`cue`・`texts`）を子の兄弟 `frame/switch_finish_box_tests.rs`（`mod box_tests;`）へ出した（818 行・201 行）。例外表には触れていない。
- `crates/areka/Cargo.toml` の `shiori-host32-host` の dev-dependency の説明を今の使い手（背骨の足場と、台本の SHIORI で起動を組む本体のテスト）に合わせた（行は残した）。
- `session_hooks_missing` の確かめ: `areka-runtime` の `SessionHooks::of` の `#[cfg(not(test))]` の枝に一時の `eprintln!`（テストの名前を出す）を入れ、`cargo test -p areka --bin areka -- runtime_tests session:: ghost_session_ --nocapture` を回した。据える前は 302 行（43 本が赤）、据えた後は 0 行（456 本緑）。一時の行は外した（`PROBE` の grep 0 件・`session_hooks.rs` の差分 0）。

走行の結果:

- `cargo check -p areka-runtime --all-targets`・`--all-targets --features test-support`・`--lib --features test-support`・`--lib`: 緑・警告 0。
- `cargo check -p areka --bin areka --tests`: 緑・警告 0。`cargo check --workspace --all-targets`: 緑（警告は前からの `shiori-abi` の manifest の 1 件だけ）。
- `cargo test -p areka-runtime`: 989 本緑・3 本は前からの `#[ignore]`・赤 0（`crash_filter_is_installed_before_the_tests_start`・`initializer_keeps_a_process_lifetime_mta` を含む）。
- `cargo test -p areka --bin areka`: 2,212 本緑・赤 3・`#[ignore]` 3。赤だけを回し直した:
  - `placement_tests::measure_template_tests::the_source_scan_drops_a_spelling_that_only_lives_in_a_comment`: 上の読み先を直して緑（4 本緑）。
  - `tick_gate_config::producers_tests::{the_areka_table_lists_every_file_that_marks_the_wake, the_wintf_module_roster_names_every_areka_zorder_producer}`: 赤のまま。本体の `src` を `CARGO_MANIFEST_DIR` から歩いて旗を立てる本番ファイルを探すが、生産者は段 1 で `areka-runtime` へ移った（走査が 0 件）。読み口へ寄せるのはタスク 6.2 の持ち物なので、この段では触っていない。
- `cargo test -p areka --bin areka -- crash_filter_is_installed_before_the_tests_start`・`runtime_tests`・`session::runtime_boot`・`ghost_session_reload`・`ghost_session_restart`: 緑（上の全体の走行と確かめの走行の中）。
- `cargo test -p log-capture-kit --test file_length_guard_test --test workspace_scan_test`: 6 本・20 本緑。
- `cargo fmt --all -- --check`: 差分 0（WIP で並びが崩れていた `use` の行の並べ替えを含む）。
- `cargo clippy -p areka-runtime -p areka --all-targets`: この段で書いたファイルに新しい警告は 0（出ているのは移す前からの形: 引数の数・`assert_eq!` の bool・型の複雑さ）。
- テストの名前の集合: 段 0 の一覧（`tests-before.txt`）の `runtime::`・`placement_tests::` の 1,515 件のうち areka のもの 1,259 件（葉の名前の多重集合・ほかは `dola`・`wintf`・`areka-ghost` の同名のモジュール）が、後の `cargo test -p areka-runtime -- --list`（992 件）と `cargo test -p areka --bin areka -- --list`（2,218 件）に欠けずにある。後に新しい名前は 0。

### 4.6 設計と違った所

- 番人の crate を引く補助を両方が使う関数（上の表の「番人の補助で本体」の 12 本と `switch_cue_tests.rs` の 5 本・計 17 本）は、写しを作らず `test-support` にも置けないので本体に残した（D4 の規則）。
- 口の据え付けは 1 ファイル 1 回にならなかった（4.3）。
- `frame/switch_finish_tests.rs` を D9 の形で割った（4.5）。
- タスク 6.2 の文は `assets_tests.rs` を本体の `../runtime_tests/assets_tests.rs` で読むとしているが、較正の当たりの説明文は `areka-runtime/src/assets_tests.rs` へ戻った（4.5）。

## 5. 段 4（本体の道を書き換え、一時の別名を外す）

非挙動・機械的な道の書き換え。対象は `crates/areka/src/**` だけ。

- 赤（書き換え前の数え）: `crate::runtime::` 209 行・`use areka_runtime as runtime;` 1 行（`main.rs`）・`use crate::runtime;` 1 行（`ghost_session.rs`）・別名経由の裸の `runtime::` 105 行（コードと doc コメント）。
- 書き換え: `crate::runtime::` と別名経由の `runtime::` を `areka_runtime::` へ。`use crate::runtime;`（`ghost_session.rs`）と `main.rs` の一時の別名（宣言と注釈）を削除。`main.rs` で別名の宣言に付いていた古い `runtime 統合結線` の doc 段落（旧 `mod runtime` の説明）が `mod session;` に連なって残ったので、これも削除。変えたファイル数: 105（`git diff --stat`・235+/245-）。
- 触っていない形: `crate::runtime_tests::…`・`session::runtime_boot`・`ghost_session_runtime*` など、本体の中の名前に `runtime` が入るだけの道。
- `target_map` を使う本体（`input_events/user_break.rs`・`mcp/{check_script,dump_balloon,dump_surface}.rs` とそのテスト）は `areka_runtime::target_map::…` で同じ値に届く。
- 緑（後の数え、`crates/areka/src`）: `crate::runtime::` 0・`areka_runtime as runtime` 0・`use crate::runtime;` 0・別名経由の裸の `runtime::` 0。
- `cargo fmt --all -- --check`: 差分 0。`cargo check --workspace --all-targets`: 緑・新しい警告 0（`shiori-abi` の `windows` 未使用は前からの警告）。
- `cargo test -p areka --bin areka -- mcp:: input_events::user_break`: 276 本緑。`cargo test -p log-capture-kit --test file_length_guard_test --test workspace_scan_test`: 6 本・20 本緑。
- 触っていない: `tick_gate_config_producers_tests.rs`（6.2）・`placement_tests/mod.rs` の doc（段 7）。

## 6. 段 5（文字で読むテスト・examples・配置のテストだけの公開）

非挙動。タスク 6.1〜6.5。

### 6.1 examples（6.1）

- 赤（移す前）: `crates/areka/examples/collision-probe.rs` と `collision-probe/` 7 ファイルが本体にあり、`collision-probe.rs` が `#[path = "../../areka-runtime/src/{target_map,hit_region}.rs"]` で 2 モジュールを私有に取り込んでいた（段 1 の一時の道）。
- 移し: `git mv` で `crates/areka-runtime/examples/collision-probe.rs` と `collision-probe/` へ。`#[path]` の 2 つの私有 include を外し、`use areka_runtime::hit_region::resolve_hit_region;` と `use areka_runtime::target_map;` に替えた（子の `super::target_map`・`super::resolve_hit_region` はルートへ持ち込んだ名前へ解決される）。頭の説明の「bin crate ゆえ `#[path]` で取り込む」段落は「crate の公開面で読む」に書き直し、起動コマンドを `cargo run -p areka-runtime --example collision-probe` へ直した。子ファイルの接続 7 本の `#[path]` は example のルートの規則（素の `mod` は新しい example の対象を生む）のため残した。
- `pub` にした項目: 0 件（`target_map`・`hit_region` は既に `pub mod`・`areka-placement` は公開面を使っていた）。
- 緑: `cargo check -p areka-runtime --examples`・`cargo check -p areka --examples`: 警告 0。

### 6.2 本体の文字で読むテスト（6.2）

- 赤: `tick_gate_config::producers_tests::{the_areka_table_lists_every_file_that_marks_the_wake, the_wintf_module_roster_names_every_areka_zorder_producer}` が段 1 から赤（本体の `src` を走査して生産者を探すが、生産者は `areka-runtime` へ移った）。
- 読みの変更:
  - `tick_gate_config_producers_tests.rs`: 表の `include_str!("../../areka-runtime/src/…")` 9 か所を `crate_source!("areka-runtime", "src/…")` へ。走査の根を `CARGO_MANIFEST_DIR/src` から `crate_path("areka-runtime", "src")` へ。表の見出しを `runtime/…` から runtime の `src` からの相対（`adapter.rs`・`frame/scale_text.rs` ほか）へ直し、照合を `ends_with` から等号へ（照合は弱めていない）。本体の `src` に `tick_wake::mark` を呼ぶファイルは 0 件（`input_events/char_watch.rs` は `arm_deadline` だけ）なので、走査は runtime の木だけで足りる。
  - `measure_template_tests.rs`: `frame_test_support.rs`・`assets.rs`・`examples/collision-probe/setup.rs` を `crate_source!("areka-runtime", …)` へ。`emo-present/setup.rs` は本体のまま相対。
  - 設計と違った所: タスク 6.2 の文は `assets_tests.rs` を相対の `../runtime_tests/assets_tests.rs` で読むとするが、段 3 の案 C で較正の当たりの説明文は `areka-runtime/src/assets_tests.rs` へ戻ったので、`crate_source!("areka-runtime", "src/assets_tests.rs")` で読む（段 3 の一時の `include_str!("../../../areka-runtime/…")` の置き換え）。
  - 分岐の対応表 2 本（`zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`）の runtime の行 12 か所: 段 3 の一時の相対（`../../../areka-runtime/src/…`）から `crate_source!("areka-runtime", …)` へ。
  - `zorder_wiring_tests.rs`: runtime の `lib.rs` の読みを `crate_source!` へ（本体の `session/runtime_boot.rs`・`ghost_session.rs`・`main.rs` は相対のまま）。`frame_schedule_tests.rs`（本体の `session/ghost_quit.rs` だけ）・`frame_move_order_tests.rs`（文字で読む所なし）は変更なし。
  - `crate_source_test_support.rs` の説明の名前を付けた例外を `areka-runtime` の `src/frame_attach_tests.rs` へ。
- grep（`crates/areka/src`、読み口と名前を付けた例外の外）: `"(\.\./)+(wintf|areka-|dola|shiori|sylphya|sample|log-capture)` と `areka-runtime` の字面の相対の読みは 0 件。
- 緑: `cargo test -p areka --bin areka -- tick_gate_config measure_template frame_move_order frame_schedule zorder_wiring zorder_group_branch zorder_inversion crate_source`: 21 本緑（赤だった 2 本を含む）。

### 6.3 `spawn_zorder_pair_deferred_tests`（6.3）

- 名簿（`src/zorder_cue.rs`・`src/frame/zorder_drain.rs`・`src/frame/zorder_descript.rs`・`src/window_state_cue.rs`・`src/frame/window_state.rs`）と較正の外れ（`src/frame/zorder_drain_test_support.rs`）は、段 3 でテストが `areka-runtime` へ移った時点で crate の根からの道（`CARGO_MANIFEST_DIR` = `areka-runtime`）になっており、書き換え不要だった。頭の説明の `runtime/…` の綴りと「本体」の語だけ直した。較正は自分の crate の正例で緑。
- grep 1: 本体 `crates/areka/src` に runtime の外の `zorder` の名の本番ファイル（`*zorder*` の名で `_tests.rs`・`test_support` を除く）は 0 件（名に `zorder` を持つ本体のファイルは分岐の対応表のテスト 2 本・その支え・`zorder_wiring_tests.rs` だけ）。
- grep 2: `crates/areka-runtime/src` で `"\.\./areka|"\.\./\.\./crates|crate_source` は 0 件（名前を付けた例外 `frame_attach_tests.rs` のワークスペースの走査もこの字面を含まない）。
- 緑: `cargo test -p areka-runtime -- spawn_zorder_pair_deferred frame_move_order frame_schedule zorder_wiring`: 14 本緑。

### 6.4 配置のテストだけの公開（6.4）

書いたのは `areka-placement` の 3 ファイル（`follow.rs`・`follow/work_area.rs`・`transition_diag.rs`）だけ。

| 項目 | 本番の使い手（crate の中） | 扱い |
|---|---|---|
| `on_balloon_drag` | `spawn.rs`（`super::follow`） | `test-support` なら `pub use`・なければ `pub(crate) use` |
| `sync_drag_writer` | `reseed`・`script_alignment`・`sticky_group_ledger` | 同上 |
| `AnchorSource` | `persist`・`reseed`・`script_alignment` | 同上 |
| `WorkAreaResolution`・`work_area_for_window_with_origin` | `follow/visibility.rs` | 同上 |
| `move_window_with_route` | `chain_realign`・`follow/move_shift.rs`・`window_move` | 同上 |
| `WORK_AREA_CLAMP_TAG` | 定義元 `work_area_clamp.rs` だけ（`follow` 経由の使い手は crate のテストだけ） | `#[cfg(any(test, feature = "test-support"))] pub use` の 1 行だけ（`pub(crate)` の行は未使用になるので置かない）。crate の自分のテスト（`follow_work_area_clamp_restore_tests.rs`）が引くので `test` も含めた |
| `MonitorSources::is_empty` | 0 件（`cfg` を付けて本番のビルドが通ることで確認） | 定義に `#[cfg(any(test, feature = "test-support"))]`。`len` に `clippy::len_without_is_empty` の許可を、同じ条件の否定で付けた |
| `MonitorDpiEntry` | 本体のテスト（公開の欄の要素の型） | 公開のまま（要件の討議 議題 4） |
| `transition_diag.rs` の `PLACEMENT_KIND_ALL`・`HOLD_DECISION_ALL`・`HOLD_SITE_ALL`・`CHAIN_STAGE_ALL`・`SNAPSHOT_FIELDS`・`HOLD_FIELDS`・`GROUND_FIELDS`・`CHAIN_FIELDS`・`OFFSET_FIELDS` | 0 件（判定器とテストだけ） | 定義に `#[cfg(any(test, feature = "test-support"))]`（定義が本番に使い手を持たないので、再公開の切り替えでなく定義に付ける） |
| `OFFSET_VERDICT_ALL` | `log_offset_rescale` の `debug_assert!`（本番のビルドが落ちて分かった） | 公開のまま |
| `KIND_*`・`FIELD_*`・`HOLD_DECISION_*`・`HOLD_SITE_*`・`CHAIN_STAGE_*`・`OFFSET_VERDICT_*`（`_ALL` 以外）・`*_line`・`stamp_of`・`write_tag`・`log_*`・公開の型の欄 | 本番が使う（行の組み立て・各発行点） | 公開のまま（本番が使う語・公開の型の欄は切り替えられない） |
| `live_work_area_bottom` | `log_char_ground`（crate の中の本番） | 公開のまま（外の使い手はテストだけだが、本番が使う関数で、同じ道・名前の切り替えには定義を私有のモジュールへ入れる必要がある。判定器の語彙ではない） |

- 緑: `cargo check -p areka-placement --lib`（機能なし）警告 0・`--lib --features test-support` 警告 0・`--tests` 警告 0・`cargo clippy -p areka-placement --lib` に `len_without_is_empty` なし。

### 6.5 走行（6.5）

- `cargo check -p areka-runtime --examples`・`cargo check -p areka --examples`: 警告 0。
- `cargo check --workspace --all-targets`: 緑（警告は前からの `shiori-abi` の 1 件だけ）。
- `cargo test -p areka --bin areka -- tick_gate_config measure_template frame_move_order frame_schedule zorder_wiring zorder_group_branch zorder_inversion crate_source`: 21 本緑。
- `cargo test -p areka-runtime -- spawn_zorder_pair_deferred frame_move_order frame_schedule zorder_wiring`: 14 本緑。`-- transition` を足した走行: 223 本緑・`#[ignore]` 2。
- `cargo test -p areka-placement`: 984 本緑・`#[ignore]` 1。
- `cargo test -p log-capture-kit --test file_length_guard_test --test workspace_scan_test`: 6 本・20 本緑。
- `cargo fmt --all -- --check`: 差分 0。

## 7. 段 6（1,000 行の上限を例外表を増やさずに守る）

非挙動。タスク 7。要件 1.10・設計 D9「行数」。数え方は番人（`file_length_guard_test.rs`・走査器 `LINE_LIMIT` = 1000）と同じで、1,000 を超えたものだけが超過。

### 7.1 赤（分割の前の実測）

| ファイル | 行数 | 上限の 1,000 に対して |
|---|---|---|
| `crates/areka/src/runtime_tests/frame/switch_finish_tests.rs` | 818 | 内（段 3 の案 C で割った後） |
| `crates/areka/src/runtime_tests/frame/switch_finish_box_tests.rs`（段 3 で新設） | 201 | 内 |
| `crates/areka-runtime/src/placement_tests/transition_judge.rs` | 999 | 内（1 行の余裕） |
| `crates/areka-runtime/src/placement_tests/transition_judge_tests.rs` | 999 | 内（1 行の余裕） |
| `crates/areka/src/main.rs` | 989 | 内 |
| `crates/areka-runtime/src/frame_test_support.rs` | 955 | 内 |
| `crates/areka-runtime/src/lib.rs` | 758 | 内 |
| `crates/areka-runtime/src/spine.rs` | 364 | 内 |
| `crates/areka/src/runtime_tests/spine.rs` | 659 | 内 |

- 段 3 で新設・移動したほかのファイル: `areka-runtime/src/assets_tests.rs` 810・`zorder_wiring_tests.rs` 421・`examples/collision-probe/probe.rs` 326・`examples/collision-probe.rs` 310・`frame/window_state_tests.rs` 302・`strict_judges_tests.rs` 248・`frame_move_order_tests.rs` 202・`frame_schedule_tests.rs` 184。すべて 1,000 以下。
- `crates/areka`・`crates/areka-runtime` の `*.rs` の最大は、例外表の 2 件（`frame_transition_branch_tests.rs` 1,271・`placement_tests/transition_judge_verdict_tests.rs` 1,024）を除き 999（`transition_judge.rs`・`transition_judge_tests.rs`）。

### 7.2 分割

- 超過しているファイルは 0 件のため、分割は行わない（`transition_judge_parse.rs` も作らない）。`transition_judge.rs` は 999 行で上限内。
- 例外表 `OVER_LIMIT_ALLOWED` の件数: 前 4・後 4（`OVER_LIMIT_ALLOWED_COUNT = 4`・項目も同じ）。増やしていない。

### 7.3 緑

- `cargo test -p log-capture-kit --test file_length_guard_test`: 6 本緑。

## 8. 段 7（8.1・8.2）

非挙動。タスク 8.1・8.2。要件 3.8・4.3・5.8〜5.10・6.2〜6.4・6.6・6.8・7.3・7.8。赤は書き換えの前の grep の件数。ログの本文は 1 文字も変えていない（変えた行は出所の値・説明・テストの見本だけ）。

### 8.1 ログの出所の名前（4 ファイル）

赤（書き換えの前）:

| grep | 件数 |
|---|---|
| `rg -n 'areka::runtime' crates/areka/tests/stay_on_top_real_run.rs crates/areka/src/log_history_tests.rs crates/areka/tests/emo2_real_run/common_tests.rs doc/ssp-mcp/log-convention.md` | 8（`stay_on_top_real_run.rs` 51〜54 行に 6 つ・`log_history_tests.rs` 124 行・`common_tests.rs` 28 行・`log-convention.md` 135 行） |

書き換え（値だけ・ほかの行は触れていない）:

- `stay_on_top_real_run.rs`: `RUST_LOG` の `areka::runtime::<道>=debug` 6 つを `areka_runtime::<道>=debug` へ（`window_state_cue`・`zorder_cue`・`frame::window_state`・`frame::zorder_drain`・`frame::zorder_descript`・`frame::switch`。6 つとも `areka-runtime` に同じ名前のモジュールがあることを確かめた）。
- `log_history_tests.rs` 124 行: `classify(Level::INFO, "areka_runtime")` が `None`（`areka` は完全一致だけの規則・`areka_runtime` はどの規則にも当たらない）。
- `emo2_real_run/common_tests.rs` 28 行: 見本の行の出所を `areka_runtime`。
- `doc/ssp-mcp/log-convention.md` 135 行: 例の `areka::runtime` を `areka_runtime`。
- 4 ファイルの中の古いパス（`src/runtime`・`runtime/`・`crate::runtime`）は 0 件で、直す所は無かった。
- `log_history.rs` の `RULES` は変えていない（差分 0）。runtime を名指す行は元から無く、警告以上は `classify` が出所を見ずに `error` へ振る（`warn_and_error_go_to_error_regardless_of_target`）ので、分け方は前後で同じ。

緑（書き換えの後）:

| 確かめ | 結果 |
|---|---|
| `cargo test -p areka --bin areka -- log_history` | 43 本緑 |
| `rg -n 'areka::runtime(::\|=\|\b)' crates tools` | 0 件 |
| 6.2（直書きの出所）: `rg -n 'target\s*:' crates/areka-runtime/src crates/areka-runtime/examples`（`*_tests.rs`・`*test_support*`・`placement_tests/` を除く） | 当たりは構造体の欄名・引数名だけで、tracing の `target:` の文字列は 0 件 |
| 6.3（語彙の出所）: `rg -n '"areka(::\|_)[a-z_:]*"\|areka::runtime\|areka::emo2' crates/areka-runtime/src`（同じ除外） | 0 件。`TARGET_SCRIPT`（`areka_kanade::script_log::TARGET`）・`TARGET_ERROR`（`"areka::log::error"`）は `log_history.rs`（本体）にだけ在り、runtime の本番に 0 件 |

### 8.2 `crates/` の説明とテストの値（8.1 の 4 ファイルを除く）

赤（書き換えの前）: `rg -n 'emo2_boot|Emo2Wiring' crates -g '!crates/areka/**'` は 14 ファイル 21 行（`areka-parsers` 2・`areka-ghost` 2・`areka-seriko` 2・`wintf` 1・`areka-emo-text` 2・`areka-kanade` 5・`log-capture-kit` 6〔`event.rs` 1・`event_tests.rs` 5〕・`ukadoc-survey` 1）。`crates/areka/src/runtime_tests/spine.rs` 37 行に `tests/emo2_boot_spine_test.rs` が 1 行。

書き換え:

- 古い名前: 説明の行は今の道（`areka_runtime::adapter`・`areka_runtime::frame`・`areka_runtime::hit_region`・`areka` の `session::reload`）と移した先のファイルへ。`log-capture-kit/src/event_tests.rs` の出所の値 5 行（と期待の文字列 1 行）は `"areka_runtime"` へ。`actor_lookahead_tests.rs` の関数は `runtime_talk_reveals_without_moving_glyphs` へ改名。`runtime_tests/spine.rs` 37 行は完了した spec `areka-P0-emo2-boot` の名前で指す説明へ（ファイル名は書かない）。
- 古いパス（`crates/areka/src/runtime/…`・`crates/areka/src/placement_tests/transition_*`・`runtime/frame/…` ほか）は、`areka-placement`（12 ファイル）・`wintf`（`test_crash_report.rs`・`runtime/tick_bridge.rs`・`ecs/window/{zorder_chain,zorder_pair_test_support,mod}.rs`・`ecs/world/tick_wake.rs`）・`areka-emo-present`（`presenter.rs`・`presenter/{refresh,transition_record}.rs`）・`areka-emo-text`・`shiori-host32-host`・`ukadoc-survey`・`areka-runtime` 自身（`talk_lifecycle.rs`・`balloon_visibility.rs`・`balloon_background_tests.rs`・`frame/wiring.rs`・`frame_transition_branch_tests.rs`・`placement_tests/transition_judge_offset_signoff_tests.rs`）・本体（`input_events/balloon.rs`・`session/ghost_quit.rs`・`placement_tests/measure_template_tests.rs`・`shell_balloon_switch_session_balloon_tests.rs`・`runtime_tests/spine_conformance_support_stage_tests.rs`・`tests/emo2_real_run.rs`）で、`crates/areka-runtime/src/…`・`crates/areka/src/runtime_tests/…` の今の場所へ。
- `areka-placement/src/test_crash_report.rs`・`wintf/src/test_crash_report.rs` の 1 行目の「写しの相手」は `crates/areka-runtime/src/crash_report_test_support.rs`。
- `frame_transition_atomicity_tests.rs` 57・150 行の `transition_judge.rs:609-613` は「`summarize` の `KIND_WRITE` の腕（`path_a_writes` を数える所）」と名前で指した（行番号は書かない・同じ行の中で直したので行数は変わらない）。
- `spine_conformance_support_stage_tests.rs` 37 行の `runtime/spine.rs:490-499` は、移す前から行がずれていた（そこは `titles()`）。「`SpineHarness::new` が検体の複製 `SampleRoot` を毎回作る」と名前で指す形へ。
- `hit_region.rs` 48〜55 行: probe のパス（`crates/areka-runtime/examples/collision-probe.rs`）・今の probe が crate の公開面で読むこと・`runtime/mod.rs` → `lib.rs` だけを直した。「`crate::` フリー規律」の文の意味は変えていない（spine の監査 8.5 へ申し送り）。
- `areka/src/placement_tests/zorder_{group,inversion}_branch_coverage_tests.rs` の見出しの値 `"runtime/…"` は `"areka-runtime/…"` へ（見出しは表示にだけ使い、`rsplit('/')` で最後のファイル名を取る使い方は同じ）。
- 手を付けなかった `runtime/…`: `wintf` 自身の `runtime/{tick_bridge,mod,window_factory,window_registry}.rs`・`dola` 自身の `runtime/…` と `tests/runtime/…`・`wintf/src/ecs/dola/mod.rs` の dola の `runtime/…`。`areka-placement` ほかの「`runtime::frame`」のような**モジュールの道**の言い方は、今の名前（`areka_runtime::frame` の道と同じ綴り）のまま。`areka_runtime` の `crates/areka-runtime/src/…` は `[^/]runtime/…\.rs` の grep に `-runtime/` で当たるが、旧いパスではない。
- `transition_judge.rs`・`transition_judge_tests.rs` はどちらも 999 行のまま（今回は触れていない）。変更はどのファイルでも行数が変わらない（差分の挿入と削除が同数）。分割は不要で、例外表も触れていない。

緑:

| 確かめ | 結果 |
|---|---|
| `rg -n 'emo2_boot\|Emo2Wiring' crates -g '!crates/areka/**'` | 0 件 |
| `rg -n 'emo2_boot' crates`（本体を含む） | 0 件 |
| `rg -n 'crates/areka/src/runtime/\|src/runtime/\|placement_tests/(transition_judge\|transition_signoff\|reset_balloon_pos\|measure_bake_log\|spawn_zorder_pair_deferred)' crates`（`wintf`・`dola` の自分の `runtime/` を除く） | 当たりは新しい置き場を名指す 4 行（`areka-placement` と `frame_transition_branch_tests.rs` の説明 2 行・番人の表の項目 1 行・`transition_signoff_procedure_tests.rs` の `src/placement_tests/…` の相対の値 1 行）だけ。旧いパスは 0 件 |
| `cargo test -p log-capture-kit` | 全部緑（`file_length_guard_test`・`event_tests` を含む） |
| `cargo test -p areka-emo-text -- runtime_talk_reveals_without_moving_glyphs` | 1 本緑 |
| `cargo check --workspace --all-targets` | 緑。警告は既存の `shiori-abi` の `windows` の 1 件だけ（新しい警告 0）。`stay_on_top_real_run.rs`・`emo2_real_run/common_tests.rs` はここで組むだけで実行していない |
| `cargo fmt --all -- --check` | 差分 0 |

## 9. 段 7（8.3・8.4・8.5）

非挙動（文書と steering と brief の書き換えだけ）。対応する要件: 2.5, 4.8, 5.5, 5.6, 5.7, 5.9, 7.1。本番のコード・テスト・`tools/` は 1 行も書いていない。

### 9.1 8.3 文書の中の runtime のパス

- 赤（書き換える前・`HEAD` の `git grep`）: 設計の Verification の式（`crates/areka/src/runtime/|src/runtime/|placement_tests/(transition_judge|transition_signoff|reset_balloon_pos|measure_bake_log|spawn_zorder_pair_deferred)|areka::runtime(::|=|\b)|emo2_boot|Emo2Wiring`）を 8.3 のファイル 10 本（`click-through.md` 含む）へ回して 99 行・10 ファイル（`COMPAT_ARCHITECTURE.md` 41・`sakura-script.toml` 34・`shiori.toml` 7・`property.toml` 6・`briefing-assets.md` 5・`areka-tools.md` 2・`click-through.md` 1・`emo2-conformance-scope.md` 1・`briefing-shiori.md` 1・`assets.toml` 1）。`crates/areka/src/runtime/<ファイル>.rs` の字面は 124 か所。`src/` を付けない書き方（`runtime/x.rs`）の当たりは 11 行。
- 書き換え（機械＋目）: `crates/areka/src/runtime/<道>.rs` を、`crates/areka-runtime/src/<道>.rs` に実在すればそこへ、なければ `crates/areka/src/runtime_tests/<道>.rs` へ（`mod.rs` は `lib.rs`）。未解決は 0。書き換えた字面は 124 か所（`COMPAT_ARCHITECTURE.md` 60・`sakura-script.toml` 34・`shiori.toml` 9・`property.toml` 9・`briefing-assets.md` 7・`areka-tools.md` 2・`emo2-conformance-scope.md` 1・`briefing-shiori.md` 1・`assets.toml` 1）。
- 案 C で同じ名前のテストのファイルが `areka-runtime` と本体の `runtime_tests/` の両方にあるもの（`frame/window_state_tests.rs`・`shell_balloon_resolve_tests.rs`・`strict_judges_tests.rs`）は、名指されたテストの関数がどちらにあるかを `fn` の名前で調べて 1 件ずつ書き分けた。
  - `COMPAT_ARCHITECTURE.md` 227・228 行: `frame/window_state_tests.rs` の `script_playback_turns_on_raises_and_turns_off`・`raise_keeps_state_and_marks` は `areka-runtime`、`log_fields_for_change_unchanged_and_raise`・`log_ghost_falls_back_to_dash_and_raise_reports_normal_band` は本体の `runtime_tests/frame/window_state_tests.rs`（2 つのパスを書いた）。
  - `COMPAT_ARCHITECTURE.md` 326・328・330・385 行: `shell_balloon_resolve_tests.rs` の `shell_lastinstalled_resolves_only_within_the_current_ghost`・`balloon_lastinstalled_resolves_only_when_in_the_catalog`・`hidden_shell_by_name_and_the_current_one_are_candidates`・`balloon_candidates_put_the_ghost_inner_first` は本体の `runtime_tests/`。329 行の `random_picks_among_visible_others_with_the_injected_pick`・`random_with_no_other_candidate_is_the_current_one` は `areka-runtime` のまま。
  - `doc/ssp-mcp/areka-tools.md` 6 行: `strict_judges_tests.rs`（この文書を `include_str!` で読む）は本体の `crates/areka/src/runtime_tests/strict_judges_tests.rs`。
- 行番号つきの所を移した後の行で確かめた（移す前の `bd2dee303` の同じ行の文字と比べた）: `COMPAT_ARCHITECTURE.md` の `zorder_descript.rs:63`・`zorder_cue.rs:128`・`transition_judge.rs:711`（`crates/areka-runtime/src/placement_tests/transition_judge.rs` へ書き換え。711・723・760 行は同じ文字）は一致。`briefing-assets.md` の `assets.rs:331`・`:322`・`:415`・`:343`（`pub(crate)` → `pub` の 1 語の差だけ）は一致。**`lib.rs` は 5 行ずれた**ので、`runtime/mod.rs:385`（`warn!(`）→ `lib.rs:390`、`:392`（`error!(`）→ `lib.rs:397` に直した（`briefing-assets.md` 221・228・229 行の 4 か所と、229 行にあった `runtime/mod.rs:392` の字面）。
- `rg -n "[^/]runtime/[a-z_/]+\.rs"` の当たりを目で確かめた（この式は `areka-runtime/…` の `-runtime/` にも当たるので、`crates/areka-runtime/src/` と `crates/wintf/src/runtime/` を除いて読んだ）:
  - 書き換えた（本体の runtime の旧い書き方で `src/` を付けないもの・`doc/COMPAT_ARCHITECTURE.md` の 8 行 11 か所）: 134 行 `runtime/move_cue_args.rs`・136 行 `runtime/move_cue_geometry.rs`・137 行 `runtime/move_cue_place.rs`・174 行 `runtime/move_cue_geometry.rs` と `runtime/frame/drain_resnap.rs`・213 行 `runtime/frame_work_area_resnap_tests.rs`・`runtime/frame_transition_atomicity_tests.rs`・`runtime/frame/balloon_offset_follow.rs`・369 行 `runtime/frame/drain_resnap.rs`・428 行 同・432 行 同 → どれも `crates/areka-runtime/src/<同じ道>`（`frame_work_area_resnap_tests.rs`・`frame_transition_atomicity_tests.rs` は `areka-runtime` に実在・本体の `runtime_tests/` には無い）。
  - wintf・dola の自分のものとして残した（`crates/wintf/src/runtime/` と `crates/dola/src/runtime/` の中）: `doc/click-through.md` 3 行（`crates/wintf/src/runtime/mod.rs`）・103 行 `runtime/mod.rs`（`wire_click_through`）・129 行 `runtime/tick_bridge.rs`（`relay_vblank`）、`doc/COMPAT_ARCHITECTURE.md` 299 行・360 行の `crates/wintf/src/runtime/wndproc_bridge*.rs`、`.kiro/steering/structure.md` 110・113〜118 行（wintf の `runtime/` の Contains）・254 行（dola の `runtime/`）・271 行（dola の `runtime/clock.rs`）、`.kiro/steering/tech.md` 53・55 行（dola の `runtime/interpolator/`・`runtime/clock.rs`）。
  - 8.3 では書き換えなかった（8.3 の触るファイルの外）: `doc/ideas/novel-platform.md` 23 行の `runtime/mod.rs` の `derive_scopes`（今は `crates/areka-runtime/src/lib.rs` 345 行）。8.6 で書き換えた（10.5）。
- 緑（書き換えた後）: 上の式を 8.3 のファイルへ回して 4 行・2 ファイルのみ＝すべて wintf の `crates/wintf/src/runtime/` を名指す行（`click-through.md` 3 行・`COMPAT_ARCHITECTURE.md` 299・360 行）と、新しい置き場 `crates/areka-runtime/src/placement_tests/transition_judge.rs`（231 行）。`areka::runtime` と `emo2_boot|Emo2Wiring` は 0 件。
- `doc/ukadoc-coverage/report/` は手で直さず、網羅台帳の道具で作り直した: `cargo run -q -p ukadoc-survey -- report`・`-- report-summary`（`report/` 5 本を書き直した）。作り直した後の `git diff -- doc/ukadoc-coverage/report` は空（報告は項目の数と状態しか書かず、根拠の場所のパスを書かない）。`git status` が `M` に見えるのは行末の正規化だけで、内容の差は 0。続けて `cargo run -q -p ukadoc-survey -- check` は「食い違い 0 件」・終了コード 0、`cargo test -p ukadoc-survey` は緑。

### 9.2 8.4 steering

- 赤: `rg -n "crates/areka/src/runtime/|src/runtime/" .kiro/steering/structure.md .kiro/steering/tech.md` が `structure.md` 171・259 行・`tech.md` 90 行など。
- `structure.md`: ① `### Runtime Crate（areka-runtime）` の節を `### Placement Crate` の後に足した（Location・Purpose・Contains〔`lib.rs`・`frame/`・`spine*`・`placement_tests/`・`examples/`・`test-support` で開く支え〕・Features・`*_cue.rs` の同型〔本体の節にあったものをここへ移した〕・Dependencies・本体を引かない約束・Consumers）。② 本体の節の `runtime/` の行を `runtime_tests/`（テスト専用・本体を引く runtime のテストの置き場）の行へ、`placement_tests/` の行を本体に残る分（`measure_template`・`spawn_assembly`・`measure_bake_count`・分岐の対応表 2 本と支え）と移った分（`transition_judge*`・`transition_signoff*` ほか）へ。③ 本体の依存の行に `areka-runtime` を足した。④ 171 行の例のパス `areka/src/runtime/spine.rs` → `areka-runtime/src/spine.rs`。⑤ 259 行の `crates/areka/src/runtime/frame.rs` → `crates/areka-runtime/src/frame.rs`。⑥ `log-capture-kit` を使う crate の数 20 → 21（`areka-runtime` を足した。`crates/*/Cargo.toml` で `log-capture-kit` を引く crate は `log-capture-kit` 自身を除いて 21）。
- `tech.md`: 90 行の `spine_wait.rs` のパス → `crates/areka-runtime/src/spine_wait.rs`。`roadmap.md` は書いていない。
- 緑: 2 つの steering を `src/runtime/|areka::runtime|placement_tests/(transition_judge|…)` で grep して、wintf・dola の自分の `runtime/` を除いて 0 件。新しい crate の節がある。`emo2_boot` の語が `structure.md` に 2 行残る（`session/` の行と新しい節の「旧名」の説明＝歴史の説明で、5.10 の範囲〔`crates`〕の外）。

### 9.3 8.5 3 本の audit の brief

- 受け持つファイルの当て方（D14）: `git ls-files crates/areka-runtime crates/areka/src/runtime_tests` の 276 ファイルに、次の規則を `crates/areka-runtime/src/` と `crates/areka/src/runtime_tests/` の基準からの相対のパスで当てた。
  - `placement_tests/transition_*` → `audit-placement-follow-residue`
  - `frame/` の下・`frame.rs`・`frame_*` → `audit-emo2-boot-frame`
  - `switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs` → `audit-emo2-boot-switch`
  - それ以外（`Cargo.toml`・`examples/`・`placement_tests/` の `transition_*` でないもの・`lib.rs` ほか）→ `audit-emo2-boot-spine`
- 結果（スクリプトの出力）: 276 ファイルのうち、frame 99（`areka-runtime` 66・本体 33）・switch 48（41・7）・spine 113（79・34）・residue 16（`areka-runtime`）。どのファイルも当たる規則は 1 つだけ（重なり 0）・どれにも当たらないファイル 0。`crates/areka-runtime` 202・`crates/areka/src/runtime_tests` 74。
- 本体の `session/` へ移した e2e の 2 本（`script_security_e2e_tests.rs`・`readme_script_e2e_tests.rs`）は上の一覧の外にあるので spine の持ち物として別に挙げた（spine は 115 ファイル）。合わせて 278 ファイルが 4 つの持ち主のどれか 1 つだけ。
- brief の書き換え: 3 本の「Current State」の「受け持つファイル」の行を、D14 のとおり `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` の両方に当てた形へ（spine は `Cargo.toml`・`examples/`・`placement_tests/` の `transition_*` でないものを入れ、`transition_*` は外して `audit-placement-follow-residue` と書いた。`session/` の分は残し、e2e の 2 本を足した）。`audit-placement-follow-residue` の brief は書いていない。
- 3 本に節「runtime-crate-split からの申し送り（2026-11-17）」を足した。内容（⑴〜⑸）:
  - ⑴ 直さなかった不具合: 0 件。
  - ⑵ 理由の文が古くなった `#[allow(...)]`（属性と文は変えていない・`rg -n "#!?\[allow\(" crates/areka-runtime crates/areka/src/runtime_tests` の全部を読んだ）: frame の `frame.rs` 96〜97 行の文と `unused_imports` 17 本（98〜134 行）・152 行の文と 153 行・`frame/wiring.rs` 12 行の文と 13 行（理由が「`areka` は lib target を持たない bin クレート」）。spine の `assets.rs` 215 行 `#[allow(dead_code)]`（lib の `pub fn` になったので、許可がまだ要るか確かめる）と、`audit-placement-follow-residue` の持ち物の `transition_judge.rs` 47〜50 行の説明（`main.rs` が `placement_tests` を宣言する）。switch は 0 件。
  - ⑶ 開けた項目: 2.5（`pub(crate)` → `pub` 112 件）を持ち主ごとに frame 10・switch 26・spine 76 に分け、4.2 の ⑴〜⑷（開けた私有の項目 19・渡し口 22＝frame は 11 と 14・switch は 7 と 7・spine は 1 と 1）・案 C の割り方の表の行・6.1（`pub` にした項目 0 件）を持ち主ごとに書いた。
  - ⑷ `audit-placement-follow` の範囲の外の古い参照 3 件（frame だけ）: `drain_resnap.rs` 554 行と `frame_chain_finalize_tests.rs` 200 行の「唯一の位置ライター」・`frame_work_area_resnap_hold_tests.rs` 429 行の `should_panic` に `cfg_attr(not(debug_assertions), ignore …)` が無い件・`frame/move_shift.rs` 75 行の注。spine には `hit_region.rs` 45〜57 行の「`crate::` フリー規律」の文（examples が `#[path]` で取り込んでいた理由が古くなった）。
  - ⑸ 6.4 の扱いの表（frame だけ・12 行）。
  - spine にだけ: 番人の crate を引く補助を使う 17 本の関数（D4）が本体に残っていること。本体の `runtime_tests/balloon_visibility_phase_tests.rs` の `world_at`（146 行）が `areka-runtime` の支え `balloon_visibility_phase_test_support.rs` の `world_at` と同じ中身の前からの写しであること（switch にも書き、支えの側へ 1 つにまとめられると提案した）。
- 緑: 3 本の brief に節がある（`rg -c "runtime-crate-split からの申し送り" .kiro/specs/audit-emo2-boot-{frame,spine,switch}/brief.md` が各 1）。

### 9.4 旧いパスの grep（触ったファイルの範囲）

| 式 | 範囲 | 結果 |
|---|---|---|
| `crates/areka/src/runtime/\|src/runtime/\|placement_tests/(transition_judge\|…)` | 8.3 の 10 本・`structure.md`・`tech.md`・`doc/ukadoc-coverage/report` | wintf の `crates/wintf/src/runtime/` と新しい置き場 `crates/areka-runtime/src/placement_tests/` を除いて 0 件 |
| `areka::runtime(::\|=\|\b)` | 同上 | 0 件 |
| `emo2_boot\|Emo2Wiring` | 8.3 の 10 本 | 0 件 |
| `crates/areka/src/runtime/`（旧パス）| 3 本の brief | 歴史の説明（「前は…」）として書いた 3 行だけ。5.9 の範囲〔`crates tools doc steering`〕の外 |

### 9.5 申し送り

- `doc/ideas/novel-platform.md` 23 行の `runtime/mod.rs` の `derive_scopes` は旧い置き場（今は `crates/areka-runtime/src/lib.rs`）。8.3 の触るファイルの外なので 8.3 では直さず 8.6 へ渡した。8.6 で `crates/areka-runtime/src/lib.rs` へ書き換えた（10.5）。
- `audit-placement-follow-residue` の brief には書き込んでいない（8.5 の境界の外）。その持ち物のファイルは `crates/areka-runtime/src/placement_tests/transition_*`（16 ファイル）。

## 10. 段 7（8.6）

非挙動（確かめだけ）。対応する要件: 5.1, 5.9, 5.10, 5.11, 5.12, 6.7。`tools/` は 1 行も書いていない。残りを直した所は 10.5 の 2 か所（説明の文 3 行と `doc/` の文書の 1 行・テストの実行ファイルは変わらない）。基準は `HEAD` `be73dd1e8` の上の作業ツリー（8.1〜8.5 の未コミットの差分を含む）。

### 10.1 grep の結果

| 項目 | 式（設計の Verification の逐語） | 範囲 | 結果 |
|---|---|---|---|
| 古いパス（5.9） | `rg -n "crates/areka/src/runtime/\|src/runtime/\|placement_tests/(transition_judge\|transition_signoff\|reset_balloon_pos\|measure_bake_log\|spawn_zorder_pair_deferred)"` | `crates tools doc .kiro/steering/structure.md .kiro/steering/tech.md` | 生の当たり 12 行 → 除外（下の 10.2）後 0 件 |
| 古い名前（5.10） | `rg -n 'emo2_boot\|Emo2Wiring' crates -g '!crates/areka/**'` | `crates`（本体を除く） | 0 件 |
| 古い出所の名前（6.7） | `rg -n "areka::runtime(::\|=\|\b)"` | 5.9 と同じ範囲 | 0 件（`crate::runtime_tests` は当たらない） |
| `tools/`（5.11） | `git diff --stat main -- tools` | `tools` | 空（`git merge-base HEAD main` と `main` はどちらも `5b3a361c9`＝main は進んでいないので `main` との比べと同じ） |
| `tools/`（5.12） | `rg -n "src/runtime/\|areka::runtime" tools` | `tools` | 0 件（5.12 は発動しない・申し送りなし） |
| 目で見る式（5.9） | `rg -n "[^/]runtime/[a-z_/]+\.rs"` | 5.9 と同じ範囲 | 10.3 |

### 10.2 5.9 の生の当たり 12 行の仕分け

- 新しい置き場 `crates/areka-runtime/src/placement_tests/` を名指す（7 行）: `doc/COMPAT_ARCHITECTURE.md` 231 行・`crates/areka-runtime/src/frame_transition_branch_tests.rs` 14 行・`crates/areka-placement/src/spawn_zorder_pair_deferred_tests.rs` 13 行・`crates/log-capture-kit/tests/file_length_guard_test.rs` 71 行・`crates/wintf/src/runtime/tick_bridge.rs` 387 行と `crates/wintf/src/ecs/window/zorder_pair_test_support.rs` 92 行（どちらも手本の道が `crates/areka-runtime/src/placement_tests/transition_signoff_tests.rs`）・`crates/areka-runtime/src/placement_tests/transition_signoff_procedure_tests.rs` 67 行（`CARGO_MANIFEST_DIR` からの相対 `src/placement_tests/…`。今の持ち主は `areka-runtime` で、実在する）。
- wintf の自分の `runtime/`（4 行）: `.kiro/steering/structure.md` 110 行・`doc/COMPAT_ARCHITECTURE.md` 299・360 行・`doc/click-through.md` 3 行（どれも `crates/wintf/src/runtime/…`）。
- dola の自分の `runtime/`（1 行）: `crates/dola/tests/runtime/conflict_resolution_test/error_handling.rs` 87 行（dola の `src/runtime/facade_tests.rs`）。
- 本体の旧い runtime を名指す行: 0 件。

### 10.3 `[^/]runtime/[a-z_/]+\.rs` の当たりを目で確かめた

- 式: `rg -n "[^/]runtime/[a-z_/]+\.rs" crates tools doc .kiro/steering/structure.md .kiro/steering/tech.md`（`doc/ideas/novel-platform.md` を書き換えた後）。当たりは 185 行・211 か所（`rg -o` で数えた）。
- `areka-runtime/…`（新しい置き場）を名指す 185 か所: `-runtime/` の前はどれも `areka`（`rg -o "[a-z]*[^/]runtime/[a-z_/]+\.rs"` の頭を数えて `areka-runtime/` 185・ほか 0）。`crates/areka/src/placement_tests/zorder_{group,inversion}_branch_coverage_tests.rs` の `"areka-runtime/frame/…"`・`"areka-runtime/zorder_cue_tests.rs"` の 26 か所は `src/` を付けない名札だが、名指すファイルは `crates/areka-runtime/src/` に実在する（`frame/zorder_descript_tests.rs`・`zorder_cue_tests.rs` ほか）。書き換えた `doc/ideas/novel-platform.md` 23 行もここに入る。
- wintf か dola の自分のもの 26 か所: `structure.md` 113〜118 行（wintf の `runtime/` の節）・271 行と `tech.md` 55 行（dola の `runtime/clock.rs`）・`doc/click-through.md` 103・129 行（wintf の `wire_click_through`・`relay_vblank`）・`crates/wintf/src/runtime/mod.rs` 210 行・`crates/wintf/src/ecs/**` の説明 7 か所・`crates/dola/tests/runtime.rs` の `#[path]` 6 行・`crates/wintf/src/ecs/dola/mod.rs` 80・82 行（dola の `runtime/`）。
- 本体の旧い runtime を指すもの: 0 件（範囲の中にも外にも残りは無い）。

### 10.4 ほかの番人の表

- `crates/log-capture-kit/tests/temp_path_guard_test.rs`・`with_default_guard_test.rs`・`sample_path_guard_test.rs` を `rg -n "runtime|placement_tests|areka-runtime"` で回して 0 件。三つの表のどの項目にも runtime と移したテストのパスは無い（`with_default_guard_test.rs` の表は `areka-placement` の分だけ）。
- `file_length_guard_test.rs` の例外の表は 8.1〜8.5 のとおり `crates/areka-runtime/src/placement_tests/` を名指す（10.2 のとおり新しい置き場）。
- `cargo test -p log-capture-kit`（crate の全部）: 9 つの実行ファイルが全部緑（30・1〔2 ignored〕・8・6・21・16・24・20・4 本）。

### 10.5 直したもの・申し送り

- 直した 1 か所（8.1〜8.5 の持ち主の中・説明の文だけ）: `crates/areka-runtime/src/placement_tests/transition_signoff_procedure_tests.rs` の `PROCEDURE_RELATIVE_PATH`・`PROCEDURE_SOURCES` の説明 3 行の「`crates/areka` から見た相対」を「`crates/areka-runtime` から見た相対」へ（手順書の道は `CARGO_MANIFEST_DIR` から引くので、持ち主が移った後は `areka-runtime` が正しい。コードと行数は変わらない・906 行のまま）。`transition_judge{,_tests}.rs` は触っていない。`cargo fmt --all -- --check` は緑。
- 直した 2 か所目（8.3 から引き継いだもの・`doc/` は 5.9 の範囲の中）: `doc/ideas/novel-platform.md` 23 行の `` `runtime/mod.rs` の `derive_scopes` `` を `` `crates/areka-runtime/src/lib.rs` の `derive_scopes` `` へ（`fn derive_scopes` は `crates/areka-runtime/src/lib.rs` 345 行に実在）。`src/` を付けない書き方なので 5.9 の式には当たらず、10.3 の目で見る式でだけ見えていた。パスの字面だけを変え、文の残りは変えていない。
- `doc/` のほかの文書（8.3 の 10 本の外）: `rg -o '.{0,30}[^-]runtime/[A-Za-z_./]*' doc`（wintf・dola・`areka-runtime` を除く）と `rg -o 'areka::runtime\b|crate::runtime::|crates/areka/src/runtime/|src/runtime\.rs' doc` で探した。残ったのは `doc/click-through.md` 103・129 行（wintf の `runtime/`）と、`crates/areka-ghost/src/runtime.rs`（`areka-ghost` の自分のファイル）を名指す `COMPAT_ARCHITECTURE.md` 299 行・`ukadoc-coverage/briefing-assets.md` 221 行・`ssp-mcp/log-convention.md` 62 行だけ。本体の旧い runtime を指すものは 0 件で、ほかに直した文書は無い。
- `tools/` への申し送り: 5.12 は発動しなかった（段 8 で `-Filter runtime::frame` の残り 1 行を見つけた・11.6 ⑵）。設計の検証の最後の申し送り（`tools/load-flake.ps1` の `-Binary` に `areka-runtime-lib`＝`-p areka-runtime --lib` を足す）は、完了の申し送りで `audit-tools-scripts` の着地の前後に応じて書く（`tools/` は触っていない）。

## 11. 段 8（9.1）

非挙動（確かめだけ）。対応する要件: 1.6, 1.7, 1.9, 2.7, 3.10, 4.7, 5.11, 5.12, 5.13, 7.1, 7.2, 7.3, 7.4, 7.9。コードと `tools/` は 1 行も書いていない。書いたのはこの節・`THIRD-PARTY-NOTICES.md`（作り直し）・`target/runtime-crate-split/tests-after.txt`（11.7）だけ。`doc/ukadoc-coverage/report/*.md` の `M` は改行の正規化だけ（差分は空）で、この節の対象外。

**基準の `main` の訂正**: 最初の記録は「`main` は `5b3a361c9` のまま進んでいない」と書いたが誤り。手元の `main` の参照が古かった（`git rev-parse main` が `5b3a361c9` のまま）。`origin/main` は `fc0035eda` で、`5b3a361c9` の後に 4 本のマージが入っていた: #468 `60bba28b6`（`wintf-surface-auto-promotion`）・#469 `6c84b2975`（`balloon-flip-side-rules`）・#470 `27df10b39`（`audit-tools-scripts`）・#471 `fc0035eda`（`pilot-windows-rs-next`）。この 4 本は `9d32594ed` でこの枝へマージ済み（`git merge-base HEAD origin/main` = `fc0035eda`）。競合は `doc/COMPAT_ARCHITECTURE.md` の 1 か所だけで、`main` の `balloon-flip-side-rules` の新しい文を採り、この spec のパスの書き換えを残した。`main` が移した範囲に入れた変更は `switch_finish_face_tests.rs` の 1 本（434 − 30 + 40 行）だけで、git は移した先の `crates/areka/src/runtime_tests/frame/switch_finish_face_tests.rs` に当てた。マージの後に `cargo check --workspace --all-targets` は緑（警告は 11.1 の既存の 1 件だけ）・`runtime_tests::frame::switch` は 34 本が通る。以後この節の「main」は `origin/main` の `fc0035eda` を指す。

- 11.1・11.2 の `cargo tree`・11.3・11.4 の数・11.5 は `fc0035eda` に対して取り直した（マージの後の `HEAD` `9d32594ed`）。
- 11.7 の名前の多重集合の比べは、マージの前の `ac47159f3` で取ったもの。この spec が自分で起こした増減だけを切り出すための比べで、`main` の 4 本が足したテストはこの比べに入らない。マージの後の状態は全体テスト（9.2）で回す。

### 11.1 警告 0（1.6・7.4）

- `cargo check --workspace --all-targets`: 完了（`Finished`）。警告は 1 件だけ: `shiori-abi` の manifest の `unused dependency windows`。この作業の前からある既存のもので、この spec の持ち物ではない（`areka-runtime` と `areka` と `areka-placement` の警告は 0）。

### 11.2 本番が本体を引かない（1.5・2.7）

- 式（設計の Verification の逐語）: `rg -n "crate::(session|ghost_session|input_events|install|update|readme|mcp|log_history|boot_config|boot_resolve|roots|app_exit|desk|menu|is_benign_boot_error|insert_persist_wiring|restore_merged_placements|crate_source_test_support|scope_bind_fixture_test_support)\b|\bareka::[a-z_]+::" crates/areka-runtime`
- 結果: 2 行。どちらも `crates/areka-runtime/src/placement_tests/transition_judge_offset_signoff_tests.rs` の 131・132 行の `//!` の説明で、ログの出所（`RUST_LOG` の対象）の文字列 `areka::persist::restore=info` を書いたもの。crate の道ではなく、コードでもない（テスト専用ファイルの説明）。本番のファイルの当たりは 0。式の `\bareka::[a-z_]+::` が出所の文字列にも当たる（式の取りこぼしではなく過検出）。**説明できる過検出として受け入れる（controller の裁定・コードは直さない）**。
- `cargo tree -p areka-runtime -e normal`（マージの後に取り直した・362 行）: 本体（`areka v`）の行 0・番人の crate（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`）の行 0（`scopeguard` だけが文字列で当たるが別の crate）。直の依存は `areka-{actor,emo-atlas,emo-compose,emo-present,emo-text,ghost,kanade,parsers,placement,sakura,seriko,sylphya}`・`bevy_ecs`・`dola`・`thiserror`・`tracing`・`windows`・`wintf`。

### 11.3 `Cargo.lock`・根の `Cargo.toml`（1.7・1.9）

- `git diff fc0035eda -- Cargo.toml`: 空（マージの後に取り直した）。
- `git diff fc0035eda -- Cargo.lock`（マージの後に取り直した・#471 の後も同じ）: 2 か所だけ。⑴ `areka` の依存に `"areka-runtime",` の 1 行 ⑵ `[[package]] name = "areka-runtime"` の 1 項目（版 0.1.0・依存 25 本はすべてワークスペース内か既存の crate で、新しい外の crate は 0）。

### 11.4 本番のファイルの差分（5.11・7.1〜7.3）

`git diff --stat fc0035eda...HEAD`（マージの後に取り直した）: 459 ファイル・+8102/−4297（最初の記録の 458 ファイル・+8041/−4296 は古い `main` の `5b3a361c9` に対する作業ツリーの数。差は 9.1 の記録のコミットとマージの解消の分。`git diff --stat fc0035eda...HEAD -- tools` は空）（`doc/ukadoc-coverage/ledger/*.toml` などの台帳の差も数に入る。これらの出所は段 7 のコミット `ac47159f3`＝`git log main..HEAD -- doc/ukadoc-coverage` の唯一の当たり。台帳の中身の確かめは本節では行っていない）。本番（`crates/areka-runtime/src`・`crates/areka/src` のテストと支えを除いた非テストのファイルと `areka-placement` の本番）を `-M` の差分で見た。ファイルの数は R 59・M 42・A 2・D 1。コメント・説明の行を除き、可視性（`pub(crate)`・`pub(in …)` → `pub`）・道（`super::super::`・`crate::runtime::` → `crate::`）・空白を正規化した後に残る変更を種類で仕分けた。

| 種類 | 当たったファイル | 判定 |
|---|---|---|
| 可視性・道・説明 | 移した 59 本の大半・`areka` の `input_events/*`・`mcp/*`・`session/*`・`ghost_session*.rs`・`app_exit.rs`・`boot_config.rs` | 道（`runtime::X` → `areka_runtime::X`）と可視性だけ |
| `rustfmt` の折り返しの変更 | `balloon_visibility_phase.rs`・`frame/scale_text.rs`・`frame/wiring.rs`・`shell_balloon_resolve.rs`・`shell_carry.rs`・`app_exit.rs` | 道が短くなって 1 行に収まる・または逆。字面だけで意味は同じ |
| 宣言の印（`cfg`・`#[path]`・`mod`） | `lib.rs`・`balloon_visibility.rs`・`frame.rs`・`frame/{drain_resnap,status_report,switch,window_state,wiring}.rs`・`consumer_ledger.rs`・`shell_*`・`areka-placement/src/{follow,follow/work_area,transition_diag}.rs`・`main.rs` | `test-support` で開く印（`#[cfg(any(test, feature = "test-support"))]` ほか）・テストの宣言の移し替え・`test_open` の再公開（設計 D3 ⑵）。本番の関数の中身は変わらない |
| 本番の項目の書き換え | `session_hooks.rs` の `FALLBACK`（`#[cfg(test)]` の本体の口を引く分岐を外し `NO_BODY` 1 本へ）・`areka-placement/src/follow.rs` の再公開（`MonitorSnapshot`・`work_area_for_window` ほか、テストだけが使う項目を `test-support` の下へ） | `session_hooks.rs` は設計 D5（口の倒れ先）・`follow.rs` の再公開の切り替えは設計 D10 のとおり。本番の振る舞いは変わらない |
| 新規・削除 | A: `areka-runtime/src/spine.rs`・`areka-runtime/src/frame/work_area_sync.rs`／D: `areka/src/runtime/frame/work_area_sync.rs` | `spine.rs` は旧い `areka/src/runtime/spine.rs`（−569 行）の ⑴⑵ の層を `areka-runtime` へ移したもの（3.2・D4）。`frame/work_area_sync.rs` は `areka-runtime/src/frame/` へ移した（246 → 275 行）。増えた分は `resnap_for_work_area_change` を `test_open` で包んだもの（設計 D3 ⑵・字下げだけ）と、⑷ の渡し口 `test_support::sync_monitor_snapshot_with`（設計 D3 ⑷・`super::sync_monitor_snapshot_with` へそのまま渡す）。ほかは可視性・道 |

- ほかの crate の本番のファイル: `areka-emo-present`・`areka-ghost`・`areka-kanade`・`areka-parsers`・`areka-seriko`・`log-capture-kit`・`ukadoc-survey`・`wintf` は `git diff --stat fc0035eda...HEAD` で 22 ファイル・+34/−34。テストのファイル（`log-capture-kit` の `src/event_tests.rs`・`tests/file_length_guard_test.rs`・`tests/workspace_scan_test.rs`）を除くと 19 ファイルで、そのうち `wintf` の `ecs/window/zorder_pair_test_support.rs`・`test_crash_report.rs` は `#[cfg(test)]` の支えなので、本番のファイルは 17（指示の見込みの「18」とは 1 違う。数え方はこの行のとおり）。22 ファイルすべてで、変わった行はコメント・説明（`//`・`//!`・`///`）だけ（`git diff -U0` の `+`/`-` の行からコメントの行を除くと 0 行）。

- 目視で本番の中身（関数の本文）が変わったと読めた所: 0 件。ただし差の量が大きいので、**レビューで `session_hooks.rs`・`frame/work_area_sync.rs`・`spine.rs` を実物で読み直すことを勧める**（この 3 つが本番の項目の書き換え・新規として上の表の「可視性・道・説明」の外に出るため）。

### 11.5 第三者の表（7.9）

- `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`: マージの後（#471 で依存が変わり得るため）に作り直した。成功。`git diff fc0035eda -- THIRD-PARTY-NOTICES.md` の差は 2 行だけ: `MIT License (262 crate)` → `(263 crate)` と `areka-runtime 0.1.0` の 1 行の追加。作り直した中身はコミット済みの `HEAD` のものと同じ（作業ツリーに差は出ない）。

### 11.6 完了の申し送りの下書き（5.12・5.13）

1. **`tools/` の追随（5.11・5.13）**: `audit-tools-scripts` はこの spec より先に着地した（#470 `27df10b39`）。`areka-runtime-lib` は足しておらず、代わりにこの spec の `brief.md` の末尾に「申し送り（audit-tools-scripts より）」を書いた（マージでこの枝に入っている・参照先は `.kiro/specs/completed/audit-tools-scripts/verification.md` の「タスク 11.1」）。挙げられた項目は 3 つ:
   - `tools/load-flake.ps1` の `-Binary` に `areka-runtime-lib`（`-p areka-runtime --lib`）の選び肢を足す
   - 使い方の例の `-Filter runtime::frame` を新しいクレートの形に直す
   - `-SelfTest` の `cargo args` の期待を 3 対象から 4 対象へ直す

   要件 5.11 によりこの spec は `tools/` を変えない（`git diff --stat fc0035eda...HEAD -- tools` は空）。5.13 により、この 3 つは完了（`/kiro-complete`）の申し送りで、その時に `/kiro-discovery` で起票する後続の spec（仮の名「完了の時に起票する tools の追随の spec」）へ渡す。この spec では `tools/` に 1 行も触っていない。
2. **`tools/` の残り（5.12・記録だけで直さない）**: `tools/load-flake.ps1` 48 行の使い方の例 `pwsh -NoProfile -File tools/load-flake.ps1 -Label case1 -Filter runtime::frame -SetEnv RUST_LOG=trace` が残る（同じファイルで `-Filter runtime::frame` に当たるのは 48 行の 1 行だけ・47 行は当たらない）。旧い `runtime` の道は `crates/areka-runtime` へ移ったので、`-p areka`（既定の `areka-bin`）の上では `runtime::frame` の絞りに当たるテストが無い（11.7 の `tests-after.txt` に `runtime::frame` を含む名前は 0 本。本体のテストは `runtime_tests::frame::…`）。段 7 の式 `src/runtime/|areka::runtime`（`rg -n "src/runtime/|areka::runtime" tools` は 0 件）はこの書き方を拾えない。上の ⑴ の 2 つ目と同じもので、同じ後続の spec へ渡す。
3. **本体の `Cargo.toml` の直に使わなくなる依存の行（`audit-areka-app` へ）**: 粗い確かめ（`crates/areka` の `[dependencies]` の各 crate について、テスト・支えを除いた `src` の中の `<crate>::` の使いを探す）では、直に使われなくなった行は 0 件（`dola`・`areka-seriko` を含め、本体の `install/`・`readme/`・`mcp/`・`ghost_session.rs` などに今も使いが残る）。よって完了の申し送りに書く行は無い見込み。`audit-areka-app` が改めて数える。`[dev-dependencies]` の `real-run-kit` は `src` に使いが無く `examples`・`tests` の 7 ファイルが使う（正しい）。
4. 2.7 の式が説明の中のログの出所の文字列に当たる 2 行（11.2）は、`audit-emo2-boot-*` の見直しの対象に書き足せる（式を `rg -n` で回す前提の説明の書き方の問題）。

### 11.7 テストの名前の多重集合の比べ（3.10・4.7・7.1〜7.3）

- 取り方: マージの前の `ac47159f3` で `cargo test --workspace --no-fail-fast -- --list`（終了コード 0・`error` 行 0）を `target/runtime-crate-split/tests-after.txt` に取り、`: test$` の行の名前の末尾（`::` で区切った最後）の多重集合を `tests-before.txt` と比べた。`test-all.ps1` は直していない。
- 数: 前 14143 本・後 14144 本（+1）。差が出た名前は次の 3 つだけ。

| 名前の末尾 | 前後の差 | 対応 |
|---|---|---|
| `emo2_boot_talk_reveals_without_moving_glyphs` | −1 | 旧い名前。`emo2_boot` を `runtime` に改めた名前（下の行）へ置き換わった（段 1〜3 の改名） |
| `runtime_talk_reveals_without_moving_glyphs` | +1 | 上の行の新しい名前 |
| `crash_filter_is_installed_before_the_tests_start` | +1 | 新しいのは本体（`areka` の bin）の `crates/areka/src/runtime_tests/mod.rs` 23 行のテスト（落ちの報告の番人・D6・一覧では `runtime_tests::…`）。`areka-runtime` の lib にある同じ名前のテスト（`crash_report_test_support::…`・`crates/areka-runtime/src/crash_report_test_support.rs` 520 行）は、旧い `runtime::crash_report_test_support::…`（`fc0035eda` の `crates/areka/src/runtime/crash_report_test_support.rs` 513 行）を移したもので、前後で数は変わらない |

- 見込みと一致: −1・+1・+1 の 3 つだけで、ほかの名前の増減は 0。
- `areka-runtime` の lib のテストの実行ファイル: 取得時の標準エラーに `Running unittests src\lib.rs (... areka_runtime-....exe)` が現れる（`--list` に拾われる・3.10）。
- 文書のテスト: `Doc-tests areka_runtime` は走るが、`(line N): test` の行は前 66・後 66 で同数（上の多重集合の比べにも差として出ない）。新しく回る文書のテストは無い。囲みを `text` にする直しは要らず、コードも触っていない。

## 12. 段 8（9.2）全体テスト

- 取り方: 排他テストの机を取り（`granted test`）、`9d32594ed`（`origin/main` `fc0035eda` を取り込んだ後）＋ 9.1 の記録のコミット `763f4ddf9` の木で `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回した。出力は `target/runtime-crate-split/test-all.txt`。走り終えてすぐ机を返した。`tools/test-all.ps1` は直していない
- 結果: 終わりの行は `all steps green`（終了コード 0）。段はすべて `OK`（`fmt --check`・`x64 workspace tests` 558 秒・`dola toml feature tests`・`i686 tests (host-32)`・`crates.io pre-publish check`・`encoding check`・`load flake check` ほか）
- `areka-runtime` のテストの実行ファイルが走ったこと: 出力の 8528 行に `Running unittests src\lib.rs (…areka_runtime-….exe)` がある（要件 3.10）。その結果は 9524 行の `989 passed; 0 failed; 3 ignored`
- 回し直しは無い（赤が無かった）


## 13. 段 8（9.3）実機テスト

- 取り方: 排他テストの机を持った状態で、`db8cee12a` の木から 3 本を 1 回ずつ回した（赤は無く、回し直しは無い）。事前に `netstat -ano | findstr 9276` は空（ポートは空き）。実機の根・記憶の置き場は `target\` の下（`target\sot-signoff\` ほか）。コードと `tools/` は触っていない。i686 の `shiori-host32-helper.exe` は建ててある（`target/i686-pc-windows-msvc/debug/`）。

| 試験 | 回したコマンド（作業ツリーの根から） | 結果 |
|---|---|---|
| `stay_on_top_real_run.rs`（要件 6.5・7.6） | `$env:AREKA_STAY_ON_TOP_REAL_RUN="1"; cargo test -p areka --test stay_on_top_real_run -- --nocapture` | `1 passed; 0 failed`（64.18 秒）。判定はすべて `JUDGE PASS`。子の終わりは `status: 0`・`by: UserClose` |
| `smoke_boot_loop_exit.rs`（要件 7.7） | `cargo test -p areka --test smoke_boot_loop_exit -- --nocapture`（この試験に環境変数のゲートは無い） | `5 passed; 0 failed`（2.23 秒）。本物・根・0 体・失敗・書けないの 5 方向 |
| `emo2_real_run.rs` と `emo2_real_run/`（要件 7.6） | `$env:AREKA_EMO2_REAL_RUN="1"; cargo test -p areka --test emo2_real_run -- --nocapture` | `14 passed; 0 failed`（23.49 秒）。実走 `emo2_real_run_boots_talks_and_exits_zero` を含む |

- emo2 の時間の 1 行: `emo2_real_run: wire_ms=791 attach_ms=974 wrap_ms=1629 close_ms=21675 talk_to_wrap_ms=41 attach_to_talk_ms=601 primary_dpi=192`
- 出力: `target/runtime-crate-split-9.3-sot.txt`・`target/runtime-crate-split-9.3-smoke.txt`・`target/runtime-crate-split-9.3-emo2.txt`。stay-on-top の子のログは `target/sot-signoff/run.log`（611 行が `areka_runtime::` の出所）。

### 13.1 診断の `debug!` の行（新しい出所）

`stay_on_top_real_run.rs` は `RUST_LOG` に `areka_runtime::window_state_cue=debug`・`areka_runtime::zorder_cue=debug`・`areka_runtime::frame::{window_state,zorder_drain,zorder_descript,switch}=debug` を渡す（試験の本文 51〜54 行）。`target/sot-signoff/run.log` の中の `DEBUG areka_runtime::` の行は 3 本（抜き書きは `target/sot-signoff/debug-excerpt.txt`）:

```
2026-11-17T01:03:49.731841Z DEBUG areka_runtime::frame::window_state: [window_state] ゴーストの窓を今いる帯の最上へ出す ghost="emo2" band="normal" windows=4
2026-11-17T01:03:54.863889Z DEBUG areka_runtime::frame::switch: 切れ目の返事が資産より先に届いた——資産がそろったので待ちを送り直す event="skin_switch_gap_recheck" kind=Shell to="copy"
2026-11-17T01:04:00.680495Z DEBUG areka_runtime::frame::window_state: [window_state] ゴーストの窓を今いる帯の最上へ出す ghost="emo2" band="topmost" windows=4
```

判定の行にも新しい出所の `INFO` が載る（`JUDGE PASS 8.1h`）: `INFO areka_runtime::frame::window_state: [window_state] 常に手前の状態が変わった ghost="emo2" directive="stayontop" before="off" after="on" windows=4`・`INFO areka_runtime::frame::window_state: [window_state] 常に手前のまま降ろした（状態は捨てる） ghost="emo2" reason="switch"`。`JUDGE PASS R2c`（帯の働きの warn が 0 行）も緑。

### 13.2 旧い出所の行が 0 であること

- `grep -c "areka::runtime" target/sot-signoff/run.log` → 0。
- `grep -c "areka::runtime" target/runtime-crate-split-9.3-sot.txt`（試験の標準出力・標準エラー全体）→ 0。
- 旧い出所の行が 0・新しい出所（`areka_runtime::`）の行が 611 行、という形で、`RUST_LOG` の `areka_runtime::…=debug` が効いて診断の `debug!` が出ていることと、旧い道の行が残っていないことの両方を示した。

### 13.3 完了の状態

3 本とも緑・診断の行の抜き書きは上のとおり。タスク 9.3 の完了の状態を満たす（段 8 のコミットは進行役が行う）。

## 14. 検証の差し戻しの直し（kiro-validate-impl 1 回目）

`/kiro-validate-impl` の 1 回目が NO-GO を返した指摘 1 件（重要）と 3 件（軽微）を直した。直したのはテストと文書の注だけで、本番のコードは変えていない（要件 7.1）。

### 14.1 木を歩く見張りの範囲を移す前と同じに戻す（重要・要件 5.4）

移す前は本体の `crates/areka/src` が runtime も含んでいたので、下の 4 本の見張りは 1 つの根で両方を覆っていた。移した後は片方の木しか歩いていなかったので、4 本とも 2 つの根（本体の `src` と、読み口 `crate_source_test_support::crate_path("areka-runtime", "src")`）を歩く形に直した。見出し・許可表の道は 2 つの木の間で取り違えないよう `<crate>/<src の中のパス>` の形にそろえた（`areka/…`・`areka-runtime/…`）。それぞれの根で 1 つも読まなければ赤になる数の確かめを足した。tick_gate の照合は `==` のまま。

| テスト | 直す前に歩いた根 | 直した後に歩く根 | 道の見出し |
|---|---|---|---|
| `crates/areka/src/tick_gate_config_producers_tests.rs`（`files_marking` を使う `the_areka_table_lists_every_file_that_marks_the_wake`・`the_wintf_module_roster_names_every_areka_zorder_producer`） | `crates/areka-runtime/src` だけ（6.2 で替えた） | `crates/areka/src`・`crates/areka-runtime/src` | `AREKA_PRODUCERS` の 8 行を `areka-runtime/adapter.rs` ほかへ |
| `crates/areka/src/readme_tests.rs` の `only_the_os_port_spells_shell_execute` | `crates/areka/src` だけ | 同上の 2 つ | 期待を `areka/readme/os_port.rs` へ |
| `crates/areka/src/update/worker_tests.rs` の `winhttp_is_spelled_only_in_two_production_files` | `crates/areka/src` だけ | 同上の 2 つ | 許可と期待を `areka/install/fetch_url.rs`・`areka/update/worker.rs` へ |
| `crates/areka/src/session_end_deadline_tests.rs` の `production_sources`（`only_session_end_calls_the_bounded_shutdown`・`deadline_call_scan_turns_red_on_one_line_removed_or_added`） | `crates/areka/src` だけ | 同上の 2 つ | 許可表を `areka/ghost_session.rs`・`areka/session_end.rs` へ・較正の写しの道も `areka/session_end.rs`・`areka/main.rs` へ |

注の「本体の本番ファイル」「`crates/areka/src` の下」は、どちらの木を覆うかを書く文に改めた。

**ほかに歩く見張りを探した結果**: 本体の `crates/areka/src` で `read_dir` と `CARGO_MANIFEST_DIR` を両方持つファイルは上の 4 本のほか `mcp/mcp_tests.rs`・`readonly_tests.rs`・`session_end_sync_send_tests.rs` の 3 本。ほかの `read_dir` の使い手（`readonly_sites_tests.rs`・`install/procedure_branch_tests.rs`・`boot_shell_tests.rs` ほか）は一時のフォルダの木を歩くもので、ソースの木ではない。

- `mcp/mcp_tests.rs` の `production_files_have_no_not_implemented_yet`: 歩くのは本体の `src/mcp/` だけ。`mcp/` は本体に残り、`areka-runtime/src` に `mcp` の木は無いので、移す前と範囲は同じ。直さない。
- `session_end_sync_send_tests.rs`: ワークスペース全体（`crates/`）を歩く名前を付けた例外で、`areka-runtime` も覆う。直さない。
- `readonly_tests.rs:78` の `engage_is_called_only_from_main`: 本体の `src` だけを歩く。判断は**直さない**。理由: 見張るのは本体の `readonly::engage` の呼び手で、`areka-runtime` は本体に依存できない（依存の向きが逆）ので、`areka-runtime` の中から `readonly::engage` を呼ぶコードはコンパイルが通らない。歩く範囲を広げても当たりは原理的に 0 で、範囲が狭まったことによる見落としは起きない。同じファイルの `withhold_everything_is_called_only_from_readonly` はワークスペース全体を歩いており、`areka-runtime` も覆う。

**較正（広げた根が `areka-runtime/src` のファイルを実際に読むこと）**: どこからも `mod` で宣言しない一時のファイル `crates/areka-runtime/src/zz_calibration_probe.rs` に 4 本の針（Shell と Execute を繋いだ名・`WinHttpOpen`・`probe.arm(budget)`・`tick_wake::mark(tick_wake::ZORDER)`）を書いて回し、全部が `areka-runtime/zz_calibration_probe.rs` を名指して赤になることを確かめてから、ファイルを消した（`git status` で残りが無いことを確認）。

- `winhttp_is_spelled_only_in_two_production_files` … FAILED（`本番の取得口の型を綴る所が増えた: [("areka-runtime/zz_calibration_probe.rs", "WinHttpOpen")]`）
- `only_the_os_port_spells_shell_execute` … FAILED（`left: ["areka/readme/os_port.rs", "areka-runtime/zz_calibration_probe.rs"]`）
- `the_areka_table_lists_every_file_that_marks_the_wake` … FAILED（`旗を立てているのに AREKA_PRODUCERS に載っていない: areka-runtime/zz_calibration_probe.rs`）
- `the_wintf_module_roster_names_every_areka_zorder_producer` … FAILED（同じファイルを名指す）
- `only_session_end_calls_the_bounded_shutdown` … FAILED（`left: ([("areka-runtime/zz_calibration_probe.rs", "let g = probe.arm(budget);")], [])`。この走査は `fn ` を含む行を定義として除くので、針は独立の 1 行で置いた）

本体の根の側は、4 本とも直す前から本体の当たり（`areka/readme/os_port.rs` ほか・許可表の行）を両側から挟む形で見ており、直した後もその照合が緑であることで読んでいることを示す。

### 14.2 軽微な 3 件（文書の注だけ）

- `crates/areka/src/placement_tests/mod.rs` の頭の注: 遷移の観測ログの判定器の置き場という記述を外し、判定器は `areka-runtime` の `src/placement_tests/` へ移ったと書いた。本体の部品の例から `runtime` を外した（今の中身が使うのは `input_events` と `areka_runtime::`）。
- `crates/areka/src/input_events/mod.rs` の「依存方向」の注: example が `#[path]` で私有 include するという記述を、配置の crate は本体に依存できないという今の理由に改めた。`crates/areka-placement/examples/window-placement.rs`・`crates/areka-runtime/examples/collision-probe.rs` はどちらも crate の公開面で使い、`#[path]` で配置のファイルを取り込んでいないことを確かめた（`collision-probe.rs` の `#[path]` は自分の子の `collision-probe/*.rs` だけ）。
- `crates/areka/examples/emo-present/reconcile.rs`: ずれていた `collision-probe.rs:34` の行番号の指しを、`areka-runtime` の `examples/collision-probe.rs` 冒頭の注の「`Anchored` 欠落」の項を名で指す形に改めた。窓配置の観測の領分の文の 2 つの example も crate の名前つきで書いた。
- `.kiro/steering/structure.md` の Placement Crate の節: Features に、`test-support` を付けるのが本体の `[dev-dependencies]` のほか `areka-runtime` の `[dev-dependencies]` と `areka-runtime` 自身の `test-support`（`areka-placement/test-support` へ連ねる）であることを書いた。Consumers に `areka-runtime`（本番の依存と、テストのときの `test-support`）を足した（`crates/areka-runtime/Cargo.toml` の 14・20・58 行で確かめた）。

### 14.3 回した確かめ（机は要らないもの）

- `cargo test -p areka --bin areka -- tick_gate_config only_the_os_port winhttp_is_spelled deadline --list` で、直した 4 本の 6 つのテスト（`tick_gate_config::producers_tests::*` の 5 本を含む）が当たることを確かめた。
- `cargo test -p areka --bin areka -- tick_gate_config only_the_os_port winhttp_is_spelled deadline` → `test result: ok. 29 passed; 0 failed`。
- 較正の一時ファイルを置いた回 → 上の 5 本が赤（14.1）・消した後にもう一度回して `29 passed; 0 failed`。
- `cargo fmt --all -- --check` → 差分なし。
- `cargo check -p areka --bin areka --tests` → 警告 0・エラー 0。`cargo check -p areka --examples` → 警告 0（`reconcile.rs` の注を直したため）。
- `cargo test -p log-capture-kit --test file_length_guard_test` → `6 passed; 0 failed`。
