# ギャップ分析: session-crate-split

> 2026-11-22・作業の木 `0dde0056`（main `00877299` の上に spec の初期化だけ）で測った。数は参考値で、設計の段で測り直す（要件の「設計の段への申し送り」1）。
> 入力: `requirements.md`（未承認）・`brief.md`（最後の「棚卸㊷の再測定」の節を優先）・steering（product・tech・structure・roadmap の「本体の crate 分け」と C25 の行）・前例 `completed/input-crate-split`・`completed/session-module-gather/research.md` 14.3・14.4。

## 1. 分析の要約

- **本番の外への参照は要件どおり**: `session/` の本番から本体の項目を名指す所は 28 行（C1〜C5）と `alert.rs:286` の 1 行で、要件 1 の行番号と一致した。ほかに `#[cfg(test)]` の口 `boot_config.rs:209`（`crate::cli::parse::parse`）が 1 行。本番のどの呼び先も「窓口が無ければ何もしない／偽／受信端の無い送出端」で戻る形なので、口が据わっていないときの倒れ方は今の「窓口が無い」ときと同じ答えにできる。
- **口の前例の形は 2 通り**: `BodySystems` は `register_systems` の**引数**で、World に残らない。`SessionHooks`・`InputHooks` は **World の資源**で、無ければ `error!` を残す。C1〜C5 の呼び先は `register_systems` の外（窓を作るクロージャ・結線・切替の系・読み直しの入口・実行系の起動）にあるので、どの案でも口は World の資源として据える必要がある。`BodySystems` の欄を足すだけでは届かない。
- **一番大きい穴（要件の仮定とぶつかる）**: `SwitchRig`（`ghost_switch_test_support.rs`）と `sample_test_support` は `sample-ghost-kit` を使う。この crate は番人（`sample_path_guard_test.rs` の `GATEWAY_USERS`＝`real-run-kit` だけ）と要件 3.7・5.9 により、`areka-session` の本番の表に optional でも書けない。そのため、要件 5.6 の仮定（`sample_test_support` を `test-support` で開く）と 5.8（`SwitchRig` を `test-support` で開く）は今の形のままでは成り立たない。検体を呼び手が渡す形に変えるか、置き場を分けるかを決める必要がある。
- **テストの分け方は「本体の項目を名指すファイル」より広い**: `resolve_boot_from`（`#[cfg(test)]`・`cli` を引く）を使う session のテストが 6 本（`boot_config_{balloon,copy,input,option,roots}_tests.rs`・`boot_config_callghost_tests.rs`）ある。`SwitchRig` を通る 32 本は `BODY_SYSTEMS` の本物の系（menu・install・update・mcp）と、逆にした後の C1〜C5 の本物の口に暗に頼っている見込みがある。ソースを読む見張りの 6 本も「本体の `src`」を歩く形のままでは、移すと見る範囲が変わる。
- **規模**: L（13〜16 タスクの見込みは妥当）。リスクは中〜高。理由は、口の形そのものより、テストの支え（`SwitchRig`・検体・`resolve_boot_from`・ソースを読む見張り）の置き場の組み合わせが多いこと。

## 2. 今の姿

### 2.1 集めた層の大きさ

| 項目 | 数 |
|---|---|
| `crates/areka/src/session/` のファイル | 122（`readme/`・`roots/` を含む） |
| うち `*_tests.rs` | 86 ファイル・28,406 行（物理行） |
| 本番（`*_tests.rs`・`*test_support*` を除く） | 非空行 9,231（brief の見込み 約 8,600 は空行と数え方の差） |
| 1,000 行に近いファイル | `ghost_switch_tests.rs` 982・`ghost_switch.rs` 928・`ghost_session.rs` 897・`reload_tests.rs` 859・`reload.rs` 753・`ghost_switch_test_support.rs` 721 |
| 1,000 行の番人の例外表で session を指す項 | 0 |

### 2.2 本番の外への参照（要件 1 の確かめ）

| 仕組み | 所 | 中身 | 呼ばれる場所の性質 |
|---|---|---|---|
| C1 | `ghost_session.rs:18`（`use crate::menu`）・`:298`・`:304`・`:307` | `menu::attach_release_handlers`・`file_drop::attach_file_drop_receivers`・`menu::shortcut::attach_shortcut_receivers` | 窓を作るクロージャの中（作業プールで動く・`world` を受ける）。入力の 2 件と交互に並ぶ: `attach_char_pointer_handlers` → **menu の解放** → `attach_balloon_pointer_handlers` → **投げ込み** → **近道** |
| C2 | `ghost_session_runtime.rs:16`（`use crate::menu`）・`:34`〜`:51` | `wire_menu`・枠の `register` 8 本・`install::names::reseed` | `rewire_runtime_consumers`（`boot_ghost` と shiori だけの読み直し `rebuild_runtime_in_place` の 2 か所から）。`wire_mouse_input` の後・`boot_seam::on_boot_ok` の前の**ひと続きの塊**。必要な引数は `&mut World` と `&GhostRuntime`（`kanade()` の写しと `reseed`） |
| C3 | `ghost_switch.rs:504`・`:513`・`:515`・`:525`・`:675` | `update::desk::on_switch_end(world, attempt == Target)` → `install::desk::on_steady` → `update::desk::on_steady`（定常に届いたとき・この順）／中止のとき `on_switch_end(world, false)`／降ろした後・起こす前に `install::desk::run_overwrite_between` | 通知の相の系の中・同期 |
| C4 | `reload.rs:309`・`:312`・`shell_balloon_switch.rs:131` | `update::desk::is_busy` → `install::desk::is_busy`（読み直しは `Updating`・`Installing` を分けて断る）／シェル・バルーンの切替は update だけ | 入口の判定・同期 |
| C5 | `runtime_boot.rs:24`・`:26`・`:89`・`:90` | 受け口の列（13 個）の 9・10 番目に `InstallCueSink::new(install::desk::raw_sender(world))`・`UpdateCueSink::new(update::desk::raw_sender(world))` | 実行系の起動のたび（`boot_runtime`） |
| ― | `alert.rs:286` | `crate::smoke_exit::smoke_exit_ms()` | 告知を出すかの判断 |
| テストだけ | `boot_config.rs:209`（`#[cfg(test)] resolve_boot_from`） | `crate::cli::parse::parse(args).options.boot` | テストの口 |

- 説明の中だけの当たり: `app_exit.rs:268` の `menu::request_close`（要件 4.6 の書き換えの対象）。
- `#[path = "../…"]` で `session/` の外を読む所は 0。

