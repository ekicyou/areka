# 実 DPI 受け入れ検証記録（Task 4.2 / 要件 7.3・7.4）

> **状態: PENDING（未実施・手動実 DPI 実行待ち）**
>
> 本記録は `cargo run -p areka --example collision-probe` を **per-monitor v2・実 DPI≠96 の2水準以上**で実走し、
> probe の rustdoc プロトコル①〜⑥を目視・実測で確認した結果を記入するための雛形である。
> **未記入セル（`—`）と `PENDING` 判定が残る限り Task 4.2 は未完了**であり、`/kiro-impl` の自律実行では埋められない
> （実 DPI≠96 のGPU窓セッションと、反トートロジー条件〔7.3(a)〕を満たす**目視**による頭/胸/背景の狙いを要するため）。
> 実施者は下記「実行手順」に従い、実測値を埋め、各項目の判定を PASS/FAIL へ更新し、本ブロックを削除すること。

- 実施日: —
- 実施者: —（実 DPI≠96 実機での目視サインオフ担当。人間または表示可能な agent）
- 対象: `cargo run -p areka --example collision-probe`（ビルド種別・PID を記入）
- 検証環境（記入）:
  - **DPI 水準①**: モニタ名 ・解像度 ・**dpi=___（___%）** ・work area ___
  - **DPI 水準②**: モニタ名 ・解像度 ・**dpi=___（___%）** ・work area ___
  - per-monitor v2（example は `WinApp` 経由で DPI awareness 設定・全座標は物理 px で実測）
  - **dpi=96 のみの確認は不合格**（プロトコル①・`window-placement` 前例に倣う）

---

## 実行手順（probe rustdoc ①〜⑥に対応）

1. 実 DPI≠96 のモニタ（125%/150%/200% ＝ dpi 120/144/192 のいずれか2水準以上）で
   `cargo run -p areka --example collision-probe` を起動する。
2. probe は自動で本番 emo2 の `surface1000`（実 bind `[1101,1206,1302,1502,1800]`）を shell target（scope=0）へ実表示し、
   placeholder 誤寸(100×100)で spawn した窓を本番 `resize_window_to` で実寸へ反映する。
3. **k=1.0 assert（③・自動 hard assert）**: probe が次フレームで `GetClientRect`＝`surface_size()`・`scale()==1.0` を assert。
   パニックせず通過すれば PASS。落ちたら実測値（client 寸法／surface 寸法／scale）をFAIL証跡として記録。
4. **描画一致 anchor（④・自動）**: probe が `read_back` で Head 中心(182,96)／Bust 中心(181,298)の不透明を assert。通過で PASS。
5. **解決一致（⑤・目視が証拠力の中核）**: 画面に見えるゴーストの**頭／胸／背景を目視で狙って**カーソルを移動し、
   各記録行で 500ms 以上静止。probe が `GetCursorPos→ScreenToClient` した client 点を `resolve_hit_region(_,0,x,y)` へ渡し
   live ログする解決結果（`"Head"`/`"Bust"`/`None`）と、視覚上その部位に載っている事実の一致を記録。
   併せてマウス経路ペア列（`client_point` vs `ScreenToClient(GetCursorPos())`・Δ=(0,0) 厳密一致）を実測表へ併記。
   - **禁止（反トートロジー 7.3(a)）**: collision 実値から合成した screen 座標への `SetCursorPos`/`SendInput` を証跡としてはならない。狙点は目視由来のみ。
6. **判定（⑥）**: 全項目 PASS かつ dpi≠96 を2水準で充足したことを下の判定行に明記する。

---

## プロトコル結果（rustdoc ①〜⑥・DPI 水準ごとに記入）

### DPI 水準①（dpi=___）

