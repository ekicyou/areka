# W5b-V: wintf 図形・画像・ブラシ × 脆弱性レビューと非破壊対策

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点・基準・範囲

- セルID: W5b-V（領域 W5b「wintf 図形・画像・ブラシ」 × 観点 V「脆弱性レビュー」）。性質: **非挙動変更**（脆弱性点検＋挙動非破壊な対策のみ）。Feature Flag Protocol 不要。
- requirements（source 番号）: 2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更を伴う対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順 T→S→V。W5b-T/W5b-S 完了済みの回帰検知器上で実行）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations（L512-516: unsafe 境界・整数変換・Win32/COM ハンドルのリーク/二重解放・外部入力（ファイルパス・画像データ）の検証欠如・panic 経路 DoS を点検し、挙動を変えない範囲＝内部チェック・debug_assert・安全な型置換のみ投入。API/エラー応答を変える対策は proposals へ）、CellExecutor 観点別規則 V（L338）、提案記録様式（L453）、セル断片様式（L440）、W5b 領域定義（L162）。
- 領域（boundary = `crates/wintf/src/ecs/widget/{shapes,bitmap_source}/` + `widget/brushes.rs`、tests/ の該当ドメイン含む）: shapes/（mod.rs・rectangle.rs）、bitmap_source/（mod.rs・alpha_mask.rs・bitmap_source.rs・resource.rs・systems.rs・task_pool.rs・wic_core.rs・tests.rs）、brushes.rs の計11ファイル。境界外には一切触れていない。
- 起点: W5b-S 適用後のクリーンなワークツリー（親検証済みベースライン 1486 passed / 0 failed）。
- **本領域の最重要点検対象**: GPU（Direct2D）/WIC（画像コーデック）依存が中心。**外部入力として画像ファイル・ファイルパス（ユーザ提供 `BitmapSource.path`）が WIC 経由でデコードされる**経路（resolve_path → load_bitmap_source）を重点追跡した。

## 点検手法

境界内11ファイル（うち in-source tests: alpha_mask.rs / brushes.rs / shapes/rectangle.rs、分離 tests.rs）を grep（`unsafe`/`unwrap(`/`expect(`/`panic!`/`unreachable!`/`todo!`/`as `/添字 `[`・パス操作・WIC 呼び出し）＋全文精読で走査。外部入力経路は `BitmapSource.path`（String）→ `on_bitmap_source_add` フック → `resolve_path` → `load_bitmap_source`（WIC デコード）を端から端まで追跡した。

unsafe 境界の健全性根拠を実証するため、windows-rs 0.62.2（本 crate 使用版、Cargo.lock 確認）の Send/Sync 生成状況を**直接 grep して確認**:
- WIC（`Imaging/mod.rs`）: `unsafe impl Send` が**0 件**（51 個の `IWIC*` インターフェイス全てに対し未生成）。
- 対照: DirectWrite 95 件・Direct2D 114 件（`ID2D1Bitmap1` は Send+Sync 付与済み＝Direct2D/mod.rs:2560-2561）。

これにより「WIC 型は windows-rs が Send/Sync を自動付与しない」＝本領域の WIC 保持型の手動 `unsafe impl` は**冗長ではなく必須**であることを実証した（W5a-V の `IDWriteTextLayout`＝付与済み・冗長、とは逆のケース）。さらに COM 初期化が **MTA**（`CoInitializeEx(COINIT_MULTITHREADED)`、win_thread_mgr.rs:98）であること、WIC オブジェクトが `WintfTaskPool`（bevy_tasks）ワーカー → メインスレッドへ移送される実利用経路を確認し、free-threaded 健全性を裏取りした。

## 発見した脆弱性候補と判定

### 1. 外部入力（画像パス）の検証欠如 — パストラバーサル未検証 → P56（提案化）。WIC デコード失敗は現状安全

