# Brief: areka-P0-test-wait-marker-gaps

> 起票: 2026-10-10（`areka-P0-ghost-session-test-load-flake` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票。`completed/areka-P0-ghost-session-test-load-flake/tasks.md` の Implementation Notes の「範囲外の申し送り」⑷⑹⑺・`load-repro.md` の 4.8 と、完了時の棚卸で見つけた 2 件）。コードの読みで確かめた。

## Problem

`ghost-session-test-load-flake` は、テストの待ちを待ちの部品（`crates/areka/src/emo2_boot/spine_wait.rs` の `wait_until` ほか）へ集め、負荷の下で許す赤の線を steering `tech.md` の「テストの待ちと、負荷の下で許す赤」に書いた。その線では、負荷の下でも待ちの文言の無い赤・`［進みは不明］`・`［進んではいた］` は許さない。ところが、まだ部品の形に揃っていない待ちが残っていて、負荷の下でそれらの赤が出うる、または許す赤と欠陥を見分けられない。

## Current State

2026-10-10 の本枝（`ghost-session-test-load-flake` の完了の直前）で数えた。

1. **目印が UI の側の仕事を数えない待ち**（申し送り ⑷）: `shell_balloon_switch_session_abort_tests.rs` の `ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` の 2 つの待ち（`lap.frames_until(CREEP, …)` と、捕捉の中の `lap.frames_until(UNBOUNDED, …)`）。目印は足場の `SwitchRig::progress_probe`（ゴーストを作った数と SHIORI の呼び出しの数）で、台詞の表示・B の定常・取りやめの記録を数えない。そのため `［止まった］` が実質 30 秒の壁時計の打ち切りになり、負荷と欠陥を見分けられない（`load-repro.md` の 4.8）。同じ足場の段の駆動器には、台詞の起動と表示の数を目印に足した前例がある（`spine_conformance_lap_tests.rs` の `progress_probe`）。
2. **`join_bounded` の目印が条件と同時にしか動かない**（申し送り ⑹）: spine の `join_bounded`（`spine_wait.rs`）と、その写しの `install/desk_pick_tests.rs`・`install/fetch_url_tests.rs` の `join_bounded`。目印は `is_finished`（0→1）で、条件と同じ時にしか増えないので、打ち切りは実質 30 秒の総時間で、文言は進み 0 回の `［止まった］` になる。`desk_pick`・`fetch_url` は素の World のテストで、ゴーストの足場でも GPU の足場でもない。負荷の下でここが赤になると、要件 6.1 の許す赤の範囲の字面に当たらない。写しが 3 つある（spine のは `ActorHandle`、install の 2 つは `JoinHandle<()>` を受ける）。
3. **`areka-nar` の短い掴みの檻**（申し送り ⑺）: `install_commit_tests.rs` の `a_destination_held_briefly_by_another_process_still_commits`。別のスレッドが約 200 ms 眠ってから掴みを放し、本番の `rename_patiently`（`install.rs`・os error 5・32 だけを 50 ms 刻みで約 2 秒まで試し直す）が待ち切ることを見る。負荷の下で放す側のスレッドの起動が大きく遅れると、2 秒を越えて揺れうる（`areka-nar` の実行ファイルには GPU のドライバの DLL が載らないので見込みは低い）。
4. **GPU の許可の待ちに上限が無い**: `GpuPermit::take`（`spine_wait.rs` の `GpuSlots::take`）は `Condvar::wait_while` で空きが出るまで眠り、打ち切りが無い。要件 3.3（どの待ちにも打ち切りの上限を持つ）の字面から外れている。許可を 2 つ取るテストが並ぶと互いに待ち合って止まりうることは、`GpuPermit` の説明に書いてある。
5. **目印の無い `spin_wait_until` の直接の呼び出し 15 か所**（`git grep -n "spin_wait_until(" -- crates/areka/src` から定義の 1 行を除いた数）。打ち切ると `［進みは不明］` になり、負荷の下では許さない赤になる。
   - spine の族 9 か所: `emo2_boot/spine_boot_smoke_tests.rs` 2・`spine_close_wiring_tests.rs` 1・`spine_conformance_support_tests.rs` 4・`spine_hold_tests.rs` 1・`spine_seriko_loop_tests.rs` 1
   - `session_end_deadline_tests.rs` 4（偽の SHIORI の `holding()`・`unblock_calls()` を待つ）
   - `mcp/dump_surface.rs` 1（`#[cfg(test)]` の `WaitAnswer::wait_answer`）
   - `ghost_session_restart_tests.rs` 1（`run_input_until` の中・作業プールの進みを数える口がテストの側に無いと説明にある）
   - あわせて見る所: 段の駆動器の目印 `StageSink::progress_probe`（`spine_conformance_support.rs`）は既定が `None`（`［進みは不明］`）で、上書きしているのは lap の足場だけ。`SpineHarness` で段を回す `spine_conformance_support_tests.rs` の檻は目印なしで待つ。

