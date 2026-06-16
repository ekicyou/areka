# 新規仕様提案候補（codebase-review-loop）

全セルが保留改善（挙動変更を伴う脆弱性対策・ロジック変更を要する簡素化・削除実証不能な非推奨コード等）を以下の様式で追記する。
最終フェーズ（タスク22）で重複統合し、優先度付き提案として report.md へ一括整理する。

様式:

```
## P{連番}: {提案タイトル}
- source: {cell-id}
- kind: 挙動変更を伴う脆弱性対策 | ロジック変更を要する簡素化 | 非推奨コード削除候補 | その他
- rationale: 本ループで実施しなかった根拠（挙動変更の内容）
- suggestion: 新規仕様としての推奨スコープ
```

---

## P1: wintf SetWindowPosCommand キューのテスト検査 API 追加
- source: A1-T
- kind: その他
- rationale: areka `on_shell_drag` の正常系（バルーン追従座標 `pos + BALLOON_OFFSET` の enqueue 内容）を検証したいが、`SetWindowPosCommand` のスレッドローカルキューには `enqueue`/`flush`（実 SetWindowPos 実行）しか公開されておらず、enqueue 内容を観測する手段がない。wintf への API 追加は A1-T セルの境界（crates/areka/src/）外であり本ループでは実施しなかった。
- suggestion: wintf `ecs/window/command.rs` にテスト用キュー検査 API（例: `#[cfg(any(test, feature = "test-util"))] pub fn take_queued() -> Vec<SetWindowPosCommand>`）を追加し、areka 側で `on_shell_drag` 正常系の座標アサーションを追加する小規模仕様。

## P2: バルーンウィンドウ位置導出のシェル実体への一元化
- source: A1-S
- kind: ロジック変更を要する簡素化
- rationale: areka `main.rs` ではバルーン位置の知識が 2 経路に重複している — 初期配置は定数（`SHELL_INITIAL_* + BALLOON_OFFSET_*`）、ドラッグ追従は シェルの実 `WindowPos + BALLOON_OFFSET_*`（`on_shell_drag`）。「シェル実位置からの相対配置」に一元化すれば重複知識を除去できるが、初期位置の導出経路の変更（生成時点のシェル実位置参照）はロジック変更にあたるため本ループでは見送った。今回のセルでは安全側の簡素化として、未使用だった `create_balloon_window` の `_shell_entity` 引数の削除のみ適用した。
- suggestion: バルーン初期配置をシェル Entity の `WindowPos` から導出する小規模仕様（A1-T 所見 3 と統合可能）。`on_shell_drag` の追従ロジックと共通のオフセット適用ヘルパに一元化し、P1 のキュー検査 API と組み合わせて座標アサーションで挙動を固定する。

## P3: SHELL_IMAGE_PATH のビルドマシン絶対パス埋め込みの解消（実行時パス解決への移行）
- source: A1-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/areka/src/main.rs` の `SHELL_IMAGE_PATH` は `concat!(env!("CARGO_MANIFEST_DIR"), "/shell/base.png")` でコンパイル時に解決され、ビルドマシンの絶対パス（ユーザー名・ディレクトリ構成）が配布バイナリへそのまま埋め込まれる。これは (a) ビルド環境情報の漏洩（情報開示）、(b) ビルドマシン以外での画像ロード不能（可用性）の2点で脆弱性関連の所見となる。対策（実行ファイル相対やリソース埋め込みへの移行）は画像解決経路という外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは実装しない（A1-T 所見4でも開発実行前提のモックとして記録済み）。
- suggestion: アセットパス解決を実行時化する小規模仕様。候補: (1) `std::env::current_exe()` 起点の相対解決（配布形態に追従）、(2) `include_bytes!` によるバイナリ埋め込み（パス情報を完全に排除、モック実装の単一画像なら最小コスト）。いずれも S7 起動テストと `shell_image_asset_exists` テストの置き換えを含める。

## P4: 起動経路の無音失敗の可観測化（不正 RUST_LOG・UI 構築コマンド送信失敗の警告出力）
- source: A1-V
- kind: その他
- rationale: 脆弱性レビューで2件の「失敗の黙殺」を検出した。(1) `RUST_LOG` が不正な構文の場合 `EnvFilter::try_from_default_env()` の Err が無音で "info" へフォールバックし、利用者は設定ミスに気付けない。(2) `run_setup` の `let _ = tx.send(...)` は受信側喪失時に UI 構築コマンドが無音で破棄される（現構成では受信側 `EcsWorld` が常に生存するため発火不能だが、構成変更で潜在化する）。いずれも警告ログの追加＝ログ出力セマンティクスの変更（外部観測可能な挙動変更）を伴うため、R5.1/R5.2 に従い本ループでは実装しない。
- suggestion: 起動経路の失敗可観測化の小規模仕様。不正 RUST_LOG 検出時に stderr へ1行警告（フォールバック値の明示）、`run_setup` の send 失敗時に `tracing::error!` を出力。挙動差分はログ出力のみで機能挙動は不変のため、S7 起動テストでの検証が容易。

## P5: dola::playback 旧型（PlaybackState / ScheduleRequest）の整理（非推奨化または削除）
- source: D1a-T
- kind: その他（将来の非推奨コード削除候補）
- rationale: `crates/dola/src/playback.rs` の `PlaybackState` / `ScheduleRequest` は dola-runtime-engine 設計時に「データモデル層として温存」とされた旧型で、現ランタイムは `InstanceState` を使用しており、ワークスペース内の利用箇所は serde ラウンドトリップテスト（`tests/general/core_types_test.rs`）のみ。ただし `#[deprecated]` 指定がなく R2.9（非推奨かつ利用ゼロの削除）の条件を満たさないため、公開 API からの削除（外部観測可能な変更）は本ループでは実施しない。なお wintf-P0-balloon-system の gap-analysis は dola↔bevy_ecs スケジュール統合での将来利用に言及しており、削除前に要確認。
- suggestion: (1) balloon-system 側で利用予定が確定しているなら現状維持、(2) 利用予定がなければ `#[deprecated]` を付与して 1 サイクル後に型・re-export・対応テストを削除する小規模仕様。

## P6: facade `cancel()`/`finish()` の到達不能な is_terminal 防御分岐の整理
- source: D1a-S
- kind: ロジック変更を要する簡素化
- rationale: `instance_manager::transition` は終了状態（Concluded/Cancelled）への遷移時にインスタンスを自動削除する（`instance_manager/mod.rs` の `transition`、is_terminal → `instances.remove`）。このため終了済み group_id への `cancel()`/`finish()` は常に手前の `get()` で `InvalidGroupId` となり、両関数内の `instance.state.is_terminal()` 分岐は現行不変条件の下で到達不能（D1a-T 所見2で実証、cancel_after_conclude / finish_after_conclude テストで外側挙動を固定済み）。除去しても挙動は同一だが、到達不能性が境界外コンポーネント（instance_manager）の不変条件に依存するため、防御分岐の除去はロジック変更を伴う簡素化として本ループでは見送った。優先度は低（残置してもコスト僅少）。
- suggestion: instance_manager の「terminal 遷移＝自動削除」不変条件を debug_assert 等で表明したうえで、`cancel()`/`finish()` の到達不能分岐を除去する（または到達不能である旨のコメント付記に留める）小規模整理。

## P7: イージング適用ロジックの重複統合（loop_controller ↔ interpolator）
- source: D1a-S
- kind: その他（セル境界をまたぐ挙動非破壊の簡素化）
- rationale: `runtime/loop_controller.rs` の `apply_easing`（EasingName 32 アーム + ParametricEasing 2 アームの match）は `runtime/interpolator/mod.rs` の `apply_named_easing` / `apply_parametric_easing` と完全に重複している。統合には interpolator 側関数の可視性変更（private → `pub(crate)`）が必要で、D1a-S 境界（loop_controller）と D1b-S 境界（interpolator）のどちらか単独では完結しないため、本ループでは見送った。
- suggestion: interpolator の `apply_named_easing` / `apply_parametric_easing` を `pub(crate)` 化し、`loop_controller::apply_easing` を委譲実装に置き換える 2 ファイルの小規模リファクタ。loop_controller 側は `apply_easing_*` テスト 3 件と分布検定 2 件、interpolator 側は既存補間テストが回帰検知器となる。


## P8: dola ストーリーボード time_scale の入力検証追加（0・負値・非有限値の拒否）
- source: D1a-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `time_scale` はスキーマ（`storyboard.rs`、serde default 1.0）・バリデーション（`validate/rules.rs`）・コンパイル（`compile/mod.rs`）のいずれでも正値検証されておらず、`facade.rs::compile_and_validate` の `loop_duration = total_base_duration / time_scale` で除算される。`time_scale == 0.0` のとき loop_duration は +inf（duration > 0）または NaN（duration == 0、0/0）となり、inf/NaN は既存の `== 0.0` / `< MIN_LOOP_DURATION` 比較を素通りして start が成功する。結果、end_time が inf/NaN のインスタンスは自然終了・ループ・トリガー発火（`process_triggers` の wall_fire_time も同じ除算で inf/NaN 化）が一切起きないまま生存し続け、リソースリーク（インスタンス・タイムテーブルエントリ・トリガーストアの解放漏れ）となる。負値 time_scale は end_time が過去になり初回 update で即 Conclude される。これらを拒否する入力検証の追加は `load_document`/`start`/`calculate_end_time` のエラー応答という外部観測可能な挙動の変更（新エラーバリアント追加を含む）を伴うため、R2.4/R5.2 に従い本ループでは実装せず、現行挙動を `tests/runtime/facade_test.rs::time_scale_boundary` の特性化テスト 5 件で固定するに留めた。
- suggestion: バリデーションルール（V 系）に「time_scale は正の有限値（`> 0.0 && is_finite()`）」を追加する小規模仕様。`RuntimeError`（または `VecDolaError`）への新バリアント追加と、`time_scale_boundary` 特性化テスト 5 件の新仕様への置き換えを含める。loop_offset の負値検証（V14/V15）と同型のルールとして実装可能。

## P9: dola process_loops の周回キャッチアップ反復上限（時刻ジャンプ DoS 耐性）
- source: D1a-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `runtime/loop_controller.rs::process_loops` の while ループは `current_time >= end_time` の間 `advance_loop` を 1 周回ずつ反復するため、反復回数は時刻ジャンプ幅 ÷ 周回長に比例する。無限ループ時の周回長下限は MIN_LOOP_DURATION = 0.1s のみのため、壁時計の大幅補正やホスト側のタイムスタンプ異常（例: 1e9 秒のジャンプ）で最大 1e10 回規模の反復となり、update() が長時間ブロックする quasi-hang（DoS）になり得る。反復上限キャップや剰余演算による一括スキップは、loops_completed・loop_start_time の進み方および loop_offset 乱数の消費回数という外部観測可能な挙動を変えるため、R2.4/R5.2 に従い本ループでは実装せず、ハザードコメントの付記に留めた。
- suggestion: 周回キャッチアップに上限（例: 1 update あたり N 周回、超過分は剰余で一括スキップ）を導入する小規模仕様。loop_offset（乱数遅延）を持つインスタンスの扱い（スキップ時の遅延サンプリング方針）を設計判断として含め、決定的 RNG での周回進行テストを更新する。


## P10: DynamicValue の Hash/Eq 契約違反（Float 0.0 / -0.0）の解消
- source: D1b-T
- kind: その他（Hash/Eq 契約の整合化、intern 重複排除挙動の変更を伴う）
- rationale: `crates/dola/src/value.rs` の `Hash` 実装は `Float(f64)` を `to_bits()` でハッシュするため、`PartialEq` では等しい `Float(0.0)` と `Float(-0.0)`（IEEE 754: 0.0 == -0.0）のハッシュ値が異なり、`k1 == k2 ⇒ hash(k1) == hash(k2)` という Hash/Eq 契約に違反する。`DynamicValue` は `runtime/interpolator` の `ObjectInternPool`（HashMap キー）で使用されており、0.0 と -0.0 が別エントリとして intern される（等値比較するキーが HashMap 内に2つ共存し得る）。メモリ安全性の問題はないが規約違反であり、修正（ハッシュ前に -0.0 を 0.0 へ正規化等）は intern の重複排除粒度＝`EvaluatedValue::Object` の `Rc::ptr_eq` 差分検出結果という外部観測可能な挙動に影響し得るため、本ループでは現行挙動の特性化テスト（`tests/general/core_types_test.rs::float_negative_zero_eq_but_hash_differs`）の追加に留めた。NaN の自己不等（Eq 前提違反）はコード内に文書化済みで、同テスト群で挙動を固定した。
- suggestion: `Hash` 実装で `Float` を正規化（`if *v == 0.0 { 0.0f64.to_bits() } else { v.to_bits() }`、必要なら NaN も単一ビットパターンへ正規化）する小規模修正。特性化テストを新仕様（hash 一致）へ置き換える。

## P11: 競合検出の wall-clock 座標非対応（時間シフトした非重複スケジュールの誤競合）
- source: D1b-T
- kind: その他（挙動変更を要する正確性改善）
- rationale: `runtime/facade.rs` はすべてのストーリーボードを `compile_and_validate(name, 0.0)` で base_time=0.0 のローカル座標にコンパイルし、`runtime/conflict_resolver.rs::detect_overlaps` は引数 `_start_time` を使用せず compile 時座標どうしでセグメント重複を判定する。このため壁時計上は重ならないスケジュール（例: [0,2] の再生終了直後 t=2.0 から同一変数の SB を開始）でも常に競合と判定され、終了戦略が適用される。実害は「先行インスタンスの不要な早期終了」であり、現行挙動は `tests/runtime/conflict_resolution_test.rs::time_shifted_start_on_same_variable_still_conflicts` で特性化済み。wall-clock 換算（各インスタンスの loop_start_time / time_scale / pause_accumulated を考慮した実効時間範囲での重複判定）は競合解決の発火条件という外部観測可能な挙動を変えるため、R5.1/R5.2 に従い本ループでは実装しない。
- suggestion: detect_overlaps を wall-clock 実効時間範囲（インスタンスごとの `loop_start_time + segment / time_scale + pause_accumulated`、ループ・無限ループの扱いを含む）で判定する仕様。`_start_time` 引数の活用とテスト（隣接スケジュール非競合・部分重複競合・Paused の凍結時間扱い）の置き換えを含む。設計判断として「ループ中インスタンスの将来周回をどこまで占有とみなすか」を明確化する必要がある。

## P12: 競合解決の終了経路における trigger_store エントリ残置（リソースリーク）の解消
- source: D1b-T
- kind: その他（挙動非破壊だがセル境界をまたぐ構造変更）
- rationale: `runtime/conflict_resolver.rs` の 4 終了経路（apply_cancel/conclude/trim/compress）は instance_manager からインスタンスを除去するが、facade が保持する `trigger_store` の同 group_id エントリは facade 側の conclude/cancel 経路でのみ削除される。競合解決でトリガー保持インスタンスが終了すると trigger_store にエントリが残置され、group_id は再利用されないためプロセス生存中に単調増加するリソースリークとなる（D1a-V 申し送り）。`process_triggers` はインスタンス起点で走査するため残置エントリが読まれることはなく、外部観測上の挙動（トリガー不発火・panic なし）は `tests/runtime/conflict_resolution_test.rs::conflict_terminated_parent_trigger_never_fires` で特性化済み。修正には conflict_resolver へ trigger_store（facade 所有）を渡すシグネチャ変更が必要で、D1a（facade）と D1b（conflict_resolver）の境界をまたぐため本ループでは実施しなかった。
- suggestion: `resolve_conflicts_excluding` に `&mut HashMap<u64, Vec<CompiledTrigger>>` を渡し各終了経路で `trigger_store.remove(&gid)` を行う（または終了 group_id リストを返して facade 側で一括削除する）小規模リファクタ。挙動は非破壊（メモリ解放のみ）で、既存の競合解決テストと特性化テストが回帰検知器となる。

## P13: document_store `get_storyboard` の整理（テスト専用 dead code の非推奨化または削除）
- source: D1b-S
- kind: その他（将来の非推奨コード削除候補）
- rationale: `crates/dola/src/runtime/document_store.rs` の `get_storyboard` は `#[allow(dead_code)]` 付きで、ワークスペース全体の grep で利用箇所は同ファイル内の単体テスト 3 件のみ（プロダクションコードからの呼び出しゼロ）。`pub(crate)` 構造体のメソッドのため公開 API ではなく削除自体は挙動非破壊だが、削除にはそれを呼ぶ既存テスト 3 件（`get_storyboard_found` / `get_storyboard_not_found` / `store_invalid_document_on_empty_store_keeps_none` の一部アサーション）の削除が必要で、本ループの制約（既存テストの削除・弱体化禁止）および R2.9（`#[deprecated]` かつ利用ゼロのみ削除可）の条件を満たさないため見送った。なお同セルで `conflict_resolver::resolve_conflicts`（dead code・テスト利用もゼロ）は削除済み。
- suggestion: (1) facade 等で名前検索の用途が見込まれるなら現状維持、(2) 用途がなければ `#[deprecated]` を付与し、1 サイクル後にメソッドと対応テスト 3 件を削除する小規模整理。

## P14: 指示書数値フィールドの有限性検証の欠如（NaN/inf の素通り）
- source: D1b-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: バリデーション（`validate/rules.rs`）の数値検査は loop_offset の負値（V14/V15）のみで、`from`/`to`/`relative_to`/`delay`/`duration`/ベジェ制御点（`ParametricEasing` の x0..x3）/`finish` の offset などの数値フィールドに有限性検証がなく、TOML の `nan`/`inf` リテラル経由で NaN/inf がランタイムへ流入し得る。点検の結果 panic 経路は存在しない（`f64::clamp` は NaN を素通り、`as i64` は飽和キャスト、easing 計算は除算なし）が、無害でない静かな縮退が3系統ある: (1) 補間結果 Float(NaN) は `EvaluatedValue::PartialEq` の NaN 自己不等により毎フレーム「変化あり」と判定され続け、変更通知がスパム化する（types.rs に NOTE 付記、`float_nan_is_never_equal_to_itself` で特性化）。(2) `conflict_resolver::detect_overlaps` は NaN 時刻のセグメントを競合として検出しない（比較が常に false。NOTE 付記）。(3) `check_finish_deadlines` は NaN deadline を発火させない（`nan_finish_deadline_never_fires` で特性化）。検証追加はバリデーションエラー応答という外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは NOTE コメントと特性化テスト（interpolator の NaN/inf/飽和テスト8件を含む）に留めた。P8（time_scale）と同根であり統合実装が望ましい。
- suggestion: バリデーションルールに「指示書の全数値フィールドは有限値（`is_finite()`）」を一括追加する小規模仕様（P8 の time_scale 正値検証と同一仕様に統合可）。delay/duration の負値検証の有無も同時に棚卸しし、D1b-V 追加の特性化テスト（NaN 系）を新仕様のエラー期待へ置き換える。

## P15: resume() の非単調時刻入力による pause_accumulated / end_time の過去方向補正
- source: D1b-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `runtime/instance_manager/mod.rs::resume` は `pause_duration = current_time - pause_start` を検証なしで加算するため、呼び出し側が pause 時刻より過去の `current_time` を渡すと pause_duration が負となり、pause_accumulated と end_time が過去方向へ補正される（end_time が現在時刻より過去になると次回 update で即時自然終了し、意図しない早期終了となる）。`clock::now()` 利用時は単調性が保証されるが、API 自体は任意の f64 を受け付ける。負値の拒否・クランプ・エラー返却のいずれも外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは NOTE コメントと特性化テスト（`resume_with_time_before_pause_start_shrinks_end_time`）に留めた。
- suggestion: `resume`（および facade の時刻受け取り経路全般）で「時刻入力は直前の観測時刻以上」を検証またはクランプ（`pause_duration.max(0.0)`）する小規模仕様。update() の時刻巻き戻り時の方針（P9 の時刻ジャンプ対策と対をなす）と合わせて設計判断を明確化する。

