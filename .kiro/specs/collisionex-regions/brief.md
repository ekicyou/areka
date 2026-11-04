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

## 2026-11-04 棚卸㉞の再測定（main `932406f9`）

- 前提の変化: 当たり判定を値にする `decode_collisions`（`crates/areka-parsers/src/shell/decode.rs`）は今も番号が数字だけの行しか読まない。合成の当たり判定（`crates/areka-emo-compose/src/hit.rs`・`hit_import.rs`）と表示層の `presenter/hit.rs` は棚卸㉔から変更 0。11-01 から着地した `areka-test-binary-stability` はこの spec のファイルに触れていない。働きの上で先に要るものは無い。
- 触るファイル: 読み手 `crates/areka-parsers/src/shell/{model.rs 303 行, decode.rs 623 行}`（`Collision` の型に形の欄を足し、`region` の行も運ぶ）、`crates/areka-emo-compose/src/{hit.rs 756 行, hit_import.rs}`（形ごとの内外の判定は新しいファイルへ）、型を直書きするテスト 12 ファイル（parsers 7・emo-compose 3・emo-present の `presenter_film_tests.rs`・`presenter_test_support.rs`）、本体の `crates/areka/src/input_events/mod.rs` の説明の 1 行（518 行目「collisionex は実装しない」）、台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 2 行、`doc/COMPAT_ARCHITECTURE.md` §8。
- 規模: 8〜11 タスクのまま。切らない。
- 先に要るもの（見直しと crate 分け）: 見直しの `audit-parsers-sakura`・`audit-emo-compose`・`audit-emo-present`・`audit-areka-input-menu` の 4 本（ロードマップのとおり・4 本とも今も重なる）。**新しく**、本体の `input_events/` を新しい crate へ移す `input-crate-split`（とその前の本体の crate 分け 3 本）も待つ。本体に触るのは説明の 1 行だけだが、その 1 行で待ちの鎖がまるごと付く。
- 同じ時に走らせない相手: 読み手の 2 ファイルを分け合う `animated-image-import`・`element-clipping-option`・`draw-methods-canon`・`seriko-script-triggers`・`config-parse-diagnostics`・`shell-tooltip`・`surfaces-basepos`、`hit.rs` を分け合う `hit-id-pass`・`wintf-d2d-composite-rules`。
- 優先度の区分: C（ロードマップの行のとおり＝正典の拾い残し。起票の根の開発者の指示「未対応の件はすべて起票」は包括のもので、この働きを名指しした頼みではない）。前の節の「A」はこれに読み替える。
- 要件定義のモデル: Opus（正典の書式がはっきりしていて、今の型に欄を足すだけ）。
- 分割の案: 無し。
- 引き継ぐ決め（前の節から）: `Collision` に形の欄を足す（`collision` と `collisionex` を書いた順に 1 つの列へ並べ、「後に書いたものが手前」を保つため）。書式は `collisionex*,名前,形,座標…` で名前が 2 番目（`collision*` は最後）。形は 5 つ（矩形・楕円・円・多角形・`region`）で、`region` は読んで運び、記録だけ残す（働きは `hit-id-pass`）。`hit_import.rs` の `regions_of` は矩形の 4 辺にずれを足す作り＝形ごとにずらす。
- 見つけた穴・古くなった記述: 本文の「4 つの形」は 5 つと読む（`region` を含む）。
