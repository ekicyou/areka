# ギャップ分析（research.md）: areka-P0-choice-interact

> `\q` 選択肢の**対話面**（実ポインタ→hover 追従→クリック確定→`ChoiceSelection` 発行）。W3 完走済み上流 choice-render の実供給契約（emo-text `TextLayerRuntime`）を消費し、W2 input-events のポインタ配線 donor に倣ってバルーン窓の対話配線を additive 増設する。
>
> 本書は**情報提供**であり実装決定ではない。設計フェーズ（kiro-design）で選択肢を裁定する材料を提示する。

---

## 1. 分析サマリ（3–5 bullet）

- **上流契約3+1 API は実物**（doc/brief の「供給予定」は解消済み）: `TextLayerRuntime`（`crates/areka-emo-text/src/actor.rs`）に `ChoiceHitRow`（:149-161・`rect: HitRectPx`＝**バルーン窓 client 物理 px**）／`inject_choice_hover`（:366）／`choice_hit_rows`（:389）／`choice_active`（:400）が実在。**hover は書込専用で hovered-row getter が無い**——本 spec は最後に注入した ordinal を**自前追跡**する必要がある（Missing capability）。
- **ポインタ配線 donor は成熟**: `crates/areka/src/input_events/mod.rs` が `MouseWiring`（NonSend）・`RegionSource`（Presenter|Mock シーム）・物理 px 素通し（DD-IE-10・k=1.0）・`attach_char_pointer_handlers`（:232 post-spawn 挿入）・借用分割（`resolve_region_owned` :273）を完備。バルーン用に**鏡写しできる**。ただしキャラ窓ハンドラは `MouseWiring`＝presenter（当たり判定名）を読むのに対し、**バルーンハンドラは `TextLayerRuntime`（`Emo2Wiring` 内 `Rc<RefCell<_>>`）を読む**必要があり、参照資源が異なる（借用規律の再設計点）。
- **バルーン窓は既存だがポインタハンドラ非装着が意図的**（`spawn.rs:80` `BalloonWindowMarker{scope}`・:163-177 `DragConfig` 付き・DD-IE-12 でハンドラ非装着）。加えて**バルーン窓は `HitTest::none()`**（:174）——窓レベルで**クリック透過**。選択肢行がポインタイベントを受け取る経路（窓 `HitTest` か content の `alpha_mask` か）は**未確定＝最大の Research Needed**。
- **`ChoiceSelection` は net-new**（コード内は actor.rs:146 の doc 言及のみ・struct 不在）。本 spec がワイヤ形正本を所有するが、**発行先シンク（Sender/inbox 型）が設計判断**。最終配送は下流 `choice-select-events`（`SakuraMsg::ResolveChoice{ id: String }`＝`contract.rs:38`・`CuePlayer::resolve_choice`＝`runtime.rs:291`）の領分で、本 spec は**発行まで**（直接呼出禁止）。
- **実機 hover 注入 donor が現存**（`crates/areka/src/emo2_boot/hover_inject.rs`＋`frame.rs:711`・env `AREKA_CHOICE_HOVER_INJECT`・本番既定 no-op）。純関数分割・借用規律（不変借用でスナップショット→可変 `inject_choice_hover`）の**参照実装**。設計は実ポインタ駆動との**置換／共存**を裁定する。

---

## 2. 既存資産の調査（Current State）

### 2.1 上流契約 API（`crates/areka-emo-text/src/actor.rs`・実測確認済み）

`TextLayerRuntime`（UI スレッド所有・NonSend・`!Send`・`Rc<RefCell<_>>` 共有）:

| API | シグネチャ（要旨） | 契約 |
|---|---|---|
| `choice_hit_rows(&self, &ActorKey) -> &[ChoiceHitRow]` | :389 | 最終提示フレーム同期スナップショット。未装着/選択肢なし/未 population は空 slice。**鮮度＝表示と同一 layout 単一導出**。|
| `inject_choice_hover(&mut self, &ActorKey, Option<usize>)` | :366 | `None`＝ハイライト無し。stale ordinal は panic せず debug ログ＋保持（縮退）。**書込専用・getter 無し**。|
| `choice_active(&self, &ActorKey) -> bool` | :400 | 表示層自身の選択肢スパン非空。未知 actor/空は `false`。|

`ChoiceHitRow`（:149-161）: `ordinal: usize`（hover 注入／選択解決の主キー）・`id: String`（`\q` ID）・`label: String`・`references: Vec<String>`（`\q` 第3引数以降）・`rect: HitRectPx`。→ **`ChoiceSelection` 構成材料が同梱済み**（再照会不要・R2.2 を上流が満たす）。

