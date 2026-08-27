# ギャップ分析: areka-P0-present-write-coherence

> 実施 2026-08-27 ／ 対象 HEAD `a6d27c73` ／ 本文の file:line は**すべて本日 Grep/Read で実測再確認**した値である。
> 位置づけ: 要件（確定済み）と既存コードの隔たりを示す資料である。**採否は決めない**（Requirement 1.5 の裁定は要件ディスカッションの仕事）。

---

## 0. 要旨（5 点）

1. **上流アンカーは全て現存・ドリフトなし**（Requirement 9.5 の再確認は充足）。`apply_show`:46／`set_visible`:375／`set_bounds`:381／`Visualize` 発行:389-398（`stage: SurfaceStage::Visualize` は :392）。wintf 側 `command.rs` の 7 アンカーも brief の −1 補正どおり。`flush_window_pos_commands()` は `tick_bridge.rs:258`＝スケジュールの外。
2. **`t_us` の起点は「プロセス開始」ではなく「その tick の開始」である**（`transition_diag.rs:692-708`）。したがって起点実測の 22,297〜339,998µs は**すべて 1 つの tick の内側**で起きており、遷移 1 回の tick は 340ms 級に伸びている。この読み替えを持たないと規模見積りが 1 桁ずれる。
3. **`apply_show` に `Present` の字面は無いが、Present は起きている**。`chain.upload()`（show.rs:306）の内側で `Present(0)`（`chain.rs:352`）が呼ばれ、`set_bounds`（show.rs:381）は `SpriteVisual::SetSize` を**直接 COM で**叩く（`mount.rs:256-263`）。つまり「絵が新寸になる」実体は upload＋SetSize であり、B-3 が遅らせる対象は `set_visible`／`set_bounds` の 2 行だけでは足りない。
4. **B-3 の到達可能域は数量から先に決まる**。窓書込の刻印は `apply_as_batch`（＝`EndDeferWindowPos` まで）が**戻った後**に発行される（`command.rs:775-796`）。ゆえに可視化を flush の直前へ置いても隙間は `flush_total_us` そのもの＝**143,231µs 以上（上限の 8.6 倍以上）**残る。上限 16,667µs へ届く置き場は **`EndDeferWindowPos` の後**しかない。
5. **その置き場は判定器の落とし穴の上にある**。`visualize_to_write_us` は同一フレーム内の `write_us.saturating_sub(visualize_us)`（`transition_judge.rs:811-822`）であり、可視化が書込より**後**になると値は飽和して **0＝満点**になる。是正の実体が伴わなくても PASS が出せてしまう形であり、Requirement 8.3（上限・判定器・観測語彙を書き換えて未達を消さない）と正面から関わる。

---

## 1. 現状の構造（実測）

### 1.1 「絵が新寸になる」の実体

`crates/areka-emo-present/src/presenter/show.rs`

| 行 | 何をするか | 画面への効き方 |
|---|---|---|
| :306 | `chain.upload(&entry.composed)` | 内側で `ResizeBuffers`（外形変化時）→`CopyResource`→**`Present(0)`**（`chain.rs:291-352`）。**即時**に供給面の内容が入れ替わる |
| :347-359 | `stage=Upload` レコード | 観測のみ |
| :375 | `mount.set_visible(world, true)` | `Visual.is_visible` と `HitTest` を**ECS component へ書くだけ**（`mount.rs:279-294`・`:140-154`） |
| :381 | `mount.set_bounds(world, size)` | ⑴ `Arrangement.offset=(0,0)`／`size` を書く（`mount.rs:242-254`）＋⑵ **`SpriteVisual::SetSize` を直接 COM で叩く**（`mount.rs:256-263`）。⑵ だけが即時 |
| :389-398 | `stage=Visualize` レコード（判定量の始点） | 観測のみ |

**帰結**: 遷移（既に可視の窓）で画面が動く瞬間は **`upload` の `Present(0)` と `SetSize`** である。`set_visible` の効き（`SetOpacity`）は `visual_property_sync_system`（`graphics/systems/visual_sync.rs:210-262`）が担い、これは **Composition スケジュール**で走る。

