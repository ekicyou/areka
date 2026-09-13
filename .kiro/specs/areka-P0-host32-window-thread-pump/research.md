# Gap Analysis: areka-P0-host32-window-thread-pump

> 作成 2026-09-11（`kiro-validate-gap`・要件確定後・設計前）。行番号は本日の HEAD `67e0a4d3` 時点。各引用は「何の定義行か」を併記する（自分の編集で行番号がずれても追えるように）。

## 1. 要約

- **実装の対象は 2 ファイル＋新しい檻 1 ファイル**で足りる。待ちの形は `crates/areka-kanade/src/shiori/real.rs` の受信ループ `run_shiori_loop`（`rx.recv()` 1 行）に閉じており、窓のメッセージを汲む部品は `crates/shiori-host32-host/src/parent_window.rs` に既存の pump 経路（`MessageLoop::run`＋heartbeat）がある。
- **「入力と窓を同時に待つ」前例はリポジトリ内に 2 つ既にある**（`areka-actor` の `spawn_ui`＝`async_channel`＋`spawn_local`／`tests/spike_ui_pump.rs`＝`MessageLoop::run`＋heartbeat）。ただしどちらも inbox が std `mpsc` でない、または別スレッドからの起こしを要する。**shiori アクターの inbox は std `mpsc::Sender<ShioriMsg>` で公開されており（`spawn_actor` が生成）、std の channel は待機ハンドルも送信フックも出さない**——ここが唯一の構造的な穴である。
- 候補は 5 つ。⑴ **案 0＝交互待ち**（`recv_timeout(T)`＋`PeekMessage` 掃き・新しい型もスレッドも Cargo 変更も要らない・T は OS の 5 秒閾値の内側）、⑵ **案 1b＝送信端に起こし（`PostMessageW`）を足す薄い包み**（真の同時待ち・だが公開型 `Sender<ShioriMsg>` が変わり Requirement 7.3 の編集集合を超える）、⑶ **案 1a＝Event＋`MsgWaitForMultipleObjectsEx`**（1b と同じ包みが要り、さらに host32 の `windows` feature 追加が要る）、⑷ **案 1d＝中継スレッド**（公開型不変・スレッド 1 本＋1 ホップ）、⑸ **案 2＝窓の専用スレッド**（`ParentShared` の `Cell`／`RefCell` を跨スレッド化・`Shiori3Client::new(&ParentMessageWindow)` の型変更が `client.rs` に及び **W13 併走 `charset-canon` と衝突**・推奨しない）。
- **檻は kanade 側の兄弟テストファイル**に置くしかない（受信ループは kanade にあり、host32 は kanade に依存できない）。実 `ParentMessageWindow`＋fake backend で本番の `run_shiori_loop` を駆動でき、送出は dev-dep 済みの `shiori-host32-ipc` の `send_copydata`（`SMTO_ABORTIFHUNG`）／`send_copydata_response`（旗なし）が使える。遅い檻はワークスペース全体テストの壁時計を **約 +42 秒**伸ばす（cargo はテストバイナリを直列に走らせる）。
- 設計で決めるべき事項は §7 に 9 件。特に **①案 0 を「同時待ち」の要件文の下で許すか**、**②案 1 系なら 7.3 の編集集合を `actor.rs`／`areka-ghost/runtime.rs` まで広げるか**、**③ `shiori-host32-ipc` に残る「待機中はメッセージを取り出さない」の説明文 2 か所をどう扱うか**（Requirement 3.3／7.3 は ipc に触れないと言い、7.1 は古い説明を書き換えよと言う）。

## 2. 現状調査（file:line）

### 2.1 待ちの形（変える対象）

| 何 | 場所 | 中身 |
|---|---|---|
| 受信ループ本体 | `crates/areka-kanade/src/shiori/real.rs:172-245` `fn run_shiori_loop` | `while let Ok(msg) = rx.recv()`（`:179`）。到達ごとに `backend.status()` で死活確認（`:181-194`）→ `Request`／`Unload`／`Close` の dispatch。終了経路は `Close` の `return`（`:241`）と `recv` の `Err`（全 Sender drop）の 2 つ |
| アクター起動 | `real.rs:263-287` `fn spawn_shiori_actor` | `spawn_actor("shiori", ..)`。connect closure をアクタースレッド上で 1 回実行し、失敗なら `event="connect_failed"` の `error!`＋`ShioriDown`（`:275-282`）——Requirement 6.2 の「既存の接続失敗と同じ」経路はここ |
| inbox の型 | `crates/areka-actor/src/spawn.rs:86-91` `fn spawn_actor` | `mpsc::channel::<M>()` を内部生成し `Receiver` を body へ move・`Sender<M>` を返す。**std の channel は待機ハンドル（HANDLE）も送信フックも公開しない** |
| inbox の消費者 | `crates/areka-kanade/src/actor.rs`（`Sender<ShioriMsg>` 8 か所: `spawn_kanade` `:70`・`spawn_kanade_with_stop_sink` `:94`・`:158`・`:227`・`round_trip_request` `:298`・`round_trip_unload` `:367`・`round_trip` `:376`・`send_shiori` `:411`）／`crates/areka-ghost/src/runtime.rs:593`（`spawn_shiori_actor` の戻り）・`:605`（`spawn_kanade_with_stop_sink` へ渡す）／テスト: `actor_tests.rs:47`・`tests/kanade/common/common_mock_shiori.rs`・`tests/kanade/real_helper_test.rs:194` | 型を変えると触る先。`areka-ghost/runtime.rs` は W14 `property-ipc-transport` の触る先（W13 では誰も触らない） |
| ログ規約 | `real.rs:184-188`・`:204-208`・`:222-226`・`:234-238`（`target: "shiori-actor"`・`event = ".."`） | Requirement 6.3 の「既存のログ規約」。e2e 手順書 §5.7 が `event=unload_clean`／`unload_failed`／`connect_failed`／`helper_exited` を grep する |
| 終了挨拶中に往復が止まる根拠 | `crates/areka-kanade/src/schedule/close.rs:55`（`ClosePending` の `Tick` は「`last_now` 更新のみ・close 中は pump しない」） | 本仕様の前提（変えない） |

### 2.2 窓と pump（既存部品）

