# Requirements Document

## Project Description (Input)

⑥ emo トラックの**増分ユニット**（並走安全・依存は emo-text-layer のみ＝M-boot 完成を待たない）。emo-text-layer ✅（2026-07-11 完了）はバルーンのスクロールを**全域ビットマップ再描画**（validrect サイズの描画面に可視窓を毎フレーム描き直す・伺か流の最小実装）で実現している。これは①スクロールのたびに可視窓の全文字を DirectWrite で再描画する（typewriter 毎グリフ更新と重なると再描画が累積する）②オフセット補間による滑らかなスクロール演出が構造的に不可能（描画面の離散差し替えしかできない）、という限界を持つ。本ユニットはこれを **viewbox 方式（クリップされた viewport 視窓＋内容 content のオフセット移動）** へ差し替え、テキスト内容を**一度だけ**描いた content を viewport の中でオフセット移動させることで、スクロール時に文字の再描画が発生しない・滑らか演出の土台が立つ形にする。差し替えは emo-text-layer が残した**描画実行の分離シーム**（可視窓決定は純粋層で不変・描画実行だけを差し替える）を消費し、可視窓決定／スクロール状態機械／レイアウトのテスト資産はそのまま生かす。受け入れ基準は**再描画方式の golden と pixel 等価**（見た目を変えない純粋等価移行・wuc-migration と同型）。方針正本は brief.md・roadmap「emo の責務範囲」節・記憶 areka-emo-ui-layer-text-roadmap／areka-emo-own-compositor-atlas。

## Introduction

areka の ⑥ emo トラックにおいて、emo-text-layer ✅ が実装したバルーンテキスト層のスクロール実現方式を**再描画方式から viewbox 方式へ差し替える**増分ユニットである。emo-text-layer は「可視窓の決定（純粋な計算）」と「描画実行」を意図的に分離しており（`LayoutEngine::visible_window` が唯一のスクロール決定点・`ContentCanvas` が全行を可視窓非適用で保持・doc コメントが本ユニットを名指しで移行先として明記）、描画実行だけが `validrect` サイズの描画面へ可視窓を毎フレーム全域再描画している。本ユニットは、この**描画実行側だけ**を「テキスト内容を一度だけ描いた content をクリップされた viewport の中でオフセット移動させる」形へ置換する。

viewbox は wintf の実装済みクリップ primitive（`ClipShape::Rectangle` を `clip_sync_system` が Arrangement 由来サイズ・DPI スケール込みで写像する `completed/visual-clip`）を**流用のみ**で組む（wintf のクリップ機構は改変しない）。viewport visual（validrect サイズのクリップ視窓）＋ content visual（テキスト全長の描画面・平行移動オフセット）の 2 層合成で、スクロール位置は content visual のオフセット更新だけで表現し、文字の再描画を発生させない。スクロール軸は `writing_mode` に追随する（横書き＝縦オフセット・縦書き＝横オフセット）。

本ユニットの成果は emo-text-layer と**交差面ゼロ**で成立する（emo-text-layer の可視窓決定・スクロール状態機械・レイアウト・純粋層は不変で、それらのテスト資産がそのまま生きる）。並走候補の emo2-boot は areka-emo-text を**消費のみ**（sink／装着 API／present_frame）するのに対し、本ユニットは**描画実行側のみを改変**するため両者は非交差であり、pixel 等価 golden が挙動不変を担保する。M1 相当の受け入れはステップスクロール等価（golden 一致）に限り、オフセット補間による滑らか演出・慣性は**シームのみ**（実挙動は M2 の新能力解禁と歩調を合わせる）とする。

## Boundary Context

