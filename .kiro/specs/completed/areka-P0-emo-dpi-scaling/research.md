# ギャップ分析: areka-P0-emo-dpi-scaling（DPI追従レンダリング基盤）

> **分析日**: 2026-07-24（validate-gap フェーズ）
> **コード基準**: worktree `kiro-gpu-test-crash-ec3b84`（`wintf-gpu-test-crash` PR#87 マージ済み＝b0de116 系譜。brief 追記㊹の行アンカーを本日実測で再検証済み）
> **入力**: requirements.md（確定）／brief.md（追記㊵㊹）／steering（product・tech・structure）／ukadoc MCP

---

## 1. 現状調査（Current State Investigation・全アンカー実測検証済み）

### 1.1 k=1.0 ハードワイヤの実体（変更の中心点）

| 資産 | 実測 | 意味 |
|---|---|---|
| `crates/areka-emo-present/src/presenter.rs:126` | `const CURRENT_COMPOSE_SCALE: f32 = 1.0;` | 廃止対象のコンパイル時定数（R1.2） |
| `presenter.rs:435` | `scale: CURRENT_COMPOSE_SCALE`（`text_slot_view` 内・唯一代入） | 照会契約への供給点 |
| `presenter.rs:116-122` | `TextSlotView::scale()` doc が「**将来 DPI スケーリング導入時の変更点はここ 1 点**」と自己宣言 | design 宣言済みの単一変更点 |
| `presenter.rs:52-77` | `PresentTarget` struct — **scale フィールド無し**。`window: Entity`（:62）は保持 | per-target k の置き場（Missing） |
| `presenter.rs:186` | `apply(&mut self, world: &mut World, cmd)` | `world.get::<DPI>(target.window)` を読める位置に既にいる（配線コストほぼゼロ） |
| `presenter.rs:457-461` | `hit_region(x, y)` — 点を無変換で純関数へ（「k=1.0 契約」doc :452-453） | ÷k は W5 `collision-dpi-hittest` 領分（本仕様不触・R7.9） |

### 1.2 合成パイプライン（Strategy A の作業面）

- **extent 計算**: `crates/areka-emo-compose/src/plan.rs:451` `compute_extent`（pub(crate)・native px の (0,0)-union・k 乗算なし・i64/u32 整数のみ）。
- **転写**: `crates/areka-emo-compose/src/blit.rs:69` `execute` — **1:1 整数 premultiplied SourceOver（`div255` 整数式）・リサンプラ不在**。モジュール規約として「**浮動小数を経路に一切持ち込まない**（決定性・要件 10.2）」を明文で背負う。→ Strategy A のリサンプラは整数/固定小数点で設計しないと既存の決定性規約と衝突する（重要制約）。
- **キャッシュキー**: `crates/areka-emo-present/src/cache.rs` `ComposeKey{surface, binds, pattern}`（W3 で pattern 拡張済み）。**k はキーに含まれない** → Strategy A（k 別の合成結果が生じる）では k のキー参加 or 再スケール時 invalidate が必須（Missing）。
- **マスク**: `cache.insert` 時に composed バイトから 1 回生成（presenter.rs:237-241 周辺）。Strategy A なら **マスクも自動的に k 寸**＝AlphaMask 座標契約（物理 px）が自然整合。

### 1.3 表示・供給面・窓の寸法連鎖（下流追従の実体）