| 何 | 場所 | 中身 |
|---|---|---|
| 窓の生成 | `crates/shiori-host32-host/src/parent_window.rs:224-236` `ParentMessageWindow::create` | `wintf_winmsg_executor::util::Window::new(WindowType::MessageOnly, ParentShared, closure)`。`Window<S>` は `!Send`（`PhantomData`＋HWND）・Drop で `DestroyWindow`（executor `util/window.rs:67-69`） |
| 窓の状態 | `parent_window.rs:120-140` `struct ParentShared` | `helper_hwnd: Cell<Option<u32>>`・`response_slot: ResponseSlot`・観測カウンタ 3 つ（`Cell<u64>`）。`ResponseSlot` は `RefCell<Option<Vec<u8>>>`（`crates/shiori-host32-ipc/src/lib.rs:220-221`）。**いずれも `!Sync`**——窓のスレッドと送出のスレッドを分ける案 2 はここを全部変える |
| 握手 pump | `parent_window.rs:251-298` `pump_until_hello_or` | heartbeat スレッド（`:266-282`・25 ms 間隔で自窓へ `PostMessageW(WM_NULL)`）＋`MessageLoop::run`（`:284-289`・filter で HELLO 確定 or 期限を見て `quit()`）。rustdoc `:12-13`「heartbeat・pump フェーズ専用」 |
| 送信パス | `parent_window.rs:318-343` `send_request` | ゲート→`ipc_send_request`（`slot.clear → SendMessageTimeoutW(REQUEST, SMTO_ABORTIFHUNG) → slot.take`）。rustdoc `:312-316`「heartbeat 不干渉・pump ループも heartbeat も起動しない」 |
| 本番 connect | `crates/areka-ghost/src/shiori_wiring.rs:39-86` `real_connect` | `create` → `spawn` → `pump_until_hello_or(5 s)` → `send_request(Load, 30 s)`。窓はアクタースレッド上で作られ `ShioriConnection` に入る |
| 実行器のループ | executor 0.0.5 `src/lib.rs:182-203` `MessageLoop::run_loop` | `GetMessageW` → filter → `DispatchMessageW`。`quit()` は**次のメッセージが来た後**に効く（`while !self.quit.get()`）＝無入力では抜けない（だから heartbeat が要る）。`block_on`（`:129-146`）は future 完了で `quit()`。`spawn_local` の起こしは executor の thread-local 窓へ `PostMessageW(MSG_ID_WAKE)`（`:192-194` で filter から保護） |
| 実行器 API の使用実績 | `parent_window.rs:44-45`（`Window`／`MessageLoop`／`FilterResult`）・`crates/areka-actor/src/ui.rs:99`（`spawn_local`）・`crates/areka-actor/tests/spike_ui_pump.rs:149-156`（`MessageLoop::run`＋`PostThreadMessageW` heartbeat） | 「入力（channel）と窓を同時に待つ」の in-repo 前例。`spawn_ui`（`ui.rs:82-126`）は `async_channel::unbounded` の `recv().await` を `spawn_local` に載せ、起こしは async-channel の waker → executor の `PostMessageW` に**委ねる**（手組みしない・DD-9） |

### 2.3 6.11 の旗と較正（維持するもの）

| 何 | 場所 |
|---|---|
| 旗の純関数 | `crates/shiori-host32-ipc/src/lib.rs:284-289` `send_flags`（`Request → SMTO_ABORTIFHUNG`・`Response → SMTO_NORMAL`）。檻は同 `:598-627` `send_flavor_tests` |
| 打ち切りの見え方 | 同 `:352-397` `send_copydata_with`——`SendMessageTimeoutW` の戻り 0 は timeout／hung 中断の区別なく **`IpcError::SendFailed`**（`:392-395`）。往復では `send_request`（`:414-431`）が `slot.take()` 空を `IpcError::Timeout` へ写す |
| 遅い檻の原型 | `crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs`（291 行・x64 で走る・`IDLE 20 s`／`ROUND_TRIP_TIMEOUT 5 s`／`CAGE_BOUND 90 s`・`:197-291` 本体）。`main.rs:585-586` が `#[path]` で取り込む。**見出し `:5-6`「本番では shiori アクターが `recv()` で待つ」・`:32`「本番の shiori アクターと同じ『待機中に pump しない』姿」が Requirement 7.1 の書き換え対象** |
| 較正値の出所 | `.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/acceptance-record.md` §13.2 行 8（「プロセス生存 20〜30 秒を過ぎた後で、対象スレッドが 14 秒以上メッセージを取り出していないこと」・コミット `c215f6ae`）・行 10（本仕様の起票行・`real.rs:179` を指す） |

### 2.4 依存と feature（何が使えて何が足りないか）

| 何 | 実測 |
|---|---|
| `crates/areka-kanade/Cargo.toml` | `[dependencies]` は `areka-actor`／`areka-talk`／`shiori-host32-host`／`tracing`／`thiserror` のみ（**`windows` 非依存**・Requirement 7.6 の現状）。`[dev-dependencies]` に **`shiori-host32-ipc`**（＝檻から `send_copydata`／`send_copydata_response`／`MsgTag` が使える）と `log-capture-kit` |
| `crates/shiori-host32-host/Cargo.toml` の `windows` features | `Win32_System_DataExchange`・`Win32_UI_WindowsAndMessaging`・`Win32_Foundation`・`Win32_System_JobObjects`・`Win32_Security` |
| `MsgWaitForMultipleObjectsEx`／`PeekMessageW`／`PostThreadMessageW`／`GetQueueStatus` | `windows-0.62.2` の **`Win32_UI_WindowsAndMessaging`**（host32 で有効済み・追加不要） |
| `CreateEventW`／`SetEvent`／`WaitForSingleObject`／`GetCurrentThreadId` | 同 **`Win32_System_Threading`**（host32 で**未有効**・案 1a はここを足す＝`Cargo.toml` の編集が要る） |
| `async_channel` | `areka-actor` の依存（`crates/areka-actor/Cargo.toml`）。kanade からは見えない（re-export なし） |
| `event-listener` | host32 の依存（`Cargo.toml`）だが `parent_window.rs` は未使用（grep 0 件） |

### 2.5 テストの置き方（既存の慣行）

- 兄弟テストファイルは `#[cfg(test)] #[path = "..."] mod ..;`（`real.rs:289-291`・helper `main.rs:568-586`）。`real_tests.rs` は 669 行——新しい檻は**別の兄弟ファイル**へ（1,000 行規律）。
- fake backend＋本番 runner の駆動は `real_tests.rs:36-140`（`FakeBackend`／`boxed`／`round_trip_via_runner`）と `:603-623`（`all_senders_dropped_terminates_runner`＝`run_shiori_loop` を素のスレッドで走らせる形）に既にある。
- 窓生成テストは直列化する（`crates/shiori-host32-host/src/lifecycle.rs:524` `WINDOW_TEST_SERIAL`・`parent_window.rs:459-466` の説明「同一プロセスで 2 組の message-only 窓を同時生成すると 2 組目が `WindowCreationError`」）。kanade の lib テストバイナリに窓を作る檻を 2 本置くなら同じ直列化が要る。
- 実 helper を使うテストは env-gate（`tests/kanade/real_helper_test.rs:6-12`・`HOST32_PASTA_DLL`）・本仕様の檻は使わない（Requirement 4.1）。

