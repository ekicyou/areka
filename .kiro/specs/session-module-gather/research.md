# ギャップ分析: session-module-gather

- 測った木: 作業の木 `claude/kiro-start-session-module-gather-1375a8`（HEAD `6ae0933e`＝main `f6405e09` に spec の初期化 1 コミット）。数は 2026-11-21 の測りで、設計の段で測り直す。
- 読んだもの: `requirements.md`・`brief.md`・`spec.json`（ja）・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C24 の行と「本体の crate 分け」の節）・本体 `crates/areka/src/` の該当ファイル・`crates/real-run-kit/src/launch.rs`・`log_history.rs` とそのテスト。
- 外の依存の調べは要らない（新しい依存は無い・移すだけ）。`bevy_ecs` 0.19.1 の系の並びの性質だけ「Research Needed」に置いた。

## 1. 分析の要約

- **既にある型**: 前例 `runtime-module-untangle` が `crates/areka/src/session/` を作っている（今の本番は `ghost_quit`・`ghost_switch`・`hooks`・`mod`・`reload`・`runtime_boot`・`same_folder_pin`・`shell_balloon_switch`・`switch_copy`）。テストは `#[cfg(test)] #[path = "<stem>_tests.rs"]` の兄弟で、ログの出所は `module_path!()` から付く。移す先の型は揃っている。
- **要件に載っていない所が 6 つある**: ⑴ `ghost_session.rs:852` が `#[path = "boot_shell_tests.rs"]` で宣言するテスト（441 行・5 本）は候補の名前に当たらないが、一緒に移さないとビルドが通らない ⑵ `.kiro/steering/logging.md:103` が `crates/areka/src/log_history.rs` を名指す ⑶ `crates/areka/Cargo.toml:44` の説明が `use areka_input as input_events;` を名指す ⑷ `crates/areka/tests/emo2_real_run.rs:58` が `default_helper_exe_path()` を `main.rs` にあると書く ⑸ `main.rs` という残るファイルの名で、移す項目を指す文書の行（`doc/COMPAT_ARCHITECTURE.md:189`・`:318`・`:319`）は、ファイル名の grep では拾えない ⑹ `register_systems` を呼ぶテストが 5 か所あり、うち `session/ghost_switch_test_support.rs:237` は 46 ファイルが使う切替の土台で、menu・install・update・mcp の登録（install と update は窓口の挿入と終了の門の登記も行う）に頼る。
- **気を付ける所が 3 つある**: ⑴ `get_log` の分け方は出所の名前で決まる。`ghost_session_runtime` の info は今、`#[path]` で `ghost_session` の下に入っていて、`areka::ghost_session::ghost_session_runtime` として status に当たる。入れ子を平らにすると status から落ちる。 ⑵ Input の段の系は、互いの並びの指定が無く、どれも `.after(dispatch_pointer_events)` だけなので、登録の順がそのまま走る順になりうる。 ⑶ ソースを読むテストのうち `readonly_tests.rs:92` の絞り込みは、移した後に黙って空になる（赤にならずに確かめる力を失う）。
- **案**: 要件どおりに `crate::session::…` へ直に書き換える案 A を推す。ほかに、根で出し直して書き換えを減らす案 B と、段を分ける案 C がある（6 章）。規模は M（9〜12 タスク）、リスクは中。
- **Research Needed**: bevy_ecs 0.19 で、並びの指定が無い系どうしが登録の順に走るのか。squash したときに、git が移動として追える類似度を保てるか（小さいファイルでパスの直しが多いもの）。

## 2. 今の姿

### 2.1 集める候補（2026-11-21・物理行）

| まとまり | ファイル（本番） | 物理行 | `#[path]` の子（テスト・入れ子） |
|---|---|---|---|
| ghost_session | `ghost_session.rs` | 881 | `ghost_session_scope_rect.rs`（279）・`ghost_session_scope_scale.rs`（115）・`ghost_session_runtime.rs`（182）を `#[path]` の入れ子で持つ。テスト 12 本を持つ（`ghost_session_{restart,strict,switch,switch_translate,reload}_tests`・**`boot_shell_tests.rs`**・`shell_balloon_switch_session{,_lap,_balloon,_abort,_update}_tests`）。孫に `ghost_session_switch_{fallback,memory}_tests`・`ghost_session_scope_{rect,scale}_boot_tests`・`ghost_session_runtime_tests` |
| 起動の判断 | `boot_resolve.rs` | 766 | `_tests`・`_option_tests`・`_balloon_tests`・`_mark_text_tests`・`_readonly_tests` |
| 構成の入力 | `boot_config.rs` | 648 | `_exe_link`・`_balloon`・`_roots`・`_copy`・`_option`・`_readonly`・`_callghost` の 7 本 |
| 終わり方 | `app_exit.rs`・`session_end.rs`・`exit_wait.rs`・`write_failure.rs` | 331・187・200・247 | `app_exit_{tests,escape_tests}`・`session_end_{tests,deadline_tests,sync_send_tests}`・`exit_wait_tests`・`write_failure_{tests,sites,probe,late}_tests` |
| 告知・読み取りだけ・履歴 | `alert.rs`・`readonly.rs`・`log_history.rs` | 520・59・570 | `alert_{tests,memory_write,dpi}_tests`・`readonly_{tests,sites,run}_tests`・`log_history_{tests,convention,capture}_tests` |
| roots の芯 | `roots/mod.rs`・`resolve.rs`・`report.rs`・`user_root.rs` | 54・290・158・206 | `resolve_tests`・`report_tests`・`user_root_tests`（`edit`・`desk`・`pick` とそのテスト 5 本は残る） |
| readme | `readme.rs`・`readme/{destination,opener,os_port,readme_cue}.rs` | 234＋960 | `readme_tests`・`destination_tests`・`opener_{tests,execute,submit}_tests`・`opener_test_support`・`os_port_tests`・`readme_cue_tests` |
| cli の型 | `cli/mod.rs` の `BootRequest`（54〜65 行）・`ModulePolicy`（67〜74 行） | 約 21 | — |

- src 直下の候補は 78 ファイル。`boot_shell_tests.rs` を足すと 79 になる。
- 1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の例外表にも、`temp_path_guard_test.rs` の表にも、集めるパスは 0 件。`log-capture-kit` に触らずに済む（要件 9.2 と合う）。

### 2.2 `main.rs` の項目（947 行・非空行 862）

| 範囲 | 項目 | 非空行 | 行き先（要件） |
|---|---|---|---|
| 1〜513 | 宣言・`fn main`・`finish_after_run`・`install_boot_context`・`first_boot_origin`・`boot_first_ghost` | 449 | 残る |
| 514〜526・555〜643 | `AfterRun`・`after_run` | 12＋88 | 残る（仮定・OPEN QUESTION 1） |
| 527〜554・644〜888 | `MarkInputs`・`Teardown`・`MarkVerdict`・`session_mark_verdict`・`settle_session_mark`・`restore_merged_placements`・`insert_persist_wiring`・`on_boot_ok`・`record_last_used_at_boot`・`record_last_used`・`boot_monitor_snapshot` | 26＋235 | `session/` |
| 889〜947 | テストの宣言 9 本 | 52 | 1 本ずつ決める（要件 2.4） |

- `after_run` を残すと、`main.rs` は非空行で約 600 になる（テストの宣言を出せばもっと減る）。brief の「約 470 行」は、`after_run` も出す数え方（527〜947 行）に近い。
- 本番の呼び手（`crate::` で引く所）: `ghost_session.rs:204`（`boot_monitor_snapshot`）・`:227`（`restore_merged_placements`）・`:720`（`on_boot_ok`）・`ghost_session_runtime.rs:55`（`on_boot_ok`）・`session/ghost_switch.rs:569`（`record_last_used`）・`session_end.rs:22`（`MarkInputs`・`Teardown`・`settle_session_mark`）。どれも集める側にあり、移した後は `session/` の中の参照になる。
- テストから引く所（残る項目を含む）: `main_session_mark_tests.rs:21-22` は `after_run`・`boot_first_ghost`・`first_boot_origin`・`install_boot_context`（残る）と、`settle_session_mark`・`session_mark_verdict`（移る）の両方を引く。`readonly_run_tests.rs:41`・`ghost_session_switch_memory_tests.rs:23`（`after_run`）、`readonly_sites_tests.rs:38`・`write_failure_sites_tests.rs:34`（`boot_first_ghost`）、`install/desk_overwrite_tests.rs:678-684`（`Teardown`・`session_mark_verdict`）。
- 出し直し `pub(crate) use boot_config::{…}`（`main.rs:115-118`）に頼る所は 11 ファイル: `boot_config.rs`（自分自身を `crate::` で引く）・`ghost_session.rs:24`・`ghost_session_runtime_tests.rs`・`main_session_mark_tests.rs`・`main_session_mark_write_tests.rs`・`session/ghost_switch_reload_tests.rs`・`session/ghost_switch_test_support.rs`・`session/runtime_boot.rs`・`session_end_tests.rs`・`shiori_demo.rs`・`shiori_host.rs`。ほかに `main_config_input_tests.rs`・`main_ghost_wiring_tests.rs`・`main_module_policy_tests.rs` が `use super::*` で通している。

