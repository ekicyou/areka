# 実 DPI・scale≠1.0 実機受け入れ記録（Task 8.2 / 要件 4.1・4.2・4.6・4.7・4.8）

> ## ⛔ 本記録は **未完（UNFINISHED）** です — 2026-08-05 時点
>
> **2 水準（125% / 200%）の目視サインオフはいずれも未実施です。本記録に「合格」の判定は 1 件も存在しません。**
>
> 本ファイルは Task 8.2 の**様式（テンプレート）と実行手順書（runbook）**であり、
> 現時点で埋まっているのは **Task 6 の実装時に機械測定で確定した事実のみ**です。
> **機械測定は目視サインオフではありません**——R4.2/R4.3 が要求する
> 「狙点を**人間の目視のみ**で決定する」行為そのものが証跡であり、
> AI エージェント単独では原理的に完遂できません（合成入力による点の注入は R4.3 が明示的に禁止）。
> 加えて OS 表示スケールの 125% ↔ 200% 切替も開発者の操作を要します。
>
> **完了条件（R4.7/R4.8）を満たすには、開発者が下の runbook に従って実機実行し、
> 「目視サインオフ記録」の各判定欄を自ら埋める必要があります。**
> 判定欄が空欄である限り、本 spec は完了できません（R4.8）。

- 対象 spec: `areka-P0-collision-dpi-hittest`
- 記録様式の作成日: 2026-08-05
- 実施日: **未実施**
- 実施者: **未定**（目視操作は開発者が行う。AI は駆動・ログ採取・記録の補助のみ）

---

## 0. 証跡の分類（読む前に必ず確認すること）

本 spec の受け入れでは、**証跡として認められるもの**と**認められないもの**が要件で厳密に分かれている。
混ぜて記録してはならない。

### 0.1 反トートロジー（R4.3 / R4.4）

- 狙点の供給源は **人間の目視のみ**。probe は `GetCursorPos`（実カーソル・screen 物理 px）→
  `ScreenToClient`（当該窓 client 物理 px）の経路からのみ点を得る
  （`crates/areka/examples/collision-probe.rs:956-968`）。
- **`SetCursorPos` / `SendInput` 等の合成入力による点の注入は禁止**。probe は当該 API を一切呼ばない
  （同ファイル `:157-162` および `:924-925` の doc 宣言・実装にも当該 API の呼出なし）。
  `ClientToScreen` と `ScreenToClient` は client 原点の平行移動の厳密な逆写像であるため、
  collision 実値から合成した点を撃ち戻すと「注入した点をそのまま読み戻す」自己整合の罠になり、
  R4.4 が「証跡と認めない」と名指しした形になる。

### 0.2 描画証跡と判定証跡（R4.4）

| 種別 | 実体 | 位置づけ |
|---|---|---|
| **描画証跡** | ④ `read_back` anchor 画素検査（Head/Bust 矩形中心を ×k 写像した画素が α=0xFF） | 「collision 値の位置に絵が描かれている」ことしか語らない。**当たり判定の証跡ではない** |
| **判定証跡** | ⑤ 実カーソル経路（`GetCursorPos`→`ScreenToClient`→`resolve_hit_region`）のログ行と目視の一致 | **これのみが R4.2 の判定証跡** |

出所: `crates/areka/examples/collision-probe.rs:144-150`（④ の位置づけ宣言）・`:750-754`
（`assert_drawn_anchor` の doc「描画証跡であり判定証跡ではない」）。
本記録でも両者を**別欄**に分けており、統合してはならない。

### 0.3 R4.6 の二脚（分離必須）

R4.6（本番ゴースト・実 pasta.dll・絶対パス起動）は**二脚に分かれる**。
Task 6 の申し送り（`tasks.md:106`）が正典:

| 脚 | 実行体 | 担うもの | 絶対パス制約 |
|---|---|---|---|
| **脚 A: ヒットテスト目視** | `collision-probe` example | 実 DPI 下の当たり判定と目視の一致（R4.1/4.2/4.5） | **かからない**——probe は `pasta.dll` を一切ロードしないため `0x8007007E`（DLL LOAD 失敗）の経路自体が存在しない。fixture ルートは `CARGO_MANIFEST_DIR` アンカーで常に絶対解決される（`collision-probe.rs:73-97`・`:265-273`） |
| **脚 B: 本番ゴースト絶対パス起動** | `areka` 本体（`areka.exe`） | 実 emo2・実 `pasta.dll` を**絶対パス**で起動した本番経路の成立（R4.6）＋ R-2 の shell 実 k 確認 | **かかる**（[[areka-emo2-signoff-needs-absolute-paths]]） |

> **脚 A の実行を脚 B（絶対パス起動）の証跡に流用してはならない。** 両者は本記録の別項目である。

---

## 1. 実施条件（共通・実施時に埋めること）

| 項目 | 値 |
|---|---|
| 実施日時 | （未記入） |
| 実行機 | （未記入） |
| OS / ビルド | （未記入・例 Windows 11 Pro 10.0.26200） |
| DPI awareness | per-monitor v2（プロセスへは wintf `WinApp` 初期化が設定する） |
| 実行モニタ（型番・解像度・rcMonitor） | （未記入） |
| ビルドプロファイル | debug（既定・`cargo run` に `--release` を付けない） |
| git HEAD | （未記入） |

> **dpi=96（100%）のみの確認は不合格**（`collision-probe.rs:120-122`）。dpi=96 では単位混在バグが
> 自己整合して隠れる。2 水準とも ≠96 であること。

---

## 2. Runbook（開発者が実行する手順）

### 2.1 脚 A: collision-probe（ヒットテスト目視の脚）

**1 実行 = 1 DPI 水準**。DPI 追従駆動は probe に無い（wintf が窓生成時に実モニタ DPI を `DPI`
component へ初期化するため初回表示で実 k が確定する）ので、**OS の表示スケールを変更して 2 回実行**する
（`collision-probe.rs:7-13`・design「CollisionProbe 改修」#5）。

#### 起動コマンド（PowerShell）

```powershell
# --- 水準①: OS 表示スケール 125%（期待 k=5/4）---
$env:AREKA_COLLISION_PROBE_EXPECT_K = "5/4"
$env:AREKA_APP_SMOKE_EXIT_MS        = "180000"   # 3 分の有界 auto-exit（目視操作の時間を確保）
$env:RUST_LOG                       = "info"
cargo run -p areka --example collision-probe *> probe-125.log

# --- 水準②: OS 表示スケール 200%（期待 k=2）---
$env:AREKA_COLLISION_PROBE_EXPECT_K = "2"
$env:AREKA_APP_SMOKE_EXIT_MS        = "180000"
$env:RUST_LOG                       = "info"
cargo run -p areka --example collision-probe *> probe-200.log
```

裏取り:

| 項目 | 値 | 出所（file:line） |
|---|---|---|
| crate / example / profile | `cargo run -p areka --example collision-probe`（debug 既定） | `crates/areka/examples/collision-probe.rs:66-69` |
| 期待ゲート env 名 | `AREKA_COLLISION_PROBE_EXPECT_K` | `collision-probe.rs:254`（`const EXPECT_K_ENV`） |
| 期待ゲート値の書式 | 分数 `"5/4"` または整数 `"2"`（＝2/1）。125% → `5/4`、200% → `2` | `collision-probe.rs:888-913`（`expected_ratio`：`split_once('/')` で分数、無ければ分母 1） |
| 期待ゲート未指定時 | assert せず実測ログのみ（開発機でそのまま実行可） | `collision-probe.rs:865-873` |
| 期待ゲート不一致時 | **hard assert で loud fail**（Task 6 実測で **exit 101**） | `collision-probe.rs:874-880`・`tasks.md:105` |
| 有界 auto-exit env 名 | `AREKA_APP_SMOKE_EXIT_MS`（ミリ秒・未設定/空/非数値はゲート OFF） | `collision-probe.rs:247`（`const SMOKE_EXIT_ENV`）・`:1004-1012`（`smoke_exit_ms`）・`:1016-1045`（`install_smoke_exit`） |
| 推奨値 | `180000`（3 分）。目視操作の時間を確保するための「人間 opt-in の直接起動専用」の寛大値 | `crates/areka/tests/emo2_real_run.rs:104-114` |
| `RUST_LOG` | `info` で足りる（probe の常設ログは全て `tracing::info!`）。未設定でも既定 `info` へフォールバックする | `collision-probe.rs:319-324`（`EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))`）・`:616`・`:983` |

