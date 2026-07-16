# ギャップ分析: areka-P0-position-persist

> 対象: 確定済み requirements.md（Req 1〜8）／brief.md／steering。生成日 2026-07-16。言語 ja。
> 目的: 確定要件と既存コードベースの差分を洗い出し、設計フェーズの実装戦略判断へ材料を渡す。**決定はしない**（選択肢と論点の提示）。

## 分析サマリ（要点）

- **注入口はほぼ全て既存**。復元は `spawn_ghost_windows` の `placements` 引数、保存トリガは `on_char_drag_end`／`on_balloon_drag`、初回ゲート＋vanish は kanade 構築時 `KanadeConfig`＋boot cascade の `Phase::BootInit` 分岐、モニタ構成変化縮退は `project_anchor`（5 アンカー射影）が既に存在する。本 spec は「新機構を作らず入力を差す」層に徹せる（brief の設計意図と整合）。
- **不在なのは永続化そのもの**: versioned `GhostState` 値モデル・原子的 IO（temp→rename）・寛容読取・復元 merge 純関数・保存ライタの結線・**初回起動ゲート（現状 boot cascade は毎回無条件で OnFirstBoot 発火）**・vanish 読取経路（`events::on_first_boot()` は Ref0="0" 固定）。
- **主要リスクは 2 点**: (1) 永続座標の表現（物理 px＋モニタ識別 vs 論理正規化）——2026-07-05 DPI 座標取り違え欠陥の再来面。Req1.6「仮想デスクトップ上の一貫位置・プライマリ丸めしない」が制約。(2) 所有境界——復元は areka 本体 placement（`open_startup_window`）で、初回ゲート／vanish は `areka-ghost::boot`→`KanadeConfig` で、**同一 GhostState を 2 箇所が読む**構図。
- **依存方針の分岐**: serde は workspace 依存に無く dola ローカル依存に留まる。areka-ghost/areka へ serde を足すのは「追加＝要承認」（brief 制約）。回避案＝既存 `areka-parsers::kv`（寛容 KV BTreeMap）＋自前 KV ライタ（既存の ghost.dat plant テストが `position.0.x,9999` の KV 形式を使っている）。
- **陳腐化テストの意図的更新**: `placement/mod.rs` の `prepare_never_reads_or_writes_ghost_dat`（503-565）は「ghost.dat を読まない/書かない」を固定する檻で、本 spec が新契約（plant→復元）へ書き換える対象（obsolete-vs-broken-test-policy）。kanade の `events::on_first_boot_is_get_with_fixed_zero_ref0` 等も vanish 経路化で更新対象。

---

## 1. 現状調査（既存資産・パターン）

### 1.1 窓配置・復元注入口（areka 本体 `crates/areka/src/placement/`）

- **`prepare_ghost_windows(ghost_root, balloon_root) -> Result<PreparedPlacement, PlacementError>`**（`mod.rs:145`）と、決定論テスト用 work_area 注入版 `prepare_ghost_windows_with_work_area`（`mod.rs:162`・**偽装境界パターンの先例**）。`PreparedPlacement { placements: Vec<ScopePlacement>, titles }`（`mod.rs:86`）。
- **復元注入口＝`spawn_ghost_windows(world, placements: &[ScopePlacement], titles) -> GhostWindows`**（`spawn.rs:139`）。外部 `placements` 引数から物理 px を `WindowPos` へ転記。**初期位置を外から与える口は既にある**。
- **陳腐化する檻**: `prepare_stages`（`mod.rs:121`）に「位置の記憶・復元（ghost.dat）は一切行わない（2.11）」と明記、`prepare_never_reads_or_writes_ghost_dat`（`mod.rs:503-565`）が「(a) ghost.dat を書かない (b) plant しても出力不変」を固定。**本 spec がこの契約を反転する**。plant 例は KV 形式 `position.0.x,9999\r\n`。
- **`ScopePlacement`**（`resolver.rs:66`）: `scope`／`char_pos`／`char_size`／`balloon_pos`／`balloon_size`／`balloon_offset`／`anchor` を持つ。全て `Copy + PartialEq + Eq + Debug`、**serde 派生なし**。値型群（`PointPx`/`SizePx`/`RectPx`/`Anchor`）は std＋tracing のみ依存（wintf 非依存＝純粋テスト可）。
- **既定位置解決**: `resolve_placement(cfg, work_area, scopes)`（`resolver.rs:124`・P1〜P5 純関数）。復元値が無いときのフォールバック本体。

