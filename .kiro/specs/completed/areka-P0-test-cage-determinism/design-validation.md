# Design Validation Report: areka-P0-test-cage-determinism

- 対象: `design.md`（HEAD `6dde882c`・コードは `origin/main f6b81078` と同一ツリー）
- 実施日: 2026-08-22（非対話・自動実行）
- 判定: **GO（条件付き）** — 下記の重要指摘 3 件を設計ディスカッションで解決してから tasks へ進む

## 1. レビュー要約

設計は「テスト支援の leaf crate（`log-capture-kit`）＋ crate ごとの薄いアダプタ」で 4 系統を 1 本にまとめる形で、既存アーキテクチャ（`wintf` の依存方向・bin crate `areka` の in-crate テスト・兄弟ファイル規約）と整合している。file:line の引用は現行ツリーとほぼ一致し、件数（`with_default(` 40 ファイル・`capture_under_filter` 96 呼出・1,000 行超 11 ファイル・錠呼出 21 箇所）は本レビューの再計測と完全一致した。一方、本番コードに唯一触れる `chain.rs` の並べ替え（④）について「失敗 6 点で状態不変」という主張が寸法変更経路で成り立たず、正準イベント型が `record_str` の生値を失う点、settle の Tick 範囲が壁時計依存になる点の 3 件は tasks 前に設計で決着させる必要がある。

## 2. 現行ツリーとの突合（主要な検証結果）

| 検証項目 | 結果 |
|---|---|
| `chain.rs` `upload` :185-241・struct :122・`read_back` :243・in-file `mod tests` :297 | 一致（現行順序は `ResizeBuffers`→`source_tex`→`staging`→`size`→`UpdateSubresource`→`GetBuffer`→cast→`Present`） |
| `spine.rs` `SPIN_WAIT` :329・`spin_wait_until` :358・module doc :20-21・`capture_logs` :506-540 | 一致 |
| `spine_display_tests.rs:410-414`・`spine_seriko_loop_tests.rs:372-375` | 一致（設計 Modified Files の「:371-375」は 1 行ずれ・軽微） |
| probe 8 コピー／keeper 3 コピー／capture-all 2 コピーの file:line | 一致（`ghost/...global_log_probe.rs` は :67-93 が実体・軽微） |
| `transition_diag.rs:622-623` `is_enabled`・`dpi_sync.rs:279`・`show.rs:347` の前置ガード | 一致（`tracing::enabled!` の本番利用は `transition_diag.rs:623` の 1 箇所） |
| `show.rs:305-310`（`prev_size`／早期 return）・`target.rs:73`・`transition_record_tests.rs:327-347` | 一致 |
| `command.rs:49` `AtomicI32`・`:76` 錠定義・実呼出 2+5+4+5+5=21 | 一致。`draw-load-parity` は brief.md のみ（未着手）＝要件 7.1 分岐が現実 |
| 1,000 行超 11 ファイルの一覧と行数 | 完全一致 |
| `wintf` の dev-deps に `tracing-subscriber`（workspace 定義 `env-filter`）・`areka-emo-present` は dev-deps に無し | 一致（kit の feature 分岐の前提は正しい） |
| 機序差（keeper＝`Registry::enabled` 無条件 true） | **CONFIRMED**: `tracing-subscriber-0.3.23/src/registry/sharded.rs:222-235`（`register_callsite`→`Interest::always()`、`enabled`→`true`）。`tracing-core-0.1.36` `NoSubscriber::register_callsite`→`never`（:676-678）・`has_just_one`（`callsite.rs:551-558`）も設計の説明どおり |
| 番兵（制御イベント）が既存 assert を変えないか | 成立。`capture` は返却前に除去、`capture_under_filter` は target 限定 directive の追加と同 target 行の除去のみ。96 呼出の directive（例 `"info,wintf::ecs::layout=debug"`）と干渉しない |
| 較正の子プロセス方式の決定性 | 成立。別プロセス・`--exact`・`#[ignore]`＋環境変数で親バイナリの実行順に依存しない。親が `1 passed` を要求するので空振りは赤 |
| `command.rs` 非接触・着手条件（改善ループのマージ後・着手時再計測） | 設計冒頭の注記と要件 7.1／2.1／4.1 の対応で明記済み |
| 要件カバレッジ 1.1〜11.6（5.7／5.8／10.1〜10.5 を含む） | トレーサビリティ表で全 ID が C1〜C7 のいずれかに対応。欠落なし |

## 3. 重要指摘（最大 3 件）

