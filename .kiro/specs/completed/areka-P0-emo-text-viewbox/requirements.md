# Requirements Document

## Project Description (Input)

⑥ emo トラックの**増分ユニット**（並走安全・依存は emo-text-layer のみ＝M-boot 完成を待たない）。emo-text-layer ✅（2026-07-11 完了）はバルーンのスクロールを**全域ビットマップ再描画**（validrect サイズの描画面に可視窓を毎フレーム描き直す・伺か流の最小実装）で実現している。これは①スクロールのたびに可視窓の全文字を DirectWrite で再描画する（typewriter 毎グリフ更新と重なると再描画が累積する）②オフセット補間による滑らかなスクロール演出が構造的に不可能（描画面の離散差し替えしかできない）、という限界を持つ。本ユニットはこれを **viewbox 方式（ダーティ矩形スクロール）** へ差し替える。すなわち古典 Win32 の `ScrollDC`／`InvalidateRect`／`WM_PAINT` を写し、**固定 validrect サイズの描画面（重なり安全のためダブルバッファ）の中で、確定済みピクセルを whole-pixel で面内平行移動（blit）し、スクロールで露出した帯＋現在描画中の行＝ダーティ領域だけを D2D 描画コマンド（`DrawTextLayout`）で（再）描画する**。確定 content は「描画面のピクセル＋blit」で保持し二度と再描画しない＝スクロール時に確定文字の再描画が発生しない・滑らか演出の土台（f32 連続スクロール位置のシーム）が立つ。差し替えは emo-text-layer が残した**描画実行の分離シーム**（可視窓決定は純粋層で不変・描画実行だけを差し替える）を消費し、可視窓決定／スクロール状態機械／レイアウトのテスト資産はそのまま生かす。受け入れ基準は**再描画方式の golden と pixel 等価**（見た目を変えない純粋等価移行・wuc-migration と同型／合成スケール k=1.0 で byte 一致・k≠1.0 は小数アキュムレータで ≤0.5px＋手動確認）。方針正本は brief.md・roadmap「emo の責務範囲」節・記憶 areka-emo-ui-layer-text-roadmap／areka-emo-own-compositor-atlas。

## Introduction

areka の ⑥ emo トラックにおいて、emo-text-layer ✅ が実装したバルーンテキスト層のスクロール実現方式を**全域再描画方式から viewbox 方式（ダーティ矩形スクロール）へ差し替える**増分ユニットである。emo-text-layer は「可視窓の決定（純粋な計算）」と「描画実行」を意図的に分離しており（`LayoutEngine::visible_window` が唯一のスクロール決定点・`ContentCanvas` が全行を可視窓非適用で保持・doc コメントが本ユニットを名指しで移行先として明記）、描画実行だけが `validrect` サイズの描画面へ可視窓を毎フレーム全域再描画している。本ユニットは、この**描画実行側だけ**を、古典 Win32 の `ScrollDC`／`InvalidateRect`／`WM_PAINT` を写した**ダーティ矩形スクロール**へ置換する。

viewbox の構成は、**validrect 物理寸に固定した単一の描画面（重なり安全な面内 blit のためダブルバッファ）**である。スクロールは、描画面内で既に描画済みの保持ピクセルを **whole-pixel で面内平行移動（blit）**して表現する（描画面自体は成長させない・viewport/content の 2 層 visual offset 構成は用いない）。スクロールで露出した帯と、typewriter で描画中の現在行の矩形＝**ダーティ領域だけ**を D2D 描画コマンド（`DrawTextLayout`）で（再）描画し、確定した content は描画面のピクセルとして保持したまま二度と再描画しない。ダーティ領域の描画は**D2D 描画コマンドを維持**する（確定 content 用に別途のグリフビットマップキャッシュ／`ID2D1CommandList` を設けず、**描画面＋blit を保持機構**とする＝`WM_PAINT` のバッキングストア規律）。この健全性は、emo-text-layer の `ContentCanvas`（全行保持・可視窓非適用の純粋層）が**アプリ側の真のモデル＝バッキングストア**として機能することで担保される（露出帯・多行ダーティは canvas から再描画できる）。

サブピクセル描画は**維持する**：ダーティ領域は真位置で D2D 描画（DirectWrite native の DPI サブピクセル AA）し、確定 content は whole-pixel blit（再サンプリングなし＝ClearType 位相不変）で移動するため、任意のスクロール位置でグリフが滲まない。スクロール位置の内部表現は **f32（連続量）**とし、blit 量への写像を **whole-pixel（物理ピクセル整数）**に量子化する（この f32／写像の分離が M2 補間のシーム）。スクロール軸は `writing_mode` に追随する（横書き＝縦 blit・下端露出／縦書き＝横 blit）。

