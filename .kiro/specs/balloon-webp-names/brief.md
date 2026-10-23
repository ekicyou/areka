# Brief: balloon-webp-names

> 2026-10-10 `/kiro-discovery`（開発者「デフォルトバルーンを作ってほしい」「画像ファイルなどは現代的なフォーマットにする」「online0.png などのアニメは動画 webp などで実現する」）で起票した。自前の既定バルーン（`default-balloon-selfmade`）が絵を WebP で持てるようにする、エンジンの側の小さい前提。roadmap「自前の既定バルーンと検体『どっとさくら』」節。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

バルーンの絵は、名前が `.png` で終わるものしか拾われない。WebP で作った絵を置くには、中身が WebP なのに名前を `.png` にするしかない（中身は署名で見分けるので動くが、名前が嘘になる）。

そのうえ、動かない WebP は Windows の読み手（WIC）に任せているので、Windows に WebP の拡張が入っていない機械では読めない。既定バルーンのように「どの機械でも必ず出る」ことが要る絵には使えない。

## Current State

- バルーンの面として拾うのは `{接頭辞}{数字}.png` だけ。接頭辞は `balloons`・`balloonk`・`balloonp{n}def` の 3 つ（`crates/areka-emo-present/src/balloon.rs` の系列の表 `BALLOON_FAMILY` と、名前から面の番号を取り出す `face_digits_of`・拡張子の定数 `FRAME_SUFFIX`）。面別の設定は `{接頭辞}{数字}s.txt`。
- 絵の中身は拡張子でなく署名で見分ける（`crates/areka-emo-atlas/src/decode/sniff.rs`）。動く APNG と動く WebP（コマ 2 枚以上）は `image` クレートで読み、動かない絵は WIC で読む。動く GIF は読まない（2026-10-04 の裁定）。
- `image` の WebP の読み手 `image-webp` はすでに依存に在る（動く WebP のため・`tech.md`）。動かない WebP を読むのに新しい依存は要らない。
- 動く絵をバルーンの面にした検体と、本物の読み手から合成までを通すテストは在る（`crates/areka-emo-compose/tests/fixtures/animated-playback/balloon`・`film_playback_e2e_tests.rs`）。どちらも名前は `.png` のまま中身を APNG・WebP にしている。
- 印の絵（`arrow*`・`online*`・`sstp`）はまだ読んでいない（`balloon-markers` が作る）。

## Desired Outcome

- バルーンのフォルダに `balloons0.webp`・`balloonk0.webp` の名前で置いた絵が、`.png` の名前の絵と同じに面として出る。動く WebP なら今と同じに動く。
- 動かない WebP が、Windows の拡張の有無に関わらず読める。
- 同じ面に `.png` と `.webp` の両方が在るときにどちらを採るかが決まっていて、テストで固定されている。
- 後から印の絵を読む `balloon-markers` が、同じ「名前から絵のファイルを探す」1 か所を通るだけで `.webp` の印も拾える。

## Approach

「名前から絵のファイルを探す」所を 1 か所にまとめ、受け付ける拡張子を `.png` と `.webp` の 2 つにする。動かない WebP は、動く WebP と同じ `image` の腕で読む（WIC に回さない）。拡張子を増やすだけで、絵の中身の見分け（署名）は今のまま。

## 議題（要件の段で決める）

1. **`.png` と `.webp` の両方が在るときの勝ち負け**。ukadoc に決まりは無い（areka の拡張）。案: `.png` を先に採る（SSP と両方で使えるバルーンが、SSP と同じ絵で出る）。
2. **拡張子はこの 2 つで止めるか**。`.apng` や AVIF は足さない案を推す（動く絵は APNG と WebP の 2 形式だけ、の裁定の範囲に収める）。
3. **動かない WebP を `image` で読む範囲**。バルーンだけでなく、シェルの `.png` の名前に WebP を入れた絵も同じ腕を通るので、読み手を替えるとシェルにも効く。色の扱い（α の掛け方）が WIC の腕と同じになることをテストで見る。
4. **シェルの絵の名前**（`surface0.webp`・element定義のファイル名）は範囲に入れない案を推す。element定義は名前を拡張子つきで書くので今でも `.webp` を名指せるかを確かめ、足りない所だけを記録して、要望が出てから起票する。
5. **SSP で読めないバルーンになること**の伝え方。`.webp` の名前の絵だけのバルーンは SSP では出ない。areka の説明書（`dist/README.txt`）と互換の記録（`doc/COMPAT_ARCHITECTURE.md`）に 1 行ずつ書く。

## Scope

- **In**: バルーンの面の絵の名前に `.webp` を受けること／名前から絵のファイルを探す所の一本化／動かない WebP を `image` で読むこと／`.png` と `.webp` の勝ち負けの決まりとテスト／`.webp` の名前の面（動かない・動く）の検体とテスト／互換の記録と説明書の 1 行ずつ／網羅台帳の備考（areka の拡張）。
- **Out**: 印の絵を読むこと・出すこと（`balloon-markers`）／シェルの絵の名前（議題 4）／動く GIF・AVIF・JPEG XL／`.pna`（使わない決まりのまま）／バルーンの descript の鍵を足すこと／既定バルーンの絵そのもの（`default-balloon-selfmade`）。

## Boundary Candidates

- 名前の解決（`crates/areka-emo-present/src/balloon.rs` の系列と拡張子）。
- 動かない WebP の読み手の振り分け（`crates/areka-emo-atlas/src/decode/`）。
- 検体とテスト・文書。

