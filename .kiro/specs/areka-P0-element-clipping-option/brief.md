# Brief: areka-P0-element-clipping-option

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


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S〜M（10〜14 タスク）。起票時の 8〜12 より上振れ（鍵・動く絵の判定・外形の 3 か所に及ぶ）。切らない。
- 前提の状態: `animated-image-decode`・`surface-element-nesting` とも着地済み。ただし同じ行（`element0,base,face.png,0,0,--clipping,…`）を読む `element-base-method` と、合成の置き方を分け合う `animated-image-playback`・`extent-element-offset` の後＝今すぐは着手しない。
- 崩れた前提／古くなった位置:
  - `surface-element-nesting` は `decode_elements`・`Element`・`manifest.rs` を**触らずに**着地した（roadmap の C3 の実績）。Constraints の「読み手と `manifest.rs` は `surface-element-nesting` と共有」は古い。今の相手は `element-base-method`（`Element` の型・`decode_elements`・転記の element を合成の element へ写す `fold.rs` の 1 か所）。
  - 動く絵として読むかの分かれ目は、焼く入口 `bake_with_limits`（`crates/areka-emo-atlas/src/lib.rs`）が鍵ごとに `animated::load` を呼ぶ所。Approach の「`animated.rs` の `load`」より 1 段手前で、鍵が `--clipping` を知る形（鍵か一覧の欄）が要る。鍵 `AtlasKey`（組の番号, 相対パス）と `manifest.rs` は今も起票時のまま。
  - 外形の計算 `flatten_extent`（`plan.rs`）は `extent-element-offset` が直す。切り抜いた後の大きさを外形へ数えるなら、あちらの裁定（element定義の X,Y を数えるか）の上に載る。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/shell/{model.rs, decode.rs}`（オプションの転記）
  - `crates/areka-emo-atlas/src/{manifest.rs, lib.rs}`（鍵を広げるなら `table.rs`）と `emo2_golden.rs` の照合
  - `crates/areka-emo-compose/src/{fold.rs, normalized.rs, atlas_bind.rs, plan.rs}`（当たり判定の大きさを変えるなら `hit.rs`）
  - `doc/ukadoc-coverage/ledger/assets.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: 起票時の 3 つのまま。
- すぐ直せる軽微な修正: この brief の「要件の段で決める議題」の直後に、起票のときの書き損じの 2 行（`</content>`・`</invoke>`）が紛れ込んでいる。消すだけ。
