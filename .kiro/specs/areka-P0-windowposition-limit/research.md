# Gap Analysis: areka-P0-windowposition-limit

> 生成: 2026-08-14（kiro-validate-gap）。要件（requirements.md・確定済）と現行コードベースの差分分析。
> 本書は情報と選択肢を提供する（最終決定は設計フェーズ・要件ディスカッションの領分）。

## 1. 現状調査（Current State Investigation）

### 1.1 `windowposition` の既存パイプライン（数値のみ・完成済）

| 層 | 場所 | 現状 |
|---|---|---|
| パース | `crates/areka-parsers/src/balloon/parse.rs:69-72` | `windowposition.x`/`y` を `get_scalar::<i32>`（`:148-149`）で読む。**非数値は無警告で `None` へ降格**（`value.parse::<T>().ok()`）＝要件 4.6 が是正対象とする「無警告な同一視」の実体。`limit` キーは**読んでいない**（`grep limit` は balloon パーサ全体で 0 件） |
| モデル | `crates/areka-parsers/src/balloon/model.rs:131-137` | `WindowPosition { x: Option<i32>, y: Option<i32> }`（`#[non_exhaustive]`・アクセサ `x()`/`y()`）。キーワードも limit も表現不能 |
| 供給 | `crates/areka/src/placement/windowposition.rs:75-120` | `to_screen_adjust`（数値→物理 px 調整量・k は `scale_signed`→`ScaleRatio::scale_len` 権威）→ `apply_windowposition` が `cfg.scopes[scope].balloon_offset` へ加算合流。resolver P1〜P5 無改変の供給層設計（design D1'） |
| 取得 | `crates/areka/src/placement/mod.rs:386-470` | `apply_scope_windowpositions` → `scope_windowposition` が scope 別 2 層マージ済みバルーン定義（`resolve_balloon_faces`＋`load_scope_balloon_model`・面 0 の上書き層）から `WindowPosition` を取り出す。**観測点 4 の `info!`（`mod.rs:411-421`）が既存の `windowposition` 観測ログ**＝要件 6.2 の「同水準」基準 |
| 配置式 | `crates/areka/src/placement/resolver.rs:126-216` | P5（`:184-194`）: バルーン基本位置は `BalloonSide::Left`＝`char_x − balloon_w`／`Right`＝`char_x + char_w`・`balloon_y = char_y`（上端揃え）＋ `balloon_offset` 加算。**「クランプなし（バルーンは work area 外へ素直にはみ出す）」が doc に明記**（`:113-116`）＝limit=0 相当・正典既定と逆 |

### 1.2 バルーン位置の**全書き込み経路**（limit 保証の掛け場所候補の全数）

要件 2.5「どの経路で最後に位置が書かれたかによらず保証」の対象となる経路を実測列挙した:

1. **初期配置（spawn）**: `resolve_placement`（P5）→ `main.rs:721` で永続 merge（`persist.rs::apply_restored_placements`）→ `spawn.rs:311` が `ScopePlacement.balloon_pos` を初期 `WindowPos` バンドルとして直書き。**単一ライター経路（`enqueue_window_set_pos`）を通らない**。
2. **復元 merge**: `persist.rs::apply_restored_placements`／`project_restore`（`persist.rs:160-244`）——**キャラ窓だけ**アンカー再射影＋work area 内 clamp。`BalloonOffset` は生値をそのまま採用（`:299-302`）＝バルーン側は復元でも無制限。
3. **`\![move]` cue／連鎖確定の随伴**: `follow/window_move.rs:39-65` `move_window_to` が対象キャラ窓を書き、`BalloonFollow.offset` 恒等式でバルーンを**直接 enqueue**（`:54-61`・可視性ガードなし）。呼出元は `emo2_boot/move_cue.rs` と **scg 新設の `emo2_boot/frame/drain_resnap.rs:324`（`finalize_chain` の反映）** の 2 つ。brief 追記(63) の「第 2 の位置ライター」はバルーン視点ではこの随伴経路。
4. **リサイズ・再スナップ随伴**: `resize_window_to`（`window_move.rs:135-331`）→ `follow_balloon`（`drag_follow.rs:350-386`）。`BalloonFollowTrigger::Placement(route)` で可視性遷移ガードが 4 route（AnchorChange/Resnap/DpiReproject/ReportedSizeReconcile）に限り発火。
5. **キャラ窓ドラッグ随伴**: `on_char_drag`／`on_char_drag_end`（`drag_follow.rs:63-242`）→ `follow_balloon(Drag)`＝**ガード適用外**（明示操作の尊重）。
6. **バルーン単独ドラッグ**: `on_balloon_drag`（`drag_follow.rs:494-539`）——バルーン窓は `DragConfig { move_window: true }` で **wndproc レベルで移動済み**。ハンドラは offset 記憶更新のみで、**位置書き込みが ECS 経路を通らない**（limit を継続適用する場合はこの経路だけ機構が別になる。要件 3.4 は SSP がユーザー操作を制限しないなら制限しないと定めるため、SSP 観測次第でこの経路は対象外になり得る）。
7. **バルーン DPI リサイズ**: `resize_window_keep_position`（`window_move.rs:638-701`）——位置据え置き・寸のみ変更。**寸が変わると位置不変でも矩形が制限領域からはみ出し得る**（内包判定は矩形＝位置×寸）。

