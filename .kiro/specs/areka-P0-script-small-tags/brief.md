# Brief: areka-P0-script-small-tags

> 2026-10-18 棚卸㉗で起票（開発者の指示「もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です。」＝v0.1.0 のゴール）。証拠はこの棚卸のぱすたの抜けの調べと、ゴースト「窓際のぱすたさん」の辞書と pasta の台本の組み立て（読むだけ・`dic/boot.pasta`・`dic/tategaki.pasta`・`dic/layout.pasta`・`scripts/pasta_layout.lua`・pasta の `appearance.lua`・`sakura_builder.lua`）。正典は ukadoc MCP で 2026-10-18 に引き直した。

## Problem

- **ゴーストの作者**: 小さい台本のタグ 4 つが areka で効かない。どれも 1 つずつは小さいが、ぱすたは毎回の会話で使う。
  - `\_u[0x3000]`（全角の空白を文字の番号で埋め込む）: ぱすたは行の頭の字下げと、起動のときにバルーンを作るための 1 文字に使う。areka では文字が出ず、起動のバルーンも開かない。
  - `\_m[0x41]`（ASCII の番号で埋め込む）: `\_u` と同じ族。
  - `\![bind-noevent,カテゴリ名,パーツ名,数値]`: pasta の着せ替えの道（`appearance.lua`）が、カテゴリを丸ごと返すときに出す。areka では受け手が無く、着せ替えが変わらない。
  - `\![*]`: pasta は選択肢ごとに前へ付ける（`sakura_builder.lua`「choice トークンを `\![*]\q[...]` へ変換する」）。areka は選択肢の前のこのタグを読んで捨てている。

## Current State

- **正典（ukadoc・要旨）**:
  - `\_u[0x0000]`: UCS-2 の文字の番号の埋め込み。10 進数なら `0x` を付けない（例 `\_u[8942]`）。
  - `\_m[0x00]`: ASCII の文字の番号の埋め込み。
  - `\![bind-noevent,カテゴリ名,パーツ名,数値]`（SSP 2.8.23）: `\![bind]` と同じく着せ替えのパーツを操作するが、イベントを起こさない。`\![bind]` のほうは実行の後に `OnDressupChanged`・`OnNotifyDressupInfo` を知らせる。
  - `\![*]`: マーカーを表示する。マーカーはバルーンが決める（バルーンの descript の `marker.filename`・SSP 2.5.51）。`%*` も同じ。