本ユニットの成果は emo-text-layer と**交差面ゼロ**で成立する（emo-text-layer の可視窓決定・スクロール状態機械・レイアウト・純粋層は不変で、それらのテスト資産がそのまま生きる）。並走候補の emo2-boot は areka-emo-text を**消費のみ**（sink／装着 API／present_frame）するのに対し、本ユニットは**描画実行側のみを改変**するため両者は非交差であり、pixel 等価 golden が挙動不変を担保する。M1 相当の受け入れはステップスクロール等価（golden 一致）に限り、オフセット補間による滑らか演出・慣性は**シームのみ**（滑らか・crisp 両立には真位置再描画が要る点も含め、実挙動は M2 の新能力解禁と歩調を合わせる）とする。

## Boundary Context

- **In scope（本ユニットが観測可能に実現する振る舞い）**:
  - **固定 validrect 物理寸の単一描画面（ダブルバッファ）**による viewbox 構成。描画面自体は成長させず、viewport/content の 2 層 visual offset 構成は用いない。封じ込めは描画面の寸（validrect）で成立し、ダーティ矩形の描画は D2D の矩形クリップ（`PushAxisAlignedClip` 相当）でダーティ領域に限定する。
  - emo-text-layer の描画実行の分離シームのうち**描画実行側だけ**を、全域再描画（validrect サイズ面へ可視窓を毎フレーム描き直す）から**ダーティ矩形スクロール**（確定ピクセルの whole-pixel 面内 blit ＋ 露出帯・現在行の D2D 再描画）へ差し替える。可視窓決定（純粋層 `visible_window`）・スクロール発火規則・レイアウト・状態機械は不変で消費する。
  - 確定した content を描画面のピクセルとして保持し、スクロール（可視窓の移動）時に**保持ピクセルを whole-pixel blit で移動**して、確定 content の文字（グリフ）再描画を発生させないこと。描画対象はスクロールで露出した帯に限る。
  - content 内容が変化する事象（Text 追記／NewLine／typewriter による可視グリフ増加）に対して、**ダーティ矩形（現在描画中の行 ∪ スクロール露出帯）だけ**を D2D 描画コマンド（`DrawTextLayout`）で（再）描画し、確定済み content を再描画しないこと。ダーティ領域の描画は D2D 描画コマンドを維持する（別途のグリフビットマップ／command list キャッシュを設けない）。
  - 描画面を validrect 物理寸に**固定**（ダブルバッファの 2 枚）し、talk が長く続いても描画面を成長（再確保）させないこと。あふれた content は面内 blit で描画面外へ送り、可視窓ぶんだけを保持する。Clear による描画面リセット（全域透明へ戻す・スクロール/ダーティ状態と行キャッシュを初期化）。
  - スクロールの blit 方向（露出帯の生じる辺）を `writing_mode` に追随させる（横書き＝縦 blit・下端露出／縦書き＝横 blit・emo-text-layer の軸読み替え正準表と一致）。
  - スクロール位置の内部表現を f32（連続量）で保持し、blit 量への写像を whole-pixel（物理ピクセル整数）に量子化すること（サブピクセル維持の構造的必然・M2 補間のシーム）。
  - 差し替え後の表示結果が**再描画方式の golden と pixel 等価**であること（オフスクリーン readback／横書き・縦書き両方／k=1.0 は byte 一致・k≠1.0 は小数アキュムレータで ≤0.5px＋手動確認）。等価検証は再描画方式（比較専用に `#[cfg(test)]` 保持）と同一プロセス・同一ターゲットで両方式を走らせる live-diff を主檻とする。
  - 固定層（`\_b --option=fixed`＝スクロールした時に画像を動かさない層）の差し込み点を、スクロール blit の対象外となる別合成層として**予約**する（M1 では実挙動を実装しない・後続の `\_b` 対応増分が構成再構築なしで済む形）。
  - 滑らか補間・慣性のシーム（M1 はステップスクロール等価・オフセット位置の f32 連続量シームのみ・実挙動は M2）。
  - 上記を単一 pass/fail で観測できる形（emo-text-layer の観測 example のスクロール経路を viewbox 方式へ差し替える）。