### 2.6 Requirement 7.1 の書き換え対象（裏取り済み）

| ファイル | 行 | 文言 |
|---|---|---|
| `crates/shiori-host32-host/src/parent_window.rs` | `:12-13`（module doc 2 項） | 「heartbeat・pump フェーズ専用」 |
| 同 | `:47-50`（`HEARTBEAT_INTERVAL` の doc） | 「pump フェーズ専用の起こし用」 |
| 同 | `:258-260`（`pump_until_hello_or` の doc） | 「heartbeat・pump フェーズ専用」 |
| 同 | `:312-316`（`send_request` の doc） | 「本関数は pump ループも heartbeat スレッドも起動しない…heartbeat は `pump_until_hello_or` の pump フェーズ専用」 |
| `crates/areka-kanade/src/shiori/real.rs` | `:159-164`（`run_shiori_loop` の doc） | 「`ShioriMsg` を blocking `recv` で受け」 |
| `crates/areka-kanade/src/shiori/mod.rs` | `:14-15` | 「単一 inbox（`Receiver<ShioriMsg>`）を専有スレッドで受け」（待ちの形には触れていない・要確認程度） |
| `crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs` | `:5-6`・`:32` | 「本番では shiori アクターが `recv()` で待つ」「本番の shiori アクターと同じ『待機中に pump しない』姿」 |
| **`crates/shiori-host32-ipc/src/lib.rs`** | **`:328-329`**（`send_copydata_response` の doc）・**`:591-593`**（`send_flavor_tests` の doc） | 「待機中はメッセージを取り出さないため OS からは『応答なし』に見える」——**Requirement 3.3／7.3 は ipc に触れないと言う**（§7 の判断事項 ③） |

## 3. 要件→資産マップ

| Req | 必要なもの | 既存資産 | ギャップ |
|---|---|---|---|
| 1.1〜1.3 待機中の応答性 | 待機中に窓のメッセージを取り出す経路 | `MessageLoop::run`／heartbeat（握手時のみ）・`PeekMessageW`（feature 有効） | **Missing**: 定常時の pump。std `mpsc` に待機ハンドルが無い（**Constraint**） |
| 1.4 到着順・取り落とし無し | inbox の FIFO | std `mpsc` が保証 | なし（案 1d の中継は 1 本の FIFO を挟むだけで順序を保つ） |
| 1.5 往復の所要に差を出さない | inbox 到着で即起きる | `recv`／`recv_timeout` は送信で即起きる | 案 0 は inbox 側の遅延 0（窓側のみ ≤T）。案 1a/1b は起こし 1 回分（µs） |
| 1.6 往復中の上限復帰 | 既存 | `send_request`（`SMTO_ABORTIFHUNG`＋timeout）・`UNLOAD_ACK_TIMEOUT 30 s`・`EXIT_OBSERVE_TIMEOUT 10 s`（`lifecycle.rs:58-63`） | なし |
| 2.1〜2.4 受理規約 | dispatch 本体不変 | `run_shiori_loop:195-243` | なし（待ち方だけ差し替える） |
| 2.3 全送信端 drop で終了 | 切断の検出が窓待ちに妨げられない | `recv_timeout` は `Disconnected` を即返す | 案 1b は**Sender drop が起こしを出さない**ため包みの `Drop` で起こす必要（**Unknown→設計**）。案 1d は中継が `Err` を見て最終起こしを出せる |
| 2.5〜2.7 握手・往復・shutdown | 既存 | `pump_until_hello_or`／`send_request`／`request_clean_shutdown`（`lifecycle.rs:203-236`） | なし（案 2 のみ全面改変） |
| 2.8 既存テスト無改変 | — | `real_tests.rs`・host32 `src/` 内 26 本＋`tests/` 5 本・helper 4 本 | 案 2 は `window_tests`／`lifecycle::tests` の窓生成前提が崩れる（**Constraint**） |
| 3.x 旗の維持 | — | `send_flags`＋檻 | なし（ipc 非接触） |
| 4.1〜4.2 x64 偽境界で本番経路 | 実窓＋fake backend＋本番 runner | `ParentMessageWindow::create`（pub）・`FakeBackend`・`run_shiori_loop`（同ファイル内 private→兄弟テストから `super::*` で可） | なし。**窓を作る檻は kanade の lib テストバイナリでは初**（直列化ロック新設） |
| 4.3 速い檻 | 別スレッドからの同期送出 | `shiori_host32_ipc::send_copydata_response`（`SMTO_NORMAL`）が dev-dep で使える | 直す前＝上限まで待って `SendFailed`（赤）。上限は 5 秒制約から ≤2 秒程度に |
| 4.4〜4.5 遅い檻 | 20 s → 往復 → 20 s → `SMTO_ABORTIFHUNG` 送出 | `send_copydata`（Request 旗）・原型は helper の檻 | 直す前＝2 回目が即 `SendFailed`（赤）。**壁時計 +約 42 秒**（cargo はバイナリ直列） |
| 4.6 assert＋診断文 | — | helper 檻の `diag` 形（`:255-262`） | 写せる |
| 4.7 較正の証跡 | 直す前の赤 | — | **手順**: 檻を先に書いて赤を記録→直す→緑（タスク順で担保） |
| 4.8 時間の仮定を置かない | — | — | 案 0 は本番に周期 T を置く（檻の上限 > T が唯一の前提）。案 1 系は無し |
| 4.9 間欠赤を足さない | — | `zorder-chain-residue` A-2（`SPIN_WAIT` 型の壁時計期限） | 速い檻の上限は「届く／届かない」の二値で `SPIN_WAIT` 型ではない。負荷下で 2 秒待ちが延びる懸念（**Research Needed**） |
| 5.x 実機 | e2e 手順書 §5.7 | `lap-procedure.md` §5.7 の語の表 | なし（走行と記録のみ） |
| 6.1〜6.2 失敗経路 | 起こし手段の生成失敗の口 | `connect` の `Err` → `connect_failed`（`real.rs:275-282`） | 案 0 には失敗しうる生成が無い（`PeekMessageW` は失敗しない）。案 1a（Event 生成）・1b（HWND 未確定）・1d（スレッド生成）は失敗経路あり |
| 7.3 編集集合 | `real.rs`・`parent_window.rs`・兄弟テスト | — | **案 1a/1b/1c は `actor.rs`・`areka-ghost/runtime.rs`・テスト 3 本に及ぶ**（Constraint／判断事項 ②） |
| 7.4 1,000 行 | — | `parent_window.rs` 659・`real.rs` 291・`real_tests.rs` 669 | 檻は新しい兄弟ファイルへ |
| 7.6 kanade に `windows` を足さない | 汲む部品は host32 | `parent_window.rs`（`windows` 有効） | なし（部品は host32 に置く） |