`HitRectPx`（`crates/areka-emo-text/src/choice.rs:155`）: `left/top/right/bottom: f32`・**バルーン窓 client 物理 px**（スクロール committed 反映済み・whole-pixel）。actor.rs:141 で `pub use crate::choice::HitRectPx` 再輸出。

**原子性は上流が保証**（apply_cue :285-309）: `Clear`/`ClearAll` で当該（または全）actor の `choice_hover` と `choice_snapshot` を**純粋状態の選択肢消去と同時**に無効化。→ `choice_active` は span 由来で即 false、`choice_hit_rows` は present を待たず空へ。本 spec は「クリック時に**現行** `choice_hit_rows`／`choice_active` を読む」ことで stale を作らず協調できる（R3・自前の複雑な無効化ロジックは不要）。

### 2.2 ポインタ配線 donor（`crates/areka/src/input_events/mod.rs`・実測確認済み）

- `MouseWiring`（NonSend・:39）: `Sender<KanadeMsg>`＋per-scope 間引き `HashMap<u32, MouseMoveThrottle>`＋`RegionSource`＋注入 clock。
- `RegionSource`（:53）: `Presenter`（本番・`resolve_hit_region`）｜`Mock(fn)`（決定論檻専用・`#[allow(dead_code)]` 恒久シーム）。
- **物理 px 素通し**（:96・DD-IE-10）: `PointerState.client_point`（窓 client 物理 px）を DPI 変換せず（k=1.0）resolver へ。R4.2 の座標契約はこの規約と整合。
- `attach_char_pointer_handlers`（:232）: `CharWindowMarker` 全窓へ `OnPointerMoved`＋`OnPointerPressed` を**post-spawn 挿入**（spawn.rs 本体を触らない・依存方向 input_events→placement）。**バルーンには意図的に付けない**旨コメント（:217 DD-IE-12）。
- ハンドラ署名（:290 `on_char_pointer_moved`・:334 `on_char_pointer_pressed`）: `fn(&mut World, sender: Entity, entity: Entity, &Phase<PointerState>) -> bool`。**Bubble 相のみ処理・Tunnel は false**。
- 借用分割（:273 `resolve_region_owned`）: `&mut World` 上の別 NonSend 資源（`MouseWiring` と `Emo2Wiring`）を**共有借用で解決→owned 取り出し→後で `&mut` 取得**する規律。バルーン版は `TextLayerRuntime` 借用に同型の規律が要る。
- テスト（:392-948）: 合成 `PointerState`／`Phase` でハンドラ直接呼び＋mpsc 観測（GPU/実窓/sleep 不要）。**本 spec の決定論檻はこの形を踏襲**（R6.4）。

### 2.3 バルーン窓（`crates/areka/src/placement/spawn.rs`・実測確認済み）

- `BalloonWindowMarker { scope: usize }`（:80）・`GhostWindowMarker`・`Window`・`window_style()`・`window_pos(...)`・**`HitTest::none()`**（:174）・`DragConfig::default()`・`OnDrag(on_balloon_drag)`（:163-177）。
- **ポインタハンドラは非装着**（:160-162 コメント: 「M1 はバルーンにマウス送出なし＝ポインタハンドラを付けない・DD-IE-12・バルーン入力は M-dialogue／choice-render の領分」）。→ 本 spec が**この非装着を埋める**。
- `GhostWindows`（Resource・:108）: `scope → ScopeWindows{ char_window, balloon_window }` の正本。`balloon_window(scope)` アクセサ（:122）。→ scope↔balloon entity の逆引きに使える。

### 2.4 下流契約辺（**呼ばない**・境界確認）

- `SakuraMsg::ResolveChoice { id: String }`（`crates/areka-sakura/src/contract.rs:38`・`#[non_exhaustive]`）: talk アクター境界の型付き入力。「`CuePlayer::resolve_choice` を外部から直接呼ぶ経路は構造的に不在・投函は W5 choice-select-events の領分」と doc 明記。
- `CuePlayer::resolve_choice`（`crates/dola/src/cue/runtime.rs:291`）・`drive.rs:355 on_resolve_choice`: 配送は kanade 経由が正規。**本 spec はスコープ外**（R2.6/R5.4）。
- `KanadeMsg`（`crates/areka-kanade/src/msg.rs:83`・`#[non_exhaustive]`）: 現状 `Mouse(MouseInput)` 等。**`ChoiceSelection`／選択解決 variant は無い**。→ 最終配送先型は choice-select-events の契約辺（本 spec は決めきらない）。

### 2.5 実機 hover 注入 donor（`crates/areka/src/emo2_boot/hover_inject.rs`＋`frame.rs`・実測確認済み）

