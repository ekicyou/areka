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

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: 議題 1 の答えで変わる。読み手の `Element` に欄を足すなら S〜M（6〜10）のまま、足さずに済むなら S（3〜5）。切らない。
- 前提の状態: 上流は全部着地済み・今すぐ着手できる。起票（`1bb449ae`）の後に `areka-parsers/src/shell/`・`areka-emo-compose/` を触ったコミットは 0。
- 崩れた前提／古くなった位置: 無し。`decode.rs` の `decode_elements` は第 2 欄が `overlay` の行だけを値にし、ほかは記録なしで捨てる。`model.rs` の `Element` の定義は `layer`・`path`・`x`・`y` の 4 欄。合成器の側の element（`normalized.rs` の `NormalizedElement`）は**もう `method` の欄を持っている**が、畳み込み（`fold.rs` の読み手の element を写す所）が `ComposeMethod::Overlay` を決め打ちしている。`method.rs` の `is_implemented` は `Overlay` だけが真。
- ukadoc の引き直し: `base` は「ベースサーフェスを新規レイヤで完全に置き換える」「着せ替えと element では最初（element0・pattern0）にしか使えず、それ以外は overlay に読み替える」。element定義の項は「element0 があると surface*.png の内容は捨てられ element0 で置き換わる」。＝ **element1 以降の `base` は記録の対象でなく overlay として描くのが正典**。element0 の `base` は、areka がすでに「element0 があれば面の番号の画像を敷かない」（`base_image.rs` の `apply_base_images`）ので、空の土台に overlay で置くのと見た目が同じになる見込み（要件の段で画素で確かめる）。
- 見つけた穴: 読み手の `Element` に欄を足すと、構造体の直書きが **6 クレート・約 33 ファイル**で壊れる（parsers の `shell/*_tests.rs` 10 本前後・emo-atlas の `manifest.rs`・`lib.rs`・`emo2_e2e.rs` ほか・emo-compose のテストと土台 10 本前後・emo-present の `shell_target.rs`・`cache_tests.rs` ほか・emo-text の `tests/` 3 本・areka の `emo2_boot/balloon_background_tests.rs`）。並走の照合で他の列（文字とバルーン・動く画像）と重なる。
- 触るファイル（欄を足さない案）: `crates/areka-parsers/src/shell/decode.rs`（`decode_elements`）＋兄弟のテスト（`decode_tests_element_collision_tests.rs`）・`crates/areka-emo-compose/src/fold.rs`（決め打ちを外すなら）・`method.rs`・`plan.rs`（未対応の記録）・台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 2 行・`doc/COMPAT_ARCHITECTURE.md` §8・`sample-ghost-kit` のクローディアの検体を使う決定論テスト。欄を足す案なら上の約 33 ファイルが加わる。
- 議題:
  1. 読み手で `base` を「overlay と同じ値」にして欄を足さずに済ませるか（正典の読み替えの規則どおり・element0 は土台が空なので同じ絵）、描画メソッドの欄を足して合成器で `Base` を描くか（他の描画メソッドの記録も型で持てるが、約 33 ファイルに波及）。
  2. 未対応の描画メソッドの行の記録をどこで出すか（読み手は記録を出さない層＝記憶「parser は転記層」。欄を足さないなら、捨てる行を数えて合成器へ渡す口が要る）。
- 同時に走らせない: `extent-element-offset`（本 spec を先に）・`animated-image-playback`（C4・`plan.rs`・`method.rs`）・`element-clipping-option`・`collisionex-regions`（`decode.rs`）。

### 同じウェーブ C4 の約束（2026-10-05 棚卸㉒・破るなら止めて報告）

- compose の `plan.rs` の外形（`flatten_extent`）は変えない（`extent-element-offset` の持ち物）。seriko・compose の `world.rs`・`atlas_bind.rs`（`animated-image-playback`）と atlas の `normalize.rs`（`self-alpha-declaration`）に触らない。台帳 `assets.toml` は自分の行だけ（`ghost-standard-balloon`・`animated-image-playback` も別の行を直す）。
