# Brief: areka-P0-present-gpu-transform-scale

> **起票 2026-09-07（旧名 `areka-P0-present-resample-budget`）・2026-09-11 に改名と方針の確定**。改名の理由: 旧名は「CPU の代金を削る」を示唆したが、開発者裁定（2026-09-11・`/kiro-discovery`）は **「画像本体は原寸で持ち、常に D2D の変換行列指定による拡大縮小とする。画像を CPU で拡大しているなら許容できない」**。目標は予算の圧縮ではなく、CPU 拡大経路の**撤去**である。上流の登記: `.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/acceptance-record.md` §13.2 行 9（症状 E）・同 requirements「改訂（2026-09-07・第 4 回）」2。M1 の完成判定（e2e）は本 spec を待たない。

## Problem

拡大率 200%（k=2）で、立ち絵の絵が変わるコマ（表情・まばたき・着せ替え）のたびに **CPU で 1.4 Mpx を拡大**（整数 bilinear・約 40 ns/px）し、そのバイト列からマスクを作り直してから GPU へ上げている。静かな機械で外れ 1 回 60〜85 ms、別プロセスが CPU を取ると 300〜600 ms。処理は UI スレッド上で同期に走るので、文字の再生（絶対時刻の台本）が止まった分だけ次のコマで字がまとめて出る（「10 字くらい急に表示」「初回起動で文字が出てこない」）。SSP は DWM 拡大で同じ負荷でも遅れない。

## Current State

- 2026-07-29 `areka-P0-emo-dpi-scaling`（PR #91）の設計 **D3＝Strategy A2**（合成後の native 面を present 段で k 倍に CPU リサンプル）・**D5**（整数固定小数点 bilinear・「GPU stretch は決定論 readback の檻と不整合」で却下）・**D6**（キャッシュのエントリ＝k 倍後の面＋その bytes 由来のマスク）。k=1 は恒等コピーなので 100% では無料——**「最初は重くなかった」のは k=1 だったから**。
- 置き場: `crates/areka-emo-compose/src/scale.rs`（リサンプラ）・`crates/areka-emo-present/src/presenter/show.rs`（`apply_show`・段階別計時 `perf(apply_show)`）・`cache.rs`（容量 3 LRU）・マスク `regenerate_from_pbgra32`・上げ口 `t_upload_us` ≈ 0.3 ms。
- 実測（e2e 記録 §13.2 行 9・brief 旧版）: 外れ 1 回＝compose 5〜7 ＋ resample 44〜61 ＋ mask 11 ＋ upload 0.3 ms。起動挨拶では 37 外れ中 32 が本体側立ち絵（`surface_id=1000`）。負荷下は全段が一様に約 5 倍。
- D3 が GPU を却下した理由は現在成り立たない: ⑴ 当たり判定は W5（`collision-dpi-hittest`・PR #100）が**点 ÷k で native 座標へ縮約**する形で着地し、k 倍のマスクを当たり判定に使っていない。⑵ 鮮明性は D2D の補間モード（bilinear／高品質）で選べる。⑶ k≠1 の golden は present 段の CPU 面を読んでいるだけで、**k=1 の合成 golden を正**とすれば決定論は失われない。
- バルーンは重くない（文字はベクタ描画・バルーン画像の外れは走行 1 回で 1 度）。文字層の見え方の問題は UI スレッドの停止の帰結。

## Desired Outcome

1. **画像本体は原寸（native）で持ち、拡大縮小は常に D2D／WUC の変換行列で行う**。present 段の CPU リサンプル（`scale.rs` の k 倍経路）と k 倍バイト列からのマスク生成を撤去する。k は変換行列の係数としてだけ現れる。
2. k=2 で、絵が変わるコマの UI スレッド処理が **1 コマ（16.7 ms）を超えない**（期待: compose 1〜7 ms ＋ upload 0.3 ms）。起動挨拶の間に `catch-up` が前回並み（3 分で ≤ 10 件）。
3. クリック透過用の α マスク: native α を保持し、窓側の照会は ÷k で native を引く（または 1 チャンネルの k 倍・SIMD 不要）。当たり判定（÷k）は不変。
4. 決定論: k=1 の合成 golden は不変。k≠1 の見た目は実機サインオフ（2 水準）へ移す。既存の k 付き golden は再導出か撤去を裁定する。
5. `emo-dpi-scaling` D3／D5／D6 を正式に覆し、`doc/COMPAT_ARCHITECTURE.md` の上書き表と steering（`areka-emo-own-compositor-atlas`／`areka-dpi-following-core-design` の該当箇所）に登記する。

## Approach

- **ⓑ GPU 変換（本命・開発者裁定）**: `apply_show` は native の composed 面をそのまま上げ、WUC visual（または D2D の描画）に `k` のスケール行列を与える。窓寸・配置は既に k 倍の物理 px で決まっているので、面の論理寸だけが native になる。マスクは native α から。cache は native 面のみ（容量は据え置きで足りる見込み・外れの代金が消えるため）。
- 却下: ⓐ CPU リサンプルの高速化（無駄を速くするだけ・開発者「許容できない」）・ⓒ 容量増（起動挨拶の表情切替には効かない）・ⓓ 別スレッド化（コマが遅れる）。

## Scope

- **In**: `crates/areka-emo-present/src/{presenter/show.rs, presenter/refresh.rs, cache.rs, mask 生成}`・WUC visual の変換の適用（`wintf` の表示レシピが持つなら lift）・`crates/areka-emo-compose/src/scale.rs` の k 倍経路の撤去（`ScaleRatio`／`scaled_extent` の寸法の権威は残す）・k≠1 golden の裁定・実機 2 水準サインオフ・`perf(apply_show)` の計時の維持。
- **Out**: 合成の規約（compose）・文字層（`areka-emo-text`）・DPI の政策（`emo-present/scale.rs` の k 導出）・当たり判定（W5 のまま）。

## Boundary Candidates

- 提示段の変換（present crate・WUC visual の行列）／マスクの native 化と窓照会（present crate＋wintf の照会口）／golden の裁定（決定論層）。3 相を 1 spec で、設計ディスカッションで D3／D5／D6 の上書きを先に確定する。

## Out of Boundary

- 機械負荷（別プロセス）の管理。
- 拡大率遷移の 2 ティックの跳ね（`areka-P0-dpi-transition-two-tick-bounce`・別 spec）。
- 文字層の行送り・折返し（完了 spec）。

## Upstream / Downstream

- **Upstream**: 完了 spec `areka-P0-emo-dpi-scaling`（D3／D5／D6 を覆す）・`areka-P0-collision-dpi-hittest`（÷k・不変）・`areka-P0-recompose-budget`（計時の型）・`areka-P0-draw-load-parity`。
- **Downstream**: `areka-P0-emo2-conformance-e2e`（症状 E の登記の消化・完成判定は待たない）・`areka-P0-dpi-transition-two-tick-bounce`（外れの代金が消えると遅れの量が変わる＝本 spec の後に採り直す）・`tick-gate-adoption`。

## Existing Spec Touchpoints

- **Extends**: `emo-dpi-scaling`（設計上書き）・`recompose-budget`（計時）。
- **Adjacent**: `emo2-conformance-e2e`（共有ファイル 0）・`zorder-chain-residue`。

## Constraints

- 決定論（同一入力→同一バイト）は k=1 の合成 golden で保つ。k≠1 は実機サインオフ。
- 1,000 行の見張り（`show.rs`・`cache.rs` は着手時に実測）。
- 開発者裁定（2026-09-11）: CPU 拡大の残置は不可。
