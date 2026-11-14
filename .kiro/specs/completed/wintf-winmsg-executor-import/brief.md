# Brief: wintf-winmsg-executor-import

## Problem
`wintf` の土台のクレート `wintf-winmsg-executor`（メッセージループ・窓の生成・UI スレッドの async）は、開発者のフォークで、areka とは別のリポジトリ `ekicyou/wintf-winmsg-executor` に在り、crates.io の 0.0.5 を `=` で固定して使っている。困っているのは開発者と、`windows` まわりの依存を動かすセッションである。

- このクレートは公開面に `HWND` を出していて（`pub hwnd: HWND`・`pub fn hwnd(&self) -> HWND`）、`windows = "0.62"` に頼る。`wintf` と同じ変更で動かさないと型が合わなくなる。別リポジトリのままだと、「向こうで版を出す → areka が受ける」の 2 段になり、1 つの PR で確かめられない。
- 2026-11-14 の `/kiro-discovery` で、windows-rs 0.101 系への移行の行き先を「`windows` クレートを使わず、`windows-bindgen` で作った束縛で API を呼ぶ」と決めた（開発者の裁定）。このクレートは C の関数しか呼ばず小さいので、移行の最初の的になる。どう転んでも、その前に areka のリポジトリへ移しておく必要がある（開発者「とりあえず、どう転んでも実施することになる、wintf-winmsg-executorの移動から」）。

## Current State
2026-11-14 に確かめた事実。

- **crates.io**: 持ち主は `ekicyou` だけ（`wintf`・`dola` と同じ）。0.0.1〜0.0.5 の 5 版はどれも手元からの公開で、Trusted Publishing は設定されていない。`repository` の欄は `https://github.com/ekicyou/wintf-winmsg-executor`。
- **中身**: `src/lib.rs`（524 行）・`src/util/mod.rs`（5 行）・`src/util/msg_filter_hook.rs`（57 行）・`src/util/window.rs`（333 行）と、例 `examples/basic.rs`・`examples/threads.rs`。依存は `async-task` 4.7.1（既定の機能なし）と `windows` 0.62（機能 `Win32_Foundation`・`Win32_Graphics_Gdi`・`Win32_System_Threading`・`Win32_UI_WindowsAndMessaging`）だけ。edition は 2021。
- **ライセンス**: `MIT OR Apache-2.0`。フォーク元は `timokroeger/winmsg-executor` で、`LICENSE-MIT`・`LICENSE-APACHE` の 2 つを同梱している。areka の `deny.toml`・`about.toml` は MIT も Apache-2.0 も受理している。areka のワークスペースの `license` は `MIT` だけなので、このクレートは `license.workspace` を使えない。
- **手元の写し**: `C:\home\maz\git\wintf-winmsg-executor`。作業木は汚れなし、`main` は `origin/main` と同じで先頭は `4f20275`。crates.io の 0.0.5 が記録しているコミット `1a71d65f` とは番号が違うが（PR を squash でマージしたため）、中身の差は 0（`git diff 1a71d65f HEAD` が空）。向こうのリポジトリは自分の `.claude`・`.kiro`・`CLAUDE.md` を持つ。
- **areka の側の使い方**: 根の `Cargo.toml` の `[workspace.dependencies]` に `wintf-winmsg-executor = "=0.0.5"`。`wintf`・`areka-actor`・`areka-emo-text`・`shiori-host32-host`・`shiori-host32-helper` が `workspace = true` で使い、`crates/pilot/Cargo.toml` だけが `"0.0.5"` を直書きしている。`shiori-host32-helper` は i686 でも組む。
- **公開の仕組み**: `tools/crates-io.ps1` の公開の一覧は `$PUBLISH = @('dola', 'wintf')` で、公開するクレートの版がリリースの版と同じことを確かめて止まる（`Test-Version`）。公開の段 `.github/workflows/crates-io.yml` は Trusted Publishing だけで認証する。手順書は `doc/crates-io-publish.md`。出さないクレートは `publish = false # 理由` の行を持つ決まり（`tech.md` の「crates.io への公開」）。

## Desired Outcome
- `crates/wintf-winmsg-executor/` に、手元の `main`（`4f20275`）と同じ中身のクレートが在り、ワークスペースの一員として組める。動きは変わらない。
- areka の全クレートが crates.io の 0.0.5 でなく、この写しを使う（`Cargo.lock` に crates.io 由来の `wintf-winmsg-executor` が残らない）。
- 次のリリースで、公開の段が `dola`・`wintf` と一緒に `wintf-winmsg-executor` をリリースの版で crates.io へ出せる形になっている（公開の一覧・版の確かめ・手順書がそろっている）。
- 元のリポジトリが「移転した」と分かる状態になっている（README に移転先・archive）。
- `tech.md`・`structure.md`・`/release` の手順が新しい形を説明している。

## Approach
手元の `main` を写すだけにする。`git archive` で追跡ファイルのうちクレートの中身（`src`・`examples`・`README.md`・`LICENSE-MIT`・`LICENSE-APACHE`・`Cargo.toml`）を `crates/wintf-winmsg-executor/` へ取り出し、取り込みのコミットに元のコミット番号 `4f20275` を書く。向こうの `.claude`・`.kiro`・`CLAUDE.md`・`.gitignore`・`Cargo.lock` は写さない。

