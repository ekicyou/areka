# 第 1 段再観測レポート（Requirement 1）— 2026-08-15

settled main（W6.5 の 3 本＝`scale-exact-rational`・`windowposition-limit`・`recompose-budget` を全て含む）上で、旧観測値（2026-08-01・859ms・`SetWindowPos` 8 回）を捨てて症状を測り直した記録。本レポートは Requirement 1 の受入項目 1.1〜1.7 に沿って書く。**機序は「仮説」と「確定」を分けて書く。**

## 1. 採取条件（1.1）

| 項目 | 値 |
|---|---|
| コミット | `7ee3394cac47ff368ef6832e6001e1466529a4c8`（origin/main `3555cf53` と同内容＋本仕様の文書のみ） |
| ビルド | `cargo build --release -p areka --bin areka`（`opt-level='z'` 据え置き）・`shiori-host32-helper.exe`（i686 release）を `target\release\` へ同居 |
| 実行ファイル | `target\release\areka.exe`（2026-08-15 20:51:43） |
| ゴースト／バルーン | `crates\pilot\examples\shiori-host-32\fixtures\emo2`／同 `emo2-kakukaku`（絶対パス） |
| 実機構成 | モニタ 2 台。主＝2880×1800・**200%**（dpi 192・作業領域下端 1704・タスクバー 96px）／副＝2560×1600・150%（dpi 144・左側 −2560）。ゴーストは主モニタ上 |
| OS | Windows 11 Pro 10.0.26200 |
| `RUST_LOG` | `info,areka::placement::diag=debug,wintf::ecs::window=debug,wintf::ecs::layout::systems::monitor_systems=debug,areka_emo_present=debug,areka::emo2_boot::frame=debug` |
| 有界自動終了 | `AREKA_APP_SMOKE_EXIT_MS=480000`（8 分）＝唯一の終了経路。exit 0 |
| profile | 新品ディレクトリ（`AREKA_PROFILE_DIR`） |
| 生ログ | `%LOCALAPPDATA%\areka-diag\atom-s1-20260815-205207\atom-s1-osdpi.log`（2,258 行）・集計 `aggregate.txt`・`meta.txt` |

## 2. 手順と充足（1.2）

`dpi-window-vanish` 診断手順書 §6.2（セッション②）と同形。ドラッグ・クリックは一切行わず、OS 設定「ディスプレイ › 拡大縮小」で主モニタを **200% ↔ 100%**（dpi 192 ↔ 96）で 3 往復。

- 遷移回数（機械判定）: `[detect_display_change_system] Updating Monitor entity` **6 件**（192→96 が 3・96→192 が 3）＝各方向 3 回以上を充足。
- 無効化チェック: `[start_preparing]` **0 件**（`SESSION2-NO-DRAG: PASS`）。
- 発話は起動時挨拶のみ。遷移中に talk は無い。

> 注: 手順書は 125% ↔ 200% を例示するが、今回は 100% ↔ 200% で実施した。k は 1/1 ↔ 2/1 となり寸法差が大きい（旧 125↔200 は 5/4 ↔ 2/1）。差分の解釈で k の比が違う点に留意。

## 3. 遷移ごとの時系列（1.3）

各行の時刻はモニタ表更新行（`Updating Monitor entity`）を **+0ms** とする相対値。フレーム番号は現行ログに無いため**時刻近似**（要件 1.3 の 2026-08-15 裁定）。集計スクリプト＝`aggregate.txt`。

### 3.1 遷移 1（192→96・11:54:14.327）——代表例・全行を引用

```
+0ms    [detect_display_change_system] Updating Monitor entity  old_work_area=0,0,2880,1704 new_work_area=0,0,2880,1752 old_dpi=192 new_dpi=96
+0ms    Redriving window DPI from updated Monitor (no WM_DPICHANGED required) entity=5v0/6v0/4v0/3v0  192→96   （4 窓・同一 system・26µs 以内）
+13ms   apply(ShowSurface) TargetId(3) scaled 288x203 size_changed=true   / perf t_total_us=12382 (upload 8563)
+13ms   [diag.window_move] route=KeepPositionResize entity=5v0 kind=balloon scope=1 x=1684 y=754 w=288 h=203
+19ms   apply(ShowSurface) TargetId(2) scaled 336x400 size_changed=true   / perf t_total_us=6316
+19ms   [diag.window_move] route=DpiReproject      entity=6v0 kind=char    scope=1 x=1560 y=1304 w=336 h=400
+19ms   [diag.window_move] route=BalloonFollow     entity=5v0 kind=balloon scope=1 x=1852 y=1154
+33ms   apply(ShowSurface) TargetId(0) scaled 382x547 size_changed=true   / perf t_total_us=13857 (upload 11745)
+33ms   [diag.window_move] route=DpiReproject      entity=4v0 kind=char    scope=0 x=2255 y=1157 w=382 h=547
+33ms   [diag.window_move] route=BalloonFollow     entity=3v0 kind=balloon scope=0 x=1987 y=899
+38ms   apply(ShowSurface) TargetId(1) scaled 400x224 size_changed=true   / perf t_total_us=4687
+38ms   [diag.window_move] route=KeepPositionResize entity=3v0 kind=balloon scope=0 x=1987 y=899 w=400 h=224
        （+38ms〜+103ms: 該当 target のログ無し）
