# ギャップ分析: areka-P0-session-mark-residue

> 2026-09-27 実施（`/kiro-validate-gap`）。対象はブランチ `claude/areka-p0-session-mark-residue-0e9e89`（ソースは main `5a232d2f` から不変）。コードは関数名・型名・定数名とファイルパスで指し、行番号では指さない。本書は分析と選択肢であり、決めごとは要件ディスカッション・設計で行う。

## 1. まとめ（先に結論）

- **穴 2（LogSink へ倒れた起動）は小さい**。判定の 1 か所 `session_mark_verdict`（`crates/areka/src/main.rs`）に「倒れた」を 1 つ足し、その材料を `boot_ghost` から運ぶだけで済む。材料の置き場を `GhostSession` にすれば、触ってはいけない `emo2_boot/ghost_switch.rs` に手を入れずに要件 8.3（切替先へ引き継がない）も自然に満たせる。しかも今日のコードでは、LogSink へ倒れたプロセスから切替へ進む道が無い（後述 3.2）。
- **穴 1（OS の終了で SHIORI を待ちすぎる）が本体**。期限を呼び手から渡す道は 1 本も無い（全部が定数か環境変数）。さらに「既に待っている往復」は kanade が `round_trip` の無期限の `recv()` で待ち、shiori のスレッドが `SendMessageTimeoutW` の中で止まっているので、**期限をメッセージで運んでも誰も読めない**。この段を切るには、外から待ちを解く手（32bit の補助プロセスを終わらせる）がどうしても要る。
- **穴 3（join 中の同期の送信）は、本番コードの静的な洗い出しでは「重なる箇所 0 件」**。areka のスレッドから UI スレッドの窓への同期の送信は無く、join で待つスレッドが UI スレッドを待つ箇所も無い。唯一の理屈の上の重なりは、**32bit の補助プロセスの中の SHIORI（や SAORI）が areka の窓へ同期で送る**形で、これは今日は SMTO の期限（60 秒／30 秒）でしか切れず、本仕様の上限 T で切れるようになる。
- **Windows の猶予**: Microsoft の文書では、`WM_ENDSESSION` の処理を **5 秒**遅らせると、利用者に「続けるか取りやめるか」の画面が出る（Vista 以降）。20 秒は古い既定（`WaitToKillAppTimeout`）。T は 5 秒より十分短くないと、要件 1 の目的（画面を出さない）を満たせない。
- **規模**: 推す案（後述 4 の案 A）で M 寄りの S（9〜12 タスク）。期限を段ごとに運ぶ案（案 B）は M〜L で、並走の他 spec が動かす `crates/areka-kanade/src/msg.rs` と衝突する。

## 2. 今あるもの（現状の調査）

### 2.1 起動中の印の判定（裁定 13 の実装）

| 役割 | 定義 | 今日の形 |
|---|---|---|
| きれいな終わりの判定 | `main.rs` の `session_mark_verdict(first, argv_session, run_ok, down_ok) -> MarkVerdict` | argv → 出所なし（`no_exit_origin`）→ 失敗の出所（`fault`・`switch_fatal`）→ `run_failed` → `down_failed` → 消す。出所は `ExitOrigin` の網羅の match |
| 判定の材料 | `main.rs` の `MarkInputs { app_profile_dir, argv_session, first }` | 組むのは `after_run`（`fn main` の後始末）と `session_end.rs` の `on_os_session_end` の 2 か所 |
| 判定の始末 | `main.rs` の `settle_session_mark(mark, run_ok, down_ok)` | 消す＝`boot_resolve::clear_session_mark`、残す＝`info!(event="session_mark_kept", reason, first)` |
| 印を書く | `main.rs` の `boot_first_ghost` → `boot_resolve::write_session_mark` | argv のプロセスは書かない |
| 判定の表のテスト | `main_session_mark_tests.rs` の `session_mark_verdict_table` | 位置引数 4 つで 12 回呼ぶ。`settle_session_mark_applies_the_verdict` は `MarkInputs` を直に組む。`ghost_session_switch_memory_tests.rs` も `settle_session_mark` を呼ぶ |

### 2.2 最初の起動と LogSink への倒れ込み

- `ghost_session.rs` の `boot_ghost` は `boot_wired` が `Err(BootWiringFailed)` を返すと `areka_ghost::boot_with_kanade_stop`（LogSink の起動）へ倒れる。成功なら `info!`（「LogSink フォールバックで起動しました」）＋`on_boot_ok`、失敗なら `is_benign_boot_error` で `warn!`／`error!` に分けて `None`。どちらも `GhostSession { seriko: None, loop_ticker: None, .. }` を返す。**「倒れた」ことを外へ伝える値は無い**（呼び手の `boot_first_ghost` は区別できない）。
- `boot_ghost` の呼び手は本番では `boot_first_ghost` だけ（テストの `ghost_switch_test_support.rs`・`ghost_session_restart_tests.rs`・`ghost_session_strict_tests.rs` も呼ぶ）。切替は `boot_ghost_strict`（倒れ込みなし）。

### 2.3 OS のセッションの終了の道すじ（期限の運び道の候補）

```
wintf  window_proc/lifecycle.rs の WM_ENDSESSION（wParam 真・World を借りたまま）
  → areka  session_end.rs の on_os_session_end（started＝Instant::now()）
     → run_ghost_quit_phase → quit_app(SessionEnd)
     → GhostSession::shutdown(CloseReason::System)          … ① loop ticker Close ② runtime.shutdown ③ seriko join（期限なし）
        → areka-ghost  runtime.rs の GhostRuntime::shutdown  … ForceQuit 送出 → kanade/dispatcher/ticker/shiori/relay×2/sylphya を順に join（すべて期限なし）
           → areka-kanade  actor.rs の受信ループ → schedule/mod.rs の force_quit
                → Action::ShioriRequest(OnClose NOTIFY) と Action::ShioriUnload
                → actor.rs の round_trip_request / round_trip_unload → round_trip（reply_rx.recv()＝無期限）
              → areka-kanade  shiori/real.rs の run_shiori_loop → ShioriBackend::notify / unload
                 → shiori-host32-host  client.rs の Shiori3Client::notify（effective_timeout＝REQUEST_TIMEOUT 60 秒／環境変数／0＝Duration::MAX）
                 → lifecycle.rs の HelperLifecycle::request_clean_shutdown（UNLOAD_ACK_TIMEOUT 30 秒 → EXIT_OBSERVE_TIMEOUT 10 秒の刻み待ち）
                    → parent_window.rs の send_request → shiori-host32-ipc の send_request（SendMessageTimeoutW・SMTO_ABORTIFHUNG）
     → settle_session_mark(&mark, true, down_ok) → info!(os_session_end_done, ms)
```