- **Out of scope（本ユニットが所有しない）**:
  - スクロール状態機械・可視窓決定（`visible_window`）・文字レイアウト・折返し・行送り・typewriter 進行・writing_mode 2 層マージ解決（すべて **emo-text-layer** の責務・不変で消費する）。全行を保持する内容モデル（`ContentCanvas`＝バッキングストア）も emo-text-layer の純粋層であり不変で消費する。
  - オフセット補間による滑らかなスクロール演出・慣性の実挙動（**M2**・本ユニットはシームまで／滑らか・crisp 両立には真位置再描画が要る点を含め M2 の設計事項）。
  - `\_b` 固定層の実装（画像読込・固定層の実描画）。本ユニットは差し込み点（scroll 面に重ねる別層）の予約のみ。
  - choice-render（M-dialogue）のクリック可能範囲の実導出（本ユニットは描画面（validrect）座標系の契約点を残すのみ・実導出は choice-render の責務）。
  - sink の main 結線（`GhostBootOptions.text_sink` への注入・実 talk 経路）・バルーン枠の描画/配置（emo-present）・surface 合成（emo-compose）・バルーン窓の生成/配置（window-placement）。

- **Adjacent expectations（隣接ユニットへの期待・依存）**:
  - **emo-text-layer** ✅ の分離シームを消費する。可視窓決定 `LayoutEngine::visible_window(...) -> VisibleWindow { first_visible_line, block_offset }` を唯一のスクロール決定点として不変で用い、内容キャンバス `ContentCanvas`（全行を住人として保持・可視窓非適用）を**バッキングストア＝ダーティ矩形（露出帯・多行ダーティ）の再描画元**として不変で用いる。emo-text-layer の呼び順の結線点（`visible_window` → `ContentCanvas::from_layout` → 描画実行）は保ち、置換するのは描画実行の中身のみ。
  - **visual-clip** ✅（`completed/visual-clip`）の `ClipShape::Rectangle`＋`clip_sync_system`（visual レベルの矩形クリップ）は、本ユニットの M1 viewbox 実現には**必須としない**（描画面が validrect 寸で自ら封じ込め・ダーティ矩形は D2D の矩形クリップで限定）。将来の固定層合成等で必要が生じた場合に消費し得るが、M1 では依存しない。
  - emo-text-layer の golden 資産（オフスクリーン readback 述語群・複数スケール検証）を pixel 等価の比較基準として消費する。再描画方式の `DrawExecutor` を比較専用の独立オラクルとして `#[cfg(test)]` に保持する（除去は後続の別決断）。
  - emo-text-layer の DPI/スケール契約（レイアウト座標＝image px・描画ターゲットに合成スケールを適用）を保ち、viewbox 化で論理/物理の混在を持ち込まない（記憶 areka-window-placement-dpi-coordinate-defect の教訓）。whole-pixel blit の量子化・小数アキュムレータもこの契約の上で行う。
  - **emo2-boot**（並走候補）は areka-emo-text を消費のみ（sink／装着 API／present_frame）するため本ユニットの改変面（描画実行）と非交差である。pixel 等価 golden が両者の挙動不変を担保する。
  - **choice-render**（M-dialogue・間接）は本ユニットが残す描画面（validrect）座標系の契約点（クリック範囲を描画面座標で消費する際の座標契約）を下流で用いる。
  - **`\_b --option=fixed` 固定層の実装増分**（後続）は本ユニットが予約する固定層の差し込み点（scroll 面に重ねる別合成層）を消費する。
  - **M2 スクロール演出**（補間・慣性）は本ユニットが残すスクロール位置の f32 連続量シームを消費する。

## Requirements

### Requirement 1: 固定 validrect 描画面（ダブルバッファ）＋面内スクロール構成

**Objective:** As a emo バルーンテキスト層, I want validrect 寸に固定した単一描画面の中で確定ピクセルを面内移動する構成を持つこと, so that テキスト内容を保持したまま可視窓だけを動かす土台が立つ

#### Acceptance Criteria

1. The emo テキスト viewbox shall テキスト表示を validrect サイズの単一描画面で構成し、面内でスクロールする（viewport visual と content visual の 2 層 offset 構成は用いない）。
2. The emo テキスト viewbox shall スクロール位置を、描画面内で既に描画済みの保持ピクセルの whole-pixel 面内平行移動（blit）で表現し、重なり安全のため描画面をダブルバッファ（2 枚）で保持する。
3. While スクロール位置が変化する, the emo テキスト viewbox shall 保持ピクセルの blit と露出帯の描画のみを行い、描画面のサイズ（validrect 物理寸）を不変に保つ。
4. The emo テキスト viewbox shall 描画面外への封じ込めを描画面の寸（validrect）で成立させ、ダーティ矩形の描画を D2D の矩形クリップ（`PushAxisAlignedClip` 相当）でダーティ領域に限定する。
5. The emo テキスト viewbox shall 描画面の validrect 寸を emo-text-layer の DPI/スケール契約（validrect 寸に合成スケールを適用した物理寸）に一致させ、任意のモニタ DPI で content が validrect からあふれて見えないようにする。

