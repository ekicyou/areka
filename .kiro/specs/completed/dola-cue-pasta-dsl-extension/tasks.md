# Implementation Plan: dola-cue-pasta-dsl-extension

## Task Overview

areka 側で確定した Cue DSL 拡張仕様を upstream pasta 実装へ引き継ぎ、公開リリースの取り込みまで完了させる。ローカル実装は仕様成果物の完成と依存更新の検証を担当し、文法実装本体は pasta v0.1.6 で完了したものとして扱う。

## Tasks

- [x] 1. Cue DSL 仕様成果物を完成させる
- [x] 1.1 要件・設計・サンプル・handoff 文書を整備する
  - `requirements.md` に Cue DSL 拡張要件を整理する
  - `design.md` に PEG 文法、責務境界、Builder 設計を記述する
  - `cue.pasta` と `implementation-request.md` を upstream 実装依頼用成果物として整備する
  - _Requirements: 1.1, 2.1, 3.1, 4.1, 5.1, 6.1, 7.1, 8.1_

- [x] 2. upstream pasta 実装完了を反映する
- [x] 2.1 pasta 側の Cue コマンド文法拡張実装と公開版を確認する
  - pasta リポジトリで Cue DSL 拡張実装が完了していることを確認する
  - 公開版 `pasta_core 0.1.6` / `pasta_dsl 0.1.6` が利用可能であることを確認する
  - 本 spec を completed へ移動できる状態にする
  - _Requirements: 2.2, 3.2, 6.2, 8.2, 8.3_

- [x] 3. areka ワークスペースへ新バージョンを取り込む
- [x] 3.1 pasta 依存を versioned release へ切り替えて検証する
  - ワークスペースの `pasta_core` 依存を git 参照から `0.1.6` へ更新する
  - ステアリングの依存記述を versioned release に合わせて更新する
  - `cargo build` と `cargo test` で取り込み結果を確認する
  - _Requirements: 8.3_