**結論**: 単一の関門は現存しない。runtime 書込は `enqueue_window_set_pos`（`window_move.rs:452`）へほぼ集約されているが、(1) spawn 初期値・(6) wndproc ドラッグの 2 つが構造的に外にある。

### 1.3 既存の近縁機構（limit と混同しやすいもの・要件は「変更しない」と規定）

- **可視性遷移ガード** `follow/visibility.rs::guard_visibility`: 「可視 → 全 work area 非交差」の**遷移だけ**を **X の clamp のみ**で防ぐ安全網。部分はみ出しは素通し（檻 `follow_visibility_guard_tests.rs:12-35` が「部分可視は clamp しない」を明示固定）。limit=1 の 4 辺完全内包とは別物。route 表 `route_applies_visibility_guard`（`visibility.rs:178-186`）と `BalloonFollowTrigger`（`drag_follow.rs:312-330`）が発火可否語彙として存在——**limit の適用時点表（要件 3.2）を同じ形で表現できる先例**。
- **キャラ窓 P4 クランプ** `resolver.rs::clamp_axis`（`:221-223`）: `v.min(hi).max(lo)`＝逆転区間で left/top 優先。**要件 2.3「キャラ窓の既存クランプと同一の優先規則」の正典実装**（1 軸・矩形版はまだ無い）。
- **モニタ帰属** `follow/work_area.rs::work_area_for_window_with_origin`: 窓中心 half-open 帰属＋最近傍 fallback。要件が「帰属規則は変更しない」とするもの。**注意: `MonitorSnapshot` は work area しか持たない**（`work_area.rs:21-24`）——要件 3.1(e) が「画面全体」に確定した場合、モニタ bounds の追加転写が必要（wintf `Monitor` は bounds を持つ）。

### 1.4 テスト規約・観測規約

- 決定論檻の配置: 純関数はモジュール内 `#[cfg(test)]`＋`#[path]` 分離ファイル（`resolver_resolve_tests.rs` 等）。DPI パラメタ化（k=1/k≠1 で同一表を回す）が既確立。
- 観測: `diag::log_window_move`（`[diag.window_move]` target・route/kind/scope/x/y/size/dpi）＋ `VISIBILITY_UNRESOLVED_TAG` 系 warn。要件 6.1 の「補正前後・契機」ログはこの語彙系に相乗りできる。
- **要件 7.3 の反転対象（実在確認済み）**:
  - `resolver_resolve_tests.rs:812`「left バルーンは work area 左外（負方向）へ素直にはみ出す」——limit 既定 1 実装後、この期待は「limit をどの層で掛けるか」次第で反転または前提書換え。
  - `follow_visibility_guard_tests.rs:29-35`「部分可視は clamp しない（美観政策は本 spec 非所有）」——limit=1 では部分はみ出しも補正対象（適用時点に該当する場合）。ガード自体は不変でも、**doc コメントの前提記述**は追随が要る。
  - `follow_visibility_balloon_wiring_tests.rs:103` `balloon_drag_trigger_neither_clamps_nor_warns`——SSP 観測 3.1(c) の結果次第で維持または改訂。
  - `resolver.rs:113-116` P5 doc「クランプなし」・`windowposition.rs` 冒頭 doc——値の意味を変えたら全下流の宣言を洗う（steering 記憶: doc 主張は file:line で裏取り）。