### Requirement 2: 描画実行の差し替え（分離シーム消費・上流不変）

**Objective:** As a emo テキスト viewbox, I want emo-text-layer の描画実行側だけを差し替えること, so that 可視窓決定・状態機械・レイアウトのテスト資産を壊さずスクロール実現方式を移行できる

#### Acceptance Criteria

1. The emo テキスト viewbox shall emo-text-layer の可視窓決定（`LayoutEngine::visible_window` の出力 `VisibleWindow { first_visible_line, block_offset }`）を唯一のスクロール決定点として不変で消費する。
2. The emo テキスト viewbox shall emo-text-layer の内容キャンバス（`ContentCanvas`・全行を住人として保持し可視窓を適用しない）をダーティ矩形の再描画元（バッキングストア）として不変で消費する。
3. The emo テキスト viewbox shall 可視窓の `first_visible_line`／`block_offset` を面内スクロール blit の量（whole-pixel）とダーティ矩形の描画位置へ写像し、描画実行（全域再描画）を「保持ピクセルの blit ＋ ダーティ矩形の D2D 描画」へ置換する。
4. The emo テキスト viewbox shall emo-text-layer のスクロール状態機械・スクロール発火規則（あふれ判定）・折返し／行送り／レイアウト・typewriter 進行・writing_mode 解決を改変せず、それらを不変で消費する。
5. When viewbox 方式へ差し替える, the emo テキスト viewbox shall emo-text-layer の純粋層（可視窓決定・状態機械・レイアウト）に対する既存テスト資産が改変なしに成立し続けるようにする。

### Requirement 3: ダーティ矩形のみ D2D 描画（確定 content 再描画レス）

**Objective:** As a emo テキスト viewbox, I want 確定 content を描画面のピクセルとして保持しダーティ領域だけを描くこと, so that typewriter 毎グリフ更新と重なる再描画の累積を解消し滑らか演出の土台を作る

#### Acceptance Criteria

1. The emo テキスト viewbox shall 確定した content を描画面のピクセルとして保持し、スクロール時に保持ピクセルを whole-pixel blit で移動して、確定 content を D2D で再描画しない。
2. When 可視窓のみが移動する（content 内容は不変）, the emo テキスト viewbox shall スクロール blit（保持ピクセルの平行移動）のみを行い、確定 content の文字（グリフ）描画を発生させず、スクロールで露出した帯だけを描画対象とする。
3. When content 内容が伸びる（Text 追記／NewLine／typewriter による可視グリフ増加）, the emo テキスト viewbox shall ダーティ矩形（現在描画中の行 ∪ スクロール露出帯）だけを D2D 描画コマンド（`DrawTextLayout`）で（再）描画し、既に確定した content 部分を再描画しない。
4. The emo テキスト viewbox shall ダーティ矩形の描画に D2D 描画コマンドを維持し、確定 content 用に別途のグリフビットマップキャッシュ／`ID2D1CommandList` を設けず、描画面＋面内 blit を確定 content の保持機構とする。
5. The emo テキスト viewbox shall スクロール時に確定 content の再描画が発生せず描画がダーティ矩形に限られることを、決定論的な描画呼び出しカウント（DirectWrite レイアウト生成・`DrawTextLayout` 実行回数がダーティ矩形分に限られること）またはログ捕捉で検証できる形にする（目視に依存しない・記憶 deterministic-test-coverage-mandate）。
6. While M1 である, the emo テキスト viewbox shall スクロールをステップ（行単位・即時の whole-pixel blit）で行い、フレーム間のオフセット補間を伴わない。

### Requirement 4: 描画面の固定寸・Clear リセット（無限成長の構造的排除）

**Objective:** As a emo テキスト viewbox, I want 描画面を validrect 固定寸に保ちあふれを面内 blit で送り出すこと, so that talk が長く続いても描画面が無限に成長しない

#### Acceptance Criteria