- env `AREKA_CHOICE_HOVER_INJECT`（`cycle`／`cycle:<ms>`・本番既定 Disabled＝完全 no-op）。`frame.rs:711` の text phase で **`present_frame` の後**に `drive(&mut runtime, talk_time)`。
- 純関数分割: `parse_hover_inject`（env→config）・`cycle_ordinal`（時刻商→ordinal・sleep なし）・`run_hover_inject_with`（driver・runtime 非依存）。
- 借用規律（:158-180 `run_hover_inject`）: `runtime.state().actors()` を**不変借用でスナップショット**（`choice_active`／`choice_hit_rows().len()` を clone/copy）してから、`inject_choice_hover` の**可変**呼出へ移る。→ 実ポインタ駆動でも同型の借用規律が要る。
- 位置づけ: 「ライブラリ公開 API の一消費者に過ぎず、emo-text 本体・本番描画経路・決定論資産に変更を加えない」（8.6）。**実ポインタ導線も同じ非侵襲原則を継ぐべき**。

---

## 3. Requirement → Asset マップ（ギャップタグ: Reuse / Missing / Constraint / Unknown）

| Req | 必要技術要素 | 既存資産 | ギャップ |
|---|---|---|---|
| R1.1-1.4 hover 追従 | ポインタ移動→hit 判定→`inject_choice_hover`／`choice_active` ゲート | `choice_hit_rows`/`choice_active`/`inject_choice_hover`（実物）・`OnPointerMoved` donor | **Missing**: 点包含 hit 判定（純関数・net-new）／バルーンハンドラ結線 |
| R1.5 病的重なりで≤1行 | 決定論的単一選択規則 | `choice_hit_rows` 出力順＝ordinal 昇順×行昇順（choice.rs derive_hit_rows doc） | **Unknown**: 重なり時に先勝ち/後勝ちのどちらか（画家のアルゴリズム＝後定義手前 vs 先頭一致）を裁定 |
| R1.6 自前描画なし | hover 状態駆動のみ | `inject_choice_hover`（描画は下流 present_actor） | **Reuse**（描画は上流所有） |
| R2.1-2.4 確定クリック→一度きり発行 | クリック hit→`ChoiceSelection` 1回 | `OnPointerPressed` donor（単発クリック分岐は char 窓では不送出＝7.3） | **Missing**: `ChoiceSelection` struct／発行シンク／単一発行ガード |
| R2.2 ワイヤ形 | id/label/scope/references 保持 | `ChoiceHitRow` に id/label/references 同梱・`BalloonWindowMarker.scope` | **Missing**: `ChoiceSelection` 型定義（本 spec 正本） |
| R2.5-2.6 現行整合／直接呼出禁止 | クリック時 `choice_hit_rows` 再読・resolve 非呼出 | 上流原子性（apply_cue）／下流 seam 分離 | **Reuse**（境界は既存で担保） |
| R3.1-3.4 stale 棄却／原子性 | 消滅後の非発行・hover None 整合 | apply_cue の hover+snapshot 同時無効化（:285-309） | **Missing（薄）**: 自前保持 ordinal の消滅追随・クリック時 present チェック |
| R4.1-4.2 消費境界／DPI 素通し | 再定義せず消費・物理 px k=1.0 | DD-IE-10 素通し規約・`HitRectPx`=物理 px | **Constraint**: 素通し規約を破らない（座標変換禁止） |
| R4.3-4.4 キャラ窓非退行／窓消費 | balloon marker/drag を消費のみ | `BalloonWindowMarker`/`DragConfig` 既存 | **Constraint**: spawn.rs 本体不改変（エスケープ条項＝position-persist へ委譲） |
| R6.1-6.6 決定論檻 | 注入ポインタ列・純関数全網羅・配線存在檻 | input_events テスト形（Mock+合成 PointerState） | **Missing**: hit 判定純関数＋バルーン檻。**Reuse**: テスト方式 |
| R7.1-7.6 実機サインオフ | 実 emo2/DPI/絶対パス・auto-exit＋grep | hover_inject.rs 導線・memory の実機定石 | **Unknown**: 実ポインタ導線と env 巡回導線の置換/共存設計 |
| R8.1-8.6 非退行 | workspace 緑・no new dep・Rust 2024・no tokio・スレッド親和 | 既存ビルド制約 | **Constraint**（横断遵守） |

---

## 4. 実装アプローチ選択肢

### 課題A: バルーンポインタ配線の設置場所

#### 選択肢 A-1: `input_events/mod.rs` を拡張（brief 事前割当契約に整合）
- 既存 `input_events` モジュールへ `BalloonWiring`（NonSend）＋`attach_balloon_pointer_handlers`＋`on_balloon_pointer_moved/pressed` を増設。
- ✅ brief 追記㊵の「バルーンポインタ配線は input_events モジュール＋emo-text 幾何消費で完結」に直結。donor と同一ファイルで規約（DD-IE-10 素通し・借用分割）を共有。
- ✅ W5 collision-dpi-hittest が同ファイルを後続共有——同居ファイル内で素通し規約を壊さないことを設計時に一望できる。
- ❌ ファイル肥大（既に ~950 行）。キャラ窓配線（kanade へ送出）とバルーン配線（runtime 直読）で**参照資源・下流が異なる**ため、同居がかえって混線を招く懸念。

