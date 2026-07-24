# 設計バリデーションレポート: areka-P0-emo-dpi-scaling

> **実施日**: 2026-07-24（kiro-validate-design・非対話モード）
> **入力**: design.md／requirements.md（確定）／research.md（§7 設計追記込み）／steering（product・tech・structure・roadmap 追記㊵㊹㊺）
> **検証方法**: design-review.md 4 基準（既存アーキ整合・一貫性・拡張性/保守性・型安全/契約）＋主要コードアンカーの当日独立再実測

## 検証済みアンカー（レビュー根拠の実測）

- `CURRENT_COMPOSE_SCALE`（presenter.rs:126・唯一代入 :435）— 実在確認。
- `measure_scope_sizes`（measure.rs:62）・`resize_window_to`（follow.rs:553）・私有 `enqueue_window_set_pos`（follow.rs:729）— 実在確認（設計の「薄い公開ラッパが唯一の正規手段」主張と整合）。
- `on_gpu_owner_thread`（wintf tests/graphics/common/mod.rs:75）・`enumerate_monitors`／`Monitor`（monitor.rs:68/:173・pub）— 実在確認。
- 依存方向: `crates/areka/Cargo.toml` が emo-compose/emo-present へ既存依存（measure.rs での `ScaleRatio` 消費は逆流なし）。
- emo-present in-crate GPU テスト: `make_world_with_gpu` 型テストが同一バイナリに 14+ 本既在・現状緑（D9 の実証基盤）。

## Review Summary

要件ディスカッション裁定（2 因子モデル）と research.md の未知 8 件を D1〜D10 で全消化し、全 39 AC のトレーサビリティ表・失敗分岐×ログ×縮退の対応表・テスト配置基準（R5.4）まで備えた、実装可能性の高い設計である。Strategy A2（合成 native 不触・present 段リサンプル）は既存の「composed 外形従属」連鎖を k 追従へ転用する筋の良い選択で、主要アンカーは本レビューで独立再実測し全一致した。以下 3 点はいずれもアーキテクチャの根本欠陥ではなく、設計ディスカッション／タスク化で潰すべき精緻化事項である。

## Critical Issues（≤3）

🔴 **Critical Issue 1**: W4 事前割当契約からの編集面逸脱（follow.rs ほか）の裁可が未取得
**Concern**: 契約（roadmap 追記㊵: dpi＝`measure.rs`＋emo-atlas/compose/present＋wintf 限定）に対し、設計は `source.rs`・`follow.rs`・`emo2_boot/`・`main.rs` への additive 増分を要すると自己申告している。特に `follow.rs` は並走 `position-persist` の観測点（DragEnd :319-350,443-488・roadmap 実測）を含む pos⇄van ホットスポットであり、W4 同一ウェーブでの同ファイル追記は契約の実質改訂にあたる。
**Impact**: 並走 spec との merge 衝突・「少しでも干渉するならウェーブを分ける」規律との緊張。逸脱が黙認のまま実装に入ると W4 の同居根拠が崩れる。
**Suggestion**: 設計ディスカッションで開発者が Boundary Deviation Notes（4 項目）を明示裁可し、roadmap の契約行へ追記反映する（不裁可なら R7.7 準用で該当部を W5 送り。`resize_window_keep_position` は関数追加位置を position-persist の観測域から離すことを実装注記に）。
**Traceability**: R7.6／R7.7・Adjacent expectations（W4 事前割当契約）
**Evidence**: design.md「Boundary Deviation Notes」・File Structure Plan（Modified Files）

