# Brief: areka-P0-tools-utf8-child-output

> 2026-10-05 `/kiro-discovery` で起票。出どころは `release-cycle` の初回の実装（3.3 `tools/package.ps1 -Check` が終了コード 3 で赤）。開発者の方針（同日）「まずはコンソールをいじるべきじゃない」＝文字化けはコンソールの文字コードを変えて逃げず、読む側を直す。段＝**バグ**。`release-cycle` の初回（`v0.0.2`）はこの直しが main に入るのを待っている。

## Problem

リリースの道具が、子のプロセス（`cargo metadata`）の UTF-8 の出力を**コンソールの文字コード**で解いている。開発者の端末（Shift_JIS＝コードページ 932）では、日本語の `description` を持つクレートの所で JSON が壊れ、`tools/package.ps1 -Check` が「版を読めない（cargo metadata の出力が JSON として読めない）」の終了コード 3 で止まる。初回リリースの手順 1（前提の確認）が通らない。

同じ穴を `tools/crates-io.ps1` は `[Console]::OutputEncoding` を UTF-8 に書き替えて避けている。これは共有物である端末の設定を道具が書き替える形で、開発者の方針に反する（スクリプトが終わった後も端末に残る）。

## Current State

- 読み方が同じ所は 3 か所（どれも `@(cargo metadata … 2>&1)` を素で受けて `ConvertFrom-Json`）:
  - `tools/package.ps1` の版を読む段（「版（正本は Cargo.toml の [workspace.package] version…）」の注記の直下）
  - `.github/workflows/release.yml` の段「版の検査」（注記に「版の読み方は tools/package.ps1 と同じ」＝同じ本文の写し）
  - `tools/crates-io.ps1` の「2〜6. 実物の判定」の先頭
- コンソールの書き替え:
  - `tools/crates-io.ps1` の冒頭（注記「cargo の出力（JSON・UTF-8）を文字化けさせずに読む」）＝手元の端末を書き替える。直す対象。
  - `.github/workflows/release.yml` の各段の先頭（9 か所）と `.github/workflows/crates-io.yml` の各段の先頭（5 か所・注記「日本語のログを化けさせない」）＝ランナーの中の、その段だけの端末。段「版の検査」の 1 か所は読み方の穴を隠してもいる。
  - `release.yml` の段「zip を作る」は子の pwsh の中で書き替えてから `package.ps1` を呼ぶ（子の中だけ）。
- 起票時の再現（2026-10-05・main `44fc0a61`・端末に触れずに確かめた）: `cargo metadata --no-deps --locked --format-version 1` の出力（30 クレート）を UTF-8 で解けば JSON は通る。同じバイト列をコードページ 932 で解くと `packages[5].description` で JSON が壊れる。クレートごとに見ると、少なくとも `areka-nar`・`areka-ghost`・`areka-mcp` の `description` で壊れる。
  - 壊れるかどうかは字の並びしだい（Shift_JIS の 1 バイト目に見えるバイトが、文字列を閉じる `"` を飲み込むと壊れる）。日本語の `description` は 13 クレートにあるが、壊れるのは一部だけ＝今まで通っていたのは偶然で、`description` を 1 字直すだけで赤にも緑にもなる。
  - α の完成判定（10-02）で `-Check` が緑だった理由は調べていない。壊れる `areka-nar` の `description` は 09-19（`nar-install`）から在るので、字が後から足されたせいではない＝回した端末の文字コードが違った見込み（今回は Claude Code の Bash〔Git Bash〕から pwsh を起こした形で、コンソールが `shift_jis`）。要件の段で確かめなくてよい（読み方を直せば理由に依らず直る）。