#### 選択肢 A-2: 新モジュール `input_events/balloon.rs`（または `choice_interact/`）
- `input_events` 配下にサブモジュールを切り、バルーン専用配線を隔離。
- ✅ 責務分離（キャラ窓=kanade 送出／バルーン=hover 駆動＋ChoiceSelection 発行）。テスト隔離が明快。
- ✅ brief の「input_events モジュール内で完結」を**サブモジュールとして**満たしつつ肥大回避。
- ❌ 素通し規約・借用分割の共有を module 跨ぎで意識する必要（設計注記で担保可能）。

**推奨傾向**: A-2（サブモジュール隔離）。参照資源・下流・テスト軸がキャラ窓配線と別物ゆえ。ただし brief の「input_events モジュールで完結」を最優先するなら A-1 も可。**裁定は設計へ**。

### 課題B: `TextLayerRuntime` へのハンドラからのアクセス経路

バルーンハンドラ（`&mut World`）は `Emo2Wiring` 内 `Rc<RefCell<TextLayerRuntime>>` を借りて `choice_active`/`choice_hit_rows`（不変）→ `inject_choice_hover`（可変）を呼ぶ。

#### 選択肢 B-1: ハンドラ内で `Emo2Wiring` を直接借りて runtime を `borrow_mut`
- `resolve_region_owned`（donor :273）と同型に、`world.get_non_send_resource::<Emo2Wiring>()` → `runtime.borrow_mut()` で hit 判定＋注入を一括。
- ✅ donor の借用分割規律をそのまま踏襲。追加資源なし。
- ❌ `Emo2Wiring` 不在（boot 前/失敗時）の正常縮退（no-op）を設計する必要（donor の presenter=None 縮退と同型で対応可）。
- ❌ hit 判定純関数を runtime borrow の外に括り出さないと R6.5 の「GPU 不要・純関数全網羅」が濁る。

#### 選択肢 B-2: NonSend `BalloonWiring` に自前状態（last-injected ordinal・ChoiceSelection Sender）を保持し、runtime は毎回 `Emo2Wiring` から借用
- hover 追跡状態（getter 不在の穴埋め）と発行シンクを `BalloonWiring` へ集約、runtime は都度借用。
- ✅ 「自前追跡 ordinal」（Missing capability）と「ChoiceSelection 発行先」を 1 資源へ集約——`MouseWiring` と同型の NonSend パターン。
- ✅ 純関数 hit 判定（点×`HitRectPx`）を `BalloonWiring` から切り離し独立テスト可能。
- ❌ 資源 2 つ（`BalloonWiring`＋`Emo2Wiring`）の借用順序を設計で固定する必要。

**推奨傾向**: B-2。R3 の自前 ordinal 追跡と R2 の発行シンクを NonSend へ束ねるのが donor（`MouseWiring`）と最も同型。

### 課題C: `ChoiceSelection` 発行シンク（ワイヤ形は本 spec 正本・配送先は下流契約辺）

#### 選択肢 C-1: `Sender<ChoiceSelection>`（std mpsc）を `BalloonWiring` に注入（`MouseWiring` の `Sender<KanadeMsg>` と同型）
- ✅ 決定論檻が `Receiver` で発行を観測（donor テストの mpsc 観測と同一形・R6.2）。
- ✅ 発行先（受信アクター）は本番結線時に choice-select-events が Sender を供給——本 spec は型と発行のみ所有。
- ❌ 受信側 inbox 型が未確定の間、`ChoiceSelection` は「発行されるが誰も受けない」暫定状態（M1 は発行到達までで正当・R7.4）。

#### 選択肢 C-2: コールバック seam（`Box<dyn FnMut(ChoiceSelection)>`）
- ✅ Sender 型に縛られず柔軟。
- ❌ donor は mpsc 一貫。coalback は areka の既存流儀（`MouseWiring`）から外れ、決定論観測もクロージャ捕捉で煩雑。

**推奨傾向**: C-1（mpsc Sender）。areka 既存の入力配信流儀と一致し、決定論観測が素直。**受信 inbox 型は choice-select-events との契約辺ゆえ本 spec では確定しない**（Sender の要素型＝`ChoiceSelection` のみ確定）。

### 課題D: バルーン窓のポインタイベント到達（**最重要 Research Needed**）

`spawn.rs:174` でバルーン窓は **`HitTest::none()`**（窓レベルのクリック透過）。この状態で `OnPointerMoved`/`OnPointerPressed` が選択肢行位置で発火するのかが未確定。