### 2.3 `register_systems`（`ghost_session.rs:43-83`・25 の呼び出し）

| # | 呼び出し | 置き場 | 段 |
|---|---|---|---|
| 1〜5 | `session::hooks::install`・`app_exit::install_input_hooks`・`areka_runtime::register_frame_system`・`ghost_quit::wire_kanade_stop`・`write_failure::install` | 集める側・runtime | — |
| 6 | `readme::register_readme_drain` | readme（この spec で session へ） | Input `.after(dispatch_pointer_events)` |
| 7〜9 | `ghost_switch::register_change_drain`・`shell_balloon_switch::register_switch_drain`・`reload::register` | session | Input `.after(…)` |
| 10 | `areka_input::user_break::register_user_break_drain` | areka-input | Input `.before(…)` |
| 11 | `menu::register_menu_poll` | 本体だけ | Input `.after(…)`＋並びの指定の無い `poll_reset_raise` |
| 12〜16 | `balloon`・`shell_box_handler`・`choice_drain`・`char_watch`・`balloon_wheel` | areka-input | Input `.after(…)`／`.before(…)` |
| 17〜19 | `install::register`・`update::register`・`mcp::register` | 本体だけ | Input `.after(…)`。install と update は窓口（`InstallDesk`・`UpdateDesk`）を挿し、終了の門を `exit_wait::register_gate` で「install」「update」の順に登記する |
| 20〜25 | `FrameFinalize` への足し込み 2 つ・`wire_zorder_pair`・`ghost_session_scope_rect::register`・`balloon_syncscale::register`・`register_balloon_rejudge` | 集める側・placement | — |

- 本体だけの系 11 本（6・10〜19）の並びは「6｜7〜9 は session｜10〜19」で、間に session の 3 本が挟まる。10〜19 の 10 本は続いている。
- `menu`（11）は、入力（10 と 12〜16）の間にある。本体にしか無い 4 本（11・17〜19）だけを出すと、続いた塊にならない。
- 呼び手: 本番は `main.rs:242` の 1 か所だけ。テストは `app_exit_escape_tests.rs:137`（固定のテスト `register_systems_installs_the_escape_hook`）・`ghost_session_reload_tests.rs:71`・`:179`・`ghost_session_restart_tests.rs:455`・`session/ghost_switch_test_support.rs:237`（`SwitchRig::new`・46 ファイルが使う）の 5 か所。`mcp/mcp_tests.rs:316` は「`register_systems` で汲む系も登録済み」を前提にしている。
- ほかの crate の説明が `ghost_session::register_systems` を名指す所: `areka-input` に 15 か所（`balloon.rs:10`・`:186`・`balloon_exit.rs:52`・`balloon_wheel.rs:29`・`balloon_wiring_tests.rs:288`・`:293`・`char_watch.rs:40`・`choice_drain.rs:153`・`:168`・`:201`・`shell_box_handler.rs:421`・`user_break.rs:9`・`:205`・`:224`・`user_break_tests.rs:713`。ほかに `hooks.rs:3` が `register_systems` を名指す）、`areka-placement` に 8 か所、`areka-runtime` に 2 か所。この 3 つの crate は同じウェーブの約束で触れない（要件 9.1・9.2）。

### 2.4 ログの出所と `get_log` の分け方

- 分け方の表 `log_history.rs:115-132` の `RULES` は上から当て、`exact=false` は「その出所か、`::` で区切った下」に当たる。集める側に関わる行: `areka`（完全一致・status）・`areka::boot_config`・`areka::boot_resolve`・`areka::ghost_session`・`areka::session::ghost_switch`・`areka::readme`（どれも下も含む・status）。`UNKEPT_TARGETS` は `areka::install::install_cue` だけ。
- `get_log` の本文に出所の名前は入らない（`draft`・`body_text`・`log_history.rs:187-238`）。出所が効くのは種別の判定だけ。
- 表の守り: `log_history_convention_tests.rs:366` `every_rule_target_names_a_module_or_literal_that_exists` が、表の出所を `src/` の下のファイルの道に写して、実在を確かめる（`areka::session::boot_config` なら `src/session/boot_config.rs` を探す）。同じファイルの 300 行あたりで、文書 `doc/ssp-mcp/log-convention.md` の表と `RULES` の一致も確かめている。表と文書とファイルの置き場は、同じコミットでそろえる必要がある。
- 直書きの出所: 本番の `target:` に、集める側の名前を写したものは 0 件（`perf_thread_report.rs:51` の `"areka::perf"`、`log_history.rs:84` の `"areka::log::error"` は語彙）。brief の「`"areka::boot_config"` 7」ほかは、分け方の表とテストと道具の文字列の数。2026-11-21 の測りで、`crates/`・`tools/`・`doc/`・steering の行数は `areka::boot_config` 13・`areka::app_exit` 9・`areka::ghost_session` 7・`areka::boot_resolve` 6・`areka::alert` 5・`areka::readme` 4（`areka::readme::opener` 1）。そのほかの名前は 0（`areka::roots` は `structure.md:419` の Rust の道 1 か所）。
- info の行の数（今の出所で当たっている種別）: `ghost_session_runtime.rs` 1（`areka::ghost_session::ghost_session_runtime`＝status）・`app_exit.rs` 3・`session_end.rs` 3・`alert.rs` 2・`exit_wait.rs` 1・`write_failure.rs` 1・`readonly.rs` 1・`roots/report.rs` 3・`roots/user_root.rs` 3（ここまでは残さない）。すでに `session/` にある `reload.rs` 6・`shell_balloon_switch.rs` 4・`ghost_quit.rs` 2 ほかは、`ghost_switch` を除いて残さない。

## 3. 要件と資産の対応

| 要件 | 使える資産 | ギャップ（Missing／Unknown／Constraint） |
|---|---|---|
| 1 ファイルの移動 | `session/mod.rs` の宣言の形・`#[path]` の兄弟の型・前例の `git mv` | **Missing**: `boot_shell_tests.rs`（`ghost_session.rs:852`）が候補の一覧に無い。**Constraint**: `ghost_session.rs` は 881 行で、`main.rs` の項目（非空 261 行）を足すと 1,000 行を越える。新しいファイルが要る。**Unknown**: squash したときの移動の検出（7 章） |
| 2 `main.rs` の項目 | 項目は `main.rs` の中でほかに使われない（2.2） | **Constraint**: `main_session_mark_tests.rs`（947 行・12 本）は残る項目と移る項目を両方確かめる。1 本ずつの仕分けか、移した先から `crate::after_run` ほかを引くか。**Unknown**: `after_run` の置き場（OPEN QUESTION 1） |
| 3 roots・readme・cli | `roots/mod.rs` は `desk::install` と `edit::*` を出し直す（`roots/mod.rs:13-15`）。`edit.rs:15-16` は `super::{report, ResolvedRoots, RootStatus, RootsOrigin, encode_memory}`、`:170` ほかは `super::user_root` | **Missing**: 残る `edit`・`desk`・`pick` のための `src/roots/mod.rs` が新しく要る（今の `mod.rs` は芯と一緒に移る）。**Constraint**: `edit.rs` の `super::` を `crate::session::roots::…` へ書き換える。名前 `crate::roots` と `crate::session::roots` が並ぶ。cli の型の置き場が決まっていない（新しいファイルか `boot_config.rs` か） |
| 4 系の登録 | 2.3 の表 | **Missing**: テストの 5 か所が使える「本番と同じ組み立て」の入口。**Constraint**: 本体だけの 11 本は続いていない（6｜7〜9｜10〜19）。Input の段の系は互いの並びの指定を持たない。**Unknown**: 登録の順で走る順が決まるか（Research Needed） |
| 5 入力の出し直し | 呼び手 26 ファイル（brief の一覧と一致） | **Missing**: `crates/areka/Cargo.toml:44` の説明が出し直しを名指す。モジュールの道でない語 `input_events` が説明に 3 か所（`app_exit_tests.rs:182`・`ghost_session.rs:276`・`menu/mod.rs:182`）。5.3 の grep には当たらないが、古い説明になる |
| 6 パスの書き換え | 本体の 186 ファイル（2026-11-21 の測りで一致・内訳は 4 章） | **Missing**: `logging.md:103`・`tests/emo2_real_run.rs:58`・`main.rs` の名で移す項目を指す文書の行。**Constraint**: `areka-runtime`（7 ファイル）・`areka-placement`（2 ファイル）が、`ghost_session.rs`・`boot_config.rs`・`readme_cue.rs` を素のファイル名で書く。どちらも触れない crate |
| 7 ログの出所 | 表の守りのテスト（2.4） | **Constraint**: `ghost_session_runtime`・`_scope_rect`・`_scope_scale` は `#[path]` の入れ子のまま移す（平らにすると status から落ちる）。`areka::session` を下まで含む 1 行に畳まない（`reload` ほかの info が status に入る） |
| 8 外への参照の一覧 | 9 章の下書き | 本番は 32 行（登録の 4 行を `main.rs` へ出せば 28 行）、テストだけのものは別に |
| 9 触らない所 | roadmap の約束 ⑵ | **Constraint**: 5 章の 7 の素のファイル名と、`areka-input` の 15 か所の説明は直せない |
| 10 検証 | 前例の比べ方（テストの名前の末尾の多重集合）・`tools/test-all.ps1` | **Constraint**: `readonly_tests.rs:92` の絞り込みは、移した後に黙って空になる（5 章の 10） |