- **`resolve_path`（systems.rs:38-51）のパストラバーサル未検証 → P56**: `BitmapSource { path: String }`（呼び出し側提供）の相対パスを `current_exe().parent().join(path)` で無検証 join する。`..` を含む相対パス（`../../secret.png`）も絶対パス（`C:\Windows\...`）もそのまま通過し、解決後パスは `load_bitmap_source` の WIC `create_decoder_from_filename` に `GENERIC_READ` で渡される。path を外部制御できる構成（設定/スクリプト由来のウィジェット構築）では意図しないファイル読み取り（情報開示）に至り得る。現状リポジトリでは path は開発者埋め込み定数のため実害未発現だが、`resolve_path` 自体は任意文字列を検証しない。検証追加（`..` 拒否・基準ディレクトリ jail・絶対パス許可ポリシー明文化）は `resolve_path` の戻り値が `Ok`→`Err` へ変わる**外部観測可能な挙動変更**（従来通る入力を弾く入力検証の厳格化）のため、R2.4/R5.2 に従い**本ループでは実装せず P56 に記録**。現行挙動は W5b-T の `resolve_path` 特性化テスト3件で固定済み。
- **WIC デコード経路（load_bitmap_source、systems.rs:75-105）— 現状安全**: 巨大画像・不正フォーマット・デコード失敗はすべて `Result` 返却ラッパ（`create_decoder_from_filename`/`frame`/`create_format_converter`/`init`/`cast`、いずれも `?` 受け）を経由し、失敗時は呼び出し側 `on_bitmap_source_add`（bitmap_source.rs:124-130）で `warn!` + return（タスク終了）に縮退する。パスの `HSTRING::from`（systems.rs:80）は不可謬の全域変換（panic なし）。**panic/未定義動作なし。**

### 2. unsafe 境界 — WIC の `unsafe impl Send/Sync` は健全かつ必須。SAFETY 注記を crate 標準へ格上げ（適用）

境界内の `unsafe impl Send/Sync` は4型: `BitmapSourceResource`（`IWICBitmapSource` 保持、resource.rs:21-22）・`BitmapSourceGraphics`（`Option<ID2D1Bitmap1>`、resource.rs:63-64）・`InsertBitmapSourceResource`（`IWICBitmapSource`、systems.rs:118）・`WicCore`（`IWICImagingFactory2`、wic_core.rs:22-23）。点検の結果:

- **WIC 保持3型（BitmapSourceResource / InsertBitmapSourceResource / WicCore）は健全かつ必須**: windows-rs 0.62.2 は WIC 型に Send/Sync を自動生成しない（上記実証）ため手動 impl は load-bearing。健全性は (a) WIC が free-threaded（thread-free marshaling）、(b) プロセスが MTA 初期化、(c) 実利用上バックグラウンドワーカーでデコード生成 → mpsc でメインスレッドへ移送（Send）し、以後 `source()`/`factory()` 読み取り参照のみ（Sync）——に依拠。**従来の注記はクレート標準（"SAFETY 条件:" 根拠ブロック）より簡素な1行コメントだった**ため、`command_list.rs:29` / `components.rs:39` / `core.rs:29` / `dcomp_resource.rs:30` の確立済み慣習に合わせて根拠を明文化する SAFETY 注記へ格上げした（適用1〜3）。
- **`BitmapSourceGraphics`（`ID2D1Bitmap1`）は冗長だが健全**: windows-rs が `ID2D1Bitmap1` に Send+Sync を付与済みのため自動導出可能で手動 impl は冗長。ただし crate 全域の COM 保持コンポーネントが一貫して明示 impl を採る慣習・将来の windows-rs 仕様変更への保険として残置し、その旨（冗長だが残置）を正直に記す SAFETY 注記を付与（適用1）。撤去は挙動非破壊だが任意・churn のため proposals 化しない（W5a-V の `TypewriterLayoutCache` と同方針）。
- **FFI 呼び出しブロック（systems.rs:197/210/215/283/301、rectangle.rs:174/187/192、wic_core.rs:29）**: `CreateCommandList`/`SetTarget`/`BeginDraw`/`EndDraw`/`GetDpi`/`CreateBitmapFromWicBitmap`/`CoCreateInstance` 等の標準 windows-rs 呼び出し。全面 D2D/WIC 依存の描画・生成システム内でユニット到達不能。引数はスタック上の有効値・直前ガード済み参照で、新たに明文化を要する非自明な不変条件はなし。**現状安全（対策不要）。**

