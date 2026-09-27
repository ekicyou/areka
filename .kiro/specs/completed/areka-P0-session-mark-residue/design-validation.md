# 設計レビュー: areka-P0-session-mark-residue

> 2026-09-27・`/kiro-validate-design`（非対話・サブエージェント）。対象は `design.md`（2026-09-27 生成）・`requirements.md`（要件 8.1 で T＝3 秒確定）・`research.md` §1〜10・`brief.md`・steering。設計が「既存コードはこうなっている」と述べた箇所は、すべて本ブランチのソースを Grep／Read で突き合わせた（下の「確認したこと」）。

## レビュー要約

設計は 3 つの穴（OS の終了で SHIORI を待ちすぎる／LogSink へ倒れた起動の印が消える／後始末の join と同期の送信の重なり）を、既存の終了経路と印の判定を延ばす形で閉じており、切替・メニュー・強制退避・`WM_CLOSE`・smoke の経路と `msg.rs`・`events.rs`・`ghost_switch.rs`・`menu/`・`areka-parsers/` に触らないことを構造で保証している。設計が根拠にした既存コードの記述（関数・定数・依存の向き・`windows` クレートの機能）は全件ソースと一致した。残る論点は、穴 1 の前提を固定するテストの判定の仕方と、テストの手動の打ち切り口の順序の 2 点で、どちらも設計の骨格を変えずに直せる。

## 確認したこと（ソースとの突き合わせ）