#### 終了方法

キャラ窓の**不透明域をダブルクリック**すると全ゴースト窓を閉じて終了する。放置しても
`AREKA_APP_SMOKE_EXIT_MS` で自動終了する（`collision-probe.rs:70-71`・`:363`）。

#### grep すべきログ（実際の書式）

```powershell
Select-String -Path probe-125.log -Pattern 'collision-probe: k='
Select-String -Path probe-125.log -Pattern 'collision-probe: client='
Select-String -Path probe-125.log -Pattern 'collision-probe: ③'
Select-String -Path probe-125.log -Pattern 'collision-probe: ④'
```

| 接頭辞 | 書式 | 出現回数 | 出所（file:line） |
|---|---|---|---|
| `collision-probe: k=` | `collision-probe: k=<num>/<den> native=<w>x<h> physical=<w>x<h>` | 1 実行 1 回（表示成立時） | `collision-probe.rs:616-628`（メッセージ本文）・`:23-25`（doc の書式宣言） |
| `collision-probe: client=` | `collision-probe: client=(x,y) surface=(sx,sy) region=<name> ⑤ マウス経路ペア列（client_point=(…)・Δ=(dx,dy) …）` | カーソル解決ごと | `collision-probe.rs:983-996` |
| `collision-probe: ③ 物理寸整合 assert 通過` | `GetClientRect == target_physical_size` の自動 assert 通過 | 1 実行 1 回 | `collision-probe.rs:724-733` |
| `collision-probe: ④ 描画一致 anchor 通過` | 物理座標へ写像した Head/Bust 中心が不透明（**描画証跡**） | 1 実行 1 回 | `collision-probe.rs:769-772` |
| `collision-probe: 期待 k ゲート通過` | 実適用 k が env 期待値と厳密一致 | 期待ゲート指定時のみ | `collision-probe.rs:881-885` |

ログ行の読み方（R4.3 遵守の要点）:

- `client=(x,y)` は **`ScreenToClient(GetCursorPos())` 由来の目視狙点**（＝判定へ渡した点そのもの）。
- `surface=(sx,sy)` はその点を presenter が ÷k 縮約した SHIORI 配信空間の値（probe 側で ÷k を再実装していない）。
- `region=` は `resolve_hit_region` の解決結果（`Head` / `Bust` / `None`）。
- 同じ行の `client_point=(…)`・`Δ=(dx,dy)` は本番マウス経路（`WM_MOUSEMOVE` lparam 直系）との
  ペア列。**静止時は Δ=(0,0) が要求**。
- **背景（`region=None`）行にはペア列が出ないことがあるが、それは正しい挙動**——透明域は
  クリック透過（`WS_EX_TRANSPARENT`）で `OnPointerMoved` が窓へ届かないため
  （`collision-probe.rs:174-175`・`:927-928`）。狙う「背景」は**不透明に描かれた判定枠外の胴体**にすること。

#### 表示対象と当たり判定矩形（両水準共通）

- 表示: emo2 `surface1000`（collision を持つ唯一のサーフェス）を有効 bind 実値
  `[1101, 1206, 1302, 1502, 1800]` 付きで shell target（scope=0 → `TargetId(0)`）へ実表示
  （`collision-probe.rs:244`・`:546`・`:574-583`）。