## 4. 実装アプローチ候補

### 案 0: 交互待ち（`recv_timeout(T)` ＋ 窓の掃き）——最小

- **形**: `run_shiori_loop` の `rx.recv()` を `rx.recv_timeout(T)` に替え、`Err(Timeout)` のたびに host32 が提供する `pump_pending_messages()`（`PeekMessageW(PM_REMOVE)` → `TranslateMessage`／`DispatchMessageW` を空になるまで）を呼ぶ。`Err(Disconnected)` で従来どおり終了。
- **根拠**: OS の応答なし判定は「`GetMessage`／`PeekMessage` 系を一定時間呼んでいない」で決まる（`IsHungAppWindow` は 5 秒・`SMTO_ABORTIFHUNG` の実測は 14 秒）。T を 5 秒より十分小さく取れば（例 500 ms〜1 s）判定に落ちない。
- **編集集合**: `real.rs`（数行）・`parent_window.rs`（自由関数 1 つ・10 行程度）・新しい檻。**公開型・Cargo・スレッド数は不変**。Requirement 7.3／7.6 をそのまま満たす。
- **Requirement との突合**: 1.1〜1.3 ○（窓側の遅延 ≤T）・1.4/1.5 ○（inbox は送信で即起きる）・2.3 ○（`Disconnected` 即時）・6.1/6.2 は「失敗しうる生成が無い」ので空振り（記録のみ）・4.8 は本番に T という周期を 1 つ置く（檻は T に依存しない・上限 > T のみ）。
- **弱点**: 要件本文の「入力と窓のメッセージを**同時に**待つ」の字義には合わない（交互待ち）。将来の helper → areka 自発通知の遅延が最大 T。利用者から見える差は無い（往復は inbox 起点）。
- **ponytail の見立て**: 最初に成立する最短の形。T の根拠（5 秒閾値の内側）を定数 doc に残せば十分。

### 案 1a: `MsgWaitForMultipleObjectsEx` ＋ Event（brief の案 1 そのもの）

- **形**: アクタースレッドで `MsgWaitForMultipleObjectsEx(1, [event], INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE)` → 起きたら `PeekMessage` 掃き → `rx.try_recv()` を空まで。送信側は `tx.send(m)` の後に `SetEvent`。
- **要るもの**: ⑴ host32 に `Win32_System_Threading` feature（`Cargo.toml` 編集）、⑵ Event を持つ送信端の**包み型**（`Sender<ShioriMsg>` の公開型が変わる → `actor.rs` 8 か所・`areka-ghost/runtime.rs` 2 か所・テスト 3 本）、⑶ 包みの `Drop` で `SetEvent`（2.3 の切断検出を妨げないため）。
- **利点**: 真の同時待ち・遅延 0・スレッド増無し。**欠点**: 1b と同じ包みが要るうえ feature 追加が増える。1b に対する優位は無い。

### 案 1b: `MessageLoop::run` ＋ 送信端が `PostMessageW` で起こす

- **形**: 送信端の包み `ShioriSender { tx: Sender<ShioriMsg>, wake: Arc<AtomicU32 /* hwnd wire */> }`。`send` は `tx.send` の後に `PostMessageW(hwnd, WM_APP+n)`（HWND 未確定＝connect 前なら投げない。ループは待つ前に `try_recv` を空にするので取り落とさない）。アクター側は `MessageLoop::run(|ml, _| { drain try_recv; Close/Disconnected なら ml.quit(); Forward })`。窓も executor も既存（`spike_ui_pump.rs` と同型）。
- **要るもの**: ⑴ 包み型と公開型の変更（1a と同じ範囲）、⑵ `Drop` で最終起こし（2.3）、⑶ **`quit()` は次のメッセージ後にしか効かない**（`run_loop:185`）ので Close／切断時に自分へ 1 通投げる。`PostMessageW` は host32 が持つ（`parent_window.rs:43` で既に import）＝**feature 追加不要**。
- **利点**: 真の同時待ち・スレッド増無し・feature 不変。**欠点**: 公開型の変更が Requirement 7.3 を超える（`actor.rs`・`areka-ghost/runtime.rs`）。W13 の併走 spec はどちらも触らない（`kanade-boot-talkdone-drop` は `schedule/{boot,mod}.rs`・`charset-canon` は `shiori3.rs`／`client.rs`）が、7.3 の改訂が要る。
- **ponytail の見立て**: 「起こしを手組みする」形。既存の `spawn_ui`（1c）が同じことを async-channel に委ねている。

### 案 1c: `async_channel` ＋ `block_on`（`spawn_ui` の写し）

- **形**: inbox を `async_channel::unbounded` にし、アクタースレッドで `block_on(async { while let Ok(m) = rx.recv().await { .. } })`。起こしは async-channel の waker → executor の `PostMessageW`（既成・手組み無し）。
- **要るもの**: inbox の型が std `mpsc` でなくなる＝`spawn_actor` を使わない or areka-actor に別の spawn を足す（`areka-actor/src/lib.rs:73-76`「select／MPMC は 2 例目まで凍結・新規依存は開発者承認」）。kanade に `async-channel` 依存追加。公開型の変更範囲は 1b と同じ＋Cargo。
- **判定**: 起こしの品質は最良だが編集集合が最大。W13 に載せる形ではない。

### 案 1d: 中継スレッド（公開型不変・真の同時待ち）

- **形**: `spawn_shiori_actor` の body が connect 後に中継スレッド "shiori-relay" を起こし、`for m in rx { inner_tx.send(m); waker.wake() }`（`waker` は host32 が返す `Send` な `WindowWaker { hwnd_wire }`・`PostMessageW`）。切断で最終起こし。アクターは 1b と同じ `MessageLoop::run`＋`inner_rx.try_recv()` 掃き。
- **利点**: `Sender<ShioriMsg>` 不変（7.3 をそのまま満たす）・feature 不変・真の同時待ち。**欠点**: スレッド 1 本＋inbox に 1 ホップ（µs）・失敗経路が 1 つ増える（スレッド生成失敗→6.2 の経路へ）。
- **判定**: 「7.3 を守ったまま同時待ち」を要求されたときの落とし所。案 0 より複雑。

### 案 2: 窓を専用スレッドへ分離（brief の案 2）

