# ギャップ分析（kiro-validate-gap）: areka-P0-baseware-root-layout

> 実測日 2026-09-23・本ブランチ `claude/areka-p0-baseware-root-layout-6ae6de`（main `92f5f448`＝`areka-P0-app-lifetime-separation` 着地後の `70bba3e3`）。
> コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。行番号を添えた箇所は本日の実測値であり、着手時に引き直すこと。brief.md の行番号（09-18／09-20 実測）は使っていない。
> 本文書は分析と選択肢を示すもので、最終決定は要件ディスカッション（要件 9 の裁定候補 1〜5 と、末尾「設計判断の議題」）で開発者が行う。

## 1. 現状の資産（実測）

### 1.1 起動と構成入力（`crates/areka/src/boot_config.rs`・`main.rs`）

- `struct ConfigInputs { ghost_root, balloon_root }`（`boot_config.rs`）が起動の構成入力の型。消費者は `fn open_startup_window(app, cfg)`（`main.rs`）→ `placement::prepare_ghost_windows(&cfg.ghost_root, &cfg.balloon_root)`、`emo2_boot::wire_emo2_boot(&app, &cfg.ghost_root, &cfg.balloon_root, …)`（`emo2_boot/mod.rs` の署名で確認）、フォールバック boot の `ghost_boot_options(cfg.ghost_root.clone(), …)` の 3 か所。**型を残して中身の決め方だけ替えれば、この 3 消費者は無改変で済む。**
- `fn resolve_config_inputs(args: &[String]) -> ConfigInputs`（`boot_config.rs`）は `args[1]`／`args[2]` を採り、無ければ `fn default_ghost_root`／`fn default_balloon_root`（`CARGO_MANIFEST_DIR` 相対の `ghost/master`・`balloon/master`）へ落ちる。純粋関数で檻は `main_config_input_tests.rs`（5 本・うち 4 本が既定パスを直接参照）。要件 1.5 で既定パスは消えるので、この 4 本は陳腐化（`obsolete-vs-broken-test-policy`）。
- `fn main`（`main.rs`）の順序: tracing 初期化 → `thread_roles::install()` → perf 報告器 → **構成入力の解決と `warn!` の存在確認ループ**（`for (label, root) in [...] { if !root.exists() { warn!(...) } }`・実測 173〜183 行）→ `default_helper_exe_path()` → `WinApp::with_exit_policy(ExitPolicy::Explicit)` → `tick_gate_config` → SHIORI デモ → `open_startup_window` → `wire_emo2_boot` → wired／fallback boot の分岐 → `app.run()` → 終了順序①〜④。**根・ゴースト・バルーンの解決はすべて `WinApp` 構築より前で完結できる**（今日の `resolve_config_inputs` と同じ位置）。
- `fn open_startup_window` の `Err(err)` アーム（実測 734〜757 行）が `is_benign_placement_error` で `warn!`／`error!` を分け、`spawn_dummy_window` を ECS コマンド経路で spawn し `None` を返す。呼び手 `main` は `None` を「作者基準 DPI を正典既定へ縮退」として**起動を続ける**（実測 228〜235 行）。要件 6.4 はここを「`error!`＋告知＋非 0 終了」へ替える。
- `AREKA_APP_SMOKE_EXIT_MS` の自動終了（`const SMOKE_EXIT_ENV`・`fn smoke_exit_ms_from(Option<&str>)`・`fn smoke_exit_ms()`）は `open_startup_window` の**戻り値を決めた後、同関数の末尾**で `spawn_local` される（実測 765〜792 行）。準備失敗で早期 return するなら、この投入位置は成功アームの後へ動く（または呼び手へ出す）。
- `fn main` の戻り値は `windows::core::Result<()>`。`Err` を返せば Rust の既定で終了コード 1 になる（既存の終了統括失敗経路がこの形・実測 365〜367 行）。0 以外の終了コードは新しい仕組みを要さない。
- `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`（`main.rs` 1 行目）: **release ビルドはコンソールを持たない**ので、配布 exe では stdout の記録が利用者に見えない。要件 6.1 の告知（メッセージボックス）が第三者に届く唯一の可視経路になる。

### 1.2 1 体のゴーストの解決 `fn resolve` と 4 つの本番の呼び手（`crates/areka-parsers/src/package/resolve.rs`）

- `pub fn resolve(ghost_root: &Path, default_encoding: DefaultEncoding) -> Result<MountModel, MountError>`。`<ghost_root>/ghost/master/descript.txt` を `std::fs::read` → `charset::decode` → `kv::parse_kv` し、鍵 `name`・`sakura.name`・`sakura.name2`・`kero.name`・`shiori`・`shiori.encoding`・`shiori.forceencoding`・`seriko.defaultsurfacedirectoryname`・`readme` を転記する。`shell/<名>/` が `is_dir()` でなければ `MountError::ShellDirMissing`。**`craftman`・`craftmanw`・`id`・`type`・`menu` は読まない**（要件本文どおり）。`MountModel.readme: Option<String>` は既にある（`package/model.rs`）。
- 本番の呼び手 4 か所（いずれも `resolve(` を直接呼ぶ）: `crates/areka/src/emo2_boot/assets.rs`・`crates/areka/src/placement/persist.rs`（`fn load_restored_state`）・`crates/areka/src/placement/source.rs`・`crates/areka-ghost/src/runtime.rs`（`fn boot_with_kanade_stop` の手順 1）。要件 2.10／7.3 のとおり触らない。
- **注意（brief の前提の補正）**: parsers は「外部パーサ依存を入れない」方針だが、`package::resolve` と `areka-ghost` の `config::resolve_shell_name` はどちらも `std::fs::read` を行う。**「parsers は I/O を持たないので走査は置けない」は事実ではない**（package は fs を読む）。置き場の選択は §3 で改めて比較する。
- 流用できる部品: `charset::decode(bytes, DefaultEncoding) -> String`（`charset/decode.rs`）と `kv::parse_kv(&str) -> BTreeMap<String, String>`（`kv/parse.rs`・trim・後勝ち・**鍵の大小は保持**）。`areka-nar` の `install.txt` 読みは `parse_kv` の後に鍵を `to_ascii_lowercase()` している（`manifest.rs` の `fn lowercased_keys`）。列挙の鍵の大小の扱い（`menu,hidden`／`type` の綴り）は設計で 1 つに決める。

