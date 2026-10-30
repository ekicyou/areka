# Brief: emo-atlas-palette-alpha

## Problem
シェルやバルーンの作者が、パレットの PNG（色の種類 3）に `tRNS`（パレットの番号ごとの透明度）を付けて、一部の番号だけを透明にした絵を作ることがある。areka はこの透明度を使わず、その絵を「α 無し」として抜き色の道で扱う。そのため、作者が透明にした所が透けず、作者が透明にしていない色が抜き色として透けることがある。作者の意図と違う絵が出る。

## Current State
- `crates/areka-emo-atlas/src/decode/wic_arm.rs` の `pixel_format_has_alpha` は、WIC の変換前の形式が 32 ビット以上の α つきの形式のときだけ真を返す。8 ビットの番号つきの形式（`GUID_WICPixelFormat8bppIndexed` など）は、`tRNS` があっても偽になる。
- 偽になった絵は「α 無し」として抜き色（左上の画素の色など）の道へ回る。WIC は PBGRA32 への変換で `tRNS` の透明度を α に入れているのに、その α を捨てて抜き色で塗り直している。
- 同じ集合が `crates/wintf/src/com/wic.rs` の `pixel_format_has_alpha` にもある（emo-atlas は wintf を参照できないので写している）。
- 見つけたいきさつ: `real-run-log-noise` の調べ（research.md 6 節）。同梱の emo2 の `purple/a/null.png` は 382×547、パレット 2 色、`tRNS` は `ff 00`（番号 0 は不透明・番号 1 は透明）、全画素が番号 1。全部が透明なので、抜き色の道でも結果が同じだった。番号の一部だけが透明な絵では結果が違う見込み（実物ではまだ確かめていない）。
- 要件の討議 8（2026-10-29・開発者が推奨の案で一括承認）で「`real-run-log-noise` では直さない・完了の棚卸で `/kiro-discovery` により起票する」と決めた。

## Desired Outcome
- `tRNS` を持つパレットの PNG（番号つきの形式）は、透明度の宣言を持つ絵として α の道で読まれる。作者が透明にした番号だけが透け、ほかの色は抜き色で抜かれない。
- `tRNS` を持たないパレットの PNG・ほかの α 無しの形式は、今のまま抜き色の道を通る。
- areka の原則（α 付きの絵を正しく使うのが原則・抜き色は譲歩）に沿う。

## Approach
- 変換前の形式だけでは `tRNS` の有無が分からない。そこで、番号つきの形式ではパレット（`IWICPalette`）の色に α が 255 未満のものがあるか（`IWICPalette::HasAlpha`）を見て、α の有無を決める案を第一の候補にする。
- emo-atlas と wintf の 2 つの写しを同じ判定にそろえる（片方だけを直さない）。
- 直す前に赤になる決定論テストを置く。番号の一部だけが透明なパレットの PNG を検体にし、透けた画素・透けない画素を読み戻す。
- 案と、どこに検体を置くかは要件・設計の段で決める。

## Scope
- **In**: 番号つきの形式の α の有無の判定（emo-atlas の `wic_arm.rs` と wintf の `com/wic.rs`）。決定論テスト。`doc/COMPAT_ARCHITECTURE.md` の画像の読み込みの行の更新（必要なら）。
- **Out**: 動く絵（APNG・WebP。`image` クレートの道で、`tRNS` は別に読まれる）。抜き色そのものの決め方。`.pna` を使わない方針（変えない）。

## Boundary Candidates
- 画像を読む層（emo-atlas の WIC の腕）の「α の有無の判定」1 か所。
- wintf の WIC の読みの同じ判定（写し）。

## Out of Boundary
- 焼き込みの全透明の判定と記録（`real-run-log-noise` で、着せ替えの「無し」の絵は参照のされ方で見分けると決めた。抜き色の道かどうかを条件にしていないので、この直しで `null.png` の警告は戻らない）。
- 合成メソッド・描画の層。

## Upstream / Downstream
- **Upstream**: WIC（`IWICPalette`）。完了済みの `areka-P0-animated-image-decode`（動く絵の道の分け方）。
- **Downstream**: シェルやバルーンの絵を読むところ全部（emo-atlas の焼き込み・wintf の絵の読み込み・メニューのサムネイルの絵の道を足す `menu-thumbnail-preview`）。

## Existing Spec Touchpoints
- **Extends**: なし。
- **Adjacent**: `compose-f16-precision`（同じ `wic_arm.rs` の変換の精度。触る行は別）。`menu-thumbnail-preview`（同じ WIC の道を使う側）。並べるときは `wic_arm.rs` を触る順を決める。

## Constraints
- 区分: **B**（バグ。作者が透明にした所が透けず、透けない所が透けることがある）。2026-10-30 `real-run-log-noise` の完了の棚卸で起票した（起票の条件: 開発者の裁定〔要件の討議 8〕で別 spec と決まっている）。
- 優先度の区分: B
- 外部クレートはそのまま使う（`windows` の WIC の公開の型で組む）。
- 実機の根・検体は作業木の `target\` の下だけ。