1. The emo テキスト viewbox shall 描画面を validrect 物理寸の固定サイズ（重なり安全のためダブルバッファの 2 枚）とし、talk 進行で描画面を成長（再確保）させない。
2. When content が現在の可視窓を超えて伸びる, the emo テキスト viewbox shall 面内スクロール blit で確定行を描画面外へ送り、可視窓ぶんだけを描画面に保持する（あふれた確定行のピクセルは保持せず、必要時は `ContentCanvas` から再描画する）。
3. When Clear cue が適用される, the emo テキスト viewbox shall 描画面を初期状態（全域透明）へリセットし、スクロール/ダーティ状態と行 TextLayout キャッシュを初期化する。
4. The emo テキスト viewbox shall 描画面が validrect 固定寸であることにより描画面のメモリを固定上限に保ち、成長する描画面の上限値管理を要さずに無限成長を構造的に排除する。
5. The emo テキスト viewbox shall 描画面のスクロール・リセットの前後で、表示結果が再描画方式の golden と pixel 等価（k=1.0）であることを保つ。

### Requirement 5: writing_mode 追随の blit 軸切替

**Objective:** As a emo テキスト viewbox, I want スクロール blit の軸を writing_mode に追随させること, so that 縦書きでも正しい方向へスクロールする

#### Acceptance Criteria

1. While `writing_mode` が横書き（`horizontal_tb`）である, the emo テキスト viewbox shall スクロール blit を縦方向（ブロック軸）に行い、露出帯を下端に生じさせて縦スクロールを実現する。
2. While `writing_mode` が縦書き（`vertical_rl`）である, the emo テキスト viewbox shall スクロール blit を横方向に行い、行が左へ流れる横スクロールを実現する。
3. The emo テキスト viewbox shall blit 方向・露出帯の生じる辺を emo-text-layer の軸読み替え正準表（可視窓 `block_offset` の符号・方向規約）と一致させ、独自の軸規則を発明しない。
4. The emo テキスト viewbox shall 描画面の寸（validrect サイズ）を `writing_mode` によらず一定に保ち、blit 方向のみを切り替える。

### Requirement 6: 再描画方式との pixel 等価（golden 一致）

**Objective:** As a 開発者, I want viewbox 方式の表示が再描画方式の golden と pixel 等価であること, so that スクロール実現方式の差し替えで見た目を一切変えないことを証明できる

#### Acceptance Criteria

1. The emo テキスト viewbox shall 同一の cue 列・同一の注入時刻列に対する表示結果を、emo-text-layer の再描画方式の表示結果とオフスクリーン readback で、合成スケール k=1.0 において pixel 等価（byte 一致）にする。等価検証は再描画方式（比較専用に `#[cfg(test)]` 保持の独立オラクル）と同一プロセス・同一ターゲットで両方式を走らせる live-diff を主檻とする。
2. The emo テキスト viewbox shall pixel 等価をスクロール発火の前後（あふれ前・あふれ後の可視窓移動）で成立させる。
3. The emo テキスト viewbox shall pixel 等価を横書き（`horizontal_tb`）と縦書き（`vertical_rl`）の両方で成立させる。
4. While 合成スケールが k≠1.0（非 96 DPI）である, the emo テキスト viewbox shall whole-pixel blit を小数アキュムレータで真位置格子へ吸着させて累積ドリフトを防ぎ、再描画方式との差を最大 0.5px の範囲に保つ（byte 等価は k=1.0 に scope・k≠1.0 は述語一致＋手動確認 R10.6）。emo-text-layer のスケール不変検証資産を差し替え後も満たす。
5. When Clear cue が適用される, the emo テキスト viewbox shall Clear 後の表示（全域透明）を再描画方式と pixel 等価にする。

### Requirement 7: 固定層（`\_b --option=fixed`）差し込み点の予約

**Objective:** As a 後続の `\_b` 対応増分, I want viewbox 構成に固定層の差し込み点が予約されていること, so that スクロールに追従しない固定画像層を構成再構築なしに増設できる

#### Acceptance Criteria

1. The emo テキスト viewbox shall スクロール blit の対象外となる固定層（保持ピクセルの面内移動の影響を受けない別合成層）の差し込み点を予約する。
2. While M1 である, the emo テキスト viewbox shall 固定層の実挙動（画像読込・固定層の実描画）を実装せず、差し込み点の構造予約に留める。
3. The emo テキスト viewbox shall 固定層を後から増設する際に、スクロール描画面／面内 blit の構成を再構築せずに済む形（scroll 面に重ねる別合成層）で予約する。
4. The emo テキスト viewbox shall 固定層差し込み点の予約が M1 の pixel 等価 golden（固定層なしの表示）に影響しないようにする。