### 1.3 バルーン descript の読み手（`crates/areka-parsers/src/balloon/parse.rs`）

- `pub fn parse(...)`／`pub fn parse_str(descript, image) -> BalloonModel` は幾何とフォントの鍵だけを型付きモデルへ写す。`type`・`id`・`name`・`craftman` は読まない（台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `descript_balloon:type`・`id` は `status = "absent"`・`owner = ""` で本日も同じ）。列挙の素性は `parse_kv` の表から直接引く（`BalloonModel` を経由する理由が無い）。

### 1.4 永続化（`crates/areka-sylphya/src/persist/`）

- `pub enum PersistScope { App, Ghost, Shell, Balloon }`・`pub struct ScopeRoots { app, ghost, shell, balloon: Option<PathBuf> }`・`pub enum PersistKey { WindowPos{scope,axis}, BalloonOffset{scope,axis}, BootCount, VanishCount }`（`Copy`）・`pub fn load_scope(scope, &ScopeRoots, &dyn PersistIo) -> Vec<(PersistKey, String)>`・`pub fn save_scope(...)`（`persist/mod.rs`）。crate 直下へ `pub use persist::{Axis, PersistKey, PersistOutcome, PersistScope, ScopeRoots, load_scope, save_scope}`・`pub use persist::io::{FakePersistIo, FsPersistIo, PersistIo}`（`lib.rs`・`persist/mod.rs`）。`FsPersistIo` はユニット構造体（`pub struct FsPersistIo;`・`persist/io.rs`）なので**ゴースト起動前に `load_scope(PersistScope::App, &roots, &FsPersistIo)` を直接呼べる**。
- 鍵の族を足す変更点は 3 関数＋型 1 つ: `PersistKey::to_canonical_key`（`match self` 網羅）・`fn apply_entry`（`match key` 網羅）・`fn doc_to_entries`（族ごとの push）・`FormatDoc`（`persist/format.rs`: `window`／`balloon_offset`／`boot_count`／`vanish_count` の 4 欄＋`is_all_absent`・`to_toml_string` の表 `window`／`balloon-offset`／`boot`／`vanish`・`read_toml_str`）。`[last]` 表に `ghost`／`balloon`／`shell` の文字列 3 欄を足す形が既存の `[boot] count` と同型。
- `PersistKey::` を綴る本番ファイルは `areka/src/placement/persist.rs`・`areka-ghost/src/prop_sink.rs`・`areka-ghost/src/runtime.rs`・`areka-sylphya/src/actor.rs`・`persist/mod.rs` の 5 つで、**`PersistKey` を網羅 `match` するのは `persist/mod.rs` の 2 関数だけ**（`actor.rs` の `match` は `parse_dotted` と `SylphyaMsg` に対するもの）。族の追加で他 crate のコンパイルは壊れない。
- 正準 key `areka.last.ghost` 等は `areka.boot.count` と同じ 3 段の点付き名なので `parse_dotted` との往復（檻 `canonical_key_round_trips_with_to_canonical_string`・`all_families()` に新 variant を足す）は既存の型で通る。`DOTTED_ROOTS`（`vocab/dotted.rs`）に `areka` は無いが、永続の投影は `MirrorImage::dotted_global` へ直接挿す（`actor.rs` の起動時ロード: `for (key, value) in load_scope(...) { img.dotted_global.insert(key.to_canonical_key(), value) }`）ので語彙台帳の登記は要らない（`areka.boot.count` の前例）。
- 行数: `persist/mod.rs` 934 行のうち `#[cfg(test)] mod tests` が 371 行目から末尾（**約 564 行がテスト**）。族を足すと目安を超えるので、テストを兄弟ファイルへ出す。命名規則（`structure.md`「ファイル名の導出規則」: `bar/mod.rs` の stem は親ディレクトリ名）により **`persist/persist_tests.rs`**、接続は `#[cfg(test)] #[path = "persist_tests.rs"] mod tests;`。テストは `crate::test_log_capture::{assert_logged, capture}` を使っているので移設後も `use super::*` で届く。`format.rs` は 464 行（テストは 249 行目から）で余裕がある。
- 書き口: `SylphyaPublisher::persist_put(&self, scope: PersistScope, entries: Vec<(PersistKey, String)>)`（`actor.rs`）。`GhostRuntime::sylphya_publisher()` が返す（`runtime.rs`）。書き込みはアクター経由の非同期（`SylphyaMsg::PersistPut` → `save_scope` → `FsPersistIo::commit`＝temp→rename）。`barrier()` で反映を待てる（`main_persist_wiring_seam_tests.rs` が `barrier` 後に別ハンドルの `load_scope` で読み戻す形の前例）。
- App スコープの根: `fn default_app_profile_dir()`（`boot_config.rs`・`AREKA_PROFILE_DIR` 優先・既定 `<exe>/profile/areka/`）を `ghost_boot_options`（`boot_config.rs`）と **`emo2_boot/mod.rs`（実測 527 行 `app_profile_dir: Some(crate::default_app_profile_dir())`）の 2 か所**が `GhostBootOptions.app_profile_dir` へ渡す。起動前の読みも同じ関数を使えば、読む場所と書く場所が同じファイル `<app_profile_dir>/sylphya.toml` になる。ゴーストの記憶の根は `boot_with_kanade_stop` が `sylphya_wiring::profile_areka_root(&mount.shiori.dir)`（`<ghost>/ghost/master/profile/areka/`）で決め、`create_dir_all` で先に作る（`runtime.rs`）。App スコープの dir は誰も先に作らないが、`FsPersistIo::commit` が commit 時に `create_dir_all` する二重の安全網がある（同所のコメント）。

