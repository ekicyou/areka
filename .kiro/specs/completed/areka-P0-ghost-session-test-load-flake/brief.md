# Brief: areka-P0-ghost-session-test-load-flake

## Problem
機械が重いとき（他のセッションの cargo・rustc・areka が約 15 本並走）に `cargo test -p areka --bin areka` を回すと、ゴーストの切り替えまわりのテストが**毎回違う組で 5〜10 件**赤になる。開発者と実装の手順（`/kiro-impl` のレビュー・`/kiro-complete` の全体テスト）は複数のワークツリーを並べて走らせるのが常なので、本物の欠陥でない赤がレビューの判定を濁し、回し直しの手間と「関係ない赤」を見逃す癖を生む。

## Current State
- 2026-10-04、`areka-P0-mcp-log-history` の 2.3 のレビューで観測した。赤になった族: `ghost_session_switch_tests.rs`・`ghost_session_switch_fallback_tests.rs`・`shell_balloon_switch_session_update_tests.rs` ほか `crates/areka/src/ghost_session_*_tests.rs`・`shell_balloon_switch_session_*_tests.rs`。1 回目の全体で約 10 件、`ghost_session::` だけの回し直しで 5 件、組は 2 回とも違った。
- 単独で回すと緑（`switch_to_self_takes_down_and_reboots_a`・`switch_translate_tests` など）。静かな時の `cargo test -p areka --bin areka ghost_session` は 41 件緑（104 秒）。`tools/test-all.ps1` の全体も緑。
- 変更の差分（log の出口の付け替え）は単体テストが通らない `fn main()` だけで、原因ではない。
- 疑い: 落ち着くのを待つテストが壁時計の締切（例: `ghost_session_restart_tests.rs` の 10 秒の deadline、`run_bounded(.., 20 秒)`）で待っており、負荷で締切を越える。**全体テストでの赤も 1 回記録がある**: 10-05、`host32-testdll-marker-race` の完了時の `tools/test-all.ps1` で `ghost_session::switch_tests::fallback_tests` の 2 本（`target_connect_fail_boots_default_with_halt_and_no_alert`・`sync_target_boot_failure_leaves_only_default_windows`）が「切替先の失敗で既定ゴーストへ戻らない」で赤（他のセッションの cargo と並走して x64 段が約 2 倍の 1,497 秒・直前に `target\nar-samples\work` の残骸の退避が os error 5・単独では 9 本すべて緑＝roadmap の覚え書き「一度だけ落ちた試験」）。os error 5 との関係も調べる。
- **追加の観測（2026-10-05・`mouse-drag-events` の 2.2・2.3 の検証）**: 負荷の高いときの `cargo test -p areka` 全体で、上の族のほかに `install` の desk の上書き・`session_end` の `sync_send`・`emo2_boot::ghost_switch::boot_event_tests::a_switch_with_a_boot_event_boots_the_target_with_that_event_instead_of_changed_or_boot`（記録の並びの比較）も、回ごとに違う組で 1〜8 件赤になった（単独・別の回では緑）。同じ「壁時計の締切で待つ」形かを、本 spec の再現の手順で一緒に見る。

## Desired Outcome
- 負荷をかけた条件で赤を再現する手順が 1 つあり、どのテストがどの待ちで落ちるかが分かっている。
- 落ちる理由が「締切が短すぎる」なら、締切を延ばすのでなく**待つ条件を観測（チャネル・状態の到達）へ置き換える**か、締切を越えたときの失敗の文言が「負荷で遅い」と区別できる形にする（sleep を足して直さない）。
- 本物の競合（順序の取り違え）が見つかったら、それを直すテストを 1 本添えて直す。
- 再現しなければ、その事実（何を試して赤 0 件か）を記録して据え置きの行へ移す。

## Approach
まず再現: `cargo test -p areka --bin areka ghost_session` を、CPU を食う並走（`cargo build` の並列や `stress` 相当の子プロセス）を付けて数回回し、落ちたテスト名と失敗の文言を集める。落ちたテストの待ちを読み、壁時計の締切に頼る待ちを観測の待ちへ置き換える。