- **必要な改変（実測）**: ⑴ `ParentShared` の `Cell`／`ResponseSlot` の `RefCell` を跨スレッド可能に（`response_slot` は **ipc crate の型**＝Requirement 3.3 に抵触、または host32 側に別の受け皿を新設）、⑵ `Window<S>` は `!Send` で Drop が `DestroyWindow`＝生成も破棄も窓スレッドで行う手順（停止の握手）が要る、⑶ `Shiori3Client::new(&'a ParentMessageWindow)`（`client.rs:86`）と `request_clean_shutdown(&mut self, window: &ParentMessageWindow)`（`lifecycle.rs:203`）が受ける型が `Send` なハンドルへ変わる→**`client.rs` を触る＝W13 併走 `charset-canon` と同一ファイル**、⑷ `pump_until_hello_or` の意味論（呼び手のスレッドで pump）が消える、⑸ `window_tests`／`lifecycle::tests` の窓生成前提が崩れる（2.8）。
- **効用**: 待ちの形は最も素直だが、往復に 1 ホップ増え、スレッドも増える。**本仕様の規模（S〜M・W13 共有ファイル 0）に収まらない。** 推奨しない。

### 比較

| | 案 0 交互待ち | 案 1a Event | 案 1b Post 起こし | 案 1c async | 案 1d 中継 | 案 2 専用スレッド |
|---|---|---|---|---|---|---|
| 同時待ち | △（交互・窓側 ≤T） | ○ | ○ | ○ | ○ | ○ |
| スレッド増 | 0 | 0 | 0 | 0 | +1 | +1 |
| inbox のホップ増 | 0 | 0 | 0 | 0 | +1 | 往復 +1 |
| 公開型の変更 | なし | `Sender<ShioriMsg>`→包み | 同左 | 同左＋spawn | なし | `&ParentMessageWindow`→ハンドル |
| 7.3 の編集集合 | 内 | **超える** | **超える** | **超える** | 内 | **超える（`client.rs`＝charset-canon 衝突）** |
| Cargo 変更 | なし | host32 feature | なし | kanade 依存 | なし | ipc or host32 |
| 6.1/6.2 の失敗経路 | 無（空振り） | Event 生成 | HWND 未確定の扱い | — | スレッド生成 | 停止の握手 |
| 2.3 切断検出 | 即時 | 包み Drop で起こす | 同左 | closed で即 | 中継が最終起こし | 別途 |
| 4.8 時間の仮定 | 本番に T | なし | なし | なし | なし | なし |
| 規模 | **S** | S〜M | S〜M | M | M | M〜L |

## 5. 檻の設計材料

- **置き場**: `crates/areka-kanade/src/shiori/real_pump_cage_tests.rs`（仮名）を `real.rs` から `#[path]` で取り込む。host32 側には「汲む部品」単体の小さな檻（窓なしで呼んでも失敗しない等）を `parent_window.rs` の兄弟へ置ける。**本番経路（Requirement 4.2）を駆動できるのは kanade 側だけ**（host32 → kanade の依存は逆向き）。
- **駆動**: テストスレッドとは別の runner スレッドで `ParentMessageWindow::create()`（本番と同じくスレッド内生成）→ `run_shiori_loop(rx, boxed(FakeBackend), on_down)`。HWND は生成後に channel で渡す。`real_tests.rs:603-623` の形の拡張。窓の生成は kanade の lib テストバイナリ内で直列化する（`Mutex<()>` を檻ファイルに置く。`all_senders_dropped_terminates_runner` 等は窓を作らないので対象外）。
- **速い檻（4.3）**: idle の runner に対し、テストスレッドから `send_copydata_response(host_hwnd, self_hwnd, MsgTag::Response, b"..", 2 s)`（旗なし・同期）を送り、`Ok` かつ所要 < 上限を assert。直す前は 2 秒待って `SendFailed`。全体 < 5 秒（4.9）。`Response` タグは親 WndProc が `StoreResponse` するだけで無害（`parent_window.rs:194-199`）。往復の混線を避けるなら未知タグ（`bad_frames` 計上）でもよい。
- **遅い檻（4.4）**: helper の檻を写す。`sleep 20 s` → `send_copydata`（`SMTO_ABORTIFHUNG`・5 s）→ `sleep 20 s` → 同送出。**2 回目**が `Ok` であること（直す前は即 `SendFailed`）。プロセス生存はテストバイナリ起動から数えて約 40 秒＝猶予期間の外（helper の檻と同条件）。上限 90 秒。
- **診断文**: helper 檻 `:255-262` の形（`first_delivered`／`second_delivered`／`idle`／`uptime_at_*`／`send_failures`）。
- **較正（4.7）**: タスク順を「檻を書く（赤を記録）→ 直す → 緑」にする。`git stash` は禁止（ハーネス規律）なので、檻の commit を先に切り、直す前のコミットで `cargo test -p areka-kanade --lib <name>` を走らせた結果（コマンド・所要・診断文）を `verification/` に残す。
- **走行時間**: `cargo test --workspace` はバイナリを直列に走らせるため、遅い檻は helper の檻（42 秒）とは**重ならず**、壁時計が約 +42 秒。許容するか、env-gate／`#[ignore]`（既存方針は「常設」）かは判断事項。

## 6. 規模とリスク

| 案 | 規模 | リスク | 理由 |
|---|---|---|---|
| 案 0 | **S** | **Low** | 既存 API のみ・型不変・失敗経路なし。唯一の判断は T の値と要件文の字義 |
| 案 1b | S〜M | Medium | 起こしの手組み（HWND 未確定・Drop・`quit()` の遅延）と公開型変更 12 か所 |
| 案 1a | S〜M | Medium | 1b＋feature 追加。優位なし |
| 案 1d | M | Medium | スレッド＋停止の順序（中継の join）を増やす |
| 案 1c | M | Medium | actor 基盤の凍結事項に触れる（開発者承認） |
| 案 2 | M〜L | High | `!Sync` 状態の跨スレッド化・`client.rs` 衝突・既存テストの前提崩し |

## 7. 設計で決める事項（要件ディスカッションへ）

