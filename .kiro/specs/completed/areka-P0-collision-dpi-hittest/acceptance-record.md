# 実 DPI・scale≠1.0 実機受け入れ記録（Task 8.2 / 要件 4.1・4.2・4.6・4.7・4.8）

> ## ✅ 本記録は **合格（PASSED）** です — 2026-08-05 実施
>
> **2 水準（125% / 200%）の目視サインオフをいずれも実施し、合格を確認しました。**
> あわせて脚 B（本番 emo2 実ゴーストの絶対パス起動）も成立し、R-2 を消化しています。
>
> `areka-P0-collision-geometry` Task 4.2 が 2026-07-18 に却下された理由は
> 「モニタ DPI を 2 水準変えてもマスコットが同一物理寸（k=1.0 固定）ゆえヒットテスト経路が同一で、
> 検証自体が成立しない」であった。本記録は **2 水準で物理寸が実際に異なる**
> （125%: 478×684 ／ 200%: 764×1094）ことを実測しており、当該観測条件が**初めて成立**した。
>
> 判定方法（R4.2）: 目視の狙いと解決結果の突合は、**開発者が狙いの順序を宣言し、
> その順序と `region=` ログの時系列区間の対応を照合する**形で行った（§0.4 に規定）。
> 加えて脚 B の本番ゴーストでは**開発者が撫で反応・さわり反応を直接目視**しており、
> 「見えているとおりの部位が当たる」ことを人間が直接判定している。

- 対象 spec: `areka-P0-collision-dpi-hittest`
- 記録様式の作成日: 2026-08-05
- 実施日: **2026-08-05**（ログ時刻は UTC 表記。JST = UTC+9）
- 実施者: 開発者（目視・OS 表示スケール切替）＋ AI エージェント（駆動・ログ採取・突合・記録）

---

## 0.4 判定方法の規定（R4.2 の突合手順・本実施で採用した形）

probe は**その場で判定結果を画面に描かない**。`region=` は標準出力へ流れるため、
本来は「開発者が端末の出力を**見ながら**狙う」ことで目視と結果を同時に確認できる。
本実施では出力をログファイルへリダイレクトしたため、その場での同時確認は成立しなかった。
そこで **R4.2 の突合を次の形で成立させた**（開発者の合意のうえ採用）:

1. 開発者が狙う順序を**あらかじめ宣言**する（本実施では「頭 → 胸 → 背景」）。
2. 各部位で数秒静止し、ログに連続した区間を作る。
3. 突合は、宣言した順序と `region=` の**時系列区間の並び**が一致するかで判定する。

この形が成立する根拠: 狙点の供給源は依然として**人間の目視のみ**であり（`GetCursorPos`→
`ScreenToClient` 経路・合成入力なし＝R4.3 遵守）、期待値（狙いの順序）は**結果を見る前に**
人間が宣言している。したがって「結果に合わせて狙いを後付けする」自己整合の罠は生じない。

**より忠実な形**（次回以降の推奨）: probe を**可視の端末**で走らせ、`region=` の変化を
見ながら狙う。本実施の脚 B（本番 emo2）では、ゴーストの**撫で反応・さわり反応**という
可視のフィードバックがあるため、開発者はその場で直接判定できている（§5）。

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

## 1. 実施条件（共通）

| 項目 | 値 |
|---|---|
| 実施日時 | 2026-08-05（UTC 03:36〜03:50 ／ JST 12:36〜12:50） |
| 実行機 | 開発者の常用機（`C:\home\maz\git\areka` の worktree `areka-p0-requirements-review-1314c6`） |
| OS / ビルド | Microsoft Windows 11 Pro 10.0.26200.0 |
| DPI awareness | per-monitor v2（プロセスへは wintf `WinApp` 初期化が設定する） |
| 実行モニタ | primary `\\.\DISPLAY1`（実効 DPI **192**＝200% 時／**120**＝125% 時）。副 `\\.\DISPLAY2` あり（**混在 DPI 環境ではない**——両実行とも primary 上で表示）。実効 DPI は脚 B の `window_dpi=Some((192, 192))`・`primary_dpi=192` で独立確認 |
| ビルドプロファイル | debug（`cargo run` に `--release` を付けない） |
| git HEAD（実施時） | `c72adab`（本記録の様式コミット時点。判定コードは Task 1〜7 で着地済み） |

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

