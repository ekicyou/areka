# draw-load-parity — 改善ループの結果まとめ

- 台帳: `loop-ledger.md`（周 3・記録 3 件）
- 相: FINAL
- 開始: 2026-08-22T23:41:05Z
- 走行トークン: 87696907

## brief 旧数値との対比

出所: .kiro/specs/completed/areka-P0-draw-load-parity/brief.md `## Problem`（2026-08-15 実測）

| 指標 | brief 旧数値（areka） | 参考（SSP） | ベースライン | 最良 | 最終 |
|---|---:|---:|---:|---:|---:|
| アイドル CPU 平均（1 コア換算 %） | 10.97 | 3.05 | 15.80 | 15.80 | 22.3（25 分最終判定・final-20260823） |
| CPU の底（アイドル・%） | 3.60 | 1.77 | - | - | - |
| CPU の頂（発話中・%） | 20.42 | 4.64 | - | - | - |
| Private メモリ（MB） | 163.4 | 54.2 | - | - | - |
| スレッド数 | 83 | 32 | - | - | - |
| 定常 catch-up 件数（release・短時間） | 17 | - | - | - | - |
| 定常 catch-up 件数（release・25 分） | 69 | - | - | - | - |

注:
- 表示 1 コマの適用経路は先行 spec `areka-P0-recompose-budget` が 22,210µs → 1,240µs（18 分の 1）まで削ったが、アイドル CPU の 3.3% しか占めておらず中央値は 9.3% のまま動かなかった。
- 残る主役はフレーム駆動そのもの——ECS の tick が毎秒 120 回・1 回あたり約 578µs で 13 本のスケジュールを全部回し、その 98% は表示に変化が無い（上位 2 本 FrameFinalize 182µs・Draw 143µs で 56%）。
- SSP 列は参考値であり合否には使わない（要件 5.2）。

## 周ごとの採否

| 周 | 日時 | 仮説 | 採否 | 前 | 後 | 差 | ばらつき | コミット |
|---:|---|---|---|---:|---:|---:|---:|---|
| 1 | 2026-08-23T04:39:30Z | tick gate default ON: try_tick_world が 13 スケジュールを 120 回/秒 全部回す（tick の 98% は表示に変化なし）。起床旗の無い tick を門で省けば UI スレッド（段② 48.91%）の定常 CPU が下がる。周 1 は仕組みの A/B（順位表からの選択は周 2 以降・tasks 9.2） | FOLLOWUP_FAIL | - | - | - | - | - |
| 2 | 2026-08-23T06:31:33Z | tick gate default ON（再 A/B・ドラッグ穴を塞いで）: UI スレッド 56% の中身は 13 本を毎コマ全部回す tick（skip 0%・119.58 回/秒）。周 1 の drag FAIL の原因＝起床旗の生産者が DraggingState 成分に依存し、権威状態のスレッド局所 DragState（Preparing/JustStarted/JustEnded）を代表していない。旗を状態機械側へ寄せてから門 ON を再 A/B する | WORSE | 9.79 | 6.12 | -3.67 | 6.99 | - |
| 3 | 2026-08-23T08:36:33Z | C17 単スレッド実行器: 名簿外 51.8%（5.859 cpu_s/60s）は tick 駆動の ComputeTaskPool ワーカー（門 ON の残置実行体では tick 省略 87.6% に比例して 0.578 cpu_s へ落ちる＝tick 由来）。既定の多スレッド実行器のままの 7 本（Input/Update/PreLayout/Layout/PostLayout/Draw/FrameFinalize）を SingleThreadedExecutor へ寄せ、120 回/秒×7 回のワーカー起床・待機（1 tick 当たりワーカー側 815µs 対 UI 自身 695µs）を消す | WORSE | 21.10 | 8.91 | -12.19 | 12.80 | - |