### 1.2 スケジュール順が効く（B-4 の可否を左右する）

13 本の順序は `Input, Update, PreLayout, Layout, PostLayout, UISetup, GraphicsSetup, Draw, PreRenderSurface, RenderSurface, **Composition**, CommitComposition, **FrameFinalize**`（`world/mod.rs:680-` ／順序固定テスト `:817-844`）。

`emo2_frame_system`（＝`apply_show` を呼ぶ相）は **`FrameFinalize`** に登録される（`emo2_boot/mod.rs:470`）。つまり **Composition は FrameFinalize より前**である。

> **⚠ B-4 に直結する制約**: `Arrangement.offset` へ書いた値が `Visual::SetOffset` として実際に効くのは**次の tick の Composition**である。遷移中の tick は 340ms 級に伸びているため、`Arrangement` 経由の一時オフセットは**次フレーム＝実質 0.3 秒後**にしか効かない。B-4 を同一 tick で効かせるには `SetSize` と同型に **`Visual::SetOffset` を直接 COM で叩く**しかなく、そのとき `Arrangement`（＝当たり判定の権威）と visual の実オフセットが遷移中だけ食い違う。

### 1.3 窓書込 flush の実体と刻印の位置

`crates/wintf/src/ecs/window/command.rs`

- `flush()`（:724）: `begin` レコード → `apply_as_batch`（:385）→ **戻った後に**指令ごとの `write` レコード（:775-796）→ `end` レコード（`total_us`＝begin からの総所要・:798-808）。
- `apply_as_batch` は `BeginDeferWindowPos` → `DeferWindowPos` ループ（各 1 件の所要が `call_us`）→ **`EndDeferWindowPos`**（:433）で全窓を一度に動かす。
- `EndDeferWindowPos` は全窓ぶんの `WM_WINDOWPOSCHANGING/CHANGED` を**同期送達**する（:365-368）。そのハンドラ（`window_proc/window_pos.rs:41`）は ①World 借用→更新→解放 ②`try_tick_on_vsync()` ③`flush_window_pos_commands()` の 3 段（:29-33・:279-290）。②は `IS_TICK_FLUSH_IN_PROGRESS`（`world/vsync.rs:37-46`）で**塞がれる**が、③の入れ子 flush は塞がれない（`command.rs:711-715`）。
- 駆動点は `tick_bridge.rs:258`＝**World 借用スコープの外・スケジュールの外**。門（`AREKA_TICK_GATE`・既定 OFF）で省略した回も必ず呼ぶ。

### 1.4 隙間の内訳（実測値からの分解）

正本＝atom 確定台帳 §10.3／§11.3／§11.6。

| 量 | 実測 | 位置づけ |
|---|---|---|
| `visualize` の `t_us`（32 件） | 22,297 … 73,104 | tick 開始からの経過 |
| `write` の `t_us`（32 件） | 253,890 … 339,998 | 同上・**`EndDeferWindowPos` が戻った後**の刻印 |
| `flush_total_us`（8 遷移） | 143,231 … 231,910 | `begin`→`end` の総所要 |
| `visualize_to_write_us`（32 窓） | 210,329 … 306,301 | 上限 16,667µs の 12.6〜18.4 倍・上限以下は 0 件 |
| Σ`call_us` ／ `total_us`（B-2b **後**） | 6.0 〜 18.1% | `call_us` は `DeferWindowPos` **投入だけ**の所要（台帳 §10.6.1） |

**導出（本分析）**——刻印の位置が確定しているので、可視化を置く場所ごとの隙間の**下限**が計算できる:

| 可視化の置き場 | 残る隙間の下限 | 上限比 |
|---|---|---|
| 現状（`FrameFinalize` の相の内側） | 210,329µs（実測） | 12.6〜18.4 倍 |
| `flush()` の直前（`tick_bridge.rs:258` の手前） | ≈ `flush_total_us` ＝ **143,231µs 以上** | **8.6 倍以上** |
| `DeferWindowPos` 投入の後・`EndDeferWindowPos` の前 | ≈ `flush_total_us` × (1 − Σ`call_us`/`total_us`) ＝ **117,000µs 程度以上** | **7 倍程度以上** |
| **`EndDeferWindowPos` の後**（＝`apply_as_batch` が戻った後） | 記録ループ 1 周ぶん（数十〜数百µs） | **1 倍未満（到達可）** |

> **これが Requirement 1 の規模見積りの核**である。「可視化を窓書込の**直前**まで遅らせる」という B-3 の文言どおりの実装では、**上限 16,667µs には構造的に届かない**。届く置き場は 1 つしかなく、それは `EndDeferWindowPos` の**後**である。

---

## 2. 要件→資産マップ

| 要件 | 既存資産（実在するもの） | 隔たり |
|---|---|---|
| 1.1 規模見積り | 本文書 §1・§3 が素材 | — |
| 1.2 「大改造」定義との突合 | §3.4 で突合 | **Constraint**: 判定は要件ディスカッションの裁定事項 |
| 2.1 `visualize_to_write_us` ≤ 16,667µs | 判定器（`transition_judge_verdict.rs:90`・:517-527） | **Missing**: 到達する置き場が現在の構造に無い（§1.4）。**かつ B-4 では原理的に満たせない**（§5-①） |
| 2.2 可視化を同一 flush 区間の内側で | `flush()`（`command.rs:724`）は wintf の thread-local 静的関数で **World を持たない** | **Missing**: 遅延実行の器も World の受け渡し口も無い |
| 2.3/2.4/2.5 B-4 の一時オフセットと解除点 | `mount.rs:74`（`physical_arrangement`）・`:242-254`（`set_bounds`）がいずれも offset を **(0.0, 0.0) 固定**で書く | **Missing**: オフセットを持つ状態と解除点の概念が無い |
| 3.x 実機サインオフ | `transition_signoff_tests.rs:59-60`（`#[ignore]`・`AREKA_TRANSITION_LOG`）／手順書 `completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md`（§5 の 8 ステップ・§6.6 の 7 行様式・§7 の記録票 20 項目） | **流用可**（新設不要）。人手作業 |
| 4.1/4.2 合否量の分離 | `Bounds::signoff()`（`transition_judge_verdict.rs:169-175`）は **2 量とも armed**。片方だけを外す構成子は無い | **Constraint**: 「`flush_total_us` は測るが合否に載せない」を機械で表すには、判定器を読み替えるか運用（記録票側）で分けるかの裁定が要る |
| 4.4 決定論 PASS を証拠と読まない | 台帳 §11.2 が既に「測っている量が違う」と登記 | — |
| 5.2/5.3 提示側の観測点 | **提示（DWM 合成）を観測する仕組みはリポジトリに 1 つも無い** | **Missing / Research Needed**（§8-①） |
| 5.4 語彙不変 | `transition_diag.rs` の 10 種の `kind=` とレコード純関数（`write_line`:553／`flush_line`:576 ほか）。逐語テスト `transition_diag_tests::write_line_is_verbatim` | **Constraint**: 既存フィールド名の変更は不可。**新しい `kind=` の追加は「チャネルの内側」と読める**が、判定器（areka 側）の解析追加を伴う |
| 5.5 既定で無効・費用 0 | `transition_diag::is_enabled()`（:622）の前置ガード形が既に確立（show.rs:347 が手本） | **流用可** |
| 6.2/6.3 当たり判定原点 | `Arrangement`→`GlobalArrangement.bounds`＝αマスク座標基準（`mount.rs:11-13`） | **Constraint**: B-4 でオフセットを入れると遷移中の当たり判定座標が動く |
| 6.4 定常アロケーション 0 | 前置ガード（show.rs:347）・`FrameBudget`（`presenter/budget.rs`） | **Constraint** |
| 6.6 門の既定 | `world/mod.rs` の `tick_gate_enabled: false` | 読むだけ |
| 7.5 檻は兄弟ファイルへ | `crates/areka-emo-present/src/presenter/*_tests.rs` の慣行・`log-capture-kit`／`temp-path-kit`（cage 着地） | **流用可** |
| 8.x 未達の登記 | atom の前例（requirements.md:119-129 の改訂欄） | — |

