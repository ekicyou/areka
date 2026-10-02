# Brief: areka-P0-balloon-element-order

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。**追跡 spec**＝`shell-balloon` が最初の版で縮めた重ね順を、element定義の並び順どおりに戻す。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- シェル内バルーンは `surface*`ブレスの element定義（`elementN,balloon,名前,X,Y`）で置くと決めた（`shell-balloon`）。element定義は並び順が重ね順（後ろほど手前）。ところが最初の版では「`balloon` の element定義は常に一番上に描く」に縮める。文字の層が画像の合成とは別の層で、常に一番上にあるため。
- このままだと、たとえば参考ゴーストのセキュリティボールが文字の手前を横切る、といった演出が書けない。書いた並び順と見た目が食い違う（最初の版は警告で知らせるだけ）。

## Current State

- 窓ごとの組み立て（`crates/areka-emo-present/src/mount.rs` の attach）は、窓の子として「文字の層の差し込み口（手前）」と「合成済みの面（奥）」の 2 つを持つ。面は emo-compose がアトラスから 1 枚に合成する（`areka-emo-compose`）。文字は emo-text が自分のスワップチェーンで描き、差し込み口に貼る。
- 当たり判定は合成の層とは別で、画家のアルゴリズム（後定義が手前）。

## Desired Outcome

- `balloon` の element定義を、他の element定義と同じく並び順どおりの重ね順で描く。最初の版の警告と縮めを外す。
- アニメーション（SERIKO の pattern）で上に重なる部品があっても、並び順の約束が保たれる。

## Approach（案・要件と設計で決める）

- 合成を「`balloon` の element定義の位置」で上下 2 枚（以上）に分け、その間に文字の層を挟む。あるいは文字を合成の中へ描き込む。性能（1 コマの予算・`recompose-budget` の先例）と、文字だけが変わるときに合成をやり直さないこと（今の利点）を天秤にかける。

## Scope

- **In**: 重ね順の挟み込み、最初の版の縮めと警告の撤去、SERIKO の重なりとの合わせ、性能の確かめ、決定論テスト。
- **Out**: `surface1000` などのサーフェスを element定義で置く機能（開発者の将来の希望・未起票）。本 spec の分け方はそれを妨げないこと。

## Boundary Candidates

- 合成の分割（emo-compose・emo-present）と、文字の層の差し込み位置。

## Out of Boundary

- シェル内バルーンの定義・行き先・切り替え（`shell-balloon`）。

## Upstream / Downstream

- **Upstream**: `shell-balloon`。
- **Downstream**: 将来の「サーフェスを element定義で置く」。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `present-write-coherence` の未達（性能）・`zorder-chain-residue`（窓どうしの重なり順＝別の話）。

## Constraints

- 文字だけが変わるコマで合成をやり直さないこと（今の性能を落とさない）。1 ファイル 1,000 行。決定論テスト網羅は必達。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- `shell-balloon` の追跡 spec。element の型とシェルのパーサ・emo-compose／emo-present の合成を触る＝`surface-element-nesting`・`animated-image-playback` と同時に走らせない（シェルの element の直列の列）。
