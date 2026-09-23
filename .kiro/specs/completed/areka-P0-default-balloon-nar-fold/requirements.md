# Requirements Document

> 本文の実測は **2026-09-23・本ブランチ**（main `92f5f448` 相当）のもの。brief（2026-09-20・`fe157df1`）の記述はすべて引き直し、食い違いは無かった。コードは「何の定義か」（関数名・定数名＋ファイルパス）で指し、行番号では指さない。

## Project Description (Input)

既定バルーン `StayseeBalloon` だけが `vendors/sample_ghost/` に展開フォルダのまま保管されており、他の検体 5 体と同じ配布形（`.nar`）＋登記表 `SAMPLES` 経由の取り出しになっていない。`.nar` へ畳み、登記表に載せ、直書きの検体数と名前の一覧を追随させ、既定バルーンの結合テストが窓口から根を取るよう付け替え、展開フォルダを追跡から外し、文書の綴りを直す（brief.md より）。

## Introduction

### 誰が困っているか

次に検体を足す開発者と、配布 zip を作る後続 spec `areka-P0-alpha-release-signoff`。

### いま何が起きているか（2026-09-23 実測）

- **既定バルーン `StayseeBalloon` だけが展開フォルダで保管されている。** `git ls-files vendors/sample_ghost/StayseeBalloon` は **29 本**を返す。他の検体 5 体は `vendors/sample_ghost/*.nar` の配布形で保管され、窓口 `sample-ghost-kit` の登記表 `SAMPLES`（`crates/sample-ghost-kit/src/lib.rs` の `SAMPLES` の定義・**5 行**）を通して取り出す。`StayseeBalloon` はこの表に**載っていない**。`vendors/sample_ghost/README.md` が定める「配布形のまま置く。展開した木は追跡しない」と、完了 spec `areka-P0-nar-install` の要件 8.1 に反した状態である。
- **経緯**: `areka-P0-default-balloon-bundle`（#37）が `areka-P0-nar-install`（#11）より先にマージされ、#37 の brief は「`.nar` 化は #11 が引き受ける」、#11 の brief は「畳み込みは #37 が行う」と書いて両方とも完了した。どちらも完了済みなので、先送りを消化できる spec が無い。
- **検体数を直書きしているテストが 2 本**（`crates/sample-ghost-kit/src/lib_tests.rs`）: `every_registered_sample_lands_where_its_registry_row_says` と `every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`。どちらも `SAMPLES.len()` を `5` と突き合わせる。母数 0 で緑にならないための数なので、消さずに数だけ直す。
- **既知の名前の一覧を直書きしているテストが 1 本**（同ファイル）: `unknown_sample_fails_with_all_five_known_names`。名前の一覧が関数内に 2 か所、関数名にも「five」が入っている。
- **畳む道具は実在する**: `cargo run -p sample-ghost-kit --example fold-samples -- --from <展開フォルダ>`（`crates/sample-ghost-kit/examples/fold-samples.rs` の `main` の引数解析）。`--from` は必須、`--check` は任意。畳むと同時に、本番の展開器で空の根へ入れ直して元の追跡ファイルと 1 バイト単位で突き合わせる。
- **展開フォルダを直接指す結合テストがある**: `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs` の定数 `STAYSEE_BALLOON_DIR`（値 `"../../vendors/sample_ghost/StayseeBalloon"`）を、`crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs` の `staysee_root()` が実体化し、テーマ別 8 ファイル（入口と `test_support.rs` を合わせて 10 ファイル・合計 2,868 行）がそこから 29 ファイルの期待値 `EXPECTED_FILE_NAMES`（`install.txt` を含む）を読む。**フォルダを追跡から外すだけでは、この結合テスト一式が赤になる。** `areka-emo-text` の dev 依存には `sample-ghost-kit` が既に在る（`crates/areka-emo-text/Cargo.toml`）。
- **窓口の迂回を見張るテストが常設されている**: `crates/log-capture-kit/tests/sample_path_guard_test.rs` の `no_sample_path_spelling_lives_outside_the_gateway` は、登記表 `SAMPLES` の名前から「展開形の検体フォルダ」などの綴りを組み立てて、窓口の外のソース（コメントを除く）に無いことを判定する。ただし展開形の走査語は**末尾に `/` を持つ形**（`sample_path_guard_test.rs` の `Form::tokens` の `CheckedInTree` 腕が `format!("{VENDOR_PARENT}/{}/", s.name)` で組む）で、定数 `STAYSEE_BALLOON_DIR` の値 `"../../vendors/sample_ghost/StayseeBalloon"` は `/` で終わらない。**よって登記しても、この見張りは `STAYSEE_BALLOON_DIR` を赤にしない**（要件ディスカッションで訂正・2026-09-23 に `Form::tokens` の定義で確認）。付け替え漏れを実際に赤にするのは、展開フォルダを消した後に読み先が実在しなくなる結合テスト（要件 4.4）である。
- **`type,balloon` の `.nar` では最上位の `install.txt` も展開先へ置かれる**（2026-09-23 に 2 通りで確認）: ① `crates/areka-nar/src/plan.rs` の `body_placement` は、最上位の `install.txt` を除くのを `InstallKind::Supplement` のときだけにしている。② 実走 `fold-samples -- --from vendors/sample_ghost/StayseeBalloon` の印字は `追跡 29 ファイル / 展開 29 ファイル / balloon/StayseeBalloon=29 / .nar 73021 バイト / 不一致 0 件`・`全ての検体で一致`・終了コード 0（確認後に生成物は消した。ブランチには残していない）。**したがって 29 ファイルの期待値はそのまま保てる。**
- **改行の変換は起きない**: `vendors/sample_ghost/.gitattributes` の `* -text` により、`git check-attr text -- vendors/sample_ghost/StayseeBalloon.nar` は `text: unset` を返す（追加前でも属性は効いている）。
- **展開フォルダの綴りを現在形で語る文書**（本 spec の brief を除く・2026-09-23 grep）: `.kiro/steering/product.md`（既定バルーンの節「`vendors/sample_ghost/StayseeBalloon/` に無改変で保管」）・`.kiro/steering/structure.md`（Sample Ghost Kit Crate の「検体の顔ぶれ」の項「展開フォルダのままで登記表に載っていない」）・`vendors/sample_ghost/README.md`（先頭の表に `StayseeBalloon` の行が無い）・`.kiro/specs/areka-P0-alpha-release-signoff/brief.md`（「zip に入れるもの」の項）・`.kiro/specs/areka-P0-baseware-root-layout/brief.md`（「保管先は展開フォルダ・`.nar` へ畳むのは `nar-install` が引き受ける」の行）。`.kiro/steering/roadmap.md` の #37 行と `roadmap-history.md` は経緯の記録なので現在形ではない。`.kiro/steering/tech.md` は既に `.nar` 前提で書かれている。