### 1.5 検体の窓口と根の形（`crates/sample-ghost-kit`・`crates/areka-nar`）

- `SampleRoot::acquire(名)`／`root()`（**ベースウェアの根**・直下に `ghost/`・`balloon/`）／`folder()`（`<根>/ghost/<名>/`）／`balloon(名)`（`<根>/balloon/<名>/`）（`sample-ghost-kit/src/lib.rs`）。登記表 `SAMPLES` は 5 行（`emo2`〈balloons: `emo2-kakukaku`〉・`R_POST_and_KOMAINU`・`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`・`konnoyayame`）。`from_copy` が `check_registry`（`installed_elements`＝`<根>/ghost/*`・`<根>/balloon/*` を `read_dir` で数えて登記と照合）を通すので、**`emo2` の根はゴースト 1・バルーン 1 が構造的に保証される**＝要件 7.5 ②「唯一のゴーストと同梱バルーン」の前提は成立する。
- `installed_elements`（同 lib.rs・`fn`・私有）は本仕様の列挙とほぼ同じ走査（`read_dir` → `NotFound` は空 → 名前を集めて `sort()`）だが **テスト専用 crate で本番依存に置けない**（`sample_path_guard_test.rs` の `the_sample_gateway_never_appears_in_a_production_dependency_table`）。形の参考にはなるが再利用はできない。
- `sample-ghost-kit` は `areka`・`areka-ghost` の `[dev-dependencies]` に既にある（両 Cargo.toml で確認）。`temp-path-kit` も同様。**新規依存 0 でテストは組める。**
- 展開先の形は完了 spec `areka-P0-nar-install` 要件 4.2（「アーカイブ最上位の内容（`install.txt`・`readme.txt`・`ghost/`・`shell/`・その他）を `<根>/ghost/<directory>/` へ置き、同時インストールのバルーンの取り出し元フォルダは除く」）・4.3（同梱バルーンは `<根>/balloon/<*.directory>/`・ゴースト側に複製を残さない）・4.8（`supplement` は最上位の `install.txt` を重ねない）で固定されている。**`<根>/ghost/<名>/install.txt` は残る**＝要件 5.2 の `balloon.directory` はそこから読める。
- `StayseeBalloon` の実物 `vendors/sample_ghost/StayseeBalloon/descript.txt`（本日 cat）: `charset,Shift_JIS`・`type,balloon`・`name,Balloon for Staysee Syncfield`・`id,StayseeBalloon`・`craftman,SSP BUGTRAQ`・`craftmanw,ばぐとら研究所/整備班`。要件 2.8 の「Shift_JIS 宣言の既定バルーンの `name`・`craftmanw` を文字化けなく」は `charset::decode` の宣言読みで満たせる（prescan が `charset` 行を見る）。`.nar` 化と `SAMPLES` 登記は並走 `areka-P0-default-balloon-nar-fold`（brief: `SampleRoot::acquire("StayseeBalloon")` が取れるようにする）。

### 1.6 `install.txt` の読み手（`crates/areka-nar/src/manifest.rs`）

- `pub(crate) fn parse_manifest(bytes) -> Result<InstallManifest, RefuseReason>`: `decode(bytes, DefaultEncoding::Ansi)` → `parse_kv` → 鍵を小文字化 → `type`／`name`／`directory` 必須で拒否 → `Companion { key: "balloon"/"balloon0"…, directory, source_directory, existing }` を並べる。`InstallManifest.companions: Vec<Companion>` は `pub`。
- **`areka-nar` は `areka`・`areka-ghost` の本番依存ではない**（依存しているのは `sample-ghost-kit` の 1 crate だけ・`grep -rln areka-nar crates/*/Cargo.toml` で確認）。`parse_manifest` も `pub(crate)`。要件 5.2 の読み手は (a) 新モジュール内に `decode`＋`parse_kv`＋`balloon.directory` の 1 鍵取り出し（数行）を書く、(b) `areka-nar` の `parse_manifest` を `pub` にして `areka-ghost`（または `areka`）の本番依存に `areka-nar`（ワークスペース path 依存・外部依存ではない）を足す、の 2 案。(b) は拒否条件（`name`／`directory` 必須・`directory` の 1 段検査）を第三者の展開済みゴーストにも課すことになり、`install.txt` が壊れているだけで同梱バルーンが引けなくなる。(a) は「`balloon.directory` 1 鍵・番号付きは読まない」という要件 2.9／Out of scope の縮退そのもの。

### 1.7 ダミー窓と終了経路（`crates/areka/src/main.rs`・`app_exit.rs`）

