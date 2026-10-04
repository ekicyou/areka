# Brief: areka-P0-text-align-shadow-canon

> 起票: 2026-09-11（棚卸⑬・`areka-P0-text-decoration-canon` の分割 ⑵＝語彙相のうち**行の置き場所を動かす**寄せ 2 項目と影 3 項目を独立 spec に切り出し）。親 spec は基盤相＋font 系 10 項目＋`default`／`disable` を持つ。正典逐語・縦書き写像・追加登記の本文は親 brief が正本で、ここでは所有と着地条件だけを書く。

## Problem

`\f[align]`／`\f[valign]`（2.5.31／2.6.19）と `\f[shadowcolor]`／`\f[shadowcolor,none]`／`\f[shadowstyle]`（2.5.27）が未実装。`draw.rs` は `DWRITE_TEXT_ALIGNMENT_LEADING`／`DWRITE_PARAGRAPH_ALIGNMENT_NEAR` を固定している。寄せは per-run 属性（親 spec）だけでは立たず、**行矩形の置き場所**（配置層 `layout.rs`）を動かすため、親 spec の基盤の上に別の 1 塊として着地させる。

## Current State

2026-09-11 実測（親 brief の再測定・全命中）:
- align/valign 固定（`crates/areka-emo-text/src/draw.rs`）。影の描画は無い。
- `\_l` からの追加登記 4 件（親 brief「`areka-P0-cursor-tag-canon` からの追加登記」節）は**すべて本 spec の所有**: ⑴ `\_l` 直後の `align` リセット（`valign` はリセットされない）⑵ `\_l` 移動後の中央揃えのインデント（`cursor_tag.rs` の X 座標を寄せへ渡す口）⑶ 疑義 SC8（縦書きでのインデント軸）の**裁定**⑷ `layout.rs` のあふれ判定が行送り方向へ後戻りした行を境界の外に置き去りにする所見（`LayoutEngine::visible_window` が最新行の遠端だけで判定・`layout_cursor_overflow_tests.rs` が今日の値を固定）。
- 縦書き写像は bvc SC1 裁定を継承し再審議しない: `align`＝left=上/right=下・`valign`＝top=右/bottom=左。

## Desired Outcome

3 書字方向で `align`／`valign`／影 3 項目が正典どおりに効き、`\_l` 直後の `align` リセットと中央揃えのインデントが成立し、SC8 が裁定され、あふれ判定が行送り方向へ後戻りした行を扱える（追加登記 4）。

## Approach

親 spec の per-run 属性を読み、配置層で行矩形を寄せる。影は描画層で run ごとに 2 回描く（offset）か輪郭（outline）。追加登記 4 は寄せの実装が同じ前提に触れるため同時に扱う。

## Scope

- **In**: align・valign（縦書き写像込み）・shadowcolor・shadowcolor,none・shadowstyle／追加登記 1〜4／`\_l` との相互作用。
- **Out**: 基盤・font 系 10・`default`／`disable`（親 spec）／descript `font.*` 基底 14 キーと `disable.font.*` の**読み取り**（`balloon-font-descript-keys` で着地済み。ただし影 4 キー〔`font.shadowcolor.r`／`.g`／`.b`・`font.shadowstyle`〕と無効表示の影 4 キーの**配線は本 spec の範囲**——取り出し口 `BalloonModel::font_shadow_raw()`／`disable_font().shadow_raw()`、配線を足す場所 `crates/areka-emo-text/src/balloon_overrides.rs` の `overrides`、受け口を開ける場所 `look.rs` の `UNOWNED_KEYS`。詳細は `balloon-font-descript-keys` の design.md「C7 引き渡し表」）／行末禁則のぶら下がり（`emo-text-canon-residue` 項目 15）。

## Boundary Candidates

- 寄せ 2（配置層）と影 3（描画層）は別の層。同居させる理由は「語彙相で残った 5 項目を 1 塊にする」だけなので、要件段階で L と判断したら影 3 を親 spec 側へ戻してよい。

## Out of Boundary

- 折返し基準／描画範囲の意味論（`emo-text-line-height-canon` で確定）。

