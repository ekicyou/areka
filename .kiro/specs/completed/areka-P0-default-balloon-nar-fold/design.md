# Design Document: areka-P0-default-balloon-nar-fold

> 2026-09-23・本ブランチ（main `92f5f448` 相当）で全ファイルを読み直して書いた。コードは「何の定義か」（関数名・定数名＋ファイルパス）で指し、行番号では指さない。要件の番号は `requirements.md` の `N.M` をそのまま使う。

## Overview

**Purpose**: 既定バルーン `StayseeBalloon` の保管形を、他の検体 5 体と同じ配布形（`vendors/sample_ghost/StayseeBalloon.nar`）＋登記表 `SAMPLES` 経由の取り出しに揃える。保管形が 1 種類になれば、「検体を 1 本足す手順」（`vendors/sample_ghost/README.md`）が例外なしに成り立つ。

**Users**: 次に検体を足す開発者／既定バルーンの結合テストを保守する開発者／下流 spec `areka-P0-baseware-root-layout`（窓口から id で引く）と `areka-P0-alpha-release-signoff`（zip へ入れる 29 ファイルを `.nar` の展開結果から採る）。

**Impact**: 展開フォルダ `vendors/sample_ghost/StayseeBalloon/`（追跡 29 本）が消え、`StayseeBalloon.nar` 1 本と `SAMPLES` の 1 行に置き換わる。`areka-emo-text` の結合テストは根の取り方だけが変わり、期待値（29 ファイル名・画像の縦横）は 1 つも変わらない。**新しい仕組みは 0**——畳む道具・登記表・窓口・プロセス寿命で根を保持する雛形は全て既存で、本設計はそれを写すだけである。

### Goals

- `.nar` 1 本＋登記 1 行で `SampleRoot::acquire("StayseeBalloon")` が根を配る（1.1〜1.3・2.1〜2.5）。
- 直書きの検体数と名前の一覧を 6 体へ揃える（3.1〜3.6）。
- 結合テスト `staysee_balloon_fixture_test` が窓口から根を取り、同じ期待値で緑のまま（4.1〜4.5）。
- 展開フォルダの綴りを現在形で語る文書を 0 件にし、下流 2 spec へ「出どころは `.nar`」を申し送る（5.1〜5.6）。
- 畳んだ直後に 29 本の sha256 を完了 spec の記録と照合し、その記録を本 spec に残す（1.4）。
- 途中状態（`.nar` も展開フォルダも無い／登記だけ在る）を履歴に残さない（1.5）。

### Non-Goals

- バルーン無指定時の既定バルーンの自動採用（`areka-P0-baseware-root-layout`）。
- 配布 zip への同梱手順（`areka-P0-alpha-release-signoff`。申し送りの 1 行だけ書く）。
- バルーンの中身の変更（0 バイト）。中身のバイト列を常設のテストで固定すること（2026-09-23 裁定＝**しない**。research.md §8）。
- `areka-nar` 本体・`fold-samples`・窓口 `sample-ghost-kit` の振る舞いの変更（0 行。`fold-samples.rs` は doc コメントの例だけ直す＝5.5）。
- 見張り `sample_path_guard_test.rs` の変更（0 行）。末尾 `/` の無い裸のフォルダ名を拾わない穴は本仕様の外（research.md §3.1・§8）。
- 検体の受け口を crate 横断の共有ヘルパへ寄せること（research.md §4 案 B・却下）。
- 他の検体 5 体（0 変更）。

## Boundary Commitments

### This Spec Owns

- `vendors/sample_ghost/` の中の `StayseeBalloon` の保管形（`.nar` 1 本・展開フォルダ 0 本）と、その README の表の 1 行。
- 登記表 `SAMPLES`（`crates/sample-ghost-kit/src/lib.rs`）の `StayseeBalloon` の行と、同 crate の自己テスト `lib_tests.rs` の検体数・名前の一覧・doc の数。
- `crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs` の `staysee_root()` の**本体**（根をどこから取るか）と、入口 `staysee_balloon_fixture_test.rs` の定数 `STAYSEE_BALLOON_DIR` の削除。
- 既定バルーンの保管を語る文書の綴り（steering 2 本・進行中 brief 2 本・roadmap #42・`fold-samples.rs` の doc の例）。
- 要件 1.4 の照合記録（`.kiro/specs/completed/areka-P0-default-balloon-nar-fold/verification/`）。

### Out of Boundary

- 窓口 `SampleRoot`／`manual_paths`／`devroot.rs`／`nar_writer.rs`／`fold-samples` の振る舞い（使うだけ）。
- 展開器 `areka-nar`（`plan.rs` の `body_placement` が最上位 `install.txt` を `type,balloon` で置くことに依存するが、変更 0）。
- 見張り `crates/log-capture-kit/tests/sample_path_guard_test.rs`（`Form::tokens` の語形を変えない）。
- テーマ別 8 ファイル（`assets`・`definition`・`faces`・`bake`・`region`・`wrapping`・`script`・`scale`）の中身と期待値（`EXPECTED_FILE_NAMES` 29 本・`EXPECTED_FRAME_SIZES`）。
- 完了 spec `areka-P0-default-balloon-bundle` のアーカイブ（書き換えない。要件 3.2「検体パスは定数 1 か所」と設計 C2 は本仕様が**上書き**する＝以後の取り出し方は「窓口から名前で引く」）。
- `.kiro/steering/roadmap.md` の #37 の行と `roadmap-history.md`（経緯の記録）。