> ⚠️ **2026-08-05 是正**: 以下の「ダブルクリックで終了」は**誤り**であった（§6.2）。
> **probe に手動終了の手段は存在しない**——`OnPointerPressed` が一切装着されていないため、
> ダブルクリックしても終了しない。**終了は `AREKA_APP_SMOKE_EXIT_MS` の有界 auto-exit のみ**。
> 目視所要時間を見込んだ値（推奨 `180000`＝3 分）を必ず与えること。
>
> <del>キャラ窓の**不透明域をダブルクリック**すると全ゴースト窓を閉じて終了する。</del>放置しても
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

> 突合の形は §0.4 の規定に従う（狙いの順序を先に宣言 → 時系列区間との対応で判定）。

### 3.1 水準①: OS 表示スケール 125%（期待 k=5/4）

#### 実施条件

| 項目 | 値 |
|---|---|
| 実施日時 | 2026-08-05T03:44:32Z 〜 03:47:31Z（JST 12:44〜12:47）／⑤ 観測は 03:44:38Z〜03:44:58Z |
| OS 表示スケール | 125% |
| モニタ実効 DPI | 120（`k=5/4 × author_dpi 96` から逆算・実測 k が 5/4 で確定） |
| `AREKA_COLLISION_PROBE_EXPECT_K` | `5/4` |
| `AREKA_APP_SMOKE_EXIT_MS` | `180000` |
| `RUST_LOG` | `info` |
| ログファイル | `probe-125.log`（2419 行の ⑤ ペア列を含む） |
| 終了 | 有界 auto-exit（03:47:31Z `Last window closed` → `ゴースト窓を despawn`）・**exit 0** |

#### 機械測定（自動 assert・ログ実測値）

| # | 項目 | 結果 | 実測値 |
|---|---|---|---|
| ① | 実 DPI ≠ 96 で実行 | **合格** | 125%（120dpi・≠96） |
| ② | 実測 k（`collision-probe: k=<num>/<den>`） | **合格** | `k=5/4`（`scale_f32=1.25`） |
| ③ | native 寸（`native=<w>x<h>`） | **合格** | `382x547` |
| ④ | physical 寸（`physical=<w>x<h>`） | **合格** | **`478x684`**（丸め権威 `scaled_extent` の実値。水準②と異なる＝§4） |
| ⑤ | 期待 k ゲート通過 | **合格** | `期待 k ゲート通過（実適用 k が期待値と厳密一致） k=5/4` |
| ⑥ | ③ 物理寸整合 assert 通過（`GetClientRect == target_physical_size`） | **合格** | `client_w=478 client_h=684` |
| ⑦ | ④ 描画一致 anchor 通過（**描画証跡・判定証跡ではない**） | **合格** | 通過（Head/Bust 中心を ×k 写像した画素が不透明） |

#### 目視サインオフ（判定証跡・R4.2）

宣言した狙いの順序（実施前に宣言・§0.4）: **頭 → 胸 → 背景**

`region=` の時系列区間（実測・時刻順）:

| # | 期間（UTC） | `region=` | 行数 |
|---|---|---|---|
| 1 | 03:44:38 → 03:44:44 | **Head** | 775 |
| 2 | 03:44:44 → 03:44:45 | None（移動中） | 79 |
| 3 | 03:44:45 → 03:44:50 | **Bust** | 660 |
| 4 | 03:44:50 → 03:44:58 | **None** | 905 |

**宣言した順序と時系列区間の並びが完全に一致**（頭→胸→背景。途切れのない塊で、
部位間の遷移は移動中の短い None 区間のみ）。

代表行（静止時・全て Δ=(0,0)）:

| 狙った部位（目視） | `client=(x,y)` | `surface=(sx,sy)` | `region=` | 一致 | 備考 |
|---|---|---|---|---|---|
| 頭 | (227,**162**) | (182,**130**) | **Head** | ✅ | Head 下端 y=130＝**閉区間の端が当たる** |
| 頭 | (227,**163**) | (182,**130**) | **Head** | ✅ | **client 2 行が同一 surface 行へ潰れる＝割り切れない縮約の実証** |
| 頭 | (171,132) | (137,106) | **Head** | ✅ | 領域内 |
| 胸 | (244,**338**) | (195,**270**) | **Bust** | ✅ | Bust 上端 y=270＝閉区間の端 |
| 胸 | (231,361) | (185,289) | **Bust** | ✅ | 領域内 |
| 胸 | (230,**407**) | (184,**326**) | **Bust** | ✅ | Bust 下端 y=326＝閉区間の端 |
| 背景 | (228,**165**) | (182,**132**) | **None** | ✅ | Head 下端の**外側**（境界外 1px 側） |
| 背景 | (226,554) | (181,443) | **None** | ✅ | 判定枠外 |

