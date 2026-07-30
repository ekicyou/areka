# ギャップ分析: areka-P0-dpi-window-vanish

> 実施日: 2026-07-31 / 対象: `requirements.md`（確定済み・本書は要件を変更しない）
> 調査方法: 現行ツリー（branch `claude/areka-p0-dpi-window-vanish-9e998e`）の実コード読解（Grep/Glob/Read）。
> 外部依存の新規調査は不要と判断（Win32 の `WM_DPICHANGED` 契約以外に外部依存が無い）。
> 立場: **情報提供であり決定ではない**。各論点は選択肢と trade-off を示し、裁定は要件ディスカッションへ送る。

---

## 0. 本書の前提（brief の陳腐化補正を織り込み済み）

`brief.md` の「確定事実」ブロックのうち以下は**現行ツリーで失効**しており、本書は補正後の事実に立つ。

| brief の記述 | 現行ツリーでの実測 |
| --- | --- |
| ④「WM_DPICHANGED でも窓 336x400 物理固定」＝唯一確定の設計 gap | **解消済み**。`WM_DPICHANGED`（`window_pos.rs:285`）→ `Changed<DPI>` → `dpi_phase_with`（`frame.rs:782-839`）→ `reconcile_window_size`（`frame.rs:689-720`）→ `resize_window_to`／`resize_window_keep_position` が結線済み（`frame.rs:1310`） |
| ③「`guarded_set_window_pos` は 0 回＝二重位置権威は反証済み」 | **偽陰性**。当該書込は `window_pos.rs:359-369` に**実在**する（`SWP_NOSIZE` で `suggested_rect.left/top` を書く）。呼出ログは `window_pos.rs:352`（`trace!`）と `command.rs:94`（`trace!`）の**両方が trace 水準**であり、2026-07-18 repro の `wintf::ecs::window_proc=debug` では**原理的に出力されない**。仮説は**未反証へ戻る** |
| ⑥「`Anchored` 未付与 WARN は `placement/mod.rs:99`」 | 現所在は `follow.rs:790-795`。`GhostWindows` Resource 定義は `spawn.rs:115`（`ScopeWindows` は `:101`） |

---

## 1. Current State — 現況調査

### 1.1 位置権威の実配線（Requirement 3・4 の核心）

`WM_DPICHANGED` 受信から窓位置が確定するまでの実経路は**3 本の書き手**が同一フレームに並ぶ。

```
WM_DPICHANGED (window_pos.rs:285-375)
 ├① DPI component を直接更新 → Changed<DPI> 発火            (window_pos.rs:320-338)
 ├② DpiChangeContext::set(new_dpi, suggested_rect)          (window_pos.rs:343-346)
 └③ guarded_set_window_pos(suggested.left, suggested.top,
       SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE)              (window_pos.rs:359-369)  ← 【書き手A】
        └ 同期発火 WM_WINDOWPOSCHANGED (window_pos.rs:36-272)
             ├ is_echo=true / dpi_context=Some → use_bypass=false      (:109)
             ├ correct_position_for_dpi_center_preserve(...)           (:131-137)
             ├ window_pos.position = corrected_pos ← **変更検知あり**   (:143-148)
             ├ ② try_tick_on_vsync()  ← ここで schedule が回り得る     (:259-262)
             └ ③ flush_window_pos_commands()                           (:268)
```

- **書き手A**: OS 推奨矩形の left/top を**そのまま**実窓へ書く。位置のみ（`SWP_NOSIZE`）。
- **書き手B**: `Changed<WindowPos>` を受けた `apply_window_pos_changes`（`graphics/systems/window_pos.rs:21-109`）が
  `WindowPos`（＝A が書いた OS 推奨位置）から `SetWindowPosCommand` を**再発行**する。
  さらに `sync_window_arrangement_from_window_pos`（`window_pos_systems.rs:133-170`）が
  `Arrangement.offset := OS 推奨位置` を書く（ヒットテスト境界の権威も OS 側へ移る）。
- **書き手C**: areka の `run_dpi_phase`（`frame.rs:865-873`）→ `resize_window_to`（`follow.rs:786-910`）。
  ここが**唯一 `Anchored` を読んで接地点規約を適用する**経路。

#### 決定的な発見 (a): `resize_window_to` の入力 `raw` は「A が書いた OS 推奨位置」である

`resize_window_to` は生位置を `WindowPos.position` から読む（`follow.rs:810-826`）。`Bottom` アンカーの射影
（`project_anchor` → `BottomSnapPolicy::resolve`・`follow.rs:85-112`）は **Y のみ再計算し X は素通し**する。
したがって **OS が提示した X は areka 側でも上書きされず最終位置として残る**。
要件 4.3「OS の推奨位置を最終位置としてそのまま残さない」は、X 軸について**現行コードでは満たされていない**。

#### 決定的な発見 (b): 再射影が「寸の変化」に条件付けられている

`dpi_phase_with` は `refresh_scale` が `Some(新物理寸)` を返したときにしか `reconcile_window_size` を呼ばない
（`frame.rs:835-837`）。`refresh_scale`（`presenter.rs:754-819`）が `None` を返す経路は **4 つ**あり、
そのすべてで**位置の再射影が一切走らない**：

