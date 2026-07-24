# ギャップ分析: areka-P0-choice-render

> 生成日: 2026-07-23 ／ 対象: 確定済み requirements.md（Req 1–9）＋brief.md
> 本書は **情報提供（分析と選択肢）** であり実装決定ではない。設計判断項目は末尾に集約する。

## 分析サマリ（要点）

- **供給側は完全着地・表示器だけが空席**。dola `CuePlayer` は `Choice` cue を配送列へ第一級 broadcast しつつ `pending_choices` バッグへも積み（二重真実源）、`WaitForChoice` バリアで停止する。emo-text の受信〜純粋状態機械〜レイアウト〜probe metrics〜viewbox 差分描画〜供給面提示までの縦経路は完成済みで、`Choice`／`Cursor` cue は **actor ごと初回 warn＋良性スキップの明示シーム**（`state.rs:234-251`）として置いてある。本 spec はこの 2 シームを描画へ置換する additive 増分。
- **再利用シームが設計時点から予約済み**。`PositionedLine.rect`（image px 絶対矩形）＋`PositionedGlyph{ch,inline_pos,advance}` は「choice-render のクリック可能範囲導出がそのまま再利用できる（導出自体は実装しない・R9.4）」と layout.rs 冒頭 doc に明記。`ResidentContent` は `#[non_exhaustive]` で additive variant 追加が非破壊。`viewbox.rs` は `ScrollPlanner::scroll_state()` を「choice-render 座標契約点（R9.3）」として公開済み。`em`＝`font.height`／`lh`＝`line_pitch=ceil(font.height×1.25)` の materials（`DWriteMetrics`／`GlyphMetrics::line_pitch`）も揃う。
- **欠落は 5 点**: (1) 選択肢 resident 表現、(2) `\_l` カーソル消費（em/lh→px 換算＋レイアウトカーソル移動）、(3) 行ヒットジオメトリ（行矩形→id）契約正本の公開、(4) hover 状態注入 API＋ハイライト描画（cursor.\* スタイル準拠＋矩形反転縮退・差分再描画）、(5) 選択肢＋hit の原子的無効化。いずれも既存 GlyphRun 経路・viewbox 差分描画・request_clear の上に additive で乗る。
- **正典の視覚仕様は設計フェーズで ukadoc＋SSP 実観察が必要**。cursor.\* スタイルマップ（style/brush.color/pen.color/font.color/blendmethod）・クリック領域の行全幅/文字幅・`em/lh` 単位のフォント高基準/行高基準・負値/省略の縮退は fixture（`emo2-kakukaku/descript.txt` 指定・scope doc §4「矩形反転で代替可」）を最小適合サンプルとしつつ ukadoc で確定する（Research Needed）。
- **下流境界は既に切り分け済み**。`ChoiceSelection` 発行・実ポインタ配線・hover 追従駆動・クリック解決は `areka-P0-choice-interact`（brief.md 実在）の領分。本 spec は「注入 hover で決定論描画」「幾何と id 対応の公開」に留まり、`WaitForChoice` の解決＝再開も担わない（照会のみ）。

## 現状調査（既存資産・パターン）

### 対象 crate と層規律（`areka-emo-text`）

`crates/areka-emo-text/src/` は 3 層一方向（lib.rs 冒頭が正本・逆流はレビューエラー）:

| 層 | モジュール | 責務 | windows 依存 |
|---|---|---|---|
| 純粋層 | `state` `writing` `region` `layout` `canvas` `viewbox` `segment` `wrap` | 決定論檻・cue→状態→レイアウト→canvas→スクロール計画 | 禁止（構造檻 `lib.rs::pure_layer_modules_have_no_windows_imports`） |
| COM 層 | `draw` `surface` `viewbox_draw` | DirectWrite/D2D/DXGI/WUC・UI スレッド専有 | 唯一許可 |
| 結線層 | `sink` `actor` | cue 受信・UI 配送・フレーム提示 | 一部 |

依存方向は `areka-parsers / areka-sakura / areka-actor → areka-emo-text ← wintf`、`areka-emo-atlas → -compose → -present → -text`。**emo-present → emo-text の逆流は禁止**（R9.5/R9.6・本 spec も emo-present 本体を改変しない）。

### Choice/Cursor cue の現行の姿（置換対象シーム）

- **cue 定義**（`dola/src/cue/command.rs:143-179`、`areka-sakura::contract` 経由で消費）:
  - `CueCommand::Choice { id: String, text: String, references: Vec<String> }`（`references` は `\q` 第3引数以降の不透明文字列列）。
  - `CueCommand::Cursor { x: String, y: String }`（`\_l` の不透明転写・単位付き `5em`/`2lh`/`50%`・裸数値・相対 `@`・空の区別を保持・dola は換算しない）。
- **純粋状態機械**（`state.rs:234-251`）: `Choice` は `choice_warned` once-guard で warn＋スキップ（テキスト状態を汚さない・actor エントリも作らない）。`Cursor` は `cursor_warned` once-guard で同様。**本 spec の宛先はこの 2 アーム**。
- **結線側**（`actor.rs:249-257`）: `TextLayerRuntime::apply_cue` は `Choice`／`Cursor` を「描画実行部への全域クリアを要さない」群として no-op 明示（catch-all なし＝variant 追加時にコンパイラが再検討強制）。
- **配送モデル**（`dola/src/cue/runtime.rs:196-234`）: `Choice` cue は配送列（`ready()`＝表示の真実源）へ FIFO 複写 **かつ** `pending_choices`（照合の真実源）へ push。`WaitForChoice` バリアで `WaitingForChoice` へ停止。emo-text は `EmoTextSink`（`sink.rs`・`dola::cue::CueSink` 実装）で受け、`TextMsg::Cue`→`runtime.apply_cue`→`state.apply_cue` の FIFO 経路で消費する。本 spec は **配送された Choice cue の消費**で確定（`pending_choices()` 直読みではない）。

