# Brief: audit-areka-app

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の残り（起動・起動オプション・ゴーストの会話の場・SHIORI の結線・終了ほか）** を受け持つ。

## Current State

- 受け持つファイル: `crates/areka/` のうち、上の 7 本の持ち物でないもの全部（`src/cli/`・`src/main.rs`・`src/session/boot_*`・`src/session/ghost_session*`・`src/shiori_host*`・`src/session/app_exit*` ほか・`tests/`・`Cargo.toml`・`build.rs`・`shell/`）。
- 大きさ: 211 ファイル・コード 12,594 行・テスト 18,655 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
- まだ audit をかけていない。指摘はギャップ分析で洗い出す（起票のときに出さなかったのは、着手までにコードが変わって指摘が古びるため）。

## Desired Outcome

- 受け持つ範囲に `/ponytail:ponytail-audit` をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。
- 直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。
- 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑。直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Approach

`/kiro-start` の各段で、次のように進める（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

- **要件**: 「受け持つ範囲の audit の指摘を、3 つの組とも全部直す」という形だけを書く。指摘の中身はまだ書かない。
- **ギャップ分析**: 受け持つ範囲だけを対象にして `/ponytail:ponytail-audit` をかける。指摘の一覧はギャップ分析の報告に残す。番号は audit の番号をそのまま使う。audit の作法のうち 2 つは外す。
  - 「最大 20 件」は外す。全件を出し、省いた件を残さない。
  - 「読まなかった所（Not checked）」は残さない。範囲のファイルは全部読む。
- **要件の討議**: 指摘の一覧を要件に落として、直す対象を確定する。直さずに残す 3 種類（「Constraints」）は、ここで議題にする。
- **設計**: 指摘ごとの直し方を決める（何を消すか・どこにまとめるか・範囲の外の呼び出し元への影響）。直し方は ponytail の「いちばん小さな完全な変更」に従う。消せるものは消し、呼び出し元・テスト・fixture まで直し切る。
- **タスク・実装**: 直す。最後に同じ範囲へ audit をかけ直して、0 件を確かめる（検証の段のタスク）。

## Scope

- **In**: 受け持つファイルの指摘の直し・そのためのテストの追加と直し・直しのために要る範囲の外の呼び出し元の直し（下の「Constraints」の約束に従う）。
- **Out**: 範囲の外の見直し（ほかの 32 本の持ち物）・新しい機能・`vendors/`・clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。

## Boundary Candidates

- 指摘の組（必ず直す・直したほうがよい・できれば）ごと、またはファイルのまとまりごとにタスクを切る。

## Out of Boundary

- ほかの 32 本の範囲のファイル（直しが及ぶときの約束は「Constraints」）。
- 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。

## Upstream / Downstream

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝`session-crate-split`（2026-10-28 `/kiro-discovery`・本体の crate 分けの 6 本目）。受け持つファイルのうち `ghost_session*`・`boot_*`・`app_exit*`・`session_end*`・`exit_wait*`・`write_failure*`・`readonly*`・`alert*`・`log_history*` と main.rs のシームは `session-module-gather` で `crates/areka/src/session/`、`session-crate-split` で `crates/areka-session/src/` へ移る。両 spec がこの brief の「受け持つファイル」を書き換える。本体に残る SHIORI のデモ（`shiori_host`・`shiori_session`・`shiori_demo`・`reference_brain`）は `AREKA_SHIORI_DEMO` を付けたときだけ動く＝消せるかはこの spec の audit で決める。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `clippy-199-lints`（同じクレートに触る）。同じ範囲に触る未着手の spec は、着手のときに roadmap の spec 台帳で確かめる。

## Constraints

- **範囲の外へ直しが及ぶとき**: 関数の形を変えたら、ツリー全体で呼び出し元を探して全部直す。相手の範囲の見直しの spec が走っていれば、そのセッションと申し合わせてから触る。
- **直さずに残してよいのは次の 3 つだけ**。どれも要件の討議で開発者の裁定を仰ぐ。
  1. 開発者の裁定に反する直し。
  2. わざと置いた先送りの印。`#[allow(dead_code)]` に実在の理由の doc を付けた語彙の口（実装規律「語彙完備・配線ゼロの追跡」）と、`shortcut:`・`ponytail:` の注釈で限界が今の負荷を越えていないものがこれに当たる。
  3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。ukadoc で確かめてから決める。
