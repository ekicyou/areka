# Requirements Document

## Project Description (Input)

困っているのは、areka の開発者（完了の全体テストと `/release` の手元の確かめを回す人）と、crates.io の `dola` を使う外の利用者である。

今の状況: `dola` の任意の機能 `yaml` が、非推奨の `serde_yaml` を通して `unsafe-libyaml` を引く。`unsafe-libyaml` は RustSec の勧告 `RUSTSEC-2024-0449`（保守されていない・作者がリポジトリを閉じた・直した版は出ない）に載っており、2026-10-15 の全体テストで、ライセンスの段の `cargo deny check` が初めて赤になった。今は `deny.toml` の `[advisories] ignore` にこの勧告 1 件を、理由とこの spec の名前を添えて入れて通している（仮の物）。ワークスペースの中で機能 `yaml` を有効にするクレートは 0 本で、areka の配布物には入らないが、`dola` は crates.io に公開しているので、この機能を有効にした外の利用者はそのまま引き継ぐ。2026-11-13 に `audit-dola` が機能 `json`・`toml` と既定の機能を消し（`main` にだけあり、まだ公開していない）、機能 `yaml` と任意の `serde_yaml` だけがこの spec の持ち物として残っている。

変えたい姿: `dola` の依存の木から `unsafe-libyaml` が消え、`deny.toml` の見逃しの指定を消しても `cargo deny check` が緑になる。`dola` の書式の機能は TOML の 1 つになり（機能 `yaml` は消え、機能 `toml` が戻る）、README・テスト・steering・crates.io の利用者への申し送りがそれに合っている。

## Introduction

この spec は、crates.io に公開しているクレート `dola` の任意の機能 `yaml` を任意の依存 `serde_yaml` ごと外して、保守されていない `unsafe-libyaml` を依存の木から無くし、依存の監査の仮の見逃し（`deny.toml` の `RUSTSEC-2024-0449`）を消すためのものである。あわせて、`dola` の書式の機能として TOML の機能 `toml` を入れる（ワークスペースの `toml` 1 を使う）。

## 討議の結論（2026-11-13）

1. **機能 `yaml` を外し、機能 `toml` を入れる**。開発者「残すならtoml、yamlを外してtomlを入れよ」。YAML を替えて残す案（`serde-saphyr` など）は採らない。`audit-dola` が機能 `toml` を外したのは開発者が決めたことではない（開発者「tomlを外せなどという判断はした覚えがない」）＝この spec で戻す。機能 `json` は `audit-dola` のまま（任意の機能に戻さない・開発専用の依存のまま）。
2. 「替える案のとき YAML の往復のテストを全体テストで常に回すか」は、YAML を外すので消えた。TOML の往復のテストは議題 4 で決める。
3. **機能 `toml` の下で `dola` が TOML の読み書きの入口を持つ**（議題 3・要件 2.2・開発者「承認」）。v0.1.0 までの機能 `toml` は依存を引くだけで `dola` に何も足さず、利用者は自分でも `toml` を依存に書く必要があった。入口があれば `features = ["toml"]` だけで TOML が使え、`toml` の版の食い違いでも迷わない。足すだけで既存の形は変えない（名前と誤りの型は設計で決める）。
4. **TOML のテストは全体テストで常に回す**（議題 4・要件 2.6・開発者「承認」）。全体テストは任意の機能を有効にしないので、前の TOML と YAML の往復のテストは一度も回っていなかった＝壊れても赤にならない。回し方（全体テストの手順に `-p dola --features toml` を足すか、ほかの道か）は設計で決める。

## Boundary Context

