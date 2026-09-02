# emo2 実機サインオフ手順（areka-P0-scope-zorder-pinning）

対象要件: requirements.md **6.1／6.2／6.4／9.4**
判定式の正典: design.md **§Testing Strategy → 「実機サインオフの改訂（要件 9.4・9.5）」**と **§Monitoring**
（行番号では指さない——design.md は本 spec の改訂で大きく動いた。節の見出しで引くこと）

本書は task 7.4 の成果物であり、**判定手順の定義・使用した検体の作り方・既知ケース較正の記録**である。
実走の結果そのものは `real-machine-signoff.md` にある。

---

## 1. 判定の全体像

| 判定 | 対象要件 | 何を見るか | 走行 |
|---|---|---|---|
| **J1** | 9.4（成立側） | 受理の記録があり、**繋いだ行が宣言どおりの本数出て**、**収まった行の宣言と実測が同一行で一致**する | グループ指定のある走行（`-Mode grouped`） |
| **J2** | 6.1／6.2／6.4／9.4（既定側） | `[zorder-group]` と `[zorder-chain]` の記録が**どちらも 0 件** | グループ指定の無い走行（`-Mode default`） |
| **J3** | 9.5 | 既存ペア機構 `[zorder-pair]` の記録が従来どおり出る | 両方 |

3 点とも**目視を一切使わない**。判定はすべて `signoff-scan.ps1` が記録の照合だけで下す。

> ### ⚠ 2026-08-30（task 6.1）: 判定語を**鎖の語彙**へ差し替えた
>
> 初版の判定は「毎巡の観測＋是正」モデルの語（`[zorder-group] fix` / `skip` / `verify-failed`）を
> 読んでいた。本版は前後関係を**所有の鎖**として書いて維持を OS へ委ねる（design DD-1）ので、
> 是正の巡そのものが存在しない。判定語は鎖の 3 語へ移った:
>
> | 初版が読んでいた語 | 本版が読む語 |
> |---|---|
> | `[zorder-group] fix`（是正を出した） | `[zorder-chain] linked`（繋いだ） |
> | `[zorder-group] skip reason=AlreadyOrdered`（次巡で既に並んでいた） | `[zorder-chain] settled`（収まった＝宣言と直後の実測を同一行に載せる） |
> | `[zorder-group] verify-failed`（次巡の検証で不一致） | `[zorder-chain] link-failed`（繋ぎを張れなかった） |
>
> **撤去した判定材料が 2 つある**——`reason=AlreadyOrdered`（既に並んでいた）と
> `reason=GaveUpAfterFailures`（失敗続きで諦めた）。どちらも**毎巡の観測を前提とした語**であり、
> 鎖の下には存在しない。鎖は OS が維持するので「以後の巡」も「連続失敗の頭打ち」も無い。
>
> **据え置いた語**（要件 9.5 の保全対象）: `[zorder-group] applied` / `[zorder-group] rejected`・
> `[zorder-pair]` の 4 本・出所の別（`action=set` / `source=Descript` / `source=Tag`）・
> `$PAIR_OWNER_FIELDS` の 5 欄・終了コードの体系 0/1/2/3。**いずれも 1 字も変えていない。**

> ### 強めた側と弱めた側（**両方を書く**）
>
> §2.0 に「**強めた側だけを書いて弱めた側を黙るのは非対称であり、それ自体が手順の欠陥である**」と
> 書いてあるので、本改訂でも両方を並べる。
>
> **強めた側 ⑴ — J2。** design は「グループ無し走行で是正の記録が 0 件」とだけ言うが、
> 1 語だけを数えると **繋ぎ・見送り・拒否が出ている走行を「何も起きていない」と読める**。
> 本手順は `[zorder-group]`（保全 2 語）と `[zorder-chain]`（鎖 7 語）の**両方の冠**について 0 件を主張する。
> 冠で数えるので、その冠の下に語が増えても数え漏らしにならない。
>
> **強めた側 ⑵ — J1 の「繋いだ行」。** 初版は是正の記録が 1 本あって指令と実測が合っていればよかったが、
> 本版は `linked` の**全行**が宣言列の隣接対に当たることを要求する（位置を偽る繋ぎで赤になる）。
>
> **弱めた側 — J1 から失敗の門が 1 つ消えた。** 初版の J1 は `GaveUpAfterFailures == 0` という
> **全走行にわたる失敗の門**を持っていた。鎖にはこの語が無いので撤去した（§5.1 の撤去表）。
> **撤去したままでは判定は初版より弱くなる**ので、代替として
> **「宣言列ごとの終状態がすべて宣言どおり」という全称**を 3 に置いた（§5.1-3a が裁定の全文）。
> 置かなかった場合に何が抜けるかは実証済みである——存在量化のままなら
> **一致 1 本＋不一致 3 本のログが exit 0 で通る**（§6.1.2 の 9 行目）。
>
> **弱めた側として残っているもの（意図的・§5.1-2 と §5.1-3 に明記）**: `linked` の**被覆**は主張しない
> （期待本数がログから導けないため。決定論テスト側が担う）。`settled` の **`nudge_ok=` は判定に使わない**
> （後押しの成否そのものは結果の並びを決めないため。印字はする）。

---

## 2. 判定に使う語（**逐語**・出典つき）

判定語はすべて**本番の記録行の組立から逐語で取った**。組立の出典は
`crates/wintf/src/ecs/window/zorder_chain_diag.rs`（鎖 7 タグの定数・保全 2 タグの定数・
行を組む純関数）と `crates/areka/src/emo2_boot/frame/zorder_drain.rs`（受理本文）である。
逐語の固定は兄弟テスト `zorder_chain_diag_tests.rs` が持つ——**書式が変われば実装側が先に赤くなる**。

**「逐語で取った」は判定語すべてについて、その語が実際に出た走行を挙げられる形で真でなければならない。**
下表の「実出の対照」欄がその走行を名指しする——語が空振りしていないことは、
その語で非 0 が出た走行を示して初めて言えるからである（§2.4）。
**鎖の 3 語の欄が「task 6.2」になっているのは未取得だからであり、6.2 の走行で埋める。**

| 語（部分一致） | 出る条件 | 水準 | 実出の対照 |
|---|---|---|---|
| `[zorder-group] applied` | 受理（台帳に載った） | debug | R2・R3 |
| `[zorder-chain] linked` | 繋いだ（この窓を 1 つ奥の窓の所有下へ置いた） | debug | **第 2 版 R2〜R8**（`real-machine-signoff.md` §7.4） |
| `[zorder-chain] settled` | 収まった（宣言と**直後の実測**を同一行に載せる） | debug | **第 2 版 R2〜R8**（同上・R8 は 2 本） |
| `[zorder-chain] link-failed` | 繋ぎを張れなかった | error | **実出なし**（下の註） |
| `[zorder-group] rejected` | 指定そのものの拒否 | warn | 初版 **R5**（§3.3 の拒否検体）・第 2 版 **R7**（`CrossGroupRedesignation`） |
| `[zorder-pair] owner-established` | ペア機構の所有関係の確立 | info | R1〜R5 |
| `[zorder-pair] fix` / `[zorder-pair] skip` | ペア機構の是正／見送り | debug | R1〜R5 |

