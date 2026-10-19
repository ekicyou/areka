# 設計の検証: areka-P0-sample-url-fetch

> 2026-10-18・`/kiro-validate-design`（対話なしで実施）。入力: `spec.json`（language: ja）・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C9）。
> 設計の主張は実物のコードで確かめた（下の「実物での確かめ」）。決まっている縛り（新しい依存なし・取り寄せた書庫はワークツリーの `target\` の下だけ・`tools/test-all.ps1` は `areka-P0-impl-watch-test-desk` の着地の後にだけ変える・ぱすたの行はこの spec が登記する・テストは通信しない）は問題として扱わない。

## 設計の検証のまとめ

要件 1〜7 のすべての受け入れ基準が部品・口・流れまで対応づけられており、今の窓口の形（`*_in` から先を変えず、書庫のパスを決める所を `sample_nar` の 1 か所に寄せる）にきれいに乗っている。登記の正本を Rust の 1 か所に置いたまま、取り寄せの段は `--fetch-list` で聞く形なので、`CARGO_TARGET_DIR` の食い違いも 2 か所に書くことも起きない。下の 3 点は、全体テストの入口での止まり方と、取り寄せの段の例外の扱いをはっきりさせる手直しで、骨組みを変えるものではない。

## 重大な問題（3 件まで）

🔴 **重大な問題 1**: 全体テストの入口で、取り寄せの段の失敗が「テスト」だけでなく全部の段を止める（置く位置もあいまい）
**懸念**: TestAllHook の契約は「終了コードが 0 でなければ、ほかの段を回さずに終了コード 1」と書き、置く位置を「最初のテストの段より前（作業の場所と `PATH` の手当ての後）」としている。要件 6.2 が求めるのは「テストへ進まずに止まる」ことで、テストでない段（`fmt --check`・`-Format` の整形・`encoding check`・各 `-SelfTest`・`-License`）まで止めることは求めていない。今の `test-all.ps1` は「赤でも最後まで回し、段ごとの合否を一覧にする」作りで、`kiro-complete` の 1 回の正本でもある。
**影響**: 通信できないときや、`sample-ghost-kit`／`areka-nar` が組めないとき、完了の門で整形・文字コード・ライセンスの合否が 1 つも出ない。`-Format` の整形も走らない。2 つの読み方（PATH の直後か、`fmt --check` の後か）で、タスクの段で実装が割れる。
**提案**: 契約を「失敗したら `x64 workspace tests`・`i686 tests (host-32)` の 2 段（`cargo test` の段）だけを回さず、`sample fetch` を FAIL として一覧に載せ、ほかの段は今どおり回して終了コード 1」に改めるか、全部を止めるのならその理由（完了の門で何を失ってよいか）を設計に書く。位置は「`fmt --check` の後・`x64 workspace tests` の前」のように段の名前で指す。
**対応する要件**: 6.2（・6.3 のゲート後のタスク）
**根拠**: design.md「TestAllHook（`tools/test-all.ps1`・ゲート後のタスクだけ）」のつなぎ方・「Error Categories and Responses」の全体テストの入口の行。`tools/test-all.ps1` の `.DESCRIPTION`「段が赤でも止めずに最後まで回し」。

🔴 **重大な問題 2**: 取り寄せの段で、行ごとの想定外の例外が全体を止めうる・消し残しの掃除が並走の走行の一時ファイルを消しうる
**懸念**: スクリプトは `$ErrorActionPreference = 'Stop'` で、`Sync-FetchedSample` の失敗の道として決めてあるのは `curl.exe` の終了コードと大きさ・SHA-256 の不一致だけ。`[System.IO.File]::Move`・`Get-FileHash`（開かれている書庫）・一時ファイルの削除が投げたときの扱いが書かれていない。また手順 1「前の走行の消し残し `<名前>.nar.part-*` を消してみる（消せないものは触らない）」は、並走する走行が `curl.exe` を終えて（ファイルが閉じて）から `Move` するまでの間の一時ファイルを消せてしまう。設計の「2 つ同時に回しても…冪等」の主張はこの隙間を見ていない。
**影響**: 例外が 1 つ出ると最後の数の行が出ずに落ち、要件 2.7（数と終了コードを必ず示す）を破る。並走（実装係とレビュー係が同じワークツリーで回す等）で、正しく取り寄せた走行が `Move` の失敗で落ちる。
**提案**: `Sync-FetchedSample` の本体を行ごとの `try`/`catch` で包み、想定外の例外は ASCII に写した文つきの `failed` にする（数の行は必ず出す）と契約に書く。消し残しの掃除は「名前のプロセス識別子が生きていないものだけ消す」（`Get-Process -Id` が無い）に絞る。`-SelfTest` に「`Move` の先が開かれている → `failed`・数の行が出る」程度の判定を 1 つ足すかはタスクの段で決めてよい。
**対応する要件**: 2.5・2.7（・2.3）
**根拠**: design.md「FetchScript（`tools/sample-fetch.ps1`）」の 1 行の処理 1〜6・「冪等」の項・「取り寄せの段」の流れの後の注（一時ファイルと消し残し）。

🔴 **重大な問題 3**（軽め）: 「取り寄せの段は `https://` だけを取る」が実行時には守られていない・`fetched_archive_path` の今の行への振る舞いが未定義
**懸念**: Security Considerations は「取り寄せの段は登記の `https://` の URL だけを取る（行の形の判定が `https://` を求める）」と書くが、その判定は Rust の決定論テストにしか無く、スクリプトの本番の道は `--fetch-list` が出した URL を確かめずに `curl.exe` へ渡す（`curl.exe` は `file://` も受ける）。また公開の `pub fn fetched_archive_path(sample: &Sample) -> Result<PathBuf, SampleError>` は、`SampleSource::Vendored` の行を渡したときに何を返すかが書かれていない。
**影響**: テストを回す前の手元（行を書き足した直後）では、形の誤った行がそのまま取り寄せられる。公開の口の振る舞いが実装係の解釈に任される。
**提案**: スクリプトの `--fetch-list` を読む所で「URL が `https://` で始まらない行は `failed`（または登記を聞けない扱いで終了コード 1）」とする 1 行を契約に足す（`file://` は `-SelfTest` が `Sync-FetchedSample` を直に呼ぶときだけ）。`fetched_archive_path` は今の行に `None` を返す `Option` にするか、`--fetch-list` の中だけで使う `pub(crate)` 相当の形にして、今の行への振る舞いを 1 文で決める。
**対応する要件**: 1.2・1.5・2.9
**根拠**: design.md「Security Considerations」・「Registry」の Contracts（`fetched_archive_path`）・「FetchList」の呼び方 2。

