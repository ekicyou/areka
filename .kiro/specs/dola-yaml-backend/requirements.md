# Requirements Document

## Project Description (Input)

困っているのは、areka の開発者（完了の全体テストと `/release` の手元の確かめを回す人）と、crates.io の `dola` を使う外の利用者である。

今の状況: `dola` の任意の機能 `yaml` が、非推奨の `serde_yaml` を通して `unsafe-libyaml` を引く。`unsafe-libyaml` は RustSec の勧告 `RUSTSEC-2024-0449`（保守されていない・作者がリポジトリを閉じた・直した版は出ない）に載っており、2026-10-15 の全体テストで、ライセンスの段の `cargo deny check` が初めて赤になった。今は `deny.toml` の `[advisories] ignore` にこの勧告 1 件を、理由とこの spec の名前を添えて入れて通している（仮の物）。ワークスペースの中で機能 `yaml` を有効にするクレートは 0 本で、areka の配布物には入らないが、`dola` は crates.io に公開しているので、この機能を有効にした外の利用者はそのまま引き継ぐ。2026-11-13 に `audit-dola` が機能 `json`・`toml` と既定の機能を消し、機能 `yaml` と任意の `serde_yaml` だけがこの spec の持ち物として残っている。

変えたい姿: `dola` の依存の木から `unsafe-libyaml` が消え、`deny.toml` の見逃しの指定を消しても `cargo deny check` が緑になる。`dola` の YAML の扱い（安全な読み書きへ替えるか、機能ごと外すか）が決まり、README・テスト・steering・crates.io の利用者への申し送りがそれに合っている。

## Introduction

この spec は、crates.io に公開しているクレート `dola` の任意の機能 `yaml` が引く、保守されていない `unsafe-libyaml` を依存の木から無くし、依存の監査の仮の見逃し（`deny.toml` の `RUSTSEC-2024-0449`）を消すためのものである。

YAML の扱いには 2 つの案がある。**案 A＝替える**（機能 `yaml` の名前と外から見た形を保ったまま、YAML の読み書きを保守されている安全な物へ替える）と、**案 B＝外す**（機能 `yaml` ごと外す）。brief はどちらを推すかを決めていない。この要件は、どちらの案でも同じになる結果と、案ごとに違う結果（要件 2・4・5・6 の中の `Where 案 A`／`Where 案 B` の項）を分けて書く。**どちらを採るかは要件の討議で決め、採らなかった案の項を消す**（下の「討議で決めること」）。

## 討議で決めること

1. **案 A（替える）か案 B（外す）か**（要件 2・要件 4・要件 6）。推奨は **案 B＝外す**。理由: ⑴ 機能 `yaml` は `dola` の公開の形に何も足していない（2026-11-13 に `main` で測った: `crates/dola/src` に `serde_yaml` の語も `cfg(feature = "yaml")` も無く、再輸出も無い。`#[cfg(feature = "yaml")]` は往復のテストの 1 か所だけ）。機能を有効にしても依存が 1 つ増えるだけで、外の利用者が YAML を読み書きするには、もともと自分で YAML のクレートを依存に書いて呼ぶしかない＝替えても利用者が `dola` を通して使える物は増えない。areka の中の使い手も 0 本 ⑵ 同じクレートで `audit-dola` が機能 `json`・`toml` を同じ理由で外した先例があり、外せば「dola は書式の機能を持たない」で揃う ⑶ 依存を足さないので、新しい依存のライセンスと保守の状況の確かめ（設計の前に要る）と、`.nan`／`.inf` などの読み方の違いの確かめが要らない。案 B の代わりに払うもの: crates.io の利用者のうち `Cargo.toml` で機能 `yaml` を有効にしていた人は、その指定を消さないとビルドが通らない（壊れる変更・直すのはその 1 行）。`dola` の README は今も「早い段階の版で、本番に使うのは勧めない」と書いている。
2. **案 A を採る場合、YAML の往復のテストを全体テストで常に回すか**（要件 2.5）。今の全体テスト（`cargo test --workspace`）は任意の機能を有効にしないので、機能 `yaml` のテストは一度も回っていない。推奨は「完了の証跡で 1 回回して記録する・全体テストの段は足さない」。案 B なら消える議題。

## Boundary Context