## 2. 要件 ↔ 資産マップ（ギャップタグ付き）

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| R1 limit 語彙受理 | scope 別 2 層マージ取得経路（`scope_windowposition` と同型で流用可） | **Missing**: パーサに `limit` キーなし・モデルにフィールドなし・0/1 検証＋警告縮退なし・runtime へ運ぶ器（ScopeConfig or Component）なし |
| R2 limit=1 の 4 辺内包補正 | `clamp_axis`（1 軸・優先規則の正典）・物理 px 通貨は全経路統一済（R2.9 は既存規約で充足） | **Missing**: 矩形版の純関数クランプなし。バルーンへの適用点なし。**Constraint**: 書込経路が 7 系統（§1.2）・うち 2 系統は単一ライター外 |
| R2.5 経路非依存の定常保証 | `enqueue_window_set_pos` 集約（部分的）・`ChainFinalized` 一度きり確定機構 | **Missing**: 最終関門なし。**Constraint**: scg で「キャラ窓の必ず域内」保証自体が確定経路で破れている（本仕様はバルーンのみ所有） |
| R3 SSP 実測 | 実測手法の先例あり（scg の DPI aware ポーリング・`ssp-oracle-notes.md`・kero-balloon R7.6） | **Unknown（プロセスギャップ）**: 3.1(a)〜(f) 全項が未観測。実装形を左右する最大の未知 |
| R3.1(e) 制限領域 | `MonitorSnapshot.work_areas` | **Unknown → 条件付き Missing**: 「画面全体」確定ならモニタ bounds の転写追加が必要 |
| R4 キーワード語彙 | `get_scalar` 寛容パース・`WindowPosition` non_exhaustive・`BalloonSide` による P5 基本位置分岐の先例 | **Missing**: キーワードの型表現・中央上/中央下の基本位置幾何（P5 は Left/Right しか知らない）・`y` 基本位置切替・不正値の警告縮退 |
| R4.7 保存値優先 | `apply_restored_placements` の merge 順序（既存） | 変更不要（キーワードは初期既定位置の供給に閉じる設計なら自動成立） |
| R5 回帰境界 | 数値経路は檻で厳密固定済（`windowposition.rs` 檻 9 ほか・`resolver` T-R 系） | **Constraint**: limit=0＋非キーワード時の bit 同一を保つ実装形が必須（供給層パターンの継承が有利） |
| R6 観測性 | 観測点 4 `info!`・`[diag.window_move]`・warn タグ体系 | **Missing（小）**: 補正発火ログ・limit/キーワード解決値ログ・縮退警告の 3 行分 |
| R7 検証・COMPAT | 檻の流儀・実機サインオフ手順（絶対パス・AREKA_APP_SMOKE_EXIT_MS・grep 突合）確立済 | **Missing**: 檻本体・COMPAT §8 :145 行の更新・R7.3 の反転（§1.4 列挙） |

## 3. 実装アプローチ選択肢

### 3.1 論点 A: limit=1 クランプをどこに掛けるか（本仕様最大の設計判断・brief 追記(63) の名指し事項）

**前提**: 適用時点は R3 の SSP 実測が確定するまで決めない（測ってから式を書く）。以下は「観測結果がどちらに出ても収容できる形」の比較。

- **案 A-1: 書き込み口ごとに適用（分散）**
  - §1.2 の該当経路（SSP 観測で確定した時点に対応する経路のみ）へ個別にクランプ呼出を置く。
  - ✅ R3.2「SSP が適用しない時点では適用しない」を経路単位で素直に表現。既存の route/trigger 語彙（`PlacementRoute`・`BalloonFollowTrigger`）へ自然に載る。
  - ❌ **新しい書き込み経路が増えるたび素通しになる**——まさに scg が `finalize_chain` でキャラ窓 P4 を素通しにした欠陥と同型の構造リスク。R2.5 の「経路非依存保証」が規律頼みになる。