## P16: instance_manager::transition の不正状態遷移エラーの InvalidGroupId への混同
- source: D1b-V
- kind: その他（エラー表現の改善、挙動変更を伴う）
- rationale: `runtime/instance_manager/mod.rs` の `transition` / `resume` は `try_transition` の失敗（不正な状態遷移）を `RuntimeError::InvalidGroupId(group_id)` として報告するが、この group_id は実在するため「指定 group_id が存在しない」というエラー型の意味（types.rs の doc）と矛盾し、API 利用者が原因（ID 不在 vs 状態不正）を区別できない。`try_transition` は `Err(current_state)` で現在状態を返しているがこの情報は破棄されている。専用バリアント（例: `InvalidStateTransition { group_id, from, to }`）の追加は公開エラー型と返却値という外部観測可能な挙動の変更を伴うため、本ループでは NOTE コメントの付記に留めた（現行挙動は D1b-T の `invalid_transition_on_existing_instance_reports_invalid_group_id` で特性化済み）。
- suggestion: `RuntimeError` に状態遷移エラー専用バリアントを追加し、`transition`/`resume`/`pause` の失敗経路で from/to 状態を含めて返す小規模仕様。特性化テストを新バリアント期待へ置き換え、facade 経由の pause/resume エラーメッセージの改善も含める。

## P17: compile エラー診断の精度改善（overlap の entry_index 固定0・循環報告の過大包含）
- source: D2-T
- kind: その他（診断情報の改善、エラー内容という外部観測可能な挙動の変更を伴う）
- rationale: テスト網羅性調査で2件の診断精度の問題を確認した。(1) `crates/dola/src/compile/mod.rs` のセグメント重複検査（Step 5）は `DolaError::CompileError { entry_index: 0, ... }`（コード内コメントで "approximate" と明記）を返すため、利用者は重複を起こしたエントリを特定できない（現行挙動は `tests/compile/error_test.rs::segment_overlap_detected` が reason 文字列で特性化済み）。(2) `crates/dola/src/compile/resolve.rs::topological_sort` の循環検出は「in_degree > 0 の全エントリ」を循環メンバーとして報告するため、循環自体には含まれず単に循環の下流にあるエントリも `KeyframeCycle::cycle` に過大包含される。また暗黙キーフレーム名 `__implicit_{idx}` が内部表現のままユーザー向けエラーに露出する。いずれの改善もエラーメッセージ／エラーフィールド値という外部観測可能な挙動の変更を伴うため、R5.1/R5.2 に従い本ループでは実装しない。
- suggestion: (1) セグメント構築時に元エントリ index を `CompiledSegment` 構築文脈で保持し、重複検査エラーへ正確な entry_index（または両エントリの index ペア）を載せる。(2) 循環検出を実際の閉路抽出（DFS バックエッジ追跡等）へ置き換え、報告名から `__implicit_` 内部名を除去または「entry N」表記へ正規化する。既存の特性化テスト（overlap reason・cycle 検出）を新仕様の期待へ置き換える小規模仕様。

## P18: compile 内の到達不能な防御分岐の整理（validate() 前提の二重防御）
- source: D2-T
- kind: ロジック変更を要する簡素化
- rationale: `compile_storyboard` は冒頭で `doc.validate()` を実行するため、以降の防御分岐の多くはバリデーション通過後には到達不能であることを確認した。具体的には (a) `resolve.rs::resolve_transition` の Named 未定義エラー（V5 が先に検出）と transition 欠落エラー（V7-V9 が先に検出）、(b) `mod.rs` の `var_def` 取得失敗時の `continue`（V4 が先に検出）、(c) `mod.rs` の Object 型に対する easing 強制 None（V10 が easing 指定自体を拒否するため、上書きは常に no-op）、(d) `resolve.rs::build_variable_type_hint` の `None → Float` フォールバック（Step 4 で var_def 解決済みの変数しか Step 5 に到達しない）、(e) `resolve_to_value` の非 Scalar from に対する relative_to スキップ（V10/V13 が relative_to と型不整合を拒否）。これらの除去・unreachable 化は「validate() が必ず先行する」という呼び出し契約への依存を強める設計判断であり、防御の意図的温存（将来 validate を経ない内部呼び出しが追加された場合の安全網）とのトレードオフがあるため、ロジック変更を要する簡素化として本ループでは実施しなかった（公開 API 経由の挙動は不変だがコード経路の削除はテスト不能な変更となる）。
- suggestion: compile モジュールの呼び出し契約を「validate 済み document のみ受け付ける」と明文化したうえで、上記防御分岐を `debug_assert!` ＋簡潔な経路へ整理する小規模仕様。または逆に契約を緩め、防御分岐を到達可能なエラー返却として正式化（validate を経ない `compile_storyboard_unchecked` の明示提供）するかの設計判断を含む。

## P19: 純粋KF/トリガー（at なし）の暗黙依存（配列直前エントリ）が依存グラフに反映されない
- source: D2-S
- kind: その他（挙動変更を要する正確性改善）
- rationale: `crates/dola/src/compile/resolve.rs` の `build_dependency_graph` は `at`/`between` による明示依存のみをエッジ化し、`at` を持たない純粋キーフレーム・トリガーエントリの「配列直前エントリの keyframe_time を継承する」という暗黙依存（`resolve_pure_keyframe_time` の else 分岐）をグラフに反映しない。このためトポロジカル順で当該エントリが配列直前エントリより先に処理されると `entry_keyframe_time` が未登録で `CompileError`（"no previous entry keyframe time available"）となる。例: e0 が後方定義の k1（e2 が定義）へ `at` 依存し、e1 が at なし純粋KF の場合、処理順は e1 → e2 → e0 となり e1 の継承元（e0）が未解決でコンパイル失敗する（バリデーションは通過する整形式文書）。修正（at なしエントリへ「配列直前エントリ」依存エッジを追加）は現在エラーになる文書をコンパイル成功へ変える外部観測可能な挙動変更のため、R5.1/R5.2 に従い本ループ（D2-S のシンプル化観点）では実装せず記録に留めた。なお本セルの簡素化（`find_previous_entry_in_sort_order` の除去・`checked_sub(1)` 化）は当該挙動を機械的に保存している。
- suggestion: `build_dependency_graph` で `at`/`between` を持たないエントリ（純粋KF・トリガー含む）に対し配列直前エントリへの依存エッジを追加する小規模修正。失敗→成功へ変わるケースの回帰テスト（後方参照 at と at なし純粋KFの組み合わせ）と、循環が新たに生じ得るか（直前依存＋明示依存の合成閉路）の検証を含める。P17（診断精度）と同一モジュールのため統合実装も可。

## P20: delay/duration の負値検証の欠如（反転セグメントのランタイム流出）
- source: D2-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `TransitionDef` の `delay`/`duration` は符号検証がなく（P14 の有限性とは別軸）、負の duration を持つ指示書は `compile/resolve.rs::resolve_entry_timing` で `segment_end = segment_start + duration < segment_start` の反転セグメントを生成する。これは `compile_storyboard` の文書化された事後条件（「全セグメントは時刻順ソート済みで重複なし」、`CompiledSegment` doc の「即時遷移の場合は start_time と等しい」= end >= start の含意）に違反したままランタイムへ流出する。反転セグメントは単独配置では重複検査（隣接ペアの `prev.end > next.start`）に掛からず、`base_duration` も負値となる（`total_base_duration` は `fold(0.0, f64::max)` の初期値でマスクされ 0.0）。負の delay も start_time より過去のセグメント開始を無検証で生成する（KeyframeRef の負 offset は「KF の 0.5 秒前」等の意図的用法があり得るため一律拒否は設計判断を要する）。検証追加はバリデーションエラー応答という外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは NOTE コメントと特性化テスト（`tests/compile/boundary_test.rs::negative_duration_produces_inverted_segment` / `negative_delay_shifts_segment_before_start_time`）に留めた。
- suggestion: バリデーションルールに「duration >= 0（または > 0）」「delay >= 0」を追加する小規模仕様（P14 の有限性検証と同一仕様への統合を推奨 — P14 suggestion の「負値検証の棚卸し」に対応する具体所見）。負 offset（KeyframeRef）の許容方針を設計判断として明記し、特性化テスト 2 件を新仕様のエラー期待へ置き換える。

## P21: 暗黙キーフレーム名プレフィックス `__implicit_` の予約欠如（明示名との衝突による黙った誤解決）
- source: D2-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: コンパイラは keyframe 省略エントリへ内部名 `__implicit_{idx}` を自動付与する（`compile/resolve.rs::entry_keyframe_name`）が、バリデーションの名前予約は `start` のみ（V3）で `__implicit_` プレフィックスを予約していない。ユーザー（外部指示書または Builder API）が明示的に `keyframe = "__implicit_{n}"` を指定すると別エントリの暗黙名と衝突し、V2（明示名同士の重複検査）も V6（参照存在検査）も通過したうえで、`kf_to_entry`／`keyframe_times` の HashMap 後勝ち上書きにより明示キーフレームの時刻が黙ってシャドウされ、`at` 参照が意図しない時刻へ解決される（panic はせず、誤ったタイムラインが正常出力として返る — 整合性の侵害）。さらに V6 は暗黙名を既知キーフレーム集合へ加えるため、ユーザーが内部名 `__implicit_{idx}` を直接 `at` 参照できてしまう（内部表現の漏出、P17 の診断露出と同根）。プレフィックス予約（新バリデーションエラー）は外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは NOTE コメントと特性化テスト（`tests/compile/boundary_test.rs::explicit_implicit_name_collision_silently_shadows_user_keyframe`）に留めた。
- suggestion: V3 を拡張し「`start` および `__implicit_` プレフィックスで始まる名前はユーザー定義不可」とする小規模仕様（`ReservedKeyframeName` の再利用で新バリアント不要）。`at`/`between` 参照側でも同プレフィックスの直接参照を拒否するかを設計判断として含め、特性化テストを新仕様のエラー期待へ置き換える。P17（`__implicit_` のエラーメッセージ露出）との統合実装も可。

## P22: TimedSchedule の同時刻エントリ配信順の不整合（insert = FIFO / extend = LIFO）
- source: D3-T
- kind: その他（配信順という外部観測可能な挙動の変更を伴う一貫性改善）
- rationale: `crates/dola/src/cue/schedule.rs` の `TimedSchedule<T>` は内部 Vec を降順ソートで保持し末尾 pop で消費するが、同一オフセットのエントリについて `insert()`（`partition_point` が既存の同値要素より前へ挿入 → 挿入順＝FIFO 配信）と `extend()`（安定降順ソートが挿入順を保持 → 末尾 pop で逆順＝LIFO 配信）で配信順が逆転する。同時刻の `Text` コマンド列など順序が意味を持つペイロードで、台本の投入 API の選択により観測順が変わる。現行挙動は `tests/cue/schedule_test.rs::insert_same_offset_payloads_delivered_fifo` / `extend_same_offset_payloads_delivered_lifo` で特性化済み。`extend` を FIFO へ揃える修正は同時刻エントリの配信順という外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは特性化テストの固定に留めた。
- suggestion: `extend()` の比較関数へ投入連番（または安定ソート前の reverse）を組み込み、同一オフセットでも挿入順 FIFO となるよう統一する小規模仕様。`compile_sheet` → `CueQueue` 経路（wintf 側の利用箇所を含む）でどちらの API が使われているかを棚卸しし、特性化テスト 2 件のうち `extend` 側を FIFO 期待へ置き換える。

## P23: validate_trigger_cycles の再帰 DFS の反復化（深いトリガーチェーンでのスタック消費）
- source: D3-S
- kind: ロジック変更を要する簡素化
- rationale: `crates/dola/src/validate/rules.rs::dfs_detect_cycle` は再帰 DFS であり、トリガーチェーンの深さに比例してコールスタックを消費する（極端に深い `trigger_storyboard` 連鎖を持つ外部指示書でスタックオーバーフローの理論的可能性）。明示スタックによる反復 DFS への変換は構造的なロジック変更であり、循環検出時に報告される `TriggerCycle::cycle` のメンバー・パス順序（DFS の訪問順に依存し、現行は D3-T 追加の `tests/trigger/validation_test.rs` の循環形状テストで特性化済み）の同一性証明を要するため、R5.1/R5.3 に従い本ループでは変換せず記録に留めた。なお現実的な指示書の規模ではスタック消費は問題にならない。
- suggestion: 明示スタック（`Vec<(ノード, 隣接イテレータ位置)>`）による反復 DFS へ置き換え、訪問順・循環抽出（path 上の next 位置以降 + 閉包ノード）を現行と同一に保つ小規模仕様。既存の循環検出特性化テスト（自己参照・相互参照・3 ノード循環・下流非包含）を回帰検知器とし、深いチェーン（例: 10^5 段）での非クラッシュテストを追加する。

## P24: validate_variable_ranges の Float/Integer 分岐の重複統合（i64 比較精度の設計判断を要する）
- source: D3-S
- kind: ロジック変更を要する簡素化
- rationale: `crates/dola/src/validate/rules.rs::validate_variable_ranges` の Float / Integer 分岐は initial の min/max 検査が構造的に同一の約 25 行 × 2 の重複だが、単純な f64 統一ヘルパへの統合は Integer 側の比較を i64 → f64 の損失変換（|v| > 2^53 で丸め）に変えるため、極値での検出有無が変わる観測可能な挙動変更となる。挙動保存のままの統合はジェネリクス + f64 変換クロージャを要し、2 箇所のための抽象として複雑さが純増する（S6 基準で不採用）ため、本ループでは現行の重複を維持した。
- suggestion: (1) 「変数値域は f64 精度で検証する」と仕様を明文化したうえで f64 統一ヘルパへ統合する（2^53 超の i64 境界値は実用上想定外とする設計判断を含む）、または (2) 重複を許容して現状維持とするかの設計判断。(1) を採る場合は極値（i64::MAX 近傍の min/max）での検証挙動を新仕様の期待として特性化する。

## P25: Cue パイプライン（CueSheet → compile_sheet → TimedSchedule）の時刻入力検証の欠如（NaN/inf の素通り）
- source: D3-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: Cue 経路には DolaDocument の validate() に相当する検証層がなく、`Cue::start_time` および `TimedSchedule` の時刻入力（insert/extend の Entry オフセット・tick の current_time/start_time）に有限性検証がない。点検で確認した縮退は3系統: (1) NaN オフセットの insert/extend は debug ビルドでは非負 debug_assert が発火する（`NaN >= 0.0` が false。"must be non-negative" という負値前提のメッセージで報告される）一方、release では素通りして `partition_point` の前提（述語の単調分割）を破り、以後の挿入位置が不定化して配信順が黙って崩れ得る（整合性侵害、panic なし）。(2) `tick(NaN)`（または start_time が NaN）は負値ガード・冪等性ガード・時刻到達比較（`entry_offset > offset`）がすべて false 化し、最初のバリアまでの全ペイロードが即時配信される（演出時系列の崩壊）。(3) `compile_sheet` は NaN start_time を `f64::min` の NaN 無視特性で最小値計算から黙って脱落させ当該 Cue へ NaN オフセットを生成し、+inf は一部 inf なら inf オフセット（当該エントリが永遠に配信されず `is_completed()` が false のままとなる liveness 喪失）、全 Cue が inf なら inf−inf=NaN オフセットとなる。検証追加（エラー返却 API 化・拒否・クランプのいずれも）は外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは NOTE コメント（schedule.rs 3 箇所・sheet.rs 2 箇所）と特性化テスト 8 件（tests/cue/schedule_test.rs 4 件・tests/cue/sheet_test.rs 4 件）に留めた。
- suggestion: cue パイプラインの数値検証を導入する小規模仕様。候補: (1) `compile_sheet` を `Result` 化（または `CueSheet::new` で検証）して非有限 start_time を拒否、(2) `TimedSchedule::insert/extend/tick` 側での防御（非有限値の拒否・スキップ）。DolaDocument 側の P14（有限性一括検証）と検証方針を揃え、特性化テスト 8 件を新仕様のエラー期待へ置き換える。P22（extend の FIFO/LIFO 統一）と同一モジュールのため統合実装も可。

## P26: loop_count の文書レベルバリデーション追加（不正値検出の前倒し）
- source: D3-V
- kind: その他（検証網羅性の改善、エラー検出時点の変更を伴う）
- rationale: `Storyboard.loop_count` のスキーマ仕様（storyboard.rs doc コメント:「0以下 = エラー、-1 = 無限ループ」）に反する値（0 や -2）は validate()・compile_storyboard のいずれでも検査されず素通りし、ランタイムの start/calculate_end_time 時に facade の `InvalidLoopCount` で初めて拒否される（後置検出）。ランタイム検査が存在するため実害（panic・黙った縮退）はないが、文書構造の検証責務が validate() に集約されていない網羅性ギャップであり、利用者は load_document 時点で不正文書と判明せず start を呼ぶまでエラーを観測できない。V 系ルールの追加はバリデーションエラー応答という外部観測可能な挙動の変更（DolaError への新バリアント追加または InvalidEntry 流用）を伴うため、R2.4/R5.2 に従い本ループでは特性化テスト（tests/validation/schema_test.rs::invalid_loop_count_passes_document_validation）に留めた。
- suggestion: バリデーションルールに「loop_count は -1 または 1 以上」を追加する小規模仕様。facade 側の InvalidLoopCount 検査は防御として温存し、特性化テストを新仕様のエラー期待へ置き換える。P8（time_scale 正値検証）・P14（有限性一括検証）と同一の「文書数値フィールド検証」仕様群への統合を推奨。

## P27: 非推奨モジュール `win_message_handler` の削除（利用箇所3件の一括移行を要する削除セット）
- source: W1-S
- kind: その他（非推奨コードの削除候補。R2.10 による記録 — 利用箇所の移行を伴うため本ループでは削除不可）
- rationale: `crates/wintf/src/win_message_handler.rs`（1,400 行）は3モジュール中唯一 `#![deprecated(since = "0.1.0")]` を持つが、ワークスペース内利用ゼロを実証できなかった（R2.9 の削除条件を満たさない）。grep（crates / examples / tests 全域）とビルド確認で実証した残存利用は3件: (1) `winproc.rs` のハンドラ dispatch 経路（`get_boxed_ptr` / `from_boxed_ptr` / `into_boxed_ptr` がトレイトオブジェクトとして使用）、(2) `win_thread_mgr.rs:142` の `WinThreadMgrInner::create_window(handler: Arc<dyn BaseWinMessageHandler>, ...)` 引数型、(3) `examples/dcomp_demo.rs:94` の `impl WinMessageHandler for DemoWindow`。なお `winproc.rs` と `win_thread_mgr.rs` 自体は `#![deprecated]` 注記を持たず（`#![allow(deprecated)]` のみ）、かつ現役（winproc の `wndproc` は `process_singleton.rs:57` でレガシークラスに登録され `WM_LAST_WINDOW_DESTROYED` アーム（モーダルループ中の終了経路）が live、win_thread_mgr は areka 本体 `main.rs:87`・examples 12 件・ecs 側 static 依存を持つ常駐基盤）のため削除候補ではない。
- suggestion: 削除セット = { `win_message_handler.rs` 全体, `winproc.rs` のハンドラ dispatch 経路（`WM_LAST_WINDOW_DESTROYED` アームは隠しメッセージウィンドウ用の最小プロシージャとして存続または `ecs` 側へ移設）, `WinThreadMgrInner::create_window`, `examples/dcomp_demo.rs`（ECS 経路 `ecs::window_proc` ベースへの書き換えまたは廃止） } を一括で扱う小規模仕様。dcomp_demo の移行が完了すればレガシーウィンドウクラス `wintf_window_class` の登録（process_singleton.rs）も隠しメッセージウィンドウ専用へ縮小できる。本削除により P28（get_boxed_ptr の健全性違反）は経路ごと消滅するため、P28 より優先して実施することを推奨。