- **In scope**:
  - `crates/dola/Cargo.toml` の YAML の依存と機能 `yaml`。
  - YAML の往復のテスト（`crates/dola/tests/general/integration_test/serialization_round_trip.rs` の 40〜55 行目）。
  - `crates/dola/tests/compile/boundary_test.rs` の 260〜262 行目の注記（NaN／inf の入り口）。案 A なら YAML の `.nan`／`.inf` の扱いを確かめる境界のテスト。
  - `crates/dola/README.md` の YAML の記述（13・17 行目）。
  - `deny.toml` の `[advisories] ignore` の `RUSTSEC-2024-0449` の 1 件（43〜46 行目のコメントを含む）の削除。
  - 根の `Cargo.lock` と `THIRD-PARTY-NOTICES.md` の追随。
  - steering の `dola` の記述（`tech.md` の「dola クレート依存」・`product.md` の dola の行・`structure.md` の dola の Dependencies の行）。
  - roadmap の「次のリリースへの申し送り」の `dola` の公開の形の項目（`audit-dola` の機能 `json`・`toml` の削除の分を含めて 1 項目）。
- **Out of scope**:
  - JSON の直列化の振る舞い・`dola` の型の形・`dola` のほかのモジュール。
  - `crates/dola/src` のコメントが外からの入力の書式の例として挙げる「JSON/TOML/YAML」（`compile/mod.rs` 72 行目・`value.rs` 7 行目。利用者の側の書式の例で、`dola` の機能を指していない）。
  - ほかのクレート（根の `Cargo.lock`・`THIRD-PARTY-NOTICES.md`・steering・roadmap を除く）。
  - `deny.toml` のほかの行。`[graph] all-features = true` を外す、機能を除外する、のような監査の範囲を狭める直しはしない。
  - `dola` の公開の手順（スキル `/release` と `doc/crates-io-publish.md`）と、版の番号の上げ方（`/release` が決める）。
- **Adjacent expectations**:
  - 同じ `crates/dola/Cargo.toml` を触る `dola-interrupt-ticks` とは同じウェーブに置かない（棚卸㊱）。ウェーブ C19 のほかの 7 本とは触るファイルの重なりが 0（`tech.md` などの steering は許す重なり）。
  - この spec は C19 の「1 ウェーブに 1 本だけの席」（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`）を使う。どちらの案でも `Cargo.lock` が動く。
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

### Requirement 2: `dola` の YAML の扱いを 1 つに決める（討議で案を選ぶ）

**Objective:** As a crates.io の `dola` の利用者, I want `dola` が YAML をどう扱うかが 1 つに決まっていてほしい, so that 機能 `yaml` を使うか、自分で書式のクレートを選ぶかを迷わず決められる

#### Acceptance Criteria

1. The `dola` クレート shall 案 A と案 B のうち要件の討議で選んだ 1 つだけの形になる（両方の形を同時に持たない）
2. Where 案 A（替える）を採るとき, the `dola` クレート shall 機能の名前 `yaml` を保ち、`Cargo.toml` で機能 `yaml` を有効にしている利用者が、その指定を書き直さずにビルドを通せる形を保つ
3. Where 案 A（替える）を採るとき, the `dola` クレート shall 機能 `yaml` の YAML の読み書きを、保守されていて安全な Rust だけで書かれた物で行う（選ぶ物は設計の段で決める・勧告が挙げる `serde-saphyr` は候補の 1 つ）
4. Where 案 A（替える）を採るとき, the 本 spec の作業 shall 選んだ依存（とそれが新しく引くクレート）のライセンスが `deny.toml` の `allow` に入っていることと、保守の状況（最後の公開の日・RustSec の勧告の有無）を、設計の段で確かめて設計の記録に残す
5. Where 案 A（替える）を採るとき, the YAML の往復のテスト shall 機能 `yaml` を有効にした実行で、`DolaDocument` 全部の書き出しと読み戻しが元と等しいことを確かめて緑になり、その実行の結果が完了の証跡に残る
6. Where 案 B（外す）を採るとき, the `dola` クレート shall 機能 `yaml` と任意の YAML の依存を持たず、任意の機能を 1 つも持たない
7. Where 案 B（外す）を採るとき, the `dola` のテスト shall YAML の往復のテストとその見出しのコメントを含まず、ファイルの先頭の説明も JSON だけを指す

### Requirement 3: JSON の振る舞いと `dola` の型の形を変えない

**Objective:** As a `dola` を使う areka のコードと外の利用者, I want YAML の直しが JSON と型の形に響かないでほしい, so that YAML を使わない利用者は何も直さずに済む

#### Acceptance Criteria

1. The この spec の直し shall `dola` の公開の型・関数・トレイトの形を変えない
2. When この spec の直しの後に `dola` のテストを回すとき, the JSON の往復のテスト shall 直しの前と同じ本数が緑になる
3. The この spec の直し shall `crates/dola/` の外のソースファイル（根の `Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`deny.toml`・steering・roadmap を除く）を変えない

### Requirement 4: NaN と無限大の入り口の説明と確かめを合わせる