- **In scope**:
  - `crates/dola/Cargo.toml` の任意の依存 `serde_yaml` と機能 `yaml` の削除、任意の依存 `toml` と機能 `toml` の追加。
  - 機能 `toml` の下で `dola` が持つ TOML の読み書きの入口（議題 3）。
  - 往復のテスト（`crates/dola/tests/general/integration_test/serialization_round_trip.rs`）の YAML の分の削除と TOML の分の追加、同じテストの束の先頭の説明（`crates/dola/tests/general/integration_test/mod.rs` 1 行目の「JSON/TOML/YAML」）。
  - `crates/dola/tests/compile/boundary_test.rs` の 3 行目の先頭の説明（「外部入力（JSON/TOML/YAML）」）と 260〜262 行目の注記（NaN／inf の入り口）、TOML の `nan`／`inf` の扱いを確かめる境界のテスト。
  - TOML のテストを全体テストで常に回すための道（議題 4・`tools/test-all.ps1` などの全体テストの手順）。
  - `crates/dola/README.md` の書式の記述（13・17 行目）。
  - `deny.toml` の `[advisories] ignore` の `RUSTSEC-2024-0449` の 1 件（43〜46 行目のコメントを含む）の削除。
  - 根の `Cargo.lock` と `THIRD-PARTY-NOTICES.md` の追随。
  - steering の `dola` の記述（`tech.md` の「dola クレート依存」・`product.md` の dola の行・`structure.md` の dola の Dependencies の行）。
  - roadmap の「次のリリースへの申し送り」の `dola` の公開の形の項目（`audit-dola` の機能 `json` の削除の分を含めて 1 項目）。
- **Out of scope**:
  - JSON の直列化の振る舞い・機能 `json` を戻すこと・`dola` の型の形・`dola` のほかのモジュール。
  - YAML の読み書きを別のクレートで残すこと。
  - `crates/dola/src` のコメントが外からの入力の書式の例として挙げる「JSON/TOML/YAML」（`compile/mod.rs` 72 行目・`value.rs` 7・8 行目。利用者の側の書式の例で、`dola` の機能を指していない）。
  - ほかのクレート（根の `Cargo.lock`・`THIRD-PARTY-NOTICES.md`・steering・roadmap・全体テストの手順を除く）。
  - `deny.toml` のほかの行。`[graph] all-features = true` を外す、機能を除外する、のような監査の範囲を狭める直しはしない。
  - `dola` の公開の手順（スキル `/release` と `doc/crates-io-publish.md`）と、版の番号の上げ方（`/release` が決める）。