### 3. リソースリーク・二重解放 — 現状安全（対策不要）

- 境界内に手動 `Release`/`mem::forget`/`transmute` は**ゼロ**。`create_d2d_bitmap`（systems.rs:298）の `colorContext: ManuallyDrop::new(None::<ID2D1ColorContext>)` は `D2D1_BITMAP_PROPERTIES1` 構築の windows-rs 定石で、`None` のラップは所有リソースを持たず**リークしない**。
- 保持 COM 資源（`IWICBitmapSource`・`IWICImagingFactory2`・`ID2D1Bitmap1`・CommandList・brush）はすべて windows-rs が返す所有インターフェイスで Rust `Drop` → Release。生成⇔破棄は対称（`BitmapSourceGraphics::invalidate`/`set_bitmap` は Option 入れ替えで旧値 Drop、`draw_*` のローカル CommandList/brush はスコープ終端 Drop）。`WintfTaskPool` の mpsc 受信側は `Mutex` 保持で `drain_commands`/`drain_and_apply` が `try_iter` 消費。**現状安全。**

### 4. 整数変換（画像寸法）— generate_alpha_mask_system は既知 P55、from_pbgra32 内部乗算は同クラスで P55 参照

- **`generate_alpha_mask_system`（systems.rs:402-403）の `stride = width*4` / `buffer_size = (stride*height) as usize` → 既知 P55**: 外部画像由来 `(u32, u32)` 同士の乗算で巨大寸法時に u32 オーバーフロー（デバッグ panic／リリース ラップ→過小確保）。**W5b-T が P55 として記録済み**。本 V セルで再確認したが新規採番せず**参照に留める**。
- **`AlphaMask::from_pbgra32`（alpha_mask.rs:36/39）の内部乗算 `row_bytes * height as usize` / `y * stride` も同クラス**: 同じ WIC 寸法 `(width, height, stride)` を消費する乗算で、巨大画像では理論上オーバーフローし得る。ただし**供給元・外部入力源が P55 と同一**（`source.get_size()` → `generate_alpha_mask_system` が `from_pbgra32` を呼ぶ）であり、P55 の対策（usize 昇格／checked_mul、寸法上限）が generate_alpha_mask_system 側で施されれば `from_pbgra32` へ渡る寸法も健全化される。**P55 と同一クラスの同根経路**のため新規採番せず P55 参照に留める（指示「類似のオーバーフローを再発見しても既知に該当すれば参照」に従う）。`from_pbgra32`/`is_hit` 単体の純粋ビットパック側は W5b-T が13件で特性化済み。

### 5. panic 経路 — 大半が現状安全。is_hit の添字に挙動非破壊の不変条件 debug_assert を適用

