# Brief: areka-P0-emo-text-canon-residue

> 起票: 2026-09-11（棚卸⑬・`areka-P0-balloon-canon-residue` の 3 軸分割 ⑶＝emo-text 帰属の項目 11・12・14・15 を独立 spec に切り出し）。項目本文の正本は分割元 brief（11・12 は「`balloon-vertical-canon` からの追加登記」節、14・15 は「`emo-text-line-height-canon` からの追加登記」節）。ここでは所有と着地条件だけを書く。

## Problem

完了 spec `balloon-vertical-canon`（bvc）と `emo-text-line-height-canon` が emo-text 側に残した 4 件が所有者不在のまま `balloon-canon-residue` の台帳に同居していた。系列解決（emo-present）とも表示寿命（kanade）とも軸が違う:

11. 縦書き字形（グリフ直立・縦書き用字形）の**観測点が repo に無い**（字形が退行しても全緑）。
12. `writing_mode` 未知値の警告文言が実挙動とずれている（**現行 main の唯一の実バグ**・文言のみ・`crates/areka-emo-text/src/writing.rs`・逐語固定のインラインテストと同時修正）。
14. 折返し基準が描画範囲の外に解決されるバルーン定義の扱い＋警告の `balloon` 欄がバルーン名でなく定数 `BALLOON_NAME_PLACEHOLDER`（`region.rs` で定義・`actor.rs` が import＝参照 2 ファイル）。`BalloonModel` に `name,` の取得口が無い（`impl Font` の `name` は別物）。
15. 行末禁則文字のぶら下がり（折返しの遅延）が未実装。二段構え（soft `wrap_threshold`／hard `inline_limit`）の足場は `layout.rs` にある。

## Current State

2026-09-11 実測（サブエージェント再測定）: 12 の文言と逐語固定テスト `unknown_value_falls_back_to_horizontal_tb_with_warn` が現存・実挙動は `WritingModeDecl::Unknown → None`（「指定なし」として合流・モジュール doc DD6 と一致）。14 の取得口は `impl BalloonModel` に無い。15 は `LayoutEngine::visible_window` の前提（行が行送り方向へ単調）のまま。`crates/areka-emo-text/src/` は `draw.rs` 988・`layout.rs` 955・`actor.rs` 952・`region.rs` 951 行＝**1,000 行番人の射程**（例外表に不在・新規ファイルで足すこと）。

## Desired Outcome

12 の文言が実挙動と一致し、14 でバルーン名が警告に出て `ledger/assets.toml` の `owner` が埋まり、15 で「」」等の禁則文字が折返し基準を超えてぶら下がり描画範囲は超えない、11 で縦書き字形の退行が赤になる観測檻がある（SSP との一致ではなく areka 内で閉じた反証可能な述語）。

## Approach

12 → 1 手（文言＋テスト）。14 → `crates/areka-parsers/src/balloon/parse.rs` の `map_merged` が `name,` を写し `BalloonModel` に取得口・`region.rs` の定数を差し替え。15 → 禁則文字集合の定義＋行末での折返し遅延の規則（既存 2 値の間で判定）。11 → `draw_readback` 系の読み戻しで字形差を述語化（AI vision 目視は補助）。

## Scope

- **In**: 項目 11・12・14・15。
- **Out**: 系列解決（分割元 ⑴）・表示寿命（`balloon-lifecycle-events`）・`\f` 寄せ（`text-align-shadow-canon`・`layout.rs` を共有＝**同居不可**）。

## Boundary Candidates

- 12 は S・前提なし・直接修正でも可（本 spec の着手前に直せば本 spec から外す）。
- 14・15 は「バルーン定義と折返しの正典」の 1 軸、11 は観測檻＝独立。

## Out of Boundary

- 折返し基準／描画範囲の意味論の再裁定（`emo-text-line-height-canon` で確定・不変）。

## Upstream / Downstream

- **Upstream**: 完了 bvc・完了 `emo-text-line-height-canon`。`text-decoration-canon` ⑴（`draw.rs` 分割）の**後**に着手（`layout.rs`／`region.rs` を触るため W13 と同居不可）。
- **Downstream**: `text-align-shadow-canon`（15 の禁則と寄せが同じ行配置の前提を共有）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-balloon-canon-residue`（分割元・11・12・14・15 を引き継ぐ）。
- **Adjacent**: `balloon-font-descript-keys`（`balloon/parse.rs`・`model.rs` を共有＝**同居不可**・先に着地させる）・`text-decoration-canon`（`region.rs`）。

## Constraints

- 編集集合: `crates/areka-emo-text/src/{writing,region,layout,actor,state_decoration}.rs`（＋兄弟テスト）・`crates/areka-parsers/src/balloon/{parse,model}.rs`・`doc/ukadoc-coverage/ledger/assets.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。（`state_decoration.rs` は 2026-09-13 に `areka-P0-text-decoration-canon` から引き受けた「装着より先に `\f` が届いた窓」のため——下の 📌 を参照）
- fixture `crates/pilot/examples/shiori-host-32/fixtures/emo2/` は無改変。
- 規模 M・要件定義は Opus で足りる（裁定は 15 の禁則文字集合の出典 1 件）。

> **📌 2026-09-13 先送りの引受（`areka-P0-text-decoration-canon` 着地・タスク 9.4 の裁定）**——**装着より先に `\f` が届いた窓**（項目 16）。cue のドレインは非同期で、`crates/areka/src/emo2_boot/frame/attach.rs::connect_balloon_text` が `text_slot_view` を`None` で受けると装着が次フレームへ委ねられる。この窓で `\f[...]` が先に届くと`crates/areka-emo-text/src/state_decoration.rs::TextLayerState::set_look_layers` の追随ガード（現在の見た目が旧い既定と同値のときだけ追随）が成立せず、**バルーン定義の既定（大きさ・色・フォント名）がその台詞のあいだ届かない**（以後の文字が素の既定 12px で描かれる。次の台詞頭の `ClearAll` で自然治癒）。親 spec は**是正せず記録だけ足した**——`set_look_layers` は旧い 2 層がまだ素の既定のときに `warn!` を 1 件残し、`state_decoration_reset_tests.rs::attaching_over_an_explicit_look_records_that_the_defaults_could_not_land`（正）と `attaching_in_the_normal_order_records_nothing`（負）が固定している。**是正が本 spec の担当**である理由＝正しく直すには「作者が明示した項目だけを新しい既定へ載せ替える」3 者併合が要り、親 spec の承認済み要件 10.4「戻す操作は項目を列挙しない」（後続仕様が `TextLook` へ足した項目も自動で含まれる）と衝突する設計判断を伴うため、要件・設計のフェーズを通す必要がある。（編集集合の行には `state_decoration.rs` を追加済み。）
