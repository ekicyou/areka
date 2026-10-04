# ギャップ分析: areka-P0-animated-image-decode

> 2026-10-04 実施（ブランチ `claude/areka-p0-animated-image-decode-66d597`・コミット `8f0232ba`）。
> 要件（`requirements.md`）と今のコードの差を調べ、設計の段で決めることを並べる。**結論は出さず、材料と選択肢を示す**。
> コードの引用は「何の定義か（関数名・型名＋ファイル）」で指す。外部クレートの中身は、この機械の cargo の保管庫（`c:\rust\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\`）に展開されている実物のソースを読んで確かめた。上流の未公開の版は GitHub の生ファイルを `target\gap-tmp\` へ取って読んだ。

> **2026-10-04 要件討議の裁定（本書より後）: 動く GIF には対応しない**（開発者「古い」）。対応するのは APNG と WebP の 2 形式。本書の GIF についての記述（3.1 節の `gif`・`weezl`・`color_quant`、3.3 節の全体、3.5 節の GIF の行、3.6 節の GIF の反復子、8 節の項目 3）は**不要になった**。謝辞に新しく載るのは約 12 本（`Cargo.lock` に新しく入るクレートは 0 本の見込み）へ減る。動く GIF は今までどおり WIC が 1 枚だけ読む。

## 1. まとめ

- **今あるもの**: 読み手の口 `ElementDecoder`（`decode.rs`）は「1 ファイル → 1 枚の絵」だけ。本番の読み手 `WicDecoderArm`（`decode/wic_arm.rs`）は `GetFrame(0)` だけを呼ぶ。焼く入口 `bake`（`lib.rs`）は鍵 1 つにつき `decode` を 1 回呼び、表 `AtlasTable`（`table.rs`）は「鍵の数＝エントリの数」を `assert_eq!` で守る。コマ・待ち時間・繰り返し回数を持つ場所は 0 か所。
- **足りないもの**: ⑴ 動く絵かどうかを中身で見分ける口 ⑵ 全コマ・待ち時間・繰り返し回数を返す口 ⑶ 表の中でコマを番号で引く欄 ⑷ 上限と縮退の経路 ⑸ 3 形式の検体 ⑹ 外部クレートの本番への登記。
- **外部クレートの裏取りで分かったこと（要件の文面と違う点・足りない点がある）**:
  - 動く WebP の欠陥は実在し、**原因は 2 つ**ある。公開版 0.2.4 のまま呼び方で避けられるのは 1 つ目だけ（3.2 節）。
  - 上流の `main` の版の欄は **0.2.4 のまま**、`release-0.2.5` の枝だけが 0.2.5。要件 7.6 の「公開版より上であることを確かめる」は、どちらを取り込むかで成り立ち方が変わる（3.2 節）。
  - GIF の繰り返し回数は、`gif` クレートを**頭だけ読む数行**で正確に取れる。重ね合わせを自前で書く必要は無い（3.3 節）。
  - `image` の口からは、3 形式のどれも**コマの総数を先に聞けない**。上限の判定は「読み進めながら数えて打ち切る」形になる（3.4 節）。
  - `png` が `flate2` を連れてくる。steering `tech.md` の `miniz_oxide` の項は「`flate2` は入れない」と書いている（3.1 節）。
- **検体の実数**: リポジトリ内の動く絵（コマが 2 枚以上の GIF・APNG・WebP）は **0 枚**（4 節）。
- **見つけた設計上の分かれ目**: `bake` はシェルとバルーンで共用で、どちらの集合かを知らない。何もしなければバルーンの動く絵も全コマ焼かれ、APNG では見た目が変わりうる（要件 5.4 と当たる・5.3 節）。
- **規模とリスク**: 規模 M（3〜7 日）、リスク 中（7 節）。

## 2. 今のコード

### 2.1 読み手の口

| 何の定義か | ファイル | 今の形 |
| --- | --- | --- |
| trait `ElementDecoder` | `crates/areka-emo-atlas/src/decode.rs` | メソッドは `decode(&self, &Path) -> Result<DecodedImage, DecodeError>` と、既定の実装つきの `probe_pna`（既定は `false`）の 2 つ |
| 構造体 `DecodedImage` | 同上 | `width`・`height`・`stride`・`bgra`・`has_alpha` の 5 欄。コマ・時間の欄は 0 個 |
| 列挙 `DecodeError` | 同上 | `NotFound {path}` と `Decode {path, source}` の 2 つ |
| 構造体 `MemoryDecoder` | 同上 | テスト用の偽の読み手。`insert`・`insert_corrupt`・`insert_pna` で登録する。`#[cfg(test)]` ではなく公開 |
| 構造体 `WicDecoderArm` と `decode_inner` | `crates/areka-emo-atlas/src/decode/wic_arm.rs` | `CreateDecoderFromFilename` → `GetFrame(0)` → 乗算済み BGRA へ変換 → `CopyPixels`。`has_alpha` は変換前の画素形式から `pixel_format_has_alpha` で決める |

- trait を実装している型は `WicDecoderArm` と `MemoryDecoder` の 2 つだけで、どちらもこのクレートの中にある（`impl ElementDecoder for` をワークスペース全体で検索した結果）。
- `DecodedImage` が届ける画素は**乗算済みの BGRA**（WIC の PBGRA）。`normalize.rs` の `Normalizer::normalize` は α を持つ絵をそのまま通す。`image` クレートが返すのは**乗算していない RGBA**なので、動く絵の読み手は「RGBA → BGRA の並べ替え」と「α の乗算」を自分で行う必要がある（今のコードにこの変換は無い）。

### 2.2 焼く入口と表