### 1.2 ドラッグ観測点・窓移動 API（`placement/follow.rs`）

- **保存トリガに使える観測点**: `on_char_drag_end`（`follow.rs:319`・非 Free アンカーのキャラ窓の最終確定位置）、`on_balloon_drag`（`follow.rs:443`・バルーン単独ドラッグ）。キャラ窓通常移動は `on_char_drag`（`follow.rs:260`）。
- **バルーン相対 offset はセッション内のみ記憶**: `BalloonFollow.offset`（`follow.rs:225`）を `on_balloon_drag` が `balloon_pos − char_pos` へ更新。doc コメントに **「永続化 ghost.dat は M-life の領分」**（`follow.rs:220-223`）＝本 spec 宛の明示的申し送り。
- **復元に使える公開 API**: `move_window_to`（`follow.rs:500`・物理 px・随伴バルーン追従）／`resize_window_to`（`follow.rs:551`）。単一ライター反映口 `enqueue_window_set_pos`（`follow.rs:727`・bypass ミラー＋Arrangement 同期）。
- **モニタ構成変化縮退の既存正本**: `project_anchor(anchor, raw, size, snapshot) -> PointPx`（`follow.rs:143`・5 アンカー射影 T・bottom 吸着維持・graceful degradation）。`MonitorSnapshot`（`follow.rs:813`）／`work_area_for_window`（`follow.rs:850`・窓中心→帰属モニタの純関数）。**Req5 の「作業領域外→アンカー再射影」はこの資産の再利用で成立する**（新規実装不要）。
- **UI スレッド契約**: follow の窓操作は `&mut World` のみ（channel/actor 型を持たない）＝UI スレッド専有を型で担保。

### 1.3 初回起動ゲート・vanish 経路（kanade `crates/areka-kanade/`）

- **現状 boot cascade は初回判定なし**: `schedule/boot.rs` の `on_reply`、`Phase::BootInit + Notified` で**無条件に `events::on_first_boot()` を発火**し `BootType` へ（`boot.rs:52-60`）。2 回目以降スキップの分岐が存在しない。
- **vanish 固定値**: `events::on_first_boot()`（`events.rs:47`）は Ref0=`"0"` 固定。doc に「M1 は vanish count 等の永続値を持たない（永続化は position-persist の領分）」（`events.rs:44-46`）。
- **構築時注入の器＝`KanadeConfig`**（`msg.rs:123`）: `spawn_kanade(config, shiori, sakura)`（`actor.rs:50`）で move 保持され `step(state, input, &config)` 全経路へ参照渡し（`actor.rs:71,95`）。**初回フラグ／vanish count の additive フィールドの自然な置き場**（brief の想定と一致）。
- **kanade は純粋状態機械＋潤沢な決定論テスト資産**: `schedule/mod.rs`／`boot.rs`／`events.rs` に happy-path 全網羅＋ログ発火檻。additive フィールドは**既定値で「毎回 OnFirstBoot」を不変に保つ形**にしないと既存テストを壊す。
- **`events` は `pub`**（DD-9 例外・`schedule/mod.rs:31`）で `tests/` 統合ハーネスが期待値に再利用。`on_first_boot()` の署名変更（vanish 引数追加）は fixture／ハーネスへ波及する。

### 1.4 ghost 結線層・起動経路（`crates/areka-ghost/`）

