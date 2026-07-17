# Implementation Plan

## Task List

- [x] 1. HitCore: hit_region() 純関数と RegionPriority 型シームを実装し、全判断分岐を単体テストで固定する
  - `RegionPriority`（`Painter` 単一 variant・`#[non_exhaustive]`・`Default`）を実装する
  - `hit_region(&SurfaceMaster, x, y, priority) -> Option<&str>` を実装する：閉区間比較（4辺すべて含端）・`collisions` を逆順走査して最初に当たった領域を返す（画家則＝後定義が手前）・α／`collision-sort`／DPI／wintf 型を一切参照しない・反転/退化矩形は正規化せず当たらない
  - `hit_region` 内の `RegionPriority` に対する match には**ワイルドカード `_` アームを置かない**（優先規則の型シーム 2.3 を守る唯一の担保機序であり、テストでは代替できないためコード doc に明記しレビュー担保とする）
  - `lib.rs` へ `mod hit;` と `pub use hit::{hit_region, RegionPriority};` を追加し、`normalized.rs` の `collisions` フィールド doc へ順序不変条件（登場順・`surface.append` は末尾連結・画家則はこの順序に意味論を載せる）を追記する（型・フィールド・挙動は不改変）
  - 単体テスト（in-source `#[cfg(test)]`）: 矩形内→領域名／矩形外→None／境界 on-off（4辺4隅の closed-interval）／重なり→後定義が勝つ／collision 未定義→None／反転退化矩形→None／同名重複→最前面が勝つ／`fold` 出力（`surface.append` 経由）を入力にした画家則の順序檻／同一入力の反復呼出で決定論
  - Observable: `cargo test -p areka-emo-compose hit::` が全緑（9 ケース網羅）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 6.1, 6.2, 7.1, 7.2_
  - _Boundary: HitCore_

- [x] 2. CurrentSurfaceRead: current_surface_id 状態と accessor を additive に実装し、ライフサイクル全遷移を単体テストで固定する
  - `PresentTarget` へ private フィールド `current_surface_id: Option<u32>` を追加し、既存の `visible` 更新点と同一の3箇所（表示成立／`EmptyComposition` 縮退／Hide）でのみ書き込む（分岐を1本も足さない・失敗経路は前値保持が自動成立）
  - `EmoPresenter::current_surface_id(target) -> Option<u32>` と `EmoPresenter::hit_region(target, x, y) -> Option<&str>`（`current_surface_id` → `EmoWorld::surface` → HitCore の `hit_region` の順に合成）を追加する
  - 単体テスト（既存 presenter テスト方式に準拠）: 未表示→None／表示後→直近 id／切替→新 id／Hide→None／InvalidateCache→不変／未登録 target→None／既存 present テストスイートの非退行
  - Observable: `cargo test -p areka-emo-present presenter::` が全緑・既存テストスイートも無退行
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Boundary: CurrentSurfaceRead_

- [x] 3. HitRegionContract と Resolver: scope→target→id→純関数を束ねる I/O 契約を実装する
  - `crates/areka/src/emo2_boot/hit_region.rs`（新規）に `HitRegion { scope: u32, region: Option<String> }` を定義する（`region` は不透明 String・意味解釈しない）。型 doc に**本リゾルバは shell 窓専用（target 偶数）であり balloon（target 奇数）は扱わない**制約を明記する
  - 同ファイルに `resolve_hit_region(presenter: &EmoPresenter, scope: u32, x: i64, y: i64) -> HitRegion` を実装する：`super::target_map::shell_target` で scope→target 写像→`EmoPresenter::hit_region` で解決→`HitRegion` へ包む。非テストコードは `crate::` パスを一切使わず `super::target_map` と外部 crate のみを参照する（ファイル冒頭 doc に規律と理由を明記）。関数 doc にも shell 窓専用の制約を明記する
  - `emo2_boot/mod.rs` へ `pub mod hit_region;` を1行追加する
  - 単体テスト（in-crate `#[cfg(test)]`・GPU 不要）: 未表示 scope（`EmoPresenter::new()` ＋未 attach）→ `HitRegion { scope, region: None }`
  - Observable: `cargo test -p areka emo2_boot::hit_region::` が緑・`cargo build -p areka` が成功
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4_
  - _Boundary: HitRegionContract, Resolver_

