# Implementation Plan

- [ ] 1. Foundation: main同期ゲートとスケール数学基盤
- [x] 1.1 position-persistマージ確認とmain同期ゲート
  - 開発者へ`areka-P0-position-persist`のmainマージ完了を確認依頼し、本worktreeブランチへorigin/mainを同期する
  - 同期後、placement系実測アンカー（measure.rs:62、follow.rs:553/:729、source.rs:102）をdiffで再確認し、design.mdのBoundary Deviation Notesとの整合を確認する
  - git logに同期コミットが記録され、アンカー行番号のずれがあれば設計への追記を済ませた状態を観測可能とする
  - _Requirements: 7.6, 7.7_

- [x] 1.2 ScaleRatio・scaled_extent実装（emo-compose scale.rs）
  - 既約有理数構造体（num/den・gcd既約化・Eq/Hash厳密）を新設
  - mul（乗算合成）・is_identity・as_f32・scale_len/scaled_extent（round half away from zero・非ゼロ最小1px）を実装
  - DPI対照表（96/120/144/168/192）×代表原寸のunit testが全て決定論的に一致する状態を確認できる
  - _Requirements: 1.1, 1.3, 1.6, 2.2, 2.5_

- [ ] 1.3 resample整数bilinearリサンプラ実装（emo-compose scale.rs）
  - premultiplied BGRAドメインの完全整数bilinear補間、k=1/1は恒等バイトコピー、エッジクランプ固定
  - 恒等コピー・2倍整数k・非整数k（5/4）のgoldenテストがバイト決定論で再現する状態を確認できる
  - _Requirements: 2.1, 2.5, 7.2_

- [ ] 1.4 ScalePolicy・derive_scale実装（emo-present scale.rs）
  - author_dpi・app_scale（ONE固定）を保持するPolicy構造体を新設
  - derive_scale純関数（DPI不在縮退・dpi_x≠dpi_y警告・author_dpi=0正規化）を実装
  - 正常/DPI不在/dpi_x≠dpi_y/author_dpi=0の全分岐がunit testで判定される状態を確認できる
  - _Requirements: 1.1, 1.4, 1.6_
  - _Depends: 1.2_

- [ ] 2. Core: 採寸源のk対応（placement系）
- [ ] 2.1 (P) author_dpi読取実装（source.rs）
  - shell `seriko.dpi`・balloon `dpi`を既存の生KVから読み取るaccessorを追加
  - 無宣言=96・不正/0=warn+96の縮退を実装
  - 無宣言/宣言あり/不正/0の全パターンがunit testで緑になる状態を確認できる
  - _Requirements: 1.1_
  - _Boundary: placement/source.rs_

- [ ] 2.2 (P) balloon窓位置維持リサイズラッパ実装（follow.rs）
  - 私有単一ライター経路`enqueue_window_set_pos`への薄い公開ラッパ`resize_window_keep_position`を、position-persistのDragEnd観測域（:319-350/:443-488）から離れた位置（ファイル末尾）に追加
  - 同寸呼び出し時は書込ゼロ（べき等skip）とする
  - 同寸呼び出しで書込ゼロとなることがunit testで確認される状態を観測できる
  - _Requirements: 3.1, 4.2_
  - _Boundary: placement/follow.rs_

- [ ] 2.3 (P) measure_scope_sizesのk適用実装（measure.rs）
  - native採寸（既存per-scopeループ・`ScopeInput`構造を温存）→k適用の2段へ関数分解する
  - per-scope balloon席（balloon_sizeがscope別値になり得る席）を潰さない写像にする
  - i32超過を既存Measureエラー流儀でガードする
  - `MeasureScaling{ONE,ONE}`指定時に既存emo2期待値（434×687等）が不変であることをunit testで確認できる状態にする
  - _Requirements: 2.5, 3.1, 3.3, 3.4, 7.6, 7.8_
  - _Boundary: placement/measure.rs_
  - _Depends: 1.2_

- [ ] 3. Core: EmoPresenterのk適用単一漏斗
- [ ] 3.1 (P) ComposeCacheキー拡張実装（cache.rs）
  - `ComposeKey`へ`scale: ScaleRatio`を参加させ、get/insertシグネチャを拡張する
  - エントリ構造・挿入時マスク生成コードは変更しない
  - 同一合成入力でscaleが異なる場合に必ずキャッシュミスすることがunit testで確認できる状態にする
  - _Requirements: 2.4, 4.1_
  - _Boundary: emo-present/cache.rs_
  - _Depends: 1.2_

