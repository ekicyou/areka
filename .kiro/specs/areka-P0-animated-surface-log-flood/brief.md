# Brief: areka-P0-animated-surface-log-flood

> 2026-10-23 `areka-P0-pasta-in-windows-verification` のタスク 5.3 で `/kiro-discovery` により起票（本 spec からの申し送り（2026-10-23））。開発者の決め（同日のチャット）「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。版に縛らない列の `real-run-log-noise` へはまとめない（同日の開発者の決め）。版は **v0.1.0**。

## Problem

- **実機の走行の判定をする人と道具**: ぱすたの実機の走行の記録の 9 割が、同じ INFO の行 `apply(ShowSurface): 表示・マスクを更新` で埋まる。判定に要る行（起動の決め方・イベント・WARN）が埋もれ、読むのにも数えるのにも手間がかかる。動く絵を持つゴーストなら、どれでも同じことが起きる。

## Current State

- **証拠**: `verification.md` 5 節の崩れた所の表の **21**。本走行の記録 `target\pasta-verify\logs\run1.stdout.log`（6,546 行）のうち 6,073 行がこの行で、そのうち面 1002 の分が 5,989 行・毎秒十数行・どれも `cache_hit=false`。補いの走り `target\pasta-verify-alt\logs\alt2.stdout.log` でも 3,650 行。
- **原因**: 再描画の回数そのものは決まりどおり。面 1002 のボールの 3 枚（`window/ball/eback.webp`・`base_rings.webp`・`efront.webp`）が動く WebP（どれも `ANIM` の塊を持つ）で、コマが替わるたびに合成し直す（完了 `areka-P0-animated-image-playback`）。記録の量は、適用 1 回ごとに INFO を 1 行出すこと（`crates/areka-emo-present/src/presenter/show.rs:402`・426 行のファイル）。
- 同じ適用の対として、すぐ下で perf の行（`timing.emit`）も 1 行出る（`present-emit-tail-latency` が段を足す行）。
- この行は観測点としても使われている: 同じ行の `size_changed` は窓寸の状態照合の観測点（同じファイルの注記）。`crates/areka/examples/`（`collision-probe.rs`・`emo-present*.rs`・`window-placement.rs`）・`crates/areka-emo-present/src/{presenter/hub.rs, presenter/timing.rs, presenter_film_tests.rs, presenter_perf_log_tests.rs}` が行の文言に触れ、完了 spec の実機の記録（`collision-dpi-hittest`・`default-balloon-bundle`・`element-base-method`・`mcp-tool-entrances` など）がこの行を grep している。
- **SSP の基準**: 記録の量は SSP に対応する物が無い（SSP は areka の記録を出さない）。基準は「判定に要る行が埋もれない」ことと、areka の決まり（記録の無い失敗の道を作らない・状態が変わったときは記録する）。

## Desired Outcome

- 動く絵のコマ送りだけで起きる適用は、既定の記録の水準（`RUST_LOG` の手順書の値）で毎回は出ない。面の替わり・窓寸の替わり（`size_changed`）・初めての表示・失敗は、今と同じに出る。
- 動く絵の適用の回数そのもの（決まりどおりの再描画）は変えない。
- 観測点として使う人（例・テスト・手順書）が、変えた後も同じ事実を読める。

## Approach

- 案は要件の段で決める: ⒜ コマ送りの適用（面も窓寸も変わらない適用）だけ `debug!`／`trace!` へ下げる ⒝ 同じ面のコマ送りを一定の間まとめて 1 行にする ⒞ 状態が変わったときだけ INFO。areka の決まり「ログ無しの失敗の道を作らない」は失敗の道の話で、成功のコマ送りを下げても触れない。

## Scope

- **In**: `show.rs` の INFO 行の出し方・それに触れるテストと例の追随・手順書 `doc/pasta-real-run.md` の `RUST_LOG` と数え方の見直し・ぱすたの走行の記録の量の確かめ（面 1002 の分が既定の水準で消えること）。
- **Out**: 動く絵の適用の回数を減らすこと（決まりどおり・`present-emit-tail-latency` も範囲の外にしている）・perf の行（`present-emit-tail-latency`）・文字の層の毎フレームの `debug!` と同梱の emo2 の WARN（`real-run-log-noise`）。

## Boundary Candidates

- emo-present の表示の段の記録（`presenter/show.rs`）

## Out of Boundary

- 適用の中身と時間（`present-emit-tail-latency`）
- 記録の仕組みそのもの（`tracing` の設定）

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手できる）。
- **Downstream**: `pasta-in-windows-verification` の回し直し・SSP 並みの自動試験（記録を grep で判定する道具）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `real-run-log-noise`（同じ「記録の量」の話だが場所が別・まとめない）・`present-emit-tail-latency`・`mcp-strict-errors`（どちらも `show.rs` に触る＝同じウェーブに置かない）。

## Constraints

- 完了 spec の正典（`animated-image-playback` の再描画の決まり）は変えない。
- 行の文言を変えるなら、grep する側（テスト・例・手順書）を同じ変更で追随させる。

## 起票時の見立て（2026-10-23・ブランチ `claude/areka-p0-pasta-windows-d4ed7a`）

- 触るファイル: `crates/areka-emo-present/src/presenter/show.rs` と兄弟のテスト（`presenter_perf_log_tests.rs` ほか）・必要なら `crates/areka/examples/` の 4 本・`doc/pasta-real-run.md`。
- 規模: S（2〜4）。
- 優先度の区分: A（開発者の指示）。段: その他。
- 要件定義のモデル: Opus。
- 議題: 上の 3 案のどれにするか。
