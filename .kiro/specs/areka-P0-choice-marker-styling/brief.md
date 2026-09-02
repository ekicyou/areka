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