| None 経路 | 位置 | 該当行 |
| --- | --- | --- |
| k 不変 | OS 推奨位置が残る | `presenter.rs:772-775` |
| 不可視（Hide／全透明退化） | OS 推奨位置が残る | `presenter.rs:776-782` |
| `last_show` 無し（未表示） | OS 推奨位置が残る | `presenter.rs:783-789` |
| **k は変わったが丸め後の物理寸が同一** | OS 推奨位置が残る | `presenter.rs:816-818`（`take_pending_resize` が `None`） |

**「位置の権威」と「寸の権威」が同じ戻り値に相乗りしている**のが構造的欠陥である。
要件 4.1/4.2/4.6 はこの分離を要求している。

#### 決定的な発見 (c): 毎フレーム resnap は Y だけを救う

`resnap_shell_targets`（`frame.rs:1157-1228`）は毎フレーム shell target の物理寸を引き
`resize_window_to` を呼ぶ（`resnap_from_sizes`）。したがって **Y は毎フレーム
「窓中心が属する（または最近傍の）モニタの work area 下端」へ吸着し続ける**。
一方 **X には一切の不変条件が無い**。
`work_area_for_window`（`follow.rs:1132-1160`）はどのモニタにも属さない窓に対して
**最近傍モニタを返す**ため、X が遠方でも `wa` は取れてしまい `warn` すら出ない。

> ⇒ 「キャラ窓が全モニタ work area と交差しない」状態は **X 軸単独で到達可能**であり、
> Requirement 3.1 に対応する防御は**現在どこにも存在しない**。Q2 の「真の画面外か見落としか」は
> 「Y は必ず何処かのモニタ下端・X は無制限」という形で事前に絞り込める。

#### 補足: `correct_position_for_dpi_center_preserve` は ghost 窓では常に不発

`dpi_helpers.rs:62-113` の中心保持補正は `BoxStyle` を要求する。ゴースト窓は
**`BoxStyle` を意図的に付けない**（U2/DD8・`spawn.rs:703` の檻テストが「付けてはならない」を固定）。
よって毎回 `dpi_helpers.rs:75-78` の `warn!("... BoxStyle not found")` 分岐へ落ちる。

- 良い面: areka 側 `resize_window_to` の下端中央付替え（`follow.rs:835-844`）との**二重補正は起きない**。
- 悪い面: **ゴースト窓の DPI 変化ごとに良性 warn が鳴る**（診断ログのノイズ源／要件 6 の趣旨と同型の
  「良性ノイズと本物の異常の取り違え」）。2026-07-18 の repro ログにこの warn の記載が無いこと自体が
  検証すべき手がかり（`info` 水準でも出るはずの warn である）。

### 1.2 観測点の実在と水準（Requirement 1）

| 要件 | 必要な出力 | 現況 | 判定 |
| --- | --- | --- | --- |
| 1.1 全モニタの識別子・bounds・**work_area**・DPI・primary | `monitor_systems.rs:97-105` が `debug!` で **bounds/dpi/is_primary のみ**。`work_area` と `handle` は**出力されない**。`Monitor` 型自体は `work_area` を保持（`monitor.rs:68-74`）し `Debug` 整形も持つ（`monitor.rs:87-96`）が未使用 | **Missing** |
| 1.1（続） | areka 側 `MonitorSnapshot` 構築点（`main.rs:645-647`）は**完全に無ログ**。`placement/mod.rs:308` の `enumerate_monitors` も work area をログしない。列挙は**3 箇所独立**（wintf `initialize_layout_root` / areka `prepare_ghost_windows` / areka `MonitorSnapshot`） | **Missing + Constraint（列挙の三重化）** |
| 1.2 位置/寸変化ごとに 物理位置・物理寸・種別・scope・DPI・経路 | 実 `SetWindowPos` は `guarded_set_window_pos`（`command.rs:94-99`）＝ **`trace!` 一本**。`apply_window_pos_changes`（`graphics/.../window_pos.rs:79-87`）は `debug!` だが **scope・種別・DPI を持たない**。areka の単一ライター `enqueue_window_set_pos`（`follow.rs:1009-1080`）は **成功時に何もログしない**（失敗時のみ warn） | **Missing** |
| 1.3 新旧 DPI・OS 推奨矩形・推奨位置に基づく位置変更の実施可否 | `WM_DPICHANGED` 冒頭の `debug!`（`window_pos.rs:302-313`）が新 DPI と suggested_rect を出す（**旧 DPI は無い**が `:325-332` の別 `debug!` が old/new を出す）。**「実際に推奨位置で SetWindowPos したか」は `trace!` のみ**（`window_pos.rs:352-357`＋`command.rs:94`）＝**今回の偽陰性の直接原因** | **Missing（水準）** |
| 1.4 診断手順書 | 該当ドキュメントは**存在しない**。有界終了 `AREKA_APP_SMOKE_EXIT_MS` は実在（`main.rs:803`）し `despawn_smoke_targets`（`main.rs:773-783`）が発火する | **Missing（成果物）** |
| 1.5 観測点が手順の水準に載っていることの明示 | 概念自体が未導入。**1.3 の trace 問題を制度化して防ぐ条項** | **Missing（規律）** |
| 1.6 有界自動終了 | **実装済み**（`main.rs:730-763`／`smoke_exit_ms_from` は純関数で単体テスト済み） | **充足** |

### 1.3 接地点保全の実配線（Requirement 4）

- 接地点＝下端中央の保存は `resize_window_to` の 2 段（`follow.rs:835-844` の X 中央付替え＋
  `follow.rs:850` の `project_anchor` による Y 再導出）で実現済み。**Bottom アンカー限定**であることが明記されている。