### 描画資産（再利用の土台）

- **住人モデル**（`canvas.rs:171-205`）: `ResidentContent`（`#[non_exhaustive]`）＝`GlyphRun(GlyphRunContent) | Image(ImageSeam) | Surface(SurfaceSeam)`。`Resident { content, transform: RegionTransform, effects: TextEffects }`。`ContentCanvas { residents: Vec<Resident>, size }`。`from_layout(lines, region, mode)` が 1 行=1 グリフ住人へ写像（行 index 1:1）。
- **行ヒット幾何の素材**（`layout.rs:101-132`）: `LineRect{left,top,right,bottom}`（image px 絶対）・`PositionedGlyph{ch,inline_pos,advance}`（行内軸絶対位置＋送り幅）。R9.4「クリック可能範囲導出がそのまま再利用可（導出は未実装）」。
- **レイアウトカーソルの起点**（`layout.rs:216-227`）: `inline_start`/`block_start` は `region.start()` 由来。`\_l` はこの起点をずらす必要があるが、**現状 layout にカーソル注入入力は無い**。改行遅延（newline-defer・`pending: Option<f32>`）・折返し（budoux）不変条件と整合させる要検討点。
- **座標契約**（`region.rs`）: `ScaleContract{scale, author_dpi}`・`to_physical`（image×k）・`image_size`。`em/lh`→物理 px は「font metrics（`em=font.height`／`lh=line_pitch`）→ image px → `to_physical` の一点」で導ける。DPI≠96 でも一貫（k は `SetTransform` 一点適用）。
- **本番描画経路**（`viewbox_draw.rs::ViewboxExecutor`）: `ScrollPlanner`（`viewbox.rs`）が `FramePlan`（`NoChange`/`FullClear`/`Update{blit,dirty,draw_lines}`）を純粋算出し、COM 層が差分描画。**ハイライトはこの差分（ダーティ矩形）に乗せる**必要（R4.4・全域再描画へ退行禁止）。`DrawExecutor`（`draw.rs`・`#[cfg(test)]`）は比較専用オラクル。両者は `LineLayoutStore` で行 TextLayout 生成を共有。
- **集約ルート**（`actor.rs::TextLayerRuntime`／`present_frame`）: actor 別 `ActorRender{surface, executor, metrics}`。`present_frame` は per-actor「reveal 進行（純粋）→ `LayoutEngine::layout` → `visible_window` → `from_layout` → `executor.render` → 変化時のみ present」。`apply_cue` が `Clear`/`ClearAll`→`executor.request_clear` を配線済み（表示と状態の原子消去の既存パターン）。
- **供給面/契約点**（`emo-present`）: `TextSlotView`（slot/window/surface_size/scale・`#[non_exhaustive]`・読み取り専用）。`PresentCommand` は Show/Hide/Invalidate のみ＝**present 層でハイライトを重ねる経路は無い**→ canvas 内自前合成（1 枚物・own compositor 思想）。`EmoPresenter::hit_region` は shell/balloon 画像の衝突（画家のアルゴリズム）で選択肢行 hit とは別物。

### 下流・隣接 spec

- `areka-P0-choice-interact`（brief.md 実在・W4）: `ChoiceSelection` 契約正本・実ポインタ配線・hover 追従駆動・クリック解決。**本 spec の行ヒットジオメトリ＋hover 注入 API を消費する。**
- `areka-P0-choice-select-events`: 選択確定→SHIORI カスケード・`Status: choosing`。
- fixture: `emo2-kakukaku/descript.txt`（cursor.\* 指定バルーン）・`menu.pasta`（`\n` 区切り 2〜4 項目の短い縦並びメニュー）。scope doc §4: cursor.\* 省略可＝矩形反転代替。

## 要件→資産マップ（ギャップ分類）

| 要件 | 既存資産（再利用） | ギャップ | 分類 |
|---|---|---|---|
| **R1** 選択肢 cue 消費＋行描画 | `state.rs` Choice シーム／`ResidentContent`＝non_exhaustive／`from_layout` グリフ経路／配送列消費経路 | 選択肢 resident 表現・cue→resident 写像・「選択肢表示中」照会状態 | Missing |
| **R2** `\_l` 消費＋字下げ | `Cursor` シーム／`ScaleContract`／`em=font.height`/`lh=line_pitch` metrics | em/lh→px 換算・レイアウトカーソル移動・newline-defer/折返し不変条件との整合 | Missing＋Constraint |
| **R3** 行ヒットジオメトリ＋id 契約 | `LineRect`／`PositionedGlyph`（R9.4 予約）／`scroll_state()` 契約点 | 行矩形→id 対応の保持・下流照会 API の契約正本・スクロール可視窓反映後の座標整合 | Missing |
| **R4** hover 注入＋ハイライト | `ScrollPlanner`/`FramePlan` 差分再描画／canvas 自前合成 | hover 注入 API 契約正本・cursor.\* スタイル塗り＋文字色切替・矩形反転縮退・ダーティ矩形限定 | Missing＋Unknown |
| **R5** ライフサイクル原子無効化 | `apply_cue`→`request_clear` 配線／viewbox `request_clear` | 選択肢 resident＋hit 幾何の同時無効化・hover クリアとの整合 | Missing |
| **R6** M1 範囲＋縮退境界 | 既存の型/語彙シーム規律（Image/Surface/TextEffects 予約） | marker.\*/`\_a`/`\__q`/`\![*]`/cursor.\* 画像 の型・語彙シーム保持 | Constraint |
| **R7** 決定論 E2E＋テスト網羅 | readback 檻／`DrawStats`／FixedMetrics 純関数檻／WarnCounter ログ檻 | 注入 hover 檻・ハイライト pixel on/off 対・em/lh 換算＋hit 幾何の純関数全網羅・test-local fixture | Missing |
| **R8** 実機サインオフ | 実 emo2 boot 経路／DPI 追従／絶対パス起動規律 | 実 DPI≠96 で選択肢＋注入 hover の人間目視 | Constraint |
| **R9** 非退行（additive） | `cargo test --workspace`／既存住人種・cue ワイヤ不変／emo-present 無改変 | 新 cue variant 新設禁止・emo-present 本体無改変の順守 | Constraint |

