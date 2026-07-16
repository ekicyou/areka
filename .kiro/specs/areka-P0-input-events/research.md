# ギャップ分析: areka-P0-input-events

> 対象: 確定済み requirements.md（Req1〜8）と既存コードベースの実装ギャップ分析。
> 調査日: 2026-07-16。方針: 情報提供に徹し、最終決定は design/要件ディスカッションへ送る。
> 言語: ja（spec.json 準拠）。

## 分析サマリ（3〜5 点）

- **背骨は「マウス入力→kanade→GET→StartTalk」で、その 3 経路の資産はほぼ揃っている**: kanade の純粋状態機械（`schedule/`）・Reference 表正本（`events.rs`）・talk 起動調停（`steady.rs` の Value→StartTalk ＋ ghost `dispatcher.rs` の単一 slot 差替）・UI 側マウス基盤（wintf `OnPointerMoved`/`OnPointerPressed`＋`PointerState`）はいずれも完成済み。**足りないのは (1) kanade へのマウス variant 追加（additive）、(2) UI イベント→kanade チャンネル配線、(3) 正典 Reference 組立関数の 2 本**。
- **ukadoc 裏取り済み**: OnMouseMove（Ref0/1=ローカル座標・Ref2=ホイール・Ref3=本体0/相方1・Ref4=当たり判定・Ref5=※SSP/NINIX 常に0・Ref6=デバイス種）／OnMouseDoubleClick（Ref2=常に0・Ref5=左0/右1）は要件と整合。**ただし OnMouseMove の Ref5（SSP/NINIX のみ・常に "0"）が要件・brief に未記載**＝新規の design 判断点。
- **並走 spec `areka-P0-collision-geometry` の resolver 実体は本ワークツリーに未実装**（brief は同時制定で存在＝I/O 契約 `HitRegion{scope, region:Option<String>}` の正本）。結線点は resolver 1 個ゆえ、**mock resolver で決定論観測を完結させる形（Req1.5・Req8）が必須の前提**。
- **stand-in 即終了は `crates/areka/src/placement/spawn.rs::on_ghost_pressed`** が全 `GhostWindowMarker` 窓 despawn で実装。退役先の正規終了経路（`KanadeMsg::CloseRequest`／`ForceQuit`）は完備。暫定退避終了は `PointerState.ctrl_down` 等の既存フィールドを使う修飾つき操作が有力（design で 1 つ確定）。
- **kanade の inbox（`Sender<KanadeMsg>`）は現在 ECS World に載っていない**。UI スレッドのポインタハンドラ（`&mut World` を受ける）が kanade へ届くよう、`GhostRuntime::kanade()`（コメントに「後続 input-events の結線点」と明記）由来の Sender を World へ露出する seam の新設が UI 配線の核心。

---

## 1. 既存資産の地図（Current State）

### 1.1 kanade 純粋状態機械（`crates/areka-kanade/`）