### Allowed Dependencies

- `sample-ghost-kit`（`areka-emo-text` の `[dev-dependencies]` に既に在る＝`crates/areka-emo-text/Cargo.toml`）。**`Cargo.toml` の変更は 0**・新規 crate 0・新規外部依存 0。
- `std::sync::LazyLock`（雛形 `crates/areka-emo-compose/src/sample_test_support.rs` の `static EMO2` と同じ）。
- 依存の向きは今と同じ: `areka-emo-text`（テスト）→ `sample-ghost-kit` → `areka-nar`。逆向きの追加は 0。

### Revalidation Triggers

- 登記の名前 `"StayseeBalloon"` または種別を変えたとき（下流 `baseware-root-layout` は id `StayseeBalloon` で引く）。
- `.nar` を作り直したとき（PR に `StayseeBalloon.nar` の差分が必ず現れる。中身の保証は `staysee_balloon_fixture_test` の 29 ファイル名＋画像の縦横で、今日と同じ強さ）。
- `SampleRoot::folder()` の返す位置（`<根>/balloon/<名>/`）が変わったとき——`staysee_root()` の契約が変わる。
- `SAMPLES` の行数が変わったとき——`lib_tests.rs` の逐語の数 `6` を追随させる（`shell-balloon-switch` が検体を足すと再発）。

## Architecture

### Existing Architecture Analysis

- **窓口の作り**（`crates/sample-ghost-kit/src/lib.rs`）: `SAMPLES` の 1 行 `Sample { name, kind, balloons }` から `SampleRoot::acquire(name)` が `vendors/sample_ghost/<名>.nar`（`devroot.rs` の `nar_dir()`）を展開した原本の使い捨ての複製を配り、`from_copy` の `check_registry` が展開結果の要素（`balloon/StayseeBalloon`）を登記と照合してから `folder()` ＝ `<根>/balloon/<名>/` を組む。既知の名前の一覧は `known_sample_names()` が毎回 `SAMPLES` から導く＝**2.5 は登記 1 行で自動的に 6 つになる**。`manual_paths`（`nar-sample-path` bin）も同じ `registered()` を通る＝**2.4 も登記だけで成り立つ**。
- **プロセス寿命で根を保持する雛形**（`crates/areka-emo-compose/src/sample_test_support.rs`）: `static EMO2: LazyLock<SampleRoot>` ＋ `emo2_root()` が `folder().to_path_buf()` を返す。`SampleRoot` は `Drop` で複製を消すので、関数内の一時値にすると借用の元がその場で消える。`static` は破棄されないため「プロセスの間ずっと同じ位置・途中で消えない」（4.3）が型で成り立つ。
- **付け替え元**（`crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs`）: `staysee_root() -> PathBuf` が `CARGO_MANIFEST_DIR` に `super::STAYSEE_BALLOON_DIR` を継ぎ足す。`staysee_root()` を直接呼ぶのは `assets.rs`・`bake.rs`・`faces.rs` と `test_support.rs` 自身（`resolve_faces`・`read_decoded`）で、他のテーマは `staysee_model()`・`resolve_staysee()` 経由。**戻り型 `PathBuf` を保てばテーマ別 8 ファイルは 0 行変更**。
- **畳む道具**（`crates/sample-ghost-kit/examples/fold-samples.rs` の `main`・`fold_one`）: `git ls-files` の追跡ファイルだけを写して `fold_tree` で畳み（名前順・日時固定・無圧縮＝同じ入力から同じバイト列）、本番の展開器で空の根へ戻して 1 バイト単位で突き合わせる。要件段階の実走で `追跡 29 / 展開 29 / balloon/StayseeBalloon=29 / .nar 73021 バイト / 不一致 0 件`。
- **`install.txt` が展開先に残る根拠**（`crates/areka-nar/src/plan.rs` の `body_placement`）: `skip_install_txt = manifest.kind == InstallKind::Supplement` なので、`type,balloon` では最上位の `install.txt` も置かれる＝`EXPECTED_FILE_NAMES` の 29 本（`install.txt` 込み）はそのまま。
- **改行の変換**: `vendors/sample_ghost/.gitattributes` の `* -text` が `StayseeBalloon.nar` にも効く（追加前でも `git check-attr` が `text: unset`）。
- **見張り**（`crates/log-capture-kit/tests/sample_path_guard_test.rs` の `Form::tokens`）: `CheckedInTree` の走査語は `format!("{VENDOR_PARENT}/{}/", s.name)`＝末尾 `/` 付き。登記後に本仕様が残す綴りは `.nar` のファイル名と検体名だけなので、**見張りは 0 件で緑**（4.5）。付け替え漏れを赤にするのは、フォルダが消えた後に読み先が実在しなくなる結合テスト（4.4）。

### Architecture Pattern & Boundary Map

