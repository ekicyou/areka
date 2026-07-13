# ギャップ分析（kiro-validate-gap）: areka-P0-surface-resize-resnap

> 生成日: 2026-07-13 / 対象: 確定済み requirements.md（承認前）＋現行コードベース
> 目的: 実行時サーフェスサイズ変化 → 窓 resize＋下端 re-snap を張る 1 本のシームについて、既存資産・欠落能力・実装案・研究事項を提示する（決定は要件/設計ディスカッションへ）。

---

## 1. 現状調査（Current State Investigation）

### 1.1 brief.md 挿入点の実在確認（2026-07-13 実コード偵察の追認）

brief.md が名指しした挿入点はいずれも**現存**し、周辺コードを実読して以下を確定した。

| 挿入点 | 実在 | 実読で確定した事実 |
|---|---|---|
| `crates/areka/src/placement/measure.rs` `measure_scope_sizes`（62） | ✅ | spawn 時のみ採寸。初期 surface（scope0=id0→434×687／scope1=id10→336×400）を bind なし合成で採寸し `ScopeInput` を返す。**実行時サイズ変化は関与しない**（採寸後アセット破棄）。 |
| `crates/areka/src/placement/spawn.rs` `spawn_ghost_windows`（149） | ✅ | `ScopePlacement` 由来の物理 px を `WindowPos{position,size}` へ焼き込む（`window_pos`・245）。`BottomSnap` marker（89）が bottom 吸着スコープのキャラ窓を識別。`GhostWindows`（Resource＋戻り値）が scope→窓 Entity の正本。 |
| `crates/areka/src/placement/follow.rs` `move_window_to`（365） | ✅ | `pub`・`#[allow(dead_code)]`（呼び手待ち）。単一ライターの正規口。内部 `enqueue_window_move`（411・private）は **`SWP_NOSIZE` 固定＝移動のみ・リサイズ不可**。`BottomSnapPolicy`（76）／`DragPositionPolicy`（56 trait）／`MonitorSnapshot`／`work_area_for_window`（516）が既存。 |
| `crates/areka-emo-present/src/presenter.rs` `apply_show`（201） | ✅ | 初回 `ShowSurface` で `SwapChainPresenter::new(w,h)`（300）を遅延生成、以後は `chain.upload`（333）＋`mount.set_bounds(world,size)`（351）。**新寸を外へ通知するシームは皆無**（要確認どおり不在）。現在サイズは `chain.size()`（`text_slot_view` 経由で参照可）。 |
| `crates/areka/src/emo2_boot/frame.rs` `run_drain_phase`（474） | ✅ | `wiring.rx.try_iter()` で `PresentCommand` を FIFO drain し `presenter.apply(world,cmd)`。**apply は `()` を返し適用寸を報告しない**（撃ちっぱなし）。ここが配送候補。 |

### 1.2 欠陥の機序（実読で確定）

- **presenter 側は既にサーフェス内容をリサイズしている**: `SwapChainPresenter::upload`（chain.rs:173）は `(w,h) != self.size` で `ResizeBuffers`＋source_tex/staging 再作成を自前で行い、`apply_show` は続けて `mount.set_bounds(world,size)` で **visual の bounds** を新寸へ更新する。つまり切替後、**キャラ画像は新寸で正しく描画される**。
- **しかし OS 窓（HWND）の位置・サイズは spawn 時のまま**。`WindowPos.size` も `enqueue_window_move` が `SWP_NOSIZE` ゆえ更新経路が無い。結果、窓は旧寸・旧 Y に居座り、新しい高さ h' に対し `Y = work_area.bottom − h`（旧 h）のままなので、下端が work area 下端からずれる（h'>h なら下へはみ出す/h'<h なら宙に浮く）。これが「宙に浮く／下端からずれる」の正体。
- **駆動シームの不在**が唯一の欠落: 「新寸を検知して placement に窓 resize＋re-snap を促す」経路が無い。

### 1.3 既存の再利用可能資産（消費・再定義しない）

