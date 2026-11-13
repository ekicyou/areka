# Brief: dola-yaml-backend

> 2026-10-15 `/kiro-discovery` で起票（`areka-P0-release-cycle` の完了の全体テストの赤から・開発者「それで」）。

## Problem

`dola` の任意の機能 `yaml` が、非推奨の `serde_yaml` を通して `unsafe-libyaml` を引く。`unsafe-libyaml` は RustSec の勧告 `RUSTSEC-2024-0449`（保守されていない・作者がリポジトリを閉じた・直した版は出ない）に載っており、2026-10-15 の全体テストで、ライセンスの段の `cargo deny check` が初めて赤になった（2026-10-11 の完了の全体テストでは緑だった＝勧告の取り込みが後から届いた）。ほかの spec の完了も `/release` の手元の確かめも、同じ所で止まる。

`unsafe-libyaml` は libyaml の C のコードを機械で Rust に写した物で、見つかった不具合は直されない。`dola` は crates.io に公開しているクレートなので、この機能を有効にした外の利用者がそのまま引き継ぐ。

## Current State

- `crates/dola/Cargo.toml`: `[dependencies.serde_yaml]`（`version = "0.9"`・`optional = true`）と、機能 `yaml = ["dep:serde_yaml"]`。既定の機能は `json` だけ。
- ワークスペースの中で `dola` の機能 `yaml` を有効にしているクレートは 0 本＝areka の配布物には入らない。`cargo deny` は `deny.toml` の `[graph] all-features = true` で全部の機能を有効にして調べるので、ここで引っかかる。
- 使い道: `crates/dola/tests/general/integration_test/serialization_round_trip.rs` の YAML の往復のテスト（`#[cfg(feature = "yaml")]`）と、`crates/dola/README.md` の「JSON・TOML・YAML で直列化できる」の説明。`crates/dola/src` に `serde_yaml` の語は無い（serde の導出だけで YAML に乗る）。`compile/boundary_test.rs` のコメントが YAML の `.nan`／`.inf` の流入経路に触れている。
- 2026-10-15 に `deny.toml` の `[advisories] ignore` へこの勧告 1 件だけを、理由とこの spec の名前を添えて入れた（`release-cycle` の完了の PR）。

## Desired Outcome

- `dola` の依存の木から `unsafe-libyaml` が消え、`deny.toml` の見逃しの指定を消しても `cargo deny check` が緑。
- `dola` の YAML の扱いが決まっていて、README・テスト・crates.io の利用者への説明（次のリリースの見どころ）がそれに合っている。

## Approach

要件の段で次のどちらかを決める（推す形は決めていない）。

- **替える**: `serde_yaml` を安全な Rust の YAML の読み書き（例 `serde-saphyr`。勧告が代わりに挙げている）へ替え、機能 `yaml` の名前と外から見た形を保つ。往復のテストをそのまま通す。`.nan`／`.inf` などの読み方の違いは境界のテストで確かめる。新しい依存のライセンスと保守の状況は設計の前に確かめる。
- **外す**: 機能 `yaml` ごと外す（使い手が areka の中に居ないため）。README と往復のテストから YAML を外す。crates.io の利用者にとっては壊れる変更なので、次の版上げの見どころの「必ず載せるもの」になる。

## Scope

- **In**: `crates/dola/Cargo.toml` の YAML の依存と機能・YAML の往復のテスト・`crates/dola/README.md` の YAML の記述・`deny.toml` の見逃しの指定の削除・`Cargo.lock` と `THIRD-PARTY-NOTICES.md` の追随。外すか替えるかで変わる公開の形は、roadmap の「次のリリースへの申し送り」へ 1 項目足す。
- **Out**: JSON・TOML の直列化・`dola` の型の形・ほかのクレート。

## Boundary Candidates

- `dola` の直列化の形式の選び方（機能の旗）
- 依存の監査の見逃しの指定（`deny.toml`）

## Out of Boundary