- **Selected pattern**: 既存の手順（README「検体を 1 本足す手順」）と既存の雛形（`static LazyLock<SampleRoot>`）の**写し**。research.md §4 の案 A。
- **Domain boundaries**: 資産（`vendors/`）／登記（`sample-ghost-kit`）／消費（`areka-emo-text` のテスト）／文書の 4 域。互いに共有するのは検体名 `"StayseeBalloon"` の 1 語だけ。
- **Existing patterns preserved**: 「`.nar` を 1 つ置いて登記表に 1 行」（完了 spec `nar-install` 要件 1.5）／「テストは在処を綴らず窓口から引く」／「保持は消費側が `static` で決める」（`areka-emo-compose` と同型）。
- **New components**: 0。
- **Steering compliance**: `tech.md`「検体は配布形 `.nar` のまま保管し、テストは窓口越しに引く」に揃える。`structure.md` の Sample Ghost Kit の項に書かれた例外（「展開フォルダのまま」）を消す。
- 図: 0（1 コンポーネントの本体差し替えなので、構造図は要らない）。

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| 資産 | `vendors/sample_ghost/StayseeBalloon.nar`（zip・全エントリ無圧縮） | 既定バルーンの保管形 | `fold-samples` が生成。新規依存 0 |
| テスト基盤 | `sample-ghost-kit`（既存・dev 依存） | 名前で根を配る窓口 | `Cargo.toml` 変更 0 |
| 標準ライブラリ | `std::sync::LazyLock` | 根のプロセス寿命での保持 | 雛形と同じ |
| 照合 | `sha256sum`（Git Bash 同梱） | 1.4 の一度きりの照合 | コードに持ち込まない |

## File Structure Plan

### Directory Structure

```
vendors/sample_ghost/
├── StayseeBalloon.nar                      # 新規（追跡）: 畳んだ既定バルーン・29 ファイル
├── StayseeBalloon/                         # 削除（追跡 29 本 → 0 本・実体も消す）
└── README.md                               # 表に 1 行・「畳んだ 4 本」を実物どおり名指し・出どころの 1 行
crates/sample-ghost-kit/
├── src/lib.rs                              # SAMPLES に 1 行
├── src/lib_tests.rs                        # 数 5→6（assert 2・doc 3）・名前の一覧 2 か所・関数名・種別テストに 1 名
└── examples/fold-samples.rs                # module doc の呼び方の例だけ（振る舞い 0 変更）
crates/areka-emo-text/tests/
├── staysee_balloon_fixture_test.rs         # 定数 STAYSEE_BALLOON_DIR と その説明の doc を削除・module doc を書き直す
└── staysee_balloon_fixture/test_support.rs # staysee_root() の本体を static LazyLock<SampleRoot> へ
.kiro/steering/
├── product.md                              # 既定バルーンの節の保管の綴り
├── structure.md                            # Sample Ghost Kit の「検体の顔ぶれ」（バルーン 2 本→3 本・例外の文を削除）
└── roadmap.md                              # 台帳 #42 の行（完了の反映）
.kiro/specs/
├── areka-P0-alpha-release-signoff/brief.md # 「zip に入れるもの」の出どころを .nar へ
├── areka-P0-baseware-root-layout/brief.md  # 保管先の行を .nar へ・「nar-install が引き受ける」を削除
└── areka-P0-default-balloon-nar-fold/verification/nar-roundtrip.md  # 新規: 1.4 の照合記録
```

新規の Rust ファイル: 0。新規 crate: 0。`Cargo.toml`／`Cargo.lock` の変更: 0。テーマ別 8 ファイル（`assets.rs`〜`scale.rs`）の変更: 0 行。

### Modified Files