### 何を変えるか

`StayseeBalloon` を他の検体 5 体と同じ保管形・同じ取り出し方に揃える。**新しい仕組みは作らない。** 既存の畳む道具で `.nar` を作り、登記表に 1 行足し、直書きの数と名前を 6 体に揃え、既定バルーンの結合テストが展開フォルダではなく窓口から根を取るように付け替え、展開フォルダを追跡から外し、文書の綴りを直す。バルーンの中身は 1 バイトも変えない。

## Boundary Context

- **In scope**: `.nar` への畳み込み／登記表 `SAMPLES` への 1 行／直書きの検体数と名前の一覧の追随／`areka-emo-text` の既定バルーン結合テストの根の取り方の付け替え／展開フォルダを追跡から外す／文書の綴りの追随と後続 spec への申し送り。
- **Out of scope**: バルーン無指定時の既定バルーンの自動採用（`areka-P0-baseware-root-layout`）／配布 zip への同梱手順（`areka-P0-alpha-release-signoff`。ただし「出どころが `.nar` になった」ことは同 brief へ申し送る）／バルーンの中身の変更（無改変が CC0 同梱の前提）／`areka-nar` 本体と `fold-samples` の振る舞いの変更（使うだけ）／他の検体 5 体。
- **Adjacent expectations**:
  - 窓口 `sample-ghost-kit` は「`.nar` を 1 つ置いて登記表に 1 行」で検体が増やせる作り（完了 spec `areka-P0-nar-install` 要件 1.5）である。本仕様はその手順（`vendors/sample_ghost/README.md`「検体を 1 本足す手順」）に従うだけで、窓口には手を入れない。
  - 常設の見張り `sample_path_guard_test.rs` は、登記した検体の展開フォルダを `/` 付きで綴った形が窓口の外に残っていれば赤にする。本仕様はこれを書き換えない（末尾 `/` の無い裸のフォルダ名を拾わない穴は、本仕様の後は読み先のフォルダが消えて結合テストが赤になるので実害が無い。塞ぐのは本仕様の外）。
  - 中身のバイト列を常設のテストで固定することは**しない**（要件ディスカッション議題 1・2026-09-23 裁定）。本仕様の目的は保管形の切り替えで、中身の保証は完了 spec `areka-P0-default-balloon-bundle` が持つ（常設の判定はファイル名 29 本と画像の縦横で、今日と同じ強さ）。中身を変えるには `.nar` を作り直すしかなく、その差分は PR に必ず現れる。中身の照合は要件 1.4 の一度きりの照合で行う。
  - 本仕様は完了 spec `areka-P0-default-balloon-bundle` の要件 3.2（検体パスは定数 1 か所）と設計 C2（`STAYSEE_BALLOON_DIR` を `CARGO_MANIFEST_DIR` 基点で組む）を**上書きする**。以後の取り出し方は「窓口から名前で引く」である。完了アーカイブは書き換えない。
  - 完了 spec `areka-P0-default-balloon-bundle` が保証した「29 ファイル無改変・表示が正しい」は、本仕様の後も同じ結合テスト（`staysee_balloon_fixture_test`）が同じ期待値で守り続ける。
  - 下流 `areka-P0-baseware-root-layout` は既定バルーンを id `StayseeBalloon` で窓口から引くだけで、登記表には触らない。下流 `areka-P0-alpha-release-signoff` は zip に入れる 29 ファイルを `.nar` の展開結果から採る。

