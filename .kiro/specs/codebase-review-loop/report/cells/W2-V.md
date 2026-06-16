# W2-V: wintf COM層 × 脆弱性レビューと非破壊対策

- status: completed
- commit: fix(W2): COM 層の unsafe 境界に debug_assert 10件・SAFETY 根拠コメント・空 HSTRING 特性化テストを追加

## findings

### 1. unsafe ブロックの境界条件

#### ulw.rs `transfer_to_hbitmap` — pitch < stride の OOB read 不変条件（W2-T 所見5の実施）
- 行単位コピー分岐は `pitch != stride` のみを判定し、`pitch < stride` なら src の行境界を越えて stride バイトを読み出す（OOB read）。D2D1 の `Map(D2D1_MAP_OPTIONS_READ)` 契約上、成功時 pitch はマップされたビットマップの 1 行分（幅 × 4 bytes/px）以上が保証され、呼び出し元契約（# Safety）で `width` は staging のピクセル幅と一致するため、現行コードで発火し得ないことを確認のうえ `debug_assert!(pitch >= stride)` を付加（ulw.rs:46-51）。`!src.is_null()`（Map 成功時の bits 非 null 契約）と `!dib_bits.is_null()`（# Safety 前提）の debug_assert も併設。既存テスト tests/graphics/compositor_transfer_test.rs（pitch≠stride 誘発ケース含む）が debug ビルドで全アサートを通過することを workspace テストで確認済み（提案不要・非破壊対策で完結）。
- 付随確認: `stride * height` の乗算は u32→usize 拡大変換後の演算で、現実的なビットマップサイズでオーバーフローしない（debug ビルドでは万一の場合も panic で検出される）。

#### d2d/mod.rs `draw_text` の `as_ref().unwrap()` — panic 経路は到達不能と実証（W2-S 申し送りの決着）
- W2-S は「HSTRING が null 表現（空 HSTRING 等）の場合に panic し得る経路」として申し送ったが、windows-core 0.62.2 のソース精査により **panic 不能** を実証: `Ref<T>::as_ref()` が None を返す唯一の経路は `Type<T>::is_null()` だが、HSTRING は `CloneType`（windows.rs `impl TypeKind for HSTRING`）であり `Type<T, CloneType>::is_null()` は常に false を返す（type.rs）。空 HSTRING（内部表現 null）でも `Some(&HSTRING)` が返り、Deref で空スライス `&[]` として DrawText へ渡る。
- 対策: unwrap の不可謬性根拠を SAFETY コメントとして明記（d2d/mod.rs:214-220）+ 特性化テスト `draw_text_with_empty_hstring_does_not_panic`（`HSTRING::new()` / `HSTRING::from("")` の両 null 表現で記録・Close 成功）を tests/com/d2d_ext_test.rs へ追加。production 入力からの panic 到達性なし → 提案不要。
- production 呼び出しは examples/dcomp_demo.rs:617（非空 `HSTRING::from_wide`）の 1 件のみ。

#### command_sink.rs — COM コールバックの生ポインタ無条件 deref と from_raw_parts
- `SetTransform` / `DrawRectangle` / `FillRectangle` / `PushAxisAlignedClip` / `PushLayer` は `*const` 値構造体引数を無条件 deref する。D2D1 のコマンドリスト再生（`ID2D1CommandList::Stream`）は記録済みコマンドから非 null を渡す契約のため発火しないことを確認し、5 箇所へ `debug_assert!(!ptr.is_null())` + SAFETY 根拠コメントを付加。
- `DrawGlyphRun` の `from_raw_parts(glyph_run.glyphIndices, glyphCount as usize)` は **len == 0 でも非 null・整列済みポインタを要求** する（Rust の from_raw_parts 契約）。glyphIndices は DWRITE_GLYPH_RUN の必須フィールド（glyphAdvances/glyphOffsets と異なり null 不許可、既存コードも後者のみ null 分岐あり）であり、D2D1 は検証済みグリフランのみ再生するため非発火。`!glyphrun.is_null()` / `!glyph_run.glyphIndices.is_null()` の debug_assert 2 件 + SAFETY コメントを付加。tests/com/d2d_ext_test.rs の Stream 再生テスト（実 DrawText 由来の DrawGlyphRun）が debug ビルドで通過することを確認。
- **COM インターフェイス引数の `Ref::unwrap()`（約 27 箇所）は panic が extern "system" vtable シム内で abort に化ける** — null COM 引数が渡った場合、HRESULT 返却ではなくプロセス即死となる。`ok()?`（E_POINTER 返却）への置換はエラー戻り値の挙動変更のため **P36** として記録（モジュール自体は利用ゼロの未完成コード = P33 の傘下。P33 で削除が選ばれれば P36 も消滅）。