- **swapchain**: `crates/areka-emo-present/src/chain.rs:178-194` — `upload()` が **ComposedSurface の外形変化を検知して `ResizeBuffers`（:180-188）＋ source_tex/staging 再作成**。→ Strategy A では合成外形が k 倍になるだけで **swapchain は自動追従**（brief の主張を実測確認）。
- **visual**: `crates/areka-emo-present/src/mount.rs:63-72` `physical_arrangement` — `LayoutScale::default()`（=1.0）＋物理 px 直接指定。`SpriteVisual::SetSize` は物理 px（:112-117）。`set_bounds` は ShowSurface 成功時に毎回呼ばれる（presenter.rs:370）→ Strategy A ではここも自動追従。Strategy B ではここに k 分散が必要。
- **窓寸の源**: `crates/areka/src/placement/measure.rs:62` `measure_scope_sizes` — scope0=id0／scope1=id10／balloon surface0 を bind なし合成して**原寸 SizePx** を返す（per-scope ループ :91-127・balloon_size は全 scope 共通の単一値 :89）。→ k 倍後の窓寸はここで吸収する（W4 事前割当契約・R3.4/R7.6）。**balloon_size が per-scope になり得る席**（ループ内 `ScopeInput{scope, char_size, balloon_size}`）を関数分解で潰さないこと（R7.8・kero-balloon 申し送り）。
- **実行時リサイズの正規経路（重要・再利用可能）**: `crates/areka/src/placement/follow.rs:553` `resize_window_to`（アンカー保存リサイズ・`surface-resize-resnap` 完了資産）＋ `on_surface_size_changed`（:667・`Changed<Anchored>` consumer）＋ WindowPos bypass ミラー（:768）。→ **R4（DPI 変化の動的追従）の窓側は新設不要でこの経路の呼び出しで成立し得る**。spawn.rs は不触のまま（R7.6）。
- **本番結線**: `crates/areka/src/main.rs:552` `prepare_ghost_windows` → `spawn_ghost_windows`（spawn.rs・不触）→ `crates/areka/src/emo2_boot/`（`frame.rs` `Emo2Wiring`/`run_attach_phase`・`adapter.rs` `DisplayCommand→PresentCommand`）。R6 実機観測はこの本番経路で行う。

### 1.4 wintf の既存 DPI 機構（consume するだけ・新規依存なし＝要件前提の実在確認）

- `crates/wintf/src/ecs/window/dpi.rs:21-28` `DPI` component（dpi_x/dpi_y u16・SparseSet）・`scale_x/y()`（**:61-66**・÷96.0）・`to_physical_*`（:114-128・**round half away from zero** の丸め実装が既にある——本仕様の丸め規約候補）。
- 実値取得: `window_handle.rs:221-244` — 窓生成時 `GetDpiForWindow` 実値で事前挿入値を補正。取得失敗（=0）は 96 縮退（既存にも縮退前例あり）。
- ライブ更新: `window_proc/window_pos.rs:285-346` `WM_DPICHANGED` — **DPI component 直接更新（`Changed<DPI>` 発火）**→ `DpiChangeContext` → SWP_NOSIZE 位置移動。サイズは ECS レイアウトが算出する方式（doc :348-351）。
- **wintf は k≠1.0 レンダリング実績あり**: `taffy_systems.rs:210-237`（`Changed<DPI>` → Window `Arrangement.scale`）→ `render.rs:111` `dc.SetTransform`。ただし **emo は mount.rs で意図的にバイパス中**（論理/物理混在事故の構造的排除）。
- モニタ DPI: `monitor.rs:123-126` `GetDpiForMonitor(MDT_EFFECTIVE_DPI)`（採寸時の初期 k 導出に利用可能）。

### 1.5 テスト基盤（R5 の前例）

- **オフスクリーン readback 決定論テストの前例が同 crate に既在**: `presenter.rs` in-crate tests（`make_world_with_gpu` :524-537・golden バイト一致 :629-667）・chain.rs/mount.rs tests。合成→upload→read_back のバイト恒等が檻の型。**k× 拡大の檻はこの型の拡張で書ける**。
- **純関数テストの前例**: blit（log_firing_tests.rs）・measure（measure.rs in-crate）・follow（follow.rs in-crate・偽 work area 注入）。k 導出・丸め・extent 導出はこの型（GPU 不要・全網羅）。
- **共有 GPU オーナースレッド fixture**: `crates/wintf/tests/graphics/common/mod.rs:75` `on_gpu_owner_thread`（実在確認）。R5.3 は「**wintf tests/graphics に新設する** WUC テスト」への制約。
- **注意（Research Needed #7）**: areka-emo-present の in-crate tests は各テストスレッドで `WucGraphicsResource::new`（=Compositor 生成）しており、これは**別テストバイナリ（別プロセス）だから現状緑**。本仕様のテスト増分を emo-present in-crate に置く限り既存パターン踏襲で可、**wintf 側に置くなら fixture 経由必須**という配置判断を design で明示すること。