境界内プロダクション経路の `unwrap()`/`expect()`/`panic!`/`unreachable!`/`todo!` を個別判定:
- **`rectangle.rs:159` `DEFAULT_FOREGROUND.as_color().unwrap()`** — `DEFAULT_FOREGROUND` は `const Brush::BLACK`（外部入力ではない Solid）で `as_color()` は当該 const に常に `Some` を返す不可謬 fallback。リリースでも到達不能 panic。**現状安全**（W5a-V の同型 `DEFAULT_FOREGROUND.as_color().unwrap()` 判定と整合）。
- **`rectangle.rs:85` の `.unwrap()`** — doc コメント（`///`）内の例示。実コードではない。**該当なし。**
- **`from_pbgra32`（alpha_mask.rs:43）`pixels.get(pixel_offset + 3).copied().unwrap_or(0)`** — `.get()` + `unwrap_or(0)` の全域アクセス。範囲外は α=0 扱い（パディング/不足バッファでも panic しない）。**現状安全。**
- **`is_hit`（alpha_mask.rs:77）`self.data[byte_index]`** — 直前の範囲チェック（:69、`x >= width || y >= height`）が `x<width, y<height` を保証し、well-formed マスク（`from_pbgra32` が `data.len() == row_bytes*height` を確立）では `byte_index <= height*row_bytes - 1 < data.len()` が常に成立し添字は範囲内。**ただし `data` 長と `width/height` の一致不変条件はチェックの外側にある**ため、手構築/将来変更で寸法と data 長が乖離すると OOB panic に至り得る（W4b-V の `index_map[index]` と同型）。→ **挙動非破壊の debug_assert を適用**（下記 適用4）。リリース挙動は不変、デバッグでのみ不整合を検出。
- **その他 `expect(`/`unwrap(`** — すべて `#[cfg(test)]`（tests.rs / rectangle.rs:353 等）。プロダクション非経路。
- **生添字 `[i]`** — `data[byte_index]`（上記）以外の危険な生添字はプロダクションになし（`from_pbgra32` の `data[byte_index] |=` も同一不変条件下で範囲内、書き込み側は range loop `0..height`×`0..width` で構造的に保証）。

## 適用した挙動非破壊対策（4 ファイル・6 箇所、+99/−2 行）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `resource.rs` | `BitmapSourceResource` の `unsafe impl Send/Sync` 直前 | windows-rs が WIC 型に Send/Sync 未生成＝**必須**である旨＋free-threaded/MTA/実利用経路の健全性根拠を記す SAFETY 注記（12 行） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。crate 標準（command_list.rs 等）の SAFETY 根拠ブロックへ格上げ。 |
| `resource.rs` | `BitmapSourceGraphics` の `unsafe impl Send/Sync` 直前 | `ID2D1Bitmap1` は付与済みで**冗長だが健全**・crate 慣習で残置する旨の SAFETY 注記（7 行） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。冗長性を正直に明記（W5a-V 方針）。 |
| `wic_core.rs` | `WicCore` の `unsafe impl Send/Sync` 直前 | `IWICImagingFactory2` 必須＋CLSCTX_INPROC_SERVER/MTA/clone→ワーカー move の健全性根拠を記す SAFETY 注記（9 行、旧1行コメント置換） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。 |
| `systems.rs` | `InsertBitmapSourceResource` の `unsafe impl Send` 直前 | `IWICBitmapSource` 必須＋デコード結果のワーカー→メイン移送（Send のみ）健全性の SAFETY 注記（4 行、旧1行コメント置換） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。 |
| `alpha_mask.rs` | `is_hit`（:77 添字直前） | `debug_assert!(byte_index < self.data.len(), ...)` ＋不変条件コメント | debug_assert（内部不変条件） | リリースで compile-out（挙動不変）。well-formed マスクでは常に真＝発火せず、手構築の寸法/長さ不整合のみ検出。R2.3「挙動を変えない内部チェック」に該当（W4b-V の index_map debug_assert と同型）。 |
| `alpha_mask.rs` | in-source `mod tests` 末尾 | `is_hit` 添字不変条件の特性化テスト 2 件 | 特性化/回帰テスト（S9 命名準拠） | 範囲外座標が添字手前で return する安全鎖＋well-formed マスク全有効座標（最右下含む）で debug_assert 非発火を固定。 |

### 追加した特性化テスト一覧（`alpha_mask.rs`、in-source 2 件）

