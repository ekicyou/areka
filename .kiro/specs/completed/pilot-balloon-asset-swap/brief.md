# Brief: pilot-balloon-asset-swap（先進坑・使い捨て）

> 2026-09-24 `/kiro-discovery` 再入（棚卸⑯）で起票（台帳 #59）。`areka-P0-shell-balloon-switch`（台帳 #50）の brief が「要件の前に先進坑を 1 本掘る」と推していたものを、`two-tunnel.md` の規約（`pilot-` 接頭辞・1 仕様＝1 フォルダ・go 判定は開発者・go 前の本坑着手は規律違反）に沿って独立の spec にした。成果物はコードではなく**知見**（go／違う／直す＋学び）。一次記録は `crates/pilot/examples/pilot-balloon-asset-swap/README.md`（3 幕）。
> 対応する本坑: `areka-P0-shell-balloon-switch`（roadmap に `_Depends(confirmed): pilot-balloon-asset-swap`）。

## Problem（何が怪しくて掘るのか）

走っているゴーストの中で、絵と文字の出し先を差し替える語彙が 1 つも無い（2026-09-24 再測定＝`reload|ReplaceTarget|Rebuild|swap_sink|replace_sinks` は seriko・present・text・dispatcher で 0 件）。出し先は起動時に `GhostBootOptions.sinks` として値で渡され固定される。本坑 #50 には 2 つの案があり、**どちらが安いかはコードを書いてみないと分からない**:

- **案 A**: ゴースト切替（#13）が作る「降ろして起こし直す」一般形を、SHIORI だけ残して回す。
- **案 B**: seriko・present・text に「資産を差し替えろ」の語を 1 つずつ足す。

案 B の鍵は `EmoPresenter::attach_target`（`crates/areka-emo-present/src/presenter/hub.rs`）——同じ id を再登録すると表示コンテキストごと置き換える。これが差し替えの最小の部品になり得るが、**差し替えの途中の 1 フレームに古い絵と新しい当たり判定が混ざらないか・空の窓が点滅しないか**はコードからは出ない（記憶 no-frame-delay-fixes-change-the-state-shape＝1 フレーム遅らせて辻褄を合わせる解は取らない）。

## Desired Outcome（確かめたい 1 点）

**SHIORI を生かしたまま、present のバルーン資産だけを差し替えて、1 フレームも崩れずに表示が続くか。**

合否基準（go／違う／直す）:
- **go**: 差し替えの前後で、表示中の窓が消えない・古い絵が残らない・当たり判定が新しい絵と同じフレームで切り替わる（readback で前後のフレームを取り、混在フレームが 0）。
- **直す**: 混在フレームが出るが、差し替えの順序（当たり判定 → 絵、または絵 → 当たり判定を同じフレームで）で消せる。
- **違う**: `attach_target` の置き換えでは崩れが消えず、案 A（降ろして起こし直す）へ倒す。

## Approach

- `crates/pilot/examples/_template/` を `crates/pilot/examples/pilot-balloon-asset-swap/` へ写して着手。
- `crates/areka` は bin だけの crate なので `build_boot_assets`／`wire_emo2_boot` は呼べない。ライブラリ crate の部品（`areka-emo-present`・`areka-emo-compose`・`areka-emo-atlas`・`areka-emo-text`・`areka-parsers`・`wintf`）から組み直す。手本は `crates/areka/examples/emo-present.rs` とそのフォルダ（計 1,271 行）。
- 検体は `sample_ghost_kit`（既に pilot の dev-dependency）で `StayseeBalloon` と `emo2` 同梱の `emo2-kakukaku` を引き、往復する。
- 観測は readback（記憶 areka-gpu-window-screenshot-readback）で差し替え前後の数フレームを取り、混在の有無を数える。目視は補助。

## Scope

- **In**: 上の 1 点の実験・README 3 幕（動機・概要・検証結果）・`crates/pilot/Cargo.toml` の dev-dependencies への追加（pilot 側が依存するだけ＝葉ノードの隔離は崩れない）
- **Out**: シェルの差し替え（バルーンで答えが出れば同じ形・出なければ本坑で別に考える）／seriko・text への語彙の追加（本坑）／本体への接続／先進坑コードの production 流用（コピペ donor 禁止・本坑はクリーンに掘り直す）

## Boundary Candidates

- 差し替えの部品（`attach_target` の置き換え）
- 観測（readback で混在フレームを数える）

## Out of Boundary

- 本坑 #50 の設計（README の検証結果を参照し二重化しない・`two-tunnel.md` 要件 3.5）

## Upstream / Downstream

- **Upstream**: なし（`crates/pilot/` と `crates/pilot/Cargo.toml` だけを触る＝**#55・#58・#13 と共有ファイル 0・並走できる**）。
- **Downstream**: `areka-P0-shell-balloon-switch`（go 判定が前提依存）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: 完了 `pilot-clickthrough-alpha-toggle`（先進坑の器の前例・`REPORT.md` の書式）・完了 `areka-P0-present-gpu-transform-scale`（atlas／compose／present の直列 3 分割の上に乗る）。

## Constraints

- 規模 **S**（タスク 3〜5 本）。段は **α**（#50 の go ゲート）。
- **要件と設計は Opus で足りる**（掘る対象は 1 点・答えは実験が出す）。go 判定は開発者（人間）が README を見て下す。Claude Code 単独で go を宣言しない。
- 実験は有界（長時間試行禁止）。`AREKA_APP_SMOKE_EXIT_MS` 相当の自動終了を example にも入れる。
- 品質は使い捨て水準でよいが、観測の較正（既知の「混在させた」フレームで数え方が赤を出すこと）は入れる（記憶 subagent-tooling-can-be-wrong-calibrate-it）。
