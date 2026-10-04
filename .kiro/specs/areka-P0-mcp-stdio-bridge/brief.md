# Brief: areka-P0-mcp-stdio-bridge

> 2026-09-29 `/kiro-discovery` の続きで起票（開発者判断「SSP と同じ対応を行うには中継 exe を作らないとダメ。どうせ作りたくなるので spec を置いてロードマップに置く」）。SSP MCP 移植の **M2 の並走枠**。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（§1・§6）。

> 2026-10-03 注記: `areka-P0-mcp-server-core` の開発者裁定で既定の待受は 9801 → 9821（どちらも使用中なら隣の +1〜+9 の計 20 候補・最初に束ねた 1 つ）になった。下の「既定 9821」はそれより前の記述。中継は同じ候補の並び（`areka_mcp::DEFAULT_PORTS`・`FALLBACK_STEPS`）と `AREKA_MCP_PORT` の意味に従うこと。areka の見つけ方は本 spec の要件で決める。

## Problem

- **Claude Desktop の利用者**が areka の MCP を使えない。Desktop の `claude_desktop_config.json` は `command` で起動する stdio のサーバしか書けず（`url` の欄が無い＝本体の検査関数で確認・survey §6）、コネクタ登録は Anthropic のクラウドからつなぎに来るので `127.0.0.1` へ届かない（公式の案内に明記）。
- 代用の `npx mcp-remote` は Node.js を入れさせる＝第三者に勧める形として重い。
- SSP は同じ理由で `data\mcp.exe`（stdio ⇔ HTTP の中継）を同梱している。

## Current State

- `mcp-server-core` が着地すれば、areka 本体は `127.0.0.1:<port>/api/mcp/v1` で HTTP を受ける（既定 9821・`AREKA_MCP_PORT`）。
- SSP の `mcp.exe` の実測（survey §6）: 引数なしで起動し、標準入力の 1 行 1 要求を `POST /api/mcp/v1`（`Host: 127.0.0.1:9801` 焼き込み）へ流し、応答を 1 行で標準出力へ返す。通知には何も返さない。JSON でない行には `-32700`・`id: null`。本体が居なければ「Server not available」「No response from server」、壊れた応答には「Invalid JSON response from server」（文字列で確認・振る舞いは未実測）。`Mcp-Method`／`Mcp-Name`／`MCP-Protocol-Version` のヘッダを付けて送る。

## Desired Outcome

- `areka-mcp-bridge.exe`（名前は要件で決める）を Claude Desktop の `mcpServers` に `command` として書けば、areka の MCP ツールが Desktop から使える。
- areka が起動していない・ポートが違う・応答が壊れている、のいずれでも Desktop 側に理由の分かる JSON-RPC のエラーが返り、中継は落ちない。
- `GET /api/mcp/help` に Desktop 用の設定例（中継の絶対パス入り）と HTTP 直結のコマンド例が並ぶ。
- 配布 zip に中継が入る。

## Approach

小さな bin クレートを 1 つ（外部依存なしを目標・JSON は `serde_json` を `mcp-server-core` と共有）。**rmcp も tokio も使わない**——中継は MCP の中身を作らず写すだけなので、本体の「MCP を自作しない」（2026-09-29 開発者判断＝rmcp 採用）には当たらない。写すだけで足りず MCP を解釈する必要が出たら（上の Boundary の問い）、そのときは rmcp の stdio と HTTP クライアントで組む。標準入力を 1 行ずつ読み、`127.0.0.1:<port>` へ HTTP/1.1 で投げ、応答本文を 1 行で書き出す。ポートは `AREKA_MCP_PORT`（Desktop の設定の `env` で渡せる）→ 既定 9821 の順（引数で渡すかは要件で決める）。

## Scope