- `test_is_hit_out_of_bounds_returns_before_indexing` — 非バイト境界幅（width=9→row_bytes=2）で x==width / y==height / 両超過 / `u32::MAX` 各種の範囲外座標が `data[byte_index]` に到達せず false を返す（panic なし）ことを固定。
- `test_is_hit_all_valid_coords_satisfy_index_invariant` — 非バイト境界幅（width=17→row_bytes=3、最右下 (16,4)）の全有効座標を走査し `byte_index < data.len()` が常に成立（debug ビルドでも debug_assert 非発火）＝最大 byte_index が安全に読まれることを固定。is_hit に追加した debug_assert がリリース挙動を変えないことの回帰検知器。

## proposals.md へ回した候補（P56）

- **P56**: `resolve_path`（systems.rs:38-51）の画像パスに対するパストラバーサル検証の欠如（相対パス無検証 join・絶対パス素通り）。kind: 挙動変更を伴う脆弱性対策。`..` 拒否・基準ディレクトリ jail・絶対パス許可ポリシー明文化は `resolve_path` の戻り値が `Ok`→`Err` へ変わる（従来通る入力を弾く入力検証の厳格化）ため記録のみ。P3（areka 側アセットパス実行時化）と方針を揃えるのが望ましい。

既知 proposals の再発見（重複記録なし・参照に留めた）:
- **P55**（W5b-T）: `generate_alpha_mask_system` の寸法 u32 乗算オーバーフロー。本 V セルで再確認したが既知のため参照のみ。`from_pbgra32` 内部乗算（同じ WIC 寸法を消費）も同一クラスの同根経路として P55 参照に含めた（新規採番せず）。
- **P51**（W4b-T）: `BitmapSourceResource` のテスト用コンストラクタ追加。本 V セルの `is_hit` 添字特性化は AlphaMask 単体（実 WIC 不要）で到達可能なため P51 非依存だが、αマスク**生成側**（generate_alpha_mask_system / BitmapSourceResource::new の実 WIC 経路）のユニット到達は本セルでも未充足で P51 が引き続き該当（参照に留めた）。
- **P53**（W4b-V）: `ColorMapData::from_image` の同型オーバーフロー。P55 と同一クラスの hit_region 側別箇所。本境界外（layout）だが整数オーバーフロー方針として統合実施が望ましい旨は P55/P56 suggestion に既述。

## verification (S2)

- BEFORE: 親検証済みベースライン（W5b-S 直後 = 1486 passed / 0 failed、クリーンワークツリー）を信頼し省略（design フェーズ0 規定 + 親指示「BEFORE S2 は省略可」に従う）。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、7.31s）。
  - `cargo test --workspace` → **1488 passed / 0 failed**（ignored 32。全 20 本の `test result` 行を awk 合算で実測、FAILED result 行ゼロ）。ベースライン 1486 から **+2 = 追加した特性化テスト 2 件と一致**（既存テストの削除・変更ゼロ）。
  - 反復検証: `cargo test -p wintf --lib widget::` で **74 passed / 0 failed**（W5b-S の 72 + 新規2）。`--lib widget::bitmap_source::alpha_mask` で **15 passed / 0 failed**（W5b-T の 13 + 新規2）。
  - 追加2件は初回実行で合格（特性化テスト＝GREEN by construction。下記 RED 代替を参照）。**debug_assert は全 well-formed マスクテスト（alpha_mask 15件）で発火せず**＝デバッグでも panic なし＝リリース挙動不変を実証。