- **案 A-2: 単一関門へ集約（`enqueue_window_set_pos` 内 or 直前）**
  - バルーン窓への全書込が通る最下流でクランプ。
  - ✅ R2.5 が構造保証になる。
  - ❌ spawn 初期値（§1.2-1）と wndproc バルーンドラッグ（§1.2-6）は関門の外＝結局 2 箇所は別掛けが要る。❌ 適用時点の除外（例: SSP がドラッグ中は適用しないと観測された場合）を関門内で trigger 分岐する必要があり、`enqueue_window_set_pos` の「挙動を持たない配管」という現契約（route は観測語彙）を変質させる。❌ ドラッグ由来の書込は route=None 契約であり判別材料が不足。
- **案 A-3: ハイブリッド（推奨候補）＝純関数 1 本＋「時点＝呼出点」の表明示**
  - (i) 矩形内包クランプの**純関数**（`clamp_axis` の矩形版・left/top 優先・limit=0 は恒等）を resolver 近傍へ 1 本だけ新設し、決定論檻で全網羅（R7.1）。
  - (ii) 適用点は SSP 観測結果を**適用時点表**（`route_applies_visibility_guard` と同じ網羅 match の流儀）として明文化し、該当経路（最低限: P5 直後 or spawn 前の merge 済み `ScopePlacement`・`follow_balloon`・`move_window_to` の随伴腕）から同一関数を呼ぶ。
  - (iii) R2.5 の定常保証は「起動系列の最後の書込点」（scg の `ChainFinalized` 確定と `spawn`/restore）を檻＋実機ログで固定する。
  - ✅ 式は 1 箇所（二重化しない）・時点は表 1 枚（黙って倒れない）・R5 の回帰境界（limit=0 恒等）が関数契約で守れる。❌ 呼出点が複数に散る事実は残る（表と wiring 檻で緩和）。

### 3.2 論点 B: `limit`・キーワードの語彙をどの層で型にするか

- **案 B-1: parsers を転記のまま太らせない（生値保持＋下流解釈）**
  - `BalloonModel` へ `windowposition.limit` と `x` の**生文字列**を additive に保持（`WindowPosition` は non_exhaustive・既存 `x(): Option<i32>` 据え置き＋`x_raw()`/`limit_raw()` 追加）。0/1 検証・キーワード判別・警告縮退（R1.3/R4.6・warn は placement 側）は `placement` で行う。
  - ✅ steering「parser は転記層・解釈は下流」「面引数は不透明文字列・解決は下流」と整合。✅ 既存消費者（`mod.rs:406`・`areka-emo-present/balloon.rs:516-523` の観測ログ）が無改変。
  - ❌ 「数値としても読めるが生値も要る」の二重取得になり、`get_scalar` との整合（数値 x はどちらの経路で読むか）を design で 1 本化する必要。
- **案 B-2: parsers で typed enum へ**（`WindowPositionX::Px(i32) | Center | Top | Bottom | Invalid(String)`）
  - ✅ 型で語彙が閉じ、下流の分岐が enum match で網羅強制。
  - ❌ `x()` の型変更は既存消費者を壊す（アクセサ追加で回避可能だが二重 API 化）。❌ 警告記録（R4.6）をパーサに置くとパーサの純粋転記契約と衝突、下流に置くと Invalid 保持が必要でどのみち生値が要る。
- 評価: **B-1 が現行アーキテクチャの摩擦最小**。B-2 は語彙の型安全が勝るが、転記層契約の変更という追加コストを伴う。

### 3.3 論点 C: キーワードの基本位置（中央上／中央下）をどこで幾何にするか