### 2.3 呼び先の「窓口が無いとき」の答え

| 呼び先 | 窓口（`InstallDesk`／`UpdateDesk`）が無いとき |
|---|---|
| `install::desk::on_steady`・`run_overwrite_between`・`update::desk::on_steady`・`on_switch_end` | 何もしない |
| `install::desk::is_busy`・`update::desk::is_busy` | 偽 |
| `install::desk::raw_sender`・`update::desk::raw_sender` | 受信端の無い送出端（`mpsc::channel().0`） |
| menu の結線・登記 | 結線の無い起動では解放を無視するだけ（`ghost_session.rs:297` の説明） |

→ 口が据わっていないときの倒れ方を「`error!` を 1 行残し、窓口が無いときと同じ答えを返す」にすると、要件 1.11（黙って何もしないことはしない）と振る舞いの不変（要件 8.1）の両方に合う。前例の `SessionHooks::of` は `#[cfg(not(test))]` で記録を絞るが、別の crate にすると、**本体のテストのビルドでは `areka-session` は `cfg(test)` にならない**。そのため、本体のテストで口を据えずに通る道（2.6）では `error!` が出る。

### 2.4 口の前例

| 前例 | 置き場 | 形 | 据え方 | 無いとき |
|---|---|---|---|---|
| `BodySystems` | `session/ghost_session.rs:36` | `#[derive(Clone, Copy)]`・欄 `menu_poll`・`after_input`（どちらも `fn(&mut World)`） | `main.rs:365` の `const BODY_SYSTEMS` を `register_systems` の第 3 引数で渡す（**World に残らない**） | ― |
| `SessionHooks` | `crates/areka-runtime/src/session_hooks.rs` | `#[derive(Resource, Clone, Copy)]`・欄 10 個（`fn(&World) -> …`・`fn(&mut World, …) -> …`） | `session::hooks::install` が `register_systems` の先頭で | `error!(session_hooks_missing)`（`cfg(not(test))`）と `NO_BODY` |
| `InputHooks` | `crates/areka-input/src/hooks.rs` | `#[derive(Resource, Clone, Copy)]`・欄 `escape: fn(&mut World)` | `session::app_exit::install_input_hooks` が `register_systems` の 2 行目で | `error!(input_hooks_missing)` と `AppExit::request_exit()` |

- コードの中で bevy の observer（`trigger`・`add_observer`）は本番に 0 件。知らせの形を observer で作ると「新しい仕組み」（要件 1.8 に反する）になる。

### 2.5 `SwitchRig` と系の登録

- `SwitchRig::new(scripts)` は World を組み、`register_systems(&mut world, notice_rx, crate::BODY_SYSTEMS)` を 1 回呼ぶ（`ghost_switch_test_support.rs:237`）。
- 使い手は 49 ファイル: session 32（と宣言の `mod.rs`）・本体 17（`install/` 4・`mcp/` 7・`update/` 2・`runtime_tests/frame_stay_on_top_lifecycle_tests.rs`・`main_session_mark{,_fallback,_write}_tests.rs`）。`SwitchRig::new` の呼び出しは 57 か所。
- `SwitchRig` が頼る crate: `sample_ghost_kit::SampleRoot`（欄に持つ・`acquire_emo2` で取る）・`shiori_host32_host`（偽の SHIORI の戻りの型）・`areka_runtime::spine`（`test-support`）。テストは欄（`rig.world` ほか）を直に触る。
- `BODY_SYSTEMS` を直に渡すのは、ほかに 4 か所（`app_exit_escape_tests.rs:140`・`ghost_session_reload_tests.rs:71`・`:179`・`ghost_session_restart_tests.rs:455`）。

### 2.6 口を据えずに C1〜C5 の道を通るテスト

`register_systems` を通らずに `boot_runtime` を直に呼ぶテストがある: `runtime_boot_tests.rs:196`・`:212`・`readme_script_e2e_tests.rs:46`・`script_security_e2e_tests.rs:82`（どれも session の中）。`runtime_tests/spine.rs` の `SpineHarness` は `session::hooks::install` だけを手で据える（`:308`）。これらは C5 の口が据わっていない状態で受け口の列を作る。今は窓口が無いので、受信端の無い送出端になっている。

### 2.7 テストの支えと番人の制約

| 支え | 行 | 頼るもの | `areka-session` の `test-support` で開けるか |
|---|---|---|---|
| `crate_source_test_support`（`crate_source!`・`crate_path`） | 45 | `areka_runtime::source_text_test_support::crate_path`（runtime の `test-support`）・`env!("CARGO_MANIFEST_DIR")`（呼び手の側で展開） | 開ける（番人の crate に頼らない） |
| `sample_test_support`（`acquire_emo2`・`emo2_root`・`emo2_balloon_root`） | 38 | `sample_ghost_kit::SampleRoot` | **開けない**（`sample-ghost-kit` を本番の表に optional でも書けない＝`sample_path_guard_test.rs` の `GATEWAY_USERS` と要件 3.7・5.9） |
| `ghost_switch_test_support`（`SwitchRig`） | 721 | `sample_ghost_kit`・`shiori_host32_host`・`areka_runtime::spine` | **今の形のままでは開けない**（同じ理由）。`shiori-host32-host` は前例（`areka-runtime` の `test-support = ["dep:shiori-host32-host", …]`）どおり optional にできる |
| `runtime_tests::spine::SpineHarness` | ― | 本体の `runtime_tests` | 本体に置いたまま（使う 2 本は `BODY_SYSTEMS`・menu も引く） |
| `log_history::capture_tests::hold`（`#[cfg(test)] pub(crate) mod`） | ― | プロセスに 1 つの写しの窓の排他 | 本体の `offline_dump_tests.rs:16` が握る。分けた後は、session のテストと本体のテストが別のプロセスになる |
| `readme::FakeOs`（`opener_test_support`） | ― | `temp_path_kit::TempPath` | 番人の crate に頼るので開けない（本体が使っているかは未確認） |

- 前例: `areka-runtime` は自分の `#[cfg(test)] sample_test_support`（`emo2_root`）を持つ（本体のものとは別）。検体の窓口 `SampleRoot::acquire("emo2")` は 1 行で呼べる。

### 2.8 本体のテストが使う session の `#[cfg(test)]` の項目（下書き）