- **単一位置ライター経路**: `move_window_to`→`enqueue_window_move`（`SetWindowPosCommand::enqueue`＋`WindowPos.position` の bypass 先行反映＋`Arrangement.offset` 直接同期）。bypass 理由（`WM_WINDOWPOSCHANGED` echo との二重発行回避）と GA ヒットテスト境界同期（バルーンのクリック死回避）が既に作り込まれている＝**resize もこの経路へ合流させれば同じ不変量を継承**できる。
- **下端吸着ポリシー**: `BottomSnapPolicy.resolve(raw, size, snapshot)`（純粋写像・X 素通し/Y=`bottom−h`・非正寸/空 snapshot は identity 縮退）。**size を差し替えて呼ぶだけ**で新寸の下端 Y が出る。
- **モニタ解決**: `MonitorSnapshot`＋`work_area_for_window`（窓中心帰属・half-open・最近傍フォールバック）。resize 後の中心が属するモニタの下端で live 算出でき、モニタ跨ぎも自然。
- **窓リサイズ基盤（wintf）**: `SetWindowPosCommand`（command.rs）は `width/height`＋`flags` を保持し `guarded_set_window_pos` で発行。**`SWP_NOSIZE` を外して cx/cy を渡すだけでリサイズ可**（新 API 不要）。echo ガード（`SELF_INITIATED_DEPTH`）も既存。
- **scope→窓 Entity 正本**: `GhostWindows::char_window(scope)`／`balloon_window(scope)`。`BottomSnap` marker で吸着スコープを判別。
- **UI 配送ブリッジ**: `areka-actor` の `spawn_ui`/`UiSender<M>`（unbounded async_channel・`M: Send+'static` 任意・非ブロック・全 sender drop で終了）。rustdoc に「窓移動指令の型を下流がそのまま載せられる」と明記（Req4.5 の器）。既存 `PresentCommand` は別途 std `mpsc::Receiver` で `run_drain_phase` が drain。
- **TargetId 採番規約**: `shell_target(scope)=2*scope`（偶数）・`balloon_target(scope)=2*scope+1`（奇数）（`target_map`・DD-3）。**シェル target のサイズのみがキャラ窓 resize を駆動**（バルーンは follow の領分）。

### 1.4 規約・制約（steering / メモリ）

- Rust 2024・新規 crates.io 依存なし・tokio 不使用（tech.md／brief）。
- **位置ライターは単一**（DragPositionPolicy 単一ライター原則・振動排除）。`enqueue_window_move` を迂回した bypass 書込は新設しない（Req3.4・バルーンのクリック死前歴）。
- **物理 px 単一通貨**・DPI 再スケールを挟まない（`follow.rs` U1/U4・2026-07-05 二重スケール欠陥の檻）。
- **本番ゴースト先行＋実 DPI（≠96）目視受け入れ**（`areka-placement-real-ghost-first` メモリ・window-placement リジェクトの教訓）。
- **log-first / silent failure 禁止**（不在・非正寸は `warn!` 以上＋no-op）。
- **areka は bin-only**（no lib）＝内部到達テストは in-crate `#[cfg(test)]`（メモリ `areka-bin-crate-internal-tests-in-crate`）。emo-present はライブラリ crate ゆえ通常の tests/in-source 双方可。

---

## 2. 要件→資産マップ（Requirement-to-Asset Map・gap タグ）