- **In**: 中継の exe・ポートの決め方・本体不在と異常応答のエラー・通知の扱い・`/api/mcp/help` への Desktop 用の設定例・配布スクリプト（`tools/package.ps1`）への同梱・決定論テスト（ループバックの偽サーバで）・SSP の `mcp.exe` と同じ入力に同じ出力を返すことの突き合わせ（要件の段で 1 度）。
- **Out**: 本体側のプロトコル（`mcp-server-core`）・ツール（M2 以降）・Desktop の拡張機能（`.mcpb`）の包み。

## Boundary Candidates

- 中継（stdio ⇔ HTTP の写し）と、本体（MCP の中身）の境。中継は MCP の中身を解釈しない（写すだけ）かどうかを要件で決める——SSP の `mcp.exe` は `initialize`・`tools/list`・`server/discover` の文字列を持つ＝何か解釈している可能性がある。

## Out of Boundary

- 127.0.0.1 以外への中継（リモートの areka）。
- SSP の `mcp.exe` と同じ 9801 の焼き込み（areka は既定 9821）。

## Upstream / Downstream

- **Upstream**: `mcp-server-core`。
- **Downstream**: `.mcpb` の包み（未起票・欲しくなったら）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `mcp-server-core`（`/api/mcp/help` の文面）・完了 `alpha-package`（配布スクリプトの同梱物）。

## Constraints

- x64＋arm64（32bit は要らない）。Desktop が起動するプロセスなので、標準出力へログを出さない（ログは標準エラー＝Desktop の `mcp-server-<名>.log` に残る）。
- 規模 S。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 S（7〜10 タスク）。`mcp-tool-entrances` と同じウェーブには置かない（help の文面のファイルと `Cargo.lock` が重なりうる）＝C3 以降。
- 配布スクリプトは `tools/package.ps1`。同梱には 3 か所を変える＝`$ALLOWED_EXECUTABLES`・ビルドの段・配置と CPU 種別の検査。**`release-package-versioned`（C1）が同じスクリプトを先に直す**（名前を改め、版入りの zip・SHA256・arm64 の zip を足す）。配布の zip は x64 と arm64 の 2 つになる（arm64 の zip に中継を入れるかは本 spec の議題で決める）。
- **触るファイル**: 新規の bin クレート `crates/areka-mcp-bridge/**`（std::net で足りる）・`tools/package.ps1`・help の文面（`areka-mcp` の中）・必要なら `dist/README.txt`・`Cargo.lock`。
- **議題**: exe の名前／ポートを引数でも渡せるか／写すだけか版のヘッダを解釈するか／arm64 版を配布物に入れるか。

## 2026-10-03 C4 の候補（10-03 の再編（開発者「MCP は複合 spec なので早めに着手したい」））

