# Brief: areka-P0-kanade

> **種別**: 本坑（main）。③ kanade（conductor）＝**実行時経路（運行表）の所有者**。runtime 制御階層の起点。
> **調査日**: 2026-07-05（actor-foundation✅／host32-request✅ の実 API 調査済み・ukadoc boot/close 調査済み 07-05）。
> **前提依存**: `areka-P0-actor-foundation` ✅（2026-07-04 完了・crates/areka-actor）。

## Problem

エンジン群（shiori 通信・parser・再生系）が揃っても、**「いつ・誰に・何を流すか」の運行表＝conductor が存在しない**。boot イベントの発火順序・毎秒 pump・SHIORI 応答（Value）の sakura への配送・close 握手——ゴーストを「生かす」中枢が空席である。

## Current State

- **areka-actor ✅（実シンボル）**: `spawn_actor`/`run_inbox`/`ActorHandle`・`reply_channel`/`ReplySender<T>`/`ReplyReceiver<T>`・`spawn_ui`/`UiSender`。停止規約＝Close 即時停止・全 Sender drop 正常終了・handler Err は記録して継続。
- **shiori 層 ✅**: `Shiori3Client::get(id, refs) -> Result<Option<String>>`（200→Some(Value)/204→None）・`notify`・`RequestError` 区別語彙・`spawn()->HelperHandle`。常駐健全性と死活報告型は **host32-lifecycle（並走中）が正本**＝kanade は消費。
- **sakura-parse ✅**: `sakura::parse(&str) -> Vec<Instruction>`（Value の中身の解釈は sakura-engine 領分・kanade は script 文字列を**不透明のまま**渡す）。
- **boot/close の正典順序（ukadoc 07-05 調査済み・roadmap kanade 行が転記正本）**: boot＝`OnInitialize`(NOTIFY)→`OnFirstBoot`(Ref0=vanish count)/`OnGhostChanged`/`OnGhostCalled`/`OnVanished` の 204 フォールスルー→`OnBoot`(Ref0=shell 名)→`basewareversion`(NOTIFY)。close＝`OnClose`(Ref0=理由)→応答スクリプト**再生完了待ち**（`\-`）→204 なら `OnCloseAll`→終了。M-boot は毎回 OnBoot 開始で可（vanish count 永続化は position-persist）。

## Desired Outcome

kanade アクター（独立スレッド・areka-actor 規約）が **boot 運行→毎秒 pump→Value 配送→close 握手**を運行表として駆動する。

**✔ 観測（単一 pass/fail・観測の独立化）**: **mock shiori アクター**（fixture 応答: OnBoot→固定 Value・OnSecondChange→204/散発 Value）＋**mock sakura sink** を繋ぎ、(a) boot 系列が正典順序で発火（NOTIFY/GET の別・Reference 構成込み） (b) Value 受領→talk 起動要求が sink に届く (c) OnClose→sink の再生完了通知を待って終了系列が完走——を**決定的に**観測（実時間 sleep 非依存・時刻注入）。実 helper 越しは env-gate 追験。

## Approach

1. **kanade アクター**: `crates/areka-kanade`（命名慣行: areka-actor/areka-emo-atlas に倣う）。inbox＝`KanadeMsg` enum（Boot 指示/Tick/TalkDone 通知/Close…）。ghost-setup が結線・boot 指示を送る（三段構えの第二段が呼び手）。
2. **運行表**: boot 系列（上記正典順序・204 フォールスルー）→定常（OnSecondChange pump・periodic tick は `recv_timeout` or 外部 Tick 注入＝**決定的テストのため時刻/Tick 注入式**が受け入れ基準）→close 握手。
3. **shiori 呼出**: `Shiori3Client` を専有スレッド（親窓 pump）で包む **shiori アクター**を立て、kanade からは channel（request/reply＝`reply_channel`）で往復——host32-request brief の「包むだけで済む形」を実行に移す。死活語彙（lifecycle 正本）で縮退判断（M1＝ログ＋停止・自動再起動しない）。
4. **Value→sakura 配送**: **talk 起動契約（本 brief が正本・sakura-engine が消費）**——`StartTalk{script: String, talk_id}` 級＋sakura 側からの `TalkDone{talk_id, quit: bool}`（`\-`＝quit 検出は sakura が担い kanade へ通知）。script は不透明文字列のまま渡す（parse は sakura 側）。
5. **エラー方針**: RequestError/死活報告の全アームを error! ログ＋観測可能な状態遷移で処理（ログ無し失敗経路の禁止・panic は致命限定）。

