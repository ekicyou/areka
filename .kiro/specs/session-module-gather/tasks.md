# Implementation Plan

design.md「Migration Strategy」の S0〜S9 の順に並べる。各段の終わりに `cargo check --workspace --all-targets` が警告 0 件で通る形を保ち、段の差分は混ぜない。各段は、その塊の `git mv`・`session/mod.rs` の宣言・`main.rs` の宣言の削除・呼び手の書き換え・その塊の出所を名前で読む所（`RULES` の行・`doc/ssp-mcp/log-convention.md` の表の行・テストの文字列・real-run-kit）・ソースを読むテストを、まとめて済ませる。移したファイルの中身はパスのほかに変えず、可視性は `pub(crate)` までにし、`main.rs` の最上位に `use` を足さない。`main.rs` の残す `fn` が移したモジュールを使う所は、その本文の先頭の `use session::{…};` で引き、本文の字面を変えない（`runtime_tests/zorder_wiring_tests.rs` と `readonly_tests.rs` が `fn main` の字面を読む）。署名と構造体の欄は `session::…` の道で書く。`RULES` の行は、そのモジュールのファイルを移す段と同じ段で付け替える（表の守り `locate` は出所の文字列からファイルを探すので、両方が同じ段でそろえば各段で緑）。各段は、その段で移すファイルを、ディレクトリを付けて名指す説明のパスも直す（9.2 を記録だけの段に保つ）。直したくなった不具合ともとから古い説明は直さず、9.2 で申し送りに載せる。同じウェーブの約束で触らない所（`crates/areka-runtime/` の 1 か所のほか・`crates/areka-placement/`・`crates/areka-input/`・`crates/log-capture-kit/`・本体の `runtime_tests/mod.rs`・`Cargo.lock`・ルートの `Cargo.toml`・`THIRD-PARTY-NOTICES.md`）には、どの段でも触れない。

- [ ] 1. 着手の測り（コードを触る前の「前」を取る）
- [ ] 1.1 テストの名前・警告・設計の数を今の木で取り直す
  - `cargo test --workspace --no-fail-fast -- --list` の `: test$` の行から名前の末尾（最後の `::` から後ろ）の多重集合を作り、比べる「前」として置き場を `research.md` に書く
  - `cargo check --workspace --all-targets` の警告が 0 件であることを記録する
  - 設計の数を `git ls-files`・`git grep` で数え直す: 移すファイルの一覧（設計の表の 62 本・`main_*_tests.rs` 6 本・readme 13 本・roots の芯 7 本）・`main.rs` の移す項目の行の範囲・`register_systems` の 25 の呼び出し・`crate::input_events` の呼び手 26 ファイル・名指す本体のファイル・`include_str!|CARGO_MANIFEST_DIR|crate_path\(` で読む所（集めたファイルの名前を字面に持つものを、設計の SourceReadingTests の表に足りない分も含めて列挙）・移す項目の中の `info!` が 0 件であること
  - 完了の形: `research.md` の新しい節に、着手の main の SHA・テストの数・名前の多重集合の置き場・警告 0 件・数え直した数と設計の数との差（差があれば行き先も）が書かれている
  - _Dispatch: mechanical — 決めた式で数えて記録するだけ_
  - _Requirements: 7.5, 10.2, 10.4_

- [ ] 2. 入力の出し直しを外す（段 1・移動と関係なく今の場所で済む）
  - `main.rs` の `use areka_input as input_events;` とその上の説明 2 行を外し、`crates/areka/Cargo.toml:44` の説明から出し直しの括弧書きを外す（依存の行は変えない）
  - 本体の 26 ファイルの `crate::input_events::` を `areka_input::` に、`use crate::input_events;` を外して `input_events::` を `areka_input::` にする。呼ぶ項目は変えない。`runtime_tests/mod.rs` は書き換えない
  - 説明の中で入力のモジュールを `input_events` の語で指す `ghost_session.rs:276`・`menu/mod.rs:182` を `areka_input` へ直す（`app_exit_tests.rs:182` は直さない）
  - 完了の形: 警告 0 件で通り、`git grep -n -E 'crate::input_events|input_events::' -- crates/areka` が 0 件、語 `input_events` の当たりは `app_exit_tests.rs:182` の 1 件だけ
  - _Dispatch: mechanical — 名前の置き換えだけで形は設計が与える_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 9.3_

