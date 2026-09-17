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
- **Out**: 基盤・font 系 10・`default`／`disable`（親 spec）／descript `font.*` 基底 13 キー（`balloon-font-descript-keys`）／行末禁則のぶら下がり（`emo-text-canon-residue` 項目 15）。

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