| 何の定義か | ファイル | 今の形 |
| --- | --- | --- |
| 関数 `bake(sets, decoder, cfg)` | `crates/areka-emo-atlas/src/lib.rs` | 鍵ごとに `decode` → `probe_pna` → `Normalizer::key_color` → `normalize` → `Trimmer::trim` → 生き残りを詰めて `Packer::pack` → `Baker::bake_pages` → `AtlasTable::new` |
| 構造体 `SurfaceSet` | `manifest.rs` | `surfaces`・`base_dir`・`alpha_params` の 3 欄。**シェルかバルーンかを示す欄は無い** |
| `ManifestDeriver::derive` | `manifest.rs` | 1 ファイル 1 鍵。（集合の番号・相対パス）の昇順で番号を振る |
| 構造体 `AtlasKey {set, rel_path}` | `table.rs` | コマの番号の欄は無い |
| `AtlasTable::new(keys, entries, pages)` | `table.rs` | `keys.len() == entries.len()` を `assert_eq!` で守る。逆引き表は `keys` を順に入れて作る（同じ鍵が 2 回あると後のものが勝つ） |
| `AtlasTable::entry`・`resolve`・`key`・`len` | `table.rs` | `ElementId` は `entries` の添字 |
| `PackConfig::default` | `pack.rs` | ページは 2048×2048・余白 1 画素 |
| `Packer::pack` | `pack.rs` | 余白込みでページを超える矩形は `error!` を出して外す。ページ数を 1 から増やしながら全部を詰め直す |
| 列挙 `BakeError` | `error.rs` | `Decode` と `Normalize` の 2 つ |

- `bake` を呼ぶ本番の経路は 2 つ: シェルは `build_shell_target_with_boxes`（`crates/areka-emo-present/src/shell_target.rs`）、バルーンは `build_balloon_target_from_faces`（`crates/areka-emo-present/src/balloon.rs`）。どちらも `&impl ElementDecoder` を受け、同じ `bake` を呼ぶ。
- `SurfaceSet { … }` を直に書いている所はワークスペース全体で約 50 か所ある（atlas・compose・present・text・areka のテストと本番）。**`SurfaceSet` に欄を足すと、その全部が書き換えになる**。
- `bake` の中は「鍵 1 つを読む → 切り詰めた画素を持ち続ける → 全部そろってから詰めて焼く」の形。つまり、**全コマの切り詰め後の画素と、焼いた後のページの両方を同時に抱える瞬間がある**（上限の値を決めるときの前提・3.4 節）。
- `emo2` のシェルの今の表は 55 行・1 ページ（`testdata/emo2_shell_golden.txt` の行数と `page=` の値を数えた）。

### 2.3 読み手を受け取る呼び手（要件 5.5 の「変更 0 ファイル」の対象）

- `&WicDecoderArm`（具体の型）を受ける: `build_shell_assets`（`crates/areka/src/emo2_boot/assets.rs`）、`switch_assets.rs` の 2 か所の `WicDecoderArm::new()`、`build_shell_assets`（`crates/areka/src/placement/measure.rs`）、および examples（`window-placement`・`collision-probe`・`emo-present`・`pilot-balloon-asset-swap`・`emo-text-*`）。
- `&impl ElementDecoder` を受ける: `shell_target.rs` の 3 関数、`balloon.rs` の 2 関数。
- **動く絵の分岐を `WicDecoderArm` の中と `bake` の中に置けば、これらの呼び方は変わらない**（brief の「推す設計」と一致）。新しい型の読み手を作って渡す形にすると、上の全部が変わる。

### 2.4 依存の今

- `image` は `crates/wintf/Cargo.toml` の `[dev-dependencies]` にだけある（`version = "0.25.9"`・`default-features = false`・機能 `png`・`webp`）。使っているのは `crates/wintf/examples/generate_test_image.rs` と `split_image.rs` の 2 本。
- `Cargo.lock` の実測: `image` 0.25.10／`image-webp` 0.2.4／`png` 0.18.1。**`gif`・`weezl`・`color_quant` は lock に無い**（ただし保管庫には `gif-0.14.2`・`weezl-0.1.12`・`color_quant-1.1.0` が展開済み）。
- `about.toml` は `ignore-dev-dependencies = true`。今の `THIRD-PARTY-NOTICES.md` に `image`・`png`・`image-webp` の行は無い（`^- <名前> <版>` で検索して 0 件）。
- `deny.toml` の許可表: MIT・Apache-2.0・Apache-2.0 WITH LLVM-exception・Unlicense・Zlib・0BSD・BSD-2-Clause・BSD-3-Clause・Unicode-3.0。`[sources]` は `unknown-git = "deny"`（**git から取り込む依存は、許可を書き足さない限り `cargo deny check` が落とす**）。

## 3. 議題ごとの裏取り

### 3.1 議題 0: 外部クレートを本番へ足す

**何が本番の木に入るか**（`cargo tree -p wintf` で今の `image` の下を展開した結果に、機能 `gif` の分を足したもの）:

| クレート | 版 | ライセンス（各 `Cargo.toml` の `license`） | 今の謝辞に在るか |
| --- | --- | --- | --- |
| `image` | 0.25.10 | MIT OR Apache-2.0 | 無い |
| `image-webp` | 0.2.4 | MIT OR Apache-2.0 | 無い |
| `png` | 0.18.1 | MIT OR Apache-2.0 | 無い |
| `gif` | 0.14.2（lock に無い・新規） | MIT OR Apache-2.0 | 無い |
| `weezl` | 0.1.12（新規） | MIT OR Apache-2.0 | 無い |
| `color_quant` | 1.1.0（新規） | MIT | 無い |
| `bytemuck` | 1.25.2 | Zlib OR Apache-2.0 OR MIT | 無い |
| `byteorder-lite` | 0.1.0 | Unlicense OR MIT | 無い |
| `moxcms` | 0.8.1 | BSD-3-Clause OR Apache-2.0 | 無い |
| `pxfm` | 0.1.30 | BSD-3-Clause OR Apache-2.0 | 無い |
| `quick-error` | 2.0.1 | MIT/Apache-2.0 | 無い |
| `crc32fast` | 1.5.2 | MIT OR Apache-2.0 | 無い |
| `fdeflate` | 0.3.7 | MIT OR Apache-2.0 | 無い |
| `flate2` | 1.1.10 | MIT OR Apache-2.0 | 無い |
| `simd-adler32` | 0.3.10 | MIT | 無い |
| `num-traits`・`bitflags`・`miniz_oxide`（0.8.9 と 0.9.1）・`adler2` | — | — | 既に在る |