- **守る決まり**:
  - 1 ファイル 1,000 行の番人を越えない。足すなら新しいファイルへ。
  - 決定論のテストで網羅する。
  - 記録の無い失敗の道を作らない（`error!`＋`Err`）。
  - 終了の道は正規の実装にする。
  - 実機の根と一時フォルダは、ワークツリーの `target` の下に置く。
  - GPU のテストは実 GPU で回す。
  - 文字化けをコンソールの文字コードで逃げない。
  - 依存を足すなら `tech.md` へ登記して、開発者の承認を得る。
- 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
- 1 spec 20 タスクを越えそうなら、要件の討議で、指摘の組ごとに同じ範囲の 2 本目として切ってよい。

## 追記（2026-11-06 `audit-sylphya-actor` の設計の討議で回された件）

`audit-sylphya-actor` の #1（使えない記憶のファイルを上書きしない）の直しで、`sylphya.toml` の置き場は書けるのに中身が使えない（TOML として壊れている・`format-version` が違う・読めない）とき、保存が `Degraded` になり、書けなかった知らせが出るようになる。ところが知らせの決まった一文 `MEMORY_WRITE_FIX`（`crates/areka/src/alert.rs`・「直し方: この置き場を書ける状態にしてください。…」）はこの場面に合わない。ウェーブ C18 では `crates/areka` に触れないので、この spec へ回した（2026-11-06 開発者の承認・`audit-sylphya-actor` の設計の討議 議題 1）。

- **件**: 理由（`PersistFailureCause::Io` の文）が「中身を消さないように書くのをやめました」の場面では、直し方の一文を「ファイルを直すか、別の名前に変えてください」の向きに出し分ける（理由の文の出どころは `areka-sylphya` の `persist/format.rs`・`persist/mod.rs`）。

## placement-crate-split からの申し送り（2026-11-06）

- **件 1: ワークスペース全体を歩く見張り 3 本の置き場**。`crates/areka/src/session_end_sync_send_tests.rs`・`crates/areka/src/emo2_boot/frame_attach_tests.rs`・`crates/areka/src/readonly_tests.rs` は、本体のテストなのにワークスペースの全部の crate（依存の向きの外の crate も含む）を歩く。placement-crate-split の読み方の規則（「自分の crate は相対・別の crate は読み口 `crate_source!`／`crate_path`」）の、名前を付けた例外にしてある（設計の「使ってよい依存」）。ワークスペース全体の見張りの置き場 `crates/log-capture-kit/tests/` へ移す（走査は `workspace_scan/mod.rs` を使う）。`emo2_boot/frame_attach_tests.rs` の受け持ちは `audit-emo2-boot-*` の範囲なので、移すときはその spec と申し合わせる。


## 2026-11-15 `audit-placement-core` の完了の申し送り

配置の crate の見直し（`audit-placement-core`）が直しの途中で見つけた、本体の crate（`crates/areka`）の古い文と写し。行は 2026-11-15 の枝の頭（`be720858` の後）で引き直した。着手のときに引き直す。