## Requirements

### Requirement 1: 保管形を配布形（`.nar`）へ切り替える

**Objective:** 次に検体を足す開発者として、既定バルーンも他の検体と同じ配布形で保管されていてほしい。保管形が 1 種類なら、置き場の決まりと手順が例外なしに成り立つからである。

#### Acceptance Criteria

1. The リポジトリ shall `vendors/sample_ghost/StayseeBalloon.nar` を追跡ファイルとして持つ。
2. The リポジトリ shall `vendors/sample_ghost/StayseeBalloon/` の配下に追跡ファイルを **0 本**持つ（`git ls-files vendors/sample_ghost/StayseeBalloon` が 1 行も返さない）。
3. When `StayseeBalloon.nar` を本番の展開器で空の根へ展開する, the 展開結果 shall `<根>/balloon/StayseeBalloon/` の直下に、畳む前の展開フォルダに在った 29 ファイル（`install.txt` を含む）と**同じ名前・同じバイト列**のファイルを持ち、それ以外のファイルを **0 本**持つ。
4. When `StayseeBalloon.nar` を畳んだ直後に窓口で展開する, the 展開結果の 29 ファイル shall 完了 spec `areka-P0-default-balloon-bundle` の `verification/provenance.md`「ハッシュ一覧」に記録された 29 本の sha256 と 1 本ずつ一致し（改行や文字コードの変換が起きていない）、その照合の記録（29 本の突き合わせ結果と不一致 0 件）を本 spec の検証記録に残す。
5. The 切り替え shall 畳み込み・登記・数の追随・結合テストの付け替え・追跡から外す操作を **1 つのコミット**で行い、「`.nar` が無いのに展開フォルダも無い」「登記があるのに `.nar` が無い」のいずれの途中状態もブランチの履歴に残さない。

### Requirement 2: 窓口から名前で取り出せる

**Objective:** テストと下流 spec の開発者として、`StayseeBalloon` を他の検体と同じ窓口から名前で引きたい。在処を自分で綴らずに済み、置き場が動いても直す場所が登記表 1 か所で済むからである。

#### Acceptance Criteria

1. The 登記表 `SAMPLES` shall `StayseeBalloon` の行を持ち、種別はバルーン、同梱バルーンは **0 本**とする。
2. When `SampleRoot::acquire("StayseeBalloon")` を呼ぶ, the 窓口 shall 根を配り、その `folder()` は `<根>/balloon/StayseeBalloon/` を指し、そのフォルダは実在する。
3. When 配られた根を捨てる, the 窓口 shall 他の検体と同じく複製の木を消す。
4. When `cargo run -p sample-ghost-kit --bin nar-sample-path -- StayseeBalloon` を実行する, the コマンド shall `root=`／`folder=` の行を印字して終了コード 0 で終わる。
5. When 未登録の名前で窓口を呼ぶ, the 失敗の理由 shall 既知の名前 **6 つ**（`StayseeBalloon` を含む）を列挙する。

### Requirement 3: 検体数と名前の一覧の直書きを 6 体に揃える

**Objective:** 窓口を保守する開発者として、検体数を直書きしている検査が新しい母数を正しく持ってほしい。数が古いままだと、検査そのものが赤になるか、あるいは「母数 0 で緑」を防ぐ役目を果たさなくなるからである。

#### Acceptance Criteria

