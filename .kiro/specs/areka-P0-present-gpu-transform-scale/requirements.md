# Requirements Document

## Introduction

### 誰が困っているか

areka でゴースト `emo2` を **拡大率 200%（k=2）** のモニタで動かす利用者。立ち絵の絵が変わるコマ（表情・まばたき・着せ替え）のたびに画面が止まり、文字が「10 字くらい急に表示」される・「初回起動で文字が出てこない」ように見える。完了 spec `areka-P0-emo2-conformance-e2e` の実機一周走行で症状 E として登記され（`verification/acceptance-record.md` §13.2 行 9・同 `requirements.md`「改訂（2026-09-07・第 4 回）」2）、M1 の完成判定には載せず本仕様が引受先となった。M1 の完成判定は本仕様を待たない（同 spec `verification/m1-completion.md`）。

### いま何が起きているか（本ブランチで実測・2026-09-11・HEAD `67e0a4d3`）

- 提示段 `EmoPresenter::apply_show`（`crates/areka-emo-present/src/presenter/show.rs`）は、引き当て外れのたびに native 原寸で合成したあと、k≠1 なら **CPU で k 倍にリサンプル**し（`FrameBudget::resample_native_into` → `areka_emo_compose::scale::resample_with`・整数固定小数点 bilinear）、その **k 倍バイト列から当たり判定マスクを再生成**し（`FrameBudget::regenerate_mask` → `AlphaMask::regenerate_from_pbgra32`）、k 倍の面を供給面へ上げている（`SwapChainPresenter::upload`）。
- 供給面・visual（`VisualMount::attach`／`set_bounds` の `SpriteVisual.SetSize`）・当たり判定マスクはすべて **k 倍後の物理寸**で揃っている。visual に拡大の変換は一切与えていない（`crates/wintf/src` に `SetScale`／`SetTransformMatrix`／brush の `SetStretch` の使用は 0 件・brush の伸縮は既定の等倍のまま）。
- 合成メモ `ComposeCache`（`crates/areka-emo-present/src/cache.rs`）は **k をキー要素**に持ち（`ComposeKey.scale`）、エントリは **k 適用済みの表示面＋その k 寸バイト由来のマスク＋原寸**（`CacheEntry{composed, mask, native}`）である。容量 3・LRU。
- 実測（症状 E）: k=2 の外れ 1 回＝compose 5〜7 ＋ resample 44〜61 ＋ mask 11 ＋ upload 0.3 ms（静かな機械）。別プロセスが CPU を取ると全段が約 5 倍。処理は UI スレッド上で同期に走るため、絶対時刻の台本で再生している文字が止まった分だけ次のコマでまとめて出る。ティッカーの `catch-up`（`crates/areka-ghost/src/ticker.rs`「ticker catch-up: skipped multiple boundaries, firing once」）が前回 10 → 60 件。
- k=1 は恒等（`ScaleRatio::is_identity` で席の交代のみ・複写もリサンプルも無し）ゆえ 100% 表示では無料——**「最初は重くなかった」のは k=1 だったから**。
- この形は完了 spec `areka-P0-emo-dpi-scaling`（PR #91）の設計判断 **D3（Strategy A2＝present 段で CPU リサンプル）／D5（整数 bilinear・「GPU stretch は決定論 readback の檻と不整合」で却下）／D6（cache エントリ＝k 倍後の面＋その bytes 由来のマスク）** の帰結である。D3 が GPU 変換を却下した理由は現在成り立たない: ⑴ 当たり判定は完了 spec `areka-P0-collision-dpi-hittest` が **点 ÷k で native 座標へ縮約**する形で着地し（`EmoPresenter::hit_region_client` → `hit_region_scaled`／`ScaleRatio::unscale_coord`）、k 倍のマスクを領域判定に使っていない。⑵ 窓のクリック透過が読む α マスクの照会（`crates/wintf/src/ecs/layout/hit_test/mod.rs` `alpha_mask_hit`）は **窓境界に対する比例写像**（`rel_x = (point.x − bounds.left) / bounds_width`・`mask_x = rel_x × mask.width`）であり、境界が物理寸・マスクが原寸なら ÷k は写像が吸収する。⑶ 鮮明性は提示側の補間モードで選べる。⑷ k≠1 の golden は present 段の CPU 面を読み戻しているだけで、**k=1 の合成 golden を正**とすれば決定論は失われない。
- バルーンは重くない（文字はベクタ描画・バルーン画像の外れは走行 1 回で 1 度）。文字層の見え方の問題は UI スレッド停止の帰結である。

### 何を変えるか