+103ms  [guarded_set_window_pos] SetWindowPos 0x702A0  (1684,754,288x203)  flags=20   ← バルーン1 寸
+124ms  [WM_DPICHANGED] entity=5v0 96→96 / suggested position write decision policy=ExternalAuthority applied=false
+181ms  [guarded_set_window_pos] SetWindowPos 0xD0AEE  (1560,1304,336x400) flags=20   ← キャラ1
+201ms  [WM_DPICHANGED] entity=6v0 applied=false
+216ms  [guarded_set_window_pos] SetWindowPos 0x702A0  (1852,1154,0x0)     flags=21   ← バルーン1 位置
+218ms  [guarded_set_window_pos] SetWindowPos 0x2109FA (2255,1157,382x547) flags=20   ← キャラ0
+230ms  [WM_DPICHANGED] entity=4v0 applied=false
+239ms  [guarded_set_window_pos] SetWindowPos 0xE095A  (1987,899,0x0)      flags=21   ← バルーン0 位置
+247ms  [WM_DPICHANGED] entity=3v0 applied=false
+255ms  [guarded_set_window_pos] SetWindowPos 0xE095A  (1987,899,400x224)  flags=20   ← バルーン0 寸
```

### 3.2 6 遷移の一覧

| # | 方向 | 描画内容の新寸可視化（4 窓） | 書込要求 enqueue 完了 | 最初の `SetWindowPos` | 最後の `SetWindowPos` | 書込回数 | 書込間隔 max | 経路 A |
|---|---|---|---|---|---|---|---|---|
| 1 | 192→96 | +13〜+38ms | +38ms | +103ms | **+255ms** | 6 | 77ms | 0 |
| 2 | 96→192 | +26〜+44ms（バルーン1 は不可視で見送り→+649ms に表示時） | +44ms（+650ms） | +63ms | +253ms（バルーン1 込みで +660ms） | 6 | 64ms（外れ値 407ms） | 0 |
| 3 | 192→96 | +19〜+41ms | +41ms | +69ms | **+263ms** | 6 | 81ms | 0 |
| 4 | 96→192 | +20〜+47ms | +47ms | +79ms | **+309ms** | 6 | 94ms | 0 |
| 5 | 192→96 | +17〜+35ms | +35ms | +94ms | **+271ms** | 6 | 61ms | 0 |
| 6 | 96→192 | +21〜+45ms | +45ms | +117ms | **+309ms** | 6 | 80ms | 0 |

- 書込 6 回の内訳は毎回同じ: キャラ窓 **1 回**（`DpiReproject`・位置＋寸）×2、バルーン窓 **2 回**（`KeepPositionResize` 寸＋`BalloonFollow` 位置）×2。研究文書 §1.1 の現行期待値と一致。
- 経路 A（`WM_DPICHANGED` の同期書込）: 24 件すべて `policy=ExternalAuthority applied=false`＝**書込 0 回**（`dpi-window-vanish` の是正が効いている）。
- `WM_DPICHANGED` は**モニタ表更新より後**（+98〜+298ms）に届き、そのときには DPI component は既に新値（`old=new`）＝二重発火は起きていない（k 不変ゲートで吸収）。
- 遷移 2 の外れ値: バルーン 1（TargetId 3）は遷移時点で不可視（`refresh_scale: 不可視ゆえ再表示しない`）だったため見送られ、+649ms に表示された時点で新寸へ（Requirement 4.6 の現状維持挙動＝欠陥ではない）。
- 起動時 5 回・遷移後 0 回＝定常状態の書込 churn なし（Requirement 4.7 の現状）。

## 4. 「跳ね」の残存判定（1.4）

### 4.1 数量による判定

| 観測量 | 旧（08-01・125↔200・commit f8bcfd0） | **今回（settled main）** |
|---|---|---|
| モニタ表更新 → 最終書込 | 859ms | **255〜309ms**（中央値 267ms） |
| `SetWindowPos` 回数 | 8（うち経路 A 4） | **6（経路 A 0）** |
| 窓ごとの逐次適用 | 60〜90ms 間隔で 1 枚ずつ | **残存**——最初の書込が +63〜117ms、以後 **61〜94ms 間隔**で 1 枚ずつ |
| 描画内容の新寸可視化 | 未観測 | **+13〜47ms に 4 窓とも新寸で可視化**（同一 `FrameFinalize` 内・順次） |
| 描画内容と窓矩形が食い違っている時間（窓ごと） | 未観測 | **50〜270ms**（描画は +13〜47ms で新寸、窓矩形は +63〜309ms まで旧寸） |

- **⑴ 逐次適用が無い → 不成立**（残っている）。
- **⑵ 全窓の書込がモニタ表更新直後の有界フレーム内で完了 → 不成立**（255〜309ms ≒ 60Hz で 15〜19 フレーム・120Hz で 31〜37 フレーム）。

### 4.2 目視所見（開発者）

開発者所見（2026-08-15・採取直後）: **「キャラは跳ねてた。縮小時に浮く感じ」**。

- ログとの突合: 縮小（192→96）では、描画内容が +13〜47ms で新寸（小）になるのに窓矩形は +63〜309ms まで旧寸・旧位置のまま＝内容が旧窓の中で縮んで見える区間があり、その後 `SetWindowPos` で窓が下端中央保存の新矩形へ動く。加えて接地点が新作業領域下端より **48px 上**に留まる（§6）ので、遷移後も浮いたまま。所見「縮小時に浮く」はこの 2 つの重ね合わせと整合する。どちらの寄与が目視の主体かは**未特定**（フレーム番号つき観測で切り分ける）。
- 拡大（96→192）についての所見は特に無し（接地点は起動時値と一致するため定常後のずれは 0。中間区間の見え方は未特定）。
- **⑶ 目視で跳ねなし → 不成立。**

### 4.3 旧観測との差分

- 所要は 859ms → 255〜309ms（**約 1/3**）。書込構成は「寸 4＋経路 A（S1）4＋末尾にバルーン 1 の位置のみ 1 行（brief 引用の 9 行目・flags=21）」→「寸 4＋`BalloonFollow` 位置 2」。経路 A の 4 回が消えた（van 是正）。旧ログでバルーン随伴の位置書込が 1 件しか見えない理由は未特定（当時 2 体のうち片方だけ随伴が発火した可能性・ログ引用が 8 回で打ち切られている可能性の両方がある）。
- 逐次パターン（1 枚ずつ 60〜90ms）は**そのまま残った**。合成コスト（perf 行 `t_total_us` は 4.7〜13.9ms／窓・4 窓計 30〜40ms）では 255〜309ms を説明できない。

## 5. 帰着（1.5）

**帰着＝遷移経路（本仕様の取り分）。** 根拠:

- 合成・アップロードの所要（perf 行）は 4 窓計 **30〜40ms**（遷移 1: 12,382＋6,316＋13,857＋4,687 = 37,242µs）で、全て +13〜47ms のあいだに収まっている。残りの **200〜260ms は `SetWindowPos` の逐次適用区間**（+63〜309ms）にあり、そこでは合成は走っていない（該当区間に perf 行なし）。
- 逐次区間の内側では、`SetWindowPos` の 1 回ごとに当該窓の `WM_DPICHANGED` 処理が同期で挟まる形が 24/24 で観測された（例: 遷移 1 で SWP 5v0 +103ms → WM_DPICHANGED 5v0 +124ms → SWP 6v0 +181ms → …）。**`SetWindowPos` 1 回が 60〜80ms を要している**（`Calling SetWindowPos` は呼出直前の行であり、次の行までの間隔＝呼出の所要と後続処理の和）。何に消えているかは**未特定**（仮説: OS 側の DPI 変更処理＋`WM_DPICHANGED`／`WM_WINDOWPOSCHANGED` の同期送達・DWM 側の再合成待ち）。
- enqueue 完了（+35〜47ms）から最初の `SetWindowPos`（+63〜117ms）までの **20〜80ms** も未特定（`FrameFinalize` の残り system・`Draw` 相・World 借用解放までの区間。フレーム番号が無いので tick 境界を名指しできない）。
- 既定 OFF の観測点はすべて点灯して採取した（`RUST_LOG` §1）。「発生 0 回」の主張は経路 A のみで、これは 24 件の `applied=false` 行を根拠とする。

## 6. 接地点の追随不全（1.6）

| 方向 | 新作業領域下端 | キャラ 0（y+h） | キャラ 1（y+h） | 差 |
|---|---|---|---|---|
| 192→96（×3） | 1752 | 1704 | 1704 | **−48px（浮き）** 3/3 |
| 96→192（×3） | 1704 | 1704 | 1704 | 0（起動時の作業領域下端と一致） |

- 接地点は全 12 件で **1704 固定**＝起動時 `MonitorSnapshot` の作業領域下端。旧観測の +36px（125↔200 のタスクバー高差 36px）に対し、今回は 100↔200 のタスクバー高差 **48px** として再現。`route=Resnap` の書込 0 件（作業領域変化を契機とする再スナップ経路は存在しない）＝Introduction の静的構造証跡と整合。
- 方向によって差が出る／出ないのは、起動時の拡大率（200%）に一致する側だけ偶然合うため。

## 7. 連鎖（二体の隣接）とキーワード基本位置（Requirement 6 の実測下敷き）

- 200%（起動時・確定済み連鎖）: キャラ 1 右端 = 1392+672 = 2064、キャラ 0 左端 = 2064 → **隙間 0**。
- 100%: キャラ 1 右端 = 1560+336 = 1896、キャラ 0 左端 = 2255 → **隙間 359px**（各窓が中央保存で縮んだ幅の半分の和 = (764−382)/2 + (672−336)/2 = 191+168）。200% へ戻すと隙間 0 に戻る。scg 申し送り（125↔200 で 52px）と同じ機序で、k の比が大きいぶん量が大きい。
- `finalize_chain` の解き直しは 6 遷移で 0 件（`ChainFinalized` 恒久標識・現行仕様どおり）。

## 8. 未特定（1.7）

1. `SetWindowPos` 1 回に 60〜80ms を要する内訳（OS／DWM／同期メッセージ処理のどこか）。
2. enqueue 完了から flush 開始までの 20〜80ms が tick のどこか（フレーム番号が無い）。
3. 「Y の浮き／沈み込み」がどの中間フレームで可視化されているか——描画内容が +13〜47ms で新寸になり窓矩形が +63〜309ms まで旧寸である区間が有力な候補だが、DWM 合成の単位でどう見えているかはフレーム番号つき観測（Requirement 2）と目視で確定する。
4. 旧観測（08-01）で `BalloonFollow` の位置書込が見えていなかった理由。

## 9. Requirement 9 への入力

- ⑴ 不成立・⑵ 不成立・⑶ 不成立 → **判定は「残存」**（3 条件とも不成立）。
- 縮退分岐は採らない。`balloon-offset-dpi` は本仕様に**従属**（先着後 rebase・Requirement 6.5 の裁定に従う）。
- 3 関心（観測基盤＋機序確定／原子性是正／作業領域追随）は**単一仕様で継続**（結論と根拠は requirements.md「第 1 段再観測の結果」節）。