| 資産 | 場所 | 本 spec との関係 |
|---|---|---|
| `Input` enum（Boot/Tick/TalkDone/CloseRequest/ForceQuit/ShioriDown/ShioriReply） | `schedule/mod.rs:36-45` | **`Input::Mouse*` を additive 追加**する拡張点。 |
| `Action` enum（ShioriRequest/ShioriUnload/StartTalk/StopSelf） | `schedule/mod.rs:104-112` | 変更不要（マウス GET も `ShioriRequest`、応答 talk も `StartTalk` を再利用）。 |
| `step()` 唯一の遷移入口（横断遷移→フェーズ委譲） | `schedule/mod.rs:120-158` | マウス Input のルーティング（横断 or `dispatch_phase`）を足す点。 |
| talk 調停（`Steady{None}+Value→StartTalk`・単調 talk_id 採番・`Steady{Some}+Value`=DD-6 破棄） | `schedule/steady.rs:89-125` | **マウス GET 応答 Value も同じアームに乗る**（新調停を発明しない・Req4）。マウス Input→GET 発行アームは steady に無く追加要。 |
| Reference 表の実装正本（純関数群・`on_boot`/`on_second_change`/`on_close` 等） | `schedule/events.rs` | **`on_mouse_move`/`on_mouse_double_click` を additive 追加**（Req2/3）。`pub` ファサード（`lib.rs:50-54`）経由でハーネスが期待値を共有。 |
| KanadeMsg inbox（Boot/Tick/TalkDone/CloseRequest/ForceQuit/ShioriDown/Close） | `msg.rs:46-61` | **`KanadeMsg::Mouse*` を additive 追加**する拡張点。 |
| `ShioriCall::{Get,Notify}`（GET/NOTIFY を型で区別） | `msg.rs:80-89` | マウス GET は `Get` variant を再利用。 |
| アクターシェル `drive()`（KanadeMsg→Input 写像・execute-batch/reinject-last・in-flight≤1 同期往復） | `actor.rs:59-70, 87-143` | マウス GET の応答は「最後の応答のみ再投入」でそのまま `ShioriReply`→`steady::on_reply` へ戻る。**新しい駆動モデルは不要**。KanadeMsg→Input の写像に Mouse* を 1 行足す。 |

### 1.2 ghost 結線層（`crates/areka-ghost/`）

- **`GhostRuntime::kanade() -> &Sender<KanadeMsg>`**（`runtime.rs:146-149`）: doc に「後続 input-events の結線点」と明記。UI 配線が握るべき唯一の投函端。
- **単一 slot 差替は ghost `dispatcher.rs:97-113`**（brief が「dispatcher.rs:97-113」と呼ぶ実体）: `on_start` が `close_active_if_any()`→新 talk spawn の Close-then-spawn を既存有無に関わらず踏む。**マウス応答 StartTalk も active talk を正しく差し替える**（stale 通知は自然棄却）。→ Req4「active talk 中の置換規律を既存棚で」が構造的に成立済み。
- boot 手順（`runtime.rs:301-389`）: shiori→kanade→dispatcher→relay×2→ticker→`KanadeMsg::Boot`。マウス配線は boot 後に kanade Sender を UI へ渡すだけで、boot トポロジ自体は不変。

### 1.3 UI 側マウス基盤（`crates/wintf/` + `crates/areka/src/placement/`）

- **ハンドラコンポーネント完備**: `OnPointerMoved`/`OnPointerPressed`/`OnPointerReleased`/`OnPointerEntered`/`OnPointerExited`（`wintf/src/ecs/pointer/dispatch/mod.rs:83-103`・Tunnel/Bubble 二相ディスパッチ）。→ **OnMouseMove は `OnPointerMoved`、ダブルクリックは既存 `OnPointerPressed` を消費できる**。
- **`PointerState`**（`wintf/src/ecs/pointer/types/mod.rs:90-129`）: `client_point`（**物理 px・クライアント座標**）・`local_point`・各ボタン押下・`shift_down`/`ctrl_down`・`double_click`（`DoubleClick::{Left,Right,…}`）・`wheel`。→ Ref0/1（ローカル座標）・Ref5（左右）・修飾（暫定退避終了）・ホイール（Ref2 の口）まで**必要な生値が揃っている**。
- **WM_MOUSEMOVE 経路**（`wintf/src/ecs/window_proc/mouse_move.rs:123-453`）: 物理 px client 座標を取り、`hit_test_in_window`→ヒット entity へ `PointerState` を挿入/更新。ドラッグ処理も同経路。マウス移動は毎 WM_MOUSEMOVE で発火＝Req5 間引きの必要性の直接根拠。
- **stand-in 即終了**（`crates/areka/src/placement/spawn.rs:321-344`）: `on_ghost_pressed` が `Phase::Bubble` の `DoubleClick::Left` で全 `GhostWindowMarker` 窓を despawn。**Req6 の退役対象**。ハンドラ signature は `fn(&mut World, sender, entity, &Phase<PointerState>) -> bool`＝World 経由で kanade Sender（Resource/NonSend）へ到達可能。
- ゴースト窓は生成時に `OnPointerPressed(on_ghost_pressed)` を付与済み（`spawn.rs:167,205`）・`HitTest::none()`（全面ヒット）。マウス移動配線は `OnPointerMoved(...)` を同様に付与するのが自然。