- バルーン随伴は `follow.rs:880-907`（原点差 Δ で `BalloonFollow.offset` を付替え → `follow_balloon`）。
  要件 4.4 に対応する機構は**存在する**が、**発火条件が 1.1(b) の `Some` ゲートに縛られている**。
- 要件 4.5（再導出結果が得られないとき現状維持）は `frame.rs:835`（`None` は触らない）＋
  `follow.rs:799-806`（非正寸ガード）で**充足済み**。
- 要件 4.6（不可視中の DPI 変化 → 次の可視化時に正しい位置・寸）は、
  `apply_show` 経由の `pending_resize` → `reconcile_reported_sizes`（`frame.rs:985-1026`）で
  **寸は追いつく**。**位置は char 窓なら `resize_window_to` が Y を再射影するが X は OS 値のまま**、
  balloon 窓は `resize_window_keep_position`（`follow.rs:1194-1240`）＝**位置据置き**である。
  balloon の位置は `follow_balloon` 依存ゆえ char 窓が動かないフレームでは補正されない。**要検証**。

### 1.4 テスト資産（Requirement 5）

**既存の偽装境界（fake boundary）は十分に揃っており、新機構をほぼ要さない。**

| 資産 | 所在 | R5 での使い道 |
| --- | --- | --- |
| `MonitorSnapshot` は純データ Resource（`Vec<RectPx>`） | `follow.rs:1094-1118` | **合成マルチモニタ・混在レイアウトを直接注入可**（R5.1/5.3） |
| 偽 HWND ＋ `SetWindowPosCommand` を flush しない headless World | `follow.rs:1260-1290` 付近のテストヘルパ | 実 `SetWindowPos` を呼ばず位置・寸を検証（R5.2） |
| `DPI::from_dpi(120/144/192, …)` を entity へ注入した `Changed<DPI>` 檻 | `frame.rs:2655/2683/2812/3025/3051` | DPI 水準パラメタ化の先例 |
| DPI 4 水準の不変テスト（`DPIS=[96,120,144,192]`＋厳密整除ヘルパ `px()`） | `resolver.rs:303-318` | **R5.6「絶対 px でなく比／不変条件」の既存お手本** |
| `prepare_ghost_windows_with_work_area`（work area と primary DPI を引数で差し替え） | `placement/mod.rs:338-345` | 起動時 k₀ と work area の合成注入 |
| `resnap` 檻（2 スコープ・偽 HWND・`MonitorSnapshot` 注入 World） | `frame.rs:1907-2020` | 本 spec の回帰檻の**直接の donor** |
| `dispatch_window_message` の headless `WM_DPICHANGED` テスト（実 HWND 不要・null HWND で `Err` になるが panic しない） | `window_proc/mod.rs:204-237` | wndproc 分岐の檻。ただし **`guarded_set_window_pos` の呼出可否を観測する手段が無い**（TLS カウンタは `is_self_initiated` のみ公開） |

**不足**: 「OS 推奨位置が最終位置として残らない」（R4.3）を決定論で判定するシームが無い。
現状 `WM_DPICHANGED` ハンドラは純関数へ切り出されておらず（`dpi_helpers.rs` は中心補正のみ）、
**「推奨矩形をどう扱うか」という判断分岐が wndproc 本体に埋まっている**。

### 1.5 レジストリ掃除（Requirement 6）

- `GhostWindows`（`spawn.rs:115-135`）は `BTreeMap<usize, ScopeWindows>` の Resource。
  **despawn 時の掃除は一切無い**。`despawn_smoke_targets`（`main.rs:773-783`）は
  `GhostWindowMarker` 付き entity を `world.despawn` するだけで Resource を触らない。
- 破棄後も `emo2_frame_system`（`frame.rs:1297-1327`）は走り続け、
  `reconcile_reported_sizes`（`frame.rs:985-1026`）／`resnap_with`（`frame.rs:1188-1228`）が
  **死んだ entity へ `resize_window_to` を呼ぶ** → `follow.rs:789-795` の
  `warn!("Anchored 未付与（char 窓は spawn で必ず付与）のため resize しない")` が鳴る。
  これが brief の「良性シャットダウン競合」の実体である。
- 要件 6.2/6.3 の「正常系として打ち切る」を満たすには、**掃除（entity を写像から外す）**か
  **消費側の存在確認**か、あるいは両方が要る（設計判断項目 D8）。

---

## 2. Requirement → 資産マップ（gap タグ付き）

