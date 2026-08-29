# emo2 実機サインオフ手順（areka-P0-scope-zorder-pinning）

対象要件: requirements.md **6.1／6.2／6.4／9.4**
判定式の正典: design.md **§Testing Strategy → 実機サインオフ（9.4・有界 auto-exit＋grep）**（`design.md:428-432`）

本書は task 7.4 の成果物であり、**判定手順の定義・使用した検体の作り方・既知ケース較正の記録**である。
実走の結果そのものは `real-machine-signoff.md` にある。

---

## 1. 判定の全体像

| 判定 | 対象要件 | 何を見るか | 走行 |
|---|---|---|---|
| **J1** | 9.4（成立側） | 受理の記録があり、是正の記録の**指令と実測が同一行で一致**し、以後の巡が `AlreadyOrdered` で落ち着く | グループ指定のある走行（`-Mode grouped`） |
| **J2** | 6.1／6.2／6.4／9.4（既定側） | `[zorder-group]` の記録が**5 タグすべてについて 0 件** | グループ指定の無い走行（`-Mode default`） |
| **J3** | 9.5 | 既存ペア機構 `[zorder-pair]` の記録が従来どおり出る | 両方 |

3 点とも**目視を一切使わない**。判定はすべて `signoff-scan.ps1` が記録の照合だけで下す。

> **J2 は design の ⑵ より強い。** design.md:431 は「グループ無し走行で `[zorder-group] fix` が 0 件」とだけ言うが、
> `fix` だけを数えると **見送り・拒否・検証不一致が出ている走行を「何も起きていない」と読める**。
> 本手順は `applied`／`fix`／`skip`／`verify-failed`／`rejected` の **5 タグすべて**について 0 件を主張する。

---

## 2. 判定に使う語（**逐語**・出典つき）

判定語はすべて**実際の走行ログの出力から逐語で取った**。組立の出典は
`crates/wintf/src/ecs/window/zorder_group_diag.rs:36-56`（5 タグの定数）と
`crates/areka/src/emo2_boot/frame/zorder_drain.rs:388-396`（受理本文）である。

**「逐語で取った」は判定語 8 つすべてについて、その語が実際に出た走行を挙げられる形で真である**
（下表は最終行に `[zorder-pair] fix` と `skip` の 2 語を並べているので、行数は 7・語数は 8）。
下表の「実出の対照」欄がその走行を名指しする——語が空振りしていないことは、
その語で非 0 が出た走行を示して初めて言えるからである（§2.4）。

| 語（部分一致） | 出る条件 | 水準 | 実出の対照 |
|---|---|---|---|
| `[zorder-group] applied` | 受理（台帳に載った） | debug | R2・R3 |
| `[zorder-group] fix` | 次巡の検証で**成立**（指令と実測を同一行に載せる） | debug | R2・R3 |
| `[zorder-group] skip` | 見送り（`reason=` 必須） | debug（`GaveUpAfterFailures` のみ warn） | R2・R3・R4 |
| `[zorder-group] verify-failed` | 次巡の検証で**不一致** | error | R4 |
| `[zorder-group] rejected` | 指定そのものの拒否 | warn | **R5**（§3.3 の拒否検体） |
| `[zorder-pair] owner-established` | ペア機構の所有関係の確立 | info | R1〜R5 |
| `[zorder-pair] fix` / `[zorder-pair] skip` | ペア機構の是正／見送り | debug | R1〜R5 |

出力先（tracing target）は `wintf::ecs::window::zorder_group` と
`wintf::ecs::window::zorder_pair` の 2 本だけである。

### 2.0 design.md:431 の ⑶ からの縮小（deviation・意図的）

design.md:431 の ⑶ は「既存 `[zorder-pair]` **6 タグ**が従来どおり出る（9.5）」を求めるが、
本手順が J3 の判定に使うのは **3 タグ**（`owner-established`／`fix`／`skip`）だけである。