| # | 項目 | 結果 | 実測証跡（物理 px） |
|---|---|---|---|
| ① | per-monitor v2・dpi≠96 で実行 | PENDING | dpi=___ / dpi=96 不使用 |
| ② | surface1000 を実 bind 付きで shell target(scope=0) へ実表示 | PENDING | 表示 id ・bind 集合 ・窓生成→resize 経路 |
| ③ | k=1.0 assert（GetClientRect==surface_size() かつ scale()==1.0・自動 hard assert） | PENDING | GetClientRect ___×___ ／ surface_size ___×___ ／ scale ___ |
| ④ | read_back 描画 anchor（Head 中心(182,96)／Bust 中心(181,298) 不透明・自動） | PENDING | Head α=___ ／ Bust α=___ |
| ⑤ | 解決一致（目視で Head／Bust／None を狙い resolve 結果一致） | PENDING | 下の「解決一致 実測表」参照 |
| ⑥ | 結果と実 DPI 値の記録 | PENDING | 本記録 |

#### 解決一致 実測表（DPI 水準①・目視狙点）

| 狙った部位 | client_x | client_y | s2c_x | s2c_y | Δx | Δy | resolve 結果 | 目視一致 |
|---|---|---|---|---|---|---|---|---|
| Head（不透明） | — | — | — | — | — | — | — | — |
| Bust（不透明） | — | — | — | — | — | — | — | — |
| 背景（None・脚注※） | — | — | — | — | — | — | None | — |

※ 背景（None）行はクリック透過（`WS_EX_TRANSPARENT`＋`HTTRANSPARENT`）によりイベント自体が窓へ届かず、
ペア列（client_point）は欠測が正しい挙動（design C-3 整合）。resolve は目視狙点（ScreenToClient 由来）で None を確認する。

### DPI 水準②（dpi=___）

| # | 項目 | 結果 | 実測証跡（物理 px） |
|---|---|---|---|
| ① | per-monitor v2・dpi≠96 で実行 | PENDING | dpi=___ / dpi=96 不使用 |
| ② | surface1000 を実 bind 付きで shell target(scope=0) へ実表示 | PENDING | 表示 id ・bind 集合 ・窓生成→resize 経路 |
| ③ | k=1.0 assert（GetClientRect==surface_size() かつ scale()==1.0・自動 hard assert） | PENDING | GetClientRect ___×___ ／ surface_size ___×___ ／ scale ___ |
| ④ | read_back 描画 anchor（Head 中心(182,96)／Bust 中心(181,298) 不透明・自動） | PENDING | Head α=___ ／ Bust α=___ |
| ⑤ | 解決一致（目視で Head／Bust／None を狙い resolve 結果一致） | PENDING | 下の「解決一致 実測表」参照 |
| ⑥ | 結果と実 DPI 値の記録 | PENDING | 本記録 |

#### 解決一致 実測表（DPI 水準②・目視狙点）

| 狙った部位 | client_x | client_y | s2c_x | s2c_y | Δx | Δy | resolve 結果 | 目視一致 |
|---|---|---|---|---|---|---|---|---|
| Head（不透明） | — | — | — | — | — | — | — | — |
| Bust（不透明） | — | — | — | — | — | — | — | — |
| 背景（None・脚注※） | — | — | — | — | — | — | None | — |

---

## 撫で一周の統合実機サインオフ（要件 7.4）— 本 spec の対象外（記入不要）

> 本判定行は測定でなく**確定済みの設計判断**であり、実行前に記入済みである。

撫で一周（マウス入力→SHIORI→応答 talk）の統合実機サインオフは**本 spec の対象外**であり、
**撫でクラスタ合流サインオフ＝`input-events` Req8.3** が1回で実施する（要件 7.4・design Coordination Notes C-4／Non-Goals で決着済み）。
本 probe の証跡は**表示側座標契約**（k=1.0）と**マウス経路の空間一致**（`client_point` ≡ `ScreenToClient`・Δ=(0,0)）に限られ、
本 spec の resolver が main へマージ済みであることが合流サインオフの前提供給となる。
マウス由来座標と collision 空間の**意味的**一致（撫で意味論・Reference4 組立～応答 talk）は合流サインオフのみが検証する。

---

## 判定

**判定: PENDING（未実施）** — 実 DPI≠96 の2水準で全項目 PASS を確認し、本行を
「**全項目 PASS（dpi≠96 必達条件を ___/___ の 2 水準で充足）**」へ更新すること。
7.4 の担当外注記（上節）は確定済みで記入済み。
