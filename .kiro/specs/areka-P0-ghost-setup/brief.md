# Brief: areka-P0-ghost-setup

> **種別**: 本坑（main）。⓪ ghost 帰属・**アプリ組み上げ三段の第二段**（app-shell（骨格）✅ → **ghost-setup（エンジン結線）** → emo2-conformance-e2e（適合証明））。
> **調査日**: 2026-07-05（kanade✅・sakura✅・shiori 全✅・app-shell✅ 完了後の実シンボル偵察）。
> **解禁根拠**: 結線対象のエンジンが揃った——①shiori トラック全完了（`Shiori3Client`/`HelperLifecycle`）・③kanade ✅（`spawn_kanade`/`spawn_shiori_actor`）・④sakura ✅（`spawn_talk`）・骨格シーム ✅（`open_startup_window`）。

## Problem

エンジン群は完成したが、**実際に繋いで起動する者がいない**。さらに並走実装の帰結として **talk 契約が二重定義でフォーク**しており（下記）、DD-1 が予定した「re-export 差し替えによる機械的移譲」が**成立しない**。結線（本ユニット）が最初に踏む地雷なので、契約統一も本ユニットが所有する。

**契約フォークの実測（2026-07-05・両クレート実コード確認）**:

| | kanade（`talk.rs`・roadmap 正本） | sakura（`contract.rs`・DD-1 暫定所有） |
|---|---|---|
| StartTalk | `{talk_id, script}`（Clone・reply なし） | `{script, talk_id, reply: ReplySender<TalkDone>}`（oneshot 同梱） |
| TalkDone | `{talk_id, quit: bool}` | `{talk_id, reason: TalkEndReason}`（**3値** Ended/Quit/Interrupted） |
| 配送 | kanade は永続 `Sender<StartTalk>` へ送出・TalkDone は自 inbox（`KanadeMsg::TalkDone`）で受領 | **per-talk `spawn_talk`＝永続 inbox が存在しない**・TalkDone は reply oneshot |

- 記憶 areka-interrupt-single-close-funnel の設計合意は **reason 3値**（中断も ACK）——kanade 実装が quit:bool で先行した形。
- kanade→sakura の中断経路も未結線（kanade は `Sender<StartTalk>` しか持たない・close 握手は deadline 依存で成立中）。

## Current State

- **kanade ✅**: `spawn_kanade(config: KanadeConfig, shiori: Sender<ShioriMsg>, sakura: Sender<StartTalk>) -> (Sender<KanadeMsg>, ActorHandle)`。`KanadeConfig{shell_name, baseware_version, baseware_name, close_talk_deadline_ms}`。`KanadeMsg::{Boot, Tick{now: MonotonicMs}, TalkDone, CloseRequest, ForceQuit, ShioriDown, Close}`。StartTalk 送出失敗は error!＋運行継続（sakura 切断耐性テスト済み）。
- **kanade::shiori ✅**: `spawn_shiori_actor(connect: impl FnOnce() -> Result<ShioriConnection, String> + Send + 'static, on_down: Sender<KanadeMsg>) -> (Sender<ShioriMsg>, ActorHandle)`——**接続はアクタースレッド上で一度だけ実行**（`ParentMessageWindow` が `!Send` の pump×inbox 問題は解決済み）・接続失敗は `ShioriDown` 死活報告。
- **sakura ✅**: `spawn_talk(start: StartTalk, surface_sink: impl SurfaceSink, text_sink: impl TextSink) -> TalkHandle{inbox: Sender<SakuraMsg>, actor}`。`SakuraMsg::{Start, Tick(f64), Close}`——**Tick は talk 起点からの経過秒・外部 ticker が注入**（「本番は kanade/clock アクター・ghost-setup 結線」と sakura 自身が明記）。
- **shiori-host32-host ✅**: `HelperLifecycle{new, status()->HelperStatus, terminate}`（sticky 死活）・`classify_failure`・`request_clean_shutdown`/`ShutdownError`（正規正常終了）。
- **app-shell ✅**: `resolve_config_inputs(args)->ConfigInputs{ghost_root, balloon_root}`・**`open_startup_window(&WinApp)`＝replace-me シーム**（下流の唯一の差し込み点）・env ゲート smoke（`AREKA_APP_SMOKE_EXIT_MS`）。
- **package-mount ✅**: `MountModel`（descript.txt 起点・SHIORI dir/file・shell dir 解決）。