要点:
- **期限を運ぶ引数はどの段にも無い**。`KanadeMsg::ForceQuit { reason }`・`ShioriMsg::Request { call, reply }`／`Unload { reply }`・`ShioriBackend::notify/unload`・`Shiori3Client::notify`・`request_clean_shutdown` のどれも期限を受け取らない。
- **既に待っている往復は途中で割り込めない**。kanade は 1 件ずつ往復し（`drive` → `execute_actions` → `round_trip`）、応答を無期限の `recv()` で待つ。`ForceQuit` は kanade の受信箱に積まれるだけで、今の往復が終わるまで読まれない。shiori のスレッドは `SendMessageTimeoutW` の中なので、`ShioriMsg` も読まれない。
- 補助プロセスは `job.rs` の `attach_kill_on_close_job`（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）で job に入っており、areka のプロセスが終われば必ず道連れで終わる。`HelperLifecycle` の `Drop` も `terminate` する。ただし `HelperHandle`（`std::process::Child` を持つ）は shiori のスレッドの `ShioriConnection` の中にあり、**別スレッドから終わらせる口は今は無い**。
- `ShioriWiring` は `Helper`（本番）・`Custom`（テストの偽物）・`InProc`（x64 の SHIORI4・本番の `fn main` では選ばれない＝`main_ghost_wiring_tests.rs` が固定）の 3 通り。
- `KanadeConfig::close_talk_deadline_ms`（30 秒）は別れの台詞の期限で、`force_quit` の経路（台詞を流さない）では使われない。

### 2.4 x64 の偽境界のテスト基盤

- `session_end_tests.rs` は `SwitchRig`（`emo2_boot/ghost_switch_test_support.rs`）で偽の SHIORI（`FakeShiori::Scripted`＝`emo2_boot/spine.rs` の `ScriptedShioriBackend`）を起こして `on_os_session_end` を実際に踏む。`capture` でログの事象と水準を判定する作法がある。
- `ScriptedShioriBackend` は応答を即座に返すだけで、**「解かれるまで固まる」台本は無い**（Missing）。32bit のテスト用 DLL（`shiori-host32-testdll`）にも固まる動きは無い。

## 3. 要件ごとの対応表（Requirement-to-Asset Map）

凡例: **Missing**＝無い／**Unknown**＝調べが要る／**Constraint**＝既存の作りからくる縛り

| 要件 | 既存の資産 | ギャップ |
|---|---|---|
| 1.1 合計を T 以下 | `on_os_session_end` の `started` | **Missing**: 期限の上限を運ぶ道・既に待っている往復を切る手。**Constraint**: kanade の往復は無期限の `recv()`、shiori は `SendMessageTimeoutW` の中 |
| 1.2 T は固定の値 | — | **Unknown**: 値（議題 1）。Windows の実際の猶予は 5 秒（下の 5 章） |
| 1.3 各段は「今日の既定」と「T の残り」の短い方 | `effective_timeout`・`UNLOAD_ACK_TIMEOUT`・`EXIT_OBSERVE_TIMEOUT` | **Missing**: 段ごとに期限を受け取る口。案 A では「T で全段を切る」になり、段ごとの計算は無くなる（議題 D3） |
| 1.4 環境変数より T が優先 | `request_timeout_from_env` | **Missing**（案 B なら min の計算・案 A なら T の打ち切りで自動的に満たす） |
| 1.5 期限内なら今日どおり | `session_end_takes_ghost_down_with_system_close_and_clears_mark` | なし（今日の檻がそのまま使える） |
| 1.6 他の段は省かない | `GhostRuntime::shutdown` の join の順 | **Constraint**: shiori の join を飛ばすと、後ろの relay×2・sylphya の段が待ち続けるか飛ぶ。案 A は shiori を解いて全段を通常どおり進める |
| 2.1 T で待ちをやめ残りを続ける | — | **Missing** |
| 2.2 どの段で達したかを `warn!` | shiori 側の各失敗の `error!`（`unload_failed` など） | **Missing**: 「今どの段で待っているか」を watchdog が読める場所。`request_clean_shutdown` の中の「UNLOAD の応答」と「終了の観測」は外から区別できない（議題 D4） |
| 2.3 印を残す・理由の語 | `MarkVerdict::Keep(&'static str)` | **Missing**: 判定の材料に「上限切れ」 |
| 2.4 補助プロセスを残さない | `HelperLifecycle::terminate`（冪等）・job の道連れ | **Missing**: 別スレッドから終わらせる口（送れる形の取っ手） |
| 2.5 告知なし・終了コードは最初の出所 | `after_run` の `SessionEnded` の枝・`fault_of` | なし。ただし案 A では補助プロセスを終わらせた結果 kanade が `Fault` で止まり、`KanadeNotice::Stopped(Fault)` が受け口に残る（プロセスは終わるので読まれない・`FirstExit` は `SessionEnd` のまま）。記録に `error!` が数件出る点は設計で扱う（議題 D5） |
| 2.6 期限内なら印の判定は今日どおり | `session_mark_verdict` | なし |
| 3.1〜3.3 他の経路の期限は不変 | `REQUEST_TIMEOUT`・`LOAD_ACK_TIMEOUT`・`UNLOAD_ACK_TIMEOUT`・`EXIT_OBSERVE_TIMEOUT` と、その値を判定する既存テスト | 案 A なら他の経路のコードは 1 行も変わらない。案 B なら「上限なし」の分岐が全段に入り、3.3 を判定するテストが段ごとに要る |
| 3.4 他の経路の期限切れの印は不変 | — | なし |
| 3.5 環境変数・依存クレートを足さない | `windows` クレートは既存（`TerminateProcess`・`TerminateJobObject`・`DuplicateHandle` は同じクレートの別の機能） | **Unknown**: `windows` の feature の追加が要るか（`Win32_System_Threading`・`Win32_System_JobObjects` は shiori-host32-host で既に使用中） |
| 4.1〜4.3 倒れたら印を残す | `boot_ghost` の LogSink の腕の `info!`／`warn!`／`error!` | **Missing**: 「倒れた」を運ぶ値と、判定の材料の欄。倒れた時点の帰結の `warn!` |
| 4.4 次の起動は既定＋Ref6/7 | `first_boot_origin`・`resolve_boot`（印を読む） | なし（印さえ残れば今日の道で起きる） |
| 4.5 既定ゴースト自身でも残す | — | なし（判定が名前を見ないので自然に満たす） |
| 4.6 argv は触らない | `session_mark_verdict` の先頭の argv 判定 | なし（順序を守れば自然に満たす） |
| 4.7 終了コード・告知・LogSink の起動は不変 | — | なし |
| 4.8 結線成功と切替は不変 | `boot_ghost_strict` | なし |
| 5.1 同期の送信の洗い出し | — | 本書 6 章で実施（**0 件**） |
| 5.2 再現のテスト | — | **Missing**: 代表の形の再現テスト（0 件なので代表の形を起こす） |
| 5.3／5.4 直す／理由を記録 | `on_os_session_end` の説明 | 0 件なら 5.4（説明と記録と常設テスト） |
| 6.1 判定は 1 か所 | `session_mark_verdict` | なし（材料を足すだけ） |
| 6.2 理由の語 | `fault`・`switch_fatal`・`no_exit_origin`・`run_failed`・`down_failed` | **Missing**: 新しい語 2 つ（例: `logsink_fallback`・`session_end_deadline`） |
| 6.4 §8 の追記 | `doc/COMPAT_ARCHITECTURE.md` §8 の 2 行（「きれいに終わらなかった次の起動（起動中の印）」「OS のシャットダウン・再起動・ログオフで終わるとき」）は実在を確認 | 追記のみ |
| 7.1 偽の時計・偽の SHIORI | `SwitchRig`・`ScriptedShioriBackend`・`capture` | **Missing**: 「解かれるまで固まる」台本、上限 T を注入する口（本番は定数・3.5 により環境変数は足せない）、実時間を待たない打ち切りの仕組み（議題 D6） |
| 7.3 表に行を足す | `session_mark_verdict_table` | 引数が増えるので既存 12 回の呼び出しも書き換え（7.6 が許す） |
| 7.7 実機確認 | 完了 spec の ⑧ の手順 | 「窓への結線を不成立にする手」は **Unknown**（例: バルーンの根を壊す・シェルの絵を消す。`wire_emo2_boot_falls_back_to_unwired_on_missing_ghost_root` が手がかり） |