**Unknown（ukadoc/SSP 実観察で確定）**: cursor.\* スタイルの具体マップ・クリック領域幅・em/lh 基準・負値/省略縮退（下記 Research Needed）。

## 実装アプローチの選択肢

### Option A: 既存住人・既存モジュールを拡張（extend）

- **選択肢 resident**: `ResidentContent` へ `Choice`/`ChoiceRow` variant を additive 追加（`#[non_exhaustive]` 済で非破壊）。中身は GlyphRun＋`{ id, hit_rect }` メタ。描画は既存 GlyphRun 経路を再利用。
- **`\_l` 消費**: `layout.rs` にカーソルオフセット入力を追加（`inline_start`/`block_start` の初期化へ em/lh 換算値を加算）。換算関数は純関数として region/新モジュールへ。
- **hover/ハイライト**: `ScrollPlanner` へ hover 状態フィールド＋ハイライトダーティ導出を追加、`ViewboxExecutor::render` にハイライト塗り＋文字色切替を追加。
- **ライフサイクル**: `apply_cue` の Clear/ClearAll 経路へ選択肢 resident＋hit 幾何の消去を相乗り。
- **トレードオフ**: ✅ 新規ファイル最小・既存の差分再描画/キャッシュ/檻を最大流用。❌ `layout.rs`（2331 行）・`viewbox.rs`（2248 行）・`draw.rs`（2178 行）・`viewbox_draw.rs` が既に大きく、選択肢＋hover の混入で単一責務が薄まるリスク。hover はテキスト reveal と直交する状態ゆえ ScrollPlanner へ載せると凝集度低下。

### Option B: 純粋モジュール＋COM モジュールを新設（new）

- **`choice.rs`（純粋層新設）**: 選択肢 resident 導出・行ヒットジオメトリ（行矩形→id）・`em/lh` 換算・hover 状態モデル・ハイライトダーティ導出を集約。`LineRect`/`PositionedGlyph`（R9.4）を入力に取り、GPU 不要で全網羅可能な純関数群。
- **`choice_draw.rs`（COM 層新設）** or `viewbox_draw` への薄い追加: ハイライト矩形塗り＋文字色切替の描画実行のみ。
- **契約 API**: hit-geometry 照会・hover 注入は `TextLayerRuntime`（結線層）へ additive アクセサ（`draw_stats`/`surface` と同型）。
- **トレードオフ**: ✅ 選択肢・hover の責務が独立ファイルに閉じ、テスト（純関数全網羅・R7.5）と設計判断（cursor.\* マップ差替シーム）が局所化。emo-text の層規律・命名（単一トークン）に整合。❌ layout カーソル注入・canvas 写像・viewbox ダーティ導出との結合点で新旧モジュール間の受け渡し型が増える。

### Option C: ハイブリッド（推奨候補・情報提供）

- **純粋な導出は新設 `choice.rs` へ集約**（Option B）＝行ヒットジオメトリ・em/lh 換算・hover モデル・ハイライトダーティを純関数化（R3.4/R7.5 の GPU 不要全網羅要件と最も整合）。
- **既存の連続経路への差し込みは最小拡張**（Option A）＝`ResidentContent` additive variant・`layout` のカーソルオフセット入力・`ViewboxExecutor` のハイライト描画一行追加・`apply_cue` の Clear 相乗り。
- **cursor.\* スタイル解決は差替シーム**として `choice.rs` に閉じ、M1 は fixture 実導出形（square 塗り＋文字色切替）／未指定は矩形反転縮退、他正典形は語彙保持で非アクティブ縮退（memory「defer-canon-with-full-vocabulary」規律）。
- **トレードオフ**: ✅ 純粋層の決定論檻・差替シーム・既存資産流用を両取り。❌ 分割線（何を choice.rs、何を既存へ）の設計判断が要る＝設計フェーズ冒頭で確定すべき。

## 工数・リスク

| 項目 | 工数 | リスク | 根拠 |
|---|---|---|---|
| 選択肢 resident＋行ヒット幾何（純関数） | S–M | Low | R9.4 再利用シーム予約済・`from_layout` パターン踏襲 |
| `\_l` em/lh 換算＋レイアウトカーソル | M | Medium | newline-defer/折返し不変条件との整合・ukadoc 単位基準確定待ち |
| hover 注入＋ハイライト描画（差分再描画） | M | Medium | ScrollPlanner 差分導出への追加・pixel 檻・cursor.\* マップ Unknown |
| ライフサイクル原子無効化 | S | Low | 既存 request_clear/ClearAll 配線の相乗り |
| 決定論 E2E＋純関数全網羅＋実機サインオフ | M | Medium | 既存 readback/DrawStats/WarnCounter 檻を流用・実機は DPI≠96 目視 |
| **合計（本 spec 全体）** | **L（1–2 週）** | **Medium** | additive だが 5 欠落＋視覚正典確定＋実機サインオフの複合 |

## Research Needed（設計フェーズ冒頭で ukadoc MCP＋SSP 実観察により確定）