`resolve_boot_from`（本体の `main_session_mark{,_fallback}_tests.rs`・session の 6 本）・`roots::single_env`（`install/startup_flow_tests.rs`・`main_session_mark*`）・`GhostSession::{for_test, with_memory_publisher, with_seriko_sink, dispatcher}`・`readme::{Opener::from_sender, FakeOs, …path}`・`log_history::capture_tests`・`write_failure::sites_tests`・`ghost_switch_test_support`。どれも `#[cfg(any(test, feature = "test-support"))]` に開けるか、本体へ移すかを 1 つずつ決める（設計の段で全数を測る）。

### 2.9 ソースを読むテスト

| ファイル | 読む所 | 移したときに起きること |
|---|---|---|
| `runtime_tests/zorder_wiring_tests.rs:33`・`:68` | `include_str!("../session/runtime_boot.rs")`・`("../session/ghost_session.rs")`・`t_zwi05` の受け口の列の字面（`crate::install::desk::raw_sender(world)` を含む） | `crate_source!("areka-session", "src/…")` へ。C5 の形で字面が変わる（要件 5.14） |
| `readonly_tests.rs:78`・`:109`・`:134` | 本体の `src` 全部・`src/main.rs`・`crates/` 全部 | `engage` を呼ぶのは `main.rs` だけ・`main.rs` での呼ぶ順を見る。session の `CARGO_MANIFEST_DIR` では本体の `main.rs` に届かない（上の crate を読むことになる） |
| `readme_tests.rs:383`〜`:390`・`session_end_deadline_tests.rs:450`〜`:457` | （`areka` の `src`・`areka-runtime`・`areka-input`）を歩く | 「areka」の根が session の `src` に替わり、本体の `src` が外れる（見る範囲が狭まる） |
| `log_history_convention_tests.rs:275`・`:319`・`:323`・`:339` | 取り決めの文書・本体の `src`・`areka-update`・`areka-ghost` | `crate_of("areka")` が本体の `main.rs` を指す。`areka_session` の行が要る |
| `session_end_sync_send_tests.rs:326` | ワークスペースの根 | 深さは同じ（`crates/<x>/../..`） |
| `boot_shell_tests.rs:347` | `crate_path(…)` の検体 | そのまま |

- `crate_source_test_support` の決まりは「読んでよいのは依存の向きで下の crate だけ」。session のテストから本体（上の crate）を読む形は、この決まりから外れる。

### 2.10 本番の依存（下書き）

session の本番が名指す crate: `tracing`・`areka_ghost`・`areka_runtime`・`areka_placement`・`windows`・`bevy_ecs`・`areka_kanade`・`areka_sylphya`・`wintf`・`areka_input`・`areka_user_data`・`areka_parsers`・`tracing_subscriber`（`log_history::init`）・`areka_seriko`・`areka_actor`・`thiserror`・`dola`・`areka_update`・`areka_emo_present`。どれも本体の表に既にある（`Cargo.lock` に新しい外の crate は増えない見込み）。`smoke_exit.rs` を移すなら `async-io` も加わる（本体の表に既にある）。`tracing-subscriber` は workspace の表で `env-filter` を持つ。番人 4（`env-filter` を宣言するのは `wintf` だけ）は crate 側で宣言しなければ当たらない。

### 2.11 ログの出所

- 直書きで `areka::session::…` を持つ所: `log_history.rs`（決まり 5 行）・`log_history_tests.rs` 29・`log_history_convention_tests.rs` 6・`reload_tests.rs` 1・`reload_skin_tests.rs` 1・real-run-kit（`launch.rs` 1・`launch_tests.rs` 10・`close_tests.rs` 1・`bin/real-run-fake-areka.rs` 2）・`tests/emo2_mcp_real_run.rs` 5・`doc/ssp-mcp/log-convention.md` 6・`doc/pasta-real-run.md` 2・`doc/testing.md` 1・`structure.md` 2。
- 既定の絞り込みは `EnvFilter::new("info")`（`log_history.rs:553`）で、名前に頼らない。`RUST_LOG` の前方一致は素の文字列なので、`areka=…` は `areka_session` も覆う。`areka::session=…` は新しい名前を覆わない（要件 6.3 の書き換えが要る）。
- `log_history::RULES` の `rule("areka", Status, true)` は完全一致なので、`areka_session` の行は別に要る（要件 6.5）。

## 3. 要件と資産の対応