### 1.6 ukadoc 正典調査（author_dpi・外部研究）

ukadoc MCP 実照会の結果（設計論点 1 の一次資料）:

- **`seriko.dpi,推奨DPI`**（shell descript.txt・SSP 2.7.21+）: 「このシェルで推奨する画面の DPI 値。**何も指定しなければ Windows 標準の 96 固定**」。対照表: 100%→96／125%→120／150%→144／175%→168／200%→192。
- **`dpi,推奨DPI`**（balloon descript.txt・SSP 2.7.21+）: 同上（バルーン側の author_dpi）。
- **emo2 fixture 実測**: `crates/pilot/examples/shiori-host-32/fixtures/emo2/` の shell/ghost descript.txt に `seriko.dpi` 宣言**なし** → 既定 96 適用＝emo2 では **k = monitorDPI / 96 = wintf `DPI::scale_x()` と一致**。
- **周辺語彙（本仕様 out-of-scope・将来乗算因子）**: SSP の実効拡大率は「ユーザー拡大率 × `\![set,scaling]` タグ × SERIKO scaling アニメ×…」の**乗算合成**（surfaces.txt `scaling` 描画メソッド・`OnShellScaling`/`OnBalloonScaling` イベント・property `currentghost.scope(ID).scaling`／`currentmonitor.dpi`）。→ [defer-canon 規律] 本仕様の k は将来この乗算列の 1 因子になる。**per-target scale を「単一の最終合成値」として保持しつつ、導出式に因子が増やせるシーム**にしておくと語彙を潰さない。
- **パーサ現状**: areka-parsers に dpi キーのモデル化は**皆無**（grep 0 件）。ただし `crates/areka/src/placement/source.rs:102` `load_descript_source` が **shell descript.txt を生 KV（BTreeMap・後勝ち）で既に読んでいる** → `seriko.dpi` は**パーサ改造なしで** `shell_kv` から読める。balloon descript の `dpi` は同様に kv 読みを足すか balloon parser の additive 拡張（小）。

---

## 2. 要件実現可能性マップ（Requirement-to-Asset Map）