## Desired Outcome

**WS-A（契約統一・先行）**: talk 契約を kanade 正本へ一本化——**reason 3値の採否**（設計合意は 3値・kanade の quit:bool 消費部の改稿）と **reply 配送方式**（oneshot 同梱 vs kanade inbox 転送——dispatcher が吸収するか型で一本化するか）を design で確定し、sakura `contract.rs` の暫定所有型を re-export へ差し替え（**下流 import パス `areka_sakura::contract::*` 不変**）・両クレートのテスト追随。**`TalkCue`/`SurfaceSink`/`TextSink`/`cue_target_of` には触れない**（並走 seriko の消費面を凍結）。

**WS-B（結線の背骨）**: descript.txt 起点で全エンジンを起動〜終了統括する `ghost` 結線層——
mount（package-mount）→ shiori actor（`spawn_shiori_actor` の connect closure に `Shiori3Client` 接続＋`HelperLifecycle` 監視を格納）→ kanade（`KanadeConfig` の値源＝descript/MountModel から解決）→ **sakura dispatcher**（kanade の永続 `Sender<StartTalk>` を受ける常駐 actor: per-talk `spawn_talk`・単一 slot・TalkDone を `KanadeMsg::TalkDone` へ転送・停止時は active talk へ `Close`＋join）→ **ticker**（実 clock から `KanadeMsg::Tick`（毎秒 pump）と `SakuraMsg::Tick(f64)`（per-talk 経過秒）を養う）→ 終了統括（kanade 停止観測→shiori Unload→`request_clean_shutdown`→全 join・exit 0）。

**✔ 観測（単一 pass/fail）**: (a) **決定論 spine e2e**＝testdll fixture＋**記録 sink**（SurfaceSink/TextSink の録音実装）で boot（OnBoot script 受領→sakura 再生→sink 発火列）→close（正規 clean shutdown・`ExitKind::Clean`・全スレッド join）が **sleep 不使用（注入 Tick）**で green ＋ (b) env-gate 実 pasta 追験（実 emo2 OnBoot 一周）＋ (c) `open_startup_window` シーム経由の起動形で app smoke green 維持（ダミー窓は維持——本物窓は window-placement 領分）。

## Approach

1. **WS-A 先行**（契約統一・上記）。kanade/sakura への**隣接クレート増分**として実施（記憶 canonical-not-minimal-lifecycle が根拠・凍結境界は下記 Constraints）。
2. **sakura dispatcher**: kanade は「永続 channel に送る」設計・sakura は「per-talk transient」設計——この非対称を吸収する常駐 actor が結線の中核。**単一 slot**（同時 talk 1・記憶 areka-interrupt-single-close-funnel）・stale TalkDone は talk_id で棄却（kanade 側 R6.6 と対）。
3. **ticker**: 1 スレッドで両方を養う（kanade 毎秒 Tick・active talk へ経過秒 Tick）か分離か——**決定論テストでは ticker を差し替え可能に**（注入 Tick が spine e2e の要）。
4. **KanadeConfig の値源**: `shell_name`（OnBoot Ref0）は shell descript の name 系から・`baseware_*` は areka 定数——design で確定。
5. **終了統括**: 停止順序＝Close→drain→join（actor-foundation 規約）を全エンジンに適用。helper は `request_clean_shutdown` の正規経路（stand-in 禁止）。失敗経路は log-first＋`Err`。
6. **表示はスコープ外**: sink は trait の**録音実装**を挿す。seriko/emo-text-layer の実装が後から同じ trait 差し込み口に挿さる（M-boot 統合＝emo2-boot）——trait 結線ゆえ stand-in ではなく**正規のシーム**。

## クロスユニット契約（後続を詰ませない事前考慮）

