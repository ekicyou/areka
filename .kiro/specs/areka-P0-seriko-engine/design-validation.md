# 設計バリデーションレポート — areka-P0-seriko-engine

- 対象: `.kiro/specs/areka-P0-seriko-engine/design.md`
- 実行モード: 非対話（検証のみ・design.md/requirements.md は変更しない）
- 判定: **GO**

## 設計レビュー要約

本設計は、非 Send な `EmoWorld` を構築スレッドで所有スナップショット（`BTreeMap<String, Vec<u32>>`）へ切り離し、実行時は所有データのみで純粋に動く「単一アクター内三層（解決/状態/発行）」という、既存 areka 並行モデル（channel アクター・UI 非依存）と決定論テスト網羅要求に正しく整合した構成である。上流契約（`SurfaceSink`／`TalkCue`／`CueCommand::Emote`／`BindSet::from_ids`／`MountModel` `#[non_exhaustive]`）は実コードと突合済みで、要件 R1〜R7（R4.5 含む）は Requirements Traceability 表で全て被覆されている。焦点 6 点はいずれも設計に反映されており、残る指摘は実装で吸収可能な非致命の精度事項に留まる。

## 焦点事項の確認結果（実コード突合済み）

- **(a) EmoWorld-not-Send の遵守** — 合格。`crates/areka-emo-compose/src/world.rs:64` の `EmoWorld` は `World`(bevy_ecs) を内包し `Send`/`Sync` 未実装。設計 DD2 は構築スレッドで alias を所有スナップショット化し、`SurfaceResolver`（所有 `BTreeMap`）＋`BindSet`（Send）のみをアクターへ move する。実行時に `&EmoWorld`／`Arc<EmoWorld>` を持ち込まない方針が明記され、越境は発生しない。
- **(b) DD4=(c) MountModel の bindgroup-default KV 拡張** — 合格。`MountModel` は `#[non_exhaustive]`（`crates/areka-parsers/src/package/model.rs:27`）ゆえフィールド追加は後方互換。`BindGroupDefaults { sakura_default_on, kero_default_on }` を増設し、既存 `names/shiori/shell` と非衝突。R4.5 は `mountmodel_bindgroup_parse` テスト（emo2 descript.txt→集合一致＋name 系保持回帰）で実行検証可能。
- **(c) bindgroup#→animation-id 恒等写像と default-on 集合** — 合格。DD3 が恒等写像を明言。emo2 fixture 実測（`descript.txt` の `sakura.bindgroupNNNN.default,1`）で `{1100,1207,1302,1500,1800}` を確認、設計値と完全一致。`BindSet::from_ids` が昇順整列＋dedup するため順不同保持でも決定的。
- **(d) multi-id alias 先頭固定規則（R2.5）と R7 決定論** — 合格。DD6 として文書化。emo2 実データ `静観,[2106,2206]`（`surfaces.txt`）に対し先頭固定で 2106 を選ぶ規則が定義され、同一入力→同一出力の不変条件で R7 を保つ。
- **(e) SurfaceSink 実装＋自前 Close variant＋単一発行点** — 合格。`SerikoSink`(`impl SurfaceSink`・infallible・send 失敗は `error!`)、自前 `SerikoMsg::Close`（`SakuraMsg::Close` 先例に準拠）、単一発行点 `emit_display` が設計され、後続 `seriko-loop` が同発行点を再利用できる形になっている。
- **(f) 決定論 mock-sink テスト（sleep 不使用）** — 合格。指令適用（`cue_sequence_emits_expected`）・解決失敗ログ＋継続（`unresolved_logs_and_skips_continue`）・非表示遷移（`scope_isolation_and_hide`）・Close/全 drop 停止（`close_stops_normally`/`disconnect_stops_normally`）を、所有テーブル直入力＋`MockSurfaceOutput.records()`＋`ActorHandle::join` で表示なし・sleep なしに閉じる。

要件被覆: R1.1–1.5 / R2.1–2.5 / R3.1–3.5 / R4.1–4.5 / R5.1–5.5 / R6.1–6.4 / R7.1–7.4 の全項が Traceability 表に対応コンポーネントと共に記載され、未被覆要件は無い。

## 重大課題（最大 3 件・いずれも非ブロッキング）

