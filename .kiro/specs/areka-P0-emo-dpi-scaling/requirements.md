# Requirements Document

## Introduction

areka の基本設計は **DPI追従**（画面 DPI に追従してマスコット/サーフェスが拡大縮小する・SSP の固定 px 等倍とは異なる思想）である。ところが現状、emo 層は合成スケール **k=1.0 がコンパイル時定数でハードワイヤ**され、マスコットは高 DPI モニタでも拡大しない（固定物理 px）。この途中状態が `collision-geometry` Task 4.2 の実 DPI 受け入れを不成立にした——モニタ DPI を 2 水準（125%/200%）変えてもマスコットが同一物理寸ゆえ、DPI追従下（scale≠1.0）の当たり判定が全く検証できなかった。

本仕様（W4・M-dpi・**DPI追従レンダリング基盤＝render-scaling foundation**）は、emo が surface を **k = 窓の実モニタ DPI ÷ 作者基準 DPI（author_dpi・正典確定は設計フェーズ）** で実際に拡大レンダリングし、合成スケール照会値（下流が参照する scale 契約）がその k を返し、窓・合成先・配置採寸もそれに追従することを確立する。前提の窓 DPI 情報（実値取得・DPI 変化のライブ更新）は表示基盤（wintf）に既存であり、本仕様はそれを消費するのみで新規依存を持たない。拡大の実現方式（鮮明ラスタ再合成か合成 transform か）・author_dpi の正典値・整数段階か連続かの k 導出規約・再スケール機構・入れ子/着せ替えとの合成機構は、いずれも設計フェーズの設計論点として確定する（本要件はその選択によらず成立する観測可能な挙動のみを定める）。

拡大率の全体モデル（2026-07-24 要件ディスカッション #1 裁定）: areka は SSP の拡大率運用（ユーザー拡大率×`\![set,scaling]`×SERIKO scaling の乗算列・モニタに関わらず固定）を輸入せず、現代的 DPI 運用として **最終拡大率 = アプリ管理拡大率 × DPI 由来係数 k** の 2 因子乗算と定める（例: アプリ 200% × モニタ 200% → 最終 400%。モニタ間移動で最終拡大率が変化するのは仕様）。本仕様はこのうち DPI 由来係数 k を実装し、アプリ管理拡大率は 1.0 固定の縮退シームとして予約する（実設定手段の導入は将来 spec・追跡の要否は別セッション棚卸しで裁定）。

当たり判定の点÷k は下流 `areka-P0-collision-dpi-hittest`（W5）の領分であり、本仕様は「k 実拡大表示＋k 照会契約」という下流の観測条件を成立させる合流ゲートである。完了時、実 DPI（≠96）の実機で本番ゴースト emo2 のマスコットが DPI 相当寸に拡大表示され、窓がその寸に追従する。

## Boundary Context

- **In scope**:
  - 窓の実モニタ DPI と作者基準 DPI から表示スケール係数 k を導出し、表示ターゲット（窓）ごとに保持すること（k=1.0 定数の廃止）。
  - emo surface 合成結果の k× 実拡大表示（マスコットが DPI 相当の物理寸で描画される）。
  - 合成スケール照会値（下流 `collision-dpi-hittest` 等が参照する scale 契約）が実適用中の k を返すこと。
  - 窓クライアント寸・合成先（swapchain/visual）寸・窓配置採寸の k 追従。
  - 窓 DPI 変化（モニタ跨ぎ移動・表示スケール変更）時の k 再導出と表示更新。
  - 決定論 unit（オフスクリーン readback・純関数全網羅）＋実 DPI（≠96）実機観測（本番ゴースト先行）。
- **Out of scope**:
  - 当たり判定の点÷k・ヒットテスト規約の k 対応（`areka-P0-collision-dpi-hittest`・W5）。
  - 混在 DPI 環境での窓消失バグの解消（`areka-P0-dpi-window-vanish`・W5。本仕様は WM_DPICHANGED 窓追従の着地でその前提を供給する）。
  - アプリ管理拡大率の実設定手段（UI・タグ等）の導入（本仕様では 1.0 固定の縮退シームとして予約・追跡 spec/roadmap 記載の要否は別セッション棚卸しで裁定）。
  - SSP の scaling 運用の輸入（`\![set,scaling]`・ユーザー拡大率のモニタ非依存固定・SERIKO scaling 乗算列）。
  - SHIORI・撫で意味論・入力イベントの変更。
  - DPI追従が波及する他消費者（`window-placement` 窓寸・`emo-text-layer` 行寸・balloon 寸・`choice-render`）の再検証**実装**（各 spec の Revalidation Trigger として W5 で消化）。
  - バルーン採寸の per-scope 化（`areka-P0-kero-balloon`・W5）。
