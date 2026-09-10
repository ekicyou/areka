# Brief: areka-P0-present-resample-budget

> **起票 2026-09-07（`areka-P0-emo2-conformance-e2e` の実機走行で開発者が「描画が異常に重い・ガクガク・遅延が目に見える」と所見・SSP 横並びで比較・開発者裁定「E は別 spec にする」）**。上流の登記: `.kiro/specs/areka-P0-emo2-conformance-e2e/requirements.md`「改訂（2026-09-07・第 4 回）」2・同 `verification/acceptance-record.md` §13.2（症状 E）。**M1 の完成判定（e2e）は本 spec を待たない**（症状 E は「判定に載せず登記」＝R7 の扱い・開発者裁定）。着手時に本 brief の数値を実測で引き直すこと。

## Problem

拡大率 200%（k=2）で、表示する絵が変わるたびに **60〜80 ms** の CPU 処理が UI スレッドに乗る。まばたきは 3〜4 コマが 60 ms 間隔で連続するので、コマごとの処理がコマ間隔を超えて「ガクガク」になる。起動挨拶は表情の切替・着せ替え・まばたきが重なって特に不安定、定常運転に入ると安定する（開発者所見と一致）。同じ機械で SSP は遅延しない（DPI 非対応のまま DWM に拡大を任せている）。

## Current State

- 1 コマの表示 `apply_show`（`crates/areka-emo-present/src/presenter/show.rs:46`）は、キャッシュ（`crates/areka-emo-present/src/cache.rs`・**容量 3・LRU**・完了 spec の裁定＝実走行の再生で命中率 56%）に当たれば **0.4 ms**、外れると **compose 5〜7 ms ＋ resample 44〜61 ms ＋ mask 11 ms ＋ upload 0.3 ms ＝ 61〜84 ms**（`perf(apply_show)` の段階別計時・2026-09-07 の 3 走行・800×448 → 1600×896）。
- 外れの割合: 18:30 の走行 163/298（55%）・20:01 の走行 78/104（75%）・SSP 横並び 42/56（75%）。本体側は同じ面 1000 でも「まばたきの絵柄×着せ替え」の組み合わせごとに別エントリで、容量 3 に収まらない。
- resample は `crates/areka-emo-compose/src/scale.rs:481`（整数専用の bilinear・行ごとの x 写像表・`resample_with`）。1.4 Mpx の出力に 44〜61 ms ＝ 30〜40 ns/px（スカラ・4 チャンネル個別）。mask は `regenerate_from_pbgra32`（物理寸で α を詰め直す・11 ms）。
- 機械負荷（別プロジェクトのジョブ）は外れ 1 回を 61 → 84 ms に押し上げる程度（約 1.35 倍）で主因ではない。
- 完了 spec `areka-P0-emo-dpi-scaling` の設計 D3 は **Strategy A2（合成は native・提示段で k 倍リサンプルした表示用サーフェスをキャッシュ）** を選び、**B＝WUC transform（GPU 拡大）をマスク不整合（W5 境界侵食）と鮮明性欠如で却下**している（`.kiro/specs/completed/areka-P0-emo-dpi-scaling/design.md:110`・`:169`）。`recompose-budget`（PR#112）は compose を 22,210 → 1,240 µs にしたが resample／mask は対象外。

## Desired Outcome

1. k=2 で、絵が変わるコマ（キャッシュ外れ）の処理が **1 コマの間隔（まばたきの 60 ms）を超えない**——目標は外れ 1 回 ≤ 16 ms（60 Hz 1 フレーム）・許容 ≤ 30 ms。実走行の `apply_show` 段階別計時で示す。
2. 起動挨拶（表情切替＋着せ替え＋まばたき）の間、ティッカーの `catch-up`（`areka_ghost::ticker`）が前回並み（3 分で ≤ 10 件）に収まる。
3. k=1 の byte 等価 golden は不変。k≠1 の出力は、決定論（同一入力→同一バイト）を保つ。A2 を保つ限り既存の k 付き golden も不変で、A2 を替えるなら裁定と再導出。
4. 判断の根拠は実測（`perf(apply_show)`・`t_resample_us`／`t_mask_us`）で、本リポジトリの perf-loop（`.claude/skills/perf-loop-iteration`・`perf-analyze`／`perf-measure`／`perf-implement`／`perf-review`）で回す。

## Approach

**候補（実測で決める・順に安い）**:

- **ⓐ 外れの代金を下げる（A2 のまま）**: ⑴ 整数倍（k=2 等）の専用高速経路（重み固定の分離型 2 パス・SIMD 友好のチャンネル一括処理）・⑵ mask の α 詰め直しの高速化（8 px 単位のパック）・⑶ 出力バッファの確保をなくす（`alloc_resample_dst` の輪番）。期待: resample 50 → 5〜10 ms・mask 11 → 1〜2 ms ＝ 外れ ≈ 15 ms。決定論と既存 golden を保ちやすい（同じ整数 bilinear の等価実装なら byte 同一）。
- **ⓒ 当たりを増やす**: キャッシュ容量 3 → まばたき周期＋着せ替え状態を収める本数（8〜16）。メモリ 1 エントリ ≈ 5.7 MB（k=2・800×448×4×4）× 本数 × 対象数。起動挨拶の表情切替には効かない。
- **ⓑ 拡大を GPU へ**（WUC／D2D の transform）: 外れの代金そのものが消えるが、**emo-dpi-scaling D3 が却下した案**（当たり判定マスクとの不整合・鮮明性）。採るなら D3 の裁定を覆す設計ディスカッションが要り、M 規模。
- **ⓓ 外れの処理を UI スレッドから外す**（別スレッドで用意し出来たコマから提示）: ガクガクは消えるがコマが遅れる／飛ぶ。最後の手段。

**推奨**: ⓐ→ⓒ の順で perf-loop を回し、目標 1 に届かなければ ⓑ の裁定へ。「ｂで収まるか」への答え＝**ⓑ単独ではマスク整合の前提を崩すので収まらない**。まずⓐで外れを 15 ms 級にし、ⓒで外れ自体を減らすのが本命。

## Scope

- **In**: `crates/areka-emo-compose/src/scale.rs`（リサンプラ）・`crates/areka-emo-present/src/{cache.rs, presenter/show.rs, presenter/budget.rs, presenter/timing.rs}`（キャッシュ容量・mask 生成・確保の輪番・計時）・perf-loop の順位表と台帳・決定論檻（byte 等価・外れ 1 回の上限）。
- **Out**: 合成の規約（`areka-emo-compose` の compose）・文字層（`areka-emo-text`）・DPI の政策（`emo-present/scale.rs`）・WUC transform への切替（ⓑ・裁定が要る場合は別議題）。

## Boundary Candidates

- リサンプラの高速経路（compose crate・純関数・byte 等価で検証）／キャッシュ容量とメモリの裁定（present crate）／mask 生成の高速化（present crate）。3 相を 1 spec で perf-loop の 1 周 1 変更で回す。

## Out of Boundary

- 機械負荷（別プロジェクトのジョブ・同期）の管理。
- k=1 の描画経路（触らない・golden 不変で保証）。

## Upstream / Downstream

- **Upstream**: 完了 spec `areka-P0-emo-dpi-scaling`（A2・D3 の裁定）・`areka-P0-recompose-budget`（compose の予算と計時の型・perf-loop の先例）・`areka-P0-draw-load-parity`（描画量の目標と門）。
- **Downstream**: `areka-P0-emo2-conformance-e2e`（症状 E の登記の消化・完成判定は待たない）・`tick-gate-adoption`。

## Existing Spec Touchpoints

- **Extends**: `recompose-budget`（同じ計時の型で resample／mask へ広げる）。
- **Adjacent**: `emo-dpi-scaling`（D3 を覆さない範囲で動く）・`emo2-conformance-e2e`（`crates/areka-emo-text/` を触っている＝共有ファイル 0）。

## Constraints

- perf-loop の規律（1 周 1 変更・順位表・台帳・byte 等価の門）。
- 1,000 行の見張り（`scale.rs` は分割済み・`show.rs`／`budget.rs` の行数を着手時に実測）。
- 決定論: 同一入力→同一バイト。高速経路は既存の整数 bilinear と byte 同一であることを檻で示す（等価でない方式を採るなら golden の再導出と裁定）。

## 追記（2026-09-11・e2e 一周の実機所見＝起動挨拶での見え方）

- **見え方**: 初回起動の挨拶で「文字が出てこない／10 字くらい一度に出る」（開発者所見・2026-09-10 22:2x と 2026-09-11 00:17 の 2 回）。文字の再生は絶対時刻の台本なので、UI スレッドが `apply_show` の外れで止まった分だけ次のコマで字がまとめて出る。起動挨拶は表情の切替が続くうえキャッシュが冷えているので、外れがほぼ毎コマ（00:17 の走行: 起動 18 秒間で 37 外れ・うち 32 は本体側 `surface_id=1000`）。
- **数字**: 静かな機械（2026-09-10 22:21）では起動窓の外れ 1 回＝resample 58 ms（最大 95）＋mask 14 ms。別リポジトリの `cargo test` が 5 コアを取っている間（22:30〜25:00・00:17）は resample 303 ms（最大 508）＋mask 62 ms＝**全段が一様に約 5 倍**。SSP は DWM 拡大で同じ負荷でも遅れない。
- **本 spec の受入に足す 1 行**: 静かな機械で、起動挨拶の間に UI スレッドが 1 コマ（16.7 ms）を超えて止まる回数を 0 に近づける（外れ 1 回 ≤ 16 ms の目標と同義）。負荷下の挙動は目標に含めないが、外れの代金が下がれば負荷下の「文字が出ない」も同じ比率で軽くなる。
- **切り分けの根拠**: e2e 記録 §13.2 行 9（症状 E）と同根。新しい spec は起こさない（2026-09-11 の判断・同じ根に 2 つの引受先を作らない）。