| 要件 | 既存資産 | ギャップ | タグ |
|---|---|---|---|
| **R1** 実行時サイズ変化検知（emo-present） | `apply_show` が新寸を保持（`chain.size()`／`entry.composed.width/height`） | **直前表示寸のベースライン保持と差分判定・変化時の外向き通知シームが無い**（`PresentTarget` に前寸フィールド無し） | Missing |
| **R2** サイズ変化通知の I/O 契約（クロスエンジン単方向） | `UiSender<M>`（器）／`PresentCommand` の Receiver drain 前例 | **サイズ変化メッセージ型（scope＋新寸）と、emo→ghost の単方向配送結線が無い**。emo-present は placement へ依存不可＝通知手段の抽象が要る | Missing / 設計判断 |
| **R3** 窓 resize＋下端 re-snap（placement） | `move_window_to`／`BottomSnapPolicy`／`BottomSnap` marker／`MonitorSnapshot`／`SetWindowPosCommand`(width/height) | **resize 対応の単一ライター口が無い**（`enqueue_window_move` は `SWP_NOSIZE` 固定）。新寸での Y 再計算・`WindowPos.size` 更新・free 分岐・べき等・balloon follow 合流が未結線 | Missing（既存部品の合成） |
| **R4** 実 DPI 本番ゴースト受け入れ＋決定論檻 | follow.rs/spawn.rs の headless 決定論テスト群（偽 HWND・合成 snapshot・96 非倍数座標）／emo-present の golden readback | **DPI パラメタ化（96/120/144/192）した下端 Y 算出の純関数檻・resize 経路の観測テスト・本番ゴースト実 DPI 目視証跡が未整備** | Missing（テスト） |
| **R5** 境界と非所有 | emo2-boot #5（初期非表示）・follow（balloon offset）・emo-present（合成中身）が各所有 | 追随入力（表示寸変化）のみを消費し所有を広げない設計規律の明文化 | Constraint |

---

## 3. 実装アプローチ（複数案・トレードオフ）

論点は 2 つに分解できる: **(A) 検知＋通知の張り方**（R1/R2）と **(B) placement 反映口の形**（R3）。

### 論点A: 検知・通知シーム（R1/R2）

#### 案A-1: emo-present 内でベースライン保持＋外向きサイズ通知（notifier 注入）
`PresentTarget` に `last_shown_size: Option<(u32,u32)>` を持たせ、`apply_show` の可視化成功直後に前寸と比較。差分時のみ、`attach_target` で注入した**汎用 notifier**（`Box<dyn Fn(TargetId,(u32,u32))>` か `UiSender<SurfaceResized>` を型引数で受ける薄いトレイト）へ発火。emo-present は placement 型に依存しない。
- ✅ 検知責務が「表示寸の source」である emo-present に閉じる（brief の第一候補）。R1.1–1.5 を自然に満たす（ベースライン＝初回 ShowSurface・同寸は非発火・寸法のみ対象）。
- ✅ TargetId で scope 識別可能（R1.5）。単方向（R2.2）。
- ❌ emo-present へ notifier 注入 API を増設（`attach_target` 署名 or 新 setter）。テストで偽 notifier を挿す偽装境界が要る。
- ❌ 依存方向の綱渡り: notifier 型を emo-present 側の中立トレイト/型引数で定義しないと `emo-present→areka` 逆流を招く。

#### 案A-2: frame.rs 側で apply 後にポーリング検知（emo-present 無改変）
`run_drain_phase` が `presenter.apply` の後、シェル target ごとに `presenter.text_slot_view(target).surface_size()`（＝`chain.size()`）を読み、wiring 側が持つ per-target ベースラインと比較して差分時に placement 反映を駆動。
- ✅ **emo-present を一切改変しない**（`text_slot_view`/新 accessor で現寸を読むだけ）。依存方向の問題が起きない。
- ✅ 検知・通知・反映が全て areka/UI スレッド・同一 World 内で完結＝チャネル不要で直接呼べる（振動源を増やさない）。
- ❌ R1「検知＝emo-present の責務」の字面から外れる（検知が areka 側へ寄る）。ただし R1 の subject は「サーフェスサイズ変化検知」機構であり所属 crate は縛っていない＝解釈の余地（設計判断）。
- ❌ `text_slot_view` は mount 未生成時 `None`＝初回表示前は読めない。シェル用に現寸 accessor（`shown_size(target)->Option<(u32,u32)>`）を emo-present に足す方が素直（小さな additive）。
- ❌ ベースライン所有が wiring 側になる（per-target `HashMap<TargetId,(u32,u32)>`）。

#### 案A-3: apply が適用寸を戻り値で返す（`apply -> Option<AppliedSize>`）
`presenter.apply` を「適用した表示寸を返す」形へ変更し、`run_drain_phase` が戻り寸を使って差分判定。
- ✅ ポーリングより直接的・None 経路が明快。
- ❌ `apply` の現行契約（`()`＋reply 経由）を破壊し、`Hide`/`InvalidateCache` との戻り型統一が要る。emo-present の既存テスト群への波及大。