- **Adjacent expectations**:
  - 窓 DPI 情報は表示基盤（wintf）の既存 DPI 機構（実値取得＋DPI 変化のライブ更新）を consume するのみで、新規の外部依存・新規の基盤改造を前提としない。
  - **W4 同居の事前割当契約（roadmap 追記㊵/㊹/㊺）**: 本仕様の編集面は採寸源（`crates/areka/src/placement/measure.rs`）＋emo-atlas/compose/present＋wintf に限定し、`spawn.rs` は `position-persist` 単独所有ゆえ**不触**（窓寸の k 倍は採寸源で吸収する）。設計が spawn.rs 改変を要求する形に着地した場合、その部分は W5 へ送る（エスケープ条項）。
  - **W5 `kero-balloon` への申し送り（追記㊹）**: 採寸関数 `measure_scope_sizes` の再構造は、後続がバルーン採寸を scope 別へ改造できる席（per-scope バルーン寸法の余地）を潰さない関数分解とする。
  - **割込 `wintf-gpu-test-crash` 完了への rebase（追記㊺）**: 本仕様のテスト増分は同一プロセス内 2 個目 Compositor 生成の AV を再導入しない。wintf 配下の graphics テストターゲット（既存テストと同一プロセスで WUC を生成する場所）へ新設する場合は、完了済み共有 GPU オーナースレッド fixture（`crates/wintf/tests/graphics/common/mod.rs` の `on_gpu_owner_thread`）経由で実行する（テスト配置の振り分け基準は設計フェーズで明文化＝research.md Research Needed #7）。
  - author_dpi の正典値（ukadoc 準拠）・k 導出規約（整数段階か連続か）・拡大方式（Strategy A: k× 鮮明ラスタ／Strategy B: 合成 transform）・再スケール機構・入れ子/mayuna 着せ替えとの合成機構の確定は、設計フェーズの設計ディスカッションで行う。

## Requirements

### Requirement 1: DPI 由来スケール係数 k の導出と照会契約

**Objective:** 下流仕様（collision-dpi-hittest）の保守者として、窓の実モニタ DPI から導出された表示スケール係数 k が第一級で保持され、合成スケール照会値が実適用中の k と常に一致することを求める。これにより、÷k 当たり判定と k× 表示が同一の真実源に載る。

#### Acceptance Criteria

1. When emo が窓へ surface を表示するとき、the emo エンジン shall 当該窓の実モニタ DPI と作者基準 DPI（author_dpi・正典値は設計フェーズで ukadoc により確定）から表示スケール係数 k を導出する（k 導出規約＝整数段階か連続かも設計フェーズで確定し、その規約を単一の権威とする）。
2. The emo エンジン shall 合成スケール照会値（下流が参照する scale 契約）として、実際に表示へ適用中の k を返し、コンパイル時定数 1.0 の固定返しを廃する。
3. While 窓の実モニタ DPI が作者基準 DPI と等しい（表示スケール 100%）とき、the emo エンジン shall k=1.0 を導出し、既存の等倍表示と同一の寸法・描画を保つ。
4. If 窓の実モニタ DPI が取得できないとき、then the emo エンジン shall エラーをログに記録したうえで k=1.0 へ縮退し、表示を失わない（ログ無し失敗経路の禁止）。
5. The emo エンジン shall k を表示ターゲット（窓）ごとに保持し、DPI の異なる複数モニタに窓が同時に存在する場合も各窓が自窓の DPI 由来 k で表示される。
6. The emo エンジン shall 最終拡大率を「アプリ管理拡大率 × DPI 由来係数 k」の乗算合成として定義し、本仕様の範囲ではアプリ管理拡大率を 1.0 に固定する（合成スケール照会値＝最終拡大率。SSP の scaling 語彙は輸入せず、アプリ管理拡大率の実設定手段は将来 spec の領分とする）。