- `vendors/sample_ghost/StayseeBalloon.nar` — `fold-samples -- --from vendors/sample_ghost/StayseeBalloon` の生成物をそのまま追跡（1.1・1.3）。
- `vendors/sample_ghost/StayseeBalloon/`（29 本） — `git rm -r --cached` の後に実体を消す（1.2）。
- `vendors/sample_ghost/README.md` — 先頭の表に `StayseeBalloon.nar | バルーン（areka の既定バルーン・CC0） | 29 | <実走のバイト数>` の行、「ここで畳んだ 4 本」を無圧縮の 4 本（`StayseeBalloon.nar` を含む）の名指しと deflate の 2 本（`konnoyayame.nar`・`emo2.nar`）の区別へ（2026-09-24 実装時の訂正: `emo2.nar` は 2026-09-20 の最新版への差し替えで全エントリが deflate になっており、「5 本」と書くと偽になる）、出どころの 1 行（後述の決定 B）（5.1）。
- `crates/sample-ghost-kit/src/lib.rs` — `SAMPLES` の末尾に `Sample { name: "StayseeBalloon", kind: SampleKind::Balloon, balloons: &[] }`（2.1）。
- `crates/sample-ghost-kit/src/lib_tests.rs` — 後述「登記の自己テストの追随」（3.1〜3.5・2.1 の直接の判定）。
- `crates/sample-ghost-kit/examples/fold-samples.rs` — module doc「呼び方」の 2 行を `--from vendors/sample_ghost/<展開フォルダ>` の置き換えに（5.5）。
- `crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs` — 後述「`staysee_root()` の契約」（4.1〜4.3）。
- `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs` — 定数 `STAYSEE_BALLOON_DIR` とその見出し・doc を削除。module doc の「このテストが塞ぐ穴」の `vendors/sample_ghost/StayseeBalloon/` の綴りと「検体パスは 1 定数だけが持つ（要件 3.2）」の節を、「検体は窓口 `sample-ghost-kit` から `StayseeBalloon` の名前で引く（出典 spec `areka-P0-default-balloon-nar-fold`）」の 1 節に置き換える。消える定数を語る文は module doc に **2 つ**あり、どちらも書き換える: 「付け替えるのは [`STAYSEE_BALLOON_DIR`] の 1 行だけで済む形にしてある」→「付け替えるのは `test_support.rs` の `staysee_root()` の本体だけ」・「本ファイルは検体パス定数と接続宣言だけを持つ入口である」→「本ファイルは接続宣言だけを持つ入口である」。テーマ別ファイルの表と「置き場がディレクトリである理由」は残す（4.1）。
- `.kiro/steering/product.md` — 既定バルーンの節「`vendors/sample_ghost/StayseeBalloon/` に無改変で保管」→「`vendors/sample_ghost/StayseeBalloon.nar` に無改変で保管・登記表経由」（5.2）。
- `.kiro/steering/structure.md` — Sample Ghost Kit の「検体の顔ぶれ」: 「バルーン 2 本」→「3 本（`StayseeBalloon`＝areka の既定バルーン・CC0 を含む）」、末尾の「展開フォルダのままで登記表に載っていない（畳むのは roadmap 台帳 #42）」の文を削除（5.2）。
- `.kiro/specs/areka-P0-alpha-release-signoff/brief.md` — 「zip に入れるもの」の出どころを「`vendors/sample_ghost/StayseeBalloon.nar` を窓口で展開した `balloon/StayseeBalloon/` の 29 ファイル」へ。冒頭の「着地すると…になる」の予告文は「着地済み」の現在形へ（5.3）。
- `.kiro/specs/areka-P0-baseware-root-layout/brief.md` — 「保管先は `vendors/sample_ghost/StayseeBalloon/`（展開フォルダ…）。`.nar` へ畳むのは `areka-P0-nar-install` が引き受ける」→「保管先は `vendors/sample_ghost/StayseeBalloon.nar`（登記表 `SAMPLES` 経由・`SampleRoot::acquire("StayseeBalloon")` で引く）」。冒頭の追記「畳む仕事は `areka-P0-default-balloon-nar-fold`（台帳 #42）が**持つ**…**偽になった**」は「#42 が畳んだ（着地済み）」の過去形へ（5.3。この 1 文は `/` 付きの綴りを持たないので 5.6 の検索には掛からない＝手で直す）。
- `.kiro/steering/roadmap.md` — 台帳 #42 の行: 状態列と段列を ✅ に・名前を `completed/default-balloon-nar-fold` に（5.4。台帳の注記どおり `/kiro-complete` が行う）。本文は #37 の行と同じく経緯として残す。#37 の行と `roadmap-history.md` は触らない。
- `.kiro/specs/completed/areka-P0-default-balloon-nar-fold/verification/nar-roundtrip.md` — 新規。1.4 の照合記録。

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|---|---|---|---|---|
| 1.1 | `.nar` を追跡 | 資産 `StayseeBalloon.nar` | `fold-samples --from` | 手順 ① |
| 1.2 | 展開フォルダ 0 本 | 資産（削除） | `git rm -r --cached` | 手順 ④ |
| 1.3 | 展開結果が 29 本・同じバイト列 | `fold-samples` の往復照合（既存） | `fold_one` の印字 `不一致 0 件` | 手順 ① |
| 1.4 | sha256 が provenance と一致・記録 | `verification/nar-roundtrip.md` | `nar-sample-path` の `folder=` ＋ `sha256sum` | 手順 ② |
| 1.5 | 1 コミット・途中状態なし | 作業順（後述） | — | 手順 ①〜⑦ |
| 2.1 | 登記 1 行（バルーン・同梱 0） | `SAMPLES` の行 | `Sample { name, kind, balloons }` | — |
| 2.2 | `acquire` → `folder()` 実在 | 窓口（既存・0 変更） | `SampleRoot::acquire`／`folder` | 登記の往復テスト |
| 2.3 | 破棄で複製が消える | 窓口（既存） | `Drop` | `every_registered_sample_lands_where_its_registry_row_says` |
| 2.4 | `nar-sample-path` が印字 | `manual_paths`（既存） | bin の標準出力 | 手順 ② |
| 2.5 | 未登録名の失敗が 6 名 | `known_sample_names()`（既存） | `SampleError::UnknownSample` | `unknown_sample_fails_with_all_six_known_names` |
| 3.1 | 数 6（landing テスト） | `lib_tests.rs` | 逐語 `6` | — |
| 3.2 | 数 6（往復テスト） | `lib_tests.rs` | 逐語 `6` | — |
| 3.3 | 名前の一覧 2 か所＋関数名 | `lib_tests.rs` | `unknown_sample_fails_with_all_six_known_names` | — |
| 3.4 | doc 3 か所・assert 文言 2 か所 | `lib_tests.rs` | — | — |
| 3.5 | 逐語の数（引き算しない） | `lib_tests.rs` | — | — |
| 3.6 | `cargo test -p sample-ghost-kit` 緑 | — | — | 手順 ⑤ |
| 4.1 | 窓口から根・綴り 0 か所 | `test_support.rs`・入口ファイル | `staysee_root() -> PathBuf` | 手順 ⑦ ⒜ |
| 4.2 | `EXPECTED_FILE_NAMES` 29 本のまま | `test_support.rs`（該当部は 0 変更） | — | — |
| 4.3 | プロセス中ずっと同じ位置 | `static STAYSEE: LazyLock<SampleRoot>` | — | — |
| 4.4 | 結合テスト緑 | — | — | 手順 ⑤ |
| 4.5 | 見張り 0 件で緑 | 見張り（0 変更） | `Form::tokens` | 手順 ⑤ |
| 5.1 | README の表と無圧縮 4 本の名指し | README | — | 手順 ⑥ |
| 5.2 | steering 2 本 | product.md・structure.md | — | 手順 ⑥ |
| 5.3 | brief 2 本 | 2 brief | — | 手順 ⑥ |
| 5.4 | roadmap #42 | roadmap.md | — | 手順 ⑥ |
| 5.5 | `fold-samples.rs` の doc の例 | examples | — | 手順 ⑥・⑦ ⒜ |
| 5.6 | 現在形の綴り 0 件 | 検索（「文書の追随」の 2 本・期待ヒット明記） | `grep` | 手順 ⑦ ⒝ |

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies | Contracts |
|---|---|---|---|---|---|
| 資産 `StayseeBalloon.nar` | vendors | 既定バルーンの配布形 | 1.1, 1.2, 1.3 | `fold-samples`（P0） | Batch |
| 登記の 1 行 | sample-ghost-kit | 名前→根の対応 | 2.1〜2.5 | `SampleRoot`（P0・既存） | State |
| 登記の自己テストの追随 | sample-ghost-kit tests | 母数 6 を逐語で持つ | 2.1, 3.1〜3.6 | `SAMPLES`（P0） | — |
| `staysee_root()` の本体 | areka-emo-text tests | 窓口から根を取り保持 | 4.1〜4.4 | `sample-ghost-kit`（P0・dev 依存済み） | Service |
| 照合記録 | spec verification | 1.4 の一度きりの照合 | 1.4 | `nar-sample-path`・`sha256sum` | Batch |
| 文書の追随 | docs | 現在形の綴り 0 件 | 5.1〜5.6 | — | — |

