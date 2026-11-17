# Brief: collisionex-regions

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-mouse-drag-events` の完了時の棚卸（`.kiro/specs/completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes・`verification/real-machine.md` 4 章 ⑵）。開発者の方針（2026-10-05）「実機で areka が未対応だったためにうまくいかなかった件はすべて起票」。

## Problem

- **利用者・ゴーストの作者**: 当たり判定を `collisionex`（矩形・円・楕円・多角形）で書いたシェルでは、areka のマウスのイベントの Reference4（当たり判定の名前）がいつも空になる。クローディアの当たり判定は全部 `collisionex` なので、撫で（`OnMouseMove`）・ダブルクリック・ドラッグのどれでも「どこを触ったか」がゴーストに届かない。
- `mouse-drag-events` の実機では、R1・R2 の 4 回のドラッグで Reference4 が全部空だった（クローディアのドラッグの台詞は当たり判定を見ないので確認には響かなかった）。

## Current State

- 読み手 `crates/areka-parsers/src/shell/decode.rs` の `decode_collisions` は、番号が数字だけの `collisionN`（矩形）だけを値にし、`collisionex` の行は記録なしに読み飛ばす。
- 当たり判定の解決は `crates/areka-emo-compose/src/hit.rs`（矩形と名前）。areka の `input_events/mod.rs` の doc にも「`collisionex` は実装しない（7.4）」とある。
- 台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `collisionex` の行は `absent`・担当なし（優先度 A12・価値「触れ合い」）。

## Desired Outcome

- `collisionex*,名前,rect|ellipse|circle|polygon,…` を読み、当たり判定の解決が形に応じて内外を判定する。`collision` と `collisionex` の重なりは今の画家の順（後に書いたものが手前）に揃える。
- 読めない行（形の名前が未知・座標の数が足りない）は記録を 1 件残す。
- 決定論のテストで 4 つの形の内外と重なりを固定し、実機でクローディアの撫でとドラッグの Reference4 に名前（`Head`・`Bust` など）が載ることを確かめる。

## Approach

ukadoc の `collisionex` の書式（`descript_shell_surfaces`）を引き直し、読み手に形の種類を足して、`hit.rs` の判定を形ごとに広げる。入れ子のサーフェス（`surface-element-nesting` で持ち込む子の当たり判定）にも同じ型で乗せる。

## Scope

- **In**: `collisionex` の読み・形ごとの内外判定・重なりの順・記録・台帳の行・`doc/COMPAT_ARCHITECTURE.md`・決定論のテスト・実機の確認。
- **Out**: `animation*.collision*` の当たり判定（アニメーション中の当たり判定）・カーソルの形（`The Hand`）。

## Boundary Candidates

- 読み手（`shell::{model,decode}`）
- 当たり判定の解決（`areka-emo-compose` の `hit.rs`）

## Out of Boundary

- マウスのイベントの送り方そのもの（kanade・`input_events`）は今のまま。名前が載るだけ。

## Upstream / Downstream

- **Upstream**: 完了 `shell-parse`・`emo-compose`・`surface-element-nesting`（子の当たり判定の持ち込み）。
- **Downstream**: 撫で・クリック系のイベント全般（Reference4 を使うゴースト）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `element-base-method`・`element-clipping-option`（同じ `decode.rs`）——同時に走らせない。

## Constraints

- 段は**優先**（シェルの element の列・既存のゴーストの当たり判定がまるごと効かない）。


## 2026-10-15 `/kiro-discovery` の申し送り（`region` の持ち主）

- 上の棚卸㉓の節が「別に起票するのが筋」と書いた `collisionex` の `region`（画像の指定色の領域）は、`hit-id-pass` が引き取った（開発者 2026-10-15「シェル仕様で、Headなどのリージョン範囲を、画像で取り込む仕様があるはずです。これに対応するためにもヒットテストIDが欲しい」）。
- **この spec に頼むこと**: 読み手は `region` の行も落とさずに型へ運ぶ（ファイル名・R・G・B・領域反転）。働き（画像を読んで当たり判定にする）は `hit-id-pass` が持つので、この spec では読めた `region` の行を使わずに記録だけを残す。こうしておけば `hit-id-pass` は読み手の `shell/{model,decode}.rs` に触れずに済む（`draw-methods-canon` と `draw-methods-blend` の分け方と同じ）。
- この spec の 4 つの図形の CPU の判定は今の計画のまま。図形も ID の描き先へ寄せるかは `hit-id-pass` の議題 2。

