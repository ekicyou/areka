# Implementation Plan

## Task List

- [x] 1. Foundation: placement モジュールの土台と依存関係を整備する
  - `crates/areka/Cargo.toml` で `areka-emo-atlas`／`areka-emo-compose` を dev-dependencies から通常の dependencies へ昇格する
  - `crates/areka/src/placement/` 配下に `mod.rs`（`resolver`／`config`／`source`／`measure`／`spawn`／`follow` の空サブモジュール宣言）を新設する
  - `PlacementError`（thiserror 構造化 enum。`Mount`／`DescriptRead`／`Measure` バリアント）を定義する
  - 観測可能な完了状態: `cargo build -p areka` が新モジュール構成（未使用コード許容）で成功し、`cargo tree -p areka` で emo-atlas／emo-compose が非 dev 依存として表示される
  - _Requirements: 1.1_
  - _Boundary: placement (scaffold)_

- [ ] 2. Core: descript 取り込み層（カスケード解決とソース読込）
- [x] 2.1 (P) 4 層カスケード解決とスコープ検出を実装する
  - `Alignment`（`Bottom`／`Free`／`Seam(String)`）・`ScopeConfig`・`BalloonSide`・`PlacementConfig` を定義し、ghost 全体＜ghost スコープ別＜shell 全体＜shell スコープ別の順で後勝ち解決する `build_placement_config` を実装する
  - `defaultx`⇔`defaultleft`／`defaulty`⇔`defaulttop` の両表記を同スロットへ寛容受理し、同層競合時は `defaultx`/`defaulty` を優先する
  - スコープ検出（scope0 常設・`kero.name` または shell `kero.*` キーで scope1・`char{n}.*` で scope n≥2）と `zorder`／`sticky-window`／shell `dpi` の転記フィールドを実装する
  - 単体テスト T-C1（4 層カスケードの全パターン勝敗）・T-C2（両表記寛容と優先順位）・T-C3（スコープ検出シグナル）・T-C4（emo2 実測 KV 相当の入力で `{0: bottom/defaultx 0/balloon left, 1: bottom/defaultx 0/balloon right}` を得ることと `(400,200)`/`(335,0)` 等のデモ定数が一切出現しないこと）・T-C5（zorder/sticky/dpi の raw 転記）を追加する
  - 観測可能な完了状態: `cargo test -p areka config::` で T-C1〜T-C5 が全て緑になる
  - _Requirements: 1.1, 1.3, 1.5, 2.2, 2.3, 2.5, 2.7, 2.8, 5.2_
  - _Boundary: placement::config_

- [x] 2.2 (P) descript ソース読込とゴーストタイトルを実装する
  - `areka_parsers::package::resolve` で shell dir を解決し、ghost/shell の `descript.txt` を `charset::decode`（既定 Ansi）→ `kv::parse_kv` で読み込む `load_descript_source` を実装する（`ghost_kv`／`shell_kv`／`shell_dir`／`GhostTitles` を返す）
  - `GhostTitles`（`sakura.name`/`kero.name`/`char{n}.name` 由来・欠落スコープは既定 `"areka"`）とアクセサ `title(scope)` を実装する
  - ghost descript 読取失敗は警告＋空 KV で継続し、shell descript 読取失敗・resolve 失敗は `Err` を返す
  - 統合テスト T-I5: emo2 fixture から `load_descript_source` を呼び、`shell_kv` に `seriko.alignmenttodesktop=bottom` 等の実測キーが含まれることを確認する
  - 観測可能な完了状態: `cargo test -p areka source::` で T-I5 が緑になり、emo2 fixture 相当の descript から実測キーが取得できることが確認できる
  - _Requirements: 1.1, 2.3_
  - _Boundary: placement::source_

