# 実機サインオフ手順書（areka-P0-windowposition-limit / tasks.md 4.4）

対応要件: **7.5**（実 emo2・実 pasta・実 DPI 2 水準でバルーンが作業領域内へ収まることの実機確認）
対応設計: **Testing Strategy / E2E・実機サインオフ（7.5）**

> **このタスクの完了状態** = 「§5 目視チェックリスト」と「§6 ログ突合」が**拡大率 2 水準ぶん**
> §7 の記録欄に記入された状態。片方だけ・1 水準だけでは未完了。

## この手順書の分担

| 区分 | 内容 | 担当 | 状況 |
|---|---|---|---|
| A | 検証用バルーン fixture の作成 | 実装（自動） | ✅ 完了（§0.3） |
| B | 水準①（現在の拡大率）での実走とログ突合 | 実装（自動） | ✅ 完了（§7.1・**dpi=192／200%**） |
| C | 水準②（**100%＝dpi=96** 固定）での実走とログ突合 | **開発者** | ⬜ 未実施（§7.2） |
| D | 画面端・モニタ境界へのドラッグと**目視**確認 | **開発者** | ⬜ 未実施（§7.3） |

C は Windows の表示スケール変更（システム設定の変更）を伴うため、D は人間の目を要するため、
いずれも自動化の対象外である。

**水準②は 100%（dpi=96）に固定であり、選択の余地は無い。** タスク 4.4 が求めるのは
「拡大率 **100%** と 100% 以外（125% または 200%）の 2 水準」であり、水準①の実走が既に
**200%（dpi=192）**＝「100% 以外」の枠を埋めている（§7.1）。ゆえに残る枠は **100%** ただ 1 つで、
ここで 125% を選ぶと {200%, 125%} となり **k=1.0 を一度も踏まないまま**タスクが終わってしまう。
k=1.0 は「DPI 追従が基本設計で k=1.0 は途中状態」という前提そのものを検める水準なので、
省いてはならない。

---

## 0. 前提条件（実測で確認済み）

### 0.1 絶対パス

