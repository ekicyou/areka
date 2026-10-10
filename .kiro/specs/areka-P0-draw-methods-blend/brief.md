# Brief: areka-P0-draw-methods-blend

> 2026-10-10 棚卸㉓で `areka-P0-draw-methods-canon` から切り出した。同 spec が初めての実測で 20〜24 タスク（合成を Direct2D へ移す道なら 30 を超える）になり、上限の 20 を超えたため。`blend-*` は語の数が多いが、触るのは合成の語の表と画素の式だけで、読み手に触らずに足せる。出どころは開発者の指示（2026-10-05「残りの描画メソッドは起票」「合成の方法は Direct2D が支える全部に対応できるよう、wintf も含めて広げる」）。

## Problem

- **ゴーストの作者・利用者**: `blend-*`（加算・乗算・スクリーン・覆い焼きなど、画像編集ソフトのレイヤーの合成にあたる描画メソッド）で書いた element定義・pattern定義が描かれない。その部品や着せ替えが消える。
- **開発者**: 合成の語の表が正典より少ない。ブレンドの種類の一覧 `BlendKind`（`crates/areka-emo-compose/src/method.rs`）は「全量・19 種」と書くが、ukadoc と網羅台帳には 28 種ある。足りない 9 種は、今は「未知の合成メソッド名」として警告される。

正典の出どころ（ukadoc の文書 MCP で `blend-` を引いた 57 件＝`blend-*` の 55 件と旧い別名 2 件。descript_shell_surfaces・文は要約）:

- 種類は 28（括弧は登場した SSP の版）: `blend-add`・`blend-multiply`・`blend-screen`・`blend-overlay`（2.8.36）／`blend-color`・`blend-color-dodge`・`blend-hard-light`・`blend-hue`・`blend-luminosity`・`blend-saturation`・`blend-soft-light`（2.8.39）／`blend-color-burn`・`blend-darken`・`blend-darker-color`・`blend-difference`・`blend-divide`・`blend-exclusion`・`blend-hard-mix`・`blend-lighten`・`blend-lighter-color`・`blend-linear-burn`・`blend-linear-light`・`blend-pin-light`・`blend-subtract`・`blend-vivid-light`（2.8.40）／`blend-dither`（2.8.44）／`blend-add-glow`・`blend-color-dodge-glow`（2.8.46）。
- `-fast` 付きは 27。`blend-dither` にだけ `-fast` が無い（見出し `blend-dither-fast` は ukadoc に無い）。
- 旧い別名は 2 つ: `overlaymultiply`＝`blend-multiply-fast`、`overlayscreen`＝`blend-screen-fast`（見出し `overlaymultiply`・SSP 2.5.91）。
- 各語の説明は「新しいレイヤを○○合成で重ねる。画像編集ソフト（Photoshop など）の○○のレイヤーの合成にあたる」の形で、**式は載っていない**。光る 2 種（`blend-add-glow`・`blend-color-dodge-glow`）は CLIP STUDIO PAINT の合成にあたると書く。`blend-dither` は「ディザ合成で重ねる」とだけ書き、あたる編集ソフトの名も無い。
- `-fast` 付き（見出し `blend-add-fast` ほか）: ベースのレイヤの不透明度に応じて重ねる。振る舞いは `overlay-fast` と同じで外形は変わらず、上書きの代わりにその合成を行う。名前の「fast」は歴史の都合で、速いわけではない。
- どの語も、着せ替え・element定義でも使える。シェルの descript.txt に `seriko.use_self_alpha,full` を置くことを勧めている。

## Current State

今の木（main `ee3af616`）で確かめたこと。

