# Brief: areka-P0-dot-sakura-specimen

> 2026-10-10 `/kiro-discovery`（開発者「ゴースト検体に『どっとさくら』を追加」）で起票した。同じ依頼の後半（同梱のバルーンを手本に既定バルーンを作る）は `areka-P0-default-balloon-selfmade` と `areka-P0-balloon-webp-names`。roadmap「自前の既定バルーンと検体『どっとさくら』」節。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

areka の検体は、pasta（`emo2`）・里々（`R_POST_and_KOMAINU`）・YAYA（`konnoyayame`・`claudia`・`emily4`）の 3 系統しか無い。2003 年ごろの古い作りのゴースト（華和梨・Shift_JIS・α の無い絵・連番のオンラインの印を持つ同梱バルーン）を、手元で繰り返し確かめる材料が無い。

開発者の自作のゴースト「．さくら」（どっとさくら）は、その古い作りをまとめて持っている。これを検体に足す。

## Current State

- 検体は `vendors/sample_ghost/*.nar`（配布形のまま保管）で、テストは `sample-ghost-kit` の登記表 `SAMPLES` 越しに名前で引く。今の登記は 7 行。
- 足すと数が合わなくなる場所は `crates/sample-ghost-kit/src/lib_tests.rs`（`SAMPLES.len()` を 7 と比べる 2 か所・知らない名前を引いたときの名前の一覧・説明の中の「7」）。
- `crates/log-capture-kit/tests/sample_path_guard_test.rs` は、登記した検体の名前と同梱バルーンの名前を「ソースに直に綴ってはいけない語」として見張る。
- 直近の例 `emily4` は、`.nar` と README の登記だけ済ませ、`SAMPLES` には載せていない（書庫が入らないため。`areka-P0-emily-ghost-verification` が持つ）。

### 検体の中身（2026-10-10 に取り寄せて確かめた）

- 出どころ: `https://ekicyou.github.io/old/dot-sakura/download/nar/dot_sakura.nar`（開発者のサイト）。1,425,534 バイト・204 項目。SHA-256 は `0c602c6a431dcbec48c02d934d98e342337d2db9628095e0a6f90d3e83b7a472`。
- 最上位の `install.txt`: `type,ghost`・`name,．さくら`・`directory,dot_sakura`・`balloon.directory,bottle`。取り出し元の `bottle/` は書庫の最上位に在る（`emily4` のような「名乗るのに無い」形ではない）。
- ゴースト: `ghost/master/descript.txt` は `charset,Shift_JIS`・`shiori,shiori.dll`。`shiori.dll` は 32bit（x86）で、中に `KAWARI.kdt/7.4.` の文字が在る（**華和梨**＝areka の検体に無かった 4 つ目の系統）。SHIORI の版（2.x か 3.0 か）は未確認。辞書は Shift_JIS のテキスト約 40 本。
- シェル 3 つ（`master`・`sakura100`・`sakura50`）。絵は `surface*.png` だけで、`.pna` は無い。`surfaces.txt`・`alias.txt`・`surfacetable.txt` を持つ。
- 同梱バルーン `bottle/`（名前は「ボトル（でふぉ改）」・`craftman,MAz`）。面は `balloons0〜3`・`balloonk0〜1`・`balloonc0〜3`、印は `arrow0/1`・`online`・`online0〜15`・`sstp`。絵は 8 ビットのパレットの PNG で α を持たず、角を赤の抜き色で抜く作り。
- 最上位に `delete.txt`（消すファイルの一覧）と `updates.txt`、`readme.txt`。
- `readme.txt` の取り扱い: 「無償ですが著作権複数のためフリーには出来ません」「転載・配布: インターネット上は不可、それ以外は各自の責任において可」。シェルの `readme.txt` は「リビルド: 日蔭 翠」「インターネット上でのアーカイブの転載不可」。

## Desired Outcome

- `dot_sakura` を検体の名前で引ける（`SampleRoot::acquire("dot_sakura")`）。同梱バルーン `bottle` も登記の行に載っている。
- 登記表の全行を展開する今のテストが、8 行で緑になる。
- `vendors/sample_ghost/README.md` に、出どころ・SHA-256・取り扱いの条件・畳み直したかが載っている。
- 実機で 1 回起こした結果（起動するか・何が出て何が出ないか）が記録に残り、areka の未対応で崩れた件は 1 件ずつ起票されている。

## Approach

受け取ったバイト列のまま `vendors/sample_ghost/dot_sakura.nar` に置き（畳み直さない）、登記表に 1 行足し、README に登記し、数を持つテストと文書を 8 へ直す。そのうえで実機で 1 回起こし、見えたことを記録する。**動くところまで直すのはこの spec の仕事にしない**（直しは起票した先）。

