# Brief: areka-P0-host32-testdll-marker-race

> 2026-10-04 `/kiro-discovery` で起票（`translate-pipeline` の完了の手順の中で見つけた、以前からの揺れ）。**種別はバグ**（テストの揺れ・製品の振る舞いは変わらない）。本文のソースの指し先は起票時（`1738ec11`）の実物＝着手時に引き直すこと。

## Problem

- **開発者**: 全体テスト（`tools/test-all.ps1`）の i686 の段が、コードを変えていないのにまれに赤になる。完了の手順の途中で赤を見ると、原因を調べて採り直す手間がかかり、本物の赤を見落とす元にもなる。
- 観測（2026-10-04・`translate-pipeline` の 6.1）: `cargo test -p shiori-host32-helper -p shiori-host32-ipc --target i686-pc-windows-msvc --no-fail-fast` で `shiori_proxy::tests::testdll_drop_invokes_courtesy_unload` が 1 度だけ「unload はまだ呼ばれていない」で落ちた（`crates/shiori-host32-helper/src/shiori_proxy.rs` の、load の直後・drop の前にある「印のファイルはまだ無い」の確かめ）。その段だけの採り直し 3 回は緑。

## Current State

- `testdll_drop_invokes_courtesy_unload` は、プロセス全体の環境変数 `HOST32_TESTDLL_UNLOAD_MARKER` に自分の一時フォルダの印のパスを差してから test DLL（`shiori-host32-testdll`）を load し、drop の courtesy unload で test DLL が印のファイルを作ることを確かめる。test DLL は unload のたびにこの環境変数を読み、値があれば印を書く（`crates/shiori-host32-testdll/src/lib.rs`）。
- 直列化の錠 `TESTDLL_SERIAL` は `shiori_proxy.rs` の `mod tests` の中だけにあり、取っているのは同じモジュールの 2 本だけ。テストのコメントは「testdll を load する唯一のテスト」と書いているが、今は違う。
- 同じテストのバイナリの中で、錠を取らずに test DLL を load して unload するテストがある: `crates/shiori-host32-helper/src/main_loopback_tests.rs`（helper の窓を自前で組み、LOAD の経路で test DLL を読む）。このテストの unload が、drop のテストが環境変数を差している間に走ると、drop のテストの印を書く＝観測した症状と一致する（最有力の仮説・着手時に確かめる）。
- `shiori_proxy_loadu_tests.rs` も `ShioriByteProxy::load` を呼ぶ。こちらが同じ環境変数を読む DLL を使うかは着手時に確かめる。

## Desired Outcome

- test DLL を load するテストが何本あっても、印のテストが他のテストの unload に影響されない。
- sleep を使わない（決定論）。
- 直す前に、揺れを決定論で起こせる形があれば赤のテストを添える（起こせなければ、競合の経路をコードで示し、その経路を塞いだことを確かめる）。
- i686 の段を 10 回続けて回して緑。

## Approach

設計で決める。候補は次の 2 つ。
- test DLL を load するテストすべてが同じ錠を取る（錠をクレート内の共有の置き場へ移す）。
- 印の置き場をテストごとに分ける（環境変数をやめ、load のたびに渡せる形にする・test DLL 側の読み方を変える）。

## Scope

- **In**: `shiori-host32-helper` のテストのうち test DLL を load するもの、`shiori-host32-testdll` の印の読み方（変える案を採るなら）。i686 の段の決定論。
- **Out**: 製品の helper・`ShioriByteProxy` の振る舞い。x64 の host32 e2e。

## Boundary Candidates

- `crates/shiori-host32-helper/src/shiori_proxy.rs` の `mod tests`（錠と印のテスト）
- `crates/shiori-host32-helper/src/main_loopback_tests.rs`・`shiori_proxy_loadu_tests.rs`（test DLL を読む他のテスト）
- `crates/shiori-host32-testdll/src/lib.rs`（印の書き方）

## Out of Boundary

- `shiori-host32-host` の側（x64）のテスト
- helper の本番の LOAD・unload の手順

## Upstream / Downstream

- **Upstream**: なし（以前からの揺れ）
- **Downstream**: なし（全体テストの安定）

## Existing Spec Touchpoints

- **Extends**: なし
- **Adjacent**: `clippy-199-lints`（同じクレートのテストに `drop_non_drop` の指摘があるが、触る行は重ならない見込み）

## Constraints

- i686 でしか走らないテスト。前提として `cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc`。
- テストの決定論（sleep で同期しない）・1 ファイル 1,000 行未満。