- 変更ファイル（`git diff --numstat` 実測）: `alpha_mask.rs`（+71/−0）・`resource.rs`（+16/−0）・`systems.rs`（+4/−1）・`wic_core.rs`（+8/−1）・`proposals.md`（+6/−0）。boundary 内（bitmap_source/）＋提案台帳に収束。tests/・shapes/・brushes.rs・他 widget サブモジュール不変。追加 `#[test]` は git diff 実測で**正確に2件**（いずれも alpha_mask.rs）。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` は 153 警告。**本セルの編集（SAFETY コメント・debug_assert・特性化テスト追加）は新規 clippy 警告を一切導入していない**——boundary 5ファイル（alpha_mask.rs / resource.rs / systems.rs / wic_core.rs / proposals は対象外）への clippy span は出力にゼロ（`ecs/widget/(bitmap_source|shapes)/`・`widget/brushes.rs` の grep ヒットなし）。153 は全て既存・boundary 外で本セル無関係（W5b-S が `manual_div_ceil`/`derivable_impls` を解消済みのため W5b-S 記録の 156 より減）。S3 規定によりブロッカーとせず記録に留める。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W5b 境界外 `tests/ecs`）: `cargo test --workspace` 全量実行で `tests/ecs` は failed=0 で合格。確認のため隔離再実行 `cargo test -p wintf --test ecs cue_performance_test` も実施し **5 passed / 0 failed**（`bench_pop_ready_empty_queue` ... ok 含む）で安定合格。本セルの変更（SAFETY コメント・αマスク debug_assert・テスト）は cue キュー timing と無関係。flaky 判定によりゲート通過。

## RED フェーズ代替の検証

追加2件はいずれも既存の安全挙動（範囲チェック→添字手前 return／well-formed マスクの添字不変条件）の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に導出: (a) 範囲外テストは `is_hit` の `x>=width||y>=height → return false` 早期 return 仕様から、(b) 不変条件テストは `row_bytes = ceil(width/8)`・`data.len() = row_bytes*height`・`byte_index = y*row_bytes + x/8 <= height*row_bytes-1` の算術から導出。初回実行で2件とも導出どおり一致し、debug_assert も全 well-formed 構築（alpha_mask 既存13＋新規2）で発火せずリリース挙動不変を S2 全量（1488=1486+2）で実証した。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダ/TODO なし）。本セルの変更は SAFETY 注記4・debug_assert 1・特性化テスト2のみで、新たな unsafe・スタブを導入していない。
- 点検は境界内11ファイルを grep＋精読で網羅。外部入力（画像パス）経路を `BitmapSource.path` → `resolve_path` → `load_bitmap_source`（WIC）まで端から端まで追跡し、unsafe 境界（WIC `unsafe impl` 4型）の健全性を windows-rs 0.62.2 ソース直接 grep（WIC=Send 0件／D2D=付与済み）＋MTA 初期化＋実利用経路で二重実証。リソースリーク・整数変換・panic 経路の5観点すべてを判定。warranted な挙動非破壊対策は (a) WIC unsafe の SAFETY 注記 crate 標準化（必須/冗長を正直に区別）と (b) is_hit 添字不変条件の debug_assert＋特性化2件に限られた。挙動変更を要する実在脆弱性（パストラバーサル）は P56 へ記録、既知 P55（寸法オーバーフロー）は参照に留めた。
- 件数の実測整合: S2 全量 1488 = 1486 + 2（追加テスト2）。widget lib 72→74、alpha_mask 13→15。追加 `#[test]` git diff 実測 = 2。clippy 153（boundary 新規ゼロ）。すべて git diff・cargo test 実測と一致（推測なし）。
- 境界遵守: 変更は `bitmap_source/{alpha_mask,resource,systems,wic_core}.rs`（W5b 境界内）＋ `proposals.md`（提案台帳）のみ。tasks.md 未更新・コミット未作成・境界外（shapes/・brushes.rs・他 widget・session 開始時に既存改変のあった layout 群・A1-S.md）/`vendors/`/機能spec文書への変更なし。
- 結論: 本境界は GPU/WIC 依存中心ながら脆弱性耐性は高い。最重要の WIC `unsafe impl Send/Sync` は**健全かつ（WIC 保持3型は）必須**で、windows-rs ソース実証に基づく crate 標準 SAFETY 注記で根拠を固定した。is_hit の添字不変条件は debug_assert で明文化。外部入力のパストラバーサル（P56）と寸法オーバーフロー（既知 P55）は挙動変更を伴うため記録に留め、リソースリーク・その他 panic 経路はすべて現状安全と判定して churn を回避した。
