# Brief: element-clipping-option

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-animated-image-decode` のタスク 6.3（要件 9.3）。2026-10-04 のギャップ分析で、正典 C2「`--clipping` を使うと動く絵としての読み込みが無効になる」を引き受ける spec が実在しないことを確かめた（`animated-image-decode` の `research.md` 5 節の要件 9 の行）。**段は優先（動く画像・シェルの element の列で `animated-image-playback` の後）**。

## Problem

- **ゴースト作者**: element定義の末尾にオプション `--clipping,左,上,右,下` を書いても、areka は無視して画像の全体を描く。正典（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html の element定義の項）は次のとおり定める。
  - 「[SSP 2.8.15～]左、上、右、下で指定された矩形部分のみを描画する。これを使用するとアニメーション読み込みは無効となる。」
  - 例: `element0,overlay,face.png,0,0,--clipping,10,10,50,50`
- 1 枚の大きな画像から部品を切り抜いて並べるシェルは、切り抜かれずに全体が重なって出る。
- `animated-image-decode` で APNG・動く WebP の全コマを読むようになった。`--clipping` 付きの element定義が動く絵を指すと、正典では動かない（1 枚目だけ）のに、areka では再生の側（`animated-image-playback`）が着地すると動いてしまう。

## Current State

- `crates/areka-parsers/src/shell/decode.rs` の `decode_elements` は `elementN,overlay,PATH,X,Y` の 5 つの欄だけを読み、6 番目以降（オプション）は黙って捨てる。型 `Element`（`crates/areka-parsers/src/shell/model.rs`）にオプションの欄は無い。
- アトラスの鍵 `AtlasKey`（`crates/areka-emo-atlas/src/table.rs`）は（組の番号, 相対パス）で、同じ画像を切り抜き違いで使い分ける区別を持たない。`manifest.rs` は相対パスで重複を除く。
- 動く絵は、`animated-image-decode` が鍵ごとに見出しを聞いて全コマを読み、`AtlasTable::animation(親)` で引ける。鍵が element定義のオプションを知らないので、`--clipping` 付きでも動く絵として読まれる。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `element*` の項は「縮退」（担当 `areka-P0-shell-parse`）。注記は `overlay` 以外の行を読み飛ばすことだけを書いていて、オプション（`--clipping` ほか）を捨てていることは、台帳にも `doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表にも書かれていない（記録なし）。Scope の「網羅台帳と対応表の更新」はこの空白を埋める。

## Desired Outcome

- `--clipping,左,上,右,下` を付けた element定義は、画像のその矩形だけを、element定義の X・Y の位置に描く。
- `--clipping` を付けた element定義が動く絵を指すときは、動く絵として読まず、1 枚目（`animated-image-decode` の縮めた 1 枚と同じ絵）を切り抜いて描く。
- 同じ画像を、切り抜き無し・切り抜き違いの複数の element定義で使っても、それぞれが正しく出る。
- 読めない値（欄の不足・数でない・左 ≥ 右など）の扱いを決め、黙って捨てずに記録する。

## Approach

- 読み手（`areka-parsers`）は element定義のオプションを**転記だけ**する（解釈は下流＝「parser は転記層」の決まり）。`--clipping` 以外のオプションも、名前と値の並びとして型に残す。
- アトラスの側で、切り抜きを鍵に含める（または切り抜きを描く側で行う）かを設計の段で決める。動く絵を読まない判定は、鍵を作る段（`manifest.rs`）か読み込みの段取り（`animated.rs` の `load`）に置く。
- 当たり判定・ビューボックスの広がりは、切り抜いた後の大きさで決まるかを要件の段で ukadoc から確かめる。

## Scope

- **In**: element定義の `--clipping,左,上,右,下` を読んで描くこと。`--clipping` 付きの element定義では動く絵として読まないこと（正典 C2）。オプションの並びを読み手で転記すること（`--clipping` 以外は値として残すだけ）。網羅台帳と対応表の更新。
- **Out**: `--alpha`・`--source`・`--scaling` の解釈（読み手が転記するところまで。描画への反映は別に起票する）。さくらスクリプト `\_b[...]` の `--clipping`（バルーンの画像の貼り付け＝別の入口）。`overlay` 以外の描画メソッド。動く絵の再生そのもの（`animated-image-playback`）。

## Boundary Candidates

- 読み手: element定義のオプションの転記（`areka-parsers` の shell の model と decode）。
- アトラス: 切り抜きの扱いと、動く絵として読まない判定（`areka-emo-atlas` の manifest・鍵・`animated.rs`）。
- 合成: 切り抜いた絵を X・Y に置く（`areka-emo-compose`）。

## Out of Boundary

- `--alpha`・`--source`・`--scaling` の描画。
- `\_b[...]` のオプション。
- 動く絵の再生・`import`・interval `always`。

## Upstream / Downstream