- **`boot(options) -> Result<GhostRuntime, GhostBootError>`**（`runtime.rs:301`）: マウント解決→`resolve_kanade_config(&mount, enc)`（`config.rs:28`）→ 各アクター spawn → `KanadeMsg::Boot`。**初回フラグ／vanish は `resolve_kanade_config` で `KanadeConfig` に載せて注入するのが自然**。
- **ghost 識別キーの供給源＝`MountModel`**（`areka-parsers/src/package/model.rs:29`）: `shiori.dir`（= ghost/master・**物理存在確定**）と `shell.dir`。**per-ghost 状態の分離キー（Req7）はこの ghost path から導出できる**。`MountModel` は `#[non_exhaustive]`・serde 派生なし。
- **永続ファイルを読む口・書く慣行は無し**: `config.rs` は shell descript の `name` のみ読取。ghost path 配下への書込み慣行なし。
- **本番の boot 呼び出しは 2 経路**: (a) `emo2_boot::wire_emo2_boot`（`emo2_boot/mod.rs:300`・実 sink 経路）、(b) `main.rs` の LogSink フォールバック（`main.rs:287`）。**どちらも `areka_ghost::boot` を通る**＝GhostState 読取を boot 内に置けば両経路を一度でカバーできる。

### 1.5 エントリポイント・終了 flush 口（`crates/areka/src/main.rs`）

- **窓生成順序**: `open_startup_window(&app, &cfg)`（`main.rs:269,476`）が `prepare_ghost_windows`→`spawn_ghost_windows` を async CommandSender 経路で結線。`MonitorSnapshot::from_monitors` を Resource 挿入（`main.rs:483,499`）。**この関数は UI スレッド・COM 初期化済みで同期 IO 可**（`prepare_ghost_windows` が既に同期 IO）。復元 GhostState 読取＋merge の自然な置き場。
- **shutdown flush 口**: `run()` 復帰後 `runtime.shutdown(CloseReason::User)`（`main.rs:321-328`）。`GhostRuntime::shutdown`（`runtime.rs:169`）は best-effort 完走。**最終保存 flush の結線候補**（main.rs または GhostRuntime::shutdown）。
- **順序の含意**: `open_startup_window`（窓復元）は `wire_emo2_boot`（ghost boot＝初回ゲート）**より前**に走る。GhostState を 2 度読むか一度読んで両者へ配るかは設計判断（§4 論点7）。

### 1.6 依存・テスト規律（steering／memory）

- **serde は workspace 依存に不在**（root `Cargo.toml:15-34`）。dola がローカルに serde/serde_json/toml を宣言。areka-ghost の依存は areka-*／windows／tracing／thiserror のみ、areka も serde 無し。**serde 追加＝新規依存＝要承認**（brief 制約「serde 系が要るなら既存ツリー内依存で・追加は要承認」）。
- **areka は bin-only（no lib）**（memory areka-bin-crate-internal-tests-in-crate）＝内部項目を触るテストは in-crate `#[cfg(test)]`。
- **決定論テスト必達**（memory deterministic-test-coverage-mandate）＝IO 以外は純関数化して全網羅、ファイル IO は temp dir 注入の偽装境界。
- **log-first・silent failure 禁止**（memory areka-log-first-no-silent-failure）＝失敗は error!/warn!＋縮退、永続化失敗で起動を殺さない（Req6）。
- **座標契約**: 物理 px 単一通貨（placement design U1-U5）。2026-07-05 DPI 座標取り違え欠陥（memory areka-window-placement-dpi-coordinate-defect）＝WindowPos は物理・BoxStyle は論理の混在が事故元。**永続座標の表現決定は実 DPI・マルチモニタで検証必須**（Req8.4）。

---