| 設計の主張 | 確認結果 |
|---|---|
| `crates/areka/src/session_end.rs` の `on_os_session_end` が `started` を取り、`run_ghost_quit_phase` → `quit_app(SessionEnd)` → `GhostSession::shutdown(CloseReason::System)` → `settle_session_mark(&mark, true, down_ok)` → `info!(os_session_end_done, ms, down_ok)` | 一致（`SessionEnded` の読み捨て・`SwitchInFlight` の取り下げも設計どおり） |
| `crates/areka/src/main.rs` の `session_mark_verdict(first, argv_session, run_ok, down_ok)` の順序＝argv → 出所なし → `fault`／`switch_fatal` → `run_failed` → `down_failed` → 消す。`ExitOrigin` は網羅の match。`MarkInputs { app_profile_dir, argv_session, first }` | 一致。設計の新しい順序（`logsink_fallback` を出所なしの前・`session_end_deadline` を `run_failed` と `down_failed` の間）は既存の「時系列で最初の理由」の規則に沿い、`main_session_mark_tests.rs` の `session_mark_verdict_table` の既存 17 行の結論を変えない |
| `crates/areka/src/ghost_session.rs` の `boot_ghost` が `boot_wired` の失敗で `ghost_boot_options(ghost_root, helper_exe)` → `boot_with_kanade_stop` へ倒れ、成功で `info!`＋`on_boot_ok`、失敗で `is_benign_boot_error` により `warn!`／`error!`、どちらも `GhostSession { seriko: None, loop_ticker: None, .. }` を返す。`boot_ghost_strict` は `boot_wired` だけ | 一致 |
| `crates/areka/src/boot_config.rs` の `ghost_boot_options` が `ShioriWiring::Helper { helper_exe }` と `app_profile_dir: Some(default_app_profile_dir())` を入れる | 一致（設計の「結線の入力の `app_profile_dir` を写して渡す」は妥当。テストの土台 `SwitchRig` は `helper_exe` に実在しないパスを渡すので、倒れた先の SHIORI の接続は速く失敗し決定論が保たれる） |
| `crates/areka-kanade/src/shiori/real.rs`: `ShioriBackend` は `get`／`notify`／`unload`／`status`／`on_idle` の 5 つ、`ShioriConnection { window, helper, negotiator }`、`spawn_shiori_actor(connect, on_down) -> (Sender<ShioriMsg>, ActorHandle)`、`run_shiori_loop(rx, backend, on_down)`。host32 の型を import するのは `real.rs` だけ（`shiori/mod.rs` の説明・`areka-kanade/Cargo.toml` の注記） | 一致。`send_request` は shiori のアクター自身のスレッドで `SendMessageTimeoutW` に入って止まる（`parent_window.rs` の `send_request`＝pump もハートビートも起こさない）ので、「今の呼び出し」をアクターが書く形で足りる |
| `crates/areka-kanade/src/actor.rs` の `round_trip` が `reply_rx.recv()`（無期限）で待つ＝期限をメッセージで運んでも既に待っている往復は切れない | 一致 |
| `crates/areka-kanade/src/schedule/mod.rs` の `force_quit` が `[OnClose の NOTIFY, Action::ShioriUnload]` を返し `Unloading{Forced}` へ、「Unloading 中の応答は Unload 完了として扱う」 | 一致 |
| `crates/shiori-host32-host/src/lifecycle.rs`: `UNLOAD_ACK_TIMEOUT`＝30 秒・`EXIT_OBSERVE_TIMEOUT`＝10 秒・`request_clean_shutdown` は先頭で `status()` の `Exited` を短絡・`terminate(&mut self)`・`Drop` で `terminate` | 一致 |
| `crates/shiori-host32-host/src/process_host.rs`: `HelperHandle { child: Child, helper_hwnd, _job }`・`terminate(&mut self)`＝`Child::kill`（`InvalidInput` を `Ok` に畳む）・`REQUEST_TIMEOUT`＝60 秒・`LOAD_ACK_TIMEOUT`＝30 秒・環境変数 `AREKA_SHIORI_REQUEST_TIMEOUT_MS`・`windows` 依存は `job.rs` に隔離 | 一致（`Child` は `&mut` でしか kill できず、別スレッドから終わらせる口は無い＝新設の理由は正しい） |
| `crates/shiori-host32-host/src/client.rs` の `effective_timeout` が環境変数 `0` を `Duration::MAX` に写す | 一致 |
| `crates/shiori-host32-ipc/src/lib.rs` の `SendMessageTimeoutW`（要求方向 `SMTO_ABORTIFHUNG`・戻り 0 で `ERROR_TIMEOUT` だけ `Timeout`、他は `SendFailed`） | 一致 |
| 本番ソースの同期の送信は `shiori-host32-ipc/src/lib.rs` の `SendMessageTimeoutW(` 1 か所と `shiori-host32-host/src/parent_window.rs` の `SendMessageW(`（テスト側）だけ | 一致（他の当たりは `tests/`・`_tests.rs`・`pilot/examples/` の中＝設計の許可表の除外に入る） |
| `Win32_System_Threading` はルート `Cargo.toml` の `[workspace.dependencies.windows]` の features にあり、`shiori-host32-host/Cargo.toml` は `workspace = true` で継ぐ（member 側の features は追加で、上書きではない） | 一致＝機能も依存クレートも足さずに `TerminateProcess` が書ける（要件 3.5） |
| 境界: `emo2_boot/ghost_switch.rs`・`change_cue.rs`・`menu/`・`areka-parsers/`・kanade `events.rs`・`msg.rs` の変種に設計上の変更なし。切替・メニュー・強制退避の期限の定数と `GhostSession::shutdown`・`GhostRuntime::shutdown` は署名・手順とも不変。T＝3 秒は `SESSION_END_SHIORI_LIMIT` の定数、テストは `end_session_within(world, limit)` の引数、環境変数は足さない | 設計の記述どおり（File Structure Plan の「変更」一覧にも上記のファイルは無い） |

## 重要な問題（最大 3 件）

### 問題 1: 穴 1 の前提を固定するテストが、壁時計と libtest の再実行に頼っている

