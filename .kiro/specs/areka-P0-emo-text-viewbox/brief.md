# Brief: areka-P0-emo-text-viewbox

> **種別**: 本坑（main）。⑥ emo トラックの**増分ユニット**（並走安全・依存は emo-text-layer のみ＝M-boot 完成を待たない）。
> **調査日**: 2026-07-09（emo-text-layer からのスクロール実現方式切り出し・開発者裁定「viewbox はあとでよいが brief 含めてロードマップ登録」）／**2026-07-11 再調整**（emo-text-layer ✅ 完了を受けた実シーム突合）。
> **前提依存（順序ゲート）**:
> ```
> _Depends: areka-P0-emo-text-layer ✅（2026-07-11 完了・分離シーム納品済み）
> ```
> **✅ ゲート解消済み＝即着手可**（emo2-boot と並走可能——あちらは areka-emo-text を**消費のみ**〔sink／装着 API／present_frame〕・本ユニットは**描画実行側のみ改変**＝交差面ゼロ・pixel 等価 golden が挙動不変を担保）。

## Problem

M1 の emo-text-layer はスクロールを**全域ビットマップ再描画**（伺か流・validrect サイズの描画面に可視窓を毎回描き直す）で実装する（開発者裁定＝最小実装）。これは SSP バルーン規模では十分だが、①スクロールのたびに全文字を DirectWrite で再描画する（typewriter 毎グリフ更新と重なると無駄が累積）②スクロール演出（滑らか補間・慣性）が構造的に不可能（描画面の離散差し替えしかできない）、という限界を持つ。**viewbox（クリップ視窓＋内容オフセット）方式へ差し替えて再描画レス・演出可能なスクロールにする**。

## Current State

- **wintf のクリップ primitive ✅（`completed/visual-clip`）**: `Visual.clip = ClipShape::Rectangle`（角丸2種も有り）を `clip_sync_system` が Arrangement 由来サイズ・**DPI スケール込み**で WUC `InsetClip` へ写像済み——「viewbox ウィジェット」は存在しないが、**クリップ visual（＝viewport）＋子 visual の translate offset（＝スクロール位置）の合成で viewbox は今日組める**（2026-07-09 実地確認）。
- **emo-text-layer ✅ の分離シーム（2026-07-11 実シンボル納品確認）**:
  - **可視窓決定（純粋・不変で流用）**: `LayoutEngine::visible_window(lines: &[PositionedLine], region: &TextRegion, mode: WritingMode) -> VisibleWindow`（layout.rs:255）＝**唯一のスクロール決定点**。出力 `VisibleWindow { first_visible_line: usize, block_offset: f32 }`（layout.rs:127）——doc コメント（layout.rs:124）が「emo-text-viewbox はこの出力を『クリップ視窓＋内容オフセット』へ写像して描画実行だけを差し替える」と本ユニットを名指しで明記。
  - **内容キャンバス（不変で流用）**: `ContentCanvas`（canvas.rs:200）は全行を `residents: Vec<Resident>` として保持し**可視窓を適用しない**（canvas.rs:194「可視窓の適用は COM 層 draw の領分」）。住人 index＝layout 行 index 1:1。将来住人 `ResidentContent::{GlyphRun, Image(ImageSeam), Surface(SurfaceSeam)}` が**固定層（`\_b --option=fixed`）の型シーム**として既設。
  - **描画実行（本ユニットが差し替える側）**: `DrawExecutor::render(&self, canvas, window: &VisibleWindow, font, mode, contract, surface)`（draw.rs:526 付近）が `first_visible_line`（skip）と `block_offset`（dx/dy 加算）を消費する**全域再描画**——viewbox はこの render 側だけを「content 追記描画＋offset 更新」へ置換。呼び順の結線点は actor.rs:448-457（`visible_window`→`ContentCanvas::from_layout`→`render`）。
  - **golden 資産 ✅**: `draw_readback_test.rs`（readback 3述語）・`scale_invariance_test.rs`（複数スケール）が既設＝pixel 等価の比較基準に流用可。
- **スクロール軸は writing_mode で回る**: 横書き＝縦オフセット・縦書き（`vertical_rl`）＝横オフセット（行が左へ流れる）——軸切替の正準表は emo-text-layer が構造化済み（`WritingMode::resolve`＝writing.rs:63・純粋層）。

## Desired Outcome

テキスト内容を**一度だけ**描いた content visual を、クリップされた viewport の中でオフセット移動させるスクロール。スクロール時に文字の再描画が発生せず、オフセット補間による滑らか演出の土台が立つ。

