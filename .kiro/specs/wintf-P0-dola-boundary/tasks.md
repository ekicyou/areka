# Implementation Plan

> **フェーズ実行順序**: Phase 2a（Tasks 1）→ Phase 1（Tasks 2–6、一部並列）→ Phase 2b+3（Tasks 7–8）→ Phase 4（Task 9）→ NFR（Task 10）
>
> **要件カバレッジ注記**: Req 4（UpdateResult 活用方針）および Req 5（設計ドキュメント整合性）は設計書・アーキテクチャ仕様レベルの要件であり、design.md がその成果物となる。コード実装タスクからは除外する。

---

- [ ] 1. wintf から DolaRuntime 誤配置コードを除去する

- [ ] 1.1 (P) cue モジュールの DolaRuntime 関連コードとシステムを削除する
  - DolaRuntime ラッパーファイル（cue/runtime.rs）を削除する
  - cue/systems.rs から DolaRuntime を更新するシステムと関連エラーハンドリングを除去する
  - cue/mod.rs から DolaRuntime の公開再エクスポート宣言を削除する
  - 削除後にコンパイルが通ることを確認する
  - _Requirements: 3.1_

- [ ] 1.2 (P) 既存の DolaRuntime 統合テストファイルを分割・整理する
  - FrameTime に関する 3 テストを ECS グラフィクスのテストグループに移動する
  - DolaRuntime に関する 5 テストを削除し、テストファイル自体を除去する（DolaAnimator の同等テストは Task 9 で新設）
  - テストモジュール宣言を更新してコンパイルが通ることを確認する
  - _Requirements: 3.1, NFR-1.3_

---

- [ ] 2. (P) dola に演出コマンド・ドメイン型を実装する

- [ ] 2.1 (P) バリア種別・ルーティングコマンド・演出コマンド・統合ペイロード型を実装する
  - バリア種別（クリック/キー入力待ち・選択肢待ち・タイムアウト の 3 種）をタイムアウト値オプション付きで実装する
  - ルーティングコマンド（配送スロット追加・切替・除去 の 3 種）を実装する
  - 演出コマンド（テキスト・クリア・感情表現・選択肢・エンティティ参照(u64)・カスタム の 6 種）を実装し serde 対応とする
  - コマンド・バリア・ルーティングを統一記述できる統合ペイロード型を実装し、各型から `From` 変換を提供する
  - 全型に `Clone + Debug + PartialEq + Serialize + Deserialize` を実装する
  - _Requirements: 1.1, 1.3, 1.5, 1.5a_

- [ ] 2.2 演出パイプラインのドメイン型を実装する
  - アクター識別子型（ActorKey: String ラッパー、Hash + Eq 対応）を実装する
  - 配送先スロット型（CueTarget: シェル / バルーン の 2 種）を実装する
  - ルーティングキー型（EntityKey: アクター / スポット / バルーン の 3 種）を実装する
  - 個別演出指示型（Cue: アクター + 相対時刻 + ペイロード の 3 フィールド）を実装する
  - 全型に serde 対応を追加する
  - _Requirements: 1.1, 1.6_

---

- [ ] 3. (P) DolaRuntime に tick() と last_result() の 2 フェーズ API を追加する
  - DolaRuntime に最終更新結果を格納するための内部フィールドを追加する
  - `tick(current_time: f64)` を実装する: 現在時刻まで内部状態を進行し結果を内部フィールドに格納する
  - `last_result(&self) -> &UpdateResult` を実装する: 直前の tick 結果を読み取り専用で返す
  - 既存の `update()` を `#[deprecated]` としてマークし、tick() + last_result().clone() で後方互換を維持する
  - _Requirements: 1.7, 1.8_

---

- [ ] 4. dola に TimedSchedule&lt;T&gt; を実装する

- [ ] 4.1 エントリの型定義と挿入 API を実装する
  - `Entry<T>` の 3 種分離 enum を定義する（Payload はジェネリクス T、Barrier は固定のバリア種別型、Routing は固定のルーティングコマンド型）
  - `TimedSchedule<T>` 構造体（絶対時刻基準・降順ソートエントリ列・ペイロードバッファ・ルーティングバッファ・現在バリア を内包）を定義する
  - 0 ベース相対オフセット降順ソートを維持した単一挿入と一括挿入 API を実装する
  - _Requirements: 1.1, 1.2_

