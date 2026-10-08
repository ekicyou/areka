# Brief: areka-P0-balloon-text-area-collapse

> 2026-10-07 `animated-image-playback` の完了時の棚卸で起票（`completed/areka-P0-animated-image-playback/tasks.md` の Implementation Notes の 6.3、`research.md`「実機の確かめ」の「範囲外（要起票）」1）。

## Problem

バルーンの文字の描画範囲が 0 以下に潰れると、文字の層が**毎フレーム**供給面（swap chain）を作ろうとして失敗し、WARN 1 行と ERROR 2 行をゴーストが終わるまで出し続ける。バルーンを隠しても止まらない。記録が洪水になり、ほかの記録が読めなくなる。

## Current State

- 実機の再現（10-06）: 8×8 のバルーンの面に `emo2-kakukaku` の余白（36〜56 px・面より大きい）を当てると、`WARN areka_emo_text::region: 解決後の validrect が退化している（幅/高さ ≤ 0）… left=36.0 top=46.0 right=-36.0 bottom=-48.0` に続いて、`ERROR areka_emo_text::surface: D3D/DXGI/WUC 呼び出しが失敗 hresult=-2005270527 context="create_composition_swap_chain"`（0x887A0001＝DXGI_ERROR_INVALID_CALL）と `ERROR areka::emo2_boot::frame::scale_text: emo2 text: present_frame が失敗（…次フレーム再試行…）` がフレームごとに 1 組出る。
- 供給面は `crates/areka-emo-text/src/surface.rs` の `create` が `create_composition_swap_chain(&d3d, &dxgi, w, h)` で作る。0 寸の面を作らずに済ます分岐は無い。
- `animated-image-playback` の前の実行体でも出る（本 spec が持ち込んだものではない）。
- 別の不具合: `emo-text-canon-residue` の項目 14（折返し基準が描画範囲の外に解決される）は折返しの話で、こちらは供給面の生成の話。

## Desired Outcome

- 描画範囲が 0 以下なら供給面を作らず、文字を描かない（正典の validrect の外には描かない）。
- 記録は「潰れた」「戻った」の状態が変わったときに 1 回だけ出す。毎フレームの ERROR は 0 行。
- 範囲が戻れば（面やバルーンの切り替え）、次のフレームから普通に描ける。

## Approach

0 寸を「失敗」ではなく「描くものが無い状態」として持つ。どの層（`region` の解決・`surface` の生成・`scale_text` の再試行）で止めるかは要件で決める。決定論のテストで「潰れた範囲では供給面を作らない・記録は 1 回」を固定する。

## Scope
- **In**: 文字の層の 0 寸の扱い・記録の回数・決定論のテスト。
- **Out**: 折返し基準と描画範囲の意味論（`emo-text-line-height-canon` で確定）・項目 14。

## Boundary Candidates
- `crates/areka-emo-text/src/{region,surface}.rs`
- `crates/areka/src/emo2_boot/frame/scale_text.rs`（再試行）

## Out of Boundary
- バルーンの定義の読み方（余白の値そのもの）

## Upstream / Downstream
- **Upstream**: なし
- **Downstream**: 小さい面のバルーンを持つすべてのゴースト

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `emo-text-canon-residue`（項目 14）・`balloon-canon-residue`（項目 14）

## Constraints
- ログ無しの失敗にしない（error! か warn! を 1 回は出す）。
- 規模の見立て: S（4〜7）。