| 要件 | 既存資産 | ギャップ | タグ |
|---|---|---|---|
| R1 k 導出・照会契約 | wintf `DPI`（実値＋ライブ更新）・`scale()` 単一変更点宣言・presenter が window Entity と `&mut World` を保持 | per-target k フィールド／k 導出純関数（author_dpi 分母・失敗時 k=1.0 縮退ログ）／`CURRENT_COMPOSE_SCALE` 廃止 | **Missing（小）** |
| R1.5 窓ごと k | `PresentTarget` が target=窓 単位で分離済み（構造は既に per-target） | k の実装置場のみ | Missing（小） |
| R2 k× 実拡大 | 合成は整数 1:1・**リサンプラ/変換経路が皆無** | Strategy A: blit/plan への k×（リサンプラ新設）or Strategy B: WUC transform 分散 | **Missing（本丸）** |
| R2.3/2.4 全要素一貫 k | 拡大点が「合成結果 or 合成計画」の単一漏斗（element/SERIKO/mayuna は上流で合成済み） | 単一 k 適用点を守れば自動成立 | Constraint（有利） |
| R2.5 丸め規約 | `DPI::to_physical_*` の round half away from zero が既存前例 | 単一規約の宣言と全消費点の統一 | Unknown（design 確定） |
| R3.1-3.2 窓/合成先追従 | chain.rs が composed 外形へ自動 ResizeBuffers・mount set_bounds 毎回呼び | A なら自動追従／B なら明示分散 | Constraint（戦略依存） |
| R3.3-3.4 配置採寸 | `measure_scope_sizes`（窓寸の単一の源）・spawn.rs は SizePx 消費のみ | 採寸値の k 倍（採寸時の k は spawn 前＝モニタ未確定 → 初期 k の根拠が要る） | **Unknown（Research #5）** |
| R4 動的追従 | `WM_DPICHANGED`→`Changed<DPI>` ライブ更新・`resize_window_to`（アンカー保存）・`on_surface_size_changed` | emo 側の再導出トリガ（`Changed<DPI>` 観測→再 Show/再合成）・**cache キーに k 不在**・照会値の一貫更新 | **Missing（中）** |
| R5 決定論檻 | readback golden 一致テスト・純関数テスト・`on_gpu_owner_thread` fixture すべて実在 | k 版テストの増分のみ（新機構不要） | Missing（小） |
| R6 実機観測 | `collision-probe.rs`（GetClientRect vs `surface_size()`×`scale()` 照合の donor）・AREKA_APP_SMOKE_EXIT_MS・絶対パス起動知見 | k 導出値・適用寸の info ログ新設・2 水準観測手順 | Missing（小） |
| R7 非退行・境界 | k=1.0 時は既存経路と等価（R1.3/R7.2）・spawn.rs 不触構造は既に成立 | measure 関数分解時の per-scope balloon 席保全（R7.8） | Constraint |

**複雑度シグナル**: 外部統合なし・新規依存なし。中核は「アルゴリズミック（リサンプラ＋丸め）＋既存配管への k 縫い込み」。

---

## 3. 実装アプローチ選択肢（design ディスカッションへの情報提供・決定はしない）

### Option A: emo-compose で k× 鮮明ラスタ（Strategy A・拡張中心）

合成段で k を焼き込む。`compute_extent` を round(k×native) 化し、blit（またはその直後）に**リサンプラを新設**。swapchain/visual/AlphaMask/窓寸は composed 外形従属ゆえ**自動追従**（chain.rs:178-194 実測確認済み）。

- 変更面: `plan.rs`/`blit.rs`（emo-compose）・`presenter.rs`（per-target k・cache キー×k）・`measure.rs`（採寸×k）。
- ✅ emo 思想（自前合成・鮮明性・[[areka-emo-own-compositor-atlas]]）と一致。マスクも k 寸で生成され **AlphaMask 物理 px 契約が無修正で整合**（W5 の ÷k は collision 領域のみに縮む）。
- ✅ 拡大の正しさをオフスクリーン readback 決定論 unit で全網羅できる（R5.1 と最も好相性）。
- ❌ リサンプラの決定性設計が必要（blit.rs は「浮動小数を経路に持ち込まない」規約 → 整数/固定小数点補間 or 「合成は f32 可・出力バイトは決定的」への規約改訂のどちらかを design で確定）。
- ❌ k 別合成はメモリ・再合成コスト増。**ComposeKey に k が必要**（または k 変化時 invalidate_all）。SERIKO 毎コマ再合成が k 倍画素で走る。
- sub-variant A2（合成は native のまま、present 段で composed→k 倍リサンプル）: compose の決定性規約を触らず、リサンプラを emo-present 側に置ける。cache は native のまま＝キー拡張不要（k 倍転写だけ再実行）。design で A1（compose 内）と比較検討の価値あり。

### Option B: WUC transform で完成 1 枚を拡大（Strategy B・低コスト）

合成・swapchain・マスクは native px のまま、`mount.rs` の visual（`Arrangement.scale` or `SpriteVisual.Scale`）＋窓寸（measure×k・resize_window_to）＋`scale()` 照会に k を分散。wintf 既存の `Arrangement.scale`→合成 transform 経路（taffy_systems.rs:210-237 前例）を emo 側でも使う。