- **懸念**: `terminator_tests.rs` ⑵（「補助プロセスを終わらせると止まっていた `SendMessageTimeoutW` が戻る」の前提）は、⒜ テストの実行体を `current_exe()` で子として起こし環境変数で「窓を作って待つ」入口へ入れる、⒝ 送信が **2 秒以内**に戻ることで「`SMTO_ABORTIFHUNG` の打ち切り（5 秒以上）ではなく終了で戻った」と判定する、の 2 つに依る。⒜ は libtest の中に子の入口を作る配管（`--exact`・`--nocapture`・標準出力から HWND を読む）が要り、⒝ は時間で理由を判定するので、Defender の再スキャンなどで遅れたときに偽の赤になる（過去の知見: 協調テストループが飢餓する）。
- **影響**: 設計全体が載っている前提のテストが不安定だと、崩れたときに気付けない（要件 7.1・7.5 の「常設で赤になる」が弱る）。
- **提案**: 前提のテストの送信の旗を `SMTO_NORMAL`（`SMTO_ABORTIFHUNG` なし）・上限を長め（例 30 秒）にする。すると戻る理由は「期限切れ（`GetLastError()==ERROR_TIMEOUT`）」か「終了」の 2 つだけになり、**戻り 0 かつ last error が `ERROR_TIMEOUT` でない**ことで理由を時間に依らず判定できる（本番の旗は変えない・前提は旗に依らない）。子プロセスは、既に i686 成果物を前提に走る `crates/areka-kanade/tests/kanade/real_helper_test.rs` の段に「本物の補助プロセスへ要求を送っている最中に `HelperTerminator::terminate` する」1 本を足す方が、`STATIC` 窓の代役より前提を直接に固定できる（x64 だけの段には ⒜ を残してよい）。
- **Traceability**: 要件 1.3・2.4・7.1・7.5
- **Evidence**: design.md「HelperTerminator」Implementation Notes ⑵、「Testing Strategy › Unit（kanade・host32）」、「Open Questions / Risks › 案 A の前提」

### 問題 2: `cut_now` は見張りが張られていないと何もしないので、「後始末に入る前から待っていた往復」のテストで順序の穴がある

- **懸念**: 設計の `ShioriProbe::cut_now()` は「張られている見張りへ `CutNow` を送る（張られていなければ何もしない・`debug!`）」。一方 `session_end_deadline_tests.rs` ⑵ の `hold_at(Get("OnBoot"))` の回は、偽の SHIORI が **`end_session_within` を呼ぶ前から**固まっており、補助のスレッドが `holding()` を見てから `cut_now` を呼ぶと、`arm` より先に呼んで空振りしうる。空振りすると見張りは 1 時間などの大きな `limit` を待ち続け、テストが止まる。
- **影響**: 要件 1.1 で最も難しい段（既に待っている往復）のテストが、スレッドの順序次第で止まるか通るかが変わる（決定論でない）。
- **提案**: どちらか 1 つ。⒜ `cut_now` を**粘る**形にする（見張りが無ければ「切る予約」を `ProbeInner` に置き、次の `arm` が予約を見つけたら即発火する）。⒝ `ShioriProbe` に「張られた」を待つ口（`wait_armed()`・テスト専用）を足し、補助のスレッドは `holding()` と `armed` の両方を待ってから `cut_now` を呼ぶ。⒜ の方が口が増えず、順序に依らない（design の「解く手が先に呼ばれていれば待たずに通る（順序に依らない）」と同じ考え）。
- **Traceability**: 要件 1.1・1.3・7.1 ⑵
- **Evidence**: design.md「ShioriProbe」Contracts（`cut_now` の説明）、「Testing Strategy › Integration › ⑵ 段ごとに固まる 4 通り」

### 問題 3: 上限ちょうどで後始末が終わる回と `idle` の段の扱いが言葉になっていない

- **懸念**: 見張りは `recv_timeout` の `Timeout` で発火する。後始末が T の直前に終わり `CutGuard::finish` が送信端を落とすのと `Timeout` が同時に起きると、見張りは `Done` を見ずに「切る」へ進み、既に終わりつつある補助プロセスへ `TerminateProcess`（冪等で `Ok`）→ `warn!(shiori_wait_cut, stage="idle")` → `ShioriCut` を返し、きれいに終わった回に印が**残る**（`Keep("session_end_deadline")`）。また `idle` の段（SHIORI の呼び出しの外で時間を使い切った）は、固まっていない健全な補助プロセスを終わらせることになり、UNLOAD がまだなら SHIORI の保存が失われる。要件 1.1 の「後始末に入った時点から T」に従うのは正しいが、記録の文言と要件 2.6（T に達しなかったときは今日どおり）との境が設計に書かれていない。
- **影響**: 頻度は低い（健全な後始末は数十 ms）が、「T に達しなかったのに印が残る」形は要件 2.6 の否定になり、次の起動が理由なく `halt` になる。
- **提案**: 見張りが `Timeout` を受けたら**切る前に `try_recv` で `Done` を 1 度だけ確かめ**、`Done` なら何もせず終わる（数行）。`idle` の段は「SHIORI は待っていないが上限に達した」と分かる文言にし、`probe_tests.rs` に「`finish` と `Timeout` が同時のときは切らない」判定を 1 本足す。要件 2.6 との対応を design の流れの決めごとに 1 行足す。
- **Traceability**: 要件 1.5・2.2・2.6・6.3
- **Evidence**: design.md「System Flows › 上限に達したときの後始末 › 流れの決めごと」（発火の 2 つの口・段の読み方）、「ShioriProbe › State Management」