- 謝辞に新しく載るのは **15 本**（うち lock に新しく入るのは `gif`・`weezl`・`color_quant` の 3 本、残り 12 本は開発専用から本番へ移る）。ライセンスは全部、今の `deny.toml` の許可表の中に収まる。**正確な本数は設計の段で `cargo tree -e normal` を取り直して確定すること**（本書の表は wintf の開発用の木から読んだもの）。
- `image` の機能 `gif` の中身は `["dep:gif", "dep:color_quant"]`（`image-0.25.10/Cargo.toml`）。`color_quant` は書き出しにしか使わないが、機能で外せない。
- `image` の `rust-version` は 1.88.0。この機械の cargo は 1.99.0。
- **steering との当たり**: `tech.md` の `miniz_oxide` の項は「zip コンテナの読み手は `areka-nar` が std だけで持つ（`zip`・`flate2` は入れない）」と書く。`png` 0.18.1 は `flate2` と `fdeflate` に依る。あの一文は `areka-nar` の話だが、本番の木に `flate2` が入ること自体は事実として変わるので、登記のときに一言要る。
- `image` の `moxcms`（色の管理）は機能に関わらず必ず入る（`[dependencies.moxcms]` に `optional` が無い）。

**`image` を通さず、下の 3 クレート（`gif`・`png`・`image-webp`）を直に使う道**もある。入る本数は減る（`image`・`bytemuck`・`moxcms`・`pxfm`・`color_quant` が要らなくなる）が、GIF と APNG の重ね合わせを自前で書くことになる（`image` の `GifFrameIterator`・`ApngDecoder::mix_next_frame` が今やっている仕事）。WebP は `image-webp` 自身が重ね合わせる。

### 3.2 議題 1: 動く WebP の透過の欠陥

**欠陥は実在する。公開版 0.2.4 のソースで確かめた原因は 2 つ。**

1. **消す色が既定で「無し」**。`image-webp-0.2.4/src/decoder.rs` の `read_frame` は、前のコマが「背景へ戻す」指定のとき `clear_color = info.background_color` とする。`background_color` の初期値は `None`（`extended.rs` の `background_color: None`）で、`set_background_color` を呼んだときだけ値が入る。つまり**何もしなければ、前のコマは消されず、次の透けたコマの下に残る**。これが image#2913（題「WebP: Transparent animation frames are decoded incorrectly」・2026-10-04 時点で open）の症状。
2. **α を持たないコマのときの消し方が 3 バイト刻み**。`extended.rs` の `composite_frame` は、キャンバスが常に 4 バイト／画素なのに、コマが α を持たない枝で `chunks_exact_mut(3)` や `* 3` の添字で消している。部分矩形のコマでは**違う場所を消す**。

**上流の状態**（2026-10-04 に確認）:

- crates.io の `image-webp` の最新は 0.2.4（2025-08-27 公開）。0.2.5 は無い。
- `release-0.2.5` の枝の `Cargo.toml` は `version = "0.2.5"`。`read_frame` は「背景色が未設定で、コマが α を持つなら透明で消す」に変わり、`composite_frame` は常に 4 バイトで消す形に直っている（0.2.4 との差分を取って確認）。
- **`main` の枝の `Cargo.toml` は `version = "0.2.4"` のまま**で、直しは同じものが入っている。
- image-webp#183（題「Ship a bugfix release」）は open。公開の日付は書かれていない。