残る 3 つ——`verify-failed`／`owner-establish-failed`／`sink-observed`
（`crates/wintf/src/ecs/window/zorder_pair_diag.rs:36-40`）——は**失敗経路と活性化由来**であり、
**健全な無人走行では原理的に出ない**。実際、本サインオフの 5 本と切り分けの 10 本、
および独立レビューの 4 本、計 19 本のログすべてで 0 件である。
これらを J3 の連言に足すと、**出ないことが正常な語で「0 件」を主張する**という
§2.4 が禁じた形になる（対照走行が作れない）。

**よって要件 9.5 の保全は、実機の記録ではなく次の 2 つが担う**:

- 既存ペア機構の**本番 5 ファイル**が**無編集**であること。実測（2026-08-29・`main..HEAD`）＝
  変更ファイル 228 本のうち、`zorder_pair.rs`／`zorder_pair_diag.rs`／`zorder_pair_establish.rs`／
  `zorder_pair_maintain.rs`／`zorder_pair_sink.rs` は **1 本も含まれない**
  （`zorder_pair*` に当たる変更は 3 本あるがすべて `*_tests.rs`）。
  この「含まれない」は、同じ問いに **228 という非 0 の対照**が出ていることで空振りでないと言える。
- タグ名簿の檻（`zorder_group_diag.rs` のタグが 5 個ちょうどで `[zorder-pair]` の語と 1 つも重ならない）

**この縮小は意図的であり、§1 で「J2 は design の ⑵ より強い」と強めた側を明記したのと対になる。**
強めた側だけを書いて弱めた側を黙るのは非対称であり、それ自体が手順の欠陥である。

### 2.1 ⚠ 手順が静かに空振りする書き方（実際に踏んだ罠）

受理行の実際の字面は

```
[zorder-group] applied action=set group_id=0 source=Descript members=b0,s0,s1 normalized=0:false
```

で、**`action=set` と `source=Descript` の間に `group_id=<N>` が挟まる**。
`action=set source=Descript` を**連結文字列**として grep すると 1 件も当たらない。
弁別は `source=` 欄だけが担う（`source=Descript`＝shell 設定由来／`source=Tag`＝台本のタグ由来）。
`action=descript` のような語は存在しない。

### 2.2 ⚠ J3 に `origin=zorder-pair` の件数を使ってはならない

グループ発行の指令は凍結済み `pair_fix_command` 経由で書かれるため、
書込側の記録（`wintf::transition` の `kind=write ... origin=zorder-pair`）では
**グループ発行分がペア発行分に見える**（tasks.md 実装上の申し送り・4.1）。
J3 の判定は `[zorder-pair]` の**診断タグ**だけで行う。

### 2.3 ⚠ PowerShell の `Select-String` で角括弧を素で渡さない

`[zorder-group]` は正規表現では文字クラスになる。`-SimpleMatch` を付けるなら
**バックスラッシュを外した素の字面**を渡すこと。`-Pattern "\[zorder-group\]" -SimpleMatch` は
「バックスラッシュを含む文字列」を探すので **必ず 0 件**になる（本 task で 1 度踏んだ）。
`signoff-scan.ps1` は `String.Contains` を使うのでこの罠を持たない。

### 2.4 ⚠ 「0 件」は対照を添えて初めて意味を持つ

J2 の「0 件」は、**同じ道具・同じ語で非 0 が出る走行**を並べて初めて主張になる。
本手順では `signoff-scan.ps1 -Mode default` を**グループ指定のある走行**にも当てて
FAIL（非 0）が出ることを毎回確かめる（§6 の較正記録）。

---

## 3. 検体（fixture）の用意

判定には **4 種類のゴースト**が要る。共有 fixture（`fixtures/emo2`）は**書き換えない**——
他 spec の走行を壊すためである。既定＝非強制を測る走行は、その共有 fixture を
**そのまま**使って得る。