## P28: `winproc::get_boxed_ptr` の健全性違反の修正（トレイト型混同 + 共有参照からの可変参照 transmute）
- source: W1-S
- kind: その他（テスト未保護の unsafe 領域に対するロジック変更を要する健全性修正。脆弱性レビュー観点（W1-V）にも関連）
- rationale: `crates/wintf/src/winproc.rs` の `get_boxed_ptr`（および格納側 `into_boxed_ptr`）には2つの健全性違反がある。(a) `into_boxed_ptr` は `Box<Arc<dyn BaseWinMessageHandler>>` として格納したポインタを、`get_boxed_ptr` が `*mut Arc<dyn WinMessageHandler>`（別トレイトのファットポインタ）として読み出す型混同。(b) `#[allow(mutable_transmutes)]` による `&dyn → &mut dyn` の transmute（共有参照からの可変参照生成 = 未定義動作領域）。areka 本体ではこの経路は実行されない（メッセージ専用ウィンドウは lpParam なしで生成され GWLP_USERDATA が null のまま → 常に None → DefWindowProcW へフォールバック）が、レガシー `create_window` 利用者（dcomp_demo）では毎メッセージ実行される。修正はテストで保護されない unsafe / Win32 領域のロジック変更となるため、R5.5/R2.8 に従い本ループでは実施せず、winproc.rs へ NOTE コメントを付して記録に留めた。
- suggestion: P27（削除セット）の実施で経路ごと消滅させるのが第一推奨。win_message_handler 経路を存続させる場合は、格納型と読出型を `Box<Arc<dyn BaseWinMessageHandler>>` に統一し、可変アクセスは transmute ではなく内部可変性（`RefCell` 等。wndproc はメインスレッド呼び出しのため `thread_local` + `RefCell` が ecs 側の既存パターンと整合）へ置き換える小規模仕様とする。

## P29: steering `structure.md` の非推奨記載と実装実態の乖離修正
- source: W1-S
- kind: その他（steering ドキュメントの整合性修正。steering はレビューセル境界外のため記録のみ）
- rationale: `.kiro/steering/structure.md` は `win_message_handler` / `win_thread_mgr` / `winproc` の3モジュールすべてを ⚠️ `#[deprecated]` と記載するが、実態として `#![deprecated]` 注記を持つのは `win_message_handler.rs` のみ（W1-T 所見を W1-S で再実証）。`win_thread_mgr.rs` は areka 本体・examples 12 件・ecs モジュール（`WM_LAST_WINDOW_DESTROYED` / `VSYNC_TICK_COUNT` / `LAST_VSYNC_TICK` / `DEBUG_WNDPROC_TICK_COUNT` の static 依存）が利用する現役の常駐基盤であり、`winproc.rs` の `wndproc` もレガシークラス登録＋終了経路の一部として現役。記載と実態の乖離は R2.9 削除判定（deprecated 指定の有無が削除条件の前提）を誤誘導するリスクがある。
- suggestion: `/kiro-steering` の同期実行時に structure.md の当該記載を実態（非推奨は win_message_handler のみ。win_thread_mgr / winproc は現役だが win_message_handler に依存する経路を含む）へ修正する。3モジュールの段階的廃止が設計意図である場合は、P27 の移行完了後に win_thread_mgr / winproc へ `#[deprecated]` を付与する手順とし、現状の一括 deprecated 記載は撤回する。

## P30: WinThreadMgr のリソース解放整備（CoUninitialize 欠如・create_window 失敗時のハンドラ Box リーク）
- source: W1-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/win_thread_mgr.rs` に2件のリソース解放漏れがある。(1) `new()` は `CoInitializeEx(None, COINIT_MULTITHREADED)` で COM を初期化するが `Drop` に対応する `CoUninitialize` がなく、インスタンスごとに COM 初期化カウントが1つ残置される（プロセス常駐の単一インスタンス運用では実害なしだが、生成/破棄を繰り返す利用形態でアンバランスが累積する）。(2) `create_window` は `into_boxed_ptr` でハンドラの `Box<Arc<dyn BaseWinMessageHandler>>` をヒープ確保してから `CreateWindowExW` を呼ぶが、回収経路は WM_NCCREATE（GWLP_USERDATA 格納）→ WM_NCDESTROY（`from_boxed_ptr` 解放）のみのため、CreateWindowExW が WM_NCCREATE 送出前に失敗した場合（不正スタイル・リソース枯渇等）は Box がリークする。(1) の修正は終了時挙動（COM アパートメント解放タイミング）の変更、(2) の修正はエラー経路の解放処理追加（WM_NCCREATE 実行済みか否かの判別設計を要する）を伴うため、R2.4/R5.2 に従い本ループでは NOTE コメント2箇所の付記に留めた。なお Drop の VSync スレッド停止順序（stop_flag → join → DestroyWindow）は点検の結果健全（破棄済み HWND への PostMessageW は構造的に発生しない）で、`crates/wintf/tests/thread_mgr.rs` の new+drop 特性化テストで非ハングを固定済み。
- suggestion: (1) Drop の最終段（DestroyWindow 後）に `CoUninitialize` を追加（new() の CoInitializeEx 成否との対応を保証する設計を含む）。(2) `create_window` のエラー経路で「WM_NCCREATE が GWLP_USERDATA へ格納済みでない場合のみ」Box を解放する（lpCreateParams 受け渡し方式の見直し、または P27 の削除セット実施で `create_window` ごと消滅させる）。P27 実施が先行する場合 (2) は不要となるため、P27 との実施順序を設計判断に含める。

## P31: WinProcessSingleton 初期化の部分失敗非冪等性と panic 診断の改善
- source: W1-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/process_singleton.rs::get_or_init` の初期化クロージャはプロセスグローバルな `RegisterClassExW` を2回呼ぶ非冪等処理だが、`OnceLock::get_or_init` はクロージャ panic 時に未初期化のまま残る仕様のため、1つ目のクラス登録成功後に2つ目が失敗して panic すると、次回の get_or_init でクロージャが再実行され、1つ目の `RegisterClassExW` が ERROR_CLASS_ALREADY_EXISTS で 0 を返し「Failed to register window class」という誤誘導メッセージで再 panic する（部分失敗から回復不能で、しかも原因が偽装される）。また両 panic メッセージは GetLastError を含まず失敗原因を診断できない。RegisterClassExW の失敗は実用上リソース枯渇時に限られ発生確率は極小だが、冪等化（ERROR_CLASS_ALREADY_EXISTS の許容）と GetLastError を含むメッセージ改善はいずれも panic 挙動・メッセージという外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは NOTE コメントの付記に留めた。
- suggestion: (1) RegisterClassExW 失敗時に `GetLastError` を取得し、ERROR_CLASS_ALREADY_EXISTS は成功扱い（冪等化）、それ以外はエラーコード付きメッセージで panic する小規模修正。(2) 併せて `GetModuleHandleW(None).unwrap()` / `LoadCursorW(None, IDC_ARROW).unwrap()`（実用上不可謬だが診断情報なし）も `expect` + 文脈メッセージへ統一する。挙動差分は失敗時の panic 経路のみで正常系は不変。

## P32: WinThreadMgr 多重生成時の ECS_WORLD 束縛固定（OnceLock 初回 set 後の黙殺）
- source: W1-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/win_thread_mgr.rs::new` は `crate::ecs::set_ecs_world(Rc::downgrade(&world))` で EcsWorld の弱参照をグローバルへ登録するが、登録先 `ecs/window_proc/mod.rs` の `ECS_WORLD` は `OnceLock` で `let _ = ECS_WORLD.set(...)` と2回目以降の set を黙って無視する。このため2個目以降の WinThreadMgr が生成された場合、その world は束縛されず、(a) 2個目の ECS ウィンドウのメッセージは初代 world へ配信される（別インスタンスの状態を破壊し得る整合性侵害）、(b) 初代 drop 後は `Weak::upgrade` 失敗で全 ECS メッセージが黙って DefWindowProc へフォールバックする（機能の無音喪失）。さらに `SendWeak` の `unsafe impl Send/Sync` は「メインスレッドのみアクセス」を前提とするが、複数スレッドで各自 WinThreadMgr を生成する利用形態では非アトミック参照カウント（Rc/Weak）への跨スレッド upgrade となり UB に至る。areka 本体は単一インスタンス・単一スレッド運用のため現行実害はなく、多重生成が panic しないことを `crates/wintf/tests/thread_mgr.rs` の特性化テストで固定し、win_thread_mgr.rs へ NOTE を付記した。修正（2個目の拒否・束縛の差し替え・thread_local 化のいずれも）は外部観測可能な挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは実装しない。
- suggestion: 設計判断を含む小規模仕様: (1) WinThreadMgr をプロセス内単一インスタンスとして明文化し、2個目の `new()` を `Err` で拒否（最小変更・契約の明示化）、または (2) ECS_WORLD を HWND→world のマップまたは thread_local へ置き換え多重インスタンスを正式サポート。(1) を推奨（ecs_wndproc・VSYNC_TICK_COUNT 等の static 群も単一インスタンス前提のため）。`SendWeak` の Send/Sync 安全性条件のコメント強化を含める。

## P33: d2d 描画コマンド録画モジュール（RecCommandSink / DrawCommand）の完成または削除
- source: W2-T
- kind: その他
- rationale: `crates/wintf/src/com/d2d/command_sink.rs` / `command_types.rs`（計 1,016 行）はワークスペース内（crates / examples / tests）に利用箇所が存在しない未完成モジュール。(1) 唯一の観測 API である `RecCommandSink::commands()` が `todo!()` のままで呼び出すと必ずパニックする（実装内コメントも「RefCell の内側への参照を返せない」という未解決の設計課題を明記）。(2) `DrawCommand` の COM 保持バリアントは `#[derive(Clone)]` だが、フィールドが `ManuallyDrop<T>` のため clone 時に `T::clone`（AddRef）で増えた参照が永久に Release されず、clone 1 回につき COM 参照を 1 つリークする。(3) `dup_com` による非所有参照保持は「元 COM オブジェクトが記録より長生きする」ことを呼び出し側に要求するが、この不変条件を強制する仕組みがない。非推奨指定がないため R2.9 の削除条件を満たさず、完成（commands() の `Ref<'_, Vec<DrawCommand>>` 返却化・Clone セマンティクスの修正）はロジック変更を伴うため、本ループでは構築・記録経路のテスト固定（tests/com/command_types_test.rs / command_sink_test.rs / d2d_ext_test.rs の Stream 再生）と本提案の記録に留めた。
- suggestion: 設計判断を含む小規模仕様: (1) 用途（D2D コマンドの記録・検査・リプレイ）を明確化して完成させる — `commands()` を `Ref<'_, Vec<DrawCommand>>` または `Vec<DrawCommand>` のクローン返却へ変更し、COM フィールドの Clone を AddRef+Release 対で管理する所有形（`ManuallyDrop` を外し通常のスマートポインタ保持）へ移行する。または (2) 利用予定がなければモジュールごと削除する（利用ゼロは本セルで実証済み。削除なら追加したテストも同時に撤去）。

## P34: DWriteTextLayoutExt のクラスタ数取得におけるエラー黙殺の解消
- source: W2-T
- kind: その他
- rationale: `crates/wintf/src/com/dwrite.rs` の `get_cluster_metrics` / `get_cluster_count` は、必要バッファサイズ取得のための 1 回目の `GetClusterMetrics(None, &mut actual_count)` の戻り値を `let _ =` で無条件に破棄している。この呼び出しは count > 0 のとき E_NOT_SUFFICIENT_BUFFER を返す Win32 イディオムだが、それ以外の失敗（無効状態のレイアウト等）でも区別されず、`actual_count` が 0 のままなら `Ok(0)` / `Ok(Vec::new())`（空成功）として返る。失敗を成功（空）へ写像する黙殺であり、呼び出し側（テキスト描画系）は異常を検知できない。HRESULT を検査して E_NOT_SUFFICIENT_BUFFER 以外を Err として伝播する修正は戻り値セマンティクスの変更（外部観測可能な挙動変更）にあたるため、R5.1/R5.2 に従い本ループでは実装しなかった。
- suggestion: 1 回目の呼び出しの HRESULT を検査し、S_OK（count==0）と E_NOT_SUFFICIENT_BUFFER のみを正常系として続行、それ以外は Err 伝播へ変更する小規模修正。`tests/com/dwrite_test.rs` の既存テスト（空文字列 → 0 クラスタ、ASCII/日本語/サロゲートペアのクラスタ数）が正常系の回帰検知器として利用できる。

## P35: D2D1CommandListExt::open の常時 E_NOTIMPL スタブの削除
- source: W2-S
- kind: ロジック変更を要する簡素化
- rationale: `crates/wintf/src/com/d2d/mod.rs` の `D2D1CommandListExt::open` は実装コメント（「ID2D1CommandList は直接 Open できない」）の通り COM 呼び出しを一切行わず、無条件に `Err(E_NOTIMPL)` を返すだけの未実装スタブである。プロダクション側の利用ゼロを grep で実証済み（`D2D1CommandListExt` を import する 4 ファイル — rectangle.rs / draw_labels.rs / typewriter_draw.rs / bitmap_source/systems.rs — はいずれも `close()` のみ使用。`open()` の呼び出しは W2-T が追加した特性化テスト `tests/com/d2d_ext_test.rs:93` の 1 件のみ）。呼んでも必ず失敗する API を公開トレイトに残すことは利用者を誤誘導するため削除が望ましいが、削除は公開 API の変更であり、かつ E_NOTIMPL を固定する特性化テストの削除を伴う（テストの削除・弱体化は R2.3/R5.5 により本ループでは不可）ため、W2-S では実施せず記録に留めた。
- suggestion: `open()` をトレイト宣言・impl から削除し、対応する特性化テスト（`command_list_open_always_returns_e_notimpl`）を撤去する小規模修正。CommandList への記録開始は `D2D1DeviceExt::create_device_context` + `SetTarget` 経路（既存の利用パターン）で代替されており機能喪失はない。P33（録画モジュールの完成または削除)と同じ d2d/mod.rs 周辺のため統合実施も可。

## P36: RecCommandSink の COM コールバックにおける `Ref::unwrap()` panic の COM ABI 境界越え
- source: W2-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/com/d2d/command_sink.rs` の ID2D1CommandSink 系コールバック実装は、COM インターフェイス引数（brush / geometry / bitmap / image / fontFace 等、約 27 箇所）を `Ref<T>::as_ref().unwrap()` で取り出している。D2D1 のコマンドリスト再生では必須引数は非 null だが、万一 null が渡った場合（または将来 Stream 以外の経路から直接呼ばれた場合）、unwrap panic は `#[implement]` が生成する extern "system" vtable シム内で発生し、Rust の extern ABI 境界での unwind は即時 abort となる（HRESULT としてエラーを返す COM の作法に反し、プロセス全体が落ちる）。`Ref::ok()?`（null → E_POINTER 返却）への置換はエラー戻り値という外部観測可能な挙動の変更にあたるため、R2.4/R5.2 に従い本ループでは非 null 前提の SAFETY 根拠コメントと値構造体ポインタへの debug_assert 付加（W2-V 実施）に留めた。なお本モジュールは利用ゼロの未完成コード（P33）であり、P33 で削除が選択される場合は本提案は消滅する。
- suggestion: P33 の「完成」パスを選ぶ場合、全コールバックの `xxx.as_ref().unwrap()` を `xxx.ok()?` へ置換し、null COM 引数を panic/abort ではなく E_POINTER の HRESULT として呼び出し元（D2D1 ランタイム）へ返却する小規模修正。`tests/com/d2d_ext_test.rs` の Stream 再生テストが正常系の回帰検知器として利用できる。P33 で削除が選択される場合は本提案も同時にクローズする。

## P37: composite_render_system のデバッグ残置コード除去（赤デバッグ枠の常時描画・DIB 全画素スキャン）
- source: W3a-T
- kind: その他
- rationale: `crates/wintf/src/ecs/graphics/compositor_systems/render.rs` の `composite_render_system` に切り分け用デバッグコードが 2 件残置されている。(1) 合成ビットマップ外周に 2px の赤枠を無条件描画するブロック（コメントに「DEBUG: レイアウト由来 vs 描画由来の切り分け用」と明記。`cfg(debug_assertions)` ガードもなくリリースビルドでも全 ULW ウィンドウに赤枠が表示される、ユーザー可視の描画アーティファクト）。(2) DIB ピクセルダンプブロックが `buf.chunks(4).position(...)` / `.count()` で全画素（W×H）を毎合成スキャンしてから `trace!` に渡しており、トレース無効時もスキャン自体は実行される（100x100 で 1 万画素、フル HD 級なら 200 万画素/フレームの無駄な CPU コスト）。いずれも除去は外部観測可能な挙動（描画結果・性能特性）の変更にあたるため R5.1 に従い本ループでは実装せず、現行挙動を characterization テスト（`tests/graphics/compositor_render_system_test.rs::composite_render_sets_dirty_and_draws_debug_border` が赤枠ピクセル [0,0,255,255] を固定）で記録した。
- suggestion: 赤枠描画ブロックと DIB ピクセルダンプブロック（または少なくとも全画素スキャン部分）を削除する小規模修正。デバッグ用途を残す場合は `cfg(debug_assertions)` + `tracing::enabled!(Level::TRACE)` ガード下へ移動する。削除時は上記 characterization テストの赤枠アサーションを「外周も透明クリアのまま」へ更新する（テスト弱体化ではなく仕様変更追随）。