| Req | 必要能力 | 既存資産 | Gap |
| --- | --- | --- | --- |
| 1.1 | モニタ全数の識別子/bounds/work_area/DPI/primary 出力 | `Monitor`（work_area 保持）・`enumerate_monitors` | **Missing**（出力項目・水準・出力点の一本化） |
| 1.2 | 窓の位置/寸変化の経路付き出力 | `enqueue_window_set_pos` 単一ライター・`guarded_set_window_pos` | **Missing**（成功パスのログ・scope/種別/DPI/経路タグ） |
| 1.3 | DPI 変化の新旧・推奨矩形・書込実施可否 | `WM_DPICHANGED` の `debug!` 2 本 | **Missing**（推奨位置書込の実施可否が trace） |
| 1.4/1.5 | 再実行可能な診断手順書＋水準整合の明示 | 有界終了・記憶 `areka-real-machine-signoff-bounded-auto-exit` | **Missing**（成果物そのもの） |
| 1.6 | 有界自動終了 | `AREKA_APP_SMOKE_EXIT_MS` | **充足** |
| 2.x | 診断レポート（Q1〜Q4） | なし（本 spec の成果物） | **Missing**（手順書と対の成果物） |
| 3.1/3.4 | 非ドラッグ要因での不可視化防止 | `work_area_for_window` は最近傍フォールバック＝**防御でない** | **Missing**（X 軸の不変条件が皆無） |
| 3.2 | モニタ構成の食い違いを警告し、不可視位置へ動かさない | 構成変化追随そのものが M1 非対象（`follow.rs:1091` の DD15 note） | **Missing / Constraint** |
| 3.3 | 入力欠落時は位置不変＋警告 | `resize_window_to` の縮退群（`follow.rs:789-826`）・`project_anchor` の identity 縮退 | **概ね充足**（`work_area_for_window` の最近傍返しだけが「入力欠落」を隠す） |
| 4.1/4.2 | DPI 変化前後で接地点保存 | `resize_window_to` 3b＋`project_anchor` | **条件付き充足**（`refresh_scale` が `Some` のときのみ発火） |
| 4.3 | OS 推奨位置を最終位置に残さない | なし | **Missing**（X 軸が素通し・`None` 経路で全軸素通し） |
| 4.4 | バルーン相対位置維持 | `follow.rs:880-907` | **条件付き充足**（同上） |
| 4.5 | 再導出不能なら現状維持 | `frame.rs:835`／`follow.rs:799-806` | **充足** |
| 4.6 | 不可視中 DPI 変化 → 可視化時に整合 | `pending_resize`→`reconcile_reported_sizes` | **Unknown**（寸は追う・位置は未検証。とくに balloon） |
| 5.1-5.4/5.6 | 96 以外の DPI・複数モニタ注入の決定論檻 | `MonitorSnapshot` 注入・偽 HWND・`DPIS` パターン・`resnap` 檻 | **部分充足**（土台あり／R4.3 判定用シームが Missing） |
| 5.5 | 実機サインオフ手順 | 有界終了＋`RUST_LOG` grep 定石 | **Missing**（文書化） |
| 6.1-6.4 | `GhostWindows` の despawn 掃除 | `GhostWindows`（掃除口なし） | **Missing** |

---

## 3. 診断で確定させるべき仮説（憶測修正の禁止＝Req2.7 に従う）

要件は「原因確定まで機構に手を入れない」ため、本節は**修正案ではなく検証設計**である。

| # | 仮説 | 予測される痕跡（診断ログで判別可能） | 現時点の確からしさ |
| --- | --- | --- | --- |
| H1 | **OS 推奨位置の X が最終位置として残り続け、DPI 境界跨ぎのたびに X が飛ぶ** | `WM_DPICHANGED` の `suggested_left` と、直後の `enqueue_window_set_pos` が書いた x が**一致**する（Y だけ `wa.bottom−h` になる） | **高**（コードから構造的に導出。§1.1(a)） |
| H2 | **`refresh_scale` の `None` 経路で位置の再射影ごと欠落する** | `Changed<DPI>` は出るのに `resize_window_to` の書込ログが無いフレームが存在する | **高**（§1.1(b)） |
| H3 | **X 軸に不可視化の防波堤が無いため、何らかの X 変位が累積して真の画面外へ出る** | 消失時の窓矩形が全 work area と X 方向で非交差・Y はいずれかのモニタ下端 | **中〜高**（§1.1(c)。「累積源」は未特定＝診断の主目標） |
| H4 | バルーン消失はキャラ窓追従の随伴（独立バグでない） | `balloon_pos − char_pos ≡ BalloonFollow.offset` が消失前後で保存 | **中**（`follow_balloon` の恒等式維持が設計上明示。ただし `resize_window_keep_position` 経路では char が動かない限り balloon も動かない＝**乖離し得る窓**がある） |
| H5 | 消失はドラッグ起因でない（Q3） | 消失時刻の前後に `[drag]` ログが無く `WM_DPICHANGED` のみがある | **未知**（要実測） |
| H6 | モニタ構成情報の陳腐化（`MonitorSnapshot` はセッション固定・`follow.rs:1091`）が原因 | 起動時 work_area ログと消失時の実配置が食い違う | **低〜中**（10 分運転で構成変化が無ければ棄却できる） |

**診断の最小追加観測**（Requirement 1 の実装が同時にこれを満たす）:
`WM_DPICHANGED` の (旧DPI, 新DPI, suggested_rect, 書込実施可否) と、
`enqueue_window_set_pos` / `guarded_set_window_pos` の (経路タグ, 窓種別, scope, 物理位置, 物理寸, 当該窓 DPI) と、
起動時の全モニタ (handle, bounds, work_area, dpi, primary)。**すべて `info` もしくは診断手順が有効化する水準に置くこと**（R1.5）。

---

## 4. 実装アプローチの選択肢

要件は**診断フェーズ（R1・R2）→ 修正フェーズ（R3・R4）→ 檻（R5）→ 掃除（R6）**の 2 段構えを定めている。
以下は**フェーズごとに独立に選べる**選択肢である。

### 4.1 診断観測（R1）の載せ方

#### Option A: 既存ログ呼出の水準引き上げ＋フィールド追加（拡張）
- 触る所: `monitor_systems.rs:97-105`（work_area/handle 追加）・`window_pos.rs:302-313/352-357`（`trace!`→診断水準）・
  `command.rs:94-99`・`follow.rs:1009-1080`（成功時ログ追加）。