## Upstream / Downstream

- **Upstream**: `text-decoration-canon`（基盤・**先行必須**）・完了 `cursor-tag-canon`・完了 `emo-text-line-height-canon`（`line_pitch` の 1 点を通す）。
- **Downstream**: `anchor-tag-canon`・`choice-marker-styling`（同じ `draw.rs`／`layout.rs` を触る＝後着）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-text-decoration-canon`（分割元・語彙相の寄せ 2＋影 3 を引き継ぐ）。
- **Adjacent**: `emo-text-canon-residue`（`layout.rs` 共有＝同居不可・どちらが先でも可）。

## Constraints

- 編集集合: `crates/areka-emo-text/src/{draw,layout,state,cursor_tag}.rs`（親 spec の分割後の新ファイルを含む）＋兄弟テスト・`crates/areka-parsers/src/sakura/decode.rs`（`"f"` 腕の内側）・`doc/COMPAT_ARCHITECTURE.md` §8。
- 1,000 行番人: `layout.rs` 955 行＝新規ファイルで足す。
- 決定論テスト必達（3 書字方向 × 5 項目＋リセット＋インデント＋追加登記 4）。**要件定義は Fable**（SC8 の裁定と追加登記 4 の前提変更）。

> **📌 2026-09-13 相互登記（`areka-P0-text-decoration-canon` 着地）**——寄せ 2 項目（`align`／`valign`）と影 3 項目（`shadowcolor`／`shadowcolor,none`／`shadowstyle`）は親 spec が引数列のまま `ActorTextState::unowned_vocab()`（`crates/areka-emo-text/src/state_decoration.rs`）に保持しており表示を変えない。実装は `look.rs` の `TextLook` にフィールドを足し、`look.rs::apply_font_tag` で `Note::Unowned` を返している腕を専用の腕へ移すだけでよく、戻す操作（`state_decoration.rs::TextLayerState::reset_decoration`）は `TextLook` を丸ごと置き換えるので新しい項目も列挙なしで自動的に戻る。影の予約名 `canvas.rs::RESERVED_EFFECT_SHADOW` の実体化も本 spec の所有（親 spec が doc で明記済み）。

## `balloon-color-emoji` からの申し送り（2026-09-29）

- `balloon-color-emoji` は、バルーン文字の描画（`crates/areka-emo-text/src/viewbox_draw.rs` の `ViewboxExecutor::render`。本番の経路はここだけで、`draw.rs` の `DrawExecutor` は `#[cfg(test)]` の照合用）に `D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT` を渡す。
- 影の複製を同じ指定で描くと、カラー絵文字の影まで多色になる。**影の複製はカラーフォントを使わずに、単色のブラシで描く**こと。
- 影の位置と幅は、書記素クラスタ単位になった後の計測に従う。
- 2026-09-30 着地時の追記: 描画オプションの定義点は `crates/areka-emo-text/src/draw.rs` の `TEXT_DRAW_OPTIONS`（`ENABLE_COLOR_FONT`）。影の複製はこれを使わず `D2D1_DRAW_TEXT_OPTIONS_NONE` を明示して描く。幅の計測（`GlyphMetrics::advance`／`advance_styled`）はクラスタ文字列（`&str`）で受ける。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 中。**`emo-text-canon-residue` への依存は消えた**（重なっていた項目 15 が `text-typesetting` へ移った）。本当の制約は `layout` 系・`viewbox_draw` 系を `text-typesetting`・`text-ruby`・`shell-balloon`・`balloon-scroll-fade` と共有すること＝文字まわりの直列の列に並ぶ。
- 記述は実物と一致（寄せは `draw.rs` に直書き・影のキーは予約だけ）。
- **20 タスクを超えるおそれ**＝要件の段で「影（描く層）」と「寄せ（配置の層）」に分ける。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M〜L（17〜21 タスク）。**今は切らない**（本 spec は `text-decoration-canon` から一度切り出したもの＝上限を少しまたぐ理由でさらに削らない）。要件の段で 20 を超えたら、次の境界で**影 → 寄せの順に**切る。
  - **① 寄せ（配置の層）・M（10〜13）**: `\f[align]`／`\f[valign]`（3 書字方向・縦書きの写像は bvc の裁定のまま）と追加登記 4 件（`\_l` 直後の `align` の戻し・`\_l` の後の中央揃えのインデント・SC8 の裁定・行送り方向へ後戻りした行のあふれの判定）。
  - **② 影（描く層）・S（6〜8）**: `\f[shadowcolor]`／`\f[shadowcolor,none]`／`\f[shadowstyle]`・descript の影 4 キーと無効表示の影 4 キーの配線（`BalloonModel::font_shadow_raw`・`disable_font().shadow_raw()` は在る）・予約名 `canvas.rs` の `RESERVED_EFFECT_SHADOW` の実体化・影の複製は `D2D1_DRAW_TEXT_OPTIONS_NONE`。
  - 順序（切ったとき）: ②影を先に。配置の層に触らず小さいので、先に済ませると①の要件（SC8）に集中できる。どちらも文字の列の中で直列（`look.rs`・`viewbox_draw_render.rs` を共有）。