- [ ] 3. 葉のモジュールを集める（段 2）
- [ ] 3.1 `alert`・`exit_wait`・`write_failure` を `session/` へ移す
  - 本番とテストを `git mv` で `crates/areka/src/session/` へ移し、`session/mod.rs` に `pub(crate) mod` を足し、`main.rs` の宣言と、その上の説明を `session/mod.rs` の宣言の上へ移す
  - 呼び手の `crate::<名>` を `crate::session::<名>` へ、`use crate::{a, b};` の束は集めた名前を `crate::session::{…}` の束へ分ける
  - `main.rs` の `fn main` ほかが使う `write_failure::stop_boot`・`alert::…`・`exit_wait::…` は、本文の先頭の `use session::{…};` で引いて字面を保つ（`.map_err(write_failure::stop_boot)?;` を `zorder_wiring_tests.rs` が読む）。`scene: Option<alert::AlertScene>` のような署名・欄は `session::alert::…` の道で書く。最上位の `use` は足さない
  - 完了の形: 警告 0 件で通り、3 つのモジュールのテストが前と同じ名前で `session::` の下に並び、`zorder_wiring_tests.rs` が緑
  - _Dispatch: mechanical — 移動とパスの書き換えだけ_
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 1.6, 6.1, 7.9_

- [ ] 3.2 `readonly` を移し、それを読むソースのテストを移した先へ向ける
  - 本番とテストを `git mv` で移し、宣言と呼び手を 3.1 と同じ形で直す
  - `readonly_tests.rs:82` の較正の針を `mod session;` に、`:92` の絞り込みを `session/readonly.rs`・`session/readonly_`（区切りは `all_sources` の書き方）にし、絞り込んだ集合が空でないことの `assert!` を足す。`:153` を `session/readonly.rs` にする
  - 完了の形: 警告 0 件で通り、`readonly_tests.rs` の 3 本が移した先を走査して緑。絞り込みの対象を一時的に実在しない名前にすると足した `assert!` で赤になることを確かめ、戻す
  - _Dispatch: standard — 絞り込みの書き方と空振りの守りを実物に合わせて組む_
  - _Requirements: 1.1, 1.2, 1.6, 6.1, 6.2_

- [ ] 3.3 `log_history` を移す
  - 本番とテスト（`log_history_tests.rs`・`log_history_convention_tests.rs` ほか）を `git mv` で移し、宣言と呼び手（`main.rs` の `init` の呼び出しほか）を直す
  - `RULES` の行はここでは変えない（`RULES` に当たるモジュールは後の段で移り、その段で付け替える）。表の守りの探し方（`locate`）は変えない
  - 3.1 で移した `alert` の直書き `"areka::alert"`（`log_history_tests.rs:125`・`log_history_convention_tests.rs:156`・`:185`・`:192`）を `"areka::session::alert"` にし、`log-convention.md` の表に同じ行があれば合わせる（散文の `:135` は 8.2）（どちらも残さない種別なので赤にはならないが、9.2 の式 4 の 0 件を段の中で満たす）
  - 完了の形: 警告 0 件で通り、`log_history` のテストが前と同じ合否で、`git grep -n '"areka::alert"'` が 0 件
  - _Dispatch: mechanical — 移動と、設計が字面を与える文字列の書き換え_
  - _Requirements: 1.1, 1.2, 1.6, 6.1, 7.3_

- [ ] 4. readme と roots の芯を集める（段 3）
- [ ] 4.1 readme を移し、出所の名前を読む所をそろえる
  - `readme.rs`・`readme_tests.rs`・`readme/` の 11 本を `git mv` で `session/` へ移し、宣言と呼び手を直す
  - `RULES` の `areka::readme` を `areka::session::readme` へ、`doc/ssp-mcp/log-convention.md` の表の同じ行と、`log_history_tests.rs`・`log_history_convention_tests.rs` の `"areka::readme"`・`"areka::readme::opener"` を同じコミットで直す
  - `readme_tests.rs:424` の `"areka/readme/os_port.rs"` を `"areka/session/readme/os_port.rs"` にする
  - 完了の形: 警告 0 件で通り、readme のテストと `log_history` の表のテストが前と同じ合否
  - _Dispatch: mechanical — 移動と、設計が字面を与える文字列の書き換え_
  - _Requirements: 1.1, 1.2, 3.2, 3.4, 6.1, 6.2, 7.1, 7.3_

