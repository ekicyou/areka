# Brief: areka-P0-text-reveal-dance

> 2026-10-01 `/kiro-discovery` で起票（開発者指示「より夢のある仕様では、ニンテンドーのゲームみたいに、文字が踊りながら表示されるエフェクトも欲しいけど、これは Option の夢 spec として別に切ってください」）。roadmap「文字の現れ方」節。**夢・任意**＝着手の順に縛られない。**正典に無い areka 独自の演出**。本文の file:line は起票時（main `9cf09f5d`）の実測＝着手時に引き直すこと。

## Problem

- **ゴーストの作者**: 字が現れる瞬間に動きをつけた、遊び心のある見せ方（任天堂のゲームの台詞のような）ができない。

## Current State

- 1 字ずつの表示は字ごとの「現れる時刻」を持つ（`crates/areka-emo-text/src/state.rs` の `visible`）。描画は字が増えたときだけ描き足し、一度描いた字は描き直さない（文字の層は自前のスワップチェーン）。**字の位置・回転・大きさを時間で変えて描く口は無い**。
- 透明度を時間で変える仕組みは `text-reveal-fade` が作る。

## Desired Outcome（2026-10-01 開発者確定）

- **字が現れるときにだけ動き、決まった動きを終えたら定位置で止まる**。例:
  - **跳ねる**: ポーンと跳ねて定位置に戻る。
  - **揺れる**: 現れるときに小さく揺れて落ち着く。
- **ずっと動き続ける演出は作らない**（開発者裁定）。負荷が大きく、ずっと動いていたら目に毒。動きの長さは有限で、終わった字は今までどおり描いたまま（描き直さない）。
- 有効にする書き方は `text-reveal-fade` と同じ枠（バルーンのキー `text_reveal` の値の種類として足す・台本 `\![text,reveal,…]`）。フェードと組み合わせられるか（跳ねながら薄く現れる）は要件で決める。
- 早送りのクリックと `\_q` の中では動かさず、即座に定位置へ出す（`text-reveal-fade` と同じ規則）。

## Approach

- 動きは「現れる時刻からの経過」だけで決まる純関数（位置のずれ・回転・大きさの組）にする。台詞の時計の上で決定論的。
- 描画は「動いている途中の字だけを、その字の箱より広い範囲（動きの振れ幅を含む）で毎コマ描き直す」。動き終えた字は焼き付けたまま。隣の字に重なる動きをどう描き直すか（重なりの範囲の計算）が設計の山。
- 動きの曲線（跳ね・減衰する揺れ）は少数の決め打ちから始める。作者が曲線を自由に書ける仕組みは要るようになってから。

## Scope

- **In**: 跳ねる・揺れるの 2 種、`text_reveal` の値の追加、動いている字の描き直し、早送り・`\_q` との合わせ、縦書きと横書き、1 コマの予算の確かめ、決定論テスト。
- **Out**: ずっと動き続ける演出（波打つ・震え続けるなど）・強調語だけを動かす範囲指定（要るなら別に検討）・作者が動きの曲線を書く仕組み。

## Boundary Candidates

- 動きの決まり（純粋層）と、描き直しの範囲（振れ幅を含む）と、描画の変換。

## Out of Boundary

- フェードそのもの（`text-reveal-fade`）。

## Upstream / Downstream

- **Upstream**: `text-reveal-fade`（現れ方の枠＝キー・台本・早送りとの合わせ・描き直しの口を先に作る）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `text-reveal-fade`・`balloon-scroll-fade`・`text-ruby`（ルビの付いた字の動き）。

## Constraints

- 既定で今の出力と同じ。動きは有限で、終わった字は描き直さない（常時の負荷を増やさない）。1 ファイル 1,000 行。決定論テスト網羅は必達。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 夢・任意のまま（`text-reveal-fade` の後・開発者が望んだときだけ）。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M（8〜11 タスク）。切らない。夢・任意のまま。
- 前提の状態: `text-reveal-fade`（現れ方の枠＝キー・台本・描き直しの口）が未着手＝未。
- 崩れた前提／古くなった位置:
  - brief の「文字の層は自前のスワップチェーン」は、描き直しの判断の持ち主としては不正確（`text-reveal-fade` の棚卸⑳と同じ）。描き直す範囲は `viewbox_diff.rs`（`derive_dirty_with_overhangs`・はみ出しの帯の計算）、描画は `viewbox_draw_render.rs`（`expand_overhang_for_band` もここ）、1 コマの流れは `actor_present.rs`。動きの振れ幅を含む描き直しは、既存の「はみ出しの帯」（`LineOverhang`・`DIRTY_GUARD_IMG_PX`＝`viewbox.rs`）の考え方に乗せられる見込み。
  - シェル内バルーンの箱にも同じ道で効く。箱は字の矩形でポインタを受ける（`actor_present.rs` の `glyph_cells`）ので、動いている途中の字の当たりは定位置の矩形のままでよいか（`text-reveal-fade` と同じ論点）。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{state.rs, viewbox.rs, viewbox_diff.rs, viewbox_draw_render.rs, actor_present.rs}`＋`text-reveal-fade` が作る透明度・現れ方の新しいファイル
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs}`（`text_reveal` の値の追加）
- 議題（答えで作業が変わるものだけ）: なし（フェードとの組み合わせの可否は要件で決める＝brief のまま）。
- 見つけた穴: なし。


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（8〜11 タスク）のまま。切らない。夢・任意（据え置き）のまま。
- 前提の状態: `text-reveal-fade`（現れ方の枠・描き直しの口）は今も未着手＝未。
- 崩れた前提／古くなった位置:
  - C3 で `state.rs`・`viewbox_diff.rs`（`derive_dirty_with_overhangs`・`LineOverhang`）・`viewbox_draw_render.rs`（`expand_overhang_for_band`）・`actor_present.rs`（`glyph_cells`）は無変更。`viewbox.rs` は注記の言い換え（「M2」→「α 後」）だけで、`DIRTY_GUARD_IMG_PX` の考え方はそのまま＝棚卸㉑の見立てが当たる。
  - 箱の写し（`actor_box.rs` の `refresh_shown_boxes`）が「見えている字の数」で箱を数えるのは `text-reveal-fade` と同じ論点。動いている途中の字の当たりは `text-reveal-fade` の答えに揃える。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{state.rs, viewbox.rs, viewbox_diff.rs, viewbox_draw_render.rs, actor_present.rs}`＋`text-reveal-fade` が作る新しいファイル
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs}`
- 議題: なし。
- 見つけた穴: なし。