## 4. 測るよう頼まれた 4 点

### (a) `after_run`・`AfterRun` と info の行

- `main.rs:566-571` の `info!(event = "session_end_already_handled")` は出所が本体の根 `areka` で、`RULES` の `rule("areka", Kind::Status, true)`（`log_history.rs:123`・完全一致）で status に当たる。
- `after_run` を `session/` の下へ移すと、出所は `areka::session::<ファイル>` になる。どの決まりにも当たらなくなり、この行は `get_log` から消える（種別が status から「残さない」に変わる）。要件 7.4・7.5 に反する。
- 同じ `main.rs` に残る info は `fn main` の `shiori_skipped`（186 行）と `ghost_slot_empty`（356 行）の 2 つ。どちらも残るので影響しない。
- 移す項目の中の info は 0 件（要件 7.5 の測りと一致）。`settle_session_mark` と `record_last_used_at_boot` は debug だけ、`record_last_used` は error だけ（error は出所によらず error の種別）、`boot_monitor_snapshot` は `areka_placement::diag` の出所。`boot_resolve::keep_session_mark` が出す info は、もともと `areka::boot_resolve` の出所。
- テストは種別でなく水準だけを見ている（`session_end_tests.rs:373`・`:407` の `levels_of(…, "session_end_already_handled")`）。出所が変わっても赤にならず、種別が変わったことに気付けない。
- 移すときの手当ての案は 3 つ。⑴ 残す。⑵ 移して `target: "areka"` を直書きする（語彙の直書きが 1 つ増える）。⑶ 移して `RULES` に新しいファイルの完全一致の行を足し、文書の表も直す。行き先のファイルにほかの info があると一緒に当たる。

### (b) 系の登録の範囲と順

- 2.3 のとおり、本体だけの 11 本は「6（readme）」「10〜19」の 2 つに割れ、間に session の 7〜9 が挟まる。
- 案ごとの、順を保つための切れ目の数:

| 出す範囲 | 本数 | `session/` に要る登録の関数 | 説明が古くなる所（触れない crate） |
|---|---|---|---|
| brief の字面（6・10〜19） | 11 | 3（1〜5／7〜9／20〜25） | `areka-input` 15 か所 |
| readme を残す（10〜19） | 10 | 2（1〜9／20〜25） | `areka-input` 15 か所 |
| 本体にしか無いものだけ（11・17〜19） | 4 | 3（1〜10／12〜16／20〜25） | 0 |

- `readme` はこの spec で `session/` に入る。入力は `areka-input`（後の `areka-session` から引ける向き）にある。どちらも「集めた層が本体だけのモジュールを名指さない」（要件 4 の目的）には関わらない。
- 本当に `session/` の外へ出す必要があるのは menu・install・update・mcp の 4 本で、外への参照が 4 行減る（9 章）。10 本案でも 11 本案でも、減る外への参照は同じ 4 行（入力は外の crate なので数えない）。
- Input の段の系は、どれも `dispatch_pointer_events` に対する前か後ろの指定しか持たず、互いの並びは指定していない（`readme.rs:99`・`menu/mod.rs:258`・`areka-input/src/*.rs` の `register_*`・`session/{ghost_switch,shell_balloon_switch,reload}.rs`・`install/mod.rs:139`・`update/mod.rs:230`・`mcp/mod.rs:52`）。登録の順を変えると、走る順が変わりうる。
- install と update の登録は、終了の門の登記（`exit_wait::register_gate`）の順も持つ。

### (c) 名指すファイルの数と、外への参照の数

- 本体で集めるモジュールを `crate::…` で名指すファイル: **186**（2026-11-21・brief と一致）。対象は `crate::{ghost_session,boot_resolve,boot_config,app_exit,session_end,exit_wait,write_failure,readonly,alert,log_history,roots,readme}`・`main.rs` から移す項目と出し直しの名前・`crate::cli::{BootRequest,ModulePolicy}`、および `use crate::{…}` の束。
  - 内訳: src 直下 52・`session/` 28・`runtime_tests/` 20・`menu/` 19・`install/` 19・`mcp/` 18・`update/` 11・`roots/` 8・`readme/` 5・`offline_dump/` 4・`cli/` 2。
  - `runtime_tests/` の brief の 26 は、`include_str!` で読むもの（`zorder_wiring_tests.rs`）と、説明の中の名指しを含む数え方。
  - ほかに `main.rs` 自身（`crate::` を付けずに書く）と、`super::` で引く `main_*_tests.rs` 10 本がある。
- `session/` の外への参照（集め終えた後・`crate::` で引くもの・説明の行を除く）: **本番 32 行**（9 章）。登録の 4 行を `main.rs` へ出せば 28 行で、brief の「約 30 件」と合う。テストだけのものは 9 章の別の表に置いた。

### (d) roadmap の誤記

- `crates/areka-kanade/src/script_log.rs` は無い。実在は `crates/areka-kanade/src/schedule/script_log.rs`。その 16 行が `crates/areka/src/log_history.rs` の `TARGET_SCRIPT` を名指す（`/// 取り決めの target（…）`）。誤記は roadmap の C24 の行（約束 ⑵ の名指すコメントの 2 か所目）だけで、brief の再測定と要件 6.3 は正しく書いている。`areka-kanade` は同じウェーブのほかの spec の触るファイルに無い。

## 5. 要件に載っていない所・気を付ける所

1. **`boot_shell_tests.rs`**: `ghost_session.rs:852` の `#[path = "boot_shell_tests.rs"]` で宣言されている（441 行・`#[test]` 5 本・`crate_source_test_support` を使う）。候補の名前（`ghost_session*` ほか）には当たらない。`ghost_session.rs` と一緒に移すか、`#[path = "../boot_shell_tests.rs"]` にするか（後者は要件 1.6 の兄弟の置き方に反する）。
2. **`.kiro/steering/logging.md:103`**: `log_history::init()`（`crates/areka/src/log_history.rs`）と書く。要件 6.6 と 6.8 の grep の範囲（`structure.md`・`tech.md`）に入っていない。
3. **`crates/areka/Cargo.toml:44`**: 説明に「本体は `use areka_input as input_events;` で今の呼び手のパスを通す」とある。出し直しを外すと誤りになる。
4. **`crates/areka/tests/emo2_real_run.rs:58`**: `default_helper_exe_path()`（`crates/areka/src/main.rs`）と書く。実体は今も `boot_config.rs` で、出し直しを外すと、はっきり誤りになる。
5. **残るファイル `main.rs` の名で、移す項目を指す行**: `doc/COMPAT_ARCHITECTURE.md:189`（`main.rs::restore_merged_placements`）・`:318`（`main.rs` の `session_mark_verdict`）・`:319`（`main.rs` の `session_mark_verdict` が 3 か所。同じ行の `after_run` は残る）。ファイル名の grep（要件 6.8）は、残るファイルの名を当たりにしない。項目の名で引いて 1 件ずつ見る必要がある。
   - 同じ形で、項目が `main.rs` に残る行（`COMPAT_ARCHITECTURE.md:301` の `finish_after_run`、`:326` の `fn main`、`ledger/shiori.toml:1649` の `first_boot_origin`）はそのままでよい。
   - もとから古い行（`doc/click-through.md:3`・`:136`。窓の登録は今 `register_systems` の `FrameFinalize` にある。`tools/perf/invoke-perf-run.ps1:29` の `SMOKE_EXIT_ENV` は `smoke_exit.rs` にある）は、この spec の移動とは関係なく古い。直さずに申し送る候補（要件 10.1）。