書庫はリポジトリに入れる（下の「決まったこと」）。

## 決まったこと（2026-10-10 開発者）

- **`.nar` はほかの検体と同じにリポジトリへ入れる。** `readme.txt` は「インターネット上は不可」と書くが、開発者「どっとさくらは私の実装なので、検体として利用する分には問題ない。私のサイトで公開してます」。README の登記には、この開発者の言葉と、areka の配布物には入れないこと・畳み直さないことを書く。
- **検体を URL から取り寄せる形へ替えるのは別の spec**（`areka-P0-sample-url-fetch`・急がない）。この spec は今の形（書庫を置く）で足す。README の登記に取り寄せ元の URL と SHA-256 を書いておけば、後の spec がそのまま使える。

## 議題（要件の段で決める）

1. **見張りの語との当たり**。同梱バルーンの名前 `bottle` と検体の名前 `dot_sakura` が見張りの語になる。今のソースに同じ綴りが在れば `sample_path_guard_test` が赤くなる。先に数えて、当たるなら見張りの側の決まり（どこまでを当たりと見るか）で解く。
2. **`install.txt` の読み**。`balloon.directory,bottle` だけで取り出し元を書かない形・最上位の `delete.txt`・`updates.txt` を、今の `areka-nar` がどう扱うか。入らないなら `emily4` と同じく登記を見送って起票する（その場合この spec は README の登記までで閉じる）。
3. **華和梨の `shiori.dll` を今の 32bit の載せ台が動かせるか**。SHIORI の版が 2.x だけなら areka は話せない。実機の 1 回で分かったことを記録し、要るなら起票する。

## Scope

- **In**: `dot_sakura.nar` の保管（リポジトリに入れる）／登記表 `SAMPLES` の 1 行（同梱バルーン `bottle` つき）／`lib_tests.rs` の数と名前の一覧／`vendors/sample_ghost/README.md` の表と節／steering（`product.md` の検体の表・`structure.md` の顔ぶれ）と根の `README.md` の動作確認の表／実機で 1 回起こした記録／崩れの起票。
- **Out**: 起動・会話・メニューが通るところまでの直し（起票した先）／華和梨や SHIORI 2.x への対応／同梱バルーンの印（矢印・オンライン・SSTP）の表示（`areka-P0-balloon-markers`）／入力の箱 `balloonc*`（`areka-P0-balloon-canon-residue`・`areka-P0-inputbox-user-input`）／シェルの縮小版 2 つの見た目の確かめ／配布物への同梱（入れない）。

## Boundary Candidates

- 検体の保管と登記（`vendors/sample_ghost/`・`crates/sample-ghost-kit/src/lib.rs`・`lib_tests.rs`）。
- 文書（`vendors/sample_ghost/README.md`・steering・根の `README.md`）。
- 実機の 1 回の記録と起票（spec の `verification/`）。

## Out of Boundary

- `areka-nar` の展開器・`shiori-host32-*`・表示の経路。直さない（崩れは起票）。
- `StayseeBalloon` と既定バルーンの解決。触らない（`areka-P0-default-balloon-selfmade` が持つ）。

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手できる）。
- **Downstream**: `areka-P0-default-balloon-selfmade`（同梱バルーン `bottle` を見比べの元にする・登記表と数のテストを続けて触るので、こちらを先に着地させる）。`areka-P0-balloon-markers`（連番のオンラインの印 16 コマ・矢印・SSTP の印を持つ検体として使える）。`areka-P0-balloon-canon-residue`（面 0〜3 と面別の設定を持つ検体として使える）。`areka-P0-sample-url-fetch`（取り寄せ先がすでに在る最初の検体）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka-P0-emily-ghost-verification`（同じ登記表と同じ数のテスト・README の同じ表を触る＝同時に走らせない）。完了 `areka-P0-nar-install`（登記表と窓口を作った）。完了 `areka-P0-ghost-standard-balloon`（同梱バルーンを持つゴーストは、起動で同梱の 1 個目が既定より先に採られる）。

## Constraints

- 受け取ったバイト列を 1 バイトも変えない（`vendors/sample_ghost/.gitattributes` の `* -text` が効く場所に置く）。畳み直さない。
- areka の配布物（`tools/package.ps1`）には入れない。
- 実機の根・展開先・一時のファイルはワークツリーの `target\` の下だけ。実走は短い絶対パスで起こす。
- 第三者の絵（シェル）を含むので、スクリーンショットを文書や PR に貼るかどうかは開発者に聞いてからにする。
- 規模の見立ては S（4〜6 タスク）。