---

## 3. B-3 の規模見積り（Requirement 1.1 の素材）

### 3.1 接触するファイルと関数

**必須（どの実装形でも触る）**

| ファイル | 関数／位置 | 触る理由 |
|---|---|---|
| `crates/areka-emo-present/src/presenter/show.rs` | `apply_show`:46 の (3) 節（:288-399） | upload／マスク同期／可視化／`Visualize` 記録を「準備」と「commit」に割る |
| `crates/areka-emo-present/src/presenter/target.rs`（`PresentTarget`） | フィールド追加 | 未 commit の保留（面・寸・可視性）を持つ器 |
| `crates/areka-emo-present/src/presenter/hub.rs` | `EmoPresenter`（:19-24）／`apply`:96 | commit の入口を生やす |
| `crates/areka/src/emo2_boot/frame.rs` | `emo2_frame_system`:158 | commit をどの相で駆動するか |

**「上限へ届かせる」ために追加で触る（§1.4 の帰結）**

| ファイル | 位置 | 触る理由 |
|---|---|---|
| `crates/wintf/src/runtime/tick_bridge.rs` | `tick_one_frame_with`:230-262（`flush_window_pos_commands()`:258 の直後） | flush 後に commit を駆動する唯一の合法な場所（`world: &Rc<RefCell<EcsWorld>>` がスコープに在り、借用は解放済み） |
| または `crates/wintf/src/ecs/window/command.rs` | `flush()`:724 の `apply_as_batch` 直後（:774 付近） | 同一 flush 区間の**内側**（Requirement 2.2 の字義どおり）。ただし World を持たず、入れ子 flush（`window_pos.rs:290`）でも発火する |

**wintf → areka の依存は存在しない**ため、どちらの置き場でも **wintf 側に「既定 None のフック口」を新設し、areka が起動時に登録する**形が要る。これは新しい仕組みである。

### 3.2 破ることになる既存の順序制約

1. **「表示成立点は 1 つ」**——`apply_show` の doc（show.rs:38-45・:400-403）が「可視化・`applied`・`native_size`・`last_show`・`current_surface_id` の更新点は 1 箇所」と宣言している。commit を分けると、成立点が 2 つになるか、成立点と可視化がずれる。
2. **「upload 直前の寸との前後比較で `resized` を得る」**（show.rs:300-317・design D4）。upload を後段へ移すと `prev_size` の意味が変わる。
3. **`observe_surface` の札は 1 つ**（show.rs:347・:389）——「2 度評価すると片方だけが変わる形を作れる」と本文が明記。相を割ると札の寿命が相をまたぐ。
4. **`reconcile_reported_sizes` は「本フレームの全 apply が済んだ後」**（frame.rs:194-206）。upload を後段へずらすと、窓寸 reconcile が読む `pending_resize` の生成点が相順の外へ出る。
5. **balloon-visibility の相は「指令適用後の実状態」を読む**（frame.rs:189-193）。可視化が未 commit だと、この相が読む `visible` の意味が変わる。
6. **入れ子 flush**（`command.rs:706-720`・`window_pos.rs:290`）——flush の内側に commit を置くと、`EndDeferWindowPos` 由来の同期メッセージ経路からも commit が呼ばれ得る。`IS_TICK_FLUSH_IN_PROGRESS` は入れ子 tick を塞ぐが**入れ子 flush は塞がない**。

### 3.3 追加が必要になる仕組み