- **知らせ**: 指摘 13 で本体の `src/runtime/move_cue_place.rs` の主モニタの作業領域の写しを、配置の crate の公開の関数 `areka_placement::work_area_px` に替えた。範囲の外のうち、ほかに直した所は `src/runtime/frame/drain_resnap.rs`・`frame.rs`・`frame/switch.rs`・`frame/zorder_drain.rs`・`skin_switch.rs` と、テストの `frame_chain_rule_tests.rs`・`frame_chain_finalize_*_tests.rs`・`switch_finish_face_tests.rs`・`switch_finish_tests.rs`（下ごしらえの 1 行・999 行）・`zorder_descript_tests.rs`・`zorder_drain_tests.rs`・`placement_tests/zorder_group_branch_coverage_tests.rs`（どれも振る舞いはその spec の記録のとおり）。
- **件 1（81 の残り）**: `examples/emo-present/balloon.rs:46` の `read_balloon_offset` が descript を自分で読み、`balloon.offsetx`／`offsety` の片方だけを「仕様外」として捨てる。配置の crate の 81 と同じ形（片方だけでも読む）に直す。
- **件 2（略称）**: spec の名の略称 `scg`（`areka-P0-scope-chain-gap`）・`atom`（`areka-P0-dpi-transition-atomicity`）が `src/` に 37 行（`git grep -nwE "scg|atom" -- crates/areka/src`・`placement_tests/transition_*` を除く・`ghost_session.rs`・`main.rs`・`main_monitor_snapshot_seam_tests.rs`・`main_restore_seam_tests.rs`・`runtime/frame.rs`・`runtime/frame/dpi.rs`・`runtime/frame/drain_resnap.rs`・`runtime/frame_balloon_offset_follow_tests.rs`・`runtime/frame_chain_finalize_{persist,restart}_tests.rs`・`runtime/frame_chain_finalize_tests.rs`・`runtime/frame_transition_atomicity_tests.rs`・`runtime/move_cue_place.rs:170`）。正式の名に書き換える。`placement_tests/transition_*` の分は追随の持ち物。
- **件 3（消えた名を名指す）**: `src/runtime/frame_diag_route_tests.rs:19` が、もう無い `areka_placement::test_support::capture_logs` を名指す（`audit-emo-atlas` で消えた）。
- **件 4（見送り点の数）**: `src/runtime/frame_transition_branch_tests.rs:501`・`:527-546` が「4 つの見送り点」と書き、5 つ目の `HoldSite::PatternMove`（`src/runtime/frame/move_shift.rs:71`）の呼び出しの綴りを固定していない。同じ所（`:545`）に名前の無い「task 6.5」。
- **件 5（記憶からの戻しの継ぎ目）**: `src/main.rs:940`・`src/ghost_session.rs:241` が継ぎ目を `load_restored_state` → `apply_restored_placements` と書く。今は `restore_and_judge` → `apply_balloon_limit`。
- **件 6（`assets_tests.rs` の古い文）**: `src/runtime/assets_tests.rs:775` の `synth_declared_dpi_ghost`（今は `test_support.rs` にある）・`:783`（配置の crate の `measure_tests.rs` に 1,000 行の余裕が無い、は誤り。今は 815 行）・`:779`・`:788`（名前の無い「タスク 4.2」）。
- **件 7（開発者の記憶の帳面の名）**: コメントが記憶のファイルの名を名指す: `src/input_events/choice_drain.rs:11`・`src/input_events/input_events_tests.rs:255`・`src/boot_config.rs:504`・`:563`・`src/smoke_exit.rs:43`。名指さずに中身を言う文にする。
- **件 8（名前の無い task 番号・古い置き場）**: `src/smoke_exit.rs:1`・`:8`・`:67` の「task 2.3」。`examples/collision-probe/fixture.rs:23` の「main.rs／他 example と同名」（今は `src/smoke_exit.rs:44`）。`src/main_restore_seam_tests.rs:24` が自分の `plant_minimal_ghost` を「persist.rs の `plant_minimal_ghost` と同型」と書く（その関数は消えた）。同じ所の「`resolve` が成功する」も、`load_restored_state` がシェルを解かなくなったので古い。
- **件 9（呼び手の数・届かない腕）**: `src/runtime/frame/drain_resnap.rs:596` が `persist_unremembered_char_positions` の本番の呼び手を「ここ 1 か所だけ」と書く（`reset_pos.rs` も呼ぶ）。同じファイル `:478` の `ChainDeferReason::DpiSyncHeld` の腕は、`collect_chain_states` がその値を返さないので届かない（害は無い）。
- **件 10（テストの支えの写し）**: `src/runtime/move_cue_apply_move_tests.rs:52` の `attach_fake_handles` は `areka_placement::test_support::give_fake_handles` の写し。`SharedFakeIo` の写し（配置の crate の 512）が `src/boot_resolve_tests.rs`・`src/input_events/drag_test_support.rs`・`src/main_persist_wiring_seam_tests.rs` にある（`areka_placement::test_support::SharedFakeIo` を使う）。同じ名の型が `src/runtime/frame/switch_finish_tests.rs:59`・`src/runtime/move_cue_apply_move_sticky_tests.rs:201`・`src/runtime/move_cue_place_tests.rs:330` にもある（写しかを着手のときに確かめる）。