### 3.1 裏で気付いたこと（要件の前提の確認）

1. **要件 1.2 と 8.1 の「既定で数秒〜20 秒程度」**: Microsoft の文書では、`WM_ENDSESSION` の処理を 5 秒遅らせると利用者に選ばせる画面が出る（5 章）。20 秒は古い `WaitToKillAppTimeout` の既定。T を決める基準は「5 秒」と読むのが安全。
2. **要件 8.3（倒れたプロセスから切替へ進む場合）は、今日のコードでは起きない**: 切替の入口は ⑴ 台本の `\![change,ghost]` → `ChangeCueSink` → `ghost_switch::ChangeRx`、⑵ 右クリックメニューの「ゴースト」枠、の 2 つ。`ChangeRx` は `wire_emo2_boot` が起動の成立後にだけ World へ据え（倒れた経路では受信端が落ちる＝`emo2_boot/mod.rs` の切替要求の channel の説明）、メニューは `boot_wired` の中の `menu::wire_menu` でだけ結線される。つまり LogSink へ倒れたプロセスは切替できない。8.3 は「将来そうなったとき」の約束として残る。
3. **要件 2.2 の「どの段で達したか」**: 案 A では、止まっていた段は shiori のスレッドの中の今の呼び出しで決まる。UNLOAD の応答待ちと終了の観測は `request_clean_shutdown` の中の 2 手で、外からは 1 つの `unload` に見える。段の粒度をどこまで取るかは議題 D4。

## 4. 実装の選択肢

### 穴 1: OS の終了のときだけ SHIORI の待ちに上限を付ける

#### 案 A（推し）: 上限 T の見張り＋補助プロセスを終わらせて待ちを解く

- **形**: `on_os_session_end` が降ろす前に「見張り」を張る（後始末に入った時点 `started` から T）。T が過ぎたら、shiori のスレッドの外から **32bit の補助プロセスを終わらせる**。補助プロセスが消えると、止まっていた `SendMessageTimeoutW`（今の往復・`OnClose` の通知・UNLOAD）は宛先の窓を失って戻り、`request_clean_shutdown` の終了の観測は `HelperStatus::Exited` で即座に抜ける。kanade は失敗の応答で終了系列へ進み、以後の join（dispatcher・ticker・shiori・relay×2・sylphya の書き出し・seriko）は今日どおり走る。見張りが発火したかどうかを判定の材料に渡し、印を残す。
- **触る所**:
  - `shiori-host32-host`: `HelperHandle`／`HelperLifecycle` から、**別スレッドへ送れる「終わらせる取っ手」**を出す（例: プロセスの取っ手の複製で `TerminateProcess`、または job の取っ手で `TerminateJobObject`）。既存の `terminate` と同じく冪等にする。
  - `areka-kanade/src/shiori/real.rs`: `ShioriBackend` に「外から待ちを解く取っ手を返す」口を 1 つ足す（既定は「無い」）。`ShioriConnection` は上の取っ手を返す。本ファイルは host32 の型を import してよい唯一の場所（Boundary Commitment）なので、ここで包むのが筋。
  - `areka-ghost/src/runtime.rs`: `connect` は shiori のスレッドで走るので、取っ手を呼び手へ渡す受け皿（一度だけ書ける置き場）を `spawn_shiori_actor` の前に用意し、`GhostRuntime` が持つ。`GhostRuntime::shutdown` と並ぶ「期限つきで降ろす」入口を足す（既存の `shutdown` は不変）。
  - `areka/src/ghost_session.rs`: `GhostSession::shutdown` と並ぶ期限つきの入口（seriko の join の手前までを見張りの内側に入れる）。
  - `areka/src/session_end.rs`: 期限つきの入口を呼び、発火の有無を印の判定へ渡し、`warn!` を 1 件残す。
