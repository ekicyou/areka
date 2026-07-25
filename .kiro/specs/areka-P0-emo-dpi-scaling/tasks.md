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

- [x] 1.3 resample整数bilinearリサンプラ実装（emo-compose scale.rs）
  - premultiplied BGRAドメインの完全整数bilinear補間、k=1/1は恒等バイトコピー、エッジクランプ固定
  - 恒等コピー・2倍整数k・非整数k（5/4）のgoldenテストがバイト決定論で再現する状態を確認できる
  - _Requirements: 2.1, 2.5, 7.2_

- [x] 1.4 ScalePolicy・derive_scale実装（emo-present scale.rs）
  - author_dpi・app_scale（ONE固定）を保持するPolicy構造体を新設
  - derive_scale純関数（DPI不在縮退・dpi_x≠dpi_y警告・author_dpi=0正規化）を実装
  - 正常/DPI不在/dpi_x≠dpi_y/author_dpi=0の全分岐がunit testで判定される状態を確認できる
  - _Requirements: 1.1, 1.4, 1.6_
  - _Depends: 1.2_

- [ ] 2. Core: 採寸源のk対応（placement系）
- [x] 2.1 (P) author_dpi読取実装（source.rs）
  - shell `seriko.dpi`・balloon `dpi`を既存の生KVから読み取るaccessorを追加
  - 無宣言=96・不正/0=warn+96の縮退を実装
  - 無宣言/宣言あり/不正/0の全パターンがunit testで緑になる状態を確認できる
  - _Requirements: 1.1_
  - _Boundary: placement/source.rs_

- [x] 2.2 (P) balloon窓位置維持リサイズラッパ実装（follow.rs）
  - 私有単一ライター経路`enqueue_window_set_pos`への薄い公開ラッパ`resize_window_keep_position`を、position-persistのDragEnd観測域（:319-350/:443-488）から離れた位置（ファイル末尾）に追加
  - 同寸呼び出し時は書込ゼロ（べき等skip）とする
  - 同寸呼び出しで書込ゼロとなることがunit testで確認される状態を観測できる
  - _Requirements: 3.1, 4.2_
  - _Boundary: placement/follow.rs_

- [x] 2.3 (P) measure_scope_sizesのk適用実装（measure.rs）
  - native採寸（既存per-scopeループ・`ScopeInput`構造を温存）→k適用の2段へ関数分解する
  - per-scope balloon席（balloon_sizeがscope別値になり得る席）を潰さない写像にする
  - i32超過を既存Measureエラー流儀でガードする
  - `MeasureScaling{ONE,ONE}`指定時に既存emo2期待値（434×687等）が不変であることをunit testで確認できる状態にする
  - _Requirements: 2.5, 3.1, 3.3, 3.4, 7.6, 7.8_
  - _Boundary: placement/measure.rs_
  - _Depends: 1.2_

- [ ] 3. Core: EmoPresenterのk適用単一漏斗
- [x] 3.1 (P) ComposeCacheキー拡張実装（cache.rs）
  - `ComposeKey`へ`scale: ScaleRatio`を参加させ、get/insertシグネチャを拡張する
  - エントリ構造・挿入時マスク生成コードは変更しない
  - 同一合成入力でscaleが異なる場合に必ずキャッシュミスすることがunit testで確認できる状態にする
  - _Requirements: 2.4, 4.1_
  - _Boundary: emo-present/cache.rs_
  - _Depends: 1.2_

- [x] 3.2 (P) attach_target拡張とPresentTargetフィールド追加（presenter.rs）
  - `PresentTarget`へ`policy`/`applied`/`native_size`/`last_show`フィールドを追加する
  - `attach_target(.., author_dpi)`へシグネチャを拡張する
  - `TextSlotView::scale()`（実適用k）・`surface_size()`（native原寸）の契約を更新する
  - attach_target呼び出し後、target毎にpolicyが窓単位で保持されることを確認できる状態にする
  - _Requirements: 1.2, 1.5_
  - _Boundary: emo-present/presenter.rs_
  - _Depends: 1.4_

- [x] 3.3 apply_showのk導出・キャッシュ統合・リサンプル実装（presenter.rs）
  - `world.get::<DPI>(target.window)` → `derive_scale` → `cache.get`（scaleキー）→ ミス時 `compose`（native）→ `resample` → `cache.insert` の流れを実装する
  - k=2/1のShowSurfaceでcacheミス時にread_back寸法がscaled_extentと一致する状態を確認できる
  - _Requirements: 1.1, 2.1, 2.3, 2.4_
  - _Depends: 3.1, 3.2, 1.3_