## クロスユニット契約（申し開き・本 brief が正本のもの）

- **talk 起動契約**: `StartTalk`/`TalkDone` のメッセージ型は kanade が正本（sakura-engine brief は消費・再定義しない）。
- **boot/close 運行表**: 発火順序・NOTIFY/GET の別・最小 Reference 構成は kanade が正本（app-shell/ghost-setup は器と結線に徹する——三段構えの分担どおり）。
- **消費する正本**: 死活報告型＝host32-lifecycle／envelope 規約＝areka-actor／`Shiori3Client`＝host32-request。

## ukadoc 必読（design 着手時に ukadoc MCP で正典参照）

- `list_shiori_event`: **OnInitialize**（Ref0="reload" 判定）・**OnFirstBoot**（Ref0=vanish count）・**OnBoot**（Ref0=shell 名・Ref6/7=crash 情報〔任意〕）・**OnGhostChanged/OnGhostCalled/OnVanished**（フォールスルー元）・**OnClose/OnCloseAll**（Ref0=理由）・**OnSecondChange**（Ref0=OS 稼働時間(h)・Ref1=clipped・Ref2=overlap・Ref3=talk 可否・Ref4=idle 秒〔SSP〕——**Ref 構成の実装値は design で全確認**）・**basewareversion**（NOTIFY・Ref0=version/Ref1=名前/Ref2=詳細）。
- **具体指示**: design 冒頭で上記イベントの Reference 表（M-boot 送出最小集合）を design.md に載せ、mock shiori の fixture 応答をこの表から生成すること。

## Scope

- **In**: kanade アクター（inbox/運行状態機械）／boot・close 運行表／OnSecondChange pump（Tick 注入式）／shiori アクター化（Shiori3Client の channel 包装）／talk 起動契約（正本）／mock 観測ハーネス。
- **Out**: script の解釈・再生（**sakura-engine**）／入力イベント配信 OnMouse* 系（**input-events**・M-life）／自発会話の選定ロジック（**idle-talk**・M-life）／vanish count 永続化（**position-persist**）／helper 常駐健全性の証明（**host32-lifecycle**）／自動再起動（M2）。

## Boundary Candidates

- 運行状態機械（純粋・イベント列→次アクション＝単体テスト可）／shiori アクター包装／配送結線（sakura sink）の三層。

## Upstream / Downstream

- **Upstream**: `areka-actor` ✅・`host32-request` ✅・（並走）`host32-lifecycle`＝死活語彙の正本・`sakura-parse` ✅。
- **Downstream**: `sakura-engine`（talk 起動契約の消費者）／`ghost-setup`（結線・boot 指示の呼び手）／`idle-talk`/`input-events`（M-life 増分が運行表に載る）。

## Existing Spec Touchpoints

- **Adjacent**: `areka-P0-host32-lifecycle`（**並走可**——契約は「死活報告型＝lifecycle 正本・kanade は消費」で先決済み。lifecycle 未完了の間は mock 死活で開発）／`areka-P0-sakura-engine`（**並走可**——talk 起動契約は本 brief 正本で先決済み）。

## Constraints

- Rust 2024・tokio 禁止・areka-actor 規約に載る（自前 channel 流儀の発明禁止）。新設クレート＝既存と非衝突。
- 決定的テスト（時刻/Tick 注入・実時間 sleep 非依存）。ログ無し失敗経路の禁止。