開発者裁定（2026-09-11・`/kiro-discovery`）: **「画像本体は原寸で持ち、常に D2D の変換行列指定による拡大縮小とする。画像を CPU で拡大しているなら許容できない」**。本仕様は提示段の CPU 拡大経路（k 倍リサンプル・k 倍バイト列からのマスク生成・k 倍面の保持）を**撤去**し、原寸の面を上げて拡大率 k を提示側の変換として与える。当たり判定（÷k）は不変、窓寸・配置は不変、k=1 の合成 golden はバイト単位で不変。k≠1 の見た目は実機 2 水準サインオフへ移し、既存の k 付き golden は再導出か撤去を裁定する。D3／D5／D6 を正式に覆し、裁量記録（`doc/COMPAT_ARCHITECTURE.md` §8・プロジェクト記憶）へ登記する。

## Boundary Context

- **In scope**（利用者・運用者から見える範囲）:
  - 提示段の CPU 拡大経路の撤去（開発者裁定・ハード制約）。画像本体は原寸で保持し、拡大縮小は提示側の変換（行列）でのみ行う。
  - k≠1 での表示の見た目（原寸画像を k 倍で表示・物理寸は従来と同じ丸め権威で一致）・DPI 変化への追従・k=2 での 1 コマ予算。
  - クリック透過用 α マスクの原寸化と、窓側の照会が ÷k で原寸を引くこと（結果の意味論は「原寸 α を ÷k した点で読む」）。
  - 合成メモのエントリの原寸化（k を保持内容から外す）。
  - 決定論: k=1 の合成 golden 不変・k≠1 を読む既存テストの裁定（再導出か撤去）・新設分岐の決定論テスト・実機 2 水準サインオフ。
  - `perf(apply_show)` の段階別計時の維持と、撤去した段の扱い。
  - 設計判断 D3／D5／D6 の上書き登記（完了 spec `areka-P0-emo-dpi-scaling` への追記・`doc/COMPAT_ARCHITECTURE.md` §8・プロジェクト記憶の該当項目）。
  - 本仕様の適用範囲は提示段が扱う **全 target（キャラ窓・バルーン窓）** である。バルーン画像も同じ漏斗を通るため撤去の対象に含まれる。
- **Out of scope**（本仕様は触らない・ゼロを明示）:
  - 合成の規約（`areka-emo-compose` の `plan`／`blit`・native 整数合成）: **変更 0**。
  - 文字層（`areka-emo-text`）の描画・供給面・行送り・折返し: **変更 0**（文字はベクタ描画で本仕様の対象外。完了 spec `emo-text-line-height-canon` の裁定はそのまま）。
  - k の**政策**（`crates/areka-emo-present/src/scale.rs` `ScalePolicy`／`derive_scale`・author_dpi・縮退）: **変更 0**。
  - k の**数学**（`ScaleRatio`・`scaled_extent`・`scale_len`・`unscale_coord` の丸め権威）: **変更 0**。寸法の権威は残す（撤去するのは k 倍リサンプルの経路のみ）。
  - 当たり判定の領域解決（完了 spec `collision-dpi-hittest`・点 ÷k）とバルーン窓のヒット経路（矩形 ×k・点は無変換）: **変更 0**。
  - 窓の寸法・配置・DPI 変化時の窓寸 reconcile（`take_pending_resize`／`target_physical_size` を消費する `crates/areka/src/emo2_boot/frame/drain_resnap.rs`）: **利用者から見える結果は変更 0**（物理寸の照会値は従来どおり `scaled_extent(native)`）。
  - 機械負荷（別プロセス）の管理: 本仕様の対象外。
  - 拡大率遷移の 2 ティックの跳ね: `areka-P0-dpi-transition-two-tick-bounce`（W14・本仕様の後に再計測）。
  - 合成メモの容量（3）と置換方式（LRU）: **変更 0**（外れの代金が消えるため容量増は採らない・brief ⓒ 却下）。
  - CPU リサンプルの高速化（brief ⓐ）・別スレッド化（brief ⓓ）: **採らない**。
- **Adjacent expectations**:
  - **Upstream**: 完了 spec `areka-P0-emo-dpi-scaling`（D3／D5／D6 を本仕様が覆す・D1／D2／D4／D7／D8 は不変）・`areka-P0-collision-dpi-hittest`（÷k・不変）・`areka-P0-recompose-budget`（計時の型と予算席・`FrameBudget` の席のうちリサンプル席は撤去対象）・`areka-P0-draw-load-parity`。
  - **Downstream**: `areka-P0-emo2-conformance-e2e`（症状 E の登記の消化・完成判定は待たない）・`areka-P0-dpi-transition-two-tick-bounce`（外れの代金が消えると遅れの量が変わる＝本仕様の後に採り直す）・`areka-P0-balloon-canon-residue`（W14・`areka-emo-present` を本仕様の後に触る）・`tick-gate-adoption`。
  - **正典参照**: 本仕様は areka 内部の描画アーキテクチャの仕様であり、ukadoc に対応する条文は無い（ukadoc 参照 0 件・捏造しない）。
  - **開発者方針**: SSP の画素実測は根拠にしない（目視証跡＝スクリーンショットは根拠に使える）。

