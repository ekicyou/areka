# Brief: audit-areka-app

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の残り（起動・起動オプション・ゴーストの会話の場・SHIORI の結線・終了ほか）** を受け持つ。

## Current State

- 受け持つファイル: `crates/areka/` のうち、上の 7 本の持ち物でないもの全部（`src/cli/`・`main.rs`・`boot_*`・`ghost_session*`・`shiori_host*`・`app_exit*` ほか・`tests/`・`Cargo.toml`・`build.rs`・`shell/`）。
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
