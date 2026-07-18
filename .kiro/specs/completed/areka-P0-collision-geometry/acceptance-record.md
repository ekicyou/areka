# 実 DPI 受け入れ検証記録（Task 4.2 / 要件 7.3・7.4）

> **✅ 合格（2026-07-18 開発者承認・k=1.0 契約下）**
>
> 本記録は本 spec 要件 **7.3 本文＝k=1.0（等倍）契約下**の座標契約を実 DPI≠96 の 2 水準（dpi=120/192）で実測し、Head/Bust/None の目視解決一致・静止 Δ=(0,0)・GetClientRect=surface_size（k=1.0）を実証した＝**7.3 充足**。
>
> **経緯（却下→切り出し→合格）**: 当初 2026-07-18 に一度**却下**——areka の基本設計は **DPI追従**（画面 DPI に追従してマスコット拡大縮小・SSP と別思想）であるのに、現状 `TextSlotView::scale()` は常に 1.0 で、両 DPI 水準ともマスコットが同一物理寸（382×547）＝ヒットテスト経路が同一だったため、「scale≠1.0（拡大表示）状態での当たり判定（点を k で縮約して照合）」が未検証だったこと。**しかしこれは本 spec 要件本文の外の設計思想ギャップ**であり、新 spec **`areka-P0-emo-dpi-scaling`**（render 基盤・emo が k× 実拡大レンダ）＋**`areka-P0-collision-dpi-hittest`**（point÷k・fake-k 決定論 unit・scale≠1.0 実機受け入れ・k=1.0 契約改訂）へ**切り出し済み**（roadmap 追記㉚・research.md §13）。開発者が「却下理由が別仕様に切り出されたことを確認したうえで合格とせよ」と承認＝**本 spec は k=1.0 契約下で合格**。
>
> **以下の実測値は k=1.0 契約下の 7.3 証跡として有効。** DPI追従下（scale≠1.0）の当たり判定受け入れは `collision-dpi-hittest` が担う（本 spec の範囲外）。

- 実施日: 2026-07-18
- 実施者: 目視操作＝開発者（実カーソルで頭/胸/背景を目視で狙う）／駆動・記録・自動 assert・外部窓寸実測＝Claude（Claudia）
- 対象: `cargo run -p areka --example collision-probe`（debug ビルド・PID 26028 他）
- 検証環境（per-monitor v2・全座標は物理 px で実測）:
  - **DPI 水準①（PRIMARY）**: DELL S3221QS・3840×2160・**dpi=120（125%）**・rcMonitor (0,0)-(3840,2160)・work (0,0)-(3840,2100)
  - **DPI 水準②（SECONDARY）**: 2880×1800・**dpi=192（200%）**・rcMonitor (-2880,365)-(0,2165)
  - DPI awareness は wintf `WinApp` 初期化がプロセスへ per-monitor v2 を設定。**dpi=96 は不使用**（両モニタとも ≠96）。
  - モニタ実効 DPI は per-monitor-v2 aware スレッドから `GetDpiForMonitor(MDT_EFFECTIVE_DPI)` で実測（120／192）。

> **反トートロジー遵守（7.3(a)）**: probe は `SetCursorPos`/`SendInput` を一切呼ばない（コード grep で確認済み）。狙点は操作者の**目視**による実カーソル移動のみ。collision 実値から合成した座標の注入は行っていない。

---

## 表示対象（②・両水準共通）

- **surface1000**（emo2 で collision を持つ唯一のサーフェス）を有効 bind 実値 `[1101,1206,1302,1502,1800]` 付きで shell target（scope=0）へ実表示。
- placeholder **誤寸 100×100** で spawn → 本番 `placement::follow::resize_window_to` で現表示実寸へ反映（戻り値 **true**）。
- 現表示実寸 = `text_slot_view(shell_target(0)).surface_size()` = **382×547**（scale=1.0）。
- 当たり判定（surfaces.txt・サーフェス px）: **Head=(93,62,271,130)** / **Bust=(133,270,229,326)**。

---

## DPI 水準①（dpi=120 / 125% / PRIMARY）