1. **cursor.\* スタイルの具体マップ**: `descript_balloon` の cursor.\* 全キー（style／brush.color／pen.color／font.color／blendmethod）の描画意味。fixture `emo2-kakukaku/descript.txt`（square・brush 105,25,25・font 白・blendmethod none）と突合。「cursor,ファイル名」＝マウスカーソル画像は**別物**（M1 外）。
2. **クリック領域の幅**: 選択肢行のヒット矩形が行全幅か文字幅か（SSP de-facto 実観察で 1 つ確定・pixel 檻の期待値へ固定）。
3. **`\_l[x,y]` の em/lh 単位定義**: x=em がフォント高基準か、y=lh が行高基準か（`line_pitch=ceil(font.height×1.25)` との対応）。負値・省略時の縮退挙動。
4. **`\q` 表示仕様**: `\q` は改行しない（自動改行は `\__q` の領分）・fixture も `\n` 手動区切り。アンカー `\_a`（emo2 未使用・M1 外）。
5. **矩形反転縮退の具体仕様**: cursor.\* 未指定バルーン向けの反転描画（色反転の方式）を 1 つ確定（scope doc §4 の代替を pixel 檻化）。

## 設計判断項目（要件ディスカッションへ供給）

1. **選択肢 resident の表現**: `ResidentContent` へ新 variant（`Choice`）を additive 追加するか、既存 GlyphRun 住人へ hit メタ（id・行矩形）を並置するレジストリ方式にするか。前者は canvas 写像・viewbox 指紋（`line_fingerprint`）・draw の match アーム全てに波及、後者は住人種を変えず脇に持つ。R8.4/R9.5 の非退行制約下でどちらが additive か。
2. **モジュール分割線（Option A/B/C）**: 純粋な導出（行ヒット幾何・em/lh 換算・hover モデル・ハイライトダーティ）を新設 `choice.rs` へ集約するか、既存 layout/canvas/viewbox へ内挿するか。R3.4/R7.5 の「GPU 不要で純関数全網羅」を満たす分割。
3. **`\_l` カーソルの layout 注入方式**: `LayoutEngine::layout` にカーソルオフセット引数を追加するか、選択肢配置だけ別経路（選択肢専用レイアウト）にするか。newline-defer（`pending: Option<f32>`）・budoux 折返し不変条件との整合をどう保つか。
4. **hover 状態の載せ場所**: hover 注入状態を `ScrollPlanner`（差分導出の主体）へ持たせるか、`TextLayerRuntime`/新 choice 状態へ分離するか。ハイライト変化をダーティ矩形へ乗せて全域再描画へ退行させない導出（R4.4）の設計。
5. **行ヒットジオメトリ照会 API の形**: 下流（choice-interact）が読む契約の型（`Vec<(LineRect, ChoiceId)>` 相当か）と、座標系（image px validrect-local か物理 px か・`scroll_state().committed` 反映後か）。emo-present `TextSlotView`/`hit_region` パターンとの整合。
6. **「選択肢表示中」照会状態の所在**: render 層が独自に「選択肢 resident 集合が存在」で表すか、`CuePlayer::WaitingForChoice` を参照するか（本 spec は照会のみ・バリア解決は下流）。
7. **cursor.\* スタイル解決の差替シーム**: M1 実導出形（fixture 指定 square 塗り＋文字色切替）と未指定縮退（矩形反転）を 1 機構でどう表し、未確定正典形（pen.color 等サブキー）を語彙保持で非アクティブ縮退させるか（memory「defer-canon-with-full-vocabulary」の 4 点セット）。
   - **要件ディスカッション #2 裁定**: 矩形反転縮退は **M1 実導出対象**（R4.3／R6.1 に確定・R6.5 の縮退リストから除外）。反転方式の具体（色計算）は Research Needed #5 のとおり設計フェーズで 1 つ確定し pixel 檻化する。
   - **将来アイデア（開発者発案・M1 外・スコープ外メモ）**: hover 行の 1.2 倍拡大表示のような areka 独自ハイライト形。cursor.\* スタイル解決の差替シームは、正典スタイル群に加えて後日こうした非正典スタイルを差し込める形（スタイル enum／trait の開放性）を意識して切ること。
8. **原子的無効化の単位**: `Clear`/`ClearAll`/新 talk での選択肢 resident＋hit 幾何＋hover クリアの同時消去を、既存 `request_clear`（executor）＋`state.apply_cue`（Clear）の 2 経路にどう相乗りさせ、片方だけ古い状態を作らないことを構造保証するか（R5.2）。
9. **M1 縮退境界の型/語彙シーム**: marker.\*／`\_a`／`\__q`／`\![*]`／cursor.\* 画像キーを「型/語彙シームとして保持・実導出せず」を、既存の `#[non_exhaustive]`＋予約名定数パターン（`ImageSeam`/`SurfaceSeam`/`TextEffects`/`RESERVED_EFFECT_*`）に倣ってどう表すか。
10. **実機 hover 注入導線の具体形**（要件ディスカッション #1 裁定＝実機サインオフは「見える」＋「注入 hover で光る」の両方必達・R8.6 新設）: 実ポインタ非依存・本番既定無効の有界なデバッグ導線をどう実現するか——AREKA_ 名前空間の env ゲート駆動（[[areka-runtime-env-naming]]・[[areka-real-machine-signoff-bounded-auto-exit]] の bounded auto-exit 流儀と併走）か、actor への注入メッセージ（`UiSender` 規約）か。本番描画経路・決定論資産を汚さない additive 形の選定。

---

# 設計フェーズ Discovery＋Design Decisions（2026-07-23 追記・design.md の根拠正本）

> 本追記が上記ギャップ分析の「Research Needed」5 項目と「設計判断項目」10 項目を確定させる。
> 手段: ukadoc MCP（正典）＋settled main（W2 mayuna マージ後）の実コード再突合。

## Discovery スコープ