| 項目 | 絶対パス | 実測状況 |
|---|---|---|
| ゴースト root（辞書 13 本込みのフルゴースト） | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39\crates\pilot\examples\shiori-host-32\fixtures\emo2` | ✅ 実在 |
| **検証用** balloon root（本タスクで新設） | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39\crates\pilot\examples\shiori-host-32\fixtures\emo2-kakukaku-wplimit` | ✅ 実在 |
| 実 SHIORI（`pasta.dll`） | `…\fixtures\emo2\ghost\master\pasta.dll` | ✅ 実在（32bit） |
| 起動実行体 | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39\target\debug\areka.exe` | ✅ ビルド済（PE machine `0x8664`） |
| 32bit helper（**areka.exe と同一ディレクトリ必須**） | `…\target\debug\shiori-host32-helper.exe` | ⚠️ **要ステージング**（§1） |
| helper の i686 成果物（コピー元） | `…\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe` | ✅ ビルド済（PE machine `0x014C`） |

**相対パスで起動しないこと。** `pasta.dll` の LOAD が `0x8007007E`（ERROR_MOD_NOT_FOUND）で
落ち、SHIORI が接続しない（steering `areka-emo2-signoff-needs-absolute-paths`）。

### 0.2 実 DPI（拡大率）

`crates/areka/src/main.rs` の `default_helper_exe_path()` と同じく、**表示スケールを指す env は無い**
——Windows の「システム > ディスプレイ > 拡大縮小」で変更し、**areka を起動し直す**。
実効 DPI は**外部ツールで推定せず、areka 自身のログ**で読むこと
（steering `effective-dpi-must-be-read-from-dpi-aware-process`）:

```
placement: 起動時 k₀ を導出（…） primary_dpi=192 shell_author_dpi=96 balloon_author_dpi=96 k_shell=2.0 k_balloon=2.0
[diag.monitor] index=0 handle=… bounds=0,0,2880,1800 work_area=0,0,2880,1704 dpi=192 primary=true
```

| 拡大率 | `primary_dpi` | `k_balloon` |
|---|---|---|
| 100% | 96 | 1.0 |
| 125% | 120 | 1.25 |
| 200% | 192 | 2.0 |

### 0.3 検証用バルーン fixture（本タスクで新設）

原本 `fixtures/emo2/emo2-kakukaku` の複製に、**面別上書き層のみ**を書き換えたもの
（画像・既定層は原本と同一。Paint.NET の作業ファイル `online.pdn` だけは実行に不要なので落とした）。

| ファイル | scope | 追加・変更した行 | 期待する基本位置 |
|---|---|---|---|
| `balloons0s.txt` | 0（sakura） | `windowposition.x,center`／`windowposition.limit,1` | **中央上**（`CenterTop`） |
| `balloonk0s.txt` | 1（kero） | `windowposition.x,bottom`／`windowposition.limit,1` | **中央下**（`CenterBottom`） |

- `windowposition.y` は原本のまま（sakura `-129`／kero `-75`）。キーワード指定でも
  **調整量の加算が続く**こと（要件 4.4）をこの残置で観測する。
- 既定層（`descript.txt`）は原本と 1 バイトも違わない。**面別上書き層が勝つ**ことが
  観測できる形になっている（要件 1.4）。
- 要件 4.1 は `top` ≡ `center` を定めるので、キーワード幾何は `center` と `bottom` の
  2 種で全被覆になる。
- **原本を書き換えていない**理由: `fixtures/emo2` を読む既存テスト（採寸・合成・起動の約 40 本）が
  原本の数値指定 `x=266`／`x=-190` を期待値に持つ。ゆえに複製は `fixtures/emo2` ツリーの
  **外**（兄弟ディレクトリ）へ置いてある——ゴーストツリーの列挙にも影響しない。

---

## 1. 事前準備（i686 helper のステージング・毎回）

`main.rs` の `default_helper_exe_path()` は **`current_exe()` の隣**の
`shiori-host32-helper.exe` を無条件に解決する（helper パスを差す env は**存在しない**）。
`cargo build/test --workspace` は **x64 版**を `target\debug\` へ吐いて上書きするので、
実走の直前に必ず i686 版をコピーし直すこと。

i686 ビルドは**必ず PowerShell** で行う（Git Bash の coreutils `link.exe` が MSVC の
`link.exe` を遮蔽する）。

```powershell
cd C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39
cargo build -p areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
Copy-Item target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe target\debug\ -Force
```

検証（`0x14C` なら OK・`0x8664` なら x64 が残っている＝SHIORI が繋がらない）:

```powershell
function Get-Machine($p){ $b=[IO.File]::ReadAllBytes($p); $o=[BitConverter]::ToInt32($b,0x3C); "0x{0:X}" -f [BitConverter]::ToUInt16($b,$o+4) }
Get-Machine target\debug\shiori-host32-helper.exe   # 期待: 0x14C
Get-Machine target\debug\areka.exe                  # 期待: 0x8664
```

---

## 2. 起動（PowerShell・絶対パス・有界 auto-exit）

```powershell
cd C:\home\maz\git\areka\.claude\worktrees\areka-p0-scope-chain-gap-004c39
$ghost   = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2").Path
$balloon = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2-kakukaku-wplimit").Path
$exe     = (Resolve-Path "target\debug\areka.exe").Path
$dpi     = 96                                              # 水準②＝100%。実測 DPI に合わせる
$log     = "$env:USERPROFILE\Desktop\wplimit-signoff-dpi$dpi.log"