- [ ] 3. Core: 純粋 resolver — 既定位置解決
- [x] 3.1 物理 px 値型と bottom 基準・スコープ連鎖・クランプの配置規則を実装する
  - `RectPx`／`PointPx`／`SizePx`／`ScopeInput`／`ScopePlacement` の物理 px 値型（wintf 非依存）を定義する
  - P1（bottom 時 `y = work_area.bottom − h`・`defaulttop` 無視）／P2（`base_x(0) = work_area.right − w(0)`、`base_x(n≥1) = char_x(n−1) − w(n−1)`、`char_x(n) = base_x(n) − defaultx(n).unwrap_or(0)`）／P4（キャラ窓のみ work area 内へクランプ）を実装する `resolve_placement` を実装する（`PlacementConfig` を読むが wintf 型は import しない）
  - `Alignment::Seam` は bottom と同一出力とする
  - DPI パラメタ化単体テスト（dpi ∈ {96,120,144,192}）T-R1（bottom 基準）・T-R2（スコープ連鎖・`kero.defaultx=0` が右端に戻らないこと）・T-R3（`defaulttop` 無視）・T-R5（`Alignment::Seam` が bottom と同一出力）・T-R6（クランプ）を追加する
  - 観測可能な完了状態: `cargo test -p areka resolver::` で T-R1・T-R2・T-R3・T-R5・T-R6 が dpi 全 4 水準で緑になる（隠れた `/96` 変換があれば 96 以外の水準で崩れることを確認する）
  - _Requirements: 1.5, 2.1, 2.2, 2.4, 2.5, 2.8, 2.9, 2.10, 3.2, 3.3, 3.4_
  - _Depends: 2.1_
  - _Boundary: placement::resolver_

- [x] 3.2 free 配置規則・バルーン暫定 offset・全モニタ和ヘルパを実装する
  - P3（free 時 `char_x = work_area.left + defaultleft`／`char_y = work_area.top + defaulttop`、未指定成分は bottom 相当へフォールバック）を実装する
  - P5（バルーン暫定 offset: `balloon.alignment=left` → `balloon_x = char_x − balloon_w`、`right` → `balloon_x = char_x + w`、`balloon_y = char_y`、`balloon.offsetx/offsety` があれば加算、クランプなし）を実装する
  - `virtual_desktop_union(monitor_bounds: &[RectPx]) -> Option<RectPx>`（全モニタ矩形の和・空入力は `None`）を実装する
  - DPI パラメタ化単体テスト T-R4（free 適用と未指定成分のフォールバック）・T-R8（バルーン offset の left/right と offsetx/y 加算・`balloon_offset ≡ balloon_pos − char_pos`）・T-R7（複数モニタ矩形の和・負座標を含む入力・空入力で `None`）を追加する
  - 観測可能な完了状態: `cargo test -p areka resolver::` で T-R4・T-R7・T-R8 が緑になる
  - _Requirements: 2.6, 4.4, 4.6_
  - _Boundary: placement::resolver_

- [ ] 4. Core: 採寸とドラッグ連動
- [x] 4.1 (P) surface／バルーンの原寸採寸を実装する
  - `measure_scope_sizes(shell_dir, balloon_root, scope_ids)` を実装し、各スコープの初期 surface（scope0=id0・scope1=id10・scope n≥2=id10 暫定＋warn）と balloon surface0 を areka-emo-atlas／areka-emo-compose で bind なし合成して原寸（物理 px）を得て `Vec<ScopeInput>` を返す
  - 合成失敗したスコープは scope0 の寸法で代替し `warn!` を出す（窓自体は生やす）
  - 採寸に使ったアセット（`EmoWorld`/`AtlasTable`）は採寸後に破棄する
  - 観測可能な完了状態: emo2 fixture 相当の shell/balloon ディレクトリに対し `measure_scope_sizes` が scope0・scope1・balloon の原寸（非ゼロの物理 px `SizePx`）を返すことをテストで確認する
  - _Requirements: 2.9, 3.2_
  - _Depends: 3.1_
  - _Boundary: placement::measure_

