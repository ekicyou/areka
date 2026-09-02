# Brief: areka-P0-tick-gate-adoption

> **起票 2026-08-27（棚卸⑪・Path C）**: `draw-load-parity`（W6.9・PR#118・2026-08-23 完了）が requirements.md 改訂欄に「引受先＝なし → 新規 spec が要る」と登記して閉じた案件の受け皿。**M1 完成を妨げない優先度（dlp と同格・開発者裁定 2026-08-22 の継承）**——解禁条件は下記「着手ゲート」参照。

## Problem

常駐アイドル時の CPU 消費が SSP 同等圏（目標 3.0% 未満）に達していない。`draw-load-parity` の最終判定は **release 定常平均 22.3%**（baseline 15.8%・自走ループ 3 周は全て不採用で頭打ち STOPPED）。未達の判定式＝⑵ catch-up・⑶ 表示用バッファ新規確保・⑷a アイドル CPU。

**合否の外で唯一目標近傍に届いた手がかりが「tick の門」**（`AREKA_TICK_GATE`・既定 OFF）である: 門 ON＋点灯 7 分の定常 CPU **3.30%**（p50 3.11・tick の 87.6% を省略）対 門 OFF **17.04%**＝約 5 分の 1。しかし dlp の自走ループでは**測定側の分解能不足で採用に至らなかった**（非点灯 A/B 7 分では A 自身が 6.3〜27.5% に散らばり Δ が埋没・副指標 count 規則が n=2 で 1 件差を悪化と読む・非昇格のため段③〔関数別帰属〕が全周 UNAVAILABLE）。

## Current State

- **門は実装済み・既定 OFF**: `crates/wintf/src/ecs/world/tick_gate.rs`（機構）＋`crates/areka/src/tick_gate_config.rs`（設定・決定論テスト同居）。dlp は `Cargo.toml` 非接触・**実行時の挙動は着手前と同じ**で閉じた。
- **計測基盤は完備**: スレッド名簿＋4 段の帰属（過程/スレッド/関数/相）・tick の相別観測・`tools/perf/perf-loop.ps1`（rank／compare）・エージェント 4 本（`.claude/agents/perf-{analyze,implement,measure,review}.md`）・スキル `perf-loop-iteration`・台帳駆動（`loop-ledger.md`）。証跡は `completed/areka-P0-draw-load-parity/results/`（baseline-20260823・iter-1..3・final-20260823・summary.md）。
- **是正すべきは測定側**（dlp 改訂欄 2026-08-23 が次 spec の要件候補として登記）: ⑴ 7 分×2 の A/B は本機の日中では分解能不足（**夜間・25 分・n≥3 のいずれかが要る**）⑵ 副指標 count 規則は n=2 で 1 件差を悪化と読む（catchup 16.5→19・allocs 0→1/5 で WORSE 判定）⑶ 昇格セッションで段③を採る。
- **未計測のまま閉じた項目**（同改訂欄 補記）: 発話中の頂（dlp 5.4・SSP 参考値 4.64）・Private メモリ/ハンドル/スレッド数（5.5）・catch-up 系統別突合の点灯走行（2.9＝「フレーム駆動の負荷が起床を遅らせる」仮説は `[tick] kind=window` 行なしで全走行判定不能）・dev の「単調上昇（未知機序候補）」。
- **残る最大項**（周 3 順位表・HEAD）: 段② `unregistered_rest`（名簿外）51.8%・次点 ui 44.2%・段④ framefinalize 34.0%／draw 22.3%。

## Desired Outcome

1. **門の本採用の可否が、十分な分解能の実測で確定している**——夜間または 25 分以上・n≥3 の A/B で、門 ON の CPU 削減と副作用（catch-up・ドラッグ・アニメ・入力応答）の有無が統計的に判読できる形。
2. 採用時: 既定 ON（または点灯条件の常時成立）で **⑷a アイドル CPU 3.0% 未満**を release 実機で満たし、catch-up 規則の再定義（省略した tick の追走意味論）が決定論テストで固定されている。
3. 非採用時: 却下根拠（副作用の実測）が登記され、残る最大項（段② 名簿外 51.8%）の次の一手が導出されている。
4. 段③（関数別帰属）が昇格セッションで一度は実採取され、順位表が 4 段とも埋まる。