- **分類**: Extension（既存 emo-text 縦経路への additive 増分）→ light discovery＋正典確定（ukadoc）。
- **行アンカー再突合（2026-07-23 実測・settled main）**: emo-text Choice アーム＝`state.rs:234-241`（warn `:240`）・Cursor アーム＝`state.rs:243-251`（warn `:249`）・`choice_warned`/`cursor_warned` once-guard＝`state.rs:167-171`。`actor.rs` の no-op 群＝`:249-257`。dola `CueCommand::Choice{id,text,references}`＝`command.rs:143-148`・`Cursor{x,y}`＝`:179`。CuePlayer の Choice 二重真実源（配送列 FIFO＋`pending_choices` push）＝`runtime.rs:196-227`・`WaitingForChoice` 遷移＝`:242-246`（**バリアは sink へ配送されない内部状態**——表示層は cue 列から WaitForChoice を観測できない）。`PositionedLine`/`LineRect`/`PositionedGlyph`＝`layout.rs:101-132`・遅延改行 `pending: Option<f32>`＝`:230-232`・ゲート順序①〜④＝`:243-249`。`ResidentContent`（`#[non_exhaustive]`）＝`canvas.rs:171-180`。`line_fingerprint`/`CommittedLine`＝`viewbox.rs:578-598`/`:388-396`（**text＋block_pos＋extent のみ＝ハイライト状態は指紋外**）。`ViewboxExecutor::render` の住人 match＝`viewbox_draw.rs:275-289`・`resident_rect`＝`viewbox.rs:605-618`。`TextLayerRuntime` アクセサ群（`surface`/`draw_stats` の additive パターン）＝`actor.rs:278-294`。emo2 結線＝`crates/areka/src/emo2_boot/frame.rs`（`register_actor_view` `:486`・text phase `present_frame` `:695`）。

## Research Log（Research Needed 5 項目の確定）

### RN-1: cursor.\* スタイルの具体マップ（確定）

- **Sources**: ukadoc `descript_balloon` cursor.\* 全キー（`cursor.style`／`cursor.brush.color.{r,g,b}`／`cursor.pen.color.{r,g,b}`／`cursor.font.color.{r,g,b}`／`cursor.font.shadowcolor.\*`／`cursor.font.shadowstyle`／`cursor.blendmethod`）。
- **Findings**:
  - `cursor.style,形状`: 選択肢マーカーの形状。`square`＝矩形塗り／`underline`＝下線／`square+underline`＝両方／`none`＝無し。**既定 square**。
  - `cursor.brush.color.{r,g,b}`: マーカー矩形**内**の色（0–255・既定 0）。
  - `cursor.pen.color.{r,g,b}`: 矩形**枠**および下線の色（既定＝`cursor.font.color`）。
  - `cursor.font.color.{r,g,b}`: hover 中の文字色（既定 0）。「ラスタオペレーションコマンドが無い場合使用」＝`blendmethod` が `none` のとき有効。
  - `cursor.blendmethod,コマンド`: ROP2 ラスタオペレーション。`none`＝無し（既定）・`notmaskpen`・`mergepennot`・SSP のみ全 SetROP2 オペレータ。
  - fixture `emo2-kakukaku/descript.txt:43-53` 実指定: `blendmethod,none`・`style,square`・`brush.color` (105,25,25)・`pen.color` (65,0,0)・`font.color` (255,255,255)＝**M1 実導出形は「brush.color の矩形塗り＋font.color への文字色切替（blendmethod=none）」**。
  - `cursor,ファイル名`（マウスカーソル画像）は別キー・別物（M1 外）で確認。
- **Implications**: M1 アクティブ形＝`square`（塗り＋文字色切替）と `none`（マーカー無し＝自明実装）。`underline`／`square+underline`／ROP 系 blendmethod は語彙保持＋warn-once 縮退。`pen.color`（枠色）は語彙保持（M1 の square 塗りは枠なし・fixture の見た目は brush 塗りが支配）・shadow 系も語彙保持のみ。

### RN-2: クリック領域の幅（確定＝文字幅）

- **Sources**: ukadoc `\q[タイトル,ID]` 記述例「`さくらスクリプトは好き？\q[好き,Like]\q[嫌い,Hate]。`」——複数 `\q` が**同一行に並置**できる。
- **Findings**: `\q` はインラインの選択スパンであり、同一行に複数共存できる以上、クリック領域は**行全幅ではなく選択肢テキストのグリフ範囲（文字幅）**でなければ正典と矛盾する。
- **Implications**: ヒット矩形＝「当該選択肢のグリフ列の行内範囲（先頭グリフ位置〜最終グリフ位置＋送り幅）× 行矩形のブロック軸範囲（font_height）」。emo2 メニュー（1 行 1 選択肢）でも自然にこの規則の特例になる。ハイライト矩形＝ヒット矩形と同一（描画とヒットの座標整合・R3.3 を単一導出で構造保証）。

### RN-3: `\_l[x,y]` の単位定義（確定）

- **Sources**: ukadoc `\_l[x,y]` 全文。
- **Findings**:
  - 裸数値＝**バルーンの文字描画範囲左上からのピクセル単位座標**（絶対）。
  - 省略＝当該軸は移動しない（両軸省略＝無効果）。
  - `XXem`＝文字高さ基準（1em＝タグ時点の文字高さ・小数可）。
  - `XXlh`＝行高さ基準（**1lh＝1em＋行間**・小数可）。
  - `XX%`＝文字高さ基準（100%＝文字高さ）。
  - `@XX`＝現在描画位置からの相対（負値＝左/上・em/% と共存可）。
  - `\_l` 実行直後は行揃えが左揃えへリセット（areka M1 は行揃え未実装＝該当挙動なし）。
  - `\c[line]` 仕様より `\_l` は「行」の区切り（意図的改行と同格）＝現在行を閉じて新位置から始める。
- **Implications**: areka 対応＝`em → ResolvedFont::height`（image px）・`lh → GlyphMetrics::line_pitch(font_height)`（`ceil(h×1.25)`＝em＋行間、正典と一致）・裸数値＝image px 恒等・原点＝validrect 左上（`TextRegion::left()/top()`＝文字描画範囲左上）。換算は image px で完結し物理化は既存 `×k` 一点適用に乗る（DPI 一貫・R2.2）。M1 実装＝絶対 px/em/lh＋省略。`%`・`@`（相対）・負値絶対＝語彙保持＋warn-once 縮退（状態不変スキップ・R2.4/R6.5——負値絶対は ukadoc に定義なし＝未確定形）。

