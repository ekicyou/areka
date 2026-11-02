# Brief: areka-test-wait-wording

> 2026-11-02 起票（`areka-test-binary-stability` の完了の棚卸・`/kiro-discovery` の Path C）。同 spec の要件 7.6 の決め「文言の無い待ちは数えて記録するだけ・本 spec の走行で赤になったものだけ移し、残りは完了のときに 1 本の起票にまとめる」に沿って、残りの待ちを全部この 1 本にまとめた。待ちの一覧の正本は `.kiro/specs/completed/areka-test-binary-stability/load-repro.md` の ⑵（数えたコミット `66f91513`・2026-11-01）。行の番号はそのときのもの＝着手時に引き直す。

## Problem

`crates/areka/src` のテストに、待ちの部品（`emo2_boot/spine_wait.rs` の `wait_until`・`wait_recv`・`run_bounded_watching`）を通らない待ちがまだ残っている。こうした待ちが負荷の下で打ち切られると、赤の文に「相手が止まっていたのか、進んではいたのか」が出ない。そのため、負荷の下で許す赤（steering `tech.md` の「テストの待ちと、負荷の下で許す赤」）と本当の欠陥を、赤の文だけでは読み分けられない。

## Current State

`load-repro.md` の ⑵ で数えた残り（あわせて 31 か所）:

- **自前の `recv_timeout` の締切 13 か所**（`git grep -n "recv_timeout(" -- crates/areka/src` の 17 件から、待ちの芯の中の 2 件〔`spine_wait.rs` の `wait_recv`・`run_bounded`〕と本番の `perf_thread_report.rs` の 2 件を除いた数）: `emo2_boot/film_playback_e2e_tests.rs` 290・`emo2_boot/frame/drain_resnap_tests.rs` 111・`emo2_boot/frame_scale_notice_integration_tests.rs` 254・`emo2_boot/install_cue_tests.rs` 263・`ghost_session_reload_tests.rs` 45・`ghost_session_runtime_tests.rs` 51・`install/desk_pick_tests.rs` 155・160・`install/fetch_url_tests.rs` 180・`mcp/check_script_tests.rs` 205・`mcp/get_expression_table_tests.rs` 729・`roots/desk_tests.rs` 155・160。
  - このうち `drain_resnap_tests.rs` の 111 は、`areka-test-binary-stability` のタスク 6.5 で `wait_recv` へ移し済み（同 spec の走行で赤になった 1 つ）＝着手時に数え直して外す。
- **`Instant::now() +` の締切 14 か所**（18 件から、本番の起こしの予約 2 件〔`emo2_boot/balloon_visibility_phase.rs`・`input_events/char_watch.rs`〕と、待たずに締切の値を作るだけの 2 件〔`menu/trigger_show_tests.rs`・`menu/trigger_tests.rs`〕を除いた数）: `emo2_boot/frame/switch_tests.rs` 641・`emo2_boot/frame_ghost_quit_logsink_tests.rs` 78・`emo2_boot/spine_close_wiring_tests.rs` 113・166・`emo2_boot/spine_display_tests.rs` 30・`emo2_boot/spine_move_cue_tests.rs` 117・`emo2_boot/spine_seriko_loop_tests.rs` 58・88・191・452・`emo2_boot/spine_talk_close_tests.rs` 262・`emo2_boot/spine_third_scope_talk_tests.rs` 241・`readme/opener_submit_tests.rs` 198・`thread_roles_tests.rs` 26。
- **総時間で打ち切る古い呼び名 `run_bounded` の呼び手 1 か所**: `session_end_sync_send_tests.rs` 205（もう 1 件の `emo2_boot/spine_wait_tests.rs` 310 は `run_bounded` そのものの檻なので数えない）。
- **`Progress::Unknown` で待つ 3 か所**（`areka-test-binary-stability` の要件 7.1 の数え直しで漏れていたもの・同 spec の ⑵ の 45 行目）: `emo2_boot/readme_script_e2e_tests.rs` 71（`wait_until("OnBoot の Leave", Progress::Unknown, ...)`）・`emo2_boot/script_security_e2e_tests.rs` 110（`wait_until("OnBoot の中断の合図", Progress::Unknown, ...)`）・`install/desk_overwrite_tests.rs` 244（`run_bounded_watching("起動の直後の記憶の書き込み", Progress::Unknown, ...)`）。前の 2 つは待ちの中で dispatcher へ Tick を注入して合図の受け手を読む形、後の 1 つは書き手のフェンスを 1 回呼ぶだけの形。目印を渡せる口があるかはまだ見ていない。
- `areka-test-binary-stability` の走行（静かな机の全体テスト・負荷つきの 5 回）で、ここに挙げた待ちが赤になったことは無い。
- 既に決まっている例外（要件 7.3・この spec では動かさない）: `mcp/check_script_tests.rs` の `dispatch_and_wait`（`Progress::Unknown`）と、`spine_conformance_support_tests.rs` の `stage_driver_returns_named_timeout_instead_of_silent_success`（`FakeStageSink::silent_unmarked`）。

## Desired Outcome

