# W2-S: wintf COM層 × シンプル化の検証と適用

- status: completed
- commit: refactor(W2): COM 層の構造的整理 — 空 dxgi モジュール削除・wic の生ポインタ排除・dwrite wcslen ループの as_wide 置換

## findings

### 適用した簡素化（4件、いずれも挙動非破壊・呼び出し側の修正ゼロ）

1. **空モジュール `dxgi.rs` の削除（W2-T 所見3の実施）** — `crates/wintf/src/com/dxgi.rs` は改行（CRLF）1つのみの空ファイル。ワークスペース全域（crates / examples / tests）の grep でモジュールパス `dxgi::` / `com::dxgi` への参照ゼロを実証（`windows::Win32::Graphics::Dxgi`（大文字 D）への参照は windows クレート側であり無関係）。ファイル削除 + `mod.rs` の `pub mod dxgi;` 宣言削除（R5.3: grep + ビルド成功で実証）。なお steering `structure.md:32` に「dxgi.rs - DXGIインターフェイス」の記載が残るが steering はセル境界外のため未修正（P29 と同種の steering 同期事項として申し送り）。
2. **wic.rs: `pguidvendor: Option<*const GUID>` → `Option<&GUID>`（W2-T 所見7の実施）** — safe API に露出していた生ポインタを参照へ置換（トレイト宣言 + impl。impl 内で `map(|guid| guid as *const GUID)` により windows-rs の生シグネチャへ変換）。全呼び出し箇所5件（hit_region/mod.rs:159・bitmap_source/systems.rs:85・examples/dcomp_demo.rs:487・tests/com/wic_test.rs×3・d2d_ext_test.rs:226）が裸の `None` を渡しており型推論で吸収されるため、呼び出し側の修正ゼロ。tests/com/wic_test.rs の直接テスト8件で保護された変更（R5.5 の test-protected 領域）。
3. **wic.rs: `WICFormatConverterExt::init` の `dstformat: *const GUID` → `&GUID`** — 所見7と同類の生ポインタ露出。全呼び出し箇所7件はすべて `&GUID_WICPixelFormat...` を渡しており（従来は参照→生ポインタの暗黙強制変換に依存）、`&GUID` 化で呼び出し側は無修正のまま型が厳密化。null/ダングリングポインタを safe 関数から unsafe FFI へ渡せてしまう健全性の穴も同時に閉じた。
4. **dwrite.rs: `create_text_layout` の手書き wcslen ループを `PCWSTR::as_wide()` へ置換** — 手書きの null 終端走査ループ + `from_raw_parts`（10行）は windows-strings 0.4.2 の `PCWSTR::as_wide()`（len() = 同一の wcslen 走査 + from_raw_parts）と完全等価であることをレジストリのソースで確認のうえ置換（22行 → 13行）。null PCWSTR → 空スライスの分岐は維持（`as_wide` は非 null 前提のため）。tests/com/dwrite_test.rs の11件（null PCWSTR・空文字列・ASCII・日本語・サロゲートペア U+20BB7 の UTF-16 走査単位特性化）で保護された変更。
5. **dcomp.rs: `scroll` の戻り値型表記 `windows_core::Result<()>` → `Result<()>`（構造的整理）** — 同一型（`windows::core::Result` は windows_core の再エクスポート）の表記揺れをファイル内の他14メソッドと統一。テキスト上の変更のみ。

### 見送り（候補と根拠）

- **d2d 録画モジュール（command_sink.rs / command_types.rs、計1,016行）** — P33 で記録済み（利用ゼロ・todo!()・Clone の COM リーク）。`#[deprecated]` なしのため R2.9 削除不可。churn 回避のため一切触れず。
- **`DWriteTextLayoutExt::get_cluster_metrics/get_cluster_count` のエラー黙殺** — P34 で記録済み。戻り値セマンティクス変更のため現状維持。
- **`D2D1CommandListExt::open`（常時 E_NOTIMPL スタブ）** — 公開 API の削除 + 特性化テスト（`command_list_open_always_returns_e_notimpl`）の撤去を伴うため本セルでは不可 → **P35** として記録。
- **`d2d/mod.rs::draw_text` の `hstring_borrow.as_ref().unwrap()`** — HSTRING パラメータが null 表現（空 HSTRING 等）の場合に panic し得る経路。簡素化ではなく堅牢性の論点であり、テスト保護下とはいえ panic 挙動の変更となるため W2-V（脆弱性点検）へ申し送り。
- **`ulw.rs::transfer_to_hbitmap` の `pitch < stride` debug_assert** — W2-V 担当（W2-T 所見5）のため不変。
- **dcomp.rs / animation.rs / d3d11.rs の本体** — いずれも引数素通しの薄いラッパー（Option→生ポインタ変換含む）でロジックを持たず、S6 基準で「最小コード」を既に満たすと判断。変更なし。

### S6（karpathy-guidelines）適合確認

- 変更5件はすべて「既存の問題（生ポインタ露出・手書き再実装・死にファイル・表記揺れ）の除去」であり、新規抽象・投機的柔軟性の追加はゼロ。各変更行は W2-T からの申し送り（所見3/7）または明白な重複除去にトレースできる（Surgical Changes）。
- 自分の変更で孤児化したものなし（dxgi.rs 削除に伴う mod 宣言削除のみ）。

## 検証（S2）

- BEFORE: 親検証済みベースラインを信頼（HEAD 48bad97・クリーンツリー・1298 passed / 0 failed）。再実行せず（セル指示に従う）。
- AFTER: `cargo build --workspace` 成功（exit 0）/ `cargo test --workspace` **1298 passed / 0 failed**（exit 0、ベースラインと完全一致 = テストの追加・変更・削除ゼロで全既存テストが新シグネチャをそのまま通過）/ `cargo build --examples -p wintf` 成功（exit 0、dcomp_demo.rs 含む全 example が無修正でコンパイル）。
- 変更ファイル: `crates/wintf/src/com/{mod.rs, wic.rs, dwrite.rs, dcomp.rs}`・`dxgi.rs` 削除・report 2ファイル。boundary（crates/wintf/src/com/ + report）内に収束し、tests/com/ への変更もゼロ。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue` を含め失敗なし（隔離再実行不要）。

## proposals

- P35: D2D1CommandListExt::open の常時 E_NOTIMPL スタブの削除（API 変更 + 特性化テスト撤去を伴うため記録のみ）
- 申し送り: `d2d/mod.rs::draw_text` の `as_ref().unwrap()` panic 経路 → W2-V へ。steering structure.md の dxgi.rs 記載の陳腐化 → steering 同期時に修正（P29 と同種）。