- **areka**:
  - `\_u`・`\_m`: 字句は読める（`crates/areka-parsers/src/sakura/lexer.rs`）が、読み手 `crates/areka-parsers/src/sakura/decode.rs` の `decode_tag` に腕が無く、「知らないタグ」として `Raw` になり、台本の組み立て（`crates/areka-sakura/src/compile.rs` の 291 行あたりの残りの腕）で記録つきで捨てられる。
  - `\![bind-noevent]`: 汎用の運び手（名前 `bind-noevent`）で届くが、受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs`（960 行）に行が無く、着せ替えの受け手 `crates/areka-seriko/src/actor.rs` は名前が `bind` ちょうどのものだけを受ける（488 行あたり）。
  - areka は `\![bind]` の後の `OnDressupChanged`・`OnNotifyDressupInfo` を送っていない（網羅台帳 `shiori.toml` で absent・持ち主なし）＝今は `bind` と `bind-noevent` の違いが外から見えない。
  - `\![*]`: 読み手が直後の `\q[…]` と畳んで選択肢にし、マーカーは「無視した」印を付けて何も描かない（`decode.rs` の `fold_choice_marker`）。単独なら名前 `*` の汎用の命令。
  - 網羅台帳 `sakura-script.toml`: `\_u[0x0000]`・`\_m[0x00]`・`\![bind-noevent,…]`・`\![*]` は absent・持ち主なし。`assets.toml` の `marker.filename` の行は absent・持ち主 `balloon-canon-residue`。

## Desired Outcome

- `\_u[…]`（16 進と 10 進）と `\_m[…]` が、その文字としてバルーンに出る（字下げ・空白 1 文字でバルーンが開く）。読めない番号は記録を残して捨てる。
- `\![bind-noevent,…]` が `\![bind,…]` と同じに着せ替えを変える。後で `OnDressupChanged` を送る spec ができても、`bind-noevent` からは送らない目印が通っている。
- `\![*]` の扱いが決まり、網羅台帳に書かれている（下の議題 1）。
- 網羅台帳の 4 行が実物と合う。

## Approach

- `\_u`・`\_m` は読み手で文字へ写す（数字の読み方は 16 進の `0x` 付きと 10 進）。サロゲートの片割れ・0・制御の文字など、出せない番号の扱いを要件で決める。
- `bind-noevent` は着せ替えの受け手の名前の選別に 1 つ足し、受け手の表に 1 行足す。中で「イベントを起こさない」の目印を持たせる。
- `\![*]` は SSP で測ってから決める。

## Scope

- **In**: `\_u`・`\_m` の文字への写し・`\![bind-noevent]` の受け手・`\![*]` の扱いの決定と記録・決定論のテスト・網羅台帳の 4 行。
- **Out**: `OnDressupChanged`・`OnNotifyDressupInfo` の送出（持ち主なし＝必要なら別に起票）・マーカーの画像の読み込みと描画（`marker.filename`＝`balloon-canon-residue`。議題 1 で描くと決まったときの扱いも議題）・選択肢の印の見た目（`choice-marker-styling`）。

## Boundary Candidates

- 台本の読み手（`decode.rs` の腕）
- 着せ替えの受け手の名前の選別（`areka-seriko`）と受け手の表

## Out of Boundary

- 文字の配置と縦書き（今の文字の層のまま）
- 着せ替えのイベント

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: `pasta-in-windows-verification`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `decode.rs` を挙げる spec（`balloon-break-position`・`range-choice-tag`・`sakura-time-directives`・`sakura-time-critical`・`seriko-script-triggers`・`talk-fast-forward`・`extra-character-windows`・`choice-marker-styling`・`text-align-shadow-canon`・`property-query-channels`）。`consumer_ledger.rs` に行を足す spec（`stay-on-top-directives`・`descript-window-keys`・`window-link-directives` ほか）。`balloon-canon-residue`（`marker.filename`）。

## Constraints

- 読み手は転記に徹する（意味は受け手が決める）。`\_u` の文字への写しは、文字の命令を作るだけにする。
- 失敗の道（読めない番号）は記録を残す。
- `consumer_ledger.rs` は 960 行＝行を足すだけ。テストは兄弟のファイルへ。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: `crates/areka-parsers/src/sakura/decode.rs`（502）と兄弟のテスト・`crates/areka-seriko/src/actor.rs`（名前の選別）と兄弟のテスト・`crates/areka/src/emo2_boot/consumer_ledger.rs`（960・1 行）・網羅台帳 `sakura-script.toml` の 4 行・新しい兄弟のテストのファイル（印: 新）。
- 規模: 4〜6 タスク。
- 先に要るもの: 無し。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。
- 版: v0.1.0。
- 要件定義のモデル: Opus（先例＝裸の `\+` の写し・`\![*]` の畳み・着せ替えの名前の選別。境目も並行も新しくない）。
- 要件の段の議題:
  1. **`\![*]` を SSP で測る**: マーカーの画像を持たないバルーン（ぱすたの縦書きのバルーンは `marker.png` を持たない）で何を描くか。何も描かないなら記録の行だけで済む。描くなら（SSP の既定の絵など）、描く仕事をこの spec に入れるか、`marker.filename` の持ち主 `balloon-canon-residue` へ渡すか。
  2. **`\_u` の出せない番号**: 0・制御の文字・サロゲートの片割れ・範囲の外。SSP で測るか、「捨てて記録」に倒すか。
  3. **`OnDressupChanged` を誰が持つか**: 今は送っていないので `bind-noevent` との違いが見えない。この spec で起票だけするか。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 触る所の中身は棚卸㉗から変わらない。✅ `mcp-reload`（10-22）が受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs` を 472 行に減らし（テストを兄弟へ）、着せ替えの読み取りのテストを `crates/areka-seriko/src/bind_tests.rs` へ出した。着せ替えの受け手 `crates/areka-seriko/src/actor.rs`（780）の名前の選別は 492 行目あたり（名前の定数 `BIND_COMMAND` は 56 行目）。同じ spec の持ち越し（`crates/areka/src/emo2_boot/shell_carry.rs`）は、ゴーストの読み直しの後に着せ替えを `\![bind,…]` と同じ合図で流し直す＝後で `OnDressupChanged` を送るようになると、読み直しのたびにイベントが飛ぶ。この spec の「イベントを起こさない」目印は、その流し直しにも使える。
- 触るファイル: `crates/areka-parsers/src/sakura/decode.rs`（502）と兄弟のテスト・`actor.rs`（780）と兄弟のテスト（新）・`consumer_ledger.rs`（472・1 行）・網羅台帳 `sakura-script.toml` の 4 行（`\![bind-noevent]` は 709 行目）。字句の読み手 `lexer.rs`（426）は今も読めるので触らない見込み。
- 規模: 4〜6 タスクのまま。
- 先に要るもの: なし。同じウェーブに置けない v0.1.0 の相手: `stay-on-top-directives`（`decode.rs`・受け手の表）・受け手の表に行を足す `window-link-directives`・`descript-window-keys`・`mcp-strict-errors`（案 2）・`script-security-level`（1 本ずつ）。後の版: `decode.rs` を触る 10 本（`range-choice-tag` ほか）・`actor.rs` を触る `animated-image-import`・`seriko-script-triggers`・`balloon-canon-residue`・`shell-hover-animation`・`talk-anchor-interrupt-ticks`。依存を足さない。机・静かな机・前後の全体テストの比べは要らない。議題 1（`\![*]`）の測りは開発者の機械の SSP。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。
- 要件定義のモデル: Opus（先例＝裸の `\+` の写し・`\![*]` の畳み・着せ替えの名前の選別）。
- 分割の案: なし。
- 見つけた穴・古くなった記述:
  - 本文 Current State と Constraints の「`consumer_ledger.rs`（960 行）」は古い（472 行）。「488 行あたり」は今 492 行目。
  - 議題 3（`OnDressupChanged` を誰が持つか）に、読み直しの持ち越しが `bind` で流し直すこと（上）を足す。送る spec を起票するなら、流し直しを「イベントを起こさない」側に寄せることも書く。
