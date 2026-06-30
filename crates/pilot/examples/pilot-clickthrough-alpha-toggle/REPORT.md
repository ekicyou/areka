# REPORT: pilot-clickthrough-alpha-toggle 検証結果

> 本 REPORT は T1〜T8 の機械的な合否・証跡の詳細台帳である（根拠）。結論（go／違う／直す ＋ 学び）は README の「検証結果」に記す。
> 検証手順（R9.1）: 人間の準備確認 → エージェントが `cargo run -p pilot --example pilot-clickthrough-alpha-toggle` を起動 → 結果のヒアリング。
> go 判定は開発者（人間）が下す。Claude Code 単独で合格判定して次フェーズに進まない（R9.6）。**＝下記「総合判定」は空欄のまま人間の記入を待つ。**

- 検証日: 2026-06-30
- 実行コマンド: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`
- 環境: Windows 11（PMv2 = PER_MONITOR_AWARE_V2 設定成功）/ DPI 倍率: 要記入（未確認）/ モニタ構成: 要記入。観測ログ上の窓クライアント矩形 ≈ 2862x1503 物理px、client_screen ≈ [425,454,3287,1957]。

## T1〜T8 合否台帳

| # | 試験項目 | 期待結果 | 合否 | 証跡（観測内容・ログ抜粋） |
|---|---------|---------|------|----------------------------------------|
| T1 | 起動確認 | 透過トップモスト窓＋中央の不透明円が表示される | ✅ | 当初**不可視**（`ShowWindow` 未呼出 = R2.1 未充足の実装欠陥）→ `ShowWindow(SW_SHOW)` 追加後、人間が「円が見えてる」と確認。背景透過＋中央円表示。 |
| T2 | 円外でのクリック透過 | 円外クリックで背面アプリが反応（窓は受領しない） | ❌ | **不成立**。`WS_EX_TRANSPARENT` が立った状態（ex_style `0x200128`・透過 ON）でも、円外クリックが全て窓に届き `WM_LBUTTONDOWN` を受領（log: `client=(1104,1073)…円外（透過漏れ）` 等を ON 区間で多数）。作成時 TRANSPARENT のまま（トグル前）の円外クリックも同様に受領。背面へ抜けない。 |
| T3 | 円内でのクリック受領 | 円内クリックで WndProc に `WM_LBUTTONDOWN`（受領＋色トグル） | ✅ | 成立。透過 OFF（ex_style `0x200108`）区間で円内クリック `client=(1466,901) 円内（正常受領）` 等。座標判定はクライアント矩形基準へ統一後、正しく内外を判別。 |
| T4 | 状態切替の発火 | 円境界をまたぐ瞬間に ON↔OFF ログ | ✅ | 成立。`[applier] ON→OFF (…0x200128→0x200108)` / `OFF→ON (…0x200108→0x200128)` をカーソル移動に追随して出力。ワーカ判定→event→applier 適用の経路は機能。 |
| T5 | 状態変化なし時の非発火 | 留まっている間 SetWindowPos 非呼び出し | ✅（暫定） | 同一領域に留まる間は applier の切替ログ／API 呼出なし（差分ガード＋notify-on-change の二重ガードが機能）。網羅的反復は未実施。 |
| T6 | マルチプロセス透過 | 背面ブラウザのリンクが円外クリックで開く | ❌ | **不成立**。T2 と同根：窓が透過 ON でも円外クリックを横取りするため、背面別プロセスへクリックが到達しない。 |
| T7 | DPI 環境での座標一致 | 高 DPI でも円判定が見た目と一致 | ⚠️ 部分 | クライアント矩形基準へ統一後、見える円と判定円は一致（座標バグ解消）。ただし高 DPI 150% 等での明示検証は未実施（要記入）。 |
| T8 | 終了処理 | 窓を閉じるとプロセス・ワーカが正常終了 | ⚪ 未検証 | クローム無し窓のため検証中は `Stop-Process -Force` で終了させており、`WM_CLOSE` 経由の正常終了（close→done→join）は実機未確認。コード上の経路は実装済み（5.1）。 |

## 必須合格基準（T1・T2・T3・T4・T6）の充足

- すべて ✅ か: **いいえ。** T1 ✅ / T3 ✅ / T4 ✅ は充足するが、**T2 ❌・T6 ❌**（核心の「別プロセスへのクリック透過」）が不成立。
- 条件付き可（T5・T7・T8）: T5 ✅（暫定）/ T7 ⚠️部分 / T8 ⚪未検証。

## 核心 Unknown の所見（事実 ＋ 裏付け調査）

**事実（本 pilot の実測）**: `WS_EX_NOREDIRECTIONBITMAP`（DComp 描画）窓に対し、`WS_EX_LAYERED` 無しで `WS_EX_TRANSPARENT` を立てても（作成時・動的トグルのいずれでも）**クリックは背面別プロセスへ透過せず、窓が受領し続ける**。`WS_EX_TRANSPARENT` ビット（0x20）が ex_style に立っていることは `0x200128` のログで確認済み。座標ズレ等の交絡要因は client 矩形基準への統一で排除済み。

**裏付け調査（権威ソース・R10.4 に従い推測を排して確認）**:
- 通常窓では `WS_EX_TRANSPARENT` は「全窓クリックスルー（全マウス入力無視・`WM_NCHITTEST` すら来ない）」を生むが、複数ソースが **「`WS_EX_TRANSPARENT` は DWM/合成窓では効かない（透明領域がマウスを捕捉し続ける）」** と報告。
- マウス透過の定石は **`WS_EX_LAYERED | WS_EX_TRANSPARENT`**（両者セット）。「mouse passthrough が要件なら `WS_EX_LAYERED` が最良」。
- だが `WS_EX_NOREDIRECTIONBITMAP` は **「`WS_EX_LAYERED` の代替（合成エンジン用）」** と位置付けられ、両者は**排他的な設計路線**。＝「DComp 描画（NOREDIRECTIONBITMAP）」と「layered なマウス透過」は同一窓で素直には両立しない。これは本プロジェクトが当初 ULW を採った理由そのもの。
- 我らと**同一状況**の Microsoft Q&A「DirectComposition click through in transparent areas」は **accepted answer 無し（未解決）**。`WM_NCHITTEST→HTNOWHERE` も `LWA_COLORKEY` も「NOREDIRECTIONBITMAP では効かなかった」と報告。

**含意（人間の go 判定の材料・判定そのものは下さない）**:
1. 「DComp 描画を捨てず `WS_EX_TRANSPARENT` 単独トグルで別プロセス透過」という当初仮説は、**この構成では不成立**。
2. 定石の `WS_EX_LAYERED` は `WS_EX_NOREDIRECTIONBITMAP` と排他傾向で、安易な追加は DComp 描画と衝突しうる（R2.3 が `WS_EX_LAYERED` 不付与を要件化していた背景と整合）。
3. 既存の ULW ルート（alpha-0 自動透過）の妥当性が相対的に補強された。

## 次に試す価値のある実験（人間の承認・指示が必要）

> いずれも本 pilot の制約（`WS_EX_LAYERED` を勝手に足さない／go 判定は人間）に抵触するため、**実施判断は開発者に委ねる**。

- (A) **診断的対照**: `WS_EX_NOREDIRECTIONBITMAP` を外した通常窓で同じ `WS_EX_TRANSPARENT` トグルを試し、「透過漏れの主因が NOREDIRECTIONBITMAP か」を切り分ける（DComp 描画は捨てる前提の確認用）。
- (B) **`WS_EX_LAYERED | WS_EX_TRANSPARENT`**: 定石の併用を試す（R2.3 違反のため要承認）。ただし NOREDIRECTIONBITMAP との排他で DComp 描画が出なくなる可能性が高い＝ULW 回帰と同義になりうる。
- (C) **DComp ネイティブの per-pixel alpha ヒットテスト**: 「DirectComposition は alpha でクリックを選択透過できる」との示唆を深掘り（ただし上記 MS Q&A では未解決）。
- (D) **現実解の再評価**: 完了済み ULW ルート（`wintf-dcomp-migration-3-ulw-integration` 他）の維持。

## 総合判定（人間が記入）

- go / 違う / 直す: ____（**未記入＝人間の判断待ち**）
- 理由・学び: ____