> **⚠ `link-failed` には実出の対照が無い（2026-08-30・task 6.2 で確定）。** 第 2 版の実走 8 本すべてで 0 件であり、
> **健全な走行では原理的に出ない**（Win32 の所有関係の書込そのものが失敗したときだけ出る）。
> よってこの語は §2.4 の意味での対照を持てず、**§2.0 の 3 タグと同じ扱い**にする——
> 道具がこの語を読めることは合成標本 `S3-grouped-linkfailed.log` で赤を作って確かめてある
> （§6.1.2 の 3 行目）。**「0 件だから成立した」と読んではならない**（J1 の成立は `linked` と
> `settled` が担い、`link-failed` は 4 つ目の連言として「失敗が無かったこと」だけを言う）。

判定に使わないが同じ冠を持つ鎖の 4 語（`unlinked` / `absent` / `skipped` / `unlink-failed`）は
J2 の「0 件」に冠ごと入る。**`skipped` の件数を判定条件にしてはならない**——連呼の抑止が
真偽値 1 枚なので、前の待ちが解けないうちに始まった次の待ちは記録されないことがある
（tasks.md 申し送り 2.3）。存在を印字するのはよいが、数えると偽の赤が出る。

出力先（tracing target）は 3 本である:

- `wintf::ecs::window::zorder_chain` ——鎖 7 語のマクロ呼出はここだけ（`absent` を含む）
- `wintf::ecs::window::zorder_chain_diag` ——保全 2 語（`applied` / `rejected`）の**新しい住処**
- `wintf::ecs::window::zorder_pair` ——既存ペア機構（無編集）

`RUST_LOG` は**前方一致**なので、`wintf::ecs::window::zorder_chain=debug` の 1 指定で
`zorder_chain` と `zorder_chain_diag` の両方が点く。`wintf::ecs::window=debug` なら 3 本とも点く。

### 2.-1 ⚠ 冠込みでも当たらない組——`linked` と `unlinked`

`[zorder-chain] linked` は `[zorder-chain] unlinked` に当たらない。冠のあとの区切りまで含めた
`] linked` と `] unlinked` が別物だからである。`link-failed` と `unlink-failed` も同様。
**冠を外して `linked` だけで数えると `unlinked` を巻き込む**ので、必ず冠込みで照合すること。

### 2.0 design の ⑶ からの縮小（deviation・意図的）

design の ⑶ は「既存 `[zorder-pair]` **6 タグ**が従来どおり出る（9.5）」を求めるが、
本手順が J3 の判定に使うのは **3 タグ**（`owner-established`／`fix`／`skip`）だけである。

残る 3 つ——`verify-failed`／`owner-establish-failed`／`sink-observed`
（`crates/wintf/src/ecs/window/zorder_pair_diag.rs:36-40`）——は**失敗経路と活性化由来**であり、
**健全な無人走行では原理的に出ない**。実際、本サインオフの 5 本と切り分けの 10 本、
および独立レビューの 4 本、計 19 本のログすべてで 0 件である。

> **⚠ 2026-08-30（task 6.2）の訂正: `sink-observed` は「無人走行では出ない」であって
> 「出ない」ではない。** 第 2 版の走行 R6 は**外から窓を活性化した**ので、この語が **2 件**出た
> （`[zorder-pair] sink-observed entity=… adjacency_ok=true foreground=0x… behind_foreground=…`・
> `real-machine-signoff.md` §8.4⑴）。**J3 はこの語を判定に使っていないので判定は不変**である。
> むしろこの実出は、外から与えた刺激が本当にアプリへ届いたことの**自己検査**として働いた
> ——刺激の到達を確かめずに「順が保たれた」と言うのは、攪乱の届かない檻で緑を取るのと同じである。
> **ただし 3 発の活性化のうち直接の証言が付いたのは 2 発で、残る 1 発（鎖の根）は
> 記録も probe の差分も残らなかった**。その 1 発の到達は「次の活性化が出した `sink-observed` の
> `entity=` が、前に活性だった対を名指す」という**非活性化枝からの含意**で閉じてある
> （`real-machine-signoff.md` §8.4⑴）。
> 残る 2 語（`verify-failed`／`owner-establish-failed`）は第 2 版の 8 本でも 0 件のままである。
これらを J3 の連言に足すと、**出ないことが正常な語で「0 件」を主張する**という
§2.4 が禁じた形になる（対照走行が作れない）。

**よって要件 9.5 の保全は、実機の記録ではなく次の 2 つが担う**:

- 既存ペア機構の**本番 5 ファイル**の**記録の語彙が無編集**であること。
  **⚠ 2026-08-30（task 6.1）に測り直した。初版の「5 本とも 1 本も変更されていない」は本版では偽である。**
  実測（`origin/main...HEAD`）＝変更ファイル **71 本**のうち `zorder_pair*` に当たるのは **2 本**で、
  1 本は `zorder_pair_deferred_vocabulary_tests.rs`（テスト）、もう 1 本が**本番の
  `zorder_pair_maintain.rs`（+7/-2）**である。ただしその差分は**説明文の訂正だけ**——
  「スコープをまたぐ owner はそもそも存在しない」という記述が、鎖の横断 edge の着地によって
  偽になったので訂正した（`zorder_pair_maintain.rs:258-267`）。**実行される行は 1 行も動いておらず、
  記録の語彙を持つ `zorder_pair_diag.rs` は変更ファイルに含まれない。**
  この「含まれない」は、同じ問いに **71 という非 0 の対照**が出ていることで空振りでないと言える。
- タグ名簿の檻（`zorder_chain_diag_tests.rs` の
  `the_seven_chain_tags_share_one_prefix_and_are_all_distinct`／
  `the_two_preserved_group_tags_kept_their_exact_spelling_through_the_move`／
  `no_extra_tag_hides_outside_the_two_rosters`／`the_preserved_tags_have_exactly_one_home_in_this_crate`）
  ——鎖のタグが 7 個ちょうど・保全の 2 語が移設の前後で 1 字も変わっていない・
  退役した語が本番ソースに 1 つも残っていないことを、実装側で赤にできる形で固定している

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

J3 の判定は `[zorder-pair]` の**診断タグ**だけで行う。

初版でこの禁が要ったのは、グループ発行の指令が凍結済み `pair_fix_command` 経由で書かれ、
書込側の記録（`wintf::transition` の `kind=write ... origin=zorder-pair`）で
**グループ発行分がペア発行分に見えた**からである（tasks.md 実装上の申し送り・4.1）。

**2026-08-30（task 6.1）: 取り違えの経路そのものは消えた**——鎖の後押しは指令キューを経由せず
直接 `SetWindowPos` を出す（design DD-4）。それでも**禁は据え置く**: 書込側の欄はどちらにせよ
ペア機構の診断記録ではなく、そこから要件 9.5 の保全を言うことはできないからである。

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

### 3.5 第 2 版（task 6.2）で足した検体と道具（**再生成が正本**）

task 6.2 は初版の 3 検体に加えて**タグ由来の検体を 3 通り**と**活性化を外から与える道具**を使った。
いずれも派生物でありコミットしない。作り方は次のとおりである（`$dst` の作成と片付けは §3.2／§3.2.1 と同一）。

| 検体 | 起動（`boot.pasta` の 12 パターン） | 通常トーク（`talk.pasta` の 48 シーン） | 何を測るための検体か |
|---|---|---|---|
| `emo2-zsp-tag` | `\![set,zorder,b0,s0,b1,s1]` | `\![reset,zorder]` | **解除**が実機で実行され既定へ戻ること |
| `emo2-zsp-tag2` | `\![set,zorder,b1,s1,b0,s0]` | `\![set,zorder,b0,s0,b1,s1]` | グループ有効中の**再指定**（実測では拒否された） |
| `emo2-zsp-tag3` | `\![set,zorder,b1,s1,b0,s0]` | `\![reset,zorder]\![set,zorder,b0,s0,b1,s1]` | 走行の途中で**鎖を逆順へ組み替える**（宣言列が 2 本になる） |