- 上の待ちが、待ちの部品（`wait_until`・`wait_recv`・`run_bounded_watching`）と目印（`Progress::Count`）を通る。負荷の下で赤になったとき、文言の `［止まった］`・`［進んではいた］`・`［進みは不明］` で、許す赤か欠陥かを読み分けられる。
- 目印を持てない待ちは、理由を書いた例外として記録する（`areka-test-binary-stability` の要件 7.3 と同じ形）。
- 移した後、`git grep` で数え直した数（自前の `recv_timeout`・`Instant::now() +` の締切・`run_bounded(` の呼び手・`Progress::Unknown`）が、例外の数と一致する。
- 静かな机の `tools/test-all.ps1` は今までどおり緑。所要時間を大きく延ばさない。

## Approach

1. 着手時に `load-repro.md` ⑵ のコマンドで数え直す（`drain_resnap_tests.rs` の 111 のように移し済みのものを外す）。
2. 1 か所ずつ、待つ相手の進みを数える口を探す（偽の SHIORI の `call_count()`・受けた指令の数・届いた閉包の数など）。`areka-test-binary-stability` のタスク 6.5 の前例（`spine_*` の族は `call_count()`・seriko の輪は受けた指令の数・作業プールの見張りはテストの側に足した `drain_counting`）に合わせる。
3. 口がある所は `wait_until`／`wait_recv`／`run_bounded_watching` と `Progress::Count` へ移す。`run_bounded` の呼び手は `run_bounded_watching` へ移す。
4. 口が無い所は、テストの側に口を足せるかを見て、足せなければ理由つきの例外として記録する。
5. 確かめは静かな机の全体テスト 1 回と、移したテストに絞った短い回し直し。高負荷の全体の走行は回さない（この spec は赤を直すのでなく、赤の文を読めるようにする仕事）。

## Scope

- **In**: 上の 31 か所（着手時の数え直しの結果）の待ちの移し替え・目印のためにテストの側に足す口・例外の記録。
- **Out**: 本番のコードの振る舞い・待ちの部品そのもの（`spine_wait.rs` の `wait_until`・`wait_recv`・`run_bounded_watching`・`Progress` の形と文言）・許す赤の線の書き換え・既に決まった例外 2 つ・テストの確かめの中身。

## Boundary Candidates

- spine の族のテスト（`emo2_boot/spine_*_tests.rs`）
- frame の族と GPU の結合テスト（`emo2_boot/frame/*_tests.rs`・`frame_*_tests.rs`・`film_playback_e2e_tests.rs`・`install_cue_tests.rs`）
- ghost_session・session_end のテスト（`ghost_session_reload_tests.rs`・`ghost_session_runtime_tests.rs`・`session_end_sync_send_tests.rs`）
- install・roots・mcp・readme のテスト（`install/desk_pick_tests.rs`・`install/fetch_url_tests.rs`・`install/desk_overwrite_tests.rs`・`roots/desk_tests.rs`・`mcp/check_script_tests.rs`・`mcp/get_expression_table_tests.rs`・`readme/opener_submit_tests.rs`）
- 起動の e2e と thread の役のテスト（`emo2_boot/readme_script_e2e_tests.rs`・`emo2_boot/script_security_e2e_tests.rs`・`thread_roles_tests.rs`）

## Out of Boundary

- 本番のファイル（`#[cfg(test)]` の部分を除く）。
- 待ちの部品 `emo2_boot/spine_wait.rs` の中身（呼ぶだけ）。
- 許す赤の線の理由の書き換え。

## Upstream / Downstream

- **Upstream**: `areka-test-binary-stability`（✅ 2026-11-02・要件 7.6 の決めと待ちの一覧・6.4〜6.5 の移し替えの前例）・`ghost-session-test-load-flake`（✅ 10-10・待ちの部品と許す赤の線）。
- **Downstream**: なし（以後のテストは steering の線に従って書く）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-test-binary-stability`（要件 7.3・7.6・`load-repro.md` の ⑵）・完了 `ghost-session-test-load-flake`（待ちの部品・steering `tech.md` の線）。
- **Adjacent**: 今まさに着地しようとしている spec は無い。`instant-to-interrupt-ticks` は本番の時計を割り込みの時刻へ寄せる spec で、テストの待ちの上限の `Instant::now()` は番人の外と決めている＝重ならない。

## Constraints

- 段は**バグ**・区分 B（テストの偽の赤を読み分けるため）。版に縛らない。
- 本番のコードの振る舞いは変えない。待ちの部品は今の形のまま使う。
- sleep を足す・締切を延ばすだけの形では直さない。GPU のテストは実の GPU のまま（WARP などへ替えない）。
- 高負荷のテストを何度も回さない（回す前に所要時間を伝える）。
- 1 ファイル 1,000 行の番人を守る（`spine_conformance_support_tests.rs`・`spine_conformance_lap_tests.rs` などの近い所は、行が増えるなら先に測る）。
- 規模の見立て: S〜M（6〜10 タスク・ファイルの族ごとに 1〜2）。
- 要件定義のモデル: Opus（前例どおりの移し替え・新しい決まりを作らない）。
- **優先度の区分**: B（テストの偽の赤を読み分ける・負荷の下で赤になったときに、許す赤か欠陥かが文言で分かるようにする）。版に縛らない。