- 退役対象（実測）: `pub struct DummyWindowMarker`（`main.rs`）・`fn spawn_dummy_window`・`fn on_dummy_pressed`（`main.rs`）・`ExitOrigin::DummyWindow`・`pub(crate) fn on_dummy_os_close`（`app_exit.rs`）・`fn despawn_app_windows` の query `Or<(With<DummyWindowMarker>, With<GhostWindowMarker>)>`（`app_exit.rs`・`With<GhostWindowMarker>` 単独へ縮小＝要件 7.2 括弧書き）・`use crate::DummyWindowMarker;`（`app_exit.rs`）。
- ダミー窓の消去で `main.rs` から不要になる import: `D2D1_COLOR_F`・`BoxSize`／`BoxStyle`／`Dimension`・`DoubleClick`／`OnPointerPressed`／`Phase`／`PointerState`・`Brushes`・`Rectangle`・`ChildOf`（実測 26〜34 行・ダミー窓とその子矩形だけが使う）。`use windows::Win32::UI::WindowsAndMessaging::*;` は `WS_POPUP` 等のために残っているが、ダミー窓が消えると `main.rs` での用途は `MessageBoxW` だけになる。
- 依存テスト: `main_startup_window_tests.rs` の 11 本のうち **ダミー窓 5 本**（`dummy_window_has_minimal_components_and_no_position_claim`・`dummy_window_has_visible_rectangle_child`・`double_click_left_despawns_all_dummy_windows`・`non_left_double_click_does_not_despawn_dummy`・`tunnel_phase_double_click_is_ignored_for_dummy`）が退役、**`smoke_exit_ms_*` 6 本**は残す（ファイル名と冒頭 doc の言い換えが要る）。`app_exit_tests.rs` は `despawn_app_windows_hits_dummy_and_ghost_only`（ゴースト窓だけへ更新）と `despawn_app_windows_skips_cascade_despawned_target_without_warning` の `probe()` 内 `world.spawn(DummyWindowMarker)`（実測 96 行・`GhostWindowMarker` 系へ差し替え）の 2 か所。
- 分類関数 `is_benign_placement_error`（`main.rs`）と `is_benign_boot_error`（`boot_config.rs`）はどちらも「`default_ghost_root()` はプレースホルダで不在が常態」を根拠にしている（doc に明記）。既定パスが消える本仕様でこの根拠は失われる。`is_benign_boot_error` は `emo2_boot::wire_emo2_boot` のフォールバック分類にも使われる（`boot_config.rs` の doc・R7.4）。檻は `main_seam_tests.rs`（2 本・`PlacementError` 全 variant を構築して分類を確かめる）と `main_ghost_wiring_tests.rs`。**退役か doc の書き換えだけかは設計判断**（§6）。
- 「ダミー窓」「`spawn_dummy_window`」を綴る本番／テストファイル（実測・`Select-String` の件数）: `main.rs` 34・`main_startup_window_tests.rs` 25・`app_exit.rs` 10・`app_exit_tests.rs` 6・`placement/mod.rs` 3・`placement/spawn.rs` 2・`placement/measure.rs` 1（placement 側は doc コメントのみ・「シームが `spawn_dummy_window` へフォールバック」の言及）。steering／`doc/COMPAT_ARCHITECTURE.md` には現在形の言及なし（`roadmap-history.md` の履歴 8 件のみ＝履歴は触らない）。

### 1.8 常設 smoke テスト（`crates/areka/tests/smoke_boot_loop_exit.rs`）

- `fn run_smoke(args: &[&str]) -> (ExitStatus, String, String)`: `CARGO_BIN_EXE_areka` を `AREKA_APP_SMOKE_EXIT_MS=500` で起動し、60 秒番犬（`WATCHDOG_DEADLINE`）・50ms ポーリング。**env を渡す口は固定 1 つ**（`AREKA_APP_SMOKE_EXIT_MS`）なので、`AREKA_ROOT`・告知抑止・`AREKA_PROFILE_DIR` を渡せる形（`envs: &[(&str, &str)]`）へ広げる。
- 2 本: `skeleton_boots_loops_and_exits_zero_within_watchdog`（引数なし・目印「窓配置の準備起点が見つかりません」「検証用ダミー窓を開きました（placement フォールバック）」）と `skeleton_boots_with_real_ghost_windows_and_exits_zero`（`EMO2.folder()`・`EMO2.balloon("emo2-kakukaku")`・目印「emo2-boot: 実 sink 結線が成立しました（wire 成立）」「本物のゴースト窓を開きました」・モニタ 0 台は「窓配置の準備に失敗しました」＋「モニタ」でダミー窓完走を受理）。
- **プロファイルの汚染**: 本物方向は argv が根の外（`AREKA_ROOT` 未設定なら根＝`target/debug/`）なので裁定 5 により記憶は書かれない。根方向（②）は記憶を書くので、`AREKA_PROFILE_DIR` を `temp_path_kit::TempPath` で作った空のフォルダへ向けないと、`target/debug/profile/areka/sylphya.toml` に前回の走行の `[last]` が残り「記憶なし」の前提が崩れる（2 回目以降は記憶経路で当たる）。同時 4 プロセスの走行でも `TempPath` は一意（`temp-path-kit` の目的）。
- `sample_path_guard_test.rs` の走査語 ⑷（同梱バルーン名でのパス組み: `join("emo2-kakukaku")`・`emo2-kakukaku/`・`/emo2-kakukaku"`・`emo2("emo2-kakukaku")`）は smoke／新テストにも効く。`EMO2.balloon("emo2-kakukaku")`・`EMO2.root()` の形は当たらない（`the_window_reader_arguments_and_prose_are_not_hits`）。根方向で「同梱バルーンで起動した」ことを確かめるなら、パスを綴らず**記録の目印**（要件 5.11 の info・経路＝同梱）で見る。

### 1.9 環境変数の読み口の型（既存の流儀）

- 本番 crate で env を読むのは `boot_config.rs`（`AREKA_PROFILE_DIR`・`var_os`）・`main.rs`（`AREKA_APP_SMOKE_EXIT_MS`）・`tick_gate_config.rs`（`AREKA_TICK_GATE`）・`shiori_demo.rs`・`perf_thread_report.rs`・`emo2_boot/balloon_visibility.rs`・`emo2_boot/hover_inject.rs` の 7 か所で、**すべて「純粋な `fn xxx_from(value: Option<&str>)` ＋ env を読む薄い `fn xxx()`」の 2 段**（`smoke_exit_ms_from`／`tick_gate_from_env_value` 等）。要件 8.2「値を注入できる形にし、プロセスの env を書き換えない」はこの型をなぞれば満たせる。`AREKA_ROOT` はパスなので `var_os` → `Option<PathBuf>` を注入値にする（非 UTF-8 パスを落とさない・`AREKA_PROFILE_DIR` と同じ）。
- `AREKA_ROOT`・`MessageBoxW`・`menu,hidden` の綴りは `crates/**/*.rs` で 0 件（本日 grep）。衝突なし。

### 1.10 番人（ワークスペース常設検査）

