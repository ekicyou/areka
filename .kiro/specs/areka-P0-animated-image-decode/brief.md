# Brief: areka-P0-animated-image-decode

> 2026-10-01 `/kiro-discovery` で起票（開発者指示「アニメーションをサポートする画像ファイルに対応して欲しい。画像読み込み時にサブエレメント分解して、アニメーション表示を回す案。webp形式サポートとかがよいかな？apngもあるけど。時期はα後」）。roadmap「動く画像」節の 1 本目＝**読み込みの側**。再生は `animated-image-playback`。
> 本文の file:line は起票時（main `5e37745e`）の実測＝**着手時に引き直すこと**。

## Problem

- **シェルの作者**: 動く GIF・APNG・WebP をシェルの絵に使っても、areka では 1 枚目のコマしか出ない。正典（ukadoc の element定義の項）は「surface*.pngまたはelement定義にアニメGIF/APNG/WebPアニメを指定すると、SERIKO定義を書かなくても自動的にアニメーションする(SSP 2.7.38～)」と定める。SSP で動くシェルが areka で止まる＝黙って壊れる。
- 再生（`animated-image-playback`）の前に、全部のコマと待ち時間を読める口が要る。

## Current State（起票時の実測）

- 本番の読み込みは WIC だけ: `WicDecoderArm`（`crates/areka-emo-atlas/src/decode/wic_arm.rs`）が **`GetFrame(0)` で 1 枚目のコマしか読まない**。口は trait `ElementDecoder`（`crates/areka-emo-atlas/src/decode.rs`）・結果は `DecodedImage {width,height,stride,bgra,has_alpha}`＝1 枚の絵で、コマ・待ち時間・繰り返し回数の欄が無い。偽実装 `MemoryDecoder` がある。
- 本番の組み立て場所: `crates/areka/src/emo2_boot/assets.rs`・`switch_assets.rs`・`crates/areka/src/placement/measure.rs`。
- 拡張子の門は無く、WIC が中身で判別する。ただし**ファイル名から面を見つける道は `.png` だけ**（`crates/areka-emo-present/src/balloon.rs` の `FRAME_SUFFIX`・`face_digits_of`＝シェルの `surfaceN.png` とバルーンの面の両方が共有）。
- アトラス: `bake`（`crates/areka-emo-atlas/src/lib.rs`）が 1 ファイルを 1 回読み、鍵 `AtlasKey {set, rel_path}`（`table.rs`）で引く＝**コマの番号の欄が無い**。焼いた後は不変・ページは 2048² で満ちれば足す（`pack.rs`）。
- 依存: `image` 0.25（`png`・`webp` の機能）は `crates/wintf/Cargo.toml` の **dev-dependency** で、example 2 本だけが使う。`gif` クレートは Cargo.lock に無い。WIC は APNG を読めない（既定の 1 枚だけを返す）。WebP は Windows の拡張機能の有無で変わる（完了 `ukagaka-desktop-mascot` の design も同じ見立て）。
- 網羅台帳: element定義の行は `degraded`（`overlay` 以外を読み捨て）。動く画像の自動アニメーションを持つ行は無い（完了 `shell-implicit-surface` の要件が「アニメ GIF／APNG／WebP の自動アニメーションも持たない」と明記して外した）。

## Desired Outcome

- element定義・`surface*.png` の画像が動く GIF・APNG・WebP のとき、**重ね済みの全部のコマ・コマごとの待ち時間（ms）・ファイルの繰り返し回数**が読め、全部のコマがアトラスに載り、コマの番号で引ける。
- 静止画は今までどおり WIC で読み、見た目もアトラスの鍵も変わらない。

## Approach（2026-10-01 discovery の開発者確定事項）