差し込みは §3.2 のスクリプトの `$tag` を替え、**通常トーク側にも同じ要領で差し込む**だけである
（`＊通常トーク` の直後の最初の発話行へ 1 つずつ）:

```powershell
$g = "$dst\ghost\master\dic\talk.pasta"
$tl = [System.IO.File]::ReadAllText($g) -split "`r`n"
$rtag = '\![reset,zorder]'          # tag3 は '\![reset,zorder]\![set,zorder,b0,s0,b1,s1]'
$m = 0; $pending = $false
for ($i = 0; $i -lt $tl.Count; $i++) {
  if ($tl[$i] -eq '＊通常トーク') { $pending = $true; continue }
  if ($pending -and $tl[$i] -match '^　[^　]+：＠[^　]*　') {
    $tl[$i] = $tl[$i] -replace '^(　[^　]+：＠[^　]*　)', "`$1$rtag"; $m++; $pending = $false
  }
}
[System.IO.File]::WriteAllText($g, ($tl -join "`r`n"), (New-Object System.Text.UTF8Encoding($false)))
```

- 差し込み数は **48**（`＊通常トーク` の全シーン）。時刻帯にも抽選にも依存させないため全シーンへ入れる。
- `　　　エモ：` の行は先頭が全角空白 3 つなので上の正規表現に当たらない。当たらない場合は
  待ち（`$pending`）が続き、そのシーンの**むらさきの発話**へ入る。どちらでも同じシーンに 1 つだけ入る。

#### 3.5.1 ⚠ 「グループ有効中に同じスコープを指名し直す」タグは拒否される

`emo2-zsp-tag2` は「途中で別の順を指定する」つもりの検体だったが、実機の答えは

```
[zorder-group] rejected reason=CrossGroupRedesignation(0,1) tokens=b0,s0,b1,s1
```

であった（既に別グループに載っているスコープを含むタグは採らない）。**組み替えを測りたいなら
先に `\![reset,zorder]` を出すこと**（＝`emo2-zsp-tag3`）。この 1 行は同時に、拒否しても起動が続き
**成立済みの鎖が保たれる**ことの証跡でもある（R7 は J1=PASS）。

#### 3.5.2 活性化を外から与える道具（利用者の操作の代わり）

判定は**ログだけ**で下すが、「利用者の操作で活性化させた」ことを確かめるには外から刺激を与える必要がある。
PowerShell の P/Invoke 5 本で足りる（`GetTopWindow`／`GetWindow(GW_HWNDNEXT=2)`／`IsWindowVisible`／
`GetWindowThreadProcessId`／`SetForegroundWindow`＋`BringWindowToTop`）。

- 窓の列挙は `GetTopWindow(NULL)` から `GW_HWNDNEXT` を辿る——**この順が手前から奥の順**である。
  対象プロセスの **可視**窓だけを拾う（不可視の既定 IME 窓を巻き込まないため）。
- **活性化させる窓はログの `declared=` から採る。** 目で見当をつけると鎖の外の窓を掴む
  （実際に 1 度掴んだ——R5 の 1 度目は鎖外の窓で、これはこれで「部外の窓が前に出ても鎖の中の順は動かない」
  という別の証跡になった）。走行中のログを読み、`[zorder-chain] settled` の `declared=` を分解して
  **根（末尾）・先頭・奥から 2 枚目**を名指しで活性化する。
- **刺激が届いたことを必ず自分で検査する。** アプリ側に `[zorder-pair] sink-observed` が出て
  `foreground=` がこちらの活性化させた窓と一致することを確かめる（§2.0 の訂正註）。
  これを見ずに「順が保たれた」と言うと、**刺激が届かない檻で緑を取る**のと同じになる。
  - **⚠ 1 発目は証言が残らないことがある。** `sink-observed` の目印は `WM_ACTIVATE` の
    **非活性化枝**でしか付かない（`crates/wintf/src/ecs/window/zorder_pair_sink.rs:52-53,87`）ので、
    **最初の活性化は「降りる側」が居らず記録を残さない**。鎖が既に最前面に居れば probe の並びも動かない。
    **活性化は 2 発以上出し、後続の `sink-observed` の `entity=` が前の発の対を名指すことで
    1 発目の到達を裏から取ること**（実例＝`real-machine-signoff.md` §8.4⑴）。
- 実走の道具は `real-machine-signoff.md` §7.3 の `*.probe.txt` を生んだスクリプトであり、
  上の 4 点を満たせば実装は問わない。

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
| `AREKA_APP_SMOKE_EXIT_MS` | `120000`（2 分） | 起動時の据え置きだけでなく、定期トーク（15〜30 秒間隔）に伴うバルーンの表示・消去が数回起き、窓の出入りで鎖が組み替わる機会が要る。**鎖の下では「維持の巡」が回らない**（OS が維持する）ので、初版のように `AlreadyOrdered` の回数で長さを測ることはできない——測るのは**繋ぎと収まりの機会が何度あったか**である |
| `RUST_LOG` | `info,wintf::ecs::window=debug` | **debug 込みが必須**。`applied`／`linked`／`settled` は debug、`rejected` は warn、`link-failed` は error。info だけに落とすと J1 の材料が丸ごと消える。`wintf::ecs::window::zorder_chain=debug` へ絞ってもよい（前方一致で `zorder_chain_diag` も点くが、その場合 `zorder_pair` を別途足すこと——足さないと J3 が判定不能になる） |
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
  `settled` 行の「宣言 vs 実測」・`linked` 行の「宣言どおりの位置か」・`link-failed` の件数）を印字する。

| 終了コード | 意味 |
|---|---|
| `0` | 判定した項目がすべて **PASS** |
| `1` | いずれかが **FAIL** |
| `2` | **判定不能**（`[zorder-pair]` が 1 件も無い＝ログ水準の誤りや起動失敗／受理の記録が 0 件） |
| `3` | 引数不正・ログを読めない |

### 5.1 J1 の判定式（2026-08-30 改訂・鎖の語彙）

**判定 1 は「繋いだ行が宣言どおりの本数出て、収まった行の宣言と実測が一致する」ことである。**

1. `[zorder-group] applied` かつ `action=set` の行が 1 件以上（`source=` は Descript／Tag のどちらでもよい）
   ——**据え置き**。0 件なら FAIL ではなく**判定不能**（沈黙を PASS と読ませない）
2. **繋いだ本数**: `[zorder-chain] linked` が 1 件以上あり、**そのすべての行が宣言どおりの位置**であること
   - 切り出しは `owned_hwnd=(\S+)\s+owner_hwnd=(\S+)\s+pos=(\d+)/(\d+)`
   - 「宣言どおりの位置」＝ `pos=i/n` の `n` が `settled` 行の `declared=` の要素数と一致し、
     その列の **i 番目が `owned_hwnd=`**、**その 1 つ後ろが `owner_hwnd=`** であること
   - 鎖は「手前の窓が 1 つ奥の窓を所有する」直線なので、宣言列と繋ぎはこの形でしか噛み合わない。
     位置を偽る繋ぎ・宣言に無い窓を繋いだ行は、ここで赤くなる
   - 最も奥の窓は所有先を持たないので `pos=n` は原理的に出ない（`1 ≤ i < n`）
   - **宣言列の窓の枚数と `linked` の本数は一致しない**（同一スコープのバルーン／キャラ窓の対は
     既存ペア機構が張るので鎖の繋ぎには数えない・design DD-2）。よって「本数」は**素の件数**ではなく
     **「宣言列のどの位置に対して出たか」**で数える
   - **⚠ 被覆は主張しない。** 見ているのは「各 `linked` 行が宣言列のどこかの隣接対に当たること」だけで、
     **宣言された横断 edge が全部出たか**は見ていない（同じ繋ぎが 3 本重複しても通る）。
     見ないのは**期待本数がログから導けない**ためである——鎖の繋ぎの本数は「宣言列の窓の枚数 − 1」から
     同一スコープ対の数を引いたものだが、その対がいくつ在るかは記録に出ないスコープ構造が決める。
     **被覆の主張は決定論テスト側（`compose_chain` の分岐網羅・要件 10.2）が担う。**
3. **収まった一致**: `[zorder-chain] settled` の行が 1 件以上あり、**宣言列ごとの最後の 1 行**（終状態）で
   **宣言と実測が一致**すること。**量化子は「宣言列ごとの終状態についての全称」である**（下の裁定を参照）
   - 切り出しは `nudged_hwnd=(\S+)\s+insert_after=(\S+)\s+declared=(\S+)\s+measured=(\S+)`
   - 例: `declared=0x4880F82,0x15218A8,0x4DC110E measured=0x4880F82,0x15218A8,0x4DC110E` → 一致
   - **この照合が 1 行だけで閉じる**のは design の裁定どおり（宣言と実測を同一行に載せる・要件 9.2）。
     `measured=` は後押しの直後に前面走査が実際に出会った並びであり、宣言列の写しではない
     （`zorder_chain_apply.rs` `measure_chain_order`。不可視の窓は読み飛ばす・要件 9.3）
   - 番兵（`-`）は「その経路にはその値が無い」であって一致ではない。`declared=-` は通さない
   - ⚠ 実際の行は 4 欄の**後ろ**に `nudge_ok=` を持つ。**欄を足すときは必ず 4 欄の後ろへ足すこと**。
     間へ割り込ませるとこの正規表現が静かに 0 件になる（`zorder_chain_diag_tests.rs` の
     `the_four_signoff_fields_of_the_settled_line_stay_adjacent_in_this_order` が隣接を字面で固定している）
   - **⚠ `nudge_ok=` は判定に使わない**（印字はする）。この欄は後押しそのものの成否であり、
     design の Error Strategy が「後押しの失敗は記録して続行する」と定めている。**後押しが失敗しても
     終状態が宣言どおりなら走行は成立しており、失敗して並びが崩れたなら 3 の終状態が捕まえる。**
     よってこの欄は診断の材料であって門ではない——`skipped` の件数を数えない禁・`origin=` を使わない禁と
     同じ筋の据え置きである
4. `[zorder-chain] link-failed` が 0 件

#### 5.1-3a ⚠ 3 の量化子の裁定（2026-08-30・レビュー round 1 の指摘による）

**採った形: 宣言列ごとの「最後の `settled`」がすべて一致していること（＝終状態の全称）。**

初版の改訂案は「**1 本でも一致すれば緑**」（存在量化）だったが、これは**穴である**。

- `settled` は**結果の並びの食い違いを報せる唯一の記録**である。`link-failed` が載せるのは
  Win32 呼出そのものの失敗だけで、**呼出が成功して並びが宣言どおりにならなかった場合は
  `settled` にしか出ない**（design の Error Strategy）。
- しかも 6.2 の走行は `settled` が**複数本出るのが前提**である（§4 が走行長を
  「繋ぎと収まりの機会が何度あったか」で測っている）。
- したがって存在量化では **N 本中 1 本当たれば緑**になり、
  **初版を実機 NO-GO にした症状（宣言した並びが実機で成立しない）をそのまま見逃す。**
  実証済み——一致 1 本＋不一致 3 本のログが exit 0 で通った（§6.1.2 の 9 行目が是正後の赤）。

**⒝ を採った理由は次の 2 点である**（初版のこの節は理由を**事実より広く**書いていた。
レビュー round 2 の指摘で下記のとおり縮めた）:

1. **語義**——「収まった（settled）」は**落ち着いた先**の意である。宣言列ごとに最後の 1 行を見て、
   その終状態が宣言どおりであることを求めるのが、この語の素直な読みである。
2. **同一宣言列の中で redeem される不一致を通すため**——`measure_chain_order` の前面走査が
   打切りになる等で 1 巡だけ `measured` が揃わなくても、**窓の集合が動いていなければ
   `declared=` の字面はバイト同一のまま**なので、後続の巡の一致が同じ鍵を上書きして救う。
   ⒜（不一致 0 本）はこれを赤にするが、⒝ は通す。

### ⚠ ⒝ が救わない形——**宣言列が縮む過渡は ⒝ でも赤になる**

**⒝ が ⒜ より通すのは「宣言列の字面がバイト同一のまま、後の `settled` で一致へ redeem される」
場合だけである。** 窓が去って**宣言列そのものが縮む**過渡は救わない——短い実測が出るのは
**窓の集合が動く巡**であり、その巡の宣言列の字面は**二度と再来しない**。よって短い実測が
その鍵の終状態のまま残り、⒝ でも赤になる。

実測済み: `S13-departing-shrink.log`／レビュアー検体 `RVA-departing-transient.log` はいずれも
**J1=FAIL / exit 1**（§6.1.2 の 13・23 行目）。

したがって、下の 2 経路のうち **⒝ が実際に吸収するのは経路 2 の「窓の集合が動かない」側だけ**である:

| # | 経路 | 出典 | ⒝ での扱い |
|---|---|---|---|
| 1 | 撤去が 1 本でも実行された巡は `acted` が立ち、**その巡でも後押しと実測が走って `settled` を 1 本出す** | `zorder_chain_apply.rs:198-228` | **吸収しない**（窓が去る巡なら宣言列が縮むため） |
| 2a | `measure_chain_order` の前面走査が打切りになる等で、**窓の集合は動かないまま** `measured` が揃わない | `zorder_chain_apply.rs:512-525` | **吸収する**（宣言列の字面が同じなので後の一致が救う） |
| 2b | 窓が去る／現れることで `measured` が短くなり、**宣言列そのものが変わる** | 同上 | **吸収しない**（赤になる） |

**⚠ よって「窓の出入りの過渡は自動的に許される」と読んではならない。** 経路 1・2b は
**例外として本節へ登記するまで通らない**。6.2 で終状態の不一致が実際に出た場合は、
その 1 件を良性と決めつけず、**ログの逐語を添えて本節へ登記してから**例外にすること。
登記の形は「どの宣言列が・どの巡で・なぜ再来しないのか」を名指しすること。

### 使っていない区切りが 1 つある（FINDING 4・意図的）

鎖が一度ほどけて同じ字面で組み直された走行では、ログに
**`[zorder-chain] unlinked reason=Teardown` という区切りが在る**のに、本判定はそれを使わず
「同じ宣言列の字面」だけで終状態を決めている。よって
**ほどける前の不一致が、組み直したあとの一致に隠される**（レビュアー検体
`RVB-episode-masking.log` が exit 0＝§6.1.2 の 24 行目）。

終状態が正しいので語義上は妥当だが、**区切りを使えばもっと細かく（挿話ごとに）測れる**ことは
記録しておく。使っていないのは、挿話の切り出しが `unlinked` の理由語（`Teardown`／`Rechain`／
`Departing`／`Diverged`）のどれを境目と見なすかという新しい裁定を要し、その裁定を下せる実測が
まだ無いためである（6.2 待ち）。

**`linked`（2）と `settled`（3）で量化子が揃っていることを確かめよ**——`linked` は全行について、
`settled` は宣言列ごとの終状態について、どちらも**全称**である。初版の J1 は
`GaveUpAfterFailures == 0` という全走行にわたる失敗の門を持っていた。それを撤去した以上、
代替の全称をここに置かないと**判定は初版より弱くなる**。

**撤去した 2 つの判定材料**（初版の 3・4）:

| 撤去した材料 | なぜ鎖の下に存在しないか |
|---|---|
| `reason=AlreadyOrdered` かつ `order_ok=true`（既に並んでいた） | 毎巡の観測をやめた（design DD-1・要件 14.2）。維持は OS が行うので「以後の巡」で確かめる相そのものが無い |
| `reason=GaveUpAfterFailures`（失敗続きで諦めた） | 連続失敗の頭打ちは是正モデルの縮退であり、鎖には再試行の巡が無い。張り失敗は `link-failed` 1 本で閉じ、その 1 本だけを飛ばして残りは張る（design Error Strategy） |

**⚠ `[zorder-chain] skipped` の件数を判定条件にしてはならない。** 連呼の抑止が真偽値 1 枚なので、
前の待ちが解けないうちに始まった次の待ちは記録されないことがある（tasks.md 申し送り 2.3）。
存在を印字するのはよいが、数えると偽の赤が出る。

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

### 5.3 頭打ち（`GaveUpAfterFailures`）は退役した

初版はここに「要件 7.4 の『是正が適用されるまで促す』は要件 8.2 の諦め（3 連続不一致）で
打ち切られる」という読みを置き、`GaveUpAfterFailures` を J1 の FAIL 条件に含めていた。

**本版にはこの語も、この読みも無い。** 鎖の下では促す相そのものが無く（維持は OS が行う・design DD-1）、
再試行の巡も連続失敗の勘定も存在しない。張り失敗はその 1 本を飛ばして残りを張り、
`[zorder-chain] link-failed` を 1 本残す——それが J1 の 4 番目の条件である。

**この節を空にせず残してあるのは、退役の事実そのものが判定の意味の一部だからである。**
初版の記録（`real-machine-signoff.md` §4）は `GaveUpAfterFailures` 8 件を根拠に数値モードの
不成立を述べており、その語を探しに来た読み手がここで行き止まりにならないようにする。

---

## 6. 既知ケース較正の記録（初版・2026-08-29／**退役した是正モデルの語彙**）

> ⚠ **本節の 13 通りは初版（毎巡の観測＋是正）の語彙に対する較正である。**
> 判定語が鎖の語彙へ移った後もそのまま通るのは **J2／J3 と終了コードの体系**だけで、
> **J1 の行（3・5・6 行目）は本版では成立しない**——`R2`〜`R4` は初版の実装で採ったログなので
> `[zorder-chain]` の記録を 1 行も持たず、本版の道具に当てると J1=FAIL になる（正しい赤である）。
> **本版の較正は §6.1 が持つ。** 本節は初版の一次証跡として残す。

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

## 6.1 既知ケース較正の記録（本版・鎖の語彙・2026-08-30／task 6.1）

判定語を差し替えたら、**差し替えた語で緑と赤の両方が出ること**を差し替えたその場で確かめる。
確かめずに 6.2 の実走へ進むと、実走の緑が「道具が常に緑」なのか「本当に成立した」のかが区別できない。

### 6.1.1 標本の作り方（**再生成が正本**）

鎖の語彙で実際に走ったログは task 6.2 まで存在しない。よって標本は**既存の一次証跡と、
本番の行組立を逐語で固定している兄弟テストの期待値**から組んだ。手で書いた字面は 1 つも無い。

- **据え置く語**（`[zorder-group] applied`・`[zorder-pair]` 4 本・受理の `action=set` / `source=`）は
  **実走ログ `R2-descript.log` / `R1-default.log` の行をそのまま使う**。据え置きが本当に据え置きで
  あることは、実際に出た行に当てて初めて言える。
- **鎖の 3 語**は `crates/wintf/src/ecs/window/zorder_chain_diag_tests.rs` が
  `assert_eq!` で逐語固定している期待値の書式へ、`R2-descript.log` の**実在の窓ハンドル**を入れて組んだ。
  書式が変われば**先に実装側のテストが赤くなる**ので、標本が黙って古くなることはない
  （固定しているのは `the_linked_line_carries_segment_both_ends_and_the_position_in_the_chain`／
  `the_settled_line_carries_the_declaration_and_the_measurement_together`／
  `the_four_signoff_fields_of_the_settled_line_stay_adjacent_in_this_order`／
  `the_link_failed_line_carries_the_segment_and_both_handles`）。
- **出力先**（tracing target）も本番どおりに分けた——鎖の 3 語は `wintf::ecs::window::zorder_chain`、
  受理は移設先の `wintf::ecs::window::zorder_chain_diag`、ペアは `wintf::ecs::window::zorder_pair`。

```powershell
$src = "$env:LOCALAPPDATA\areka-diag\zsp-signoff-20260829-145858"
$dst = "$env:LOCALAPPDATA\areka-diag\zsp-chain-calibration"      # 任意の作業先
New-Item -ItemType Directory -Force $dst | Out-Null