### RN-4: `\q` 表示仕様（確定）

- **Sources**: ukadoc `\q[タイトル,ID]`・`\__q[ID,...]`。
- **Findings**: `\q` は自動改行しない（自動改行は `\__q` の領分と明記）。fixture も `\n` 手動区切り。選択後イベントは OnChoiceSelect(Ex)（下流 choice-select-events の領分）。
- **Implications**: 表示層は選択肢テキストへ明示改行を挿入しない（改行は台本の `\n`＝NewLine cue が担う）。選択肢グリフが折返し閾値を超えた場合は既存折返しに従う（M1 縮退・emo2 メニュー実測範囲では発火しない）。

### RN-5: 矩形反転縮退の具体仕様（確定・要件ディスカッション #2 裁定の実装形）

- **Context**: cursor.\* 未指定バルーン向けのハイライト（R4.3／R6.1・M1 実導出対象）。emo-text の文字面は**透明サーフェス上の自前合成**（下地バルーン画像は emo-present 側レイヤ）ゆえ、下地ピクセルを読んで反転する古典 ROP は層構造上不可能。自層内で完結する決定論的な「反転」を 1 つ確定する。
- **確定仕様**: hover 中の選択肢セグメントについて、(a) セグメント矩形をバルーン既定文字色 `font.color` で塗り、(b) セグメント内の文字色を `(255−r, 255−g, 255−b)`（各成分反転・α 不変）へ切り替える。既定の黒文字なら「黒矩形＋白文字」＝古典反転マーカーと同じ見た目。同一入力→同一ピクセル（pixel 檻可能）。
- **Implications**: cursor.\* 指定形（square 塗り）と未指定形（反転）は「矩形塗り色＋文字色」の 2 パラメータで統一表現できる→ 差替シーム（ResolvedChoiceStyle）は単一の描画実行に写像できる。

## Architecture Pattern Evaluation（設計判断 #1/#2 の裁定根拠）

| Option | 概要 | 強み | リスク | 裁定 |
|---|---|---|---|---|
| A: 既存モジュール内挿 | layout/viewbox/canvas へ選択肢・hover を直接追加 | 新規ファイル最小 | 2000 行級ファイルの責務混濁・hover が ScrollPlanner の凝集度を下げる | ✗ |
| B: 全面新設 | choice.rs＋choice_draw.rs 新設 | 責務独立 | 既存差分再描画・キャッシュと二重機構化 | ✗ |
| **C: ハイブリッド（採用）** | 純粋導出は新設 `choice.rs` へ集約・連続経路への差し込みは最小拡張 | 純関数全網羅（R3.4/R7.5）と既存資産流用の両取り | 分割線の規律が要る（下記 Design Decisions で確定） | ✅ |

## Design Decisions（ギャップ分析「設計判断項目」10 項目の確定）

### DD-1: 選択肢 resident の表現＝`ResidentContent::Choice` additive variant（新 variant 案を採用）

- **Alternatives**: (a) 新 variant／(b) GlyphRun 住人＋脇レジストリ並置。
- **Selected**: (a)。`ResidentContent`（`#[non_exhaustive]`・canvas.rs:171）へ `Choice(ChoiceLineContent)` を追加。`ChoiceLineContent`＝グリフ行（`GlyphRunContent` 同形）＋選択肢セグメントメタ（ordinal・行内範囲）＋hover 印。
- **Rationale**: (i) 定義 crate 内は non_exhaustive でも網羅 match が強制される＝全 match 箇所（`line_fingerprint`・`resident_rect`・`viewbox_draw::render`・`draw.rs` oracle）の再検討をコンパイラが強制（no-catch-all 規律）。(ii) **hover 変化を行指紋（`CommittedLine`）差分に乗せられる**＝ScrollPlanner のダーティ導出アルゴリズム無改変で R4.4（差分再描画）が成立する（DD-4）。(b) は指紋外の状態となり差分機構の別口改造が要る。
- **Trade-offs**: match アーム追加の波及はあるが、いずれも「GlyphRun と同じ扱い＋ハイライト」の薄い分岐。既存 GlyphRun/Image/Surface の解決・描画は無変更（R9.5）。

### DD-2: モジュール分割線＝Option C（純粋導出は新設 `choice.rs`・差し込みは最小拡張・**循環禁止の DAG 配置**）

- **`choice.rs`（純粋層・新設・DAG 最下流）が所有**: 選択肢スパン→行セグメント注釈・行ヒットジオメトリ導出・canvas 装飾（GlyphRun→Choice 住人への写像＋style→paint 正規化焼込）・ハイライトスタイル解決 `ResolvedChoiceStyle`（差替シーム）・窓物理座標への写像式。全て windows 非依存の純関数（lib.rs 構造檻へ登録・R3.4/R7.5）。
- **循環回避の配置裁定**（設計レビューゲート修理 #1）: `CursorCoord`/`CursorUnit`/`parse_cursor_coord`/`ChoiceSpan` は **state.rs**（cue 消費層＝`TextItem::CursorMove`・`ActorTextState.choices` の同居地）、`cursor_to_image_px` 換算は **layout.rs**（レイアウトカーソル意味論・`GlyphMetrics` の在処）、`ChoiceLineContent`/`ChoiceRowSegment`/`HighlightPaint` は **canvas.rs**（住人モデルの同居地・純データ）へ置く。これにより純粋層内は `layout→state`・`canvas→layout/region`・`choice→state/layout/canvas/region` の一方向 DAG となり、`choice` を import するのは結線層 `actor` のみ（viewbox/COM 層は住人内の解決済み純データだけを読む）。
- **最小拡張**: state.rs（Choice/Cursor アームの消費化＋上記語彙型）・layout.rs（pending_cursor 遅延実体化＋換算）・canvas.rs（variant＋純データ型）・viewbox.rs（指紋・矩形アーム）・viewbox_draw.rs（ハイライト描画＋`scroll_state()` 読み口）・actor.rs（契約 API）。

