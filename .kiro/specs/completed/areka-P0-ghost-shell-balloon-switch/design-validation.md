# 設計レビュー — areka-P0-ghost-shell-balloon-switch

> 実施: 2026-09-26・非対話（kiro-validate-design・報告のみ）。対象は `design.md`（2026-09-26 生成）・`requirements.md`・`research.md` §8〜§11・steering。設計が名指しした定義はすべて本ブランチのソースで突き合わせた（`frame.rs` の `run_ghost_quit_phase`／`KanadeStopRx`・`ghost_session.rs` の `boot_ghost`／`reopen_ghost_windows`／`open_ghost_windows`・`app_exit.rs` の `close_windows_for_restart`／`fault_of`／`ExitOrigin`・kanade `schedule/mod.rs` の `step`／`on_talk_done`／`dispatch_phase`／`awaits_reply`・`close.rs` の `deadline_from`・`boot.rs` の `on_prefetch_reply`・`steady.rs` の `on_reply`・`events.rs` の `ALLOWED_EVENT_IDS`（11 語）・`actor.rs` の `drive`／`stop_cause_of`／`notify_stop`・`msg.rs` の `KanadeConfig::new`（構造体リテラル 1 か所）・`emo2_boot/mod.rs` の `wire_emo2_boot`・`boot_resolve.rs` の `GhostRoute`／`LastUsed::record`・`alert.rs` の `AlertScene`（5 場面）・`consumer_ledger.rs` の `canonical()`（8 組）・`menu/mod.rs`・`readme_cue.rs`・`persist/mod.rs` の `PersistKey`・行数）。要件 11 の裁定 1〜12 は確定事項として扱い、設計がそれを守っているかだけを見た。

## レビュー要約

設計は既存の形（登録 1 回・載せ替え n 回・停止通知の受け口・`close.rs` の写し・`*_cue.rs` の 2 段・`wire_*`／`register_*_drain` の対）へ腕を足す拡張で、依存の向き（talk → kanade → ghost → areka）を崩さず、裁定 1〜12（初回起動最優先・既定へ戻す・Ref6/7・告知なし・既定の失敗は致命・単独起動の失敗も既定へ）を `boot_root`／`BootOrigin::Halted`／`switch_to_default`／`fatal`／`record_halt` で忠実に写している。コードに対する記述の食い違いは 0 件（行数・変種数・構造体リテラルの数・`GhostWindows` を外していないこと・`ForceQuit` の空振り・`pending_close` の消化点まで一致）。書き方の規律（日本語・定義名で指す・spec 名で呼ぶ・禁じた語なし・1 フレーム遅らせる解なし・失敗経路の記録）も満たす。残る問題は Flow 3 の失敗経路 1 本と、目印の判定の穴 1 つ、要件 6.8 の適用範囲の 1 点で、いずれも構造を変えずに設計討議で閉じられる。

## Critical Issues

### 🔴 Critical Issue 1: 切替先の起動（⑤）が同期で失敗すると、切替先の窓と既定ゴーストの窓が両方生える

**Concern**: `switch_to` の順序は ④ `reopen_ghost_windows` → ⑤ `boot_ghost_strict`。`open_ghost_windows` は配置の準備だけを同期で行い、窓の spawn は `WintfTaskPool` へ渡した閉包が**次の `Input` 段**で行う（`ghost_session.rs` の `open_ghost_windows`・`task_pool.spawn(|tx| …)`）。⑤ が失敗した時点で切替先の窓はまだ無いので、続く `switch_to_default` の「切替先を降ろして窓を閉じる」（`close_windows_for_restart`）は 0 枚を閉じ、既定ゴースト用に ④ をもう 1 度呼ぶ。次の `Input` 段で 2 つの閉包が順に走り、切替先の窓（4 枚）が spawn されたあと既定の窓が spawn されて `GhostWindows` を上書きする。切替先の窓は `GhostWindows` から外れた孤児の `GhostWindowMarker` として残り（次の `close_windows_for_restart`／`quit_app` まで消えない）、ポインタハンドラも二重に付く。切替先が既定のとき（`fatal`）も、`quit_app` の後に閉包が窓を生やす。
**Impact**: 要件 6.1 の想定どおりの失敗（マウントの失敗・シェル不在＝`wire_emo2_boot` が `boot_with_kanade_stop` の `Err` で `wired=false`）が、まさにこの経路を踏む。利用者には「壊れたゴーストの空の窓が居座り、emo2 が隣に出る」形で見える。
**Suggestion**: 窓を作る前に起動の成否を決める。`open_ghost_windows` を「準備（同期・`StartupDescriptValues` と spawn の閉包を返す）」と「投函（閉包を作業プールへ渡す）」に分け、`switch_to` は準備 → ⑤ `boot_ghost_strict` → 成功したときだけ投函、の順にする（`prepare` が返す値は今日と同じ 1 度の読取）。初回起動の `main` の順序（窓 → 起動）は据え置いてよい（そこでは失敗が告知＋終了で終わる）。代案として spawn の閉包に「世代」を持たせて古い世代を捨てる形もあるが、上の分割の方が状態を増やさない。あわせて Flow 3 の `SyncFail` の腕と `switch_to` の手順の記述を改める。
**Traceability**: 要件 6.1・6.5・4.10・3.2
**Evidence**: design.md「GhostSwitch › State Management」の `switch_to` ①〜⑥・「Flow 3」の `SyncFail → Fallback → ③〜⑤`・「Existing Architecture Analysis › 窓」