1. **3 形式を同時に入れる**（GIF・APNG・WebP）。`image` クレートは 3 形式とも重ね済みのコマと待ち時間を同じ形で返すので、1 形式に絞っても手間は変わらない。
2. **動く絵だけ `image` で読み、静止画は WIC のまま**。動くかどうかはファイルの頭で見分ける（APNG の `acTL`・GIF の 2 枚目のコマ・WebP の `ANIM`）。
3. **`image` を本番の依存へ移し `gif` の機能を足す**＝`tech.md` の「意図的依存追加」への登記と開発者の承認が要る（要件の段で承認をもらう）。`deny.toml`・`THIRD-PARTY-NOTICES.md` も追随。
4. コマの鍵は「ファイルの道＋コマの番号」。合成側（`animated-image-playback`）からは各コマが普通の画像の element と同じに見える形にする（読み込み時にサブエレメントへ分解する＝開発者の案）。

## Scope

- **In**: 読み込みの口の複数コマ化（`ElementDecoder`／`DecodedImage` か、それに並ぶ口）・`image` による 3 形式の読み込み・動くかどうかの見分け・アトラスの鍵へのコマの番号・全部のコマの焼き込み・検体（3 形式の小さな動く絵をリポジトリ内に）・決定論テスト・依存の登記。メモリの上限（コマ数・総画素の上限を超えたら 1 枚目だけにしてログに残す）。
- **Out**: 再生・時計・SERIKO への写し・`import` メソッド・`always`（`animated-image-playback`）。`.png` 以外の名前での面の発見（`surface0.webp` など＝正典が求めるか要件で確かめる）。`--clipping` など element のオプション（未実装のまま）。

## Boundary Candidates

- 読み込みの口（`areka-emo-atlas` の decode）と、アトラスの鍵と焼き込み（`manifest.rs`・`table.rs`・`lib.rs`）。

## Out of Boundary

- 合成（`areka-emo-compose`）と seriko の時計には触らない。

## Upstream / Downstream

- **Upstream**: α の完成宣言（`alpha-release-signoff`）。
- **Downstream**: `animated-image-playback`。`mcp-dump-images`（合成の結果を読み戻すだけ＝隣接）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**: `shell-balloon`（バルーンの面の発見を共有する `balloon.rs`）・完了 `shell-implicit-surface`（同じファイル群）。

## 依存の裏取り（2026-10-01 discovery のサブエージェント調査・着手時に版を引き直すこと）

- **GIF・APNG は計画どおり成り立つ**。`image` 0.25.10 の 3 つの読み手（`GifDecoder`・`PngDecoder::apng()`・`WebPDecoder`）はどれも `AnimationDecoder::into_frames()` で、画面いっぱいに重ね済みの RGBA8 のコマと待ち時間を返す（遅延評価のイテレータ）。繰り返し回数は `loop_count()`。
- **⚠ 動く WebP の透過に欠陥**。crates.io の最新 `image-webp` 0.2.4 では、透過のあるコマが前のコマに重なって描かれる（背景に戻す処理が効かない＝image#2913）。直しは上流の main と `release-0.2.5` の枝にだけあり、0.2.5 はまだ公開されていない（image-webp#183）。採れる道は 2 つで、要件の段で決める。
  - 公開を待つ。
  - `[patch.crates-io]` で rev を固定する。その場合は版が 0.2.4 より上であることを確かめる（記憶「版が合わないと cargo は黙って crates.io 版を引く」）。
- **細部の注意**:
  - APNG は `ImageReader` 経由では読めない（image#3038）＝`PngDecoder::apng()` を直に呼ぶ。16 bit の APNG は断られる。
  - GIF の繰り返し回数:
    - NETSCAPE の塊が無い GIF も `Infinite` になる。
    - N は「1 回目の後に繰り返す回数」で、`image` は N をそのまま渡す。
    - 正確に扱うなら `gif` クレートを直に呼ぶ（`gif::Decoder::repeat()`）。
  - GIF が動くかどうかの安い見分けは `image` には無い。コマを 2 枚読むか、`gif` の `skip_frame_decoding` を使う。
  - 待ち時間 0 を丸めるのは呼ぶ側。
