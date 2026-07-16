# Brief: areka-P0-input-events

> **種別**: 本坑（main）増分。③ kanade 帰属（M-life「撫でクラスタ」の片側＋M-dialogue の入口）。roadmap 増分「③ kanade: `input-events`（OnMouseMove/OnMouseDoubleClick/OnChoiceSelectEx 配信）」のうち **OnMouseMove/OnMouseDoubleClick を本 spec が所有**（OnChoiceSelectEx は選択肢 UI＝M-dialogue `choice-render` 完了後の増分へ分離）。
> **調査日**: 2026-07-16（再入精査⑦・実装シーム偵察＋ukadoc 裏取り）。
> **クロスエンジン結合**: 撫で＝`collision-geometry`（⑥入力解決）⟷ **本 spec**（③SHIORI 配信）——**region/actor I/O 契約の正本は collision-geometry brief**（2026-07-16 同時制定・本 brief は消費側＝再定義しない）。
> **並走性**: cue モデル非接触＝実装中の `areka-P0-cue-playback-duration` と**並走可**。

## Problem

マウス入力を SHIORI へ届ける経路が**ゼロ**。emo2 の撫で（`dic/touch.pasta:7`＝OnMouseMove）とダブルクリックメニュー（`dic/menu.pasta:10`＝OnMouseDoubleClick→`\q` 選択肢）は M1 ゴール（boot→talk→**touch**→**menu**→close）の中核だが:

- kanade の `Input` enum（`crates/areka-kanade/src/schedule/mod.rs:36-45`）に**マウス系 variant が無い**。`OnMouseMove`/`OnMouseDoubleClick` 文字列は areka 実装コードに存在しない（host 仕様 doc のみ）。
- 現在のダブルクリックは **stand-in の即終了**（`on_ghost_pressed`＝`crates/areka/src/placement/spawn.rs:321-344` が全 GhostWindowMarker 窓を despawn）——本来は OnMouseDoubleClick を SHIORI へ送り、ゴーストがメニューで応える（終了もメニュー経由 `\-`）。[[canonical-not-minimal-lifecycle]] の精神で正規経路へ差し替える時期。

## Current State（2026-07-16 実装偵察）

- **kanade の受け皿**: `Input`（Boot/Tick/TalkDone/CloseRequest/ForceQuit/ShioriDown/ShioriReply）・`Action`（ShioriRequest/ShioriUnload/StartTalk/StopSelf）＝`mod.rs:36-45,104-112`。**応答 Value→StartTalk の調停は Steady に既存**（`steady.rs:92-103`・active talk 単一 slot は dispatcher `dispatcher.rs:97-113` が Close→差替で吸収）——マウス GET の応答も**同じ棚**に乗せられる。
- **UI 側の観測点**: ゴースト窓は `OnPointerPressed`（`spawn.rs:321-344`）・`DragConfig`＋`OnDrag`（`spawn.rs:196-205`）を既に張っている＝wintf のマウスイベント基盤（event-mouse-basic/hit-test ✅）は完備。**足りないのは「イベント→kanade チャンネル→SHIORI」の配線だけ**。
- **当たり判定名の解決**: `collision-geometry`（同時 brief）の resolver が UI スレッドで `HitRegion{scope, region}` を返す契約（正本は同 brief）。
- **正典 Reference layout（ukadoc 裏取り済み 2026-07-16）**:
  - OnMouseMove: Ref0=x（ローカル）・Ref1=y・Ref2=ホイール回転量・Ref3=本体0/相方1・Ref4=当たり判定の識別子・Ref6=入力デバイス種（SSP は mouse 等）。
  - OnMouseDoubleClick: Ref0/1=座標・Ref2=常に0・Ref3=scope・Ref4=当たり判定・**Ref5=左0/右1**・Ref6=デバイス種。

## Desired Outcome

撫で（OnMouseMove）とダブルクリック（OnMouseDoubleClick）が SHIORI へ正典 Reference で届き、実機で **Head を撫でると touch.pasta が反応**し、**ダブルクリックで menu.pasta の応答 talk が起動**する（`\q` 選択肢の**見た目の完成度は M-dialogue `choice-render` の領分**——本 spec は「応答 talk が再生される」まで）。stand-in の dblclick 即終了は正規経路へ退役し、**暫定の退避終了手段**が明示的に残る。