🔴 **Critical Issue 2**: 初期 k₀≠実窓 k の補正が Changed<DPI> のエッジ観測に依存し、初回 show 前に消費されると窓寸不整合が残置し得る
**Concern**: Flow 3 手順 5 は「k≠k₀ なら Flow 2 の reconcile が補正」とするが、`refresh_scale` は `last_show` 保持時のみ再表示・reconcile する。窓生成時の `GetDpiForWindow` 補正による `Changed<DPI>` が初回 ShowSurface（drain フェーズ）より先に `run_dpi_phase` で観測・消費されると、`last_show=None` で None が返り、以後トリガ不在のまま「表示は k 寸・窓 client は k₀ 寸」の不一致（R3.1 違反＝見切れ/余白）が固定化し得る。非 primary モニタ起動＝混在 DPI はまさに本仕様の主戦場である。
**Impact**: R3.1/R4.2 の「窓 client＝round(k×原寸)」保証が起動順序に依存する偶発性を持ち、実機 2 水準サインオフ（R6）で再現困難な失敗になり得る。
**Suggestion**: reconcile 条件をエッジ（Changed）純依存から状態照合併用へ——例: `apply_show` 表示成立点で「今回 scaled 寸が前回適用寸と異なる」場合に新物理寸を返し（または frame 側で `窓 client ≠ scaled_extent(applied, native)` を照合し）、初回 show 直後にも窓寸 reconcile を走らせる。べき等 skip があるため常時照合でも振動しない。
**Traceability**: R3.1／R4.1／R4.2／R1.5
**Evidence**: design.md Flow 2 キー決定(d)・Flow 3 手順 5・`refresh_scale` シグネチャ（presenter.rs 節）

🔴 **Critical Issue 3**: D9（GPU 檻の emo-present in-crate 配置）の安全根拠が roadmap 追記㊺の文言と要整合
**Concern**: roadmap 追記㊺は「新規 WUC(Compositor) 生成テストは必ず `on_gpu_owner_thread` 経由（素の別スレッド生成は AV）」と包括的に述べる一方、設計 D9 は emo-present in-crate（fixture 非経由）へ新設する。根拠「別プロセスゆえ構造的に無縁」はバイナリ間には正しいが、同一テストバイナリ内の並列スレッド Compositor 生成という真のリスク軸には触れていない（実測では同型テスト 14+ 本が現状緑＝経験的には安全）。
**Impact**: R5.3（AV 非再導入）の論拠が文書上不完全なまま実装に入ると、テスト増分後の偶発 AV 時に配置判断へ立ち戻ることになる。
**Suggestion**: 設計ディスカッションで「㊺の fixture 必須は wintf tests/graphics（同一プロセス集約ターゲット）にスコープされる」ことを確認・記録し、D9 の根拠へ「emo-present テストバイナリは並列スレッド Compositor 生成の既存実績 14+ 本緑」という経験的基盤を明記する（タスクに既存 GPU テスト群の事前フル実行を含める）。
**Traceability**: R5.3／R5.4
**Evidence**: design.md Testing Strategy 冒頭（振り分け基準）・D9・research.md §1.5 注意書き

## Design Strengths

1. **Strategy A2 の資産転用設計**: chain.rs の外形自動追従（:178-194）と挿入時マスク生成という実測済みの既存連鎖に k 適用済みサーフェスを流すだけで swapchain/visual/マスク/照会が一貫追従する構成は、変更面を最小化しつつ AlphaMask 物理 px 契約を無修正で保つ。W5 ÷k の観測条件（合流ゲート）を最短で開通させる。
2. **`ScaleRatio`（既約有理）による決定性の構造的担保**: f32 を画素経路から排除して blit の整数規約と両立させ、cache キー等価・丸め単一権威 `scaled_extent`・k=1/1 恒等バイトコピー（既存 golden 不変＝R7.2 の錨）までを単一型で貫徹しており、将来のアプリ管理拡大率因子も乗算シームで温存されている（2 因子モデル裁定準拠）。

## Final Assessment

**Decision: GO**

**Rationale**: 既存アーキテクチャとの整合（依存方向・スレッド親和・log-first・spawn.rs 不触）は実測レベルで裏付けられ、全要件が具体的コンポーネントと流れに割り当て済み。3 件の指摘はいずれも契約裁可・条件精緻化・根拠明文化であり、設計ディスカッションとタスク化で吸収可能な受容可能リスクの範囲内。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1〜3 を裁定（特に Issue 1 の契約改訂裁可と Issue 2 の reconcile 条件の設計反映）。
2. 裁定結果を design.md へ反映後、`/kiro-spec-tasks areka-P0-emo-dpi-scaling` でタスク生成へ進む。
