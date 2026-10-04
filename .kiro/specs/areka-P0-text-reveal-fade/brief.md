# Brief: areka-P0-text-reveal-fade

> 2026-10-01 `/kiro-discovery` で起票（開発者指示「文字のタイプライター表現において、フェードインしながら文字を表示するモードが欲しい。よくノベルゲームで見ますよね」）。roadmap「文字の現れ方」節。**正典に無い areka 独自の演出**。普通のバルーンにもシェル内バルーンにも効く。本文の file:line は起票時（main `9cf09f5d`）の実測＝着手時に引き直すこと。

## Problem

- **ゴーストの作者**: 1 字ずつの表示で字がいきなり現れる表現しか選べない。ノベルゲームで定番の「字がふわっと現れる」見せ方ができない。

## Current State

- 1 字ずつの表示は、台詞の時計に対して字ごとの「現れる時刻」を持ち、その時刻を過ぎた字を見せるだけ（`crates/areka-emo-text/src/state.rs` の `visible`＝現れる時刻の列への `partition_point`）。単位は書記素クラスタ（`balloon-color-emoji` 完了）。
- 描画は字が増えたときだけ描き足す作りで、一度描いた字は描き直さない（文字の層は自前のスワップチェーン・`surface.rs`）。**薄れている途中の字を毎コマ描き直す口が無い**。
- 透明度を時間で変える仕組みは無い。`balloon-scroll-fade`（起票済み・押し出される行が薄れて消える）が同じ仕組みを要る。

## Desired Outcome（2026-10-01 開発者確定）

- **字が、現れる時刻から決めた長さをかけて透明から不透明へ変わる**モード。既定は無効（今と同じ＝いきなり現れる）。
- **有効にする書き方は 2 つ**:
  - バルーンのキーで既定を決める（`balloon.*`ブレスにも書ける）。例 `text_reveal,fade 150ms`。
  - 台本で切り替える。例 `\![text,reveal,fade,150]`。効くのは台詞の終わりまで（正典の `\![set,balloonwait]` と同じ持続）。`\![text,…]` は areka の文字組みの指示の入り口（ルビ・縦中横と同じ枠）。
- **既定のフェードの長さは 150ms**（`text_reveal,fade` とだけ書いたとき）。1 字の表示の間隔（今は 50ms）より長く、2〜3 字が重なって薄れていく見え方にする。
- **早送り（`talk-fast-forward`）のクリックでは、フェード中の字もまだの字も即座に不透明にする**。`\_q`（瞬間表示の区間）の中もフェードせず即座に見せる。
- 選択肢・ルビ・縦中横・絵文字も同じ規則で現れる（ルビは親文字と同時）。

## Approach

- 透明度は「今の時刻 − その字の現れる時刻」と長さから決まる純関数にする（台詞の時計の上で決定論的・模擬時刻で確かめられる）。
- 描画は「フェード中の字の範囲だけを毎コマ描き直す」。フェードが終わった字は今までどおり描いたまま。同時にフェード中の字は数文字なので重さはわずかと見込むが、1 コマの予算を確かめる（`recompose-budget` の先例）。
- 字の透明度を時間で変える仕組みは `balloon-scroll-fade` と共用する。先に着地した方が作り、後の方が使う。

## Scope

- **In**: キー `text_reveal` と台本 `\![text,reveal,…]`、透明度の純関数、フェード中の字の描き直し、早送り・`\_q` との合わせ、縦書きと横書き、普通のバルーンとシェル内バルーン、1 コマの予算の確かめ、決定論テスト、`doc/COMPAT_ARCHITECTURE.md` §8 への areka 独自の語の登記。
- **Out**: 字が動く演出（`text-reveal-dance`＝夢）・押し出される行のフェード（`balloon-scroll-fade`）・`\c` で消えるときのフェード。

## Boundary Candidates

- 透明度の決まり（純粋層・`state.rs` の周り）と、描き直しの範囲の判断（ダーティ）と、描画（`draw.rs`／`surface.rs`）。