## 申し送り（runtime-crate-split より）

- `crates/areka/src/runtime/` を新しい crate `crates/areka-runtime` へ移した（PR は `runtime-crate-split`）。本体の `crates/areka/Cargo.toml` の依存の説明のうち、runtime が使うために通常依存へ上げたと書く行（58・62〜63・71 行ほか「runtime 統合結線…で通常依存へ昇格」「placement の採寸…で通常依存へ昇格」）は、使い手が `areka-runtime` へ移って理由が古くなった。行と説明は変えていない。
- 本体が直に使わなくなった依存の行は、粗い確かめ（`[dependencies]` の crate 名を本体の本番の `src` で grep）では 0 件だった。`use` を経ない使い方（マクロ・型の再公開）まで含めて数え直し、要らない行は外し、残す行の説明を今の使い手に合わせる。

## audit-emo2-boot-spine からの申し送り（2026-11-17）

`audit-emo2-boot-spine` のギャップ分析（`.kiro/specs/completed/audit-emo2-boot-spine/research.md` の 6 節）で、範囲の外のこの spec の持ち場に見つけた件。中身は向こうでは直さない。

- **オ（`code_only` の写し）**: `crates/areka/src/tick_gate_config_producers_tests.rs:L70` の `code_only` は、`source_text_test_support::code_only` と 1 字も違わない。そちらを使う形に寄せる。
- **キ（古い説明）**: `crates/areka/src/boot_config.rs:L617`・`L621-L622` の `is_benign_boot_error` の説明は「`wire_runtime` が再利用する」と書くが、呼び手は `ghost_session.rs:L783`・`session/runtime_boot.rs:L137` だけ。
- **ク（古い説明）**: `crates/areka/examples/emo-present/setup.rs:L88` の「本番（`build_boot_assets`）」は古い（`audit-emo2-boot-spine` の 43 番と同じ形の文）。

## audit-emo2-boot-frame からの申し送り（2026-11-21）

`audit-emo2-boot-frame` の範囲の外で見つけた、本体の側の件（`.kiro/specs/completed/audit-emo2-boot-frame/research.md` の 6 節オ・コ・サ・シ）。

- **オ（本体の分）**: `crates/areka/src/tick_gate_config_producers_tests.rs` の `code_only` は `source_text_test_support::code_only` の写し。
- **コ**: `let _ = ….handle.join();` が本体に 8 か所ある（2026-11-21 に grep で数え直した）。記憶の書き手の panic を捨てている。`audit-emo2-boot-frame` の 20 番と同じ型で、`areka_runtime::spine::join_bounded` で待って panic を捨てない形に直す。
  - この spec の分は 7 か所: `crates/areka/src/boot_resolve_tests.rs` の 483・576・757・805 行、`install/names_tests.rs` の 28 行、`main_persist_wiring_seam_tests.rs` の 85・128 行。
  - 残りの 1 か所 `input_events/drag_test_support.rs` の 283 行は `audit-areka-input-menu` の持ち物（上の 2026-11-15 の申し送りの読み替えで `drag_test_support.rs` は向こうへ移った）。C23 の `input-crate-split` が `input_events/` を別の crate へ移すので、行の場所はその後で読み替える。
  - `areka-runtime` の 2 か所（`move_cue_*`）は `audit-emo2-boot-switch` の brief へ回した。
- **サ**: 本体の `capture_logs` の写し（`runtime_tests/spine.rs`・`input_events/choice_drain.rs`・`readme/readme_cue_tests.rs`）と、GPU の World の写し（`runtime_tests/spine.rs`。`areka-runtime` の `frame_test_support::gpu_on` を使える）。
- **シ**: 起動の文脈 `BootContext` を組む補助の写しが 5 か所（`install/desk_tests.rs`・`menu/ghost_frame_tests.rs`・`readme/opener_submit_tests.rs`・`session/ghost_switch_tests.rs`・`session/reload_tests.rs`）。

