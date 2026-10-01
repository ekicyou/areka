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

## Constraints

- 新しい依存は開発者の承認が先（`tech.md` の登記）。1 ファイル 1,000 行。決定論テスト網羅は必達。ログ無しの失敗の経路を作らない（読めないコマ・上限超えは `warn!`／`error!`＋1 枚目へ縮退）。