- [x] 3.4 apply_showの成立点記録・状態照合・観測ログ実装（presenter.rs）
  - 表示成立点で`applied`/`native_size`/`last_show`/`current_surface_id`を記録し、失敗経路は手前でearly returnして前値を維持する
  - 今回scaled寸が前回適用寸と異なる場合に新物理寸を呼び手へ報告する状態照合を実装する（設計ディスカッション#2裁定）
  - info log（target/k_num/k_den/k/author_dpi/window_dpi/native/scaled）を出力する
  - 寸法変化時に呼び手へ新物理寸が報告され、ログにk値等が出力される状態を確認できる
  - _Requirements: 1.2, 2.3, 3.1, 3.2, 4.4, 6.1_
  - _Depends: 3.3_

- [x] 3.5 applied_scale・refresh_scale実装（presenter.rs）
  - `applied_scale`照会（表示成立前はNone）を実装する
  - `refresh_scale`（窓DPIから再導出・`last_show`保持時のみ再表示・差分なければNone・失敗はerror!+前表示維持）を実装する
  - DPI差替後にrefresh_scaleを呼ぶとapplied_scaleが新k相当を返す状態を確認できる
  - _Requirements: 1.2, 4.1, 4.3, 4.4_
  - _Depends: 3.4_

- [ ] 4. Integration: DPI変化の動的追従とboot結線
- [x] 4.1 BootAssetsへauthor_dpi搬送（assets.rs）
  - `BootAssets`へ`shell_author_dpi`/`balloon_author_dpi`を追加する
  - BootAssetsからauthor_dpiが取得できる状態を確認できる
  - _Requirements: 1.1_
  - _Depends: 2.1_

- [x] 4.2 (P) run_dpi_phase実装とemo2_frame_systemへの組込（frame.rs）
  - `Changed<DPI>`を`Local<SystemState>`で観測し（`anchor_changed_system`先例踏襲）、対象targetの`refresh_scale`を呼ぶ
  - `refresh_scale`が新物理寸を返した場合、char窓は`resize_window_to`、balloon窓は`resize_window_keep_position`で窓寸をreconcileする
  - apply_showの状態照合報告（3.4）を受けて同一フレーム内でreconcileする第2経路も実装する（エッジ消費順序に依存しない）
  - `attach_target`呼び2箇所（shell/balloon）へauthor_dpiを供給する
  - DPI差替後、同一フレーム内で窓clientがscaled_extent(applied, native)と一致する状態を確認できる
  - _Requirements: 3.1, 4.1, 4.2, 4.3_
  - _Boundary: emo2_boot/frame.rs_
  - _Depends: 3.4, 3.5, 2.2, 4.1_

- [x] 4.3 (P) main.rs boot結線（k₀導出・MeasureScaling構築）
  - author_dpi読取＋`enumerate_monitors()`でprimaryモニタDPIを取得（不能時96相当+errorログ）し`MeasureScaling`を構築する
  - `measure_scope_sizes`へ構築した`MeasureScaling`を供給する
  - 起動時にk₀倍後の物理窓寸で窓生成が行われる状態を非96環境のログで確認できる
  - _Requirements: 1.4, 3.3_
  - _Boundary: areka/main.rs_
  - _Depends: 2.1, 2.3, 4.1_

- [x] 5. Integration: 既存呼び手のシグネチャ追随
  - `attach_target`/`measure_scope_sizes`の既存呼び出し箇所（`examples/emo-present.rs`ほか）をシグネチャ変更に追随させる（k=1相当値で挙動不変）
  - 既存呼び出し箇所がコンパイル通過し、挙動が変わらないことを確認できる状態にする
  - _Requirements: 7.1, 7.2_
  - _Depends: 3.2, 2.3_