- [ ] 4.2 tick() と ready() の 2 フェーズ API を実装する
  - `tick(current_time: f64)` を実装する: 絶対時刻から相対オフセットへ変換し、時刻到達済みの Payload をペイロードバッファに、Routing をルーティングバッファに蒐集しながら進行する。Barrier 到達または末尾到達で停止。Timeout Barrier は時刻比較で自動解除。冪等性保証（同一時刻の再呼び出し安全）
  - `ready(&self) -> &[T]` を実装する: 次の tick() 呼び出しまで何度でも読み取り専用で参照可能にする
  - `remaining()` / `is_completed()` / `clear()` のユーティリティを実装する
  - _Requirements: 1.2_

- [ ] 4.3 バリア管理とルーティング収集 API を実装する
  - `current_barrier()` で現在の停止バリア種別を照会できるようにする（UI 表示用）
  - `notify_barrier_resolved(choice_id: Option<String>)` でプッシュ通知によるバリア解除を実装する（入力待ち: None、選択肢待ち: Some(選択ID)、非バリア時は no-op）
  - `next_routing()` でルーティングバッファから記述順に 1 件ずつ取得できるようにする（ready() には含まれない）
  - _Requirements: 1.2, 1.3_

---

- [ ] 5. dola に CueSheet と compile_sheet を実装する

- [ ] 5.1 CueSheet 型と Actor フィルタリング API を実装する
  - `CueSheet` を実装する（生成時に start_time 昇順ソートを保証）
  - 全 Cue、Actor 絞り込み、出演 Actor 一覧取得などの参照系メソッドを実装する
  - pasta DSL 出力との JSON / TOML / YAML 接続のために serde 対応を実装する
  - _Requirements: 1.1, 1.4, 1.9_

- [ ] 5.2 compile_sheet 関数を実装する
  - CueSheet の相対 start_time を整列し、最小値を 0 基準に正規化して 0 ベース相対オフセットを生成する
  - 統合ペイロード型の into_entry() で各 Cue を `Entry<CueCommand>` に変換する
  - コンパイル済みエントリを Actor ごとに分類した構造体 `CompiledCue { offset, actor, entry }` のリストを返す
  - _Requirements: 1.1, 1.4_

---

- [ ] 6. dola cue モジュールを統合・公開してユニットテストを実装する

- [ ] 6.1 lib.rs に cue モジュールを公開して主要型を再エクスポートする
  - lib.rs に cue サブモジュールを追加し、cue/mod.rs を schedule / command / sheet の再エクスポートハブとして構成する
  - 2 エンジン（連続値アニメ / 離散コマンド配信）の責務分離が型レベルで表現されていることを確認する
  - _Requirements: 1.1, 1.8_

- [ ] 6.2 (P) TimedSchedule のユニットテストを実装する
  - 時刻到達済み Payload の正確な収集・冪等性・同一時刻複数 Payload を検証する
  - バリア到達での停止・notify_barrier_resolved() 後の再進行・Timeout 自動解除を検証する
  - next_routing() による順次取得と ready() への非混入を検証する
  - _Requirements: 1.2, 1.3, NFR-1.2_

- [ ] 6.3 (P) CueSheet と compile_sheet のテストを実装する
  - CueSheet 生成時の start_time 昇順ソートを検証する
  - compile_sheet() の 0 ベース正規化の正確性を検証する
  - ペイロード統合型の into_entry() における 3 種変換を検証する
  - _Requirements: 1.4, NFR-1.2_

- [ ] 6.4 (P) DolaRuntime tick/last_result のテストを実装する
  - tick() + last_result() の組み合わせが旧 update() と同等の結果を返すことを検証する
  - last_result() の冪等性（tick() 呼び出しなしで同一結果を返す）を検証する
  - deprecated update() の後方互換を検証する
  - _Requirements: 1.7, NFR-1.2_

---

- [ ] 7. wintf cue の型定義を dola 再エクスポートに置き換える

- [ ] 7.1 cue/command.rs を再エクスポートのみのファイルに置き換える
  - wintf cue/command.rs の全型定義を削除し、dola cue から一括再エクスポートする形に書き換える
  - cue/mod.rs から不要なインポート・宣言が残っていないことを確認する
  - _Requirements: 3.5_