## Requirements

### Requirement 1: CPU 拡大経路の撤去（開発者裁定・ハード制約）

**Objective:** 開発者として、画像本体を原寸で持ち拡大縮小を常に提示側の変換行列で行う形へ提示段を改め、CPU で画像を拡大する経路を残さないことを求める（2026-09-11 裁定「画像を CPU で拡大しているなら許容できない」）。それにより、絵が変わるコマの代金から k 倍リサンプルとマスク再生成が消える。

#### Acceptance Criteria

1. The 提示段 shall 表示のために保持・供給する画像本体（合成結果・供給面へ上げるバイト列・合成メモのエントリ）を **native 原寸**のみとし、k の値に依らず k 倍のバイト列を作らない。
2. The 提示段 shall 拡大率 k を**提示側の変換（行列の係数）としてのみ**現し、CPU で画素を書き換える拡大縮小を k のいかなる値でも行わない。
3. When 引き当て外れで合成が成功した, the 提示段 shall 合成結果を原寸のまま供給面へ上げ、リサンプルの段を経由しない（k=1 と k≠1 で通る手順が同じであること）。
4. The 本仕様 shall 提示段の CPU リサンプル経路（`FrameBudget::resample_native_into`・そのリサンプル作業席・`areka_emo_compose::scale::resample`／`resample_with`／`ResampleScratch` の本番消費）を撤去し、本番経路に消費者の無い k 倍リサンプルの実装を残さない（撤去後に `resample` 系の消費者が 0 件であることを機械で確認する）。
5. The 本仕様 shall `ScaleRatio` とその寸法権威（`scaled_extent`／`scale_len`／`unscale_coord`）を残し、撤去の対象を k 倍リサンプルの経路に限る（寸法の権威の変更 0）。
6. The 本仕様 shall CPU リサンプルの高速化・別スレッド化・合成メモの容量増を**採らない**ことを設計で明記する（brief ⓐ／ⓒ／ⓓ 却下）。
7. If 実装後に提示段のいずれかの経路が k≠1 で CPU 拡大を行っていることが見つかった, then the 本仕様 shall それを完了の阻却事由とし、最適化で薄めず撤去する。

### Requirement 2: k≠1 での表示の見た目と物理寸の一致

**Objective:** 利用者として、拡大率 k≠1（125%／200% など）のモニタでも、立ち絵とバルーンがこれまでと同じ大きさ・同じ位置に表示され、DPI が変わればその場で追従することを求める。それにより、CPU 拡大の撤去が見た目の退行にならない。

#### Acceptance Criteria

1. While 拡大率 k≠1 で表示している, the 提示段 shall 原寸 (w, h) の画像を画面上で `ScaleRatio::scaled_extent(w, h)`（round half away from zero・非ゼロ入力は最小 1px）と同じ物理寸で表示する（丸め権威は従来と同一）。
2. The 提示段 shall 表示物理寸の照会値（`TextSlotView::physical_size`・`EmoPresenter::target_physical_size`）を従来どおり `scaled_extent(applied, native)` で返し、窓寸 reconcile の呼び手（`drain_resnap.rs`）から見える値を変えない。
3. The 提示段 shall 窓の寸法・配置・バルーンのオフセット・キャラ窓の原点（下端中央）を本仕様の前後で変えない（**変更 0**・実機 2 水準の目視で確認）。
4. When 窓の DPI が変わり k が再導出された, the 提示段 shall 同一フレーム内で新しい k の表示を成立させ、表示物理寸が変わった場合は従来どおり窓寸 reconcile 要求（`take_pending_resize`）を積む。
5. While 拡大率 k≠1 で表示している, the 提示段 shall 要素間の相対配置・重なり（element 入れ子・SERIKO パターン・着せ替え）を等倍時と同じ見た目関係に保つ（合成済みの 1 枚へ単一の k を掛ける形は従来と同じ）。
6. The 提示段 shall 拡大時の補間（bilinear 相当以上の品質）で原寸画像を表示し、最近傍拡大による画素幅ムラを出さない。補間モードの選択は設計フェーズで確定する。
7. The 本仕様 shall 実機 2 水準（125%／200%）で本番ゴースト `emo2` を有界 auto-exit（`AREKA_APP_SMOKE_EXIT_MS`）で起動し、表示成立点の `info!` 行（「apply(ShowSurface): 表示・マスクを更新」の `k_ratio`／`native_w`／`native_h`／`scaled_w`／`scaled_h`）と目視証跡（スクリーンショット）で、2 水準が互いに異なる物理寸で・崩れなく描かれたことをサインオフする。
8. The 表示成立点の `info!` 行 shall `scaled_w`／`scaled_h` に**物理寸**を出し続ける（供給面が原寸になっても値の意味を変えない・実機サインオフの grep 契約を壊さない）。