6. **登録の入口とテスト**: 2.3 の 5 か所は、本番と同じ全部の登録を前提にしている。とくに `SwitchRig::new` は 46 ファイルが使い、`update::desk::is_busy`（`session/reload.rs:309`・`shell_balloon_switch.rs:131`）ほか窓口の有無で分かれる道がある。本体だけの登録を `fn main` の中に書き込むと、テストの World からそれが消える。
7. **素のファイル名**（`crates/areka/src/` を付けない書き方）:
   - 触れない crate: `areka-runtime/src/lib.rs:345`・`strict_judges.rs:6`（`boot_config.rs`）・`frame_persist_test_support.rs:247`（`ghost_session.rs`）・`change_cue.rs:4`・`user_break_cue.rs:31`・`consumer_ledger_test_support.rs:31`・`:33`（`readme_cue.rs`）、`areka-placement/src/config.rs:106`・`spawn_zorder_pair_deferred_tests.rs:28`（`ghost_session.rs`）。
   - 触れる crate: `areka-ghost/src/translate_wiring_e2e_tests.rs:6`（`ghost_session_switch_translate_tests.rs`）・`wintf/src/ecs/window/zorder_pair_deferred_vocabulary_tests.rs:27`（`ghost_session.rs`）。
   - 要件 6.8 の「`src/` を付けない書き方を含む」が素のファイル名まで含むなら、触れない crate の 9 か所は直せない。
8. **日付つきの写しの線**: `doc/roadmap-overview-2026-10-16.html` に 1 か所ある。`doc/ideas/novel-redesign-2026-11-20/` の 5 ファイル（6 か所）は、名前に日付を持つが、要件 6.5 と申し送り 3 は書き換える側に数えている。6.8 の「日付つきの写し」の線を、設計の段で文字にしておく必要がある。
9. **`get_log` を確かめるテストの穴**: 表のテストは文字列で出所を渡して確かめる（`log_history_tests.rs:103-111`・`:168-174`）。表の守り（2.4）は、ファイルが在ることまでしか見ない。ファイルを `#[path]` の入れ子から平らに置き直したり、名前を変えたりして、`module_path!()` が表とずれても気付けない。要件 10.8 を確かめる証跡として、移した後の本番の出所（`module_path!()` の実の値）で種別を引く較正があるとよい。足すかどうかは要件 10.3 の「新しく足したテスト」の扱いになる。
10. **ソースを読むテスト**（要件 6.2 の具体）:
    - `runtime_tests/zorder_wiring_tests.rs:68`・`:71`: `include_str!("../ghost_session.rs")`・`("../main.rs")`。`main.rs` の字面 `let opened = ghost_session::open_ghost_windows(app.world()…` と `.map_err(write_failure::stop_boot)?;` を探す。`main.rs` の呼び出しを `session::ghost_session::…` と書くと赤になる。
    - `readonly_tests.rs:82`: 較正に `main.rs` の `mod readonly;` を使う。`:92` の絞り込み `rel == "readonly.rs" || rel.starts_with("readonly_")` は、移した後に黙って空になる。`:153` の `rel == "readonly.rs"` は赤になる。
    - `session_end_deadline_tests.rs:405`・`:409`・`:413`・`:519`・`:533`: 表の `"areka/ghost_session.rs"`・`"areka/session_end.rs"`。
    - `readme_tests.rs:424`: `"areka/readme/os_port.rs"`。
    - `session_end_sync_send_tests.rs:437`・`:445` の `"crates/areka/src/main.rs"` は較正の写しで、`main.rs` は残るので直さなくてよい。
11. **EnvFilter の前方一致**（roadmap の共通の約束に書かれた性質）: `RUST_LOG=areka::roots=debug` は今、芯と `edit`・`desk`・`pick` の両方を開く。分けた後は 2 つの指定が要る。今の文書とテストに、この指定は無い（測り）。`areka::session=debug` は、集めた後は今よりずっと多くの出所を開く。どちらも既定の動きには関わらない。
12. **`areka-input` の説明 15 か所**: 入力の登録を `main.rs` へ出すと、「呼び手は `ghost_session::register_systems`」が誤りになる。直すのは同じウェーブの `audit-areka-input-menu` の範囲（触れない crate）。

## 6. 実装の案

### 案 A: 要件どおりに直に書き換える（`crate::session::…`）

- `git mv` で `session/` の下へ移し、`session/mod.rs` に宣言を足す。`#[path]` の入れ子（`ghost_session` の下の 3 本・テストの子）は、そのままの形で移す。
- `main.rs` の項目は、新しいファイル（例: 起動中の印の判定のまとまりと、起動の後の結線のまとまりの 1〜2 本）へ移す。
- 呼び手 186 ファイルと、出し直しに頼る 11＋3 ファイルを、`crate::session::<名>::…` へ書き換える。`main.rs` は `use crate::session::{ghost_session, write_failure, readonly, …};` で今の呼び出しの字面を保てる（`zorder_wiring_tests.rs` の字面がそのまま通る）。
- 良い所: 要件 2.3・5・6.1 と同じ考え（実在しない名前を残さない）で、grep の 0 件が素直に示せる。前例 `runtime-module-untangle` と同じ。
- 悪い所: 差分が一番大きい（186 ファイル）。`session-crate-split` で、`session/` の中の `crate::session::X` が `crate::X` に、外の `crate::session::X` が `areka_session::X` に、もう一度書き換わる。

### 案 B: 根で出し直す（`main.rs` に `pub(crate) use session::{alert, app_exit, …};`）

- `crate::alert` ほかの道がそのまま通る。書き換えは、ソースを読むテスト・文書・ログの出所だけになる。
- `session/` の中の `crate::alert` は、`session-crate-split` で `session/` が crate の根になると、そのまま正しい道になる。2 本を合わせた書き換えは一番少ない。
- 悪い所: 要件 5（出し直しを外す）・2.3（`boot_config` の出し直しを外す）と逆向きの出し直しを 12 個増やす。要件 6.1（パスを移した先へ書き換える）を満たさない。実在しないモジュールの名前が本体に残る。要件の改訂が要る。

### 案 C: 段を分ける（入力と出し直しの書き換え → 移動 → パス）

- 段 1 で、入力の出し直しを外して 26 ファイルを `areka_input::…` へ直す（この段は移動と関係なく、今の場所で済む）。段 2 で `git mv` と宣言と `crate::` の書き換え。段 3 で文書・台帳・ログの出所・道具。各段で `cargo check` を緑に保つ。
- 案 A の作業の順の取り方で、A と組み合わせて使う。タスクの切り方（要件 1〜7 の境界）とも合う。
- 良い所: 1 段ごとに確かめられる。移動の段の差分が小さく、git が移動として追いやすい。
- 悪い所: 段 2 と段 3 の間は、文書が古いまま（PR は 1 本で squash するので main には出ない）。

推す組み合わせ（議題の候補 10 と 11）: 案 A の書き方を、案 C の順で進める。

## 7. 規模とリスク

- **規模: M**（9〜12 タスク・brief の見込みと同じ）。中身は移すだけだが、触るファイルが本体の約 190 本と文書・道具・ほかの crate の説明にまたがり、テストの登録の入口と `main.rs` のテストの仕分けに設計が要る。
- **リスク: 中**。形は前例どおりで、技術的に未知なものは無い。ただし、黙って変わる所が 3 つある（`get_log` の種別・Input の段の走る順・ソースを読むテストが空になること）。どれも全体テストの緑では見えない。
- **Research Needed**:
  1. bevy_ecs 0.19.1 で、並びの指定が無い系どうしが登録の順で走るか（`Schedule` の並べ方の決まり）。要件 4.2 が順を保つので、結論で作業は変わらない。順を保たない案を採るときだけ要る。
  2. squash したときの移動の検出: `git diff -M --stat main...HEAD` で、移したファイルが全部 `R`（移動）になっているかを PR の前に確かめる手順。小さいファイル（`readonly.rs` 59 行・`roots/mod.rs` 54 行ほか）で、パスの直しの割合が大きいもの。
  3. `session/` の外への参照の一覧（9 章）は、移した後の木で測り直す（設計の段か完了の前）。

## 8. 設計の段への申し送り

- 要件の「数の取り直し」に足すもの: `boot_shell_tests.rs`・素のファイル名 11 か所・`main.rs` の名で移す項目を指す行・`register_systems` のテストの呼び手 5 か所・`areka-input` の説明 15 か所。
- 9 章の一覧は下書きで、要件 8 の記録は移した後に取り直す。
- 同じウェーブの兄弟と重ならないかの確かめ: 5 章で新しく見つけた `logging.md`・`crates/areka/Cargo.toml`・`tests/emo2_real_run.rs`・`areka-ghost`・`wintf` の各 1 か所は、C24 のほかの 3 本の触るファイルに無い（roadmap の C24 の行の ①③④）。