### DD-3: `\_l` の layout 注入＝`TextItem::CursorMove`＋pending-cursor 遅延実体化

- **Alternatives**: (a) `LayoutEngine::layout` へカーソルオフセット引数追加／(b) 選択肢専用の別経路レイアウト／(c) items 列への `CursorMove` アイテム追加＋遅延実体化。
- **Selected**: (c)。`\_l` は items 流（Text/NewLine/Choice の到着順）の**中**に現れるため、順序を保存できるのは items 列上の表現だけ（(a) は単一オフセットしか運べず `\q[..]\n\q[..]\_l[..]\q[..]` の途中移動を表現できない）。実体化は既存 newline-defer と同型の遅延規則: `CursorMove` 到着時は保留（`pending_cursor`）のみ・次の可視グリフ配置直前に「現在行確定→保留改行 Σratio 適用→カーソル指定軸の上書き」の順でフラッシュ・後続可視グリフの無い末尾 `CursorMove` は蒸発。newline-defer の不変条件（保留のみで行を開かない・空行を出さない・ビューボックス不変）と完全整合（R2.5）。
- **`\_l` の行区切り性**: フラッシュ時に現在行が非空なら確定する（`\c[line]` 正典の「`\_l` は行の区切り」に一致）。

### DD-4: hover の載せ場所＝`TextLayerRuntime` 保持・ダーティは行指紋差分に乗せる（ScrollPlanner 無改変）

- **Selected**: hover 注入状態（`Option<usize>`＝選択肢 ordinal）は `TextLayerRuntime` の per-actor マップが保持（reveal と直交する注入状態を純粋状態機械・ScrollPlanner のどちらにも混ぜない）。毎フレームの canvas 装飾（`choice.rs`）が hover を Choice 住人の `hovered` 印として焼き込み、`line_fingerprint` が Choice アームで hover 印を指紋に含める（`CommittedLine` へ crate 内 additive フィールド）→ hover 変化＝当該行だけ指紋差分＝既存 `derive_dirty` が当該行矩形のみをダーティ化。**旧 hover 行と新 hover 行の双方が「自行の印の変化」で独立にダーティ化される**ため、切替・解除とも差分再描画で完結し全域再描画へ退行しない（R4.4）。

### DD-5: 行ヒットジオメトリ照会 API＝`TextLayerRuntime::choice_hit_rows`（バルーン窓物理 px・提示フレーム同期スナップショット）

- **型**: `ChoiceHitRow { ordinal: usize, id: String, label: String, references: Vec<String>, rect: HitRectPx }`（`Send` 所有データ）。`rect` は**バルーン窓 client 座標系の物理 px**（f32 矩形）。
- **座標写像（正本式）**: canvas-local（validrect-local image px）の行セグメント矩形 `(inline, block)` に対し、行内軸＝`(region_inline_origin + inline) × k`・ブロック軸＝`(region_block_origin + block) × k + committed`（`committed`＝`ScrollPlanner::scroll_state().committed`＝面に反映済みスクロール・viewbox.rs R9.3 契約点の消費）。surface の窓内 offset（`validrect 原点 × k`）は region 原点項が担う。
- **鮮度契約**: スナップショットは**最後に提示（present）したフレームの導出**（表示と同一の layout・同一の可視状態から単一導出）。cue 適用〜次フレーム提示の間は表示・ヒットが揃って 1 フレーム前＝「片方だけ古い」状態は構造的に生じない（R3.3/R5.2）。下流（choice-interact）は必ずこの口から読む（`pending_choices()` や自前レイアウト再現の禁止）。
- **付帯**: `choice_active(actor) -> bool`（DD-6）・hover 注入（DD-4）と同じ actor.rs の additive アクセサ群（`surface`/`draw_stats` と同型）。

### DD-6: 「選択肢表示中」照会＝表示層自身の選択肢集合で表す

- **根拠**: `WaitForChoice` バリアは CuePlayer 内部状態であり sink へ配送されない（runtime.rs 実測）＝表示層は cue 列からバリアを観測できない。よって表示層の「選択肢表示中」（R1.3）は**保持する選択肢スパン集合が非空**で表す（`choice_active`）。バリア自体の真実源は従来どおり供給側 `CuePlayerState::WaitingForChoice`（照会は下流の領分・本仕様はバリアを解決しない）。

### DD-7: cursor.\* スタイル差替シーム＝`ResolvedChoiceStyle`（開放 enum・「塗り色＋文字色」正規形）

- **形**: `#[non_exhaustive] enum ResolvedChoiceStyle { SquareFill { fill: (u8,u8,u8), text: (u8,u8,u8) }, Invert, NoMarker }`＋解決関数 `resolve(cursor_model, font)`。
  - cursor.\* 指定あり＋`style=square`（既定含む）＋`blendmethod=none`（既定含む）→ `SquareFill { fill: brush.color, text: font.color }`（fixture 実導出形・R4.2）。
  - cursor.\* 指定なし → `Invert`（RN-5 の確定仕様＝塗り＝既定 font.color・文字＝各成分 255−c・R4.3/R6.1）。
  - `style=none` → `NoMarker`（正典・自明実装）。
  - `style=underline|square+underline` → warn-once＋`SquareFill` へ縮退（語彙は parser モデルに保持・R6.5）。ROP 系 `blendmethod` → warn-once＋`none` 扱い。`pen.color`／shadow 系 → 語彙保持のみ（M1 の描画は参照しない）。