- **保留の器**（`PresentTarget` に未 commit の面・寸・可視性）。emo-present に前例なし（`pending_resize` は pull 型の報告であって遅延実行ではない）。
- **commit の入口**（`EmoPresenter::commit_visualize(&mut World)` 相当）。
- **flush 後フック**（wintf 側・既定 None・areka が登録）。**現在 wintf にコールバック登録の前例は無い**（thread_local はデータ用のみ）。
- **可視化が commit されないまま次の遷移が来たときの規律**（取りこぼし・二重 commit の防止）。
- 上記すべてを**既定 OFF ではなく本番経路**に入れるので、Requirement 6.4（定常アロケーション 0）を壊さない実装が要る。

### 3.4 Requirement 1.2 の「大改造」定義との突合

| 定義 | B-3（flush 直前へ commit） | B-3′（flush 後／flush 内側へ commit） |
|---|---|---|
| ⑴ スケジュールの順序・構成の変更 | **該当しない** | **該当しない** |
| ⑵ tick の駆動と flush の駆動の関係の変更 | 該当しない（相の内側で完結） | **判定が割れる**——flush の**後ろに新しい駆動を足す**行為が「関係の変更」に当たるか |
| ⑶ flush の呼出位置をスケジュールの内側へ移す | 該当しない | **該当しない**（flush は動かさない） |

> **裁定が要る点**: ⑵ の解釈が B-3 の採否を直接決める。そして **B-3 のうち上限へ届くのは B-3′ だけ**である（§1.4）。「⑵ に当たる」と読めば Requirement 1.3 が発動して B-4 か見送りへ落ち、「当たらない」と読めば B-3′ が候補として生きる。

---

## 4. 実装アプローチの選択肢

### Option A: 既存部品の拡張だけで済ませる（B-4 系）

**内容**: `mount.rs` に「遷移中の一時オフセット」を持たせ、`apply_show` の可視化の段で設定・窓書込の完了後に解除する。オフセットは `((win_w−surf_w)/2, win_h−surf_h)`。

**接触**: `mount.rs`（`physical_arrangement`:74／`set_bounds`:242 に offset 引数を通す）・`show.rs:381` の呼出・解除点の駆動元（`frame.rs` の相末尾）。**wintf 非接触**。

**効くための必須条件（本分析で判明）**: `Arrangement` 経由では**次 tick まで効かない**（§1.2）。同一 tick で効かせるには `Visual::SetOffset` を `SetSize` と同型に直接 COM で叩く必要があり、そのあいだ `Arrangement`（当たり判定の権威）と visual の実位置が食い違う。

**トレードオフ**
- ✅ 接触集合が小さく、tick 構造に一切触れない（Requirement 1.2 のどの定義にも該当しない）
- ✅ 目視の症状（跳ね）には直接効く
- ❌ **`visualize_to_write_us` は 1µs も縮まない** → Requirement 2.1 を満たせない（§5-① 参照）
- ❌ 当たり判定の原点が遷移中に動く（Requirement 6.2/6.3・`collision-dpi-hittest` の確定物）
- ❌ 既存の決定論テストを 1 本壊す（§6-①）
- ❌ 解除の取りこぼしが「定常の配置契約を静かに壊す」形になる（Requirement 2.5 が既に警戒している）

### Option B: 新しい部品を作る（B-3′ 系）

**内容**: emo-present に「準備／commit」の 2 相と保留の器を新設し、wintf に既定 None の flush 後フックを新設して `tick_bridge.rs:258` の直後（または `command.rs` の `apply_as_batch` 直後）から commit を駆動する。

**トレードオフ**
- ✅ **上限 16,667µs へ到達し得る唯一の形**（§1.4）
- ✅ 責務の切れ目が明確（可視化の段＝本 spec・窓書込の駆動＝atom のまま）
- ❌ wintf にコールバック登録という**前例の無い仕組み**を足す（wintf は areka を知らない）
- ❌ 「表示成立点は 1 つ」を含む 6 本の順序契約に手が入る（§3.2）
- ❌ Requirement 1.2⑵ に当たるかの裁定が先に要る（§3.4）
- ❌ **判定量が飽和して 0 になる**（§5-②）——「直った」と「順序が逆転しただけ」を判定器が区別できない
- ❌ 既存の字面走査テストを壊す（§6-②）

### Option C: ハイブリッド／段階（規模を測ってから縮小する）

