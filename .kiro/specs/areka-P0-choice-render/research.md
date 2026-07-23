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
8. **原子的無効化の単位**: `Clear`/`ClearAll`/新 talk での選択肢 resident＋hit 幾何＋hover クリアの同時消去を、既存 `request_clear`（executor）＋`state.apply_cue`（Clear）の 2 経路にどう相乗りさせ、片方だけ古い状態を作らないことを構造保証するか（R5.2）。
9. **M1 縮退境界の型/語彙シーム**: marker.\*／`\_a`／`\__q`／`\![*]`／cursor.\* 画像キーを「型/語彙シームとして保持・実導出せず」を、既存の `#[non_exhaustive]`＋予約名定数パターン（`ImageSeam`/`SurfaceSeam`/`TextEffects`/`RESERVED_EFFECT_*`）に倣ってどう表すか。
10. **実機 hover 注入導線の具体形**（要件ディスカッション #1 裁定＝実機サインオフは「見える」＋「注入 hover で光る」の両方必達・R8.6 新設）: 実ポインタ非依存・本番既定無効の有界なデバッグ導線をどう実現するか——AREKA_ 名前空間の env ゲート駆動（[[areka-runtime-env-naming]]・[[areka-real-machine-signoff-bounded-auto-exit]] の bounded auto-exit 流儀と併走）か、actor への注入メッセージ（`UiSender` 規約）か。本番描画経路・決定論資産を汚さない additive 形の選定。