**R2 の配送実体（チャネル vs 直接呼び）**: 検知（emo-present, UI スレッド）と反映（placement, UI スレッド・`&mut World`）は**同一 UI スレッド・同一 World**。ゆえに Req2 の「メッセージを placement へ届ける」は (i) `UiSender<SurfaceResized>`＋別 drain（PresentCommand と同型・疎結合だが 1 フレーム遅延の可能性）と (ii) 同一 frame system 内での直接関数呼び（`resize_window_to(world,...)`・即時・チャネル無し）の二択。**Req2.4 は「既存 UI 配送ブリッジへ載せ新規フレームワークを導入しない」**を要求するが、直接呼びは「新規フレームワークを導入しない」を最も強く満たす一方、「単方向メッセージの I/O 契約」(R2.1) の字面は message 型の存在を示唆＝**設計判断で確定すべき**（下記 DD-2）。

### 論点B: placement 反映口（R3）

#### 案B-1: `resize_window_to` を additive 新設（推奨候補・brief 準拠）
`move_window_to` と同型の `pub fn resize_window_to(world, window, new_size) -> bool` を follow.rs に足し、内部は「①現在 `WindowPos.position` を読む ②`BottomSnap` 有無で分岐: 吸着なら `BottomSnapPolicy.resolve(pos, new_size, snapshot)` で新 Y、free なら現 Y 保持 ③新寸＋新座標を **resize 対応の単一ライター**で 1 回書く ④`follow_balloon` で随伴」。resize 対応のため `enqueue_window_move` を `enqueue_window_set_pos(world, window, x, y, Some(size))`（`SWP_NOSIZE` を size 有無で切替）へ一般化するか、姉妹 private を足す。
- ✅ 既存部品（policy/snapshot/marker/balloon follow）を最大再利用。単一ライター原則・bypass 規律・Arrangement 同期を継承（振動・クリック死を回避）。
- ✅ べき等（R3.6）・非正寸縮退（R3.9・policy が既に identity 縮退）・不在/未付与 no-op（R3.8・`enqueue` が warn+false）を既存挙動で満たしやすい。
- ❌ `enqueue_window_move` の一般化は既存 6 テストに影響しないよう慎重に（`SWP_NOSIZE` 経路の後方互換を保つ）。
- ❌ `WindowPos.size` の bypass 先行反映も足す必要（現行は position のみミラー）。

#### 案B-2: `move_window_to` を拡張して size も受ける
`move_window_to(world,window,x,y,Option<size>)` に統合。
- ✅ 口が 1 つ・呼び手が明快。
- ❌ 既存 `move_window_to` は `#[allow(dead_code)]` の呼び手待ちだが署名変更は全テスト（7 本）改修。resize と move の意味が混ざる。

**推奨方向（情報提供）**: A は **A-2（emo-present 無改変ポーリング＋小 accessor）か A-1（notifier 注入）**が有力で依存方向の綺麗さと R1 字面の忠実さのトレードオフ。B は **B-1（`resize_window_to` additive）**が既存原則との整合で有力。いずれも「新機構は最小＝検知＋駆動口」に徹する brief 意図と一致。最終選択は設計ディスカッションで確定。

---

## 4. 研究事項（Research Needed・設計フェーズへ持ち越し）