- [ ] 5.1 examples/emo-present.rs の startup golden を k 対応させる（task 5 レビューで検出）
  - `assert_startup_golden`（:705-752）は `read_back` を **native 原寸**の直接合成とバイト比較するため、k≠1.0 の実機（例: 本開発機 125%）では**手動検証エイドが動かせない**
  - golden を実適用 k で resample してから比較する（design「Testing Strategy > Integration Tests」が檻に課すのと同じ契約を手動エイドにも適用）
  - **task 6.5 より前に着地させる**（6.5 は本番バイナリのみ実行するため、この破れは 6.5 では検出も解消もされない）
  - 非96DPI環境で `cargo run -p areka --example emo-present` が完走する状態を確認できる
  - _Requirements: 7.1, 7.2_
  - _Depends: 5_

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
- 5: **W5 `areka-P0-collision-dpi-hittest` への申し送り（完了時に roadmap へ転記すること）**: `crates/areka/examples/collision-probe.rs` の ③ アサート 2 本が**陳腐化**した。(a) `:546-553` が `GetClientRect` を `v.surface_size()`（＝native 原寸へ変更済み）と比較、(b) `:557-560` が `TextSlotView::scale() == 1.0` を hard assert。いずれも本 spec が意図的に廃した **k=1.0 ハードワイヤそのものを検査**しており、R7.9 で本 spec の領分外。当該 spec が collision-probe を書き換える際に更新すること。※(a) の方が先に発火する（実装者の申告は (b) のみで不完全だった）。
- 5: examples は `cargo test` では**コンパイルのみ**で実行されない（実行するのは `CARGO_BIN_EXE_areka` を起動する `tests/emo2_real_run.rs`・`tests/smoke_boot_loop_exit.rs` で、これらは**本番バイナリ**）。両テストが本機 120DPI で実ゴースト窓ごと緑＝**本番の k 経路は非96DPI で健全**という積極的証拠。
- 5: **テスト World への `DPI` 明示挿入は design の要求**（design.md:593）。`spawn_empty()` のままだと `derive_scale` が縮退分岐（error!＋k=1.0）を通り、「正常系のふり」で緑になる。`DPI::from_dpi(96,96)` 挿入後も期待値が 1 つも動かなかったことが、この変更が非挙動的である証拠。
- 4.3: **実機 1 水準の先行観測が取れている**（本機 primary DPI=120＝125%）。有界起動＋絶対パスで `primary_dpi=120 shell_author_dpi=96 k_shell=1.25 k_shell_ratio=ScaleRatio{num:5,den:4}` → `char 543×859 / 420×500, balloon 500×280` を grep 確認し、実ゴースト窓がその寸で開いた。**434×1.25=542.5→543** が本番経路の round-half-away-from-zero の生証拠。**task 6.5 は 200% 水準の追加と目視サインオフが主眼**。
- 4.3: k₀ の居所は `placement::prepare_stages`（tasks.md は main.rs と記すが、descript 読取と `measure_scope_sizes` 呼びが両方そこに在る）。`main.rs` は結果を `wire_emo2_boot` へ中継するのみ。**親が境界を placement/mod.rs＋emo2_boot/mod.rs＋main.rs へ拡張して裁可**。
- 4.3: **1度の読取が2消費者へ**（Flow3 手順1）。`AuthorDpi{shell,balloon}` 名前付き型で束ね、`prepare_stages → PreparedPlacement.author_dpi → main → wire_emo2_boot → build_boot_assets_for → attach_target`。`emo2_boot` は production で descript を一切読まない（読取の二重化＝2経路が食い違う欠陥を構造で防ぐ）。
- 4.3: `primary_work_area` を `primary_monitor` + `work_area_of` へ分解。**DPI と work area を同一の Monitor 選択から配る**ため（2度選ぶと警告二重化＋別モニタ指しの危険）。
- 4.3: boot 縮退は 96 を**DPI 分子**として代入する（author=192 なら k₀=1/2）。D7「取得不能は 96 相当」の読みであり、k=1.0 平坦返しではない。author=96（emo2 含む正典既定）では結果的に native 採寸。R1.4 の「k=1.0」は `derive_scale`（Flow1）の話。
- 4.2: **f32 で丸めを再実装してはならない（D4 違反・実証済み1px乖離）**。`ScaleRatio::as_f32` の doc が明示するとおり寸法演算に f32 を使うと、`num/den` が f32 で非厳密なとき厳密 .5 の積が .4999… へ落ちて切り下がる。実測反例＝author_dpi=96・窓DPI=112（k=7/6）・native=27 → 権威 **32** / f32 **31**。author_dpi は任意宣言値を通し、Windows のカスタム倍率は 1% 刻みゆえ分母が 2 冪でない k は普通に発生する。結果は**恒久的な1px見切れ**（同寸判定で固着）。
- 4.2: **境界拡張を親が裁可**＝`TextSlotView::physical_size()` と `EmoPresenter::target_physical_size()` を `presenter.rs` へ additive 追加（いずれも `applied.scaled_extent(..)` 直行）。frame.rs 側で `ScaleRatio` を組み直す案は **k 導出権威（D2）の二重化**になるため却下。design の State Management が既に「物理寸 = scaled_extent(k, surface_size)」を契約として明記しており、新設ではなく既存契約を呼べるようにしたもの。
- 4.2: **`resnap_shell_targets` は task 3.2 の契約変更で壊れていた**（`surface_size()` が native を返すようになったため k≠1 で窓を原寸へ引き戻す）。design「Revalidation Triggers」が明記する下流トリガゆえ修理はスコープ内。消費点は `text_slot_view()` を経由せず `target_physical_size()` を直接呼ぶ形にし、**native/物理の二択そのものを消した**（`text_slot_view` へ差し替えると E0308 でコンパイル不能＝型で構造的に閉じた）。
- 4.2: **`/kiro-validate-impl` へ持ち上げる残課題**: `frame.rs` の resnap 呼び出しで `shell_target` → `balloon_target` の1トークン変異が**生存**する（492緑のまま）。Req4.5/DD-5「shell 限定駆動」の性質で**本 spec 以前から無檻**・resnap 資産の所有 spec 側の課題。frame.rs で檻に入れるには GPU が要り D9/R5.3 に抵触するため現配置規律では原理的に不可。消えさせないこと。
- 4.2: **偽装交差検査の教訓**: 権威との同値テストは、標本が「必ず一致する入力（既約分母が2冪＝f32厳密）」だけだと檻にならない。乖離クラスを構造的に含む標本を選ぶこと。
- 4.1: **タスク計画の穴（親が裁定）**: `BootAssets` への pub フィールド追加は crate 内の全 exhaustive destructure / struct literal を壊す。tasks.md が所有者を定めていたのは `frame.rs`（4.2）だけで、**`mod.rs`・`spine.rs`・`input_events/balloon.rs` は無所有**だった。親判断で 4.1 に機械的追随として同梱し、暫定 `96` の 4 箇所すべてに `// scaffold（task N）:` で所有タスクを明記した（**4.2 が frame.rs の2箇所、4.3 が mod.rs:254 を実値へ差し替える**）。`spine.rs:499` の 96 は stand-in ではなく emo2 fixture の実測真値。
- 4.1: `BootAssets` の author_dpi は**呼び手供給**（`build_boot_assets` の末尾 2 引数）。`assets.rs` からの内部導出は不可＝`DescriptSource` が crate 外から構築不能（`GhostTitles.titles` が private・コンストラクタは `#[cfg(test)]`）かつ `parse_author_dpi` も private。加えて Flow 3 手順1 が「main が 1 度読んだ値を採寸 k₀ と attach の**双方**へ配る」ことを要求するため、内部導出は両者を別読取に載せてしまう。
- 4.1: **4.2/4.3 レビューの必須項目**: `build_boot_assets(.., shell_author_dpi, balloon_author_dpi)` は `u16, u16` の隣接位置引数で**取り違えても型で落ちない**。関数内部の取り違えは 192/144 の相異値テストが檻に入れているが、**呼出側は未防護**。引数順の目視照合を必ず行うこと。
- 3.5: **task 4.2 の最重要契約**＝`pending_resize` の消費責任は「表示成立を引き起こした者が消費する」。①`refresh_scale` が再表示に成功→自ら take して返す（drain 側は `None`＝二重 resize なし）②ゲート不成立→触らない（初回 k₀ 補正などの未消費要求を drain が拾う＝取りこぼしなし）③再表示失敗→触らない。**4.2 は `run_dpi_phase` の `refresh_scale` 戻り値と drain フェーズの `take_pending_resize` の両方を処理すること**（フェーズ順序に依存しないことはレビューで確認済み）。実装 doc の `# take_pending_resize との関係` 節が正本。
- 3.5: `refresh_scale` の成否判定は再表示後の `applied == Some(scale)` 照合（`apply_show` が `()` を返し reply も無いため）。同時にゲート導出と `apply_show` の権威導出の一致検査も兼ねる。`EmptyComposition`→Hide 縮退は `applied` を書かないので「不成立」に正しく分類される。
- 3.4: **状態照合の報告機構＝`PresentTarget.pending_resize` ＋ `take_pending_resize()`**（drain 可能な target 単位状態）。`PresentOutcome` の拡幅は**不可**——本番 drain 経路 `emo2_boot/frame.rs:539-551` は `reply: None` の撃ちっぱなしで報告が届かない（実コードで確認済み）。**task 4.2 はこの accessor から取り出すこと**。
- 3.4: 報告は**初回表示でも必ず出る**（前値 `None` ≠ `Some(size)`）。Flow 3 手順5＝起動時 k₀ 見積もり窓寸と実窓 DPI 由来 k の差分補正がこれに依存するため、初回を黙らせてはならない。
- 3.4: 未 drain の `pending_resize` を持ち越しても安全（値は常に「最後に成立した表示の物理寸」＝窓があるべき寸法。resize 側もべき等 skip）。`apply_hide` は触らない（Hide は窓寸を変えない・消すと正当な未処理要求を失う）。
- 3.4: **design 内部矛盾を是正**: D10 は `k_num`/`k_den` を個別ログフィールドに求めるが、`ScaleRatio` の num/den は design 自身が非公開と規定。実装は `k_ratio`(Debug)＋`k`(f32) で出力し design.md を是正済み。**6.5 の RUST_LOG grep は `k=` を使うこと**。
- 3.4: `areka-emo-present` でも**ログ発火を檻に入れられる**——`tracing` は既に通常依存ゆえ、テストモジュール内に最小 `Subscriber` を手書きし `tracing::subscriber::with_default`（スレッドローカル・並列GPUテストと混線しない）で捕捉すればよい。`tracing-subscriber` の dev 依存追加は不要（**1.4 の申し送りを上書き**）。
- 3.2+3.3: **親判断で一括実装**（分割すると `ScaleRatio::ONE` の stand-in を挟む中間状態が不可避のため・小細工禁止の規律）。
- 3.2+3.3: **task 3.5 への申し送り**: `last_show` の `#[allow(dead_code)]` は `refresh_scale` が読み手になった時点で除去すること。
- 3.2+3.3: **task 5 への申し送り（範囲拡大）**: `attach_target` の破断呼び手は tasks.md が挙げる `examples/emo-present.rs` だけではない。実測＝`areka/src/emo2_boot/frame.rs:409/:450`（4.2 の領分）・`areka/examples/{emo-present.rs:620,:659, collision-probe.rs:442, window-placement.rs:419}`・**`areka-emo-text/examples/{emo-text-layer.rs:762, emo-text-typewriter-demo.rs:290}`・`areka-emo-text/tests/{attach_wiring_test.rs:156,:159, draw_readback_test.rs:306}`**。`areka-emo-text` の lib 自体は無傷。
- 3.2+3.3: **別 spec 候補（レビュー裁定）**: `chain.upload` の失敗を注入する seam が `chain.rs` に無いため、「表示成立後に upload 失敗→後続ヒット」経路（シナリオb）は構成による証明どまりで end-to-end 駆動できていない。device-failure seam が要るなら別タスクへ。
- 3.2+3.3: `native_size` は `cached_native`（cache スロットと対で書く私有フィールド）経由で**表示成立点に無条件コピー**。compose 経路でのみ書くと、失敗→後続キャッシュヒットで復帰した表示が `surface_size()` を永久に `None`／前サーフェス寸のまま返す（レビュー検出）。
- 3.2+3.3: **DPI 縮退テストの罠**: `author_dpi=96` で書くと、縮退の 1.0 と `Some((96,96))` 埋めの `96/96=1` が数値的に区別できず**檻が空虚**になる。非96（192）で attach すること。
- 3.1: **意図的な中間状態**＝このコミット時点で `areka-emo-present` はビルド不成立（`get`/`insert` 拡張により `presenter.rs:229/:241/:279/:347` が E0061）。**task 3.3 が本実装で修復する**。worktree は squash-merge 前提ゆえ main には現れない。暫定 stand-in（`ScaleRatio::ONE` 直書き）は入れていない（小細工禁止の規律）。
- 3.1: `cache.rs` は **変更前から rustfmt 非準拠**だった（3箇所）。今回整形されたため diff に無関係hunkが混じるが、HEAD版への `rustfmt --check` が同一出力を返すことで整形のみと実証済み。
- 3.1: 検証テクニック＝`presenter.rs` を一時パッチ（`ScaleRatio::ONE`）してテスト実行→**復元**。復元時は **mtime を更新**しないと cargo がリビルドを飛ばして**偽緑**になる罠あり。
- 2.3: **task 5 の一部を前倒し適用済み**（親コントローラ判断）。`placement/mod.rs:126` の `prepare_stages` が `&MeasureScaling::IDENTITY` を渡すよう追随済み（そうしないとツリーがビルド不能で 3.x/4.x の検証が全て塞がるため）。**task 5 の残件は `attach_target` 系の呼び手（`examples/emo-present.rs` ほか）のみ**。
- 2.3: `PlacementError::Measure` の `scope` は、native 段の balloon 失敗（全スコープ共通の採寸が倒れる）が `0` 固定、**k適用段の balloon 失敗は実スコープ番号**。消費側 `main.rs:760 is_benign_placement_error` は variant のみ照合し `scope` を見ないため非互換なし（レビュー確認済み）。
- 2.2: **task 4.2 への申し送り**: `resize_window_keep_position` は**同寸べき等 skip でも `false` を返す**（`resize_window_to` の既存慣行と一致）。呼び手は `false` を失敗と解釈してはならない（skip は `debug!`・失敗は `warn!` でログ層が分離されている）。
- 2.2: `SetWindowPosCommand` の TLS キュー（`wintf/src/ecs/window/command.rs:124`）は**私有・件数照会APIなし**・`flush()` は偽HWNDへ実 `SetWindowPos` を撃つ。headless で「書込ゼロ」を観測するには `enqueue_window_set_pos` 内で enqueue と不可分に走る **`Arrangement.offset` 同期の sentinel 据え置き**を witness に使う（レビューで健全性確認済み）。
- 2.2: **task 4.2 への申し送り（レビュー推奨）**: 新規テストは `Anchored`/`MonitorSnapshot` 不在の World で走るため「誤ってアンカー再射影を混入させた」変異が identity 縮退で見逃され得る。4.2 で両者同居下の位置不変ケースを足すと檻が強くなる。
- 2.1: ukadoc 正典で D1 を裏取り済み（shell `seriko.dpi` / balloon `dpi`・ともに「推奨DPI」既定96・SSP 2.7.21〜・対照表 100%→96/125%→120/150%→144/175%→168/200%→192）。**不正値・0 の扱いは正典が規定しておらず**、design D1 の warn+96 に従った。emo2 fixture は shell/balloon とも **DPI 無宣言**＝96（既存採寸期待値に影響なし）。
- 2.1: `placement/source.rs` は冒頭 doc で依存規約（areka-parsers＋std＋tracing のみ・emo/wintf/bevy_ecs へ依存しない）を宣言している。`DEFAULT_AUTHOR_DPI` を emo-present から import せず**ローカル定数で二重定義**したのはこの規約を守るため（レビュー承認済み）。
- 1.4: **task 6.2 への申し送り（レビュー推奨）**: `areka-emo-present` には log-capture ハーネスが無く（emo-compose の `log_capture` は crate 私有・`tracing-subscriber` の dev 依存も無し）、`derive_scale` の縮退ログ4本は**発火が檻に入っていない**。6.2 で emo-compose 型の `log_capture` モジュール＋`tracing-subscriber` dev 依存を追加すること（workspace 既存依存の dev 追加ゆえ R7.3 抵触なし）。現状は私有 `ScaleDecision` フラグで分岐選択のみ檻に入れている。
- 1.4: **task 3.2/3.3 への申し送り**: wintf `DPI` component を `Option<(u16,u16)>` へ写像するとき、**component 不在を `Some((96,96))` で埋めてはならない**。埋めると R1.4 の縮退分岐が「正常系のふり」で素通りする（design の Integration Tests 節と同趣旨）。
- 1.4: `cargo clippy -p areka-emo-present --all-targets` は**依存 `wintf` の deny-by-default lint**（`not_unsafe_ptr_arg_deref` × 20・`com/d2d/command_sink.rs`）で exit 101 になる既存事象。`RUSTFLAGS=--cap-lints=warn` で再走して自クレートを判定すること。
- 1.3: **task 6.1 への申し送り（レビュー非ブロッキング指摘）**: `resample` の premultiplied 検証が弱い。厳密値を主張するテストは全て α=255 で、α 可変ケースは `B,G,R ≤ A` の不変条件しか見ていないため、**非乗算化→再乗算する実装でも緑になる**。6.1 で「α 可変の厳密 golden」を1本足して締めること。
- 1.3: 色補間の丸めは 16bit 重み量子化ゆえ厳密有理 bilinear の **tie で下側に落ちる**（k=5/4 で 71.5→71）。決定論であり D5（整数固定小数点）の規定どおり。R2.5 の「単一の丸め規約」は**寸法**（`scaled_extent`）の話で色ではない、と裁定済み。
- 1.2: `plan.rs:334` に **既存の** clippy `collapsible_if` 警告あり（本 spec の境界外・触らない）。crate 全体に既存の `cargo fmt` 差分（import 順）もあるため、fmt は変更行のみで判定する。