- 1,000 行: `log-capture-kit/tests/file_length_guard_test.rs` の例外表 `OVER_LIMIT_ALLOWED`（10 件・件数定数 `OVER_LIMIT_ALLOWED_COUNT`）。本仕様が触る `main.rs` 909・`boot_config.rs` 159・`app_exit.rs` 167・`persist/mod.rs` 934・`format.rs` 464・`areka-ghost/src/lib.rs` 49・`runtime.rs` 683（触らない見込み）は表に無い。見積り: `main.rs` はダミー窓 2 関数＋型（約 100 行）と存在確認ループ・フォールバック各アーム（約 40 行）が減り、解決の配線と告知の呼び出し（約 60 行）が増えて **約 830 行**。`boot_config.rs` は既定パス 2 関数が減り、根の解決（約 40 行）が増えて 200 行前後。列挙・解決順・告知は新規ファイルなら各 200〜400 行で収まる。
- 一時パス: `temp_path_guard_test.rs` は `std::env::temp_dir(` の直接呼びを許可表（16 件）以外で赤にする。新テストは `temp_path_kit::TempPath::new(札)`／`child(名)` を使う。
- 台帳: `doc/ukadoc-coverage/ledger/assets.toml` の `descript_shell:menu,hidden`（`priority = "A11"`）・`descript_balloon:type`／`id`・`descript_ghost:craftman`／`craftmanw` 等は `status = "absent"`・`owner = ""`。互換機能を着地させた spec は同じ PR で台帳を更新してきた（`structure.md`「ukadoc Survey Toolkit」・PR#159／#162 の前例）。本仕様の実装 PR も「配布物の素性」束のうち読むようになる欄（`name`・`craftman`・`craftmanw`・`id`・`type`・`readme`・`menu,hidden`）の判定を更新する（`cargo test -p ukadoc-survey` が整合を見張る）。

### 1.11 `MessageBoxW`

- `windows` 0.62.2（`c:/rust/cargo/registry/src/index.crates.io-…/windows-0.62.2/src/Windows/Win32/UI/WindowsAndMessaging/mod.rs`）に `pub unsafe fn MessageBoxW<P1, P2>(hwnd: Option<HWND>, lptext: P1, lpcaption: P2, utype: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT` がある。機能 `Win32_UI_WindowsAndMessaging` はワークスペースの `[workspace.dependencies.windows].features` に含まれ、`main.rs` が `use windows::Win32::UI::WindowsAndMessaging::*;` で既に読み込んでいる。**新規依存 0・feature 追加 0。** `hwnd: None` で `WinApp` 構築前（COM／DPI 初期化前）でも出せる。`lptext` は `PCWSTR`（`HSTRING` から `&HSTRING` → `PCWSTR` に変換・既存の `windows` 流儀）。

## 2. 要件→資産マップ

| 要件 | 既存資産 | ギャップ | 区分 |
|---|---|---|---|
| 1.1〜1.4 根の解決 | `default_app_profile_dir`（`AREKA_PROFILE_DIR` → `current_exe().parent()`）が同じ形の前例 | 根の型と `AREKA_ROOT` の読み口・実在検査・`current_exe()` 失敗時の扱い（今日は `"."` へ寛容フォールバック・要件 1.4 は非 0 終了） | Missing |
| 1.5 既定パスの撤去 | `default_ghost_root`／`default_balloon_root`＋檻 4 本 | 撤去と檻の退役。`is_benign_*` の doc の根拠も消える | Constraint |
| 1.6 検体の根 | `SampleRoot::root()`・`check_registry` | なし（`AREKA_ROOT=root()` を渡すだけ） | — |
| 1.7 記憶の保存先 | `default_app_profile_dir`（2 呼び手） | なし | — |
| 2.1〜2.7 列挙と素性 | `charset::decode`・`kv::parse_kv`・`MountModel.readme`・`readme::resolve_path`（bin・`readme` 鍵 → 既定 `readme.txt`） | 走査（`read_dir`）・`ghost/master/descript.txt` の有無・`menu,hidden`・`type` の判定・`thumbnail.png`・並び（バイト順）・読めない項目の `warn!` と除外 | Missing |
| 2.8 charset | `charset::decode` | なし | — |
| 2.9 読まない素性 | — | 縮退の明記（台帳の判定を「読む／読まない」で書き分ける） | Constraint |
| 2.10 `fn resolve` 不変 | 4 呼び手 | なし | — |
| 3.1 鍵 3 つ | `PersistKey` 4 族・`FormatDoc`・`[boot] count` の前例 | 族の追加（`to_canonical_key`／`apply_entry`／`doc_to_entries`／`FormatDoc`／TOML 表 `[last]`）＋テストの兄弟ファイル化 | Missing |
| 3.2〜3.5 起動成功時の書き込みと根の内外の判定 | `runtime.sylphya_publisher().persist_put(scope, entries)`・`GhostRuntime::mount().shell.dir` | 「起動成功」の時点の定義・根の直下かの判定（Windows のパス比較）・書き込みの反映を終了前に保証するか（`barrier`） | Unknown |
| 3.6〜3.7 同じ経路・往復の檻 | `save_scope` の read-modify-write・`FakePersistIo`・`main_persist_wiring_seam_tests.rs` の前例 | なし（前例をなぞる） | — |
| 3.8 シェルの記憶を起動に使わない | `resolve` は `seriko.defaultsurfacedirectoryname` しか見ない | なし（後続 spec の口） | — |
| 4.1〜4.11 ゴーストの解決順 | `resolve_config_inputs` の argv 採用 | 純粋な判断関数（argv／記憶／唯一／既定 `emo2`／無作為／0 体・記憶の指す先が無い・2026-09-23 裁定 3 で改定）と経路の info | Missing |
| 5.1〜5.11 バルーンの解決順 | `SampleRoot::balloon` が示す「`install.txt` の `balloon.directory` → `<根>/balloon/<名>/`」の対応 | `install.txt` の読み手（§1.6 の (a)/(b)）・既定バルーン id の定数・7 分岐の純粋関数 | Missing |
| 6.1〜6.3・6.5 告知 | `MessageBoxW` 結線済み・env の 2 段の型 | 告知関数（4 場面・日本語文面・絶対パス）・抑止 env 名・`error!` との対 | Missing |
| 6.4 起動窓を開けない | `open_startup_window` の `Err` アーム・`PlacementError` 4 variant | ダミー窓フォールバック → 非 0 終了へ。`smoke_exit_ms` の投入位置の移動。`None` を受けて続行する `main` 側の縮退分岐（作者基準 DPI）の撤去 | Constraint |
| 6.6 退役 | §1.7 の一覧 | コード・doc の一掃（placement 側の doc 3 ファイルも） | Constraint |
| 6.7 4 場面の `error!` 檻 | `log-capture-kit`（`areka` の dev-dep）・`main_seam_tests.rs` が `PlacementError` を構築する前例 | 告知関数を「抑止フラグを引数で受ける純粋寄りの形」にして檻に入れる | Missing |
| 7.1 argv 起動不変 | `emo2_real_run.rs`・examples は `SampleRoot` 経由 | なし（argv は上書きとして残す） | — |
| 7.4〜7.5 テストの追随 | §1.7・§1.8 | `run_smoke` の env 口・3 方向・モニタ 0 台の受理条件の書き換え | Constraint |
| 7.6 1,000 行 | §1.10 | `persist/mod.rs` のテスト分離が必須 | Constraint |
| 8.1〜8.4 決定論テスト | `sample-ghost-kit`・`temp-path-kit`・`log-capture-kit`（すべて dev-dep 済） | 一時フォルダに根を組むヘルパ（ゴースト 2 体・シェル 2 つ・`menu,hidden`・`type` 違い・descript 無し・偽 `StayseeBalloon`） | Missing |

