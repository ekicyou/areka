# 設計検証レポート: wintf-clickthrough-alpha-toggle

> 実行モード: 非対話（kiro-validate-design / design-review.md 準拠）
> 検証日: 2026-07-02
> 対象: `.kiro/specs/wintf-clickthrough-alpha-toggle/` の design.md（FINALIZED）／requirements.md（R1〜R10）／research.md（ギャップ分析＋設計 discovery）／spec.json（language=ja, phase=design-generated）
> 参照 steering: product.md / tech.md / structure.md ほか（ULW 並走・依存方針・レイヤ境界）

## 検証サマリ

本設計は「表示層（GPU 合成 visual/content）と当たり判定層（HWND `WS_EX_TRANSPARENT`）の二層分離」を中核に、当たり判定を既存シーングラフ・ヒットテスト（`hit_test_in_window` → `Option<Entity>`／`None`＝透過）へ全面委譲する、合成バックエンド非依存・GPU readback 不要の構造として一貫している。実測アンカー（`hit_test_in_window` L464／`snapshot_drag_state` L215／`VsyncEventBridge`／`WinStyle::commit` L24 が FRAMECHANGED 非対応／areka の shell・balloon 窓 spawn 箇所）はコードベースと照合済で全て整合し、要件トレーサビリティ（R1〜R10）も網羅されている。フォーカス領域（ヒットテスト源・スレッド境界・ex-style トグル・ドラッグ抑止・座標一致・areka WUC 化・リリース互換・R6.5 規律）はいずれも設計に honored されており、実装着手可能な水準にある。

## Critical Issues（最大 3・重大なもののみ）

本設計に GO を妨げる重大な設計的欠陥は検出されなかった。以下は「実装時に閉じるべき軽微な確認事項」であり、いずれも要件の欠落・矛盾ではなく、GO を覆さない（design-review.md の「acceptable risk」枠内）。参考として 3 点を明示する。

🟡 **Minor 1**: 差分ガードの状態基盤が二重に見える
**Concern**: `ClickThroughController::resolve_transition` は引数 `last_applied: DesiredState` を受け取り差分判定する一方、`ClickThroughRegistry` も per-window `last_applied` を保持する。純関数テストのための切り出しは妥当だが、「唯一の真実源（レジストリ）」と「純関数へ渡す値」の同期規律（適用成功時のみ更新／失敗時は据え置き）が §Components に文章化されているものの、状態機械としての単一所有が図で明示されていない。
**Impact**: 適用失敗（`SetWindowPos` エラー）時に `last_applied` を更新しない旨は Error Handling に記載済のため実害は小さいが、実装者が純関数側とレジストリ側で更新タイミングをずらすと差分ガード（R3.2）が破れうる。
**Suggestion**: 実装時、`last_applied` の更新は「適用成功後にレジストリへ書き戻す」1 経路のみとする不変条件をコメント/テストで固定する（既に Testing Strategy の `resolve_transition` テストで部分カバー）。
**Traceability**: R3.2 / R3.3
**Evidence**: design.md §Components（ClickThroughController Service Interface `resolve_transition` / ClickThroughRegistry State Management）

🟡 **Minor 2**: ドラッグ終了「再収束」の起床契機が受動依存
**Concern**: `JustEnded` は `snapshot_drag_state` で観測されるが、`JustEnded` 遷移そのものはカーソル移動 notify とは独立に起こりうる。ワーカはカーソル「移動」時のみ notify するため、ドラッグ終了直後にカーソルが静止していると、次の起床まで再収束（R5.2）が遅延する可能性がある。
**Impact**: マウスボタンを離した瞬間にカーソルが動かない稀ケースで、透過状態の再収束が次のカーソル移動まで遅れる。実運用上ドラッグ終了直後は微小な移動を伴うことが多く実害は限定的だが、R5.2 の「終了時に再収束」を厳密に満たすなら明示が望ましい。
**Suggestion**: ドラッグ終了（`JustEnded`）遷移点で `cursor_event.notify` を 1 回発火する結線を設けるか、UI ループの周期起床（tick 相乗り）で `JustEnded` を拾う旨を実装ノートに追記（設計変更ではなく結線点の明示）。
**Traceability**: R5.2
**Evidence**: design.md §System Flows（ドラッグ終了再収束）／§Components（ClickThroughController 起床ごとの手順）