## Approach

dlp が建てた自走ループ（perf-loop）を**測定側 3 是正を先に済ませてから**再入する。1 周 1 変更・台帳駆動・Cargo.toml の扱いは要件で再裁定（dlp の非接触制約は「挙動不変で閉じる」ための制約だったため、本採用 spec では既定変更が本務＝制約の形が変わる）。

## Scope

- **In**: 門の本採用裁定（A/B 設計の作り直し＝夜間/長時間/n≥3）・catch-up 規則の再定義・count 規則の是正・段③昇格・採用時の既定 ON 化と決定論テスト・dlp 未計測 4 項目の採取。
- **Out**: 門以外の新規最適化手段の探索（段② 名簿外の解体は「次の一手の導出」まで＝実装は別途）・SSP 側の再採取（開発者裁定 2026-08-22＝調べない）・描画方式の変更。

## Boundary Candidates

- 測定側是正（perf-loop 道具の改修）と製品側変更（門の既定 ON）は独立に検証可能な 2 群。
- catch-up 規則の再定義は `tick_gate.rs`／`tick_gate_config.rs` 内で閉じる（kanade/dola の絶対時刻台本は門の省略と独立＝dlp 設計の帰結を継承）。

## Out of Boundary

- `presenter/show.rs`（`apply_show` は CPU の 3.3%＝dlp が対象外と確定済み・pwc の領分）。
- クリック透過・αマスク追随の「毎フレーム評価」前提そのものの改廃（調停は dlp 設計が済ませた範囲を継承し、逸脱が要るなら要件で再裁定）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-draw-load-parity`（門・計測基盤・台帳・改訂欄が申し送り正本）・`completed/areka-P0-dpi-transition-atomicity`（`wintf::transition` 観測チャネル）・`completed/areka-P0-recompose-budget`（判定式の系譜）。
- **Downstream**: なし（M1 完成宣言は本 spec に依存しない）。

## Existing Spec Touchpoints

- **Extends**: なし（新規境界）。
- **Adjacent**: `present-write-coherence`（W6.95・提示タイミングの軸＝pwc が tick 実形の上で規模を見積もる関係は dlp⇄pwc 台帳の継承。**本 spec は pwc より後**——同じ理由で直列）。`emo2-conformance-e2e`（W7・一周走行と干渉するため並走不可）。

## Constraints

- **着手ゲート**: M1 完成（e2e 完走）後、または開発者の明示裁定による前倒し。**夜間または 25 分以上の実機走行を n≥3 回せる環境**が実測の前提（日中 7 分では分解能不足が dlp で実証済み）。
- 実機判定は areka 実ゴースト（emo2）＋実 DPI・有界 auto-exit＋ログ grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。
- 1 ファイル 1,000 行の目安・兄弟テスト配置・ログ捕捉は `log-capture-kit`／一時パスは `temp-path-kit`（cage 着地形）に従う。

---

> **📌 2026-09-02 棚卸⑫**——アンカー **ドリフト 0**（`tick_gate.rs`:154/:53/:58・`tick_gate_config.rs:25`・`tools/perf/perf-loop.ps1`・agents 4 本・dlp `results/` 全実在）。ウェーブ番号整数化＝本文の W6.9→**W10**・W6.95→**W11**（roadmap 冒頭対応表）。編成＝**M1 完成後・単独**（e2e と並走不可）。⚠ **開発者方針「長時間試行禁止」**（zsp の 4,440 走行の教訓・記憶 areka-p0-scope-zorder-pinning）と本 spec の走行時間要求（夜間/25 分/n≥3）が正面衝突する——要件段階で「始める前に決着可能な A/B 設計」を先に組むこと。分割シーム＝測定側是正 ⇄ 製品側変更（brief 記載どおり）。zsp 残件 B-3（生産者名簿の穴・`tick_gate_tests.rs`／`tick_gate_config_producers_tests.rs`）は `zorder-chain-residue` が持つ（本 spec は名簿を読む側）。