- [ ] 4.2 roots の芯を移し、残る 3 本の宣言を `roots.rs` に置く
  - `roots/mod.rs`・`resolve.rs`・`report.rs`・`user_root.rs` とテスト 3 本を `git mv` で `session/roots/` へ移す。`session/roots/mod.rs` から `mod desk;`・`mod edit;`・`mod pick;` と `pub(crate) use desk::install;`・`pub(crate) use edit::*;` を外し、それを今の字面のまま新しい `src/roots.rs` に置く（新しい `src/roots/mod.rs` は作らない）
  - `roots/edit.rs` の `super::report`・`super::{ResolvedRoots, RootStatus, RootsOrigin, encode_memory}`・`super::user_root` を `crate::session::roots::…` へ。`desk.rs` の `super::add_root` はそのまま
  - 呼び手の `crate::roots::<名>` を、芯の項目なら `crate::session::roots::<名>` へ、残る 3 本の項目ならそのままにする
  - 完了の形: 警告 0 件で通り、`roots/edit_tests.rs`・`desk_tests.rs`・`pick_tests.rs` と芯のテスト 3 本が前と同じ合否。`RUST_LOG=areka::roots=debug` が芯を開かなくなることを `research.md` に記録する
  - _Dispatch: standard — 項目ごとに芯か残る側かを分ける書き換えがある_
  - _Requirements: 1.1, 1.2, 1.4, 3.1, 3.4, 3.5, 6.1_

- [ ] 5. 起動の構成を集める（段 4）
- [ ] 5.1 cli の型 2 つを `session/boot_request.rs` へ移す
  - `cli/mod.rs` 54〜74 行の `BootRequest`・`ModulePolicy` を、`derive`・欄・doc の字面のまま新しい `session/boot_request.rs` へ写し、可視性だけを `pub(crate)` に上げる
  - `cli/mod.rs` は私有の `use crate::session::boot_request::{BootRequest, ModulePolicy};` で引き、出し直さない。`crate::cli::{BootRequest, ModulePolicy}` の呼び手（13 ファイル・まだ src 直下にある `boot_config.rs` を含む）を `crate::session::boot_request::…` へ
  - `main.rs` の `cli::ModulePolicy::Skip`（184 行付近）と署名 `modules: cli::ModulePolicy`（431 行付近）も、この段で型の移動に合わせる（出し直さないので、ここで直さないとコンパイルが通らない）。本文は先頭の `use session::…` で、署名は `session::boot_request::…` の道で書く
  - 完了の形: 警告 0 件で通り、`cli/parse*_tests.rs`・`install/startup_tests.rs` が前と同じ合否で、`git grep -n -E 'cli::(\{[^}]*)?\b(BootRequest|ModulePolicy)\b' -- crates/areka` が 0 件
  - _Dispatch: mechanical — 型を字面のまま写し呼び手のパスを直すだけ_
  - _Requirements: 3.3, 3.4, 6.1_

- [ ] 5.2 `boot_config`・`boot_resolve` を移し、出し直しを外し、テスト 3 本を仕分ける
  - `boot_resolve*` 6 本・`boot_config*` 8 本を `git mv` で移し、宣言と呼び手を直す
  - `main.rs` の `pub(crate) use boot_config::{ConfigInputs, default_app_profile_dir, default_helper_exe_path, ghost_boot_options, is_benign_boot_error}` を外し、`crate::{ConfigInputs, …}` の形の呼び手を全部この段で `crate::session::boot_config::…` へ（今の測りで `ghost_session.rs:24`・`ghost_session_runtime_tests.rs:11`・`session_end_tests.rs:21`・`main_session_mark_tests.rs:23`・`main_session_mark_write_tests.rs:12`・`session/ghost_switch_test_support.rs:36`・`session/ghost_switch_reload_tests.rs:376`・`:433`・`:591`・`session/runtime_boot.rs:138`・`shiori_demo.rs:231`。着手のときに数え直す）。`main.rs` の残す `fn` が使う分は、その本文の先頭の `use session::{…};` で引く
  - `main_config_input_tests.rs`・`main_ghost_wiring_tests.rs`・`main_module_policy_tests.rs` を `git mv` で `session/boot_config_{input,ghost_wiring,module_policy}_tests.rs` へ移し、`boot_config.rs` に今のモジュール名（`config_input_tests` ほか）で `#[cfg(test)] #[path]` の宣言を足す。足りなくなる名前は `crate::session::…` で足す
  - `boot_config.rs` の `//!` の「`main.rs`（`pub(crate) use` で crate 直下へ再輸出）」と檻の名前の説明を、出し直しの無い形と新しいファイル名へ直す
  - `RULES` の `areka::boot_config`・`areka::boot_resolve` と `log-convention.md` の表の行・`log_history` のテストの文字列・`tests/emo2_mcp_real_run.rs` の `rust_log` と前置き `BOOT_CONFIG_PREFIX` と説明・real-run-kit の `"areka::boot_config"`（`launch_tests.rs`・`bin/real-run-fake-areka.rs`）を同じコミットで新しい出所へ
  - 完了の形: 警告 0 件で通り、移した 3 本のテストが前と同じ名前と合否で、`main.rs` に `pub(crate) use` と最上位の `use session` が無い。`main_*_tests.rs` の仕分け（移した 3 本）を `research.md` に記録する
  - _Dispatch: standard — 出し直しを外して呼び手と `main.rs` の本文の引き方を組み直す_
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 1.6, 2.3, 2.4, 6.1, 7.1, 7.3, 7.9_

