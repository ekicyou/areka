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


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M〜L（17〜21 タスク）のまま。今は切らない（一度切り出したもの）。要件で 20 を超えたら棚卸㉑の境界（②影 S 6〜8 → ①寄せ M 10〜13 の順）で切る。
- 前提の状態: 上流の完了 spec はそろっている。列の上では `balloon-scroll-fade` の後、`text-typesetting` の後の前提で並ぶ＝その意味では未。
- 崩れた前提／古くなった位置:
  - 棚卸㉑の位置は全部そのまま当たる（`layout_scan.rs` の `Scan::cursor_move` と `finish_line`・`layout.rs` の `LayoutEngine::visible_window`・`layout_cursor_overflow_tests.rs`・`draw.rs` の `DirectionRecipe` の組み立ての中の `DWRITE_TEXT_ALIGNMENT_LEADING`／`DWRITE_PARAGRAPH_ALIGNMENT_NEAR`・`TEXT_DRAW_OPTIONS`・`look.rs` の `UNOWNED_KEYS`（4 つ）・`balloon_overrides.rs` の `overrides`・`BalloonModel::font_shadow_raw` と `shadow_raw`）。
  - C3 での変化は注記だけ: `canvas.rs`・`draw.rs`・`look.rs` の「M2 予約」が「予約」へ言い換わった（`RESERVED_EFFECT_SHADOW` の定義と所有の注記は残っている）。
- 触るファイル（並走の照合用）:
  - ①寄せ: `crates/areka-emo-text/src/{layout.rs, layout_scan.rs, layout_scan_glyph.rs, cursor_tag.rs, draw.rs, look.rs, state_decoration.rs}`・`layout_cursor_overflow_tests.rs` ほか兄弟テスト・`crates/areka-parsers/src/sakura/decode.rs`（`"f"` の腕の内側）
  - ②影: `crates/areka-emo-text/src/{look.rs, canvas.rs, balloon_overrides.rs, viewbox_draw_render.rs, viewbox_draw_decoration.rs}`
  - 共通: `doc/COMPAT_ARCHITECTURE.md` §8
- 並走の見立て: ②影だけなら `budoux-reveal-reflow`（`actor_present.rs`・`state.rs`・`segment.rs`）とはファイルが重ならない。ただし `balloon-font-file`（`viewbox_draw_render.rs` の `ensure_format`・`look.rs`）と `anchor-tag-canon`（`viewbox_draw_render.rs` の既定の見た目）とは重なる＝C4 の席では並べられない。
- 議題: SC8（縦書きでのインデントの軸）の裁定（①の要件で・棚卸㉑のまま）。
- 見つけた穴: なし。


---

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - **寄せと 1 字ずつの表示**: 中央寄せ・右寄せは、行の幅が決まらないと置き場所が決まらない。1 字ずつ出している途中は行の幅が伸びていくので、届いた字だけで寄せると、出した字が同じ行の中で横へ動く。`budoux-reveal-reflow`（10-06 着地）が止めたのは行の割り当ての動きで、行の中の位置ではない。台本の全文は先に分かるようになった（`crates/areka-emo-text/src/lookahead.rs`）が、配置は見える数で走査を止め（`layout_scan_glyph.rs` の先頭）、1 字ずつの折り返しでは全文を使わない。①寄せの要件で「寄せは区間の全文の行幅で決める」かを決める（`text-typesetting` が全文の使い分けを広げた後なら、それに乗る）。
  - `mcp-author-tools`（10-08 着地）: 台本の検査が `look.rs` の `apply_font_tag` の結果で「受け取るだけのキー」を答える（`crates/areka/src/mcp/check_script_judge.rs`）。寄せ・影に持ち主が付くと答えが変わる＝`check_script_judge_tests.rs` の「効かない `\f`」の見本 `\f[align,center]` を替え、`look_font_tag_tests.rs` の所有外の 4 つの見本も直す。
  - 台本を読む段（`crates/areka-parsers/src/sakura/decode.rs` の `"f"` の腕）は引数を写すだけで、印を返す形になっても同じ＝寄せ・影のために読む段へ手を入れる必要は無い（注記の「残る 31 形」の数を直すだけ）。
  - 棚卸㉒の位置はすべて当たる（`layout_scan.rs` の `cursor_move` と `finish_line`・`layout.rs` の `visible_window`・`draw.rs` の寄せの直書きと `TEXT_DRAW_OPTIONS`・`look.rs` の `UNOWNED_KEYS` の 4 つ・`canvas.rs` の `RESERVED_EFFECT_SHADOW`・`balloon_overrides.rs` の `overrides`）。`look.rs` は 802 行。
- 触るファイル: 棚卸㉒の一覧に、`crates/areka/src/mcp/check_script_judge_tests.rs` と（寄せを全文で決めるなら）emo-text の `lookahead.rs`・`actor_present.rs` を足す。網羅台帳で本 spec が持ち主の行は `sakura-script.toml` 5・`assets.toml` 4。検査の兄弟のファイルを足す＝**emo-text にファイルを足す**（`lib.rs` の席を使う）。
- 規模: 18〜22 タスク（棚卸㉒は 17〜21）。
- 分割の案: 今は切らない（`text-decoration-canon` から一度切り出した spec＝上限を少しまたぐ理由では削らない）。要件ではっきり 20 を超えたら、棚卸㉑の境界（②影 6〜8 → ①寄せ 11〜14）で切る。
- 先に要るもの: 働きの前提は満たした。`text-typesetting`（全文の使い分け）の後だと ①寄せが楽。列の順は `balloon-scroll-fade` の後。
- 優先度の区分: C（ukadoc の `\f[align]` ほか 5 項目の拾い残し・棚卸⑬で切り出し）。
- 要件定義のモデル: Fable（SC8 の裁定・寄せと 1 字ずつの表示の噛み合わせ）。
- 議題: SC8（縦書きでのインデントの軸）。⑵（新）寄せを区間の全文の行幅で決めるか。
- 見つけた穴・古くなった記述: コードの穴は無い。Constraints の編集集合にある `decode.rs` は実質は注記だけ。