| # | 項目 | 結果 | 実測証跡（物理 px） |
|---|---|---|---|
| ① | per-monitor v2・dpi≠96 で実行 | **PASS** | dpi=120（≠96）で実行。dpi=96 不使用 |
| ② | surface1000 を実 bind 付きで shell target(scope=0) へ実表示 | **PASS** | `apply(ShowSurface) surface_id=1000 width=382 height=547`・bind `[1101,1206,1302,1502,1800]`・placeholder 100→resize_window_to→382×547（true） |
| ③ | k=1.0 hard assert（GetClientRect==surface_size ∧ scale==1.0） | **PASS** | probe 内自動 assert 通過: `client_w=382 client_h=547 surface_w=382 surface_h=547 scale=1.0`（次フレームで実窓 `GetClientRect`・`WindowPos` ミラーではない） |
| ④ | read_back 描画 anchor（Head 中心(182,96)／Bust 中心(181,298) 不透明） | **PASS** | `assert_drawn_anchor` が panic せず通過（α=0xFF）。read_back は合成パイプライン由来ゆえ DPI 非依存 |
| ⑤ | 解決一致（目視で Head／Bust／None を狙い resolve 結果一致） | **PASS** | 下の「解決一致 実測表①」参照 |
| ⑥ | 結果と実 DPI 値の記録 | **PASS** | 本記録 |

### 解決一致 実測表①（dpi=120・目視狙点・静止行 Δ=(0,0)）

| 狙った部位（目視） | client_x | client_y | s2c_x | s2c_y | Δx | Δy | resolve 結果 | 目視一致 |
|---|---|---|---|---|---|---|---|---|
| Head（頭・不透明） | 179 | 84 | 179 | 84 | 0 | 0 | **Some("Head")** | ✓ |
| Bust（胸・不透明） | 187 | 278 | 187 | 278 | 0 | 0 | **Some("Bust")** | ✓ |
| None（腹〜下半身・不透明・判定枠外） | 204 | 500 | 204 | 500 | 0 | 0 | **None** | ✓ |

- Head 静止サンプル: client_y ∈ [77,127]（Head 枠 62–130 内）・28 サンプル・全 Δ=(0,0)。
- Bust 静止サンプル: client_y ∈ [273,325]（Bust 枠 270–326 内）・全 Δ=(0,0)。
- None 静止サンプル: client_y ∈ [131,546]（枠間 131–269・枠下 327–）・193 サンプル・全 Δ=(0,0)。**None 行も本番マウス経路で観測できたのは、狙点が「透明余白」でなく「不透明に描かれた判定枠外の胴体」だから**（クリック透過は透明画素のみ・design C-3 整合）。
- 動作中の過渡のみ Δ=(0,0) から外れる（dy=-1・下方向移動時の 1px サンプル時刻差）＝設計脚注どおり・記録行（静止）は厳密一致。

---

## DPI 水準②（dpi=192 / 200% / SECONDARY）

char 窓を目視操作で SECONDARY（200%）モニタへドラッグして計測。

| # | 項目 | 結果 | 実測証跡（物理 px） |
|---|---|---|---|
| ① | per-monitor v2・dpi≠96 で実行 | **PASS** | char 窓 `GetDpiForWindow=192`（≠96）・SECONDARY モニタ上 |
| ② | surface1000 を shell target(scope=0) で表示（水準①から継続） | **PASS** | 同一 char 窓（surface1000）を 200% モニタへ移動。窓は再合成せず内容不変 |
| ③ | k=1.0 数値確認（外部 per-monitor-v2 aware `GetClientRect`） | **PASS** | 別プロセス（PowerShell・per-monitor v2）実測: char 窓 **client=382×547**（=surface_size）・windowRect (-432,1618)-(-50,2165)＝width 382/height 547（frameless ゆえ client≡window）・`GetDpiForWindow=192`。**窓は DPI 比例拡大せず（もし k≠1.0 なら 764×1094）＝k=1.0 が dpi=192 で数値確定** |
| ④ | read_back 描画 anchor | **PASS（DPI 非依存）** | ④ は合成ビットマップ（emo compose パイプライン）由来で表示モニタに非依存。水準①で通過済み・200% でも合成内容は不変 |
| ⑤ | 解決一致（目視で Head／Bust／None を狙い resolve 結果一致） | **PASS** | 下の「解決一致 実測表②」参照 |
| ⑥ | 結果と実 DPI 値の記録 | **PASS** | 本記録 |

### 解決一致 実測表②（dpi=192・目視狙点・静止行 Δ=(0,0)）

| 狙った部位（目視） | client_x | client_y | s2c_x | s2c_y | Δx | Δy | resolve 結果 | 目視一致 |
|---|---|---|---|---|---|---|---|---|
| Head（頭・不透明） | 171 | 108 | 171 | 108 | 0 | 0 | **Some("Head")** | ✓ |
| Bust（胸・不透明） | 177 | 296 | 177 | 296 | 0 | 0 | **Some("Bust")** | ✓ |
| None（首〜胴・不透明・判定枠外） | 177 | 197 | 177 | 197 | 0 | 0 | **None** | ✓ |