- ✅ 実装最小・GPU stretch でランタイムコスト最小・cache キー不変・compose 不触。
- ✅ DPI 変化時の再スケールが「visual scale と窓寸の更新」だけで済む（再合成不要）。
- ❌ bitmap-stretch の甘い拡大（鮮明性で emo 思想と不一致）。
- ❌ **AlphaMask が native 寸のまま窓 client が k 倍** → wintf hit-test の物理 px 契約と即座に不整合（クリック透過・当たり判定が壊れる）。マスクの k 倍再サンプル or hit-test 側 ÷k が**本仕様内で**必要になり、「÷k は W5」の境界（R7.9）を実質侵食する。**この整合コストを織り込むと A との差は縮む**（gap 分析としての最重要注意点）。
- ❌ `mount.rs` の「論理/物理混在事故の構造的排除」doc 方針への逆行。

### Option C: 契約先行の段階実装（Hybrid）

Phase 1: per-target k の第一級化（導出・保持・`scale()` 照会・k=1.0 恒等パス・失敗縮退ログ・純関数檻）＝レンダリング方式に依存しない R1/R5/R7 部分を先に着地。Phase 2: R2/R3/R4 の実拡大を A（推奨方向）で実装。

- ✅ 「観測可能な挙動は方式選択によらず成立」という要件構造（Introduction 明記）と同型。task 分割が自然。レビュー単位が小さい。
- ✅ Phase 1 完了時点で下流 `collision-dpi-hittest` の契約面（scale() が k を返す）が先に固まる。
- ❌ Phase 1 単独では R2/R6 は未成立（k≠1.0 で照会だけ動く中間状態を作らないよう、phase 間で「照会値=実適用 k」不変条件（R1.2/R4.2）を保つ順序設計が要る）。

### 工数・リスク

| Option | Effort | Risk | 根拠 |
|---|---|---|---|
| A（A1 compose 内） | **L**（1-2週） | **Medium** | リサンプラ決定性設計＋cache キー拡張＋再スケール機構。パターンは既存（readback 檻・整数 blit）に接続 |
| A2（present 段リサンプル） | M-L | Medium | compose 規約不触・cache 不触で A1 より局所。転写 2 段のコスト増 |
| B | **M**（3-7日） | **Medium-High** | 実装は小さいが**マスク/ヒット整合の隠れコスト**が W5 境界を侵食し、鮮明性で正典思想と不一致 |
| C（+A） | L | **Low-Medium** | 総量は A と同じだが中間検証点が増えリスク分散。推奨検討軸 |

---

## 4. Research Needed（design フェーズへ持ち越す未知）

1. **author_dpi の読み取り経路**: 正典は `seriko.dpi`（shell）／`dpi`（balloon）・既定 96（ukadoc 確認済み・emo2 は宣言なし=96）。`load_descript_source` の `shell_kv` から読むか、balloon descript の kv 読み・parser additive 拡張のどれか。**将来のユーザー拡大率×タグ scaling 乗算列の因子シーム**（完全語彙・縮退シーム規律）をどう予約するか。
2. **k 導出規約**: 整数段階（96/120/144/168/192 表）か連続か。X/Y 独立（`dpi_x/dpi_y`・SSP scaling も横/縦独立）か単一スカラーか——要件文言は「単一の k」だが wintf DPI は 2 軸保持。リサンプラ選択（nearest/bilinear）と連動。
3. **丸め規約の単一権威**: 既存前例 `DPI::to_physical_*`（round half away from zero）を採るか。round(k×native) の適用点（extent・窓 client・visual bounds）の全列挙。
4. **cache と再スケール**: ComposeKey への k 参加 vs `Changed<DPI>` 時 invalidate_all＋再 Show。SERIKO 進行中（R4.3）のコマ切替との競合順序。emo2_boot frame system での `Changed<DPI>` 観測方式。
5. **採寸時の初期 k**: `measure_scope_sizes` は spawn 前＝窓もモニタ確定も無い時点で走る。初期 k の根拠（primary モニタ `GetDpiForMonitor` か、spawn 後の `GetDpiForWindow` 実値で `resize_window_to` により自己補正するか）。後者なら「measure は native、初回表示で k 追従」という順序も選べる。
6. **Strategy B 採用時のマスク整合**: AlphaMask native 寸 vs k 倍 client の不整合の解消コスト見積り（B を選ぶ場合のみ・W5 境界侵食の明示）。
7. **テスト配置と WUC 2 個目 Compositor**: 新設 graphics テストの置き場（areka-emo-present in-crate＝既存パターン・別プロセス／wintf tests/graphics＝`on_gpu_owner_thread` fixture 必須）の振り分け基準を design で明文化（R5.3）。
8. **実機 2 水準の観測手段**: 単一実機で 125%/200% を切り替える手順（OS 表示スケール変更→WM_DPICHANGED 経路と重なる）と、ログ決定論判定（k 導出値・GetClientRect・scale() の info ログ様式）。