Δ=(dx,dy): 2419 行中 **2412 行が (0,0)**。残る 7 行は急移動中のサンプル間ずれで、
静止時の判定行はすべて厳密一致。

#### DD-1 丸め規約の実機検算（k=5/4 → `s(v) = ⌊(8v+4)/10⌋`）

| client | 計算 | surface | 実測 |
|---|---|---|---|
| y=162 | (8·162+4)/10 = 1300/10 = **130** | 130 | ✅ |
| y=163 | (8·163+4)/10 = 1308/10 = 130.8 → **130** | 130 | ✅ |
| y=165 | (8·165+4)/10 = 1324/10 = 132.4 → **132** | 132 | ✅ |
| y=338 | (8·338+4)/10 = 2708/10 = 270.8 → **270** | 270 | ✅ |
| y=407 | (8·407+4)/10 = 3260/10 = **326** | 326 | ✅ |
| x=227 | (8·227+4)/10 = 1820/10 = **182** | 182 | ✅ |
| x=244 | (8·244+4)/10 = 1956/10 = 195.6 → **195** | 195 | ✅ |
| x=230 | (8·230+4)/10 = 1844/10 = 184.4 → **184** | 184 | ✅ |
| x=171 | (8·171+4)/10 = 1372/10 = 137.2 → **137** | 137 | ✅ |

**全 9 件が DD-1 の画素中心逆写像と厳密一致。** 割り切れない縮約（k=5/4）が
実機で規約どおりに効いていることの実測証跡である。

#### 水準① 判定

**判定: 合格（PASSED）**

根拠: 期待 k ゲート `5/4` 通過・物理寸 478×684（水準②と相違）・宣言順序と時系列区間の完全一致・
閉区間の端（Head 下端 130／Bust 上下端 270・326）が当たり境界外 1px（132）が外れる・
DD-1 丸め規約の実機検算 9/9 一致。

---

### 3.2 水準②: OS 表示スケール 200%（期待 k=2）

#### 実施条件

| 項目 | 値 |
|---|---|
| 実施日時 | 2026-08-05T03:36:41Z 〜 03:39:41Z（JST 12:36〜12:39）／⑤ 観測は 03:36:42Z〜03:37:11Z |
| OS 表示スケール | 200% |
| モニタ実効 DPI | 192（脚 B の `window_dpi=Some((192, 192))` で独立確認） |
| `AREKA_COLLISION_PROBE_EXPECT_K` | `2` |
| `AREKA_APP_SMOKE_EXIT_MS` | `180000` |
| `RUST_LOG` | `info` |
| ログファイル | `probe-200.log`（2630 行の ⑤ ペア列を含む） |
| 終了 | 有界 auto-exit（03:39:41Z `Last window closed` → `ゴースト窓を despawn count=4`）・**exit 0** |

#### 機械測定（自動 assert・ログ実測値）

| # | 項目 | 結果 | 実測値 |
|---|---|---|---|
| ① | 実 DPI ≠ 96 で実行 | **合格** | 200%（192dpi・≠96） |
| ② | 実測 k（`collision-probe: k=<num>/<den>`） | **合格** | `k=2/1`（`scale_f32=2.0`） |
| ③ | native 寸 | **合格** | `382x547` |
| ④ | physical 寸 | **合格** | **`764x1094`**（水準①と異なる＝§4） |
| ⑤ | 期待 k ゲート通過 | **合格** | `期待 k ゲート通過（実適用 k が期待値と厳密一致） k=2/1` |
| ⑥ | ③ 物理寸整合 assert 通過 | **合格** | `client_w=764 client_h=1094` |
| ⑦ | ④ 描画一致 anchor 通過（**描画証跡**） | **合格** | 通過 |