## 2. 要件→資産マップ（ギャップ種別: Missing / Unknown / Constraint）

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| **R1 窓位置保存/復元** | `spawn_ghost_windows` の `placements` 注入口・`on_char_drag_end` 観測点・`move_window_to`／`resolve_placement` フォールバック | **Missing**: GhostState 値モデル・保存トリガ結線・復元 merge 純関数。**Unknown**: 保存座標表現（物理 px＋モニタ識別 vs 論理正規化・R1.6 の「仮想デスクトップ一貫位置・プライマリ丸めしない」制約） |
| **R2 バルーン相対 offset 永続化** | `BalloonFollow.offset`（session-only・`follow.rs:220` 申し送り）・`on_balloon_drag` 更新 | **Missing**: offset の GhostState 保存＋boot 復元適用。session-only の持ち上げ |
| **R3 初回ゲート（OnFirstBoot）** | boot cascade `Phase::BootInit` 分岐・`KanadeConfig` 構築時注入・204 フォールスルー（`boot.rs:65-69` 既存） | **Missing**: 初回判定分岐（現状無条件発火）・起動記録の保存。**Constraint**: kanade 決定論テスト資産を既定値で不変に保つ additive フィールド形 |
| **R4 vanish 読取経路（Ref0）** | `events::on_first_boot()`（Ref0="0" 固定・`events.rs:47`） | **Missing**: vanish count を GhostState→Ref0 へ転記。`on_first_boot()` 署名変更（pub・fixture/ハーネス波及） |
| **R5 画面内縮退（モニタ構成変化）** | `project_anchor`（5 アンカー射影・`follow.rs:143`）・`MonitorSnapshot`／`work_area_for_window` | **ほぼ充足**: 既存射影の boot 復元パスでの再利用のみ。**Missing**: 復元時に射影を通す結線（現状 project_anchor 消費者は drag/resize のみ） |
| **R6 頑健性（寛容読取・失敗縮退）** | log-first 規律・`PlacementError` 分類の先例（`is_benign_*`）・寛容パーサ（`areka-parsers::kv`） | **Missing**: 寛容読取（破損/欠損/未知形式→既定縮退）・原子的書込（temp→rename・部分失敗で旧状態非破壊） |
| **R7 ゴースト単位スコープ** | `MountModel.shiori.dir`（ghost path・存在確定） | **Missing**: ghost 識別キー導出＋per-ghost 保存先解決。未知バージョン→R6 縮退（Constraint: 将来 vanish／ゴースト切替消費者向けのキー構造だけ最初から持つ・過剰実装しない） |
| **R8 受入検証（決定論 unit＋実機）** | 偽装境界パターン（work_area 注入・temp dir・fake handle）・in-crate test 慣行 | **Missing**: roundtrip／寛容縮退／再射影／初回ゲートの決定論檻。**Constraint**: 実機（実 emo2・実 DPI≠96・マルチモニタ）人間サインオフ必達（R8.4） |

---

## 3. 実装アプローチの選択肢

永続化には複数の独立した設計軸があり、各軸で選択肢がある。以下は「主要 4 軸 × 案」で整理する。

### 軸A. 所有クレートと GhostState 読取箇所

**Option A1（areka-ghost 所有・boot 内単一読取）**: `GhostState` と IO を `areka-ghost` に置き、`boot()` 内で一度読み、初回フラグ/vanish は `KanadeConfig` へ、窓復元値は `GhostRuntime` 経由で main へ返す。
- ✅ 読取が boot の 2 経路（wire/fallback）を一度でカバー・単一真実源。
- ❌ 窓復元は `open_startup_window`（boot より前）で必要＝`GhostRuntime` 経由の受け渡しだと順序が合わない。boot を前倒すか復元値だけ別 API に切る必要。

**Option A2（areka-ghost 所有・読取ヘルパを 2 サイトが呼ぶ）**: `GhostState::load(ghost_key)` を公開し、`open_startup_window`（窓復元）と `boot`（初回/vanish）が各々読む。
- ✅ 既存の窓生成順序（placement→ghost boot）を崩さない・各サイトが必要分だけ読む。
- ❌ 同一ファイルを 2 度読む（IO 重複・軽微）。読取の一貫性（間で書き換わらない）は起動直後ゆえ実害小。

**Option A3（areka 本体所有）**: `GhostState` を areka 本体に置く。
- ✅ 窓復元・保存トリガ（follow）と同じクレート。
- ❌ 初回/vanish は areka-ghost→kanade の構築経路にあり、areka 本体型を areka-ghost へ渡す逆依存になる。層規律に反する。

> 推奨検討: **A2** が順序制約と層規律の両立で最有力。ghost 識別キー導出（MountModel）を areka-ghost が持つ点とも整合。設計で確定。

### 軸B. 直列化形式