### 🔴 Critical Issue 2: `SendOff` で届いた停止通知の `handoff` を見ていない（kanade が受理していない停止まで切替として捌く）

**Concern**: research §9 B-1 は `handoff` を「kanade が切替の相を経て止まったことの証」と定義しているが、Flow 3 と `on_ghost_stopped` は `SendOff` なら `handoff` の有無を問わず `switch_to` へ進む。UI が `KanadeMsg::ChangeGhost` を送ったあと、kanade がそれを処理する前に止まる経路がある: ⑴ 利用者の終了（`CloseRequest` → 別れの台詞 → `StopSelf`）と切替要求がほぼ同時で、要求が `StopSelf` の後にアクターの inbox へ着く（アクターの loop は `Drive::Stop` で戻り、残った要求は `ChangeCancelled{Rejected}` を返さずに捨てられる。`Sender::send` が失敗するのは受信端が drop された後だけ）。⑵ `ShioriDown` で `Unloading{Fault}` に入った直後。どちらも `Stopped{handoff: None}` が `SendOff` の目印の下に届き、⑴ では**利用者が終了を選んだのに別のゴーストが起きる**。
**Impact**: 発生は狭い競合だが、本仕様の中核の不変条件（終了は終了で終わる・切替は受理された要求だけ）を破る。しかも設計が用意した証を使うだけで閉じる。
**Suggestion**: `on_ghost_stopped(SendOff, stopped)` に「`stopped.handoff.is_none()` → `warn!(event="ghost_switch_not_accepted", cause)` の上で目印を下ろし、今日どおり `quit_app(KanadeStopped(cause))`（`Fault` なら告知と終了コード 1）」の腕を足す。`handoff` の不変条件（`begin_change` で必ず `Some`・`raise_event` 無しと `Fault` でも `Some(ChangeHandoff{script: None})`）を「Data Models › 不変条件」に明記し、`frame_ghost_quit_switch_tests.rs` に `SendOff + Stopped{handoff: None}` → `AppExit` 要求の 1 本を足す。
**Traceability**: 要件 2.9・3.3・3.4・8.4
**Evidence**: design.md「Flow 3」（`Marker → SendOff → switch_to`）・「GhostSwitch › on_ghost_stopped」・research.md §9 B-1（`handoff` の定義）

### 🔴 Critical Issue 3: 要件 6.8（裁定 11）の適用範囲を設計が黙って広げている

**Concern**: 要件 6.8／裁定 11 は「単独起動で起こしたゴーストが**起動系列の途中で**失敗して終わる」ときの記憶の書き換えを定める。設計の Main は `FirstExit == KanadeStopped(Fault)` かつ経路が `Argv` でなく既定でもないとき、と書き、起動系列を終えて定常に入ったあと（何時間も動いたあと）の `Fault` でも `record_halt` が走る。`KanadeStopped` は相の情報を運ばないので、設計のままでは両者を区別できない。
**Impact**: 定常で落ちた（SHIORI の通信切れ・期限切れ）ゴーストが次回起動で既定ゴーストに置き換わり、emo2 が「○○は起きてこなかった」と話す（正典の Ref6/7「前回の処理中で落ちた」には合うが、裁定 11 の文言より広い）。広げるか絞るかで `main` の判断と要件 10.14 の檻の形が変わる。
**Suggestion**: 設計討議で 2 択を確定する。(a) 裁定 11 を「`Fault` で終わるとき全般」へ広げ、要件 6.8 と §8 (i) の文言を改める（設計は今のまま）。(b) 文言どおり起動系列に限る: 設計が既に受ける `KanadeNotice::Steady` を UI が控える（`BootContext.current` に「定常到達」の旗を 1 つ足し、`on_notice(Steady)` で立てる）だけで区別でき、`record_halt` はその旗が偽のときだけ走る。どちらでも新しい線は要らない。
**Traceability**: 要件 6.8・10.14・11.11
**Evidence**: design.md「Main（`crates/areka/src/main.rs`）› 単独起動の失敗（要件 6.8）」・「Logical Data Model」

