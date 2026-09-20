# Brief: areka-P0-default-balloon-nar-fold

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票。roadmap 台帳 **#42**（2026-09-19 から「登記だけの行」だった）に brief を与える。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: 次に検体を足す開発者と、配布 zip を作る `alpha-release-signoff`。

既定バルーン `StayseeBalloon` だけが、`vendors/sample_ghost/` の中で**展開フォルダのまま**保管されている（追跡ファイル 29 本・`git ls-files vendors/sample_ghost/StayseeBalloon | wc -l`）。他の検体 5 体は配布形（`.nar`）で保管され、登記表 `SAMPLES` を通して窓口 `sample_ghost_kit::SampleRoot` から取り出す。`vendors/sample_ghost/README.md` が定める「この場所に置くもの」と、完了 spec `areka-P0-nar-install` の要件 8.1 に反した状態である。

こうなった経緯: `default-balloon-bundle`（#37）が `nar-install`（#11）より先にマージされた。#37 の brief は「`.nar` 化は #11 が引き受ける」と書いて完了し、#11 の brief は「畳み込みは #37 が行う」と書いて完了した。**双方が相手を指したまま両方とも着地し、誰も畳まなかった。** どちらも完了済みなので、先送りを消化できない。

## Current State

- 登記表 `SAMPLES` は **5 行**（`crates/sample-ghost-kit/src/lib.rs` の `SAMPLES` の定義）。`StayseeBalloon` は載っていない。
- 検体数を直書きしているテストが 2 本: `every_registered_sample_lands_where_its_registry_row_says` と `every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`（`crates/sample-ghost-kit/src/lib_tests.rs`）。どちらも `5` を持つ。母数 0 で緑にならないための数なので、消さずに数だけ直す。
- 既知の名前の一覧を直書きしているテストが 1 本: `unknown_sample_fails_with_all_five_known_names`（同ファイル）。一覧が 2 か所、関数名にも数が入っている。
- 畳む道具は実在する: `cargo run -p sample-ghost-kit --example fold-samples -- --from <展開フォルダ>`。`--from` は必須、`--check` は任意（`crates/sample-ghost-kit/examples/fold-samples.rs` の引数解析）。
- **roadmap の手順に欠けていた 1 点**（2026-09-20 の実測で判明）: `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs` の定数 `STAYSEE_BALLOON_DIR` が展開フォルダを直接指し、`crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs` の `staysee_root()` がそれを実体化している。**フォルダを追跡から外すと、この結合テスト一式（配下 2,868 行）が赤になる。**

## Desired Outcome

完了時に次が真になっている。

1. `vendors/sample_ghost/StayseeBalloon.nar` が在り、展開フォルダ 29 本は追跡から外れている。
2. `SAMPLES` に `StayseeBalloon` の行が在り、`SampleRoot::acquire("StayseeBalloon")` で根が取れる。
3. `areka-emo-text` の StayseeBalloon 結合テストが、展開フォルダではなく窓口から根を取る形で、**同じ 29 ファイルの期待値のまま**緑である。
4. 直書きの検体数 2 か所と名前の一覧 2 か所と関数名が 6 体に揃っている。
5. `StayseeBalloon/` という展開フォルダの綴りを現在形で語る文書（steering・`vendors/sample_ghost/README.md`・後続 brief）が、`.nar` の綴りへ直っている。

## Approach

既存の道具と既存の雛形だけで済ませる。新しい仕組みは作らない。

- 畳む: `fold-samples -- --from vendors/sample_ghost/StayseeBalloon`。
- テストの付け替え: `SampleRoot` は借用パスの寿命を持つので、`LazyLock` で保持する。雛形は `crates/areka-emo-compose/src/sample_test_support.rs`。`areka-emo-text` の dev 依存には `sample-ghost-kit` が既に在る（`crates/areka-emo-text/Cargo.toml`）。
- 29 ファイルの期待値が保たれる根拠: `type,balloon` では最上位の `install.txt` も展開先へ置かれる（`crates/areka-nar/src/plan.rs`・除くのは `supplement` だけ）。要件段階でこの 1 点を実走で確かめる。

## Scope

- **In**: `.nar` への畳み込み／`SAMPLES` への 1 行／直書きの数と名前の一覧の追随／`areka-emo-text` の結合テストの根の取り方の付け替え／展開フォルダを追跡から外す／文書の綴りの追随。
- **Out**: 既定バルーンの自動採用（`baseware-root-layout`）／配布 zip への同梱手順（`alpha-release-signoff`。ただし「出どころが `.nar` になった」ことは同 brief へ申し送る）／バルーンの中身の変更（無改変が CC0 同梱の前提）。

## Boundary Candidates

- 保管形の切り替え（`vendors/` と登記表）
- 既存テストの根の取り方（`areka-emo-text/tests/staysee_balloon_fixture*`）

## Out of Boundary

- `areka-nar` 本体と `fold-samples` の振る舞い（使うだけ・変えない）
- 他の検体 5 体

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-nar-install`（窓口と畳む道具）・完了 `areka-P0-default-balloon-bundle`（バルーンの実物と表示の保証）。どちらも着地済み＝**いつでも着手できる**。
- **Downstream**: `areka-P0-baseware-root-layout`（既定バルーンの id 定数。窓口を呼ぶだけで `SAMPLES` には触らない）・`areka-P0-alpha-release-signoff`（zip へ入れる 29 ファイルの出どころが `.nar` の展開結果に変わる）・`areka-P0-shell-balloon-switch`（2 つ目のシェルを持つ検体を登記するなら、直書きの数の数え直しが再発する）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: なし。触るファイルは `vendors/sample_ghost/`・`crates/sample-ghost-kit/src/{lib.rs,lib_tests.rs}`・`crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs`・`crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs` と文書だけで、**2026-09-20 の実測で同じウェーブの他 spec と共有 0**。

## Constraints

- 規模 **XS**（タスク 2〜3 本）。
- 畳み込み・登記・数の追随・テストの付け替え・追跡から外す操作は**同じコミット**で行う（途中の状態は必ず赤になる）。
- 検体数は手で書き換える箇所が 3 つある。書き換えたあと `cargo test -p sample-ghost-kit` と `cargo test -p areka-emo-text --test staysee_balloon_fixture_test` を必ず回す。
- バルーンの中身は 1 バイトも変えない。畳んだ `.nar` を窓口で展開した結果が元の 29 本と一致することを、畳む前に控えた一覧で突き合わせる（改行コードの変換を `.gitattributes` が起こさないことを含む）。
