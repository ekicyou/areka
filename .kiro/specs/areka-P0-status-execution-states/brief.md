# Brief: areka-P0-status-execution-states

> **種別**: 本坑（main）増分・**追跡/調整 spec（Phase 0 器・台帳）**。③ kanade（`Status` ヘッダ組立）が窓口・実行状態の源は多エンジン横断。`areka-P0-idle-talk` の要件ディスカッション #2（2026-07-17）で ukadoc `Status` 実行状態語彙全体を第一級化し **M1 は `talking` のみ実導出**と決裁——残状態の実導出を追跡するために立ち上げた器。
> **調査日**: 2026-07-17。
> **規律**: 憶測の過剰設計をしない（roadmap「spec 工場の禁止」「憶測で先に書かない」）。本 brief は**残状態と源の台帳**であって、未着地サブシステムの先行設計ではない。各状態は源サブシステムが実物として着地した時点で just-in-time に実導出を切る（または M2 として送る）。

## Problem

ukadoc `Status [SSP拡張]`（`ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`）はゴーストの実行状態を**カンマ連結の状態集合**で定義する（talking／choosing／minimizing／induction／passive／timecritical／nouserbreak／online／opening(種類)／balloon(ID群)）。`areka-P0-idle-talk` がこの**語彙全体を第一級構造として確立**し、`talking` を kanade `Steady{talk}` から実導出、残9状態を**非アクティブへ縮退＋実測差替シーム**（Reference1/Reference2 と同型）で保持した。残状態の**実導出（源サブシステムからの実値算出）は未実施**で、源サブシステムの多くが M1 に存在しない。放置＝手抜きにせず、正典への宿題として台帳化する。

## Current State（2026-07-17）

- `talking` = ✅ 実導出（`areka-P0-idle-talk`・kanade `events.rs`）。
- `choosing` = 選択肢表示中。**源＝選択肢UI＝`areka-P0-choice-select-events` が既に所有**（roadmap ③増分行「`Status: choosing`〔idle-talk の口消費〕」）＝本 spec は重複せず調整のみ。
- 残（源サブシステム未着地）: minimizing／induction／passive／timecritical／nouserbreak／online／opening(種類)／balloon(ID群)。

## Desired Outcome

ukadoc `Status` 実行状態語彙の各状態が、それぞれの源サブシステム着地時に idle-talk の確立したカンマ連結送出契約（書式・ヘッダ位置・空集合→行省略、いずれも不変）へ**実測差替**され、正典どおりの実行状態が OnSecondChange（および他リクエスト）に載る。

## 状態別 源マッピングと帰属（台帳）

| ukadoc 状態 | 意味 | 権威ある源 | 帰属 / 送り先 | 想定 M |
|---|---|---|---|---|
| talking | 喋っている途中 | kanade `Steady{talk}` | ✅ `idle-talk` 完了 | M1 |
| choosing | 選択肢表示中 | 選択肢UI | **`choice-select-events`**（既存 brief） | M1（M-dialogue） |
| balloon(ID群) | バルーン表示中 | emo バルーン表示状態（UIスレッド） | 本 spec（UI→kanade 配線＝TickInfo 同型） | M1〜M2 |
| minimizing | 最小化中 | UIスレッド窓状態 | 本 spec | M1〜M2 |
| induction | `\![enter,inductionmode]`中 | sakura 再生状態 | 本 spec（sakura mode 拡張連動） | M2 |
| passive | `\![enter,passivemode]`中 | SSTP | 本 spec（SSTP＝M2 送り） | M2 |
| timecritical | `\t` 区間中 | sakura 再生（`\t`） | 本 spec | M2 |
| nouserbreak | `\![enter,nouserbreak]`中 | SSTP／中断funnel | 本 spec | M2 |
| online | ネットワーク通信中 | ネットワーク層 | 本 spec（network＝M2 送り） | M2 |
| opening(種類) | 入力ボックス等表示 | ダイアログ/入力UI | 本 spec（dialog＝M2 送り） | M2 |

## Approach（源着地時 just-in-time）

1. **本 spec は台帳＝先行設計しない**。源サブシステムが実物として着地したら、その状態の実導出（純関数＋源読み口）を idle-talk の `Status` 構造へ additive に差し込む。
2. **契約不変**: idle-talk が確立した「カンマ連結・ヘッダ位置・空集合→行省略」の送出契約は変えない（idle-talk Req2.6 の差替シーム）。
2b. **消費側互換の檻（2026-07-17 合流裁定で登記・idle-talk Req2.6 ただし書きの台帳側正本）**: 実 pasta の talk 抑制ゲートは `status == "talking"` の**完全一致比較**（`vendors/pasta/.../virtual_dispatcher.lua:98,123`）＝カンマ連結値（例 `talking,online`）や非 talking 単独値で **fail-open**（talk 中に OnTalk が漏れる）。任意状態の実導出を解禁する際は、emo2 系消費側の互換検証（複合値 wire の実機/harness 観測）を差替の**受け入れ条件**に含めること。`choosing`（M1 最初の非 talking 値）は `choice-select-events` が同檻を先に踏む。
3. `choosing` は `choice-select-events` に委譲（重複実装しない・調整のみ）。
4. UI スレッド源（balloon／minimizing）は Reference1/Reference2 の窓 geometry 配線（TickInfo 拡張）と同一の配線問題＝相乗り可能。

## Scope

- **In**: ukadoc `Status` 残状態（choosing 除く）の実導出を、源サブシステム着地時に idle-talk の `Status` 構造へ差し替える調整・実装。
- **Out**: idle-talk が所有する `Status` 語彙構造・`talking` 実導出・カンマ連結送出契約（変更せず消費のみ）／choosing 実導出（`choice-select-events`）／源サブシステム自体の新設（SSTP・network・dialog＝M2／窓最小化ハンドリング等）。

## Boundary Candidates

- 各状態の実導出（純関数＝源スナップショット→状態トークン・全網羅可能）／送出契約への additive 差替。

## Out of Boundary

- `Status` 送出契約（idle-talk 所有）／源サブシステムの新設。

## Upstream / Downstream

- **Upstream**: `areka-P0-idle-talk`（`Status` 語彙・カンマ連結契約・差替シームの正本）。各源サブシステム spec（`choice-select-events`／将来の SSTP・network・window-state）。
- **Downstream**: `areka-P0-emo2-conformance-e2e`（適合検証に `Status` 実行状態を含める場合）。

## Existing Spec Touchpoints

- **Depends**: `areka-P0-idle-talk`（Status 語彙構造の正本）。
- **Coordinates**: `areka-P0-choice-select-events`（choosing の実導出を所有・本 spec は非重複）。

## Constraints

- Rust 2024・新規依存なし・tokio 不使用。
- **契約不変**: idle-talk の `Status` 送出契約（カンマ連結・空集合→省略）へ additive で乗る。
- 正典は ukadoc（`ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`）・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。
- **規律**: 憶測で先に設計しない（roadmap「spec 工場の禁止」）。源着地まで本 spec は台帳に留める。