- 参照: memory「クリック透過はHitTest設定が必須——窓=none()・画像=alpha_mask()・既定 Bounds が透過を殺す」。キャラ窓は content 画像側で `alpha_mask()` を設定してイベントを受ける。
- **設計で確定すべき**: (i) バルーン窓自体の `HitTest` を変える必要があるか（→ spawn.rs 改変＝エスケープ条項で position-persist へ委譲の可能性）、(ii) 選択肢 content（emo-text の描画面/widget）へ `alpha_mask()` 相当を与えて行位置でイベントを受けるか（→ choice-render/emo-text の描画面所有との境界）、(iii) バルーン窓の `HitTest` を選択肢表示中のみトグルするか。
- (i)/(ii) はいずれも**本 spec 単独では閉じない可能性**（spawn.rs もしくは emo-text 描画面の改変）。**エスケープ条項（R4.4・position-persist へ委譲）または上流協調が必要か**を設計で判定。

---

## 5. Effort / Risk

| 項目 | 見積 | 根拠 |
|---|---|---|
| 純関数 hit 判定（点×`HitRectPx`・重なり規則）＋単体檻 | **S** | 純データ・GPU 不要・donor テスト形を踏襲 |
| バルーンポインタ配線（`BalloonWiring`＋ハンドラ＋attach）＋配線存在檻 | **M** | donor 鏡写しだが参照資源（runtime borrow）・下流（ChoiceSelection）が別・借用規律再設計 |
| `ChoiceSelection` 型＋発行シンク＋単一発行/stale ガード＋檻 | **S–M** | 型は素直・ガードは上流原子性に乗る |
| 実機サインオフ導線（実ポインタ駆動 vs env 巡回の置換/共存） | **M** | hover_inject.rs との整理・実機目視（R7）は非決定的で慎重 |
| **合計** | **M（3–7日）** | additive・donor 潤沢・ただし課題D の到達経路が振れ幅 |

**Risk 総合: Medium**（donor と上流契約が実物ゆえ大半 Low。**課題D のポインタ到達経路が Medium–High**——`HitTest::none()` 下でのイベント到達が spawn.rs/emo-text 描画面の改変を要すると判明した場合、スコープ/エスケープ条項の再判定が要る）。

---

## 6. 設計判断項目（要件ディスカッション／設計へ送る）

1. **【最重要】バルーンポインタイベント到達経路**: `HitTest::none()`（spawn.rs:174）のバルーン窓で選択肢行のポインタ移動/クリックをどう受けるか。窓 `HitTest` 変更（spawn.rs 改変）／content 側 `alpha_mask`（emo-text 描画面境界）／選択肢表示中トグル、のいずれか。**〔要件ディスカッション議題1で裁定済〕**: 到達に窓生成側の最小改変が要る場合、**本 spec がその最小改変を負う**（R7 無条件 DoD 維持・委譲しない）。窓生成側を扱う `areka-P0-position-persist` は同時進行中で停止不可のため、**衝突時は position-persist へ rebase/merge して統合**（先送りしない）。**合流機構の選択（窓 `HitTest` トグル／content `alpha_mask`／表示中トグル）は本 R-1 の設計課題として残す**。
2. **配線設置場所**: `input_events/mod.rs` 拡張（A-1・brief 事前割当に直結）か サブモジュール隔離（A-2・責務分離）か。
3. **runtime アクセス／自前状態の器**: `Emo2Wiring` 直借用（B-1）か 新 NonSend `BalloonWiring`（B-2・自前 ordinal＋発行シンク集約）か。hover getter 不在の穴埋め（last-injected ordinal 追跡）の置き場所。
4. **`ChoiceSelection` 発行シンク**: `Sender<ChoiceSelection>`（C-1・mpsc・donor 同型）か callback（C-2）か。**要素型 `ChoiceSelection` のフィールド確定**（id/label/scope/references＋その型——scope は `usize`?／references は `Vec<String>` 転写?）。受信 inbox 型は下流契約辺ゆえ本 spec では確定しない旨の明記。
5. **重なり行の決定規則（R1.5）**: `choice_hit_rows`（ordinal 昇順×行昇順）に対し先頭一致か最終一致か。memory「衝突の重なりは画家のアルゴリズム＝後定義が手前」との整合（後 = 高 ordinal を手前とするか）。
6. **バルーン移動の間引き要否**: hover 駆動は runtime 内 pure＋非 kanade 送出ゆえ `MouseMoveThrottle` を流用するか無throttleか（毎移動 inject でも安価か）。
7. **実機サインオフ導線の整理（R7）**: 実ポインタ駆動を hover_inject.rs の env 巡回導線と**置換**するか**共存**（env 導線は残しデバッグ用）か。auto-exit＋`RUST_LOG` grep のサインオフ材料（`ChoiceSelection` 発行 info ログ）の定義。
8. **stale 追随の実装点**: 自前保持 ordinal を選択肢消滅（`choice_active` false 遷移）にどう追随させるか——毎移動/クリックで `choice_active`＋`choice_hit_rows` を再読し、非存在 ordinal は None 整合（R3.4）。上流原子性（apply_cue）に完全に乗れるか、自前の消滅検知が要るか。

