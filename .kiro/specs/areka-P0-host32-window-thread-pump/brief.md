# Brief: areka-P0-host32-window-thread-pump

> **起票 2026-09-11**（`/kiro-discovery`・開発者「起票候補はすべて起票せよ」）。出所: `areka-P0-emo2-conformance-e2e` の症状 D（解放の IPC 失敗・2026-09-07）の調査で分かった構造。登記: 同 spec `verification/acceptance-record.md` §13.2 行 10・requirements「改訂（2026-09-07・第 4 回）」・Requirement 16.5・design D16。**実害は 6.11（`SendFlavor::Response`・`ce545350`）で塞がれており、走行では発現しない。**

## Problem

32bit の脳（`shiori-host32-helper.exe`）と WM_COPYDATA で話す**ホスト窓**は SHIORI アクターのスレッドに在るが、そのスレッドは入力待ち（`crates/areka-kanade/src/shiori/real.rs:179` の `rx.recv()`）で止まり、要求の往復と起動時の握手以外ではウィンドウメッセージを取り出さない。Windows は「プロセス開始から 20〜30 秒を過ぎ、かつ 14 秒以上メッセージを汲まない窓」を応答なし（hung）と判定するので、その窓へ `SMTO_ABORTIFHUNG` 付きで送る側は即失敗する。6.11 は helper の**応答方向**から旗を外して実害を消したが、「窓を持つスレッドがメッセージを汲まない」構造は残っている。

## Current State

- ホスト窓の生成と待ち: `crates/shiori-host32-host/src/parent_window.rs`（pump フェーズ専用の heartbeat＝別スレッドから `WM_NULL` を撃って `GetMessage` を起こす仕組みが在る・`:12`／`:49`／`:255-267`）。定常時は `rx.recv()` が占有し、pump は走らない。
- helper 側は要求方向に `SMTO_ABORTIFHUNG` を残している（応答方向だけ外した・`crates/shiori-host32-ipc/src/lib.rs` の `SendFlavor`）。要求方向は areka → helper なので helper の窓が汲んでいれば問題ないが、逆向きの将来の経路（helper → areka の自発通知など）は同じ構造に当たる。
- e2e の計測: 終了挨拶中は kanade が `OnSecondChange` を止めるため、ホスト窓スレッドが 19 秒無応答になる（記録 §13.2 行 8／10）。

## Desired Outcome

1. SHIORI アクターのスレッドが、入力（`rx`）と窓のメッセージを**同時に**待つ（`MsgWaitForMultipleObjectsEx`＋`PeekMessage` のループ、または窓を専用スレッドへ分けて `rx.recv()` から切り離す）。どちらでも「20 秒以上何もしなくても hung にならない」を決定論の檻（既存の `main_response_flavor_hung_cage_tests.rs` の形＝20 秒待ち → 往復 → 20 秒待ち → 往復）で示す。
2. 6.11 の応答方向の旗の扱いは維持（二重の守り）。
3. `zorder-chain-residue` の A 系（wintf／host32 の常設の飢餓）と整合し、間欠的な赤を増やさない。

## Approach

- 案 1: `real.rs` の受信ループを `MsgWaitForMultipleObjectsEx(1, [rx の起こしイベント], INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE)` に置き換え、起きたら `PeekMessage` で窓を汲んでから `rx.try_recv()`。crossbeam/std の channel は待機ハンドルを出さないので、送信側に `SetEvent` を足す薄い包みが要る。
- 案 2: 窓を専用スレッドへ移し、WM_COPYDATA の中身を channel で SHIORI アクターへ渡す。構造は素直だがスレッドが 1 本増え、応答の往復に 1 ホップ足す。
- 設計で選ぶ（推奨は案 1・変更が 1 か所）。

## Scope

- **In**: `crates/areka-kanade/src/shiori/real.rs`（待ちの形）・`crates/shiori-host32-host/src/parent_window.rs`（pump の常設化）・決定論の檻（hung 判定の較正つき）。
- **Out**: IPC の方式（WM_COPYDATA 一本化は不変）・helper 側の実装（応答方向の旗は 6.11 のまま）。

## Boundary Candidates

- 待ちの合流（アクター）／窓の pump（host32）。1 spec・S〜M。

## Out of Boundary

- kanade の起動系列（`kanade-boot-talkdone-drop`）・終了の握手の配線。

## Upstream / Downstream

- **Upstream**: `areka-P0-emo2-conformance-e2e`（症状 D の調査・6.11 の檻）・`areka-host32-ipc-and-i686-build`（WM_COPYDATA 一本化の決定）。
- **Downstream**: helper → areka 向きの自発通知を要する将来の spec。

## Existing Spec Touchpoints

- **Extends**: `shiori-host32-*`（完了 spec の IPC）。
- **Adjacent**: `zorder-chain-residue`（A 系の飢餓）・`emo2-conformance-e2e`。

## Constraints

- 決定論の檻は x64 の偽境界で（i686 は常時テストに載せない・steering）。実機の確認は e2e 手順書 §5.7 の `unload_failed` 0 行で足りる。