| 要件 | 今ある資産 | 差 | 印 |
|---|---|---|---|
| 1.1〜1.7 向きを逆にする | 呼び先は 2.2 のとおり・前例 3 つ | 口の型・据え付け・呼び手の書き換え | Missing |
| 1.8 前例にそろえる | `BodySystems` は引数・ほか 2 つは資源 | どの前例の「形」に寄せるか（2.4） | Constraint |
| 1.11 口が無いとき `error!` | `SessionHooks::of`・`InputHooks::escape` | `cfg(not(test))` の絞りが別の crate では本体のテストで効かない（2.3・2.6） | Unknown |
| 2.1〜2.4 順と時 | 今の順は 2.2 のとおり | C1 は入力と交互・C3 は `on_switch_end` → install → update の順。順を記録する決定論のテストは無い | Missing |
| 3.1〜3.7 crate の新設・依存 | 前例 `areka-input` の `Cargo.toml` | 新しい `Cargo.toml`・依存表（2.10） | Missing |
| 3.8 1,000 行 | 例外表に session は 0 | `ghost_switch.rs` 928・`ghost_session.rs` 897 に口の行が足される | Constraint |
| 4.1〜4.7 公開面 | ― | `pub(crate)` の外からの使い手の全数（117 ファイル＋テストの `#[cfg(test)]` 項目 2.8） | Unknown |
| 5.1〜5.5 テストの分け方 | 本体の項目を名指す 21 本＋`resolve_boot_from` の 6 本＋`SwitchRig` の 32 本 | どれが本物の `BODY_SYSTEMS` を要るかは走らせないと分からない | Unknown |
| 5.6 支えの置き場 3 つ | 2.7 | `sample_test_support` は `test-support` で開けない | **Constraint（要件の仮定とぶつかる）** |
| 5.8 `SwitchRig` を `test-support` で | 2.5 | `sample-ghost-kit` に頼るので今の形では開けない | **Constraint（要件の仮定とぶつかる）** |
| 5.10・5.11 MTA と落ちの報告 | 前例 `areka-input/src/crash_report_tests.rs`（初期化子 3 行＋確かめ 1 本） | 同じものを `areka-session` に 1 つ | Missing（形は決まっている） |
| 5.12 `load-flake.ps1` | `-Binary` の 5 つの値（`:55`・`:72`・`:210`・`:276`） | `areka-session-lib` を 4 か所に | Missing（前例どおり） |
| 5.13・5.14 ソースを読むテスト | 2.9 | 本体を読む見張りの置き場 | Unknown |
| 6.x ログ | 2.11 | 書き換えだけ | Missing（機械的） |
| 7.x パスの書き換え | 要件 7.1〜7.4 の一覧 | 機械的・`areka-placement` の 4 か所は書き換えない | Missing（機械的） |
| 8.x 検証 | 前例の手順 | 実機の 6 つの道は前と後で同じ手順（作業の木の `target\` の下） | Missing（形は決まっている） |

## 4. 口の形（C1〜C5）の案

どの案でも、口は World の資源として `register_systems` の中で据える（2.4）。呼び先はどれも同期で呼ぶ（要件 2.3・「1 フレーム遅らせる解は取らない」）。

### 案 A: `BodySystems` を広げ、そのまま資源として据える（1 つの束）

- `BodySystems` に欄を足し、`#[derive(Resource, Clone, Copy)]` にする。`register_systems` が `world.insert_resource(body)` を先頭で行い、今の `(body.menu_poll)(world)`・`(body.after_input)(world)` はそのまま呼ぶ。
- 足す欄の見込み（C1 の順を保つため 2 つに分ける）:
  - C1 `after_char_handlers: fn(&mut World)`（menu の解放）・`after_balloon_handlers: fn(&mut World)`（投げ込み → 近道）
  - C2 `after_runtime_wired: fn(&mut World, &GhostRuntime)`（`wire_menu` → 登記 8 本 → `reseed`）
  - C3 `on_switch_end: fn(&mut World, bool)`・`on_steady: fn(&mut World)`（install → update）・`overwrite_between: fn(&mut World)`
  - C4 `install_busy: fn(&World) -> bool`・`update_busy: fn(&World) -> bool`
  - C5 `cue_sinks: fn(&World) -> [Box<dyn BootCueSink>; 2]`（列の 9・10 番目へそのまま入れる）
- ✅ 前例 `BodySystems` と同じ名前・同じ渡し手（`main.rs` の `BODY_SYSTEMS`）で、新しい型が要らない。`SwitchRig` が「登録の組」を受ける形（要件 5.4 の仮定）と同じ値をそのまま使える。
- ❌ 欄が 2 → 約 11 個になり、「登録のときだけ使う欄」と「後から何度も読む欄」が 1 つの束に混ざる。欄の取り違え（`session-module-gather` 13.10 で実験した型の同じ欄）が起きやすくなる。`ghost_session.rs`（897 行）に欄の説明が約 30 行足され、上限に近づく。

### 案 B: 登録の束 `BodySystems` はそのまま、別の資源の束（例 `BodyHooks`）を足す

- `BodySystems` は今の 2 欄のまま（登録だけ）。C1〜C5 は `SessionHooks` と同じ形の新しい束（`#[derive(Resource, Clone, Copy)]`・`of(world)` で引き、無ければ `error!(body_hooks_missing)` と「窓口が無い」の答え）に置く。束は session の新しいファイル（例 `body_hooks.rs`）に置き、中身は本体（`main.rs` か本体の新しいファイル）が持ち、`register_systems` が `BodySystems` と一緒に受けて据える（または `BodySystems` に束を 1 欄として持たせる）。
- ✅ 前例 `SessionHooks`・`InputHooks` と同じ「資源の束・無いときの倒れ方」。登録と呼び出しが分かれて読みやすい。`ghost_session.rs` の行が増えない。
- ❌ 型が 1 つ増える。`SwitchRig` へ渡す値が 2 つになるか、`BodySystems` が束を抱える形にする。

### 案 C: 本体が実装する trait（brief の古い節の推奨）

- `trait BodyPorts { fn after_char_handlers(&self, …); … }` を session に置き、本体が実装した値を `Box<dyn BodyPorts>` の資源（NonSend）として据える。
- ✅ 1 つの名前で口が揃い、偽物は trait の実装で書ける。
- ❌ 今の前例 3 つはどれも関数のポインタの束で、trait の口は無い（要件 1.8「前例にそろえる」から外れる）。`dyn` の資源を引く所で借用の形（`&mut World` を渡しながら資源を借りる）が複雑になる。brief の再測定の節も「新しい trait は要らない見込み」としている。

### 案 D: 仕組みごとの別の形（brief の古い節の C3・C4）

- C3 を知らせ（イベント・チャネル・observer）、C4 を install と update が立てる「忙しい」の資源を session が読む形にする。
- ✅ session が install と update の名前を一切持たない。
- ❌ チャネルやイベントの列は次のフレームに読まれるので、1 フレーム遅れる（要件 2.3 に反する）。同期の observer は本番に前例が無く（2.4）、install と update の 2 つの受け手の順の約束が要る。「忙しい」の資源は、窓口の状態（`busy`・`queue`・`overwrite`・`stage`・`after_switch`）が変わる全部の所で書き直す必要があり、「変わった時点が分からないものはプル」の決まりに反する。読む時点の状態がずれるおそれもある（要件 1.5）。

### 細部の論点（どの案でも決めること）

- **C1 の順**: 1 欄にすると、入力の 2 件との交互の順（2.2）が崩れる。崩さないなら欄を 2 つにする。崩しても差が無いことを示す案（同じ窓に別々の種類の受け手を差すだけ）もありうるが、要件 2.1 は「今の順と同じ」を求めている。
- **C3 の順**: `on_switch_end` は定常の枝の中で install・update の `on_steady` より先に呼ばれる。`on_steady` を 1 欄（install → update を本体の中で）にすれば順は本体の側で保てる。
- **C5 の口が無いとき**: 2 つの受け口を抜く（列が 11 個）か、受信端の無い送出端を持つ受け口を入れる（session は `InstallCueSink` を名指せないので、本体の型を使わない「捨てる受け口」が要る）か。`t_zwi05` は 13 個の並びを字面で押さえる。
- **口が無いときの `error!` の絞り**: `#[cfg(not(test))]` は `areka-session` 自身のテストでしか効かない。本体のテストで据えずに通る道（2.6 の `SpineHarness` ほか）が `error!` を出すかと、出たときに落ちるテスト（ログを捕まえて error の 0 件を確かめるもの）があるかを測る。
- **据え付けの確かめ**（要件 1.11）: 前例は `app_exit_escape_tests.rs` の「`register_systems` を通すと口が据わる」1 本。本体の `BODY_SYSTEMS` を渡したときに全部の欄が本物を指すこと（欄の取り違えの実験と同じ形）を確かめるテストの置き場は本体になる。