## 2026-11-17 棚卸㊴の再測定（main 7f41b53e）
- **前提の変化**: 棚卸㉞の後、待っていた見直しのうち `audit-parsers-sakura`（PR#428）・`audit-emo-compose`（PR#430）・`audit-emo-present` が着地した。行数は `shell/model.rs` 356・`shell/decode.rs` 680・`hit.rs` 677・`hit_import.rs` 289。`collisionex` を読み飛ばすのは今も `decode_collisions`（`decode.rs` 245 行）。C21 の着地はこの spec のファイルに触れていない（`input_events/mod.rs` は `runtime-crate-split` の取り込みの直しだけ）。
- **触るファイル**: 読み手 `crates/areka-parsers/src/shell/{model,decode}.rs`・合成 `crates/areka-emo-compose/src/{hit,hit_import}.rs`（形ごとの判定は新しいファイルへ）・`Collision {` を直書きする檻（parsers の `decode_charset_tests`・`decode_tests_append_tests`・`decode_tests_element_collision_tests`・`decode_tests_lenient_input_tests`・`parse_tests`・`validation_tests`、emo-compose の `plan_test_support.rs`、emo-present の `presenter_test_support.rs` 890 行）・`presenter/hit.rs`・台帳 `assets.toml` の 2 行・§8 への追記。本体は `crates/areka/src/input_events/mod.rs` 518 行の説明の 1 行（「The Hand／collisionex は実装しない（7.4）」）だけ。
- **規模**: 8〜11 タスクのまま。
- **先に要るもの（C24 に置いた理由）**: 働きの依存は無い。本体の説明の 1 行が、見直し `audit-areka-input-menu`（未完了・`input_events/` の持ち主）と crate 分け `input-crate-split`（未完了・`input_events/` を新しい crate へ移す）の範囲に入り、「v0.2.0 の作り直しと重なる見直しは先に」の決まりで両方の後になるから。読み手と合成の見直しは着地したので、残る待ちはこの 1 行だけ。シェルの element の列で前に居た `animated-image-import` は版の外（行き先は次の版を決めるとき）へ出たので待たない。
- **早めるなら**: 説明の 1 行をこの spec の範囲から外して `audit-areka-input-menu` へ申し送れば、本体に 1 行も触らない。そうすると見込みの C22 の 5 本（`balloon-text-visual-tree`・`audit-placement-follow-residue`・`audit-test-kits`・`audit-ukadoc-survey`・`audit-pilot`）と触るファイルの重なりは 0 の見込み（`audit-placement-follow-residue` は `input_events/drag_test_support.rs` だけ・`balloon-text-visual-tree` が emo-present の `presenter_test_support.rs` に触るかは向こうの設計で確かめる）。
- **見込みの C24 の穴**: 今の見込みは C24 でこの spec と `audit-areka-input-menu` を並べていて、説明の 1 行（`input-crate-split` の後は新しい crate の側の同じファイル）で重なる。上の申し送りの形にするか、片方をずらす。
- **同じ時に走らせない相手**: 読み手の 2 ファイルを分け合う `animated-image-import`・`element-clipping-option`・`draw-methods-canon`・`seriko-script-triggers`・`config-parse-diagnostics`・`shell-tooltip`・`surfaces-basepos`、`hit.rs` を分け合う `hit-id-pass`（どれも未完了・どれも C22〜C24 の見込みに居ない）。
- **優先度の区分**: C（正典の拾い残し。起票の根の開発者の指示「未対応の件はすべて起票」は包括のもので、この働きを名指しした頼みではない）。版は v0.2.0。
- **要件定義のモデル**: Opus（正典の書式がはっきりしていて、今の型に欄を足すだけ）。
- **分割の案**: 無し。
- **引き継ぐ決め（前の節から）**: `Collision` に形の欄を足す（`collision` と `collisionex` を書いた順に 1 つの列へ並べ、「後に書いたものが手前」を保つ）。書式は `collisionex*,名前,形,座標…` で名前が 2 番目（`collision*` は最後）。形は 5 つ（矩形・楕円・円・多角形・`region`）で、`region` は読んで型へ運び、記録だけ残す（働きは `hit-id-pass`）。`hit_import.rs` の `regions_of` は形ごとにずらす。本文の「4 つの形」は 5 つと読む。実機の確かめ（クローディアの撫でとドラッグ）は画面を使う＝排他テストの机で順を取る。