1. **`SetWindowPos` の move+resize 同時発行時の echo/`WM_WINDOWPOSCHANGED` 二重反映**: `enqueue_window_move` の bypass は move 前提で作り込まれている。size も同時に変えると `apply_window_pos_changes`／`sync_window_arrangement_from_window_pos` の再発行や `WindowGraphics`/swap chain 側の窓リサイズ追随がどう絡むかを実 DPI で確認（GA 零寸ガードの前提が size 変化で崩れないか）。
2. **emo-present の visual bounds（`mount.set_bounds`）と OS 窓寸の関係**: 窓を新寸へ resize した後、`VisualMount`/WUC visual の bounds・swap chain 供給面が窓クライアント領域と一致するか（描画が窓内に収まり切るか・クリップされないか）を本番ゴースト実表示で確認。
3. **バルーン随伴の Y 追随**: キャラ窓の Y が re-snap で動くと `BalloonFollow.offset` 維持で balloon も動く。balloon 窓自身の下端吸着は無い（follow の領分）ため、resize 起因のキャラ Y 変化で balloon が画面外へ出ないか（offset が負で上方の場合）を確認（R3.7 の非破壊範囲）。
4. **通知の粒度と多発**: `\s` 切替のたびに同寸なら非発火（R1.3/R3.6）だが、talk 中の高頻度切替で resize が連続する場合の視覚的安定性（案A-2 の 1 フレーム遅延 vs 直接呼びの即時）。
5. **DPI パラメタ化檻の設計**: 下端 Y 算出（`bottom − h`）自体は DPI 非依存の純算術ゆえ、R4.4 の「DPI をパラメタ化（96/120/144/192）」は**何を DPI で振るのか**を明確化（work_area/寸法は物理 px 単一通貨で DPI に依らない＝檻は「異なる寸法値の集合」で網羅すれば足りる可能性）。実 DPI 目視は最終確認に留める線引きを設計で固定。

---

## 5. 複雑度・リスク（Effort / Risk）

- **Effort: S–M（3–7 日）**。新規部品は「サイズ変化メッセージ型（or notifier）1 個」＋「emo-present の小 accessor or notifier 注入」＋「placement `resize_window_to` 1 関数（既存 policy/enqueue の一般化）」＋「決定論檻＋実 DPI 目視」。既存パターンの合成が主で、新奇技術は無い。
- **Risk: Medium**。
  - 位置ライター経路（bypass／Arrangement 同期／echo ガード）に size 変化を足す部分は、**バルーンのクリック死・窓振動という既発の実機ブロッカ面**に触れる（follow.rs の doc が警告）＝実 DPI 実表示での回帰確認が必須。
  - emo-present↔placement の依存方向を汚さない通知抽象の設計が肝（案 A の選択次第）。
  - dpi=96 自己整合が欠陥を隠す前歴（window-placement リジェクト）＝**実 DPI 目視証跡が DoD 前提**で、決定論檻だけでは GO にできない観測条件が重い。

---

## 6. 設計ディスカッションへ送る決定事項（番号付き）

> **要件ディスカッション（2026-07-13）での再framing**: requirements.md を「**シェル座標系（下端アンカー）→ ウィンドウ座標系（サーフェス寸法）の変換 T の恒常維持**」を幹に組み替えた。T = 既存 `BottomSnapPolicy`（`window.top_Y = work_area 下端 − surface.height`）。サーフェス切替のたび新 `surface.height` で T を再適用するのが本質。旧 R1（検知エンジン）は Req3 のべき等・冗長回避へ降格、旧 R2（クロスエンジン通知メッセージ契約）は Req4 の「配送実体は設計判断・新規フレームワーク不導入」制約へ降格（検知＝emo-present・反映＝placement は同一 UI スレッド／同一 World ゆえ「通信」でなく同一 World のデータ依存）。ドラッグ（アンカー移動）と resize（T の入力 h 変更）は同一 T・同一単一ライターへ合流。以下 DD の設計判断としての有効性は不変（特に DD-2 が Req4.3 の委任先）。
>
> **理想形指示（2026-07-13・議題2）**: 開発者判断で T を `bottom` 特化でなく **`seriko.alignmenttodesktop` 全 5 値（`top`／`bottom`／`left`／`right`／`free`）汎用のアンカー射影**として要件化（ukadoc: 既定 `bottom`・優先度チェーン＝ゴースト全体＜ゴーストスコープ個別＜シェル全体＜シェルスコープ個別・実行時 `\![set,alignmenttodesktop]` で可変）。`left`／`right` では **幅 w** が再射影の駆動軸（`right`: `left_X = wa.right − w`）＝「高さだけ見る」は `bottom` 限定。既存 `BottomSnapPolicy` は T の `bottom` 事例、他アンカーはその一般化。新規 DD を下に追加。旧 R4.4（DPI パラメタ化）議題は「物理 px 単一通貨ゆえアンカー辺算出は DPI 非依存＝檻は寸法・work area 値の網羅＋全アンカー、DPI 依存性は R5.1–5.3 の `bottom` 実機目視」で確定（Req5.4）。