## 2026-11-21 棚卸㊶の再測定（main `31ac3eaf`）
- **前提の変化**: PR#488（コメントだけの整理）で、次の件が消えた（`git grep` で 0）。2026-11-15 の申し送りの件 2（`atom` の本体の分）・件 7（記憶の帳面の名）・件 8 の `smoke_exit.rs` の「task 2.3」と、`audit-emo2-boot-spine` の申し送りのキ・ク。件 8 の `examples/collision-probe/` はもう無い。`input-crate-split`（PR#486）は `app_exit.rs`（`install_input_hooks`。新しい `app_exit_escape_tests.rs` は `audit-areka-input-menu` の持ち物）・`ghost_session.rs`（`file_drop`・`shortcut` のパス）・`main.rs`（`use areka_input as input_events;`）を直した。`audit-emo2-boot-frame`（PR#484）が上の申し送りのオ・コ・サ・シを足した。
- **残る件**（行は今の main）:
  - 件 1（見張りは `session_end_sync_send_tests.rs`・`readonly_tests.rs` の 2 本。`frame_attach_tests.rs` は `areka-runtime` へ移った）・件 5（`main.rs:906`・`ghost_session.rs:210` が `apply_restored_placements` と書く。今の呼び所は `restore_and_judge` → `apply_balloon_limit`）・件 8（`main_restore_seam_tests.rs:24`）・件 10（`boot_resolve_tests.rs:421`・`main_persist_wiring_seam_tests.rs:13` の `SharedFakeIo`）・オ（`tick_gate_config_producers_tests.rs:68`）。
  - コ（`boot_resolve_tests.rs` の 482・575・756・804 行、`main_persist_wiring_seam_tests.rs` の 80・123 行、`install/names_tests.rs` の 28 行）・`MEMORY_WRITE_FIX`（`alert.rs:88`）・`Cargo.toml` の古い依存の説明（60・73 行ほか）。件 6 は着手のときに引き取り先を決める。
- **ほかの見直しの持ち物を指す件（重なりの穴）**: コの `install/names_tests.rs`、サの `readme/readme_cue_tests.rs`、シの `install/desk_tests.rs`・`readme/opener_submit_tests.rs` は `audit-areka-install-update` の持ち物。シの `session/ghost_switch_tests.rs` は `audit-emo2-boot-switch`、シの `menu/ghost_frame_tests.rs` とサの `input_events/choice_drain.rs`（今は `crates/areka-input/src/choice_drain.rs`）は `audit-areka-input-menu` の持ち物。
  - **棚卸㊶の決め**: 持ち主の分は持ち主が直す＝`install/names_tests.rs`・`readme/readme_cue_tests.rs`・`install/desk_tests.rs`・`readme/opener_submit_tests.rs` は `audit-areka-install-update`、`session/ghost_switch_tests.rs:84` は `audit-emo2-boot-switch`（どちらも向こうの brief の「ほかの brief から指される件」に在る）、サの `crates/areka-input/src/choice_drain.rs` は `audit-areka-input-menu`（C24）。この spec が直すのは残り（`boot_resolve_tests.rs`・`main_persist_wiring_seam_tests.rs`・`runtime_tests/spine.rs`・`menu/ghost_frame_tests.rs`・`session/reload_tests.rs`）。シの写しは持ち主ごとに、その範囲の中の補助へ寄せる。1 か所へまとめるのは 3 本の着地の後。これで C26 の 3 本は 0。
  - 本体の `file_drop.rs`・`file_drop_*tests.rs`・`app_exit_escape_tests.rs` は、`audit-areka-input-menu` が C24 に入るために範囲から外したので、この spec の受け持つファイル（ほかの 7 本の持ち物でない本体の残り）に入る（棚卸㊶）。
