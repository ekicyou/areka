# Brief: areka-P0-underline-bottom-row-clip

> 起票: 2026-10-10（`areka-P0-anchor-tag-canon` の実機の確かめ〔タスク 7.2〕で発見。範囲の外の問題として、完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-anchor-tag-canon/tasks.md` の Implementation Notes「7.2（範囲外・起票する）」と「7.2（結果）」。実機の撮影 2 枚で見た。原因はコードの読みで、コードを変えて確かめてはいない。区分 B（バグ）。

## Problem

- **利用者・ゴーストの作者**: バルーンの文字の領域（validrect）の下端に来た行では、下線が出ない。作者が `\f[underline,1]` で引いた下線も、アンカー（`\_a`）の下線も同じ。
- アンカーは、マウスが乗っていないときの目印が下線だけ（`doc/anchor-compat.md` の 2.1 の 8）。最下行のアンカーは、押せるのに、押せる所だと見分けられない。
- アンカーに固有の問題ではない。下線の描画は完了 `areka-P0-text-decoration-canon` が入れたもので、`anchor-tag-canon` より前からある。`anchor-tag-canon` でアンカーの既定の見た目が下線になり、利用者に見える場面が増えた。

## Current State

2026-10-10 時点（`anchor-tag-canon` のブランチ）:

- **見たこと**（実機・開発機 1 台・emo2 と同梱のバルーン `emo2-kakukaku`・文字の領域に 4 行が入る台詞）:
  - 4 行目（文字の領域の最下行）: 作者の `\f[underline,1]` の下線も、アンカーの下線も出ない（0 本）。
  - 3 行目: 両方出る。別の撮影では 1〜3 行目のアンカーの下線が出ている。
  - 撮影は 2 枚（MCP の `dump_balloon`）。画像は追跡外の作業用フォルダに置いたので残らない＝要件の段で撮り直す。
  - 最下行のアンカーは、下線が無くても押せた（実マウスで複数回・押下の記録 `anchor_selected` あり）。押せる範囲は描画と別に作るからである。
- **原因の見立て**（確度は高い。コードを変えて確かめてはいない）:
  - 下線の位置は DirectWrite に任せている。`crates/areka-emo-text/src/viewbox_draw_decoration.rs` の `apply_font_ranges` は、区間に `SetUnderline` の真偽を渡すだけで、線の位置を自分で決めない（完了 `text-decoration-canon` の要件 5.7）。
  - Yu Gothic UI の 28 画素では、下線は行の上端から 33 画素の所に出る（文字の大きさの箱 28 より 5 画素下・行送り 30 より 3 画素下）。
  - 文字の面は validrect ちょうどの大きさで作る（高さ 122。`actor_present.rs` の `present_actor` が `ScaleContract::physical_extent` で出す `physical_size`）。描き替える範囲も面の大きさで切る（`viewbox_diff.rs` の `expand_guard_clamp`）。
  - 4 行目の下線は 90 + 33 = 123 で、面（高さ 122）の外になる。途中の行の下線は次の行の帯に落ちるので見える。
  - 「あふれたか」は行の矩形（文字の大きさの箱）の下端で数える（`layout.rs` の `visible_window`）。4 行目の下端は 90 + 28 = 118 で、122 に収まると数えられ、スクロールは起きない。
- 最下行の下線を見る検査は無い（0 本）。
- 確かめていないこと: 拡大率が 100% でないとき・ほかのフォントと大きさ・縦書き（端の列で同じ形になるか）・シェルの中のバルーンの箱・打ち消し線（行の中に出るので当たらない見込み）。

## Desired Outcome

- 文字の領域のどの行でも、下線を引いた字には下線が見える。最下行も同じ。作者の下線とアンカーの下線の両方。
- 途中の行の見た目は変えない。行の位置と「何行入るか」は、選んだ直し方が求める分だけ変え、変えたら互換の記録に残す。
- 決定論のテストで「最下行の下線が面の中に描かれる」を固定し、実機で撮り直して確かめる。

## Approach

直し方は決めていない（**未決**）。候補は 3 つあり、要件・設計で選ぶ。

- **面を広げる**: 文字の面を、下線がはみ出す分だけ validrect より大きく作る。はみ出しの量をどこから得るか、面の位置・当たりの矩形・箱の写しがずれないかを先に確かめる。
- **下線を自分で決めた位置に描く**: DirectWrite 任せをやめ、行の箱の中に収まる位置へ線を引く。完了 `text-decoration-canon` の要件 5.7（線を描き分けない）を改めることになる。
- **入る行の数え方を変える**: 行の下端を、下線まで含めた高さで数える。最下行に下線が無い台詞でも、入る行が減りうる。

## Scope

- **In**: 文字の領域の端の行の下線が見えるようにする直し・それを固定する決定論のテスト・実機の撮り直し・決めを変えたときの互換の記録。
- **Out**: アンカーの見た目の作者指定と、縦書きの下線の位置の決め直し（`anchor-style-canon`）・選択肢の印の「本物の下線」（`choice-marker-styling`）・文字の描画範囲が 0 以下に潰れたときの扱い（`balloon-text-area-collapse`）・あふれのスクロールの見せ方（`balloon-scroll-fade`）。

## Boundary Candidates

- 下線の位置を誰が決めるか（DirectWrite か areka か）。
- 文字の面の大きさと validrect の関係（`actor_present.rs`・`viewbox_diff.rs`）。
- 入る行の数え方（`layout.rs` の `visible_window`）。

## Out of Boundary

- 押せる範囲の作り方（最下行のアンカーは今も押せる＝変えない）。
- 下線の太さ・色・形の作者指定。

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-text-decoration-canon`（下線の描画）・`areka-P0-anchor-tag-canon`（アンカーの既定の見た目を下線にした。互換の記録は `doc/anchor-compat.md`）。
- **Downstream**: `areka-P0-anchor-style-canon`（アンカーの下線の解決を差し替え、縦書きの下線の位置を決め直す。本 spec が下線の位置を自分で決める形を選ぶなら、その上に乗る）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `text-decoration-canon` の下線の決めを改めることがある。改めたら `doc/COMPAT_ARCHITECTURE.md` に記録する）。
- **Adjacent**: 文字とバルーンの列（`crates/areka-emo-text/src/` の `actor*.rs`・`viewbox_draw*.rs`）＝同じ列の spec と同時に走らせない。近いのは `anchor-style-canon`（`viewbox_draw_decoration.rs`・`viewbox_draw_render.rs`）・`balloon-text-area-collapse`（`actor_present.rs`）・`choice-ranges-one-function`（`actor_present.rs`）。

## Constraints

- 段は**バグ**。先に要るものは `anchor-tag-canon` の着地だけ（アンカーの下線で確かめるため。作者の下線だけなら今の main でも再現する見込み）。
- 要件で決める未知: 直し方の 3 案のどれにするか・下線のはみ出しの量を拡大率とフォントごとにどう得るか・縦書きと箱で同じ形が起きるか。
- 規模の見込み: S（4〜7 タスク。面を広げる案は当たりの矩形と箱の写しまで確かめるので上の側）。
- 1 ファイル 1,000 行未満・決定論のテスト必達・1 フレーム遅らせる解は取らない・GPU のテストは実 GPU で回す。