**Option B1（自前 KV・areka-parsers 再利用）**: 保存を `key,value` 行の KV とし、読取は `areka-parsers::kv::parse_kv`（寛容 BTreeMap・後勝ち）、書込は自前フォーマッタ。バージョンは `version,1` 行。
- ✅ **新規依存ゼロ**（要承認回避）。既存 plant テストの KV 形式と一致・寛容読取が既定で手に入る（未知キー passthrough＝R6/R7 の未知形式縮退と親和）。
- ❌ ネスト構造（scope×種別×座標）を平坦キー（`position.<scope>.x` 等）へ手で写像。型安全は自前 parse/format で担保。

**Option B2（serde＋TOML/JSON）**: `#[derive(Serialize/Deserialize)]` した `GhostState` を toml/serde_json で。
- ✅ 構造保持・派生で完結・dola に serde 実績。
- ❌ **areka-ghost/areka への新規依存＝要承認**。寛容読取（破損許容）は serde だと「厳格失敗→縮退」で受ける形になり R6 の粒度設計が要る。

> 推奨検討: **B1** が brief 制約（新規依存回避）と R6 寛容読取に最も素直。B2 は承認前提の対抗案として設計に併記。

### 軸C. 保存トリガの頻度と flush 結線

**Option C1（DragEnd 毎に即書込）**: `on_char_drag_end`／`on_balloon_drag` で毎回 GhostState 更新＋原子的書込。
- ✅ シンプル・クラッシュ耐性（都度確定）。
- ❌ ドラッグ終了ごとにファイル IO（頻度低ゆえ許容範囲だが UI スレッド上の同期 IO）。

**Option C2（dirty フラグ＋終了時 flush）**: DragEnd では在メモリ GhostState を更新しフラグ立て、`main.rs` shutdown（`main.rs:321`）／`GhostRuntime::shutdown` で最終 flush。
- ✅ IO 回数最小。
- ❌ クラッシュ時に未 flush 分が失われる（R6.2/6.3 の「保存失敗で旧状態非破壊」は守れるが最新位置は落ちる）。正常終了経路（run 復帰）が主で M1 受容可か要判断。

> いずれも**保存ライタの所在（GhostState を持つ Resource か actor か）と単一ライター規律・UI スレッド契約**が論点（§4-8）。C1/C2 ハイブリッド（DragEnd 即書込＋shutdown flush の二重化）も可。

### 軸D. 保存座標の表現（最重要・R1.6）

**Option D1（物理 px＋モニタ識別子）**: WindowPos の物理 px をそのまま＋所属モニタ識別（デバイス名/矩形）を併記。復元時に該当モニタを引き当て、無ければ `project_anchor` で作業領域内へ再射影。
- ✅ WindowPos が既に物理 px＝変換ゼロ・単一通貨維持（2026-07-05 欠陥面を作らない）。
- ❌ モニタ構成/解像度変化で物理座標が意味を失う→R5 再射影で救済（既存 project_anchor で成立）。モニタ識別子の永続表現を決める必要。

**Option D2（論理正規化・仮想デスクトップ相対）**: 仮想デスクトップ原点相対や DPI 論理値で保存。
- ✅ DPI 変化に理屈上頑健。
- ❌ **論理/物理混在の再来リスク**（2026-07-05 欠陥）。WindowPos は物理ゆえ保存/復元で往復変換が要り、変換式の検証負荷が高い。

> 推奨検討: **D1**（物理 px＋モニタ識別＋復元時 project_anchor 再射影）が既存座標契約と R5 資産に最も整合。R1.6 の「プライマリ丸めしない・仮想デスクトップ一貫位置」は、保存を物理 px の仮想スクリーン絶対座標として持てば満たせる（モニタ識別は再射影のヒント）。**設計冒頭で ukadoc `descript_shell` の `seriko.alignmenttodesktop` と bottom 吸着の整合を確認して確定**。

### 初回ゲート／vanish の実装形（軸横断・kanade）