- **メモリ**: `image::Limits` にコマ数の上限は無い（`collect_frames()` は上限を見ない＝image#2109）＝コマを 1 枚ずつ流し、「コマ数 × 幅 × 高さ × 4」の上限は自前で持つ（500×500 で 1,000 コマなら約 1 GB）。
- **ライセンス**: `gif` の機能で新しく 3 クレート（`gif` 0.14・`weezl` 0.1・`color_quant` 1.1）。どれも依存を持たない。`color_quant` だけ MIT 単独（`deny.toml` の許可表に MIT あり＝通る）、他は MIT／Apache-2.0。RustSec の勧告は `image` の 2019〜2020 年の古いもの（0.23 未満）だけ。

## Constraints

- 新しい依存は開発者の承認が先（`tech.md` の登記）。1 ファイル 1,000 行。決定論テスト網羅は必達。ログ無しの失敗の経路を作らない（読めないコマ・上限超えは `warn!`／`error!`＋1 枚目へ縮退）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 M（11〜15 タスク）。コードの面では陳腐化していない（起票の後に `areka-emo-atlas` へ入ったコミットは 0）。
- **足りなかった事実**: ⑴ decoder の trait には `probe_pna` もある。⑵ 本番の 3 か所（`emo2_boot/assets.rs`・`emo2_boot/switch_assets.rs`・`placement/measure.rs`）は**具体の型 `&WicDecoderArm`** を受け取る＝新しい型の decoder を作るとこの 3 ファイルと examples まで変わる。動く絵の分岐を `WicDecoderArm` の中に入れれば（trait に既定の実装つきのメソッドを足す形）、呼ぶ側は 0 ファイルで済む。
- **⚠ brief の「アトラスの鍵にコマの番号」は波及が大きい**: `AtlasKey{set, rel_path}` に欄を足すと、構造体を直に書いている `manifest.rs`・`emo2_golden.rs` と、**`areka-emo-compose` の `blit.rs`（7 か所）・テスト 2 本**まで変わり、「合成器に触らない」と矛盾する。**コマの番号は `table.rs` の内側の索引に持たせ、`AtlasKey` は変えない**設計を推す。そうすれば `surface-element-nesting` との共有（`manifest.rs`）も消える。
- **依存**: `image` 0.25.10 は今 `crates/wintf` の dev-dependency だけ（機能は `png`・`webp`）。本番へ移すと、その木全体が `THIRD-PARTY-NOTICES.md` に入る（`about.toml` は dev の依存を無視する設定）。`gif`・`weezl`・`color_quant` は lock に無い。`image-webp` は lock 上 0.2.4。
- **触るファイル（推す設計）**: `crates/areka-emo-atlas/src/{decode.rs, decode/wic_arm.rs, decode/<新規>.rs, lib.rs, table.rs}` と検体・`crates/areka-emo-atlas/Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`.kiro/steering/tech.md`。
- **議題**: `image-webp` の動く WebP の透過の欠陥を版上げで待つか patch で固定するか／GIF の繰り返し回数のために `gif` を直に呼ぶか／コマ数と総画素の上限／コマの鍵の持ち方／16 bit の APNG。
- **並べ方**: `mcp-server-core` と同じウェーブに置かない（依存の登記のファイルが重なる）。`surface-element-nesting` とは、両方が上の「推す設計」を守るときだけ共有 0。

## 2026-10-03 ウェーブ C1-⑦（10-03 の組み直し（開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」））

- 段は「優先」（動く画像）。依存を足す席を `mcp-server-core`（C3 へ）から譲り受けた。**上の「推す設計」（`AtlasKey` と `manifest.rs` を変えない・動く絵の分岐は `WicDecoderArm` の中）で作る**＝C3 の `surface-element-nesting` と共有 0。