## 9. `session/` の外への参照の一覧（下書き・集め終えた後の形で数えた）

### 9.1 本番

| ファイル:行（今の場所） | 参照先 | 項目 | 登録を `main.rs` へ出したとき |
|---|---|---|---|
| `alert.rs:286` | `smoke_exit` | `smoke_exit_ms` | 残る |
| `ghost_session.rs:21`・`:282` | `menu` | `use crate::menu`・`attach_release_handlers` | 残る |
| `ghost_session.rs:57` | `menu` | `register_menu_poll` | 消える |
| `ghost_session.rs:63`・`:64`・`:65` | `install`・`update`・`mcp` | `register` | 消える |
| `ghost_session.rs:288` | `file_drop` | `attach_file_drop_receivers` | 残る |
| `ghost_session.rs:291` | `menu::shortcut` | `attach_shortcut_receivers` | 残る |
| `ghost_session_runtime.rs:19`・`:35`・`:37`・`:39`・`:40`・`:42`・`:44`・`:46`・`:48`・`:50` | `menu` | `use`・`wire_menu`・`ghost_frame`／`shell_frame`／`balloon_frame`／`install_frame`／`roots_frame`／`update_frame`／`reset_pos_frame`／`reset_balloon_pos_frame` の `register` | 残る |
| `ghost_session_runtime.rs:52` | `install::names` | `reseed` | 残る |
| `session/ghost_switch.rs:502`・`:523` | `update::desk` | `on_switch_end` | 残る |
| `session/ghost_switch.rs:511`・`:513` | `install::desk`・`update::desk` | `on_steady` | 残る |
| `session/ghost_switch.rs:673` | `install::desk` | `run_overwrite_between` | 残る |
| `session/reload.rs:309`・`:312` | `update::desk`・`install::desk` | `is_busy` | 残る |
| `session/runtime_boot.rs:24`・`:26`・`:89`・`:90` | `install::install_cue`・`update::update_cue`・`install::desk`・`update::desk` | `InstallCueSink`・`UpdateCueSink`・`raw_sender` | 残る |
| `session/shell_balloon_switch.rs:131` | `update::desk` | `is_busy` | 残る |

- 計 32 行。登録を出すと 28 行。参照先の内訳: menu 13・install 7・update 8・mcp 1・file_drop 1・smoke_exit 1・menu::shortcut 1・cli 0（cli の型 2 つは移るので中の参照になる）。
- 入口 `install::startup::resolve_boot`（`main.rs:205`）は本体の側から `session/` を引く向きなので、この一覧に入らない。

### 9.2 テストだけ

- `boot_config.rs:209`（`#[cfg(test)]` の `resolve_boot_from` が `cli::parse::parse` を引く）。
- テストのファイル（2026-11-21・ファイル 参照先 件数）:
  - `boot_config_callghost_tests` cli 1
  - `boot_resolve_mark_text_tests` install 1
  - `ghost_session_reload_tests` cli 2・runtime_tests 1・sample_test_support 1
  - `ghost_session_restart_tests` cli 1・menu 2・runtime_tests 1・sample_test_support 1
  - `ghost_session_scope_rect_boot_tests` mcp 2
  - `ghost_session_scope_scale_boot_tests` mcp 2・shiori_host 1
  - `ghost_session_strict_tests` cli 3・menu 1
  - `ghost_session_switch_tests` menu 1
  - `log_history_convention_tests` crate_source_test_support 2
  - `readme_tests` crate_source_test_support 2
  - `readonly_run_tests` install 3・update 3
  - `readonly_sites_tests` install 1
  - `session/ghost_switch_test_support` cli 1・sample_test_support 1
  - `session/reload_tests` install 1・update 1
  - `session/runtime_boot_tests` sample_test_support 1
  - `session/shell_balloon_switch_tests` update 1
  - `session_end_deadline_tests` crate_source_test_support 2
  - `shell_balloon_switch_session_tests` menu 7
  - `shell_balloon_switch_session_update_tests` update 2
  - `write_failure_sites_tests` install 1
- `cli` の件数は、`BootRequest`・`ModulePolicy` を移すと減る（`cli::parse` を引くものだけが残る）。
- 根の項目を引くテスト: `after_run`（`ghost_session_switch_memory_tests`・`readonly_run_tests`・`main_session_mark*_tests` を移すなら、その分）・`boot_first_ghost`（`readonly_sites_tests`・`write_failure_sites_tests`）・`first_boot_origin`・`install_boot_context`（`main_session_mark_tests`）。
- 登録の入口を `main.rs` に置くなら、2.3 のテストの呼び手 5 か所も、ここに入る。

## 10. 議題の候補（要件の討議へ）

1. **`after_run`・`AfterRun` の置き場**（OPEN QUESTION 1）: 証跡は `main.rs:514-643`・`:566-571`（出所 `areka`）・`log_history.rs:123`（完全一致で status）・`session_end_tests.rs:373`・`:407`（水準しか見ない）。推す案は「`main.rs` に残す」（要件の仮定どおり）。分け方の表に触れずに済み、`main.rs` は非空行で約 600。brief の約 470 には届かないことを、要件 2.2 の数に書き添える。
2. **`main.rs` へ出す登録の範囲**（OPEN QUESTION 2）: 証跡は `ghost_session.rs:43-83`（2.3 の表）・Input の段の系の並びの指定（4 章 (b)）・`areka-input` の説明 15 か所。推す案は「`readme` を残し、続いた 10 本（10〜19）を出す」。`session/` の登録の関数は前と後ろの 2 つで済み、全体の順はそのまま保てる。`areka-input` の説明が古くなる分は `audit-areka-input-menu` へ相談のうえ申し送る。次点は「本体にしか無い 4 本だけ」（説明は古くならないが、切れ目が 3 つ）。
3. **登録の入口とテスト**: 証跡は `app_exit_escape_tests.rs:137`・`ghost_session_reload_tests.rs:71`・`:179`・`ghost_session_restart_tests.rs:455`・`session/ghost_switch_test_support.rs:237`（46 ファイル）・`mcp/mcp_tests.rs:316`・`install/mod.rs:134-141`・`update/mod.rs:225-232`。推す案は「`main.rs` に全部の登録を順に呼ぶ関数を 1 つ置き、`fn main` とテストの 5 か所がそれを呼ぶ」。テストの側から根を引く参照は 9.2 に載せ、`session-crate-split` へ渡す。テストの土台に順を写す案は、順がずれるおそれがあるので推さない。
4. **`main.rs` から移す項目の置き場**: 証跡は `ghost_session.rs` 881 行（足すと 1,000 行を越える）・`boot_resolve.rs` 766 行・移す項目の非空 261 行。推す案は「`session/` に新しいファイルを置く」。起動中の印の判定（`MarkInputs`〜`settle_session_mark`）と起動の後の結線（`restore_merged_placements`〜`boot_monitor_snapshot`）の 1〜2 本にし、テストはその兄弟へ名前を改めて `git mv` する。既にあるファイルへ足すと、要件 1.2（移したファイルの中身を書き換えない）に反する。
5. **`main_*_tests.rs` の仕分け**（要件 2.4）: 証跡は 2.2 の表と `main_session_mark_tests.rs:20-25`（残る項目と移る項目の両方）。推す案は次のとおり。
   - `main_config_input_tests`・`main_ghost_wiring_tests`（`boot_config.rs:8` が自分の檻と書く）・`main_module_policy_tests`・`main_restore_seam_tests`・`main_persist_wiring_seam_tests`・`main_monitor_snapshot_seam_tests` は、確かめる相手と一緒に移す。
   - `main_finish_after_run_tests` は残す。
   - `main_session_mark{,_fallback,_write}_tests` は、`after_run` と `boot_first_ghost` を引くので、議題 1 の結論に従う。残すなら `main.rs` に置いたまま、移った項目を `crate::session::…` で引く。
