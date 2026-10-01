# Brief: areka-P0-balloon-scroll-fade

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。**正典に無い areka 独自の演出**（開発者「正典にはない項目ですし、独立 spec かも」）。普通のバルーンにも効く。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- シェル内バルーンの小さな箱（参考ゴーストの右の台詞欄は 2 列）では、あふれた列が自動スクロールでいきなり押し出されて消える。参考資料は「古い列のフェード」を挙げている。

## Current State

- あふれは正典どおりの自動スクロールだけ。行（縦書きなら列）単位で即時に動き、最新の行が常に見える（`crates/areka-emo-text/src/layout.rs` の visible window・`viewbox.rs` の整数 px のずらし描き）。

## Desired Outcome（2026-10-01 開発者確定）

- **あふれの基本は正典の自動スクロールのまま**（書き手が区切りたいところは `\x` と `\c`）。
- そのうえで、バルーンのキー（areka 独自・名前は要件で決める。CSS に当たる語が無いので areka の語で付ける）で有効にすると、**押し出される行（列）が消える前に薄れていく**演出を足す。既定は無効＝今と同じ。
- **自動の改ページ（クリックで次へ）は作らない**（開発者裁定: クリック待ちは台本に明示した `\x` だけ）。

## Approach

- 押し出しの瞬間に、消える行を別の層として残し、決めた時間で透明度を下げて消す（台詞の時計の上で決定論的に）。スクロールそのものの規則は変えない。

## Scope

- **In**: フェードのキー（有効・時間）、縦書きと横書き、シェル内バルーンと普通のバルーン、手動スクロール（`balloon-markers`）との合わせ、決定論テスト、`doc/COMPAT_ARCHITECTURE.md` §8 への登記。
- **Out**: 自動の改ページ・スクロールの規則の変更・押し出し以外の消え方（`\c` で全部消すときのフェードなど＝要るなら要件で検討）。

## Boundary Candidates

- 押し出しの検出（layout の visible window の変化）と、薄れる層の描画。

## Out of Boundary

- 矢印と手動スクロール（`balloon-markers`）。

## Upstream / Downstream

- **Upstream**: `balloon-markers`（同じスクロールの部分を触る＝直列）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `balloon-markers`・`text-align-shadow-canon`（あふれの判定が最新の行の遠辺だけを見る件）。

## Constraints

- 既定で今の出力と同じ（既存テストが無改変で緑）。1 ファイル 1,000 行。決定論テスト網羅は必達。1 フレーム遅らせる解は取らない。

---

> **📌 2026-10-01 `/kiro-discovery`（文字の現れ方）からの注記**——新 spec `areka-P0-text-reveal-fade`（字が透明から不透明へ変わる現れ方）が、本 spec と同じ「字の透明度を時間で変える仕組み」と「薄れている途中の字だけを毎コマ描き直す口」を要る。**2 本は同時に走らせず、先に着地した方が作り、後の方が使う**（roadmap「文字の現れ方」節）。