### 1.4 結線シーム（`crates/areka/src/emo2_boot/`）

- `wire_emo2_boot`（`emo2_boot/mod.rs:220-344`）が `areka_ghost::boot` を呼び、**`GhostRuntime` を `Emo2BootOutcome.ghost` で main へ返す**。kanade Sender はここから取り出せるが、**現在 World には載せていない**（＝UI ハンドラから届かない）。UI 配線の追加点はこの結線層（brief 記載の「emo2_boot 結線層」と一致）。
- `Emo2Wiring` を NonSend 挿入し `FrameFinalize` に system 登録する前例あり（`mod.rs:326-333`）＝マウス配線資源（kanade Sender・throttle 状態・resolver）を World へ載せる作法の donor。

### 1.5 collision-geometry 契約（相方 spec）

- **`.kiro/specs/areka-P0-collision-geometry/brief.md` は存在**（同時制定 2026-07-16・I/O 契約の正本）。契約: `HitRegion{scope:usize, region:Option<String>}`／入力 `(scope, 窓 client 物理 px 座標)`／「現在の surface id は emo 側が内部で引く」／提供形は UI スレッド同期呼出 resolver（channel 化不要）。
- **resolver の実体コードは本ワークツリーに未実装**（`HitRegion`/resolver 型は grep 不検出）。→ 本 spec は**消費側**。並走を詰ませないため **mock resolver で全経路を決定論観測**（Req1.5/Req8）する形が前提。
- 参考: wintf に `hit_region`/`hit_test` モジュールが既存（`event-hit-test-named-regions` 完了・`wintf/src/ecs/layout/hit_region/`）。collision-geometry がこれを土台にする可能性はあるが、**本 spec は collision-geometry の出力契約のみを消費**し、幾何解決を自前で再定義しない（Req1.3）。

### 1.6 観測ハーネスの母体（Req8）

- `crates/areka-kanade/tests/kanade/common/`（`spawn_harness`/`spawn_harness_gated`/mock shiori・mock sakura sink・`Fixture`・`RecordedCall`）が既存。**mock shiori＋注入入力＋quit 経路での決定論同期（sleep 不使用）**という Req8 が要求する流儀そのものが確立済み（`steady_test.rs` 冒頭 doc）。→ Req8(a)〜(e) はこのハーネスの additive 拡張で檻化可能。

### 1.7 ukadoc 正典（裏取り・2026-07-16）

| Reference | OnMouseMove | OnMouseDoubleClick |
|---|---|---|
| Ref0/1 | x/y（ローカル座標） | x/y（ローカル座標） |
| Ref2 | ホイール回転量・方向 | 常に 0 |
| Ref3 | 本体 0／相方 1（SSP/CROW は 2 以降も） | 同左 |
| Ref4 | 当たり判定の識別子 | 当たり判定の識別子 |
| Ref5 | **※SSP/NINIX のみ・常に 0** | 左 0／右 1 |
| Ref6 | ※SSP のみ・デバイス種（touch/pen/eraser/mouse） | ※SSP/NINIX のみ・デバイス種 |

→ 要件 Req2.2／Req3.2 と整合。**OnMouseMove の Ref5 は要件・brief が触れていない**（design 判断点として下記に追加）。GET/NOTIFY の使い分けはマウス系＝スクリプト応答があり得るため GET が基本（`ukadoc:memo_shiorievent`・brief 記載）。

---

## 2. Requirement → Asset マップ（gap タグ: Missing / Unknown / Constraint）