6. **cli の型 2 つの置き場と引き方**: 証跡は `cli/mod.rs:54-74`・使う所（`install/startup_tests.rs` 20・`cli/parse_tests.rs` 7・`main_module_policy_tests.rs` 8 ほか）。推す案は「`session/` に新しいファイル（例 `boot_request.rs`）を置き、cli は出し直さずに `crate::session::…` で引く」（議題 7 と同じ考え）。書き換えを減らしたいなら、`cli/mod.rs` で `pub(crate) use` する次点の案もある。
7. **`boot_config` の出し直し**（OPEN QUESTION 4）: 証跡は `main.rs:115-118`・頼る 11＋3 ファイル（2.2）。推す案は「外す」（要件の仮定どおり）。
8. **roots の分け方**: 証跡は `roots/mod.rs:6-15`（`desk::install`・`edit::*` の出し直し）・`edit.rs:15-18`・`:170`・`:203`・`:231`（`super::`）・`main.rs:244`（`roots::install`）。推す案は「`src/roots/mod.rs` を新しく作って `edit`・`desk`・`pick` だけを宣言し、今の出し直し 2 つを残す。`edit.rs` の `super::` は `crate::session::roots::…` に直す」。名前が `crate::roots` と `crate::session::roots` で並ぶのは、議題 9 で受ける。
9. **モジュールの名前と入れ子**（OPEN QUESTION 5）: 証跡は `ghost_session.rs:26-32`（`#[path]` の入れ子）・`ghost_session_runtime.rs` の info 1 件（今は status）・`log_history.rs:115-132`。推す案は「名前も入れ子も今のまま。分け方の表は 1 行ずつ `areka::session::<名>` に直し、`areka::session` を下まで含む 1 行には畳まない」。
10. **書き換えの書き方**（6 章の案 A・B）: 推す案は案 A（直に書く）。`main.rs` だけは `use crate::session::{…}` でモジュールを引き、呼び出しの字面を保つ（`zorder_wiring_tests.rs` の字面がそのまま通る）。案 B（根で出し直す）は書き換えが一番少ないが、要件 5・6.1 と逆向きなので推さない。
11. **作業の順**（6 章の案 C）: 推す案は、入力の出し直しを外す → 移動と `crate::` の書き換え → ログの出所と分け方の表と文書を同じコミットで → 文書・台帳・道具、の順。各段で `cargo check --workspace --all-targets` を緑に保ち、PR の前に `git diff -M --stat` で全部が移動として見えることを確かめる。
12. **grep の範囲と、要件に載っていない所**（5 章の 1〜8）: 推す案は次のとおり。
    - `boot_shell_tests.rs` を移す候補に足す。
    - `logging.md:103`・`crates/areka/Cargo.toml:44`・`tests/emo2_real_run.rs:58`・`COMPAT_ARCHITECTURE.md:189`・`:318`・`:319` を書き換える側に足す。
    - 要件 6.8 の grep は「ディレクトリを付けた古いパス（`crates/areka/src/<名>`・`areka/src/<名>`）」とする。素のファイル名は要件 6.9 のモジュールの名前と同じく、移した後も読めるものとして当たりにしない。触れる crate の 2 か所（`areka-ghost`・`wintf`）も、同じ理由で直さない。
    - 「日付つきの写し」は `doc/roadmap-overview-2026-10-16.html` の形（日付の付いた 1 枚の写し）とし、`doc/ideas/novel-redesign-2026-11-20/` は書き換える側。
    - もとから古い行（`doc/click-through.md:3`・`:136`・`tools/perf/invoke-perf-run.ps1:29`・説明の中の語 `input_events` 3 か所）は直さず、`audit-areka-app` へ申し送る。
13. **受け持ちの持ち主**（OPEN QUESTION 3）: 証跡は `audit-areka-install-update/brief.md:11`（`roots*`・`readme*`）・`audit-areka-app/brief.md:11`・`:49`・roadmap の棚卸㊶の裁定 8 の ⑴（C26 の 3 本の重なりを 0 に戻した）。推す案は「持ち主は変えず、パスだけ書き換える」（要件の仮定どおり）。持ち主を `session/` で 1 本に寄せると、C26 の 3 本の範囲の組み直しになる。
14. **`get_log` の分け方の証跡の強さ**（要件 10.8・5 章の 9）: 推す案は、`log_history` のテストに、移したモジュールの実の `module_path!()` で種別を引く較正を 1 本足すこと（足したテストは要件 10.3 の対応表で説明する）。足さない次点の案は、表の守り（ファイルが在るか）と文字列のテストだけで示すもの。
15. **ソースを読むテストの絞り込み**（5 章の 10）: 推す案は、`readonly_tests.rs:92` の絞り込みを移した先の道（`session/readonly.rs`・`session/readonly_`）へ直し、`:82` の較正を `session/mod.rs` の `mod readonly;` へ向けること。同じ事実を確かめる形を保つ（要件 6.2）。絞った結果が空でないことを確かめる行を足すかは、議題 14 と同じく、足したテストの扱いになる。

## 11. 要件の討議で設計の段へ回したもの（カテゴリ B）

10 章の候補のうち、how の判断で要件を変えないものは設計の段（`/kiro-design`）で決める。

1. **登録の入口とテスト**（10 章の 3）: `fn main` とテストの 5 か所（`app_exit_escape_tests.rs:137`・`ghost_session_reload_tests.rs:71`・`:179`・`ghost_session_restart_tests.rs:455`・`session/ghost_switch_test_support.rs:237`）が同じ順で全部の登録を呼べる入口を 1 つ置く形。テストの側から根を引く参照は 9.2 に載せる。
2. **`main.rs` から移す項目の置き場**（10 章の 4）: `session/` の新しいファイル 1〜2 本（起動中の印の判定・起動の後の結線）と、テストの兄弟への名前の改め。
3. **`main_*_tests.rs` の仕分け**（10 章の 5）: 1 本ずつの行き先（要件 2.4）。
4. **cli の型 2 つの置き場と引き方**（10 章の 6）。
5. **roots の分け方**（10 章の 8）: 残る `edit`・`desk`・`pick` のための新しい `src/roots/mod.rs` と、`edit.rs` の `super::` の書き換え。
6. **書き換えの書き方**（10 章の 10・案 A）と **作業の順**（10 章の 11・案 C）。各段で `cargo check --workspace --all-targets` を緑に保ち、PR の前に `git diff -M --stat` で全部が移動として見えることを確かめる。
7. **Research Needed**（7 章）: bevy_ecs 0.19.1 の並びの指定の無い系の走る順（順を保つので結論で作業は変わらない）・squash したときの移動の検出。

## 12. 設計の段（2026-11-21・`/kiro-design`）

- 測った木: 作業の木 `claude/kiro-start-session-module-gather-1375a8`（HEAD `cc912590`・main `f6405e09` に spec の文書のコミットだけ）。
- 発見の種類: **light**（既にある本体の中の移動・新しい依存なし・外の調べなし）。前例 `completed/runtime-module-untangle/design.md` の形にそろえた。
- 使った手引き: `kiro-spec-design` の `rules/design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`。領域の skill（Pasta・UI ほか）は当たらない。下請けの調べは出していない（ギャップ分析の測りが細かく、確かめ直しは数か所の grep で足りた）。

### 12.1 測り直したもの

- src 直下の候補は **62 ファイル**（本番 13・テスト 49・`boot_shell_tests.rs` を含む）。2 章の「78」は数え方が違う（`main_*` や roots を混ぜたとみられる）。設計の表（design.md の Supporting References）を正とし、実装の段の始めに `git ls-files` で数え直す。
- `main.rs` 947 行。移す項目は 527〜547 行（`MarkInputs`・`Teardown`）と 643〜884 行（`MarkVerdict`〜`boot_monitor_snapshot`）。テストの宣言は 9 本（890〜947 行）。`main_session_mark_fallback_tests.rs` は `main_session_mark_tests.rs` の子（842 行）で `main.rs` は宣言しない。
- `register_systems` は `ghost_session.rs:43-80`・25 の呼び出し（2.3 の表と一致）。呼び手は本番 `main.rs:242`・テスト 5 か所（一致）。
- `crate::input_events` の呼び手 26 ファイル（一致）。語 `input_events` は `Cargo.toml:44`・`ghost_session.rs:276`・`menu/mod.rs:182` と、もとから古い `app_exit_tests.rs:182`。
- `cli::BootRequest`・`cli::ModulePolicy` を引くのは 13 ファイル（`boot_config.rs` 5 か所ほか）。
- 外への参照（本番）: 2026-11-21 の測りの 32 行と一致（9.1 の表）。登録の 4 行を出すと **28 行**＝`menu` 12・`menu::shortcut` 1・`install` 6・`update` 7・`file_drop` 1・`smoke_exit` 1。
- 網羅台帳の古いパス: 32 行（43 か所）。`doc/COMPAT_ARCHITECTURE.md` 33 か所。`doc/ideas/novel-redesign-2026-11-20/` 5 ファイル 7 か所。`tools/package.ps1` 3・`tools/perf/judge-perf.py` 1。
- 移す `main.rs` の項目の中の `info!` は 0 件（`settle_session_mark`・`record_last_used_at_boot` は debug、`record_last_used` は error）。`classify` は debug と trace を残さず、warn 以上を出所によらず error にするので、出所が根 `areka` から変わっても `get_log` の分け方は変わらない（要件 7.5 は手当て不要）。
- `runtime_tests/mod.rs` は集めるモジュールをコードで引かない（説明の行だけ）。書き換えずに済む（要件 5.4・9.3）。
- 移すファイルの `#[path]` に `../` を使うものは 0 件。`include_str!` を持つ移すファイルも 0 件。