- ✅ 新モジュール 0・差分が小さい・偽陰性の直接原因を最短で潰す。
- ❌ 診断用フィールド（scope・窓種別）が **wintf 層に無い**ため、`follow.rs` 側に寄せざるを得ず出力が 2 箇所に割れる。
- ❌ `trace!` を恒久的に `debug!/info!` へ上げるとホットパス（ドラッグ中の毎イベント）でログ spam。
  → **経路タグ付きの専用 target**（例 `areka::placement::trace`／既存 `areka::persist::save` の先例が `follow.rs:705`）で
  水準ではなく target で切り分けるのが repo 慣行に合う。

#### Option B: 診断専用の観測モジュールを新設（新規）
- 例: `crates/areka/src/placement/diag.rs` — 「窓位置イベント」を 1 つの構造化レコード型に集約し、
  `enqueue_window_set_pos` と `WM_DPICHANGED` ブリッジの 2 点から供給する。
- ✅ R1.1〜1.3 の出力語彙が 1 箇所に集まり、R1.5（水準整合）を**型で**担保できる。手順書の grep 語も 1 系統。
- ✅ 純データ型なので出力内容そのものを決定論テストに載せられる（R5 と相互補強）。
- ❌ wintf 層（`window_pos.rs`）から areka のモジュールは呼べない（依存方向）。
  wintf 側は既存ログ強化（Option A）と併用する**ハイブリッド必須**。

#### Option C: 起動時 1 回の「環境スナップショット」ログ＋変化点ログの二層（ハイブリッド）
- 起動時に全モニタ＋全ゴースト窓の初期状態を `info!` で 1 回だけ吐き、以降は変化点のみを専用 target で吐く。
- ✅ ログ量が有界（10 分運転で読み切れる）・R1.1 と R1.2 の性格差（静的構成 vs 変化）に素直。
- ❌ 起動後のモニタ構成変化を取りこぼす（M1 では構成変化非追随ゆえ整合はする・§1.2 の Constraint）。

> **推奨**: wintf 側は A（最小の水準/フィールド是正）、areka 側は B の軽量版（レコード型＋専用 target）、
> 全体の骨格は C。ただし**診断フェーズは修正でないため、恒久 API を増やしすぎない**線引きが要る（設計判断 D1）。

### 4.2 位置権威の是正（R3・R4）— **診断で原因確定後にのみ着手**

#### Option A: 「OS 推奨位置を採用しない」を wndproc 側で決める（wintf 修正）
- `WM_DPICHANGED` の `guarded_set_window_pos` を、`SWP_NOMOVE` を含む形（DPI 更新のみ）へ変える、
  あるいは「推奨位置を採用するか」を純関数の判断へ切り出す。
- ✅ 二重権威を**源で**断つ。areka 側の `raw` が汚染されなくなる。
- ✅ 判断分岐を純関数化すれば決定論檻に載る（`dpi_helpers.rs` と同型の先例あり）。
- ❌ **wintf は areka 専用ではない**。Per-Monitor v2 の作法上、推奨矩形の無視は他の消費者（examples・
  将来の通常窓）に影響する。`BoxStyle` を持つ窓（＝レイアウト主導窓）と持たない窓（＝ゴースト窓）で
  挙動が分かれるのが自然だが、それは**新しい暗黙契約**になる。
- ❌ 編集面が wintf へ広がる（W5 同居の観点では他 3 本と非衝突ゆえリスクは低い）。

#### Option B: areka 側で「DPI 変化後の位置を必ず自分で決め直す」（areka 修正）
- `dpi_phase_with` を「`refresh_scale` の戻り値に関わらず、`Changed<DPI>` の char 窓は必ず
  `project_anchor` を再適用する」形へ変える（＝寸の権威と位置の権威を分離）。
- ✅ 発見 (b) を構造的に消す。要件 4.1/4.2/4.6 に直接対応。
- ✅ 編集面が `frame.rs`＋`follow.rs` に閉じ、既存の単一ライター規律を継承できる。
- ❌ 「OS 推奨位置がいったん実窓に適用され、次フレームで戻る」＝**1 フレームのちらつき**が残る。
- ❌ X の権威が依然として「直前の `WindowPos.position`」のままなので、**H1（X 素通し）は解決しない**。
  X をどう決めるか（前回位置を DPI 比で写す／保存された論理位置から再射影する）という**新しい設計決定**が要る（D4）。

#### Option C: ハイブリッド（源で汚染を止め、areka で不変条件を保証する）
- (i) wintf: DPI 変化時の位置書込を「推奨位置採用可否の純判断」へ切り出す（Option A の最小形）。
- (ii) areka: `Changed<DPI>` で必ず再射影（Option B）＋**可視性不変条件**（R3.1）を
  `project_anchor` の下流に 1 本だけ足す（X 方向の work area 交差保証）。
- ✅ H1・H2・H3 の 3 仮説すべてに対応でき、要件 3.1/4.3 を同時に満たす。
- ✅ 「可視性保証」を `project_anchor` と別の純関数（例 `ensure_visible(rect, snapshot) -> PointPx`）に
  切ることで、**ドラッグ経路と DPI 経路の両方に同一関数を通せる**（既存の「T を二重化しない」規律と整合）。
- ❌ 変更面が最大。診断結果が「H1 のみ」だった場合は過剰（Req2.7「確定した機構以外を変更しない」に抵触し得る）。
- ❌ 「明示ドラッグでの画面外持ち出しは尊重する」（Boundary Context の Out of scope）ため、
  `ensure_visible` を**ドラッグ経路に通してはならない**。適用範囲の線引きが必須（D5）。