- 当たり判定（作者定義サーフェス px）: **Head = (93, 62)-(271, 130)** / **Bust = (133, 270)-(229, 326)**
  （`collision-probe.rs:237`・`:239`）。**これらは native 空間の値**であり、画面上では k 倍の位置に見える。

### 2.2 脚 B: 本番 emo2 実ゴースト（絶対パス起動）＋ R-2 消化

**絶対パス必須。** 相対パスで渡すと 32bit helper 側の `LoadLibrary(pasta.dll)` が
`0x8007007E`（ERROR_MOD_NOT_FOUND）で失敗し、SHIORI 接続が確立しない
（[[areka-emo2-signoff-needs-absolute-paths]]）。

#### 事前準備（i686 helper の配置）

`areka.exe` は `current_exe()` 隣の `shiori-host32-helper.exe` を helper として解決する
（`crates/areka/src/main.rs:155-161`）。`cargo build --workspace` は **x64 版**を
`target/debug/` へ落とすため、**i686 版を上書きコピー**しておくこと
（`crates/areka/tests/emo2_real_run.rs:35-40`・`.kiro/specs/completed/areka-P0-idle-talk/design.md:679-682`）。

```powershell
cargo build -p areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
Copy-Item target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe target\debug\ -Force
```

#### 起動コマンド（PowerShell・絶対パス）

```powershell
$ghost   = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2").Path
$balloon = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku").Path
$env:AREKA_APP_SMOKE_EXIT_MS = "180000"
$env:RUST_LOG = "info,areka_emo_present=debug"
& "target\debug\areka.exe" $ghost $balloon *> emo2-real.log
```

裏取り:

| 項目 | 値 | 出所（file:line） |
|---|---|---|
| 位置引数 | `argv[1]` = ghost_root、`argv[2]` = balloon_root（欠落時は `CARGO_MANIFEST_DIR` 相対の既定へフォールバック） | `crates/areka/src/main.rs:122-143`（`resolve_config_inputs`）・`:110-120`（既定） |
| emo2 ghost_root | `crates/pilot/examples/shiori-host-32/fixtures/emo2` | `crates/areka/tests/emo2_real_run.rs:18`・`:111-112` |
| emo2 balloon_root | 同 fixture 下 `emo2-kakukaku` | `crates/areka/tests/emo2_real_run.rs:19`・`:112-113` |
| 絶対パス必須の理由 | 相対だと helper の `LoadLibrary(pasta.dll)` が `0x8007007E` | 記憶 [[areka-emo2-signoff-needs-absolute-paths]] |
| 有界 auto-exit env 名 | `AREKA_APP_SMOKE_EXIT_MS`（probe と同名） | `crates/areka/src/main.rs:664`（doc）・`:768`・`:808`・`:868` |
| 推奨値 | `180000`（3 分・人間の直接起動専用） | `crates/areka/tests/emo2_real_run.rs:105-114` |
| `RUST_LOG` | `info,areka_emo_present=debug`。`apply(ShowSurface)` は `info!`、`hit_region_client` は `debug!` | `crates/areka-emo-present/src/presenter.rs:584-588`（info であることが契約）・`:945-948`（`RUST_LOG=areka_emo_present=debug` で grep する旨） |

#### R-2（shell target の実 k を debug ログ grep）の grep 手順

```powershell
# ① 表示成立点の k（info・1 表示につき 1 行）
Select-String -Path emo2-real.log -Pattern 'apply\(ShowSurface\): 表示・マスクを更新'

# ② 判定ごとの k と縮約前後座標（debug）
Select-String -Path emo2-real.log -Pattern '\[hit_region_client\] client 物理 px を ÷k して当たり判定を解決'
```

| 観測面 | メッセージ | レベル | 主要フィールド | 出所（file:line） |
|---|---|---|---|---|
| 表示成立点 | `apply(ShowSurface): 表示・マスクを更新` | `info!` | `target_id`・`k_ratio`・`k`・`author_dpi`・`window_dpi`・`native_w/h`・`scaled_w/h` | `crates/areka-emo-present/src/presenter.rs:595-612` |
| 判定入口 | `[hit_region_client] client 物理 px を ÷k して当たり判定を解決` | `debug!` | `target`・`k_ratio`・`client_x/y`・`surface_x/y`・`region` | `crates/areka-emo-present/src/presenter.rs:992-1002` |