**✔ 観測（単一 pass/fail）**: emo-text-layer の観測 example のスクロール経路を viewbox 方式へ差し替え、(a) **表示結果が再描画方式と pixel 等価**（オフスクリーン readback golden・横書き/縦書き両方）(b) **スクロール時に content の再描画が発生しない**（描画呼出しの決定論的カウント/ログ捕捉で固定）(c) 縦書き＝横オフセット・横書き＝縦オフセットの軸が writing_mode に追随する。

## Approach

1. **viewbox 合成**: viewport visual（validrect サイズ・`ClipShape::Rectangle`）＋ content visual（**バルーン内容キャンバス**＝テキスト全長の描画面・translate offset）。クリップは wintf 実装済み primitive をそのまま使う（wintf 改変なし・組み合わせのみ）。
   - **固定層のシーム（SSP 正典裏付け・2026-07-09）**: `\_b` の **`--option=fixed`＝「スクロールした時に画像を動かさない」**が SSP に存在＝バルーン内部は**スクロール内容層＋固定層の二層**が de-facto。viewbox 構成は自然に対応可能（固定層＝offset を受けない viewport 直下の別 visual）——**M1 相当では実装しない**が、visual 構成に固定層の差し込み点を最初から予約（`\_b` 対応増分が visual 再構成なしで済む形）。
2. **描画実行の差し替え**: emo-text-layer の分離シーム（可視窓決定/描画実行）のうち**描画実行側だけ**を「全域再描画」→「content 追記描画＋offset 更新」へ置換。可視窓決定（純粋層）とスクロール発火規則は**不変**＝状態機械のテスト資産がそのまま生きる。
3. **content 描画面の成長規則**: talk が進むほど content が伸びる——描画面の初期サイズ・伸長（再確保）規則・上限（`Clear` でリセット）を design で確定（無限成長させない）。
4. **滑らか補間のシーム**: M1 相当機能はステップスクロール等価（golden 一致が受け入れ基準）。オフセット補間（滑らか演出・慣性）は**シームのみ**（WUC アニメーション活用は M2 の新能力解禁と歩調を合わせる）。

## Scope

- **In**: viewport/content の2層 visual 合成（ClipShape 流用）／描画実行の差し替え（分離シーム消費）／content 描画面の成長・リセット規則／writing_mode 追随の軸切替／pixel 等価 golden＋再描画レス観測。
- **Out**: スクロール状態機械・可視窓決定・レイアウト（**emo-text-layer** 不変）／滑らか補間・慣性演出の実挙動（**M2**・シームまで）／wintf クリップ機構の改変（流用のみ）／choice-render のクリック範囲（あちらが viewport 座標系を消費する際の契約は design で1判断）。

## Boundary Candidates

- viewbox 合成（visual 構成）／描画実行アダプタ（シーム差し替え点）／成長規則（content 面管理）の三片。

## Out of Boundary

- テキストの意味論・タイミング（emo-text-layer）／バルーン枠（emo-present）。

## Upstream / Downstream

- **Upstream**: `areka-P0-emo-text-layer` ✅（2026-07-11 完了＝分離シーム納品済み）／wintf `visual-clip` ✅（`ClipShape::Rectangle`＋`clip_sync_system`）。
- **Downstream**: M2 スクロール演出（補間・慣性）／`choice-render`（viewport 座標系でのクリック範囲・間接）／`\_b --option=fixed` 固定層の実装増分（固定層差し込み点の消費者）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-emo-text-layer`✅（描画実行シームの差し替え・`DrawExecutor::render` 側のみ・決定/状態機械/レイアウトのテスト資産は不変で生きる）。
- **Adjacent**: `completed/visual-clip`（primitive 提供・不改変）／`areka-P0-choice-render`（M-dialogue・viewport 座標系の消費者候補）／`areka-P0-emo2-boot`（並走候補・areka-emo-text の消費者——本ユニットの改変面〔draw/render〕と非交差・pixel 等価 golden で挙動不変を保証）。

## Constraints

- Rust 2024・tokio 禁止。visual 操作は UI スレッド固定。wintf 改変なし（クリップ primitive は流用のみ・不足が判明したら wintf への増分 issue として申し送り）。
- **pixel 等価が受け入れ基準**（再描画方式の golden と一致）＝方式差し替えで見た目を変えない（純粋等価移行の規律・wuc-migration と同型）。
- **決定論テスト網羅**（記憶 deterministic-test-coverage-mandate）: 再描画レスの証明は描画呼出しカウントの決定論的観測で（目視でない）。