- **Adjacent expectations**:
  - 同じ `crates/dola/Cargo.toml` を触る `dola-interrupt-ticks` とは同じウェーブに置かない（棚卸㊱）。ウェーブ C19 のほかの 7 本とは `crates/dola/`・`deny.toml` の重なりが 0（`tech.md` などの steering は許す重なり）。全体テストの手順に触る場合は、C19 で同じファイルに触る spec が無いことを設計の前に確かめる。
  - この spec は C19 の「1 ウェーブに 1 本だけの席」（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`）を使う。`Cargo.lock` から `serde_yaml`・`unsafe-libyaml`・`ryu` が消え、`dola` の依存に `toml`（ワークスペースに既にある 1 系）が加わる。
  - 次の `/release` は「次のリリースへの申し送り」を段 1 で読む。この spec はその項目を書くだけで、見どころの本文は書かない。

## Requirements

### Requirement 1: 依存の監査を見逃しなしで緑にする

**Objective:** As a areka の開発者, I want 依存の監査が仮の見逃しなしで通ってほしい, so that 完了の全体テストと `/release` の手元の確かめが、同じ勧告で止まらず、見逃しの指定も残らない

#### Acceptance Criteria

1. The `dola` クレート shall どの機能を有効にしても、依存の木に `unsafe-libyaml` と `serde_yaml` を含まない
2. When この spec の直しが済んだとき, the `deny.toml` shall `[advisories] ignore` に `RUSTSEC-2024-0449` の項目とその説明のコメントを含まない
3. When `deny.toml` の見逃しを消した状態で `cargo deny check` を回すとき, the 依存の監査 shall 終了コード 0 で終わる
4. The この spec の直し shall `deny.toml` の `[graph] all-features = true` を含むほかの行を変えず、新しい見逃しの指定（`ignore`・機能の除外・クレートの除外）を足さない
5. If `cargo deny check` がこの spec の直しの後に別の勧告で赤になったとき, then the 本 spec の作業 shall その勧告を見逃しの指定で黙らせず、勧告の番号と引いているクレートを報告に残して開発者の判断を仰ぐ

### Requirement 2: 機能 `yaml` を外し、機能 `toml` を入れる

**Objective:** As a crates.io の `dola` の利用者, I want `dola` の書式の機能が TOML の 1 つにはっきり決まっていてほしい, so that TOML で定義を書くなら機能 `toml` を、ほかの書式なら自分で選んだクレートを使えばよいと迷わず分かる

#### Acceptance Criteria

1. The `dola` クレート shall 機能 `yaml` と任意の YAML の依存を持たず、任意の機能は `toml` の 1 つだけを持つ（既定の機能は無いまま）
2. Where 機能 `toml` を有効にしたとき, the `dola` クレート shall `DolaDocument` を TOML の文字列から読む入口と TOML の文字列へ書く入口を、利用者が自分で `toml` を依存に書かずに使える形で提供する（名前と誤りの型は設計で決める・任意の依存 `toml` はワークスペースの版の指定に従う）
3. If TOML の読み書きの入口が失敗したとき, then the `dola` クレート shall panic せず、失敗の理由を持つ誤りを返す
4. When 機能 `toml` を有効にして `DolaDocument` 全部を 2 の入口で TOML に書き出して読み戻すとき, the 読み戻した文書 shall 元の文書と等しい
5. When 同じ `DolaDocument` を 2 回 TOML に書き出すとき, the 2 つの文字列 shall 1 バイトも違わない（`BTreeMap` の鍵の順が決まっている）
6. When 全体テスト（`tools/test-all.ps1`）を回すとき, the 機能 `toml` のテスト（往復のテストと要件 4 の境界のテスト） shall 毎回回り、その結果が全体テストの結果に入る
7. The `dola` のテスト shall YAML の往復のテストとその見出しのコメントを含まない
8. The `dola` のテストの先頭の説明（`serialization_round_trip.rs` の 1 行目・`integration_test/mod.rs` 1 行目・`boundary_test.rs` 3 行目） shall 消えた YAML を挙げず、直しの後に `dola` が持つ書式の扱い（JSON は開発専用・TOML は機能 `toml`）だけを書く

### Requirement 3: JSON の振る舞いと `dola` の型の形を変えない

**Objective:** As a `dola` を使う areka のコードと外の利用者, I want 書式の機能の直しが JSON と型の形に響かないでほしい, so that 機能 `toml` を使わない利用者は何も直さずに済む

#### Acceptance Criteria

1. The この spec の直し shall `dola` の公開の型・関数・トレイトの既存の形を変えない（機能 `toml` の下の入口〔要件 2.2〕は足すだけにする）
2. When この spec の直しの後に `dola` のテストを回すとき, the JSON の往復のテスト shall 直しの前と同じ本数が緑になる
3. The この spec の直し shall `crates/dola/` の外のソースファイル（根の `Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`deny.toml`・steering・roadmap・要件 2.6 のための全体テストの手順〔`tools/test-all.ps1` と、その段の一覧を書いた文書＝`tech.md` のフルテストの節・`.claude/skills/kiro-complete/SKILL.md` の段の一覧〕を除く）を変えない

### Requirement 4: NaN と無限大の入り口の説明と確かめを合わせる

**Objective:** As a `dola` の保守をする開発者, I want NaN と無限大がどこから入りうるかの説明が今の形と合っていてほしい, so that 数値の規則（validate）が守る範囲を読み違えない

#### Acceptance Criteria

1. The `crates/dola/tests/compile/boundary_test.rs` の注記 shall 「NaN/inf の流入経路は YAML のみ」「機能 `yaml` により既定ビルド外」という古い書き方を残さず、TOML（`nan`・`inf`・`-inf` を書ける）が入り口になりうることを書く
2. When 機能 `toml` を有効にして、TOML の `nan`・`inf`・`-inf` を時刻と補間に使う数の欄に含む定義を読ませたとき, the 境界のテスト shall 読み込みで断られるか、読めた値が validate の数値の規則で拒まれるかのどちらかになり、panic しないことを確かめる
3. The 境界のテスト shall 2 の確かめを、時刻と補間に使う数の欄のうち型の決まった `f64` の欄と、`#[serde(untagged)]` の数の欄（遷移の `to` の数・`loop_offset`・`at` の `offset`・ベジェの点）のそれぞれで行う
4. When 機能 `toml` を有効にして、Object の値（`DynamicValue`・補間せずに運ぶ中身）に `nan`・`inf` を含む定義を読ませたとき, the 境界のテスト shall validate と compile が panic せずに終わることを確かめる（Object の値の中の数は数値の規則の外で、読めて validate も通る＝設計の討議 議題 1・2026-11-13 開発者「承認」）
5. The `crates/dola/tests/compile/boundary_test.rs` の注記 shall Object の値の中の数が数値の規則の外であることを書く