1. **待ちの形の字義**——案 0（交互待ち・窓側の遅延 ≤T）を Requirement 1.1「継続的に取り出し」の実現として認めるか、Introduction の「同時に待つ」を字義どおり求めるか。認めるなら T の値（5 秒閾値の内側・例 500 ms〜1 s）と、その根拠を定数 doc に残す。
2. **Requirement 7.3 の編集集合**——案 1a/1b/1c は `Sender<ShioriMsg>` の公開型を変え、`crates/areka-kanade/src/actor.rs`（8 か所）・`crates/areka-ghost/src/runtime.rs:593,605`・テスト 3 本に及ぶ。7.3 を広げるか、7.3 を守れる案 0／案 1d に限るか。
3. **ipc に残る古い説明文**——`crates/shiori-host32-ipc/src/lib.rs:328-329`・`:591-593`「待機中はメッセージを取り出さない」。Requirement 3.3／7.3 は ipc 非接触、7.1 は古い説明の書き換えを求める。⑴ コメントのみ例外で書き換える、⑵ 触れずに検証記録へ登記（引受先: 次に ipc を触る `property-ipc-transport` W14）、のどちらか。
4. **握手の heartbeat の扱い**——常設の pump に統合して heartbeat スレッド（`parent_window.rs:266-282`）を消すか（例: `MsgWaitForMultipleObjectsEx` に期限を渡す）、握手の期限意味論を守るため無改変で残すか。無改変が最小。
5. **6.1/6.2 の空振り**——案 0 では「起こし手段の生成失敗」が構造上存在しない。要件を「該当なし」と記録して閉じるか、案 1 系を選んで実在させるか。
6. **遅い檻の常設**——`cargo test --workspace` に約 +42 秒。常設（既存方針）か、env-gate か。常設なら helper の檻と合わせて hung 系 2 本で約 84 秒。
7. **速い檻の上限と間欠赤**——同期送出の上限（例 2 秒）は負荷下で延びうる（`zorder-chain-residue` A-2 の族に見えないか）。判定は「届く／届かない」の二値で `SPIN_WAIT` 型ではないが、上限の余裕（2 秒 vs 4.9 の 5 秒）を設計で固定する。
8. **速い檻の送出タグ**——`Response`（`StoreResponse` に落ちる・無害）か未知タグ（`bad_frames` 計上・より無害）か。
9. **`doc/COMPAT_ARCHITECTURE.md` §8 への追記要否**——§8 は「正典沈黙箇所の裁量」の表（`:122-`）。本仕様は ukadoc の沈黙に関わらない（OS の挙動）ので追記不要が妥当。要件 7.5 は「追記する場合は自節のみ」。

## 8. Research Needed（設計フェーズへ持ち越し）

- **OS の応答なし判定と `PeekMessage`**: `PeekMessageW(PM_NOREMOVE)` でも「取り出した」扱いになるか（案 0 で `PM_REMOVE` 掃きにすれば問題は消える・確認のみ）。
- **`SendMessageTimeoutW` 中の再入と案 0**: 案 0 の掃きは `send_request` の外でしか走らないため、`clear→store→take` 不変条件は既存どおり保たれる（`parent_window.rs:312-316` の論法）。設計で明文化。
- **速い檻の負荷耐性**: 上限 2 秒で負荷走行（e2e `isolation-decision.md` §4.5.1 の条件）でも赤にならないか。1 度だけ計る（長時間試行禁止）。
- **executor の thread-local 窓**: `MessageLoop::run`／`PeekMessage` 掃きは executor の wake メッセージも配送する（無害）。案 1b で `spawn_local` を使わないなら影響なし。


---

## 9. 設計フェーズの記録（2026-09-13・`kiro-spec-design`）

> §1〜§8 は要件確定後の gap analysis（2026-09-11）をそのまま残す。本節は設計生成時の調査と決定を、`.kiro/settings/templates/specs/research.md` の構成（Summary → Research Log → Pattern Evaluation → Design Decisions → Risks → References）で追記する。行番号は HEAD `9e6095cd` 時点。

### 9.1 Summary

- **Feature**: `areka-P0-host32-window-thread-pump`
- **Discovery Scope**: Extension（既存システムの結合の是正・`design-discovery-light.md` を適用。外部ライブラリの追加なし・WebSearch 不要）
- **Key Findings**:
  - 待ちの形は §4 の案 0（交互待ち）で確定（要件 Introduction の裁定・2026-09-13）。`ShioriBackend::on_idle`（既定は何もしない）を唯一の追加契約とし、host32 の `ShioriConnection` だけが `ParentMessageWindow::pump_pending_messages` へ委譲する。kanade は Win32 の語彙を持たない。
  - `pump_pending_messages` に要る `PeekMessageW`／`PM_REMOVE`／`MSG`／`TranslateMessage`／`DispatchMessageW` はすべて `windows` 0.62.2 の `Win32_UI_WindowsAndMessaging`（host32 で有効済み・`PostMessageW` と同 feature）にあり、`Cargo.toml` の変更は不要。in-repo の前例は `crates/wintf/src/com/wuc.rs` `pump_current_thread_messages`（現 `:107-117`）。
  - **較正のためには「直す前の構造」でテストがコンパイルできなければならない**。`on_idle` が存在しない木では速いテストが書けないので、実装を「段階 1＝契約と部品（受信ループは `recv()` のまま）」「段階 2＝待ちの形」の 2 コミットに分け、段階 1 の時点で赤を採る。
  - **同一プロセスで親窓の組を 2 つ同時に持てない既知制約**（`lifecycle.rs` `WINDOW_TEST_SERIAL` の説明・`tests/kanade/real_helper_test.rs` 冒頭）と 4.9（速いテストは 5 秒以内）は、速い窓テストと遅い窓テストを同じテストバイナリに置くと両立しない（直列化すると速い側が 41 秒待たされる）。遅いテストだけを別バイナリ `crates/areka-kanade/tests/idle_pump_hung.rs` へ置く（7.3 の字義からの唯一の逸脱・設計ディスカッションで確認）。
  - `shiori/mod.rs` の rustdoc「メッセージ到達のたびに冒頭で `status` を確認」（現 `:44-46`）は変更後も真（手空き時の確認が加わるだけ）なので書き換えない。偽になるのは `real.rs` `run_shiori_loop` の doc（現 `:161`「blocking `recv`」・`:167`「タイマー poll は持たない」）で、こちらは 7.1 の対象。

### 9.2 Research Log

#### OS の応答なし判定と `PeekMessage`（§8 の持ち越し ①）

- **Context**: 案 0 は `PeekMessageW(PM_REMOVE)` で掃く。「取り出した」と判定される条件の確認。
- **Sources Consulted**: Win32 の `IsHungAppWindow`／`SendMessageTimeoutW`（`SMTO_ABORTIFHUNG`）の文書・6.11 の較正（`acceptance-record.md` §13.2 行 8・`main_response_flavor_hung_cage_tests.rs` 冒頭「5.5 秒／10 秒では検出できない」）。
- **Findings**: 判定は「窓を所有するスレッドが `GetMessage`／`PeekMessage` 系を一定時間呼んでいない」で決まる（文書上 5 秒・6.11 の実測では `SMTO_ABORTIFHUNG` の打ち切りは 14 秒＋プロセス生存 20〜30 秒超）。`PM_REMOVE` で取り出せば確実に「取り出した」扱いになる（`PM_NOREMOVE` の扱いに依存しない）。
- **Implications**: 周期 500 ms で `PM_REMOVE` 掃きを行えば、文書の 5 秒に対し 10 倍の余裕。`WM_QUIT` の特別扱いは不要（このスレッドで `PostQuitMessage` を呼ぶ者はいない・`DispatchMessageW` は `WM_QUIT` を無視）。

#### `SendMessageTimeoutW` 中の再入と案 0（§8 の持ち越し ②）