- **良い点**:
  - 「既に待っている往復」も切れる（要件 1.1 の最も難しい段）。**これを切れるのは外から解く手だけ**で、案 B も結局これが要る。
  - 他の経路は 1 行も変わらない（要件 3.1〜3.3 を構造で満たす）。環境変数の意味も変わらない（1.4 は「T で切る」で自動的に満たす）。
  - `crates/areka-kanade/src/msg.rs`（`KanadeMsg`・`ShioriMsg`）と `schedule/mod.rs` の `Action` に触らずに済む見込み＝並走する `shell-balloon-switch`・`ghost-install`・`network-update` と当たらない。
  - 補助プロセスを残さない（要件 2.4）を、同じ一手で満たす。
- **弱い点**:
  - 要件 1.3（段ごとに min を取る）と 7.1 ⑶（環境変数が長い・無限・短いの 3 通りで段ごとの期限を判定）の**字面とずれる**。案 A では段ごとの期限の計算そのものが無く、「どの段でも T で切れる」を判定するテストになる（議題 D3）。
  - 補助プロセスを終わらせた結果、kanade は `Fault` として止まり、shiori 側も `helper_exited`・`unload_failed` などの `error!` を出す。上限切れの回に「本物の失敗」と見分けにくい記録が混ざる（議題 D5）。
  - `InProc`（x64 の SHIORI4 をプロセス内に読む）には終わらせる手が無い。本番の `fn main` は `Helper` だけなので今日は踏まないが、M2 で `InProc` を本番に使うときの宿題になる（議題 D7）。
  - 補助プロセスが消えれば `SendMessageTimeoutW` がすぐ戻る、という前提は **Research Needed**（下の 7 章）。

#### 案 B: 期限を段ごとに運ぶ（brief の Approach の字面どおり）

- **形**: `KanadeMsg::ForceQuit` に期限（`Option<Instant>`）を足し、`force_quit` → `Action::ShioriRequest`／`ShioriUnload` → `ShioriMsg::Request`／`Unload` → `ShioriBackend::notify`／`unload` → `Shiori3Client::notify`（`effective_timeout` と残りの短い方）・`request_clean_shutdown`（`UNLOAD_ACK_TIMEOUT` と `EXIT_OBSERVE_TIMEOUT` をそれぞれ残りで縮める）へ運ぶ。
- **良い点**: 要件 1.3・1.4・7.1 ⑶ の字面に一致する。`OnClose` の通知と UNLOAD の配分を段ごとに設計できる。どの段で切れたかが自然に分かる（2.2）。
- **弱い点**:
  - **既に待っている往復は切れない**（期限を載せたメッセージを誰も読めない）。要件 1.1 を満たすには結局、案 A の「外から解く手」を足すことになる＝実質は案 C。
  - `crates/areka-kanade/src/msg.rs` の `KanadeMsg::ForceQuit`・`ShioriMsg` の形を変える。境界の節は `msg.rs` の新しい変種を後続 3 本の持ち物としており、同じファイルの既存の変種の欄を変えるのは衝突の元（並走の都合で「触らない」に近い）。
  - `ShioriBackend` の署名が変わり、偽物（`emo2_boot/spine.rs` の `ScriptedShioriBackend`・`areka-ghost/tests/ghost/spine_e2e_test.rs`・`shiori_inproc.rs` の backend）をすべて直す。
  - 上限なしの分岐が全段に入り、要件 3.3（1 ミリ秒も違わない）を段ごとに判定するテストが要る。
  - 規模が M〜L に膨らむ（brief の見積り S を超える見込み）。

#### 案 C: 混ぜる（A の外から解く手＋B の段ごとの期限）

- 既に待っている往復は案 A で切り、`OnClose` の通知と UNLOAD は案 B で残りに合わせて縮める。要件の字面を最も広く満たすが、案 B の弱い点（`msg.rs`・`ShioriBackend` の署名・全段の分岐）をそのまま抱える。
- 差が出るのは「今の往復が T の途中で終わり、`OnClose` の通知が残りの時間で固まった」回だけ。案 A でも T で切れるので、利用者から見える結果（T で終わる・印が残る）は同じ。差は記録の「どの段」の精度だけ。

### 穴 2: LogSink へ倒れた起動の印

| 案 | 形 | 良い点 | 弱い点 |
|---|---|---|---|
| **2-A（推し）: `GhostSession` に「倒れた」を持たせる** | `boot_ghost` の LogSink の腕で作る `GhostSession` に印（例: 欄 1 つ）を立てる。`after_run` と `on_os_session_end` は置き場から取り出した単位の印を読んで `MarkInputs` に載せ、`session_mark_verdict` が理由 `logsink_fallback`（仮）で残す | 印が「いま動いているゴースト」に付くので、将来切替が入っても切替先の単位には付かない＝要件 8.3 を自然に満たす。`ghost_switch.rs` に触らない。倒れた先の成否（`runtime` の有無）に依らない（4.2） | 置き場が空（切替の致命・OS の終了で降ろし済み）のときは材料が無い。ただしどちらも既に別の理由で判定される（`switch_fatal`・`on_os_session_end` の中で判定済み） |
| 2-B: World の資源（プロセスに 1 つ）に「倒れた」を置く | `boot_first_ghost` が資源を挿し、両方の後始末が読む | 読む側が単純 | 8.3 を守るには切替の成功で資源を外す必要があり、それは `ghost_switch.rs`（触らないファイル）の仕事になる |
| 2-C: 出所 `ExitOrigin` に載せる | — | — | 要件 8.2 で不採用（倒れた先が成功した場合を塞げない） |

共通の作業: `session_mark_verdict` の材料を増やす（引数を足すか、材料をまとめた値にする）。判定の順は議題 D8。倒れた時点の `warn!`（4.3・「終わり方によらず印を残す」）は `boot_ghost` の LogSink の腕か `boot_first_ghost` に 1 件。

### 穴 3: join と同期の送信の重なり