- [ ] 3.2 (P) attach_target拡張とPresentTargetフィールド追加（presenter.rs）
  - `PresentTarget`へ`policy`/`applied`/`native_size`/`last_show`フィールドを追加する
  - `attach_target(.., author_dpi)`へシグネチャを拡張する
  - `TextSlotView::scale()`（実適用k）・`surface_size()`（native原寸）の契約を更新する
  - attach_target呼び出し後、target毎にpolicyが窓単位で保持されることを確認できる状態にする
  - _Requirements: 1.2, 1.5_
  - _Boundary: emo-present/presenter.rs_
  - _Depends: 1.4_

- [ ] 3.3 apply_showのk導出・キャッシュ統合・リサンプル実装（presenter.rs）
  - `world.get::<DPI>(target.window)` → `derive_scale` → `cache.get`（scaleキー）→ ミス時 `compose`（native）→ `resample` → `cache.insert` の流れを実装する
  - k=2/1のShowSurfaceでcacheミス時にread_back寸法がscaled_extentと一致する状態を確認できる
  - _Requirements: 1.1, 2.1, 2.3, 2.4_
  - _Depends: 3.1, 3.2, 1.3_

- [ ] 3.4 apply_showの成立点記録・状態照合・観測ログ実装（presenter.rs）
  - 表示成立点で`applied`/`native_size`/`last_show`/`current_surface_id`を記録し、失敗経路は手前でearly returnして前値を維持する
  - 今回scaled寸が前回適用寸と異なる場合に新物理寸を呼び手へ報告する状態照合を実装する（設計ディスカッション#2裁定）
  - info log（target/k_num/k_den/k/author_dpi/window_dpi/native/scaled）を出力する
  - 寸法変化時に呼び手へ新物理寸が報告され、ログにk値等が出力される状態を確認できる
  - _Requirements: 1.2, 2.3, 3.1, 3.2, 4.4, 6.1_
  - _Depends: 3.3_

- [ ] 3.5 applied_scale・refresh_scale実装（presenter.rs）
  - `applied_scale`照会（表示成立前はNone）を実装する
  - `refresh_scale`（窓DPIから再導出・`last_show`保持時のみ再表示・差分なければNone・失敗はerror!+前表示維持）を実装する
  - DPI差替後にrefresh_scaleを呼ぶとapplied_scaleが新k相当を返す状態を確認できる
  - _Requirements: 1.2, 4.1, 4.3, 4.4_
  - _Depends: 3.4_

- [ ] 4. Integration: DPI変化の動的追従とboot結線
- [ ] 4.1 BootAssetsへauthor_dpi搬送（assets.rs）
  - `BootAssets`へ`shell_author_dpi`/`balloon_author_dpi`を追加する
  - BootAssetsからauthor_dpiが取得できる状態を確認できる
  - _Requirements: 1.1_
  - _Depends: 2.1_

- [ ] 4.2 (P) run_dpi_phase実装とemo2_frame_systemへの組込（frame.rs）
  - `Changed<DPI>`を`Local<SystemState>`で観測し（`anchor_changed_system`先例踏襲）、対象targetの`refresh_scale`を呼ぶ
  - `refresh_scale`が新物理寸を返した場合、char窓は`resize_window_to`、balloon窓は`resize_window_keep_position`で窓寸をreconcileする
  - apply_showの状態照合報告（3.4）を受けて同一フレーム内でreconcileする第2経路も実装する（エッジ消費順序に依存しない）
  - `attach_target`呼び2箇所（shell/balloon）へauthor_dpiを供給する
  - DPI差替後、同一フレーム内で窓clientがscaled_extent(applied, native)と一致する状態を確認できる
  - _Requirements: 3.1, 4.1, 4.2, 4.3_
  - _Boundary: emo2_boot/frame.rs_
  - _Depends: 3.4, 3.5, 2.2, 4.1_

- [ ] 4.3 (P) main.rs boot結線（k₀導出・MeasureScaling構築）
  - author_dpi読取＋`enumerate_monitors()`でprimaryモニタDPIを取得（不能時96相当+errorログ）し`MeasureScaling`を構築する
  - `measure_scope_sizes`へ構築した`MeasureScaling`を供給する
  - 起動時にk₀倍後の物理窓寸で窓生成が行われる状態を非96環境のログで確認できる
  - _Requirements: 1.4, 3.3_
  - _Boundary: areka/main.rs_
  - _Depends: 2.1, 2.3, 4.1_