#### Send/Sync
- com/ 配下に `unsafe impl Send` / `unsafe impl Sync` は存在しない（grep で 0 件）。点検対象なし・問題なし。

#### out パラメータの初期化前提
- dwrite.rs `get_cluster_metrics`: 2 段階取得（サイズ照会 → default 初期化済み Vec へ書き込み）で未初期化メモリの露出なし。1 回目の HRESULT 黙殺は既知 P34（再記録せず）。
- dcomp.rs `begin_draw`: `POINT::default()` 初期化 + `?` 伝播で健全。
- d3d11.rs `d3d11_create_device` / `create_texture2d` の `unwrap()`: 非 null out ポインタ + HRESULT 成功時の書き込み保証 + `?`/`map` による成功経路限定で健全。根拠を NOTE コメント 2 件として明記（tests/com/d3d11_test.rs の幅 0 エラーテストが失敗経路の非到達を特性化済み）。

### 2. COM ハンドルのリーク・二重解放

- `ManuallyDrop` の使用箇所は d2d/command_types.rs（`dup_com` パターン）に集中し、**他のモジュールに ManuallyDrop / mem::forget / 手動 Release は存在しない**（grep で確認）。
- `dup_com`（AddRef なしの transmute_copy + Release 抑止の ManuallyDrop）は構築 → drop の経路では参照カウント収支が均衡しており、リークも二重解放も発生しない。既存テスト `command_struct_drop_does_not_release_original_com_object` が drop 後の元オブジェクト生存を固定済み。リークは `#[derive(Clone)]`（clone の AddRef が永久に Release されない）経由のみで、これは既知 P33（再記録せず）。
- その他の COM ラッパーはすべて windows-rs バインディングが返す所有インターフェイス（drop で Release）を素通しで返却しており、手動の Release 対応付けを要する箇所なし。二重解放の懸念箇所なし。

### 3. 整数変換の切り捨て

- com/ 配下の縮小変換は ulw.rs の `AC_SRC_OVER as u8` / `AC_SRC_ALPHA as u8` の 2 件のみで、いずれも定数 0x00 / 0x01 の無損失変換（問題なし）。
- カウント・サイズ系の変換（`glyphCount as usize`・`actual_count as usize`・`pitch as usize`・`width as usize`）はすべて u32 → usize の拡大変換で切り捨てなし（該当箇所に注記を付加）。
- スライス長 → u32 の変換は windows-rs バインディング側で `len().try_into().unwrap()`（CopyPixels / GetClusterMetrics / DrawText で確認）であり、4GiB 超で silent truncation ではなく panic となる設計。本クレート側に切り捨て箇所なし。

### 変更ファイル（すべて挙動非破壊: debug_assert・コメント・追加テストのみ）

- `crates/wintf/src/com/ulw.rs` — debug_assert 3件 + SAFETY 不変条件コメント
- `crates/wintf/src/com/d2d/mod.rs` — draw_text unwrap の不可謬性 SAFETY コメント
- `crates/wintf/src/com/d2d/command_sink.rs` — debug_assert 7件 + SAFETY 根拠コメント
- `crates/wintf/src/com/d3d11.rs` — out パラメータ unwrap の NOTE 2件
- `crates/wintf/tests/com/d2d_ext_test.rs` — 特性化テスト 1件追加（空 HSTRING の draw_text）
- `report/proposals.md` — P36 追記

## 検証（S2）

- BEFORE: 親検証済みベースラインを信頼（HEAD 57015da・クリーンツリー・1298 passed / 0 failed）。再実行せず（セル指示に従う）。
- AFTER: `cargo build --workspace` 成功（exit 0）/ `cargo test --workspace` **1299 passed / 0 failed**（+1 は追加した特性化テスト。既存テストの変更・削除ゼロ）/ `cargo build --examples -p wintf` 成功（exit 0）。
- 追加した debug_assert 10件は debug プロファイルのテスト実行（tests/graphics/compositor_transfer_test.rs・tests/com/d2d_ext_test.rs の Stream 再生含む）で全件非発火を実証。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue` が 1 回目の全体実行で失敗（負荷依存）。隔離再実行 1 回で成功、続く 2 回目の全体実行でも成功 → pass-through と判定し記録。

## proposals

- P36: RecCommandSink の COM コールバックにおける `Ref::unwrap()` panic の COM ABI 境界越え（null COM 引数 → E_POINTER 返却への置換は挙動変更のため記録。P33 の削除パス選択時は同時クローズ）
- 既知 P33（dup_com / Clone リーク）・P34（GetClusterMetrics エラー黙殺）・P35（open E_NOTIMPL スタブ）は本セルの点検範囲と重複するが再記録せず。
- 申し送り: なし（W2-T 所見5・W2-S の draw_text 申し送りはいずれも本セルで決着）。