- [ ] 6. 終わり方と起動中の印を集める（段 5）
- [ ] 6.1 `app_exit`・`session_end` を移し、`session_mark.rs` を作る
  - `app_exit*` 3 本（`app_exit_escape_tests.rs` を含む）・`session_end*` 4 本を `git mv` で移し、宣言と呼び手を直す
  - `main.rs` の `MarkInputs`・`Teardown`・`MarkVerdict`・`session_mark_verdict`・`settle_session_mark` を、doc と `[main]` の前置きのある記録の文を含む字面のまま新しい `session/session_mark.rs` へ写し、可視性だけを `pub(crate)` に上げる。`main.rs` の残す項目は本文の先頭の `use session::{…};` で引く
  - `main_session_mark_tests.rs`（子 `main_session_mark_fallback_tests.rs`）・`main_session_mark_write_tests.rs` は `main.rs` に残し、移った項目を `crate::session::session_mark::…` で引く
  - `session_end_deadline_tests.rs:405`・`:409`・`:413`・`:519`・`:533` の `"areka/ghost_session.rs"`・`"areka/session_end.rs"` のうち `session_end` の分を `"areka/session/session_end.rs"` にする（`ghost_session` の分は 7.1）
  - real-run-kit の `RELIED_TARGETS`（`launch.rs:372`）と `launch_tests.rs`・`close_tests.rs`・`bin/real-run-fake-areka.rs` の `"areka::app_exit"`、`log_history` のテストの `"areka::app_exit"`・`log-convention.md` の行を `areka::session::app_exit` へ
  - `launch_tests.rs` の `compose_rust_log_table` の前方一致の 2 行も同じ意味のまま新しい名前へ: `areka::app=debug`（246〜247 行・何も足さないことを確かめる行）を `areka::session::app=debug` に、似て非なる名の `areka::app_exits`（252〜253 行）を `areka::session::app_exits` に。式 4 の `\b` では拾えないので、直書きの出所の一覧（9.2）にも載せる
  - 完了の形: 警告 0 件で通り、印の判定のテスト・`session_end_deadline_tests.rs`・real-run-kit のテストが前と同じ合否。残した `main_session_mark*_tests.rs` の仕分けを `research.md` に記録する
  - _Dispatch: standard — 項目の写しと、残すテストの引き方・real-run-kit の名指しを合わせる_
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 1.6, 2.1, 2.4, 6.1, 6.2, 7.3, 7.7, 7.9_

