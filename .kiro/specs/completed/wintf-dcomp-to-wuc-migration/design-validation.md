# 設計バリデーション報告: wintf-dcomp-to-wuc-migration

> 実行: 非対話バリデーション（kiro-validate-design / design-review.md 準拠）。日付: 2026-07-01。言語: ja。
> 対象: requirements.md（確定）／design.md（確定）／research.md／steering。本報告は design.md を変更しない。

## Design Review Summary

DComp→WUC の**純粋等価移行**として、設計は既存アーキテクチャ（lazy 単一 Resource・UI スレッドアフィニティ・13 段 schedule・事前配置）を温存する Option C（混成）を採り、境界・トレーサビリティ・移行順序が明瞭で実装準備は総じて高品質。全 10 要件（R5.4 clip 等価／R8.5–8.7 自動ピクセル差分ハーネスを含む）が Requirements Traceability 表で被覆され、唯一の構造変化（`SetContent`→`SurfaceBrush`）は生成・解除の両経路に正しく写像されている（コードで実在確認: surface.rs L174 生成／L259 解除、begin_draw 戻り型 `(ID2D1DeviceContext3, POINT)` は dcomp.rs L235 と一致）。DispatcherQueue apartment 種別を R1 スパイクへ委ねる判断は「推測せず実測」の健全な verify-don't-guess であり未解決ギャップではない。重大な設計不整合はなく、実装着手可能。

## Critical Issues（≤3）

本移行は重大なブロッカーを含まないが、実装前に明文化しておくと手戻りを防げる中程度の懸念を最大 3 点、設計ディスカッション用に提示する。

🟡 **Issue 1: R8.6 サーフェス層「移行前ゴールデン」の取得手順が未固定**
**Concern**: R8.6 は移行前 D2D 出力を参照ゴールデンとしてビット等価比較する主受け入れ手段だが、DComp コードが WUC へ差し替わった後にゴールデンをどう保存・再現するか（差し替え前のコミットで生成し固定バイナリ/ハッシュを commit するのか、D2D 描画コード不変性を根拠にランタイム同時生成するのか）が design.md に明記されていない。
**Impact**: ゴールデン基準が曖昧だと「ビット等価」判定の再現性が損なわれ、R8.5「主たる受け入れ手段」の担保が弱まる。
**Suggestion**: ゴールデン生成タイミング（移行前コミット時点で固定 or 同一 CommandList のランタイム二重描画）と保存形態（ハッシュ/PNG を repo へ commit するか test 内自己完結か）を design か tasks で 1 行確定する。
**Traceability**: 8.5, 8.6
**Evidence**: design.md「Testing Strategy › E2E / 描画等価性」・「surface_pixel_equivalence_test」

🟡 **Issue 2: DispatcherQueue drop 順序保証が「フィールド順」記述に依存し検証項目化されていない**
**Concern**: 設計は DispatcherQueueController を Compositor より長寿命に保つ制約を「型のフィールド順で保証」と述べるが（要件 3.3 の ShutdownQueueAsync ドレインと連動）、Rust の drop はフィールド宣言順であり `WucGraphicsResourceInner` の宣言順・`invalidate()` の null 化順序で崩れうる。R1 スパイクの検証チェックリストに drop/shutdown 順の明示確認が入っていない。
**Impact**: 順序誤りは終了時ドレイン漏れ・稀な shutdown クラッシュを生み、等価性テストで顕在化しにくい。
**Suggestion**: `WucGraphicsResourceInner` のフィールド宣言順（controller を最後）と `invalidate()` の解放順を design か impl ノートで固定し、R1 スパイクの受け入れ項目へ「終了時ドレイン成立」を追加する。
**Traceability**: 3.3
**Evidence**: design.md「WucGraphicsResource › Implementation Notes（Risks: DispatcherQueue drop 順）」

🟡 **Issue 3: clip PathGeometry 等価の合格基準が「目視フォールバック」寄りで自動判定閾値が未定義**
**Concern**: `RoundedRectangleIndividual`（4 独立半径）は WUC 直接等価が無く PathGeometry 弧構築で写像するが、その幾何一致の合否は「合成層キャプチャ比較（R8.7）／過渡は残差目視」に委ねられ、ピクセル差分の許容閾値（完全一致 or 近似許容）が未定義。areka 本体は個別半径を未使用（example/ULW guard のみ）と明記される一方、R8.5 は自動判定を主手段とする。
**Impact**: 閾値未定義だと個別半径 clip の等価判定が主観化し、R8.7「明示した範囲に限り目視」の線引きが曖昧化する恐れ。
**Suggestion**: 個別半径 clip を R8.7 の「決定論的キャプチャ不能＝目視残差」範囲として明示するか、固定シーンでの許容差分閾値（例: 完全一致要求 or 端数丸め許容）を tasks で 1 行確定する。areka 本体未使用ゆえ実害小との前提も併記。
**Traceability**: 5.4, 8.7, 9.4
**Evidence**: design.md「clip_sync_system › Risks」・「Testing Strategy › 合成層キャプチャ比較」

## Design Strengths

- **境界規律とトレーサビリティが厳格**: Boundary Commitments（This Spec Owns / Out of Boundary / Revalidation Triggers）と Requirements Traceability 表が全 10 要件を被覆し、R9 のスコープ非侵（ULW 除去・クリックスルー・WUC 新能力の全排除、`compute_ex_style` 不変流用）が File Structure Plan・Non-Goals・Out of Boundary で三重に固定されている。スコープクリープ耐性が高い。
- **唯一の構造変化の対称化と等価根拠の実証性**: `SetContent`→`SurfaceBrush` 一段挟みを生成/解除の両経路で単一システム対称化し brush 寿命をフィールド保持で担保、begin_draw 戻り型を現行と byte-identical に保つ設計は、R-High リスクを最小化する妥当な打ち手（コードで生成 L174／解除 L259／戻り型一致 L235 を確認済み）。

## Final Assessment

**Decision: GO**

**Rationale**: 純粋等価移行として全 10 要件が被覆され、唯一の構造変化は create/teardown 両経路に正しく写像、DispatcherQueue apartment・32bit×WUC runtime・透過共存の不確実点は R1 スパイクで実測確定する verify-don't-guess 設計であり、残る 3 点はいずれも実装で吸収可能な中程度の明文化課題で、重大な architectural misalignment・要件欠落・過大複雑性はない。

**Next Steps**:
- 設計ディスカッション（kiro-design-discussion）で上記 3 Issue を検討し、必要なら design.md へ 1 行規模の明確化を反映。
- 反映後 `/kiro-spec-tasks wintf-dcomp-to-wuc-migration` でタスク生成へ進む。
- release z/LTO 疎通（R8.1）・i686 可搬（R8.4）は tasks の各層完了チェックポイントへ確実に組み込む。