| 検体 | 用途 | 重なり指定 |
|---|---|---|
| `fixtures/emo2` | 既定＝非強制（J2） | **無し**（`seriko.zorder` を宣言していない） |
| `fixtures/emo2-zsp-descript` | shell 設定由来（J1） | `shell/master/descript.txt` に `seriko.zorder` |
| `fixtures/emo2-zsp-tag` | 台本のタグ由来（J1） | `dic/boot.pasta` の起動 12 パターンに `\![set,zorder,b0,s0,s1]` |
| `fixtures/emo2-zsp-descript`（値を差し替え） | **`rejected` の実出の対照**（§2 の表・§3.3） | `seriko.zorder,Balloon0,zzz`（解釈不能） |

派生検体は**共有 fixture から機械的に作れる**。`ghost` 側／`shell` 側のうち
**書き換えない方はジャンクションで原本を指す**ので、複製されるのは実際に手を入れる側だけである
（`pasta.dll` はハードリンク）。リポジトリ内に置くこと——**ゴーストツリーをリポジトリ外へ複製すると
挨拶トークが返らない**という既知の罠がある。

### 3.1 shell 設定由来の検体を作る

```powershell
$ErrorActionPreference='Stop'
$fx  = "<worktree>\crates\pilot\examples\shiori-host-32\fixtures"
$src = "$fx\emo2"; $dst = "$fx\emo2-zsp-descript"
if (Test-Path $dst) { throw "$dst が既にある。§3.2.1 の片付け手順で先に外すこと（素の Remove-Item -Recurse はジャンクションを辿る）" }
New-Item -ItemType Directory -Force "$dst\shell\master" | Out-Null
New-Item -ItemType Junction -Path "$dst\ghost" -Target "$src\ghost" | Out-Null
foreach ($e in Get-ChildItem "$src\shell\master") {
  if ($e.PSIsContainer) { New-Item -ItemType Junction -Path "$dst\shell\master\$($e.Name)" -Target $e.FullName | Out-Null }
  elseif ($e.Name -eq 'descript.txt') { }   # ← ここだけ実体を置く
  else { New-Item -ItemType HardLink -Path "$dst\shell\master\$($e.Name)" -Target $e.FullName | Out-Null }
}
$d = [System.IO.File]::ReadAllText("$src\shell\master\descript.txt")
$d = $d -replace "seriko.alignmenttodesktop,bottom", "seriko.alignmenttodesktop,bottom`r`nseriko.zorder,b0,s0,s1"
[System.IO.File]::WriteAllText("$dst\shell\master\descript.txt", $d, (New-Object System.Text.UTF8Encoding($false)))
```

`ghost/master/descript.txt`（原本）は `seriko.defaultsurfacedirectoryname` を宣言していないので
shell ディレクトリ名は既定の `master` に解決される（`areka-parsers/src/package/resolve.rs:82-86`）。
よって `ghost` をジャンクションで共有したまま、**shell だけを差し替えられる**。

**指定を変えるときは `descript.txt` の `seriko.zorder,...` の 1 行を書き換えるだけでよい。**

### 3.2 タグ実行の台本を持つ検体を作る