| Req | 必要能力 | 既存資産 | ギャップ |
|---|---|---|---|
| **R1** 取得・配信 | OnPointerMoved/Pressed・PointerState 物理px | wintf 完備・spawn.rs ハンドラ付与済み | **Missing**: UI→kanade 配線（Sender の World 露出＋ハンドラから送出）。**Constraint**: resolver は相方 spec（未実装）。**Missing**: mock resolver（Req1.5）。 |
| **R2** OnMouseMove 組立 | 正典 Ref 組立の純関数 | `events.rs` の同型純関数群 | **Missing**: `on_mouse_move`（additive）。**Unknown(design)**: Ref4 の `None` 値（空文字/省略）・Ref6 デバイス種の具体値・**Ref5 発行可否**。 |
| **R3** OnMouseDoubleClick 組立 | Ref5=左0/右1 含む純関数 | `events.rs` 同型・PointerState.double_click | **Missing**: `on_mouse_double_click`（additive）。Ref4 は R2 と同一規則。 |
| **R4** talk 起動調停（既存棚） | Value→StartTalk＋単一slot差替 | `steady.rs:89-125`＋ghost `dispatcher.rs:97-113` 完備 | **Missing**: マウス Input→GET 発行アーム（steady へ additive）。**Unknown(design)**: talk 再生中のマウス GET 扱い（送出/抑止/NOTIFY 化）。 |
| **R5** 送出間引き | 純粋・決定的な間引き判定 | `events.rs` の純関数化前例 | **Missing**: 間引き関数＋その状態の置き場。**Unknown(design)**: 規則を 1 つ確定（当たり判定変化＋一定間隔 等）。 |
| **R6** stand-in 退役＋暫定退避 | 正規終了経路への差し替え | `on_ghost_pressed`（退役対象）・kanade CloseRequest/ForceQuit 完備 | **Missing**: despawn→OnMouseDoubleClick 送出への差替。**Unknown(design)**: 暫定退避手段 1 つ（例 Ctrl+ダブルクリック→ForceQuit）。 |
| **R7** 送出集合限定 | Move/DblClick のみ・Wheel/Click 単発は送らない | 配線層の分岐点 | **Missing**: 送出集合の分岐（純関数檻）。idle-talk のホワイトリスト檻と整合表を design 冒頭で確定。 |
| **R8** 決定論観測＋実機 | mock shiori・注入入力・sleep 不使用 | `tests/kanade/common` ハーネス母体 | **Missing**: (a)〜(e) の檻を additive 追加。実機サインオフは手動（Constraint: 実 emo2/pasta.dll/DPI）。 |

---

## 3. 実装アプローチ Options

本 spec は **kanade 増分（純粋・全網羅）** と **UI 配線（結線層・薄い）** の 2 面を持つため、全体としては **Option C（Hybrid）** が構造的に自然。各面での選択肢を以下に示す。

### 3.1 kanade 面 — Option A（既存拡張）を推奨

- **内容**: `KanadeMsg::Mouse*`／`Input::Mouse*`（座標＋scope＋region＋修飾＋左右/種別）を additive 追加 → `events.rs` に `on_mouse_move`/`on_mouse_double_click` を追加 → `actor.rs` の写像に 2 行 → steady にマウス Input→GET 発行アームを追加 → 応答 Value は既存 `steady::on_reply` の Value→StartTalk へ合流。
- **トレードオフ**: ✅ 確立済みパターンの additive で決定論資産（純粋 step・log 檻・pub events ファサード）をそのまま活用。✅ 新しい調停/駆動モデルを発明しない（Req4.4）。❌ steady.rs が担う入力種が増える（ただし Tick/ShioriReply/TalkDone/CloseRequest に 1 種追加の範囲）。
- **副論点（ルーティング）**: マウス Input を (a) `step()` の横断アームで受け Steady のみ処理し他 Phase は防御的無視、か (b) `dispatch_phase` 経由で各フェーズへ委譲（boot/close の `_` 防御アームが warn+無視）。**boot/close 中のマウスは M1 で無視が妥当**だが、どちらの実装で無視するかは additive 判断（design）。