$r2 = [System.IO.File]::ReadAllLines("$src\R2-descript.log")
# 退役した是正モデルの 3 語を落とす。据え置く語（applied / pair 系）はそのまま残す。
$base = @($r2 | Where-Object { -not ($_.Contains('[zorder-group] fix') -or
                                     $_.Contains('[zorder-group] skip') -or
                                     $_.Contains('[zorder-group] verify-failed')) })
# 受理は出力先だけが移設先へ移った（字面は 1 字も変えない）。
$base = @($base | ForEach-Object { $_ -replace 'zorder_group: \[zorder-group\] applied',
                                              'zorder_chain_diag: [zorder-group] applied' })

$P  = '2026-08-29T06:00:59.531463Z DEBUG actor{actor=emo-text}: wintf::ecs::window::zorder_chain: '
$P2 = '2026-08-29T06:00:59.531581Z DEBUG actor{actor=emo-text}: wintf::ecs::window::zorder_chain: '
$PE = '2026-08-29T06:00:59.531700Z ERROR actor{actor=emo-text}: wintf::ecs::window::zorder_chain: '
$H1='0x4880F82'; $H2='0x15218A8'; $H3='0x4DC110E'   # R2 実走の実在ハンドル（b0 / s0 / s1）

$LINKED  = "$P[zorder-chain] linked segment=g0 owned=23v0 owner=25v0 owned_hwnd=$H2 owner_hwnd=$H3 pos=2/3"
$SETTLED = "$P2[zorder-chain] settled nudged_hwnd=$H1 insert_after=0x2071C declared=$H1,$H2,$H3 measured=$H1,$H2,$H3 nudge_ok=true"
$SKIPPED = "$P2[zorder-chain] skipped reason=NoChange"
$ABSENT  = "$P2[zorder-chain] absent group_id=0 element=b1"
$BAD     = "$P2[zorder-chain] settled nudged_hwnd=$H1 insert_after=0x2071C declared=$H1,$H2,$H3 measured=$H2,$H1,$H3 nudge_ok=true"
# 窓の出入りの過渡で `measured` が `declared` より短くなる形（§5.1-3a の経路 2）。
$SHORT   = "$P2[zorder-chain] settled nudged_hwnd=$H1 insert_after=0x2071C declared=$H1,$H2,$H3 measured=$H1,$H3 nudge_ok=true"