## Out of Boundary

- `crates/areka-parsers/src/balloon/`（descript の読み手）。鍵は足さない。
- 合成器と文字の層。絵の読み口より先は今のまま。
- 動く絵の上限（コマ数・画素数）。変えない。

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手できる）。完了 `areka-P0-animated-image-decode`・`areka-P0-animated-image-playback` の上に乗る。
- **Downstream**: `default-balloon-selfmade`（面の絵を `.webp` の名前で持つ）。`balloon-markers`（印の絵を同じ探し方で拾う）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `balloon-markers`（同じ系列の表 `SeriesFamily` に装飾の系列の行を足す＝同じファイルを触るので同時に走らせない・こちらを先に置く）。`balloon-canon-residue`（面の偶奇・`defaultsurface`＝系列解決の続き・同じファイル）。`animated-image-import`（シェルの読み手の側・重ならない見込み）。`config-parse-diagnostics`（読み手の診断・重ならない見込み）。

## Constraints

- 新しい依存を足さない（`image-webp` は在る）。
- `.png` の名前だけのバルーン（今の検体の全部）の見え方を 1 画素も変えない。
- 動く GIF に対応しない裁定（2026-10-04）と、α 付きの絵を正しく使う原則・`.pna` を使わない決まりを動かさない。
- 規模の見立ては S（4〜7 タスク）。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）
- 前提の変化: 版の段で v0.1.0（見た目＝`default-balloon-selfmade` の前）。起票（10-10）から触る所の変更は 0。本文の指し先は今も合う（拡張子の定数・面の番号を取る関数・系列の表は `crates/areka-emo-present/src/balloon.rs`、動く絵の見分けは `crates/areka-emo-atlas/src/decode/sniff.rs`）。
- 触るファイル: `crates/areka-emo-present/src/balloon.rs`（701 行）・静止画を WIC へ、動く絵を `image` へ回す振り分け `crates/areka-emo-atlas/src/decode.rs`（380 行）と `decode/image_arm.rs`（151 行）・兄弟のテスト・`.webp` の名前の検体（新しいファイル）・`dist/README.txt` と `doc/COMPAT_ARCHITECTURE.md` §8 の 1 行ずつ・台帳の備考。
- 規模: 4〜7 タスク（変わらず）。
- 先に要るもの: 無し。後ろで `default-balloon-selfmade`（絵を `.webp` で持つ場合）・`balloon-markers`（版に縛らない列）・`balloon-canon-residue`（いつか）が待つ。
- 同じウェーブに置けない相手: `placement-measure-bake-once`（案 B を選ぶと `balloon.rs` の `build_balloon_target_from_faces` に上限を渡す口を足す）。ほかの v0.0.3・v0.1.0 とは 0（`animated-image-import` は絵の読み手の `decode` に触らない）。
- 優先度の区分: A（開発者「画像ファイルなどは現代的なフォーマットにする」）。
- 要件定義のモデル: Opus（10-15 の基準に当たる所が無い）。
- 分割の案: 無し。
- 見つけた穴・古くなった記述: 本文に要件定義のモデルが書かれていない。議題 3（読み手を替えるとシェルの絵にも効く）は `decode.rs` の振り分けの 1 か所で決まる。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 前提の変化: v0.1.0 のゴールが「SSP 並みの自動試験」と「ぱすたが SSP と同じに動く」に組み直され、開発者の決め（10-18）で**この spec は v0.1.0 から外れた**（行き先は後で決める）。
  - 例外: `default-balloon-selfmade`（v0.1.0 に残る）が議題 3 で絵を `.webp` で持つと決めたら、この spec はその働きの前提になり、v0.1.0 へ戻る。α 付きの PNG を選べば外れたまま。
- ソースの変更: 起票から 0。`crates/areka-emo-present/src/balloon.rs`（701 行）の拡張子の定数 `FRAME_SUFFIX`（45 行）・系列の表 `BALLOON_FAMILY`（72 行）・面の番号を取る `face_digits_of`（238 行）と、振り分け `crates/areka-emo-atlas/src/decode.rs`（380 行）・`decode/image_arm.rs`（151 行）・`decode/sniff.rs`（122 行）はそのまま。
- 棚卸㉗で足した範囲: 無し。
- 触るファイル: 棚卸㉕のとおり。
- 規模: 4〜7 タスク（変わらず）。
- 先に要るもの: 無し。
- 同じウェーブに置けない相手: `placement-measure-bake-once`（v0.1.0・`balloon.rs` に上限を渡す口を足す案のとき）。版に縛らない `balloon-markers`・`balloon-canon-residue`・`extra-character-windows`・`talk-fast-forward` も `balloon.rs` を挙げる。`decode/` を挙げる `offline-surface-dump`（v0.1.0）は WIC の腕 `wic_arm.rs` を使うだけで振り分けには触らない見込み＝重なり 0。`install-live-target-hazards`・`compose-f16-precision` は読み手の腕を挙げる（着手の前に照らす）。
- 優先度の区分: A（開発者「画像ファイルなどは現代的なフォーマットにする」）。
- 要件定義のモデル: Opus（10-15 の基準に当たる所が無い）。
- 分割の案: 無し。
- 見つけた穴・古くなった記述: 本文の Downstream「`default-balloon-selfmade`（面の絵を `.webp` の名前で持つ）」は、向こうの議題 3 が決まるまでは見込み。v0.1.0 へ戻るかは、向こうの要件の段の最初の答えで決まる。