**Objective:** As a `dola` の保守をする開発者, I want NaN と無限大がどこから入りうるかの説明が今の形と合っていてほしい, so that 数値の規則（validate）が守る範囲を読み違えない

#### Acceptance Criteria

1. The `crates/dola/tests/compile/boundary_test.rs` の注記 shall 直しの後の形での NaN／inf の入り口を正しく書く（機能 `yaml` が既定のビルドの外にあるという古い書き方を残さない）
2. Where 案 A（替える）を採るとき, the 境界のテスト shall 機能 `yaml` を有効にして、YAML の `.nan`・`.inf`・`-.inf` を含む定義を読ませたとき、読み込みで断られるか、読めた値が validate の数値の規則で拒まれるかのどちらかになることを確かめる
3. Where 案 A（替える）を採り、2 の結果が `serde_yaml` のときと違う振る舞いになったとき, the 境界のテスト shall その違いを、そのテストのコメントに書く
4. Where 案 B（外す）を採るとき, the `crates/dola/tests/compile/boundary_test.rs` の注記 shall `dola` が YAML の読み込みを持たないこと、利用者が自分で選んだ書式から来た NaN／inf は validate の数値の規則が拒むことを書く

### Requirement 5: 説明と生成物を直しの後の形に合わせる

**Objective:** As a `dola` の README と steering を読む人, I want 書いてある YAML の扱いが実物と合っていてほしい, so that 無い機能や古い依存を信じて使わずに済む

#### Acceptance Criteria

1. The `crates/dola/README.md` shall 直しの後の YAML の扱いと合う（案 A なら機能 `yaml` で YAML に乗ること、案 B なら書式の機能を持たず serde の導出で利用者が書式を選べること）
2. The `.kiro/steering/tech.md` の「dola クレート依存」 shall `serde_yaml` の行を含まず、案 A なら新しい YAML の依存の行（版・機能の名前・使い道）を含む
3. The `.kiro/steering/product.md` の dola の行と `.kiro/steering/structure.md` の dola の Dependencies の行 shall 消えた機能 `json`・`toml`（と案 B なら `yaml`）を挙げない
4. The 根の `Cargo.lock` shall `serde_yaml` と `unsafe-libyaml` の項目を含まない
5. The `THIRD-PARTY-NOTICES.md` shall 生成の道具（`cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`）で作り直した物と 1 バイトも違わない（手で直さない）

### Requirement 6: crates.io の利用者への変わる点を 1 項目で申し送る

**Objective:** As a 次の `/release` を回す開発者, I want `dola` の公開の形が変わる点を 1 か所で読みたい, so that 見どころに載せ漏れず、`audit-dola` の分と別々に探さずに済む

#### Acceptance Criteria

1. The roadmap の「次のリリースへの申し送り」 shall `dola` の公開の機能の変わる点を 1 項目にまとめて持つ（2026-11-13 の棚卸㊱が足した `audit-dola` の項目を、この spec の結果を足して書き直す形でよい。同じ話の項目を 2 つ並べない）
2. The その項目 shall `audit-dola` が消した機能 `json`（既定で有効だった）・`toml` と任意の依存 `toml`、既定の機能が無くなったことを、壊れる変更として書く
3. Where 案 B（外す）を採るとき, the その項目 shall 機能 `yaml` も消えたこと（壊れる変更・`Cargo.toml` の機能の指定を消せば直る）と、利用者が自分で serde の書式のクレートを選べば同じ型を読み書きできることを書く
4. Where 案 A（替える）を採るとき, the その項目 shall 機能 `yaml` はそのまま指定できること、機能 `yaml` が引く YAML のクレートが替わったこと、要件 4.3 で見つけた読み方の違いがあればそれを書く
5. The その項目 shall 「次のリリースへの申し送り」の頭の決まり（書いたときの最新の版を「`v{その版}` の後」の形で頭に書く）に従う
6. The その項目 shall 「機能 `yaml` の行き先は `dola-yaml-backend` が決める」のような未決の書き方を含まず、決まった結果だけを書く

### Requirement 7: 完了の前に全体を確かめる

**Objective:** As a areka の開発者, I want 直しが全体テストとライセンスの段を通ったことを確かめてから完了したい, so that 依存の木を動かした直しで、ほかのクレートが壊れていないと言える

#### Acceptance Criteria

1. When 完了の全体テストをライセンスの段込みで回すとき, the 全体テスト shall `cargo deny check` と `cargo about generate` の段（作り直した `THIRD-PARTY-NOTICES.md` に差分が無いことの判定を含む）を含めて全部緑になる
2. If 全体テストでこの spec が触っていないクレートだけが赤になったとき, then the 完了の手順 shall 止めて報告し、赤のクレートだけをあと 1 回回して、変化が無ければ申し送りを残して先へ進む