### 資産（vendors）

#### `StayseeBalloon.nar`

| Field | Detail |
|---|---|
| Intent | 展開フォルダ 29 本を 1 本の `.nar` に畳んだもの |
| Requirements | 1.1, 1.2, 1.3 |

**Responsibilities & Constraints**
- 中身は追跡 29 本と同じ名前・同じバイト列（`fold-samples` が畳む時点で往復照合し、`不一致 0 件` でなければ終了コードが 0 にならない）。
- 生成は 1 回。以後は `--check` も使えない（展開形が無い）ので、作り直すときは展開して `.nar` を作り直す＝PR に差分が必ず現れる。
- 全エントリ無圧縮（`fold_tree` の性質）。README「守る 5 点 ⑷」の本数が 5 になる。

**Contracts**: Batch [x]

##### Batch / Job Contract
- Trigger: `cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/StayseeBalloon`（展開フォルダがまだ在る状態で 1 回）。
- Input / validation: `git ls-files` の追跡 29 本（0 件なら落ちる）。
- Output: `vendors/sample_ghost/StayseeBalloon.nar`・印字 `StayseeBalloon: 追跡 29 ファイル / 展開 29 ファイル / balloon/StayseeBalloon=29 / .nar <N> バイト / 不一致 0 件`＋`全ての検体で一致`・終了コード 0。
- Idempotency: 名前順・日時固定なので同じ入力から同じバイト列。

### 登記（sample-ghost-kit）

#### 登記の 1 行

`SAMPLES` の末尾に 1 行。既存の行と同じ形。

```rust
Sample {
    name: "StayseeBalloon",
    kind: SampleKind::Balloon,
    balloons: &[],
},
```

- `name` は `.nar` のファイル名（拡張子無し）＝`install.txt` の `directory,StayseeBalloon`（実物で確認済み。`type,balloon`）。
- 2.2〜2.5 はこの 1 行で成り立つ（`check_registry`・`Drop`・`manual_paths`・`known_sample_names()` は全て `SAMPLES` から導く）。窓口の本体の変更は 0。

#### 登記の自己テストの追随（`lib_tests.rs`）

| 箇所（何の定義か） | 今 | 後 |
|---|---|---|
| module doc「既知の 5 つを含む失敗になる」 | 5 | 6 |
| `every_registered_sample_lands_where_its_registry_row_says` の doc「登記した 5 つの検体は」 | 5 | 6 |
| 同関数の `assert_eq!(SAMPLES.len(), 5, "登記は検体 5 つ")` | 5・「5 つ」 | 6・「6 つ」 |
| `unknown_sample_fails_with_all_five_known_names` の関数名 | five | six |
| 同関数の doc「既知の名前 5 つを含む」 | 5 | 6 |
| 同関数の名前の一覧（`assert_eq!(known, &[...])` と `for name in [...]` の 2 か所） | 5 名 | 末尾に `"StayseeBalloon"`（登記の順） |
| `every_sample_nar_installs_exactly_the_elements_its_registry_row_declares` の `assert_eq!(SAMPLES.len(), 5, "登記は検体 5 つ（…）")` | 5・「5 つ」 | 6・「6 つ」 |
| `registry_records_kind_and_bundled_balloons` のバルーンのループ `for name in ["emo2-kakukaku-offsetdpi", "emo2-kakukaku-wplimit"]` | 2 名 | 3 名（`"StayseeBalloon"` を追加） |