- [ ] 7.2 コンパイルエラーを解消して後方互換を確認する
  - 型パス変更に伴うコンパイルエラーをすべて解消する
  - wintf cue 系既存テスト（75 件）が全パスすることを確認する
  - _Requirements: 3.5, NFR-1.1_

---

- [ ] 8. CueQueue を TimedSchedule&lt;CueCommand&gt; 委譲設計に再設計する

- [ ] 8.1 CueQueue に TimedSchedule&lt;CueCommand&gt; を内包した push/pop API を再設計する
  - CueQueue 内部のスケジューリングロジック（並び替え・時刻消費・バリア管理）を TimedSchedule への委譲に置き換える
  - CueQueue の ECS 固有状態（再生状態・再生レート・選択肢蓄積・シートエンティティ参照 等）は CueQueue に残す
  - CueQueue から TimedSchedule の tick() を呼び出し、ready() でコマンドスライスを取得する設計に統一する
  - _Requirements: 3.2, 3.3_

- [ ] 8.2 新 CueSheet 投入時の全破棄・再構築フローを実装する
  - dispatch システムで compile_sheet() を呼び出し、Actor ごとに CompiledCue を分配する処理を実装する
  - 新 CueSheet 投入時に既存スケジュールを全破棄（クリア → 新規 → 一括挿入）する挙動を実装する
  - バリア中でも強制的に新スケジュールへ切り替えることを保証する
  - _Requirements: 3.2, 3.3_

- [ ] 8.3 エンティティ参照の変換を push/pop 境界に実装する
  - CueCommand::EntityRef を CueQueue に投入する際に ECS Entity → u64 変換を行うヘルパーを実装する
  - CueCommand::EntityRef を取り出す際に u64 → ECS Entity へ復元し、Query による存在確認を行う処理を実装する
  - _Requirements: 3.4_

- [ ] 8.4 CueQueue + TimedSchedule の統合フローをテストする
  - 投入 → tick → ready → バリア停止 → 解除通知 の一連フローを統合テストで検証する
  - 新 CueSheet 投入による全破棄・再構築の挙動を検証する
  - Entity 参照のラウンドトリップ変換を検証する
  - _Requirements: 3.2, 3.3, 3.4, NFR-1.1_

---

- [ ] 9. wintf に DolaAnimator ECS Component を実装する

- [ ] 9.1 DolaAnimator Component と tick_dola_animators System を実装する
  - `ecs/dola/` モジュールを新設し、DolaRuntime を所有する `DolaAnimator` Component を実装する（Rc を内包するため unsafe impl Send + Sync、安全性根拠をコード内 doc comment に記載）
  - `tick_dola_animators` System を実装する（全 DolaAnimator エンティティを Res<FrameTime> の時刻で一括 tick、Update スケジュール先頭に配置）
  - DolaAnimator の公開 API（生成・tick・結果参照・ランタイム読み取り）と crate 内部限定の可変アクセサを実装する
  - 消費者システムが Query<&DolaAnimator> と .after(tick_dola_animators) で結果を安全に読み取れるパターンを実現する
  - balloon06 の DolaBridgeResource 設計を DolaAnimator Component 設計で上書きする旨と配置先モジュールをコード内 doc comment に記載する
  - ecs/mod.rs に dola モジュールを追加し System を登録する
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 9.2 DolaAnimator の ECS 統合テストを実装する
  - DolaAnimator の spawn → tick → last_result 読み取りの一連フローを tests/ecs/dola/ に実装する
  - tick_dola_animators System による全エンティティ一括 tick を検証する
  - internal な可変アクセサが crate 内部からのみアクセス可能であることを確認する
  - _Requirements: 2.1, 2.2, NFR-1.1_

---

- [ ] 10. 全体回帰テストと後方互換を確認する

- [ ] 10.1 wintf 全テストスイートの全パスを確認する
  - wintf の全テスト（920+ 件、cue 系 75 件含む）がパスすることを確認する
  - 全サンプルアプリケーション（examples/）がパニックなく起動することを確認する
  - _Requirements: NFR-1.1_

- [ ] 10.2 dola 全テストスイートの全パスを確認する
  - dola 既存テストがすべてパスし、連続値タイムライン機能（DolaRuntime・DolaDocument・compile_storyboard）の動作が変わっていないことを確認する
  - 新規追加した cue/ モジュールのテストが全パスすることを確認する
  - _Requirements: NFR-1.2_
