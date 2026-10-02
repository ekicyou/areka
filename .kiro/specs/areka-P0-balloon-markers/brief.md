# Brief: areka-P0-balloon-markers

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。開発者指示「未実装だった項目も、実装が必要な spec として今回の最終作成 spec に含めて」。普通のバルーンにもシェル内バルーンにも効く**正典の実装**。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- シェル内バルーンは「バルーンの中身はすべて互換」と決めた（`shell-balloon`）。ところがバルーンの descript.txt の印の類は、**普通のバルーンでもまだ 1 つも動いていない**。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` で次がすべて `absent`:
  - `arrow.filename`・`arrow0/1.x/y`（スクロールできるときの矢印）
  - `onlinemarker.filename/x/y/interval`（通信中の印）
  - `sstpmarker.filename/x/y`（SSTP で来た台詞の印）
  - `number.font.*`・`number.xr/y`（ファイル受信の進み具合・`\![set,balloonnum,ファイル名,現在,最大]`＝これも `absent`・**引受先なし**）
  - `sstpmessage.font.*`・`sstpmessage.x/xr/y`（SSTP の送り主の表示・**引受先なし**）
- 印の画像の系列（`arrow*.png`・`sstp*.png`・`online*.png` ほか）の解決も無い（`balloon-canon-residue` 項目 1）。

## Current State

- バルーンの descript.txt の読み手（`crates/areka-parsers/src/balloon/parse.rs`）は上のキーを持っていない（着手時に確かめる）。系列の解決は `crates/areka-emo-present/src/balloon.rs` の `SeriesFamily`（本体の系列だけ。複数の旧名の配列は設計済み＝装飾の系列は表の行の追加で乗る）。
- スクロールは自動だけ（最新の行が常に見える）。手動で戻す手段（矢印のクリック・ホイール）は無い（`crates/areka-emo-text/src/viewbox.rs`・`layout.rs` の visible window）。
- 通信: ネットワーク更新（`network-update` 完了）は通信の始まりと終わりを持つ＝印のきっかけはある。SSTP の受信は無い（roadmap「α 後」の予約・本起票で登記だけの行を立てた）。

## Desired Outcome（2026-10-01 開発者確定）

- **矢印と手動スクロール**: 押し出された文字が残っているとき `arrow0`（上／縦書きでは右）・`arrow1`（下／縦書きでは左）の印を出し、矢印のクリックとホイールで手動スクロールできる。縦書きでの矢印の意味は ukadoc `vertical` の項（右・左へのスクロール）に従う。
- **通信中の印**: ネットワーク更新の通信中に `onlinemarker` を出す（`interval` で点滅）。
- **ファイル受信の進み具合**: `\![set,balloonnum,ファイル名,現在,最大]` と `number.*` の表示。
- **SSTP の 2 項目は表示だけ作る**（開発者確定 (a)）: `sstpmarker`・`sstpmessage` を描けるようにし、「SSTP で届いた台詞」という目印と送り主の名前を受け取る口だけ用意する。試験では目印を直接渡して確かめる。**実際に画面に出るのは SSTP の受信が入ってから**（登記だけの行「SSTP の受信」）。
- **印の画像の系列**: `SeriesFamily` に装飾の系列の行を足す（旧名の段・スコープごと）。`clickwait*` の系列も同じ仕組みで解けるようにする（印を出すのは `talk-fast-forward`）。
- シェル内バルーンでも同じ印が同じ規則で出る（位置は箱の左上を (0,0) として読む）。

## Approach

- 印は文字の層の上に重ねる小さな絵として扱い、位置のキーは箱（普通のバルーンなら絵の大きさ）に対して読む（負の値は反対側の端から）。
- SSTP の目印は台詞の付帯情報（出どころ）として台本の入り口に持たせ、今は常に「SSTP でない」を流す縮退の口にする（完全語彙＋縮退の口＋追跡の行＋roadmap 明記）。

## Scope

- **In**: 上の 5 群のキーと `\![set,balloonnum]`、印の画像の系列（`balloon-canon-residue` 項目 1 を引き取る）、手動スクロール、網羅台帳の更新、決定論テスト。
- **Out**: SSTP の受信そのもの・`\x` と `clickwaitmarker` の振る舞い（`talk-fast-forward`）・押し出しのフェード（`balloon-scroll-fade`）・面の偶奇と左右向き・`defaultsurface`・`\![reload,balloon]`・`balloonc*`・多面の検体（`balloon-canon-residue` の項目 2〜6 のまま）。

## Boundary Candidates

- 系列の解決（`areka-emo-present`）と、印の配置と描画（`areka-emo-text` または present の重ね）と、きっかけの配線（更新の通信・`balloonnum`・SSTP の縮退の口）と、手動スクロール（`viewbox`）。

## Out of Boundary

- 自動スクロールの規則そのもの（正典どおり・変えない）。

## Upstream / Downstream

- **Upstream**: `shell-balloon`（シェル内バルーンの箱の上でも同じ規則にするため、箱の形が決まってから）。数字の表示のフォントは `balloon-font-file` の読み手を通る（機能の前提ではない）。
- **Downstream**: `balloon-scroll-fade`（同じスクロールの部分を触る＝直列）。SSTP の受信（登記だけの行）が入ると SSTP の 2 項目が実際に出る。

## Existing Spec Touchpoints

- **Extends**: `balloon-canon-residue`（項目 1「装飾の系列」を本 spec が引き取る＝同 brief へ追記済み）。
- **Adjacent**: `talk-fast-forward`（`clickwait*` の系列を共用）・`network-update-canon-order`（通信のイベントの順序）。

## Constraints

- `viewbox.rs` 871 行・`layout.rs` 977 行＝新しいファイルで足す。1 ファイル 1,000 行。
- 決定論テスト網羅は必達。印の画像の検体はリポジトリ内に作る（既定バルーン `StayseeBalloon` が装飾の系列を持つかは着手時に確かめる）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 M〜L（14〜19 タスク）。文字まわりの直列の列（`shell-balloon` の後）。
- 合っていた点: `SeriesFamily`（emo-present の `balloon.rs`）は旧名の列を積める・パーサにこれらのキーは無い。既定バルーン（Staysee）には `arrow0/1.png`・`online0〜8.png`・`sstp.png`・`marker.png` が在り、`clickwait*` と数字の画像は無い。
- **触るファイル**: `crates/areka-parsers/src/balloon/{model.rs 774, parse.rs}`（キーが多く 850 行を超えうる＝新しいファイルへ）・emo-present の `balloon.rs`・emo-text の `viewbox` 系・`layout` 系（`visible_window`）・`actor` 系・`state.rs`（`balloonnum`）・`crates/areka/src/input_events/` のバルーン（矢印のクリックとホイール）・通信のきっかけの配線（`emo2_boot/update_cue.rs` か `crates/areka/src/update/`）・台帳。