| 案 | 形 | いつ選ぶ |
|---|---|---|
| **3-A（推し・静的な見立てが当たった場合）: 直さず理由を記録し、崩れたら赤になる檻を常設** | ⑴ `on_os_session_end` の説明に理由（誰がどのスレッドの窓へ送るか・なぜ循環しないか）を書く ⑵ 代表の形の再現テスト（UI スレッドの窓を作ったスレッドが別スレッドの join で塞がっている間に、3 つ目のスレッドがその窓へ期限つきの同期の送信をする → join は送信に依らず戻り、送信は UI スレッドが次にメッセージを取り出した時点で返る）⑶ 本番のソースに新しい同期の送信（`SendMessageW`・`SendMessageTimeoutW` など）が現れたら赤になる判定（許すのは host32 の 2 か所だけ） | 6 章の洗い出しが 0 件のとき（要件 5.4） |
| 3-B: join の間もメッセージを配る | 降ろす処理を別スレッドで走らせ、UI スレッドは `MsgWaitForMultipleObjects` で待ちつつ送られてきたメッセージだけ配る | 重なりが見つかったとき（要件 5.3）。ただし World を借りたまま窓の手続きへ再入するので、`wndproc_bridge.rs` の `try_borrow` の安全スキップで大半のメッセージが捨てられる点を設計で詰める必要がある |
| 3-C: 同期の送信を非同期に替える | 該当の送り手を `PostMessageW`／channel に | 送り手が areka の中に見つかったとき（今は 0 件） |

補足: 3-A の ⑶（ソースの判定）は、方針「検査は表示するだけでなく判定させよ」「報告を読むだけでは実在が判断できない」に沿う。一方で要件 5.2 の「待ちの最中に別スレッドから同期の送信を 1 通」は、そのままでは「送信は後始末が終わるまで返らない」ことを観測するテストになる（戻るのは後始末の後）。テストが何を緑とするか（後始末が上限内に戻る・送信は後始末の後に返る）を設計で言葉にする。

## 5. Windows がくれる猶予（上限 T の参考）