- [ ] 5. Integration: 既存呼び手のシグネチャ追随
  - `attach_target`/`measure_scope_sizes`の既存呼び出し箇所（`examples/emo-present.rs`ほか）をシグネチャ変更に追随させる（k=1相当値で挙動不変）
  - 既存呼び出し箇所がコンパイル通過し、挙動が変わらないことを確認できる状態にする
  - _Requirements: 7.1, 7.2_
  - _Depends: 3.2, 2.3_

- [ ] 6. Validation: 決定論テストと実機サインオフ
- [ ] 6.1 (P) emo-compose scale.rs純関数テスト全網羅
  - 既約正準化・mul合成・is_identity・as_f32厳密値、DPI対照表境界（half丁度）・min1px・非溢れのテストを実装する
  - `cargo test -p areka-emo-compose`が新規テストを含めて全緑になる状態を確認できる
  - _Requirements: 1.3, 2.2, 2.5, 5.2_
  - _Boundary: areka-emo-compose tests_
  - _Depends: 1.2, 1.3_

- [ ] 6.2 (P) emo-present/measure/source純関数テスト全網羅
  - derive_scale全分岐、parse_author_dpi全パターン、measure k適用のper-scope写像・i32ガードのテストを実装する
  - 該当crateの`cargo test`が新規テストを含めて全緑になる状態を確認できる
  - _Requirements: 1.4, 5.2, 7.8_
  - _Boundary: areka-emo-present/scale, placement_
  - _Depends: 1.4, 2.1, 2.3_

- [ ] 6.3 emo-present in-crate GPU readback決定論テスト
  - k=2/1・k=5/4のShowSurface→read_back寸法・goldenバイト一致、DPI差替→refresh_scale→ResizeBuffers自動追従、マスクk寸検証のテストを実装する
  - k=1/1既存テスト群の期待値不変を確認する（回帰の錨）。テストWorldへDPI componentを明示挿入する規律を適用する
  - 別プロセス配置（emo-present in-crateテストバイナリ）であり、wintf tests/graphicsへの新設が無いことを確認する
  - `cargo test -p areka-emo-present`（GPU readback含む）が全緑となり、2個目Compositor AVが再現しない状態を確認できる
  - _Requirements: 2.1, 2.4, 4.1, 4.2, 5.1, 5.3, 5.4, 7.2, 7.5, 7.9_
  - _Depends: 3.5, 6.1_

- [ ] 6.4 workspace回帰ゲート実行
  - i686 host-32成果物の事前ビルドを含む`cargo test --workspace`を実行する
  - `cargo test --workspace`がexit 0となり、既存テスト数からの純増のみで失敗ゼロである状態を確認できる
  - _Requirements: 5.5, 6.1, 7.1, 7.3, 7.4_
  - _Depends: 6.1, 6.2, 6.3, 5_

- [ ] 6.5 実DPI2水準の実機サインオフ
  - OS表示スケール125%→200%の2回起動×本番ゴーストemo2（実pasta.dll・絶対パス起動）を実行する
  - `AREKA_APP_SMOKE_EXIT_MS`有界自動終了＋`RUST_LOG` grep（k・scaled寸・GetClientRect照合）で決定論的に判定する
  - マスコット拡大表示・窓追従・モニタ跨ぎ移動追従を人間目視で確認する
  - 2水準それぞれでログのk値（1.25/2.0）とGetClientRectが一致し、目視で拡大・追従を確認した記録が残る状態を観測できる
  - _Requirements: 2.2, 6.1, 6.2, 6.3, 6.4_
  - _Depends: 6.4_

## Implementation Notes

- 1.1: main同期は追加コミット不要（ブランチ == origin/main）。placement アンカーは `follow.rs` のみ行番号がずれた（`resize_window_to` :553→:786・`enqueue_window_set_pos` :729→:1009）。design.md へ差分表を記録済み。
- 1.2: `scale_len` の中間型は design の u64 では `len≈num≈u32::MAX` で溢れるため **u128** が正。design.md を是正済み。
- 1.2: `areka-emo-compose` の縮退経路は `tracing::warn!` 必須（steering `logging.md`）。ログ発火は `crate::log_capture::capture_logs` で檻に入れる（先例 `log_firing_tests.rs`・`plan.rs`）。純関数モジュールでも「無言縮退」はレビューで落ちる。
- 1.2: `plan.rs:334` に **既存の** clippy `collapsible_if` 警告あり（本 spec の境界外・触らない）。crate 全体に既存の `cargo fmt` 差分（import 順）もあるため、fmt は変更行のみで判定する。