```powershell
$ErrorActionPreference='Stop'
$fx  = "<worktree>\crates\pilot\examples\shiori-host-32\fixtures"
$src = "$fx\emo2"; $dst = "$fx\emo2-zsp-tag"
if (Test-Path $dst) { throw "$dst が既にある。§3.2.1 の片付け手順で先に外すこと（素の Remove-Item -Recurse はジャンクションを辿る）" }
New-Item -ItemType Directory -Force "$dst\ghost\master" | Out-Null
New-Item -ItemType Junction -Path "$dst\shell" -Target "$src\shell" | Out-Null
foreach ($e in Get-ChildItem "$src\ghost\master") {
  $t = "$dst\ghost\master\$($e.Name)"
  if ($e.Name -eq 'dic' -or $e.Name -eq 'profile') { Copy-Item -Recurse $e.FullName $t }
  elseif ($e.PSIsContainer) { New-Item -ItemType Junction -Path $t -Target $e.FullName | Out-Null }
  elseif ($e.Name -eq 'pasta.dll') { New-Item -ItemType HardLink -Path $t -Target $e.FullName | Out-Null }
  else { Copy-Item $e.FullName $t }
}
# 起動 12 パターン（boot.pasta の 12〜77 行）の先頭発話へタグを差し込む
$f = "$dst\ghost\master\dic\boot.pasta"
$lines = [System.IO.File]::ReadAllText($f) -split "`r`n"
$tag = '\![set,zorder,b0,s0,s1]'
for ($i = 11; $i -le 76; $i++) {
  if ($lines[$i] -match '^　むらさき：＠[^　]*　') {
    $lines[$i] = $lines[$i] -replace '^(　むらさき：＠[^　]*　)', "`$1$tag"
  }
}
[System.IO.File]::WriteAllText($f, ($lines -join "`r`n"), (New-Object System.Text.UTF8Encoding($false)))
```

- `shell` はジャンクションで原本（`seriko.zorder` 無し）を指すので、**この検体でグループを作るのはタグだけ**である。
  `source=Tag` と `source=Descript` の弁別がそのまま経路の弁別になる。
- `profile` は**複製**する（ジャンクションにすると pasta のトランスパイル結果を共有 fixture と共有してしまう）。
- 起動 4 バンド（朝／昼／夜／深夜）× 3 パターンの**全部**に差し込むので、実走の時刻帯に依存しない。
- 生の さくらスクリプトを pasta 台詞へ直書きできることは原本 `dic/boot.pasta:79`
  （`　　　エモ：＠通常　\1\![move,-353,,,0,base,base]`）が先例である。

### 3.2.1 派生検体はコミットしない（**再生成が正本**）

`emo2-zsp-descript` / `emo2-zsp-tag` は **§3.1・§3.2 のスクリプトから機械的に再生成できる**
派生物であり、ジャンクションとハードリンクを含むので **git はそのままでは保存できない**
（ジャンクションを追跡できず、辿った先の実体を二重に取り込む）。
コミットせず、走行のたびに作り直すこと。**正本はこの手順書のスクリプトである。**

**⚠ 片付けは必ずジャンクションを先に外すこと。** Windows PowerShell 5.1 の
`Remove-Item -Recurse` は**ジャンクションを辿って実体を消す**——素で走らせると
共有 fixture の中身ごと消える。次の順で消すこと（`cmd /c rmdir` は reparse point だけを外す）。

```powershell
$dst = "<fixtures>\emo2-zsp-descript"   # emo2-zsp-tag も同様
Get-ChildItem $dst -Recurse -Force |
  Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint } |
  ForEach-Object { cmd /c rmdir "`"$($_.FullName)`"" }
Remove-Item -Recurse -Force $dst
git status --short crates/pilot/examples/shiori-host-32/fixtures/emo2   # ← 空であることを確認する
```

最後の 1 行は省略しないこと。**共有 fixture が無傷であることを毎回確かめる。**

### 3.3 拒否の対照検体（`rejected` を実際に出す）

`[zorder-group] rejected` は**正しい指定を書いている限り一生出ない**語である。
出ない語で「0 件」を主張すると、それは「本当に出ていない」ではなく
「語が空振りしている」かもしれない（§2.4）。よって**この語だけは対照走行を 1 本用意する**。

作り方は §3.1 と同一で、**最後から 2 行目の差し込む値だけを替える**（実走で使ったのはこの形である）。

```powershell
$d = $d -replace "seriko.alignmenttodesktop,bottom", "seriko.alignmenttodesktop,bottom`r`nseriko.zorder,Balloon0,zzz"
```

あとは §4.2 と同じ形で 45 秒走らせるだけでよい（`AREKA_APP_SMOKE_EXIT_MS=45000`）。

`Balloon0` は語彙の一致が**小文字ちょうど**であるがゆえに解釈不能になる
（`crates/areka/src/placement/zorder_group_ledger.rs:168-173`）。`zzz` も解釈不能なので、
どちらが先に落ちても拒否は成立する。実出は §6 の較正表と `real-machine-signoff.md` §3.3 にある。

この走行は**拒否したうえで起動が続く**ことも同時に見せる（要件 5.4／8.3）——
`[zorder-group]` の記録は `rejected` の 1 件だけで、`applied` は 0 件、
ペア機構は従来どおり動く。

### 3.4 ⚠ 検体の選び方の落とし穴

- **共有 `fixtures/emo2` を書き換えないこと。** 既定＝非強制の走行は「設定を持たない側」をそのまま使って得る。
- **areka の窓は `WS_EX_TOPMOST` を持たない。** 全画面の端末やエディタの背後に完全に隠れる。
  「出ていない」の第一容疑は Z 順であって欠陥ではない。本手順は目視を使わないので実害は無い。

---

## 4. 実走の手順

### 4.1 事前準備（i686 helper の配置）

`areka.exe` は `current_exe()` の隣の `shiori-host32-helper.exe` を helper として解決する。
32bit の `pasta.dll` を `LoadLibrary` できるよう **i686 版を上書きコピー**しておく。
ビルドは**必ず PowerShell** で行う（Git Bash の coreutils `link.exe` が MSVC の `link.exe` を遮蔽する）。

```powershell
cargo build -p areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
Copy-Item target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe target\debug\ -Force
```

### 4.2 走行（PowerShell・**絶対パス必須**）

相対パスで起動すると helper の `LoadLibrary(pasta.dll)` が `0x8007007E`（ERROR_MOD_NOT_FOUND）で
失敗し、SHIORI が接続しない。**必ず絶対パス**で渡す。

```powershell
$stamp = Get-Date -Format yyyyMMdd-HHmmss
$dst = "$env:LOCALAPPDATA\areka-diag\zsp-signoff-$stamp"
New-Item -ItemType Directory -Force $dst | Out-Null

$balloon = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku").Path
$env:AREKA_APP_SMOKE_EXIT_MS = "120000"                  # 2 分・有界 auto-exit
$env:RUST_LOG = "info,wintf::ecs::window=debug"          # ← debug 込みが必須

foreach ($r in @(
    @{n='R1-default';  g='emo2'},
    @{n='R2-descript'; g='emo2-zsp-descript'},
    @{n='R3-tag';      g='emo2-zsp-tag'})) {
  $ghost = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\$($r.g)").Path
  & "target\debug\areka.exe" $ghost $balloon *> "$dst\$($r.n).log"
}
```

| 走行条件 | 値 | 理由 |
|---|---|---|
| `AREKA_APP_SMOKE_EXIT_MS` | `120000`（2 分） | 起動時の据え置きだけでなく、定期トーク（15〜30 秒間隔）に伴うバルーンの表示・消去が数回起き、**再表示の追随（要件 7.3）で維持の巡が何度も回る**長さが要る。実測で `AlreadyOrdered` が 6〜7 回出る |
| `RUST_LOG` | `info,wintf::ecs::window=debug` | **debug 込みが必須**。`applied`／`fix`／`skip` は debug、`rejected` は warn、`verify-failed` は error。info だけに落とすと J1 の材料が丸ごと消える |
| ゴースト／バルーン | **絶対パス** | 相対だと `pasta.dll` の LOAD が `0x8007007E` で落ちる |
| ビルド | debug プロファイル | — |
| helper | i686 版を `target\debug\` へ配置済み | 実 pasta は 32bit |

拒否の対照走行（R5・§3.3）は同じ形で `AREKA_APP_SMOKE_EXIT_MS=45000` の 1 本を追加する。
拒否は起動の段で 1 度きり出るので、長く走らせる理由が無い。

実 pasta は `OnBoot` で挨拶するので**辞書込みのフルゴースト**が要る（bare DLL は timeout する）。
本手順の 4 検体（§3 の表）はいずれも原本と同じ `dic`／`profile` を持つのでこの条件を満たす。

### 4.3 ログの保全

実走ログは再採取の効かない一次証跡である。判定にかける前に読み取り専用として扱い、
`Get-FileHash -Algorithm MD5` を控える（`real-machine-signoff.md` §1 に転記する）。

---

## 5. 判定スクリプトの使い方

```powershell
pwsh -NoProfile -File .kiro\specs\areka-P0-scope-zorder-pinning\signoff-scan.ps1 `
     -Log <ログのパス> -Mode grouped|default
```

- PowerShell 7（`pwsh`）標準機能のみ。外部モジュール不要。
- `-Mode grouped` は J1＋J3 を、`-Mode default` は J2＋J3 を判定する。
- 出力は日本語で、**判定の根拠となる実測値**（タグ別件数・受理行の逐語・
  `fix` 行の「指令 vs 実測」・`AlreadyOrdered`／`GaveUpAfterFailures` の件数）を印字する。

| 終了コード | 意味 |
|---|---|
| `0` | 判定した項目がすべて **PASS** |
| `1` | いずれかが **FAIL** |
| `2` | **判定不能**（`[zorder-pair]` が 1 件も無い＝ログ水準の誤りや起動失敗／受理の記録が 0 件） |
| `3` | 引数不正・ログを読めない |

### 5.1 J1 の判定式

1. `[zorder-group] applied` かつ `action=set` の行が 1 件以上（`source=` は Descript／Tag のどちらでもよい）
2. `[zorder-group] fix` の行が 1 件以上あり、その行の中で**指令と実測が一致**すること
   - 期待列 ＝ `head=` ＋ `moves=` の「動かした窓」を順に並べたもの
   - 実測列 ＝ `measured=`
   - 例: `head=0x4880F82 moves=0x15218A8@0x4880F82,0x4DC110E@0x15218A8 measured=0x4880F82,0x15218A8,0x4DC110E` → 一致
   - **この照合が 1 行だけで閉じる**のは design の裁定どおり（指令と実測を同一行に載せる・要件 9.1／9.2）。
     `measured=` は前面走査が実際に出会った並びであり、宣言列の写しではない
     （`zorder_group.rs:359-378` `measured_members`）
3. 以後の巡に `reason=AlreadyOrdered` かつ `order_ok=true` が 1 件以上
4. `reason=GaveUpAfterFailures` が 0 件

### 5.2 J3 の「従来どおり」が指す錨

「従来どおり」は本 spec 導入**前**の記録に錨を張っている。
`.kiro/specs/completed/areka-P0-ghost-window-zorder/verification/plan-a-gate.md:51-54` は
本機能が存在しなかった時点の実機ログであり、`owner-established` の欄立ては現行と**同一**である。

```
（導入前・plan-a-gate.md:51-54）
[zorder-pair] owner-established entity=3v0 peer=4v0 owned_hwnd=0x670AD0 owner_hwnd=0x4A0AFC measured_prev=0xBA09CE
[zorder-pair] owner-established entity=5v0 peer=6v0 owned_hwnd=0x410AF2 owner_hwnd=0xBA09CE measured_prev=0x670AD0
[zorder-pair] skip entity=5v0 peer=6v0 reason=AlreadyAdjacent
[zorder-pair] fix  entity=3v0 peer=4v0 insert_after=0xBA09CE measured_next_after_fix=0x4A0AFC
```

`entity=` / `peer=` / `owned_hwnd=` / `owner_hwnd=` / `measured_prev=` の 5 欄が揃うことを
`signoff-scan.ps1` が全 `owner-established` 行について照合する。
**件数だけを数える判定は欄が 1 つ落ちても素通りする**ので、欄立ても機械で見る。

欄名は**語の頭から**（`\b` 付きの正規表現で）照合する。素の部分一致だと `entity=` が
`char_entity=` にも当たって**改名を素通り**させる——`char_entity=` / `balloon_entity=` は
導入前の `[zorder-pair] declared`（`plan-a-gate.md:49-50`）が実際に使っていた語なので仮想の話ではない。
較正済み（§6 の 12 行目）。

**この判定が保証しないこと**（3 点）:

1. **欄の値は見ていない。** 見ているのは欄名の有無だけで、`measured_prev=GARBAGE` は通る
   （§6 の 13 行目で実測）。値の妥当性は J1 の `fix` 行（指令と実測の突合）が担う。
2. **ペア是正の回数の同一性。** 件数の述語は `owner-established == 2` かつ `fix+skip >= 1` なので、
   **グループ機構がペア是正の回数を増やしても素通りする**。導入前の記録は同じ 2 分走行のものではなく、
   回数を比べる条件が揃わないためこの形にした。
3. **`[zorder-pair]` の残り 3 タグ**（§2.0 の deviation）。

保証しているのは「記録が出続けていること」と「欄立て（欄名）が変わっていないこと」の 2 点であり、
回数と語彙の不変は**既存ペア機構の本番 5 ファイルの差分が 0 であること**が担う（§2.0）。

### 5.3 頭打ち（`GaveUpAfterFailures`）の読み方

要件 7.4 の「是正が適用されるまで促す」は、要件 8.2 の諦め（3 連続不一致）で**打ち切られる**。
手順は「**適用されるまで、または warn つきで諦めるまで**促す」と読む。

ただし本判定では `GaveUpAfterFailures` を **J1 の FAIL 条件に含める**。
諦めの記録が出ること自体は仕様どおりの縮退だが、**その走行では「指定が成立した」とは言えない**からである。
頭打ちが出た走行は「機構が黙って壊れた」のではなく「宣言された重なりが実機で成立しなかった」と読む。

---

## 6. 既知ケース較正の記録（2026-08-29）

**赤しか出せない道具は「常に赤」かもしれず、緑しか出せない道具は「常に緑」かもしれない。**
13 通りの当て方で 4 種類の終了コードすべてが出ることを確かめた。

| # | 当てたログ | `-Mode` | 期待 | 実測 |
|---|---|---|---|---|
| 1 | `R1-default.log`（指定なし） | `default` | J2 PASS | **J2=PASS J3=PASS / exit 0** |
| 2 | `R1-default.log`（指定なし） | `grouped` | 受理が無いので判定不能 | **J1=INCONCLUSIVE J3=PASS / exit 2** |
| 3 | `R2-descript.log`（設定由来） | `grouped` | J1 PASS | **J1=PASS J3=PASS / exit 0** |
| 4 | `R2-descript.log`（設定由来） | `default` | **非 0 が出る対照** | **J2=FAIL J3=PASS / exit 1** |
| 5 | `R3-tag.log`（タグ由来） | `grouped` | J1 PASS | **J1=PASS J3=PASS / exit 0** |
| 6 | `R4-numeric.log`（数値モード） | `grouped` | 成立しなければ赤 | **J1=FAIL J3=PASS / exit 1** |
| 7 | `R5-rejected.log`（解釈不能値） | `default` | **`rejected` 1 語だけで非 0 が出る対照** | **J2=FAIL J3=PASS / exit 1** |
| 8 | 存在しないパス | `default` | 引数不正 | **exit 3** |
| 9 | **読めるが読めない**（ディレクトリを渡す） | `default` | 引数不正（FAIL ではない） | **exit 3**（`Access to the path ... is denied.`） |
| 10 | `R1-default.log` から `measured_prev=` 欄だけを落とした複製 | `default` | 欄立ての崩れで赤 | **J3=FAIL / exit 1**（`⚠ 欄が欠けている: measured_prev=` ×2） |
| 11 | `R1-default.log` 素（10 の対照） | `default` | 欄立て一致で緑 | **J3=PASS / exit 0** |
| 12 | `R1-default.log` の `entity=` を `char_entity=` へ改名した複製 | `default` | 欄名の改名で赤 | **J3=FAIL / exit 1**（`⚠ 欄が欠けている: entity=` ×2） |
| 13 | `R1-default.log` の `measured_prev=` の**値**を `GARBAGE` にした複製 | `default` | **緑のまま**（値は見ていない） | **J3=PASS / exit 0** |

- **4 行目**が §2.4 の対照である——**同じ道具・同じ語で非 0 が出る**ことを示すので、
  1 行目の「0 件」は「語が空振りしている」ではなく「本当に記録が無い」を意味する。
- **7 行目**が `rejected` 単独の対照である。この語は R5 以外の走行（本サインオフ 14 本＋独立レビュー 4 本）
  すべてで 0 件なので、この 1 行が無いと **J2 の 5 つ目の連言が恒真**になる
  （出ない語で「0 件」を主張することになる）。
- **9 行目**は「存在するが読めない」を **FAIL（exit 1）ではなく引数不正（exit 3）**へ落とすことの確認である。
  道具の失敗を判定の失敗と同じコードにすると、赤を見た人が製品の欠陥だと読む。
- **10〜13 行目**は J3 の欄立て照合（§5.2）が効いていること、および**効かない範囲**の確認である。
  件数だけを数える判定は欄が 1 つ落ちても素通りするので、変異を注入して赤を作った。
  5 欄それぞれを個別に落とすと**いずれも** J3=FAIL / exit 1 になり、`owner-established` が 0 行の
  ログでは PASS ではなく**判定不能（exit 2）**になる（＝恒真化しない）。
  **12 行目**が欄名の改名を捕まえること、**13 行目**が欄の値までは見ていないことを示す。
  変異体は `diagnostics/mutant-rename-char_entity.log`（79,106 bytes・md5 `7DE6E5DE77447E7095C610F5B8982DBF`）と
  `diagnostics/mutant-value-garbage.log`（78,917 bytes・md5 `F6BCAC230A8E2FD7F657FA4B6C36A753`）に保全した。
  変異体はこの 1 行で作れる（保全済み＝`diagnostics\mutant-no-measured_prev.log`）:

  ```powershell
  $mut = [System.IO.File]::ReadAllText("$dst\R1-default.log") -replace ' measured_prev=0x\w+', ''
  [System.IO.File]::WriteAllText("$dst\diagnostics\mutant-no-measured_prev.log", $mut)
  ```

---

## 7. 走行時の申し送り

- **`[zorder-group] skip group_id=- reason=PairFixThisPass`** は既存ペア機構との調停の記録であり、
  異常ではない（同じ巡にペア是正が出ていればグループ側は動かない）。**毎回出るとは限らない**——
  ペア是正の巡にグループが既に載っているかどうかで決まる。実測（5 走行）＝
  R2・R4 が各 1 件、R1・R3・R5 は 0 件（R3 はタグが届くのがペア是正の巡より後）。
- **`reason=MemberMissing ... declared=N existing=0` が走行終了時に 1 件出る**（窓を despawn した後の
  最後の巡）。異常ではない。**グループが載っている走行だけ**に出る——実測＝R2・R3・R4 が各 1 件、
  R1・R5 は 0 件。
- **数値モード（`seriko.zorder,0,1` 等）は 2026-08-29 時点で実機で成立しない。**
  詳細と切り分けは `real-machine-signoff.md` §4。本手順で J1 を測るときは、
  この欠陥が是正されるまで、成立する形（例 `b0,s0,s1`）と数値モードの**両方**を走らせて
  対比を残すこと。
- **その欠陥の根因候補は 2 つあり、本サインオフでは切れていない**（`real-machine-signoff.md` §4.3）。
  **切る方法はある**——4 枚の連鎖を 1 つの `DeferWindowPos` バッチに積まず**1 件ずつ flush** して
  同じ走行を行い、成立するかを見ればよい。成立すれば根因はバッチ内解決順、成立しなければ所有関係である。
  ただしこれは本番コード（`crates/wintf/src/ecs/window/zorder_group_maintain.rs`）の変更を要するので
  **サインオフの境界の外**である。是正を担当する者が最初に行う切り分けとして残す。
- **`cargo test --workspace` の既知間欠赤**（`areka-P0-test-cage-determinism` 所有）は本判定と因果独立。
