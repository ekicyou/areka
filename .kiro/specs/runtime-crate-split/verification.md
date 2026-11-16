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
