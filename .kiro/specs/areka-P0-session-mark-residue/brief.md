# Brief: areka-P0-session-mark-residue

> 2026-09-27 棚卸⑱起票（`/kiro-discovery` 再入）。`areka-P0-ghost-shell-balloon-switch`（PR#192・裁定 13＝起動中の印）の完了で残った 3 つの穴を 1 本にまとめた。うち 2 つは roadmap の「登記だけの行」（「OS のセッションの終了では SHIORI の待ちを短くする」「起動がまるごと失敗すると起動中の印が消える」）から格上げ、1 つは完了 spec の `signoff.md`「気付いたこと」8 に残っていた未確認の申し送り（引受先 0）。段は **バグ**（潜在）。

## Problem

裁定 13（2026-09-27）で areka は「ゴーストを起こしたら記憶に印を書き、きれいに終わったときだけ消す。次の起動で印が残っていれば前回は落ちた＝既定ゴースト emo2 を `OnBoot` の Reference6＝`halt`・Reference7＝落ちたゴースト名で起こす」形になった。この「きれいに終わった」の判定と、Windows のシャットダウン／ログオフのときの後始末に、次の 3 つの穴が残っている。

1. **OS の終了で SHIORI を待ちすぎる**。SHIORI が固まっていると、`WM_ENDSESSION` を受けた後始末（`crates/areka/src/session_end.rs` の `on_os_session_end`）が IPC の期限まで止まる。待ちは `OnClose` の NOTIFY（`crates/shiori-host32-host/src/client.rs` の実効の期限＝既定 60 秒・`AREKA_SHIORI_REQUEST_TIMEOUT_MS`）＋UNLOAD の応答（`lifecycle.rs` の `UNLOAD_ACK_TIMEOUT`＝30 秒）＋終了の観測（同 `EXIT_OBSERVE_TIMEOUT`＝10 秒）で**最悪約 100 秒**。利用者から見ると「このアプリがシャットダウンを妨げています」が出て、Windows に打ち切られる。打ち切られると印が残り、次の起動は emo2＋`halt` になる（中身は正しいが、利用者は何も壊していない）。**開発者の決め（2026-09-27）＝OS のセッションの終了のときだけ SHIORI を待つ期限を短くする。切替とメニューの終了の期限は変えない。**
2. **起動が LogSink へ倒れると印が消える**。最初の起動は `crates/areka/src/ghost_session.rs` の `boot_ghost` が窓への結線（`boot_wired`）に失敗すると LogSink の起動へ倒れる（完了 `emo2-boot` 以来の非致命の契約）。倒れた先が失敗しても成功しても、窓の見えないゴーストが残り、その後 OS のサインアウトや `WM_CLOSE` できれいに終えると `crates/areka/src/main.rs` の `session_mark_verdict` が「きれいな終わり」と判定して印を消す。次の起動は同じゴーストで同じ失敗を繰り返し、`halt` は付かない＝裁定 13「きれいに終わらなければ次は emo2」に反する。
3. **OS の終了の受け手が、降ろす処理の join の間メッセージを配らない（未確認）**。`on_os_session_end` の中の `GhostSession::shutdown` の join は、送られてきたウィンドウメッセージを配らない。受け手の処理中に別スレッドから UI の窓へ同期の送信（`SendMessage`）が重なると止まりうる。完了 spec の実機確認（`signoff.md` の ⑪・所要 22 ms）は 1 通ずつ返りを待って送ったので、重なる形が起きておらず確かめていない。

## Current State

- 印の書き込みと判定: `main.rs` の `session_mark_verdict`（`ExitOrigin` の網羅 match）・`after_run`。テスト表は `main_session_mark_tests.rs` の `session_mark_verdict_table`。
- 最初の起動: `ghost_session.rs` の `boot_ghost`（結線あり → 失敗なら LogSink）。切替は倒れ込みを持たない `boot_ghost_strict` を通るので、この穴は踏まない。
- OS の終了: wintf の `OnSessionEnd` 部品（`crates/wintf/src/ecs/window/components.rs`）と `WM_ENDSESSION` の腕 → `session_end.rs` の `on_os_session_end` → `GhostSession::shutdown` → `areka-ghost/src/runtime.rs` の `GhostRuntime::shutdown` → kanade の強制終了（`schedule/mod.rs` の `force_quit`・`shiori/real.rs`）→ `shiori-host32-host` の `client.rs`・`lifecycle.rs`。期限を運ぶ経路は無い（すべて定数か環境変数）。
- α の第三者の一周（zip 展開 → 起動 → `.nar` の投げ込み → 切替 → 更新 → 終了 → 再起動）では、3 つとも踏まない。1 は SHIORI が固まったまま Windows を終了したとき、2 は前に動いたゴーストのファイルが外で壊れたときか同梱の emo2 が壊れたときに踏む。

## Desired Outcome