- 履歴は写さない。本線へは squash マージしか入れないので、コミットの履歴は残せない。履歴の置き場は元のリポジトリ。
- 一時のサブモジュールにはしない（開発者と 2026-11-14 に検討）。最後は areka の木にファイルが入っていなければ crates.io へ出せず、`.gitmodules` の足しと外しで手間が 2 回増えるだけだから。
- 根の `Cargo.toml` の依存は、`dola` と同じ「版＋path」の形にする。`crates/pilot/Cargo.toml` の直書きは `workspace = true` にする。
- 版はワークスペースの版に合わせる（`version.workspace = true`）。公開の道具が「公開するクレートの版＝リリースの版」を確かめているので、独自の版を保つと道具に例外が要る。次のリリースで 0.0.5 からリリースの版へ飛ぶが、crates.io では問題にならない。

## Scope
- **In**:
  - `crates/wintf-winmsg-executor/` の新設（中身は手元の `main` の写し）と、その `Cargo.toml` の areka 向けの直し（`version.workspace`・`repository`・`publish`・`license = "MIT OR Apache-2.0"` を自分で持つ）。
  - 根の `Cargo.toml` の `[workspace.dependencies]` の行と、`crates/pilot/Cargo.toml` の直書きの直し。`Cargo.lock` の更新。
  - `tools/crates-io.ps1` の公開の一覧と、名前を決め打ちしている較正・文言の直し。`.github/workflows/crates-io.yml` の注記。
  - 文書: `doc/crates-io-publish.md`（出すものが 3 つ・出す順・Trusted Publishing をこのクレートにも設定する手順）・`.kiro/steering/tech.md`（Key Libraries の `=0.0.5` の項・「crates.io への公開」の節・版上げで動かす行）・`.kiro/steering/structure.md`・スキル `/release` の手順のうち版上げで動かす行。
  - 全体テスト（`tools/test-all.ps1`）が緑であること。i686 の補助 exe が組めること。
  - 元のリポジトリの後始末（README に移転先を書く・archive）。**実行の前に開発者の許しを得る**。
- **Out**:
  - クレートの動きの変更・公開面の変更。
  - `windows` 0.62 からの移行（`windows-bindgen` の束縛へ移すのは、先進坑 `pilot-windows-rs-next` の後に起票する移行の本体）。
  - edition 2021 から 2024 への引き上げ（写すだけにする。上げるかは要件の段で決めてよいが、既定は上げない）。
  - フォーク元 `timokroeger/winmsg-executor` への追随。

## Boundary Candidates
- クレートの写しとワークスペースへの組み込み（`crates/wintf-winmsg-executor/`・根の `Cargo.toml`・`crates/pilot/Cargo.toml`・`Cargo.lock`）
- crates.io への公開の引き継ぎ（`tools/crates-io.ps1`・`crates-io.yml`・`doc/crates-io-publish.md`・`/release` の手順）
- steering の登記（`tech.md`・`structure.md`）
- 元のリポジトリの後始末（areka のリポジトリの外）

## Out of Boundary
- `wintf` の `runtime/`（このクレートを使う側）の作り
- crates.io の Trusted Publishing の設定そのもの（開発者のログインが要る操作。手順書に書き、開発者に頼む）
- `wintf`・`dola` の公開の仕組みの作り替え

## Upstream / Downstream
- **Upstream**: 完了 spec `wintf-winmsg-executor`（このクレートを `wintf` の土台にした）・完了 spec `areka-P0-crates-io-publish`（公開の道具と手順書を作った）。
- **Downstream**: `pilot-windows-rs-next`（取り込んだクレートの写しを `.sys()` の束縛へ移して手間を測る）と、その後に起票する windows-rs の移行の本体（このクレートが最初の的）。

## Existing Spec Touchpoints
- **Extends**: 無し。
- **Adjacent**: `tech.md`・`structure.md` を書き換える spec（本体のクレート分けの直列・見直しの spec）。根の `Cargo.toml`・`Cargo.lock` に触る spec（`/update` を含む）。どちらも触る行は少ないが、同じウェーブに置くなら後の方が取り込み直す。

## Constraints
- **動きを変えない**。写した `src`・`examples` は、全体テストの整形（`cargo fmt --all`）が動かす分のほかは書き換えない。`windows` は 0.62 のまま。
- **ライセンスの表示を保つ**。`LICENSE-MIT`・`LICENSE-APACHE` をクレートのフォルダに置き、フォーク元の著作権の表示を消さない。`license` は `MIT OR Apache-2.0` のまま。
- **Trusted Publishing は開発者の操作**。crates.io にログインして `wintf-winmsg-executor` の Settings に `ekicyou`／`areka`／`crates-io.yml`（environment は空）を足す。**取り込みが本線に入った後の最初のリリースのタグより前に**済ませる。済んでいないと公開の段が途中で止まる（`dola` だけ上がった状態になりうる）。完了の報告でこの 1 点を必ず伝える。
- **元のリポジトリを触る前に許しを得る**（README の PR・archive はリポジトリの設定の変更で、外から見える）。
- **外部クレートはそのまま使う決まり**（フォークも `[patch]` もしない）には当たらない。このクレートは開発者の持ち物で、取り込みは持ち物の置き場所を移すだけ。
- 依存を増やさない（`async-task` は今も `Cargo.lock` に在る）。
- 新しいクレートを足すときの決まり（`publish` の行・`cargo-about` 0.9.2 が `[package]` の最後の行の辺りへ 1 行を差し込む件）に従う。
- **規模**: S（5〜8）。