- **規模**: 指摘の数で決まる。範囲は起票の約 1.7 倍で、20 を越える見込みが高い＝要件の討議で指摘の組ごとに同じ範囲の 2 本目を切る（Constraints のとおり。今は切らない）。上限に近いのは `tests/stay_on_top_real_run.rs` 966・`main_session_mark_tests.rs` 947・`main.rs` 947。
- **先に要るもの**: `session-crate-split`（その前に `session-module-gather`）。範囲の大半（`ghost_session*`・`boot_*`・`app_exit*`・`main.rs` ほか）がこの 2 本で動く＝先には始められない。見込み C26。
- **同じウェーブに置けない相手**: crate 分けの 2 本（待つ）・`doc-compat-architecture-rename`（`ghost_session_scope_scale.rs` が文書を名指す。`reference_brain.rs` の名指しは PR#488 で消えた）・`multi-ghost-runtime`。`audit-areka-install-update`・`audit-emo2-boot-switch` とは、上の穴を持ち主へ寄せれば 0。
- **優先度の区分**: A（開発者「とにかく指摘事項は全部直す方針です」）。要件定義のモデル: Opus（見直しの型どおり）。分割の案: 起票のままでは無し（指摘が 20 を越えたら上のとおり）。

## audit-areka-input-menu からの申し送り（2026-11-21）

`audit-areka-input-menu` の直しで入力の crate のテストを消し、ファイルの名を変えたため、本体の説明が古い名を指すようになった。本体（`crates/areka/`）は C24 の約束で向こうが触れないので、持ち主のこの spec へ回す（`.kiro/specs/completed/audit-areka-input-menu/research.md` の 11.4 節）。行は 2026-11-21 の枝の頭（`5757e6f7f`）で引き直した。着手のときに引き直す。

- **件 1（消したテストの名指し）**: `crates/areka/src/app_exit_escape_tests.rs:73` の説明が `handler_left_double_click_without_ctrl_does_not_despawn_and_sends` を名で指す。このテストは入力の crate の側で消した。Ctrl なしの左ダブルクリックの送出は入力の crate の `handler_double_click_left_and_right_send`（`crates/areka-input/src/char_pointer_tests.rs`）が見ており、窓を閉じない側は古い名を書いている当の `only_escape_terminates_ghost_windows` 自身が見ている。その説明を、消した名でなく役目（入力の crate の送出のテスト）で指す形に書き直す。
- **件 2（テストのファイルの古い名）**: `crates/areka/src/app_exit_tests.rs:182` の説明が「`input_events_tests.rs` の `world_with_wiring` は私有」と書く。ファイルは `crates/areka-input/src/char_pointer_tests.rs` に名を変えた。`world_with_wiring` は入力の crate の `pointer_test_support` の公開の補助なので「私有」も事実でない。パスと名を書かずに、役目で指す形に直す。

- **件 3（古いコメント・2026-11-22 完了の棚卸で追記）**: `crates/areka/src/ghost_session.rs:820` のコメント「`BalloonWiring`／`ChoiceSelectionInbox`」は、結線する `areka_input::balloon::wire_balloon_choice`（`crates/areka-input/src/balloon.rs` の 407〜412 行）が `BalloonWiring`・`BalloonHover`・`ChoiceSelectionInbox`・`ShellBoxHover` の 4 つを挿すので古い。本体のファイルは `audit-areka-input-menu` の範囲の外（要件 7.3）。

## 申し送り（session-module-gather より・2026-11-22）

- **前提の変化**: 起動・終了・切替の層が `crates/areka/src/session/` へ移った（`git mv`・中身はパスだけ）。`main.rs` の項目は `session/session_mark.rs`・`session/boot_seam.rs`・`session/boot_request.rs` へ分かれ、本体だけの系の登録は `main.rs` の `BODY_SYSTEMS`（`BodySystems`）から `session::ghost_session::register_systems` へ渡す。ログの出所は `areka::session::…`。詳しくは `.kiro/specs/completed/session-module-gather/research.md` の §14.6。
- **件 1（所見の文のパス）**: この brief の所見の文（2026-11-06〜11-21 の申し送りの節）に、移す前のパス（`boot_config.rs`・`boot_resolve_tests.rs`・`main_persist_wiring_seam_tests.rs`・`session_end_sync_send_tests.rs`・`readonly_tests.rs` など）が残る。要件の段で `session/…`（`main_persist_wiring_seam_tests.rs` は `session/boot_seam_persist_wiring_tests.rs`）へ読み替える。
- **件 2（呼び手の説明）**: `crates/areka/src/menu/mod.rs:223`・`:254`・`crates/areka/src/mcp/mod.rs:48` の doc は呼び手を `ghost_session::register_systems` と書く。関数ポインタ経由で今も正しいが、名目上の渡し手は `main.rs` の `BODY_SYSTEMS`。直すかどうかを見直しで決める。
- 完了の時にその場で直したもの（この brief の件ではなくなった）: `doc/click-through.md:3`・`:136`、`tools/perf/invoke-perf-run.ps1:29`、`session/app_exit_tests.rs:182`、`areka-placement` の 4 か所、`doc/COMPAT_ARCHITECTURE.md` の `shell_balloon_resolve_tests.rs` の 4 か所。

