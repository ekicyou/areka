# Brief: areka-P0-dola-yaml-backend

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