1. OS のセッションの終了のときだけ、SHIORI を待つ期限の合計が OS の猶予より十分短い（値は要件で決める）。期限を超えたら SHIORI を待たずに後始末を終え、印の扱いは「きれいに終わらなかった」に倒す（次の起動は emo2＋`halt`）。切替・メニューの終了・強制退避の期限は 1 つも変わらない。
2. 最初の起動が LogSink へ倒れたとき、その後どう終わっても印が残る（次の起動は emo2＋`halt`・Reference7＝倒れたゴースト名）。LogSink の起動が成功した場合も失敗した場合も同じ。
3. 3 の形（join 中に別スレッドから同期の送信が重なる）を再現して、止まるか止まらないかを確かめる。止まるなら直す（join の間メッセージを配るか、同期の送信を非同期に替えるか）。止まらないなら、その理由を記録に残す。
4. どれも決定論のテストで固定する（1 は期限の受け口に偽の時計・偽の SHIORI、2 は `session_mark_verdict` の表に行を足す、3 は再現のテスト）。

## Approach

- 1: 「OS の終了」の出所（`ExitOrigin` の該当の腕）から、降ろす処理へ**期限の上限**を 1 つ渡す。kanade の強制終了と `shiori-host32-host` の NOTIFY／UNLOAD／終了の観測は、その上限と自分の既定の期限の短い方を使う。定数を書き換えるのではなく、呼び手が上限を渡せる口を足す（他の経路は上限なし＝今のまま）。
- 2: 台帳の 2 案のうち「LogSink へ倒れたこと自体を印を残す理由にする」を推す（倒れた先が成功した場合も塞げる）。もう 1 案「LogSink の起動の失敗を終了の出所に載せる」は、倒れた先が成功した場合を塞げない。
- 3: 先に再現のテストを書く。止まらなければ理由の記録だけで閉じる。
- 3 つとも `session_mark_verdict`（`ExitOrigin` の網羅 match）を必ず通す（`ghost-shell-balloon-switch` の申し送り）。

## Scope

- **In**: OS の終了のときだけの期限の上限と、その運び道（areka → areka-ghost → kanade → shiori-host32-host）／LogSink へ倒れた起動の印の扱い／join 中の同期の送信の再現と、要れば是正／それぞれの決定論テスト／`doc/COMPAT_ARCHITECTURE.md` §8 への 1〜2 行。
- **Out**: 切替・メニューの終了・強制退避の期限の変更／起動中の印の方式そのもの（裁定 13 は変えない）／登記だけの行「壊れたゴーストを表示し続ける」（SSP 忠実の切替失敗の形）／emo2 の `halt` の台詞（開発者が辞書に足す・`alpha-release-signoff` の前提 4）／main スレッドの panic で後始末を通らない件（方針「panic は致命の場合だけ」と整合・棚卸⑰で捨てた）。

## Boundary Candidates

- 期限の運び道（OS の終了の出所 → 降ろす処理 → kanade → host32）
- 起動の倒れ込みと印の判定（`boot_ghost` の LogSink の腕・`session_mark_verdict`）
- OS の終了の受け手の中の join（wintf の `WM_ENDSESSION` の腕・`on_os_session_end`）

## Out of Boundary

- `ghost_switch.rs`（切替の経路）・`emo2_boot/change_cue.rs`・メニュー・`areka-parsers/src/sakura/`＝`ghost-change-name-resolution` と並走するため触らない。
- kanade の `events.rs`（許可表）・`msg.rs` の新しい変種＝`shell-balloon-switch`・`ghost-install`・`network-update` が動かす。

## Upstream / Downstream

- **Upstream**: 完了 `ghost-shell-balloon-switch`（起動中の印・`session_mark_verdict`・`on_os_session_end`）・完了 `shiori-fault-notice`（`finish_after_run`・Fault の終了コード 1）・完了 `ghost-restart-unit`（`GhostSession::shutdown`）・完了 `host32-window-thread-pump`。
- **Downstream**: `shell-balloon-switch`・`ghost-install`・`network-update`（どれも `ghost_session.rs`・`main.rs` を触る＝本仕様の後に建つ）。`alpha-release-signoff` の既知の制限の候補「SHIORI が固まったまま Windows を終了した場合」は本仕様の完了で消える。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec は書き換えない。裁定 13 の読みを変える箇所が出たら §8 に書く）。
- **Adjacent**: `ghost-change-name-resolution`（同じウェーブで並走・触るのは `emo2_boot/ghost_switch.rs` とその兄弟テスト・`areka-parsers/src/sakura/decode.rs` だけ＝共有 0）。

## Constraints

- 想定の触るファイル: `crates/areka/src/{session_end,main,ghost_session,app_exit}.rs`・`main_session_mark_tests.rs`・`crates/areka-ghost/src/runtime.rs`・`crates/areka-kanade/src/schedule/mod.rs`・`crates/areka-kanade/src/shiori/real.rs`・`crates/shiori-host32-host/src/{client,lifecycle}.rs`・要れば wintf の `WM_ENDSESSION` の腕（`crates/wintf/src/ecs/window/` 以下）。**`emo2_boot/ghost_switch.rs`・`areka-parsers/` には触らない**。
- 規模: S（7〜10 タスク・1 は 4〜6・2 は 2〜3・3 は 1〜2）。20 を超えない。
- 1 の値は開発者が決める（Windows の猶予は既定で数秒〜20 秒程度。要件で ukadoc `OnClose` の Reference0＝`system` の意味と合わせて議題にする）。
- ログ無しの失敗経路は作らない（期限切れは `warn!` 以上で残す）。
- Fable: −（議題は開発者の決めごと 2 件＝期限の値・倒れ込みの扱いの 2 案。答えで作業が大きく分かれない）。