- 赤の実物（`release-cycle` の初回・`pwsh -NoProfile -File tools/package.ps1 -Check`・段「==> 前提の確認」・終了コード 3＝`$EXIT_BAD_ARGS`）: 「版を読めない（cargo metadata の出力が JSON として読めない: JSON からの変換が次のエラーで失敗しました: After parsing a value an unexpected character was encountered: s. Path 'packages[5].description', line 1, position 24246.）」。`packages[5]` は `areka-nar`。環境は pwsh 7.6.6・`[Console]::OutputEncoding`＝`shift_jis`・`[Text.Encoding]::Default`＝`utf-8`。
- `release-cycle` のセッションから聞き取った事実（2026-10-05・起票のときに相談）:
  - 赤になった環境は pwsh 7.6.6・コンソールは `shift_jis`。止まっているのは手順の 3.3（`-Check`）だけで、3.1・3.2（全体テストほか）と 3.4（`crates-io.ps1 -Verify -Version 0.0.2`）は緑。3.4 が緑なのは `crates-io.ps1` が自分でコンソールを書き替えているから。
  - 壊れた `description` は版上げの前から在る＝main でも同じく赤のはず（main の上では回していない）。
  - `tools/package.ps1` には、子の出力を素で受けて**値として使う**所がもう 1 か所ある: `Read-SamplePaths`（`cargo run … --bin nar-sample-path` の出力の `key=value` からパスを読み、`Test-Path` に掛ける）。作業ツリーのパスに日本語が入れば同じ穴になりうる（今の開発者のパスは英数字だけなので表に出ていない）。`@(git status --porcelain)` の 2 か所は `core.quotepath` で ASCII になるので対象外でよい。
  - `-Check` は最初の段で止まったので、その後の段に文字コードで落ちる所があるかは実走では見ていない。
  - `crates-io.ps1` の読み方は `tools/test-all.ps1` の「crates.io 公開前の確認」からも通る（書き替えだけを先に消すと、そこも赤になる）。
- 手本が main に在る: `tools/perf/perf-loop.common.ps1` の `Invoke-Child`（`ProcessStartInfo` の `StandardOutputEncoding` を UTF-8 に固定して読み、端末のコードページには触らない。注記に理由が書いてある）。

## Desired Outcome

- 端末の文字コードが何であっても（932 でも 65001 でも）、`tools/package.ps1 -Check` と `tools/crates-io.ps1 -Verify` が同じ結果を出す。
- `tools/` の道具は `[Console]::OutputEncoding` も `chcp` も書き替えない。道具を回した後の端末の文字コードは回す前と同じ。
- `release.yml` の段「版の検査」は、端末の設定に頼らずに版を読む。
- 直したことを判定で固定する: 日本語の `description` を持つ今のワークスペースを、コードページ 932 の端末（か、それと同じ条件を作った子のプロセス）から読んで版が取れること。表示するだけでなく合否を付ける。

## Approach

**読む側を直す**。子の標準出力をバイトで受けて UTF-8 で解く（`ProcessStartInfo` の `StandardOutputEncoding` を UTF-8 に固定する形＝`perf-loop.common.ps1` の `Invoke-Child` と同じ形）。`cargo metadata` を読む 3 か所をこの読み方に揃え、`crates-io.ps1` の冒頭の書き替えを消す。

- 採らない案: 呼ぶ側で `[Console]::OutputEncoding` を UTF-8 にしてから回す（開発者が却下）／`chcp 65001` を手順に足す（同じ）／`description` を英語に直す（穴は残り、次に日本語を書いたクレートでまた壊れる）／`cargo metadata` をやめて `Cargo.toml` を正規表現で読む（版の正本の読み方が 2 つになる）。

## Scope

- **In**:
  - `tools/package.ps1`・`tools/crates-io.ps1`・`release.yml` の段「版の検査」の、`cargo metadata` の読み方
  - `tools/package.ps1` の `Read-SamplePaths`（`nar-sample-path` の出力からパスを読む所）を同じ読み方に
  - `tools/crates-io.ps1` の冒頭の `[Console]::OutputEncoding` の書き替えの撤去（消した後に `-Verify`・`-Pending` の日本語の出力と、`crates-io.yml` が受け取るクレート名の並びが変わらないことの確かめ）
  - コードページ 932 の条件での判定（上の Desired Outcome の最後）
  - `tools/` のほかのスクリプトに同じ読み方（子の出力を素で受けて字面や JSON として判定する所）が無いかの棚卸。在れば同じ形に直す（表示するだけの所は対象外＝下）
- **Out**:
  - 版上げそのもの・タグ・Release（`release-cycle`）
  - `tools/perf/`（既に正しい形）
  - 人が読むだけの表示の文字化け（例 `package.ps1` が組み立ての出力を 1 行ずつ写す所）。合否に関わらないので直さない。見つけたら brief か覚え書きに記録する
  - workflow の中身の作り替え（段の順・権限・待ち方）