## P38: ClipGuard の geometricMask `transmute` による COM 参照リーク修正（角丸クリップの毎フレームリーク）
- source: W3a-T
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/graphics/compositor_systems/render.rs` の `ClipGuard::push` は、`RoundedRectangle` / `RoundedRectangleIndividual` バリアントで `D2D1_LAYER_PARAMETERS1 { geometricMask: unsafe { std::mem::transmute(Some(geo_mask)) }, .. }` と記述している。`geometricMask` の型は `ManuallyDrop<Option<ID2D1Geometry>>`（windows-rs 0.61 で確認）であり、`transmute`（move）で所有権ごと `ManuallyDrop` に移されるため、`layer_params` のドロップ時に Release が走らず、`factory.create_rounded_rectangle_geometry` / `create_path_geometry` で生成した COM オブジェクトが push 1 回につき 1 個リークする。角丸クリップ付き Visual を持つ ULW ウィンドウは再合成のたびにジオメトリを新規生成するため、アニメーション等で毎フレーム再合成される場合は無制限にメモリが増加する。修正は unsafe コードの変更を伴い R5.1（テスト追加のみ）の範囲外のため、本ループでは ULW クリップの観測挙動（角の透明化）のピクセル固定テスト（`rounded_rectangle_clip_clears_corners` / `individual_corner_clip_applies_per_corner_radii`）の追加に留めた。
- suggestion: `geo_mask` の束縛を PushLayer 呼び出しまで生存させたうえで `std::mem::transmute_copy(&geo_mask)`（借用コピー）へ置換する、または `ManuallyDrop::new(Some(geo_mask))` で構築し PushLayer 後に `ManuallyDrop::into_inner` で回収して Release させる小規模修正。`opacityBrush: std::mem::zeroed()` も `ManuallyDrop::new(None)` へ置換すると意図が明確になる。描画結果は不変のため既存のクリップピクセルテストがそのまま回帰検知器になる。

## P39: render_surface の未使用システムパラメータ削除
- source: W3a-S
- kind: ロジック変更を要する簡素化
- rationale: `crates/wintf/src/ecs/graphics/systems/render.rs` の `render_surface` は、関数本体で一切使用しないシステムパラメータを 2 件受け取っている（`_graphics_core: Option<Res<GraphicsCore>>` と `_frame_count: Res<FrameCount>`。後者はコメントアウトされた eprintln デバッグログの名残）。削除自体は数行だが、ECS システムのリソースアクセス集合の変更にあたり、(a) `Res<FrameCount>` 必須要求の消失により FrameCount 未挿入ワールドでの実行時 panic 有無が変わる、(b) スケジューラから見たアクセスセットが変わる、という外部観測可能な性質の変更を含むため、R5.1/R5.3 に従い W3a-S では実施せず記録に留めた。
- suggestion: 両パラメータをシグネチャから削除する小規模修正。`tests/graphics/surface_systems_test.rs` の render_surface 系テスト 5 件（begin_draw→clear→DrawImage→end_draw 経路・invalid スキップ）がそのまま回帰検知器になる。`commit_composition` 側の `frame_count` はエラーログで使用中のため対象外。

## P40: ULW 合成経路のデバイスロスト検出の欠如（GraphicsCore::invalidate() の発火経路がプロダクションに不在）
- source: W3a-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: デバイスロスト復旧機構（`init_graphics_core` の再作成 → `invalidate_dependent_components` の一括無効化 → `compositor_init_system` の再作成）は `GraphicsCore::is_valid() == false` を起点とするが、ワークスペース全 grep の結果、プロダクションコードに `GraphicsCore::invalidate()` の呼び出しが存在しない（呼び出しはテストと examples/graphics_reinit_test.rs のみ）。このため GPU リセット等でデバイスが失われると、`composite_render_system` の `EndDraw` / `CopyFromBitmap` が D2DERR_RECREATE_TARGET 系エラーを毎フレーム返し続けるが error ログのみで復旧機構は永久に発火せず、ULW ウィンドウは最終提示フレームのまま恒久的に固まる（可用性の縮退。panic はせず DoS クラッシュには至らない — unwrap/expect の点検で当該経路に panic 経路がないことは確認済み）。DComp モード側も同根で、Phase 2 以降 `invalidate_dependent_components` は WindowGraphics/VisualGraphics/SurfaceGraphics を無効化対象から外しており、検出が入っても DComp 側の再初期化トリガーは別途設計を要する。HRESULT 検査によるデバイスロスト検出の追加はエラー処理の挙動変更のため R2.4/R5.2 に従い本ループでは NOTE コメント（compositor_systems/render.rs の EndDraw エラー経路）に留めた。
- suggestion: `composite_render_system` の EndDraw / CopyFromBitmap 失敗時に HRESULT を検査し、D2DERR_RECREATE_TARGET（および DXGI_ERROR_DEVICE_REMOVED/RESET。`d3d11.rs` の `GetDeviceRemovedReason` ラッパーが既存）の場合に `GraphicsCore::invalidate()` を呼ぶ小規模仕様。`invalidate_dependent_components` の無効化対象に DComp 系コンポーネント（WindowGraphics/VisualGraphics/SurfaceGraphics）を再追加するか否かの設計判断を含める。既存の `device_lost_recreates_compositor_with_generation_carryover` テスト（tests/graphics/compositor_init_system_test.rs）が復旧側の回帰検知器になる。

## P41: compositor_init_system の負サイズ入力の事前検証（i32 → u32 ラップによる巨大サイズ生成試行の解消）
- source: W3a-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `compositor_systems/init.rs` は `WindowPos.size`（`SizeI`、i32）を `as u32` で変換するため、負値はラップして巨大値になる（例: -1 → 4294967295）。`w == 0 || h == 0` ガードは負値を捕捉せず、巨大サイズで `WindowD3D11Compositor::new` が呼ばれ、D2D CreateBitmap の最大ビットマップサイズ超過 Err → error ログとなる（panic / UB なし。`tests/graphics/compositor_init_system_test.rs::negative_window_pos_size_does_not_create_compositor_and_does_not_panic` で特性化済み）。毎フレーム Changed が立つ構成では無駄な生成試行とエラーログのスパムになり得る。負値の事前スキップ（`size.width <= 0` で continue）は「エラーログ出力 → 無音スキップ」というログ挙動の変更を伴うため、R2.4/R5.2 に従い本ループでは NOTE コメントと特性化テストに留めた。
- suggestion: ガードを `size.width <= 0 || size.height <= 0` に変更し（u32 変換前に i32 のまま検査）、ゼロ・負値を同一の「無効サイズスキップ」経路に統一する 1 行規模の修正。特性化テストのアサーションはそのまま回帰検知器になる（compositor 未作成という観測結果は不変）。

## P42: create_dib_section の GDI エラー経路整備（契約上到達不能なリーク経路の解消・SelectObject 失敗検査）
- source: W3a-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `compositor.rs::create_dib_section` に 2 件のエラー経路の不備がある。(1) `CreateDIBSection` が有効な HBITMAP を返しつつ `dib_bits` が null の場合のガードは Err を返すが hbitmap を解放しない（GDI ハンドル 1 個のリーク）。ただし CreateDIBSection の成功契約上、有効戻り値なら ppvBits も非 null が設定されるため、この組み合わせは API 契約上到達不能。(2) `SelectObject` の失敗（戻り値 null / HGDI_ERROR）を検査しておらず、万一失敗すると memory_dc は 1x1 ストックビットマップのまま ULW へ渡り、空内容が黙って提示される（実用上、直前で有効性確認済みの DC + DIBSECTION では失敗しない）。いずれも修正はエラー経路の挙動変更（解放処理追加・新 Err 分岐）にあたるため、R2.4/R5.2 に従い本ループでは NOTE コメント 2 箇所の付記に留めた。発生確率・実害ともに極小のため優先度は低（P30 の WinThreadMgr エラー経路整備と同系統で統合実施可）。
- suggestion: (1) null-bits ガードに `DeleteObject(hbitmap)` を追加。(2) SelectObject の戻り値を検査し、失敗時は memory_dc/hbitmap を解放して Err を返す。既存の lifecycle テスト（new の 4 リソース作成・Drop 解放）が正常系の回帰検知器になる。

## P43: SetWindowPosCommand キューのテスト観測 API 追加（apply_window_pos_changes の出力検証を可能にする）
- source: W3b-T
- kind: その他
- rationale: `crates/wintf/src/ecs/graphics/systems/window_pos.rs` の `apply_window_pos_changes` の唯一の出力は `SetWindowPosCommand::enqueue`（`crates/wintf/src/ecs/window/command.rs` の thread-local `RefCell<Vec<_>>` キューへの push）だが、キュー内容を覗き見る公開 API が存在せず、取り出し手段は実 `SetWindowPos` Win32 呼び出しを伴う `flush()` のみ。このため「クライアント→ウィンドウ座標変換の結果」「`build_flags_for_system` のフラグ合成」「ドラッグ中の `SWP_NOMOVE` 強制」という本システムの中核出力をテストで検証できない。W3b-T では CW_USEDEFAULT スキップ・座標変換フォールバックの 2 分岐を完走 characterization（`tests/graphics/window_pos_systems_test.rs`）で固定したが、enqueue 内容のアサーションは不可能であり、ドラッグ中 `SWP_NOMOVE` 経路はテスト自体を見送った。観測 API の追加はプロダクションコードの変更にあたるため R5.1 に従い本ループでは実施せず記録に留めた。
- suggestion: `SetWindowPosCommand` に `take_pending() -> Vec<SetWindowPosCommand>`（または `with_pending(|cmds| ...)` 形式の read-only peek）を追加する小規模修正。TLS キューのため統合テストから安全に呼べ、実 SetWindowPos を伴わずに enqueue 内容（座標・サイズ・フラグ・hwnd_insert_after）を検証できる。追加後、W3b-T の characterization 3 件を内容アサーション付きに強化し、ドラッグ中 `SWP_NOMOVE` 強制のテストを新設する。

## P44: resolve_inherited_brushes のフィールド単位継承解決（部分解決親のデフォルト落ち解消）
- source: W3b-T
- kind: 挙動変更を伴う改善提案
- rationale: `crates/wintf/src/ecs/graphics/systems/brushes.rs` の `find_parent_brushes` は「foreground/background の両方が非 Inherit」の祖先のみを継承元として返すため、片フィールドだけ解決済みの親（例: `Brushes::with_foreground(RED)` — 背景は Inherit）を持つ子は、親の解決済みフィールド（前景 RED）すら継承せず、さらに上位に完全解決の祖先が無ければデフォルト（前景=黒・背景=透明）に落ちる。「親に前景だけ赤を設定したのに子の前景が黒になる」のは継承の直感（CSS 的なフィールド単位カスケード）に反し得る。現行挙動は W3b-T 追加の characterization テスト（`tests/graphics/brushes_system_test.rs::partially_resolved_parent_without_resolved_ancestor_yields_defaults` / `partially_resolved_parent_is_skipped_for_grandparent`）で固定済み。解決規則の変更は描画色という外部観測可能な挙動の変更にあたるため、R5.1 に従い本ループでは実施せず記録に留めた。なお親側がウィジェット経由で生成される通常フローでは `BrushInherit` マーカーにより親自身が先行フレームで完全解決されるため、実害が出るのは「マーカーなしで部分解決 Brushes を直接挿入した親」という限定的な構成に限られる（優先度は低）。
- suggestion: `find_parent_brushes` を「フィールドごとに最初の非 Inherit 値を採用する」走査（前景・背景を独立にカスケード解決）へ変更する小規模修正。`resolve_brush_fields` 側は親の該当フィールドが `as_color()` Some の場合のみ採用する既存構造をほぼ流用できる。変更時は上記 characterization テスト 2 件の期待値を新仕様（親の解決済みフィールドを継承）へ更新する（テスト弱体化ではなく仕様変更追随）。

## P45: visual_resource_management_system の未使用 Commands パラメータ削除
- source: W3b-S
- kind: ロジック変更を要する簡素化
- rationale: `crates/wintf/src/ecs/graphics/visual_manager.rs` の `visual_resource_management_system` は `Commands` パラメータを受け取るが、Phase 6（Surface 作成の Draw スケジュール遅延移行）以降、関数本体で一切使用していない。W3b-S では私有ヘルパー `create_visual_only` 側の未使用 `_commands: &mut Commands` パラメータは削除済み（内部シグネチャのみの構造的整理）だが、システムパラメータ自体の削除は ECS スケジューラから見たアクセスセットの変更（外部観測可能な性質の変更）にあたるため、P39（render_surface の同系統事案）と同じ判断基準により R5.1/R5.3 に従い `_commands` への改名と NOTE コメント付記に留めた。
- suggestion: `_commands: Commands` をシグネチャから削除する 1 行規模の修正。P39 と同一仕様で一括実施するのが効率的。`tests/visual/component_test.rs`・`tests/visual/graphics_auto_creation_test.rs`・`tests/visual/resource_management_gap_test.rs` の既存テスト群がそのまま回帰検知器になる。

## P46: apply_window_pos_changes の重複 debug ログ統合
- source: W3b-S
- kind: ロジック変更を要する簡素化
- rationale: `crates/wintf/src/ecs/graphics/systems/window_pos.rs` の `apply_window_pos_changes` は、1 回の enqueue につき内容が重複する `debug!` ログを 2 回出力している（enqueue 直前の `[apply_window_pos] Enqueue SetWindowPos`（client/window 両座標を含む完全版）と enqueue 直後の `[apply_window_pos_changes] Command enqueued`（client 座標のみの劣化版））。後者は前者の部分集合であり情報価値がないが、ログ出力はトレーシング購読者から観測可能な挙動のため、R5.1 に従い本ループでは削除せず記録に留めた。
- suggestion: 後段の `Command enqueued` ログを削除（または前段に統合）する数行規模の修正。`tests/graphics/window_pos_systems_test.rs` の characterization テストはログを検証していないため影響なし。P43（SetWindowPosCommand キュー観測 API）と同一ファイルのため統合実施可。

## P47: visual_hierarchy_sync_system の再ペアレント未検出（parent_visual キャッシュ方式の盲点）
- source: W3b-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/graphics/systems/visual_sync.rs` の `visual_hierarchy_sync_system` は未同期検出を `parent_visual().is_none()` のみで行うため、既に親 Visual を持つ子の ChildOf 変更（再ペアレント）を検出しない（ドキュメントコメントの「ChildOf変更...を検出」という主張と実装が乖離）。再ペアレント直後は DComp Visual が旧親に接続されたまま ECS 階層と乖離し（画面上は旧親配下に描画され続ける）、その後旧親側が別の未同期子により再同期されると `remove_all_visuals` で切り離されるが新親へは再接続されないため Visual が画面から消失し、`parent_visual` キャッシュは実体（どの親にも属さない孤立 Visual）と乖離した旧親参照を保持し続ける。既存テスト `test_childof_change_moves_visual_to_new_parent`（tests/visual/hierarchy_sync_test.rs）は `parent_visual().is_some()` しか検証しないため本ギャップを見逃していた（テスト名と検証内容の乖離）。現行挙動は `tests/visual/hierarchy_reparent_gap_test.rs` の特性化テスト 2 件（`Interface::as_raw` によるポインタ同一性検証）で固定済み。なお現行コードベースに実行時再ペアレントを行うプロダクション経路は確認できず（grep）、潜在バグの段階。検出条件の変更は挙動変更のため R5.2 に従い本ループでは NOTE コメントとテスト固定に留めた。
- suggestion: (a) ChildOf の置換を検知して `parent_visual` キャッシュを None にリセットする（既存の「未同期」検出にそのまま乗る。リセット時に旧親から `remove_visual` も行うとより正確）、または (b) Phase 1 の収集条件を「parent_visual が None、または現在の ChildOf.parent() の VisualGraphics と不一致」へ拡張する数行〜十数行規模の修正。修正時は hierarchy_reparent_gap_test の期待値を新仕様（新親への再接続）へ更新し（テスト弱体化ではなく仕様変更追随）、`test_childof_change_moves_visual_to_new_parent` をポインタ同一性アサーション付きへ強化する。

## P48: ChildOf 祖先走査の巡回ガード欠如（巡回階層での無限ループによる UI スレッドハング）
- source: W3b-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: wintf の祖先走査 3 箇所 — `visual.rs::find_owner_window_composition_mode`（`on_visual_add` フックから同期的に呼出）、`systems/visual_sync.rs` の親深さ計算ループ、`systems/brushes.rs::find_parent_brushes` — はいずれも ChildOf チェーンを終端まで辿るが巡回ガードを持たない。bevy_ecs 0.18 の Relationship 管理は自己参照（A→A）を警告付きで除去する（bevy_ecs-0.18.0/src/relationship/mod.rs:125 で確認）が、間接巡回（A→B→A）は構築可能であり、その場合これらのループは無限ループとなり UI スレッドが恒久的にハングする（可用性 DoS。特にフック内の走査はエンティティ挿入と同期的に走るため影響範囲が大きい）。通常の API 使用（spawn 時の親子付け・despawn・Relationship 経由の付替え）では巡回は生成されないため発生確率は極小。ガード追加（深さ上限超過時の warn + 打ち切り）はエラー処理の挙動変更にあたるため、R5.2 に従い本ループでは 3 箇所への NOTE コメント付記に留めた。優先度は低。
- suggestion: 3 箇所に共通の深さ上限（例: 10_000 — 実用階層深さの数桁上）を導入し、超過時は `warn!` ログ + 打ち切り（None 返却 / 該当エンティティのスキップ）とする数行規模の修正。既存の hierarchy_sync / brushes / find_owner_composition_mode 系テストがそのまま正常系の回帰検知器になる。

## P49: LengthPercentageAuto / LengthPercentage の taffy 変換でパーセント正規化（÷100）が欠落
- source: W4a-T
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/layout/dimension.rs` の 3 つの寸法型はいずれもドキュメントコメントで「パーセント値は **0.0～100.0** の範囲で指定」「変換は自動的に行われます」と謳うが、taffy 変換の実装が不整合。`Dimension::Percent(v)` は `taffy::Dimension::percent(v / 100.0)`（正規化あり、dimension.rs:104）なのに対し、`LengthPercentageAuto::Percent(v)` → `taffy::LengthPercentageAuto::percent(v)`（dimension.rs:172）と `LengthPercentage::Percent(v)` → `taffy::LengthPercentage::percent(v)`（dimension.rs:218）は ÷100 を行わない。taffy は 1.0 = 100% を期待するため、ドキュメント通りに `BoxMargin`/`BoxPadding`/`BoxInset` へ `Percent(50.0)` を指定すると 5000% として解釈され、レイアウトが大きく破綻する。現状リポジトリ内の利用は Px のみのため潜在バグ（margin/padding/inset の Percent 指定経路は未使用）。修正はレイアウト結果という外部観測可能な挙動の変更にあたるため、R5.1 に従い本ループでは現状挙動の特性化テスト（`tests/layout/dimension_conversion_test.rs::test_length_percentage_auto_percent_to_taffy_not_normalized` / `test_length_percentage_to_taffy`）で固定するに留めた。
- suggestion: 2 箇所の変換に `/ 100.0` を追加する 2 行規模の修正（`Dimension` と同一仕様に統一）。修正時は上記特性化テスト 2 件の期待値を正規化後の値（`percent(0.5)` 等）へ更新する（テスト弱体化ではなく仕様変更追随）。`Dimension::Percent` の正規化テスト（`test_dimension_percent_to_taffy_normalized`）が整合確認の対照になる。

## P50: From<taffy::Dimension> for Dimension のスタブ実装（常に Auto を返す）の実装または削除
- source: W4a-T
- kind: その他
- rationale: `crates/wintf/src/ecs/layout/dimension.rs:110-118` の `From<taffy::Dimension> for Dimension` は、taffy の内部表現（CompactLength）からの値取り出しを実装せず、入力に関わらず常に `Dimension::Auto` を返すスタブ（ソース内 TODO 明記済み）。`taffy::Dimension::length(100.0)` を変換しても `Auto` になるため、ラウンドトリップが成立しない。grep で確認した範囲ではプロダクションコードからの呼び出しは存在せず実害はないが、公開 trait 実装として誤用リスクがある（呼び出し側はコンパイルエラーにならず黙って Auto を受け取る）。現状挙動は `tests/layout/dimension_conversion_test.rs::test_dimension_from_taffy_is_stub_returning_auto` で特性化済み。実装変更（正確な変換の実装）も削除（公開 API の除去）も外部観測可能な変更のため、R5.1 に従い本ループでは記録に留めた。
- suggestion: (a) `taffy::Dimension` の `into_option()` / タグ判定 API（taffy 0.9 の CompactLength アクセサ）を用いて Px/Percent（×100 復元）/Auto を正確に逆変換する実装を追加する、または (b) 未使用のため trait 実装ごと削除する。(a) を採る場合は P49 と同時に実施し、正規化の往復一貫性テスト（Px/Percent/Auto のラウンドトリップ）を追加するのが効率的。

## P51: BitmapSourceResource のテスト用コンストラクタ追加（AlphaMask ヒットテスト変換ロジックの単体到達）
- source: W4b-T
- kind: その他
- rationale: `crates/wintf/src/ecs/layout/hit_test/mod.rs` の AlphaMask モード（`hit_test_entity` mod.rs:218-249 / `hit_test_entity_ex` mod.rs:350-377）は、矩形通過後に screen→mask 正規化（`rel = (point - bounds.left)/bounds_width`）と mask 座標化（`(rel * alpha_mask.width()) as u32`）を経て `AlphaMask::is_hit` を呼ぶ。この座標変換ロジック自体は純粋（デバイス非依存）だが、到達には `BitmapSourceResource` に `set_alpha_mask` 済みの実体が必要で、唯一の公開コンストラクタ `BitmapSourceResource::new(source)`（`ecs/widget/bitmap_source/resource.rs:26`）が実 `IWICBitmapSource` を要求する（COM/WIC 初期化が前提）。このため W4b-T の in-source テストは「BitmapSourceResource なし/αマスク未生成」のフォールバック経路のみを固定し、変換本体（rel→mask の `as u32` 切り捨て・bounds 原点減算の結合）と AlphaMask 退化 bounds 分岐（mod.rs:236-238）には未到達のまま残った。layout ドメインテストへ COM/WIC 依存を持ち込むのは過剰なため本ループでは見送り（`AlphaMask::from_pbgra32`/`is_hit` 単体は bitmap_source/alpha_mask.rs の in-source 10件で網羅済み）。API 追加＝公開面の変更のため R5.1/R2.4 に従い提案記録に留める。
- suggestion: `BitmapSourceResource` に WIC ソース不要のテスト専用コンストラクタ（例: `#[cfg(any(test, feature = "test-util"))] pub fn for_test_with_alpha_mask(mask: AlphaMask) -> Self`、`source` は遅延/Option 化または最小ダミーで保持）を追加する小規模仕様。これにより hit_test 統合層の AlphaMask 座標変換と退化 bounds フォールバックをデバイス非依存で特性化できる。あわせて W4b-T 所見1/3 の未到達分岐を埋めるテストを追加する。