**C-1「観測だけ先に足す」**: 提示側の観測点（§8-①）を Requirement 5.3 の枠内で足し、「絵と窓のどちらが先に見えるか」を初めて機械で名指しできる状態にしてから、B-3′／B-4／見送りを裁定する。是正の実装は 0 行。
- ✅ Requirement 5.2 を満たす唯一の道・Requirement 1 の裁定に事実の裏づけを与える
- ✅ tick 構造に一切触れない・並走 3 spec と競合しない
- ❌ Requirement 2.1 は未達のまま（Requirement 8 の登記へ）

**C-2「B-4 で見た目を無害化＋未達を登記」**: Option A を採り、`visualize_to_write_us` の未達は Requirement 8 で登記する（Requirement 2.6 の見送り扱いを 2.1 だけに限定して適用する読み）。
- ✅ 開発者裁定（M1 を妨げない・大改造なら治さなくてよい）と整合
- ❌ Requirement 2.1 の文言（B-4 でも 16,667µs 以下）と衝突する（§5-①）

**C-3「見送り＋登記」**: Requirement 1.5 で見送りを裁定し、Requirement 8 の登記のみで閉じる。実行時の挙動は不変。
- ✅ 最小コスト・`draw-load-parity` の前例あり
- ❌ 引受先を実在の spec として名指しできない場合は「引受先なし」の明記が要る（Requirement 8.2）

---

## 5. 判定器の落とし穴（設計討議へ必ず上げる）

### ① B-4 は Requirement 2.1 を原理的に満たせない

Requirement 2.1 は「**B-3 または B-4 を採用したとき**…`visualize_to_write_us` を 16,667µs 以下にする」と書く。しかし brief:78-79 は B-4 を「**隙間は消さず見た目を無害にする**」と定義しており、B-4 は可視化の時刻も窓書込の時刻も動かさない。判定器は両者の時刻差しか見ない（`transition_judge.rs:811-822`）。したがって **B-4 を採ると 2.1 は必ず FAIL する**。要件文書の内側に閉じた矛盾であり、ディスカッションでの裁定が要る。

### ② 可視化を書込の後ろへ動かすと判定量が飽和して 0 になる

```rust
// transition_judge.rs:815-818
last_write_us_per_window_frame
    .get(&(window.clone(), *frame))
    .map(|write_us| write_us.saturating_sub(*visualize_us))
```

- 同一 `frame` であることが測定の前提（別フレームになると `Unmeasured(VisualizeToWriteUs)` が立つ——`transition_judge_verdict.rs:558-564`。ここは**穴が開いていない**）。
- `frame` は flush でも同じ値になる（`stamp()` は tick のスレッド局所ミラーを読む・`transition_diag.rs:683-708`）。
- ゆえに commit を `EndDeferWindowPos` の**後**へ置くと `visualize_us > write_us` となり、**飽和して 0＝満点**になる。

これは「未測定として落ちる」でも「違反として立つ」でもなく、**満点として通る**。Requirement 8.3・Requirement 3.5 の趣旨に照らして、設計時に明示的な対処（新しい観測点で符号付き差を取る／commit を `EndDeferWindowPos` の直後かつ `write` レコード発行の**直前**に置いて正の小さな値を出す／判定器に絶対値の口を足すことの可否）を裁定する必要がある。

### ③ `flush_total_us` を合否から外す機械的な口が無い

`Bounds::signoff()`（`transition_judge_verdict.rs:169-175`）は 2 量とも armed である。Requirement 4.2 の「測るが合否に載せない」を実現する手段は、⑴ 判定器に新しい構成子を足す（＝atom の確定物への変更か？）／⑵ ランナーの出力はそのままに記録票（§6.6 の 7 行）側で読み分ける、の 2 択。Requirement 5.3（判定器の**新設**は不可）との境界の裁定が要る。

---

## 6. 既存の決定論テストへの影響（Requirement 6.5 の素材）