- 段は「優先」。C3 に入れなかった理由: 依存の席を C3 の `animated-image-decode` が使い、`dist/README.txt` を C3 の winget と分け合う。

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S〜M（8〜12 タスク）。切らない（areka の見つけ方が 1〜2 タスク増えた）。
- 前提の状態: **満たす**。`mcp-server-core`（PR#219）は着地済み。台帳の前提に残る `package-check-temp-cleanup` は 10-02 に `release-package-versioned` へ合流し、その spec が着地済み（PR#218）＝前提は `mcp-server-core`・`release-package-versioned` の 2 本と読み替える（roadmap の台帳の行の前提の欄も同じく古い）。`release-ci-workflow`（PR#224）・`crates-io-publish`（PR#225）も着地済み。
- 崩れた前提／古くなった位置:
  - **既定の待受は 9801 が先**: `crates/areka-mcp/src/port.rs` の `DEFAULT_PORTS = [9801, 9821]`・`FALLBACK_STEPS = 9`＝候補は 9801・9821・9802・9822 … 9810・9830 の 20 個で、先に空いていた 1 つ。本文の「既定 9821」と Out of Boundary の「SSP と同じ 9801 の焼き込み（areka は既定 9821）」は古い。**SSP が 9801 を持っているとき、候補を順に試すだけの中継は SSP につながる**（下の穴）。
  - areka の名乗り: `initialize` の応答の `serverInfo.name` は `"areka-mcp-server"`（`crates/areka-mcp/src/handler.rs` の `SERVER_NAME`）。help の HTML の題は「areka MCP サーバの登録」（`help.rs`）。
  - 候補の定数を共有する手段: 中継が `areka-mcp` に依存すると rmcp・tokio・hyper を引き込む（`Cargo.lock` には既に在るので新しい外部依存にはならないが、exe が太る）。定数を写して、一致を検査で判定する手もある。
  - help の Desktop の節は今「中継は今後の版で用意します」の 1 文（`help.rs` の `help_html`・`help_tests.rs` は「Claude Desktop」と「中継」の 2 語だけを見る）。設定例に中継の**絶対パス**を入れるには、`help_html(port)` の引数に exe の置き場を足す＝呼び手 `crates/areka-mcp/src/dispatch.rs`（`help_html(port)` の 1 か所）と、その値を渡す `areka_mcp::start` の引数・`crates/areka/src/main.rs` の結線まで波及しうる。
  - 配布: `tools/package.ps1`（726 行・`release-package-versioned` の後）で同梱に変える所は ⑴ `$ALLOWED_EXECUTABLES` ⑵ 本体ビルドの段（`cargo build --locked --release -p areka --target …` の CPU 種別ごとの繰り返しに `-p <中継>` を足す）⑶「静的リンクの確認」の `$exes` の表 ⑷ `Test-ZipContent` の根の項目の一覧と機種の表（`$machines`）。`.github/workflows/release.yml` は `tools/package.ps1 -Arch all` を呼んで zip と `.sha256` を照らすだけで exe の一覧を持たない＝変える所 0 の見込み。
  - 新しいクレートはワークスペースに自動で入る（根の `members = ["crates/*"]`）＝根の `Cargo.toml` は 0 行・`Cargo.lock` にパッケージの行が 1 つ増える。`tools/crates-io.ps1` の判定により、新しいクレートの `Cargo.toml` に「`publish = false # 理由`」の行が要る。
  - `dist/README.txt`: 本体のクレジットの見出し「◆ areka 本体（areka.exe・shiori-host32-helper.exe）」に中継を足し、使い方（Desktop の設定）を 1 節。
- 触るファイル（並走の照合用）:
  - **新規** `crates/areka-mcp-bridge/**`（名前は要件で決める・bin・`Cargo.toml` に `publish = false # 理由`）
  - `Cargo.lock`（パッケージの行 1 つ・外部依存を足さなければ `THIRD-PARTY-NOTICES.md` は変わらない）
  - `crates/areka-mcp/src/help.rs`・`help_tests.rs`（Desktop の設定例）。絶対パスを入れるなら `crates/areka-mcp/src/dispatch.rs`・`server.rs`／`lib.rs`（`start` の引数）・`crates/areka/src/main.rs` の結線
  - `tools/package.ps1`・`dist/README.txt`
  - 他の spec の brief への申し送り: `release-code-signing`（署名する exe の一覧に中継を足す）・`winget-manifest-submission`（`NestedInstallerFiles` に中継を入れるか）
- 議題（答えで作業が変わるものだけ）:
  - areka の見つけ方: 候補を順に試して最初に応えた所へつなぐか、`initialize` の `serverInfo.name`（または help の題）で areka だと確かめてからつなぐか。前者だと SSP が 9801 を持っているとき SSP へつながる。
  - 候補の定数を `areka-mcp` への依存で共有するか、写して一致を検査するか。
  - help に中継の絶対パスを載せるか（載せるなら `mcp-server-core` の `start` の引数と `dispatch.rs` まで触る）。
  - （既存）exe の名前／ポートを引数でも渡せるか／arm64 の zip に入れるか／winget の `PortableCommandAlias` を付けるか。
- 見つけた穴: 本文が前提にする「既定 9821」は今の実装（9801 が先）と逆。素朴に候補を辿る中継は、SSP が起動していると黙って SSP の MCP につながり、Desktop の利用者は SSP のゴーストを操作していることに気付けない。
