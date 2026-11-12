# Requirements Document

## Project Description (Input)

困っているのは、areka の開発者（完了の全体テストと `/release` の手元の確かめを回す人）と、crates.io の `dola` を使う外の利用者である。

今の状況: `dola` の任意の機能 `yaml` が、非推奨の `serde_yaml` を通して `unsafe-libyaml` を引く。`unsafe-libyaml` は RustSec の勧告 `RUSTSEC-2024-0449`（保守されていない・作者がリポジトリを閉じた・直した版は出ない）に載っており、2026-10-15 の全体テストで、ライセンスの段の `cargo deny check` が初めて赤になった。今は `deny.toml` の `[advisories] ignore` にこの勧告 1 件を、理由とこの spec の名前を添えて入れて通している（仮の物）。ワークスペースの中で機能 `yaml` を有効にするクレートは 0 本で、areka の配布物には入らないが、`dola` は crates.io に公開しているので、この機能を有効にした外の利用者はそのまま引き継ぐ。2026-11-13 に `audit-dola` が機能 `json`・`toml` と既定の機能を消し、機能 `yaml` と任意の `serde_yaml` だけがこの spec の持ち物として残っている。

変えたい姿: `dola` の依存の木から `unsafe-libyaml` が消え、`deny.toml` の見逃しの指定を消しても `cargo deny check` が緑になる。`dola` は機能 `yaml` を持たず（書式の機能を 1 つも持たない）、README・テスト・steering・crates.io の利用者への申し送りがそれに合っている。

## Introduction

この spec は、crates.io に公開しているクレート `dola` の任意の機能 `yaml` を、任意の依存 `serde_yaml` ごと外し、保守されていない `unsafe-libyaml` を依存の木から無くして、依存の監査の仮の見逃し（`deny.toml` の `RUSTSEC-2024-0449`）を消すためのものである。

## 討議の結論（2026-11-13）

1. **機能 `yaml` は外す**（替える案は採らない）。開発者「yamlってarekaでは使ってないよね」「残すならtoml」。理由: ⑴ areka の中で `dola` の定義をファイルから読む所は無い（使い手はどれも `DolaDocumentBuilder` などで Rust のコードの上で組む）。機能 `yaml` は依存を 1 つ増やすだけで、`dola` の公開の形に何も足していない（`crates/dola/src` に `serde_yaml` の語も `cfg(feature = "yaml")` も再輸出も無い） ⑵ 同じクレートで `audit-dola` が機能 `json`・`toml` を同じ理由で外した先例があり、外せば「`dola` は書式の機能を持たない」で揃う ⑶ 書式の機能を将来持つとしても YAML ではなく TOML を選ぶ（開発者の方針）＝YAML の読み書きを替えて残す理由が無い。今は TOML の使い手も居ないので、この spec では TOML の機能を戻さない（要るようになったら別に起票する）。払うもの: crates.io の利用者のうち `Cargo.toml` で機能 `yaml` を有効にしていた人は、その指定を消さないとビルドが通らない（壊れる変更・直すのはその 1 行・要件 6 で申し送る）。
2. 「替える案のとき YAML の往復のテストを全体テストで常に回すか」は、外す案に決まったので消えた。

## Boundary Context

- **In scope**:
  - `crates/dola/Cargo.toml` の任意の依存 `serde_yaml` と機能 `yaml`。
  - YAML の往復のテスト（`crates/dola/tests/general/integration_test/serialization_round_trip.rs` の 1・40〜55 行目）と、同じテストの束の先頭の説明（`crates/dola/tests/general/integration_test/mod.rs` 1 行目の「JSON/TOML/YAML」）。
  - `crates/dola/tests/compile/boundary_test.rs` の 3 行目の先頭の説明（「外部入力（JSON/TOML/YAML）」）と 260〜262 行目の注記（NaN／inf の入り口）。
  - `crates/dola/README.md` の YAML の記述（13・17 行目）。
  - `deny.toml` の `[advisories] ignore` の `RUSTSEC-2024-0449` の 1 件（43〜46 行目のコメントを含む）の削除。
  - 根の `Cargo.lock` と `THIRD-PARTY-NOTICES.md` の追随。
  - steering の `dola` の記述（`tech.md` の「dola クレート依存」・`product.md` の dola の行・`structure.md` の dola の Dependencies の行）。
  - roadmap の「次のリリースへの申し送り」の `dola` の公開の形の項目（`audit-dola` の機能 `json`・`toml` の削除の分を含めて 1 項目）。
- **Out of scope**:
  - JSON の直列化の振る舞い・`dola` の型の形・`dola` のほかのモジュール。
  - 書式の機能（TOML ほか）を新しく足すこと（要るようになったら別に起票する）。
  - `crates/dola/src` のコメントが外からの入力の書式の例として挙げる「JSON/TOML/YAML」（`compile/mod.rs` 72 行目・`value.rs` 7・8 行目。利用者の側の書式の例で、`dola` の機能を指していない）。
  - ほかのクレート（根の `Cargo.lock`・`THIRD-PARTY-NOTICES.md`・steering・roadmap を除く）。
  - `deny.toml` のほかの行。`[graph] all-features = true` を外す、機能を除外する、のような監査の範囲を狭める直しはしない。
  - `dola` の公開の手順（スキル `/release` と `doc/crates-io-publish.md`）と、版の番号の上げ方（`/release` が決める）。