- [ ] 7. ゴーストの単位と系の登録を集める（段 6）
- [ ] 7.1 `ghost_session` の家族を移し、`boot_seam.rs` を作る
  - `ghost_session*`（`_runtime`・`_scope_rect`・`_scope_scale` を含む）・`shell_balloon_switch_session_*`・`boot_shell_tests.rs` を `git mv` で移し、`#[path]` の入れ子とモジュール名は今のままにする
  - `main.rs` の `restore_merged_placements`・`insert_persist_wiring`・`on_boot_ok`・`record_last_used_at_boot`・`record_last_used`・`boot_monitor_snapshot` を字面のまま新しい `session/boot_seam.rs` へ写し、可視性だけを上げる。本番の呼び手（`ghost_session.rs:204`・`:227`・`:720`・`ghost_session_runtime.rs:55`・`session/ghost_switch.rs:569`）を `crate::session::boot_seam::…` へ
  - `main_restore_seam_tests.rs`・`main_persist_wiring_seam_tests.rs`・`main_monitor_snapshot_seam_tests.rs` を `git mv` で `session/boot_seam_{restore,persist_wiring,monitor_snapshot}_tests.rs` へ移し、`boot_seam.rs` が今のモジュール名で宣言する。古い名を説明で名指す所（`boot_resolve_tests.rs:420` の `main_persist_wiring_seam_tests.rs` ほか）を新しい名へ直す
  - `runtime_tests/zorder_wiring_tests.rs:68` を `include_str!("../session/ghost_session.rs")` に、`session_end_deadline_tests.rs` の `ghost_session` の分を `"areka/session/ghost_session.rs"` にする。`RULES` の `areka::ghost_session` と `log-convention.md` の行・`log_history` のテストの文字列・`tests/emo2_mcp_real_run.rs` の `areka::ghost_session` を同じコミットで直す
  - 完了の形: 警告 0 件で通り、`SwitchRig::new` を使う 46 ファイルと `zorder_wiring_tests.rs`・移した 3 本のテストが前と同じ合否。`main.rs` が `fn main`・`finish_after_run`・`after_run`・`AfterRun`・`install_boot_context`・`first_boot_origin`・`boot_first_ghost` と宣言だけを持つ（非空行が約 600）。`main_*_tests.rs` の仕分けの表（9 本全部・5.2 と 6.1 で書いた分に、この段の 3 本と残す `main_finish_after_run_tests.rs` を足す）が `research.md` にそろう
  - _Dispatch: standard — 最大の塊で、呼び手と入れ子とソースを読むテストを同時に合わせる_
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 1.6, 2.1, 2.2, 2.4, 6.1, 6.2, 7.1, 7.3, 7.9_

- [ ] 7.2 本体だけの系の登録を `main.rs` から渡す
  - `session/ghost_session.rs` に `BodySystems`（`menu_poll`・`after_input` の関数のポインタ 2 つ・`Clone, Copy`）を足し、`register_systems` に `body: BodySystems` を足して、`menu::register_menu_poll(world)` を `(body.menu_poll)(world)` に、`install`・`update`・`mcp` の 3 行を `(body.after_input)(world)` にする。ほかの呼び出しの順は変えない
  - `main.rs` に `BODY_SYSTEMS` と `register_body_after_input`（`install::register` → `update::register` → `mcp::register`）を足し、`fn main` の 1 か所で渡す
  - 登録を呼ぶテスト 5 か所（`app_exit_escape_tests.rs:137`・`ghost_session_reload_tests.rs:71`・`:179`・`ghost_session_restart_tests.rs:455`・`session/ghost_switch_test_support.rs:237`）に `crate::BODY_SYSTEMS` を足す
  - 完了の形: 警告 0 件で通り、`register_systems_installs_the_escape_hook`・`ghost_session_restart_tests.rs`・`ghost_session_reload_tests.rs`・`SwitchRig::new` を使うテストが前と同じ合否で、`session/` の本番に `menu::register_menu_poll`・`install::register`・`update::register`・`mcp::register` の名指しが 0 件
  - _Dispatch: standard — 設計が形を与える契約の変更と呼び手の追従_
  - _Requirements: 2.5, 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 7.3 `BODY_SYSTEMS` の欄の取り違えを一度だけ試して記録する
  - `BODY_SYSTEMS` の 2 つの欄を一時的に入れ替えて本体のテストを回し、赤になるテストを一覧にする。入れ替えは戻し、コミットしない
  - 完了の形: `research.md` に赤になったテストの一覧か、0 件のときは「取り違えを止める守りは字面の見直しだけ」の記録があり、0 件のときは design.md の RegistrationInjection の Risks にも同じ一文がある。作業の木は入れ替える前と同じ
  - _Dispatch: mechanical — 決めた試しを回して結果を書くだけ_
  - _Requirements: 4.2_