- [x] 4.2 (P) バルーン追従コンポーネントと窓移動の公開 API を実装する
  - `BalloonFollow { balloon: Entity, offset: PointPx }` コンポーネントを定義する
  - `on_char_drag`（キャラ窓の `WindowPos` 読取＋`BalloonFollow.offset` 加算で `SetWindowPosCommand::enqueue`。物理 px のみ・再スケールなし）を実装する
  - R7 公開 API `move_window_to(world: &mut World, window: Entity, x: i32, y: i32) -> bool`（`SetWindowPosCommand` 経由・物理 px 直渡し・`BalloonFollow` を持つ対象は随伴移動・`WindowHandle` 未付与時は `false` を返し `warn!`）を実装する
  - シグネチャに channel／actor 型を一切持たない（UI スレッド関数として `&mut World` のみで完結する）
  - 観測可能な完了状態: headless `World` 上で `move_window_to` を呼び対象窓の `WindowPos` が期待座標に更新されること、および未生成窓に対しては `false` を返すことをテストで確認する
  - _Requirements: 1.5, 3.2, 3.3, 4.4, 7.1, 7.2, 7.3_
  - _Depends: 3.1_
  - _Boundary: placement::follow_

- [ ] 5. Integration: 窓 entity 組立と後続への引き渡し
- [x] 5.1 キャラ窓・バルーン窓の entity 組立と公開データ構造を実装する
  - `CharWindowMarker{scope}`／`BalloonWindowMarker{scope}`／`GhostWindowMarker` コンポーネントと `GhostWindows` Resource（`char_window(scope)`／`balloon_window(scope)`／`scopes()`）を定義する
  - `spawn_ghost_windows(world, placements, titles)` を実装し、`ScopePlacement` 由来の位置・寸法のみを使って `WindowPos`／`WindowStyle`（`WS_EX_TOPMOST` を含めない）／`HitTest::none()`／`DragConfig::default()`／`OnDrag(on_char_drag)`（キャラ窓のみ）／`BalloonFollow`（キャラ窓のみ）／`OnPointerPressed`（ダブルクリックで全 `GhostWindowMarker` despawn）を持つ窓 entity を組み立てる。`BoxStyle` と `DragConstraint` は一切付けない
  - 座標・offset のリテラル定数（`(400,200)`／`(335,0)` 等）がこのモジュールに一切存在しないことを確認する
  - 統合テスト T-I1（bare `World` で 2 スコープぶん spawn し窓 4 entity・markers 正値・`GhostWindows` の scope×種別引き当てが `ScopePlacement` と一致）・T-I2（全窓の `WindowStyle.ex_style` に `WS_EX_TOPMOST` が含まれない）・T-I3（窓 entity に `BoxStyle` 不在・`DragConstraint` 不在・`DragConfig.move_window=true`）を追加する
  - 観測可能な完了状態: `cargo test -p areka spawn::` で T-I1・T-I2・T-I3 が緑になる
  - _Requirements: 1.1, 1.2, 1.5, 4.1, 4.5, 5.1, 6.1, 6.2, 6.3, 7.2_
  - _Depends: 2.1, 2.2, 3.1, 3.2, 4.2_
  - _Boundary: placement::spawn_

- [x] 5.2 placement 窓を αマスク clickthrough 機構へ登録する
  - `register_ghost_windows_click_through`（`Added<WindowHandle>` で `GhostWindowMarker` 窓を `ClickThroughRegistryHandle` へ登録。emo-present donor `register_click_through_windows` の一般化）を実装する
  - 観測可能な完了状態: headless `World` に `GhostWindowMarker` 窓を追加後、システム実行で clickthrough レジストリへの登録呼び出しが発生することをテストで確認する
  - _Requirements: 6.1_
  - _Depends: 5.1_
  - _Boundary: placement::spawn_

- [x] 5.3 バルーン追従の幾何と窓移動 API の統合を検証する
  - `spawn_ghost_windows` の出力する `BalloonFollow.offset` が `resolve_placement` の `balloon_offset` と一致することを確認する
  - `move_window_to` 呼び出し後、対象が `BalloonFollow` を持つ場合にバルーンが offset を保った状態で追従することを確認する
  - 統合テスト T-I4 を追加する
  - 観測可能な完了状態: `cargo test -p areka` で T-I4 が緑になり、spawn 済み entity に対する `move_window_to` の追従結果が期待座標と一致する
  - _Requirements: 4.2_
  - _Depends: 4.2, 5.1_
  - _Boundary: placement::spawn, placement::follow_