[System.IO.File]::WriteAllLines("$dst\S1-grouped-chain.log",      ($base + @($LINKED,$SETTLED,$SKIPPED,$SKIPPED,$ABSENT)))
$bad  = "$P2[zorder-chain] settled nudged_hwnd=$H1 insert_after=0x2071C declared=$H1,$H2,$H3 measured=$H2,$H1,$H3 nudge_ok=true"
[System.IO.File]::WriteAllLines("$dst\S2-grouped-mismatch.log",   ($base + @($LINKED,$bad,$SKIPPED)))
$lf   = "$PE[zorder-chain] link-failed segment=g0 owned_hwnd=$H2 owner_hwnd=$H3 error=HRESULT(0x80070005)"
[System.IO.File]::WriteAllLines("$dst\S3-grouped-linkfailed.log", ($base + @($LINKED,$SETTLED,$lf)))
$liar = "$P[zorder-chain] linked segment=g0 owned=23v0 owner=25v0 owned_hwnd=$H1 owner_hwnd=$H3 pos=2/3"
[System.IO.File]::WriteAllLines("$dst\S4-grouped-poslie.log",     ($base + @($liar,$SETTLED,$SKIPPED)))
[System.IO.File]::WriteAllLines("$dst\S5-grouped-nolink.log",     ($base + @($SETTLED,$SKIPPED)))
$noapplied = @($base | Where-Object { -not $_.Contains('[zorder-group] applied') })
[System.IO.File]::WriteAllLines("$dst\S6-grouped-noapplied.log",  ($noapplied + @($LINKED,$SETTLED)))
$r1 = [System.IO.File]::ReadAllLines("$src\R1-default.log")
[System.IO.File]::WriteAllLines("$dst\S7-default-chainleak.log",  ($r1 + @($LINKED)))
# task 1.1 が受理行へ足した新しい綴り（相棒窓が供給されたとき `+bN` / `+sN` が付く）。
$newspell = '2026-08-29T06:00:59.235162Z DEBUG wintf::ecs::window::zorder_chain_diag: ' +
            '[zorder-group] applied action=set group_id=1 source=Tag members=b1,s1,b0,s0 normalized=1:false+s1,0:false+b0'