- 数は逐語の `6`。`SAMPLES.len()` の写しや引き算では書かない（3.5）。
- 改めた後、ファイル内で検体数として「5」を語る箇所は 0（3.4）。`BOOT_RECORD: [&str; 5]` の 5 は配列長であって検体数ではない（触らない）。
- **決定 A**（research.md §8 の持ち越し項目 3）: `registry_records_kind_and_bundled_balloons` に `StayseeBalloon` を**足す**。足し方は既存のバルーンのループに名前を 1 つ加えるだけ（新しい assert は 0）。理由: 2.1「種別はバルーン・同梱 0 本」をこの 1 語で直接判定でき、ループの assert 文言 `"{name} は同梱バルーン 0"` が「同梱 0」を明示的に書く。`konnoyayame` がこのテストに無いことは本仕様の外（他の検体 5 体は 0 変更）。

### 消費（areka-emo-text のテスト）

#### `staysee_root()` の本体

| Field | Detail |
|---|---|
| Intent | 既定バルーンのフォルダを窓口から取り、プロセス寿命で保持して `PathBuf` で配る |
| Requirements | 4.1, 4.2, 4.3, 4.4 |

**Responsibilities & Constraints**
- 呼び手（`assets.rs`・`bake.rs`・`faces.rs`・`test_support.rs` 内の `resolve_faces`／`read_decoded`）から見た契約は今と同じ `pub(crate) fn staysee_root() -> PathBuf`。戻り型を変えないので**テーマ別 8 ファイルは 0 行変更**。
- 返す位置は `SampleRoot::folder()`＝`<根>/balloon/StayseeBalloon/`。直下に 29 本・サブフォルダ 0（`assets.rs` の `stored_folder_holds_exactly_the_upstream_29_files` が `read_dir` で判定する形はそのまま通る）。
- 根の保持は `static`。テストバイナリ 1 本につき複製 1 つ。プロセス終了で札は OS が閉じ、残った木は次の取得時の回収（`nar-install` の設計どおり）が消す。

**Dependencies**
- Outbound: `sample_ghost_kit::SampleRoot` — 根の取得（P0・`[dev-dependencies]` に既存）。
- External: 0。

**Contracts**: Service [x]

##### Service Interface

```rust
// crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs
use std::sync::LazyLock;
use sample_ghost_kit::SampleRoot;

/// 既定バルーンの検体。プロセス寿命で保持する（`SampleRoot` は破棄で複製の木を消すため）。
static STAYSEE: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire("StayseeBalloon").expect("StayseeBalloon は登記済みの検体")
});

/// 既定バルーンのフォルダ `<根>/balloon/StayseeBalloon/`。
pub(crate) fn staysee_root() -> PathBuf {
    STAYSEE.folder().to_path_buf()
}
```

- Preconditions: `SAMPLES` に `StayseeBalloon` が登記済み・`vendors/sample_ghost/StayseeBalloon.nar` が在る。
- Postconditions: 返したパスは実在するフォルダで、プロセスの間は同じ値・消えない（4.3）。
- Invariants: 検体の在処の綴り（`vendors/sample_ghost/StayseeBalloon`）はこのファイルにも入口にも 0 か所（4.1）。持つのは検体名 `"StayseeBalloon"` の 1 語だけ。
- 失敗: 登記漏れ・`.nar` 欠落は `expect` で名指しの panic（黙って空のパスを配らない）。

**Implementation Notes**
- Integration: 入口ファイル `staysee_balloon_fixture_test.rs` から定数 `STAYSEE_BALLOON_DIR`（とその見出しコメント）を消し、`test_support.rs` の `staysee_root` の doc「`crate::STAYSEE_BALLOON_DIR` を実体化する…`super::` で引くに留める」を上の doc に置き換える。`test_support.rs` の module doc の出典行「要件 2.2／3.2／5.4・設計 C2」は「要件 2.2／5.4（根の取り方は `areka-P0-default-balloon-nar-fold` 4.1〜4.3）」へ。
- Validation: `cargo test -p areka-emo-text --test staysee_balloon_fixture_test` 全件緑（4.4）。展開フォルダが消えた後に走らせることで、付け替え漏れがあれば読み先の不在で赤になる。
- Risks: `static` は `SampleRoot: Sync` を要るが、`areka-emo-compose` の `static EMO2` が同じ形で既にコンパイルされている＝新しい前提は 0。

### 照合記録（spec verification）

#### `verification/nar-roundtrip.md`