- **Context**: 掃きが往復の `clear→store→take` 不変条件を壊さないか。
- **Sources Consulted**: `parent_window.rs` `send_request` の doc（現 `:309-312`）・`shiori_host32_ipc::send_request` の手順（`lib.rs` 現 `:398-431`）。
- **Findings**: `on_idle` は受信ループの `Timeout` の腕からしか呼ばれず、`get`／`notify`／`unload`（往復）と同一スレッドで直列。往復の内側（`SendMessageTimeoutW` にブロック中）では受信ループは待ちに戻っていないので `on_idle` は走らない。手空き中に `Response` タグの迷子フレームが届いて `response_slot` に積まれても、次の `send_request` は `slot.clear()` から始まる。
- **Implications**: 不変条件は既存どおり。`pump_pending_messages` の rustdoc に「往復の内側から呼ばない」を明記し、Revalidation Triggers に載せる。

#### 速いテストの負荷耐性（§8 の持ち越し ③）

- **Context**: 送出上限 2 秒が負荷下で延びて間欠赤にならないか（4.9・`zorder-chain-residue` A-2 の族）。
- **Findings**: 判定は「届く／届かない」の二値で `SPIN_WAIT` 型ではない。周期 500 ms に対し上限 2 秒＝4 倍。`const _: () = assert!(SEND_BOUND.as_millis() >= 4 * IDLE_INTERVAL.as_millis())` で比を固定する。
- **Implications**: 実装時に 1 度だけ負荷条件（e2e `isolation-decision.md` §4.5.1）で計り `calibration.md` に 1 行残す。常設の負荷テストは置かない（長時間試行禁止）。

#### executor の thread-local 窓（§8 の持ち越し ④）

- **Context**: `pump_pending_messages` が executor の起こしメッセージを配ってしまわないか。
- **Sources Consulted**: `wintf-winmsg-executor-0.0.5` `src/lib.rs`（`EXECUTOR_WINDOW` thread-local・`MSG_ID_WAKE`＝`WM_USER`・`run_loop` 現 `:182-203`）。
- **Findings**: executor の窓はそのスレッドで `MessageLoop::run`／`spawn_local` を使ったときに遅延生成される。アクタースレッドでは `pump_until_hello_or`（握手時）が `MessageLoop::run` を使うので窓は存在するが、`spawn_local` を使う者はおらず `MSG_ID_WAKE` は飛ばない。仮に飛んでも `DispatchMessageW` が executor の WndProc へ配るだけで無害。
- **Implications**: 影響なし。

#### `recv_timeout` の腕の区別（新規）

- **Context**: 2.3（全 Sender drop で正常終了）を `recv_timeout` でも保つ。
- **Findings**: `std::sync::mpsc::Receiver::recv_timeout` は `Err(RecvTimeoutError::Timeout)` と `Err(RecvTimeoutError::Disconnected)` を区別する。切断は即時に返る（待ちの上限まで寝ない）。
- **Implications**: `Disconnected` の腕で `return`。既存テスト `all_senders_dropped_terminates_runner`（`real_tests.rs` 現 `:603-623`）がそのまま見張る。

#### テストの置き場と窓の制約（新規）

- **Context**: §5 は「kanade の兄弟テストファイルに置く」と書いたが、速いテストと遅いテストの同居を調べていなかった。
- **Sources Consulted**: `lifecycle.rs` 現 `:519-524`（`WINDOW_TEST_SERIAL`・「同一プロセスで 2 組の message-only 窓を同時生成すると 2 組目が `WindowCreationError`」）・`tests/kanade/real_helper_test.rs` 現 `:22-24`（「親窓 1 枚制約ゆえ単一の `#[test]` に集約」）・kanade の lib テストバイナリに親窓を作るテストが現状 0 本であること（grep）。
- **Findings**: 遅いテストは親窓を約 41 秒持ち続ける。同じバイナリに速い窓テストを置いて直列化すると、順序次第で速い側が 41 秒待つ（4.9 の 5 秒を満たせない・間欠的に遅い）。cargo はテストバイナリを直列に走らせるので、別バイナリなら窓は同時に存在しない。
- **Implications**: 速いテスト群（Test-A〜E）は兄弟 `real_idle_tests.rs`（lib バイナリの唯一の親窓・ロック不要）、遅いテスト（Test-F）は `tests/idle_pump_hung.rs`＋`tests/idle_pump_hung/hung_test.rs`（structure.md の `tests/{domain}.rs` 入口＋`#[path]` 規約）。`WindowOnlyBackend`（15 行程度）は両方に持つ（別バイナリから lib テストの private 項目へ届かない）。

### 9.3 Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|---|---|---|---|---|
| 交互待ち（案 0・**採用**） | `recv_timeout(T)`＋手空き時 `on_idle` | 型・Cargo・スレッド不変・失敗経路なし・7.3 の内 | 窓側の遅延 ≤ T・本番に周期 T を 1 つ置く | 要件 Introduction の裁定 |
| 起こしを手組み（案 1a/1b） | Event／`PostMessageW` を送信端に足す | 真の同時待ち | 公開型変更（12 か所）・feature 追加・`Drop` での起こし | 7.3 超過 |
| 中継スレッド（案 1d） | 中継が起こしを出す | 公開型不変 | スレッド＋ホップ＋停止順序 | 案 0 で足りる |
| 窓の専用スレッド（案 2） | 窓を移す | 待ちは素直 | `!Sync` 状態の跨スレッド化・`client.rs` で `charset-canon` と衝突 | 不採用 |
| 窓へ ping | 別スレッドから撃つ | 変更が小さく見える | 判定は「取り出しているか」なので効かない | 不採用 |

### 9.4 Design Decisions

#### Decision: 手空きの周期 `IDLE_INTERVAL` は 500 ms

- **Context**: 要件は「OS の応答なし判定の閾値に対する余裕を根拠に決め、定数の説明文に残す」と定める。開発者の当初案は 2 秒（通常運転で静か）、設計側の推奨は 500 ms。
- **Alternatives Considered**: 1. 2 秒（5 秒に対し 2.5 倍） 2. 1 秒（5 倍） 3. 500 ms（10 倍）
- **Selected Approach**: 500 ms。`real.rs` に `pub const IDLE_INTERVAL` として 1 か所だけ置く。
- **Rationale**: ⑴ スケジューラの遅れ（Defender の再スキャンで協調ループが飢餓した実績）に対し 2 秒は 1 回の遅れで 5 秒に届きうる。⑵ 速いテストの上限（周期の 4 倍・全体 5 秒以内）は 500 ms でしか余裕を持って組めない。⑶ 手空き中の死活検出・将来の自発通知の遅れが ≤ 0.5 秒。⑷ 「静か」の要請はログ 0 行（6.3）で満たされ、周期の長短に依存しない。コストは毎秒 2 回の空の `PeekMessage`＋`try_wait`。
- **Trade-offs**: 起床が毎秒 2 回（無視できる）。
- **Follow-up**: 設計ディスカッションで開発者が 2 秒を望むなら、Test-D の `SEND_BOUND` を 8 秒以上にせねばならず 4.9（5 秒以内）を改訂する必要がある——その場合は要件へ戻す。