- [ ] 6. Integration: main.rs 起動窓シームの差し替え
- [x] 6.1 窓配置の準備処理（`prepare_ghost_windows`）を実装する
  - `prepare_ghost_windows(ghost_root, balloon_root)` を実装し、`load_descript_source` → `build_placement_config` → `measure_scope_sizes` → `enumerate_monitors()` の `is_primary` モニタ work area 取得 → `resolve_placement` の順に同期実行し、`PreparedPlacement { placements, titles }`（Send な結果のみ）を返す
  - 準備段階の失敗は `PlacementError` として呼び出し側（シーム）が捕捉できる形で返す（この関数自体はフォールバックしない）
  - 位置の記憶・復元（`ghost.dat` 読み書き）を一切行わないことを確認する
  - 観測可能な完了状態: emo2 fixture 相当のパスで `prepare_ghost_windows` を呼び、2 スコープぶんの `ScopePlacement` を含む `PreparedPlacement` が返ることをテストで確認する
  - _Requirements: 2.11, 2.12_
  - _Depends: 2.1, 2.2, 3.1, 3.2, 4.1_
  - _Boundary: main.rs seam_

- [x] 6.2 `open_startup_window` シームを本物のゴースト窓生成へ差し替える
  - `open_startup_window(app: &WinApp, cfg: &ConfigInputs)` へ署名変更し、`prepare_ghost_windows` 成功時は `spawn_ghost_windows`＋`register_ghost_windows_click_through` の schedule 結線を既存 ECS コマンド経路で実行する
  - 準備失敗時（fixture 不在等）は `MountError::StartPointMissing` 系を `warn!`、他は `error!` の上で既存 `spawn_dummy_window` へフォールバックする（`spawn_dummy_window`／`DummyWindowMarker` は退役せず残置）
  - `AREKA_APP_SMOKE_EXIT_MS` smoke 自動 close の despawn 対象を `Or<(With<DummyWindowMarker>, With<GhostWindowMarker>)>` へ拡張する
  - 観測可能な完了状態: fixture ありの環境で smoke 実行（`AREKA_APP_SMOKE_EXIT_MS` 設定）が本物のゴースト窓構成で完走し、fixture なしの環境では `warn!` ログとともにダミー窓へフォールバックして完走することを確認する
  - _Requirements: 1.4_
  - _Depends: 5.1, 5.2, 6.1_
  - _Boundary: main.rs seam_

- [ ] 7. Validation: 実 DPI 受け入れ example と手動検証
- [x] 7.1 実 DPI 受け入れ example を実装する
  - `crates/areka/examples/window-placement.rs` を新設し、`prepare_ghost_windows`（emo2 fixture パス）→ `spawn_ghost_windows` → emo-present donor と同型の装着経路（`EmoPresenter::attach_target`、dev-dependency の areka-emo-present を使用）で scope0 キャラ窓に surface0・scope1 キャラ窓に surface10・両バルーン窓に balloon target を装着する（`crates/areka/src/placement/` 本体は `EmoPresenter` を import しない）
  - rustdoc に手動観測プロトコル（①per-monitor v2・dpi≠96 で実行 ②scope0 が work area 右下・scope1 がその左に画面内出現 ③キャラ窓ドラッグでバルーン追従 ④モニタ境界を跨ぐドラッグで消失しない ⑤結果と実 DPI 値を記録）と、scope1 バルーンが scope0 キャラ窓に重畳するのは暫定規則の正常挙動であり受け入れ判定の対象外である旨を明記する
  - 観測可能な完了状態: `cargo build --example window-placement -p areka` が成功し、rustdoc に手動観測プロトコル①〜⑤と重畳注記が記載されている
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 4.5, 6.3_
  - _Depends: 6.2_
  - _Boundary: examples/window-placement_