- Trigger: `.nar` を畳んだ直後（手順 ②）。
- Input: `cargo run -p sample-ghost-kit --bin nar-sample-path -- StayseeBalloon` が印字する `folder=` の 29 本（2.4 の確認を兼ねる）。
- Method: `sha256sum` を 29 本に当て、完了 spec `.kiro/specs/completed/areka-P0-default-balloon-bundle/verification/provenance.md` §3.1 の表（ファイル名・バイト数・sha256）と行単位で突き合わせる。
- Output: 29 行の突き合わせ結果と「不一致 0 件」、使った命令、`fold-samples` の印字（`.nar` のバイト数を含む）、`nar-sample-path -- StayseeBalloon` の印字 2 行（`root=`／`folder=`）と終了コード 0 を逐語で（2.4 の直接の証跡）、`git check-attr text -- vendors/sample_ghost/StayseeBalloon.nar` の `text: unset`。
- 常設化: しない（2026-09-23 裁定）。

### 文書の追随

- **決定 B**（research.md §8 の持ち越し項目 6）: `vendors/sample_ghost/README.md` には**節を設けず、表の 1 行＋出どころの箇条書き 1 行**にする。書くのは「CC0（`LICENSE`・`readme.txt`）・上流 `https://github.com/ponapalt/StayseeBalloon`・areka の既定バルーン・畳み直し可・配布物へ同梱可・29 本のハッシュと上流との突合は `.kiro/specs/completed/areka-P0-default-balloon-bundle/verification/provenance.md` §1・§3」。理由: `konnoyayame.nar` の節が在るのは、読む人が守らねばならない禁止（畳み直さない・配布物へ入れない）を持つからで、`StayseeBalloon` にはその禁止が 0。29 本の sha256 を README にも写すと同じ表が 2 か所になり、片方が黙って古びる。`tech.md`「第三者の検体は出どころとライセンスを同フォルダの README に登記」は 1 行で満たす（出どころとライセンスは書き、詳細はポインタ）。
- 完了判定の検索（手順 ⑦・要件 4.1／5.5／5.6）は **2 本**で、それぞれ期待する結果を先に書いておく（2026-09-23 の設計ディスカッションで確定。着手前の実測は下の「今日の当たり」）。
  - ⒜ **末尾 `/` の無い綴り・ソース全域**: `grep -rn "sample_ghost/StayseeBalloon" crates/` → **0 件**。要件 4.1（テストの綴り 0 か所・コメント込み）と 5.5（`fold-samples.rs` の doc の例）の直接の判定。定数 `STAYSEE_BALLOON_DIR` の値は `/` で終わらないので、見張り（`/` 付きの語形）にも ⒝ にも掛からない＝削除漏れはこの ⒜ だけが拾う。今日の当たり: 入口 `staysee_balloon_fixture_test.rs` の module doc 1 行と定数 1 行・`fold-samples.rs` の module doc 2 行の計 4 件。
  - ⒝ **末尾 `/` 付きの綴り・文書**: `grep -rn "vendors/sample_ghost/StayseeBalloon/" .kiro/steering .kiro/specs/*/brief.md vendors/sample_ghost/README.md` → 当たってよいのは **`roadmap.md` の #37 の行と #42 の行、`roadmap-history.md` の 2 行の計 4 件だけ**で、それ以外は **0 件**。4 件はいずれも要件 5.6 が除外する「経緯の記録」である。台帳の完了した行は本文を残して状態列を ✅ にする慣例（`roadmap.md` の台帳の注記「状態列は `/kiro-complete` が ✅ に更新」・#37 の行がその実例）なので、#42 の行の本文「展開フォルダのまま残っている」は書き換えず、状態列が ✅ になった時点で経緯の記録になる。今日の当たり: 上の 4 件に加えて `product.md`・`structure.md`・brief 2 本（`alpha-release-signoff` は 2 行）・入口ファイルの module doc の計 6 件（すべて本設計の Modified Files で直す）。
  - `.kiro/specs/completed/` と本 spec の自身の文書は要件どおり検索に含めない。

## System Flows

### 作業順（1 コミットに畳む前提・1.5）

途中状態はどれも赤になるので、次の順で作業し、①〜⑥ を **1 つのコミット**に入れる（文書 ⑥ を同じコミットに含めるのが最短。分けるなら ⑥ だけ後のコミットにしてよい＝コードと資産が分かれなければ途中状態にならない）。

1. **畳む**: `cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/StayseeBalloon` → `不一致 0 件`／`全ての検体で一致`／終了コード 0 を確認。
2. **登記**: `SAMPLES` に 1 行。**照合**: `nar-sample-path -- StayseeBalloon` の `folder=` で 29 本に `sha256sum` を当て、provenance §3.1 と突き合わせて `verification/nar-roundtrip.md` に記録（1.4・2.4）。
3. **自己テストの追随**: `lib_tests.rs` の表どおり（3.1〜3.5）。**付け替え**: `test_support.rs` の本体差し替え・入口の定数と doc の削除（4.1〜4.3）。
4. **追跡から外す**: `git rm -r --cached vendors/sample_ghost/StayseeBalloon` → 実体のフォルダを消す（1.2。`.gitignore` は当たらないので、消さなければ未追跡として `git status` に残る）。
5. **判定**: `cargo test -p sample-ghost-kit`／`cargo test -p areka-emo-text --test staysee_balloon_fixture_test`／`cargo test -p log-capture-kit --test sample_path_guard_test` の 3 本が全件緑（3.6・4.4・4.5）。
6. **文書**: README・`product.md`・`structure.md`・brief 2 本・`fold-samples.rs` の doc・roadmap #42（5.1〜5.5）。
7. **検索**: 「文書の追随」の 2 本——⒜ `crates/` の `/` 無しの綴りが 0 件、⒝ 文書の `/` 付きの綴りが `roadmap.md` #37・#42 と `roadmap-history.md` の 4 件以外 0 件。