### Requirement 3: 絵が変わるコマの 1 コマ予算（k=2）

**Objective:** 利用者として、拡大率 200% でも絵が変わるコマで画面が止まらず、文字がまとめて出ないことを求める。それにより、SSP と同様に拡大率に依らず滑らかに再生される。

#### Acceptance Criteria

1. While 拡大率 k=2 で表示している, when 絵が変わるコマ（引き当て外れ）が来た, the 提示段 shall その適用の UI スレッド処理（compose ＋ mask ＋ upload の合計・`t_total_us`）を **16.7 ms 以下**に収める（静かな機械・期待値 compose 1〜7 ms ＋ mask 数 ms ＋ upload 0.3 ms・ギャップ分析の見込み 8〜12 ms）。合否の物差しは brief の 16.7 ms（1 コマ）であり、完了 spec `emo2-conformance-e2e` `verification/acceptance-record.md` §13.2 行 9 の「目標 16 ms・許容 30 ms」はこれに置き換わる（記録は改変しない）。別プロセスが CPU を取る負荷下の値は Requirement 3.5 で報告するが合否に載せない（brief Out of Boundary「機械負荷の管理」）。
2. While 拡大率 k=2 で起動挨拶を再生している, the 実機走行 shall ティッカーの `catch-up`（「ticker catch-up: skipped multiple boundaries, firing once」）を **3 分で 10 件以下**に収める（前回並み）。
3. The 提示段 shall 段階別計時 `perf(apply_show)`（`PERF_LINE_MESSAGE`＝「perf(apply_show): 段階別計時」）の発行と、その既存消費者（`tools/perf/judge-perf.py`・`J_PERF_LINE_MESSAGE` で同じ文言を照合）が読める形を維持する。
4. The 提示段 shall 撤去した段（`Stage::Resample`・`t_resample_us`・`alloc_resample_dst`・`alloc_xmap`）について、フィールドを残すなら常に 0 を出し、外すなら消費者（`judge-perf.py` の `J_PERF_STAGE_FIELDS`／`J_PERF_ALLOC_FIELDS`＝`J_PERF_REQUIRED_FIELDS` の構成要素）を同時に更新し、`python tools/perf/judge-perf.py --selftest` が緑のままであること（`tools/perf/fixtures` の旧スキーマ行は余分なフィールドとして無害）を機械で確認する。どちらを採るかは設計で確定する。
5. The 本仕様 shall k=2 の走行で段階別計時の分布（中央値・p90）を採り、症状 E の前回値（中央値 41〜78 ms・p90 65〜227 ms）と並べて報告する。
6. The 提示段 shall 定常状態（寸法不変・初回確保後）の毎コマ経路で新規確保 0 を保つ（`recompose-budget` の予算席の規律を崩さない・撤去した席の分だけ席が減る）。

### Requirement 4: クリック透過と当たり判定（原寸 α マスク・÷k）

**Objective:** 利用者として、拡大率 k≠1 でも立ち絵の透明な所ではクリックが後ろへ抜け、絵のある所ではつかめて、頭・胸などの当たり判定がこれまでどおり当たることを求める。それにより、マスクを原寸に戻しても操作感が変わらない。

#### Acceptance Criteria

1. The 提示段 shall クリック透過用の α マスクを **native 原寸の合成バイト列**から 1 回だけ生成し（閾値は従来の `ALPHA_THRESHOLD`＝128）、k 倍バイト列由来のマスクを作らない。
2. While 拡大率 k≠1 で表示している, when 窓 client 物理 px の点でクリック透過の判定が行われた, the 判定 shall その点を **÷k した原寸座標の α** で判定する（結果の意味論＝「原寸 α を ÷k した点で読む」）。窓境界に対する比例写像（`alpha_mask_hit`）が原寸マスクと物理寸境界で自然にこれを与えるか、別の経路で与えるかは設計で確定する。
3. The 本仕様 shall 従来の「k 倍 bilinear 面から作ったマスク」と境界画素 1px 以内で異なる判定結果を**退行とみなさない**ことを明記する（原寸 α の ÷k 照会が新しい正典・完了 spec `collision-dpi-hittest` の点 ÷k と同じ規約）。
4. The 提示段 shall 表示バッファとマスクの**原子対**（同一 `apply` 呼び出し内で更新・同一バイト列由来）を維持する。
5. The 本仕様 shall 領域の当たり判定（`EmoPresenter::hit_region_client`・点 ÷k・`ScaleRatio::unscale_coord` の丸め権威）を変えない（**変更 0**）。
6. The 本仕様 shall バルーン窓のヒット経路（行矩形を ×k・点は無変換）に座標のスケール変換を追加しない（**変更 0**・二重縮約の禁止は `collision-dpi-hittest` Requirement 6.4 のまま）。
7. The 本仕様 shall クリック透過の判定経路（`crates/wintf/src/ecs/clickthrough/controller.rs` `evaluate_targets` → `hit_test_in_window` → `alpha_mask_hit`）に ÷k を**二重に**掛けないことを決定論テストで固定する（原寸マスク×物理寸境界で 1 回だけ縮約される）。
8. The 本仕様 shall 実機 2 水準で、透明部のクリック抜け・不透明部のつかみ・頭／胸の当たり判定をサインオフする（`hit_region_client` の `debug!` 行「[hit_region_client] client 物理 px を ÷k して当たり判定を解決」の grep ＋目視）。