### Requirement 5: 説明と生成物を直しの後の形に合わせる

**Objective:** As a `dola` の README と steering を読む人, I want 書いてある書式の扱いが実物と合っていてほしい, so that 無い機能や古い依存を信じて使わずに済む

#### Acceptance Criteria

1. The `crates/dola/README.md` shall 機能 `yaml` を挙げず、機能 `toml` で TOML に乗ること（ほかの書式は serde の導出で利用者が選べること）を書く
2. The `.kiro/steering/tech.md` の「dola クレート依存」 shall `serde_yaml` の行を含まず、`toml` の行（版・機能の名前・使い道）を含む
3. The `.kiro/steering/product.md` の dola の行と `.kiro/steering/structure.md` の dola の Dependencies の行 shall 消えた機能 `json`・`yaml` を挙げず、書式の機能を `toml` だけと書く
4. The 根の `Cargo.lock` shall `serde_yaml` と `unsafe-libyaml` の項目を含まない
5. The `THIRD-PARTY-NOTICES.md` shall 生成の道具（`cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`）で作り直した物と 1 バイトも違わない（手で直さない）
6. The `crates/dola/Cargo.toml` shall docs.rs が機能 `toml` を有効にして説明書を作る指定（`[package.metadata.docs.rs]` の `features = ["toml"]`）を持ち、TOML の入口が docs.rs に出る（設計の討議 議題 2・2026-11-13 開発者「承認」）

### Requirement 6: crates.io の利用者への変わる点を 1 項目で申し送る

**Objective:** As a 次の `/release` を回す開発者, I want `dola` の公開の形が変わる点を 1 か所で読みたい, so that 見どころに載せ漏れず、`audit-dola` の分と別々に探さずに済む

#### Acceptance Criteria

1. The roadmap の「次のリリースへの申し送り」 shall `dola` の公開の機能の変わる点を、公開済みの最新の版と比べた正味の差として 1 項目にまとめて持つ（2026-11-13 の棚卸㊱が足した `audit-dola` の項目を書き直す形でよい。同じ話の項目を 2 つ並べない）
2. The その項目 shall 機能 `json`（既定で有効だった）と既定の機能が無くなったことを、壊れる変更として書く
3. The その項目 shall 機能 `yaml` と任意の依存 `serde_yaml` が消えたこと（壊れる変更・`Cargo.toml` の機能の指定を消せば直る）を書く
4. The その項目 shall 機能 `toml` は続けて指定できること、引く `toml` の版が 0.8 から 1 に上がったこと、機能 `toml` の下で `dola` が TOML の読み書きの入口を持つようになったこと（利用者が自分で `toml` を依存に書かなくてよい）を書く
5. The その項目 shall 「次のリリースへの申し送り」の頭の決まり（書いたときの最新の版を「`v{その版}` の後」の形で頭に書く）に従う
6. The その項目 shall 「機能 `yaml` の行き先は `dola-yaml-backend` が決める」のような未決の書き方を含まず、決まった結果だけを書く

### Requirement 7: 完了の前に全体を確かめる

**Objective:** As a areka の開発者, I want 直しが全体テストとライセンスの段を通ったことを確かめてから完了したい, so that 依存の木を動かした直しで、ほかのクレートが壊れていないと言える

#### Acceptance Criteria

1. When 完了の全体テストをライセンスの段込みで回すとき, the 全体テスト shall `cargo deny check` と `cargo about generate` の段（作り直した `THIRD-PARTY-NOTICES.md` に差分が無いことの判定を含む）を含めて全部緑になる
2. If 全体テストでこの spec が触っていないクレートだけが赤になったとき, then the 完了の手順 shall 止めて報告し、赤のクレートだけをあと 1 回回して、変化が無ければ申し送りを残して先へ進む