## P52: hit_test_entity / hit_test_entity_ex の重複統合（NamedRegions の挙動差異の整合が前提）
- source: W4b-S
- kind: ロジック変更を要する簡素化
- rationale: `crates/wintf/src/ecs/layout/hit_test/mod.rs` の `hit_test_entity`（mod.rs:164-250, 戻り値 `bool`）と `hit_test_entity_ex`（mod.rs:296-418, 戻り値 `RegionHit`）は、モード解決・None早期return・GlobalArrangement取得・`bounds.contains` 判定・Bounds 合成α判定（`ALPHA_THRESHOLD = 128/255`, opacity × foreground.a）・AlphaMask 座標変換（screen→mask 正規化と `is_hit`）まで本体がほぼ完全に重複している（差分は戻り値型と `RegionHit` の包み方のみ）。本来は `hit_test_entity` を `matches!(hit_test_entity_ex(..), RegionHit::Hit(_))` の薄いラッパーへ縮約すれば約 60 行の重複を除去できる。**ただし両者には観測可能な挙動差が1点ある**: 非ex版は `mode == NamedRegions` のとき `if mode == Bounds` ブロックを素通りして **AlphaMask 経路**（BitmapSourceResource 取得 → 不在時フォールバック `true`、mod.rs:218-249）を実行するのに対し、ex版は `NamedRegions` を独立 match アームとして `HitRegionMap` で判定する（mod.rs:378-416）。このため `HitTestMode::NamedRegions` のエンティティを `hit_test_entity`（非ex）に通すと、ex への委譲化で結果が変わり得る（非ex は現状 AlphaMask フォールバックで `true`、委譲後は HitRegionMap 参照によりリージョン外なら `false`）。この差異は外部観測可能な挙動変更（R5.1）にあたり、現状この経路を固定するテストは存在しない（`hit_test/tests.rs` に NamedRegions ケースなし）が、未テストでも挙動を変えてはならないため本ループでは統合を見送った。なお Bounds 合成α判定・AlphaMask 座標変換の各ブロック単体は `tests_ex.rs`（α境界 128/255 上下・低 foreground α・inherit、AlphaMask フォールバック各種）で特性化済みのため、挙動を揃えたうえでの抽出は安全に行える状態にある。
- suggestion: まず非ex `hit_test_entity` の NamedRegions 挙動を ex 版に合わせる（NamedRegions を AlphaMask 経路ではなく HitRegionMap 判定へ正す）か、または非ex を `hit_test_entity_ex` への委譲（`RegionHit::Hit(_) => true / Miss => false`）へ置き換えて挙動を一本化する小規模仕様。一本化に先立ち、非ex の NamedRegions 経路を固定する特性化テスト（HitRegionMap あり/なし × リージョン内/外）を `hit_test/tests.rs` に追加して挙動差を明示し、新仕様の期待へ更新する。委譲化により共有の Bounds α判定ヘルパ・AlphaMask 変換ヘルパの抽出も不要（重複が構造的に消える）となる。

## P53: ColorMapData::from_image の画像寸法に対する整数オーバーフロー検証の欠如（外部 PNG 由来 u32 乗算）
- source: W4b-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/layout/hit_region/mod.rs::ColorMapData::from_image` は、外部 PNG 画像（`HitRegionMap::from_color_map` / `from_image` に渡される `image_path`）からデコードした幅・高さ `(width, height): (u32, u32)`（`converted.get_size()`、mod.rs:178）を用いて `stride = width * 4`（:179）、`buffer_size = (stride * height) as usize`（:180）、`pixel_count = (width * height) as usize`（:187）を計算する。これらは u32 同士の乗算であり、巨大な画像寸法（例: 65536×65536 → `width*height = 2^32`、`stride*height = 2^34`）では u32 範囲を超える。Rust の算術は **デバッグビルドでは桁あふれで panic**（外部ファイル由来データから到達可能な DoS panic 経路）、**リリースビルドでは黙ってラップ**して `buffer`/`index_map` が過小確保となり、後続の `copy_pixels`（:184）へ不整合な stride/バッファを渡す（WIC がエラーを返す可能性が高いが、サイレントな整数切り詰めのハザードが残る）。`index_map` 添字アクセス（:225）自体は範囲チェック＋不変条件（本セルで debug_assert 追記）により安全だが、確保段階の乗算オーバーフローはその手前にある。なお本経路は実 WIC/COM 初期化を要し（P51 と同根でユニット到達不能）、リポジトリ内の実利用画像は小サイズのため現状実害は未発現。対策（`checked_mul` による事前検証＋寸法過大時の新エラー応答、または `usize` 昇格後の乗算）は `from_image` の戻り値（`windows::core::Result` への新規エラー経路追加）という外部観測可能な挙動の変更を伴う（デバッグ panic→Err 化も観測挙動変更）ため、R2.4/R5.2 に従い本ループでは実装せず記録に留めた。
- suggestion: `from_image` で寸法乗算を `usize` への昇格後に行う（`(width as usize) * (height as usize)` 等。u32→usize は 64bit で無損失）か、`width.checked_mul(4)` / `checked_mul(height)` で事前検証し、オーバーフロー時・過大寸法時は専用エラー（`HitRegionError` への新バリアント、または既存 `ImageLoadFailed` へ寸法エラーを写像）を返す小規模仕様。あわせて最大ビットマップ寸法の上限チェックを設けると WIC 側の二次失敗も前倒しできる。P51（テスト用コンストラクタ）が整えば本経路の境界（極大寸法での Err 返却）を特性化可能となるため、P51 と統合実施が望ましい。

## P54: convert_to_timeline の純粋ロジック分離（DirectWrite 非依存なタイムライン構築の単体到達）
- source: W5a-T
- kind: その他
- rationale: `crates/wintf/src/ecs/widget/text/typewriter_layout.rs::convert_to_timeline`（typewriter_layout.rs:188-241）は、Stage1 IR（`TypewriterToken` 列）と `Typewriter`（default_char_wait）から Stage2 IR（`TypewriterTimeline`）を構築する。本体ロジック（Text トークンの文字数ぶん Glyph を `default_char_wait` 累積時刻で生成、Wait の `start_at`/`current_time` 更新、FireEvent の `fire_at` 記録、`cluster_index < total_cluster_count` による打ち切り）は **デバイス非依存な純粋計算** だが、関数シグネチャが `text_layout: &IDWriteTextLayout` を受け取り冒頭で `text_layout.get_cluster_metrics()?`（実 DirectWrite TextLayout を要する COM 呼び出し）から `total_cluster_count` を得るため、この関数全体がユニットテスト不能になっている。`total_cluster_count` さえ与えられればトークン→タイムライン変換は決定的で、time 累積・cluster_index 打ち切り・各 TimelineItem 生成順序を特性化できる。同関数はリポジトリ内で `init_typewriter_layout` から1箇所のみ呼ばれる private fn。リファクタ（純粋内側関数の抽出）自体は観測挙動を変えない見込みだが、device 依存システムファイル内の構造変更であり、判断に迷う構造変更は提案へ回す方針（タスク指示）に従い本ループでは見送り、現行は呼び出し側システムを通じた間接実行（実 DirectWrite を要する統合テスト）のみが回帰検知器となる状態に留めた。
- suggestion: `convert_to_timeline` を `convert_to_timeline_inner(tokens: &[TypewriterToken], default_char_wait: f64, total_cluster_count: u32) -> TypewriterTimeline`（純粋・COM 非依存）と、`get_cluster_metrics()` を呼んで `total_cluster_count` を渡す薄い外側関数に分割する小規模リファクタ。抽出後、トークン列（Text/Wait/FireEvent 混在）に対する Glyph 累積時刻・cluster_index 打ち切り（total_cluster_count 未満ぶんのみ Glyph 生成）・total_duration を `tests/widget/` のデバイス非依存テストで特性化できる（W5a-T で TypewriterTalk::update の timeline 消費側は固定済みのため、生成側と消費側が両端から保護される）。

## P55: generate_alpha_mask_system の画像寸法に対する整数オーバーフロー検証の欠如（外部画像由来 u32 乗算）
- source: W5b-T
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/widget/bitmap_source/systems.rs::generate_alpha_mask_system`（systems.rs:393-414）は、WIC でデコードした画像の幅・高さ `(width, height): (u32, u32)`（`source.get_size()`、:393）から `stride = width * 4`（:402）、`buffer_size = (stride * height) as usize`（:403）を u32 同士の乗算で計算し、`vec![0u8; buffer_size]` を確保したうえで `source.copy_pixels(None, stride, &mut buffer)` へ渡す。巨大な画像寸法（例: 33000×33000 → `width*4*height ≈ 4.36e9 > u32::MAX`）では u32 範囲を超え、**デバッグビルドでは桁あふれで panic**（外部画像ファイル由来データから到達可能な DoS panic 経路）、**リリースビルドでは黙ってラップ**して `buffer` が過小確保となり、`copy_pixels` へ不整合な stride/バッファ長を渡す（WIC が `WINCODEC_ERR_INSUFFICIENTBUFFER` を返す可能性が高いが、サイレントな整数切り詰めのハザードが残る）。これは W4b-V の P53（`hit_region/mod.rs::ColorMapData::from_image` の同型オーバーフロー）と**同一クラスの別箇所**であり、ヒットテスト用 αマスク生成経路に存在する。本経路は実 WIC/COM 初期化＋実画像デコードを要し（αマスク生成は `Added<BitmapSourceResource>` 駆動で `IWICBitmapSource` 実体が前提）ユニット到達不能なため、W5b-T では `AlphaMask::from_pbgra32` 以降の純粋ビットパック側のみ特性化した（width=0/padded stride/byte 境界など 5 件追加）。対策（`checked_mul` による事前検証＋寸法過大時のスキップ/エラー、または `usize` 昇格後の乗算）は当該システムの挙動（panic→スキップ化、または新たな警告ログ）という外部観測可能な変更を伴うため、R2.4/R5.2 に従い本ループでは実装せず記録に留めた。なおリポジトリ内の実利用画像は小サイズ（8x8〜16x16）のため現状実害は未発現。
- suggestion: `stride`/`buffer_size` 計算を `usize` への昇格後に行う（`(width as usize) * 4 * (height as usize)`。u32→usize は 64bit で無損失）か、`width.checked_mul(4).and_then(|s| s.checked_mul(height))` で事前検証し、オーバーフロー時・過大寸法時は当該エンティティの αマスク生成をスキップしてエラーログを出力する小規模仕様。P53（hit_region 側）と同一の「外部画像寸法の整数オーバーフロー検証」方針で統合実施が望ましい。あわせて最大ビットマップ寸法の上限チェックを設けると WIC 側の二次失敗（`copy_pixels` の巨大確保）も前倒しできる。

## P56: BitmapSource 画像パスのパストラバーサル検証の欠如（resolve_path の相対パス無検証 join）
- source: W5b-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/widget/bitmap_source/systems.rs::resolve_path`（systems.rs:38-51）は、`BitmapSource { path: String }`（ユーザー/呼び出し側提供の画像パス）を受け取り、相対パスを `current_exe().parent().join(path)` で実行ファイルディレクトリ配下へ解決する。この join は `..` を含む相対パス（例 `../../secret.png`）を一切検証せず、結合後の正規化パスが実行ファイルディレクトリの外側を指すことを許す（パストラバーサル）。絶対パス（例 `C:\Windows\...`）もそのまま通過する。解決後のパスは `load_bitmap_source` で WIC `create_decoder_from_filename` に渡され、`GENERIC_READ` でファイルが開かれてデコードされる。すなわち BitmapSource の path を外部から制御できる経路（将来的に設定ファイル・スクリプト・ネットワーク由来でウィジェットを構築する構成）では、意図しないファイルの読み取り（情報開示）やシンボリックリンク経由の到達が可能になる。現状リポジトリでは path は開発者が埋め込む定数（モックシェル画像）であり実害は未発現だが、`resolve_path` 自体は任意文字列を受け付け検証を行わない。検証追加（`..` コンポーネント拒否・解決後パスが基準ディレクトリ配下にあることの確認・絶対パス許可ポリシーの明文化・シンボリックリンク追跡の制限）は、従来解決できていた相対/絶対パスを弾くようになる外部観測可能な挙動変更（`resolve_path` の戻り値が `Ok` → `Err` へ変化、エラー応答の追加）を伴うため、R2.4/R5.2 に従い本ループでは実装せず記録に留めた。現行挙動（絶対そのまま返却・相対は exe ディレクトリ基準 join・サブディレクトリ保持）は W5b-T 追加の `resolve_path` 特性化テスト 3 件で固定済み。
- suggestion: `resolve_path` に基準ディレクトリ閉じ込め（jail）検証を追加する小規模仕様。候補: (1) 相対パスに `Component::ParentDir`（`..`）が含まれる場合を拒否、(2) 解決後に `std::fs::canonicalize` して基準ディレクトリ（`current_exe().parent()`）の `starts_with` を確認、(3) 絶対パスの許可可否をポリシーとして明文化（アセットは常に相対のみ許可する等）。`resolve_path` の戻り値型は既に `std::io::Result` のため新エラーバリアントは不要（`ErrorKind::InvalidInput`/`PermissionDenied` で表現可）。実装時は W5b-T の特性化テスト 3 件のうち絶対パス系を新ポリシーの期待（拒否 or 許可）へ更新し、`..` トラバーサル拒否の回帰テストを追加する。P3（areka 側のアセットパス実行時化）と外部入力アセット解決の方針を揃えるのが望ましい。

## P57: process_pointer_buffers / process_mouse_buffers がワークスペース全域で未使用のデッド/レガシー pub 関数（削除または非推奨明示の整理候補）
- source: W6a-T
- kind: その他
- rationale: `crates/wintf/src/ecs/pointer/systems.rs::process_pointer_buffers`（systems.rs:24-157）と、その `#[deprecated(since = "0.1.0", note = "Use process_pointer_buffers instead")]` エイリアス `process_mouse_buffers`（systems.rs:160-162、本体は `process_pointer_buffers` へ委譲）は、いずれも `pub` で公開され `ecs/mod.rs:41`・`ecs/pointer/mod.rs:27`（および `process_mouse_buffers` が `ecs/mod.rs:48`・`pointer/mod.rs:34`）から再エクスポートされているが、**どのスケジュールにも `add_systems` で登録されておらず、ワークスペース全域で本番呼び出しがゼロ**である。`world/mod.rs:114-116` に「注: process_pointer_buffersは廃止／WndProc スレッドの thread_local バッファは try_tick_world() 内の transfer_buffers_to_world() で直接 World に転送される」と明記されており、`Input` スケジュールには `Schedule::new(Input)`（world/mod.rs:74）として挿入後 `dispatch_pointer_events`・`dispatch_drag_events` 等は登録される一方、`process_pointer_buffers` は登録されない。ワークスペース全域の grep でも `add_systems(... process_pointer_buffers ...)` は**本セルで新規追加した特性化テスト2件（systems.rs:407, 463）のみ**で、本番経路には存在しない。本番で WndProc スレッドの thread_local バッファ（`POINTER_BUFFERS` / `BUTTON_BUFFERS` / `WHEEL_BUFFERS` / `DOUBLE_CLICK_BUFFERS` / `MODIFIER_STATE`、buffers.rs:20-35）を消費するのは `transfer_buffers_to_world`（buffers.rs:134、`try_tick_world` 冒頭の world/mod.rs:458 から WndProc スレッド上・`try_run_schedule(Input)` の前に同期呼び出し）であり、buffers.rs:129-133 のドキュメントコメントが「`try_tick_world()` の冒頭（Input スケジュール実行前）で呼ばれ、WndProc スレッド（メインスレッド）で収集したポインター情報をマルチスレッドで実行されるシステムがアクセスできるように転送する」という意図的設計を明記している。したがって `process_pointer_buffers` は `transfer_buffers_to_world` への移行（廃止）の残骸であり、現状は thread_local を直読するが本番では呼ばれない**デッドコード（および後方互換目的のレガシー `pub` エイリアス）**である。なお参考として、bevy の並列実行に必要な `ComputeTaskPool` 自体は本番で初期化済み（`common/tree_system.rs:140` が `ComputeTaskPool::get_or_init(TaskPool::default)` を transform 階層の並列伝播のために呼ぶ）であり、「ComputeTaskPool 不在」を前提とした懸念は成り立たない。本セルでは現行挙動（DOWN 優先ボタンルール・位置/ホイール/ダブルクリック/修飾キー取り込みと消費）を特性化する2件を追加したが、これは thread_local を読む当該関数を**バッファ投入スレッドと同一スレッドで決定論的に駆動する**ため `ExecutorKind::SingleThreaded` を用いただけで、本番実行経路を再現するものではない（本番には実行経路が存在しない）。削除・整理は `pub` API 表面の変更（外部観測可能）を伴うため R2.9/R2.10 に従い本ループでは実装せず記録に留めた。優先度は低（実害なし。死コード/レガシー API の整理であり並行性脆弱性ではない）。
- suggestion: ワークスペース利用ゼロを実証のうえ（本セルで確認済み: 本番 `add_systems` ゼロ・呼び出し元は新規テスト2件のみ）、`process_pointer_buffers` と `process_mouse_buffers` を整理する小規模仕様。候補: (1) 両関数とその再エクスポート（`ecs/mod.rs:41,48`・`pointer/mod.rs:27,34`）を削除し、`world/mod.rs:114-116` の「廃止」コメントとコード状態を一致させる（thread_local 消費は `transfer_buffers_to_world` に一本化済みのため挙動同値）。削除に伴い本セル追加の特性化テスト2件（`test_process_pointer_buffers_*`、systems.rs:368, 426）は対象消失で削除する。(2) もし将来の代替実装やデバッグ用途で残す判断なら、`process_pointer_buffers` 自体にも `#[deprecated]` を付与し未使用 `pub` であることをドキュメント化する。整理時は `transfer_buffers_to_world` 側の特性化テスト（本セル追加 buffers 9件）が本番経路の回帰検知器として残るため安全。なお `transfer_buffers_to_world` と（削除予定の）`process_pointer_buffers` が位置・ボタン・修飾キーをそれぞれ別に World へ書く二重実装の重複も、削除により構造的に解消する。
- 解決状況: **W6a-S（タスク15.2）で候補(1)を実施・解消済み**。`process_pointer_buffers`/`process_mouse_buffers` 本体（systems.rs）、再エクスポート（`pointer/mod.rs`・`ecs/mod.rs`）、特性化テスト2件を削除。`cargo build --workspace`（areka 本体含む）成功・`cargo test --workspace` 1516→1514（−2 は削除テスト）で挙動非破壊を実証。`transfer_buffers_to_world`/`process_pointer_buffers` 間の二重実装の重複も削除で構造解消。

