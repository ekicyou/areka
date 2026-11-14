# Brief: balloon-text-visual-tree

> 2026-11-14 `wintf-visual-damage-redraw` の完了の棚卸で `/kiro-discovery` で起票（同 spec の `requirements.md` の Out of scope「文字の層（`areka-emo-text`）のスクロール」）。
>
> 出どころ（2026-11-14 開発者・同 spec の要件の討議）: 「本来、バルーンが使うはず。使ってないなら使うべきですね」＝バルーンの文字の層を wintf の描画の木（文字の区画を自分の描画面に置く形）と、`wintf-visual-damage-redraw` の平行移動の写しへ寄せる。

## Problem

- バルーンの文字の層は、wintf の描画の木の外で自前のスワップチェーンを持ち、スクロールの写しも自前で書いている。`wintf-visual-damage-redraw` が描画の木の側に同じ仕組み（範囲だけの描き直しと平行移動の写し）を入れたので、同じことを 2 か所で持つことになった。
- 文字の層が描画の木の外にあるので、描画の木の働き（不透明度の層・混ぜ方・切り抜き・変形・入れ物の読み戻し）が文字には効かない。後に来るフェード（`balloon-scroll-fade`・`text-reveal-fade`）や切り抜き（`element-clipping-option`）が、文字の層だけ別の作りを要る。
- wintf の供給面の作り手（`crates/wintf/src/com/dxgi.rs` の `create_composition_swap_chain`）の本番の使い手は文字の層だけ。

## Current State（2026-11-14・main）

- 文字の面は `crates/areka-emo-text/src/surface.rs`（960 行）の `TextSurface`。`wintf::com::dxgi::create_composition_swap_chain`（flip model・α 掛け済み・B8G8R8A8・2 枚）＋`CompositorInteropExt::create_composition_surface_for_swap_chain` で自前のスワップチェーンを作り、WUC の `SpriteVisual` に brush で付ける。描き面は D3D11 のテクスチャ 2 枚（front・back）で、スクロールは `copy_front_to_back_shifted`（`CopySubresourceRegion` で front を整数の物理ピクセルだけずらして back へ写す）＋`flip`、提示は `present`（`CopyResource` → `Present(0)`）。
- 付け先: バルーンの窓の文字の差し込み口（`crates/areka-emo-present/src/mount.rs` の `text_slot`・名前 `text-layer-slot`）。箱の文字の面はシェルの窓の直接の子（`crates/areka-emo-text/src/surface_window_child.rs`・当たりのマスクもここで焼く）。差し込み口への付け外しは `crates/areka-emo-text/src/actor_present.rs`（872 行・`TextSurface::attach`／`attach_window_child`）、areka 本体の結線は `crates/areka/src/runtime/frame/{attach,wiring,scale_text,drain_resnap}.rs`。
- 描きの計画: `crates/areka-emo-text/src/viewbox_draw_plan.rs`（変化なし＝写しも描きも提示も 0／写し＋汚れた矩形だけの描き）と `viewbox_draw_render.rs`（835 行）。スクロールの面に描かない固定の層（`surface.rs` の 240 行目付近）と、印の面（`marker_surface.rs`）が別に在る。
- 字形は DirectWrite と D2D で描く（`draw.rs` 804 行・`draw_line_store.rs`・`draw_metrics.rs`）。
- wintf の側: 描画の木は `crates/wintf/src/ecs/graphics/visual/`、描画面ごとの合成の入れ物は `ecs/graphics/composite.rs`、範囲だけの描き直しと平行移動の写しは `wintf-visual-damage-redraw`（`visual/draw/README.md`「差分の描き直し」「写し」・記録は target `wintf::damage` の 1 行）。

## Desired Outcome

- バルーンの文字は wintf の描画の木の節として描かれ、文字の層の自前のスワップチェーンは無くなる。
- 文字の区画は自分の描画面を持つ（区画の持ち主を描画面の持ち主の直下に置き、`Arrangement` のずれで動かす）。スクロールは、その描画面の中で `wintf-visual-damage-redraw` の平行移動の写しで済み、露出した帯だけを描く（`wintf::damage` の行の `shift_area`・`exposed_area` で確かめる）。
- 見た目は今の文字の層と画素で一致する（決定論のテスト・WARP）。

## Approach

**推す形**（要件の段で覆してよい）:

1. **文字の区画を描画の木の節にする**: バルーンの窓の描画面の下に、文字の区画の持ち主（自分の描画面を持つ Visual）を置き、字形・下線・選択肢の飾りを描く列の項として出す。区画の位置は `Arrangement` のずれで動かす。
2. **スクロールは wintf の写しに任せる**: 文字の層の `copy_front_to_back_shifted`・`flip`・自前の汚れた矩形の計画は、描画の木の比べと写しに置き換える。行の送りは今と同じく物理ピクセルの整数に置く。
3. **固定の層・印の面・箱の文字の面**: それぞれ描画の木のどこに置くかを要件で決める（固定の層はスクロールの区画の外の節、印の面は別の節が見込み）。
4. **答えを固定する**: 今の文字の層の読み戻しと、移した後の入れ物の読み戻し（`SurfaceReadback`）が画素で一致することを WARP で確かめる。