- `KanadeConfig` に additive フィールド（例 `first_boot: bool`／`vanish_count: u32`・**既定＝初回扱い/0 で現行不変**）を追加。
- boot cascade `Phase::BootInit + Notified` の分岐を「`first_boot` なら OnFirstBoot GET（BootType へ）／既起動なら OnBoot GET（BootMain へ直行）」に変更。既存 204 フォールスルー（`boot.rs:65-69`）は不変。
- `events::on_first_boot()` を `on_first_boot(vanish_count)` 相当へ（Ref0 を count 由来に）。pub 署名変更ゆえ fixture／ハーネス／`events.rs` の檻を更新。
- 起動記録の書込タイミング（初回 boot 完了時＝`Phase::BootVersion` 完了 or Steady 到達）を決める（R3.4）。boot 完了の観測点は areka-ghost/main 側か kanade 側か（kanade は純粋層ゆえ IO 不可＝ghost 側で「初回だった」事実を保存する結線が要る）。

---

## 4. 設計判断アイテム（要件ディスカッションへ供給）

1. **保存座標の表現**（軸D）: 物理 px＋モニタ識別＋復元時 `project_anchor` 再射影（D1）か、論理正規化（D2）か。R1.6「仮想デスクトップ一貫・プライマリ丸めしない」の解釈と 2026-07-05 DPI 欠陥の回避を両立する形を確定。**最優先論点**。
2. **保存先ディレクトリと ghost 識別キー**: ukadoc `file_structure` の profile 慣行（SSP は `ghost/master/profile/` 系）を design 冒頭で ukadoc MCP 参照して確定。`MountModel.shiori.dir`（ghost/master）基準か areka 名前空間サブディレクトリか。**emo2 fixture を汚さない配慮**（fixture は read-only テスト資産）。ghost 識別キーは path そのままか hash か（Req7・将来のゴースト切替消費者向け構造）。
3. **直列化形式**（軸B）: 自前 KV＋`areka-parsers::kv` 再利用（B1・新規依存ゼロ）か、serde＋TOML（B2・要承認）か。R6 寛容読取の粒度（破損/欠損/未知形式/未知バージョン→既定縮退）をどちらで実現するか。
4. **所有クレートと GhostState 読取箇所**（軸A）: areka-ghost 所有・2 サイト読取（A2）か boot 内単一読取（A1）か。窓復元（`open_startup_window`・placement）と初回/vanish（`areka-ghost::boot`→`KanadeConfig`）が同一 GhostState を読む構図の解決。
5. **初回ゲートの kanade 注入形**: `KanadeConfig` の additive フィールド（既定で現行不変）とし boot cascade `Phase::BootInit` 分岐を追加する形。既存の kanade 決定論テスト資産（happy-path・ログ檻）を壊さない既定値設計。`resolve_kanade_config`（ghost/config.rs）での値源解決の結線。
6. **vanish 読取経路の署名変更**: `events::on_first_boot()`（pub・Ref0="0" 固定）を count 引数化。fixture／統合ハーネス／`events.rs` の檻（`on_first_boot_is_get_with_fixed_zero_ref0` 等）の更新範囲。M1 実値は常に 0（`\![vanish]` 未実装）だが読取経路を正にする。
7. **起動記録の書込タイミングと結線**: 「初回だった」事実を誰がいつ保存するか。kanade は純粋層ゆえ IO 不可＝ghost/main 側で boot 完了を観測して GhostState へ記録する結線（R3.4）。
8. **保存トリガ頻度と保存ライタの所在**（軸C）: **【要件ディスカッション#1 決定】耐久性レベルはハイブリッド（C1＋C2）に確定＝DragEnd で即時に永続ストレージへ確定し、shutdown で最終フラッシュを行う。クラッシュ／強制終了でも直近 DragEnd 時点の位置を次回起動で復元可能に保つ（要件 R1.1／R1.2／R1.3／R2.1 が明文化）。** design に残る論点は機構のみ: GhostState を持つのは Resource か actor か、単一ライター規律・UI スレッド契約（follow が `&mut World` のみで完結する既存契約）との整合、shutdown flush 口（`main.rs:321`／`GhostRuntime::shutdown`）の選定、DragEnd 即時書込の原子性（§4-11 の temp→rename と併せて）。
9. **バルーン相対 offset の永続化と復元適用**（R2）: `BalloonFollow.offset`（session-only）の GhostState 保存と、boot 復元時の `spawn_ghost_windows` への適用（`ScopePlacement.balloon_offset` 経路へ復元値を差すか、別途 offset 復元パスか）。
10. **陳腐化テストの更新範囲**（obsolete-vs-broken-test-policy）: `placement/mod.rs:503-565`（ghost.dat 不使用の檻）を新契約（plant→復元）へ書換え。kanade `events` の vanish 固定値檻・boot cascade の初回無条件発火を前提にした檻の更新。
11. **原子的書込の具体**（R6.3）: temp→rename の Windows 上の挙動（同一ボリューム前提・`std::fs::rename` の atomic 性）、書込中断で旧状態を破壊しない保証、tempfile クレート不使用（手書き temp 名＋rename）の可否。