## Desired Outcome

- 上の待ちがすべて、steering `tech.md` の線のとおり、負荷の下で出る赤が許す赤（`［止まった］`）か、待ちの文言で欠陥と読み分けられる形になる。`［進みは不明］` を出しうる待ちが `crates/areka/src` のテストに残らない（残すなら理由を記録した例外にする）。
- 2 の `desk_pick`・`fetch_url` は、目印を良くするか、開発者の裁定で許す赤の範囲を広げるかのどちらかで決着する（議題）。
- 3 の檻が負荷で揺れない形になる（本番の `rename_patiently` の振る舞いは変えない）。
- 4 の待ちに上限が付き、打ち切りは文言で分かる。
- 静かな机と `tools/test-all.ps1` は今までどおり緑。所要時間を大きく延ばさない。

## Approach

- 1 は足場の目印に UI の側の仕事（台詞の起動・表示・定常の知らせなど）を足す。lap の段の駆動器の前例に合わせる。`LapRig::frames_until` はほかの lap のテストも使うので、目印を変える影響を全部の呼び手で見る。
- 2 は、`join_bounded` が待つ相手の進み（スレッドの中の仕事の数など）を目印にできるかを先に調べ、できなければ開発者に範囲の広げ方を諮る。写し 3 つを 1 つにまとめるかもここで決める。
- 3 は、放す時機を時刻でなく「1 度拒まれた」ことの観測に結ぶなど、待ちに頼らない形を探す。本番の試し直しの間隔と上限は変えない。
- 4 は、許可の待ちを待ちの部品へ載せるか、`wait_timeout_while` で上限と文言を付ける。
- 5 は、相手の進みを数える口が在る所（偽の SHIORI の呼び出しの数など）から目印つきの部品へ移す。口の無い所は、口を足すか、理由つきの例外として記録する。
- 確かめは `tools/load-flake.ps1` の負荷の再現で行う。全体の回は記録の最初と最後だけにし、2 回目からは赤が出たテストに絞って短く回す。回す前に所要時間を伝え、静かな机で回す。

## Scope

- **In**: 上の 1〜5 の待ちと、そのための足場・偽の SHIORI の観測口の追加・檻・負荷の再現の記録。
- **Out**: 本番のコードの振る舞い（`rename_patiently`・`GpuPermit` の数を含む）・許す赤の線そのものの書き換え（2 の裁定で広げる場合を除く）・テストの確かめの中身。

## Boundary Candidates

- 待ちの部品（`spine_wait.rs`）と足場（`ghost_switch_test_support.rs`・lap の足場）
- spine の族のテストファイル・`session_end_deadline_tests.rs`・`mcp/dump_surface.rs` のテストの部分・`ghost_session_restart_tests.rs`
- `install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`
- `areka-nar` の `install_commit_tests.rs`

## Out of Boundary

- 本番のファイル（`#[cfg(test)]` の部分を除く）
- 許す赤の線の理由の書き換え

## Upstream / Downstream