## 5. `smoke_exit` の置き場（要件 1.7）

| 案 | 中身 | ✅ | ❌ |
|---|---|---|---|
| S1（要件の仮定） | `smoke_exit.rs`（72 行）と `smoke_exit_tests.rs` を `areka-session` へ移す。本体の `main.rs:267`（`arm`）・`install/startup.rs:235`（`smoke_exit_ms`）は移した先を呼ぶ | 向きの逆転が 1 つ減る。`arm` は `wintf::WinApp`・`async_io`・`app_exit::quit_app` しか使わない | `areka-session` の依存に `async-io` が増える（本体の表に既にある）。出所 `areka::smoke_exit` が `areka_session::smoke_exit` に変わる（`get_log` の決まりには当たらない見込み・確かめる） |
| S2 | 判断の `smoke_exit_ms`・`smoke_exit_ms_from`・定数だけを session へ移し、`arm` は本体に残す | `async-io` が増えない | 1 つのファイルが 2 つに分かれ、テストも分かれる |
| S3 | 口（欄 `smoke_exit_ms: fn() -> Option<u64>` か告知の抑えの答え）を足す | ファイルは動かない | 欄が 1 つ増える。`alert::raise` は窓を開く前の道（起動の解決の失敗）でも呼ばれるので、口が据わる前に呼ばれないかを確かめる必要がある |

## 6. テストの分け方の案（要件 5.1〜5.5）

### 6.1 本体に残る候補（本物どうしをつなぐ）

- 根の関数を引く 7 本（要件 5.3）。
- menu・mcp・shiori_host の本物を見る: `ghost_session_restart_tests.rs`・`ghost_session_reload_tests.rs`（`SpineHarness`）・`ghost_session_switch_tests.rs`（とその子の `fallback`・`memory`）・`ghost_session_strict_tests.rs`・`shell_balloon_switch_session_tests.rs`・`ghost_session_scope_rect_boot_tests.rs`・`ghost_session_scope_scale_boot_tests.rs`。
- install・update の本物を見る: `readonly_run_tests.rs`・`readonly_sites_tests.rs`・`write_failure_sites_tests.rs`・`boot_resolve_mark_text_tests.rs`・`shell_balloon_switch_session_update_tests.rs`・`reload_tests.rs` の一部（忙しい窓口の 1 本・`:186`〜`:202`）・`shell_balloon_switch_tests.rs` の一部（`:39`）。
- `cli` を通す: `boot_config_callghost_tests.rs` と、`resolve_boot_from` を使う 5 本。
- 本体を読む見張り: `readonly_tests.rs` の `main.rs` を読む 2 本。

→ 1 本のファイルの中で一部のテストだけが本体を要るもの（`reload_tests.rs` 859 行・`shell_balloon_switch_tests.rs` 596 行）は、ファイルを分けることになる。テストの名前の末尾は変えずに分けられる（要件 8.2 の比べ方は名前の末尾の多重集合）。
- 本体に残すテストは `super::` で session の私有の項目を引いているもの（`use super::*` は `reload_tests.rs`・`shell_balloon_switch_tests.rs`・`ghost_session_restart_tests.rs`・`write_failure_sites_tests.rs`・`boot_resolve_mark_text_tests.rs` ほか）が多い。本体へ移すと、使っている私有の項目を `pub` か `test-support` で開く必要があり、公開面が広がる（要件 4.1・4.5 の記録が増える）。
- 置き場は `crates/areka/src/session/` が使えない（要件 3.3）ので、前例 `placement_tests/`・`runtime_tests/` にならった本体のテスト専用のモジュール（例 `crates/areka/src/session_tests/`）が要る。

### 6.2 `SwitchRig` の扱い

| 案 | 中身 | ✅ | ❌ |
|---|---|---|---|
| T1 | `SwitchRig` を `areka-session` の `test-support` で開く。検体（根・ゴーストのフォルダ・同梱バルーンの道と、寿命を持つ値 `Box<dyn Any>`）と登録の組（`BodySystems`／口の束）を呼び手が渡す形にする。各 crate に、検体を取って渡す数行の作り口を `#[cfg(test)]` で置く | 写しは作り口の数行だけ。session の 32 本は session で、本体の 17 本は本物の `BODY_SYSTEMS` で走る。`shiori-host32-host` は前例どおり optional | 作り口の 57 か所の呼び出しを書き換える（`SwitchRig::new(scripts)` の形を各 crate の作り口に残せば、書き換えは本体の 17 本の `use` だけになる）。欄の `pub` 化が要る |
| T2 | `SwitchRig` とそれを使うテストを全部本体に残す（`sample_test_support` と同じ所） | `SwitchRig` の形を変えない | session の 32 本（約 1.2 万行の見込み）が本体の実行ファイルに残り、「集めた層のテストは本体と別の実行ファイルで走る」（Introduction）が半分しか満たされない。私有の項目を大きく開く必要がある |
| T3 | `sample-ghost-kit` の番人の例外表に `areka-session` を足して `test-support` に optional で入れる | `SwitchRig` の形を変えない | 要件 3.7・5.9 に反し、番人の決まりを変える（`GATEWAY_USERS` は「本番の表に持ってよい」唯一の例外＝`real-run-kit`）。推さない |
| T4 | テスト専用の新しい crate（例 `areka-session-test-kit`）に `SwitchRig` を置き、本体と session が dev-dependency で使う | 番人の決まりに当たらない | session がその crate を dev-dependency で使うと、session の型が 2 組できる（dev-dependency の循環）。前例が無い |

- 偽物の組で session のテストを回すと、今は暗に通っている本物の系（`menu_poll`・`install::register`・`update::register`・`mcp::register`）と、逆にした後の C1〜C5 の本物が抜ける。session の 32 本のうちどれが本物に頼っているかは、偽物の組で走らせて測るのが確実（Research Needed 1）。

### 6.3 `resolve_boot_from`（`#[cfg(test)]`・`cli` を引く）