① **`crates/areka-emo-present/src/presenter_upload_failure_tests.rs`** — `Arrangement.offset` に番兵を仕込み、「`set_bounds` は size 引数に関わらず offset を **(0.0, 0.0) へ無条件で**書く」ことを前提に `set_bounds` の呼出有無を判別している（:14-16・:76-78・:122-124・:169-177・:360・:384・:450・:516）。**B-4 を採るとこの前提が崩れる**。

② **`crates/areka-emo-present/src/presenter/transition_record_tests.rs`** — `apply_show` の**本文の字面**を走査する。`"if observe_surface {"` の出現数が **2** であること（:311-316）と `"pub(super) fn apply_show("` の存在（:320-323）を assert する。**B-3 で相を割るとどちらも壊れる**。

③ **`crates/wintf/src/ecs/window/command_batch_tests.rs`（8 本）** — `flush()` の形（`in_batch`・O9 行の本数）を固定する。**flush の内側へ commit を差し込む形（Option B の後者）はここに触れる**。

④ **`crates/areka/src/placement/transition_signoff_procedure_tests.rs`** — 手順書 `completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md` の語彙と実装の逐語一致を検査する（読むパスは :41 の const で、アーカイブ移動に追随済み）。**手順を変えるなら手順書とこのテストが対になる**。

⑤ **`crates/wintf/src/ecs/window/transition_diag_tests.rs::write_line_is_verbatim`** — レコードの字面を固定。Requirement 5.4 の機械的な守り手。

> 参考（本 spec の担当ではないが記録）: `transition_signoff_procedure_tests.rs:3` と `transition_judge_reobservation_tests.rs:13` の**doc コメント内**の spec パスが `completed/` 移動前のまま残っている（実行に影響しない・:41 の const は是正済み）。

---

## 7. Effort / Risk

| 案 | Effort | Risk | 一言 |
|---|---|---|---|
| Option A（B-4） | **M**（3〜7 日） | **中〜高** | 接触は狭いが、当たり判定原点・`Arrangement` と visual の乖離・解除の取りこぼしという 3 つの新しい壊し方を持ち込む。かつ合否量には効かない |
| Option B（B-3′） | **L〜XL**（1〜2 週＋） | **高** | 前例の無い wintf フックと 6 本の順序契約の書き換え。合否は出せる可能性があるが、判定量の飽和（§5-②）を先に裁定しないと「PASS の意味が無い」 |
| Option C-1（観測のみ） | **S〜M**（1〜5 日） | **低〜中** | 是正 0 行・実行時挙動不変。DWM 提示時刻の取得手段が未調査（§8-①） |
| Option C-2（B-4＋登記） | **M** | 中〜高 | Option A に Requirement 8 の登記を足したもの |
| Option C-3（見送り＋登記） | **S**（1〜3 日） | **低** | `draw-load-parity` の前例どおり。要件文書・design・境界節・steering の 4 点追随が仕事（Requirement 8.4） |

---

## 8. Research Needed（設計フェーズへ持ち越す）

① **提示側の観測点をどう作るか（Requirement 5.2/5.3 の前提）** — リポジトリに DWM 合成・提示の観測は 1 つも無い。候補は ⒜ `IDXGISwapChain::GetFrameStatistics`（`PresentCount`／`SyncQPCTime`・`chain.rs` の swapchain が既に手元にある）、⒝ `DwmGetCompositionTimingInfo`、⒞ WUC の `CompositionTarget`／`Compositor` 側の口。**ただし emo の窓は `WS_EX_NOREDIRECTIONBITMAP` の GPU 合成窓であり、swapchain は WUC の `ICompositionSurface` へ束ねられている**ため、⒜ が「画面に出た時刻」を返すかは未確認。要調査。

② **`flush_total_us` の内訳**——B-2b 後の `EndDeferWindowPos` 単独の所要は台帳に無い（L7 ⑷「OS 側の内訳は未特定」）。§1.4 の下限表は Σ`call_us`/`total_us`＝6.0〜18.1% からの推定であり、`EndDeferWindowPos` の直前・直後に計時点を足せば確定できる。**この 1 点の実測が Option B の到達可否を確定させる**。