- [ ] 7.2 実 DPI（≠96）での手動受け入れ検証を実施する
  - per-monitor v2・dpi≠96（例 125%）環境で `examples/window-placement.rs` を実行し、rustdoc プロトコル①〜⑤（既定位置出現・全面ドラッグ・バルーン追従・モニタ境界跨ぎドラッグでの非消失）を確認する
  - マルチモニタ環境がある場合はモニタ境界跨ぎドラッグ（4.5/4.6）を必ず含める
  - 観測結果（実 DPI 値・各プロトコル項目の pass/fail）を記録する。dpi=96 のみの確認は不合格として扱う
  - 観測可能な完了状態: 実 DPI（≠96）値と①〜⑤各項目の pass/fail が記録され、全項目 pass であることが確認できる
  - _Requirements: 3.1, 3.5, 4.3_
  - _Depends: 7.1_
  - _Boundary: examples/window-placement_

## Implementation Notes

- 1: `areka_parsers::package::MountError` は Display/std::error::Error 未実装（Clone/Debug/PartialEq/Eq のみ・#[non_exhaustive]）。`PlacementError::Mount` は `{0:?}` Debug 整形・`#[from]` 不可 → 後続タスクは `PlacementError::Mount(e)` を明示構築すること（areka-parsers は改変禁止）。
- 2.1: `Alignment::Seam` の `warn!` 発火は config でなく resolver（task 3.1）へ委譲済み（config.rs:22 に明記）→ 3.1 で解消（resolver.rs:124-130 で発火・T-R5 経路で実行）。スコープ検出は DD6 厳密読み（`char1.*` 単独では scope1 を作らない・kero シグナルのみ）。同層クロス競合は正典プレフィックス外側優先。
- 4.1: emo2 fixture 実測原寸 = scope0 surface0: 434×687 / scope1 surface10: 336×400 / balloon surface0: 400×224（テストで檻化済み・example/受入検証の期待値に使える）。balloon エラーは `Measure { scope: 0, reason: "balloon: ..." }` 規約。
- 6.2: smoke テスト `run_smoke` は親の RUST_LOG を継承（`RUST_LOG=error` 環境で info マーカー assert が偽陽性 fail の可能性→ `.env("RUST_LOG","info")` ピン候補）。resolver.rs:18 等に文言陳腐化した allow(dead_code) コメント残（棚卸し候補）。
- 5.3: 5.2 由来の `t_i4_register_*` テスト名は design の T-I4（follow 幾何）と衝突（実害なし・一意名で全実行）→ 余裕があれば `t_reg_*` 等へ改名候補。
- 5.2: clickthrough 登録 system の schedule 結線は 6.2（donor の slot は `FrameFinalize`）。レジストリ NonSend 未挿入 tick の Added 消費に注意（donor 同挙動・WinApp::run が先に handle 挿入する結線順で緩和）。
- 4.2: `enqueue_window_move` は SetWindowPosCommand enqueue＋`bypass_change_detection()` で WindowPos ミラー（wintf echo 規約と一致・apply_window_pos_changes 二重発行防止・headless 観測シーム）。`on_char_drag` は pub(crate)（5.1 の OnDrag 結線用）。WindowPos はクライアント座標だが WS_POPUP 枠なし窓では窓座標と同一。headless テストは偽 WindowHandle（wintf tests の確立パターン）。
- 3.1: P2 連鎖は post-clamp 前スコープ実位置基準（design 未規定領域の確定・t_r6_chain_uses_clamped_previous_position で檻化）。balloon_pos=char_pos/offset=0 は 3.2 P5 までの正直な暫定。極値 `defaultx=i32::MIN` で debug オーバーフローの理論穴 → 3.2 で saturating 演算検討（非ブロッキング）。`cargo clippy` は wintf 既存エラーで全体 fail（本 spec 非起因・不改変境界）。
