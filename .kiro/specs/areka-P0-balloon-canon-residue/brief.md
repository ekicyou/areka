# Brief: areka-P0-balloon-canon-residue

> **種別**: 追跡 spec（正典先送り 4 点セット＝完全語彙＋縮退シーム＋**追跡 spec＋roadmap 明記**——`kero-balloon` は前 2 点を完備したが後 2 点が欠けていた 6 項目の一括登記）。⑥emo（balloon 系列解決）帰属。
> **源**: `kero-balloon` requirements.md Out of scope（:28-:34）＋ COMPAT §8（:141/:144）。2026-08-01 未登記先送り棚卸で「語彙とシームはあるが追跡 spec が無い」孤児群と判定。
> **着手ゲート**: **M1 外**。これらの語彙を実際に使うゴースト／バルーンの適合が必要になった時（M2 互換面拡大）、または `emo2-conformance-e2e` #14（未使用系列の未描画無破綻確認）が検出欠陥を just-in-time 起票する時に解禁。**e2e は検出の場であって修正の担当ではない**（同 brief Scope が明記）——検出されたら本 spec が受け皿。

## Problem

`kero-balloon` が確立した系列解決権威（`SeriesFamily` テーブル・`crates/areka-emo-present/src/balloon.rs:62`）は balloon 本体系列（`balloons`/`balloonk`/`balloonp{n}def`）のみを収載する。正典にはさらに以下があり、いずれも**縮退シーム済み・実装なし**:

1. **装飾系列の per-scope 化＋深い旧名**: `arrow*`（矢印）・`marker*`（マーカー）・`sstp_new*`・`clickwait*` 等の付属画像族。旧名がさらに深い（例: `arrows`→`arrow` の 2 段旧名）ため `SeriesFamily.scope0_legacy` が**複数要素の配列**として設計済み（COMPAT §8 :141 に「装飾族はより深い旧名」と記録済み）。テーブル 1 行追加で拡張可能。
2. **面 ID 偶奇＝左右向き面集合＋自動切替**: 正典は偶数面=左向き・奇数面=右向きの対を規定し、バルーンがキャラのどちら側に出るかで自動選択される。areka は面 0 固定（kero-balloon R2.6 の attach `surface_id: 0` literal）。
3. **`balloon.defaultsurface`／`kero.balloon.defaultsurface` の非追従**: descript でバルーン初期面を指定する語彙。未実装（COMPAT §8 :144 登記済み）。
4. **`\![reload,balloon]`**: 実行時のバルーン資産再解決。未実装。
5. **`balloonc*`（入力窓系列）の kero 側**: 入力ボックス用バルーン系列の per-scope 対応。未実装。
6. **多面バルーンの面別上書き実被覆**: emo2 fixture は各系列とも面 0 の 1 枚のみ＝面 ID 単位フォールバック（kero-balloon R1.3/R2.3）の**多面での実走証跡が無い**（合成 fixture 檻のみ）。多面 fixture の整備。

**注**: `\b[N]`@scope≥1 の end-to-end 檻（kero-balloon 既知シーム）は `balloon-visibility` が多面シナリオを組む際の**合流候補が第一候補**（同 brief へ申し送り済み）。**vis が要件フェーズで拒否した場合に本 spec が拾う**——二重登記ではなく受け皿序列の明記。

## Current State

- 系列解決は単一権威（`prefix_chain`/`resolve_balloon_faces`）＝新系列はテーブル行の追加で乗る設計（kero-balloon R1.8/R1.9 の帰結）。
- 上記 6 項目すべて: 実装ゼロ・COMPAT §8 登記済み（1・3 のみ）・emo2 は未使用ゆえ M1 実害なし。
- `emo2-conformance-e2e` #14 が「`arrow`/`marker`/`online`/`balloonc`/`sstp` 未描画で破綻なし」の**負検証のみ**を持つ。

## Desired Outcome

（解禁時）対象ゴーストが実際に使う項目から、`SeriesFamily` テーブル拡張＋面選択規則＋descript 語彙＋`\![reload,balloon]` を実需要順に実装。全項目で ukadoc 全文→SSP 実挙動→COMPAT 記録の順（乖離があれば SSP 正・kero-balloon R7.6 の先例）。

## Approach

実需要駆動（M2 の実物ゴーストで使用語彙を実測してから優先順を決める）。先行して固定できるのは:
- `SeriesFamily` の複数旧名配列は実装済み構造＝装飾族は行追加＋面別上書きの層マージ再利用のみ。
- 面偶奇の自動切替は `balloon_alignment`（実装済み・kero-balloon R3.2 が消費）を入力に取る純関数になる見込み。

## Scope

- **In**: 上記 6 項目（＋vis が拒否した場合の `\b[N]`@scope≥1 e2e）。
- **Out**: balloon 本体系列の解決規則（kero-balloon で完成・不変）／バルーン表示ライフサイクル（`balloon-visibility`）／windowposition 族（`windowposition-limit`）。

## Boundary Candidates

- 系列テーブル拡張（データ行のみ＝檻はテーブル駆動で自動拡大）。
- 面偶奇選択の純関数。
- 多面 fixture（合成でなく実 PNG 複数面——vis の多面シナリオと共用できれば 1 回で済む）。

## Out of Boundary

- placement 層全般。
- SERIKO アニメーション定義の per-scope 化そのもの（kero-balloon R5.6 で完成・バルーン表が空なのは fixture 事実であって欠陥ではない）。

## Upstream / Downstream

- **Upstream**: `kero-balloon`（系列解決権威・完成）／`balloon-visibility`（多面 fixture の先行整備者になる可能性・`\b[N]` e2e の第一受け皿）。
- **Downstream**: M2 互換面拡大の各ゴースト適合。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `balloon-visibility`（受け皿序列: `\b[N]` e2e は vis 優先・本 spec は次順）／`emo2-conformance-e2e`（#14 は検出のみ・修正は本 spec へ）。

## Constraints

- M1 では着手しない（`surfaces-basepos`・`sakura-time-directives` と同じ M2 解禁ゲート棚）。
- 面引数は不透明文字列・解決は下流（areka-surface-args-opaque-string-downstream-resolve）。