## P58: transfer_buffers_to_world のボタン down/up 転送における match ブロック重複（本番ポインター入力経路の DRY 整理候補）
- source: W6a-S
- kind: ロジック変更を要する簡素化
- rationale: `crates/wintf/src/ecs/pointer/buffers.rs::transfer_buffers_to_world`（buffers.rs:127-225）のボタン状態転送部は、`buf.down_received` 真の分岐（buffers.rs:168-183、各ボタン `= true`）と `buf.up_received` 真の分岐（buffers.rs:185-201、各ボタン `= false`）で、それぞれ全5ボタン（Left/Right/Middle/XButton1/XButton2）への代入を `match button { ... }` で記述しており、`= true`／`= false` の値だけが異なる**ほぼ同形の match が2ブロック重複**している。`is_pressed: bool` を `down_received → true` / `up_received → false` で求め、イベントなし（どちらも false）の場合は代入をスキップ（既存状態を維持＝エッジ検出）するよう統合すれば、match を1ブロックに集約でき重複を除去できる。本経路は本番のポインター入力反映経路（WndProc スレッドで `try_tick_world` 冒頭から呼ばれる）であり、本セル追加の `transfer_buffers_to_world` 特性化9件（位置/速度・ボタンエッジ検出・全ボタン写像・修飾キー・PointerState 不在スキップ）で保護されている。統合自体は挙動非破壊の見込みだが、(a) 「down も up もない場合は代入を行わない（維持）」というエッジ検出セマンティクスを統合後も厳密に保つ必要があり（`is_pressed` を `Option<bool>` 等で表現し None で skip する設計が必要）、naive な「常に bool 代入」リファクタは無イベント時に既存状態を上書きする退行を生む罠がある、(b) 本番クリティカルな入力反映経路の制御フロー構造変更である、ため、churn 回避（karpathy）と本番経路保護の観点から本ループでは適用せず記録に留めた。
- suggestion: `transfer_buffers_to_world` のボタン転送を、`down_received`/`up_received` から `Option<bool>`（`Some(true)`=押下 / `Some(false)`=解放 / `None`=イベントなしで維持）を求め、`Some(v)` のときだけ単一の `match button` で対応フィールドへ `v` を代入する形に統合する小規模リファクタ。`process_pointer_buffers`（W6a-S で削除済み）が用いていた「is_down を求めてから match で代入」パターンと同型だが、エッジ検出（無イベント時 skip）を維持する点が要点。実装後も本セル追加の buffers 9件（特に `test_transfer_buffers_to_world_button_edge_detection_and_reset`）が回帰検知器として有効。優先度は低（純粋な可読性向上であり挙動変更なし。実害なし）。

## P59: WHEEL_BUFFERS / DOUBLE_CLICK_BUFFERS が消費されない thread_local（ホイール入力が PointerState に未反映の潜在ギャップ＋デッドストレージ）
- source: W6a-V
- kind: その他
- rationale: `crates/wintf/src/ecs/pointer/buffers.rs` の thread_local `WHEEL_BUFFERS`（buffers.rs:28）と `DOUBLE_CLICK_BUFFERS`（buffers.rs:31）は、本番の消費経路（`transfer_buffers_to_world`、buffers.rs:127-225）から**一切読まれない**。本番でこれらを読んでいたのは `process_pointer_buffers` だが、同関数は W6a-S（P57）で削除済みであり、削除前から**どのスケジュールにも `add_systems` 登録されておらずデッド**だった（`world/mod.rs:114-116` で廃止明記。W6a-T 起点 commit 6e7e1ea の `world/mod.rs` を grep して `process_pointer_buffers` の登録ゼロを実コードで確認済み）。したがって W6a-S は回帰を導入しておらず、これは**W6a-S 以前から存在する潜在ギャップ**である。具体的には: (a) `WHEEL_BUFFERS` は `add_wheel_vertical`/`add_wheel_horizontal`（buffers.rs:91/101、WM_MOUSEWHEEL/WM_MOUSEHWHEEL（window_proc/mouse_dblclick_wheel.rs:200/218）から駆動）で**書き込まれるが本番で読まれず**、`pointer_state.wheel` を非既定値へ書く本番経路は現状**存在しない**（`systems.rs:25` は clear_transient のリセット、:90-94 はデバッグ読み取りのみ。ワークスペース全 grep で `*.wheel = WheelDelta{非既定}` の本番代入は process_pointer_buffers 削除後ゼロ）。すなわちマウスホイール入力は PointerState 経由のハンドラ（`OnPointer*`）には届かない＝write-only デッドストレージ兼**機能ギャップ**。(b) `DOUBLE_CLICK_BUFFERS` は**書き込み箇所がワークスペース全域でゼロ**（`.insert` 皆無、buffers.rs のテスト helper の `.clear()` のみ）で、ダブルクリックは `window_proc/mouse_dblclick_wheel.rs:83-111` が直接 `PointerState.double_click` を component へ書く経路で機能しているため、当該 thread_local は純粋なデッドストレージ（機能欠落なし）。本セル（V 観点・バッファ枯渇点検）でこれらが消費されない点を確認した。脆弱性ではない（メモリ枯渇は P60 で別途評価、当該 thread_local 自体の存在はクラッシュ/DoS 経路ではない）が、(a) のホイール反映を本番経路へ繋ぐ（`transfer_buffers_to_world` に WHEEL_BUFFERS 消費を追加し `pointer_state.wheel` を設定）のは**外部観測可能な挙動変更**（ホイールイベントが PointerState・OnPointer ハンドラへ到達し始める）であり、R2.4/R5.2 に従い本ループでは実装せず記録に留めた。(b) のデッドストレージ削除は挙動非破壊だが S 観点（簡素化）の領分であり、V セルでは適用せず記録のみ（W6a-S の P57 デッドコード整理テーマの延長）。
- suggestion: 二段で整理する小規模仕様。(1) **機能**: ホイール入力を PointerState へ反映する設計判断（ホイールを `OnPointer*` ハンドラへ届けるべきか）を確定し、必要なら `transfer_buffers_to_world` に WHEEL_BUFFERS 消費ブロック（POINTER/BUTTON/MODIFIER と同様に `world.get_mut::<PointerState>` で `pointer_state.wheel` 設定 → 転送後 reset）を追加する。FrameFinalize の `clear_transient_pointer_state` が既に wheel をリセットするため「1フレームのみ有効」契約は維持される。回帰検知器として「ホイール転送後 pointer_state.wheel が設定され、clear_transient で消える」特性化テストを追加。(2) **デッドストレージ**: 上記(1)でホイール反映が不要と判断されるなら `WHEEL_BUFFERS` と `add_wheel_*`（および window_proc 側呼び出し）を、`DOUBLE_CLICK_BUFFERS` は無条件に、デッドコードとして削除（grep で利用ゼロ実証済み・S 観点で実施）。優先度は中（(a) はホイール機能の欠落、(b) は低優先のデッドストレージ）。なお (b) の削除は P57/W6a-S と同じ「ポインター系デッド/レガシー整理」文脈。

## P60: ポインター thread_local バッファの HashMap キーがエンティティ単位で単調増加（despawn/leave 時の除去なし）
- source: W6a-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/pointer/buffers.rs` の thread_local `POINTER_BUFFERS`/`BUTTON_BUFFERS`/`WHEEL_BUFFERS`/`DOUBLE_CLICK_BUFFERS`/`MODIFIER_STATE`（buffers.rs:20-35）は、`entry(entity).or_insert*` でエントリを生成する（buffers.rs:52/62/78/94/104/114）が、**本番経路にエントリの `.remove()`/`.retain()` が一切存在しない**（grep 実証。`transfer_buffers_to_world` は `PointerBuffer::clear()`（VecDeque 空化・キー保持）と `ButtonBuffer::reset()`（フラグ 0 化・キー保持）を行うのみで、マップのキーは残置。全マップ wipe はテスト helper の `reset_all_buffers` のみ）。したがって、ポインター入力を受けたことのある**個別 Entity ごとに 1 エントリがマップに永久蓄積**し、エンティティが despawn してもポインターが leave しても対応エントリは除去されない。バッファ枯渇（V 観点）の点検として: 個別 Entity の**サンプル列は MAX_SAMPLES=5 で上限**（`PointerBuffer::push` が pop_front。types.rs:225-230）・`ButtonBuffer`/`WheelBuffer` は単一構造体で飽和（types.rs:326-333 saturating_add）であり、**1 エントリあたりのメモリは定数上限**。増加し得るのは**マップのキー数のみ**で、その上限は「ポインター入力を受けた distinct Entity 数」である。bevy の Entity は despawn 時に世代付きインデックスを再利用するため、生存スロット数（UI ツリーのウィンドウ＋ウィジェット数）で実質的に有界であり、**イベント発生量（マウス移動回数）には比例しない**。したがって現実的なメモリ枯渇 DoS には至らず（増加量は UI 要素数オーダーで微小・自己抑制的）、**現状は安全**と判定し本セルでは挙動非破壊対策を投入していない（断片に安全根拠を記録）。ただし厳密には despawn 済みエンティティの世代スロットが別エンティティに再利用されるまで stale キーが残る理論的リークであり、長寿命プロセスで多数のエンティティ生成/破棄を繰り返す構成では微増し得る。除去対策（leave/despawn フックでの該当キー削除、または転送時に空エントリを除去する `retain`）は、空バッファの存在/非存在という内部状態の変化を伴い、特に「`transfer_buffers_to_world` で空になったエントリを除去」する案は ButtonBuffer のエッジ検出（無イベント時の押下状態維持）と相互作用し得る（エントリ除去＝次回 down_received/up_received の初期状態が変わる可能性）ため、挙動非破壊性の厳密検証を要する。R2.4/R5.2 に従い本ループでは実装せず記録に留めた。優先度は低（実害は実質なし。理論的な stale キー残置のみ）。
- suggestion: ポインター thread_local バッファのキー寿命を管理する小規模仕様。候補: (1) `WM_MOUSELEAVE`/エンティティ despawn 時に該当 Entity の全マップエントリを除去するヘルパー（`POINTER_BUFFERS`/`WHEEL_BUFFERS`/`DOUBLE_CLICK_BUFFERS`/`MODIFIER_STATE` は `entity` キー、`BUTTON_BUFFERS` は `(entity, button)` キーなので該当 entity の全 button を走査除去）。(2) より簡素には `transfer_buffers_to_world` 末尾で「POINTER_BUFFERS の空（is_empty）エントリを `retain` で除去」。ただし BUTTON_BUFFERS は reset 後も「直近押下状態の維持」のためエントリ保持が必要なエッジ検出契約があるため、除去対象は POINTER_BUFFERS（サンプル列）に限定するのが安全。実装時は本セル追加の buffers 特性化（エッジ検出・全ボタン写像）が回帰検知器。bevy の `RemovedComponents<PointerState>` や despawn フックと連動させると leave 検出に自然に乗る。優先度は低。

## P61: DraggingState.prev_frame_pos のデッドストア整理（毎ドラッグフレーム書き込み・本番読み取りゼロのフィールド）
- source: W6b-T
- kind: 非推奨コード削除候補（デッドストア／将来用フィールド）
- rationale: `crates/wintf/src/ecs/drag/mod.rs:76-78` の `DraggingState.prev_frame_pos`（`pub prev_frame_pos: PhysicalPoint`）は、コメントに「前回ECSフレームの位置（デルタ計算用、**現在は未使用**）」と明記されたフィールドである。`dispatch_drag_events` は Started 時に `prev_frame_pos: start_pos` で初期化し（dispatch.rs:160）、DragEvent 発火時に毎回 `dragging_state.prev_frame_pos = flush_result.current_position` で更新する（dispatch.rs:382-388）が、**ワークスペース全域でこのフィールドを読む箇所が一切存在しない**（grep 実証: `.prev_frame_pos` の出現は定義 1・書き込み 2 のみで読み取りゼロ。デルタは `flush_result.delta`／`current_position - start_pos` から都度算出されており prev_frame_pos を経由しない）。すなわち「毎ドラッグフレームに書き込まれるが決して読まれないデッドストア」であり、ドラッグ移動という高頻度経路で無駄な書き込みが行われている。削除は `pub` フィールドの除去（外部観測可能な型シグネチャの変更）かつ DragEvent 経路の `get_mut` ブロック（dispatch.rs:383-389）の整理を伴うため、R2.9/R2.10（公開 API・挙動非破壊性の厳密検証）に従い本 T セルでは実装せず記録に留めた（本セルはテスト追加のみ）。本セルで追加した `dispatch_emits_drag_event_when_delta_nonzero` は現行の prev_frame_pos 更新挙動を特性化しており、削除/変更時の回帰検知器となる（フィールド除去時は当該アサーションも対象消失で除去）。
- suggestion: `DraggingState.prev_frame_pos` を削除する小規模仕様。(1) フィールド定義（mod.rs:78）、(2) Started 時の初期化（dispatch.rs:160）、(3) DragEvent 経路の更新ブロック（dispatch.rs:382-389、`get_mut::<DraggingState>` のみが当該更新のために存在するなら丸ごと除去可能）を削除する。将来デルタ計算で前フレーム位置が必要になった場合に再導入すれば足りる（現状は `current_position` と `drag_start_pos` で全ドラッグ計算が成立）。あわせて `CaptureGuard::is_released`（capture_guard.rs:47-50）も本番読み取りゼロ・`#[allow(dead_code)]` 付きのテスト専用アクセサであり、整理候補として同時検討可（ただしテスト 2 件が参照するため削除時はテスト側の検証手段の置換が必要。優先度は prev_frame_pos より低）。優先度は低〜中。
- **resolution (W6b-S)**: `DraggingState.prev_frame_pos` の本番読み取りゼロをワークスペース全域 grep で再実証（定義1・書込2[dispatch.rs:160,387]・読取0／デルタは accumulated_delta と current_position−drag_start_pos で算出され prev_frame_pos を経由しない）のうえ、フィールド定義（mod.rs）・2書込・専用 `get_mut` 更新ブロック（dispatch.rs）を**除去・解消済み**。W6b-T 追加テストの prev_frame_pos アサート/リテラルは追従調整（DragEvent 検証は残存）。S2 全量 1566 passed / 0 failed で挙動非破壊を確認（±0）。`CaptureGuard::is_released` はテスト2件が `released` フラグの観測 API として参照する（本番デッドストアとは性質が異なる）ため W6b-S では**見送り**、低優先候補として維持。

## P62: ドラッグ閾値判定 check_threshold の本番未使用＋インライン複製と距離二乗算術の i32 桁あふれ境界
- source: W6b-V
- kind: その他（本番未使用 pub 関数の整理＋複製統合。整数境界の堅牢化を伴う場合は挙動変更を伴う脆弱性対策）
- rationale: `crates/wintf/src/ecs/drag/state.rs::check_threshold`（state.rs:503-）は `dx = current_pos.x - start_pos.x` / `dy = ...` の i32 座標差から `distance_sq = dx*dx + dy*dy`、`threshold_sq = threshold*threshold` を i32 乗算で求め `distance_sq >= threshold_sq` を返す。V 観点の点検で2点を確認した。(1) **本関数は本番呼び出しがゼロ**: ワークスペース全 grep で `check_threshold` の呼び出し元は同ファイル in-source テスト（本セル追加分含む）のみで、`window_proc/`（W7a）からも他クレートからも呼ばれない（`mod.rs:21` で re-export はされる）。本番の閾値判定は `window_proc/mouse_move.rs:201-204` に**完全に同一の算術がインライン複製**されており（`dx*dx + dy*dy >= threshold*threshold`）、`check_threshold` を経由しない。すなわち `check_threshold` は W6a の P57（process_pointer_buffers 死関数）と同型の「本番未使用 pub 関数＋二重実装」である。(2) **i32 桁あふれの理論境界**: 両コピーとも `dx*dx` は `|dx| > 46340`（46341² > i32::MAX=2_147_483_647）で debug ビルドでは桁あふれ panic、release ビルドではラップする。ただし本番座標は WM lparam の i16 クライアント座標（[-32768,32767]、mouse_move.rs:110-111 の `as i16 as i32`）＋ウィンドウ位置オフセット（実モニタ幾何で有界、system 制御）であり、実用座標差は i16 幅オーダーで桁あふれ境界（46340）に達しないため**現実的には安全**（本セルで i16 極値デルタ 32767 が桁あふれせず正評価されることを `test_check_threshold_i16_extent_delta_no_overflow` で特性化）。整理（`check_threshold` の削除または mouse_move.rs インラインを `check_threshold` 呼び出しへ統合）は (a) pub API 表面の変更（外部観測可能）か (b) W6b（state.rs）と W7a（mouse_move.rs）の境界をまたぐ統合であり、いずれも本 V セルの境界・挙動非破壊制約を超える。整数境界の堅牢化（飽和/checked 乗算化）も極値での判定結果を変える挙動変更となる。R2.4/R2.9/R2.10/R5.2 に従い本ループでは実装せず、現行算術の安全鎖を特性化テスト（負デルタ対称性・i16 極値非桁あふれ）と check_threshold への整数境界ハザードコメントで固定するに留めた。優先度は低（実害は実質なし。死コード/二重実装の整理であり、桁あふれは実用座標では未発現）。
- suggestion: 二段で整理する小規模仕様。(1) **二重実装の解消**: ワークスペース利用ゼロを実証のうえ（本セルで確認済み）、`mouse_move.rs:201-204` のインライン閾値算術を `crate::ecs::drag::check_threshold(current_pos, drag_config.threshold)` 呼び出しへ統合し、算術の単一ソース化を図る（W6b state.rs と W7a mouse_move.rs の境界をまたぐため両領域同時のタスクとして実施）。または `check_threshold` 自体が将来も未使用なら削除し re-export（mod.rs:21）も整理する。(2) **整数境界の堅牢化**（任意・低優先）: 距離二乗を `i64` 昇格（`(dx as i64)*(dx as i64) + ...`）または `saturating_mul`/`saturating_add` で計算し、極値座標差でも debug panic/ラップを排除する。これは極値での判定結果が変わり得る挙動変更のため新仕様の判断を要する。(1) の統合後は本セル追加の check_threshold 特性化（境界・符号・i16 極値）と mouse_move 経路の起動テストが回帰検知器となる。なお実用座標では桁あふれ未発現のため (2) は (1) 統合時に併せて検討すれば足りる。

## P63: SetWindowPosCommand キューのテスト用検査 API 追加（enqueue 内容の非破壊観測手段欠如・wintf 側）
- source: W7a-T1
- kind: その他（テスト容易化のためのテスト専用 API 追加）
- rationale: `crates/wintf/src/ecs/window/command.rs` の `SetWindowPosCommand` は thread_local `WINDOW_POS_COMMANDS` への `enqueue`（push）と `flush`（`guarded_set_window_pos` = 実 SetWindowPos で消費しながら drain）しか公開しておらず、**キューに積まれたコマンドの座標/サイズ/フラグ/insert_after を非破壊で観測する検査 API がない**。このため「`apply_window_pos_changes` 等が enqueue したコマンドの内容が正しいか」をユニットで検証できない（flush すると実 SetWindowPos が呼ばれ実ウィンドウが必要になり、かつキューが空になる）。W7a-T1 では `SetWindowPosCommand::new`（フィールド格納）と空キュー flush の no-op までを特性化したが、enqueue→内容アサーションの経路は API 欠如で到達不能。これは A1-T の **P1**（areka `on_shell_drag` の正常系で `pos + BALLOON_OFFSET` の enqueue 内容を検証したいが同 API 欠如で不能）が「wintf への API 追加は A1-T 境界外」として保留した所見と**完全に同根の wintf 側ギャップ**であり、本セル（wintf `ecs/window/` 担当）から再確認した。テスト専用 API の追加自体は本番挙動を変えないが、`#[cfg(test)]` でない公開 API として足すか feature gate するか、enqueue 経路の本番 pub 面に影響するかの設計判断を要するため、本 T セルでは実装せず記録に留めた（R2.8 適用域）。
- suggestion: wintf `ecs/window/command.rs` にテスト用キュー検査 API（例: `#[cfg(any(test, feature = "test-util"))] pub fn take_queued() -> Vec<SetWindowPosCommand>` または非破壊の `peek_queued()`）を追加する小規模仕様。**P1 と統合実装を推奨**: 同 API を追加すれば (a) wintf 側 command.rs で `apply_window_pos_changes` 等の enqueue 内容（座標/フラグ/ZOrder→insert_after 写像）を直接アサートでき、(b) areka 側 `on_shell_drag` の正常系（バルーン追従座標）も同 API で固定できる。`SetWindowPosCommand` は既に `#[derive(Debug, Clone)]` 済みのため Vec 取り出しは容易。