- **In scope（本ユニットが観測可能に実現する振る舞い）**:
  - viewport visual（validrect サイズ・矩形クリップ視窓）＋ content visual（テキスト全長の描画面・平行移動オフセット）の 2 層 visual 合成。クリップは wintf 実装済み primitive（`ClipShape::Rectangle`）を流用のみで用い、wintf を改変しない。
  - emo-text-layer の描画実行の分離シームのうち**描画実行側だけ**を、全域再描画（validrect サイズ面へ可視窓を毎フレーム描き直す）から「content を一度描き＋オフセット更新」へ差し替える。可視窓決定（純粋層 `visible_window`）・スクロール発火規則・レイアウト・状態機械は不変で消費する。
  - テキスト内容を**一度だけ**描いた content visual を、クリップされた viewport の中でオフセット移動させるスクロール。スクロール（可視窓の移動）時に content の文字再描画が発生しないこと。
  - スクロール以外で content 内容が変化する事象（Text 追記／NewLine／typewriter による可視グリフ増加）に対して、content の増分だけを追記描画し、既に確定した content を再描画しないこと。
  - content 描画面の成長規則（初期サイズ・talk 進行に伴う伸長＝再確保の規則・上限）と、Clear によるリセット規則（content を初期状態へ戻す）。content 描画面を無限成長させない。
  - スクロールのオフセット軸を `writing_mode` に追随させる（横書き＝縦方向オフセット・縦書き＝横方向オフセット・emo-text-layer の軸読み替え正準表と一致）。
  - 差し替え後の表示結果が**再描画方式の golden と pixel 等価**であること（オフスクリーン readback での比較・横書き／縦書き両方・複数スケール）。
  - 固定層（`\_b --option=fixed`＝スクロールした時に画像を動かさない層）の差し込み点を visual 構成に**予約**する（M1 では実挙動を実装しない・後続の `\_b` 対応増分が visual 再構成なしで済む形）。
  - 滑らか補間・慣性のシーム（M1 はステップスクロール等価・オフセット補間は型/構造シームのみ・実挙動は M2）。
  - 上記を単一 pass/fail で観測できる形（emo-text-layer の観測 example のスクロール経路を viewbox 方式へ差し替える）。

- **Out of scope（本ユニットが所有しない）**:
  - スクロール状態機械・可視窓決定（`visible_window`）・文字レイアウト・折返し・行送り・typewriter 進行・writing_mode 2 層マージ解決（すべて **emo-text-layer** の責務・不変で消費する）。
  - オフセット補間による滑らかなスクロール演出・慣性の実挙動（**M2**・本ユニットはシームまで）。
  - wintf のクリップ機構（`ClipShape`／`clip_sync_system`）の改変（流用のみ・不足が判明した場合は wintf への増分 issue として申し送る）。
  - `\_b` 固定層の実装（画像読込・固定層の実描画）。本ユニットは差し込み点の予約のみ。
  - choice-render（M-dialogue）のクリック可能範囲の実導出（本ユニットは viewport 座標系の契約点を残すのみ・実導出は choice-render の責務）。
  - sink の main 結線（`GhostBootOptions.text_sink` への注入・実 talk 経路）・バルーン枠の描画/配置（emo-present）・surface 合成（emo-compose）・バルーン窓の生成/配置（window-placement）。

- **Adjacent expectations（隣接ユニットへの期待・依存）**:
  - **emo-text-layer** ✅ の分離シームを消費する。可視窓決定 `LayoutEngine::visible_window(...) -> VisibleWindow { first_visible_line, block_offset }` を唯一のスクロール決定点として不変で用い、内容キャンバス `ContentCanvas`（全行を住人として保持・可視窓非適用）を content の描画元として不変で用いる。emo-text-layer の呼び順の結線点（`visible_window` → `ContentCanvas::from_layout` → 描画実行）は保ち、置換するのは描画実行の中身のみ。
  - **visual-clip** ✅（`completed/visual-clip`）の `ClipShape::Rectangle`＋`clip_sync_system`（Arrangement 由来サイズ・DPI スケール込みで WUC `InsetClip` へ写像）を改変なしで消費する。
  - emo-text-layer の golden 資産（オフスクリーン readback 述語群・複数スケール検証）を pixel 等価の比較基準として消費する。
  - emo-text-layer の DPI/スケール契約（レイアウト座標＝image px・描画ターゲットに合成スケールを適用）を保ち、viewbox 化で論理/物理の混在を持ち込まない（記憶 areka-window-placement-dpi-coordinate-defect の教訓）。
  - **emo2-boot**（並走候補）は areka-emo-text を消費のみ（sink／装着 API／present_frame）するため本ユニットの改変面（描画実行）と非交差である。pixel 等価 golden が両者の挙動不変を担保する。
  - **choice-render**（M-dialogue・間接）は本ユニットが残す viewport 座標系の契約点（クリック範囲を viewport 座標で消費する際の座標契約）を下流で用いる。
  - **`\_b --option=fixed` 固定層の実装増分**（後続）は本ユニットが予約する固定層の差し込み点を消費する。
  - **M2 スクロール演出**（補間・慣性）は本ユニットが残すオフセット補間のシームを消費する。