## 3. 実装アプローチ

### 3.0 置き場（brief の「`areka-ghost` に置く」を再検討）

| 候補 | 根拠 | 難点 |
|---|---|---|
| **⑴ `areka-ghost` に新モジュール**（brief 案・roadmap「A1 後段」の見込み） | parsers と sylphya の両方に依存する結線層で、後続の切替（`SwitchRequest`）・`property-catalog-lists`（`sylphya_wiring` の provider）が同じ crate で列挙を引ける。`lib.rs` は 49 行で余裕 | `GhostBootOptions.ghost_root` を組む主体（bin）と列挙の主体（ghost）が分かれる。`areka-ghost` の名は「1 体の結線」だが、複数体の目録が入る |
| ⑵ `areka-parsers` の `package/` の隣（`catalog`） | descript の読み方（`decode`＋`parse_kv`・鍵の名）が 1 crate に閉じる。`package::resolve` が既に fs を読むので「I/O 禁止」には当たらない | parsers は「1 体の解決」しか知らない設計方針（`package/mod.rs` の前置き「install.txt / balloon 系 / NAR には触れない」）。`install.txt` の読みを持ち込むと `validation_tests.rs`（それが解決結果へ漏れないことの檻）と方針が衝突する |
| ⑶ `areka` bin の新モジュール | 起動解決・告知・記憶の書き込みはどのみち bin にしか置けない（`WinApp`・`main`）ので、列挙も同じ場所に置けば crate 境界を跨がない | bin は他 crate から依存できない。後続の `property-catalog-lists`（`areka-ghost` の provider）が列挙を引けず、複製が生まれる |

判断の軸は「後続の消費者がどこにいるか」。切替 spec の brief は `list_shells`・`last.shell`・`last.balloon` を Upstream として本仕様に置き（`areka-P0-shell-balloon-switch/brief.md` の Upstream 欄）、`property-catalog-lists` は provider（`areka-ghost`）から供給する。**列挙と鍵の定義は ⑴、起動解決の純粋関数と告知は ⑶（bin・`boot_config.rs` の隣）**が自然な分割で、brief の見込みと一致する。

### Option A: 既存の拡張だけで組む

- `boot_config.rs` の `resolve_config_inputs(args)` を `resolve_config_inputs(args, root, last, listed)` へ広げ、`ConfigInputs` を返し続ける。列挙は `areka-ghost` の 1 ファイル、鍵の族は `persist/mod.rs`・`format.rs` へ追加。告知は `main.rs` の私有関数。
- ✅ 消費者 3 か所と型が不変。差分が最小。
- ❌ `boot_config.rs` に「根」「解決順」「告知」が同居し 300 行超へ。ゴースト／バルーンの 2 本の解決順（5＋7 分岐）を 1 関数に混ぜると檻が読みにくい。

### Option B: 新規コンポーネントに切り出す

- `areka-ghost/src/catalog.rs`（根の型・`list_ghosts`／`list_shells`／`list_balloons`・素性の型）＋ `catalog_tests.rs`／`catalog_test_support.rs`（一時フォルダに根を組むヘルパ）。
- `areka/src/boot_resolve.rs`（`AREKA_ROOT` の読み口・ゴーストとバルーンの解決順の純粋関数・経路の enum）＋ `boot_resolve_tests.rs`。`ConfigInputs` はこの関数の出力として残す。
- `areka/src/alert.rs`（4 場面の enum・日本語文面の組み立て・`MessageBoxW` の 1 呼び出し・抑止フラグ）＋ `alert_tests.rs`。
- `persist/mod.rs` へ族追加（ここは拡張以外の選択肢が無い）＋ `persist_tests.rs` へテスト移設。
- ✅ 判断分岐ごとに檻が独立し、要件 4.9／5.10／6.7／8.2 が 1 対 1 で対応する。`main.rs` は配線だけ。
- ❌ 新規 3〜4 ファイル。`main.rs` と `boot_config.rs` の doc・`mod` 宣言の更新。

### Option C: ハイブリッド（推奨の候補）