---

## 7. Research Needed（設計フェーズで深掘り）

- **R-1（最重要）**: wintf ポインタ配信が `HitTest::none()` 窓へイベントを届けるか、content `alpha_mask` 側で受けるのか（`crates/wintf/src/ecs/pointer/`／`layout/hit_test/`／`hit_region/` の配信規則）。到達しなければ課題D の設計が spawn.rs/emo-text 改変へ波及。
- **R-2**: emo-text 描画面（`TextSurface`/バルーン content widget）と `BalloonWindowMarker` 窓の entity 関係——選択肢行の `HitRectPx`（窓物理 px）とポインタ `client_point`（窓 client 物理 px）の原点一致を実測確認（`HitRectPx` doc は「バルーン窓 client 座標系物理 px」と明記＝整合見込みだが要検証）。
- **R-3**: `ChoiceSelection` 受信先（choice-select-events W6 の inbox 型）の暫定 seam——M1 で「発行されるが受け手なし」を許容する結線形（Sender を no-op receiver へ／未結線のまま檻観測のみ）の妥当性。
- **R-4**: `PointerState` のクリック種別——`on_char_pointer_pressed` は `DoubleClick` を見るが、R2.1 は**左シングルクリック**確定。`PointerState` に単発左クリックの相（press/release）がどう表現されるか（`DoubleClick::None`＋押下相か別フィールドか）を確認（`wintf::ecs::pointer::PointerState`）。

---

## 8. 設計フェーズへの推奨

- **好ましいアプローチ（暫定）**: A-2（サブモジュール隔離）×B-2（新 NonSend `BalloonWiring` に自前 ordinal＋発行シンク集約）×C-1（`Sender<ChoiceSelection>` mpsc）。donor（`MouseWiring`）と最も同型で、決定論檻が mpsc 観測に乗り、hit 判定純関数を独立網羅できる。**ただし課題D（R-1）の結論が全体スコープを左右する**ため、設計は R-1 を最優先で解いてから配線形を確定すること。
- **キャリー研究**: R-1〜R-4（特に R-1 の `HitTest::none()` 到達経路）→ **設計フェーズで全件解決済み（§9）**。
- **境界厳守**: 上流（choice-render/emo-text）の描画・幾何・hover API・cue ワイヤ形を消費のみ（R8.5・新 cue variant 新設禁止）。spawn.rs 本体を改変せず（R4.4・改変必要ならエスケープ条項で position-persist へ）。`CuePlayer::resolve_choice` 直接呼出禁止（R2.6/R5.4）。DD-IE-10 物理 px 素通し（k=1.0）を破らない（R4.2）。
- **次コマンド**: `/kiro-design areka-P0-choice-interact`（設計生成→検証→設計ディスカッション）。

---

## 9. 設計フェーズ追記（2026-07-24・kiro-spec-design 実施）

> Discovery 種別: **Extension（light discovery・統合点特化）**。外部依存追加なしのため Web 調査は不要、コードベース実測のみで裁定。以下は design.md の決定の証跡。

### 9.1 Research Log: R-1（最重要）——バルーンポインタ到達経路の解決