> **期待ゲートの loud fail 実測（空虚でないことの証跡）**: 200% 環境で
> `AREKA_COLLISION_PROBE_EXPECT_K=5/4` を与えると hard assert で **exit 101**
> （`実適用 k=2/1 が期待 k=5/4（env AREKA_COLLISION_PROBE_EXPECT_K）と不一致`）となることを
> Task 6 で実測済み。＝期待ゲートが「水準を偽って通す」余地を持たない。

#### 目視サインオフ（判定証跡・R4.2）

宣言した狙いの順序（実施前に宣言・§0.4）: **頭 → 胸 → 背景**

`region=` の時系列区間（実測・時刻順）:

| # | 期間（UTC） | `region=` | 行数 |
|---|---|---|---|
| 1 | 03:36:42 → 03:36:49 | **Head** | 518 |
| 2 | 03:36:49 → 03:36:51 | None（移動中） | 153 |
| 3 | 03:36:51 | Head 20 → None 90（通過） | 110 |
| 4 | 03:36:51 → 03:36:56 | **Bust** | 576 |
| 5 | 03:36:56 → 03:37:11 | **None** | 1272 |

**宣言した順序と時系列区間の並びが一致**（頭→胸→背景）。区間 3 は頭から胸へ移動する際に
Head 領域の縁を再度かすめた通過であり、宣言順序と矛盾しない。

代表行（静止時・Δ=(0,0)）:

| 狙った部位（目視） | `client=(x,y)` | `surface=(sx,sy)` | `region=` | 一致 | 備考 |
|---|---|---|---|---|---|
| 頭 | (459,233) | (229,116) | **Head** | ✅ | 領域内 |
| 頭 | (358,203) | (179,101) | **Head** | ✅ | 領域内 |
| 頭 | (348,**260**) | (174,**130**) | **Head** | ✅ | Head 下端 y=130＝**閉区間の端が当たる** |
| 胸 | (356,**544**) | (178,**272**) | **Bust** | ✅ | Bust 上端近傍 |
| 胸 | (372,595) | (186,297) | **Bust** | ✅ | 領域内 |
| 胸 | (364,**653**) | (182,**326**) | **Bust** | ✅ | Bust 下端 y=326＝閉区間の端 |
| 背景 | (352,**262**) | (176,**131**) | **None** | ✅ | **Head 下端の 1px 外側が外れる**（130 は当たり・131 は外れ） |
| 背景 | (514,878) | (257,439) | **None** | ✅ | 判定枠外 |
| 背景 | (229,966) | (114,483) | **None** | ✅ | 判定枠外 |

Δ=(dx,dy): 2630 行中 **2611 行が (0,0)**。残る 19 行は急移動中のサンプル間ずれ。

#### DD-1 丸め規約の実機検算（k=2/1 → `s(v) = ⌊(4v+2)/4⌋ = ⌊v/2⌋`）

| client | 計算 | surface | 実測 |
|---|---|---|---|
| y=260 | (2·260+1)·1 ÷ 4 = 521/4 = 130.25 → **130** | 130 | ✅ **Head 下端（当たり）** |
| y=262 | (2·262+1)·1 ÷ 4 = 525/4 = 131.25 → **131** | 131 | ✅ **境界外 1px（外れ）** |
| y=233 | 467/4 = 116.75 → **116** | 116 | ✅ |
| y=653 | 1307/4 = 326.75 → **326** | 326 | ✅ Bust 下端 |
| x=459 | 919/4 = 229.75 → **229** | 229 | ✅ |

**閉区間の内外が 1px 単位で k=2 の実機でも保存されている**（R2.3 の実機確認）。

#### 水準② 判定

**判定: 合格（PASSED）**

根拠: 期待 k ゲート `2` 通過・物理寸 764×1094（水準①と相違）・宣言順序と時系列区間の一致・
Head 下端 130 が当たり／131 が外れる境界 1px の保存・DD-1 検算 5/5 一致。

---

## 4. 2 水準の相互照合（R4.1 の証跡）

R4.1 は「各水準でマスコットが**互いに異なる拡大寸**で表示されている証跡」を要求する。
2 実行の `physical=` を突き合わせ、**異なること**を記録上で確認する。

| 水準 | 実測 k | native 寸 | **physical 寸** |
|---|---|---|---|
| ① 125% | `5/4` | `382x547` | **`478x684`** |
| ② 200% | `2/1` | `382x547` | **`764x1094`** |

**照合結果: 合格（PASSED）**