- **shell target の識別**: scope0 の shell target は `TargetId(0)`（採番規約 `shell_target(scope) = TargetId(2*scope)`・
  balloon は奇数）。`target_id=TargetId(0)` の行を読むこと
  （`crates/areka/src/emo2_boot/target_map.rs:19-29`）。
- **`k_ratio` の表記**: `ScaleRatio` は `num`/`den` 非公開のため `Debug` 出力＝
  `ScaleRatio { num: 2, den: 1 }` 形（`crates/areka-emo-compose/src/scale.rs:46-47` の `derive(Debug)`・
  期待形の実測は `crates/areka-emo-present/src/presenter.rs:3762-3766`）。
- **`k`（f32）** も同じ行に出る（`k=2.0` 等）。

---

## 3. 目視サインオフ記録

> **以下は全て未実施。判定欄に「合格」と書けるのは、開発者が実機で目視を行った後だけである。**

### 3.1 水準①: OS 表示スケール 125%（期待 k=5/4）

#### 実施条件

| 項目 | 値 |
|---|---|
| 実施日時 | （未記入） |
| OS 表示スケール | 125%（設定値・実施時に確認） |
| モニタ実効 DPI（`GetDpiForMonitor` 等） | （未記入・期待 120） |
| `AREKA_COLLISION_PROBE_EXPECT_K` | `5/4` |
| `AREKA_APP_SMOKE_EXIT_MS` | （未記入・推奨 `180000`） |
| `RUST_LOG` | （未記入・推奨 `info`） |
| ログファイル | （未記入） |

#### 機械測定（自動 assert・ログ実測値）

| # | 項目 | 結果 | 実測値 |
|---|---|---|---|
| ① | 実 DPI ≠ 96 で実行 | **未実施** | （未記入） |
| ② | 実測 k（`collision-probe: k=<num>/<den>`） | **未実施** | （未記入・期待 `5/4`） |
| ③ | native 寸（`native=<w>x<h>`） | **未実施** | （未記入・期待 `382x547`） |
| ④ | physical 寸（`physical=<w>x<h>`） | **未実施** | （未記入・**期待値は書かない＝要確認**。k=5/4 は割り切れないため丸め権威 `ScaleRatio::scaled_extent` の結果を手計算で先取りしない。ログ実値を転記すること。判定は水準②との**相違**（§4）で行う） |
| ⑤ | 期待 k ゲート通過（`collision-probe: 期待 k ゲート通過`） | **未実施** | （未記入） |
| ⑥ | ③ 物理寸整合 assert 通過（`GetClientRect == target_physical_size`） | **未実施** | （未記入） |
| ⑦ | ④ 描画一致 anchor 通過（**描画証跡・判定証跡ではない**） | **未実施** | （未記入） |

#### 目視サインオフ（判定証跡・R4.2）

狙点は**目視のみ**で決定すること（R4.3）。各部位につき静止状態のログ行を 1 行以上転記する。

| 狙った部位（目視） | `client=(x,y)` | `surface=(sx,sy)` | `region=` | 目視の狙いと一致したか | Δ=(dx,dy) |
|---|---|---|---|---|---|
| 頭（Head・不透明） | （未記入） | （未記入） | （未記入） | **未実施** | （未記入・静止行は (0,0) 要求） |
| 胸（Bust・不透明） | （未記入） | （未記入） | （未記入） | **未実施** | （未記入） |
| 背景（判定枠外・**不透明に描かれた胴体**を狙う） | （未記入） | （未記入） | （未記入・期待 `None`） | **未実施** | ペア列欠測は正しい挙動（透明域を狙った場合） |

#### 水準① 判定