- B の分割を採りつつ、**根の読み口は `boot_config.rs` の `default_app_profile_dir` の隣**に置く（同じ `AREKA_` env と `current_exe()` の作法を 1 ファイルで見せる）。列挙は `areka-ghost` の新ファイル、解決順と告知は bin の新ファイル 2 つ、永続は既存拡張。
- 段取り: ① 永続の族と読み書き（`persist`）→ ② 列挙（`areka-ghost`）→ ③ 解決順の純粋関数 → ④ 告知 → ⑤ `main` の配線とダミー窓の退役 → ⑥ smoke 3 方向 → ⑦ 台帳と doc。①〜④ は互いに独立で並走でき、⑤ で合流する。
- ✅ 触る既存ファイルが `boot_config.rs`・`main.rs`・`app_exit.rs`・`persist/{mod,format}.rs`・`areka-ghost/src/lib.rs`・テスト 5 本・smoke 1 本に限られ、`runtime.rs`・`menu/`・`resolve` の呼び手に触らない（要件 7.2／7.3）。
- ❌ 配線の段（⑤）が最後に集中する。ダミー窓の退役と解決順の差し替えを同じタスクで行うと差分が大きい（分けるなら「退役」→「差し替え」の順）。

## 4. 規模とリスク

- **Effort: M（3〜7 日）**。新規ファイル 3〜4・既存改変 6・テスト改変 6・smoke 1。純粋関数の分岐（5＋7＋4 場面）が中心で、新しい技術は `MessageBoxW` 1 呼び出しだけ。
- **Risk: Medium**。理由:
  1. 記憶の書き込みの**反映のタイミング**（アクター非同期）と、smoke／実機の短い寿命（500ms 自動終了）との競合。`barrier()` を挟むか、終了統括（`runtime.shutdown` → sylphya の handle join）に任せるかで実機 ② の再現性が変わる。
  2. 「根の直下か」の**パス比較**（Windows: 大小無視・`\\?\` 接頭辞・末尾区切り・`canonicalize` の失敗）。argv で根の内側の絶対パスを渡した場合も要件 3.2 は記憶を書けと言うので、比較は避けられない。
  3. `open_startup_window` の失敗を非 0 終了にすると、**`WinApp` 構築後・`run()` 前に戻る経路**が初めてできる（`WinApp` の drop・COM の後始末・`thread_roles` の名簿）。既存の終了統括はすべて `run()` 復帰後の形。
  4. smoke ③ は空の根＋抑止 env で `error!` を出して終わる経路で、**stderr／stdout の捕捉が終了コードより先に閉じない**ことに依存する（既存の `wait_with_output` で足りる見込み）。

## 5. 設計フェーズへの推奨と Research Needed

推奨: **Option C**。理由は §3.0 と要件 7.2／7.3／7.6 の制約（触らないファイル・1,000 行）に最も素直に収まるため。

Research Needed（設計で確定する）:

- **R1 書き込みの反映保証**: `persist_put` 直後に `publisher.barrier()` を挟むか、`GhostRuntime::shutdown` が sylphya のアクターを join して未処理の `PersistPut` を流し切るかを `runtime.rs`／`actor.rs` で確かめる（`shutdown` の順序は本仕様で変えない＝要件 7.2）。
- **R2 パス比較の作法**: `std::fs::canonicalize` の戻り（`\\?\C:\…`）同士の `parent()` 比較で足りるか、`dunce` 等の外部依存は入れない（要件 7.7）ので std だけで決める。大小の違いは `canonicalize` が実体の綴りへ揃える。
- **R3 `WinApp` 構築後の早期 return**: `open_startup_window` の失敗で `Err` を返したときに `WinApp` の drop が安全か（`wintf` の `WinApp` の Drop 実装と `ExitPolicy::Explicit` の相互作用）。代替は「準備を `WinApp` 構築の前に済ませる」だが、準備（`prepare_ghost_windows`）は WIC＝COM 初期化済みスレッドを要する（`open_startup_window` の doc）ので順序は変えられない。
- **R4 `install.txt` の読み手**: §1.6 の (a)（数行の自前読み）か (b)（`areka-nar` を本番依存へ・`parse_manifest` を `pub` へ）。(b) は `ghost-install` が同じ依存を足す予定なので二重にはならないが、拒否条件の厳しさが要件 5.7（無ければ `warn!` で次へ）と合わない。
- **R5 鍵の大小**: `menu,hidden`・`type,balloon` の鍵と値の大小（`parse_kv` は保持・`areka-nar` は鍵を小文字化）。ukadoc の綴りは小文字。設計で「鍵は小文字化して引く／値は trim のみ」等を 1 行で決める。
- **R6 並びの定義**: 「フォルダ名の昇順（バイト順）」を `OsString` の `Ord`（Windows では WTF-8 の並び＝UTF-8 として有効な名前なら UTF-8 バイト順）で実装するか、`to_string_lossy()` の `String` で比べるか。非 UTF-8 のフォルダ名を列挙に含めるか（`GhostBootOptions.ghost_root` は `PathBuf` なので含められるが、記憶の値は `String`）。
- **R7 台帳の更新範囲**: 「配布物の素性」束 22 件のうち本仕様で `status` が変わる欄と、`menu,hidden`（A11・束「作り付けのメニュー」）の判定文。`cargo test -p ukadoc-survey` の整合検査の要求形。
- **R8 `default-balloon-nar-fold` 着地後の番人**: `StayseeBalloon` が `SAMPLES` に載ると `sample_path_guard` の走査語 ⑵ に `vendors/sample_ghost/StayseeBalloon/` が加わる。本仕様の本番定数 `"StayseeBalloon"` と `join("StayseeBalloon")` は ⑵⑷ の形に当たらない（⑷ はゴースト検体の `balloons` だけから組む）が、着地順が逆でも赤にならないことを確かめる。

## 6. 設計判断の議題（要件ディスカッションへ）

要件 9 の裁定候補 1〜5 に加えて、コードの実測から増えた分かれ目。番号は要件 9 の続き（6 は「覆す必要が出たら議題に」の条項なので 7 から）。

7. **`wired=false` の `LogSink` フォールバック boot を残すか。** 今日の `main` は `wire_emo2_boot` が `wired=false` を返すと `LogSink`×2 の boot へ倒して起動を続ける（`main.rs` の `else` アーム）。ダミー窓が消え「起動窓を開けない＝終了」になっても、この経路は「窓は開いたが実 sink の結線が倒れた」場合に残る。要件はこの経路に触れていない。残すなら `is_benign_boot_error` の doc の根拠（既定パスの不在が常態）だけを書き換える。退役させるなら要件 6 に 5 つ目の場面が要る。
8. **`is_benign_placement_error`／`main_seam_tests.rs`（2 本）の扱い。** 要件 6.4 で準備失敗は全て `error!`＋終了になるため「良性（`warn!`）」の分類は使い道を失う。退役（陳腐化として除く）か、`MountError::StartPointMissing` を「ゴーストが無い（要件 4.8 と同じ告知）」と「その他（要件 6.4 の告知）」の文面の分岐として残すか。
9. **記憶を書く時点。** 「ゴーストの起動が成功する」（要件 3.2〜3.4）を (a) `areka_ghost::boot` が `Ok` を返した時点、(b) 本物のゴースト窓が開いた時点（`open_startup_window` の Ok アーム＋boot Ok）、のどちらで取るか。(a) は wired／fallback 両経路で 1 か所（`insert_persist_wiring` と同じ場所）に置ける。(b) は窓の生成が ECS コマンド経由の非同期なので「開いた」の観測点が `FrameFinalize` の後になる。
10. **告知を抑止する env の名。** 候補: `AREKA_ALERT=0`（`AREKA_TICK_GATE=1|0` と同型）／`AREKA_NO_ALERT=1`／`AREKA_QUIET=1`。要件 6.5 は「`AREKA_` 名前空間で 1 つ」とだけ定める。smoke 3 方向と実機 ③ で使う。
11. **`current_exe()` が失敗したときの根。** 今日は `default_app_profile_dir`／`default_helper_exe_path` が `"."` へ寛容に倒れる。根も同じにすると要件 1.4 の「実在しない根」検査が `"."`（カレントディレクトリ）に対して走り、意図しない場所を根として受理しうる。`current_exe()` 失敗を「根が決まらない」として要件 1.4 の告知に含めるか。
12. **argv で根の内側のパスを渡したときの記憶。** 要件 3.2／3.3 は「`<根>/ghost/` の直下にある」で書くと決めており、argv でも該当すれば書く。実装は R2 のパス比較に依存する。比較を避けるなら「argv 起動は記憶を書かない」へ要件 3.5 を広げる案がある（裁定 5 の趣旨＝開発者の実機起動が第三者向けの記憶を汚さない、とは整合する）。
13. **列挙の素性の型と後続への供給形。** `property-catalog-lists`（`ghostlist`／`balloonlist`・α 後）が provider（`areka-ghost` の `sylphya_wiring`）から引ける形にするため、素性の型を `areka-ghost` の公開型にする（Option C ⑴）。bin 側の解決順は `&[GhostEntry]` を受ける純粋関数にすれば、供給源が変わっても判断は変わらない。
14. **`persist/mod.rs` のテスト移設のタイミング。** 族の追加と同じタスクで移すか（差分が大きい）、先に移設だけの機械的タスク（挙動 0 変更・`git mv` 相当）を切るか。1,000 行の番人は着手時点で赤にならない（934 行）が、族を足した瞬間に超えるので順序は「移設 → 追加」が安全。

### 6.1 要件ディスカッション（2026-09-23）での処理状況

| 議題 | 処理 | 行き先 |
|---|---|---|
| 7（`wired=false` フォールバック boot） | 本仕様の範囲外と確定。要件の Out of scope に明記 | doc の根拠の書き換えだけ設計（8 と同じ箇所） |
| 8（`is_benign_placement_error`／`main_seam_tests.rs`） | 設計判断 | `/kiro-spec-design` |
| 9（記憶を書く時点） | (a) `areka_ghost::boot` が `Ok` を返した時点、と要件 3.1 に定義（`main` の順序上、起動窓の準備はその前に済む） | 確定 |
| 10（抑止 env の名） | 設計判断（要件 6.5 のとおり） | `/kiro-spec-design` |
| 11（`current_exe()` 失敗時の根） | 要件 1.4 に「根が決まらない」として告知に含めた | 確定 |
| 12（argv で根の内側のパス） | 裁定 5 と統合して開発者議題へ | 要件 9.5 |
| 13（素性の型の公開形）・14（テスト移設の順序）・R1〜R8 | 設計判断 | `/kiro-spec-design` |
| 裁定 3（複数から 1 つ） | 既定ゴースト `emo2`（配布物に必ず同梱）→ 無作為。名前順は使わない | 確定（要件 4.4・4.5・4.11） |
| 裁定 4（バルーンの記憶） | `Ghost` スコープ・記憶 → 同梱。**設計への含意**: 起動前に起動するゴーストの `Ghost` スコープ（`sylphya_wiring::profile_areka_root(<ghost>/ghost/master)`）を `load_scope(PersistScope::Ghost, …)` で直接読む口が要る（§1.4 の App スコープの読みと同型・R1 の反映保証は両スコープに効く） | 確定（要件 3.1・5.2・5.3） |

## 7. 参照した正典・記憶

- ukadoc: 「全体の構成」（`manual_directory`）・`descript_shell` `menu,hidden`・`descript_balloon` `type`／`id`・`descript_install` `*.directory`（要件本文の表と同じ）。
- 記憶: areka-log-first-no-silent-failure（`error!`＋`Err`）・obsolete-vs-broken-test-policy（陳腐化テストは除く・壊れたら更新）・test-only-decision-branches-not-proven-wiring（配線の再テストをしない）・areka-runtime-env-naming（`AREKA_` 名前空間）・areka-real-machine-signoff-bounded-auto-exit・cite-by-what-it-is-not-by-line-number。