### 12.2 設計で分かったこと

- **`#[path]` の読み方**: `mod.rs` でないファイル（`ghost_session.rs`）に書いた `#[path]` は、そのファイルのあるフォルダから読む。フォルダごと移せば `#[path]` は書き換え不要。
- **`main.rs` の最上位の `use` は crate 全体の出し直しになる**: 根の私有の項目は crate のどこからも `crate::<名>` で引ける。今の `use areka_input as input_events;` はこの性質で働く。移した後に `main.rs` の最上位へ `use session::ghost_session;` を置くと、古い道 `crate::ghost_session::…` が黙って通る。`fn` の本文の中の `use` は外から見えないので、本文の字面（ソースを読むテストが読む）を保つのにはこちらを使う。
- **`roots/mod.rs` の移動の検出**: 残る 3 本のために同じパスへ新しい `mod.rs` を置くと、squash の後に git は「書き換え＋追加」と見て、芯の履歴が `session/roots/mod.rs` へ続かない。`src/roots.rs`（`readme.rs` と同じ形）にすれば、`roots/mod.rs` は移動（R）になる。
- **触れない crate の説明 25 か所**（`areka-input` 15・`areka-placement` 8・`areka-runtime` 2）が `ghost_session::register_systems` を名指す。入口を 3 つに分けて `main.rs` へ全体の入口を置くと、全部が古くなる。入口の名前と中身を残して本体だけの系を引数で受ければ、どれも古くならない。
- **ソースを読むテスト**: `readonly_tests.rs:82` の較正の針 `mod readonly;` は `main.rs` から消えるので、`mod session;` に替える（同じ事実＝走査が `main.rs` の本文を見ている）。`:92` の絞り込みには空でないことの `assert!` を足す。`zorder_wiring_tests.rs` は `fn main` の本文の字面を保てば、読むファイルのパスだけの直しで済む。

### 12.3 合成（design-synthesis）

- **一般化**: 要件 1・3・6・7 は「1 つのモジュールの塊を移し、その名前を読む所（呼び手・出所・ソースを読むテスト・文書）を追従させる」同じ操作の繰り返し。塊ごとに 1 段で済ませ、段ごとに `cargo check` を緑にする手順を 1 つ決めた（design.md の Migration Strategy）。
- **作るか使うか**: 道具は作らない（`git mv`・`git grep`・`cargo test -- --list`・`tools/test-all.ps1`・`git diff -M`）。較正のテストは標準の `std::any::type_name`／`type_name_of_val` を使い、移したファイルに定数を足さない。
- **簡素化**: 出し直しを 1 つも足さない（研究の案 B を採らない）。新しい型は `BodySystems`（関数のポインタ 2 つ）だけで、本体だけの系の名指しを `main.rs` へ出しながら入口の名前と順を残す唯一の形として要る。入口を 3 つに分ける案は、関数が 2 つ増え、触れない crate の説明 25 か所が古くなるので採らない。`main.rs` の項目の行き先は 2 本（印の判定・起動のシーム）で、呼び手（`session_end` と `ghost_session`）が違うので 1 本にまとめない。

### 12.4 Research Needed の閉じ方

1. bevy_ecs の並びの指定の無い系の走る順: 登録の呼び出しを 25 行とも同じ順に保つ設計にしたので、調べなくてよい（結論で作業が変わらない）。
2. squash したときの移動の検出: PR の前に `git diff -M --stat main...HEAD` で全部が `R` に見えることを確かめる手順を設計に入れた（design.md の VerificationRecord）。`roots/mod.rs` は `roots.rs` の形で `R` を保つ。
3. 外への参照の一覧: 下書きを design.md に置いた（本番 28 行）。集め終えた木で数え直して、この記録の新しい節に置く（要件 8.1）。

### 12.5 設計の討議へ持ち越したもの

design.md の「設計の討議への持ち越し（OPEN QUESTIONS）」の 7 件（登録の形・新しいファイルの名前・残る roots の宣言の置き場・`main.rs` の書き方・較正の取り方・`briefing-assets.md` の 2 か所・`roadmap.md` の対応表）。どれも仮定を置いて設計を書き切った。

## 13. 着手の取り直し（実装の段・タスク 1.1・2026-11-21）

- 着手の木: HEAD `8a1581cebf399cc6c93bbd1cddbc46d127fcce04`（作業の木 `claude/kiro-start-session-module-gather-1375a8`）。`git merge-base HEAD main` ＝ `main` ＝ `f6405e09f9cb94e469bb7e23c84162b0a0151b94`（main は動いていない・HEAD は spec の文書のコミットだけ先）。コードの変更は 0。

### 13.1 テストの名前の多重集合（比べる「前」・要件 10.2）

- 取り方: `cargo test --workspace --no-fail-fast -- --list`（ビルドと一覧のみ・机なし）の `: test$` の行 **14,437 行**。名前の末尾＝行から `: test` を落とし、最後の `::` から後ろだけを残した（`::` が無い名前はそのまま）。`LC_ALL=C sort` した全行（重複を残す＝多重集合）。同じ名前の別のテストは別の行として数える。
- 置き場（`.kiro/specs/session-module-gather/` の下に置くのは名前の一覧だけ）:
  - `baseline-test-names.txt` — 末尾の名前の多重集合（14,437 行・ソート済み・異なる名前 14,318）。完了のときに同じ式で作った一覧と `diff` で比べる（差は KindCalibration の 1 本だけのはず・10.3）。
  - 末尾を切る前の `: test$` の行（モジュールのパス付き・14,437 行）は重いのでコミットしない。差が出たときは、`cargo test --workspace --no-fail-fast -- --list | grep ': test$'`（13.1 の式から末尾を切る `sed` の段を除いたもの）で作り直して、どのモジュールから移ったかを引く。
- 作り直しの式（完了のとき・同じ式）: `cargo test --workspace --no-fail-fast -- --list | grep ': test$' | tr -d '\r' | sed -E 's/: test$//; s/^.*:://' | LC_ALL=C sort`。

### 13.2 警告（要件 10.4）

- `cargo check --workspace --all-targets`: 終了 0・`^warning` **0 件**（`pilot` を含め全 crate を `Checking` した出力）。
- 参考（比べるときに取り違えない）: `cargo test --workspace -- --list` のテストのプロファイルの組み立てでは、`pilot` の**マニフェストの警告 2 件**（`crates/pilot/Cargo.toml` の `event-listener`・`wintf-winmsg-executor` の `unused dependency`・`cargo::unused_dependencies`）が出る。`cargo check --workspace --all-targets` には出ず、この spec とも関係ない（もとから）。10.4 の対象は `cargo check --workspace --all-targets` の警告 0 件。

### 13.3 設計の数の数え直し（`git ls-files`・`git grep`・差と行き先）

