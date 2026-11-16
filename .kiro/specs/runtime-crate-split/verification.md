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