- **Context**: `spawn.rs:174` の `HitTest::none()` 下でバルーン窓の選択肢行にポインタイベントが届くかが最大の未確定点だった（§4 課題D）。
- **Sources**: `crates/wintf/src/ecs/layout/hit_test/mod.rs`（`HitTestMode`・`hit_test_entity`/`hit_test_in_window`・`AlphaMaskResource`）／`crates/wintf/src/ecs/pointer/nchittest_cache.rs`（HTCLIENT/HTTRANSPARENT）／`crates/wintf/src/ecs/window_proc/mouse_move.rs`・`mouse_click.rs`・`mouse_dblclick_wheel.rs`（hit→`PointerState` 付与）／`crates/wintf/src/ecs/pointer/dispatch/mod.rs`（Tunnel/Bubble 配信）／`crates/wintf/src/ecs/clickthrough/controller.rs`（`WS_EX_TRANSPARENT` トグル判定）／`crates/areka-emo-present/src/mount.rs:144`（surface entity＝`HitTest::alpha_mask()`＋`AlphaMaskResource`・offset (0,0)）／`crates/areka/src/emo2_boot/frame.rs:432-453`（バルーン窓へ `attach_target`＋初回 `ShowSurface` 面0）／`crates/areka/src/placement/spawn.rs`（clickthrough 登録は `GhostWindowMarker` 全窓＝バルーン含む）。
- **Findings**:
  1. `HitTest::none()` は**窓 entity 自身**をヒット候補から外すだけ。ヒットテストは窓サブツリーを走査し、バルーン窓には emo-present mount の **surface entity**（窓の子・offset (0,0)）が `HitTest::alpha_mask()`＋`AlphaMaskResource`（presenter `apply` ごとに合成結果から更新）で存在する。バルーン枠ビットマップ（面0）は本体不透明ゆえ選択肢行位置でヒット成立。
  2. OS 段: clickthrough 機構はカーソル位置の `hit_test_in_window` 結果で `WS_EX_TRANSPARENT` をトグル（ヒットあり→OFF＝自窓受領）。`cached_nchittest` も同判定で HTCLIENT を返す。→ 不透明バルーン上では WM_MOUSEMOVE／WM_LBUTTONDOWN が届く。
  3. ECS 段: WM_MOUSEMOVE はヒットした surface entity へ `PointerState` を付与（ヒット無しフォールバックは窓 entity）。`dispatch_pointer_events` が親チェーン（surface→窓）を Tunnel→Bubble で巡回し経路上の `OnPointerMoved`/`OnPointerPressed` を呼ぶ——**バルーン窓 entity にハンドラを装着すれば Bubble 相で受信できる**。
  4. WM_MOUSELEAVE は `PointerState` 除去＋`PointerLeave` マーカー 1 フレーム付与。`OnPointerExited` は dispatch されない（Moved/Pressed のみ）。
- **Implications（裁定）**: **窓 `HitTest` トグル／content `alpha_mask` 付与／表示中トグルのいずれも不要**。既存到達性の上にハンドラ post-spawn 装着のみで足りる。**spawn.rs 改変ゼロ＝position-persist との衝突なし（rebase/merge 条項は不発動）**。到達契約＝「presented バルーン面のα不透明領域」であり、Revalidation Trigger として design.md に明記。

### 9.2 Research Log: R-2（座標原点）／R-3（発行シーム）／R-4（クリック表現）

- **R-2 解決**: emo-present mount は `Arrangement.offset = (0,0)`（`mount.rs:63-72`,`:166`）＝ surface 原点はバルーン窓 client 原点に一致。`HitRectPx` は `to_window_physical`（`choice.rs:260-289`）で validrect 原点（＝TextSurface 窓内装着 offset と同源）からバルーン窓 client 物理 px へ写像済み。`PointerState.client_point`（`Point{x,y}: i32`・窓 client 物理 px）と**同一原点**。DPI 変換なし（k=1.0・DD-IE-10 整合）。
- **R-3 解決**: M1 は `wire_balloon_choice` が std mpsc チャネルを生成し `Sender` を `BalloonWiring` へ、`Receiver` を NonSend `ChoiceSelectionInbox` へ格納（Receiver 生存で send は Err にならない・檻は `try_recv` で一度きり発行を観測）。W6 `choice-select-events` が Inbox を受信処理へ置換する seam として design.md Revalidation Triggers に明記。scope→actor 写像は既存 `ActorKey::from(scope.to_string())`（`frame.rs:461`）の消費。
- **R-4 解決**: `PointerState` に「単一クリック」フィールドは無い。左シングルクリック＝`left_down: bool`（transfer のエッジ検出→1 dispatch のみ有効・dispatch 後クリア）。`OnPointerPressed` は left/right/middle いずれかの down で発火するためハンドラで `left_down` を選別。`double_click: DoubleClick` は WM_LBUTTONDBLCLK 由来の別表現で、DBLCLK 2 打目も `left_down=true` を伴う（donor `on_char_pointer_pressed` は `double_click` のみ参照＝単一クリックは不送出）。

### 9.3 Design Decisions（DD-CI-1〜11・design.md の正本裁定）