| 項目 | 設計の数 | 今の木の数 | 差 | 行き先 |
|---|---|---|---|---|
| src 直下の移す候補（本番／テスト） | 62（13／49） | 62（13／49） | 0 | — |
| 　内訳 `ghost_session*`・`shell_balloon_switch_session_*`・`boot_shell_tests.rs` | 4／18 | 4／18 | 0 | — |
| 　内訳 `boot_resolve*`・`boot_config*` | 2／12 | 2／12 | 0 | — |
| 　内訳 `app_exit*`・`session_end*`・`exit_wait*`・`write_failure*` | 4／10 | 4／10 | 0 | — |
| 　内訳 `alert*`・`readonly*`・`log_history*` | 3／9 | 3／9 | 0 | — |
| `main_*_tests.rs`（src 直下の全部／移す） | 6 本移す | 10 本ある・移す 6（`config_input`・`ghost_wiring`・`module_policy`・`restore_seam`・`persist_wiring_seam`・`monitor_snapshot_seam`）・残す 4（`finish_after_run`・`session_mark`・`session_mark_fallback`・`session_mark_write`） | 0 | 設計の仕分け表どおり |
| `readme`（`readme.rs`・`readme_tests.rs`・`readme/`） | 13 | 13（本番 5・テスト 8） | 0 | — |
| `roots/` の芯 | 7（本番 4＋テスト 3） | 7（`mod.rs`・`resolve.rs`・`report.rs`・`user_root.rs` と `resolve_tests.rs`・`report_tests.rs`・`user_root_tests.rs`） | 0 | 残る 3 本は `desk`・`edit`・`pick` とテスト 5 本（`desk_tests`・`edit_tests`・`edit_temporary_tests`・`edit_write_failure_tests`・`pick_tests`）の計 8 本 |
| `main.rs` の行数 | 947 | 947 | 0 | — |
| `main.rs` の移す項目の行 | 527〜547・643〜884 | `struct MarkInputs` 527〜`Teardown` の閉じ 547（doc は 526〜）・`MarkVerdict` の doc 643〜`boot_monitor_snapshot` の閉じ 884 | 0 | 間の 548〜642 は `after_run` で残る。734〜736 の区切りのコメントと `// Startup Window` は 643〜884 の範囲に入るので移す側に付く |
| `main.rs` のテストの宣言 | 9（890〜947 行） | 9（`config_input`・`ghost_wiring`・`restore_seam`・`persist_wiring_seam`・`monitor_snapshot_seam`・`finish_after_run`・`module_policy`・`session_mark`・`session_mark_write`） | 0 | 移す 6・残す 3 |
| `register_systems`（`ghost_session.rs:43-80`）の呼び出し | 25（11 番目と 17〜19 番目が本体だけ） | 25（`hooks::install`・`install_input_hooks`・`register_frame_system`・`wire_kanade_stop`・`write_failure::install`・`register_readme_drain`・`register_change_drain`・`register_switch_drain`・`reload::register`・`register_user_break_drain`・`menu::register_menu_poll`・入力 5 本（balloon・box・choice・char_watch・wheel）の 5・`install`・`update`・`mcp`・`add_systems` 2・`wire_zorder_pair`・`scope_rect`・`syncscale`・`rejudge`）。11 番目＝`menu::register_menu_poll`（入力の間）・17〜19 番目＝`install`・`update`・`mcp` | 0 | — |
| `register_systems` の呼び手（本番 1・テスト 5） | 本番 `main.rs:242`・テスト 5 | 本番 `main.rs:242`・テスト `app_exit_escape_tests.rs:137`・`ghost_session_reload_tests.rs:71`・`:179`・`ghost_session_restart_tests.rs:455`・`session/ghost_switch_test_support.rs:237` | 0 | — |
| `crate::input_events` の呼び手 | 26 ファイル（`runtime_tests/` 8・`menu/` 含む） | 26（`runtime_tests/` 8・`menu/` 6） | 0 | `runtime_tests/mod.rs` は引かない（0 件）ので書き換えない |
| 語 `input_events` の当たり（出し直しの道を除く） | `Cargo.toml:44`・`ghost_session.rs:276`・`menu/mod.rs:182`・`app_exit_tests.rs:182` | 同じ 4 か所＋`main.rs:61`（出し直しそのもの・外す） | 0 | `app_exit_tests.rs:182` は直さず申し送る |
| `cli::BootRequest`・`cli::ModulePolicy` を引くファイル | 13 | 15（`main.rs`・`cli/parse_tests.rs`・`cli/parse_readonly_tests.rs` を含む。この 3 本を除くと 12） | +2 | 設計は呼び手の数の目安。書き換えはコンパイルが漏れを止めるので設計は変えない。`main.rs`（`:184`・`:431` の 2 か所）は `session::boot_request::ModulePolicy` の道へ、`cli/` の 2 本は `cli/mod.rs` の私有の `use` を通る形に合わせる（実装の段で確かめる） |
| 集めるモジュールの名前を `crate::…` で引く本体のファイル | 186 | 183（`crate::<名>`・`crate::{…}` の束・`crate::ConfigInputs` ほか `main.rs` の項目の名で引く所の和集合）／`cli::BootRequest` 等も足して 188 | −3（名前の式）／+2（cli を足した式） | 設計の 186 は数え方が分からず再現できない。差は数え方による（行をまたぐ束 `crate::{…}` を含めて数えた）。**実装は漏れをコンパイルが止める**ので、この数は目安にとどめる。移す側のファイル自身（60 本）を含む |
| 移す `main.rs` の項目の中の `info!` | 0 | 0（マクロの呼び出しは debug 4・error 1。`info!` の字面は `settle_session_mark` の doc コメントの 2 行だけ） | 0 | `get_log` の分け方は変わらない（7.5 は手当て不要） |

### 13.4 ソースを読む所（`include_str!|CARGO_MANIFEST_DIR|crate_path\(`・要件 6.2）

`git grep -n -E 'include_str!|CARGO_MANIFEST_DIR|crate_path\(' -- crates/areka/src` は 40 行。集めるファイルの名前を字面に持つものは設計の表に全部ある（足りない分は 0）。

- 設計の表にある所: `runtime_tests/zorder_wiring_tests.rs:68`・`readonly_tests.rs:82`・`:92`・`:153`・`session_end_deadline_tests.rs:405`・`:409`・`:413`・`:519`・`:533`・`readme_tests.rs:424`・`session_end_sync_send_tests.rs:437`・`:445`（直さない）。
- 集めるファイルの名前を持たず、書き換えが要らない所（確かめた）:
  - `runtime_tests/zorder_wiring_tests.rs:33`（`../session/runtime_boot.rs`・既に `session/`）・`:71`（`../main.rs`・残る）。
  - `readonly_tests.rs:78`・`:105`（`src/main.rs`・残る）・`:130`（`crates/` の走査）。
  - `log_history_convention_tests.rs:275`（文書）・`:319`〜`:339`（`src` の走査・根のファイルは `main.rs`）。表の文字列 `areka::…` は 7.2 の表（`:152`・`:156`・`:185`〜`:192`）。
  - `readme_tests.rs:383`・`session_end_deadline_tests.rs:450`・`tick_gate_config_producers_tests.rs:187`・`update/worker_tests.rs:887`（いずれも `src` の木の走査・ファイルを名で絞らない）。
  - `boot_shell_tests.rs:347`（`areka-emo-compose` のフィクスチャを `crate_path` で引く・本体のファイル名は含まない）・`placement_tests/measure_bake_count_tests.rs:159`・`file_drop_wiring_tests.rs:443`（`file_drop.rs`・残る）・`png_image_tests.rs:151`・`mcp/*`・`update/*` ほか。
- 表に足す分: 0 件。実装の段（段 2 の `readonly_tests.rs`・段 5 の `session_end_deadline_tests.rs`・段 3 の `readme_tests.rs`・段 6 の `zorder_wiring_tests.rs`）で、その塊の移動と一緒に直す。

### 13.5 roots の芯の出所（タスク 4.2）

- 芯（`mod.rs`・`resolve.rs`・`report.rs`・`user_root.rs`）は `session/roots/` へ移り、出所は `areka::session::roots::{report,user_root,resolve}` になった。残る `edit`・`desk`・`pick` は `areka::roots::{edit,desk,pick}` のまま。
- このため `RUST_LOG=areka::roots=debug` は、移した後は芯を開かない（`edit`・`desk`・`pick` だけを開く）。芯も開くには `areka::session::roots=debug` を足す。この指定は文書にもテストにも無く（5 章の 11・design.md の RootsSplit の Risks）、既定の動きには関わらない。
- `RULES`・`log-convention.md`・ログのテストの文字列に `areka::roots` は 0 件（どちらの出所も決まりの表に当たらず、種別は前と同じ「残さない」）。`structure.md:418`・`:419` の `areka::roots::user_root` は 8.3 で直す。

### 13.6 `main_*_tests.rs` の仕分け（段 4・タスク 5.2・要件 2.4）

| 前のファイル | 行き先 | 宣言する所とモジュール名 | 足した名前 |
|---|---|---|---|
| `main_config_input_tests.rs` | `session/boot_config_input_tests.rs` | `session/boot_config.rs`・`config_input_tests` | 0（`use super::ConfigInputs` は移った先の `super` で同じ型に当たる） |
| `main_ghost_wiring_tests.rs` | `session/boot_config_ghost_wiring_tests.rs` | `session/boot_config.rs`・`ghost_wiring_tests` | 0 |
| `main_module_policy_tests.rs` | `session/boot_config_module_policy_tests.rs` | `session/boot_config.rs`・`module_policy_tests` | `crate::session::boot_config`・`crate::{ghost_session, install_boot_context}`・`bevy_ecs::prelude::World`（前は `use super::*` が `main.rs` から引いていた） |

- 3 本のテストの名前の末尾と数（`config_input_tests` 13・`ghost_wiring_tests` 4・`module_policy_tests` 3）は前と同じで、道が `session::boot_config::…` に変わっただけ。
- 宣言の上の説明は `main.rs` の字面のまま `boot_config.rs` へ移した（`config_input_tests` は説明なし）。
- 残る `main_persist_wiring_seam_tests.rs` は `use super::*` で `boot_resolve` を引いていたので、`use crate::session::boot_resolve::{…}` に直した（段 6 で移る）。
- 残る 4 本（`finish_after_run`・`session_mark`・`session_mark_fallback`・`session_mark_write`）と段 6 の 3 本は 6.1・6.2 で足す。
- real-run-kit の `bin/real-run-fake-areka.rs` に `"areka::boot_config"` は無かった（出所は `areka::app_exit` だけ・段 5 で直す）。この段で直したのは `launch_tests.rs:211`・`:212` だけ。