### 指摘 1: `upload` の prepare→commit は「寸法変更＋後段失敗」で前状態を保たない（主張の過大）
- **問題**: Flow 3 と C4 State Management は「`Present` 以外の 6 失敗点で `size`／`source_tex`／`staging`／swap chain の 4 つとも不変」と述べるが、設計どおり commit（3 フィールド一括更新）を `ResizeBuffers` 直後に置くと、寸法変更経路で `SourceTexCast`／`GetBuffer`／`BackbufferCast` が失敗したとき `size` は新値・`source_tex` は空の新テクスチャになる（Flow 3 の G/H/I → X「状態不変・read_back 旧内容」は寸法変更時には偽）。`chain_fault_tests.rs` の「7 点 × {外形不変, 外形変更} の意味のある組」で期待値が定義できない。
- **影響**: 要件 5.2／5.7 の実行テストが設計の主張と食い違い、実装段階で「緩めた assert」か「設計違反」のどちらかを選ばされる。
- **提案**: ⒜ commit を `BackbufferCast` の後（`UpdateSubresource` の直前）まで遅らせる——失敗し得る操作は `UpdateSubresource` より前にすべて終わるので、`Present` 以外の 6 点で struct の 4 フィールドは自己整合のまま旧値（`read_back` は旧内容・旧寸）になる。swap chain だけは `ResizeBuffers` 後に戻せないが、次回 `upload` は `self.size` 不一致で再度 `ResizeBuffers` を通るため回復する。⒝ 「`ResizeBuffers` 成功後の後段失敗は表示面（backbuffer）が未定義」を `Present` と同じ残余として登記し、テストの期待値を（失敗点 × 経路）ごとに表で固定する。
- **Traceability**: 5.2, 5.7, 5.8, 5.5 / **Evidence**: design.md「Flow 3」「C4 State Management」「C4 Validation」、research.md §9.6

### 指摘 2: 正準イベント型が `record_str` の生値を落とし、keeper 3 crate のアダプタが成立しない
- **問題**: `CapturedEvent.fields` は「(名前, Debug 表現)」のみを持つ。ところが `areka-kanade/src/schedule/log_capture.rs:82-92` と `areka-ghost/src/test_log_capture.rs:66-72` は `record_str` で `event`／`outcome` の**生文字列**（引用符なし）を拾い、`assert_logged` は `e.event == event_name` の完全一致で判定する。設計 C2 の「`field("event")`／`field("outcome")` から組み立て」では Debug 表現（`"\"name\""`）が返るため、アダプタ側で引用符剥がしとエスケープ解除を再実装することになる（research §9.3 はこの差を見落としている）。
- **影響**: 要件 2.3（判定内容不変）・6.1 に対し、最も壊れやすい箇所（文字列の再解釈）を各 crate に複製する形になり、kit の「正準型」が正準でなくなる。
- **提案**: `CapturedEvent` の値を `FieldValue { debug: String, str_raw: Option<String> }`（訪問順保持）にし、`field(name)` は Debug 表現・`field_str(name)` は `record_str` の生値を返す。行整形の byte 一致は `debug` 側で保つ。kit の自己テストに「`event = "x"` を `field_str` で `x` として取り出せる」を 1 本足す。
- **Traceability**: 2.3, 6.1, 1.7 / **Evidence**: design.md「C1 Service Interface」「C2 Responsibilities（型対応）」

### 指摘 3: Tick を兼ねる settle で「注入時刻の範囲は旧テストと同一」が保証されない
- **問題**: `settle_bounded` は最小持続 200ms かつ連続 50 回空で返り、各反復に短い sleep を挟む。`spine_display_tests.rs:410` の呼出形では実際に注入される Tick は到達した反復数ぶん（目安 200 前後）で止まり、旧ループの 5,000 Tick（模擬時間 5 秒）を踏むかは壁時計次第になる。頭打ちで「追い越さない」は守られるが、「範囲同一」は成り立たず、負検証が踏む模擬時間が環境依存で縮む。
- **影響**: 要件 4.3／4.5（回収機会が負荷で縮まない）・6.1（主張を弱めない）の趣旨に反する余地。
- **提案**: Tick 注入は決定論的な前段として旧範囲 `1_000_000..1_000_000+5_000` を毎回すべて注入し（各 Tick 後に drain・待機ではない）、その後に `settle_bounded(|| drain のみ)` を置く。これで Tick 生成と打ち切り条件は完全に分離され（4.3）、注入範囲は不変、待機だけが壁時計＋観測量で有界化される。
- **Traceability**: 4.3, 4.5, 6.1 / **Evidence**: design.md「Flow 4」「C3 Service Interface」の `spine_display_tests.rs:410-414` の形

## 4. 設計の強み

- **正典の選定が機序で裏付けられている**: keeper 方式が `Registry::enabled` 無条件 true により本番の前置ガード契約（`transition_diag::is_enabled`）と衝突することを、依存 crate の実コードで根拠付けて排した。研究に基づく採否で、移行後に「どれが正しいか」を再審理しなくてよい。
- **再発防止が機械化されている**: 直接呼出検知・1,000 行番人・dev-deps-only の 3 番人を kit の統合テストに置き、いずれも既知陽性で赤になる自己較正を持つ。例外表の追加を明示編集に限定しているため、後続 spec が黙って規律を崩せない。

## 5. 最終判定

- **判定: GO（条件付き）**
- **理由**: アーキテクチャ整合・依存方向・要件カバレッジは十分で、3 件の指摘はいずれも設計の内側（`chain.rs` 内の commit 位置、kit の型、settle 呼出形）で閉じる修正であり、他 spec へ波及しない。
- **次の手順**: 設計ディスカッションで指摘 1〜3 を裁定し design.md に反映（Flow 3・C4 Validation の期待値表、C1 の `FieldValue`、Flow 4／C3 の呼出形）した後、`/kiro-spec-tasks areka-P0-test-cage-determinism` へ進む。軽微な行ずれ（`spine_seriko_loop_tests.rs:371→372`・`global_log_probe.rs:60→67`）は着手時の全面再計測で吸収してよい。