- native 寸は両水準で同一（`382x547`）＝作者定義値は不変（R6.2 の実機側の裏付け）
- **physical 寸は互いに異なる**（478×684 ≠ 764×1094）＝マスコットが実際に**異なる拡大寸**で
  表示されていた（R4.1 の証跡）
- 縦横比も保存（478/382 = 684/547 相当の丸め・両軸とも同一 k）

判定基準: 両水準の `physical=` が**互いに異なる**こと。同一なら DPI 追従が効いておらず、
`collision-geometry` Task 4.2 却下時と同じ「ヒットテスト経路が同一で検証自体が成立しない」状態である。
**本記録は当該条件を初めて突破した。**

---

## 5. 脚 B: 本番 emo2 実ゴースト絶対パス起動 ＋ R-2 消化（R4.6）

> **脚 A（probe）の実行をここへ流用してはならない**（§0.3）。

| 項目 | 値 |
|---|---|
| 実施日時 | 実行 B-1: 2026-08-05T03:46:56Z〜03:47:56Z ／ 実行 B-2: 03:48:5x Z〜03:49:5x Z（JST 12:46〜12:50） |
| ghost_root（**絶対パス**） | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-requirements-review-1314c6\crates\pilot\examples\shiori-host-32\fixtures\emo2` |
| balloon_root（**絶対パス**） | 同上 `\emo2-kakukaku` |
| 実行体 | `…\target\debug\areka.exe`（**絶対パス**で起動） |
| i686 `shiori-host32-helper.exe` を `target/debug/` へ配置したか | **実施済**（`cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` → `target\debug\` へ上書きコピー） |
| `AREKA_APP_SMOKE_EXIT_MS` | `60000` |
| `RUST_LOG` | `info,areka_emo_present=debug` |
| ログファイル | `emo2-legB.log`（B-1）／`emo2-legB2.log`（B-2） |
| OS 表示スケール | 200%（`window_dpi=Some((192, 192))`・`primary_dpi=192`） |
| 終了 | 両実行とも有界 auto-exit → **exit 0**（`shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)）`） |

### R-2: shell target（`TargetId(0)`）の実 k

| # | 観測 | 結果 | 実測値 |
|---|---|---|---|
| ① | `apply(ShowSurface): 表示・マスクを更新` の `target_id=TargetId(0)` 行が出た | **合格** | `target_id=TargetId(0) surface_id=1000 cache_hit=false` |
| ② | 同行の `k_ratio` | **合格** | **`ScaleRatio { num: 2, den: 1 }`** |
| ③ | 同行の `k`・`author_dpi`・`window_dpi`・`native_w/h`・`scaled_w/h` | **合格** | `k=2.0 author_dpi=96 window_dpi=Some((192, 192)) native_w=382 native_h=547 scaled_w=764 scaled_h=1094` |
| ④ | `[hit_region_client] client 物理 px を ÷k して当たり判定を解決`（debug）の `k_ratio` が ② と一致 | **合格** | B-2 で **2271 行**採取。全行 `k_ratio=ScaleRatio { num: 2, den: 1 }`（②と一致） |
| ⑤ | `pasta.dll` の LOAD 失敗（`0x8007007E`）が出ていない＝絶対パス起動が成立 | **合格** | `0x8007007E` の出現 **0 件**。`emo2-boot: 実 sink 結線が成立しました（wire 成立）`・`kanade: 起動種別にスクリプト——OnBoot をスキップし basewareversion へ event` |

起動時の k₀ 導出（独立の裏付け）:
`placement: 起動時 k₀ を導出（primary モニタ DPI ÷ 作者基準 DPI・D7） primary_dpi=192 shell_author_dpi=96 balloon_author_dpi=96 k_shell=2.0 k_balloon=2.0`

### 本番判定経路の実証（B-2・本 spec の中核の production 証跡）

**probe ではなく本番バイナリ**が新しい ÷k 判定入口を通っていることの証跡。
開発者が本番ゴーストの上でマウスを動かし、頭・胸を撫でた際の実測:

| target | `region=` | 行数 |
|---|---|---|
| **`TargetId(0)`（sakura shell）** | **`Some("Head")`** | 638 |
| `TargetId(0)` | **`Some("Bust")`** | 445 |
| `TargetId(0)` | `None` | 224 |
| `TargetId(2)`（kero shell） | `None` | 964 |

代表行（生ログ）:

```text
[hit_region_client] client 物理 px を ÷k して当たり判定を解決
  target=TargetId(0) k_ratio=ScaleRatio { num: 2, den: 1 }
  client_x=266 client_y=220 surface_x=133 surface_y=110 region=Some("Head")