## Boundary Candidates

- 子の出力を UTF-8 で読む小さな関数（`tools/` の中・1 つ）
- それを使う 3 か所の付け替え
- コードページ 932 の条件を作って回す判定

## Out of Boundary

- 端末・ランナーの設定を変える手順
- `Cargo.toml` の `description` の言い回し

## Upstream / Downstream

- **Upstream**: `release-package-versioned`・`release-ci-workflow`・`crates-io-publish`（どれも完了。直す先の持ち主）
- **Downstream**: `release-cycle` の初回（`v0.0.2`）＝この直しが main に入ってから、段 3（全体テスト → `-Check` → `-Verify`）を頭から回し直す。main での `release.yml` の乾いた走りもこの後に回す（`release.yml` が変わるため）。`winget-manifest-submission` が足す `winget.yml` は、この spec の読み方に倣う。

## Existing Spec Touchpoints

- **Extends**: なし（完了した 3 本の道具の直し。完了した spec の文書は書き替えない）
- **Adjacent**: `release-cycle`（作業木に版上げの 4 ファイルが未コミットで残っている。`tools/`・`.github/` には触らない約束＝重ならない）／`doc/crates-io-publish.md`（手順書に文字コードの記述があれば合わせる）

## Constraints

- コンソールの文字コードを書き替えない（`[Console]::OutputEncoding`・`[Console]::InputEncoding`・`chcp`・`$OutputEncoding` を道具の中から変えない）。
- `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`dist/README.txt` に触らない。版を上げない・依存を足さない（`release-cycle` の作業木に残る版上げの 4 ファイルと重ねない）。
- `crates-io.ps1` の冒頭の書き替えを消すのは、読み方の直しと**同じ PR**で（先に消すと公開前の確かめ `-Verify` が赤になる）。
- 直った確かめは、コンソールを `shift_jis` のままにして `tools/package.ps1 -Check` を最後まで緑にすること（版を読む段を越えた後にも同じ穴が無いことを見る。窓が出るので開発者の手元で）。
- **`release-cycle` との約束**: 初回リリースはこの直しのマージを待つ。マージの後、`release-cycle` のワークツリーで `/kiro-impl areka-P0-release-cycle` を打ち直す（1.2 で main を取り込み、3.1 の全体テストから回し直す）。
- `release.yml` は「入力は環境変数だけ・手元でも同じ本文を回せる」形を崩さない。直した後、`package.ps1` の読み方と写しのままにするか、1 つの出どころへ寄せるかは議題 1。
- 終了コードの約束（`package.ps1` の 3＝引数・前提の誤り）と失敗の文は変えない。
- 秘密を印字しない（子の環境変数を表示させない）。

## 想定

- 規模 XS〜S（3〜5 タスク）。Opus で足りる。並び＝**C4 のバグの席・`release-cycle` の初回の前**。触るのは `tools/package.ps1`・`tools/crates-io.ps1`・`.github/workflows/release.yml`（議題 2 しだいで `crates-io.yml`）だけで、C4 のほかの 16 本と触るファイルの重なりは 0。
- 議題（答えで作業が変わるものだけ）:
  1. **読む関数の置き場**。`package.ps1` と `crates-io.ps1` は別々のスクリプトで、`release.yml` は本文の写しを持つ。⒜ 3 か所にそれぞれ同じ数行を書く（今の「写し」の作りのまま）⒝ `tools/` に小さな共通のスクリプトを 1 つ置いて 3 か所が読み込む ⒞ `release.yml` は自前で読まず `package.ps1` に版だけ答えさせる口を足す。推しは ⒝（同じ穴を 3 か所に写していたのが今回の元）。
  2. **workflow の各段の先頭の `[Console]::OutputEncoding`（`release.yml` 9 か所・`crates-io.yml` 5 か所）**。ランナーの端末はその段だけの物で、開発者の端末ではない。⒜ 読み方を直す段「版の検査」だけ頼らない形にし、ログを化けさせないための書き替えは残す ⒝ workflow からも全部消し、子の出力を読む所はすべて関数を通す。推しは ⒜（ランナーのログが化けると赤の原因を読めなくなる。方針の「コンソールをいじらない」は開発者の端末の話と読んだ＝要件ディスカッションで確かめる）。