## Out of Boundary

- 1 字ずつの表示の時刻の決め方（`\w`・`balloonwait`・`\_q`）は変えない。

## Upstream / Downstream

- **Upstream**: α の完成宣言。早送りとの合わせは `talk-fast-forward` が先に着地していればそちらの口を使い、後なら向こうが本 spec の口を使う（機能の前提ではない）。
- **Downstream**: `text-reveal-dance`（同じ「現れる時刻からの経過」で動きを決める）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `balloon-scroll-fade`（透明度の仕組みを共用・同じ描画の部分を触る＝直列）・`talk-fast-forward`・`sakura-time-directives`（`\_q`・`balloonwait`）・`text-ruby`。

## Constraints

- 既定で今の出力と同じ（既存テストが無改変で緑）。1 フレーム遅らせる解は取らない。1 ファイル 1,000 行（`areka-emo-text` の大きいファイルは新しいファイルで足す）。決定論テスト網羅は必達。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 M（8〜12 タスク）。文字まわりの直列の列。
- **brief の誤り**: 「一度描いた字は描き直さない」の実体は `surface.rs` ではない。判断は見える範囲の計画（`ScrollPlanner`・`DirtyRect`）と `viewbox_draw` 系の `render_styled` が持ち、`actor` 系の `present_actor` から動く。触るのは `state.rs`・`actor` 系・`viewbox_draw` 系・`viewbox_draw_plan.rs`・場合により `viewbox` 系。
- **`\_q` は実装が無い**（パーサにも compile にも腕が無い・担当は `sakura-time-directives`）＝「`\_q` の中は即座」はつなぐ先が無い。口だけ用意し、後から着地する側がつなぐ。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M（9〜12 タスク）。切らない。
- 前提の状態: 機能の前提は無く、満たす（`emo-text-file-split`・`shell-balloon` 着地済み）。列の上では `text-ruby` の後・`balloon-scroll-fade` の前。早送り（`talk-fast-forward`）は列で先に着地する予定なので、その口を使う側になる見込み。`\_q` は今もパーサ・compile に腕が無い（担当 `sakura-time-directives`・据え置き）＝口だけ用意する、は棚卸⑳のまま。
- 崩れた前提／古くなった位置:
  - 分割での移り先: 1 コマの流れ `present_actor` は `actor_present.rs`、描き直す範囲の導出（`derive_dirty`・`derive_dirty_with_overhangs`）は `viewbox_diff.rs`、描画（`render_styled`）は `viewbox_draw_render.rs`、統計と描画の設定は `viewbox_draw.rs`。現れる時刻の列は `state.rs` の `TextLayerState::visible`（`partition_point`）のまま。
  - **シェル内バルーンの箱にも効く**: 箱は同じ `present_actor` と描画を通る（`actor_present.rs` の `TextPlace::Box` の腕は面の挿し場所が違うだけ）。キー `text_reveal` は箱の定義（`balloon.名前`ブレス）も `areka-emo-compose/src/boxes.rs` で同じ `balloon::parse` を通るので書ける＝バルーン定義ごとの値（`ResolvedBalloonText` の側）に載せること。
  - 箱だけの論点: 箱の面は「表示されている字の矩形」でポインタを受ける（`actor_present.rs` の末尾で `glyph_cells` → `set_hit_cells`）。薄れている途中の字を当たりに入れるか（今の規則のままなら、現れる時刻を過ぎた字は透明度に関わらず当たる）を決める。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{state.rs（透明度の純関数・`CueCommand::Custom` の腕で `\![text,reveal,…]`）, actor.rs（ResolvedBalloonText）, actor_present.rs, viewbox_diff.rs, viewbox_draw_render.rs, viewbox_draw.rs, viewbox_draw_plan.rs}`＋新規（透明度の仕組み・`balloon-scroll-fade` と共用）＋`lib.rs`
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs}`（`text_reveal`）
  - `doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: 箱の当たりの矩形に、薄れている途中の字を入れるか。
- 見つけた穴: なし。