```

| 部位 | `client` | `surface` | 矩形との関係 | DD-1 検算 |
|---|---|---|---|---|
| 頭 | (266,220) | (133,110) | Head=(93,62)-(271,130) の内側 | 533/4=133 ✅ / 441/4=110 ✅ |
| 頭 | (296,224) | (148,112) | 内側 | ✅ |
| 頭 | (319,249) | (159,124) | 内側 | ✅ |
| 胸 | (336,591) | (168,295) | Bust=(133,270)-(229,326) の内側 | 1183/4=295 ✅ |
| 胸 | (341,598) | (170,299) | 内側 | ✅ |
| 胸 | (347,604) | (173,302) | 内側 | ✅ |

**開発者による直接目視判定**: 本番ゴーストで**撫で反応・さわり反応を確認**（2026-08-05）。
ゴーストが実際に反応することは、`region` が正しく解決され SHIORI へ配信されていることの
**人間が直接下した判定**であり、R4.2 の「目視の狙いと一致する Head / Bust / None を解決する」を
最も直接に満たす証跡である。

### 脚 B 判定

**判定: 合格（PASSED）**

根拠: 絶対パス起動で SHIORI 実結線成立（`0x8007007E` 0 件）・shell target `TargetId(0)` の
実 k が `ScaleRatio { num: 2, den: 1 }`（R-2 消化）・本番判定経路 `hit_region_client` が
2271 行すべて同一 k で Head/Bust/None を解決・開発者が撫で／さわり反応を直接目視確認・
正規 clean shutdown で exit 0。

---

## 6. 不一致欄（R4.8）

> **プロセス規定**: いずれかの水準で目視の狙いと解決結果が一致しない場合、
> **その不一致を本欄に記録したうえで是正し、再実施して一致を確認するまで本 spec を完了としない**（R4.8）。
> 不一致を「環境のせい」「誤差」として黙って握り潰すことは禁止する。
> 是正が本 spec の boundary 外に及ぶと判明した場合も、本欄にその判断と担当 spec を明記する
> （担当 spec の実在検証を伴うこと＝記憶 [[deferral-requires-verified-owner]]）。

### 6.1 当たり判定の不一致（R4.8 本来の対象）

| # | 発生日時 | 水準 | 狙った部位 | 期待 region | 実際の region | `client=` / `surface=` | 原因分析 | 是正内容 | 再実施日時 | 再実施結果 |
|---|---|---|---|---|---|---|---|---|---|---|
| — | — | — | — | — | — | — | **不一致なし**（2 水準とも宣言順序と時系列区間が一致・脚 B の撫で／さわり反応も正常） | — | — | — |

### 6.2 実施中に発見した probe の不具合（判定結果とは無関係・是正済み）

| # | 発生日時 | 事象 | 原因 | 是正 |
|---|---|---|---|---|
| 1 | 2026-08-05T03:37Z（水準②実施中） | **キャラ窓をダブルクリックしても probe が終了しない**。wintf 層では `[handle_double_click_message] Double-click detected` が 3 回記録されているのに、その先の despawn が起きない | probe の doc・起動時バナー・結線コメントが**陳腐化**していた。かつて終了を担っていた stand-in `spawn_ghost_windows` の `OnPointerPressed(on_ghost_pressed)` は `areka-P0-input-events` task 2.7 で**退役**し（`placement/spawn.rs` の当該コメント）、正典の脱出口（**Ctrl+左ダブルクリック**・DD-IE-7）は `input_events::attach_char_pointer_handlers` へ移った。同関数は `pub(crate)` かつ内部が `crate::` パスを使うため example から `#[path]` include できず、**probe には `OnPointerPressed` が一切装着されていない**＝手動終了の手段が存在しない | **doc を実態へ是正**（`collision-probe.rs` の「使い方」節・起動時バナーの「終了」行・`OnPointerMoved` 装着箇所のコメント）。終了は `AREKA_APP_SMOKE_EXIT_MS` の有界 auto-exit のみである旨と、その理由（退役した stand-in／`#[path]` include 不能）を明記した。**脱出口の結線そのものは本 spec の射程外**（`input_events` の `#[path]` include 可能化が要る） |