- Head 静止: (203,72)/(203,77)/(171,108) 等（Head 枠 62–130 内）・Δ=(0,0)。
- Bust 静止: (177,296)/(177,295)/(165,280) 等（Bust 枠 270–326 内）・Δ=(0,0)。
- None 静止: (177,197)/(177,244)/(177,162) 等（枠間 131–269）・Δ=(0,0)。
- **動作中の過渡 Δ が最大 9px** まで観測（水準①は最大 1px）。**これは欠陥ではない**——同一の視覚上の手の速さでも 200% モニタは物理 px 移動量が 2倍ゆえ、`WM_MOUSEMOVE` lparam（メッセージ投函時刻）と `GetCursorPos`（ハンドラ実行時刻）のサンプル時刻差が physical px で約 2倍に出るだけ（過渡 23/899 行）。**静止行（記録対象）は全て Δ=(0,0)＝系統的オフセット皆無**（awareness 経路不一致なら静止でも Δ≠0 になるはずだが、それが無い）。design プロトコル ⑤ 脚注どおり。

---

## 撫で一周の統合実機サインオフ（要件 7.4）— 本 spec の対象外

> 本判定行は測定でなく**確定済みの設計判断**。

撫で一周（マウス入力→SHIORI→応答 talk）の統合実機サインオフは**本 spec の対象外**であり、
**撫でクラスタ合流サインオフ＝`input-events` Req8.3** が1回で実施する（要件 7.4・design Coordination Notes C-4／Non-Goals で決着済み）。
本 probe の証跡は**表示側座標契約**（k=1.0・両水準）と**マウス経路の空間一致**（`client_point` ≡ `ScreenToClient`・静止行 Δ=(0,0)・両水準）に限られ、
本 spec の resolver が main へマージ済みであることが合流サインオフの前提供給となる。
マウス由来座標と collision 空間の**意味的**一致（撫で意味論・Reference4 組立～応答 talk）は合流サインオフのみが検証する。

---

## 設計方向の注記（k=1.0 は現実装状態・DPI追従が基本設計）

areka の**基本設計は DPI追従**（画面 DPI に追従してマスコット/サーフェスが拡大縮小する）であり、SSP（既定は固定ピクセル・等倍）とは**異なる設計思想**である。本記録が数値確定した **k=1.0（高 DPI でも非拡大）は現実装の状態であって設計目標ではない**。本 spec（collision-geometry）は現実装の k=1.0 契約下で座標解決を検証する範囲に閉じており、DPI追従が実装された時点で **Revalidation Trigger 2**（供給値の単一変更点 `TextSlotView::scale()`・現状 1.0 固定）により本 probe の再実行と、純関数への「照合前に点を k で除す」変更が要る（design 座標契約の確定表に既記載）。**「k=1.0 が正しい最終挙動」「SSP 同様に等倍が基本」とは解釈しないこと。**

---

## 判定

**判定: 合格（PASS・k=1.0 契約下・2026-07-18 開発者承認）**

- 本 spec 要件 **7.3 本文（k=1.0 契約下の座標契約実証）を dpi=120（125%）／192（200%）の 2 水準で充足**: Head/Bust/None の目視解決一致・静止 Δ=(0,0)・GetClientRect=surface_size（k=1.0）・read_back 描画 anchor・k=1.0 plumbing の DPI-clean 性（dpi/96 誤再スケール無し）。反トートロジー遵守（狙点は目視のみ）。
- **DPI追従（基本設計）下の当たり判定＝scale≠1.0（拡大表示）状態でのヒットテストは本 spec の範囲外**であり、新 spec `areka-P0-emo-dpi-scaling`＋`areka-P0-collision-dpi-hittest` へ切り出し済み（roadmap 追記㉚・research.md §13）。当初の却下（2026-07-18）はこのギャップが未追跡だったことが理由で、切り出し確認の上で承認へ転じた。
- 7.4 の統合実機サインオフは撫でクラスタ合流（input-events Req8.3）へ帰属（本 spec 対象外）。

- 表示側恒等契約（k=1.0）: 水準① probe 内 hard assert（GetClientRect==surface_size ∧ scale==1.0）／水準② 外部 per-monitor-v2 `GetClientRect`＝382×547（=surface_size）・DPI192・非拡大＝数値確定。
- マウス経路の空間一致: 両水準とも静止記録行 Δ=(0,0) 厳密一致（過渡の physical px 差は DPI 比例で説明でき系統的オフセットなし）。
- 解決一致: 両水準とも目視で狙った Head／Bust／None が resolve 結果と一致（反トートロジー遵守＝狙点は目視のみ・SetCursorPos/SendInput 不使用）。
- 7.4 の統合実機サインオフは撫でクラスタ合流（input-events Req8.3）へ帰属（本 spec 対象外）。