### Requirement 5: 合成メモと DPI 変化（エントリの原寸化）

**Objective:** 運用者として、合成メモが原寸の面だけを持ち、DPI が変わっても再合成を要しないことを求める。それにより、外れの代金が消えるだけでなく k 変化の代金も消え、メモリも小さくなる。

#### Acceptance Criteria

1. The 合成メモ shall エントリに **原寸の合成面＋その原寸バイト由来のマスク**だけを保持し、k 適用済みの面を保持しない（`CacheEntry` から k 寸の内容が消える）。
2. The 合成メモ shall 引き当てのキーを合成入力（surface id ＋ bind 集合 ＋ pattern 状態）とし、**k をキー要素から外す**（D6 の「k 変化＝ミス→再合成＋再サンプル」を覆す）。
3. When 窓の DPI が変わり k だけが変わった（合成入力は同じ）, the 提示段 shall 再合成せずに引き当てを成立させ（`info!` 行の `cache_hit=true`）、新しい k の表示を成立させる。
4. The 合成メモ shall 「同一入力なら再利用・1 ビットでも異なれば必ずミス」（surface id・binds・pattern）と、容量 3・LRU（`CAPACITY`・`touch`／`get`／`take_recycled`／`invalidate_all` の意味論）を変えない（**変更 0**）。
5. The 提示段 shall 1 target あたりの合成メモの保持量を k≠1 で従来（k=2 で約 10.3MB＝表示寸 764×1094×4×3）より小さくする（原寸 3 件ぶん）。
6. The 提示段 shall 「k 変化後に旧 k の絵が表示に載らない」ことを、エントリが k を持たない構造（表示の k は変換の係数であって面ではない）で担保する。

### Requirement 6: 決定論と既存テストの裁定

**Objective:** 開発者として、k=1 の合成 golden をバイト単位で不変に保ち、k≠1 を読んでいた既存テストの扱いを裁定で確定し、新設した分岐を決定論テストで固定することを求める（決定論的テスト網羅は必達）。

#### Acceptance Criteria