## Design Strengths

1. **コードに対する忠実さと「状態で解く」姿勢**: 設計の主張はすべて実物の定義で裏が取れ（`close_windows_for_restart` が `GhostWindows` を外していない・`stop_cause_of` を `step` の直前で控える・`KanadeConfig` のリテラルは `new` の 1 か所・`on_reply` のマウス 2 語の腕は 1 行の書き換えで済む）、切替を 1 つの排他 system の中で完結させ、装着の相は `GhostWindows` の有無というゲートで自然に待つ。運行の通知を 1 本の `mpsc` に広げる B-2 は、順序保証を FIFO から得て新しい配線を 1 本も増やさない。
2. **裁定の写しと失敗経路の網羅**: `boot_root` の表（初回 → 切替 → 根なし）・`BootOrigin::Halted` による Ref6/7・戻す試み 1 回・`GhostFallbackFailed` を今日の `fault_of` へ合流させる形は裁定 2・3・10・11 を最短で満たす。Error Handling の表は着地先を 4 つに絞り、すべての行に記録の語彙とレベルが付いている。テスト戦略は判断の分岐に限り、既存の檻は字面の追随だけで落とさない。

## Final Assessment

**Decision: GO（条件付き）**

**Rationale**: 既存アーキテクチャとの整合・裁定の順守・要件の追跡は満たしており、構造の変更（`GhostSlot`・`BootContext`・`KanadeNotice`・`change.rs`）はどれも根拠がある。Issue 1 は本仕様が最初に扱う失敗経路そのものを壊すが、`open_ghost_windows` の準備と投函の分割（局所の順序変更）で閉じ、Issue 2・3 は腕 1 つ・旗 1 つで閉じる。いずれも設計討議の範囲で `design.md` を改めれば、タスク生成へ進める。

**Next Steps**: 設計討議で Issue 1〜3 を確定し `design.md`（`switch_to` の手順・Flow 3・`on_ghost_stopped`・Data Models の不変条件・Main の判断）へ反映 → `/kiro-spec-tasks areka-P0-ghost-shell-balloon-switch`。

## 討議で拾っておきたい小さな点（判定には影響しない）

- `run_ghost_quit_phase` は `pub(super)`（`emo2_boot` の中だけ）。設計の `ghost_session_switch_tests.rs`（crate 直下）から呼ぶには `pub(crate)` へ広げる必要がある（Modified Files に 1 行足す）。
- `fatal`（既定ゴーストの同期の失敗）の時点では `GhostSlot` が空なので、`main` の告知の場面の `ghost_name` は `GhostSession::names()` から取れない。`BootContext.current.ghost` のフォルダ名へ倒す旨を Main に書いておく。
- `Steady{Some}` に `pending_change` があり、その台本を利用者が中断して `take_user_break_quit` が真（`\-` の予約あり）のとき、設計の `on_talk_done` の腕 ⑵ は定常なので効かず、終了系列へ進んで保留の切替は捨てられる（`\![change,ghost,B]` と `\-` を同じ台本に書いた作者の意図はどちらとも読める）。要件 5.5 の「切替を続ける」との関係を 1 行決めておくとよい。
- UI スレッドで同期に降ろす（B-3）は設計が自ら計測の議題にしている。`GhostRuntime::shutdown` は kanade → dispatcher → ticker → shiori（helper プロセスの unload を含む）→ relay → sylphya の join を順に待つので、host32 経由の x86 SHIORI では unload が支配項になりやすい。`ghost_switch_down_ms` に段ごとの内訳は要らないが、超えたときの切り分けのため `GhostRuntime::shutdown` の既存の段ごとの記録の水準を signoff の `RUST_LOG` に含めておく。