| 案 | 中身 | ✅ | ❌ |
|---|---|---|---|
| R1 | 本体のテストの支えへ移し、使う session の 6 本を本体に残す | テストの中身を変えない | 本体に残るテストが増える。6 本は `boot_config` の私有の項目も引くか確かめる |
| R2 | 口の引数に読み方（`fn(&[String]) -> BootRequest`）を足し、session のテストは起動の求めを手で組む | session のテストが session に残る | テストの中身（CLI の引数を通すこと）が変わり、要件 5.2 の「中身を変えない」に近い所に触れる |
| R3 | `cli::parse` の起動の求めの部分を session へ移す | ― | 範囲の外（cli は本体に残す・Out of scope） |

## 7. テストの支えの置き場 3 つ（要件 5.6）

| 支え | 要件の仮定 | 測った制約 | 代案 |
|---|---|---|---|
| `crate_source_test_support` | session の `test-support` へ移し、本体はそこから使う | 成り立つ（`crate_source!` は呼び手の `CARGO_MANIFEST_DIR` で展開・`crate_path` は runtime の `test-support`）。`#[macro_export]` か `pub use` の形を設計で決める | 各 crate が `areka_runtime::source_text_test_support::crate_path` を直に使い、マクロ 10 行だけを持つ |
| `sample_test_support` | session の `test-support` へ移す | **成り立たない**（2.7） | session のテストは窓口 `SampleRoot::acquire("emo2")` を直に呼ぶ（前例 `areka-runtime` は自分の `#[cfg(test)]` の支えを持つ）。本体の支えは本体に残す |
| `SpineHarness` | 本体に置いたまま・使う 2 本も本体 | 成り立つ | ― |

- `log_history::capture_tests::hold`: 分けた後は、session のテストと本体の `offline_dump_tests` が別のプロセスになる。本体の中で写しの窓を触るほかのテストが無ければ、本体は自分の排他を持つだけで足りる。あれば、session の `test-support` で `hold` を開く（番人の crate に頼らない）。

## 8. ソースを読む見張り（要件 5.13）

| 案 | 中身 | ✅ | ❌ |
|---|---|---|---|
| W1 | 本体を読む見張り（`readonly_tests.rs` の 2 本・`readme_tests.rs`・`session_end_deadline_tests.rs` の歩く 2 本・`log_history_convention_tests.rs` の在りかの判定）は本体に置き、歩く根の並びに `areka-session` を足す（前例 input-crate-split D11 の「根の並びに 1 つ足す」） | 「下の crate だけを読む」決まりに合う。見る範囲が狭まらない | 見張りのファイルを分けるか本体へ移す |
| W2 | session に置いたまま、`crate_path("areka", "src")` で本体を読む | ファイルが動かない | 上の crate を読む（決まりに反する）。`crate_path` の決まりの説明を変える必要がある |

## 9. 規模とリスク

- **規模: L（1〜2 週間・13〜16 タスク）**。口の逆転そのものは小さい（呼び先 29 行）が、テストの分け方（約 30〜40 ファイルの仕分け・ファイルの分割・私有の項目の開け方）・117 ファイルの書き換え・ソースを読む見張り・ログと文書の書き換えが重なる。
- **リスク: 中〜高**。理由: (1) `SwitchRig` と検体の制約で要件の仮定 2 つ（5.6・5.8）が崩れる。(2) 偽物の組で走らせたときに、どの session のテストが本物に頼っていたかは走らせるまで分からない。(3) 口が無いときの `error!` が本体のテストのビルドで出る（`cfg(test)` の絞りが効かない）。口の形そのものは前例があり、リスクは低い。

## 10. 設計の段への申し送り（Research Needed）

1. **偽物の組で session のテストを回す実験**: `SwitchRig` に何もしない登録の組を渡し、session の 32 本のうち赤になるものを数える（どれが本物に頼るかの全数・要件 5.4 の 1 本ずつの記録の元）。
2. **本体に残すテストが引く私有の項目の全数**: `use super::*` で引いている項目のうち、本体へ移すと開く必要があるもの（要件 4.5 の記録）。
3. **`#[cfg(test)]` の項目で本体が使うものの全数**（2.8）と、それぞれ `test-support` で開けるか（番人の crate に頼るか）。
4. **口が据わらない道の全数**（2.6）と、`error!` が出て赤になるテストがあるか（ログを捕まえて error の 0 件を確かめるもの）。
5. **C5 の口が無いときの列の形**（11 個か、捨てる受け口を入れて 13 個か）と、`t_zwi05` の新しい字面。
6. **`smoke_exit` を移したときの出所の名前**が `log_history` の決まりと実機の名指しに当たらないかの確かめ（`tools/perf/invoke-perf-run.ps1:29` の古い名指しは `audit-areka-app` への申し送り済み）。
7. **`readme::FakeOs`**（`temp_path_kit` に頼る）を本体のテストが使っているか。
8. **`areka-session` の本番の依存表の確定**（2.10）と、`cargo check -p areka-session --lib --features test-support` の警告。
9. **数の取り直し**（要件の申し送り 1）: 本体で `session::` を引くファイルは 117 と一致した（根 9・`install/` 20・`mcp/` 20・`menu/` 19・`update/` 11・`runtime_tests/` 24・`roots/` 6・`cli/` 4・`offline_dump/` 4）。`tests/` は `emo2_mcp_real_run.rs` が `session::` を引き、`smoke_boot_loop_exit.rs` は説明の中の `ghost_session::` だけ。

## 10b. 設計の段で決めること（要件の討議で設計へ回した）

1. 口の束を 1 つ（案 A）にするか 2 つ（案 B）にするか（どちらも関数のポインタの束を World の資源として据える）。
2. C1 の欄を 2 つに分け、入力の 2 件との交互の順を保つ形（要件 2.1）。
3. 口が据わっていないときの倒れ方: `error!` を 1 行残し、窓口が無いときと同じ答え（何もしない／偽／受信端の無い送出端）を返す。`cfg(not(test))` の絞りが本体のテストのビルドで効かない件の扱い（2.3・2.6）。
4. C5 の口が無いときの受け口の列を 13 個に保つ形（捨てる受け口）と、`t_zwi05` の新しい字面（要件 5.14）。
5. 本体に残すテストの置き場のモジュールの名前（例 `crates/areka/src/session_tests/`）と、一部だけ本体を要るファイル（`reload_tests.rs`・`shell_balloon_switch_tests.rs`）の分け方。本体に残すテストが引く私有の項目の開け方（要件 4.5 の記録）。
6. `crate_source!` を `#[macro_export]` で開くか、各 crate がマクロを持つか。`log_history::capture_tests::hold` を開くか。
7. 10 の Research Needed 1〜9。