---

## 5. design フェーズへの推奨（情報提供・最終決定は開発者）

- **方向性**: emo 思想（鮮明ラスタ・自前合成・マスク物理 px 契約の無修正整合）と R5 決定論檻への適合から **Option A 系（C の段階分割で A1/A2 を比較確定）** が有力。B は「マスク/ヒット整合の隠れコスト」を必ず併記して比較すること。
- **鍵となる先行決定**: (a) k 導出規約（整数段階 or 連続・単軸 or 2 軸）→ リサンプラと丸めが従属、(b) cache キー×k、(c) 採寸時初期 k の根拠（Research #5）。
- **不変条件**: 「照会値 `scale()` ＝ 実適用 k」（R1.2/R4.2）を全 phase で保つこと。k=1.0 で既存テスト期待値不変（R7.2）が回帰の錨。
- **境界遵守**: spawn.rs 不触（窓寸は measure 源＋`resize_window_to` 再利用で吸収）・`measure_scope_sizes` 分解は per-scope balloon の席を保全・÷k は書かない。

---

## 6. 追記（2026-07-24 要件ディスカッション #1 開発者裁定）

SSP の乗算列（ユーザー拡大率×`\![set,scaling]`×SERIKO scaling・モニタ非依存固定）は**輸入しない**。areka は現代的 DPI 運用として **最終拡大率 = アプリ管理拡大率 × DPI 由来係数 k（モニタ DPI ÷ author_dpi）** の 2 因子モデルを採る（例: アプリ 200% × モニタ 200% = 最終 400%。モニタ間移動で最終拡大率が変化する＝SSP と真逆の思想）。本仕様はアプリ管理拡大率を 1.0 固定の縮退シームとして予約し、実設定手段は将来 spec（追跡の要否は別セッション棚卸しで裁定）。§1.6 の SSP 周辺語彙は「輸入対象」ではなく「写像し得る将来因子の参考資料」へ位置付けを変更。Research #1 のシーム設計はこの 2 因子モデルを前提に確定すること。

---

## 7. 設計フェーズ追記（2026-07-24 kiro-spec-design・discovery 再検証＋設計決定の記録）

### 7.1 Discovery 再検証（light discovery・アンカー実測）

§1 の全主要アンカーを design 当日に再実測し**全一致**を確認した（`CURRENT_COMPOSE_SCALE` presenter.rs:126／`TextSlotView.scale` 唯一代入 :435／`scale()` 単一変更点宣言 :116-122／`PresentTarget` scale 無し :52-77／`compute_extent` plan.rs:451／blit.rs 整数規約／chain.rs `upload` の外形変化 `ResizeBuffers` :178-194／mount.rs `physical_arrangement`＝物理 px 直指定／`measure_scope_sizes` measure.rs:62・per-scope ループ／`resize_window_to` follow.rs:553＋単一ライター `enqueue_window_set_pos` :729／wintf `DPI` dpi.rs（`scale_x/y` :61-66・`to_physical_*`＝round half away from zero :114-128）／`on_gpu_owner_thread` wintf tests/graphics/common/mod.rs:75）。追加確認:

- **wintf `enumerate_monitors()`（monitor.rs:173・pub）＋ `Monitor{ dpi, is_primary }`** が既に公開 → 採寸時の初期 k₀ 導出（Research #5）は **wintf 改造ゼロ**で consume できる。
- `load_descript_source`（source.rs:102）の `shell_kv` は生 BTreeMap → `seriko.dpi` はパーサ改造なしで読める（§1.6 確認どおり）。
- `ComposeCache` は容量 1 スロット・`ComposeKey{surface_id, binds, pattern}`（cache.rs:51-55）・`insert` がマスクを 1 回生成（:87-109）。
- `Emo2Wiring`／`run_attach_phase`（frame.rs）は presenter を直接所有し `attach_target` を 2 箇所（shell :391／balloon :432）で呼ぶ。`anchor_changed_system`（follow.rs:665）が `Changed<T>` 観測の `Local<SystemState>` 先例。

### 7.2 設計決定（D1〜D10・design.md 本文が正本、ここは根拠ログ）

- **D1 author_dpi**: shell=`seriko.dpi`／balloon=`dpi`・既定 96（ukadoc 正典・§1.6）。読取は既存生 KV（shell_kv／balloon descript の lenient KV 読み）＝パーサ改造なし。不正値・0 は warn＋96 縮退。emo2 は宣言なし＝96。→ Research #1 解消。
- **D2 k 導出規約**: **連続・単一スカラー・有理数表現 `ScaleRatio{num, den}`（既約正準）**。`k = app_scale(1/1 固定) × window_dpi_x / author_dpi`。dpi_x≠dpi_y は warn＋dpi_x 採用。整数段階は不採用（R2.2 の 125%→約1.25 倍が要件文言）。**有理数化により f32 を画素経路から排除**（blit.rs「浮動小数を経路に持ち込まない」規約と無衝突）・cache キー等価も厳密。f32 は照会契約（`scale()`）の出口ビューのみ。→ Research #2 解消。
- **D3 拡大方式＝Strategy A2（present 段リサンプル）**: 合成（plan/blit）は native 整数のまま**不触**。presenter が合成結果を k× リサンプルした「表示用サーフェス」を cache エントリとして保持し、以後の upload／mask／set_bounds／read_back は従来コードのまま k 寸法へ自動追従（chain の外形追従 :178-194・mount set_bounds 毎回呼び :370 実測済）。B 却下（マスク native 寸 vs client k 倍の不整合が W5 境界を侵食＋bitmap-stretch の鮮明性欠如＋mount.rs 方針逆行）。A1 却下（compose 公開 API とモジュール規約 10.2 への侵食が大きく、emo-compose の決定性檻を再検証させる）。リサンプラ実体は `ComposedSurface` 内部バイトへ到達する必要から **emo-compose の新設 `scale.rs`** に置く（合成経路 plan/blit は不触）。
- **D4 丸め規約**: **round half away from zero**（既存 `DPI::to_physical_*` と同規約）を単一権威 `scaled_extent`（emo-compose scale.rs）に一本化。非ゼロ入力は最小 1px（0 化クリップ禁止・R2.5）。全消費点（リサンプル出力外形・窓 client・visual bounds・採寸 k 倍）が同関数を通る。→ Research #3 解消。
- **D5 リサンプラ**: 整数固定小数点 **bilinear（premultiplied BGRA ドメイン・α 込み）**。座標写像は num/den 有理演算＋固定小数点で完全整数・決定論。`k=1/1` は恒等（バイトコピー）＝既存 golden 不変（R7.2）。
- **D6 cache と再スケール**: `ComposeKey` へ **scale（既約有理）をキー参加**。エントリの `composed` は「k 適用済み表示用」・`mask` はその bytes から生成（既存 insert コード不変）→ AlphaMask 物理 px 契約が無修正整合。DPI 変化＝キー相違＝ミス→再合成＋再サンプル（稀イベントの再合成コストは許容・容量 1 スロット維持）。→ Research #4 解消。
- **D7 採寸時の初期 k₀**: `enumerate_monitors()` の primary モニタ DPI ÷ author_dpi（shell/balloon 各々）。primary 不明・失敗は 96 相当＋error ログ（R1.4 と同型縮退）。窓生成後は **窓 DPI が正**（`GetDpiForWindow` 実値補正 → `Changed<DPI>` → D8 の reconcile が自己補正・`resize_window_to` のべき等 skip で k₀ 一致時は無振動）。→ Research #5 解消。
- **D8 動的追従**: emo2_boot frame へ **`run_dpi_phase`** を新設（`Changed<DPI>` を `Local<SystemState>` 先例で観測）→ `EmoPresenter::refresh_scale`（保持した最終 show 入力＝last_show で再表示・照会値と実表示の一貫更新）→ 窓寸 reconcile（char 窓＝`resize_window_to` 再利用／balloon 窓＝follow.rs への **additive** ラッパ `resize_window_keep_position`＝私有 `enqueue_window_set_pos` の位置維持形。単一ライター規律を迂回しない）。「照会値＝実適用 k」不変条件は applied の更新点を表示成立点のみに限定して保つ。
- **D9 テスト配置の振り分け基準（R5.4 明文化）**: (a) 純関数（k 導出・既約化・丸め・リサンプラ・author_dpi parse・採寸 k 適用）＝各 crate in-crate・GPU 不要・全網羅。(b) GPU readback 檻＝ **areka-emo-present in-crate**（既存 `make_world_with_gpu` 型・別テストバイナリ＝別プロセスゆえ 2 個目 Compositor AV と無縁・R5.3）。(c) **wintf tests/graphics へは wintf 自身の資産を檻に入れる場合のみ**新設し、その場合は `on_gpu_owner_thread` fixture 経由必須——本仕様は wintf 改造ゼロゆえ新設なし。→ Research #7 解消。§1.5 の注意と一致。
- **D10 実機 2 水準観測**: `apply_show` 成功点の info ログ（target/`k`（num/den・f32）/author_dpi/window_dpi/native 寸/scaled 寸）＋ collision-probe 型の GetClientRect 照合。OS 表示スケール 125%→200% を切替えた 2 回の有界起動（`AREKA_APP_SMOKE_EXIT_MS` 有界 auto-exit・絶対パス起動・RUST_LOG grep 決定論判定）。→ Research #6（B 不採用で消滅）・#8 解消。

### 7.3 Synthesis（3 レンズの記録）

- **一般化**: 「最終拡大率＝アプリ管理拡大率×k」を単一型 `ScaleRatio` の乗算として写像（将来因子は有理係数の乗算で増やせる・語彙温存・§6 裁定準拠）。丸め・k 倍寸法の全消費点を `scaled_extent` 単一権威へ一般化。
- **Build vs Adopt**: wintf `DPI`/`WM_DPICHANGED`/`enumerate_monitors`/`resize_window_to`/`ComposeCache`/`AlphaMask::from_pbgra32` を全面 adopt（新規 crates.io 依存ゼロ・R7.3）。自作は有理スケール型＋整数 bilinear のみ（D2D/GPU stretch は決定論 readback 檻と Strategy B 却下理由により不適）。
- **単純化**: 新 ECS component なし・新 PresentCommand variant なし（DPI 追従は UI 側直呼び `refresh_scale`）・cache は容量 1 のまま・スケール専用の再スケールエンジンを持たず「キー相違＝ミス」の既存経路に相乗り。