**判定: 未実施（UNTESTED）**

（実施後に「合格」／「不合格」と実測根拠を記入すること。）

---

### 3.2 水準②: OS 表示スケール 200%（期待 k=2）

#### 実施条件

| 項目 | 値 |
|---|---|
| 実施日時 | （未記入） |
| OS 表示スケール | 200%（設定値・実施時に確認） |
| モニタ実効 DPI | （未記入・期待 192） |
| `AREKA_COLLISION_PROBE_EXPECT_K` | `2` |
| `AREKA_APP_SMOKE_EXIT_MS` | （未記入・推奨 `180000`） |
| `RUST_LOG` | （未記入・推奨 `info`） |
| ログファイル | （未記入） |

#### 機械測定（自動 assert・ログ実測値）

> **以下 ②〜④ は Task 6（probe 改修）の実装時に開発機で実測済みの値である**
> （出所: `tasks.md:105` の Task 6→8.2 申し送り「開発機の実表示スケールは 200%（実適用 k=2/1）と
> 実測済み。probe 実行の実測値は `k=2/1 native=382x547 physical=764x1094`」）。
> **これは機械測定分であり、目視サインオフではない。** 8.2 の実施時に改めて採り直すのが正道だが、
> 少なくとも「200% 水準で probe が k=2/1 を実適用して表示する」ことは確定している。

| # | 項目 | 結果 | 実測値 |
|---|---|---|---|
| ① | 実 DPI ≠ 96 で実行 | **機械測定済** | 開発機 200%（≠96） |
| ② | 実測 k（`collision-probe: k=<num>/<den>`） | **機械測定済** | `k=2/1`（Task 6 実測・`tasks.md:105`） |
| ③ | native 寸 | **機械測定済** | `382x547`（同上） |
| ④ | physical 寸 | **機械測定済** | `764x1094`（同上・382×2 / 547×2） |
| ⑤ | 期待 k ゲート通過 | **未実施**（8.2 の実行で `AREKA_COLLISION_PROBE_EXPECT_K=2` を与えて確認する） | — |
| ⑥ | ③ 物理寸整合 assert 通過 | **未実施** | — |
| ⑦ | ④ 描画一致 anchor 通過（**描画証跡**） | **未実施** | — |

> 参考（期待ゲートの loud fail 実測）: 200% 環境で `AREKA_COLLISION_PROBE_EXPECT_K=5/4` を与えると
> hard assert で **exit 101** となることを Task 6 で実測済み（`tasks.md:105`）。
> ＝期待ゲートが「水準を偽って通す」余地を持たないことの証跡。

#### 目視サインオフ（判定証跡・R4.2）

| 狙った部位（目視） | `client=(x,y)` | `surface=(sx,sy)` | `region=` | 目視の狙いと一致したか | Δ=(dx,dy) |
|---|---|---|---|---|---|
| 頭（Head・不透明） | （未記入） | （未記入） | （未記入） | **未実施** | （未記入） |
| 胸（Bust・不透明） | （未記入） | （未記入） | （未記入） | **未実施** | （未記入） |
| 背景（判定枠外・不透明胴体） | （未記入） | （未記入） | （未記入・期待 `None`） | **未実施** | — |

#### 水準② 判定

**判定: 未実施（UNTESTED）**

（機械測定分が確定していても、目視の突合が未了である以上「合格」とは書かない。）

---

## 4. 2 水準の相互照合（R4.1 の証跡）

R4.1 は「各水準でマスコットが**互いに異なる拡大寸**で表示されている証跡」を要求する。
2 実行の `physical=` を突き合わせ、**異なること**を記録上で確認する。

| 水準 | 実測 k | native 寸 | physical 寸 |
|---|---|---|---|
| ① 125% | （未記入） | （未記入） | （未記入） |
| ② 200% | `2/1`（Task 6 機械測定） | `382x547` | `764x1094` |

**照合結果: 未実施**（水準① の実測が無いため比較不能）