**✔ 観測（単一 pass/fail）**: 決定論（mock shiori・注入入力・sleep 不使用）＝(a) MouseMove 入力→GET・Ref0〜6 が期待 layout（region 転写含む） (b) DoubleClick（左）→GET・Ref5="0" (c) 応答 Value→StartTalk（既存 slot 調停・active talk 中の置換規律） (d) 204→無動作 (e) 送出間引き規則の決定論檻。＋実機＝実 emo2 で撫で反応とダブルクリックメニュー talk の人間サインオフ。

## Approach

1. **kanade 増分（additive）**: `Input::Mouse*`（Move/DoubleClick・座標＋scope＋region＋修飾）variant 追加→ `events.rs` に正典 Reference 組立（region は**不透明 String 転写**・collision 外の Ref4 値は ukadoc/SSP 挙動で確定）→ Steady で GET 発行・応答 Value は既存 StartTalk 棚へ（**新しい調停を発明しない**）。active talk 中のマウス GET の扱い（送る/抑止/NOTIFY 化）は SSP 挙動を ukadoc で確認し design で確定。
2. **UI 配線（emo2_boot 結線層）**: ゴースト窓ハンドラ→collision-geometry resolver（同時 brief の契約消費）→`KanadeMsg` 送出（ghost の既存 relay/channel 規約＝[[areka-concurrency-model]] に載せる・自前流儀を発明しない）。
3. **dblclick 即終了の退役と退避**: `on_ghost_pressed` の despawn を OnMouseDoubleClick 送出へ差し替え。**暫定退避終了**（例: Ctrl+ダブルクリック or 既存 env-gate `AREKA_APP_SMOKE_EXIT_MS` 系）を design で1つ確定して残す——メニューからの `\-` 終了（M-dialogue 完成）までアプリを閉じる手段を絶やさない。
4. **OnMouseMove の送出間引き**: 毎 WM_MOUSEMOVE→GET は過剰。**「撫での解釈（連打の意味論）は SHIORI 側の領分」**（scope doc §1）ゆえ送る側は意味論を発明せず、**機械的な間引き規則**（例: collision 変化時＋一定間隔・SSP の de-facto を ukadoc/観察で確認）を design で1つ確定・決定論檻に。
5. **wheel は口だけ**: Ref2（ホイール）は layout 上存在するが emo2 M1 実需外——0 固定＋increment シーム（OnMouseWheel イベント自体は送らない）。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-16）

- **collision-geometry が契約正本**: `HitRegion{scope, region}`・resolver の提供形は同 brief の「region/actor I/O 契約」節を消費（**再定義しない**）。並走時の結線点は resolver 1 個＝どちらが先に完了しても接続可能（片側未完なら mock resolver で決定論観測が完結する形に）。
- **cue-playback-duration と交差面ゼロ**: 編集面＝kanade（Input/events/steady）＋emo2_boot 結線＋spawn.rs ハンドラ。dola/sakura/emo-text/seriko 不触。**実機サインオフの判定分離**: 「応答 talk が起動する」ことのみ本 spec・再生タイミング品質は cue-playback 帰属。
- **position-persist との近接**: 双方 `spawn.rs`/kanade を触るが、persist＝placements 注入＋follow.rs DragEnd／本 spec＝pressed ハンドラ＋Input variant——**別関数・additive**＝並走可（マージ近接注意のみ）。
- **idle-talk との近接**: 双方 kanade events/steady を触る——idle-talk＝OnSecondChange の Ref/Status 充足・本 spec＝マウス2イベント新設＝**別イベント・additive アーム**。`Status` ヘッダの口（idle-talk が設計）へ将来 `choosing` を足すのは M-dialogue 側と申し送り。
- **M-dialogue への申し送り（詰み防止）**: OnChoiceSelectEx は `\q` 表示（choice-render）＋選択 UI が前提＝本 spec から**明示分離**。ただし本 spec の「マウス入力→kanade→GET→StartTalk」背骨と Ref 組立の型は choice 確定イベントがそのまま再利用できる形（イベント種 enum の拡張余地）に切る。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-16 裏取り済み）