### Requirement 2: マスコットの k× 実拡大表示

**Objective:** ユーザとして、高 DPI モニタでマスコットが DPI 相当の大きさに拡大表示されることを求める。これにより、モニタの表示スケールに関わらずマスコットが意図された見かけ寸で常駐する。

#### Acceptance Criteria

1. When k≠1.0 の窓へ surface を表示するとき、the emo エンジン shall 合成結果を surface 原寸の k 倍の物理寸で描画する。
2. When 実モニタ DPI の異なる 2 水準（例: 125% と 200%）で同一 surface を表示するとき、the emo エンジン shall 各水準の k に従った互いに異なる物理寸で描画し（200% 水準では約 2 倍。連続 k 規約なら 125% 水準は約 1.25 倍）、両水準が同一物理寸となる k=1.0 固定の途中状態を残さない。
3. The emo エンジン shall マスコットを構成する全ての表示要素（ベース surface・SERIKO アニメパターン・mayuna 着せ替えパーツ・element 入れ子）を単一の k で一貫拡大し、要素間の相対配置・重なりが等倍時と同一の見た目関係を保つ。
4. When SERIKO ループや着せ替えにより表示 surface・パターンが切り替わるとき、the emo エンジン shall 切替後の合成結果も同一の k で拡大表示する。
5. The emo エンジン shall 拡大後の合成結果に欠け（クリップ）・意図しない切り捨てを生じさせず、端数寸法は設計フェーズで確定する単一の丸め規約に従って一貫処理する。

### Requirement 3: 窓・合成先・配置採寸の k 追従

**Objective:** ユーザとして、マスコットの拡大に伴い窓とその配置もその寸法に追従し、見切れ・余白・配置ズレのない表示を求める。これにより、拡大が表示の一部でなく窓ぐるみで成立する。

#### Acceptance Criteria

1. When k≠1.0 で surface を表示するとき、the 表示基盤 shall 窓のクライアント領域物理寸を k 倍後の合成寸（round(k × surface 原寸)・丸め規約は設計フェーズ確定の単一規約）に一致させる。
2. The 表示基盤 shall 合成先（swapchain/visual）の寸法を k 倍後の表示寸へ整合させ、拡大内容の見切れ・意図しない余白を生じさせない。
3. When 窓配置の採寸が窓寸を消費するとき、the 表示基盤 shall k 倍後の物理窓寸で配置計算を行い、画面端揃え等の配置結果が物理寸で正しく着地する。
4. The 表示基盤 shall 窓寸の k 追従を採寸の源で成立させ、既存の窓生成・窓移動の責務（`position-persist` 領分）に変更を要求しない。

### Requirement 4: DPI 変化への動的追従

**Objective:** ユーザとして、窓を DPI の異なるモニタへ移動したり OS の表示スケールを変更したとき、マスコットが新しい DPI 相当寸へ追従することを求める。これにより、混在 DPI 環境でも DPI追従思想が破綻しない。

#### Acceptance Criteria

1. When 窓の DPI が変化する（DPI の異なるモニタへの移動・表示スケール変更）とき、the emo エンジン shall 新しい窓 DPI から k を再導出し、マスコット表示を新 k 相当の物理寸へ更新する。
2. When k の再導出により表示が更新されるとき、the emo エンジン shall 窓クライアント寸・合成先寸・合成スケール照会値を新 k へ一貫更新し、更新完了後に照会値と実表示寸が一致する。
3. While talk 再生・SERIKO ループ等の進行中挙動があるとき、when 窓の DPI が変化したとき、the emo エンジン shall 表示の継続を保ち、クラッシュ・表示消失・進行中挙動の喪失を生じさせない。
4. If DPI 変化に伴う再導出・表示更新が失敗したとき、then the emo エンジン shall エラーをログに記録し、直前の k による表示を維持する（表示を失わない）。

### Requirement 5: 決定論テストによる檻

**Objective:** 開発者として、k× 拡大の判断分岐と純関数領域が GPU 実描画込みの決定論テストで檻に入ることを求める。これにより、DPI追従基盤が回帰檻で保護され、workspace テストゲートの決定論的緑が保たれる。