## 11. 議題の候補（要件の討議へ）

1. **口の形**（要件 1.8・未決 1）: 案 A（`BodySystems` を広げて資源にする）か案 B（登録の束と資源の束を分ける）か。どちらも World の資源として据えるのは同じ。案 C（trait）・案 D（知らせ・資源）は前例と「1 フレーム遅らせない」から外れる。
2. **C1 を 2 つの欄に分けて入力との交互の順を保つか**（要件 2.1）。
3. **口が無いときの倒れ方**: `error!` を残し、窓口が無いときと同じ答えを返す形にするか。C5 は捨てる受け口を入れて 13 個を保つか、11 個にするか。
4. **`smoke_exit` の置き場**（要件 1.7・未決 2）: S1（移す・仮定）・S2（判断だけ移す）・S3（口）。
5. **`SwitchRig` の扱い**（要件 5.4・5.8・未決 5）: 要件の仮定のまま `test-support` で開くことは、検体の番人のため成り立たない。T1（検体と登録の組を呼び手が渡す）か T2（本体に残す）か。要件 5.8 の文の直しが要る。
6. **テストの支えの置き場**（要件 5.6・未決 6）: `sample_test_support` は `test-support` で開けない。session のテストが窓口 `SampleRoot::acquire` を直に呼ぶ形にするか。要件 5.6 の仮定の直しが要る。
7. **`resolve_boot_from` とそれを使う 6 本**（要件 5.1 の数に入っていない）: R1（本体に残す）か R2（引数で読み方を渡す）か。
8. **本体を読む見張りの置き場**（要件 5.13）: W1（本体に置いて根の並びに `areka-session` を足す）か W2（session から上の crate を読む）か。
9. **本体に残すテストの置き場の名前**（例 `crates/areka/src/session_tests/`）と、1 本のファイルの一部だけが本体を要るもの（`reload_tests.rs`・`shell_balloon_switch_tests.rs`）を分けてよいか。
10. **`session_end.rs` の名前**（要件 3.9・未決 3）・**呼び手の書き換えの形**（要件 4.4・未決 4）・**`load-flake.ps1`**（要件 5.12・未決 7）: 測った事実で答えが変わる所は無い。前例どおりの仮定で足りる見込み。

## 12. 設計の段の調べ（2026-11-22・作業の木 `b21fd552`）

> 入力: `requirements.md`（要件の討議の決め 7 つを含む）・この research.md の 1〜11・brief・steering（product・tech・structure・roadmap の「本体の crate 分け」と C25 の行）・前例 `completed/{input-crate-split,runtime-crate-split,runtime-module-untangle,session-module-gather}`・`crates/areka-runtime/src/session_hooks.rs`・`crates/areka-input/src/hooks.rs`・`BodySystems`・`BODY_SYSTEMS`。
> 使った手順: `kiro-spec-design` の `rules/design-principles.md`・`design-discovery-light.md`（既にある系の拡張なので軽い調べ）・`design-synthesis.md`・`design-review-gate.md`。外の依存を足さないので Web の調べはしていない。走らせたのは `git grep` と読むことだけで、`cargo` は回していない（数は静かに読んだ見込みを含む）。

### 12.1 数の取り直し（要件の申し送り 1）

| 項目 | 2026-11-22 の作業の木 | 要件・brief の数 |
|---|---|---|
| 本番の外への参照 | 28 行＋`alert.rs:286`（行番号は 2.2 と一致）。ほかに `#[cfg(test)]` の `boot_config.rs:209` | 一致 |
| 本体で `session::` を引くファイル | 117（根 9・`install/` 20・`mcp/` 20・`menu/` 19・`update/` 11・`runtime_tests/` 24・`roots/` 6・`cli/` 4・`offline_dump/` 4）＋`tests/emo2_mcp_real_run.rs`。`smoke_exit.rs` は移す側に入る | 一致 |
| 本体が名指す集めた層の項目のパス | 157 通り | （新しい数） |
| `SwitchRig` を使うファイル | 集めた層 30・本体 17。`SwitchRig::new` の呼び出し 57（集めた層 35・本体 22） | 49 ファイル（`mod.rs` の宣言を含めた数え方の差） |
| 本体の項目を引く集めた層のテストのファイル | 26 ファイル（`crate::<本体の項目>` を 1 つでも名指すもの・重なりを除く。支え 2 つ〔`crate_source_test_support` 4・`sample_test_support` 4〕だけを使うファイルと `ghost_switch_test_support.rs` を含む）＋`resolve_boot_from` だけを使う 5 本 | 21＋6（数え方の差） |
| 文書の中のパス | `doc/COMPAT_ARCHITECTURE.md` 38・網羅台帳 38（18・10・9・1）・`doc/ideas/` 8 ファイル | 一致 |
| 古い出所の名前 `areka::session` | 14 ファイル（`log_history.rs` 5・`log_history_tests.rs` 29・`log_history_convention_tests.rs` 6・`reload_tests.rs` 1・`reload_skin_tests.rs` 1・real-run-kit 14・`emo2_mcp_real_run.rs` 5・文書 9・`structure.md` 2） | 一致 |
| 番人の表で集めた層を指す項目 | 0（`file_length_guard_test`・`temp_path_guard_test`・`workspace_scan_test`・`with_default_guard_test`） | 一致 |
| 理由の文を持つ `#[allow]` | 0（`clippy::too_many_arguments`・`clippy::type_complexity` の 2 つだけ） | （新しい数） |

### 12.2 Research Needed 1〜9 の答え