[System.IO.File]::WriteAllLines("$dst\S8-grouped-newspelling.log",($base + @($newspell,$LINKED,$SETTLED)))

# ---- ⑶ の量化子を露出させる標本（§5.1-3a の裁定を測るのはここ）----
# S1〜S8 はすべて `settled` が 1 本しかないので、量化子の違いを構造的に露出できない。
[System.IO.File]::WriteAllLines("$dst\S9-multi-good-then-bad.log",   ($base + @($LINKED,$SETTLED,$BAD,$BAD,$BAD)))
[System.IO.File]::WriteAllLines("$dst\S10-multi-all-good.log",       ($base + @($LINKED,$SETTLED,$SETTLED,$SETTLED)))
[System.IO.File]::WriteAllLines("$dst\S11-transient-then-settle.log",($base + @($LINKED,$SHORT,$BAD,$SETTLED)))
$GOOD2 = "$P2[zorder-chain] settled nudged_hwnd=$H2 insert_after=0x2071C declared=$H2,$H3 measured=$H2,$H3 nudge_ok=true"
$BAD2  = "$P2[zorder-chain] settled nudged_hwnd=$H2 insert_after=0x2071C declared=$H2,$H3 measured=$H3,$H2 nudge_ok=true"
[System.IO.File]::WriteAllLines("$dst\S12-two-lists-one-bad.log",    ($base + @($LINKED,$SETTLED,$SETTLED,$GOOD2,$BAD2)))