- **WS-A の凍結面**: `TalkCue`/`SurfaceSink`/`TextSink`/`cue_target_of`/dola cue 型は**不改変**（並走 seriko が消費中）。触るのは `StartTalk`/`TalkDone`/`SakuraMsg::Start` の授受面のみ。
- **seriko/emo-text-layer への差し込み口**: dispatcher の sink 注入点を公開形に（構築時注入 or setter——design 判断）。M-boot 統合はこの口に実 sink を挿すだけで済むこと。
- **window-placement との境界**: 本ユニットは**窓を作らない**（ダミー窓維持）。`open_startup_window` シームの中身の「本物ゴースト窓生成」は window-placement（emo-present ゲート下）の領分——本ユニットはシームの**周りの結線**（エンジン起動と終了）を所有。
- **ticker の cadence**: OnSecondChange は 1 秒周期（ukadoc `list_shiori_event`）。sakura Tick(f64) の粒度（wait 解像度）は sakura 実装の要求から design で確定（1 秒では粗い可能性——`\w` は ms 級）。

## ukadoc 必読（design 着手時）

- boot/close の発火順序は **kanade 実装済み＝正本**（再調査不要・kanade の schedule/ が ukadoc Reference 表を写経済み）。
- `list_shiori_event` の **OnSecondChange**（1 秒周期の確認のみ）。
- SHIORI/3.0 unload 意味論は host32-lifecycle ✅ が実装済み（`request_clean_shutdown` 消費のみ）。

## Scope

- **In**: WS-A 契約統一（kanade 正本化・reason 3値・sakura re-export 移譲・テスト追随）／sakura dispatcher（単一 slot・転送・Close funnel）／shiori actor 結線（connect closure・HelperLifecycle）／ticker（差し替え可能）／KanadeConfig 値源解決／終了統括（正規 clean shutdown・全 join）／決定論 spine e2e＋env-gate 実 pasta 追験。
- **Out**: 表示結線（seriko/emo-present/emo-text-layer——sink は録音実装）／本物ゴースト窓生成（**window-placement**）／位置永続化（**position-persist**・M-life）／OnSecondChange の自発会話（**idle-talk**・M-life。ticker は Tick を送るだけ・204 は kanade が処理済み）／SSTP/FMO 等（M2）。

## Boundary Candidates

- WS-A（契約統一・エンジン側是正）／dispatcher＋ticker（runtime 結線）／lifecycle 統括（起動順・終了順）の三片。

## Out of Boundary

- kanade の運行表・sakura の再生意味論そのもの（完了済み仕様の領分——WS-A は授受型の統一のみ）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-kanade` ✅／`completed/areka-P0-sakura-engine` ✅／①shiori 全✅（`Shiori3Client`/`HelperLifecycle`/`request_clean_shutdown`）／`completed/areka-P0-app-shell` ✅（シーム）／`completed/areka-P0-package-mount` ✅／`completed/areka-P0-actor-foundation` ✅。
- **Downstream**: M-boot 統合（emo2-boot＝sink 差し込み口に seriko/emo-text-layer を挿す）／`emo2-conformance-e2e`（三段の第三段）／`idle-talk`・`input-events`（kanade 増分が本結線上で動く）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-kanade`・`completed/areka-P0-sakura-engine`（WS-A の隣接増分・talk 授受面のみ）・`completed/areka-P0-app-shell`（シーム消費）。
- **Adjacent**: `areka-P0-seriko-engine`（**並走**・sink trait 消費者＝WS-A 凍結面で保護）／`completed/areka-P0-emo-present` **✅**（2026-07-09 完了・非衝突: example＋emo 新層 vs main.rs＋結線層）／`areka-P0-window-placement`（境界: 窓=あちら・結線=こちら）。

## Constraints

- Rust 2024・tokio 禁止・`std::sync::mpsc` 系（areka-actor 規約）。
- **凍結境界**: `shiori-host32-ipc`／`Shiori3Client`／`RequestError`／`LifecycleReport` 語彙／sakura の出力契約（TalkCue/sink trait）——不改変。
- **正規実装の原則**（記憶 canonical-not-minimal-lifecycle）: 終了経路は正規の clean shutdown 一本・stand-in 禁止。
- **決定論的テスト網羅**（記憶 deterministic-test-coverage-mandate）: ticker 注入で spine 全経路（boot 成功/SHIORI 死活/close 握手/deadline/全断線）を実行テスト化。sleep 不使用。
- workspace test は i686 host-32 成果物の事前ビルドが必要（記憶 workspace-test-needs-i686-host32-artifacts）。
- ログ規律: log-first・silent failure 禁止（記憶 areka-log-first-no-silent-failure）。
