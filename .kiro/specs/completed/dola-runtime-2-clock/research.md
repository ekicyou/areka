# Research & Design Decisions — dola-runtime-2-clock

## Summary
- **Feature**: `dola-runtime-2-clock`
- **Discovery Scope**: Simple Addition
- **Key Findings**:
  - QueryPerformanceCounter はハードウェアタイマーベースの高精度カウンター（マイクロ秒級）
  - `Win32_System_Performance` feature がワークスペース Cargo.toml に既存
  - 既存 wintf の `get_precise_time()` が Win32 unsafe パターンの参考実装として利用可能

## Research Log

### QueryPerformanceCounter / QueryPerformanceFrequency API 仕様

- **Context**: Req 2 の技術選定根拠として、QPC API の正確な仕様を確認
- **Sources Consulted**:
  - [QueryPerformanceCounter (profileapi.h)](https://learn.microsoft.com/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter)
  - [QueryPerformanceFrequency (profileapi.h)](https://learn.microsoft.com/windows/win32/api/profileapi/nf-profileapi-queryperformancefrequency)
  - [Acquiring high-resolution time stamps](https://learn.microsoft.com/windows/win32/sysinfo/acquiring-high-resolution-time-stamps)
- **Findings**:
  - `QueryPerformanceCounter(lpPerformanceCount: *mut i64) -> BOOL`: 高精度パフォーマンスカウンターの現在値を取得
  - `QueryPerformanceFrequency(lpFrequency: *mut i64) -> BOOL`: カウンター周波数（1秒あたりのカウント数）を取得
  - カウンター値 / 周波数 = 秒数（f64）
  - Windows XP 以降では常に成功（BOOL は常に非ゼロ）
  - カウンター起点は OS 起動時（実装依存だが現代 Windows では保証）
  - 分解能: 通常 1MHz 以上（マイクロ秒級）。システムのハードウェアタイマーに依存（TSC, HPET 等）
  - マルチプロセス・マルチスレッドで共有可能
  - 単調増加保証（ハードウェアレベル）
- **Implications**:
  - `counter as f64 / frequency as f64` の単純な除算で秒数が得られる
  - エラーハンドリング不要（常に成功）
  - frequency は通常セッション中不変だが、毎回取得しても性能影響は無視できる水準

### windows クレート 0.62 での QPC API シグネチャ

- **Context**: Rust windows クレートでの正確な関数シグネチャを確認
- **Sources Consulted**: windows crate 0.62.2 ドキュメント
- **Findings**:
  - `windows::Win32::System::Performance::QueryPerformanceCounter` — unsafe fn
  - `windows::Win32::System::Performance::QueryPerformanceFrequency` — unsafe fn
  - 引数: `*mut i64` (LARGE_INTEGER)
  - 戻り値: `windows_core::Result<()>` （成功/失敗）
  - 実運用上は常に成功するため、unwrap 可能（Windows XP+ 保証）
- **Implications**: 
  - 既存の `GetSystemTimePreciseAsFileTime` パターン（wintf 内）と同様に、関数内 `use` + 最小 `unsafe` ブロックで実装

### ワークスペース windows 依存との整合性

- **Context**: dola Cargo.toml に追加する features が既存ワークスペースと整合するか確認
- **Sources Consulted**: `Cargo.toml` L50-78
- **Findings**:
  - `Win32_System_Performance` はワークスペース Cargo.toml に **既存** (L65)
  - `workspace = true` で参照すればバージョン統一性を保証
  - dola 側で独自に `features = ["Win32_System_Performance"]` を指定しても、ワークスペースの features union に統合される
- **Implications**: 新規 feature 追加不要。`workspace = true` + features 指定のみ

## Design Decisions

### Decision: GetTickCount64 → QueryPerformanceCounter 移行

- **Context**: アニメーションエンジンにおけるフレーム間時間差計測の精度要件
- **Alternatives Considered**:
  1. `GetTickCount64` — ms 精度（10~16ms 分解能）、シンプル
  2. `QueryPerformanceCounter` — マイクロ秒精度、ハードウェアタイマーベース
  3. `IUIAnimationTimer::GetTime()` — COM 依存、f64 秒直接取得
  4. `std::time::Instant` — Rust 標準、OS 起動時起点ではない
- **Selected Approach**: QueryPerformanceCounter / QueryPerformanceFrequency
- **Rationale**:
  - アニメーションエンジンではフレーム間の < 1ms の時間差を正確に計測する必要がある
  - GetTickCount64 の 10~16ms 分解能では不足（60fps = 16.67ms に対してジッター大）
  - COM 非依存方針により IUIAnimationTimer は不採用
  - OS 起動時起点かつマルチプロセス共有可能
- **Trade-offs**: QPC は 2回の API 呼び出し + 除算が必要（GetTickCount64 は 1回 + 除算のみ）。性能差はナノ秒オーダーで無視可能
- **Follow-up**: 実装時に frequency の呼び出し頻度を検討（毎回 vs キャッシュ）。毎回でも性能上問題ないが、設計上はキャッシュ不要（ステートレス維持を優先）

### Decision: Feature Gate 削除 → cfg(target_os) 移行

- **Context**: Windows 専用ユーティリティ関数の条件コンパイル方式選定
- **Alternatives Considered**:
  1. `#[cfg(feature = "windows-clock")]` — ユーザー選択式
  2. `#[cfg(target_os = "windows")]` — OS 自動判定
- **Selected Approach**: `#[cfg(target_os = "windows")]`
- **Rationale**: clock::now() は完全なユーティリティ関数であり、利用者の選択肢ではない。areka は Windows 専用プロジェクト。OS 自動判定で十分
- **Trade-offs**: 非 Windows 環境で clock モジュールが利用不可になるが、代替実装は要件外

## Risks & Mitigations
- **Risk**: QPC の frequency がシステムによって異なり、テスト環境間で精度差が生じる
  - **Mitigation**: テストでは相対的な精度検証のみ（sleep 前後の差分が閾値内）。絶対値は検証しない
- **Risk**: f64 精度が長期運用で劣化する可能性
  - **Mitigation**: f64 仮数部 53bit で約 285 年の精度維持が可能。非現実的な長期利用でも問題なし

## References
- [QueryPerformanceCounter (profileapi.h)](https://learn.microsoft.com/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter) — 公式 API ドキュメント
- [QueryPerformanceFrequency (profileapi.h)](https://learn.microsoft.com/windows/win32/api/profileapi/nf-profileapi-queryperformancefrequency) — 周波数取得 API
- [Acquiring high-resolution time stamps](https://learn.microsoft.com/windows/win32/sysinfo/acquiring-high-resolution-time-stamps) — 高精度タイマー設計ガイド
- [windows crate 0.62.2](https://docs.rs/windows/0.62.2/windows/) — Rust Win32 バインディング

---

## Benchmark Results (Task 4)

**実施日**: 2026-02-15  
**環境**: Windows (dev profile, unoptimized + debuginfo)  
**手法**: `std::time::Instant` による手動計測（10,000 回反復、1,000 回ウォームアップ）

### 計測結果

| 項目 | 値 |
|------|-----|
| 反復回数 | 10,000 |
| 合計時間 | 806 μs |
| **平均呼出時間** | **80.6 ns/call** |
| 60FPS フレーム間隔比 | 0.000484% |
| 現在の `now()` 値 | 291,427.069592 秒（OS 起動後 約 81.0 時間 ≒ 3.4 日） |

### 分析

- **性能**: 平均 80.6 ns/call（デバッグビルド）。リリースビルドではさらに高速化が見込まれる
- **frequency 毎回取得のコスト**: QPC + QPF の 2 回 API 呼び出しで 80.6 ns。frequency をキャッシュした場合との差はナノ秒オーダーであり、ステートレス設計の維持が妥当
- **60FPS 影響**: 16.67ms フレーム間隔に対して 0.000484%。**完全に無視可能**
- **結論**: 「性能影響はナノ秒オーダー」の design.md の主張を実測で確認。frequency 毎回取得によるステートレス設計を維持する判断は妥当

### now() 返却値の検証

- OS 起動後 291,427 秒 ≒ 81.0 時間 ≒ 3.4 日（実際の uptime と整合）
- 値は正の有限値であり、f64 精度で小数点以下 6 桁（マイクロ秒級）を維持