- [Shutdown Changes for Windows Vista](https://learn.microsoft.com/en-us/windows/win32/shutdown/shutdown-changes-for-windows-vista): Vista 以降、`WM_QUERYENDSESSION` に TRUE を返したアプリは `WM_ENDSESSION` への応答を **5 秒**遅らせることができ、その後は利用者に「続けるか取りやめるか」を選ばせる全画面の UI が出る。見える窓の無いアプリは 5 秒で自動的に終わらされる。
- [WM_ENDSESSION](https://learn.microsoft.com/en-us/windows/win32/shutdown/wm-endsession): wParam が TRUE なら、すべてのアプリがこのメッセージの処理から戻った後、いつでもセッションは終わりうる。
- [WaitToKillAppTimeout（Windows Server 2003 の資料）](https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-server-2003/cc737288(v=ws.10)): 既定 20000 ms。要件の「〜20 秒程度」はこの値に当たるが古い資料。`HungAppTimeout`（既定 5000 ms）は Windows 10 以降は推奨されない。

読み: 画面を出さないための実際の線は **5 秒**。T はそれより十分短く、かつ後ろの段（kanade・dispatcher・sylphya の書き出し・seriko の join）の時間も 5 秒の内に入る必要がある。完了 spec の実機の所要は 22 ms（SHIORI が健全なとき）。候補の目安として 1〜3 秒の幅があり、値は議題 1 で開発者が決める。なお areka の窓は見える窓（透過でも可視）なので「見える窓の無いアプリは 5 秒で自動終了」の方ではなく、利用者に選ばせる画面の方に当たる見込み（**Research Needed**: WUC・透過の窓を OS がどちらに数えるかは実機で見る）。

## 6. 要件 5.1 の洗い出し（UI スレッドの窓への同期の送信）

調べ方: 本番コード（`_tests.rs`・`tests/`・`examples/`・`#[cfg(test)]` を除く）で `SendMessage*`・`SendNotifyMessage`・`BroadcastSystemMessage` と、別スレッドから呼ぶと同期の送信になる API（`SetWindowPos`・`ShowWindow`・`SetWindowLongPtrW`・`DestroyWindow` など）を grep し、呼ぶスレッドを確かめた。あわせて、join で待つスレッドが UI スレッドを待つ箇所（channel の同期の待ち・`reply_channel` の `recv`）も洗った。

| # | 箇所 | 送り手のスレッド → 宛先 | join と重なるか |
|---|---|---|---|
| 1 | `shiori-host32-ipc` の `send_copydata`（`SendMessageTimeoutW`・`SMTO_ABORTIFHUNG`）← `parent_window.rs` の `send_request` | areka の shiori のスレッド → **32bit の補助プロセスの** message-only 窓（別プロセス） | UI スレッドの窓ではない。循環しない |
| 2 | `shiori-host32-ipc` の応答の送り（`SendMessageTimeoutW`・`SMTO_ABORTIFHUNG` なし）← `shiori-host32-helper/src/main.rs` | 補助プロセス → areka の shiori のスレッドの message-only 親窓 | UI スレッドの窓ではない（親窓は shiori のスレッドが持ち、`SendMessageTimeoutW` の待ちの中で受ける） |
| 3 | `parent_window.rs` の握手の起こし（`PostMessageW(WM_NULL)`） | 握手の間だけの補助スレッド → 親窓 | 非同期。起動時だけ |
| 4 | `menu/win32.rs` の `PostMessageW(WM_NULL)` | UI スレッド自身 | 非同期 |
| 5 | wintf の `SetWindowPos`・`ShowWindow`・`SetWindowLongPtrW`・`DestroyWindow`（`api.rs`・`win_style.rs`・`window/command.rs`・`runtime/window_factory.rs` ほか） | UI スレッド（ECS の系と窓の手続き） | 自分の窓なので同期の送信にならない |
| 6 | wintf の VSync のスレッド（`runtime/tick_bridge.rs` の `vsync_loop`） | `DwmFlush` と `event_listener::Event::notify` だけ | 窓に触らない。OS の終了の後始末は join しない |
| 7 | ゴーストの実行系のスレッド（kanade・shiori・dispatcher・ticker・relay×2・sylphya・seriko・sakura の talk） | UI への知らせはすべて `mpsc::Sender::send`（`KanadeNotice`・`PresentBridge`・`MoveCueSink`・`ZOrderCueSink`・`BalloonLifecycleSink`・`ReadmeCueSink`・`ChangeCueSink`・`NoUserBreakCueSink`）＝非同期 | UI を待たない。`reply_channel` の同期の待ちは、UI が kanade に尋ねる向き（`menu/captions.rs` の `send_query`・覗くだけ）と、同じスレッドの中で閉じるもの（`emo-present` の `show_target`）だけ |
| 8 | COM | UI スレッドは `COINIT_MULTITHREADED`（`runtime/mod.rs`）、WUC の DispatcherQueue は UI スレッドに `DQTYPE_THREAD_CURRENT` で付く | MTA なので別スレッドからの COM 呼び出しはメッセージで取り次がれない |

**結論: areka の中で、join と重なって UI スレッドを待つ箇所は 0 件**。したがって要件 5.4（直さず理由を記録）に当たる見込み。

理屈の上で残る重なり（areka の外のコード）:
- **補助プロセスの中の SHIORI・SAORI が、`OnClose` の処理中などに areka のトップレベル窓（または `HWND_BROADCAST`）へ同期で送る**場合: UI スレッド（join）→ kanade（往復の待ち）→ shiori のスレッド（`SendMessageTimeoutW`）→ 補助プロセスの DLL（areka の窓への `SendMessage`）→ UI スレッド、と輪になる。今日は #1 の期限（60 秒・UNLOAD は 30 秒）でしか切れない。本仕様の上限 T（案 A なら補助プロセスを終わらせる）で切れるようになる。要件 5 の記録にはこの形を「外のコードが作りうる唯一の輪。上限 T で切れる」と書くのが妥当。
- 他のプロセス（IME・シェル・他アプリの `HWND_BROADCAST`）が UI の窓へ送る場合は、送り手の側が待つだけで、areka の join はそれに依らないので輪にならない。

ついでの観察（本仕様の外）: 右クリックメニューの `TrackPopupMenuEx` は自前のメッセージループを回す。メニューを開いている最中に `WM_ENDSESSION` が届き、そのとき World が借りられていれば、既存の `os_session_end_world_busy` の `warn!` の道（後始末をせず印が残る）へ進む。今日の設計どおりの道で、本仕様では扱わない。

## 7. 調べが要ること（Research Needed）

1. **補助プロセスを終わらせたとき、止まっている `SendMessageTimeoutW` がすぐ戻るか**（案 A の前提）。受け手のスレッドが消えれば送り手の待ちは解ける、が Win32 の一般的な振る舞いだが、`SMTO_ABORTIFHUNG` との組み合わせと戻り値（`IpcError::SendFailed` になるか）を i686 の実物で確かめる。常設テストは x64 の偽境界で書き（方針「常時テストは x86 回避」）、実物の確認は実機サインオフか一度きりの e2e で行う。
2. **`SMTO_ABORTIFHUNG` が、送った後で相手が固まったときに途中で打ち切るか**。打ち切るなら、固まった SHIORI の実際の待ちは 60 秒より短い（parent_window.rs の説明に「`SMTO_ABORTIFHUNG` の実測は 14 秒」とある）。今日の「約 100 秒」の見積りが縮むだけで、本仕様の結論は変わらない。
3. **別スレッドから補助プロセスを終わらせる口の形**: `HelperHandle` の `Child` を共有するか、プロセスの取っ手を複製するか、job の取っ手で `TerminateJobObject` するか。`windows` クレートの feature の追加が要るか（要件 3.5 は依存クレートの追加を禁じるが、feature の追加の扱いは要確認）。
4. **OS が areka の窓（WUC 合成・`WS_EX_TRANSPARENT` のトグル）を「見える窓」と数えるか**（5 章の 2 つの扱いのどちらに当たるか）。
5. **実機で LogSink へ倒す手**（要件 7.7 ⑵）: 起動前の解決（`resolve_boot`）は通り、`wire_emo2_boot` だけが失敗する壊し方を探す（例: シェルやバルーンの資材を壊す）。

## 8. 設計へ持ち越す議題（要件ディスカッションの材料）

- **議題 1（要件 8.1・確定＝3 秒・2026-09-27 開発者）: 上限 T の値**。Windows の実際の線は 5 秒（5 章）。T の後ろにも後始末（join・書き出し）が続くので、5 秒から後ろの段の分を引いた内側に置く必要がある。
- **D2: 穴 1 の方式**＝案 A（見張り＋補助プロセスを終わらせる）／案 B（段ごとに期限を運ぶ）／案 C（両方）。既に待っている往復を切れるのは外から解く手だけ、という点が分かれ目。
- **D3: 要件 1.3・1.4・7.1 ⑶ の字面**。案 A を採るなら「各段の期限を短い方にする」は「どの段で待っていても T で切れる」に言い換えることになる（要件の改訂が要る）。案 B・C なら字面どおり。
- **D4: 要件 2.2 の「どの段で達したか」の粒度**。案 A では shiori のスレッドが「今の呼び出し」を共有の置き場に書き、見張りが発火時に読む形が要る。UNLOAD の応答と終了の観測を分けるには `request_clean_shutdown` の中にも印が要る。粒度を「今の往復／`OnClose`／UNLOAD（応答と観測をまとめる）」の 3 つに縮めるかどうか。
- **D5: 上限切れの回に出る副次の記録**。補助プロセスを終わらせると kanade は `Fault` で止まり、shiori 側も `helper_exited`・`unload_failed` などの `error!` を出す。これを許す（上限切れの `warn!` 1 件で因果が分かるようにする）か、終わらせた事実を shiori 側へ伝えて記録の語を変えるか。
- **D6: テストで実時間を待たない仕組み**。見張りの時計を差し替える口（偽の時計）か、見張りの発火をテストが手で起こす口か。上限 T は本番では定数、テストでは引数で渡す（要件 3.5 で環境変数は足せない）。`ScriptedShioriBackend` に「解かれるまで固まる」台本を足すのは必須。
- **D7: `InProc`（x64 の SHIORI4）の扱い**。外から終わらせる手が無い。本番では選ばれないので本仕様では「対象外」と記録するか、join を期限で見切る二段目の守りも入れるか。
- **D8: `session_mark_verdict` の判定の順と材料の形**。「倒れた」「上限切れ」を既存の理由（`fault` など）より前に置くか後に置くか（例: 倒れた先の LogSink の起動が後で SHIORI の失敗で止まったとき、理由はどちらを記録するか）。引数を足すか、材料をまとめた値にするか（既存テストの書き換え量が変わる）。
- **D9: 要件 5.2 の再現テストが何を緑とするか**。洗い出しは 0 件なので代表の形を起こすが、「送信は後始末が終わるまで返らない（UI スレッドがメッセージを取り出した時点で返る）」が正しい振る舞い。緑の条件を「後始末は上限内に戻る・送信は後始末の後に返る」と言葉にするか。加えて、本番のソースに新しい同期の送信が現れたら赤になる判定を置くか。

## 9. 規模と危うさ

- **規模: M 寄りの S（案 A・2-A・3-A で 9〜12 タスク）**。内訳の目安: 穴 1＝5〜7（host32 の取っ手・`ShioriBackend` の口・`GhostRuntime`／`GhostSession` の期限つきの入口・`on_os_session_end`・固まる台本と見張りのテスト・他の経路が不変のテスト）、穴 2＝2〜3（`GhostSession` の印・判定の材料と表・1 周のテスト）、穴 3＝1〜2（説明と記録・代表の再現テスト・ソースの判定）、§8 の追記と実機確認で 1。案 B・C なら M〜L（`msg.rs`・`Action`・`ShioriBackend` の署名と全偽物の書き換えが加わる）。
- **危うさ: 中**。理由: 穴 1 は Win32 の振る舞い（補助プロセスが消えたときの `SendMessageTimeoutW` の戻り方）に依る新しい形で、x64 の偽境界では実物を踏めない。穴 2・3 は既存の型を延ばすだけで低い。
- **並走との当たり**: 案 A は `emo2_boot/ghost_switch.rs`・`change_cue.rs`・`menu/`・`areka-parsers/`・`schedule/events.rs`・`msg.rs` に触らない見込み。案 B・C は `msg.rs` の既存の変種（`KanadeMsg::ForceQuit`・`ShioriMsg`）を変えるので、後続 3 本と当たる。

## 10. 設計フェーズの決定（`/kiro-spec-design`・2026-09-27）

> 上の 1〜9 章はギャップ分析（2026-09-27）。本章は設計で決めたことと、設計のために追加で調べたこと。決めごとの本文は `design.md` にあり、ここは経緯と採らなかった案。

### 10.1 まとめ
- **Discovery Scope**: Extension（既存の終了経路・印の判定・テストの土台の延長。外部の新しい依存は無い）。
- **Key Findings**:
  - `TerminateProcess` が要る機能 `Win32_System_Threading` はワークスペースの `windows` 依存（ルート `Cargo.toml` の `[workspace.dependencies.windows]`）に既にあり、`shiori-host32-host` は `workspace = true` で継ぐ。**機能も依存クレートも足さずに済む**（要件 3.5 の懸念は消えた）。`TerminateJobObject`（job の取っ手で終わらせる）も既存の機能で書けるが、job の割り当てが縮退（`None`）した補助プロセスに効かないので採らない。
  - `Child` の取っ手は std の `AsHandle` と `OwnedHandle::try_clone_to_owned` で複製でき、`Send + Sync` の取っ手になる（`Child::kill` は `&mut` なので別スレッドからは使えない）。
  - kanade は補助プロセスが消えた後も今日の道で止まる: `OnClose` の失敗は `Unloading{Forced}` の中の応答として `Stopped` へ（`schedule/mod.rs` の「Unloading 中の応答は Unload 完了として扱う」）、`request_clean_shutdown` は `status()` の `Exited` で短絡する。新しい遷移は要らない。
  - LogSink の腕の `ghost_boot_options` は App スコープの置き場を `default_app_profile_dir()` で埋める。本番では結線ありの腕の入力（`GhostBootInputs::production`）と同じ値だが、テストで倒れた先を成功させると実行体の隣の実 profile へ書く。結線の入力の `app_profile_dir` をそのまま渡す（本番の値は不変）。
  - テストの窓は `CreateWindowExW(STATIC, HWND_MESSAGE)` で登録なしに作れる（機能 `Win32_UI_WindowsAndMessaging` はワークスペースに既にある）。message-only 窓への同期の送信は、窓を持つスレッドがメッセージを取り出す関数（`PeekMessageW` など）を呼んだ時点で配られる。

### 10.2 決定

#### D2: 穴 1 の方式＝案 A（見張り＋補助プロセスを終わらせて解く）
- **Context**: 既に待っている往復（kanade の無期限の `recv()`・shiori のスレッドの `SendMessageTimeoutW`）は期限をメッセージで運んでも切れない。
- **Alternatives**: 案 B（段ごとに期限を運ぶ・`msg.rs` と `ShioriBackend` の署名を変える・既に待っている往復は切れない）／案 C（A＋B・差は記録の「どの段」の精度だけ）。
- **Selected**: 案 A。見張りは kanade の `ShioriProbe::arm`（`recv_timeout` の別スレッド）、解く手は host32 の `HelperTerminator`（プロセスの取っ手の複製＋`TerminateProcess`）、`ShioriConnection::unblock_handle` が両者を結ぶ。
- **Rationale**: 他の経路のコードが 1 行も変わらず（要件 3 を構造で満たす）、並走の `msg.rs` に触らない。2.4（補助プロセスを残さない）を同じ一手で満たす。
- **Trade-offs**: 段の記録は shiori のアクターが書く「今の呼び出し」に依る（D4）。補助プロセスを終わらせた後の shiori 側の `error!` が残る（D5）。
- **Follow-up**: 前提（終わらせると `SendMessageTimeoutW` が戻る）を `terminator_tests.rs` の別プロセスの窓で固定し、実機でも見る。

#### D3: 要件 1.3・1.4・7.1 ⑶ の字面
- 要件 1.3 は要件ディスカッションで「どの段で待っていても T で打ち切る（今日の期限が先に切れる段は今日どおり）」に改まっているので、案 A と一致する。1.4 は「見張りが環境変数を読まず外から解く」ことで構造的に満たす。7.1 ⑶ は「SHIORI 側の期限が T より長い・無限・短い」の 3 通りとして起こす（長い・無限＝固まる台本で T の打ち切り／短い＝台本が自分で `Err(Timeout)` を返し、打ち切りは起きず今日どおり）。要件の改訂は要らない。

#### D4: 「どの段で達したか」の粒度＝3 語＋`idle`
- shiori のアクターが往復の直前に `ShioriBusy::Request(id)`／`Unload` を書き、見張りが読む。`Request("OnClose")` → `on_close_notify`、他の `Request` → `in_flight_request`、`Unload` → `unload`（UNLOAD の応答と終了の観測は `request_clean_shutdown` の中で分けられないのでまとめる）、`Idle` → `idle`。要件 2.2 の「少なくとも 3 つ」を満たし、`request_clean_shutdown` の中に印を足さない。

#### D5: 上限切れの回に出る副次の記録＝許す
- 補助プロセスを終わらせると shiori 側の `helper_exited`・`unload_failed` などの `error!` と kanade の `Fault` が出る。その直前に `warn!(shiori_wait_cut, stage, limit_ms, elapsed_ms)` が 1 件あるので因果は読める。終わらせた事実を shiori 側へ伝えて語を変える案は、`ShioriBackend` と `run_shiori_loop` の失敗の分類に手を入れるので採らない。`os_session_end_done` に `shiori_cut` の欄を足し、`session_mark_kept` の理由は `session_end_deadline`。

#### D6: テストで実時間を待たない＝見張りを手で起こす口（`cut_now`）
- 偽の時計は持たない（見張りは `recv_timeout` 1 本）。テストは `ScriptedShioriBackend` の「解かれるまで固まる」台本（`hold_at`）で該当の段に固め、補助のスレッドが固まりを観測してから `ShioriProbe::cut_now` を呼ぶ。期限の口そのものは、応答しない台本と数十 ms の `limit` で 1 本だけ確かめる（台本が自分では応答しないので結果は時刻に依らない）。上限 T は本番の定数 `SESSION_END_SHIORI_LIMIT`、テストは `end_session_within(world, limit)` の引数（要件 8.1・環境変数は足さない）。

#### D7: `InProc` の扱い＝対象外として記録
- `unblock_handle` の既定 `None` で、見張りは `error!(shiori_unblock_unavailable)` を残して待ちは今日どおり続く。本番の `fn main` は `Helper` だけ（`main_ghost_wiring_tests.rs` が固定）。join を期限で見切る二段目の守りは、shiori のスレッドを取り残す（資材の所有者が消える）ので採らない。M2 で `InProc` を本番に使うときの宿題。

#### D8: 判定の順序と材料の形＝時系列で最初の理由・`Teardown` の小さな値
- 順序: `argv` → `logsink_fallback` → `no_exit_origin` → `fault`／`switch_fatal` → `run_failed` → `session_end_deadline` → `down_failed` → 消す。既存の順（出所の失敗 → run → down）も時系列なので、同じ規則で 2 つを差し込める。LogSink へ倒れたプロセスが後で SHIORI の失敗で止まっても理由は `logsink_fallback`（どちらでも印は残り、4.3 の語は満たす）。
- 材料: `MarkInputs` に `logsink_fallback` を足し、降ろした結果を `Teardown { run_ok, down_ok, shiori_cut }` にまとめる。位置引数の bool を 6 つ並べる形は避けた（両枝が同じ誤りを書いても気付けない）。既存 21 行の表は形を書き換えるだけで結論は不変。

#### D9: 要件 5.2 の再現が何を緑とするか
- 緑＝「後始末は送信に依らず戻る（順序: 後始末の戻り < 送信の戻り）」「送信は UI スレッドが次にメッセージを取り出した時点で返る（10 秒の上限で切れたら赤）」。あわせて本番ソースの同期の送信の許可表（`shiori-host32-ipc/src/lib.rs` の `SendMessageTimeoutW(` ×1・`shiori-host32-host/src/parent_window.rs` の `#[cfg(test)]` の `SendMessageW(` ×1）を判定する検査を常設する（表に無い当たりも当たりの無い行も赤）。

#### 追加の決定: LogSink へ倒れた腕の App スコープの置き場
- テストで倒れた先を成功させる（`FakeShiori::BalloonMissing`）と、`ghost_boot_options` の `default_app_profile_dir()` が実行体の隣の実 profile へ書く。結線の入力の `app_profile_dir` を写して渡す（本番の値は同じ・要件 4.7 の「LogSink の起動そのものを変えない」は守る）。

#### 追加の決定: 倒れた時点の `warn!` の置き場＝`boot_first_ghost`
- `boot_ghost` は argv を知らない（argv では印に触れないので「印を残す」とは書けない）。`boot_first_ghost` が戻りの `logsink_fallback()` と文脈の argv を見て 1 件（argv なら `debug!`）。

### 10.3 危うさと緩和
- 案 A の前提（終わらせると `SendMessageTimeoutW` が戻る）— `terminator_tests.rs` で別プロセスの窓に対して固定（`SMTO_ABORTIFHUNG` の 5 秒より短く戻ることで理由を判定）＋実機サインオフ。崩れても悪化はしない（待ちが今日どおり長くなるだけで印は残る）。
- 接続前（HELLO 待ち）に上限に達する — 解く手が無く記録だけ。補助プロセスは job の道連れで終わる。
- 見張りのスレッドの取り残し — `CutGuard::finish` が必ず join する。`shutdown_within` は `finish` を呼んでから戻る。
- 既存テストの署名の追随（`settle_session_mark` 6 か所・`spawn_shiori_actor` 3 か所）— 機械的。振る舞いの判定は変えない（要件 7.6）。

### 10.4 参照
- [WM_ENDSESSION](https://learn.microsoft.com/en-us/windows/win32/shutdown/wm-endsession)・[Shutdown Changes for Windows Vista](https://learn.microsoft.com/en-us/windows/win32/shutdown/shutdown-changes-for-windows-vista) — 5 秒の猶予（5 章）。
- [TerminateProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess) — 既に終わったプロセスには `ERROR_ACCESS_DENIED`（冪等に畳む根拠）。
- [SendMessageTimeoutW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagetimeoutw) — 宛先の窓が無ければ 0 で戻る・`SMTO_ABORTIFHUNG` は相手が応答なしと判定されたときだけ打ち切る。
- 正典 [`OnClose`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnClose:1)・[`OnBoot`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBoot:1)（要件の「正典の位置づけ」）。
