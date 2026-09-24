# Brief: areka-P0-dpi-transition-two-tick-bounce

> **起票 2026-09-11**（`/kiro-discovery`・開発者「起票候補はすべて起票せよ」）。出所: `areka-P0-emo2-conformance-e2e` 走行 D（2026-09-10 23:26〜23:36・拡大率 200%↔150% を 4 往復）。登記: 同 spec `verification/acceptance-record.md` §13.1 行 1（判定に載せない・**引受先なし**のままだった）・§9／§12（読み分け 3 行目）。開発者裁定（2026-09-10）「DPI 切替は頻繁に起こらないため許容」＝M1 の判定には載せない。本 spec はその「引受先なし」を埋める。

## 2026-09-20 棚卸⑮の再測定

**実測（main `fe157df1`）**

- `run_dpi_phase`（`crates/areka/src/emo2_boot/frame/dpi.rs`）と `refresh_scale`（`crates/areka-emo-present/src/presenter/refresh.rs`）は行番号のずれ 0。上流の `present-gpu-transform-scale` は完了したが、**跳ねが残るかは測り直していない**。
- **2026-09-24: 判定器の穴は spec を立てずには直せないと分かった**（roadmap「直接修正候補」の注記＝既存テスト 3 本が、見送られたバルーンの書込 2 回も違反として数えることを固定している。要件 4.5 と 4.6 のどちらを優先するかの裁定待ち）。以下は当時の記述。
- **判定器の穴は、spec を立てずに直す**（roadmap「直接修正候補」）。`crates/areka/src/placement/transition_judge_verdict.rs` の窓ごとの書込の上限（要件 4.5）は `summary.writes_per_window` をそのまま回し、見送りの窓を除いていない。ところが同じファイルに「見送りの窓を除いた、書込のあった窓」を返す `judged_windows` が**既に在り**、被覆の検査だけが使っている。直しは「上限の検査でも `judged_windows` を回す」＋兄弟テスト 1 本で、`dpi.rs` には触らない。
- 残る本体（2 ティックの跳ねの相の順）は開発者裁定「拡大率の切替は頻繁に起こらないため許容」のまま据え置く。

## Problem

拡大率が変わったとき、**最初のティックで窓の位置が動き、次のティックでシェルの寸法が直る**。150%→200% では二体が一瞬跳ねて見える。機械判定（`transition_signoff`）の実機専用系統は 8 遷移すべてで「可視化から書込まで 100〜267 ms ＞ 上限 16.7 ms」「一括書込の総所要 64〜202 ms ＞ 16.7 ms」で FAIL（決定論系統は PASS＝フレーム単位の量は満たしている）。

## Current State

- 遷移の流れ: `Changed<DPI>` → `run_dpi_phase`（`crates/areka/src/emo2_boot/frame/dpi.rs:500`）→ `refresh_scale`（`crates/areka-emo-present/src/presenter/refresh.rs:58`・保持した最終 show 入力で再表示）→ 窓寸の reconcile（char＝`resize_window_to`／balloon＝`resize_window_keep_position`）→ 一括書込（`enqueue_window_set_pos` の flush）。完了 spec `areka-P0-dpi-transition-atomicity` の D15「一度書き」は「同一 tick で位置と寸法を 1 回で書く」を保証したが、**可視化（絵が新しい寸法で描かれる）から書込までの遅れ**は上流が「見送り＋登記」で閉じた（§13.1 行 1 の由来）。
- 2026-09-10 の実測: 可視化→書込 100〜267 ms。うち大半は present 段の CPU 拡大（`areka-P0-present-gpu-transform-scale` の対象）で、**それが消えた後にも 2 ティックの順序（位置→寸法）が残るかは未確認**。

## Desired Outcome

1. 拡大率の切替で、位置と寸法が**同じ画面更新**で変わる（目視で跳ねない）。
2. 機械判定の実機専用系統（`visualize_to_write_us` ≤ 16,667・`flush_total_us` ≤ 16,667）が静かな機械で全遷移 PASS。上限は動かさない（e2e R6.2／6.3）。
3. 判定器の既知の限界（見送り窓を `writes_per_window` から除いていない・e2e 手順書 §6.2）を直し、非表示中のバルーンの追従書込が違反に数えられないようにする。

## Approach

- 先に `present-gpu-transform-scale` を着地させて可視化→書込の遅れから CPU 拡大の分を除き、残った遅れと 2 ティック構造を走行 D と同じ採取で再計測する。残るなら、`run_dpi_phase` で「絵の準備」と「窓の書込」を同一 tick に揃える（寸法の確定を `refresh_scale` の報告と同じ tick で行う）。
- 判定器の見送り窓の扱いは `crates/areka/src/placement/transition_judge_verdict.rs:466-477` の 1 か所（`skipped_windows` を除く）。

## Scope

- **In**: `crates/areka/src/emo2_boot/frame/dpi.rs`・`crates/areka/src/placement/{follow, transition_judge*}`・決定論の檻・走行 D 形式の実機採取。
- **Out**: present 段の拡大方式（別 spec）・当たり判定・配置の既定位置。

## Boundary Candidates

- 遷移の相順（frame）／判定器の見送り窓（judge）。2 相を 1 spec・M。

## Out of Boundary

- CPU 拡大の撤去（`present-gpu-transform-scale`）。

## Upstream / Downstream

- **Upstream**: `areka-P0-present-gpu-transform-scale`（先に着地）・完了 spec `areka-P0-dpi-transition-atomicity`（D15・判定器）・`areka-P0-emo2-conformance-e2e`（走行 D の記録）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: `dpi-transition-atomicity`（判定器の限界の是正・上限は不変）。
- **Adjacent**: `zorder-chain-residue`・`window-placement`。

## Constraints

- 上限値・量の名前・判定器の意味を目視に合わせて緩めない（e2e R6.2／6.3・手順書 §6.5）。
- 開発者方針「長時間試行禁止」——採取は走行 D の形（10 分・3 往復）に限る。

## 申し送り

- `present-gpu-transform-scale` の着地で外れの代金が消え、drain 相が `Update` へ移って表示・窓書込・文字が同一 tick に揃ったので、走行 D 形式で再計測してから設計する（2026-09-12）。
