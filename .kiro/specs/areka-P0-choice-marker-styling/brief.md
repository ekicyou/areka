# Brief: areka-P0-choice-marker-styling

> 起票: 2026-08-27（bvc 要件ディスカッション議題 5 の開発者指示による `/kiro-discovery` 再入・文字装飾系 3 spec 分割の 3 本目＝最小）
> **`\f[cursor*]` 10 項目＝選択肢マーカーの実行時上書き。** 完了 spec `choice-render` が descript `cursor.*` キーで既に解決しているレンダラ経路の上に乗る、3 本中もっとも安い spec。

## Problem

選択肢（`\q`）のマーカー見た目を実行時に変える `\f[cursor*]` 系 10 項目が未実装——`cursorstyle`（square|underline|square+underline|none）・`cursorcolor`/`cursorbrushcolor`・`cursorfontcolor`・`cursorpencolor`・`cursormethod`（Win32 `SetROP2` 演算子名・`default` で復帰）・`cursornotselect*` ×5。descript の静的指定（`cursor.*`）はあるが、スクリプトからの動的上書きができない。

## Current State

- descript 側は完了 spec `choice-render` が解決済み: `choice.rs:440-476` が `cursor.style` 等を読む。**`underline` スタイルは `SquareFill` へ warn-once 縮退**（`choice.rs:472-476`・`:519-521`）——下線描画そのものが無いための縮退で、`text-decoration-canon` の下線基盤が立てば実装可能になる（この縮退解除は本 spec の中核）。
- `\f[cursor*]` は `\f` 族共通のパススルー破棄経路（`text-decoration-canon` brief 参照）。
- hover 描画の `DWRITE_TEXT_RANGE` 適用先例は既存（`viewbox_draw.rs:346-354`）。

## Desired Outcome

`\f[cursor*]` 10 項目が descript 指定の実行時上書きとして効き、`cursorstyle,underline` が本物の下線で描かれ（縦書きでは列の右側——bvc の下線写像を継承）、`default` 指定で descript 値へ復帰する。

## Approach

`text-decoration-canon` の `"f"` 解読腕＋装飾 CueCommand に cursor 系 10 項目を追加し、`choice-render` の既存解決（descript 層）の上に実行時層を重ねる（2 層・後勝ち＝バルーン定義の 2 層マージと同じ形）。

## Scope

- **In**: `\f[cursor*]` 10 項目・descript 値との 2 層解決・`underline` 縮退の解除・`SetROP2` 演算子名の受理と縮退（未知名）・決定論テスト。
- **Out**: `\f` 解読基盤と per-run 属性（`text-decoration-canon`）・選択肢の機構そのもの（完了済み・不変）・アンカー系（`anchor-tag-canon`）。

## Boundary Candidates

- 解決層（descript × 実行時の 2 層）と描画（underline 実装・ROP2）の 2 相。

## Out of Boundary

- `\q`/`\__q` の選択・イベント発火（choice-select-events・完了済み）。

## Upstream / Downstream

- **Upstream**: `text-decoration-canon`（解読腕・下線基盤・必須先行）・`choice-render`（descript 解決・完了済み）・bvc（下線の縦書き写像）。
- **Downstream**: 選択肢の見た目を演出するゴースト資産の互換。

## Existing Spec Touchpoints

- **Extends**: `choice-render` の `underline`→`SquareFill` 縮退（warn-once 檻の退役を伴う＝完了 spec 正典の追随規律）。
- **Adjacent**: `anchor-tag-canon`（同型の 3 状態装飾・別経路）。

## Constraints

- ウェーブ配置: **M2 解禁ゲート**（`text-decoration-canon` の後段・3 本中最小＝S）。
- 決定論テスト必達（10 項目 × descript 有無 × 縦横の下線位置）。

---

> **📌 2026-09-02 棚卸⑫**——アンカー **実質ドリフト 0**（`choice.rs` :432 impl／:450 `resolve`／:470 `match cursor.style()`／:472-476 underline→`SquareFill` warn-once／:519-521 `style_has_underline`・`viewbox_draw.rs:346-354`）。前提: decoration 未着手（必須先行）・`choice-render` ✅・bvc ✅。編成＝W14 裁定枠（anchor と同居・`decode.rs` は decoration の `"f"` 腕の内側＝所有分割を design で確認）。規模 S・要件定義は Opus で足りる。

> **📌 2026-09-13 相互登記（`areka-P0-text-decoration-canon` 着地）**——`\f[cursor*]` 10 項目は親 spec が `ActorTextState::unowned_vocab()`（`crates/areka-emo-text/src/state_decoration.rs`）に保持するだけで表示を変えない。`\f[color,default.cursor*]` の色源は `look.rs::LookLayers::cursor_text`（`draw.rs::ResolvedFont::resolve_with_background` が `choice.rs::ResolvedChoiceStyle::resolve` から取る）、`default` 復帰の実体は `state_decoration.rs::TextLayerState::reset_decoration`。下線の描画基盤は着地済み（`viewbox_draw_decoration.rs::apply_font_ranges` が `SetUnderline` を区間へ渡す）ため、`choice.rs` の underline 系→`SquareFill` 縮退は解除できる。

## `balloon-color-emoji` からの申し送り（2026-09-30 着地）