$env:AREKA_APP_SMOKE_EXIT_MS = "600000"                    # 10 分・目視とドラッグの猶予
$env:RUST_LOG                = "info,areka::placement::diag=debug"
& $exe $ghost $balloon *> $log
```

- `AREKA_APP_SMOKE_EXIT_MS`（`main.rs` の `SMOKE_EXIT_ENV`）: 指定 ms 後に全ゴースト窓を
  despawn して**正常終了**する有界 auto-exit。目視とドラッグを行う水準②では**大きめ**にすること
  （水準①の自動実走は 120000＝2 分だった）。
- `RUST_LOG` の `areka::placement::diag=debug` は**必須**。位置ログ `[diag.window_move]` は
  `debug!`（`diag.rs::log_window_move`）ゆえ、`info` だけでは突合の片側が採れない。
  補正ログ `[balloon-limit] Clamp` と縮退警告 `[balloon-limit] Unresolved`、
  解決値ログ（観測点 4）はいずれも `info`／`warn` なので `info` で足りる。
- 途中終了は **Ctrl+左ダブルクリック**（キャラ窓の不透明域）。放置しても指定 ms で自動終了する。

### 2.1 ログの保全

```powershell
$dst = "$env:LOCALAPPDATA\areka-diag\wplimit-signoff-$(Get-Date -Format yyyyMMdd-HHmmss)"
New-Item -ItemType Directory -Force $dst | Out-Null
Copy-Item $log $dst\
Get-FileHash $dst\*.log -Algorithm MD5
```

---

## 3. grep パターン（3 語＋2 語）

第 1 段は接頭辞 1 語で本機能の記録を全部拾える。第 2 段の `context=` で 3 関門を弁別する
（`balloon_limit.rs` の `BALLOON_LIMIT_*_CONTEXT` 定数）。

```powershell
$L = "<保全したログのパス>"

# 第1段: 本機能の記録すべて（補正＋縮退）
Select-String -Path $L -Pattern '\[balloon-limit\]'

# 第2段: 関門の弁別
Select-String -Path $L -Pattern 'context="boot-gate"'      # 起動時関門
Select-String -Path $L -Pattern 'context="runtime-gate"'   # 実行時関門（enqueue_window_set_pos 内）
Select-String -Path $L -Pattern 'context="release-gate"'   # バルーンドラッグ解放時補正

# 縮退（1 件でも出たら要調査。作業領域が解決できず補正を諦めた印）
Select-String -Path $L -Pattern '\[balloon-limit\] Unresolved'

# 位置ログ（最終位置・route つき）
Select-String -Path $L -Pattern '\[diag\.window_move\]'

# 解決値ログ（観測点 4・limit と x_mode の実値）
Select-String -Path $L -Pattern 'windowposition を初期既定位置の調整量へ変換した'

