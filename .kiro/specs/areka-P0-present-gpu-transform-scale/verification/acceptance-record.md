# 実機 2 水準サインオフ記録（Task 8.2 / 要件 2.3・2.7・3.1・3.2・3.5・4.8・6.6）

> 2026-09-12 実施。**水準②（200%・k=2）は自動走行でログ採取・判定済み。水準①（125%・k=5/4）と目視項目は開発者の手動確認待ち**（下記 §4）。
> 自動走行を行った機械では OS 表示スケールの切替（Windows の設定変更）を自動化では行わない方針のため、走行時点の primary モニタ（DPI 192＝200%）の 1 水準のみを採った。

## 1. 実行条件（水準②・200%）

| 項目 | 値 |
|---|---|
| コミット | `2f8c5769`（Task 8.1 着地後・作業木 clean） |
| 起動 | `target\debug\areka.exe <絶対 ghost_root> <絶対 balloon_root>`（fixture `crates/pilot/examples/shiori-host-32/fixtures/emo2`・`…/emo2-kakukaku`） |
| helper | `shiori-host32-helper.exe` を i686 で再ビルドし `target\debug\` へ差し替え（273,408 B） |
| env | `AREKA_APP_SMOKE_EXIT_MS=180000`・`RUST_LOG=info,areka_emo_present=debug,areka_ghost=info,wintf::tick=debug,wintf::transition=debug` |
| モニタ | primary `\\.\DISPLAY1`（`primary_dpi=192`・`window_dpi=Some((192, 192))`＝200%）。副 `\\.\DISPLAY2` は未使用 |
| 走行 | 08:16:17 起動 → 08:19:18 有界 auto-exit（3 分 1 秒）・**exit 0**・`unload_clean`（正規 clean shutdown） |
| ログ | `verification/emo2-real-200.log`（760 行・ANSI 除去済み） |

## 2. ログ判定（水準②）

| # | 観点 | 要件 | 結果 | 根拠（ログ） |
|---|---|---|---|---|
| 1 | 成立点 `info!` の物理寸 | 2.7／2.8 | **合格** | `apply(ShowSurface): 表示・マスクを更新` 142 行。shell `TargetId(0)`: `native_w=382 native_h=547 scaled_w=764 scaled_h=1094`（k=2/1）。相方 `TargetId(2)`: 336×400 → 672×800。バルーン `TargetId(1)`: 400×224 → 800×448、`TargetId(3)`: 288×203 → 576×406。**設計の見込み（764×1094）と一致**。フィールド名・意味は不変（`target_id`／`surface_id`／`cache_hit`／`k_ratio`／`k`／`author_dpi`／`window_dpi`／`native_w`／`native_h`／`scaled_w`／`scaled_h`／`size_changed`） |
| 2 | 引き当て（ヒット） | 5.3 | **観測（DPI 変化なし）** | ヒット 79／外れ 63。DPI 変化は走行中に発生しなかった（単一モニタ・ドラッグなし）ため「DPI 変化直後の `cache_hit=true`」は実機では未観測。決定論檻 `same_scale_hits_cache_and_window_dpi_change_still_hits`・`refresh_scale_after_dpi_change_reapplies_new_k` が固定 |
| 3 | 外れ 1 回の代金（`t_total_us`） | 3.1／3.5 | **合格（中央値・p90）／最大値は超過** | n=63: **中央値 10.69 ms・p90 16.12 ms・最大 29.83 ms**。内訳 中央値: compose 6.67 ms・mask 3.52 ms・upload（原寸 bitmap 生成＋命令記録）0.35 ms・cache 0.01 ms。**前回（症状 E）: 中央値 41〜78 ms・p90 65〜227 ms** → 約 1/4〜1/7 |
| 4 | 同 tick の wintf 再描画代金 | 3.1 | **合格** | `wintf::tick` の `rendersurface_us`: tick あたり中央値 0.075 ms・p90 0.151 ms（`prerendersurface_us` 同値・`composition_us` 0.044 ms）。外れの中央値 10.69 ms ＋ 再描画 ≪1 ms ＝ **16.7 ms の内側**。p90 16.12 ms は再描画分を足すと境界（≈16.3 ms） |
| 5 | ヒットの代金 | 5.3 | **合格** | ヒット n=79: 中央値 0.19 ms・p90 0.28 ms・`t_upload_us` は常に 0（GPU 呼び出し 0） |
| 6 | ティッカーの追い付き | 3.2 | **合格** | `loop ticker catch-up: skipped multiple boundaries, firing once` **8 件／3 分**（≤10・前回 10 → 60 件の悪化が解消） |
| 7 | 定常確保 | 3.6 | **合格** | 外れ 63 回のうち `alloc_compose_dst≠0` 10 回・`alloc_mask≠0` 10 回（いずれも初回確保＝target 4 本 × 席の立ち上がり）。以後 0 |
| 8 | `perf` 行のスキーマ | 3.3／3.4 | **合格** | フィールド: `target_id surface_id cache_hit t_cache_us t_compose_us t_mask_us t_upload_us t_total_us alloc_compose_dst alloc_mask key_hash frame`（11＋frame・撤去 3 名は出現 0） |
| 9 | 遷移観測 `surface` 行 | 7.6 | **合格** | `kind=surface stage=upload … w=800 h=448 resized=true` → `stage=visualize` が同 frame（frame=2）。`w/h` は物理寸 |
| 10 | 失敗経路 | 7.1／7.2 | **合格** | `ERROR` 0 件・`unload_clean` |
| 11 | 当たり判定（`hit_region_client`） | 4.8 | **未観測** | クリック操作を行っていないため `[hit_region_client]` 0 件 → §4 |

**外れの最大値 29.83 ms について**: compose 段の最大が 23.09 ms（本仕様の変更 0 の合成）で、走行機は本セッションの並走エージェント（cargo）と同居していた＝「静かな機械」ではない。要件 3.1 は負荷下の値を合否に載せない（報告のみ）。静かな機械での再計測は開発者側で（§4 ①）。

**外れの回数（shell 53 回）について**: まばたき（SERIKO パターン）で pattern 状態が変わるたびに合成入力が変わり、容量 3 の LRU を超えて回るため外れが続く。容量と置換方式は本仕様の変更 0（brief ⓒ 却下）。外れの代金が 1/4〜1/7 になったことが本仕様の効果であり、外れの回数を減らすのは別 spec の領分。

## 3. `judge-perf.py` について

`tools/perf/judge-perf.py` は `cpu.csv`（CPU 時系列・`invoke-perf-run.ps1` の計測ハーネスが生成）と `run-meta.txt` を要するため、本走行（生の areka.exe 起動）では実行していない。中央値・p90 は同じ `perf(apply_show)` 行から Python で集計した（前回値も同じ行の集計）。`--selftest` は Task 8.1 で緑を確認済み。

## 4. 開発者の手動確認（未実施・サインオフに必要）

1. **水準①（125%・k=5/4）**: OS の表示スケールを 125% にして同じコマンドで 3 分走行し、`apply(ShowSurface)` 行の `scaled_w/h` が **478×684**（shell）で水準②と異なることを確認する。可能なら静かな機械で走らせ、外れの `t_total_us` の最大値も採る。
2. **目視（両水準）**: 立ち絵とバルーンの大きさ・位置・原点（下端中央）が前回と同じ／拡大の画素ムラなし（bilinear）／透明部でクリックが後ろへ抜け不透明部でつかめる／絵が変わるコマで文字が止まらない／面切替のコマで絵と文字が同じコマに載る。スクリーンショットを `verification/` へ。
3. **当たり判定**: 相方の頭・胸をクリックし `RUST_LOG=areka_emo_present=debug` で `[hit_region_client] client 物理 px を ÷k して当たり判定を解決` が出ることを確認（`dic/touch.pasta` の `Head1通常`／`Bust1通常`）。
4. **DPI 変化直後の引き当て**: 窓を DPI の異なるモニタへドラッグ（または表示スケール切替）し、直後の `apply(ShowSurface)` 行が `cache_hit=true` であることを確認する（要件 5.3）。
5. **窓ドラッグ中の tick 診断**: ドラッグ中の `wintf::tick` 行を 1 つ読み、`rendersurface_us`（新規の定常代金・設計見込み 0.2〜1 ms/tick）を記録する。
6. ついでに e2e §13.1 行 3（初回起動限定の位置調整）の目視（roadmap の申し送り）。

## 5. 結論

- 本仕様の主目的（k≠1 の外れの代金）は水準②で **中央値 10.69 ms・p90 16.12 ms**（前回 41〜78／65〜227 ms）へ縮み、ヒットは GPU 呼び出し 0・ティッカーの追い付きは 8 件／3 分。
- 自動化で採れない項目（水準①・目視・クリック・DPI 変化・ドラッグ）は §4 に列挙した。**Task 8.2 の完了は §4 の開発者サインオフをもって確定する。**