- **DD-9（アンカー射影 T の一般化構造）**: `BottomSnapPolicy`（`DragPositionPolicy` trait）を 5 アンカー射影へどう一般化するか。案: (a) `AnchorProjection` enum＋辺固定の共通式、(b) アンカーごとの policy 実装＋dispatch。既存 `move_window_to`／`enqueue_window_move`（`SWP_NOSIZE` 固定）を X/Y 双方＋size 対応へ一般化する方式（`right` は X 駆動）を含む。
- **DD-10（解決済みアンカーの表現と provenance）**: 現行は `BottomSnap` marker の有無で `bottom`／`free` の二値のみ。5 値アンカーをキャラ窓へどう表現・保持するか（marker 群 vs `Anchor` component）。優先度チェーン解決（parsers／window-placement）と `\![set,alignmenttodesktop]` routing（seriko）は本 spec 非所有（Req4.2／Req6.3）＝上流が解決済みアンカーを供給する interlock を design で確定。
- **DD-11（アンカー変化トリガの配送）**: サイズ変化（Req1.3）に加えアンカー変化（Req1.4）も T 再適用トリガ。両トリガを同一反映口へどう合流させるか（DD-2 の配送実体選定と併せて確定）。

- **DD-1（検知の所属 crate）**: サイズ変化検知を emo-present 内（案A-1: notifier 注入）に置くか、areka/frame.rs 側（案A-2: apply 後ポーリング＋emo-present に現寸 accessor 追加）に置くか。R1 の subject「サーフェスサイズ変化検知」の所属 crate を確定する。
- **DD-2（R2 配送の実体）**: 通知を UI 配送ブリッジ上の**単方向メッセージ型**（`SurfaceResized{ scope/target, size }` を `UiSender` で送り別 drain）とするか、同一 UI スレッド・同一 World ゆえ**同一 frame system 内の直接関数呼び**（チャネルなし・即時）とするか。Req2.1（メッセージ）と Req2.4（新規フレームワーク不導入）の両立線を引く。
- **DD-3（ベースライン所有）**: 「直前表示寸」を emo-present の `PresentTarget` が持つか、areka wiring の per-target マップが持つか（R1.2）。
- **DD-4（placement 反映口の形）**: `resize_window_to` を additive 新設（案B-1）か `move_window_to` を size 付きへ拡張（案B-2）か。併せて内部 `enqueue_window_move` を resize 対応へ一般化する方式（`SWP_NOSIZE` の条件切替・`WindowPos.size` の bypass ミラー追加）を確定。
- **DD-5（対象の限定）**: キャラ窓 resize を駆動するのは**シェル target（偶数＝`2*scope`）のサイズのみ**とし、バルーン target（奇数）のサイズ変化は窓 resize を駆動しない（バルーン配置は follow の領分）ことを明文化。TargetId→`GhostWindows::char_window(scope)` の写像規約（`scope = target/2`）を固定。
- **DD-6（free スコープの扱い）**: `BottomSnap` marker 不在（free）のキャラ窓は寸法のみ反映し Y 保持（R3.3）。marker 有無で分岐する既存 `on_char_drag` と同じ静的引き方に合わせる。
- **DD-7（R4 決定論檻の DPI パラメタ化の意味）**: 物理 px 単一通貨で下端 Y 算出が DPI 非依存な点を踏まえ、R4.4 の「DPI をパラメタ化（96/120/144/192）」を「多様な寸法値の集合での網羅」と読むか「実 DPI を模した入力生成」と読むかを確定（実 DPI 目視は最終確認に限定する線引き）。
- **DD-8（多発 resize の視覚安定）**: 同寸非発火（R1.3/R3.6）に加え、talk 中の連続切替での即時性 vs 1 フレーム遅延（DD-2 の帰結）をどう受容するか。
