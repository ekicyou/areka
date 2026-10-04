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


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: L（16〜20 タスク）＝20 の直前。箱で出す分（下記）を足すと超えうる＝**要件の段で超えたら切る**。切り方: ①「矢印と手動スクロール（`arrow*`・矢印のクリック・ホイール）＋装飾の系列の土台（`SeriesFamily` の行・`clickwait` を含む）」M（10〜12）→ ②「通信中・数字・SSTP の印（`onlinemarker`・`number.*`／`\![set,balloonnum]`・`sstpmarker`／`sstpmessage` の表示だけ）」S〜M（7〜9）。①が先（`balloon-scroll-fade` は①だけを待てばよい）。
- 前提の状態: `shell-balloon`（PR#227）着地済み＝満たす。
- 崩れた前提／古くなった位置:
  - 分割での移り先: 見える範囲の計算は `layout.rs` の `LayoutEngine::visible_window`、送りの計画は `viewbox.rs`（`ScrollPlanner` の前半）と `viewbox_diff.rs`（描き直す範囲）、描画は `viewbox_draw_render.rs`、1 コマの流れは `actor_present.rs`。矢印のクリックは `input_events/balloon_pressed.rs`、純関数の判定は `input_events/balloon.rs`（`hit_choice_row`・`click_selection` ほか）。
  - **ホイールは入力の口から無い**（`balloon_pressed.rs` の doc に「wheel/keyboard は本 spec 未実装」）＝バルーン窓のホイールの受け口を新しく作る。
  - **箱（シェル内バルーン）**: 箱の位置と大きさは `actor_box.rs`（`ShownBox`）が持ち、箱への押下は `input_events/shell_box_handler.rs` → `shell_box.rs` の `judge_box_click` を通る。箱の上の矢印のクリック・ホイールはこの道に足す。箱は描画を普通のバルーンと共有するので印の重ね方は 1 つで済むが、**印の画像（`arrow0.png` ほか）をどのフォルダから引くか**は箱では決まっていない（普通のバルーンはバルーンのフォルダ。箱はシェルのフォルダか、今のバルーンのフォルダか）。
  - キーは今もパーサに無い（`areka-parsers/src/balloon/validation_tests.rs` が「accessor が無い」ことを記録している＝キーを足すとこの記録の意図を書き換える）。箱は `areka-emo-compose/src/boxes.rs` で同じ `balloon::parse` を通るので、キーを足せば `balloon.名前`ブレスにも自動で書ける。
  - 通信のきっかけは `emo2_boot/update_cue.rs`（`UpdateCueSink`）と `crates/areka/src/update/`。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/balloon/{model.rs（774 行＝新しいファイルへ）, parse.rs, validation_tests.rs}`
  - `crates/areka-emo-present/src/balloon.rs`（`SeriesFamily` の行）
  - `crates/areka-emo-text/src/{layout.rs, viewbox.rs, viewbox_diff.rs, viewbox_draw_render.rs, actor.rs, actor_present.rs, actor_box.rs, state.rs（`balloonnum`）}`＋新規（印の配置）＋`lib.rs`
  - `crates/areka/src/input_events/{balloon.rs, balloon_pressed.rs, shell_box.rs, shell_box_handler.rs}`＋ホイールの新しい受け口
  - `crates/areka/src/emo2_boot/update_cue.rs` か `crates/areka/src/update/`
  - 検体（`clickwait*`・数字の画像は既定バルーンに無い）・`doc/ukadoc-coverage/ledger/assets.toml`
- 議題（答えで作業が変わるものだけ）:
  1. 箱の印の画像をどのフォルダから引くか（シェルのフォルダ／今のバルーンのフォルダ）。答えで系列の解決の入力が変わる。
  2. 手動で戻している最中に新しい文字が来たときの扱い（最新へ戻すか、戻したままか）。`balloon-scroll-fade` の押し出しの判定にも効く。
- 見つけた穴: なし。