- **Adjacent expectations**:
  - 同じ `crates/dola/Cargo.toml` を触る `dola-interrupt-ticks` とは同じウェーブに置かない（棚卸㊱）。ウェーブ C19 のほかの 7 本とは触るファイルの重なりが 0（`tech.md` などの steering は許す重なり）。
  - この spec は C19 の「1 ウェーブに 1 本だけの席」（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`）を使う。`Cargo.lock` から `serde_yaml`・`unsafe-libyaml`・`ryu` が消える。
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

### Requirement 2: 機能 `yaml` を外す

**Objective:** As a crates.io の `dola` の利用者, I want `dola` が書式の機能を持たないことがはっきりしていてほしい, so that 自分で書式のクレートを選んで `dola` の型を読み書きすればよいと迷わず分かる

#### Acceptance Criteria

1. The `dola` クレート shall 機能 `yaml` と任意の YAML の依存を持たず、任意の機能を 1 つも持たない
2. The `dola` のテスト shall YAML の往復のテストとその見出しのコメントを含まず、`serialization_round_trip.rs` と `integration_test/mod.rs` の先頭の説明も JSON だけを指す
3. The `dola` のテストの先頭の説明（`integration_test/mod.rs` 1 行目・`boundary_test.rs` 3 行目） shall `audit-dola` で消えた TOML と、この spec で消える YAML を、`dola` が持つ書式の機能として挙げない

### Requirement 3: JSON の振る舞いと `dola` の型の形を変えない

**Objective:** As a `dola` を使う areka のコードと外の利用者, I want YAML の直しが JSON と型の形に響かないでほしい, so that YAML を使わない利用者は何も直さずに済む

#### Acceptance Criteria

1. The この spec の直し shall `dola` の公開の型・関数・トレイトの形を変えない
2. When この spec の直しの後に `dola` のテストを回すとき, the JSON の往復のテスト shall 直しの前と同じ本数が緑になる
3. The この spec の直し shall `crates/dola/` の外のソースファイル（根の `Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`deny.toml`・steering・roadmap を除く）を変えない

### Requirement 4: NaN と無限大の入り口の説明を合わせる

**Objective:** As a `dola` の保守をする開発者, I want NaN と無限大がどこから入りうるかの説明が今の形と合っていてほしい, so that 数値の規則（validate）が守る範囲を読み違えない

#### Acceptance Criteria

1. The `crates/dola/tests/compile/boundary_test.rs` の注記 shall 機能 `yaml` が既定のビルドの外にあるという古い書き方を残さない
2. The `crates/dola/tests/compile/boundary_test.rs` の注記 shall `dola` が YAML の読み込みを持たないこと、利用者が自分で選んだ書式から来た NaN／inf は validate の数値の規則が拒むことを書く

### Requirement 5: 説明と生成物を直しの後の形に合わせる

**Objective:** As a `dola` の README と steering を読む人, I want 書いてある書式の扱いが実物と合っていてほしい, so that 無い機能や古い依存を信じて使わずに済む

#### Acceptance Criteria

1. The `crates/dola/README.md` shall 機能 `yaml` を挙げず、`dola` が書式の機能を持たず serde の導出で利用者が書式を選べることを書く
2. The `.kiro/steering/tech.md` の「dola クレート依存」 shall `serde_yaml` の行を含まない
3. The `.kiro/steering/product.md` の dola の行と `.kiro/steering/structure.md` の dola の Dependencies の行 shall 消えた機能 `json`・`toml`・`yaml` を挙げない
4. The 根の `Cargo.lock` shall `serde_yaml` と `unsafe-libyaml` の項目を含まない
5. The `THIRD-PARTY-NOTICES.md` shall 生成の道具（`cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`）で作り直した物と 1 バイトも違わない（手で直さない）

### Requirement 6: crates.io の利用者への変わる点を 1 項目で申し送る

**Objective:** As a 次の `/release` を回す開発者, I want `dola` の公開の形が変わる点を 1 か所で読みたい, so that 見どころに載せ漏れず、`audit-dola` の分と別々に探さずに済む

#### Acceptance Criteria

1. The roadmap の「次のリリースへの申し送り」 shall `dola` の公開の機能の変わる点を 1 項目にまとめて持つ（2026-11-13 の棚卸㊱が足した `audit-dola` の項目を、この spec の結果を足して書き直す形でよい。同じ話の項目を 2 つ並べない）
2. The その項目 shall `audit-dola` が消した機能 `json`（既定で有効だった）・`toml` と任意の依存 `toml`、既定の機能が無くなったことを、壊れる変更として書く
3. The その項目 shall 機能 `yaml` と任意の依存 `serde_yaml` も消えたこと（壊れる変更・`Cargo.toml` の機能の指定を消せば直る）と、利用者が自分で serde の書式のクレートを選べば同じ型を読み書きできることを書く
4. The その項目 shall 「次のリリースへの申し送り」の頭の決まり（書いたときの最新の版を「`v{その版}` の後」の形で頭に書く）に従う
5. The その項目 shall 「機能 `yaml` の行き先は `dola-yaml-backend` が決める」のような未決の書き方を含まず、決まった結果だけを書く

### Requirement 7: 完了の前に全体を確かめる

**Objective:** As a areka の開発者, I want 直しが全体テストとライセンスの段を通ったことを確かめてから完了したい, so that 依存の木を動かした直しで、ほかのクレートが壊れていないと言える

#### Acceptance Criteria

1. When 完了の全体テストをライセンスの段込みで回すとき, the 全体テスト shall `cargo deny check` と `cargo about generate` の段（作り直した `THIRD-PARTY-NOTICES.md` に差分が無いことの判定を含む）を含めて全部緑になる
2. If 全体テストでこの spec が触っていないクレートだけが赤になったとき, then the 完了の手順 shall 止めて報告し、赤のクレートだけをあと 1 回回して、変化が無ければ申し送りを残して先へ進む