### Requirement 8: 滑らか補間・慣性のシーム

**Objective:** As a M2 スクロール演出, I want オフセット補間・慣性のシームが用意されていること, so that 滑らかなスクロール演出を破壊的変更なしに解禁できる

#### Acceptance Criteria

1. While M1 である, the emo テキスト viewbox shall スクロールをステップスクロール等価（whole-pixel blit の即時追従）で行い、その結果が再描画方式の golden と pixel 等価（k=1.0）である。
2. The emo テキスト viewbox shall スクロール位置の内部表現を f32（連続量）で保持し、blit 量への写像（whole-pixel 量子化）を分離することで、オフセット補間（フレーム間の中間位置生成）・慣性を型/構造シームとして保持し、その実挙動を M1 で実装しない。
3. The emo テキスト viewbox shall 補間シームを、スクロール位置（f32 連続量）が可視窓決定（`block_offset`）と補間過程とで分離できる形で残し、M2 の補間実装（滑らか・crisp 両立には真位置再描画が要る点を含む）が描画実行の再設計を要さないようにする。

### Requirement 9: クロスユニット契約シーム

**Objective:** As a 下流/並走ユニット, I want 本ユニットの差し替えが emo-text-layer・emo2-boot・choice-render を詰ませないこと, so that 各ユニットへの接続が破壊的変更なしに進む

#### Acceptance Criteria

1. The emo テキスト viewbox shall emo-text-layer の可視窓決定・状態機械・レイアウト・純粋層を改変せず、それらの公開契約・テスト資産を不変に保つ。
2. The emo テキスト viewbox shall emo2-boot が消費する経路（sink／装着 API／present_frame）を再定義せず、本ユニットの改変を描画実行側に閉じて emo2-boot と非交差にする。
3. The emo テキスト viewbox shall choice-render（M-dialogue）がクリック可能範囲を消費する際に用いる描画面（validrect）座標系の契約点（スクロール位置と描画面クリップの座標関係）を構造として残す（クリック範囲の実導出は実装しない）。
4. The emo テキスト viewbox shall wintf のクリップ機構（`ClipShape`／`clip_sync_system`）を本ユニットの M1 viewbox 実現の必須依存とせず（描画面の寸で封じ込め・ダーティ矩形は D2D の矩形クリップ）、wintf を改変しない（不足が判明した場合のみ wintf への増分 issue として申し送る）。
5. The emo テキスト viewbox shall スクロール実現方式の差し替えを additive に行い、emo-present／emo-compose／sakura／balloon-parse の既存契約を再定義しない。

### Requirement 10: 観測用 example（スクロール経路の viewbox 差し替え・単一 pass/fail）

**Objective:** As a 開発者, I want 単一 pass/fail で viewbox スクロールを確認できる観測経路, so that 再描画レス・pixel 等価・軸追随を実機で証明できる

#### Acceptance Criteria

1. The emo テキスト viewbox 観測経路 shall emo-text-layer の観測 example のスクロール経路を viewbox 方式（ダーティ矩形スクロール）へ差し替え、fixture スクリプト由来の cue 列（Text／NewLine／Clear）を注入時刻駆動で流す（実時間 sleep 不使用・決定論）。
2. When スクロールが発火する, the emo テキスト viewbox 観測経路 shall 表示結果が再描画方式の golden と pixel 等価（k=1.0 byte 一致）であることを観測可能にする（横書き／縦書き両方・オフスクリーン readback）。
3. When 可視窓のみが移動する, the emo テキスト viewbox 観測経路 shall 確定 content の再描画が発生せず描画がダーティ矩形（露出帯）に限られることを決定論的な描画呼び出しカウントまたはログ捕捉で観測可能にする。
4. When `writing_mode` が縦書き（`vertical_rl`）である, the emo テキスト viewbox 観測経路 shall スクロールの blit 方向が横方向へ切り替わることを観測可能にする。
5. When Clear cue を注入する, the emo テキスト viewbox 観測経路 shall 描画面がリセットされ表示が全域透明へ戻ることを観測可能にする。
6. The emo テキスト viewbox 観測経路 shall DPI/スケールの正しさを検証する——pixel 等価を非 96 DPI を含む実 DPI/合成スケールで観測可能にし（k=1.0 は byte 一致・k≠1.0 は述語一致＋≤0.5px）、実 DPI 実行を経ない「テスト緑」を正しさの証明としない（記憶 areka-placement-real-ghost-first）。