- **将来シーム**（開発者発案メモの反映）: `#[non_exhaustive]` ゆえ非正典スタイル（例: hover 行 1.2 倍拡大）を variant 追加で差し込める。描画実行は「スタイル→(塗り色, 文字色) 正規形」の一点写像に集約し、将来 variant はこの写像の追加アームで済む。
- **語彙シームの充足方式（設計 synthesis の簡素化裁定）**: marker.\*／`\_a`／`\__q`／`\![*]`／cursor 画像キーは**既存シームが既に保持**している——balloon KV 寛容パースの passthrough（未知キー非落失）・`\![*]`＝`CueCommand::Custom` 汎用キャリア・`\__q`/`\_a`＝parser 未発行（dic 不使用）。本仕様で投機的な空型を新設しない（R6.2 は既存シームの明示で満たす）。

### DD-8: 原子的無効化の単位＝「単一導出＋提示フレーム同期」の構造保証

- 選択肢スパンは `ActorTextState` に同居し `Clear`/`ClearAll` で items と**同時に**初期化される（新規の消去経路を作らない——既存 `state.apply_cue` の全消去に相乗り・R5.1/R5.2）。新 talk は talk 冒頭 ClearAll（既存規約）で同経路。
- 表示（canvas）とヒット（スナップショット）は present_frame の**同一 layout 導出**から同時更新（DD-5）＝片側だけ古い状態が構造的に無い。
- hover は runtime の `apply_cue` の `Clear`/`ClearAll` アーム（既存 `request_clear` 相乗り点）で当該 actor（ClearAll は全 actor）につき `None` へリセット（R5.4——新 talk の新選択肢へ stale ordinal が誤ハイライトする経路を閉塞）。

### DD-9: reveal との関係＝選択肢グリフも配送 duration 由来の typewriter に従う

- Choice cue のテキストは Text cue と同じ時刻式（`interval = duration / glyph_count`・duration=0 は即時全可視）で items へ追記する。選択肢だけの特例ペースを発明しない（服従＝再生時間の単一真実源・既存 R7 系規律の踏襲）。部分リビール中のヒット矩形は配置済みグリフ範囲（決定論）。

### DD-10: 実機 hover 注入導線＝emo2_boot 結線層の env ゲート駆動（ライブラリ無改変）

- **Selected**: `AREKA_CHOICE_HOVER_INJECT` env（AREKA_ 名前空間規約）。未設定/空＝**無効（本番既定）**。`cycle`（既定周期 700ms）または `cycle:<ms>` ＝ text phase（frame clock 駆動）で `choice_active` な actor の選択肢 ordinal を周期巡回で `inject_choice_hover` する（無し→0→1→…→無し→…）。実ポインタ非依存・frame clock 由来の決定論駆動・`AREKA_APP_SMOKE_EXIT_MS` の bounded auto-exit と併走（[[areka-real-machine-signoff-bounded-auto-exit]]）。
- **Rationale**: ライブラリ（areka-emo-text）は公開注入 API（DD-4）を持つだけで導線を知らない＝本番描画経路・決定論資産は無変更（R8.6）。導線は結線層（`emo2_boot`）の 1 モジュールに閉じ、下流 choice-interact の実ポインタ配線と衝突しない（同じ注入 API の別ドライバ）。

## 座標・単位の確定値（設計の定数表）

| 項目 | 値 | 根拠 |
|---|---|---|
| `em` | `ResolvedFont::height`（image px） | RN-3・「タグを書いた時点での文字高さ」＝actor のフォント解決値 |
| `lh` | `GlyphMetrics::line_pitch(font_height)`＝`ceil(h×1.25)` | RN-3「1lh＝1em＋行間」＝行送りピッチ |
| 裸数値 | image px 恒等 | RN-3「文字描画範囲左上からのピクセル」＝バルーン画像 px≡image px |
| `\_l` 原点 | validrect 左上（`TextRegion::left()/top()`） | RN-3「文字描画範囲」 |
| ヒット矩形 | 選択肢グリフ範囲 × 行 font_height 帯 | RN-2（文字幅）＋layout.rs 行矩形規約 |
| ハイライト矩形 | ヒット矩形と同一 | R3.3 の単一導出保証 |
| 物理化 | image px × k 一点適用＋ブロック軸 `+committed` | region.rs/viewbox.rs 既存契約（R2.2/R9.3） |

## Risks & Mitigations（設計フェーズ更新）

- **cursor.\* 未モデル化**（balloon parser は cursor.\* を意図的非モデル化・validation_tests が distractor 扱い）→ `areka-parsers/balloon` へ additive な `Cursor` サブ構造体（style/brush.color/pen.color/font.color/blendmethod）を追加。既存テスト（font へ巻き込まない檻）は不変緑のまま成立。
- **`SetDrawingEffect` の cache 前提**（行 TextLayout は `LineLayoutStore` でキャッシュ）→ Choice 行の描画毎に「全範囲へ既定 effect リセット→hover セグメント範囲へ適用」の順を正準列とし、キャッシュ再利用と決定論を両立（実装ノートに檻を設ける）。
- **byte 等価 golden との干渉**（`DrawExecutor` oracle は選択肢ハイライトを持たない）→ 既存 golden 群は非選択肢内容のまま不変。選択肢の pixel 檻は `ViewboxExecutor` readback を直接期待値化（oracle 比較は使わない）。oracle の Choice アームは素のグリフ描画（warn なし）に留める。
- **提示前スナップショットの stale 窓**（cue 適用〜次 present の 1 フレーム）→ 契約に明記（DD-5 鮮度契約）。下流 interact 側の解決時再検証（選択確定時の choice_active 確認）は interact の設計事項として申し送る。

## References

- ukadoc `descript_balloon` cursor.\* 各項・`list_sakura_script` `\_l[x,y]`／`\q[タイトル,ID]`／`\__q[ID,...]`／`\c[line,数値]`／`\f[align,寄せる側]`（ukadoc MCP 取得・2026-07-23）。
- fixture: `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/descript.txt:43-53`・`.../ghost/master/dic/menu.pasta:15/33/62`。
- settled main 実コード（本追記「行アンカー再突合」記載の file:line）。