1. **偽物の組で赤になる集めた層のテスト**（見込み・走らせていない）: 集めた層のテストで本体の項目を名指さないものは、本体の系の効き目を直には見られない。間接の効き目は 2 つ: ⑴ `update::register` が据える `UpdateDesk` は定常のたびに `homeurl` の照会を偽の SHIORI へ送る（`update/desk.rs:443〜`）。`SwitchRig` の記録はもともとこの照会を除いて見る（`ghost_switch_test_support.rs:446〜458`）が、進みの目印 `progress_probe` は照会も数えるので、待ちの打ち切りの時点が変わりうる。⑵ `install::register`・`update::register` は `exit_wait` の門を据える。終わりの待ちを見るテストが門の数に頼っていれば変わる。→ 設計の D9（段 2 で走らせ、赤だけを中身を変えずに本体へ移す）。
2. **本体に残すテストが引く私有の項目**: `use super::*` の親は、`ghost_session.rs`（6 本）・`boot_config.rs`（7 本）・`readonly.rs`（2 本）・`session_end.rs`・`write_failure.rs`・`boot_resolve.rs`・`ghost_session_switch_tests.rs`（子 2 本）・`ghost_session_scope_{rect,scale}_tests.rs`（子 1 本ずつ）・`reload.rs`・`shell_balloon_switch.rs`。全数はコンパイラのエラーで数える（設計の D7・4.5 の記録）。
3. **本体のテストが使う集めた層の `#[cfg(test)]` の項目**: `GhostSession::{for_test, with_memory_publisher, with_seriko_sink, dispatcher}`（本体の 30 ファイル余り）・`roots::single_env`（3）・`shell_balloon_switch::SwapFinish` の出し直し（6）・`log_history::capture_tests::hold`（`offline_dump_tests.rs`）・`ghost_switch_test_support`（17）・`boot_config::resolve_boot_from`（2）。どれも番人の crate に頼らない（`SwitchRig` は検体を呼び手から受ける形にすれば頼らない）。`readme::FakeOs`（`temp_path_kit` に頼る）・`boot_seam::insert_persist_wiring`・`opener::from_sender`・`readme::path` は本体が使わない（0 件）。
4. **口が据わらない道**: 本番は 0（`register_systems` を呼ぶのは `main.rs` だけで、窓を開く・最初のゴーストを起こすのはその後。オフラインの面の書き出し・SHIORI のデモ・cli は口の道を通らない）。テストでは、集めた層の `runtime_boot_tests.rs`・`readme_script_e2e_tests.rs`・`script_security_e2e_tests.rs`（`boot_runtime` を直に・C5）、本体へ移す `ghost_session_strict_tests.rs`（`boot_ghost` を直に）・`ghost_session_restart_tests.rs`（窓を開く道・C1）。2.6 の「`SpineHarness` は C5 の道を通る」は誤り: `SpineHarness` は `runtime_boot_options` だけを使い、`runtime_sinks`・`boot_runtime` を通らない（`runtime_tests/spine.rs:429`）。error の 0 件を確かめるテスト（`install/procedure_tests.rs:483`・`install/worker_tests.rs:158・170`）は口の道を通らない。warn 以上を数えるテスト（`reload_tests.rs` の `refuse`・`shell_balloon_switch_tests.rs` の `refuse`）は `SwitchRig` を通り口が据わる。→ 設計の D13。
5. **C5 の口が無いときの列**: `runtime_boot_tests.rs` の `sinks_are_thirteen_with_reload_then_window_state_last`・`rebuilt_window_state_sink_reaches_the_same_line` は口の無い `World::new()` で `runtime_sinks` を呼び、13 個と 12・13 番目を確かめる → 13 個を保つ（`areka_ghost::sink::DiscardSink` が既にある・設計の D4）。`t_zwi05` の新しい字面は設計の SourceGuards。
6. **`smoke_exit` の出所**: `areka::smoke_exit`・`areka_session::smoke_exit` のどちらも `log_history` の決まりに当たらない。real-run-kit・`RUST_LOG` の名指しも 0。`tools/perf/invoke-perf-run.ps1:736` の `smoke_exit=` は地の文。
7. **`readme::FakeOs`**: 本体は使わない（0 件）→ `#[cfg(test)]` のまま。
8. **本番の依存**: 集めた層の本番（34 ファイル）が名指す crate は 19（tracing・tracing-subscriber・bevy_ecs・windows・thiserror・wintf・dola・areka-ghost・areka-kanade・areka-sylphya・areka-parsers・areka-seriko・areka-actor・areka-user-data・areka-update・areka-emo-present・areka-runtime・areka-placement・areka-input）＋`smoke_exit.rs` の async-io で 20。`windows` の機能は `Win32_UI_Controls`（タスクダイアログ）ほかで、本体が上乗せする `Win32_UI_Controls_Dialogs` は `install/pick.rs` だけが使う。`cargo check` は回していない（段 3 で確定）。
9. **数の取り直し**: 12.1。

### 12.3 2.11 の訂正

- 2.11 の「`rule("areka", Status, true)` は完全一致なので、`areka_session` の行は別に要る」は誤り。集めた層の出所は今も `areka::session::…` で、`areka`（完全一致）には当たらない。根の `areka::session` もどの決まりにも当たらない。移した後の `areka_session` も当たらないので、前と同じ＝新しい行は要らない。直すのは決まりの 5 行の字面だけ。

### 12.4 設計の決め（合成の 3 つの見方）

- **一般化**: C1〜C5 は「集めた層の決まった所で本体の手続きを同期に 1 回呼ぶ」の同じ形で、今ある `BodySystems`（系の登録の 2 欄）もその特別な場合。型を増やさず、`BodySystems` を資源にして欄を足す（設計の D1）。中身の実装は本体の関数をそのまま指すか、今の行を写した小さな関数 4 つ。
- **作るか使うか**: 新しい仕組みは作らない（要件 1.8）。関数のポインタの束を `Resource` で据える形は `SessionHooks`・`InputHooks` の前例、口が無いときの倒れ方は `SessionHooks::of`、捨てる受け口は `areka_ghost::sink::DiscardSink`、`SwitchRig` の検体は `sample_ghost_kit::SampleRoot` の窓口、落ちの報告は `areka_runtime::crash_report_test_support`、実機の操作は `tools/mouse-input.ps1` を使う。外の crate は足さない。
- **簡素化**: 束を 2 つに分けない（`SwitchRig` へ渡す値が 1 つで済む）。C3 の `on_steady` は install → update を本体の中で続けて呼ぶ 1 欄、C2 は 10 件を 1 欄（1 つのひと続きの塊）。C4 は理由を分けて断るので 2 欄、C1 は入力の差し込みと交互なので 2 欄に留める。番人の crate に頼る足場を分け合うファイル（`reload_tests.rs`・`shell_balloon_switch_tests.rs`）は、足場を写さずファイルごと本体へ移す。

### 12.5 設計の討議へ回したこと

設計書の「設計の討議への議題（仮の決めつき）」の 8 件（束の数・口が無いときの記録の絞り・C5 の捨てる受け口・本体に残すテストの置き場と分け方・呼ぶ順の確かめの形・`SwitchRig` の作り口・口を据えずに通る本体のテスト・実機の 6 つの道の取り方）。