## Scope
- **In**: 上のテストの族の待ち方の見直し・再現の手順の記録・本物の競合があればその修正。
- **Out**: 本番のゴーストの切り替えの振る舞いの変更（競合が見つかった場合を除く）・他の族（`zorder-chain-residue` の族）。

## Boundary Candidates
- テストの待ちの部品（共有の待ち合わせの手伝い）
- 本番側の競合（見つかった場合だけ）

## Out of Boundary
- 全体テストのスクリプト（`tools/test-all.ps1`）の並列度の変更
- `zorder-chain-residue` の間欠赤の族

## Upstream / Downstream
- **Upstream**: なし
- **Downstream**: 並走する全ての spec のレビューと完了の手順（赤の雑音が減る）

## Existing Spec Touchpoints
- **Extends**: なし（完了 spec の切り替え系のテストを触る）
- **Adjacent**: `zorder-chain-residue`（同じ「間欠赤」の型・別の族）

## Constraints
- sleep を足して直さない・1 フレーム遅らせる解を取らない（状態の持ち方で解く）。
- 実機の一時フォルダはワークツリーの `target\` の下だけ。

## 2026-10-05 `shell-balloon-frame-align` の完了前の観測

- 同じ族の赤をもう 1 回観測した。`cargo test -j 2 -p areka --bin areka`（全体・他のセッションと並走）で `install::desk::overwrite_tests` の 3 本と `ghost_session::switch_translate_tests` の 1 本が、`spin_wait_until`（30 秒）の期限切れで赤になった。どちらも足場 `ghost_switch_test_support.rs` を使うテスト。
- 2 つのモジュールだけを回し直すと、落ちたのは別の 2 本だった。`--test-threads=1` では 8 本すべて緑。直後の `tools/test-all.ps1`（`-j 4`）は全段緑。
- 対象の族に `install/desk_overwrite_tests.rs` も数える（足場を使うファイルは `git grep ghost_switch_test_support -- crates/areka/src` で 9 本）。

## 2026-10-05 `mcp-dump-images` の完了前の観測

- 他のセッションの cargo が 25〜35 本動く机で `tools/test-all.ps1` を回すと、同じ族の 4 本がときどき赤になった: `ghost_session_switch_fallback_tests.rs` の `default_ghost_fault_after_fallback_exits_through_shiori_fault_path`、`ghost_session_switch_tests.rs` の `script_change_tag_switches_a_to_b_and_reaches_steady`・`switch_to_b_without_boot_record_sends_first_boot_not_ghost_changed`、`emo2_boot/frame_ghost_quit_switch_tests.rs` の `stop_with_handoff_under_reservation_switches_without_exit`（4 本目も足場 `ghost_switch_test_support` を使う＝対象の族に数える）。
- 1 本ずつ流すと緑。`mcp-dump-images` の新しいテストを外した対照でも出た。静かな机の `tools/test-all.ps1` は全段緑（`b222af2e`）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S〜M（5〜8 タスク）。起票時の S（3〜6）より広い＝赤の族が増えた（下）。切らない（20 に遠い）。
- 前提の状態: 上流なし・今すぐ着手できる。C3 の 11 本の完了の全体テストのうち少なくとも 4 回で、この族の赤が出た（本文の観測 3 つと roadmap の覚え書き）。毎回「回し直して緑」で通しており、レビューの判定を濁す害は起票時より大きい。
- 崩れた前提／古くなった位置:
  - 足場 `ghost_switch_test_support` を使うファイルは本文の「9 本」でなく **28 本**（`git grep -l ghost_switch_test_support -- crates/areka/src`）。C3 で `mcp/get_property_tests.rs`・`mcp/get_expression_table_tests.rs`・`mcp/dump_surface_gpu_test_support.rs`（実際の GPU を使う）の 3 本が増えた。
  - 待ちの部品は 3 つ: `emo2_boot/spine.rs` の `spin_wait_until`（30 秒の期限・最初の 100 万回は `yield_now` の空回し）と `run_bounded`（呼び手が期限を渡す・spine の降ろしは 10 秒）、足場の `wait_steady`（`recv_timeout` 20 秒）。`pump_talking_until` は実時間 1 ms ごとに台詞の時計を 100 ms 進める Tick を注入する。
  - **`emo2_boot/spine.rs` はちょうど 1,000 行**（行数の番人 `file_length_guard_test.rs` は 1,000 を超えると赤）。待ちの部品を直すなら、最初に部品を別ファイルへ出す必要がある。
  - roadmap の覚え書きの `emo2_boot::spine` の 3 本（10-05・`run_bounded` の 10 秒で赤）は、`zorder-chain-residue` の A-2（spine の族）と同じ部品。本 spec で一緒に扱うのが自然（下の議題 1）。
- 見立て（要件の段で確かめる）: 期限が短いだけでなく、同じテストの実行ファイルの中で多数のテストが同時に `yield_now` の空回しで CPU を取り合う（`spin_wait_until` の doc 自身が「巻き添えの flake」を記録している）。負荷のときは自分たちで飢えを強めている見込み。
- 触るファイル: `crates/areka/src/emo2_boot/spine.rs`（待ちの部品を出す）・新規の待ちの部品のファイル（`emo2_boot/` の下）・`emo2_boot/ghost_switch_test_support.rs`・`ghost_session_switch_tests.rs`・`ghost_session_switch_fallback_tests.rs`・`ghost_session_restart_tests.rs`（10 秒の deadline）・`install/desk_overwrite_tests.rs`・`session_end_sync_send_tests.rs`・`emo2_boot/ghost_switch_boot_event_tests.rs`・`emo2_boot/frame_ghost_quit_switch_tests.rs`。部品の形を変えるなら足場を使う 28 本と `spin_wait_until` を呼ぶ 20 本に波及しうる。
- 議題:
  1. `zorder-chain-residue` の A-2（spine の族の壁時計の期限）を本 spec へ移すか。同じ `spine.rs` の部品を直すので、別々に走らせると同じファイルを取り合う。
  2. 直し方の向き: 期限を観測の待ちへ置き換える／期限切れの文言に「実時間で何秒待ったか・相手のスレッドが進んだか」を足して負荷と欠陥を見分けられるようにする、のどちらを本命にするか（両方なら規模は上の上限側）。
- 同時に走らせない: `areka-test-threads-av`（同じ `areka` のテストの実行ファイル・二分探索の測定を互いに汚す）。

### 棚卸㉒の裁定（2026-10-05）

- `zorder-chain-residue` の A-2 と、roadmap の覚え書きにあった `emo2_boot::spine` の 3 本（`run_bounded` の 10 秒の締切で赤・`drag-cancel-borrow-miss` の完了時の全体テスト）と、`install::desk::overwrite_tests` の 2 本・`sample-ghost-kit` の展開テストの os error 5（`target\nar-samples\work` の退避と同じ族と見る）を本 spec が引き取った。
- `spine.rs` はちょうど 1,000 行＝最初のタスクで待ちの部品（`spin_wait_until`・`run_bounded`）を別ファイルへ出す。
- `areka-test-threads-av` と同じウェーブに置かない（どちらもテストの土台）。

### 同じウェーブ C4 の約束（2026-10-05 棚卸㉒・破るなら止めて報告）

- 触るのは `emo2_boot/spine.rs`（待ちの部品を新しいファイルへ出す）・`ghost_switch_test_support.rs`・切替と起こし直しと上書きと終了のテストのファイルだけ。`emo2_boot/mod.rs`・`frame/`・`ghost_switch.rs` の本番の処理は触らない（`balloon-lifecycle-events`・`char-position-save-on-exit` の持ち物）。