### 4.3 回帰檻（R5）の作り方

#### Option A: 既存 in-crate テストの拡張（推奨度：高）
- `follow.rs` の `mod tests`（偽 HWND＋`MonitorSnapshot`）と `frame.rs` の resnap 檻（`frame.rs:1907-2020`）に
  「混在 DPI・複数モニタ」ケースを追加。`resolver.rs:303-318` の `DPIS`／`px()` 方式をそのまま踏襲して
  **絶対 px でなく比・不変条件で判定**（R5.6）。
- ✅ 新機構ゼロ・記憶「檻に入れるのは判断分岐のみ」「x64 偽境界を優先」に完全整合。
- ❌ `WM_DPICHANGED` ハンドラ内の判断（R4.3 の是正が wintf 側なら）は in-crate では届かない。

#### Option B: wndproc の判断を純関数へ抽出して wintf 側 in-source テスト
- `dpi_helpers.rs` に「推奨矩形をどう扱うか」の純関数を追加し、`#[cfg(test)] mod tests` で全分岐を網羅。
  既存 `correct_position_for_dpi_center_preserve` のテスト群（`dpi_helpers.rs:260-389`）が完全な donor。
- ✅ R5.4「是正前コードは dpi=96 で通り 96 以外で落ちる」を素直に書ける（96 では suggested_rect が
  現位置と一致し差が出ないため、**96 通過・120/192 失敗**が自然に成立する）。
- ❌ 抽出のために wndproc の構造を触る＝診断前の先走り改造になりかねない（フェーズ順序の裁定が要る・D2）。

#### Option C: 実 DPI 実機サインオフのみ（決定論化しない）
- ❌ 記憶「決定論的テスト網羅は必達」に反する。**R5.5 が対象とするのは
  「OS が実際に提示する推奨矩形」「実モニタ列挙」だけ**であり、それ以外を実機へ逃がすのは不可。

### 4.4 レジストリ掃除（R6）

#### Option A: despawn 時フックで `GhostWindows` から除去
- `GhostWindowMarker` の `on_remove` フック、または `despawn_smoke_targets`（`main.rs:773-783`）の直後に
  `GhostWindows` を再構築／該当 scope を除去。
- ✅ 要件 6.1 に直接対応。`Monitor::on_add`（`monitor.rs:40-52`）に component hook の先例あり。
- ❌ `GhostWindows` は「scope → (char, balloon)」の対であり、片方だけ死んだ中間状態の扱いを決める必要（D8）。

#### Option B: 消費側で存在確認して正常系打ち切り
- `reconcile_reported_sizes`／`resnap_with` が `world.get_entity(e).is_err()` を見て `debug!` で skip。
- ✅ 要件 6.2/6.3 に直接対応・掃除漏れがあっても警告は出ない。
- ❌ 「レジストリに死んだ entity が残る」という状態自体は放置（要件 6.1 を満たさない）。

> **A と B は排他でない**。要件 6.1（掃除）＋6.2/6.3（警告を出さない）＋6.4（生存窓に影響しない）を
> すべて満たすには**両方**が自然。ただし B 単独で 6.1 は満たせない点に注意。

---

## 5. 工数・リスク

| フェーズ | 規模 | リスク | 一行根拠 |
| --- | --- | --- | --- |
| R1 観測増設 | **S**（1–3 日） | **Low** | 既存ログ呼出の水準/フィールド是正が主。新規機構は診断レコード型 1 つ程度 |
| R2 実機診断＋レポート | **S–M** | **Medium** | コード変更ゼロだが、**実マルチモニタ混在 DPI 実機と再現の不確実性**に依存（「再現性が微妙」）。縮退条項が保険 |
| R3/R4 修正 | **S（H1 のみ）〜 L（H1+H3+可視性不変条件）** | **Medium–High** | 確定原因次第で 4.2 の A/B/C が分かれる。C は wintf 層に及び、**「明示ドラッグは尊重」との線引き**が設計の勘所 |
| R5 回帰檻 | **S–M** | **Low** | 偽装境界・DPI 水準パラメタ化・resnap 檻の donor が全部揃っている |
| R6 掃除 | **S** | **Low** | 局所。ただし despawn の中間状態定義が要る |

**全体**: 診断が「再現せず」なら **S（掃除＋檻のみ）**、H1+H3 が確定すれば **M–L**。
最大の不確実性は**実機再現の可否**であり、それは実装難度ではなく段取りのリスクである。

---

## 6. Research Needed（設計フェーズへ持ち越す未解決）

1. **R1**: `try_tick_on_vsync()`（`window_pos.rs:259-262`）が `WM_WINDOWPOSCHANGED` の**内側**で schedule を回すため、
   `apply_window_pos_changes`（書き手B）と `emo2_frame_system`（書き手C）の**実行順が
   vsync タイミングに依存して入れ替わり得る**。どちらの `SetWindowPosCommand` が最後に flush されるかを
   実機ログで確定する必要がある（コード読解だけでは決まらない＝**非決定性そのものが欠陥候補**）。
2. **R2**: `dpi_helpers.rs:76` の `warn!("... BoxStyle not found")` が 2026-07-18 repro のログに現れていない。
   `info` 水準で出るはずの warn が無いのは (a) brief への転記漏れ、(b) 当時 `WM_DPICHANGED` 経路が
   別だった、(c) `use_bypass` 分岐が実は成立していた、のいずれか。**新診断で必ず確認する**。