# 実効 DPI（areka 自身の申告）
Select-String -Path $L -Pattern '起動時 k₀ を導出|\[diag\.monitor\] '
```

Git Bash なら `grep -n '\[balloon-limit\]' "$L"` 等で同じ。

---

## 4. 合否判定（ログ突合・機械判定できる部分）

### J1: 解決値ログが fixture のキーワードどおりであること（要件 1.4／4.1／6.2）

観測点 4 の行が scope 0／1 の**両方**出ており、かつ:

| scope | 期待 `limit` | 期待 `x_mode` | 期待 `windowposition_x` |
|---|---|---|---|
| 0 | `true` | `CenterTop` | `None`（キーワードなので数値 x は存在しない） |
| 1 | `true` | `CenterBottom` | `None` |

`windowposition_y` は `Some(-129)`（scope 0）／`Some(-75)`（scope 1）で、
`adjusted=true`・`adjust_dx=0`・`adjust_dy = -129×k`／`-75×k`（丸めは `ScaleRatio::scale_len`＝
0 から遠い側へ half away from zero）。

> **行が出ない scope があっても即「未実装」と読まないこと。** `scope_windowposition` が
> `None` を返す 3 経路（scope が u32 に収まらない／系列解決失敗／面 0 不在）では観測点 4 の行は
> 出ない。ただしその場合は**必ず直前に warn が出ている**ので、warn の有無で切り分ける。

### J2: キーワード基本位置の幾何（要件 4.2／4.3／4.4）

`merge_scope restore` 行（`char_x`／`char_y`／`char_w`／`char_h`）と
`[balloon-limit] Clamp context="boot-gate"` 行の `from`／`balloon_size` から、
**補正前**の基本位置が式どおりであることを検算する。

- 共通（水平中央・DD8 の整数除算＝0 方向切り捨て）:
  `from.x == char_x + (char_w - balloon_w) / 2`
- `CenterTop`（scope 0）: `from.y == char_y - balloon_h + adjust_dy`
- `CenterBottom`（scope 1）: `from.y == char_y + char_h + adjust_dy`

補正が起きない（＝基本位置が既に作業領域内）ときは `Clamp` 行が出ないので、
`from` の代わりに `[diag.window_move]` の `x`／`y` を使う。

### J3: 補正ログと位置ログの一致（要件 6.1）

各 `[balloon-limit] Clamp` 行の直後（同一 `entity=` / 同一 `scope=`）に
`[diag.window_move] … kind=balloon` が出ており、**`to=PointPx { x, y }` と
位置ログの `x=`／`y=` が完全一致**すること。

> 起動時関門（`boot-gate`）は窓書込より前のデータ変換なので**対になる位置ログを持たない**。
> 対応づけるのは `runtime-gate`／`release-gate` の 2 種だけである。

### J4: 補正結果が作業領域に内包されること（要件 2.1／2.3／2.4）

`Clamp` 行の `to`・`balloon_size`・`work_area` から算出する。`right`／`bottom` は**排他**:

```
to.x >= work_area.left  かつ  to.x + balloon_size.w <= work_area.right
to.y >= work_area.top   かつ  to.y + balloon_size.h <= work_area.bottom
```

バルーンが作業領域より大きい逆転区間では `to.x == work_area.left`／`to.y == work_area.top`
（左辺・上辺優先）になり、はみ出したままが**正しい**（要件 2.4）。

### J5: 縮退が起きていないこと

`[balloon-limit] Unresolved` が **0 件**。1 件でも出ていたら、その scope は補正されずに
素通ししているので、目視の合否をその scope について主張してはならない。

### J6: キャラ窓が動いていないこと（要件 2.8）

`[balloon-limit] Clamp` の前後で `[diag.window_move] … kind=char` の値が変わらないこと
（補正はバルーン窓だけに作用する）。

---

## 5. 目視チェックリスト（開発者の担当・要件 7.5）

**各拡大率で以下の 5 点を行う。** 実走中（§2 の auto-exit の猶予内）に行うこと。

1. **左端**: キャラ窓をドラッグして画面の**左端**へ寄せる。バルーンの左辺が
   作業領域の左端で止まり、画面外へ切れないこと。
2. **右端**: 同じく**右端**へ寄せる。バルーンの右辺が作業領域の右端で止まること。
3. **上端**: **上端**へ寄せる。scope 0 は中央上（キャラの上）なので、ここが最も
   はみ出しやすい。バルーンの上辺が作業領域の上端で止まること。
4. **下端**: **下端**へ寄せる。scope 1 は中央下（キャラの下）なので、ここが最も
   はみ出しやすい。バルーンの下辺が作業領域の下端（タスクバーの上）で止まること。
5. **モニタ境界**: キャラ窓を**もう一方のモニタ**へまたがせる。基準は
   **キャラ窓が属するモニタ**の作業領域（要件 5.5）なので、キャラの中心が属する側の
   作業領域で止まること。副モニタは作業領域＝境界（タスクバーなし）のことがあるので、
   `[diag.monitor]` 行の `work_area=` を先に読んでから判定する。

**加えて、基本位置が意図どおりであること**（キーワードの目視・要件 4.2／4.3）:

6. 画面中央付近にキャラを置いた状態で、**sakura（scope 0）のバルーンがキャラ画像の
   真上・水平中央**に、**kero（scope 1）のバルーンがキャラ画像の真下・水平中央**に
   出ていること。`windowposition.y` の負値ぶん（sakura `-129×k`／kero `-75×k`）
   上へずれているのが正しい。

**任意（`release-gate` を踏みたいとき）**: バルーン窓そのものを掴んで画面外へ引っ張り、
**そこで手を離す**。解放位置が作業領域外なら `context="release-gate"` の `Clamp` が
1 行だけ出て、表示位置だけが引き戻される（保存される相対位置は生値のまま）。
バルーン単独ドラッグをしなければこの語は 0 件で正常。

---

## 6. 完了判定

| 判定 | 内容 | 水準①（済） | 水準②（済） |
|---|---|---|---|
| J1〜J6 | §4 のログ突合が全て PASS | ✅ | ✅ |
| §5 の 1〜5 | 画面端・モニタ境界での目視 | ✅ | ✅ |
| §5 の 6 | キーワード基本位置の目視 | ✅ | ✅ |

**2 水準とも**全項目が埋まってはじめてタスク 4.4 は完了である。

---

## 7. 記録欄

### 7.1 水準①（自動実走・2026-08-14）— ログ突合のみ完了

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-14 07:03:03Z 〜 07:05:04Z（121.0 秒・`AREKA_APP_SMOKE_EXIT_MS=120000`） |
| 終了 | exit 0（`shiori-actor: 正規 clean shutdown 完了` → `ghost shutdown sequence completed`） |
| **areka 自身が申告した実効 DPI** | `primary_dpi=192`（**200%**）・`k_shell=2.0`・`k_balloon=2.0` |
| モニタ構成 | 2 台。index0 `bounds=0,0,2880,1800` `work_area=0,0,2880,1704` `dpi=192` primary／index1 `bounds=-2560,195,0,1795` `work_area` 同値 `dpi=144` |
| ログ | `%LOCALAPPDATA%\areka-diag\wplimit-signoff-20260814-160303\wplimit-dpi192.log`（101,253 bytes・md5 `CA3D795DA2CD14A593476E570A1F95C6`・389 行） |

**J1（解決値ログ）: PASS**

```
scope=0 windowposition_x=None windowposition_y=Some(-129) balloon_side=Left  adjust_dx=0 adjust_dy=-258 adjusted=true k=2.0 limit=true x_mode=CenterTop
scope=1 windowposition_x=None windowposition_y=Some(-75)  balloon_side=Right adjust_dx=0 adjust_dy=-150 adjusted=true k=2.0 limit=true x_mode=CenterBottom
```

面別上書き層のマージも `file=balloons0s.txt`／`file=balloonk0s.txt` で確定している。
`-129×2 = -258`・`-75×2 = -150` で調整量も一致（キーワード指定でも加算が続いている＝要件 4.4）。

**J2（キーワード基本位置の幾何）: PASS**

| scope | 実測（`merge_scope restore` ＋ `Clamp` の `from`） | 検算 |
|---|---|---|
| 0 | `char=(2012,330) 868x1374`・`balloon 800x448`・`from=(2046,-376)` | x: `2012 + (868-800)/2 = 2046` ✅／y: `330 - 448 - 258 = -376` ✅（CenterTop） |
| 1 | `char=(1340,904) 672x800`・`balloon 576x406`・`from=(1388,1554)` | x: `1340 + (672-576)/2 = 1388` ✅／y: `904 + 800 - 150 = 1554` ✅（CenterBottom） |

**J3（補正ログ ⇄ 位置ログ）: PASS**

`Clamp` は 4 件（`boot-gate` 2・`runtime-gate` 2・`release-gate` 0）。
`runtime-gate` の 2 件がいずれも直後の位置ログと逐語一致した。

```
[balloon-limit] Clamp runtime 関門 … scope=0 context="runtime-gate" entity=3v0 route=Some(BalloonFollow)
    from=PointPx { x: 2098, y: -96 } to=PointPx { x: 2080, y: 0 } balloon_size=SizePx { w: 800, h: 448 }