### 3.2 UI 配線 面 — Option B（新規結線モジュール）を推奨

- **内容**: emo2_boot 結線層（または placement）に「ポインタ→kanade」配線を新設。`OnPointerMoved` ハンドラで `PointerState.client_point`（物理 px）→ (mock/実) resolver → `HitRegion` → 間引き判定 → `KanadeMsg::MouseMove` 送出。`on_ghost_pressed` は despawn を撤去し `DoubleClick::Left/Right` → `KanadeMsg::MouseDoubleClick` 送出へ差替。
- **kanade Sender の World 露出**: `GhostRuntime::kanade().clone()` を **Resource（`Sender<KanadeMsg>` は `Send`）** か **NonSend** で World へ挿入。resolver は「UI 所有データ」ゆえ NonSend が自然（channel 化不要・collision brief）。両者の同居方式は design。
- **トレードオフ**: ✅ 責務分離（配線層は薄い・kanade は純粋のまま）。✅ mock resolver 差し替えで headless 決定論檻（Req1.5）。❌ UI スレッド依存ゆえ実機確認が必須（Req8.3）。❌ throttle 状態・resolver・Sender の 3 資源を World へ載せる結線が増える。

### 3.3 talk 再生中のマウス GET（design の核心分岐）

3 案（要件 Req4.3 が design 送り）:
- **(A) GET 送出し Value を破棄**: 既存 `steady_value_during_talk`（DD-6 防御）に自然に落ちるが、SHIORI にイベントは届くのに応答が捨てられる＝無駄・状態不整合の懸念。
- **(B) 抑止**（talk 中は GET を出さない）: 最も安全だが SHIORI がイベントを取りこぼす。
- **(C) NOTIFY 化**: OnSecondChange の DD-6 と同型（talk 中は NOTIFY・応答は構造的破棄）。既存パターンの延長で最も一貫。
- → **SSP 実挙動を ukadoc/実機で確認して 1 つ確定**。C が既存資産と最も整合するが要検証。

### 3.4 暫定退避終了（Req6.2）

- **有力案**: `on_ghost_pressed` で `PointerState.ctrl_down && DoubleClick::Left`（既存フィールド）→ `KanadeMsg::ForceQuit`（正規経路・`GhostRuntime::shutdown` 相当）を送出。stand-in の直接 despawn を新設しない（Req6.3）。
- **別案**: 既存 env-gate（`AREKA_APP_SMOKE_EXIT_MS` 系）を退避手段として明示。
- → design で 1 つ確定し「暫定」と記録。

---

## 4. Effort / Risk

| 面 | Effort | Risk | 根拠 |
|---|---|---|---|
| kanade 増分（Input/KanadeMsg/events/steady/actor 写像＋純粋テスト） | **S〜M** | **Low** | 確立済み純粋状態機械への additive・events.rs 同型・純関数は GPU 不要で全網羅。 |
| UI 配線（ハンドラ＋throttle＋Sender/resolver 露出＋mock 境界） | **M** | **Medium** | UI スレッド・実機依存・mock/実 resolver の差し替え境界設計。新規結線が 3 資源。 |
| stand-in 退役＋暫定退避 | **S** | **Low〜Medium** | 実アプリ挙動変更ゆえ実機人間確認が必須（Req6/8.3）。 |
| 観測ハーネス拡張（Req8 a〜e） | **S〜M** | **Low** | `tests/kanade/common` 母体あり・sleep 不使用の前例。 |
| **全体** | **M** | **Medium** | 主因は (1) collision-geometry 並走＝mock 境界前提、(2) talk 再生中 GET の SSP 挙動確定、(3) 実機サインオフの手動性。 |

---