#### Decision: 手空き通知の名前と置き場

- **Selected Approach**: `ShioriBackend::on_idle(&mut self) {}`（`real.rs`・既定は何もしない）。host32 側の部品は `ParentMessageWindow::pump_pending_messages(&self)`（`parent_window.rs`・メソッド）。`ShioriConnection::on_idle` は 1 行の委譲。
- **Rationale**: 自由関数でなくメソッドにするのは、`ParentMessageWindow` が `!Send` ゆえ `&self` を持てるのが窓を作ったスレッドだけであり、「`PeekMessageW` は呼び出しスレッドのキューを見る」前提を型で保証できるため。契約の説明文に Win32 の語彙を書かない（2.10）。
- **Trade-offs**: `on_idle` は失敗を返さない（`()`）。6.1 の「保守の失敗」は backend の `status()` で表し、手空きの腕がその直後に必ず確認する既存の死活経路へ一本化する（新しい終了経路を作らない）。

#### Decision: 握手時の heartbeat は無改変

- **Alternatives Considered**: 1. 常設の保守に統合し heartbeat スレッドを消す 2. 無改変で説明文だけ追随
- **Selected Approach**: 2。
- **Rationale**: 握手の期限意味論は `MessageLoop::run`（`quit()` は次のメッセージ後にしか効かない）と heartbeat の組で成り立っており、統合しても得るものがなく 2.5 の観測結果を危うくする。説明文 4 か所の「pump フェーズ専用」を「握手フェーズの起こし専用・定常時は `pump_pending_messages`」へ。

#### Decision: 速いテストの上限と送出タグ

- **Selected Approach**: `SEND_BOUND = 2 s`（周期の 4 倍・const assert で比を固定）・全体 5 秒以内。タグは `MsgTag::Response`（本番の `StoreResponse` の腕を踏む・6.11 の遅いテストと同じ・`response_slot` は次の `send_request` が `clear`）。速いテストは `send_copydata_response`（旗なし）、遅いテストは `send_copydata`（`SMTO_ABORTIFHUNG` つき）で送る——検査対象は旗であってタグではない。
- **Alternatives Considered**: 未知タグ（`bad_frames` 計上）は ipc の公開 API（`MsgTag` 列挙）からは送れず、生の `SendMessageTimeoutW` を使うと kanade のテストに `windows` 依存が要る（7.6 の趣旨に反する）。`Load`／`Request`／`Unload`（`IgnoreKnown`）は状態を書き換えないが「親が Request を受ける」形が読みにくい。

#### Decision: 較正は 2 段階のコミットで採る

- **Selected Approach**: 段階 1（`on_idle`・`IDLE_INTERVAL`・`pump_pending_messages`・委譲・Test-A〜F／受信ループは `recv()` のまま）で Test-A・B・D・F の赤を `verification/calibration.md` に写し、段階 2（`recv_timeout` の形）で緑を併記する。
- **Rationale**: 「直す前の構造」＝待ちの形。契約と部品が無い木では速いテストがコンパイルできない。`git stash` は禁止（ハーネス規律）。

#### Decision: `doc/COMPAT_ARCHITECTURE.md` §8 は追記しない

- **Rationale**: §8 は ukadoc の沈黙に対する裁量の表。本仕様の裁定は OS の挙動に対するもので正典の沈黙に関わらない。裁定の正本は design.md「待ちの形の裁定と不採用案」節。

#### Decision: `real_tests.rs` の `FakeBackend` は共有ヘルパへ集約しない

- **Rationale**: 2.8 が `real_tests.rs` を無改変と定める。`real_idle_tests.rs` の `IdleProbeBackend` は「他スレッドから死活を切り替え・手空きを数える」責務で `FakeBackend`（閉包で台本化）と異なり複製ではない。3 つ目のテーマが現れたときに `real_test_support.rs` へ集約する。

### 9.5 Risks & Mitigations

- `ShioriConnection::on_idle` の 1 行の委譲はテストで踏まれない（fake が同じ部品へ委譲する形）— 配線は再テストしない規律に従い、実装レビューで委譲先を目視確認する。実機走行（5.x）は 6.11 の二重の守りで緑になるため、この 1 行の欠落を検出しない点を `real-machine.md` に注記する。
- 速いテストの 2 秒上限が負荷下で薄くなる — 周期の 4 倍を const assert で固定・1 度だけ負荷計測。
- `Timeout` と `Disconnected` の取り違え — 既存テストが赤にする。
- kanade の lib テストバイナリに親窓を作るテストが将来 2 本目以降置かれる — Revalidation Triggers に登記。`WINDOW_TEST_SERIAL` 同型のロックを置く。
- 遅いテストの +42 秒 — 4.11 で受容済み。

### 9.6 §7 の判断事項の決着

| # | 事項 | 決着 |
|---|---|---|
| 1 | 案 0 の字義 | 要件 Introduction の裁定で採用。T＝500 ms（9.4） |
| 2 | 7.3 の編集集合 | 案 0 ゆえ広げない。遅いテストの別バイナリ 2 ファイルのみ字義から逸脱（9.2） |
| 3 | ipc の古い説明文 | コメントのみ書き換える（要件 7.1 の改訂で 3.3 の例外として明文化済み） |
| 4 | 握手の heartbeat | 無改変・説明文のみ（9.4） |
| 5 | 6.1／6.2 の空振り | 6.2 は該当なしで閉じる（要件改訂済み）。6.1 は `status` 経由の既存経路へ一本化 |
| 6 | 遅いテストの常設 | 常設・無条件（要件 4.11 で裁定済み） |
| 7 | 速いテストの上限 | 2 秒＝周期の 4 倍・const assert（9.4） |
| 8 | 送出タグ | `MsgTag::Response`（9.4） |
| 9 | §8 の追記 | しない（9.4） |

### 9.7 References

- `.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/acceptance-record.md` §13.2 行 8（較正値）・行 10（起票行）。
- `.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/lap-procedure.md` §5.7（判定に使う語の表）。
- `crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs`（遅いテストの原型と診断文の形）。
- `crates/wintf/src/com/wuc.rs` `pump_current_thread_messages`（pump の前例）。
- `wintf-winmsg-executor` 0.0.5 `src/lib.rs` `MessageLoop::run_loop`・`src/util/window.rs` `Window::new_ex`。
- steering: `structure.md`（兄弟テストファイルの命名・`tests/{domain}.rs` 入口）・`tech.md`（常時テストは x64）・`logging.md`。