[diag.window_move] route=BalloonFollow entity=3v0 kind=balloon scope=0 x=2080 y=0 w=- h=- dpi=192      ← to と一致

[balloon-limit] Clamp runtime 関門 … scope=1 context="runtime-gate" entity=5v0 route=Some(BalloonFollow)
    from=PointPx { x: 1440, y: 1554 } to=PointPx { x: 1440, y: 1298 } balloon_size=SizePx { w: 576, h: 406 }
[diag.window_move] route=BalloonFollow entity=5v0 kind=balloon scope=1 x=1440 y=1298 w=- h=- dpi=192    ← to と一致
```

起動時関門の 2 件（対になる位置ログを持たない）:

```
scope=0 context="boot-gate" from=(2046,-376) → to=(2046,0)    balloon 800x448
scope=1 context="boot-gate" from=(1388,1554) → to=(1388,1298) balloon 576x406
```

**相対位置が焼き付いていないこと（DD6）も実測で確認できた**: 起動時関門が scope 0 の表示位置を
`y=-376 → 0` へ引き戻したにもかかわらず、その後の追従書込は
`char(2064,610) + 生 offset(34,-706) = (2098,-96)` を素で提示している（＝補正後の `0` ではなく
生値の `-706` が保たれている）。scope 1 も同様（`char(1392,904) + 生 offset(48,650) = (1440,1554)`）。

**J4（作業領域への内包）: PASS**（`work_area=0,0,2880,1704`・右下は排他）

| scope | `to` | 右辺 | 下辺 | 判定 |
|---|---|---|---|---|
| 0（runtime） | (2080, 0) | `2080+800 = 2880 <= 2880` | `0+448 = 448 <= 1704` | ✅ |
| 1（runtime） | (1440, 1298) | `1440+576 = 2016 <= 2880` | `1298+406 = 1704 <= 1704` | ✅ |

**J5（縮退なし）: PASS** — `[balloon-limit] Unresolved` 0 件。

**J6（キャラ窓が動かない）: PASS** — `kind=char` の書込は**ログ全体で 2 件のみ**
（L65 `scope=0 (2064,610) route=ReportedSizeReconcile`／L77 `scope=1 (1392,904) route=MoveCue`）で、
いずれも関門とは無関係な経路である。runtime 関門 L66 の直後に現れるのはバルーン書込 L67 だけ、
同じく L78 の直後は L79 だけで、**補正に起因するキャラ窓の書込は 1 件も無い**
（関門がキャラ窓へ波及していない）。

> 上記 2 件の `kind=char` は、時系列では起動時関門 L25／L26（07:03:04.267）の**後**に位置する
> （L65 = 07:03:05.742・L77 = 07:03:07.936）。ただし両者は `ReportedSizeReconcile`／`MoveCue`
> という別経路の書込であり、起動時関門の結果ではない。「補正の後にキャラ窓の書込が一切無い」
> という読み方は事実に反するので採らないこと。

なお本実走はドラッグを含まないため、キャラ窓の書込そのものが 2 件しかない——
「端へ寄せてもキャラが跳ねない」ことの確認は水準②の目視（§5）の担当である。

**未実施**: §5 の目視（1〜6）。この実走は自動実行のため、ドラッグも目視も行っていない。

### 7.2 水準②（開発者・拡大率 **100%（dpi=96）固定**）

**完了（2026-08-14）。** 実走中に要件 4.2 の欠陥を 1 件発見し、本仕様内で是正したうえで再走した
（詳細は tasks.md のタスク 4.5）。以下は**是正後**の走行の実測である。

| 項目 | 値 |
|---|---|
| areka 自身が申告した実効 DPI | `primary_dpi=96`・`k_shell=1.0`・`k_balloon=1.0`（＝**k=1.0 を踏んだ**） |
| モニタ | index0 `work_area=0,0,2880,1752`（タスクバー 48px）／index1 `work_area=-2560,195,0,1795` `dpi=144`（タスクバー無し） |
| 解決値 | scope0 `limit=true x_mode=CenterTop`／scope1 `limit=true x_mode=CenterBottom`（面別上書き層が勝っている） |
| ログ | `%LOCALAPPDATA%\areka-diag\wplimit-dpi96-fixed-20260814-203002\wplimit-dpi96-fixed.log`（266,010 bytes・md5 `0A3168BD606CA88ACC42101D122DD410`） |

**J1 解決値ログ: PASS** — 上記のとおり両 scope とも `limit` と `x_mode` の実値が出ている。

**J2 キーワード基本位置の幾何: PASS** — `char=(2472,1205) 382x547`・`balloon 400x224` に対し
`x = 2472 + (382−400)/2 = 2463`・`y = 1205 − 224 = 981`。実測 `balloon x=2463 y=981` と完全一致。

**J3 補正ログ ⇄ 位置ログ: PASS** — `boot-gate` 1 件・`runtime-gate` 1 件、いずれも直後の
`[diag.window_move]` と値が一致。

**J4 作業領域への内包: PASS** — 是正後は中央位置が最初から領域内に収まり、右端クランプが不要になった
（是正前は `x=2489` で 9px はみ出し、関門が 2480 へ引き戻していた）。

**J5 縮退なし: PASS** — `[balloon-limit] Unresolved` 0 件。

**J6 キャラ窓が動かない: PASS** — 補正に起因する `kind=char` の書込なし。

**キーワード基本位置の再導出（本仕様 4.5 で新設）: 実機で確認**

```
[balloon-keyword] Rederive  scope=0  mode=CenterTop
  old_offset=(+17,-224) → new_offset=(-9,-224)
  old_char_size=434x687 → new_char_size=382x547
