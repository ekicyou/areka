# Brief: areka-P0-balloon-scroll-fade

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。**正典に無い areka 独自の演出**（開発者「正典にはない項目ですし、独立 spec かも」）。普通のバルーンにも効く。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- シェル内バルーンの小さな箱（参考ゴーストの右の台詞欄は 2 列）では、あふれた列が自動スクロールでいきなり押し出されて消える。参考資料は「古い列のフェード」を挙げている。

## Current State

- あふれは正典どおりの自動スクロールだけ。行（縦書きなら列）単位で即時に動き、最新の行が常に見える（`crates/areka-emo-text/src/layout.rs` の visible window・`viewbox.rs` の整数 px のずらし描き）。

## Desired Outcome（2026-10-01 開発者確定）

- **あふれの基本は正典の自動スクロールのまま**（書き手が区切りたいところは `\x` と `\c`）。
- そのうえで、バルーンのキー（areka 独自・名前は要件で決める。CSS に当たる語が無いので areka の語で付ける）で有効にすると、**押し出される行（列）が消える前に薄れていく**演出を足す。既定は無効＝今と同じ。
- **自動の改ページ（クリックで次へ）は作らない**（開発者裁定: クリック待ちは台本に明示した `\x` だけ）。

## Approach

- 押し出しの瞬間に、消える行を別の層として残し、決めた時間で透明度を下げて消す（台詞の時計の上で決定論的に）。スクロールそのものの規則は変えない。

## Scope

- **In**: フェードのキー（有効・時間）、縦書きと横書き、シェル内バルーンと普通のバルーン、手動スクロール（`balloon-markers`）との合わせ、決定論テスト、`doc/COMPAT_ARCHITECTURE.md` §8 への登記。
- **Out**: 自動の改ページ・スクロールの規則の変更・押し出し以外の消え方（`\c` で全部消すときのフェードなど＝要るなら要件で検討）。

## Boundary Candidates

- 押し出しの検出（layout の visible window の変化）と、薄れる層の描画。

## Out of Boundary

- 矢印と手動スクロール（`balloon-markers`）。

## Upstream / Downstream

- **Upstream**: `balloon-markers`（同じスクロールの部分を触る＝直列）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `balloon-markers`・`text-align-shadow-canon`（あふれの判定が最新の行の遠辺だけを見る件）。

## Constraints

- 既定で今の出力と同じ（既存テストが無改変で緑）。1 ファイル 1,000 行。決定論テスト網羅は必達。1 フレーム遅らせる解は取らない。

---

> **📌 2026-10-01 `/kiro-discovery`（文字の現れ方）からの注記**——新 spec `areka-P0-text-reveal-fade`（字が透明から不透明へ変わる現れ方）が、本 spec と同じ「字の透明度を時間で変える仕組み」と「薄れている途中の字だけを毎コマ描き直す口」を要る。**2 本は同時に走らせず、先に着地した方が作り、後の方が使う**（roadmap「文字の現れ方」節）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 文字まわりの直列の列（`balloon-markers` の後）。透明度を時間で変える仕組みは `text-reveal-fade` と共用＝先に着地した方が作る。棚卸⑳では個別の再測定をしていない。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S〜M（6〜9 タスク）。切らない。透明度を時間で変える仕組みを `text-reveal-fade` が先に作る（列で前）なら S。
- 前提の状態: `balloon-markers`（同じスクロールの部分・手動スクロール）が未着手＝未。`text-reveal-fade` も未（列で前・仕組みを共用）。
- 崩れた前提／古くなった位置:
  - 分割での移り先: 見える範囲は `layout.rs` の `LayoutEngine::visible_window`、押し出しの計画は `viewbox.rs`（`ScrollPlanner` の前半・型）と `viewbox_diff.rs`（描き直す範囲の導出）、描画は `viewbox_draw_render.rs`、1 コマの流れは `actor_present.rs`。
  - **シェル内バルーンの箱が本来の使い手**（参考ゴーストの 2 列の欄）。箱は同じ見える範囲・計画・描画を通る。キーはバルーン定義ごとの値（`ResolvedBalloonText` の側）に載せれば `balloon.名前`ブレスにも書ける（箱の定義も `balloon::parse` を通る）。
  - 箱だけの論点: 箱の面は表示されている字の矩形でポインタを受ける（`actor_present.rs` の `glyph_cells` → `set_hit_cells`）。薄れている途中の押し出された行を当たりに入れるか。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{layout.rs, viewbox.rs, viewbox_diff.rs, viewbox_draw_render.rs, viewbox_draw_plan.rs, actor_present.rs, actor.rs}`＋透明度の仕組み（`text-reveal-fade` が作る新しいファイル）
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs}`（areka 独自のキー）
  - `doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: 箱の当たりの矩形に薄れていく行を入れるか（`text-reveal-fade` と同じ答えにする）。
- 見つけた穴: なし。


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S〜M（6〜9 タスク）のまま。切らない。
- 前提の状態: `balloon-markers`（手動スクロール・同じスクロールの部分）と `text-reveal-fade`（透明度の仕組み）は今も未着手＝未。
- 崩れた前提／古くなった位置:
  - 見える範囲・押し出しの計画・描画の各ファイル（`layout.rs` の `LayoutEngine::visible_window`・`viewbox.rs`・`viewbox_diff.rs`・`viewbox_draw_render.rs`・`actor_present.rs`）は C3 で無変更＝棚卸㉑の位置がそのまま当たる。`viewbox.rs` の変更は注記の言い換え（「M2」→「α 後」）だけ。
  - 箱の置き場所は `shell-balloon-frame-align` で「いま表示している絵の番号」から決まるようになった。替わった絵で箱の置き場所が変わる（または箱が無くなる・絵が隠れる）と、次の同期が箱の登録を外して面を片付け、作り直す（`actor_box.rs` の `sync_box_bindings` → `unregister_box`）＝押し出された行が薄れている途中にそうなれば、薄れる層ごと消える。その扱い（消えてよい、で足りる見込み）を要件で一言決める。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{layout.rs, viewbox.rs, viewbox_diff.rs, viewbox_draw_render.rs, viewbox_draw_plan.rs, actor_present.rs, actor.rs}`＋`text-reveal-fade` が作る透明度の新しいファイル
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs}`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題: 棚卸㉑のまま（箱の当たりに薄れていく行を入れるか・`text-reveal-fade` と同じ答え）。
- 見つけた穴: なし。