## 設計の強み

1. **他の経路は 1 行も変わらないことを構造で保証している**。期限をメッセージで運ぶ案（B・C）を退け、見張りを `shutdown_within` だけが張る形にしたので、`msg.rs`・`ShioriBackend` の既存 5 メソッド・`GhostSession::shutdown`・`GhostRuntime::shutdown`・期限の定数・環境変数の意味がすべて不変。並走の `ghost-change-name-resolution` と後続 3 本の持ち物（`msg.rs`・`events.rs`・`ghost_switch.rs`・`menu/`）に当たらない。要件 3.3「1 ミリ秒も違わない」を、ソース構造の検査（`shutdown_within(`・`.arm(` の呼び手を判定）で固定する案も方針「検査は表示するだけでなく判定させよ」に沿う。
2. **印の判定が 1 か所に残り、材料が型で運ばれる**。`session_mark_verdict` に `logsink_fallback` と `Teardown { run_ok, down_ok, shiori_cut }` を足す形は、既存の「時系列で最初の理由」の規則と `ExitOrigin` の網羅の match を保ち、位置引数の bool を 6 つ並べる形（両枝が同じ誤りを書いても気付けない）を避けている。LogSink へ倒れた欄を `GhostSession`（いま動いている単位）に付けることで、要件 8.3（切替先へ引き継がない）を `ghost_switch.rs` に触らずに満たす。

## 補足（重要な問題には数えない）

- `SwitchRig` の `FakeShiori::BalloonMissing`（倒れた先が成功）では、倒れた先の SHIORI の接続の失敗で kanade が `Fault` で止まる知らせが**非同期に**届く。1 周のテストは `first` の値（`OsClose` か `KanadeStopped(Fault)` か）を判定に使わないこと（印の理由は順序により `logsink_fallback` で安定する）。
- 要件 7.1 ⑵ は「UNLOAD の応答の段」と「終了の観測の段」を別に挙げるが、設計 D4 は `unload` 1 語にまとめ、偽の SHIORI の `hold_at(Unload)` も `unload()` 全体を固める。終了の観測の途中で補助プロセスが消えたときの脱出は本物の `request_clean_shutdown` の `status()` の `Exited` に依るので、対応表（7.1 ⑵ の行）に「`Unload` の固まりで両段を代表する。観測の段の脱出は `lifecycle.rs` の既存の短絡と同じ道」と明記しておくと、後から読む人が欠けと誤解しない。

## 最終判定

**GO**

- **理由**: 既存アーキテクチャとの整合（依存の向き `shiori-host32-host` → `areka-kanade`（`real.rs` だけ）→ `areka-ghost` → `areka`・テストは本番ファイルの兄弟ファイル・ログ無しの失敗経路なし）に反する点は無く、要件 1〜8 はすべて部品と流れに対応づけられ、設計が根拠にした既存コードの記述は全件ソースと一致した。上の 3 件はいずれもテストの判定の仕方と見張りの数行の決めごとで、設計の骨格（見張り＋外から解く・印の判定 1 か所・欄は単位に付ける）を変えずに設計ディスカッションで確定できる。
- **次の一歩**: 設計ディスカッション（`/kiro-design-discussion`）で問題 1〜3 の扱いを決め、design.md の該当節（前提のテストの判定・`cut_now` の形・見張りの `Done` の再確認と `idle` の文言）に反映してから `/kiro-spec-tasks areka-P0-session-mark-residue` へ進む。