### 🟡 Critical Issue 1: `-1` 以外の負数／`u32` 範囲外の数値分岐が resolve のテストで明示被覆されていない
- **Concern**: 決定5/Implementation Notes は「負の非 `-1` 値は `Unresolved`（防御）」「`-1` は `i64` で受け、非負を `u32` へ」と述べるが、Testing Strategy の `resolve_numeric` は `"2100"`/`"-1"`/`"0"` のみで、`"-2"` や `u32::MAX` 超（例 `"4294967296"`）の分岐が実行テストに現れない。
- **Impact**: `key.parse::<i64>()` 成功後に `u32` へ落とす境界（オーバーフロー時の `Unresolved` フォールバック）が回帰檻に入らず、決定論テスト網羅要求（R7.4「構造担保のみで代替しない」）にわずかな穴が残る。
- **Suggestion**: `resolve_numeric` に負の非 `-1`（`"-2"`→`Unresolved`）と `u32` 範囲外（`Unresolved`）のケースを 1 行ずつ追加する。タスク生成時にテスト列挙へ含めれば足り、設計変更は不要。
- **Traceability**: R2.1, R2.4, R7.4
- **Evidence**: design.md「SurfaceResolver / Implementation Notes」「Testing Strategy / Unit Tests（`resolve_numeric`）」

### 🟡 Critical Issue 2: `alias_snapshot` 増設が emo-compose の外部設計に触れるが、その increment の被覆テストが本 spec のテスト戦略に明記されていない
- **Concern**: DD2 は emo-compose へ公開 accessor `EmoWorld::alias_snapshot(&self) -> BTreeMap<String, Vec<u32>>` を増設すると決めているが、Testing Strategy はこの accessor 自体（`resolve_alias` と同一データを返す・非衝突）を検証するテストを列挙していない。R7.3 の追験は seriko 側 `resolve` のみを対象にしている。
- **Impact**: 増設 accessor が `AliasMap` の内容と乖離した場合（将来の fold 変更等）、seriko の解決が静かにズレるが本 spec のテストでは捕捉されない。越境増設ゆえ Revalidation Trigger 対象でもある。
- **Suggestion**: emo-compose 側に「`alias_snapshot()` が `resolve_alias` と同キー集合・同値を返す」最小回帰テストを 1 本、本 spec のタスクに含める（emo2 の `通常`/`静観` で突合）。
- **Traceability**: R2.2, R2.3, R7.3
- **Evidence**: design.md「決定1（DD2）スナップショット取得口」「Modified Files（world.rs）」「Testing Strategy」

### 🟡 Critical Issue 3: `SerikoSink` の send 失敗（inbox 全受信端消失）を観測する実行テストが無い
- **Concern**: bridge の `emit` は infallible で、送出失敗を `error!` ログのみで扱う（R6.3）と設計されるが、Testing Strategy にこの経路（アクター停止後に `emit` を呼ぶ等）を叩く決定論テストが無い。他の失敗経路（`Unresolved`・`EntityRef`）にはテストがある。
- **Impact**: silent failure 禁止（R6.3）の一角である「send 失敗時にログが出て panic しない」ことが実行テストで担保されず、log-first 規律の回帰檻に隙が残る（記憶: ログ発火も保留ゲート等で決定論化して檻を作る方針）。
- **Suggestion**: `close`→`join` 後に `SerikoSink::emit` を 1 回呼び、panic せず継続する（＝infallible 契約維持）ことを確認する軽量テストを追加。ログ内容の照合までは不要で、非 panic の確認で足る。
- **Traceability**: R6.3, R6.4
- **Evidence**: design.md「SerikoActor / Responsibilities（SurfaceSink bridge）」「Error Handling（sink send 失敗）」「Testing Strategy」

## 設計の強み

1. **非 Send 上流の切り離しが型で保証される所有スナップショット設計**: `EmoWorld`（bevy_ecs World 内包・非 Send）を構築スレッドで `BTreeMap` クローンへ落とし、アクターへは Send データのみ move する DD2 は、areka 並行モデル（大型データは所有ハンドオフ）と R7 の所有テーブル直入力による決定論観測の双方を同時に満たす、境界の綺麗な判断である。
2. **単一発行点＋冪等ガードによる後続シームの明示**: `ScopeStates::apply` が「変化したか」を返し `emit_display` 単一関数へ集約する構造は、後続 `seriko-loop`（時間駆動）／`mayuna-compose`（bind 置き場差し替え）が発行点と置き場のみを再利用・差し替えできる形を先取りしており、Boundary Commitments のシーム約束を具体化している。

## 最終評価

- **判定**: **GO**
- **根拠**: 上流契約は実コードと全て突合済みで、焦点 6 点（EmoWorld 非越境・MountModel 拡張・恒等写像・先頭固定・Close/単一発行点・決定論 mock テスト）は設計に正しく反映され、R1〜R7（R4.5 含む）に未被覆要件は無い。指摘 3 件はいずれもテスト列挙の追記で吸収可能な非致命事項で、アーキテクチャ的な矛盾や高い失敗リスクは無い。
- **次段**: 上記 3 件をタスク生成時のテスト列挙へ反映のうえ、`/kiro-spec-tasks areka-P0-seriko-engine` へ進む。emo-present API 確定と `alias_snapshot`/`MountModel` KV 保持形の変更は Revalidation Trigger として継続監視。