## P64: window_proc メッセージパラメータ抽出ロジックの純粋ヘルパ抽出（インライン埋め込みによる単体到達不能＋3ファイル複製）
- source: W7a-T2
- kind: ロジック変更を要する簡素化（挙動非破壊な構造抽出。判断に迷う構造変更）
- rationale: `crates/wintf/src/ecs/window_proc/` の各 `pub(super)` メッセージハンドラは、メッセージパラメータ（WPARAM/LPARAM）からデバイス非依存な値を抽出する純粋な式を**ハンドラ本体にインライン埋め込み**しており、抽出関数として分離されていないため、実 HWND/World/hit_test/drag thread_local と同一スコープに閉じ込められ単体テストで到達できない。確認した具体箇所: (1) **LPARAM クライアント座標の符号付き lo/hi ワード抽出** `(lparam.0 & 0xFFFF) as i16 as i32` / `((lparam.0 >> 16) & 0xFFFF) as i16 as i32` が **mouse_move.rs:110-111・mouse_click.rs:33-34・mouse_dblclick_wheel.rs:40-41 の3ファイルに同一複製**、(2) **WPARAM 修飾キー抽出** `(wparam_val & 0x04) != 0`（MK_SHIFT）/`(wparam_val & 0x08) != 0`（MK_CONTROL）が mouse_move/mouse_click/mouse_dblclick_wheel に重複、(3) **XBUTTON 抽出** `((wparam.0 >> 16) & 0xFFFF) as u16` → 1 なら XButton1 / else XButton2（mouse_click.rs:410-415,427-432・mouse_dblclick_wheel.rs:177-182）、(4) **wheel delta 符号付き抽出** `((wparam.0 >> 16) & 0xFFFF) as i16`（mouse_dblclick_wheel.rs:199,217）、(5) **WM_ACTIVATE activation_state** `(wparam.0 & 0xFFFF) as u32` + WA_INACTIVE(0) 判定（keyboard.rs:121）、(6) **DoubleClick→PointerButton マッピング** 6 アーム match（mouse_dblclick_wheel.rs:49-56）。これらはすべて純粋算術/写像でデバイス非依存だが、ハンドラ本体に埋め込まれ抽出関数がないため、本 T セルでは特性化できず所見化した（W7a-T2 所見4〜7）。なお同様の `DPI::from_WM_DPICHANGED`（WPARAM DPI 解析）は既に `window/dpi.rs` の独立関数として抽出済みで W7a-T1 が特性化しており、抽出すれば単体テスト可能であることの先例。抽出は4ファイルにまたがるプロダクション構造変更（観測可能挙動は不変だが、R2.9/R2.10 の「判断に迷う構造変更」に該当）のため本 T セルでは実装せず記録した。
- suggestion: `window_proc/` に挙動非破壊な純粋ヘルパを抽出する小規模仕様。候補シグネチャ: `fn extract_client_point(lparam: LPARAM) -> (i32, i32)`（符号付き lo/hi ワード）、`fn extract_modifier_keys(wparam: WPARAM) -> (bool /*shift*/, bool /*ctrl*/)`、`fn extract_xbutton(wparam: WPARAM) -> PointerButton`、`fn extract_wheel_delta(wparam: WPARAM) -> i16`、`fn extract_activation_inactive(wparam: WPARAM) -> bool`、`fn double_click_to_button(dc: DoubleClick) -> Option<PointerButton>`。各ハンドラのインライン式を呼び出しへ置換し（特に LPARAM 座標抽出は3ファイルの複製を単一ソース化）、各ヘルパに符号境界（i16 負座標・lo/hi 分離）・XBUTTON 既定（HIWORD≠1→XButton2）・wheel 符号・WA_INACTIVE 判定のデバイス非依存テストを付す。観測可能挙動はビット演算同値で不変。W6a-T の P58（buffers の down/up 転送 match 重複）と同系統の「メッセージ→ECS 変換の DRY/抽出」整理であり、抽出後は本セルで特性化済みの World ベースヘルパ（`find_ancestor_with_drag_config`/`collect_entities_to_leave`）と接続点をユニットで固定できる。
- note（W7a-S 再確認）: 17.3 W7a-S（S 観点）で抽出可否を慎重に検討したが、(a) 本提案を W7a-T2 自身が「判断に迷う構造変更」と分類済み、(b) 抽出対象が実 WndProc ハンドラ（テスト保護外・R5.5 域）本体に分布、(c) DoubleClick→PointerButton マッピングが `None => return Some(LRESULT(0))` というハンドラ制御フローを内包し純粋値写像として綺麗に切り出せない、(d) 残る重複は低害・自己文書的で除去の可読性向上が cross-file 構造 churn に見合わない、の総合判断で **P64 を維持し W7a-S では抽出を見送った**（R5.5 + karpathy 2/3）。本提案は引き続き有効。

## P65: create_windows の CompositionMode→ex_style 分岐の純粋関数抽出（実 CreateWindowExW システム内のロジック分離）
- source: W7a-S
- kind: ロジック変更を要する簡素化（挙動非破壊な構造抽出。実 GUI システムへの構造変更）
- rationale: `crates/wintf/src/ecs/window/window_system.rs:79-85` の `create_windows`（排他システム）は、`CompositionMode` に基づく ex_style 調整 `match composition_mode { ULW => style_comp.ex_style, DComp => (style_comp.ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP }` を**システム本体にインライン**している。この写像は純粋（入力 ex_style + CompositionMode → 出力 ex_style）でデバイス非依存だが、`create_windows` 全体が `WinProcessSingleton::get_or_init()`（プロセスシングルトン・ウィンドウクラス登録）と実 `CreateWindowExW`/`ShowWindow`/`GetDpiForSystem` に密結合し、システム単位ではユニット到達不能（W7a-T1 所見5）。純粋関数 `fn ex_style_for_composition_mode(base: WINDOW_EX_STYLE, mode: CompositionMode) -> WINDOW_EX_STYLE` への抽出で ULW=base / DComp=WS_EX_NOREDIRECTIONBITMAP 設定＋WS_EX_LAYERED 除去のビット演算をデバイス非依存テストで固定できる。挙動は不変（同一ビット演算）だが、抽出は**実 CreateWindowExW システム（テスト保護外の GUI 域）へのロジック構造変更**であり、R5.5 が本ループでの実施を禁ずる（テスト保護外の unsafe/GUI は構造的整理に限定、ロジックに踏み込む簡素化は提案記録）。17.3 W7a-S で構造整理に限定する判断のもと記録。
- suggestion: `window/window_system.rs` または `window/components.rs`（CompositionMode 定義の近傍）に純粋関数 `ex_style_for_composition_mode` を切り出し、`create_windows` のインライン match を呼び出しへ置換する小規模仕様。in-source テストで ULW 恒等・DComp の WS_EX_LAYERED 除去 + WS_EX_NOREDIRECTIONBITMAP 付与（および両ビットの排他性）を固定する。観測可能挙動はビット演算同値で不変。実起動 S7（areka 起動・ウィンドウ生成）が統合的な回帰検知器。

## P66: FrameCount(u32) の tick 加算オーバーフロー堅牢化（debug panic 化の回避）
- source: W7b-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/world/mod.rs::try_tick_world` の `frame_count.0 += 1`（FrameCount は `u32`、schedule_labels.rs:8）は毎 tick で増加し、~60Hz では 2^32 到達に約828日の連続 tick を要する。到達時、debug ビルドは Rust の加算オーバーフローチェックで **panic**（長時間連続稼働の準ハング = 理論的 DoS）、release ビルドはラップして 0 に戻る。点検の結果（`git grep -n "frame_count.0"` でワークスペース全列挙）、`FrameCount.0` の消費は次の2形態のみ: (1) **tracing ログの `frame = ...` フィールド**（`window/window_system.rs:40`・`layout/systems/window_pos_systems.rs:40,54,72,90,103`・`graphics/visual_manager.rs:98,104,128`・`graphics/systems/init.rs`（多数）・`graphics/systems/render.rs:167`）、(2) **`graphics/systems/surface.rs:44` の `dirty.requested_frame = frame_count.0 as u64`**（`as u64` 変換で永続コンポーネント `SurfaceGraphicsDirty.requested_frame`（graphics/components.rs:243）へ格納）。後者の `requested_frame` は本番で読み取られず（`git grep -n "requested_frame"` の本番読み取りゼロ。読み取りは graphics/tests.rs 等のテストのみ）、surface.rs:25-26 ドキュメント「フレーム番号更新方式」が示すとおり**前回値と異なることで `Changed<SurfaceGraphicsDirty>` をトリガーする「変化ノンス」**として使われる（`render_surface` は数値ではなく `Changed` フラグに反応）。いずれの消費も**算術上の大小比較・厳密比較・配列添字には用いられない**。したがってラップ MAX→0 が起きても、ログ値は 0 に戻るだけ、`requested_frame` は「前回値と異なる」性質を満たし続け Changed 検出が継続するため、ラップは正当性に影響しない（surface.rs:196 の並行経路 `wrapping_add(1)` も同じく Changed トリガー目的でラップ許容）。堅牢化（`wrapping_add(1)` で debug でもラップ統一、または `saturating_add(1)` で上限張り付き）は **debug ビルドの panic 挙動を変える**（= 外部観測可能な挙動変更。R5.1）ため本ループでは実装せず、現行挙動は W7b-T2 の `try_tick_world_increments_frame_count_each_call` が低カウント域で特性化済み。本セルでは mod.rs:445-447 に整数境界ハザードコメントを付記するに留めた。
- suggestion: `frame_count.0 = frame_count.0.wrapping_add(1)`（フレームカウンタは循環値として扱う旨を明文化）へ置換する小規模仕様。FrameCount をフレーム識別子として将来**数値の厳密比較や添字・差分計算**に使う設計が入る場合（現状の surface.rs:44 は値を読まない変化ノンス用途なのでラップ無害だが、値そのものを比較・演算する用途が加わるとラップが意味を持つ）は u64 への型拡張も併せて検討する（u64 なら ~60Hz で 9.7×10^9 年規模となり実機到達不能）。EcsWorld 内部の `frame_count: u64`（measure_and_log_framerate 用・10秒ごとに 0 リセット）は到達前にリセットされるため対象外。`win_thread_mgr` の VSYNC_TICK_COUNT/LAST_VSYNC_TICK（u64・本セル境界外）は周回が実機到達不能のため堅牢化対象外（atomic ordering の妥当性は本セルで mod.rs:try_tick_on_vsync にコメント明記）。

## P67: dispatch_cue_sheet_internal の配送アーム重複統合（RouteAdd≡RouteSwitch 同一本体・Command/Barrier 近重複の DRY 整理候補）
- source: W8-S
- kind: ロジック変更を要する簡素化（挙動非破壊な構造抽出。テスト保護外の System 本体への構造変更）
- rationale: `crates/wintf/src/ecs/cue/dispatch.rs::dispatch_cue_sheet_internal`（dispatch.rs:27-179）に2系統の重複がある。(1) **`RoutingCommand::RouteAdd`（dispatch.rs:48-62）と `RouteSwitch`（dispatch.rs:63-77）のアーム本体が、tracing ログ文字列（"RouteAdd applied"/"RouteAdd target ... not found" vs "RouteSwitch applied"/"RouteSwitch target ... not found"）以外バイト同一**である（いずれも `if let Some(entity) = registry.resolve(to) { registry.register_actor(cue.actor.clone(), target.clone(), entity); ...debug } else { ...warn skipping }`）。現状の RouteAdd と RouteSwitch は registry への作用が完全に同一（どちらも `register_actor` で後勝ち上書き）であり、log ラベルのみが両者を区別する。(2) **`CuePayload::Command`（dispatch.rs:93-131）と `CuePayload::Barrier`（dispatch.rs:134-172）のアームが、`Entry::Payload(absolute_time, cmd.clone())` vs `Entry::Barrier(absolute_time, barrier.clone())` の Entry 構築種別と warn 文言（"command"/"barrier"）以外ほぼ同一**である（`absolute_time` 算出 → `routes_for_actor` → 空ならスキップ警告 → routes ループで `queue.insert(entry)`・`set_cue_sheet`・`seen_targets` 重複排除・`all_targets` 蓄積、という約38行の手続きが両アームに複製）。共通化は (a) routing 2 アームをラベル引数付きヘルパ `fn apply_route(registry, actor, target, to, label) -> ` へ、(b) Command/Barrier 2 アームを `Entry` を受け取る配送クロージャ `fn broadcast_entry(absolute_time, entry_builder, routes, cue_queues, ...) ->` へ括り出す形となるが、**いずれも `dispatch_cue_sheet_internal`（純関数だが `Query<&mut CueQueue>`/`EntityRegistry` を可変借用する System 補助関数）の制御構造そのものを変更**する。dispatch.rs は **in-source テストを持たず**（W8-T1 が確認・過剰回避で in-source 追加を見送り）、配送経路の検証は `tests/ecs/cue_dispatch_e2e_test.rs`（6件）が担うが、当 E2E は **`CuePayload::Command`（Text/Clear）と `RouteSwitch` のみを駆動**し、**`CuePayload::Barrier` 配送アーム・`RouteAdd`・`RouteRemove` は一切駆動しない**（grep 実証: cue_dispatch_e2e_test.rs に Barrier 配送・RouteAdd 構築なし。RouteAdd の wintf 側構築は `cue_data_model_test.rs:114` の型検査のみで dispatch 駆動なし）。したがって統合対象アームの一部（Barrier 配送・RouteAdd）は**回帰検知器が存在しない未保護分岐**であり、構造変更は実起動 S7 でしか挙動非破壊を担保できない。R5.2/R2.8（安全に適用できないロジック構造変更は提案記録）に従い本ループでは実装せず記録した。W6a-T の **P58**（`transfer_buffers_to_world` の down/up 転送 match 重複の DRY 整理候補）と同系統の「System 本体の配送ロジック DRY 整理」である。
- suggestion: `dispatch.rs` に2つの挙動非破壊ヘルパを抽出する小規模仕様。(a) `fn apply_route(registry: &mut EntityRegistry, actor: &ActorKey, target: &CueTarget, to: &EntityKey, label: &str)` — RouteAdd/RouteSwitch の共通本体（resolve→register_actor→debug/warn）を1関数化し、ラベル引数で log を区別。RouteRemove は現状 `let _ = key`（"full impl deferred"）のため統合対象外（P1 拡張枠）。(b) `CuePayload::Command`/`Barrier` の配送手続きを `Entry<CueCommand>` を生成する小クロージャ + 共通配送ループへ括り出し、`routes_for_actor`→空スキップ→`insert`→`set_cue_sheet`→`seen_targets`/`all_targets` 集約の38行複製を単一ソース化。抽出に先立ち、E2E が未駆動の **Barrier 配送アーム・RouteAdd アームの特性化テスト**（dispatch 経由で barrier が各スロット queue に Entry::Barrier として入ること、RouteAdd が registry を更新すること）を `tests/ecs/cue_dispatch_e2e_test.rs` に追加して回帰検知器を整備したうえで実施する。観測可能挙動（配送結果・registry 状態・log は debug/warn レベルゆえ非観測扱い）は不変。実起動 S7 が統合的回帰検知器。なお RouteAdd/RouteSwitch の registry 作用が将来分岐する（例: RouteAdd は既存ルートに追加・RouteSwitch は置換、という名前空間統合 P1 拡張）場合は統合せず別ロジックとして保つべきであり、本提案は「現状 registry 作用が同一である限りの DRY」として適用範囲を限定する。

## P68: DolaAnimator をマルチスレッドスケジュールへ配線する際の Sync ハザード対策（内部 Rc の Arc 化 or SingleThreaded スケジュール固定）
- source: W8-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/dola/mod.rs` の `unsafe impl Send + Sync for DolaAnimator`（mod.rs:59-60、W8-S が SAFETY 注記を格上げ済み）の健全性を W8-V が実スケジュール構成で裏取りした結果、**現状は到達不能ゆえ安全だが、本型をプロダクションへ配線すると `Sync` 側に潜在ハザードがある**ことを確認した。裏取り事実: (1) `tick_dola_animators`/`DolaAnimator` はワークスペース全域で**プロダクションのいずれのスケジュールにも未登録・未 spawn**（grep 実証: 出現は `ecs/dola/mod.rs`・`ecs/mod.rs` 再エクスポート・`tests/ecs/dola_animator_test.rs` のみ。`ecs/world/mod.rs` の `schedules.add_systems` 群に `tick_dola_animators` は不在）。(2) ワークスペースは bevy_ecs の `"multi_threaded"` feature を有効化（ルート Cargo.toml `[workspace.dependencies.bevy_ecs] features` に `"multi_threaded"`）し、`Schedule` の既定 `ExecutorKind` は `MultiThreaded`（bevy_ecs-0.18.0 `schedule/executor/mod.rs:65-66`）。`ecs/world/mod.rs:82-84` は `UISetup` のみ `set_executor_kind(SingleThreaded)` で固定し、`Input`/`Update` 等は既定=マルチスレッドエグゼキュータで走る。`DolaAnimator` を `Send` にした本 impl により `Query<&mut DolaAnimator>` を持つシステムは `is_send==true` と判定され（`function_system.rs:84` / `multi_threaded.rs:545` のメインスレッド固定条件 `!is_send && local_thread_running` を満たさない）、**ワーカースレッド上で実行され得る**。`&mut` 排他アクセス経路（tick）は競合スケジューリングにより同時1スレッドが保証され健全だが、`Sync` は「複数の読み取り専用システム（`Query<&DolaAnimator>`）が跨スレで `&DolaAnimator` を同時共有する」ことを許す。`last_result()` が返す `UpdateResult.changes` は `EvaluatedValue::Object(Rc<DynamicValue>)`（dola `runtime/types.rs:24,144-146`）を含み得るため、もし将来そうした並列消費者が内部 `Rc` を `clone` すると非アトミック参照カウントにデータ競合（UB）が生じ得る。W8-S の SAFETY 注記が当初主張した「スケジュールが par_iter_mut で並列化されない／単一スレッドが排他を保証」は実構成（既定マルチスレッド）と不整合であり、W8-V が SAFETY 注記をコメントのみで是正（裏取り事実＋本ハザードの明文化）した。対策（下記）は型変更ないしスケジュール属性変更を伴い**観測挙動・スレッドモデルを変える**ため、R2.4/R5.2 に従い本ループでは実装せず記録のみ。
- suggestion: `DolaAnimator` を実際にプロダクションのスケジュールへ配線する設計（消費者システムで `last_result().changes` を ECS Component へ反映する経路）を入れる際に、いずれかを併せて実施する小〜中規模仕様: (a) `tick_dola_animators` と全 `DolaAnimator` 消費者システムを `SingleThreaded` 実行のスケジュール（既存 `UISetup` 同様 `set_executor_kind(SingleThreaded)`）へ載せ、跨スレッド共有を構造的に排除する（最小変更・dola 側非改変）。(b) より堅牢には dola `DolaRuntime` 内部の `Rc<DynamicValue>`（`interpolator::ObjectInternPool`・`EvaluatedValue::Object`）を `Arc<DynamicValue>` へ置換し、`DolaAnimator` を真に `Send + Sync` にして `unsafe impl` を撤去する（dola D 領域の型変更を要し影響大・本レビューループ境界外）。配線前に `tick_dola_animators` + 消費者の実スケジュール駆動テスト（マルチスレッドエグゼキュータ下で並列読み取りが起きないことの確認）を整備する。現状（未配線）では到達不能のため緊急度は低いが、配線とセットで必須の前提条件として扱う。

