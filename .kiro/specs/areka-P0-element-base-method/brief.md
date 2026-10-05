# Brief: areka-P0-element-base-method

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-mouse-drag-events` の完了時の棚卸（`.kiro/specs/completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes・`verification/real-machine.md` 4 章 ⑴）。開発者の方針（2026-10-05）「実機で areka が未対応だったためにうまくいかなかった件はすべて起票」。

## Problem

- **利用者・ゴーストの作者**: element定義の描画メソッド `base`（`element0,base,<絵>,X,Y`）を使う面が正しく描かれない。クローディア（`sample-ghost-kit` の `claudia`）の surface6・11・26 は `element0,base,surface0.png,0,0` の後に `element1,overlay,<顔の部品>` を重ねる形で、areka では上乗せの部品の大きさ（surface26＝100×56・surface11＝71×42）だけで描かれる。キャラクターが消えたように見える。
- 起動直後の台詞（`\s[26]`）でも起きるので、開発者が「シェルが表示されないことがある」と気付いた。記録は何も出ない。

## Current State

- 読み手 `crates/areka-parsers/src/shell/decode.rs` の `decode_elements` は、第 2 欄が `overlay` の行だけを `Element` にして、ほかの描画メソッドの行を黙って捨てる。`Element` 型（`shell/model.rs`）には描画メソッドの欄が無い。
- 合成器 `crates/areka-emo-compose/src/method.rs` の `ComposeMethod::is_implemented` は `Overlay` だけが真。`base` は `ComposeMethod::Base` へ写るが、描けない型の継ぎ目のまま。
- `element0` があると、面の番号の画像（`surfaceN.png`）を土台に敷く `base_image::apply_base_images` は手を出さない。そのため `element0,base,surfaceN.png` のように自分の番号の画像を指す面（surface19・29）は偶然正しく見え、別の画像を指す面だけが壊れる。
- 台帳 `doc/ukadoc-coverage/ledger/assets.toml`: element定義の行は `degraded`（担当は完了済みの `areka-P0-shell-parse`・注記に「overlay 以外の描画メソッドが読み飛ばされる縮退」）。`descript_shell_surfaces:base:1` の行は `vocabulary-only`・担当なし。進行中の担当 spec は無い。
- `mouse-drag-events` の実機（4.2）は、根の写しの `surfaces.txt` だけ `base`→`overlay` に書き替えて進めた（製品と検体は無改変）。

## Desired Outcome

- `element*,base,…` の行が読み手で値になり（描画メソッドの欄を持つ）、合成器が ukadoc の `base` の意味で描く。クローディアの surface6・11・26 が 333×500 で、土台の絵と顔の部品が重なって出る。
- `base` 以外の未対応の描画メソッドの行は、黙って捨てずに記録を 1 件残す（どの面のどの行か）。
- DLL を使わない決定論のテストで、`element0,base,<別の絵>`＋`overlay` の面の大きさと画素を固定する。実機でクローディアの `\s[26]` を確かめる。

## Approach

ukadoc の `base` の定義（`descript_shell_surfaces` の描画メソッドの項）を引き直してから、読み手に描画メソッドの欄を足し、合成器で `Base` を実装する。`overlay` 以外の残り（`overlayfast`・`replace`・`interpolate`・`asis`・`reduce`・blend 系）は本 spec では記録を出すところまでにして、必要なら別に切る。

## Scope

- **In**: 読み手の `Element` へ描画メソッドの欄を足すこと・`base` の描画・未対応の描画メソッドの行の記録・台帳の 2 行（element定義・`base`）と `doc/COMPAT_ARCHITECTURE.md` §8・決定論のテスト・クローディアでの実機の確認。
- **Out**: `base` 以外の描画メソッドの描画・element定義のオプション（`--clipping` ほか＝`element-clipping-option`）・動く絵（`animated-image-playback`）。

## Boundary Candidates

- 読み手（`areka-parsers` の `shell::{model,decode}`）
- 合成器（`areka-emo-compose` の `method`・`plan`・`blit`）

## Out of Boundary

- 外形の計算の規則（`extent-element-offset` の持ち物）・pattern定義の描画メソッド（SERIKO 側）。

## Upstream / Downstream

- **Upstream**: 完了 `shell-parse`・`emo-compose`・`surface-element-nesting`。
- **Downstream**: `element-clipping-option`・`animated-image-playback`（どちらも `decode_elements` と `plan.rs` を触る＝同じ列で直列）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の縮退を埋める）。
- **Adjacent**: `extent-element-offset`（`plan.rs`）・`element-clipping-option`（`decode_elements`）——同時に走らせない。

## Constraints

- 段は**バグ**（キャラクターが消える・記録なし）。シェルの element の列に入る。