## 棚卸㊷からの申し送り（2026-11-22）

- 本体の `crates/areka/src/runtime_tests/zorder_wiring_tests.rs` の作業番号の札つきのテスト 2 本（`t_zwi05_…`・`t_zwi08_…`）の名から札を外す（`test-name-work-tags` の受け持ちだったが、同じウェーブ C25 の `session-crate-split` がこのファイルの `include_str!` のパスを直すので、本体の残りを受け持つこの見直しへ寄せた）。外し方は `test-name-work-tags` と同じ＝札だけを外し、ぶつかるときだけ中身を表す語を足す。名指し（`doc/COMPAT_ARCHITECTURE.md` §8 ほか）も同じコミットで直す。
- 同じ 2 本の改名と同じコミットで、本体の `runtime_tests/zorder_wiring_tests.rs` の冒頭の説明コメントの名指しを改名表（`.kiro/specs/test-name-work-tags/rename-table.tsv`）の新しい名へ直し、番号の札を消す。11 行の `areka-runtime` の側の 1 本は `the_frame_system_is_ordered_before_the_chain_apply` に当たる。13 行の到達性は `a_script_tag_reaches_the_ledger_through_the_wired_channel`・`a_script_without_the_tag_leaves_the_mechanism_absent`、相順と相の呼出は `the_frame_system_is_ordered_before_the_chain_apply`・`the_frame_calls_the_zorder_drain_after_the_size_reconcile`、基底を据える順は `the_boot_seats_the_descript_base_before_inserting_the_wiring` に当たる。4・5・10 行は本体の 2 本を指すので、2 本の改名に合わせて番号を消し、中身の言葉の説明を残す（2 本の新しい名は札を外した `the_boot_wires_the_channel_the_sink_and_the_handoff`・`the_entry_point_carries_the_shell_setting_into_the_wiring`）。`areka-runtime` の `zorder_wiring_tests.rs` の冒頭は、番号の札を消して中身の言葉の説明を残す形にしてある。

## 申し送り（dola-cue-test-builder より）

- 正準のキャリアの合図（`\![name,tokens...]`・発火時刻 0・長さ 0）を組むテスト用の口が `dola::cue::test_support::carrier_cue(actor, name, tokens)` の 1 か所になった（`dola` の `test-support` 機能の下）。本体のテストは `areka-runtime` の `consumer_ledger_test_support::carrier_cue(name, args)`（演者 `"0"`）と `actor_carrier_cue(actor, name, args)` で引ける（`areka-runtime` の `test-support` 機能が `dola/test-support` を前送りする）。
- 本体の session のテスト 3 本の `prop_set_cue`（`readonly_run_tests.rs:94`・`readonly_sites_tests.rs:118`・`write_failure_sites_tests.rs:139`。行は 2026-11-22 の main）は、同じ合図を自分の本文で組む写しのまま残っている。上の口へ寄せるかを、この spec の範囲で見る（`session-crate-split` が `session/` を移した後の場所で読み替える）。
- 観測: `crates/areka/tests/smoke_boot_loop_exit.rs` が、機械が混んでいた `cargo test -p areka` の 1 回で 60 秒の待ちを越えて落ちた（単独の回し直しと全体テスト 2 回は緑）。揺れが続くようなら `tools/load-flake.ps1` で測る。
