# 設計バリデーションレポート: areka-P0-emo-present

- 実施日: 2026-07-06
- 対象: `.kiro/specs/areka-P0-emo-present/design.md`（requirements.md 確定済み・spec.json language=ja）
- 手法: design-review.md プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）＋実コード突合（wintf hit_test / GraphicsCore / com/wuc / areka-actor / emo-compose 実シンボル / mock-shell donor）

## レビューサマリ

設計は R1〜R8 全要件をコンポーネント・インターフェイス・フローへ完全にトレースし、主要な設計判断（自前 swap chain 供給面・`Hide` 専用 variant・DPI 全物理 px 契約・バルーン統一経路）はいずれも research.md のディスカッション決定と ukadoc 正典に根拠づけられている。引用された既存シンボル（`Composer::compose_into`・`BindSet::from_ids`・`AlphaMask::from_pbgra32`・`GraphicsCore::d3d()/dxgi()`・`CompositorInteropExt`・`ReplySender`/`spawn_ui`・mock-shell donor）は全て実在をコード上で確認した。実装可能性は高く、下記 2 点の局所修正を設計ディスカッションで確定すれば着手可能である。

## Critical Issues

🔴 **Critical Issue 1**: hit-test 改修点の特定が実コードと不整合（clickthrough は `hit_test_entity_ex` を通る）
**Concern**: Modified Files は「`hit_test_entity` の AlphaMask 読みで `AlphaMaskResource` を最優先」とするが、クリックスルーの実経路は `evaluate_targets` → `hit_test_in_window`（`hit_test/mod.rs:464`）→ **`hit_test_entity_ex`**（同 `:509`・`BitmapSourceResource` 読みは `:352`）であり、`hit_test_entity`（`:220`）だけを改修すると R2.2/R2.3 が成立しない。マウス系 window_proc（mouse_click/mouse_move/mouse_dblclick_wheel）も `hit_test_in_window` 経由。
**Impact**: 文言どおり実装すると本 spec の中核ゴール（キャラ領域のみクリック捕捉・透明域透過）が実行時に沈黙して失敗する。設計自身の「evaluate_targets は無改変のまま恩恵を受ける」という主張（AlphaMaskResource 節）とも矛盾。
**Suggestion**: 改修対象を「`hit_test_entity` と `hit_test_entity_ex` の両 AlphaMask 分岐」（または両者が共有する読み出しヘルパへの抽出）と明記し、単体テストも `hit_test_in_window` 経由の優先読みを檻に含める。
**Traceability**: R2.2, R2.3（クリック捕捉/透過）
**Evidence**: design.md「File Structure Plan > Modified Files」「AlphaMaskResource＋hit-test 読み口」／実コード `crates/wintf/src/ecs/layout/hit_test/mod.rs:220,352,509`・`clickthrough/controller.rs:202`

🔴 **Critical Issue 2**: 退化ケースの写像が不整合（`ZeroExtent` は到達不能・`EmptyComposition` の扱いが未確定）
**Concern**: emo-compose は外形 0×0 の退化を `Err(ComposeError::EmptyComposition)` で返し（`plan.rs:454`・Ok で 0×0 の `ComposedSurface` を返す経路はない）、これは `PresentError::Compose(#[from])` に吸収される。Error Categories では Compose 系＝「指令エラー → skip・表示不変」だが、EmoPresenter 節は「全透明退化（0×0）は `ZeroExtent` として **Hide 相当に縮退**」と規定しており、同一事象（0×0 退化）に skip と Hide の二解釈が併存する。設計どおりだと `ZeroExtent` 変換は事実上デッドコード。
**Impact**: 実装者の解釈次第で「全透明 surface 指定時に非表示になる／前の表示が残る」という観測可能な挙動差が生じ、seriko 結線後の `\s` 意味論にも波及する。
**Suggestion**: `Compose(EmptyComposition)` の扱いを一意に確定する（例: EmptyComposition → warn＋Hide 縮退へ写像し、`ZeroExtent` variant は balloon 合流等 compose 外由来の防御として残すか削除）。テスト（apply 経路）にこのケースを追加。
**Traceability**: R3.4（解決不能指令の扱い）・R3.3（非表示遷移）
**Evidence**: design.md「EmoPresenter > Responsibilities（全透明退化）」「Error Handling > 退化入力/指令エラー」／実コード `crates/areka-emo-compose/src/plan.rs:439-454`

（第 3 の必須指摘なし。`BoxStyle` 不使用時に surface entity の `Arrangement`/`GlobalArrangement.bounds`（＝マスク座標変換の基準）を誰がどう物理 px で確立するかは設計に明示がなく、Issue 1 の修正と併せてタスク生成時に 1 行明記することを推奨する——伝播機構自体は `propagate_global_arrangements` が既存で成立する。）

## Design Strengths

1. **証拠駆動の設計**: 上流実シンボル・wintf 公開 API・ukadoc 正典（`\s[-1]`・`descript_balloon` 3 分類）を逐一実測して引用し、DPI は「全物理 px・恒等写像」契約で window-placement 欠陥（論理/物理混在）の再発を構造的に排除。Revalidation Triggers で隣接 spec（seriko/placement/text-layer）との契約変更点も明文化されている。
2. **R8 の単一真実源設計**: 自前 swap chain＋`source_tex`＋D2D 非経由の純バイト転送により、golden 決定論（R6.2）・readback（R8.3）・将来の直読みヒットテスト基盤が 1 本の経路に統合され、spike 先行（WARP 可＝CI 決定論）でリスクも前倒しで潰す構成。`CacheEntry{composed, mask}` 同梱による原子入替（R2.4）の構造的担保も簡潔で堅い。

## Final Assessment

**Decision: GO**（条件付き——上記 2 点を設計ディスカッションで design.md に反映してからタスク生成へ）

**Rationale**: アーキテクチャ整合（層分離・依存方向・UI スレッド規律・ログ規律）と要件充足は全面的に成立しており、2 つの指摘はいずれも局所的な記述修正で解消できる（構造の変更を要しない）。Issue 1 は文言どおりの実装だと中核機能が失敗するため、必ず修正を反映すること。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1/2 の修正方針を確定し design.md へ反映
2. `/kiro-spec-tasks areka-P0-emo-present` でタスク生成（spike 先行タスクを先頭に）