- [ ] 8. 本体の外の説明・文書・steering のパスを追従させる（段 7・モジュールが全部移った後）
- [ ] 8.1 (P) ほかの crate の説明と `tools/` のパスを直す
  - `crates/areka-runtime/src/placement_tests/transition_judge_offset_signoff_tests.rs:57`（`crates/areka-runtime/` で触るのはこの 1 か所だけ）・`crates/areka-kanade/src/schedule/script_log.rs:16`・`crates/pilot/examples/pilot-windows-rs-next/REWRITE_TABLE.md:66`・`crates/areka/tests/emo2_real_run.rs:58` のパスを移した先へ。`crates/real-run-kit/` の説明に残る本体のパスは `git grep` で所と行を挙げてから直す
  - `tools/package.ps1:149`・`:153`・`:695`・`tools/perf/judge-perf.py:96` は説明のパスだけを直し、道具が字面で読む目印の本文は変えない
  - 完了の形: 警告 0 件で通り、上の各所が移した先の実在するパスを指し、`git diff --name-only` で `crates/areka-runtime/` の変更が 1 ファイルだけ
  - _Boundary: 本体の外の crate の説明・本体の `tests/emo2_real_run.rs` の説明・tools_
  - _Dispatch: mechanical — 設計が所と行き先を与える説明のパスの書き換え_
  - _Requirements: 6.3, 6.4, 9.1_