#### Acceptance Criteria

1. The emo エンジン shall k× 拡大の出力正しさ（拡大後寸法・拡大描画結果）をオフスクリーン readback による決定論 unit テストとして `cargo test` で実行可能にする（実 DPI モニタ・synthetic pointer・sleep に依存しない）。
2. The emo エンジン shall k 導出・寸法丸め・拡大後 extent 導出など純関数化可能な判断分岐を GPU 不要の実行テストで全網羅する。
3. The 本仕様のテスト増分 shall 同一プロセス内 2 個目 Compositor 生成によるクラッシュ（完了済み `wintf-gpu-test-crash` が根因解消した AV）を再導入しない。
4. Where 新設テストを wintf 配下の graphics テストターゲット（既存テストと同一プロセスで WUC を生成する場所）へ配置するとき、the 当該テスト shall 完了済み共有 GPU オーナースレッド fixture（`on_gpu_owner_thread`）経由で実行する（配置の振り分け基準は設計フェーズで明文化する）。
5. The 本仕様のテスト増分 shall 適用後も `cargo test --workspace` を exit 0 の決定論的緑に保つ。

### Requirement 6: 実 DPI 実機観測と人間サインオフ

**Objective:** 開発者として、実 DPI 環境の実機で本番ゴーストのマスコットが実際に DPI 相当寸へ拡大されることを人間の目視と決定論的判定で確認できることを求める。これにより、collision-geometry Task 4.2 を却下させた「k=1.0 途中状態」の解消が実機で保証される。

#### Acceptance Criteria

1. When 実 DPI（≠96 相当）の 2 水準（例: 125%/200%）で本番ゴースト emo2 を表示するとき、the emo エンジン shall マスコットを各水準の k 相当寸で描画し、窓クライアント寸が k 倍後の合成寸に一致すること・合成スケール照会値が k であることを観測可能にする。
2. The 実機サインオフ shall 本番ゴースト（実 emo2・実 pasta.dll）の表示を先行させたうえで行い、単発デモへの合わせ込みを判定根拠にしない。
3. Where 実機確認を自動化するとき、the 実機確認 shall 有界の自動終了とログ観測による決定論的判定（k 導出値・適用寸のログ出力）を可能にする。
4. Where 実機起動を行うとき、the 実機確認 shall pasta.dll をロード可能にするため絶対パスで起動する。

### Requirement 7: 既存資産の非退行と W4 同居境界

**Objective:** 保守者として、本増分が既存の全テスト緑・依存方針・並走 spec の編集面割当・後続 spec の席を崩さないことを求める。これにより、W4 同居と W5 以降の直列計画が担保される。

#### Acceptance Criteria

1. The 本増分 shall 適用後も既存テストをすべて緑に保ち、`cargo test --workspace` を exit 0 で成功させる。
2. While 窓 DPI が作者基準 DPI と等しい（k=1.0）とき、the emo エンジン shall 既存の表示寸・描画結果と等価な出力を保ち、既存決定論テストの期待値変更を要しない。
3. The 本増分 shall 新規の外部（crates.io）依存を追加せず、窓 DPI 情報は表示基盤の既存機構の consume に留める。
4. The 本増分 shall Rust 2024 で構築し、tokio を導入しない。
5. The 本増分 shall WUC/D2D 操作を UI スレッド固定で行い、既存のスレッド親和制約を破らない。
6. The 本増分 shall `spawn.rs`（`position-persist` 単独所有）を改変しない（W4 事前割当契約）。
7. If 設計が spawn.rs 改変を要求する形に着地したとき、then the 本仕様 shall 当該部分を本仕様で実装せず W5 へ送る（事前割当契約のエスケープ条項）。
8. When 採寸関数（scope ごとの窓寸採寸）を再構造するとき、the 本増分 shall バルーン寸法が scope 別になり得る席を潰さない関数分解とし、後続 `areka-P0-kero-balloon`（W5）の per-scope バルーン採寸改造を妨げない。
9. The 本増分 shall 当たり判定の点÷k・ヒット規約の変更を行わず、`areka-P0-collision-dpi-hittest`（W5）の領分として明示的に除外する。