順序の要点: ①は展開フォルダが**在るうち**にしか出来ない。②の照合は登記後でなければ `nar-sample-path` が断る。④は⑤の前——フォルダが残っていると付け替え漏れが赤にならない。

## Error Handling

- `fold-samples` は不一致があれば終了コード 0 にならない（既存）。0 以外なら畳み直さず原因を見る（`git ls-files` が 0 件＝パスの綴り違いが典型）。
- `staysee_root()` は登記漏れ・`.nar` 欠落を `expect` で名指しの panic にする（黙って存在しないパスを配らない。窓口の `SampleError` の表示に既知の名前 6 つが載る）。
- 照合（1.4）で 1 本でも不一致なら `.nar` を追跡せず、`fold-samples` の印字と `git check-attr` から原因を切り分ける（改行変換なら `.gitattributes` の効き、ファイル欠落なら追跡の漏れ）。
- 監視・ログの追加: 0（テストと一度きりの手順だけで、実行時コードに触らない）。

## Testing Strategy

新しいテストは 0 本。既存のテストの数と名前を追随させ、既存の判定をそのまま使う。

- **登記の往復**（`every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`）: `StayseeBalloon.nar` を空の根へ展開すると要素が `Balloon balloon/StayseeBalloon` の 1 つちょうど＝2.1 の種別・同梱 0 と 1.3 の置き場を判定。母数は逐語の 6。
- **位置と破棄**（`every_registered_sample_lands_where_its_registry_row_says`）: `folder()` が `<根>/balloon/StayseeBalloon` で実在し、捨てると根が消える＝2.2・2.3。
- **未登録名**（`unknown_sample_fails_with_all_six_known_names`）: 既知の名前 6 つに `StayseeBalloon` が載る＝2.5。
- **種別の登記**（`registry_records_kind_and_bundled_balloons`）: `StayseeBalloon` が `Balloon`・同梱 0＝2.1（決定 A）。
- **既定バルーンの結合テスト**（`staysee_balloon_fixture_test` の全テーマ）: `stored_folder_holds_exactly_the_upstream_29_files` が窓口の `folder()` 直下に 29 本ちょうど（`install.txt` 込み）＝1.3・4.2・4.4。他のテーマの期待値は 0 変更で、根の取り方だけが変わったことの証拠になる。
- **見張り**（`no_sample_path_spelling_lives_outside_the_gateway`）: 6 体の名前で組んだ走査語が窓口の外に 0 件＝4.5。
- **手順の判定**（テストではない）: `fold-samples` の `不一致 0 件`（1.3）・`sha256sum` 29 本一致（1.4）・`git ls-files vendors/sample_ghost/StayseeBalloon` が 0 行（1.2）・5.6 の `grep` が 0 件。
- 常設しないもの（明示）: 29 本の sha256 の常設判定（裁定）・見張りの語形の追加（境界の外）。

## 設計判断のまとめ（採った案と採らなかった案）

| 項目 | 採った | 採らなかった（理由） |
|---|---|---|
| 根の取り方 | 案 A: `test_support.rs` の本体を `static LazyLock<SampleRoot>` に差し替え（呼び手 0 変更） | 案 B: crate 横断の共有ヘルパ（利用者 2 crate・各 5 行の重複を消すために crate を足す利得が無い）／案 C: 見張りの語形追加（境界の外・`log-capture-kit` に触る） |
| 決定 A: 種別テスト | `registry_records_kind_and_bundled_balloons` のループに 1 名追加 | 足さない（2.1 の直接の判定を 1 語で得られるのに手放す理由が無い）／新しいテスト関数（既存のループで足りる） |
| 決定 B: README の出どころ | 表の 1 行＋箇条書き 1 行（ポインタ） | `konnoyayame` と同じ節（禁止事項が 0・ハッシュ表の複製は黙って古びる） |
| 中身の固定 | 一度きりの照合＋記録（1.4） | sha256 の常設判定（2026-09-23 裁定＝しない） |
| コミットの粒度 | コードと資産は 1 コミット | 畳む→登記→付け替えを別コミット（途中状態が赤） |

## Supporting References

- research.md §2（既存資産の対応表）・§3.1（見張りの語形）・§3.7（1.4 が一度きりである根拠）・§4（案 A/B/C）・§8（要件ディスカッションの行き先と裁定）。
- 完了 spec `areka-P0-nar-install`（窓口と畳む道具・要件 1.5「2 手で終わる」）／`areka-P0-default-balloon-bundle`（`verification/provenance.md` §1 資産の素性・§3.1 の 29 本の sha256）。
- `vendors/sample_ghost/README.md`「検体を 1 本足す手順」（本設計の作業順はこの手順 1〜5 に 1.4 の照合と結合テストの付け替えを挟んだもの）。