- バルーンの文字の単位は `char` から書記素クラスタ（人が 1 文字と見る単位）に替わった。範囲（`ChoiceSpan::glyph_range`・`style_runs`・`segment_text_range`）はクラスタの通し番号で数え、UTF-16 の位置はクラスタ文字列の長さを積む。
- `TextItem::Glyph` と `PositionedGlyph` の中身は `text: Arc<str>`（`Copy` なし）。構築は `TextItem::glyph(&str)`、切り方は `areka_sakura::cluster::clusters` だけが決める。文字を比べる処理（行末のぶら下げの判定など）は `&str` で比べる。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 低。中身は今も正しいが、行番号はすべてずれた（警告は今 `choice.rs` の 585〜591 行あたり）。`text-align-shadow-canon` への依存は同じファイルを触るだけ（`look.rs`・`viewbox_draw` 系）＝文字まわりの直列の列に並ぶ。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S（5〜8 タスク）。切らない。
- 前提の状態: `text-decoration-canon`（下線の基盤）・`choice-render` は着地済み。列の上では `text-align-shadow-canon` の後（同じ `look.rs`・`viewbox_draw_render.rs`）＝その意味では未。
- 崩れた前提／古くなった位置:
  - 行番号はすべて古い。下線系→塗りへの縮退は `choice.rs` の `ResolvedChoiceStyle::resolve`（`style_has_underline` の腕で warn「cursor.style underline 系は M1 未対応: SquareFill へ縮退」）。ホバーの描画は分割で `viewbox_draw_render.rs`（`ChoiceDraw`・`ChoiceHover`・`highlight_rect`）へ移った。下線を区間へ渡す `apply_font_ranges` は `viewbox_draw_decoration.rs`。所有外のキーの判定は `look.rs` の `is_unowned`（`starts_with("cursor")`）。
  - **シェル内バルーンの箱にも自動で効く**: descript の `cursor.*` は `ResolvedBalloonText::choice_style` に解かれ、箱も同じ `ResolvedBalloonText::resolve_with_background` を通る（`actor_box.rs` の `register_box`）。実行時の `\f[cursor*]` は箱の `font.follow` の振り分け（`state_decoration.rs`）に乗る。箱の選択肢の当たりは `input_events/shell_box.rs`／`shell_box_handler.rs` で、本 spec は触らない。
  - **`anchor-tag-canon` の装飾の側と形がほぼ同じ**（形状 4 種・ブラシ／ペン／文字の色・`SetROP2` の名前の描画方法・非選択の 5 項目）。2 本を列で隣に並べ、先に着地する方が「descript × 実行時の 2 層で印の見た目を解く型」と `SetROP2` の名前の受け取り（未知の名前の縮退込み）を作り、後の方が使う形を推す。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{choice.rs, look.rs, state_decoration.rs, viewbox_draw_render.rs, viewbox_draw_decoration.rs}`
  - `crates/areka-parsers/src/sakura/decode.rs`（`"f"` の腕の内側で済むなら触らない）
  - `doc/ukadoc-coverage/ledger/sakura-script.toml`
- 議題（答えで作業が変わるものだけ）: `SetROP2` の描画方法（`cursormethod`）を Direct2D でどこまで再現するか（D2D に ROP2 は無い。`copypen` 以外を合成モードへ写すか、既定へ縮退して記録するか）。`anchor-tag-canon` の装飾の側と同じ答えにする。
- 見つけた穴: なし。


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S（5〜8 タスク）のまま。切らない。
- 前提の状態: 機能の前提（`text-decoration-canon` の下線の基盤・`choice-render`）は着地済み。列の上の待ちは `text-align-shadow-canon`（同じ `look.rs`・`viewbox_draw_render.rs`）だけで、機能の依存ではない。
- 崩れた前提／古くなった位置:
  - C3 で `choice.rs`・`state_decoration.rs`・`viewbox_draw_render.rs`・`viewbox_draw_decoration.rs` は無変更、`look.rs` は注記 1 行（「M2 予約」→「予約」）だけ。棚卸㉑の位置はそのまま当たる: 下線系→塗りへの縮退は `choice.rs` の `ResolvedChoiceStyle::resolve` の `style_has_underline` の腕（警告「cursor.style underline 系は M1 未対応: SquareFill へ縮退」）、同じ関数の中のすぐ上に `cursor.blendmethod` の ROP 系の縮退の警告（「none 扱い（色ベース描画）へ縮退」）もある（`cursormethod` の議題と同じ扱いになる）。所有外の判定は `look.rs` の `is_unowned`（`starts_with("cursor")`）。
  - 台本の側は `areka-parsers/src/sakura/decode.rs` の `"f"` の腕が引数をそのまま運ぶ（`Instruction::Font { args }`）＝`decode.rs` は触らずに済む見込みが強まった。
  - 元の名で書いていた「`anchor-tag-canon` の装飾の側」は、10-04 に `anchor-style-canon` へ切り出された。議題の「同じ答えにする」相手は `anchor-style-canon`。
- 触るファイル（並走の照合用）: `crates/areka-emo-text/src/{choice.rs, look.rs, state_decoration.rs, viewbox_draw_render.rs, viewbox_draw_decoration.rs}`・`doc/ukadoc-coverage/ledger/sakura-script.toml`
- 並走の見立て: `budoux-reveal-reflow`（`actor_present.rs`・`state.rs`・`segment.rs`）とはファイルが重ならない。`balloon-font-file`（`viewbox_draw_render.rs`・`look.rs`）・`anchor-tag-canon`（`choice.rs`・`viewbox_draw_render.rs`）とは重なる。列の順（`text-align-shadow-canon` の後）は同じファイルを触るからで機能の依存ではないので、列を飛ばして前へ出すなら相手は `budoux-reveal-reflow` だけ。
- 議題: `SetROP2` の描画方法（`cursormethod`・descript の `cursor.blendmethod` の縮退と合わせて）を D2D でどこまで再現するか。`anchor-style-canon` と同じ答えにする。
- 見つけた穴: なし。