```

発火は**1 回のみ**（要件 4.7「キーワードは初期既定位置の供給にとどめる」が実機で成立）。

**保存・復元の実機確認（追加検証）**

- 同一 DPI 内の往復は**完全恒等**: 保存 `(-101,-158)`／`(154,-95)` → 復元も同値。
  再起動 3 サイクル連続で 1 ビットも動かず（ドリフト無し）。
- 保存値がある scope では**再導出が発火しない**（3 サイクルとも 0 件）＝要件 4.7 の保護が実機で成立。
- **DD6 が実機で確認できた**（拡大率 200% 側）: バルーンを画面外 `y=-139` で解放したとき、
  保存されたのは生値 `persist=(-1923,-749)`、表示だけが `release-gate` で `y=0` へ補正された。
  補正は相対位置へ焼き付いていない。

### 7.3 目視サインオフ（開発者・両水準）

| 項目 | 水準①（200%） | 水準②（100%） |
|---|---|---|
| 5-1 左端 | ✅ PASS | ✅ PASS |
| 5-2 右端 | ✅ PASS | ✅ PASS |
| 5-3 上端 | ✅ PASS | ✅ PASS |
| 5-4 下端 | ✅ PASS | ✅ PASS |
| 5-5 モニタ境界 | ✅ PASS | ✅ PASS |
| 5-6 中央上・中央下の基本位置 | ✅ PASS | ✅ PASS |
| サインオフ者 / 日付 | 開発者 / 2026-08-14 | 開発者 / 2026-08-14 |

#### 水準①の目視実走（`primary_dpi=192`・k=2.0）

| 項目 | 値 |
|---|---|
| 起動 | `AREKA_APP_SMOKE_EXIT_MS=600000`・`RUST_LOG=info,areka::placement::diag=debug`・絶対パス・検証 fixture `emo2-kakukaku-wplimit` |
| areka 自身が申告した実効 DPI | `primary_dpi=192`・`k_shell=2.0`・`k_balloon=2.0` |
| モニタ | index0 `work_area=0,0,2880,1704`（タスクバー 96px）／index1 `work_area=-2560,195,0,1795`（**タスクバー無し＝作業領域＝画面全体**） |
| 解決値 | scope0 `limit=true x_mode=CenterTop adjust_dy=-258`／scope1 `limit=true x_mode=CenterBottom adjust_dy=-150` |
| ログ | `%LOCALAPPDATA%\areka-diag\wplimit-signoff-visual-20260814-191549\wplimit-signoff-visual.log`（1,122,588 bytes・md5 `B7B209F1C79AF3227F5386BCB313E6E4`・4,262 行） |

**補正の実測（実ドラッグを伴う走行）**

| 関門 | 件数 |
|---|---|
| `boot-gate` | 2 |
| `runtime-gate` | 921（`route=BalloonFollow` 919・`KeepPositionResize` 2） |
| **`release-gate`** | **7** |
| `Unresolved`（縮退） | **0** |

- **補正 930 件すべてについて、補正後の矩形が当該 `work_area` へ完全内包されていることを
  機械的に検算した（はみ出し 0 件）。** 判定は各行が自分で記録している `work_area=` に対して行い、
  モニタを跨いだ分も取り違えないようにした。
- **`release-gate` に初めて実機証跡が付いた**（自動実走 §7.1 では 0 件だった）。
  バルーン単独ドラッグを画面外で解放し、表示位置だけが引き戻されることを目視で確認済み。
- `Unresolved` が 1 件も出ていない＝縮退経路を一度も踏まずに全補正が成立している。

---

## 8. 申し送り

- **実走のたびに i686 helper を貼り直すこと。** `cargo test --workspace` を挟むと
  `target\debug\shiori-host32-helper.exe` が x64 で上書きされ、SHIORI が繋がらなくなる
  （症状は「窓は出るが一言も喋らない」）。
- **永続状態**: `fixtures/emo2/ghost/master/profile/areka/sylphya.toml`（gitignore 済み）。
  2026-08-14 の実走後も `[boot] count = "1"` のままで、窓位置は保存されていなかった。
  ただし**バルーンやキャラをドラッグすると相対位置が保存され、次回起動の初期位置が変わる**。
  水準を変えて比較するときは、実走前にこのファイルの窓位置の項目を消すか、
  ファイルごと削除してから起動すること（削除しても boot count が 0 に戻るだけで無害）。
- **`[diag.window_move]` の `w=` / `h=` は `-`（未知）になることがある。** 内包判定
  （J4）に使う寸法は `[balloon-limit] Clamp` 行の `balloon_size=` から採ること。
- **`Clamp` が 0 件でも即 FAIL ではない。** `limit_correction` の `None` は
  「クランプしても位置が動かない」の意であり、基本位置が最初から作業領域内なら補正は起きない。
  その場合は `[diag.window_move]` の最終位置に対して J4 の内包判定を行う。
- **本 fixture の `windowposition.limit,1` は正典既定と同値**である。`limit=0` の素通し経路は
  決定論テスト（`follow_balloon_limit_tests.rs` ほか）の担当で、実機サインオフの対象ではない。
  実機で `limit=0` を見たければ fixture の 2 行を `0` へ書き換えて再走し、
  `Clamp` が 0 件になり**バルーンが画面外へはみ出したままになる**ことを確認すればよい。
- `cargo test --workspace` には本仕様と因果独立な既知間欠赤が 1 本ある
  （`areka-emo-atlas` の `warn_fires_on_all_transparent_element`・W6.9 `test-cage-determinism` 所有）。
  実機サインオフの判定には無関係。