- **Upstream**: `animated-image-decode`（動く絵の読み込み・縮めた 1 枚）。`surface-element-nesting`（element定義でサーフェスを置く＝同じ `decode_elements` と `Element` を触る）。
- **Downstream**: `animated-image-playback`（`--clipping` 付きでは動かさない前提で再生を組める）。`--alpha`・`--scaling` を引き受ける将来の spec。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**: `surface-element-nesting`（element定義のファイル名の欄に数字だけ＝サーフェスの番号。オプションと同じ行を読む）。`animated-image-playback`（再生の側の判定と重ならないように、動く絵として読まない判定はこちらで持つ）。`self-alpha-declaration`（`normalize.rs` を触る）。

## Constraints

- 直列の列「シェルの element」（`crates/areka-parsers/src/shell/{model,decode}.rs`・`crates/areka-emo-compose/`・`crates/areka-emo-atlas/src/manifest.rs`）に入る。読み手（`decode_elements`・`Element`）と `manifest.rs` は `surface-element-nesting` と、合成（`areka-emo-compose` の置き方）は `animated-image-playback` と触るファイルが重なるので、両方の着地の後に着手する。
- 正典の意味は ukadoc から輸入する（SSP の実測はしない）。
- 鍵の形を変えると `emo2` の照合（`emo2_golden`）が動く。オプションの無いシェルでは表が変わらないことを守る。

## 要件の段で決める議題

1. 切り抜きの矩形が画像の外にはみ出す・左 ≥ 右・欄が 4 つに足りないときの扱い（正典は書いていない）。
2. 同じ画像を切り抜き違いで使うときのアトラスの持ち方（鍵に矩形を足すか、1 枚を載せて描く側で切るか）。
3. 当たり判定（collision）とビューボックスの広がりは、切り抜き後の大きさで決まるか。

## 2026-11-04 棚卸㉞の再測定（main `932406f9`）

- 前提の変化: 2026-10-16 に v0.2.0 へ戻すと決まり（roadmap の「版をまたぐ依存の確かめ」）、`--clipping` は D2D の節の切り抜き（`wintf-visual-tree` の `VisualClip`）で描く形にそろえる。emo の合成を節の木へ移すのは `draw-methods-canon` なので、**順は `draw-methods-canon` → この spec**（`wintf-visual-tree` の brief の Downstream と同じ。シェルの element の列の「この spec → `draw-methods-canon`」は CPU の道のときの順で古い）。Approach の「合成の側で切る」は節の切り抜きと読む。読み手（`decode_elements` は 5 つの欄だけ読みオプションを黙って捨てる）・アトラスの鍵 `AtlasKey`（組の番号, 相対パス）・動く絵の分かれ目 2 段（焼く入口 `bake_with_limits`〔`crates/areka-emo-atlas/src/lib.rs`〕と分解 `decompose`〔`crates/areka-emo-compose/src/film.rs`〕）は今の main でも起票の読みのまま。
- 触るファイル: `crates/areka-parsers/src/shell/{model.rs, decode.rs}`（303・623 行）・`crates/areka-emo-atlas/src/{manifest.rs, lib.rs}`（563・875 行・鍵を広げるなら `table.rs`）と `emo2_golden.rs`・`crates/areka-emo-compose/src/{fold,normalized,atlas_bind,film,plan,plan_extent}.rs`・節の切り抜きを置く emo の表示の側（`crates/areka-emo-present/`・`draw-methods-canon` の設計で決まる）・台帳 `assets.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。`Element` に欄を自分で足すなら直書きの 44 ファイル（6 クレート）も。
- 規模: 11〜15 タスクのまま（`draw-methods-canon` がオプションの欄を先に足していれば 10〜14、自分で 44 ファイルを直すなら 12〜16）。
- 先に要るもの: `wintf-d2d-composite-rules`・`draw-methods-canon`。働きでは `extent-element-offset`（外形に element の X,Y を数えるかの裁定の上に載る）が要るが、今どの版にも入っていない（「v0.1.0 から外した 5 本」）＝v0.2.0 へ入れて前に置くか、この spec が外形の扱いを自分で決めるかを決める。同じ 5 本の `animated-image-import`（読み手・`manifest.rs`・`plan.rs`・`film.rs`）も同じ。見直しは台帳の `audit-emo-compose`・`audit-emo-atlas`・`audit-parsers-sakura` に、表示の側の分で `audit-emo-present` を足す。`Element` の欄を自分で足すなら `audit-emo-text-state-actor`・`audit-emo2-boot-spine`・`audit-placement-core` と本体の crate 分け（`placement-crate-split`・`runtime-module-untangle`・`runtime-crate-split`）も。同じウェーブに置けない相手＝`draw-methods-canon`・`draw-methods-blend`・`collisionex-regions`（読み手）。
- 優先度の区分: C（正典の拾い残し）。
- 要件定義のモデル: Opus（鍵を広げるのも節の切り抜きも今ある口に足す形＝新しい境界ではない）。
- 分割の案: 無し。
- 見つけた穴・古くなった記述（前の再測定の節から持ち越し）: 同じ画像を「切り抜きあり」と「なし」で使うと片方は動き片方は止まる＝鍵を分ける形が要る（議題 2）。Constraints の「読み手と `manifest.rs` は `surface-element-nesting` と共有」は古い（今の相手は上の spec たち）。外形の計算は `plan.rs` でなく `plan_extent.rs`。