## Requirements

### Requirement 1: viewbox 2 層 visual 合成（viewport クリップ＋content オフセット）

**Objective:** As a emo バルーンテキスト層, I want クリップされた viewport 視窓と平行移動する content の 2 層で viewbox を構成すること, so that テキスト内容を一度描いたまま可視窓だけを移動させる土台が立つ

#### Acceptance Criteria

1. The emo テキスト viewbox shall テキスト表示を viewport visual（validrect サイズの矩形クリップ視窓）と content visual（テキスト全長の描画面）の 2 層で構成する。
2. The emo テキスト viewbox shall viewport visual の矩形クリップを wintf 実装済みのクリップ primitive（`ClipShape::Rectangle`）で表現し、wintf のクリップ機構を改変しない（流用のみ）。
3. The emo テキスト viewbox shall スクロール位置を content visual の平行移動オフセットとして表現し、viewport の外へ出た content 部分をクリップで不可視にする。
4. While スクロール位置が変化する, the emo テキスト viewbox shall content visual のオフセットのみを更新し、viewport のクリップ矩形（validrect サイズ）は不変に保つ。
5. The emo テキスト viewbox shall viewport のクリップ矩形を、emo-text-layer の DPI/スケール契約（validrect 寸に合成スケールを適用した物理寸）に一致させ、任意のモニタ DPI で content が validrect からあふれて見えないようにする。

### Requirement 2: 描画実行の差し替え（分離シーム消費・上流不変）

**Objective:** As a emo テキスト viewbox, I want emo-text-layer の描画実行側だけを差し替えること, so that 可視窓決定・状態機械・レイアウトのテスト資産を壊さずスクロール実現方式を移行できる

#### Acceptance Criteria

1. The emo テキスト viewbox shall emo-text-layer の可視窓決定（`LayoutEngine::visible_window` の出力 `VisibleWindow { first_visible_line, block_offset }`）を唯一のスクロール決定点として不変で消費する。
2. The emo テキスト viewbox shall emo-text-layer の内容キャンバス（`ContentCanvas`・全行を住人として保持し可視窓を適用しない）を content の描画元として不変で消費する。
3. The emo テキスト viewbox shall 可視窓の `first_visible_line`／`block_offset` を content visual の平行移動オフセットへ写像し、描画実行（全域再描画）を「content 描画＋オフセット更新」へ置換する。
4. The emo テキスト viewbox shall emo-text-layer のスクロール状態機械・スクロール発火規則（あふれ判定）・折返し／行送り／レイアウト・typewriter 進行・writing_mode 解決を改変せず、それらを不変で消費する。
5. When viewbox 方式へ差し替える, the emo テキスト viewbox shall emo-text-layer の純粋層（可視窓決定・状態機械・レイアウト）に対する既存テスト資産が改変なしに成立し続けるようにする。

### Requirement 3: content の一度描き・スクロール再描画レス

**Objective:** As a emo テキスト viewbox, I want テキスト内容を一度だけ描いてスクロール時に再描画しないこと, so that typewriter 毎グリフ更新と重なる再描画の累積を解消し滑らか演出の土台を作る

#### Acceptance Criteria

1. The emo テキスト viewbox shall テキスト content を content visual へ描画し、スクロール（可視窓の移動）時に既に描画済みの content を再描画しない。
2. When 可視窓のみが移動する（content 内容は不変）, the emo テキスト viewbox shall content visual のオフセット更新だけを行い、文字（グリフ）の描画を発生させない。
3. When content 内容が伸びる（Text 追記／NewLine／typewriter による可視グリフ増加）, the emo テキスト viewbox shall content の増分だけを追記描画し、既に確定した content 部分を再描画しない。
4. The emo テキスト viewbox shall スクロール時に content の再描画が発生しないことを、決定論的な描画呼び出しカウント（DirectWrite レイアウト生成・描画実行回数の観測）またはログ捕捉で検証できる形にする（目視に依存しない・記憶 deterministic-test-coverage-mandate）。
5. While M1 である, the emo テキスト viewbox shall スクロールをステップ（行単位・即時）で行い、フレーム間のオフセット補間を伴わない。