- **案 C-1: resolver P5 を additive 分岐で拡張（第一級）**
  - `ScopeConfig`（または供給される scope 別バルーン配置指定）へ「配置モード」＝`Side(BalloonSide) | CenterTop | CenterBottom` を導入し、P5 が `char_x + char_w/2 − balloon_w/2`（水平中央）・`char_y − balloon_h`（center/top）／`char_y + char_h`（bottom）を直接計算。`y` 数値は既存 `balloon_offset` 加算がそのまま調整量になる（R4.4）。
  - ✅ 幾何が寸法入力を持つ層（resolver）に置かれ、式が読める・恒等式 `balloon_offset ≡ balloon_pos − char_pos` は事後条件計算で自動維持。✅ 非キーワード時は既存分岐へ 1 bit も触れない形にでき R5.2 を守りやすい。
  - ❌ 「P1〜P5 無改変」を保ってきた供給層設計（design D1'）を初めて破る＝resolver 檻の増設が必要（ただし additive）。
- **案 C-2: 供給層で offset へ焼き込む（P5 無改変を維持）**
  - `apply_scope_windowpositions` が採寸済み寸（`MeasureScopeSizes`）から「Left 基本位置との差」を計算して `balloon_offset` へ合流。
  - ✅ resolver 無改変。
  - ❌ **boot 採寸寸で焼き込んだ offset は実表示寸の確定（scg の一度きり連鎖再解決・`resize_window_to` の窓相対維持）とずれる**——「中央」が実表示寸で中央でなくなる。SSP 裁定（バルーンは窓相対・リサイズで offset 補正しない）とキーワードの「常に中央」意味論の整合を offset 表現で取るのは不可能に近い（R3.1(f) の焼き付き観測とも絡む）。❌ 式が Left 基本位置経由の回りくどい差分になり読めない。
  - 補足: そもそもキーワードは「**初期既定位置**の供給」に閉じる（R4.7・保存値優先）ため、初期配置の一回性だけなら C-2 でも成立し得るが、初期配置自体が実表示寸確定でやり直される現行構造（scg）では C-1 が安全。
- 評価: **C-1 優勢**。ただし resolver 拡張の影響半径（R5.2 の bit 同一檻）を design で先に固定すること。

### 3.4 論点 D: limit 値を runtime へ運ぶ器

初期配置時は `PlacementConfig`/`ScopePlacement` 経由で足りるが、SSP 観測が「追従・ドラッグ中も適用」に出た場合は UI スレッド runtime（`follow_balloon` 等）から scope 別 limit を引く必要がある。候補:
- **D-1**: `BalloonWindowMarker`（scope 既持ち）への相乗り or 新 Component `BalloonLimit`（spawn で焼込み）——`Anchored` の先例と同型・単一真実源が窓 entity に住む。
- **D-2**: `ScopeConfig.balloon_limit` ＋ Resource 化した配置構成を runtime に残す——現在 `PlacementConfig` は spawn 後に捨てられており新規保持が要る。
- 評価: D-1 が既存パターン（`Anchored(Anchor)` 焼込み→follow が消費）に一致。

## 4. 工数・リスク

| 項目 | 見積 | 根拠 |
|---|---|---|
| Effort | **M（3–7 日）** | 純関数＋語彙は S 相当だが、SSP 実測（R3・実機 2 水準）・適用点 wiring・R7.3 の既存檻反転・実機サインオフが加算。scg/kero-balloon の同型作業実績に整合 |
| Risk | **Medium** | 技術は既知パターンの延長（Low 要素）だが、⑴ SSP 観測結果が実装形（A/C の選択・atom⇄wpl ウェーブ再判定）を左右する・⑵ バルーン位置ライター 7 系統の網羅漏れが「静かに素通し」になる構造リスク・⑶ resolver 接触時の R5 bit 同一維持、の 3 点で Medium |

## 5. Research Needed（設計フェーズへ持ち越す調査項目）