3. **R3**: OS が `WM_DPICHANGED` で提示する suggested_rect の**実値**（モニタ跨ぎ時に X がどれだけ動くか）。
   Win32 の契約上は「新 DPI に合わせて拡縮した矩形」だが、混在 DPI マルチモニタでの実挙動は実測が要る。
4. **R4**: 不可視 balloon 窓の DPI 変化 → 可視化時の位置（要件 4.6）。
   `resize_window_keep_position` は位置を据え置くため、char 窓が動かないフレームでの整合が未検証。
5. **R5**: `areka-P0-position-persist`（完了済み）の `project_restore` との境界。
   本 spec が「運転中に不可視位置を作らない」側を持つとして、**保存時に既に不可視だった位置**の扱いは
   どちらの所有か（要件 Adjacent expectations が「設計フェーズで突合」と明記）。
6. **R6**: 実機診断の再現手順が確立しない場合の**縮退判定基準**（何を以て「再現しない」と結論するか）。
   運転時間・跨ぎ回数・観測すべきログ語の定量的な閾値が未定義。

---

## 7. W5 同居（4 本並走）の衝突リスク

| spec | 編集面（brief 実測） | 本 spec との衝突 |
| --- | --- | --- |
| `collision-dpi-hittest` | `emo2_boot/hit_region.rs`・`input_events/mod.rs` | **なし** |
| `choice-select-events` | `input_events/balloon.rs` | **なし** |
| `kero-balloon` | `areka-emo-present/src/balloon.rs`・`emo2_boot/assets.rs`・`placement/measure.rs` | **要注意**: roadmap は kero-balloon が `frame.rs:928`（`run_text_scale_phase`）と `frame.rs:545`（`balloon_models`）に触ると明記。本 spec も `frame.rs` の DPI 相（`:782-839`／`:985-1026`／`:1157-1228`）に触る見込み＝**同一ファイル・異ハンク**。ハンク距離は十分あるが、**先着後 rebase の申し送りが要る**（roadmap 干渉台帳の流儀） |
| `balloon-visibility`（W6） | `frame.rs`＋`spawn.rs` の可能性 | roadmap に「van(W5)⇄vis(W6)〔spawn.rs＝vis が触るかは design 次第〕」と既登記。**本 spec が `spawn.rs` を触る場合（R6 の掃除フック）は W6 へ申し送り** |

**本 spec の想定編集面**: `crates/areka/src/placement/{follow,spawn,mod}.rs`・`crates/areka/src/emo2_boot/frame.rs`・
`crates/areka/src/main.rs`・`crates/wintf/src/ecs/window_proc/{window_pos,dpi_helpers}.rs`・
`crates/wintf/src/ecs/layout/systems/monitor_systems.rs`。
**wintf 側は W5 の 4 本いずれも触らない**＝そこは完全に単独所有。
`placement/measure.rs` は kero-balloon の所有ゆえ**本 spec は触らない**こと。

---

## 8. 設計判断項目（要件ディスカッションへ送る・番号付き）

1. **D1｜診断観測の恒久性**: R1 の観測増設は「診断が終わったら外す一時計測」か「恒久的な運用ログ」か。
   恒久なら専用 target（`areka::placement::trace` 等）と水準規約を決める必要がある。一時なら
   Requirement 1.4 の手順書は「そのビルドでのみ再現可能」になる（第三者再実行性の解釈が変わる）。
2. **D2｜フェーズ順序と「先走り改造」の線引き**: R5 の檻を Option B（wndproc 判断の純関数抽出）で作る場合、
   抽出そのものが修正フェーズの先取りになる。Req2.7「確定した機構以外を変更しない」との整合をどう取るか
   （抽出＝挙動不変リファクタなので許容、とするかどうか）。
3. **D3｜OS 推奨位置の扱いをどの層で決めるか**: wintf（源で断つ・4.2 Option A）か areka（下流で必ず上書き・Option B）か。
   wintf 側にすると `BoxStyle` 有無で挙動が分岐する新しい暗黙契約が生まれる。
4. **D4｜DPI 変化時の X 座標の権威**: 現行は「直前の `WindowPos.position`（＝OS 推奨値）素通し」。
   あるべき姿の候補は (a) 直前の areka 確定位置を保持（OS 値を採らない）、(b) 旧位置を DPI 比で写像、
   (c) 保存された論理位置から再射影。要件 4.1/4.2 は「接地点保存」しか言っておらず、
   **X をモニタ跨ぎでどう定義するか**は未定（同じ絶対 X か、同じモニタ内相対位置か）。
5. **D5｜可視性不変条件（R3.1）の適用範囲**: 「明示ドラッグでの画面外持ち出しは尊重」（Out of scope）ため、
   clamp 相当の保証を **DPI 変化・自動再配置経路にのみ**通し、ドラッグ経路には通さない設計が要る。
   `project_anchor` の内側に入れると両方に効いてしまう＝**別の純関数として下流に置く**か、
   経路タグを引数に取るか。
6. **D6｜`work_area_for_window` の最近傍フォールバック**: 現在「どのモニタにも属さない窓」に対して
   最近傍を返す（`follow.rs:1146-1159`）ため、**異常が異常として観測されない**。
   Requirement 3.2/3.3 の「警告として記録する」を満たすには、
   最近傍フォールバックの発火自体を観測点にするか、可視性判定を別関数で持つかを決める必要がある。