- `cargo deny` の設定のほかの行（`[graph] all-features` を外して見逃す、のような監査の範囲を狭める直しはしない）。
- `dola` の公開の手順（スキル `/release` と `doc/crates-io-publish.md`）。

## Upstream / Downstream

- **Upstream**: なし（いつでも着手できる）。
- **Downstream**: 次の `/release`（`dola` の公開の形が変われば見どころに載る）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `dola` を触る spec（`clippy-199-lints`・`range-choice-tag`・`element-animation-easing` ほか。触るのはソースで、`Cargo.toml` の依存の行とは重ならない）。根の `Cargo.lock`・`THIRD-PARTY-NOTICES.md` を動かすので、依存を足す spec と版上げ（スキル `/release`）とは同じウェーブに置かない（roadmap の「1 ウェーブに 1 本だけの席」）。

## Constraints

- 依存の監査の見逃しは、この spec が片づくまでの仮の物。片づいたら `deny.toml` の `ignore` からこの 1 件を消す（理由の文にこの spec の名前を書いてある）。
- 規模の見立て: XS〜S（3〜6 タスク）。替えるなら新しい依存を足す＝`tech.md` の依存の表も直す。

## 2026-11-13 棚卸㊱の再測定（main 069f1a48）

- 前提の変化: `audit-dola`（11-13 #440）が機能 `json`・`toml` と任意の `serde_json`・`toml` を消し、`serde_json` を `[dev-dependencies]` へ移し、既定の機能を無くした（あちらの #25）。機能 `yaml` と任意の `serde_yaml` はこの spec の持ち物として残した＝Current State の「既定の機能は `json` だけ」は古い。YAML の往復のテストは `crates/dola/tests/general/integration_test/serialization_round_trip.rs` 41〜55 行目（TOML の往復は消えた）、README は 13・17 行目で「YAML via the `yaml` feature flag」。`deny.toml` 43〜46 行目の見逃しの 1 件はそのまま。機能を外す形には同じ crate の先例ができた（json・toml）。
- 触るファイル: `crates/dola/Cargo.toml`（31 行）・往復のテスト（55）・`crates/dola/README.md`（28）・`deny.toml`（61）・`crates/dola/tests/compile/boundary_test.rs` 261〜262 行目の注記（NaN の流入の道）・根の `Cargo.lock` と `THIRD-PARTY-NOTICES.md`・`.kiro/steering/tech.md` 54 行目。
- 依存の席: 替える形は依存を足す（`serde-saphyr` などは今の `Cargo.lock` に無い）。外す形は依存を足さないが `Cargo.lock`・`THIRD-PARTY-NOTICES.md` から `serde_yaml`・`unsafe-libyaml` が消える＝どちらでも C19 の `Cargo.lock` の席を使う。
- 規模: XS〜S（3〜6）のまま。
- 先に要るもの: 無い（`audit-dola` が着地＝本文の「Upstream: なし」がまた正しくなった）。同じ `crates/dola/Cargo.toml` を触る `dola-interrupt-ticks` とは同じウェーブに置かない。C19 の見込みのほかの 7 本とは重なり 0（`crates/dola/` と `deny.toml` に触る相手が居ない・`tech.md` は許す重なり）。
- 優先度の区分: C（`release-cycle` の完了の全体テストの赤から起票した dola の手入れ）。
- 要件定義のモデル: Opus。
- 分割の案: 無し。
- 見つけた穴・古くなった記述: ⑴ `tech.md` の「dola クレート依存」の 52・53 行目（`serde_json` の機能 `json`・既定で有効／`toml` の機能 `toml`）と 32 行目の「`dola` の `toml` 機能は別に 0.8 のまま」は `audit-dola` の後で古い。⑵ `audit-dola` が消した機能 `json`・`toml` は crates.io の利用者にとって壊れる変更なのに、roadmap の「次のリリースへの申し送り」に項目が無い＝この spec が足す 1 項目に `audit-dola` の分もまとめて載せる。機能 `yaml` を外す形を選べば、それも同じ項目へ。