## P69: resolve_entity_ref の Entity ビット復元を try_from_bits 化（不正ビット・外部 CueSheet 由来の panic 経路の Result/None 縮退）
- source: W8-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: `crates/wintf/src/ecs/cue/queue.rs::resolve_entity_ref`（queue.rs:156-161）は `CueCommand::EntityRef(bits)` を `Entity::from_bits(*bits)` で復元するが、bevy_ecs-0.18.0 の `Entity::from_bits`（`entity/mod.rs:576-581`）は**不正ビット（`to_bits()` 由来でない値）に対して panic する**非フォールバック版である（下位 32bit = index ワードが 0 のとき `EntityIndex::try_from_bits` の `NonZero::new` 検査（`entity/mod.rs:201-208`）が拒否 → None → `panic!("Attempted to initialize invalid bits as an entity")`。`NonMaxU32` の transmute 表現により raw=0 が無効インデックスに対応する。W8-V が一時 probe で実測確認: `bits=0x0000_0001_0000_0000`（generation=1,index=0）→ panic、`0x0000_0000_FFFF_FFFF` → 正常復元）。`CueCommand` は `Serialize, Deserialize` 導出（dola `cue/command.rs:117-118`）で `EntityRef(u64)`（同:128）は**外部 CueSheet（ファイル/設定由来）の任意 u64 を運び得る**ため、これは外部入力到達可能な panic 経路（UI スレッドクラッシュ = DoS）である。現状リポジトリでは `push_entity_command`（queue.rs:139-146）が常に有効な `entity.to_bits()` を挿入し外部 CueSheet 経路で EntityRef を流す利用箇所が未実装のため実害は未発現だが、`resolve_entity_ref` 自体は任意ビットを検証しない。W8-T1 が往復恒等を特性化済みだが有効ビットのみを供給しており、本 panic 経路は未保護だった。W8-V で**現状の panic 挙動を `#[should_panic]` で固定する特性化テスト** `resolve_entity_ref_panics_on_malformed_bits`（queue.rs in-source）を追加（挙動非破壊・回帰検知器）。堅牢化（`from_bits`→`try_from_bits` で `None` 縮退）は `resolve_entity_ref` の戻り値が panic→`None` へ変わる**外部観測可能な挙動変更**（ドキュメント queue.rs:148-155 は既に「無効な Entity が返る可能性があるため消費者は Query で存在確認を行うこと」と謳い `None` 返却と整合的だが、現実装は panic）であり、R2.4/R5.2 に従い本ループでは実装せず記録した。P25（Cue パイプラインの時刻入力 NaN/inf 検証欠如）と同じく「外部 CueSheet 由来データの検証欠如」系統。
- suggestion: queue.rs:158 を `CueCommand::EntityRef(bits) => Entity::try_from_bits(*bits)`（`from_bits`→`try_from_bits`、戻り値は既に `Option<Entity>` なので `Some(...)` ラップを外す）へ置換する1行規模の修正。これにより不正ビットは panic せず `None` を返し、消費者（`Query` 存在確認）が安全にスキップする（ドキュメント記載の契約に実装を一致させる）。修正時は本ループ追加の `resolve_entity_ref_panics_on_malformed_bits`（`#[should_panic]`）を「不正ビットは `None` を返す」期待へ更新する（テスト弱体化ではなく仕様変更追随）。`push_entity_command` の正常往復テスト（`push_entity_command_inserts_entity_ref_with_matching_bits` 等 W8-T1）が有効ビット側の回帰検知器として整合確認になる。外部 CueSheet からの EntityRef 経路を実装する設計（P1 系のルーティング拡張）と併せて入れるのが望ましい。

## P70: 検証コマンド（S2）が feature ゲート付きテストを既定で実行しない（serde toml/yaml 往復テストの回帰非保護）
- source: X1-T
- kind: その他（テスト構成・検証スコープの是正候補。挙動変更なし）
- rationale: 本レビューループの正準検証コマンド S2（`cargo build --workspace` → `cargo test --workspace`、design.md プロファイル節）は **dola の既定 feature（`default = ["json"]`）のみ**でテストをビルド・実行する。`crates/dola/tests/general/integration_test.rs` の `#[cfg(feature = "toml")]` モジュール（`complete_document_toml_roundtrip`・`btreemap_key_order_deterministic_toml`、行193-212）と `#[cfg(feature = "yaml")]` モジュール（`complete_document_yaml_roundtrip`、行218-229）の計3テストは `toml`/`yaml` feature が既定外（`crates/dola/Cargo.toml` `[features]`: `toml = ["dep:toml"]`・`yaml = ["dep:serde_yaml"]`）のため、S2 では**1件もビルドされず実行もされない**。X1-T で実測確認: `cargo test --workspace` = **1713 passed**、`cargo test --workspace --all-features` = **1716 passed**（差分 +3 はこの3テストと厳密一致。`--all-features` ビルド・実行とも成功・失敗ゼロ）。feature ゲート自体は健全（toml/yaml dep 不在時に当該 crate を参照しない正しい条件分割であり、`toml::`/`serde_yaml::` 参照は cfg モジュール内に限定済み）だが、**ループの標準回帰検知（S2）が非既定 feature のシリアライズ往復経路を保護しない**点が設定起因のテスト網羅ギャップである。本ループ進行中（A1〜W8 の全 V/S/T セル）はこの3テストが一度も回帰検知に掛かっておらず、toml/yaml 経路を壊す変更が S2 を素通りし得た。なお既存の P14（D1b-V、指示書数値フィールドの有限性検証欠如）が「NaN/inf 流入経路は TOML/YAML のみで feature ゲートにより既定ビルド外」と言及しているが、P14 は脆弱性観点の記録であり、**検証スコープ（S2 が非既定 feature を実行しない）というテスト構成側の事実は未記録**であったため本提案で補う。S2 の定義変更は design.md「Revalidation Triggers」（検証コマンド S2 の変更＝全セルの非破壊確認の意味が変わる）に該当し、プロファイル（X1-S/X1-V）の領分かつ本ループの検証契約に影響するため、X1-T では設定変更を行わず記録に留めた（CI 欠落と同様、構成是正の判断は上位セル/別仕様へ）。
- suggestion: 検証コマンド S2 に feature 全網羅の一巡を追加する小規模のプロファイル是正。選択肢: (a) S2 を `cargo test --workspace` に加えて `cargo test --workspace --all-features`（または dola 限定で `cargo test -p dola --all-features`）の二段実行とし、非既定 feature の往復テストを回帰検知へ取り込む（最小・既存テストの increase は +3 で実測済み）。(b) feature 組合せを CI（本ループ対象外・別仕様）で網羅する場合は、CI 新設仕様（後述の CI 欠落所見）に「`--all-features` ジョブ」を含める。挙動変更は伴わず（テスト実行範囲の拡大のみ）、(a) は design.md プロファイル節 S2 の値差し替え + Revalidation Triggers に基づく全セル再検証不要の追補（既存セルの成果物は不変、検知範囲が広がるのみ）で済む。導入時は dola の `toml`/`yaml`/`json` 各 feature 単独および全部有効の組合せがビルド・テスト可能であることを確認する。

## P71: 各クレートの `publish = true` 上書きと要件前提（`publish = false`）の整合（公開ポリシーの意図確認）
- source: X1-S
- kind: その他（公開ポリシー＝公開可否という挙動相当の設定。意図確認を要する）
- rationale: ルート `Cargo.toml` の `[workspace.package]` は `publish = false`（行13）を宣言するが、3クレートの各 `[package]` がいずれも `publish = true` で明示上書きしている（`crates/areka/Cargo.toml:10`・`crates/dola/Cargo.toml:10`・`crates/wintf/Cargo.toml:10`、実ファイルで確認）。これは requirements.md「Boundary Context」Adjacent expectations（行25「本ワークスペースのクレートは未公開（`publish = false`）であり外部利用者を想定しない。したがって非推奨かつ利用箇所ゼロのコードは後方互換性を考慮せず削除できる（R2.9 の前提）」）と矛盾する。実効的には各クレートは `cargo publish` 可能（crates.io への公開が許可）であり、本ループが R2.9/R5.3 で前提とした「外部利用者なし・後方互換性考慮不要」という土台が設定実態と食い違う。本ループの削除判断（W1-S 等）はこの前提に依拠しているため、整合性確認が必要。ただし `publish` フラグの変更（true→false への是正、または要件前提の更新）は**クレートが公開可能か否か**という外部に対する公開可否＝挙動相当の設定変更であり、design.md「Revalidation Triggers」（行52「クレートの公開ポリシー変更（`publish = false` 解除）— R2.9 の非推奨コード削除前提が崩れる」）が明示的にトリガー対象とする領分のため、明白な設定ミスと断定して X1-S で機械的に書き換えることはせず記録に留めた（W1-S → X1-T → 本 X1-S と申し送られた所見。X1-V の依存監査と併せて最終判断するのが安全）。なお `publish = true` は crates.io 既定値であり、ルート workspace の `false` を各クレートが個別に true へ戻す構成は「将来の公開を見据えた意図的設定」「テンプレート由来の惰性上書き」のいずれの可能性もあり、コードからは意図を断定できない。
- suggestion: 公開ポリシーの単一の真実源を確定する小規模仕様。選択肢: (a) 本当に未公開運用なら各クレートの `publish = true` を削除し `[workspace.package]` の `publish = false` を継承させる（requirements.md 前提と一致、R2.9 の削除判断の土台が設定実態に裏打ちされる）。(b) 公開予定があるなら requirements.md Adjacent expectations（行25）と design.md Revalidation Triggers（行52）を「クレートは公開対象」と改訂し、非推奨/dead コード削除時に後方互換性（SemVer）を考慮する方針へ更新する。いずれも公開可否という挙動相当の設定/前提変更を伴うため、本レビューループ（挙動非破壊）の範囲外として別途意図確認のうえ適用する。

## P72: `[profile.release]` のビルド最適化設定（opt-level='z' / lto / codegen-units / strip 注記）の見直し
- source: X1-S
- kind: その他（ビルド成果物の最適化＝観測可能な成果物特性を変える設定）
- rationale: ルート `Cargo.toml` の `[profile.release]`（行82-92）は `opt-level = 'z'`（サイズ最優先）・`lto = true`・`codegen-units = 1`・`panic = 'unwind'`・`strip = false` を設定している。S6（karpathy）観点では (1) `opt-level = 'z'`（サイズ最優先）はデスクトップマスコット（areka）の実行時性能（描画・アニメーション）とトレードオフであり、用途的に `'s'`（サイズ寄りだが 'z' ほど積極的でない）や `3`（速度優先）が適切な可能性がある、(2) 各設定行の末尾コメント「(変更)」（行89・91）は過去の編集を指す陳腐化した注記で説明価値が低い、という簡素化/見直し候補がある。しかしこれらはいずれも**リリースビルド成果物のバイナリサイズ・実行時性能・パニック戦略・デバッグシンボル有無という観測可能な成果物特性を変える**（または変える設定ブロック内の記述である）ため、本タスク本文の指示（「ビルド成果物の挙動（リリース最適化・LTO 等）を変える設定変更は適用せず `report/proposals.md` へ記録する」）および R5.1 に従い、X1-S では一切変更せず記録のみとした（コメント注記の除去も、成果物特性を決定する profile ブロック内への編集は churn かつ判断を要するため見送り）。現状の設定は妥当に機能しており（`cargo build --workspace` 成功・成果物生成可能）、緊急の不具合ではない。
- suggestion: リリースプロファイルの最適化方針を用途（デスクトップマスコットの起動速度・描画性能 vs 配布サイズ）に照らして見直す小規模仕様。`opt-level`（'z'/'s'/2/3）・`lto`（true/"thin"/false）・`codegen-units`・`strip`（true でシンボル除去しサイズ削減 vs 現状 false でバックトレース優先）の各トレードオフをベンチ（起動時間・バイナリサイズ・実行時 FPS）で評価し、合意した値へ更新する。あわせて「(変更)」等の陳腐化コメントを実態説明へ整理する。いずれもリリース成果物の特性を変えるため、ベンチ計測を伴う独立タスクとして扱う（本レビューループの挙動非破壊原則の範囲外）。

## P73: `rand` クレートの RustSec advisory（RUSTSEC-2026-0097 unsound）対応パッチ更新（0.10.0→0.10.1 / 0.9.2→0.9.4）
- source: X1-V
- kind: 挙動変更を伴う脆弱性対策
- rationale: X1-V の依存監査（`cargo audit` 0.22.2 実行、RustSec Advisory DB 1132 件で Cargo.lock の 300 クレート依存をスキャン）で検出された唯一のプロダクション混入アドバイザリ。`rand@0.10.0`（`crates/dola/Cargo.toml:18` 経由の直接 dep → wintf → areka）と `rand@0.9.2`（`pasta_core@0.1.6` → dola → wintf → areka、`cargo tree -i` 実測）が **RUSTSEC-2026-0097（informational = unsound）** に該当する。advisory（`advisory-db/crates/rand/RUSTSEC-2026-0097.md` 直読）の UB 発火条件は「rand の `log`+`thread_rng` feature 有効 ＋ カスタムロガー（`impl log::Log`）定義 ＋ そのロガーが `rand::rng()` を呼び再シード中に再入」の全成立が必要だが、本ワークスペースでは (1) Cargo.lock 上 rand の依存に `log` 不在＝**rand の `log` feature オフ**（rand はログを発行しない）、(2) `set_logger`/`impl log::Log` のヒット 0 件＝**カスタムロガー不在**（tracing/tracing-subscriber を使用）——の2条件が独立に不成立で、現状は **到達不能（発火経路なし）**。したがって緊急の挙動非破壊対策（コード変更）は不要だが、informational/unsound advisory として patched 版（`>=0.10.1` / `>=0.9.3`）へ追随する防御的価値はある。`cargo update -p rand@0.10.0 --dry-run` = `0.10.0 → 0.10.1`、`rand@0.9.2 --dry-run` = `0.9.2 → 0.9.4` と実測（いずれも解決可能）。本ループで適用しない根拠: (a) `rand@0.9.2` は `pasta_core`（`vendors/pasta` 配下＝R1.5 で本ループ改変禁止）が引くため、その更新は pasta サブモジュール側の領分（areka 側 Cargo.toml の変更では届かない）、(b) Cargo.lock が未追跡（P75）のため `cargo update` の結果は永続コミットされず、各環境/CI は caret 範囲（`rand="0.10.0"` は `<0.11` を許容）から都度最新を解決する＝Cargo.toml は既にパッチ版を許容済み、(c) パッチ更新でも乱数列・API の挙動互換性評価（dola loop_offset 乱数への影響）を要し「挙動影響を評価のうえ慎重に」（design.md L516）の対象。R2.4/R5.2 に従い記録のみ。
- suggestion: rand を patched 版へ更新する小規模仕様。(a) areka/dola 側は `crates/dola/Cargo.toml` の `rand` を `"0.10.1"`（または `"0.10"`）へ引き上げ、S2 全量グリーン＋乱数依存テスト（`facade_test` の loop_offset 系・分布検定）で挙動非破壊を確認。(b) `rand@0.9.2` 側は `vendors/pasta` の `pasta_core` が `rand` を `>=0.9.3` へ更新するのを待つ（または pasta 側へ upstream PR）。(c) 併せて P75（Cargo.lock 追跡）を実施すると更新が固定・再現可能になる。現状は到達不能のため緊急度は低い。

## P74: 依存監査（`cargo audit`）の CI 導入
- source: X1-V
- kind: その他（CI 新設＝本ループ対象外。継続的な依存監査基盤の所見）
- rationale: X1-V で `cargo audit`（0.22.2）を手動実行し、脆弱性 0 件・情報的警告 5 件（RUSTSEC-2026-0097 rand unsound ×2、RUSTSEC-2024-0436 paste unmaintained、RUSTSEC-2026-0105 core2 unmaintained+yanked）を検出した。うち paste/core2 は `image`（wintf の dev-dependency）経由で出荷物には非混入だが、これら informational/unmaintained 検出や将来の新規 advisory を**継続的に検知する基盤がない**（X1-T が確認したとおり CI 自体が不在: `.github/workflows` 等いずれも不在）。CI 新設は design.md Non-Goals（本レビューループ対象外）であり X1-V では実装しないが、依存監査を一度きりでなく回帰的に行うには CI への `cargo audit` ジョブ組み込みが妥当。X1-T が記録した CI 欠落所見・P70（S2 の `--all-features` 網羅）と同じく「CI 新設仕様」へ集約すべき構成是正候補。
- suggestion: CI 新設仕様（P70・X1-T の CI 申し送りと統合）に `cargo audit`（または `cargo deny`）ジョブを含める。informational 警告（unmaintained/yanked）を fail とするか warn に留めるかの方針、dev-only 依存（paste/core2 等）の扱い（`--ignore` 指定 or 許容）を設計判断として含める。本ループの挙動非破壊原則の範囲外（CI = ビルド/配布インフラ）として別仕様で扱う。

## P75: Cargo.lock の git 追跡方針の確定（バイナリ生成ワークスペースの再現性・依存固定）
- source: X1-V
- kind: その他（ビルド再現性＝依存解決を全消費者へ固定する挙動相当の方針変更）
- rationale: X1-V の依存固定点検で、`.gitignore` 行2が `Cargo.lock` を除外し **Cargo.lock が git 未追跡**（`git ls-files Cargo.lock` 該当なし、ローカルには 72621 バイトで実在）であることを確認した。`git log --all -- Cargo.lock` も空＝**一度も追跡されたことがなく**、`.gitignore` への `Cargo.lock` 追加は初回コミット f189c1b（プロジェクト基盤）からの意図的設定。本ワークスペースは `areka` が `[[bin]]` を持つ**バイナリ生成プロジェクト**であり、Cargo 自身のガイダンス上はバイナリ（実行ファイル）を生成するパッケージでロックファイルをコミットするのが推奨される（推移的依存の版を固定し、ビルド再現性・サプライチェーン固定性を確保するため）。現状の未追跡では、上記 P73 の rand 等を含む推移的依存が各開発環境/CI で caret 範囲（例 `rand="0.10.0"` は `<0.11`）から都度最新へ解決され、再現性が保証されない（ある環境では 0.10.0、別環境では 0.10.1 等）。ただし「Cargo.lock を追跡する」変更は **build が解決する依存集合を全消費者に対して固定する＝ビルド再現性の方針変更**であり、初回コミットからの意図的除外を覆す挙動相当の設定変更（かつ 72KB のロックファイルを追跡対象に加える）のため、本 V セルでは適用せず記録のみ。なお lib クレート（dola/wintf）単体観点では lock 非追跡にも一定の合理性があり、本ワークスペースが「配布バイナリ areka 中心」か「再利用ライブラリ群」かの位置付け（P71 の公開ポリシーと連動）で判断が変わる。
- suggestion: 配布/公開ポリシーの単一の真実源（P71）と併せて Cargo.lock 追跡方針を確定する小規模仕様。(a) バイナリ配布（areka）の再現性を重視するなら `.gitignore` から `Cargo.lock` を除去して追跡開始し、以後は依存更新（P73 等）が lock に固定・レビュー可能になる。(b) ライブラリ再利用を重視し下流に解決を委ねるなら現状維持（非追跡）を明文化する。追跡開始は依存集合を固定する挙動相当の変更のため、P71（publish 方針）・P74（cargo audit CI）と統合した配布/サプライチェーン方針仕様として扱う。