# ---- 宣言列が**縮む**過渡（窓が去る形）——⒝ でも赤になることを固定する標本 ----
# ⚠ S11 と混同しないこと。S11 は宣言列の字面が**動かない**まま不一致が後の一致で redeem
#    される形であり、こちらは窓の集合が動くので**その宣言列は二度と再来しない**。
$SHRUNK  = "$P2[zorder-chain] settled nudged_hwnd=$H1 insert_after=0x2071C declared=$H1,$H2 measured=$H1,$H2 nudge_ok=true"
$LINKED2 = "$P[zorder-chain] linked segment=g0 owned=23v0 owner=25v0 owned_hwnd=$H1 owner_hwnd=$H2 pos=1/2"
$DEPART  = "$P2[zorder-chain] unlinked segment=g0 owned=23v0 owned_hwnd=$H3 owner_hwnd=$H2 reason=Departing"
[System.IO.File]::WriteAllLines("$dst\S13-departing-shrink.log",     ($base + @($LINKED,$SETTLED,$SHORT,$DEPART,$LINKED2,$SHRUNK)))
```

### 6.1.1a 生成物の md5（2026-08-30・再現の錨）

生成は決定論的である。上のスクリプトを走らせて下表と md5 が一致しなければ、
**入力の一次証跡（`R1`/`R2`）か本番の行組立のどちらかが動いている**——先に原因を突き止めること。

| 標本 | サイズ | md5 |
|---|---|---|
| `S1-grouped-chain.log` | 77,265 bytes | `946978F4818D30CDCDAF714E5577B24D` |
| `S2-grouped-mismatch.log` | 76,998 bytes | `3E64885105C3D746A985EFEBAD4ED88C` |
| `S3-grouped-linkfailed.log` | 77,065 bytes | `6CEA9149130A7BC424FF7C8B2303A250` |
| `S4-grouped-poslie.log` | 76,998 bytes | `E6FAA9CAB4F8887CDCD104328CCDC580` |
| `S5-grouped-nolink.log` | 76,801 bytes | `DA1D55DB142E9060BB42D9220461568F` |
| `S6-grouped-noapplied.log` | 76,696 bytes | `47B8DA50A4501F86BE3F92FD1866AE47` |
| `S7-default-chainleak.log` | 79,118 bytes | `B39458B40881910729190955DB254B91` |
| `S8-grouped-newspelling.log` | 77,050 bytes | `F6DE306AF11E5633CDB710EE9304DCB5` |
| `S9-multi-good-then-bad.log` | 77,617 bytes | `059819020E293F5AF87690C1B989A2C3` |
| `S10-multi-all-good.log` | 77,367 bytes | `B75E310F631B7B189D7B13F163AE9C0B` |
| `S11-transient-then-settle.log` | 77,357 bytes | `44CBC335EEFC03EF017F2DEAA62BA7FB` |
| `S12-two-lists-one-bad.log` | 77,577 bytes | `9E19FF18251A7EC02311E679D440979D` |
| `S13-departing-shrink.log` | 77,731 bytes | `10359866755C423B81407A00F6A22A61` |

レビュアーが独立に作った検体（`%LOCALAPPDATA%\areka-diag\rv61-chain-calibration\` および
`rv61b-chain-calibration\`）:

| 標本 | サイズ | md5 |
|---|---|---|
| `MD-good-masks-bad.log` | 77,617 bytes | `059819020E293F5AF87690C1B989A2C3` |
| `MH-perm-decl.log` | 77,117 bytes | `95571613B25C46284BF18B7835716359` |
| `RVA-departing-transient.log` | 77,731 bytes | `825B3CBACB4ECE64D3F76879E3130AC6` |
| `RVB-episode-masking.log` | 77,760 bytes | `4A83DAB2097E30943A45041728781E49` |

> **`MD-good-masks-bad.log` と `S9-multi-good-then-bad.log` は md5 が同一である**
> （`0598…A2C3`）。レビュアーと実装者が別々に組んだ標本がバイト一致したということであり、
> §6.1.1 の作り方（一次証跡＋兄弟テストの逐語期待値）が**再現可能な手順として働いている**
> ことの偶然でない証跡になる。

### 6.1.2 較正の結果（25 通り・4 種類の終了コードすべてが出た）

| # | 当てたログ | `-Mode` | 何を確かめるか | 実測 |
|---|---|---|---|---|
| 1 | `S1-grouped-chain.log` | `grouped` | 成立形が緑になる | **J1=PASS J3=PASS / exit 0** |
| 2 | `S2-grouped-mismatch.log` | `grouped` | 収まった行の宣言と実測の食い違いで赤 | **J1=FAIL J3=PASS / exit 1** |
| 3 | `S3-grouped-linkfailed.log` | `grouped` | 張り失敗が 1 件でも赤 | **J1=FAIL J3=PASS / exit 1** |
| 4 | `S4-grouped-poslie.log` | `grouped` | 繋いだ行が宣言の並びと食い違えば赤（位置を偽る変異） | **J1=FAIL J3=PASS / exit 1** |
| 5 | `S5-grouped-nolink.log` | `grouped` | 収まっただけで 1 本も繋いでいなければ赤 | **J1=FAIL J3=PASS / exit 1** |
| 6 | `S6-grouped-noapplied.log` | `grouped` | 受理が無ければ FAIL ではなく**判定不能** | **J1=INCONCLUSIVE J3=PASS / exit 2** |
| 7 | `S7-default-chainleak.log` | `default` | **既定走行に鎖の記録が 1 行混じれば赤**（初版の道具はここを緑で素通りした） | **J2=FAIL J3=PASS / exit 1** |
| 8 | `S8-grouped-newspelling.log` | `grouped` | task 1.1 の新しい綴り `normalized=…+s1` で壊れず、`source=Tag` も当たる | **J1=PASS J3=PASS / exit 0**（受理 2 件＝Descript 1 / Tag 1） |
| **9** | `S9-multi-good-then-bad.log`（一致 1＋不一致 3） | `grouped` | **⑶ の量化子——「1 本でも一致すれば緑」なら通ってしまう形が赤になること** | **J1=FAIL J3=PASS / exit 1**（`終状態…宣言列 1 本中 一致 0 / 不一致 1`） |
| **10** | `S10-multi-all-good.log`（一致のみ 3 本） | `grouped` | **9 の対照——複数本でも全部一致なら緑のまま**（恒真な赤にしていない） | **J1=PASS J3=PASS / exit 0** |
| **11** | `S11-transient-then-settle.log`（**宣言列の字面は不変**のまま 不一致→不一致→一致） | `grouped` | **同一宣言列の中で過渡の不一致が後の一致で redeem される形が緑になること**（＝§5.1-3a の経路 2a だけ。窓の集合は動いていない） | **J1=PASS J3=PASS / exit 0** |
| **12** | `S12-two-lists-one-bad.log`（宣言列 2 本・片方だけ終状態が不一致） | `grouped` | **全称が宣言列ごとに効くこと**（他の列が揃っていても赤） | **J1=FAIL J3=PASS / exit 1**（`宣言列 2 本中 一致 1 / 不一致 1`） |
| **13** | `S13-departing-shrink.log`（窓が去って**宣言列が縮む**過渡） | `grouped` | **⒝ でも赤になること**——短い実測の宣言列は二度と再来しないので終状態のまま残る（§5.1-3a の経路 1・2b） | **J1=FAIL J3=PASS / exit 1**（`⚠ 終状態が宣言どおりでない宣言列: 宣言=…,0x4DC110E 最後の実測=0x4880F82,0x4DC110E`） |
| 14 | `S1-grouped-chain.log` | `default` | J2 の「0 件」の非 0 対照（同じ道具・同じ語で赤が出る） | **J2=FAIL J3=PASS / exit 1** |
| 15 | `R1-default.log`（実走・素） | `default` | **据え置いた語が実走ログに当たり続ける** | **J2=PASS J3=PASS / exit 0** |
| 16 | `R1-default.log`（実走・素） | `grouped` | 受理が無い実走は判定不能のまま | **J1=INCONCLUSIVE J3=PASS / exit 2** |
| 17 | `R5-rejected.log`（実走） | `default` | **据え置いた `rejected` 1 語だけで非 0 が出る** | **J2=FAIL J3=PASS / exit 1** |
| 18 | `mutant-no-measured_prev.log` | `default` | J3 の欄立て照合が生きている | **J3=FAIL / exit 1** |
| 19 | `mutant-rename-char_entity.log` | `default` | J3 が欄名の改名を捕まえる | **J3=FAIL / exit 1** |
| 20 | `mutant-value-garbage.log` | `default` | J3 は欄の**値**までは見ていない（効かない範囲の確認） | **J3=PASS / exit 0** |
| 21 | 存在しないパス | `default` | 道具の失敗は判定の失敗と別のコード | **exit 3** |
| **22** | `MD-good-masks-bad.log`（**レビュアー作成**・一致 1＋不一致 3） | `grouped` | レビュー round 1 の反例が赤になること | **J1=FAIL J3=PASS / exit 1** |
| **23** | `MH-perm-decl.log`（**レビュアー作成**・宣言列 2 本） | `grouped` | 同上（無関係な一致 `settled` が不一致を隠さないこと） | **J1=FAIL J3=PASS / exit 1** |
| **24** | `RVA-departing-transient.log`（**レビュアー作成**・宣言列が縮む過渡） | `grouped` | 13 と同型を独立の検体で（**⒝ が経路 1・2b を救わない**ことの裏取り） | **J1=FAIL J3=PASS / exit 1** |
| **25** | `RVB-episode-masking.log`（**レビュアー作成**・`Teardown` を挟んで同じ字面で再構成） | `grouped` | **区切りを使っていないので、ほどける前の不一致が組み直し後の一致に隠れる**（§5.1-3a の末尾に登記した既知の緩さ） | **J1=PASS J3=PASS / exit 0** |

- **7 行目が本改訂で最も重要な赤である。** 差し替え前の道具に同じログを当てると **J2=PASS / exit 0** で
  素通りした——`[zorder-chain]` を 1 語も知らないので「グループ系の記録が 1 件も無い」と読んだためである。
  **沈黙が合格に見える形**であり、これを潰すのが J2 を両方の冠へ広げた理由である。
- **15〜20 行目が「据え置く語を触っていない」ことの証跡である。** 実走ログ（初版の一次証跡）へ
  当てて、受理・拒否・ペア系 4 本・欄立ての 5 欄・終了コードの体系がすべて従来どおり当たる。
  内訳＝**15〜17 行目が実走ログ**（`R1` の 2 モードと `R5`）・**18〜20 行目が初版の変異体**で、
  変異体はそのまま流用したので **初版の較正と逐語で同じ結果**が出ている。
  （⚠ 2026-08-30 訂正: この註は 16 行版の行番号のままだった。10・12・13・14 行目は合成標本であり、
  実走ログではない。表を増やす task は**この註の行番号も振り直すこと**。）
- **4 行目・5 行目が「繋いだ本数」の判定が恒真でないことを示す。** 繋ぎが宣言の並びと噛み合わない
  変異と、繋ぎが 1 本も無い変異の**両方**で赤が出る。
- **9〜13 行目と 22〜25 行目が ⑶ の量化子の較正である**（レビュー round 1 の FINDING 1・2、
  および round 2 の FINDING 1・2 への応答）。
  改訂前の存在量化（「1 本でも一致すれば緑」）では **9・22・23 がいずれも exit 0 で素通り**した
  ——道具自身が「収まった本数: 4 件（うち一致 1）」と印字しながら PASS を出す形だった。
  **10・11 は逆向きの対照**であり、終状態の全称が「常に赤い門」になっていないことを示す。
  **12 は全称の適用単位が宣言列ごとであること**を示す。
- **⚠ 11 と 13 を取り違えないこと。両者の違いが ⒝ の射程そのものである。**
  11（緑）は**宣言列の字面が動かない**まま不一致が後の一致で redeem される形＝⒝ が吸収する唯一の形。
  13（赤）は窓が去って**宣言列が縮む**形で、短い実測の宣言列が二度と再来しないので ⒝ でも救われない。
  **「窓の出入りの過渡なら自動的に許される」わけではない**（§5.1-3a の経路表）。
  初版の本表はこの 2 つを区別せず、11 に「経路 1・2 を殺さない」と書いていた——**測っていない
  ことを書いていた**ので、round 2 で 11 の説明を実測どおりに縮め、13 を足した。
- **25 行目（`RVB`）が既知の緩さである。** `unlinked reason=Teardown` という区切りがログに在るのに
  使っていないため、ほどける前の不一致が組み直し後の一致に隠れて緑になる。
  終状態が正しいので語義上は妥当だが、緩さとして §5.1-3a の末尾に登記した。
- **22〜25 はレビュアーが独立に作った検体である。** 自分で作った標本だけで較正すると、
  自分が思いつかなかった穴は標本にも現れない——**穴を見つけた側の検体をそのまま台帳へ入れる**。
  所在は `%LOCALAPPDATA%\areka-diag\rv61-chain-calibration\`（22・23）と
  `rv61b-chain-calibration\`（24・25）。md5 は §6.1.1a。
- **`Get-SettledLine` の 4 欄の較正**（round 2 の FINDING 3）: `nudge_ok=` 有り／無しの 2 本を
  当てて `nudged_hwnd`／`insert_after`／`declared`／`measured` の 4 欄が**どちらでも正しく取れる**
  ことを確かめた。`$Matches` は `-match` のたびに丸ごと差し替わるので、4 欄を先にローカルへ
  退避してから `nudge_ok` を評価する。**この 2 欄は判定に出ないので、較正と印字が無いと
  静かに壊れる**——実際 round 2 の時点で `nudged_hwnd` に `true` が入り `insert_after` が空だった。
  是正後は判定器の出力に 4 欄すべてが載る（`settled 後押し=… 錨=… 宣言=… 実測=…`）。

## 6.2 実走ログでの較正（2026-08-30・task 6.2・**合成ではない標本**）

§6.1 の 25 通りは**合成標本**が主である（鎖の語彙で実際に走ったログが当時まだ無かったため）。
task 6.2 の 8 走行でその欠を埋めた——**同じ道具を実走ログ 11 通りに当てて、3 種類の終了コードが出た**。
表と逐語は `real-machine-signoff.md` **§9** にある。要点だけを引く:

- **合成標本の期待が実走で裏返らなかった**——成立形は緑（exit 0）、既定走行に鎖の記録が無いことは緑、
  グループ走行を `-Mode default` で測ると赤（exit 1・§2.4 の対照）、受理の無い走行は判定不能（exit 2）。
- **⒝（宣言列ごとの終状態の全称）が実走で複数の宣言列に効いた初めての例**が R8 である
  （1 走行の中で鎖を逆順へ組み替え、`宣言列 2 本中 一致 2 / 不一致 0`）。
  §6.1.2 の 9〜13・22〜25 行目は合成標本でこの量化子を測っていたが、**実走で 2 本以上の宣言列が
  出たのはここが最初**である。
- **終状態の不一致は 1 件も出なかった。** よって §5.1-3a への例外の登記は**行っていない**
  （登記すべき事象が起きなかった）。

---

## 7. 走行時の申し送り

> ⚠ **2026-08-30（task 6.1）: 以下の申し送りのうち、初版の是正モデル固有のものは退役した。**
> `PairFixThisPass`／`MemberMissing`／`AlreadyOrdered`／`GaveUpAfterFailures` はいずれも
> 鎖の下では出ない語である。本版で「異常ではない記録」として見込むのは、次の鎖の 3 つに置き換わる。

- **`[zorder-chain] skipped reason=NoChange`／`reason=TooFewPresent`／`reason=HandleMissing`** は
  見送りの記録であり、異常ではない。`NoChange` は望む鎖が前回と同じで出す操作が 1 つも無い巡、
  `TooFewPresent` は実在する窓が 2 枚未満の巡、`HandleMissing` は窓ハンドルがまだ取れていない巡である。
  **⚠ 件数を判定に使ってはならない**（連呼の抑止が真偽値 1 枚なので取りこぼしうる・tasks.md 申し送り 2.3）。
- **`[zorder-chain] absent group_id=N element=b1` が出ることがある。** 宣言された要素の窓がまだ生まれて
  いない／既に壊れているときの記録であり、異常ではない（要件 1.4——存在する窓だけで鎖を組み、
  グループからは取り除かない）。同じ内容が続く間は 1 度だけ出る。
- **`[zorder-chain] unlinked reason=Departing`** が走行終了時に出る（窓が去る巡）。異常ではない。
  > **⚠ 2026-08-30（task 6.2）の訂正: 実際には出なかった。** 8 走行のログで `reason=Departing` は
  > **0 件**である（実出した `unlinked` は 2 件でどちらも `reason=Teardown`＝解除・組み替えの巡）。
  > 有界終了は 4 枚を**一斉に** despawn するので（`areka: smoke 自動 close … count=4`）、鎖の相が走る頃には
  > 帳簿の component ごと消えており、切離しの対象が残らない。終了時に実際に出るのは
  > **`[zorder-chain] absent` × 宣言要素数 ＋ `skipped reason=NoChange`** である。どちらも異常ではない。
  > **`Departing` を「出るはず」の前提にして待たないこと。**
- **数値モード（`seriko.zorder,0,1` 等）は初版（毎巡の観測＋是正）の実機で成立しなかった。**
  **本 spec の改訂そのものがその不成立への応答であり、鎖の下で成立するかを確かめるのが task 6.2 である**
  （task 6.2 が「数値モードと明示モードの両方で窓 4 枚」を求めているのはこのため）。
  詳細と切り分けは `real-machine-signoff.md` §4。**本版で J1 を測るときも、成立する形（例 `b0,s0,s1`）と
  数値モードの両方を走らせて対比を残すこと**——初版が落ちたのはまさに数値モードの 4 枚だからである。
  > **✅ 2026-08-30（task 6.2）の実測: 鎖の下では成立した。** 数値 `0,1`（R3）・明示 `b0,s0,b1,s1`（R2）の
  > **両方**で `declared` と `measured` が同一行で一致し、`link-failed` は 0 件だった
  > （`real-machine-signoff.md` §8.2）。初版の `verify-failed` 24 件・`fix` 0 件という形は再現しない。
  > 以後この項は**歴史の記録**として読むこと。
- **初版の欠陥の根因候補 2 つ（`real-machine-signoff.md` §4.3）と、その切り分け手順は退役した。**
  切り分けの対象だった本番ファイル `zorder_group_maintain.rs` は本 spec の改訂で退役しており、
  もはや存在しない。根因がどちらであっても答えは同じ「毎巡の観測＋是正をやめる」であり、
  それが本改訂そのものである（design DD-1）。**残っているのは実測で確かめる仕事だけで、それが 6.2 である。**
- **`cargo test --workspace` の既知間欠赤**（`areka-P0-test-cage-determinism` 所有）は本判定と因果独立。