### Requirement 4: content 描画面の成長・上限・リセット

**Objective:** As a emo テキスト viewbox, I want content 描画面の成長と上限とリセットの規則を持つこと, so that talk が長く続いても content 描画面が無限に成長しない

#### Acceptance Criteria

1. The emo テキスト viewbox shall content 描画面の初期サイズと、talk 進行に伴う伸長（再確保）規則を定義し、追記された content が content 描画面へ収まるようにする。
2. When content が現在の content 描画面を超えて伸びる, the emo テキスト viewbox shall content 描画面を伸長（再確保）し、既存 content を失わない。
3. When Clear cue が適用される, the emo テキスト viewbox shall content 描画面を初期状態へリセットし、以後の content を初期サイズから再び積み上げる。
4. The emo テキスト viewbox shall content 描画面のサイズに上限を設け、無限成長させない（上限到達時の扱いは Clear によるリセットを含む・具体値は設計で確定する）。
5. The emo テキスト viewbox shall content 描画面の成長・リセットの前後で、表示結果が再描画方式の golden と pixel 等価であることを保つ。

### Requirement 5: writing_mode 追随のオフセット軸切替

**Objective:** As a emo テキスト viewbox, I want スクロールのオフセット軸を writing_mode に追随させること, so that 縦書きでも正しい方向へスクロールする

#### Acceptance Criteria

1. While `writing_mode` が横書き（`horizontal_tb`）である, the emo テキスト viewbox shall content のオフセットを縦方向（ブロック軸）へ適用し、縦スクロールを実現する。
2. While `writing_mode` が縦書き（`vertical_rl`）である, the emo テキスト viewbox shall content のオフセットを横方向へ適用し、行が左へ流れる横スクロールを実現する。
3. The emo テキスト viewbox shall オフセット軸の切替を emo-text-layer の軸読み替え正準表（可視窓 `block_offset` の符号・方向規約）と一致させ、独自の軸規則を発明しない。
4. The emo テキスト viewbox shall viewport のクリップ矩形（validrect サイズ）を `writing_mode` によらず一定に保ち、オフセット軸のみを切り替える。

### Requirement 6: 再描画方式との pixel 等価（golden 一致）

**Objective:** As a 開発者, I want viewbox 方式の表示が再描画方式の golden と pixel 等価であること, so that スクロール実現方式の差し替えで見た目を一切変えないことを証明できる

#### Acceptance Criteria

1. The emo テキスト viewbox shall 同一の cue 列・同一の注入時刻列に対する表示結果を、emo-text-layer の再描画方式の表示結果とオフスクリーン readback で pixel 等価にする。
2. The emo テキスト viewbox shall pixel 等価をスクロール発火の前後（あふれ前・あふれ後の可視窓移動）で成立させる。
3. The emo テキスト viewbox shall pixel 等価を横書き（`horizontal_tb`）と縦書き（`vertical_rl`）の両方で成立させる。
4. The emo テキスト viewbox shall pixel 等価を複数の合成スケール（非 96 DPI を含む）で成立させ、emo-text-layer のスケール不変検証資産を差し替え後も満たす。
5. When Clear cue が適用される, the emo テキスト viewbox shall Clear 後の表示（全域透明）を再描画方式と pixel 等価にする。

### Requirement 7: 固定層（`\_b --option=fixed`）差し込み点の予約

**Objective:** As a 後続の `\_b` 対応増分, I want viewbox 構成に固定層の差し込み点が予約されていること, so that スクロールに追従しない固定画像層を visual 再構成なしに増設できる

#### Acceptance Criteria