🟡 **Minor 3**: `latest_cursor()` と起床のレース詳細が未記述
**Concern**: ワーカは `latest_pos: Arc<AtomicI64>` を pack して更新し notify、UI 側が `latest_cursor()` で読む設計。listen-before-work 規律は踏襲されているが、「notify 後に UI が読む座標が最新である」保証（AtomicI64 の store と notify の順序）が Contracts では invariant に留まり、順序（store→notify）が明文化されていない。
**Impact**: 実害はほぼ無い（1 サイクル古い座標でも次サイクルで収束、かつ UI 側も差分ガード）。ただし実装者が notify→store の順で書くと、稀に 1 通知分の座標遅延が生じうる。
**Suggestion**: 実装時「`latest_pos.store(...)` → `event.notify(...)` の順」を Implementation Notes に一行固定（`VsyncEventBridge` と同じ規律）。
**Traceability**: R3.1 / R4.1
**Evidence**: design.md §Components（CursorMonitorBridge Service Interface / Invariants）

## 設計の強み（Strengths）

- **「αバッファ readback 不要」の構造的解決**: 要件ディスカッションで確定した「実描画αはシーングラフ評価が体現する」という結論を、`hit_test_in_window` への委譲＋判定を UI スレッドへ寄せる設計で忠実に具現化している。これにより座標変換の二重化・スナップショット整合ずれ（DPI 変化・モニタ跨ぎ・表示更新追随）を構造的に消し、R2.3/R2.4/R8 を一貫した 1 つの写像問題へ縮約できている（design.md §判定実行スレッド境界／§Architecture）。
- **既存資産の Adopt と新規 Build の境界が明確**: スレッド跨ぎ起床（`VsyncEventBridge` テンプレ）・当たり判定（`hit_test_in_window`）・ドラッグ判定（`snapshot_drag_state`）・FRAMECHANGED レシピ（`apply_initial_state`）を Adopt とし、新規 Build を「カーソル監視ワーカ」「TRANSPARENT トグル関数」の 2 点（grep 0 件で空白実証済）に限定。R6.5（推測改変せず変更対象＝結線点を事前提示）・R9.3（依存追加ゼロ）を File Structure Plan の Modified Files 明示で honored している。

## 最終判定

### Decision: **GO**

**Rationale**: フォーカス全 8 領域（ヒットテスト源＝`Option<Entity>`／`None` 委譲・GPU readback 排除／ワーカは `GetCursorPos` のみ・判定と ex-style 適用は UI スレッド `spawn_local`・event_listener 起床・tokio 不使用／`SetWindowLongPtr(GWL_EXSTYLE)`＋`SetWindowPos(SWP_FRAMECHANGED)` トグル・LAYERED 同伴のみ・NCHITTEST 不採用／`snapshot_drag_state` ドラッグ抑止／`GetCursorPos` 物理→client→bounds→mask 座標チェーンの既存委譲／areka `CompositionMode::DComp` 化＋ULW 並走残置／opt-level='z'・lto・32bit・依存最小／`docs/click_through.md` 新規／R6.5 変更面の事前提示）が設計に反映され、実測アンカーがコードベースと全て整合。重大な設計的欠陥・要件矛盾・過剰複雑性はなく、実装経路が明瞭で残リスクは acceptable。

### Next Steps

- 上記 Minor 1〜3 は設計変更を要さない実装時規律。`/kiro-spec-tasks wintf-clickthrough-alpha-toggle` でタスク生成に進み、タスク内の Implementation Notes / テスト観点として吸収する。
- 実装着手前に File Structure Plan の Modified Files 一覧（`win_style.rs` トグル追加・`runtime/mod.rs` 起動フック・`ecs/mod.rs` 宣言・`areka/src/main.rs` DComp 化＋登録）を依頼者へ提示し確認を得る（R6.5 運用規律）。