- [ ] 4. Probe: リゾルバの座標契約を実 DPI・本番 emo2 表示で実測する
- [ ] 4.1 実 DPI 受け入れ example を実装する（本番窓寸規則駆動・マウス経路照合込み）
  - `crates/areka/examples/collision-probe.rs` を新設し、`#[path]` で `target_map.rs`／`hit_region.rs`／`placement/mod.rs` を私有 include する（read-only 再利用・production 不改変）
  - `placement::spawn::spawn_ghost_windows` を意図的に誤った placeholder 寸で呼び窓を生成（`Anchored` 付与）→ `attach_target` → `apply(ShowSurface { surface_id: 1000, binds: 実 bind 値集合 })` を適用する
  - `text_slot_view(shell_target(0)).surface_size()` を読み、本番の `placement::follow::resize_window_to` で窓へ適用する（戻り値 `true` を assert）
  - 本番 resize 適用の次フレーム以降に、実窓の `GetClientRect`（`WindowPos` ミラーではなく）が `surface_size()` と一致し `scale() == 1.0` であることを assert する（k=1.0 assert）
  - `EmoPresenter::read_back()` で Head/Bust 各矩形の中心画素が不透明であることを assert する（マウス非依存の描画一致 anchor）
  - probe 窓へ `OnPointerMoved` ハンドラを装着し、記録行ごとに `PointerState.client_point` と `ScreenToClient(GetCursorPos())` をペア列（Δ=(0,0) 厳密一致）で実測表へ記録する仕組みを組み込む
  - `GetCursorPos`→`ScreenToClient` で得た目視由来の client 点を `resolve_hit_region` へ渡し解決結果を live ログする（collision 実値からの座標合成・`SetCursorPos`/`SendInput` 注入は禁止）
  - Observable: `cargo build --example collision-probe -p areka` が成功し、rustdoc に①〜⑥のプロトコルが連番で記載されている
  - _Requirements: 4.3, 7.3_
  - _Depends: 3_
  - _Boundary: Probe_

- [ ] 4.2 実 DPI（≠96）2 水準での手動受け入れ検証を実施し acceptance-record.md へ記録する
  - per-monitor v2・実 DPI ≠96 を2水準以上（例: 125%/150%/200%）で collision-probe example を実行し、①〜⑥のプロトコル全項目を確認する
  - k=1.0 assert・read_back anchor assert・マウス経路ペア列（Δ=(0,0) 一致）・Head/Bust/None の目視解決一致を実測値（物理 px）で表へ記録する
  - 撫で一周（マウス→SHIORI→応答 talk）の統合実機サインオフは本 spec の対象外（7.4・`input-events` Req8.3 が実施）である旨を判定行に明記する
  - Observable: `acceptance-record.md` が2 DPI 水準の全項目 PASS と 7.4 の担当外注記を含んだ状態で存在する
  - _Requirements: 7.3, 7.4_

## Implementation Notes

- **並列マーカー不使用の理由**: 4 major task の新規コードは design.md「Allowed Dependencies」の crate 依存方向（`areka-parsers → areka-emo-atlas → areka-emo-compose → areka-emo-present → areka(bin)`）どおりに前段を直接呼び出す線形連鎖（Task2 の `EmoPresenter::hit_region` は Task1 の `hit_region()` を呼ぶ／Task3 の `resolve_hit_region` は Task2 の `EmoPresenter::hit_region` を呼ぶ／Task4 は Task3 の `resolve_hit_region` を呼ぶ）。真のデータ依存が全段に渡るため `(P)` マーカーは1件も付与していない。
- **要件 7.4 は本 spec 内で実装しない**: 「撫で一周の統合実機サインオフを撫でクラスタ合流サインオフへ帰属させる」は Coordination Notes C-4／Non-Goals で既に決着済みの設計判断であり、実装作業を要さない。Task 4.2 の acceptance-record 判定行にその旨を明記することでカバーする（意図的な繰延・追跡先＝`input-events` Req8.3）。
- **Task 1/2/3 が単一チェックボックスに畳まれている理由**: 各 major task の実質的な作業単位が「1コンポーネントの実装＋その単体テスト」という不可分の1成果物であるため（Task Hierarchy Rules の畳み込み規則）。Task 4 のみ「example 実装」と「実 DPI 手動検証・記録」という独立した2つの検証可能な成果物を持つため `4.1`/`4.2` の2段構成を維持する。