1. The emo テキスト viewbox shall viewport 直下に、content visual のオフセットを受けない固定層 visual の差し込み点を予約する。
2. While M1 である, the emo テキスト viewbox shall 固定層の実挙動（画像読込・固定層の実描画）を実装せず、差し込み点の構造予約に留める。
3. The emo テキスト viewbox shall 固定層を後から増設する際に viewport／content の 2 層構成を再構成せずに済む形で予約する。
4. The emo テキスト viewbox shall 固定層差し込み点の予約が M1 の pixel 等価 golden（固定層なしの表示）に影響しないようにする。

### Requirement 8: 滑らか補間・慣性のシーム

**Objective:** As a M2 スクロール演出, I want オフセット補間・慣性のシームが用意されていること, so that 滑らかなスクロール演出を破壊的変更なしに解禁できる

#### Acceptance Criteria

1. While M1 である, the emo テキスト viewbox shall スクロールをステップスクロール等価（可視窓のオフセットへ即時追従）で行い、その結果が再描画方式の golden と pixel 等価である。
2. The emo テキスト viewbox shall オフセット補間（フレーム間の中間オフセット生成）・慣性を型/構造シームとして保持するに留め、その実挙動を M1 で実装しない。
3. The emo テキスト viewbox shall 補間シームを、content のオフセットが可視窓決定（`block_offset`）と補間過程とで分離できる形で残し、M2 の補間実装が描画実行の再設計を要さないようにする。

### Requirement 9: クロスユニット契約シーム

**Objective:** As a 下流/並走ユニット, I want 本ユニットの差し替えが emo-text-layer・emo2-boot・choice-render を詰ませないこと, so that 各ユニットへの接続が破壊的変更なしに進む

#### Acceptance Criteria

1. The emo テキスト viewbox shall emo-text-layer の可視窓決定・状態機械・レイアウト・純粋層を改変せず、それらの公開契約・テスト資産を不変に保つ。
2. The emo テキスト viewbox shall emo2-boot が消費する経路（sink／装着 API／present_frame）を再定義せず、本ユニットの改変を描画実行側に閉じて emo2-boot と非交差にする。
3. The emo テキスト viewbox shall choice-render（M-dialogue）がクリック可能範囲を消費する際に用いる viewport 座標系の契約点（content オフセットと viewport クリップの座標関係）を構造として残す（クリック範囲の実導出は実装しない）。
4. The emo テキスト viewbox shall wintf のクリップ機構（`ClipShape`／`clip_sync_system`）を改変せず、不足が判明した場合は wintf への増分 issue として申し送る（本ユニットで wintf を改変しない）。
5. The emo テキスト viewbox shall スクロール実現方式の差し替えを additive に行い、emo-present／emo-compose／sakura／balloon-parse の既存契約を再定義しない。

### Requirement 10: 観測用 example（スクロール経路の viewbox 差し替え・単一 pass/fail）

**Objective:** As a 開発者, I want 単一 pass/fail で viewbox スクロールを確認できる観測経路, so that 再描画レス・pixel 等価・軸追随を実機で証明できる

#### Acceptance Criteria

1. The emo テキスト viewbox 観測経路 shall emo-text-layer の観測 example のスクロール経路を viewbox 方式へ差し替え、fixture スクリプト由来の cue 列（Text／NewLine／Clear）を注入時刻駆動で流す（実時間 sleep 不使用・決定論）。
2. When スクロールが発火する, the emo テキスト viewbox 観測経路 shall 表示結果が再描画方式の golden と pixel 等価であることを観測可能にする（横書き／縦書き両方・オフスクリーン readback）。
3. When 可視窓のみが移動する, the emo テキスト viewbox 観測経路 shall content の文字再描画が発生しないことを決定論的な描画呼び出しカウントまたはログ捕捉で観測可能にする。
4. When `writing_mode` が縦書き（`vertical_rl`）である, the emo テキスト viewbox 観測経路 shall スクロールのオフセットが横方向へ切り替わることを観測可能にする。
5. When Clear cue を注入する, the emo テキスト viewbox 観測経路 shall content 描画面がリセットされ表示が全域透明へ戻ることを観測可能にする。
6. The emo テキスト viewbox 観測経路 shall DPI/スケールの正しさを検証する——pixel 等価を非 96 DPI を含む実 DPI/合成スケールで観測可能にし、実 DPI 実行を経ない「テスト緑」を正しさの証明としない（記憶 areka-placement-real-ghost-first）。