③ **wintf にコールバック登録の前例が無いこと**の設計上の扱い（thread_local クロージャ／`EcsWorld` の resource／`WinApp` の facade のどれに載せるか）。

④ **`AREKA_TICK_GATE` が有効な採取での集計の妥当性**（Requirement 9.4）——`frame=` を鍵にした突合の意味が変わる。既定 OFF のまま採るのが安全側だが、記録票へ状態を明記する必要がある（手順書 §7 の 20 項目には「tick の門の状態」の欄が無い＝**手順書の記録票に 1 項目足す必要があるか**の裁定が要る。Requirement 3.6 は明記を求めている）。

⑤ **B-4 の一時オフセットと `balloon-offset-dpi`（同ウェーブ）の照合**（Requirement 9.2）——bod は `placement/follow` 系・本 spec は `mount.rs`。roadmap:89 が「B-4 を採る場合のみ意味論上近接」と登記済み。

---

## 9. 設計判断項目（要件ディスカッションへ）

1. **B-4 と Requirement 2.1 の矛盾をどう裁くか**——B-4 は `visualize_to_write_us` を動かさないので 2.1 を満たせない（§5-①）。⒜ 2.1 を B-3 限定へ読み替える／⒝ B-4 採用時は Requirement 2.6 の見送り扱いを 2.1 だけに適用する／⒞ B-4 は最初から候補から外す、のいずれか。
2. **「B-3」の到達可能域を要件がどう受け取るか**——文言どおり（可視化を窓書込の**直前**へ）では上限に**構造的に届かない**（§1.4）。上限へ届く唯一の形は `EndDeferWindowPos` の**後**であり、これを B-3 と呼ぶか、候補表の外と見て Requirement 1.4 で禁じるかの裁定が要る。
3. **Requirement 1.2⑵「tick の駆動と窓書込 flush の駆動の関係を変更する」の解釈**——`flush_window_pos_commands()` の**後ろ**に新しい駆動（commit フック）を足すことは⑵ に当たるか。この 1 点で B-3′ の生死が決まる（§3.4）。
4. **判定量の飽和（`saturating_sub`）への対処**——可視化が書込の後ろへ回ると `visualize_to_write_us` は 0＝満点になる（§5-②）。合格の意味を保つために何を足すか（新しい観測点／commit の置き場を `write` レコード発行の直前に限定する／判定器に口を足すことの可否）。
5. **`flush_total_us` を「測るが合否に載せない」の機械的な表し方**（§5-③）——判定器に構成子を足すのは Requirement 5.3 の「新設しない」に触れるか。
6. **提示側の観測点（Requirement 5.2）を本 spec の内側で作るか**——作らないなら「見え方の順序」は永久に未特定のまま Requirement 5.1 で扱うことになる。作るなら §8-① の調査が設計の前提になる。
7. **B-4 の一時オフセットを `Arrangement` 経由にするか直接 COM にするか**——`Arrangement` 経由は**次 tick まで効かない**（§1.2）。直接 COM だと当たり判定の権威（`Arrangement`）と visual の実位置が遷移中だけ食い違う（Requirement 6.2/6.3）。
8. **壊れる既存テストの扱い**（Requirement 6.5）——`presenter_upload_failure_tests.rs` の番兵手法（B-4 で崩れる）と `transition_record_tests.rs` の字面走査 2 件（B-3 で崩れる）を、退役とするか更新とするか。
9. **手順書の記録票に「tick の門の状態」を足すか**（Requirement 3.6 は明記を求めるが、手順書 §7 の 20 項目には欄が無い。手順書は `completed/` の資産で、逐語テストが対になっている）。
10. **見送り時の引受先**（Requirement 8.2）——実在の spec を名指しできるか。現時点で `visualize_to_write_us` を引き受け得る spec は無く、`tick-gate-adoption`（M2 解禁ゲート・棚卸⑪起票）は tick の門の本採用が担当で本量は担当しない。「引受先なし」の明示になる可能性が高い。