- 前提の状態: `text-decoration-canon`・`cursor-tag-canon`・`emo-text-line-height-canon`・`emo-text-file-split` は着地済み。列の上では `balloon-scroll-fade` の後。`text-typesetting`（同じ折り返し・配置）が先に着地している前提で並んでいる＝その意味では未。
- 崩れた前提／古くなった位置:
  - 分割での移り先（File Structure Plan のとおり）: `\_l` 直後の寄せの戻しは **`layout_scan.rs` の `Scan::cursor_move`**、行矩形の置き場所は **`layout_scan.rs` の `finish_line`**、後戻りした行のあふれの所見は **`layout.rs` の `LayoutEngine::visible_window`**（固定しているテスト `layout_cursor_overflow_tests.rs` は今 `layout_scan.rs`＋`layout_scan_glyph.rs` を読む）。行の描画は `viewbox_draw_render.rs`。寄せの直書き（`DWRITE_TEXT_ALIGNMENT_LEADING`・`DWRITE_PARAGRAPH_ALIGNMENT_NEAR`）は今も `draw.rs` の `DirectionRecipe` の組み立ての中。
  - Constraints の「`layout.rs` 955 行＝新規ファイルで足す」は古い（`layout.rs` 489・`layout_scan.rs` 428・`layout_scan_glyph.rs` 174 行）。
  - 所有外のキーの判定は `look.rs` の `UNOWNED_KEYS`（`align`・`valign`・`shadowcolor`・`shadowstyle` の 4 つ）のまま。
  - **シェル内バルーンの箱**: `\f` の指定は箱の `font.follow`（`Scope`＝スコープに付いて回る／`Balloon`＝箱だけ・`areka-emo-compose/src/boxes.rs` の `FontFollow`、文字の層では `state_decoration.rs` と `actor_box.rs`）に従う。寄せ・影を `TextLook` の欄として足せば、この振り分けに自動で乗る（`reset_decoration` が `TextLook` を丸ごと置き換えるのと同じ理由）。descript の影キーは箱の定義でも `balloon::parse` を通る。
- 触るファイル（並走の照合用）:
  - ①: `crates/areka-emo-text/src/{layout.rs, layout_scan.rs, layout_scan_glyph.rs, cursor_tag.rs, draw.rs（DirectionRecipe）, look.rs, state_decoration.rs}`・`layout_cursor_overflow_tests.rs` ほか兄弟テスト・`crates/areka-parsers/src/sakura/decode.rs`（`"f"` の腕の内側）
  - ②: `crates/areka-emo-text/src/{look.rs, canvas.rs, balloon_overrides.rs, viewbox_draw_render.rs, viewbox_draw_decoration.rs}`
  - 共通: `doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: SC8（縦書きでのインデントの軸）の裁定（brief のまま・①の要件で）。
- 見つけた穴: なし（後戻りした行が境界の外に置き去りになる所見は brief の追加登記 4 のまま。今の決定論テストが今日の値を固定している）。