- 語の表: `crates/areka-emo-compose/src/method.rs` の `BlendKind` は 19 種（`Add`・`Multiply`・`Screen`・`Overlay`・`Darken`・`Lighten`・`DarkerColor`・`LighterColor`・`ColorBurn`・`ColorDodge`・`HardLight`・`HardMix`・`Difference`・`Exclusion`・`Divide`・`Hue`・`Saturation`・`Color`・`Luminosity`）。綴りを種類へ写す `parse_blend` も同じ 19 語。同じファイルのテストが「全 19 種」を名指しで数えている。
- 無い 9 種: `add-glow`・`color-dodge-glow`・`dither`・`linear-burn`・`linear-light`・`pin-light`・`soft-light`・`subtract`・`vivid-light`。それぞれの `-fast` 付きも読めない。
- `-fast` は種類と別の印（`BlendMode` の `fast`）で持つ。旧い別名 2 つは `parse_blend` が `fast` 付きへ写す。`blend-dither-fast` のような正典に無い組は、今の `parse_blend` の作りだと種類を足したとたんに読めてしまう。
- 描く側: 「描ける語か」の 1 関数（`ComposeMethod::is_implemented`）は `Overlay` だけ真。転写（`crates/areka-emo-compose/src/blit.rs`・782 行）は premultiplied の SourceOver を CPU の整数で行い、ほかの語の命令は警告して飛ばす。
- seriko: `crates/areka-seriko/src/table.rs` のテストが、`blend-multiply` のコマは「描けない」と確かめている。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml`: `blend-*` の行は 55（うち `-fast` 付き 27）。旧い別名の 2 行は `alias`。

## Desired Outcome

- ukadoc が書く `blend-*` の 28 種と、`-fast` 付きの 27、旧い別名 2 つを、element定義・pattern定義・着せ替えで描く。
- 語の表が正典と同じ数になり、「全 19 種」の記述とテストが実物に合う。正典に無い組（`blend-dither-fast`）は読まない。
- 語ごとの画素を決定論のテストで固定する。
- 描かないと決めた語が残るなら、記録を出す今の作りを保ち、台帳に未対応と書く。

## Approach

- 合成をどこで行うか（CPU の整数の道か、Direct2D の道か）は `draw-methods-canon` の要件の議題 1 で決まる。この spec はその答えに従う。
  - CPU の整数の道: 種類ごとの画素の式を新しいファイルに置き、`blit.rs` は振り分けだけを足す。`-fast` は、`draw-methods-canon` が入れる `overlay-fast` の「ベースの不透明度に応じて重ねる」形に、種類の式を差し込む。
  - Direct2D の道: `draw-methods-canon` の要件でこの道が選ばれたら、そのとき wintf だけの spec が起票される。この spec はその口を使う側になり、規模と触るファイルを測り直す。
- 読み手には触らない。`draw-methods-canon` が、`blend-*` の語も含めて描画メソッドの語を落とさずに運ぶ形にしておく（同 spec の brief の「棚卸㉓の分割」に書いた決め）。

## Scope

- **In**: `blend-*` の 28 種・`-fast` 付き 27・旧い別名 2 つ。語の表の 9 種の追加と「19 種」の記述の直し。画素の式・決定論のテスト・網羅台帳の 57 行・`doc/COMPAT_ARCHITECTURE.md` §8（正典が黙っている所の決めごと）・実機の確かめ。
- **Out**: 読み手が描画メソッドを運ぶこと・「描ける語か」の門・pattern定義の `base`・重ね方の基本の 5 語（`draw-methods-canon`）。`move`・`auto`。wintf の Direct2D の口（選ばれたときに起票される別の spec）。α の宣言（完了 `self-alpha-declaration`）。

## Boundary Candidates

- 語の表（`method.rs` の `BlendKind`・`parse_blend`）。
- 画素の式（新しいファイル）と、転写の振り分け（`blit.rs`）。
- 網羅台帳の 57 行。

## Out of Boundary

- 読み手（`crates/areka-parsers/`）。
- 合成の命令を組む段（`plan.rs`・`nesting.rs`・`plan_always.rs`）。門の形は `draw-methods-canon` が決め、この spec は「描ける語」を増やすだけにする。
- 表示の段（wintf の窓・スワップチェーン）。

## Upstream / Downstream

- **Upstream**: `draw-methods-canon`（読み手が語を運ぶ・門・`overlay-fast` の形・CPU か Direct2D かの裁定）。完了 `self-alpha-declaration`（α の宣言。正典は `blend-*` に `seriko.use_self_alpha,full` を勧める）。
- **Downstream**: `blend-*` を使うシェル全般。

## Existing Spec Touchpoints

- **Extends**: `draw-methods-canon` の範囲から `blend-*` を引き取る。
- **Adjacent**: `draw-methods-canon`（同じ `method.rs`・`blit.rs`＝直列）。`seriko-trigger-intervals`・`seriko-script-triggers`・`seriko-interval-combinations`・`animated-image-import`（seriko の `table.rs`。この spec が触るのはテストの期待 1 か所だけ）。

## Constraints

- 合成の決定論を保つ（CPU の整数の道なら浮動小数を持ち込まない。同じ入力から同じ画素）。
- `blit.rs` は 782 行。式は新しいファイルへ置く。1 ファイル 1,000 行以下。テストは兄弟のファイルへ。
- 意味は ukadoc から取り、SSP の実測で決めない。正典が式を書いていない所は、採った式と出どころを §8 に記す。
- 描けない語を黙って捨てる経路を作らない。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**:
  - `crates/areka-emo-compose/src/method.rs`（415 行・`BlendKind`・`parse_blend`・「19 種」の記述とテスト）
  - `crates/areka-emo-compose/src/blit.rs`（782 行・振り分けだけ）と、画素の式の新しいファイル＋兄弟のテスト
  - `crates/areka-seriko/src/table.rs` のテストの期待 1 か所（`blend-multiply` のコマが「描けない」と確かめている行）
  - `doc/ukadoc-coverage/ledger/assets.toml` の 57 行（`blend-*` の 55 行と旧い別名の 2 行）・`doc/COMPAT_ARCHITECTURE.md` §8
  - 読み手（`crates/areka-parsers/`）には触らない。
- **規模**: 9〜12 タスク（CPU の整数の道のとき。Direct2D の道なら測り直す）。
- **先に要るもの**: `draw-methods-canon`。
- **優先度の区分**: A（開発者 10-05「残りの描画メソッドは起票」「Direct2D が支える全部に対応」。切り出し元の区分を引き継ぐ）。
- **要件定義のモデル**: Fable（正典に式が無い語の決め・半透明どうしの合成の決め・決定論の取り方）。
- **議題**（答えで作業が変わるものだけ）:
  1. `blend-dither` をどう描くか。正典は「ディザ合成」とだけ書き、式も、あたる編集ソフトの名も無い。描き方を areka が決めて §8 に記すか、描かずに未対応として記録するか。
  2. 式の出どころ。正典は「画像編集ソフトの○○合成にあたる」と書くだけなので、採る式の出どころ（広く公開されている合成の仕様か、Direct2D のブレンドの効果の定義か）と、整数での丸め方を決める。光る 2 種は別の編集ソフトの合成にあたると書かれているので、出どころが別になる。
  3. 半透明どうしの重ね方。`-fast` 付きは「ベースの不透明度に応じて重ね、外形は変わらない」と正典が書く。`-fast` の付かない語で、ベースが透明な所に新しいレイヤが来たときの扱い（そのまま載せるか）は正典が黙っている。
  4. Direct2D の道になったとき、Direct2D のブレンドの効果に無い種類をどうするか（この brief では Direct2D の種類の一覧と照らしていない。対応表は要件で作る）。