---

## 5. 工数・リスク

| 領域 | 工数 | リスク | 一言根拠 |
|---|---|---|---|
| GhostState 値モデル＋原子的 IO＋寛容読取 | S–M | Low–Med | 純粋モデルは既存値型に倣える。IO は temp→rename＋偽装境界で決定論化。形式決定（軸B）次第 |
| 復元 merge 純関数（復元値∪既定 resolver） | S | Low | `resolve_placement` 出力形へ merge するだけ・全網羅容易 |
| モニタ構成変化の再射影（R5） | S | Low | `project_anchor` 既存資産の boot パス再利用のみ |
| 保存トリガ結線（DragEnd／shutdown flush） | M | Med | UI スレッド・単一ライター・保存ライタ所在（Resource/actor）の設計判断 |
| 初回ゲート＋vanish 経路（kanade boot cascade） | M | Med | `KanadeConfig` additive＋boot 分岐＋`events` 署名変更。既存決定論テスト資産を壊さない既定値設計と fixture 波及 |
| 保存座標表現の確定＋実機検証 | S–M | **High** | 2026-07-05 DPI 欠陥の再来面。実 DPI≠96・マルチモニタでの人間サインオフ必達（R8.4） |

**総合**: 工数 **M–L**（3-7 日〜1-2 週の下寄り）。リスク **Medium**（座標表現の確定と kanade 署名波及が主。機構自体は既存注入口が揃い新機構不要）。

---

## 6. Research Needed（設計フェーズへ持ち越す調査）

- **ukadoc `file_structure`**（MCP `get_doc`/`search_docs`）: ゴーストフォルダの profile 慣行＝保存先ディレクトリ確定材料。ghost.dat の中身形式は正典に無い（baseware 自由）＝areka versioned 形式で可を design に明記。
- **ukadoc `list_shiori_event:OnFirstBoot`**: 「初回起動時に発生・204 なら OnBoot フォールスルー」「Reference0＝vanish された回数」の裏取り（brief で裏取り済みだが design で再確認）。
- **ukadoc `descript_shell:seriko.alignmenttodesktop`**: 復元位置と bottom 吸着の整合（復元後も吸着規則優先＝project_anchor 再射影）。
- **`std::fs::rename` の Windows atomic 性**: 同一ボリューム temp→rename が既存ファイルを原子的に置換するか（R6.3 の旧状態非破壊保証）。
- **モニタ識別子の永続表現**（D1 採用時）: `enumerate_monitors()` が返す `Monitor`（handle/bounds/work_area/dpi/is_primary）のうち再起動をまたいで安定に引き当てられる識別子（デバイス名 or work_area 矩形）の選定。

---

## 7. 次ステップ

要件ディスカッション（`/kiro-requirements-discussion areka-P0-position-persist`）で §4 の設計判断アイテム（特に論点1 座標表現・論点2 保存先/識別キー・論点3 直列化形式・論点4 所有境界）を詰め、確定後に `/kiro-design` で技術設計へ進む。本 spec は「新機構を作らず既存注入口へ入力を差す」層に徹する方針（brief・既存資産調査と整合）。