| DD | 裁定 | 根拠 |
|----|------|------|
| DD-CI-1 | **到達経路＝既存αマスク合流・変更ゼロ**。バルーン窓 entity へハンドラ post-spawn 装着のみ（R-1） | spawn.rs／HitTest／clickthrough／emo-text すべて不改変・position-persist 衝突なし |
| DD-CI-2 | **A-2**: サブモジュール `input_events/balloon.rs` 隔離 | 参照資源（runtime 直読）・下流（ChoiceSelection）がキャラ窓配線と別物・mod.rs 肥大回避・brief「input_events モジュールで完結」をサブモジュールとして充足 |
| DD-CI-3 | **B-2**: NonSend `BalloonWiring`（last-injected ordinal per scope＋`selection_tx`）。runtime は `Emo2Wiring::runtime()` 新設アクセサ（additive・`presenter()` 同型）から都度 `Rc` clone | hover getter 不在の穴埋めと発行シンクを donor `MouseWiring` 同型の 1 資源へ集約 |
| DD-CI-4 | **C-1**: `Sender<ChoiceSelection>`（std mpsc）。`ChoiceSelection = { id: String, label: String, scope: usize, references: Vec<String> }`。**ordinal はワイヤ形に含めない**（解決キーは id・ordinal は表示層内部主キーの非漏洩） | donor の入力配信流儀と一致・mpsc 観測が檻に乗る・下流 `ResolveChoice{id}` と整合 |
| DD-CI-5 | 重なり規則＝**逆順走査・最終一致**（`choice_hit_rows` は ordinal 昇順×行昇順→後定義が手前＝画家のアルゴリズム整合）。包含は半開区間 `[left,right)×[top,bottom)` | memory「emo当たり判定の重なりは画家のアルゴリズム」・whole-pixel 行矩形と整合 |
| DD-CI-6 | **throttle なし**・inject は**遷移時のみ**（last-injected dedup） | hover はプロセス内状態書込のみ（kanade 非送出）で安価・`MouseMoveThrottle` の適用対象外 |
| DD-CI-7 | env 巡回導線 `hover_inject.rs` と**共存**（不変・既定 no-op・デバッグ用）。実機サインオフは実ポインタ経路の `info!(event = "choice_selected", ...)` grep＋`AREKA_APP_SMOKE_EXIT_MS` 有界 auto-exit。両方有効時は後勝ち書込（デバッグ限定・許容） | 置換は choice-render の決定論資産を壊すリスクだけあって益なし |
| DD-CI-8 | stale 追随＝**毎イベント現行スナップショット再読**（`choice_active`＋`choice_hit_rows`）。click は現行 rows からのみ構成。`choice_active` 偽観測時は自前 hover 状態のみ None 整合（注入しない——上流 apply_cue 原子性が正本） | §2.1 の上流原子性に完全に乗る・自前の消滅検知不要 |
| DD-CI-9 | click 確定＝**Bubble 相かつ `left_down`**。`double_click` フィールド不参照（DBLCLK 2 打目も独立 press）。二重発行防止は dispatch のエッジ検出（構造的）＋1 dispatch＝高々 1 send | R2.1（左シングルクリック）・R2.4（同一クリック二重発行なし）を機構レベルで充足 |
| DD-CI-10 | handled 戻り値: moved＝常に `false`（非侵襲）・pressed＝発行時のみ `true` | 既存挙動（ドラッグ等）への非干渉 |
| DD-CI-11 | **窓外離脱の hover 解除**: `PointerLeave` マーカー（既存機構・FrameFinalize クリア）を読む排他システム `clear_balloon_hover_on_leave` を Input スケジュール（dispatch 後）へ登録。判断は `hover_action(active, None, last)` の再利用 | `OnPointerExited` 非配信＋高速離脱のエッジサンプル飛びで hover が残置し R1.3 意図／R7.1 目視を毀損するため（設計レビューゲートでの補修） |

### 9.4 Synthesis 結果

- **Generalization**: 過剰一般化なし。純関数核は rows スライス入力の汎用形に留め、M2（ホイール/キーボード）用の抽象は作らない。
- **Build vs Adopt**: wintf の `OnPointer*` dispatch・`PointerLeave`・clickthrough・αマスク hit test、donor の NonSend/attach/借用分割/合成 PointerState テスト形、hover_inject の借用規律をすべて**採用**。新規外部依存ゼロ。
- **Simplification**: throttle 撤去（DD-CI-6）・runtime mock シーム不採用（純関数分割で不要——「檻に入れるのは判断分岐のみ」）・`ChoiceSelection` は要件必須 4 フィールドのみ・`RegionSource` 型シームの複製もしない。

### 9.5 Risks & Mitigations（設計後更新）

- ~~課題D: ポインタ到達経路（Medium–High）~~ → **解消**（DD-CI-1・変更ゼロ）。
- バルーン面のα不透明性への依存（Low）: 選択肢行下が透明なバルーン素材では到達しない——到達契約として design.md Revalidation Triggers に明記。emo2 バルーンは不透明本体で成立。
- クリック押下がドラッグ Preparing も開始（Low・既存挙動）: press 時点で発行済みのため対話の正しさに影響なし。挙動改変はしない（R4.4）。
- `AREKA_CHOICE_HOVER_INJECT` 併用時の hover 交互書込（Low・デバッグ限定）: 文書化のみ。
- M1 で Inbox 未消費のままキュー滞留（Low）: 発行はユーザークリック頻度のみ・W6 で受信処理へ置換。