判定基準: 両水準の `physical=` が**互いに異なる**こと。同一なら DPI 追従が効いておらず、
`collision-geometry` Task 4.2 却下時と同じ「ヒットテスト経路が同一で検証自体が成立しない」状態である。

---

## 5. 脚 B: 本番 emo2 実ゴースト絶対パス起動 ＋ R-2 消化（R4.6）

> **脚 A（probe）の実行をここへ流用してはならない**（§0.3）。

| 項目 | 値 |
|---|---|
| 実施日時 | （未記入） |
| ghost_root（**絶対パス**） | （未記入） |
| balloon_root（**絶対パス**） | （未記入） |
| i686 `shiori-host32-helper.exe` を `target/debug/` へ配置したか | **未実施** |
| `AREKA_APP_SMOKE_EXIT_MS` | （未記入・推奨 `180000`） |
| `RUST_LOG` | （未記入・推奨 `info,areka_emo_present=debug`） |
| ログファイル | （未記入） |

### R-2: shell target（`TargetId(0)`）の実 k

| # | 観測 | 結果 | 実測値 |
|---|---|---|---|
| ① | `apply(ShowSurface): 表示・マスクを更新` の `target_id=TargetId(0)` 行が出た | **未実施** | （未記入） |
| ② | 同行の `k_ratio` | **未実施** | （未記入・形は `ScaleRatio { num: N, den: D }`） |
| ③ | 同行の `k`（f32）・`author_dpi`・`window_dpi`・`native_w/h`・`scaled_w/h` | **未実施** | （未記入） |
| ④ | `[hit_region_client] client 物理 px を ÷k して当たり判定を解決`（debug）の `k_ratio` が ② と一致 | **未実施** | （未記入） |
| ⑤ | `pasta.dll` の LOAD 失敗（`0x8007007E`）が出ていない＝絶対パス起動が成立 | **未実施** | （未記入） |

### 脚 B 判定

**判定: 未実施（UNTESTED）**

---

## 6. 不一致欄（R4.8）

> **プロセス規定**: いずれかの水準で目視の狙いと解決結果が一致しない場合、
> **その不一致を本欄に記録したうえで是正し、再実施して一致を確認するまで本 spec を完了としない**（R4.8）。
> 不一致を「環境のせい」「誤差」として黙って握り潰すことは禁止する。
> 是正が本 spec の boundary 外に及ぶと判明した場合も、本欄にその判断と担当 spec を明記する
> （担当 spec の実在検証を伴うこと＝記憶 [[deferral-requires-verified-owner]]）。

| # | 発生日時 | 水準 | 狙った部位 | 期待 region | 実際の region | `client=` / `surface=` | 原因分析 | 是正内容 | 再実施日時 | 再実施結果 |
|---|---|---|---|---|---|---|---|---|---|---|
| — | — | — | — | — | — | — | （現時点で記録すべき不一致なし＝**未実施のため**） | — | — | — |

---

## 7. 総合判定

**総合判定: 未実施（UNTESTED）— 本 spec は完了できない**

完了（R4.7/R4.8 充足）に必要な残作業:

1. [ ] OS 表示スケール 125% で probe を実行し、期待 k ゲート（`5/4`）を通過させる
2. [ ] 125% で頭・胸・背景を**目視のみ**で狙い、`client=/surface=/region=` と目視の一致を §3.1 へ記録する
3. [ ] OS 表示スケール 200% で probe を実行し、期待 k ゲート（`2`）を通過させる
4. [ ] 200% で頭・胸・背景を**目視のみ**で狙い、§3.2 へ記録する
5. [ ] §4 で 2 水準の `physical=` が互いに異なることを照合する
6. [ ] 脚 B（本番 emo2 実ゴーストの**絶対パス**起動）を実施し、§5 の R-2 grep 結果を記録する
7. [ ] 不一致があれば §6 に記録し、是正 → 再実施まで完了としない
8. [ ] 上記が全て揃った時点で本節の総合判定を「合格」へ改める（開発者の承認をもって確定）