- **`ukadoc:list_shiori_event:OnMouseMove:1`**（裏取り済み・Ref0-6 layout）・**`ukadoc:list_shiori_event:OnMouseDoubleClick:1`**（裏取り済み・Ref5=左0/右1）。**右ダブルクリックの SSP 既定動作**（本体メニュー？ゴースト送出？）を design で確認——M1 は owner-draw メニュー無し（M2）ゆえ右も SHIORI へ素直に送る案を既定に検証。
- **`ukadoc:memo_shiorievent`**: GET/NOTIFY の使い分け総論（マウス系はスクリプト応答があり得る＝GET が基本）。
- **具体指示**: design 冒頭で「M1 で送るマウスイベント＝OnMouseMove/OnMouseDoubleClick の2つだけ・OnMouseClick 単発は未ハンドル 204 ゆえ M1 省略可（scope doc §1）・OnMouseWheel 不送出」の送出集合表を確定し、idle-talk のホワイトリスト檻と整合させること。

## Scope

- **In**: kanade `Input::Mouse*` variant＋正典 Reference 組立（GET）／応答 Value→既存 StartTalk 棚（調停規律の檻）／UI 配線（resolver 消費→KanadeMsg）／dblclick 即終了の退役＋暫定退避終了／MouseMove 間引き規則（決定論檻）／実機サインオフ（撫で反応・メニュー talk 起動）。
- **Out**: `\q` 選択肢の表示・選択 UI・OnChoiceSelectEx（M-dialogue: `choice-render`＋増分）／撫で意味論の解釈（SHIORI 側）／OnMouseWheel・OnMouseClick 単発・The Hand（M1 外/M2）／collision 解決そのもの（**collision-geometry**）／owner-draw 右クリックメニュー（M2）。

## Boundary Candidates

- kanade イベント組立（純粋・全網羅）／UI 配線（結線層・薄い）／間引き規則（純関数）／stand-in 退役（アプリ挙動変更＝実機確認必須）。

## Out of Boundary

- 選択肢クラスタ（M-dialogue の I/O 契約は着手時に先決）／窓ドラッグ・位置（placement 完了域）。

## Upstream / Downstream

- **Upstream**: **`areka-P0-collision-geometry`**（契約正本・並走可・結線点 resolver 1個）／`completed/areka-P0-kanade`（Steady・StartTalk 棚）／`completed/areka-P0-ghost-setup`（channel/relay 規約）／`completed/areka-P0-emo2-boot`（結線層）／wintf event 基盤 ✅。
- **Downstream**: M-life 統合（撫で一周）／M-dialogue（OnChoiceSelectEx が背骨を再利用）／`emo2-conformance-e2e`（touch→menu を一周適合に含める）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-kanade`（Input/Action の additive 増分・純粋状態機械の決定論資産を壊さない）。
- **Adjacent**: `areka-P0-collision-geometry`（契約先決の相方）／`areka-P0-position-persist`・`areka-P0-idle-talk`（kanade/spawn.rs 近接・別関数 additive＝並走可）／`areka-P0-cue-playback-duration`（**交差面ゼロ**）。

## Constraints

- Rust 2024・新規依存なし・tokio 不使用。エンジン間は actor-foundation 規約の channel（[[areka-concurrency-model]]・自前流儀禁止）。
- **決定論**: mock shiori＋注入入力で全経路網羅（[[deterministic-test-coverage-mandate]]）。間引き・Ref 組立は純関数化（[[test-only-decision-branches-not-proven-wiring]]）。
- region は不透明 String 転写（[[areka-surface-args-opaque-string-downstream-resolve]]）。終了経路は正規実装（[[canonical-not-minimal-lifecycle]]＝stand-in 退役・退避手段は明示的暫定として記録）。
- **実機受け入れ**: 実 emo2・実 pasta.dll・実 DPI で撫で反応＋メニュー talk（[[areka-placement-real-ghost-first]]）。起動は絶対パス必須（MOD_NOT_FOUND 運用注意）。