- [ ] 8.2 (P) `doc/` と網羅台帳のパスを直し、報告を作り直す
  - `doc/COMPAT_ARCHITECTURE.md` §8 の既にある行のパスだけを書き換え、行を足さない・消さない。`:189` の `restore_merged_placements` は `session/boot_seam.rs` へ、`:318`・`:319` の `session_mark_verdict` は `session/session_mark.rs` へ、項目の名で引いて直し、同じ行の `main.rs` に残る項目は直さない
  - `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所を別々の行だけ書き換え、`cargo run -p ukadoc-survey -- report` と `report-summary` で `report/` を作り直す（手で直さない）
  - `doc/ukadoc-coverage/briefing-assets.md:174`・`:213` を `crates/areka/src/session/log_history.rs` の `init`（行番号は書かない）へ。`doc/testing.md:47`・`doc/ssp-mcp/log-convention.md` の `:3`・`:39`・`:135`・`:143`（`crate::log_history::last_id()` を `crate::session::log_history::last_id()` へ）・`doc/pasta-real-run.md:127`・`:141`・`doc/ideas/novel-redesign-2026-11-20/` の 5 ファイルのパスと `RUST_LOG` を新しい名前へ。`doc/README.md:33` と `doc/click-through.md:3`・`:136` は直さない
  - 完了の形: `report/` の作り直しが台帳の変更だけを差分に出し、§8 の行数が前と同じで、上の各所が移した先を指す
  - _Boundary: doc_
  - _Dispatch: mechanical — 設計が所と行き先を与える文書のパスの書き換えと決まった作り直し_
  - _Requirements: 6.5, 7.3, 9.4_

- [ ] 8.3 (P) steering 3 本と audit 2 本の brief を移した先へ合わせる
  - `.kiro/steering/structure.md` の本体の節（`session/`・`roots/`・`cli/`・`readme`・起動と終了の最上位のファイル・`log_history.rs`・`:281` の出し直し・`:319`・`:348`・`:418`・`:419`）・`.kiro/steering/tech.md:171`・`.kiro/steering/logging.md:103` を移した先と新しい出所へ
  - `.kiro/specs/audit-areka-app/brief.md`・`.kiro/specs/audit-areka-install-update/brief.md` の「受け持つファイル」のパスだけを書き換え、持ち主は変えない（roots の芯と readme は `audit-areka-install-update` のまま）
  - 完了の形: steering 3 本が `session/` の新しい形と `main.rs` の残る範囲を述べ、audit 2 本の brief の受け持つファイルが全部実在するパスを指す
  - _Boundary: steering・brief_
  - _Dispatch: mechanical — パスの書き換えで持ち主は変えない_
  - _Requirements: 6.6, 6.7_

- [ ] 9. 較正と構造の証跡（段 8）
- [ ] 9.1 実のモジュールのパスで種別を引く較正を 1 本足す
  - `log_history_tests.rs` に 1 本（例 `moved_modules_keep_their_kinds_by_real_module_path`）を足す。行ごとに、移したモジュールの本番の項目 1 つ（型か総称でない関数）の `std::any::type_name`／`type_name_of_val` から最後の `::<項目名>` を落とした文字列を導き、期待の字面（`areka::session::alert` ほか）と `assert_eq!` で比べてから `classify(Level::INFO, …)` を引く
  - 期待: `boot_config`・`boot_resolve`・`ghost_session`・`ghost_session::ghost_session_runtime`・`readme`・`readme::opener` は status、`alert`・`app_exit`・`session_end`・`exit_wait`・`write_failure`・`readonly`・`roots::report`・`roots::user_root` は残さない
  - 完了の形: 足した 1 本が緑で、`RULES` の 1 行を一時的に古い字面へ戻すと赤になることを確かめて戻す。移したファイルの中身には触れない
  - _Dispatch: mechanical — 期待の値と取り方を設計が与えるテスト_
  - _Requirements: 7.1, 7.4, 10.3, 10.8_

- [ ] 9.2 grep の式 1〜6・外への参照・触らない所・申し送りを記録する
  - 設計の VerificationRecord の grep の式 1〜6 を集め終えた木に当て、結果と除いた所（理由つき）を `research.md` に書く。当たりが残れば直してから記録する
  - 直書きの出所の扱いの一覧（設計の表に、実装で触った所を足したもの）を `research.md` に置く
  - OutboundRefRecord の式で `session/` の本番が本体の外を参照する所を、ファイルと行と項目の一覧にし、テストだけの参照（`crate::BODY_SYSTEMS` を引く 5 か所・根の `after_run`・`boot_first_ghost` を引くテストほか）を別の表にする。向きは逆にしない。`session-crate-split` への申し送り（テストが本体の `BODY_SYSTEMS` を引けなくなる見込み）を書く
  - `git diff --name-only main...HEAD` で触らない所（9.1〜9.5）が 0 件（`crates/areka-runtime/` は 1 ファイル）であることを確かめる
  - 申し送りを書く: `crates/areka-input/src/user_break.rs:201`（まだ `crates/areka/src/readme.rs` を名指すなら）・`doc/click-through.md:3`・`:136`・`tools/perf/invoke-perf-run.ps1:29`・`app_exit_tests.rs:182` を `audit-areka-app`・`audit-areka-install-update` へ
  - 完了の形: `research.md` に式 1〜6 の 0 件の結果と除いた所の一覧・直書きの出所の一覧・外への参照の本番とテストの 2 表・触らない所の確認・申し送りがそろっている
  - _Dispatch: mechanical — 決めた式を当てて記録する_
  - _Requirements: 1.3, 5.3, 6.8, 6.9, 7.2, 7.8, 8.1, 8.2, 8.3, 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 10.1_

- [ ] 10. 振る舞いが変わらないことの最終の検証（段 9）
- [ ] 10.1 テストの名前・警告・全体テスト・移動の検出を確かめる
  - 1.1 と同じ取り方で名前の末尾の多重集合を作り、前と比べる。差は 9.1 の 1 本だけであることを対応表で説明する
  - `cargo check --workspace --all-targets` が警告 0 件・`pwsh -NoProfile -File tools/test-all.ps1` が緑（裏で回す）
  - `git diff -M --stat main...HEAD` で移したファイルが全部移動に見えることを確かめ、そうでないものがあれば一覧に残す
  - 完了の形: `research.md` に多重集合の比べと対応表・警告 0 件・全体テストの緑・移動の検出の結果が書かれている
  - _Dispatch: mechanical — 決めた手順を回して記録する_
  - _Requirements: 1.1, 10.2, 10.3, 10.4, 10.5_

- [ ] 10.2 実機テスト 2 つを新しい出所の名前で 1 回ずつ走らせる
  - 排他テストの机で順を取り、`crates/areka/tests/emo2_mcp_real_run.rs`（`rust_log("warn,areka::session::boot_config=info,…")` と前置き `areka::session::boot_config: `）と `crates/areka/tests/smoke_boot_loop_exit.rs` を 1 回ずつ走らせ、終わったら机をすぐ返す。実機の根と一時フォルダはワークツリーの `target\` の下・絶対パス
  - 実機で回すのはこの 2 つだけにする
  - 完了の形: 2 つが緑で、`research.md` に走らせた `RUST_LOG`・結果・起動の記録と MCP の窓口が拾えたことが書かれている
  - _Dispatch: mechanical — 決めた 2 本を回して記録する_
  - _Requirements: 2.5, 4.4, 7.6, 7.7, 10.6, 10.7_