**公開版をさかのぼって調べた結果（2026-10-04 要件討議で追加。crates.io の実物の `.crate` を `target\gap-tmp\webp\` へ展開して読んだ）**:

| 版 | 前のコマを消す色（`decoder.rs` の `read_frame` の `clear_color`） | 症状 |
| --- | --- | --- |
| 0.2.0・0.2.1 | ファイルの頭に書かれた背景色（`info.background_color`・`[u8; 4]`） | 背景色が不透明と書かれたファイルでは、透けるべき場所がその色で塗られる（仕様は「無視してよい」とし、ブラウザは透明で消す） |
| 0.2.2・0.2.3・0.2.4 | `Option` の `None`（`set_background_color` を呼ぶまで空） | 消されず、前のコマが残る |

- 原因 2（3 バイト刻みで消す）は 0.2.0〜0.2.4 の全部に在る。
- 0.1.x は `image` 0.25.10 が受け付けない（`image-webp` の要求は `0.2.0` 以上）。上流のタグは `v0.1.0`〜`v0.1.3` と `v0.2.4` だけ。
- **欠陥の無い公開版は 0 本**。開発者の裁定: **当面は GitHub から直接取り込む（道 ⑴）**。

**採れる道と、それぞれで分かったこと**:

| 道 | 中身 | 分かったこと |
| --- | --- | --- |
| ⑴ 未公開の直しを固定して取り込む | `[patch.crates-io]` で git の rev を指す | `release-0.2.5` の枝を指せば版は 0.2.5 で、要件 7.6 の「公開版より上」の検査がそのまま書ける。`main` を指すと版は 0.2.4 で公開版と同じになり、「上であること」では見分けられない（`Cargo.lock` の `source` が git であることを見る検査になる）。`deny.toml` の `unknown-git = "deny"` に許可を足す必要がある。0.2.5 が公開されたら patch を外す作業が残る |
| ⑵ 公開を待ち、動く WebP は 1 枚へ縮める | 動く WebP を見つけたら `warn!` を出して静止画として読む | 作業は最も少ない。**ただし静止画の読み方は WIC のままなので、WebP の拡張機能が無い機械では 1 枚も出ない**（Out of scope の「静止画の WebP の性質は変えない」どおりだが、要件 6 の「少なくとも 1 枚は出る」とは機械しだいになる）。縮めた 1 枚だけを `image` で読む、という変種もありうる |
| ⑶ 公開を待ち、欠陥のまま読む | 何もしない | 要件 2.7 を満たさない期間ができる |
| ⑷ **公開版のまま、呼び方で避ける**（要件に無い道） | `image::codecs::webp::WebPDecoder::set_background_color(Rgba([0,0,0,0]))` を読む前に呼ぶ | 原因 1 はこれで消える（消す色が「透明」になる）。原因 2 のうち「全面のコマ」は、消す色が全部 0 なので 3 バイト刻みでも結果が同じ。**残るのは「α を持たない部分矩形のコマが、背景へ戻す指定のコマの次に来る」場合だけ**で、ここは 0.2.4 では違う場所が消える。α を持つコマ（透過のある動く WebP＝要件 2.7 の対象）は正しくなる見込み。patch も git の許可も要らない |

- ⑷ はソースを読んだ上での見立てで、**動かして確かめてはいない**。設計の段で、要件 8.3 の「透過のあるコマを持つ動く WebP」の検体を先に作り、公開版 0.2.4 に ⑷ を当てて通るかを確かめると、道の選び方が決まる（下の「調べ残し」1）。
- 0.2.5 でも「背景色が未設定で、コマが α を持たない」ときは消さない（`(_, true) => Some([0,0,0,0])`・それ以外は `None`）。つまり **0.2.5 を取り込んでも、`set_background_color` を呼ぶかどうかは別に決めることになる**。

### 3.3 議題 2: GIF の繰り返し回数

**事実**（`gif-0.14.2/src/reader/mod.rs`・`encoder.rs`、`image-0.25.10/src/codecs/gif.rs`）:

| ファイルの中身 | `gif::Decoder::repeat()` | `image` の `loop_count()` |
| --- | --- | --- |
| 繰り返しの指定（NETSCAPE2.0 の塊）が無い | `Repeat::Finite(0)`（`Repeat::default()`） | `LoopCount::Infinite` |
| 指定が 0 | `Repeat::Infinite` | `LoopCount::Infinite` |
| 指定が N（1 以上） | `Repeat::Finite(N)` | `LoopCount::Finite(N)` |

- 要件の言うとおり、**`image` の口では「指定なし」と「終わりなく繰り返す」が区別できない**（`gif::Repeat::Finite(0) | gif::Repeat::Infinite => LoopCount::Infinite`）。
- **`gif` の口では区別できる**。しかも `repeat()` は頭の読み込み（`Decoder::init`）が終わった時点で決まる（NETSCAPE の塊は最初のコマより前にあるのが普通で、`init` は最初のコマの手前まで読む）。
- つまり「正確に読む」ための追加は、**`gif::DecodeOptions::new().read_info(ファイル)` を開いて `repeat()` を聞く数行**で足りる。重ね合わせは `image` に任せたままでよい。`gif` は機能 `gif` で既に木に入るので、`Cargo.toml` に直の依存を 1 行足すだけで、入るクレートは増えない。
- **N の意味が形式で違う**（要件 2.5「3 形式で同じ意味」に効く）:
  - APNG の `num_plays` と WebP の繰り返し回数は「全体を合計 N 回」（0 は終わりなし）。`image` は APNG を `Finite(num_plays)`、WebP を `Finite(n)` でそのまま渡す。
  - GIF の NETSCAPE の N は「1 回目の後に繰り返す回数」と読む実装が多い（合計 N+1 回）。`image` は N をそのまま渡す。**GIF だけ +1 するかどうかを設計で決め、対応表に書く必要がある**。
- 下流（`areka-P0-animated-image-playback`）が繰り返し回数を使わないと決めるなら、この追加は要らない。ただし本 spec の要件 2.6 と 8.3（繰り返しの指定の無い GIF のテスト）は今の文面のまま残っている。

**同じ数行で、GIF が動くかどうかの安い見分けもできる**: `DecodeOptions::skip_frame_decoding(true)` にして `next_frame_info()` を 2 回呼べば、画素を解かずに「2 枚目のコマが在るか」が分かる（`gif-0.14.2/src/reader/mod.rs` の説明「This is useful to count frames without incurring the overhead of decoding.」）。同じ読み方でコマの総数も数えられる。

### 3.4 議題 3: 上限の値

**`image` の口から先に聞けること・聞けないこと**:

| 形式 | 動くかどうか | コマの総数を先に聞けるか | 絵の寸法 |
| --- | --- | --- | --- |
| GIF | 口が無い（`gif` を直に使えば上のとおり安く分かる） | 聞けない（`gif` を直に使って数えることはできる） | `dimensions()` |
| APNG | `PngDecoder::is_apng()`（`acTL` の有無） | 聞けない（`ApngDecoder` の `remaining` は非公開。`png` を直に使えば `animation_control().num_frames`） | `dimensions()` |
| WebP | `WebPDecoder::has_animation()` | 聞けない（`image_webp` を直に使えば `num_frames()`） | `dimensions()` |

- コマの並びは 3 形式とも遅延評価（`into_frames()` が返すのは反復子で、1 枚ずつ重ねて返す）。**「1 枚ずつ読みながら数え、上限を超えた時点で打ち切る」形なら、`image` の口だけで要件 6.3 を満たせる**。打ち切るまでに抱えるのは上限ぶんまで。
- `image::Limits` はコマの数を見ない。GIF と APNG は 1 コマぶんの確保を `Limits` に照らすが、既定は `Limits::no_limits()`（`GifDecoder::new`・`PngDecoder::new`）。

**メモリの実数**（1 画素 4 バイト。`bake` は切り詰め後の画素と焼いた後のページを同時に持つので、山は下の表のおよそ 2 倍まで）:

| 画素の量の上限 | バイト | ページ換算（2048²＝16 MiB） | 336×400 のコマ（`emo2` の面の寸法） | 500×500 のコマ |
| --- | --- | --- | --- | --- |
| 16,777,216 画素 | 64 MiB | 4 ページ | 124 枚まで | 67 枚まで |
| 67,108,864 画素 | 256 MiB | 16 ページ | 499 枚まで | 268 枚まで |
| 268,435,456 画素 | 1 GiB | 64 ページ | 1,997 枚まで | 1,073 枚まで |

- brief の目安「500×500 で 1,000 コマなら約 1 GB」は 250,000 × 4 × 1,000 ＝ 1,000,000,000 バイトで合っている。
- 透明な余白は `Trimmer::trim` が削るので、実際にページを食う量は上の数字より小さい。逆に、**上限の判定を「幅 × 高さ × 枚数」でやるなら余白込みの数字で判定することになる**（要件 6.1 の定義どおり）。
- 目安: 30 コマ／秒で 10 秒の絵は 300 コマ。
- **ページの一辺を超えるコマは今でも焼けない**: 余白込みで 2048 を超える矩形は `Packer::pack` が `error!` を出して外す。つまり一辺が 2,046 画素を超える動く絵は、上限とは別にここで落ちる。動く絵でこれが起きたときに 1 枚へ縮めるのか、今の静止画と同じ扱いにするのかは決まっていない。
- **詰める手間**: `Packer::pack` はページ数を 1 から増やしながら、毎回全部の矩形を詰め直す。数百コマ・数十ページになると詰め直しの回数が増える。今の実測は `emo2` の 55 枚・1 ページだけで、数百枚での時間は測っていない（調べ残し 3）。
- **シェル全体の合計の上限**（議題 3 の後半）を置く場合、`bake` は鍵を順に処理するので「何番目の絵で超えたか」が鍵の並び順で決まる（決定的ではある）。

### 3.5 議題 4: 透明度の情報を持たない動く絵の抜き色

**`image` が返すコマの形**: 3 形式とも RGBA 8 ビット・絵の全体の寸法。透明度の情報を持つかどうかは、コマの画素からは直接は分からない。

| 形式 | 「透明度の情報を持つか」を聞ける口 | 補足 |
| --- | --- | --- |
| APNG | `PngDecoder::color_type()` が α つき（`La8`・`Rgba8`）か | 読む前に分かる |
| WebP | `WebPDecoder::color_type()`（中は `image_webp` の `has_alpha()`＝ファイルの頭の旗） | 読む前に分かる。旗が立っていなければ `image` は RGB で読んで不透明の RGBA へ変える |
| GIF | **口が無い** | `image` の `GifFrameIterator` は透明な地（`Rgba([0,0,0,0])`）から始め、背景色は使わない（ソースの注記「intentionally ignore the background color for web compatibility」）。透明色の指定が無くても、1 枚目のコマが絵の全体を覆っていなければ透明な画素が出る。`gif` を直に使えばコマごとの `transparent` の欄で分かる。または「全コマの画素を見て α が 255 でないものが 1 つでも在るか」で決める手もある |

**今の抜き色の仕組みとの関係**:

- `Normalizer::key_color`（`normalize.rs`）は、**渡された 1 枚の絵の左上**から色を取る。「1 枚目のコマの色を全コマに使う」（要件 3.3 の仮の案）を、今の `Normalizer` の口のままでは表せない。
- 道は 2 つ: ⑴ 動く絵の読み手の中で 1 枚目の左上の色を全コマから抜き、`has_alpha = true` で渡す（`normalize.rs` は無改変。ただし `bake` が出している「抜き色で透過しました」の `debug!` は通らなくなる）／⑵ `Normalizer` に「抜く色を外から渡す」口を足す（`normalize.rs` が変わる。並走の約束の 3 ファイルには入っていない）。
- 「コマごとに左上を取り直す」案なら、今の `Normalizer` をコマごとに呼ぶだけで済む（追加 0）。
- **今の WIC が GIF をどう返しているかは未確認**（`has_alpha` が偽になって抜き色の腕に落ちているのか、透明色が α として届いているのか）。検体が 0 枚なので、今の見た目の基準が手元に無い（調べ残し 2）。

### 3.6 議題 5: 途中まで読めた動く絵・16 ビットの APNG

- **16 ビットの APNG**: `ApngDecoder::animatable_color_type`（`image-0.25.10/src/codecs/png.rs`）が `L16`・`Rgb16`・`La16`・`Rgba16` を `Unsupported` で断る。ソースの注記は「We further do not support compositing 16-bit colors」。**1 枚目のコマを読む時点で失敗が返る**ので、「途中まで読めた」にはならず、必ず 0 枚で失敗する。要件 6.4 の「1 枚へ縮める」なら、WIC が既定の 1 枚を 8 ビットへ変えて返す（今と同じ）。
- **途中のコマが壊れている**: APNG は失敗すると以後のコマを返さない（`mix_next_frame` が「Shorten ourselves to 0 in case of error」）。GIF は失敗を返した後も反復子が続くことがある（`is_end` は読み込みが尽きたときだけ立つ）ので、**呼ぶ側が最初の失敗で止める**必要がある。WebP は `read_frame` が失敗を返す。
- どの形式でも「失敗するまでに読めたコマ」は手元に残るので、「読めたコマまでで動く絵にする」案も作れる。そのときは、繰り返しの切れ目で絵が飛ぶ（最後のコマが欠ける）ことと、同じ壊れたファイルから毎回同じ枚数になること（要件 2.8）を確かめる必要がある。
- **APNG の「既定の絵」**: APNG は、動きに含めない既定の絵（サムネイル）を持てる。`image` の `ApngDecoder` はそれを飛ばす（`has_thumbnail`）。つまり**その種の APNG では、動く絵の 0 番のコマと、WIC が今返している 1 枚（既定の絵）が別の絵になる**。要件 4.3「0 番のコマを返す（今までどおり 1 枚のコマが出る）」は、この種の APNG では「今までと同じ絵」にならない。1 枚へ縮めたとき（WIC が読む）と動く絵として読んだときで、出る 1 枚が違うことにもなる。

## 4. リポジトリ内の動く絵の数（要件 5.3）

**結果: 0 枚。**

| 範囲 | 調べたファイル | 画像 | PNG | APNG | GIF | WebP | 動く絵（コマ 2 枚以上） |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `vendors/sample_ghost/`（`.nar` 7 本の中身を含む） | 489 | 243 | 243 | 0 | 0 | 0 | **0** |
| ワークツリー全体（`target`・`.git` を除く。`.nar`・`.zip` の中身を含む） | 4,700 | 332 | 325 | 0 | 0 | 7 | **0** |

- 検体は `.nar`（zip）で置かれているので、**中身を展開して 1 つずつ見た**（展開しないと 0 枚に見えて当たり前になる）。
- **数え方**: 拡張子ではなく中身で見る。GIF はブロックをたどって画像の記述子の数を数える。PNG はチャンクをたどって `IDAT` より前の `acTL` を探す。WebP は `VP8X` の動きの旗と `ANMF` の数を見る。道具は `target\gap-tmp\count-animated.ps1`（一時ファイル）。
- **較正**:
  - GIF: Windows に付いている実物の動く GIF（`C:\Windows\SystemResources\Windows.UI.ControlCenter\Assets\Images\StudioEffectsBackgroundBlur_Color.gif` ほか 5 枚）を同じ道具に通し、6 枚とも「動く」と出た（1 枚は 250 コマ）。
  - WebP: リポジトリ内の実物の WebP 7 枚（`crates/wintf/tests/assets/seikatu*.webp`）を WebP と認め、7 枚とも静止画と判定した。**動く WebP の実物での当たりは取れていない**（この機械で見つからなかった）。
  - APNG: **実物での当たりは取れていない**（同上）。
- **当たりが取れていない 2 形式を補う別の数え方**: 画像 333 個（`.nar` の中身を含む）の生のバイト列から、`acTL`・`fcTL`・`ANMF`・`ANIM`・`NETSCAPE2.0`・`GIF8` の 6 つの綴りをそのまま探した。**6 つとも 0 件**。この探し方は実物の動く GIF で `GIF8` と `NETSCAPE2.0` が当たることを確かめた。チャンクのたどり方に誤りがあっても、綴りそのものが 1 つも無いので、0 枚は動かない。
- したがって要件 5.3 の「0 枚でなければ絵ごとに記す」の対象は無い。**同時に、今の WIC が動く絵をどう返しているか（寸法・透過）を確かめる実物がリポジトリに 1 枚も無い**ことも意味する。

## 5. 要件と今のコードの対応

凡例: **無い**＝作るもの／**制約**＝今の形が縛るもの／**不明**＝調べ残し。

| 要件 | 今あるもの | 差 |
| --- | --- | --- |
| 1.1〜1.4 見分け | 無し。WIC が中身で形式を決めて 1 枚返すだけ | **無い**: 中身で 3 形式と「2 枚以上」を見分ける口。GIF だけ `image` に安い口が無い（3.3 節） |
| 1.5 静止画を 2 回解かない | `bake` は鍵 1 つに `decode` 1 回 | **制約**: 見分けは頭の数十バイト〜チャンクの走査で済ませる。静止画の PNG はシェルの大半（`emo2` で 55 枚）なので、ここで全体を解くと起動が倍になる |
| 2.1〜2.4 コマと待ち時間 | 無し | **無い**。待ち時間は GIF が 10 ms 刻み、WebP が整数 ms、APNG が分数（`delay_num × 1000 / delay_den`・分母 0 は 100 と読む）。**端数が出るのは APNG だけ**。`image` の `Delay::numer_denom_ms()` は約分した分数を返す |
| 2.5〜2.6 繰り返し回数 | 無し | **無い**。GIF の「指定なし」は `gif` を直に聞く必要あり。GIF の N の意味が他と違う（3.3 節） |
| 2.7 動く WebP の透過 | 無し | **制約**: 公開版 0.2.4 に欠陥 2 つ（3.2 節） |
| 2.8・4.7 決定的 | `bake` は決定的（`lib.rs` の説明） | 読み手は状態を持たないので保てる |
| 3.1〜3.5 透過 | `Normalizer` の 2 つの腕（α 採用・抜き色） | **制約**: 抜き色は「その 1 枚の左上」しか見ない（3.5 節）。`image` の画素は乗算前の RGBA で、変換が要る（2.1 節）。全画素が透明なコマは、今の `bake` が `warn!`（「全透明…制作者ミスの可能性」）を出す。動く絵の正当な 1 コマにこの `warn!` を出すかどうかは決まっていない |
| 4.1〜4.4 表でコマを引く | `AtlasTable` は鍵 1 つにエントリ 1 つ | **無い**: コマの欄と引く口（6 節の案） |
| 4.5 形を変えない | `AtlasKey`・`manifest.rs`・`AtlasTable::new` | **制約**: `new` の逆引き表は同じ鍵が 2 回あると後が勝つので、コマを足す組み立ては別の口にする |
| 4.6 静止画は枚数 1 | 無し | **無い** |
| 5.1〜5.2 静止画は不変 | `emo2_golden.rs` と `testdata/emo2_shell_golden.txt` | 動く絵が 0 枚の集合では、番号・位置とも変わらない作りにできる。動く絵を含む集合では、静止画の**位置**は詰め直しで変わる（番号は、コマを末尾へ足せば変わらない） |
| 5.3 検体の数 | — | **0 枚**（4 節） |
| 5.4 バルーンは不変 | `bake` はシェルとバルーンで共用 | **制約**: 見分ける欄が無い（下の 5.3 項） |
| 5.5 呼び手は不変 | 2.3 節 | 分岐を `WicDecoderArm` と `bake` の中に置けば 0 ファイル |
| 6.1〜6.7 上限と縮退 | `BakeError` は 2 種。`bake` は失敗しても次へ進む | **無い**: 上限・縮退の経路・`warn!`。縮退は「失敗」ではないので `BakeResult.errors` に載せるかどうかは決まっていない |
| 7.1〜7.6 依存の登記 | `tech.md` の Key Libraries・`deny.toml`・`about.toml` | 3.1 節。`unknown-git = "deny"` が道 ⑴ に効く |
| 8.1〜8.6 検体とテスト | 検体 0 枚。偽の読み手 `MemoryDecoder` | **無い**: 3 形式の検体（自作）。`MemoryDecoder` に動く絵を登録する口。WebP のテストは `image` で読むので Windows の拡張機能に依らない。**1 枚へ縮めたときの WebP は WIC が読む**ので、そのテストだけは拡張機能の有無に左右される（要件 8.6 と当たる） |
| 9.1〜9.4 記録 | — | 9.3: `--clipping` を引き受ける spec は**実在しない**。`areka-P0-animated-image-playback` の brief は element のオプションを Out と書き、`roadmap.md` に element のオプションを読む spec の行は無い（`clipping` で `.kiro/steering/` と全 brief を検索し、当たったのは本 spec と playback の brief の 2 つだけ）。起票が要る |

### 5.3 項: バルーンと `bake` の共用

- `bake` はシェル（`shell_target.rs`）とバルーン（`balloon.rs`）の両方から同じ形で呼ばれ、`SurfaceSet` にはどちらかを示す欄が無い。
- 動く絵の分岐を `bake` の中に置くと、**バルーンの面に動く絵が置かれていれば、それも全コマ焼かれる**。画面に出るのは 0 番のコマだけなので、GIF と WebP なら見た目は今とほぼ同じ（メモリは余分に使う）。**APNG で既定の絵を持つものは、出る 1 枚が変わる**（3.6 節）。要件 5.4「バルーンの絵の見た目を変えない（変更 0）」と当たる。
- 道:
  - ⑴ バルーンも同じに扱い、要件 5.4 の例外として書く（コードの追加 0）。
  - ⑵ `bake` に並ぶ新しい入口（動く絵を読む集合を名指しする引数つき）を足し、シェルの側だけがそれを呼ぶ。今の `bake` は「動く絵を読まない」まま残す。このとき `shell_target.rs` の 1 行が変わる（要件 5.5 の「変更 0 ファイル」の対象に `shell_target.rs` が入っているので、文面と当たる）。
  - ⑶ `SurfaceSet` に欄を足す（約 50 か所が書き換えになるので現実的でない）。
  - ⑷ 既定の絵を持つ APNG では、0 番のコマとして既定の絵（WIC の 1 枚）を使い続ける、などで見た目だけ揃える（決まりが増える）。

## 6. 作り方の案

### 案 A: 今の部品を広げる（brief の「推す設計」）

- `ElementDecoder` に既定の実装つきのメソッドを 1 つ足す（例: 「動く絵として読む。動く絵でなければ『違う』を返す」。既定は常に『違う』）。
- `WicDecoderArm` がそのメソッドを実装し、中で `image` を呼ぶ。新しいファイル `decode/<動く絵の読み手>.rs` に `image` を使う部分を閉じ込める。
- `MemoryDecoder` に動く絵を登録する口を足す。
- `bake` は鍵ごとにまずそのメソッドを呼び、『違う』なら今までどおり `decode` を呼ぶ。動く絵なら各コマを正規化・切り詰めして詰める。
- `AtlasTable` に、`bake` だけが呼ぶ別の組み立ての口と、コマを引く口を足す。

**利点**: 呼び手は 0 ファイル。`AtlasKey`・`manifest.rs`・`AtlasTable::new` は無改変。並走の `surface-element-nesting` と共有 0。
**欠点**: `WicDecoderArm` という名前の型が WIC でない読み方も持つ。`lib.rs` は今 614 行（うちテストが約 380 行）で、`bake` に枝が増える（1 ファイル 1,000 行の決まりに対して、テストを兄弟ファイルへ出す余地はある）。バルーンも巻き込む（5.3 項）。

### 案 B: 新しい読み手の型を作って合成する

- 例: 「WIC の読み手を包み、動く絵は `image` で読む」新しい型を作り、本番の組み立て場所でそれを渡す。

**利点**: `WicDecoderArm` は WIC だけのまま。責務がきれいに分かれる。
**欠点**: `&WicDecoderArm` を受ける本番 3 ファイル（`assets.rs`・`switch_assets.rs`・`measure.rs`）と examples が変わる（要件 5.5 に反する）。brief の棚卸⑳がこの理由で退けている。

### 案 C: 混ぜる（A を土台に、足りない所だけ別の口）

- 読み手は案 A。
- 焼く入口は、今の `bake` を残したまま、動く絵を読む新しい入口を並べる（5.3 項の道 ⑵）。シェルの側だけが新しい入口を呼ぶ。
- GIF の「動くか」「繰り返し回数」だけ `gif` を直に聞く（3.3 節）。

**利点**: バルーンの見た目が変わらないことが構造で決まる。GIF の見分けが安い。
**欠点**: `shell_target.rs` が 1 行変わる（要件 5.5 の文面と当たるので、要件討議で確かめる）。入口が 2 つになる。

### 表の中のコマの持ち方（どの案でも決める）

| 持ち方 | 中身 | 効くこと |
| --- | --- | --- |
| ㋐ コマも `ElementId` を持つ。2 枚目以降は鍵の列の**後ろ**へ足し、鍵の欄には親と同じ `AtlasKey` を入れる。別の欄に「親の `ElementId` → コマの `ElementId` の列・待ち時間の列・繰り返し回数」を持つ | 0 番のコマは親の鍵のエントリそのもの | 合成の側は `entry(ElementId)` で静止画と同じに描ける（要件 4.4）。静止画の番号は今と同じ。`len()` はコマの数だけ増える（表の全エントリをなめる `emo2_golden.rs` の `snapshot_table` のような読み手は、同じ鍵が並ぶのを見る）。逆引き表は親の鍵が 0 番を指すように別の口で組む |
| ㋑ コマは別の列に持ち、別の番号の型で引く | `entries` は今のまま鍵と 1 対 1 | 表の今の読み手は何も変わらない。合成の側は `ElementId` でコマを描けないので、下流（playback）で合成の側に手が入る |
| ㋒ コマに作り物の相対パス（例「`face.gif` ＋区切り＋番号」）を振って普通の鍵にする | `AtlasTable::new` だけで済む | 実在のファイル名と当たりうる（開発者の方針「重複判定の鍵に連結した綴りを使わない」に反する） |

## 7. 規模とリスク

- **規模: M（3〜7 日）**。読み手の口・`bake` の枝・表の欄・検体 3 形式・依存の登記はどれも今の型に沿って足せる。brief の見立て（11〜15 タスク）と合う。
- **リスク: 中**。理由は 3 つ: ⑴ 動く WebP の欠陥の道が未定で、道 ⑴ なら git の依存と `deny.toml` の変更が入る ⑵ 検体が 0 枚で、今の WIC の返し方との差（GIF の寸法・APNG の既定の絵・画素の丸め）を実物で比べられていない ⑶ 数百コマを詰めたときの時間を測っていない。

## 8. 設計の段へ持ち越すこと

### 決めること（要件討議の材料）

1. **依存の承認**（議題 0）: 謝辞に新しく載るのは約 15 本（3.1 節の表）。`flate2` が本番の木に入る。`image` を通さず下の 3 クレートを直に使う道もある（重ね合わせを自前で書く代わりに 5 本減る）。
2. **動く WebP**（議題 1）: 道 ⑴〜⑶ に加えて、**公開版のまま `set_background_color` を透明で呼ぶ道 ⑷**がある。道 ⑴ を採るなら、取り込む枝は `release-0.2.5`（版 0.2.5）か `main`（版 0.2.4 のまま）かで、要件 7.6 の検査の形が変わる。
3. **GIF の繰り返し回数**（議題 2）: `gif` を頭だけ読む数行で正確に取れる。あわせて **GIF の N に 1 を足すか**（他の 2 形式は「合計 N 回」）。
4. **上限の値**（議題 3）: 3.4 節の表から選ぶ。コマの枚数の上限を画素の量の上限と別に置くか。シェル全体の合計を置くか。**ページの一辺（2,046 画素）を超える動く絵をどう扱うか**。
5. **抜き色**（議題 4）: 仮の案（1 枚目の色を全コマに）は、読み手の中で抜くか `Normalizer` に口を足すかの 2 通り。GIF の「透明度の情報を持つか」の見分け方（`gif` の `transparent` の欄か、画素を見るか）。
6. **途中まで読めた絵**（議題 5）: 16 ビットの APNG は必ず 0 枚で失敗するので「1 枚へ縮める」しか無い。GIF は最初の失敗で呼ぶ側が止める。
7. **バルーンの動く絵**（新しく見つけた・5.3 項）: `bake` が共用なので、何もしなければバルーンも全コマ焼かれる。要件 5.4・5.5 の文面のどちらかと当たる。
8. **既定の絵を持つ APNG**（新しく見つけた・3.6 節）: 0 番のコマと、今 WIC が出している 1 枚が別の絵になる。要件 4.3 の「今までどおり」の読み方を決める。
9. **表の中のコマの持ち方**（6 節の ㋐〜㋒）。
10. **全画素が透明なコマの `warn!`**: 今の `bake` は全透明の絵に「制作者ミスの可能性」と出す。動く絵のコマ（要件 3.5 が正当と認めるもの）にも出すか。
11. **縮退した絵を `BakeResult.errors` に載せるか**: バルーンの側は `errors` が空でないと構築の失敗にする（`build_balloon_target_from_faces`）。縮退を `errors` に載せると、バルーンでは 1 枚も出なくなる。
12. **`--clipping` を引き受ける spec の起票**（要件 9.3）: 実在しない。

### 調べ残し

1. 透過のある動く WebP の検体を作り、公開版 0.2.4 に道 ⑷ を当てて正しく重なるかを動かして確かめる（本書はソースを読んだ見立て）。
2. 今の `WicDecoderArm` が動く GIF・APNG・WebP の 1 枚目をどう返すか（寸法がコマの矩形か絵の全体か・`has_alpha` の値・画素）を、自作の検体で測る。`image` の 0 番のコマとの差が、要件 4.3 の「今までどおり」に効く。
3. 数百コマ・数十ページを `Packer::pack` に通したときの時間。
4. `image` の画素（乗算前）を乗算済みへ変えるときの丸めを、WIC の結果と揃える必要があるか（揃えないと、同じ絵を静止画として読んだときと画素が 1 ずれうる）。
5. 本番の木に入るクレートの正確な一覧（`cargo tree -e normal` を依存を足した状態で取り直す）。
6. `image-webp` 0.2.5 の公開の有無（着手時に crates.io を引き直す）。