1. **SSP 実測 R3.1(a)〜(f) 全項**（本仕様の最優先・実装形の分水嶺）。手法は scg の DPI aware 読み取り専用ポーリング＋プロファイル削除初回起動の先例を流用。emo2 実機（キャラを画面端へドラッグ）で limit=1 既定挙動を観測。
2. **制限領域が「画面全体」だった場合の `MonitorSnapshot` 拡張要否**（3.1(e) 従属・wintf `Monitor.bounds` の転写 1 フィールド追加で足りる見込み）。
3. **バルーン DPI リサイズ（`resize_window_keep_position`）で寸が変わった直後の SSP 挙動**——位置不変でも内包が破れるケースを SSP が補正するか（3.1 の派生観測・要件行列には明示されていないが k≠1 サインオフで踏む）。
4. **`\![move]` でバルーン相対ごと画面外へ出る台本指示の扱い**——明示操作（MoveCue）は SSP でどうなるか（3.1(b) の亜種として同時観測可能）。
5. **既存檻の反転インベントリの確定**（§1.4 の 4 件が起点・実装時に `grep "はみ出\|クランプなし"` で全数再確認）。
6. **atom⇄wpl ウェーブ再判定のトリガ**: SSP 観測が「追従・ドラッグ中も適用」に出たら follow 系ファイルへ触れるため roadmap 干渉台帳（`roadmap.md:93`）の再判定を仰ぐ（要件 Adjacent expectations 明記済・確定後に報告）。

## 6. 設計判断アイテム（要件ディスカッションへの送付事項）

1. **limit クランプの掛け場所**（§3.1・案 A-1/A-2/A-3）——推奨候補は A-3（純関数 1 本＋適用時点表＋定常保証檻）だが、SSP 観測確定が前提。
2. **語彙の型化層**（§3.2・案 B-1/B-2）——parsers 転記契約を守る B-1 か、型安全の B-2 か。
3. **キーワード基本位置の幾何の所在**（§3.3・案 C-1/C-2）——P5 拡張（D1' の「無改変」原則を破る）を許すか。
4. **limit 値の runtime 伝搬**（§3.4・案 D-1/D-2）——Component 焼込み（`Anchored` 同型）か構成 Resource 残置か。
5. **可視性遷移ガードとの適用順序**——limit=1 が掛かる経路ではガードの X clamp と二重補正になり得る（limit の 4 辺内包はガードの「交差あり」条件を常に満たすため、順序を「limit 後にガード」とすればガードは恒常 Keep＝無害。ただし limit=0 scope ではガードが従来どおり最後の安全網）。ガード自体は不変（要件 Out of scope）だが順序の明文化が要る。
6. **`enqueue_window_set_pos`／`WindowMoveRecord` への補正観測の載せ方**（R6.1）——既存 `[diag.window_move]` に補正フラグを足すか、独立 warn/info 行か。
7. **バルーン単独ドラッグ経路（wndproc 移動）の扱い**——SSP 観測 3.1(c) が「戻さない」なら現行不変で確定（R3.4）。「戻す」なら wndproc 移動窓への事後補正という新機構が要る（影響大・観測前に設計しない）。

## 7. 分析手法の記録（Document Status）

- コードベース調査: Grep/Glob/Read によるバルーン位置ライターの全経路実測列挙（§1.2）・パース〜配置〜追従〜永続の各層の file:line 裏取り（doc 主張は書く前に file:line で裏取りの規律に従う）。
- 外部調査: 不要と判断——正典（ukadoc）の確定事項は要件 Introduction に転記済み・SSP 実測は R3 が要件として所有（設計前の実測フェーズで実施）。
- steering 整合: `roadmap.md`（W6.5・scg 実形へ rebase 必達・atom⇄wpl/wpl⇄bod 干渉行）・COMPAT §8 :145（追跡行）・parser 転記層/丸め権威/実機サインオフ各記憶と突合済み。本 worktree は scg マージ済み main 由来であり brief 追記(63) の 3 ファイル（`chain_finalize.rs`・`drain_resnap.rs`・`spawn.rs` の `default_char_pos`）は突合済み（§1.2-3）。

## 8. 次のステップ

- `/kiro-requirements-discussion areka-P0-windowposition-limit` で §6 の設計判断アイテムを裁定（特に 1〜4）。
- SSP 実測（R3）は設計フェーズ冒頭のコード非接触作業として実施可能（scg の実測プローブ手順を流用）。
- その後 `/kiro-design areka-P0-windowposition-limit` へ。