- **Upstream**: `ghost-session-test-load-flake`（✅ 2026-10-10 完了・待ちの部品と許す赤の線）。
- **Downstream**: なし（以後のテストは steering の線に従って書く）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-ghost-session-test-load-flake`（待ちの部品・要件 3.3・6.1・steering `tech.md` の線）。
- **Adjacent**: `areka-test-threads-av`（同じテストの実行ファイル `--bin areka` を回す＝同時に走らせない）・`test-roots-under-target`（同じテストを書き換える・同時に走らせない）・`actor-thread-log-capture`（背景のスレッドの記録が赤の文言に出れば読み分けが楽になる）・`emo2-real-run-wrap-timeout`（壁時計の待ちを観測の待ちへ置き換える方針が同じ）。

## Constraints

- `areka-test-threads-av` と同時に走らせない（同じテストの実行ファイル）。
- 負荷の再現は静かな机で回す。高負荷のテストを何度も回さない（全体は記録の最初と最後だけ・2 回目からは赤のテストに絞る・回す前に所要時間を伝える）。
- GPU のテストは実の GPU のまま（WARP などへ替えない）。sleep を足す・締切を延ばすだけ・1 フレーム遅らせる形では直さない。
- 1 ファイル 1,000 行の番人を守る（`spine_wait.rs` に足すなら行数を先に測る）。
- 規模の見立て: S〜M（6〜10）。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 起票（10-10・着地の直前の枝）の数は main でも同じ。目印の無い `spin_wait_until` の直接の呼び出しは 15 か所（spine の族 9・`session_end_deadline_tests.rs` 4・`mcp/dump_surface.rs` 1・`ghost_session_restart_tests.rs` 1）。`areka` の中の `join_bounded` の写しは 3 つ（`emo2_boot/spine_wait.rs`・`install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`）。GPU の装置の許可の待ち（`spine_wait.rs` の `GpuSlots::take`）は今も上限なし。崩れた前提は無い。
- **触るファイル**（どれも `crates/areka/src/` の下・テストと足場だけ）: `emo2_boot/spine_wait.rs`（462 行）・`spine_wait_tests.rs`・`ghost_switch_test_support.rs`（672）・`spine_conformance_support.rs`（773）・`spine_conformance_support_tests.rs`（**959 行＝上限の近く**。目印つきへ移すと増えるので先に分ける）・`spine_boot_smoke_tests.rs`・`spine_close_wiring_tests.rs`・`spine_hold_tests.rs`・`spine_seriko_loop_tests.rs`・`shell_balloon_switch_session_abort_tests.rs`（536）・足場の目印を変えるなら `shell_balloon_switch_session_lap_tests.rs`（791）・`session_end_deadline_tests.rs`（523）・`ghost_session_restart_tests.rs`・`mcp/dump_surface.rs`（テストの部分だけ）・`install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`。ほかに `crates/areka-nar/src/install_commit_tests.rs`（802）。`emo2_boot/spine_conformance_lap_tests.rs` は 911 行。
- **規模**: M（8〜11）。待ちの種類 5 つ＋負荷の記録。切らない。
- **仕事の芯は書き換え**で、測定は確かめのためだけ。項目 2・3・5 は負荷なしで直せる（打ち切りの文言は時計を注入した檻で見る。静かな机の全体は 1 回 約 70 秒）。重い回は 2 度だけにする: 途中は赤が出たテストの名前で絞って回し（`tools/load-flake.ps1 -Filter`・1 回 数分）、全体の負荷つき 5 回（約 50 分）は最後に 1 度。回す前に所要時間を伝える。
- **先に要るもの**: 働きの依存は済み（`ghost-session-test-load-flake`）。`areka-test-threads-av` の後に置く（同じテストの実行ファイル・足場のファイルが重なる・あちらの落ちが負荷の回を無効にする）。ほかのファイルの重なり: `install-live-target-hazards`（`session_end_deadline_tests.rs`・`areka-nar` の `install.rs` の近く）・`mcp/dump_surface.rs` を触る MCP の spec・`test-roots-under-target`（呼び出しを 1 つずつ書き換える案のときだけ `install/fetch_url_tests.rs` ほか）。
- **優先度の区分**: B（バグ＝テストの穴。負荷の下で、許さないと決めた赤が出うる）。
- **要件定義のモデル**: Fable（待ちと並行・項目 2 に開発者の裁定の分かれ目がある）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 同じ名前の `join_bounded` が、別の実行ファイルのテストにもある（`crates/areka-ghost/tests/ghost/` の 3 本〔接続の失敗・helper の生死・切断〕と `crates/areka-kanade/tests/kanade/common/` の 2 本）。待ちの部品は `areka` の実行ファイルの中にしか無いので、これらは steering `tech.md` の線の外に居る＝範囲に入れるかを要件で決める。`GpuPermit` の説明は「許可を 2 つ取るテストは 1 本だけ」と書く＝上限を付けるとき、その 1 本が待ち合いで打ち切られる形も檻に入れる。