## 5. design フェーズへの申し送り

### 5.1 推奨アプローチ

- kanade = **Option A（additive 拡張）**、UI = **Option B（新規結線モジュール）** の **Hybrid**。マウス応答 talk は既存 `steady::on_reply`＋ghost `dispatcher` の単一 slot 差替に合流させ、新調停を発明しない（Req4）。
- 決定論の要: **mock resolver** を第一級に置き、collision-geometry 未完でも Req8(a)〜(e) が単一 pass/fail で閉じる形にする。

### 5.2 design で確定すべき Research Needed 項目

要件が明示している 6 項目（requirements.md「design 送り事項」）に加え、本分析で判明した論点:

1. **（要件既載）** talk 再生中のマウス GET の扱い（送出/抑止/NOTIFY 化）を SSP 挙動で確定。→ 3.3 の C（NOTIFY 化・DD-6 整合）を既定に検証。
2. **（要件既載）** 右ダブルクリックの SSP 既定動作（本体メニュー/ゴースト送出）。M1 は owner-draw メニュー不在ゆえ右も SHIORI へ素直に送る案を検証。
3. **（要件既載）** OnMouseMove 間引き規則を 1 つ確定（当たり判定変化時＋一定間隔 等）。
4. **（要件既載）** 暫定退避終了の具体手段を 1 つ（Ctrl+ダブルクリック→ForceQuit が有力・3.4）。
5. **（要件既載）** 当たり判定 `None` 時の Ref4 値（空文字転写/省略）・Ref6 デバイス種の具体値（"mouse" 固定が既定候補）を ukadoc/SSP で確定。
6. **（要件既載）** M1 送出マウスイベント集合表（Move/DblClick の 2 種）を idle-talk のホワイトリスト檻と整合。
7. **（要件ディスカッション #1 で解決＝SSP 準拠で送出）** **OnMouseMove の Ref5** を固定値 "0" で送出する（SSP/NINIX と同一・移動時は常に 0 の予約枠・Ref2 wheel の "0" 固定 seam と対称で Reference 構造を DoubleClick と一致）。requirements Req2.2／Req2.5 に反映済み。OnMouseDoubleClick の Ref5（左 0／右 1）とは意味が別（あちらは実ボタン識別）である点に注意。
8. **（新規・本分析）** マウス Input のフェーズルーティング（`step()` 横断アームで Steady のみ処理 vs `dispatch_phase` 経由で boot/close は防御的無視）。
9. **（新規・本分析）** kanade Sender の World 露出方式（Resource vs NonSend）と、間引き状態・(mock/実) resolver の同居・置き場（per-scope/per-window 粒度）。
10. **（新規・本分析）** 座標契約の明文確認: PointerState.`client_point`（物理 px）＝Ref0/1 の「ローカル座標」＝resolver 入力の「窓 client 物理 px」の三者一致（DPI 等倍・collision brief §座標系）を design で固定。

### 5.3 mock/実 境界（決定論の要）

- **resolver 境界**: 偽装境界パターン（本 repo 慣行・`ClickThroughRegistrar`/`ScriptedShioriBackend` と同精神）で `trait` 化し、mock resolver（固定 region 返却）を注入。UI 配線の headless 檻はここで閉じる。
- **観測ハーネス**: `tests/kanade/common` を母体に、注入マウス Input→GET・Ref0〜6 期待 layout（region 転写含む）／左ダブルクリック Ref5="0"／応答 Value→StartTalk（active talk 中の置換）／204→無動作／間引き規則、を単一 pass/fail で檻化（sleep 不使用）。

### 5.4 次ステップ

- 本ギャップ分析を要件ディスカッションで確認 → `/kiro-design areka-P0-input-events` で技術設計へ進む。
- design 冒頭で ukadoc `OnMouseMove`/`OnMouseDoubleClick`/`memo_shiorievent` を再参照し、5.2 の 10 項目を確定表にすること。