**採らない方向**:

- 文字の層に自前のスワップチェーンを残したまま、wintf の写しだけを呼ぶ形（描画の木の外のままなので、Problem の 2 点目が残る）。

## Scope

- **In**: 文字の区画の描画の木への移し（バルーンの文字・固定の層・印の面・箱の文字の面）・スクロールを平行移動の写しへ寄せること・自前のスワップチェーンと写しの作りを消すこと・画素の一致の決定論のテスト。使い手が 0 になった `create_composition_swap_chain` を消すか残すか（要件で決める）。
- **Out**: 字の並べ方・折り返し・選択肢の当たり（今のまま）・DirectWrite の下回りの差し替え（`windows-text` の採用）・フェード（`balloon-scroll-fade`・`text-reveal-fade`）・描画面の自動の昇格・降格（`wintf-surface-auto-promotion`）。

## Boundary Candidates

- 文字の層の描き先（`surface.rs`・`surface_window_child.rs`・`viewbox_draw*.rs`）＝描画の木の項を出す側へ。
- 差し込み口と結線（emo-present の `mount.rs`・areka の `runtime/frame/`）。
- wintf の描画の木（使う側だけ。写しの決まりそのものは変えない）。

## Out of Boundary

- `wintf-visual-damage-redraw` の比べ・範囲・写しの決まりそのもの（足りない所が見つかったら wintf 側の spec へ申し送る）。
- バルーンの枠の絵（emo-compose の合成）。

## Upstream / Downstream

- **Upstream**: `wintf-visual-damage-redraw`（平行移動の写し・完了の手続き中）・✅ `wintf-visual-tree`・✅ `wintf-d2d-composite-rules`（入れ物と読み戻し）。
- **Downstream**: `balloon-scroll-fade`・`text-reveal-fade`（不透明度の層で文字を薄くできる）・`element-clipping-option`・`balloon-inline-image`（行の中の絵）・`compose-f16-precision`（文字の層の 8 ビットの描き先が無くなる）・`pasta-idle-surface-cost-measure`（WorkingSet の内訳が変わる）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。
- **Adjacent**: `draw-methods-canon`（emo-present の表示を触る）・`wintf-surface-auto-promotion`（`visual/systems.rs`・`render.rs`・節の下の描画面の持ち主）・`emo-text-windows-text`（同じ `areka-emo-text` の DirectWrite の下回り・v0.2.0）・`balloon-*` の各 spec（文字とバルーンの列）・`emo-text-canon-residue`。

## Constraints

- **写しが効く条件**（`crates/wintf/src/ecs/graphics/visual/draw/README.md`「写し」）を文字の区画の形で満たす:
  - スクロールの量は物理ピクセルの整数。拡大を掛けて整数にならない量（例: 拡大 1.25 で 1 行が 15 論理ピクセル）では写さず、範囲だけの描き直しになる。
  - その描画面に根の命令の列を置かない（区画の描画面は字の項だけにする）。
  - 字の項に重なる絵を置かない。
  - 行の範囲がにじみの余白込みで重ならない。重なると露出した帯が行から行へ広がり、写しは捨てられる（答えは一致するが、全部描くのと同じ量になる）。
  - 字形はベクタで描くので、縮める絵の件（`HIGH_QUALITY_CUBIC`）の範囲の広げ方は当たらない。行の中に絵（`balloon-inline-image`）を置くときだけ当たる。
- 見た目は今と画素で一致する（ずれは欠陥）。決定論のテストは必達・WARP。
- 1 フレーム遅らせる解は取らない・時刻は正確に扱う・記録の無い失敗の道を作らない。
- 1 ファイル 1,000 行の番人を越えない（`surface.rs` は 960 行・`viewbox_draw_render.rs` は 835 行）。

## 2026-11-14 起票時の見立て

- 触るファイル: `crates/areka-emo-text/src/{surface.rs, surface_window_child.rs, marker_surface.rs, viewbox_draw*.rs, actor_present.rs, actor_scroll.rs, draw.rs, lib.rs}` と兄弟のテスト・`crates/areka-emo-present/src/mount.rs`・`crates/areka/src/runtime/frame/{attach,wiring,scale_text,drain_resnap}.rs`・`crates/areka/src/mcp/dump_balloon.rs`（文字の差し込み口を読む）。使い手が 0 になれば `crates/wintf/src/com/dxgi.rs`。
- 規模: L（16〜20）。超えそうなら要件の段で箱の文字の面と印の面を切る。
- 先に要るもの: `wintf-visual-damage-redraw` の着地。
- 優先度の区分: A（開発者 2026-11-14「本来、バルーンが使うはず。使ってないなら使うべきですね」）。
- 版: v0.2.0（wintf の基幹の整備・`wintf-visual-damage-redraw` の後）。次の棚卸で確かめる。
- 席: 文字とバルーンの列（emo-text の席を使う）。`emo-text-windows-text`・`draw-methods-canon`・`wintf-surface-auto-promotion` とは同時に走らせない見込み（触るファイルは設計で確かめる）。
- 要件定義のモデル: Opus。
- 分割の案: 上限を超えるなら、箱の文字の面（`surface_window_child.rs`）と印の面（`marker_surface.rs`）を後の 1 本に切る。
