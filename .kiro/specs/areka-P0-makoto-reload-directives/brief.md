# Brief: areka-P0-makoto-reload-directives

> 2026-10-04 棚卸㉑で `areka-P0-makoto-dll-host` から切り出し（開発者「負荷が高すぎる仕様は分割を検討せよ」）。元の brief の Boundary Candidates ⓒ（付け外し命令 3 種）と、棚卸⑳で範囲に入った「シェルの切替と更新の後の付け直し」を持つ。

## Problem

ゴーストの作者は `\![unload,makoto]`・`\![load,makoto]`・`\![reload,makoto]`（SSP 2.5.58・ukadoc `list_sakura_script`）で MAKOTO の DLL を付け外しする。またシェル側の MAKOTO（`shell/<名>/makoto.dll`）はシェルごとに違う。`makoto-dll-host` は起動時に鎖を組むだけなので、命令は効かず、シェルを切り替えたり更新で読み直したりすると古いシェルの MAKOTO が残る（または外れる）。

## Current State

- 鎖の差し込み口は `crates/areka-kanade/src/translate.rs` の `MakotoChain`（`Box<dyn Fn(&str, &str) -> String + Send>`）。kanade の起動時に 1 回渡され（本番の結線は `crates/areka-ghost/src/runtime.rs`）、中身を変えられず、差し替える口も無い。
- シェルの切替は kanade を作り直さない（`crates/areka/src/emo2_boot/shell_balloon_switch.rs`）。更新の読み直しは `crates/areka/src/update/procedure.rs`。
- `\!` の消費者の台帳は `crates/areka/src/emo2_boot/consumer_ledger.rs`（選び手つきの登記）。`load`／`unload`／`reload` の消費者は 0。
- `mcp-reload`（α 後・未着手）が `\![reload,…]` と MCP の `reload` を作り、`makoto` だけ `NG:` を返す縮退の口を置く約束（`makoto-dll-host` の brief の 2026-09-29 の申し送り）。

## Desired Outcome

- `\![unload,makoto]` で鎖の全 DLL を降ろして以後は素通し・`\![load,makoto]` で再び載せる（載っていれば何もしない）・`\![reload,makoto]` で降ろして載せる。命令はゴースト側・シェル側の両方に効く。
- シェルを切り替えたら、新しいシェルの `makoto,` で鎖のシェル側を付け直す。更新で読み直したら、鎖を組み直す。
- MCP の `reload` の `target: "makoto"` も同じ操作へつなぐ。
- 決定論のテストで、状態の移り変わり（載っている／降りている・冪等）と付け直しを固定する。

## Approach

`MakotoChain` の閉包の中に共有の入れ物（鎖の状態）を持たせ、命令と付け直しはその入れ物を差し替える（kanade の型は変えない）。命令は cue の受け口から UI のスレッドを塞がずに届ける（ゴースト側へ届く受け口の型を決める）。

## Scope

- **In**: 命令 3 種と消費者の台帳 3 行・鎖の入れ物・シェルの切替と更新の後の付け直し・`mcp-reload` の口との接続・決定論のテスト・網羅台帳 3 行・COMPAT §8（命令は鎖全体に効く）。
- **Out**: DLL のホスティング・codec・descript・起動時の鎖（`makoto-dll-host`）・`\![reload,shiori]` など他の reload（`mcp-reload`）。

## Boundary Candidates

- 受け口（命令）／鎖の入れ物（状態）／付け直し（切替・更新）。

## Out of Boundary

- helper のライフサイクルそのもの・MAKOTO の通信（`makoto-dll-host`）。

## Upstream / Downstream

- **Upstream**: `makoto-dll-host`（鎖と helper）・`mcp-reload`（どちらが先でも後着がつなぐ）・完了 `shell-balloon-switch`・完了 `network-update`。
- **Downstream**: MAKOTO を付け外しする実ゴースト（YAYA as MAKOTO）の適合。

## Existing Spec Touchpoints

- **Extends**: `makoto-dll-host`（分割元）。
- **Adjacent**: `mcp-reload`（同じ命令の口）・`network-update-canon-order`（`update/procedure.rs`）。

## Constraints

- ログ無し失敗経路の禁止（読み込み失敗は `error!` で、翻訳なしで続ける）。決定論のテスト必達・x64 の偽の境界。
- `emo2_boot` の切替系（`shell_balloon_switch.rs`・`ghost_switch.rs`）を触る spec と同時に走らせない。

## 2026-10-04 棚卸㉑で切り出し

- 元の spec: `makoto-dll-host`（⒝）。
- 規模: M（7〜10 タスク）。
- 前提: `makoto-dll-host`（未）。`mcp-reload` は前でも後でもよい。
- 触るファイル: `crates/areka/src/emo2_boot/consumer_ledger.rs`＋新規の受け口・`emo2_boot/mod.rs`・`emo2_boot/shell_balloon_switch.rs`・`crates/areka/src/update/procedure.rs`・`crates/areka/src/mcp/reload.rs`・`crates/areka-ghost/src/makoto_wiring.rs`（`makoto-dll-host` が作る）・`doc/ukadoc-coverage/ledger/sakura-script.toml`（3 行）。
- 共有しうる相手: `mcp-reload`・`network-update-canon-order`・`emo2_boot` の切替系を触る spec。
- 議題: なし（命令が鎖の両側に効くのは元の brief の推奨のまま）。