1. The 本仕様 shall k=1（恒等）の合成 golden（`crates/areka-emo-compose/src/golden_tests*.rs`・`crates/areka-emo-present` の k=1 の `read_back` 比較・`crates/areka/examples/emo-present/reconcile.rs` の k=1 の golden）を**バイト単位で不変**に保つ。
2. The 本仕様 shall `read_back`（`EmoPresenter::read_back` → `SwapChainPresenter::read_back`）が返すバイト列の意味を「供給面＝原寸」と定義し直し、k=1 では従来と同一バイトであることを固定する。
3. The 本仕様 shall k≠1 を読む既存テスト（付録 A の台帳）について、**再導出**（原寸の面・原寸マスク・物理寸境界という新しい不変条件へ期待値を導き直す）か**撤去**（撤去した経路の純関数テスト＝リサンプラ golden など、対象が消えるもの）かを、テストごとに要件ディスカッションまたは設計ディスカッションで**開発者が裁定**する。本仕様はこれを一方的に決めない。
4. The 本仕様 shall 裁定の材料として付録 A に、ファイルごとの対象テスト名・現在固定している性質・撤去した場合に失う檻・再導出した場合の新しい期待値の方向を並べる。
5. The 本仕様 shall 新設・改変した判断分岐（原寸面の供給・変換の係数の適用・原寸マスクの生成・÷k 照会・k 変化時の引き当て・撤去段の計時の扱い・変換適用失敗の経路）を GPU 非依存の決定論テスト（純関数・偽境界）で固定し、GPU を要する確認はオフスクリーン readback の既存の型に限る。「変換の係数の適用」の檻は、供給面の寸（原寸）と visual／`Arrangement.size`（物理寸＝`scaled_extent`）の整合として固定し、GPU の伸縮結果そのものは檻に入れない（Requirement 6.6）。
6. The 本仕様 shall k≠1 の**見た目**の確認を実機 2 水準サインオフ（Requirement 2.7／4.8）へ移し、k≠1 の画素バイトを決定論 golden で固定しない（GPU 補間の出力は檻に入れない・D5 の却下理由を逆に採る）。
7. The 本仕様 shall 実装で触るファイルを 1,000 行以下に保つ（`show.rs` 486 行・`cache.rs` 378 行・`budget.rs` 657 行・`mount.rs` 462 行・`presenter_perf_log_tests.rs` **956 行（残 44 行・再導出で増やさない）**・`crates/areka-emo-compose/src/scale.rs` 603 行（撤去で縮む）・2026-09-11 実測）。既に例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs` `OVER_LIMIT_ALLOWED`）に載る `cache_tests.rs`（1,618 行）・`presenter/budget_tests.rs`（1,081 行）は、触る場合に行数を増やさない。
8. If 撤去により消費者が 0 になったテスト補助（`FrameBudget` のリサンプル席の観測口・`alloc_resample_dst`／`alloc_xmap` の計数）が残った, then the 本仕様 shall それを陳腐化テストとして除外し、壊れたまま残さない。

### Requirement 7: 失敗経路とログ

**Objective:** 運用者として、変換の適用や供給に失敗したときにログ無しで縮退しないこと、成功時のログの意味が変わらないことを求める。それにより、実機サインオフと障害調査がこれまでの grep 契約のまま行える。

#### Acceptance Criteria

1. If 提示側の変換（拡大率 k の適用）の設定に失敗した, then the 提示段 shall `error!` を出して `Err`（`PresentError::Device` 相当）を返し、表示は適用前の状態（前 k・前表示）を保つ。
2. If 原寸面の供給（upload）に失敗した, then the 提示段 shall 従来どおり `error!` ＋ `Err` とし、表示は前状態を保つ（**変更 0**）。
3. The 提示段 shall k=1 と k≠1 で失敗経路の分岐を増やさない（変換の適用は k の値に依らず同じ手順で行い、恒等 k だけを特別扱いする分岐を新設しない）。
4. The 提示段 shall panic を致命（内部不変条件の破れ）に限り、変換・供給・マスクの失敗で panic しない。
5. The 表示成立点の `info!` 行 shall フィールド（`target_id`／`surface_id`／`cache_hit`／`k_ratio`／`k`／`author_dpi`／`window_dpi`／`native_w`／`native_h`／`scaled_w`／`scaled_h`／`size_changed`）の名と意味を変えない。
6. The 遷移観測（`transition_diag` の `surface` 行・`resized`）shall 供給面が原寸になることで `resized` の意味が「原寸の外形が変わった回」になることを明記し、判定側（`transition_judge`）が k 変化を「供給面のリサイズ」として期待していないことを確認する（期待していれば追随する）。2026-09-11 grep: `crates/areka/src/placement/transition_judge.rs`／`transition_judge_offset.rs`／`transition_judge_verdict.rs` に `resized` の読み手は 0 件＝**該当 0**。同ディレクトリのテストの `resized=true` は実機ログの写し（fixture）であり判定器は読まないので追随不要。追随は present 側テスト `a_scale_change_records_the_buffer_resize`（`presenter/transition_record_tests.rs`）の再導出のみ（付録 A #8）。

### Requirement 8: 設計判断の上書き登記

**Objective:** 保守者として、D3／D5／D6 が覆されたことと新しい正典が、完了 spec・裁量記録・プロジェクト記憶のどれを読んでも同じに見えることを求める。それにより、次の実装者が旧設計を「現行」と誤読しない。

#### Acceptance Criteria

1. The 本仕様 shall 完了 spec `areka-P0-emo-dpi-scaling` の `design.md` の D3／D5／D6 行に「2026-09-11 `areka-P0-present-gpu-transform-scale` が上書き」の追記を置き、D1／D2／D4／D7／D8 は不変であることを併記する。
2. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8（沈黙ルール対応表）へ【上書き】行を 1 行加え、「画像本体は原寸・拡大縮小は提示側の変換行列・CPU 拡大は不可」（開発者裁定 2026-09-11）と出典 spec を記す。
3. The 本仕様 shall プロジェクト記憶の該当項目（`areka-emo-own-compositor-atlas`・`areka-dpi-following-core-design`）に同じ裁定を追記する（メモリの索引 `MEMORY.md` から辿れる形）。
4. The 本仕様 shall `crates/areka-emo-present/src/cache.rs`・`presenter.rs`・`presenter/show.rs`・`presenter/budget.rs`・`crates/areka-emo-compose/src/scale.rs` のモジュール doc に残る「k 適用済み」「リサンプル」の記述を新しい形へ書き換え、旧設計の説明を残さない（doc の主張は file:line で裏取り）。あわせて `crates/wintf/src/ecs/layout/hit_test/mod.rs` の `AlphaMaskResource` doc（「マスク原寸＝bounds 寸」）と `alpha_mask_hit` doc（「bounds==マスク原寸で恒等写像」）を「マスク＝原寸・bounds＝物理寸・比例写像が ÷k を与える」へ書き換える（判定コードは不変・Requirement 9.5）。
5. The 本仕様 shall 完了 spec `areka-P0-collision-dpi-hittest` の点 ÷k 契約が不変であること、および同 spec の設計文書が言及する「k 倍マスク」の記述があれば原寸マスクへ改訂することを確認する（無ければ「該当 0」と記録）。2026-09-11 grep: 同 spec の `design.md`／`requirements.md` に「マスク」「mask」は 0 件＝**該当 0**（`acceptance-record.md`／`brief.md` の `info!` 文言の引用のみ・改訂不要）。
6. The 本仕様 shall 完了 spec `areka-P0-emo2-conformance-e2e` の `acceptance-record.md` §13.2 行 9 の「引受先」欄を本仕様の完了で埋め、`.kiro/steering/roadmap.md` の状態列を `/kiro-complete` で更新する。
7. The 本仕様 shall 完了時に、下流 `areka-P0-dpi-transition-two-tick-bounce` へ「外れの代金が消えたので走行 D 形式で再計測してから設計」を申し送る（roadmap W14 ① の条件のまま）。

### Requirement 9: 非退行と境界（ゼロの明示）

**Objective:** 開発者として、本仕様が触らないものを触らないまま完了することを求める。それにより、並走 spec との干渉と意図しない意味論の変化を防ぐ。

#### Acceptance Criteria

1. The 本仕様 shall 合成の規約（`areka-emo-compose` の `plan.rs`／`blit.rs`・native 整数合成・`compose_into`）を変更しない（**変更 0**）。
2. The 本仕様 shall 文字層（`areka-emo-text`）のコード・供給面寸・`ScaleContract` を変更しない（**変更 0**）。
3. The 本仕様 shall k の政策（`crates/areka-emo-present/src/scale.rs`）と導出のタイミング（show 適用ごと・`refresh_scale` のゲート）を変更しない（**変更 0**）。
4. The 本仕様 shall バルーンのオフセット・DPI 系の完了 spec（`balloon-offset-dpi`・`balloon-vertical-canon`）の裁定を変更しない（**変更 0**）。
5. The 本仕様 shall `wintf` の変更を、提示側の変換の適用に必要な表示レシピの lift（在れば）に限り、DPI 機構・窓生成・クリック透過の判定手順を変更しない（**変更 0**・変更が要る場合は設計で file 単位に列挙する）。Requirement 8.4 の doc 2 行の書き換えは判定手順の変更に当たらない。
6. The 本仕様 shall 並走 W13 の 8 本（`kanade-boot-talkdone-drop`・`host32-window-thread-pump`・`sakura-tag-word-boundary`・`charset-canon`・`ukadoc-coverage-roadmap`・`text-decoration-canon`・`sylphya-set-ledger`・`balloon-font-descript-keys`）と共有ファイル 0 を保つ（roadmap の干渉台帳どおり）。
7. The 本仕様 shall 既存の全テスト（ワークスペース）を緑に保つ。ただし Requirement 6.3 の裁定で撤去・再導出したテストはその裁定の結果に従う。

## 付録 A: k≠1 を読む既存テストの台帳（Requirement 6.3 の裁定材料・2026-09-11 grep）

> 分類の意味: **撤去候補**＝対象（CPU リサンプラ）そのものが消えるので固定する性質が無くなる。**再導出候補**＝固定している性質は残る（原寸面・原寸マスク・物理寸境界へ期待値を導き直す）。**不変**＝本仕様の前後で期待値が変わらない見込み。最終の裁定は開発者。

| # | ファイル | 対象テスト（代表） | いま固定している性質 | 分類（案） |
|---|---|---|---|---|
| 1 | `crates/areka-emo-compose/src/scale_resample_tests.rs`（835 行） | `resample_two_times_matches_golden`・`resample_five_quarters_is_deterministic_golden`・`resample_downscale_matches_oracle_and_golden`・`resample_with_is_byte_equivalent_to_resample` ほか計 16 本 | 整数 bilinear リサンプラの golden・premultiplied 不変・エッジクランプ・作業席の再利用 | 撤去候補（リサンプラごと消える）。`resample_identity_is_byte_copy` 相当の恒等性は k=1 golden が担う |
| 2 | `crates/areka-emo-compose/src/scale_prior_path_tests.rs`（565 行） | `resample_matches_the_frozen_prior_path_byte_for_byte`・`resample_matches_the_frozen_prior_path_at_real_device_extents` ほか計 6 本 | リサンプラの旧経路とのバイト等価（k=2/1・5/4） | 撤去候補 |
| 3 | `crates/areka-emo-present/src/presenter_dpi_scale_tests.rs` | `show_surface_scales_display_to_scaled_extent_at_k2`・`target_physical_size_uses_rounding_authority_and_matches_view_and_chain`・`same_scale_hits_cache_and_window_dpi_change_misses_and_resamples` ほか | 供給面寸＝`scaled_extent`・`chain.size()` と照会値の一致・k 変化＝ミス | 再導出候補（供給面は原寸・物理寸は変換後・k 変化＝ヒット） |
| 4 | `crates/areka-emo-present/src/presenter_fractional_scale_tests.rs` | `alpha_mask_bits_come_from_k_scaled_display_bytes`・`show_surface_scales_display_mask_and_bounds_at_k_five_quarters`・`refresh_scale_shrinks_display_mask_and_bounds_to_smaller_k`・`show_surface_scales_layered_bind_and_pattern_content_with_single_k` | マスクが k 寸バイト由来・マスク寸＝物理寸・k=5/4 のバイト golden | 再導出候補（マスクは原寸・境界は物理寸）。`alpha_mask_bits_come_from_k_scaled_display_bytes` は**新正典と正反対**ゆえ再導出（原寸由来）か撤去 |
| 5 | `crates/areka-emo-present/src/presenter_budget_equivalence_tests.rs` | `the_budget_path_produces_the_same_display_bytes_and_mask_as_a_fresh_buffer_path`・`repeating_the_same_surface_keeps_the_display_bytes_and_mask_equivalent` | 予算席経路と使い捨て経路の表示バイト・マスク等価（期待値を `resample` で作る） | 再導出候補（期待値の生成に `resample` を使えなくなる） |
| 6 | `crates/areka-emo-present/src/presenter_budget_steady_state_tests.rs`・`presenter/budget_tests.rs` | `a_steady_scaled_run_reuses_every_buffer_and_allocates_nothing`・`the_resample_scratch_is_the_budgets_own_seat_not_a_throwaway`・`the_resample_scratch_seat_reaches_its_width_once_and_never_regrows` ほか | 定常状態の確保 0・リサンプル席の伸長規律 | 定常確保 0 は再導出・リサンプル席の檻は撤去候補 |
| 7 | `crates/areka-emo-present/src/presenter_perf_log_tests.rs` | `identity_scale_miss_reports_exact_zero_resample_stage_and_no_resample_allocs`・`miss_apply_emits_adjacent_info_and_perf_pair_with_every_stage_and_alloc_wired` | perf 行の全段配線・k≠1 外れで `t_resample_us` 非零 | 再導出候補（Requirement 3.4 の裁定に従う） |
| 8 | `crates/areka-emo-present/src/presenter_refresh_and_log_tests.rs`・`presenter_visibility_tests.rs`・`presenter/transition_record_tests.rs`・`presenter_resize_report_tests.rs` | `refresh_scale_after_dpi_change_reapplies_new_k`・`external_reshow_while_invisible_updates_scale_bounds_and_mask`・`a_scale_change_records_the_buffer_resize`・`dpi_change_reports_new_physical_size_to_caller` | k 変化時の再表示・戻り値＝`scaled_extent`・遷移観測の `resized`・窓寸報告 | 戻り値・窓寸報告は不変の見込み。`a_scale_change_records_the_buffer_resize` は供給面が原寸ゆえ再導出（Requirement 7.6） |
| 9 | `crates/areka-emo-present/src/presenter_read_accessor_tests.rs` | `visible_surface_hit_uses_applied_scale_at_k2` | 領域判定の ÷k | 不変（Requirement 4.5） |
| 10 | `crates/areka/examples/collision-probe/probe.rs`（`assert_drawn_anchor`）・`crates/areka/examples/emo-present/reconcile.rs`（起動 golden 比較） | `read_back` が k 適用後の面を返す前提で物理寸のアンカー画素・golden 長を検証 | 再導出候補（`read_back` は原寸・Requirement 6.2）。k=1 の golden 比較は不変 |
| 11 | `crates/areka/src/emo2_boot/frame_dpi_tests.rs` | `dpi_phase_reconciles_changed_window_to_scaled_extent` | 窓寸のみ | 不変（Requirement 2.2） |

## 付録 B: 設計フェーズへ送る選択肢（要件は選択によらず成立する）

- 提示側の変換の置き場（visual の scale／transform か、brush の stretch＋補間か、D2D 描画の行列か）と補間モード（Requirement 2.6）。
- 原寸マスクの ÷k 照会の実現（`alpha_mask_hit` の比例写像に任せるか、照会口を明示するか・Requirement 4.2）。
- 撤去段の perf フィールドの扱い（0 固定か、消費者と同時に撤去か・Requirement 3.4）。
- `chain.size()`（原寸）と物理寸の単一真実源の置き直し（`show.rs` の `size_changed`／`pending_resize` の判定材料）。