1. The `every_registered_sample_lands_where_its_registry_row_says` shall 登記の数を **6** と突き合わせる。
2. The `every_sample_nar_installs_exactly_the_elements_its_registry_row_declares` shall 登記の数を **6** と突き合わせる。
3. The 未登録名の失敗を検査するテスト shall 既知の名前の一覧（関数内の 2 か所）に `StayseeBalloon` を持ち、関数名の「five」に当たる語も 6 体を表す語へ改める。
4. The `crates/sample-ghost-kit/src/lib_tests.rs` shall 検体数「5」を語る doc コメント（module doc・`every_registered_sample_lands_where_its_registry_row_says` の doc・未登録名テストの doc の 3 か所）と assert の文言（「登記は検体 5 つ」の 2 か所）を 6 へ改め、検体数として「5」を語る箇所を **0 か所**持つ。
5. The 直書きの検体数 shall 引き算や `SAMPLES.len()` の写しで導かず、逐語の数として書く（母数 0 で緑になる形を作らない）。
6. When `cargo test -p sample-ghost-kit` を実行する, the テスト shall 全件緑で終わる。

### Requirement 4: 既定バルーンの結合テストは窓口から根を取る

**Objective:** 既定バルーンの表示保証を守る開発者として、`areka-emo-text` の結合テストが展開フォルダを綴らずに窓口から根を取ってほしい。展開フォルダが消えてもテストが赤にならず、同じ 29 ファイルの期待値がそのまま守られるからである。

#### Acceptance Criteria

1. The `staysee_balloon_fixture_test` とテーマ別ファイル群 shall 検体の根を窓口 `sample-ghost-kit` から `StayseeBalloon` の名前で取り、`vendors/sample_ghost/StayseeBalloon` の綴りを**コメントを含めて 0 か所**持つ。消えた定数 `STAYSEE_BALLOON_DIR` を説明する記述（入口ファイルの module doc と `test_support.rs` の doc）も残さない。
2. The 期待値 `EXPECTED_FILE_NAMES` shall 29 ファイル（`install.txt` を含む）のまま変えない。
3. While 結合テストのプロセスが生きている, the 取得した根 shall テーマ別ファイル群のどの読み出しからも同じ位置として見え、途中で消えない。
4. When `cargo test -p areka-emo-text --test staysee_balloon_fixture_test` を実行する, the テスト shall 全件緑で終わる。
5. When `cargo test -p log-capture-kit --test sample_path_guard_test` を実行する, the 見張り shall 窓口の外に展開形の検体フォルダの綴りを **0 件**検出して緑で終わる（登記した 6 体の名前で組んだ走査語が窓口の外に現れないことの確認。付け替え漏れそのものを赤にするのは 4.4）。

### Requirement 5: 文書の綴りを追随させ、下流へ申し送る

**Objective:** 次に検体を足す開発者と下流 spec の開発者として、文書が「展開フォルダで保管されている」と現在形で語らないでほしい。古い綴りを読んだ人が存在しないフォルダを探し、手順を誤るからである。

#### Acceptance Criteria

1. The `vendors/sample_ghost/README.md` shall 先頭の表に `StayseeBalloon.nar` の行（種別・中のファイル数 29・大きさ）を持ち、「ここで畳んだ 4 本（`konnoyayame.nar` 以外）は全エントリが無圧縮」の記述を実物に合わせ、`fold_tree` で畳んだ無圧縮の 4 本（`StayseeBalloon.nar` を含む）を名指しし、deflate 圧縮の 2 本（`konnoyayame.nar`・`emo2.nar`）と区別して語る（2026-09-24 実装時の訂正: `emo2.nar` は 2026-09-20 の最新版への差し替えで全エントリが deflate になっており、「5 本」と書くと偽になる）。
2. The `.kiro/steering/product.md` と `.kiro/steering/structure.md` shall 既定バルーンの保管を `.nar`＋登記表経由として語り、「展開フォルダのまま」「登記表に載っていない」の記述を **0 か所**持つ。`structure.md`「検体の顔ぶれ」の「バルーン 2 本」は 3 本へ改める。
3. The `.kiro/specs/areka-P0-alpha-release-signoff/brief.md` と `.kiro/specs/areka-P0-baseware-root-layout/brief.md` shall 既定バルーンの出どころを `vendors/sample_ghost/StayseeBalloon.nar`（窓口で展開した結果）として語り、「`.nar` へ畳むのは `nar-install` が引き受ける」の申し送りを持たない。
4. The `.kiro/steering/roadmap.md` の台帳 #42 の行 shall 本 spec の完了を反映する（経緯を記した #37 の行と `roadmap-history.md` は書き換えない）。
5. The `crates/sample-ghost-kit/examples/fold-samples.rs` の module doc shall 呼び方の例を実在しないフォルダではなく `<展開フォルダ>` の置き換えで示す（doc コメントだけの変更で、`fold-samples` の振る舞いは変えない）。
6. When `vendors/sample_ghost/StayseeBalloon/` の綴りを、本 spec 自身と完了 spec の保存文書と経緯の記録を除く現行文書（steering・進行中 spec の brief・`vendors/sample_ghost/README.md`）と `crates/` のソース（コメントを含む）から検索する, the 結果 shall 現在形の記述を **0 件**とする。