## 設計の良いところ

1. **差分を 1 か所に閉じ込めた形**: 今の行と URL の行の違いを `devroot::sample_nar` だけに置き、`fresh_root`・`manual_root`・`cached_root` は `nar_dir().join(...)` を差し替えるだけで、`*_in` から先（展開・刻印・複製・後始末）と呼び手は 1 文字も変えない。置き場の位置も登記も Rust の 1 か所が正本で、取り寄せの段は `--fetch-list` で聞くので、`CARGO_TARGET_DIR` の食い違い・第 2 の綴り・見張り（`sample_path_guard_test.rs`）との衝突（結合テストに名前空間の綴りを書かない）をまとめて避けている。後続の URL の行や今の 8 本の移し替えも、行の `source` を替えるだけで済む。
2. **通信なしで本物の道を通す自分の検査**: `-SelfTest` が `file:///` の偽の行で本物の `curl.exe` の道（終了コード・一時ファイル・付け替え）を通し、差し替えの仕掛けを作らない。「2 回目は在りえない URL に替えて `present`」で「通信しない」ことまで判定にしている。窓口の照合も `NarBuilder` と私有の名前空間で 4 通りを決定論で固め、母数 0 の緑を防ぐ較正（URL の行が 1 つ以上・悪い入力で落ちる）まで入っている。

## 最終の判断

- **判断**: **GO**
- **理由**: 既存の作りとの食い違い・要件の抜けは無く、実装の道もはっきりしている。上の 3 点は全体テストの入口での止まり方と、取り寄せの段の例外と入力の扱いを契約に足す手直しで、設計の討議かタスクの段で畳める大きさである。
- **次の手**:
  1. 設計の討議（`/kiro-design-discussion areka-P0-sample-url-fetch`）で上の 3 点を議題にし、design.md の TestAllHook・FetchScript・Security・Registry の契約に反映する。
  2. その後 `/kiro-spec-tasks areka-P0-sample-url-fetch`。最初のタスク（実物の確かめ・要件 5.2・5.7）とゲート後のタスク（要件 6.3）の位置は設計の「実装の順」どおりに保つ。

## 実物での確かめ（2026-10-18・この枝の HEAD `27bdf160`）

- `crates/sample-ghost-kit/src/lib.rs`: `Sample` は `#[derive(Clone, Copy, Debug)]`、`SampleError` は `#[non_exhaustive]`（種類を足しても外は壊れない）、`registered` は crate 内の関数。設計の記述どおり。
- `crates/sample-ghost-kit/src/devroot.rs`: `nar_dir().join(format!("{name}.nar"))` を組むのは `cached_root`（`#[cfg(test)]`）・`fresh_root`・`manual_root` の 3 か所。`namespace_dir` は `CARGO_TARGET_DIR` か実行ファイルの祖先の `target`。`sweep` は `work\` だけ、`reclaim_stale` は `cache\` だけを見る（`fetched\` を走査しない）。設計どおり。
- `crates/sample-ghost-kit/src/lib_tests.rs`: 数の直書きは `every_registered_sample_lands_where_its_registry_row_says`（8）と `every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`（`SAMPLES.len()` の判定）の 2 か所で、後者は `devroot::nar_dir()` を直に組む（設計は `sample_nar` へ替えるとしている）。`emily4.nar` を直に読む判定は今の行なので影響なし。
- `crates/sample-ghost-kit/src/bin/nar-sample-path.rs`: 今は成功 0・失敗はすべて 2。`tests/nar_sample_path_test.rs` は全部の行で 0、未登録で 2 を判定している。
- `crates/log-capture-kit/tests/sample_path_guard_test.rs`: ⑵⑷ の語は `SAMPLES` から組み、走査から外れるのは `crates/sample-ghost-kit/src/` だけ（`tests/` は見張られる）。設計の「結合テストに名前空間の綴りを書かない」はこれに合う。
- `tools/encoding-check.ps1`: `tools/*.ps1` の直下を規則 A〜D で見る。規則 D は素の `cargo` 呼び出しだけを捕まえ、`Invoke-Utf8Child cargo @(...)` は当たらない。規則 E の必須の表に新しいスクリプトを足す必要はない。
- `tools/utf8-child.ps1`: `Invoke-Utf8Child` は絶対パスの実行体と引数の配列を受け、`Code`・`Out`・`ErrLines` を返す（`curl.exe` にも使える）。
- `tools/test-all.ps1`: 段は `Step` で並び、赤でも最後まで回して一覧にする作り（重大な問題 1 の前提）。
- `.github/workflows/`: `cargo test` を回す段は無い（`release.yml` は `package.ps1` 経由で今の行の `emo2`・`StayseeBalloon` だけを引く）ので、URL の行を足しても CI は取り寄せに左右されない。
- 根の `.gitignore` の `target` で置き場は追跡外（要件 1.6）。