> **判定への影響なし**: 両水準とも有界 auto-exit（`AREKA_APP_SMOKE_EXIT_MS=180000`）で
> 正常終了（exit 0）しており、⑤ の目視観測はすべて終了前に採取済みである。
> R4.5 が要求する「有界の自動終了とログ検索による決定論的判定」は満たされている。

---

## 7. 総合判定

**総合判定: 合格（PASSED）— 2026-08-05**

| # | 完了条件 | 結果 |
|---|---|---|
| 1 | OS 表示スケール 125% で probe 実行・期待 k ゲート（`5/4`）通過 | ✅ 合格 |
| 2 | 125% で頭・胸・背景を**目視のみ**で狙い §3.1 へ記録 | ✅ 合格（宣言順序と時系列区間が完全一致） |
| 3 | OS 表示スケール 200% で probe 実行・期待 k ゲート（`2`）通過 | ✅ 合格 |
| 4 | 200% で頭・胸・背景を**目視のみ**で狙い §3.2 へ記録 | ✅ 合格 |
| 5 | §4 で 2 水準の `physical=` が互いに異なることを照合 | ✅ 合格（478×684 ≠ 764×1094） |
| 6 | 脚 B（本番 emo2 実ゴーストの**絶対パス**起動）＋ R-2 grep | ✅ 合格（`TargetId(0)` k=`ScaleRatio { num: 2, den: 1 }`・`0x8007007E` 0 件） |
| 7 | 不一致があれば §6 に記録し是正 → 再実施 | ✅ 当たり判定の不一致は**なし**。probe の doc 陳腐化 1 件は是正済（§6.2） |

### 要件充足の対応

| 要件 | 充足根拠 |
|---|---|
| **4.1**（2 水準で互いに異なる拡大寸の証跡） | §4：125% physical `478x684` ／ 200% physical `764x1094`・native は両者 `382x547` で不変 |
| **4.2**（目視の狙いと Head/Bust/None が一致） | §3.1/§3.2 の時系列区間対応（§0.4 の突合手順）＋ §5 の**撫で／さわり反応の直接目視** |
| **4.3**（合成入力の禁止・反トートロジー） | probe は `SetCursorPos`/`SendInput` を呼ばない（呼出 0 件）。狙点は `GetCursorPos`→`ScreenToClient` のみ |
| **4.4**（矩形値から合成した点を証跡にしない） | 判定証跡は実カーソル経路のログのみ。④ anchor 画素検査は**描画証跡**として別欄に分離（§0.2） |
| **4.5**（k・縮約前後座標・結果のログ観測／有界自動終了） | `collision-probe: k=`・`client=/surface=/region=`・`[hit_region_client]` を grep で採取。全実行が有界 auto-exit で exit 0 |
| **4.6**（本番ゴースト・絶対パス起動） | §5 脚 B：`areka.exe` と emo2 ghost/balloon を**絶対パス**で起動し実 `pasta.dll` で SHIORI 実結線成立 |
| **4.7**（判定・実測値・実施条件を含む受け入れ記録） | 本文書 |
| **4.8**（不一致は是正まで未完了） | §6：当たり判定の不一致なし。発見した probe doc の陳腐化は§6.2 に記録し是正済み |

### 得られた実機知見（後続への申し送り）

1. **DD-1 の丸め規約が実機で厳密に成立**——k=5/4（割り切れない縮約）で 9/9、k=2 で 5/5 の検算一致。
   特に k=5/4 では **client の 2 行が同一 surface 行へ潰れる**（162→130・163→130）ことを実測。
2. **閉区間の内外が 1px 単位で k によらず保存**——k=2 で Head 下端 surface y=130 が当たり・
   y=131 が外れる。k=5/4 でも Head 下端 130 が当たり・132 が外れる。
3. **probe には手動終了の手段が存在しない**（§6.2）。実行時は必ず `AREKA_APP_SMOKE_EXIT_MS` に
   目視所要時間を見込んだ値（推奨 `180000`）を与えること。
4. **probe は可視の端末で走らせること**（§0.4）。出力をファイルへリダイレクトすると
   その場での目視突合ができなくなる。本実施は宣言順序との時系列対応で代替した。