7. **D7｜`refresh_scale` の `None` を位置経路から切り離すか**: 4.2 Option B の中核。
   presenter の戻り値契約（`areka-emo-present` 所有）を変えずに frame 側だけで解決できるか、
   それとも presenter に「位置再射影が必要か」を別途問う口を足すか（＝クレート境界を跨ぐ設計判断）。
8. **D8｜`GhostWindows` 掃除の粒度**: scope 単位（char/balloon の対がまとめて消える前提）か entity 単位か。
   片方だけ despawn された中間状態を「不整合として warn」するか「正常な部分破棄」とするか。
   また掃除の駆動点は component hook（`on_remove`）か明示呼出（`despawn_smoke_targets` 直後）か。
9. **D9｜診断が「再現せず」だった場合の完了条件**: Requirement 3・4 を「現行挙動が既に条件を満たすことの検証」
   として消化する（Boundary Context の縮退条項）とき、**その検証は決定論檻で足りるか実機サインオフが要るか**。
   本書の §1.1 の発見 (a)(b)(c) は「実機で消失が再現しなくても要件 3.1/4.3 は未充足」を示すため、
   **縮退しても修正が残る可能性が高い**——この解釈の是非を裁定する必要がある。
10. **D10｜モニタ構成変化（`MonitorSnapshot` セッション固定）への態度**: Requirement 3.2 は
    「食い違いを警告し、不可視位置へ動かさない」までを求めるが、追随実装は Out of scope（M1 非対象）。
    「警告だけ出して動かさない」で要件充足とするか、診断で H6 が確定した場合のみ最小追随を入れるか。

> 以下 D11・D12 は要件ディスカッション（2026-07-31）で追加収集した設計判断項目。

11. **D11｜Requirement 1.2 の「経路」タグの語彙と配管**: 「変化を引き起こした経路」を何の集合として定義するか
    （例: ドラッグ／再スナップ／DPI 再射影／位置復元／spawn 初期配置／OS 推奨矩形書込）、およびそのタグを
    単一ライター `enqueue_window_set_pos`（`follow.rs:1009-1080`）へどう引き回すか（引数追加か呼出元ごとの
    ラッパか）。タグ集合は Requirement 2.4「最終位置を書き込んだ主体を名指しで記録する」の充足語彙でもあるため、
    **診断レポートの結論語彙と一致させる**必要がある。D5（可視性不変条件を経路で分岐させる案）とも結合する。
12. **D12｜モニタ列挙の三重化と Requirement 1.1 の出力点**: 列挙は wintf `initialize_layout_root`／areka
    `prepare_ghost_windows`／areka `MonitorSnapshot`（`main.rs:645`）の**3 箇所独立**（§1.2）。
    1.1 の出力をどこに置くか——(a) wintf の列挙点に一本化、(b) areka の `MonitorSnapshot` 構築点に置く、
    (c) 3 箇所すべてを出して**相互の食い違い自体を観測点にする**（＝Requirement 3.2 の「食い違いを警告」を
    構成変化検出なしに部分的に満たせる可能性がある）。(c) は D10 の裁定に影響する。

### 8.1 要件ディスカッション再精査による自明裁定（2026-07-31）

再推論の結果、以下は開発者裁定を要さず一意に決まると判定した（根拠付き・設計フェーズは配管のみを扱う）。

- **D1｜恒久ログで確定**。診断観測は**恒久コード**とし、専用 target（既定 OFF・診断手順が `RUST_LOG` で点灯）で
  水準でなく target で切り分ける。根拠: R1.4「第三者が同一手順を再実行できる」＋ R5.5 の恒常実機サインオフは
  **main に観測が焼き込まれていること**を要求する（一時計測ビルドでは第三者再実行性が崩壊）。
  記憶〈ログ無し失敗経路の禁止〉〈実機サインオフ＝RUST_LOG grep 定石〉とも整合。target 命名・配管は D11（設計）。
- **D2｜議題1へ統合**。「挙動不変リファクタ・観測増設は Req2.7 の『変更』に数えない」を 2.7 改稿に明文化する
  （記憶〈檻に入れるのは判断分岐のみ・純関数化で全網羅〉が純関数抽出を檻の前提として要求している）。
- **D4｜X は「直前の areka 確定接地点の物理 X を保持」（案a）で確定**。Per-Monitor DPI aware の screen 座標は
  **物理 px** であり、DPI 変化はモニタ配置（物理座標系）を動かさない。ゆえに要件 4.1「接地点を変化の前後で保つ」の
  文言自体が「接地点の物理 X/Y 不変（Y は work area 下端スナップ規約に従う）」を一意に含意する。
  (b) DPI 比写像は物理座標系では無意味・(c) 論理位置からの再射影は position-persist（復元経路）の所有。
  残る設計判断は「OS 推奨値をどの層で棄却するか」（D3/D7）のみ。
- **D5｜要件レベルでは裁定済み**。3.1 の「ユーザーの明示的なドラッグ以外の要因によって」が適用範囲を確定している。
  実装配置（`project_anchor` の外に純関数として置く／経路タグ分岐）は設計フェーズへ（D5 を設計判断側へ移管）。
- **D10｜裁定不要**。3.2＋Adjacent expectations で方針は完結している（警告＋不可視位置へ動かさない＝充足。
  追随実装は H6 が実機で確定した場合に限り最小範囲）。
