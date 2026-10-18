# 実機の確かめの手順（areka-P0-zorder-scope-inversion）

対象要件: requirements.md **8.4・8.5・8.6**（あわせて 5.1〜5.4 の実機の証跡）
設計の正本: design.md **Testing Strategy →「実機の確かめ（要件 8.4〜8.6）」**・Risks の **R3**

本書は task 5.1 の成果物であり、判定の手順・検体の作り方・判定の道具の較正の記録である。
実走（task 5.2）の結果は本書の §7 へ追記する。

道具は本書と同じフォルダの 3 本である（どれも PowerShell 7・端末へ出す文は ASCII だけ）。

| 道具 | 役目 |
|---|---|
| `signoff-run.ps1` | 検体の派生（`-Prepare`）と、有界の時間で自動で終わる走行・前面走査の採取・押下（`-Run`／`-Press`） |
| `signoff-judge.ps1` | 走行のログと前面走査の控えを読んで判定する（終了コード 0／1／2／3） |
| `signoff-calibrate.ps1` | 判定の道具を既存の実走のログと作ったログへ当て、合格と不合格の両方を正しく返すことを確かめる |

---

## 1. 判定の全体像

同じ回に 4 種類の走行を取る（要件 8.6 の対照を同じ回に揃える）。

| 種類（`-Case`） | 起動の台本へ差し込む指定 | 何を確かめるか | 要件 |
|---|---|---|---|
| `inverted` | `\![reset,zorder]\![set,zorder,s0,b0,s1]`（ぱすたと同じ） | 手前から s0 → b0 → s1 の重なり。押した後・掴んで動かした後・他のアプリから戻った後も崩れない | 8.4・8.5（5.1〜5.4） |
| `explicit` | `\![reset,zorder]\![set,zorder,b0,s0,b1,s1]`（逆転の無い明示の記法） | 本機能の前と同じ組の並び | 8.6 |
| `numeric` | `\![reset,zorder]\![set,zorder,0,1]`（数字だけの記法） | 本機能の前と同じ組の並び | 8.6 |
| `default` | なし | 受理も鎖も 1 行も出ない・各組のバルーンがキャラクターのすぐ手前 | 8.6 |

判定の道具が下す項目:

| 項目 | 対象 | 合格の条件 |
|---|---|---|
| **A** 受理 | `inverted`／`explicit`／`numeric` | `[zorder-group] applied` の `action=set` の行が、種類ごとの字面（§2）を持つ。`normalized=` 欄に旧語 `N:true` が 1 つも無い（要件 1.4）。`[zorder-group] rejected` が 0 件 |
| **R** 組の外し | 同上 | `inverted`: `[zorder-chain] pair-released` が 1 件以上あり、外した組のバルーン b0 とキャラクター s0 の窓が引ける。`explicit`／`numeric`: 0 件（組は隣り合うので外さない＝本機能の前と同じ） |
| **L** 繋ぎ | 同上 | `[zorder-chain] linked` が 1 件以上あり、全行が収まった行の宣言列の「pos 番目 → pos+1 番目」に当たる |
| **S** 収まり | 同上 | `[zorder-chain] settled` の**宣言の列と実測の列が同じ**こと（宣言列ごとの最後の行で判定・過渡は通す）。`link-failed`／`unlink-failed`／`link-refused` が 0 件。正の宣言列（一致した行のうち最長）が 3 枚以上。`inverted` では正の宣言列の頭 2 つが s0・b0（手前から s0 → b0 → … → s1・要件 8.4） |
| **D** 指定なし | `default` | `[zorder-group]` と `[zorder-chain]` の行が**どちらも** 0 件 |
| **P** 前面走査 | `inverted` は必須・他は任意 | 各場面で、正の宣言列の窓が宣言の順に並び、その間に他のアプリの窓が挟まらない（塊のまま）。`inverted` では s0 と b0 が両方見えていて s0 が手前（要件 5.2）。場面 `boot`・`press-s0`・`press-b0`・`drag-s0`・`other-app`・`returned` が揃っていること。`default` では各組のバルーンがキャラクターのすぐ手前 |

終了コード: **0** = すべて PASS／**1** = いずれかが FAIL／**2** = 判定不能（材料が無い: `[zorder-pair]` が 1 行も無い・受理が無い・前面走査が無いか場面が欠ける）／**3** = 引数不正。

目視は判定の道具の外で行い（§4.3）、結果を §7 へ書く。

## 2. 判定に使う語（冠込み・逐語）

語は**必ず冠（角括弧）込み**で照合する。走行のログは作業ツリーの絶対パスや設定を何行も吐くので、冠を外すと巻き込む。

| 語 | 出典 |
|---|---|
| `[zorder-group] applied`・`[zorder-group] rejected` | `crates/wintf/src/ecs/window/zorder_chain_diag.rs` の `APPLIED_TAG`・`REJECTED_TAG` |
| `[zorder-chain] linked`・`settled`・`pair-released`・`link-failed`・`unlink-failed`・`link-refused` | 同ファイルの `LINKED_TAG` ほか |
| `[zorder-pair]`・`[zorder-pair] owner-established` | `crates/wintf/src/ecs/window/zorder_pair_diag.rs` |

- `[zorder-chain] linked` は `[zorder-chain] unlinked` に当たらない（`] linked` の区切りが別物）。較正の標本 `inv-good.log` は `unlinked` の行を含めて、これを確かめている。
- 受理の行の字面（`members=` と `normalized=` は隣り合う。出典は兄弟テスト）:

| 種類 | 照合する字面 | 出典 |
|---|---|---|
| `inverted` | `members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1` | `zorder_drain_written_order_tests.rs`（`s0,b0,s1` の受理の行） |
| `explicit` | `members=b0,s0,b1,s1 normalized=0:false,1:false` | 同（隣り合いは `false`・補い無し） |
| `numeric` | `members=b0,s0,b1,s1 normalized=-` | `zorder_drain_tests.rs` の `t_zdr04`（数字だけは番兵）・`t_zdw08`（展開は `b0,s0,b1,s1`） |

- `settled` の 4 欄（`nudged_hwnd`・`insert_after`・`declared`・`measured`）は兄弟テストが隣接を固定している。判定は `declared=` と `measured=` の 2 欄だけを読む。
- `pair-released` の `owned_hwnd` はバルーン b0、`owner_hwnd` はキャラクター s0（`zorder_chain_apply.rs` の組の外し）。結び目がまだ張られていなかった組では `owner_hwnd=-` になるので、そのときは同じ実体（`peer=`）を名指す `linked` の行から s0 の窓を引く（較正の標本 `inv-untied.log`）。

## 3. 検体の用意（**再生成が正本・コミットしない**）

検体は既存の検体ゴースト emo2（`vendors\sample_ghost\emo2.nar`）から `signoff-run.ps1 -Prepare` が機械的に作る。
置き場は**ワークツリーの `target\zorder-inversion-signoff\` の下だけ**である（根・検体・一時フォルダ・書ける根のすべて）。

```powershell
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Prepare
```

`-Prepare` がすること:

1. `cargo build -p areka --bin areka`・`cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`（32bit であることを PE の見出しで確かめる）
2. `nar-sample-path emo2` で展開した emo2 を、種類ごとに `target\zorder-inversion-signoff\<種類>\root\` へ写し、`areka.exe` と補助プロセスを隣に置く
3. ゴースト自身のトーク（`dic\dispatch.pasta`・`dic\hour.pasta`）を外し、窓の出入りを起動の場面と操作の結果だけにする
4. `default` 以外は `dic\boot.pasta` の起動の各場面（`＊起動朝`〜`＊起動halt`・`＊OnFirstBoot`。`＊OnClose` より前）の**最初のむらさきの発話の頭**へ §1 の指定を差し込む。時刻帯にも前回の終わり方にも依らず指定が出るよう全部の場面へ入れる（16 場面。12 未満なら止まる）

作り直すたびに `<種類>\` を丸ごと消してから作るので、前の走行の記憶は残らない。

## 4. 走行（task 5.2）

### 4.1 前提

- 入力デスクトップが `Default`（画面がロックやスクリーンセーバーでない）こと。そうでなければ道具は操作せずに止まる。
- `-Press` は **SendInput でポインタを動かして押す**。開発者の許可を得てから、**走行の間だけ**使う。走行中は机の上の操作をしない。
- 走行は `RUST_LOG=info,wintf::ecs::window=debug`（`applied`・`linked`・`settled`・`pair-released` は debug）・`AREKA_APP_SMOKE_EXIT_MS`（有界の自動終了）・ゴーストは絶対パスで起こす。書ける根（`%LOCALAPPDATA%`）と一時フォルダは子プロセスだけ `target\zorder-inversion-signoff\<種類>\` の下へ向ける。

### 4.2 走行のコマンド

```powershell
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run inverted -Press
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run explicit
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run numeric
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run default
```

所要は `-Press` 付きが約 90 秒、他は約 30 秒（`-ExitMs` で変えられる）。採れるもの:

| ファイル（`target\zorder-inversion-signoff\<種類>\`） | 中身 |
|---|---|
| `run.log` | 走行のログ（判定の材料） |
| `probe.txt` | 前面走査の控え。1 行 1 場面: `probe label=<場面> order=<tok>,... foreground=0x...` |
| `steps.txt` | 操作の控え（押した点・その後の前面の窓・止まった理由） |

前面走査は `GetTopWindow` から `GW_HWNDNEXT` を辿った**手前から奥の順**で、見えている窓だけを拾う。areka の窓は `0xHEX`（ログの窓の書式と同じ）、それ以外の窓の連なりは `*` 1 つに畳む。

`-Press` の場面の順（各操作の後に前面走査を採る）:

| 場面 | 操作 |
|---|---|
| `boot` | 起動して `[zorder-chain] settled` が出てから 3 秒後（`-Press` なしの走行もこの 1 場面を採る） |
| `press-s0` | s0（逆転したスコープのキャラクター）を押す |
| `press-b0` | b0（そのバルーン）を押す |
| `drag-s0` | s0 を掴んで左へ 60 画素動かして離す |
| `other-app` | 道具が出す別プロセスの窓（他のアプリの代わり）を押す |
| `returned` | s1 を押してゴーストへ戻る |

- 押す窓はログから採る: b0 は `pair-released` の `owned_hwnd`、s0 は正の宣言列の頭、s1 は末尾（判定の道具が §2 の方法で引き直して照合する）。
- 窓は透過の画素で押下が下の窓へ抜けるので、前面の窓が狙った窓になるまで押す点を替える（6 点）。届かなければ止まって `steps.txt` に理由を残す。

### 4.3 目視（要件 8.4）

`inverted` の走行の `boot` の場面で、手前から むらさき（s0）→ むらさきのバルーン（b0）→ エモ（s1）の重なりになっていることを目で確かめ、§7 へ書く。
`press-s0` の後にむらさきのバルーンがむらさきより手前へ出ていないことも見る（要件 5.2）。

### 4.4 止める条件（設計の見直しの条件・R3）

`press-s0`／`press-b0`／`returned` の場面で、ゴーストの窓の間に他のアプリの窓が挟まる（判定 P の `another app window sits inside the chain`）か、並びが崩れる（`order differs from declared`）なら、**塊で上がっていない**。そこで止めて報告する（design.md Revalidation Triggers: 要件 5 の作りの見直し）。

## 5. 判定のコマンド

```powershell
$s = 'C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\target\zorder-inversion-signoff'
$j = 'C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-judge.ps1'
pwsh -NoProfile -File $j -Log "$s\inverted\run.log" -Case inverted -Probe "$s\inverted\probe.txt"; "exit=$LASTEXITCODE"
pwsh -NoProfile -File $j -Log "$s\explicit\run.log" -Case explicit -Probe "$s\explicit\probe.txt"; "exit=$LASTEXITCODE"
pwsh -NoProfile -File $j -Log "$s\numeric\run.log"  -Case numeric  -Probe "$s\numeric\probe.txt";  "exit=$LASTEXITCODE"
pwsh -NoProfile -File $j -Log "$s\default\run.log"  -Case default  -Probe "$s\default\probe.txt";  "exit=$LASTEXITCODE"
```

4 本とも **exit=0** で task 5.2 の判定の側が成立する。判定は終了コードで取る（`| tail` 等で包まない）。
ログは再採取の効かない一次証跡なので、判定の前に `Get-FileHash -Algorithm MD5` を控えて §7 へ書く。

## 6. 判定の道具の較正（task 5.1・2026-10-19）

判定の道具が「常に緑」でも「常に赤」でもないことを、実走の前に確かめる。

### 6.1 標本の作り方（**再生成が正本**）

- **既存の実走のログ**: 指定の無い emo2 の実走ログ（`RUST_LOG` に `wintf::ecs::window=debug` を含む・`[zorder-pair] owner-established` が 2 行・`[zorder-group]`／`[zorder-chain]` が 0 行）。既定は本書と同じフォルダの `signoff-real-default.log`（コミット済み）で、task 5.2 の `default` の走行の `run.log` の写し（2026-10-19・214 行・md5 `5F1CF6AA6DE98D3E05D230C2964D6B21`）である。別の実走ログは `-RealLog` に渡して当てられる。task 5.1 の初回の較正は `areka-P0-anchor-tag-canon` の実走ログ `run-Box.log`（2026-10-16・359 行・md5 `484B262DC6921399DD1551463BE5ADEA`・この機械の `target\` にしか無い）で行い、同じ 25 通り 0 食い違いだった。
- **作ったログ**: 実走のログをそのまま頭に置き、その後ろへ本番の行の組立（`zorder_chain_diag.rs` の書式・`zorder_drain.rs` の受理の本文。どちらも兄弟テストが逐語で固定）で組んだ行を足す。窓のハンドルは実走のログの `owner-established` の 2 行から取る（手で決めた値を使わない）。前面走査の控えも同じハンドルで組む。

```powershell
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-calibrate.ps1; "exit=$LASTEXITCODE"
```

道具は標本を作り、25 通りに判定の道具を当てて期待の終了コードと比べ、全部合えば exit 0 を返す。

### 6.2 較正の結果（25 通り・0 食い違い・4 種類の終了コードすべてが出た）

| # | ログ | 種類 | 前面走査 | 確かめること | 期待 | 実測 |
|---|---|---|---|---|---|---|
| 1 | `real.log`（実走） | default | なし | 指定なしの実走が緑 | 0 | 0 |
| 2 | `real.log`（実走） | default | `probe-pairs-good.txt` | 組ごとにバルーンがキャラクターのすぐ手前なら緑 | 0 | 0 |
| 3 | `real.log`（実走） | default | `probe-pairs-flip.txt` | キャラクターがバルーンより手前なら赤 | 1 | 1 |
| 4 | `real.log`（実走） | inverted | `probe-good.txt` | 逆転の記録の無いログは赤 | 1 | 1 |
| 5 | `real.log`（実走） | explicit | なし | 鎖の記録の無いログは赤 | 1 | 1 |
| 6 | `def-chainleak.log` | default | なし | 指定なしに鎖の行が 1 行混じれば赤 | 1 | 1 |
| 7 | `inv-good.log` | inverted | `probe-good.txt` | ぱすたと同じ指定・過渡の不一致の後に収まる・`unlinked` 混じりで緑 | 0 | 0 |
| 8 | `inv-untied.log` | inverted | `probe-good.txt` | `owner_hwnd=-` の組の外しでも s0 を `linked` から引いて緑 | 0 | 0 |
| 9 | `inv-good.log` | inverted | `probe-press-flip.txt` | 押した後に b0 が s0 より手前なら赤 | 1 | 1 |
| 10 | `inv-good.log` | inverted | `probe-split.txt` | 他のアプリの窓が並びの間に挟まれば赤（塊で上がらない） | 1 | 1 |
| 11 | `inv-good.log` | inverted | `probe-missing.txt` | 場面が欠ければ判定不能 | 2 | 2 |
| 12 | `inv-good.log` | inverted | なし | 前面走査が無ければ判定不能 | 2 | 2 |
| 13 | `inv-oldword.log` | inverted | `probe-good.txt` | 旧語 `0:true` なら赤 | 1 | 1 |
| 14 | `inv-noreleased.log` | inverted | `probe-good.txt` | 組の外しが無ければ赤 | 1 | 1 |
| 15 | `inv-unsettled.log` | inverted | `probe-good.txt` | 最後の `settled` の宣言と実測が違えば赤 | 1 | 1 |
| 16 | `inv-refused.log` | inverted | `probe-good.txt` | `link-refused` が 1 行でもあれば赤 | 1 | 1 |
| 17 | `inv-noapplied.log` | inverted | `probe-good.txt` | 受理が無ければ判定不能 | 2 | 2 |
| 18 | `inv-pairorder.log` | inverted | なし | 逆転の語で受理したのに組の並びに収まっていれば、前面走査の前に赤 | 1 | 1 |
| 19 | `exp-good.log` | explicit | なし | 逆転の無い明示の記法の組の並びが緑 | 0 | 0 |
| 20 | `exp-released.log` | explicit | なし | 逆転が無いのに組を外していれば赤 | 1 | 1 |
| 21 | `num-good.log` | numeric | なし | 数字だけの記法の組の並びが緑 | 0 | 0 |
| 22 | `num-wrongword.log` | numeric | なし | 数字だけなのに `normalized=` に語が載れば赤 | 1 | 1 |
| 23 | `inv-good.log` | default | なし | 指定のあるログを指定なしとして読めば赤 | 1 | 1 |
| 24 | `nopair.log` | default | なし | `[zorder-pair]` が 1 行も無ければ判定不能 | 2 | 2 |
| 25 | `missing.log`（無い） | default | なし | 道具の失敗は判定の失敗と別のコード | 3 | 3 |

変異の確かめ（判定の道具を控えから書き換えて較正を回し、戻した）:

- 前面走査の「他のアプリの窓が挟まらない」の検査を外す → 10 行目が exit 0 になり食い違い 1。
- `inverted` の「正の宣言列の頭 2 つが s0・b0」の検査を外す → 18 行目が exit 2 になり食い違い 1。
- 戻した後は 25 通り 0 食い違い。

### 6.3 標本の md5（2026-10-19・task 5.2 の後に既定の実走ログを差し替えて取り直した・再現の錨）

生成は決定論である。`signoff-real-default.log` が同じなら下表と一致する。一致しなければ、本番の行の書式か受理の本文が動いている。

| 標本 | md5 |
|---|---|
| `signoff-real-default.log`・`real.log` | `5F1CF6AA6DE98D3E05D230C2964D6B21` |
| `inv-good.log` | `F9236DB721CFF3104418956D9FF422DE` |
| `inv-untied.log` | `6ADDC372AA513384FEDB3934C44F1955` |
| `inv-oldword.log` | `0BB1BCB177D651BE939838DE2E5DBD1E` |
| `inv-noreleased.log` | `E70CFDE7C114437A80C07662131B12DF` |
| `inv-unsettled.log` | `34D220D4291BCFC5F2D4D7828800309F` |
| `inv-refused.log` | `9931D2B1DEF77CF7CB25DA149EE04CC8` |
| `inv-noapplied.log` | `F270423A512B1A08CB12E4BB800236AD` |
| `inv-pairorder.log` | `B398771CDB23DB6373B355AA83C14B4A` |
| `exp-good.log`・`num-wrongword.log`（同じ中身） | `183A501A30047A13BC74848F9AFB6349` |
| `exp-released.log` | `2F3771F255494A49AA72227DA612FA4E` |
| `num-good.log` | `0394F4AD182B94F108659DF60F66CE43` |
| `def-chainleak.log` | `3BB9AA205D355B486CFE42D353A9005E` |
| `nopair.log` | `DAA54E8E57FA114919D4ED4A676EF22B` |
| `probe-good.txt` | `76FF712BE5F1DA8D611A01D77CD85C24` |
| `probe-press-flip.txt` | `1233CF81D1CFD2C679D2FADC9DFF8B1C` |
| `probe-split.txt` | `266C99654D7DD6DB324CF30F052FA38C` |
| `probe-missing.txt` | `688131CFA6B9C3AC6EDC20802605D3CE` |
| `probe-pairs-good.txt` | `AC1683FF41EBF544092A132926782BA3` |
| `probe-pairs-flip.txt` | `72CB8307FD382ED03955447D764ED960` |

### 6.4 判定の道具が見ないこと（意図的）

- `linked` の**被覆**（宣言された繋ぎが全部出たか）は見ない。期待の本数はスコープの組の構造で決まり、記録からは導けない。被覆は決定論のテスト（要件 8.2 の対応表）が担う。
- `settled` の `nudge_ok=` は判定に使わない。後押しが失敗しても並びが宣言どおりなら成立しており、崩れたなら S が終状態で捕まえる。
- 前面走査は見えている窓だけを読む（不可視の補助窓を読まない・R2）。`default` の前面走査は組の隣り合いだけを見る（鎖が無いのでスコープの間の順は決まりの外）。

## 7. 実走の記録（task 5.2・2026-10-19）

### 7.1 走らせたもの

入力デスクトップは `Default`（`steps.txt` の頭の行 `desktop=Default`）。押下（SendInput）は開発者の許可を得て `-Press` の走行の間だけ使った。4 本を同じ回に順に走らせた。

| # | コマンド | 走行の終了コード | 時刻（開始 → 終了） |
|---|---|---|---|
| 0 | `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Prepare` | 0（4 種類とも `areka=8664`・`helper=014C`・指定入りは `tagged_scenes=16`） | — |
| 1 | `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run inverted -Press` | 0（`inverted exit=0 ok=True`） | 07:34:54 → 07:36:26 |
| 2 | `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run explicit` | 0（`explicit exit=0 ok=True`） | 07:36:41 → 07:37:12 |
| 3 | `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run numeric` | 0（`numeric exit=0 ok=True`） | 07:37:14 → 07:37:45 |
| 4 | `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run default` | 0（`default exit=0 ok=True`） | 07:37:47 → 07:38:18 |
| 5 | `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run inverted -ExitMs 75000`（目視のための追加の 1 本・押下なし） | 0 | 07:54:48 → 07:56:03 |

5 本目は §7.6 の目視のために、押さずに画面を撮るだけの走行として足した（SendInput は使っていない）。道具は `inverted\` へ書くので、走った後に採れたものを `inverted-visual-run\` へ移し、1 本目（`-Press`）の採れたものを `inverted\` へ戻した（戻した `inverted\run.log` の md5 は §7.3 と同じ。控えの写しが `inverted-press-run\` にもある）。そのため `inverted-visual-run\steps.txt` の頭の行の `root=` は移す前の `inverted\root` を指している。

### 7.2 判定（§5 のコマンド・終了コードで取った）

| 種類 | 判定の終了コード | 項目 |
|---|---|---|
| `inverted` | **0** | A=PASS R=PASS L=PASS S=PASS P=PASS（6 場面とも ok） |
| `explicit` | **0** | A=PASS R=PASS L=PASS S=PASS P=PASS |
| `numeric` | **0** | A=PASS R=PASS L=PASS S=PASS P=PASS |
| `default` | **0** | D=PASS P=PASS |
| `inverted`（5 本目・`inverted-visual-run\`） | 2 | A=PASS R=PASS L=PASS S=PASS P=INCONCLUSIVE（`missing probes: press-s0,press-b0,drag-s0,other-app,returned`） |

5 本目の 2 は、押下の無い走行なので前面走査が `boot` の 1 場面しか無く、`inverted` の判定が 6 場面を求めるため（§6.2 の 11 行目と同じ「場面が欠ければ判定不能」）。採れた `boot` の 1 場面は ok で、ログの側の 4 項目は緑。task 5.2 の判定は 1〜4 本目の 4 本が担い、5 本目は目視の材料である。判定の道具は変えていない。

### 7.3 一次証跡の md5（判定の前に控えた）

置き場は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\target\zorder-inversion-signoff\<種類>\`（コミットしない）。`default` の `run.log` だけは `signoff-real-default.log` として本書と同じフォルダへ控えた（§6.1）。

| ファイル | 行数 | md5 |
|---|---|---|
| `inverted\run.log` | 335 | `A744A3005E12B672C88BB1A3806C8A74` |
| `inverted\probe.txt` | 6 | `E01CA69564D7F5234D3ABCFDE03AAA08` |
| `explicit\run.log` | 205 | `60AF1C6E12F54130A5527F47EF5A0D90` |
| `explicit\probe.txt` | 1 | `68952D8A27D3165271576CEB8DB067D0` |
| `numeric\run.log` | 213 | `8E9C725492AD7E40C8DA9F41717CC247` |
| `numeric\probe.txt` | 1 | `9B2CF5663AF26D3885B9F3598CB31976` |
| `default\run.log`（＝`signoff-real-default.log`） | 214 | `5F1CF6AA6DE98D3E05D230C2964D6B21` |
| `default\probe.txt` | 1 | `1A882BE76ED32D6E93B2C4AA0B4933BB` |
| `inverted-visual-run\run.log`（5 本目） | 148 | `6AA351F14F929D44B7E68432A4798D6C` |
| `inverted-visual-run\probe.txt`（5 本目） | 1 | `2CCA51C699851ED36DA7034849EC4695` |

### 7.4 鍵の行（冠込み・時刻と発信元の前置きを外して逐語）

`inverted`（窓: s0=`0x121474`・b0=`0x1F00D34`・b1=`0x80141A`・s1=`0x600D56`）:

```
[zorder-pair] owner-established entity=25v0 peer=26v0 owned_hwnd=0x1F00D34 owner_hwnd=0x121474 measured_prev=0x600D56
[zorder-pair] owner-established entity=27v0 peer=28v0 owned_hwnd=0x80141A owner_hwnd=0x600D56 measured_prev=0x1F00D34
[zorder-group] applied action=reset groups=0 base=-
[zorder-group] applied action=set group_id=0 source=Tag members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1
[zorder-chain] pair-released entity=25v0 peer=26v0 owned_hwnd=0x1F00D34 owner_hwnd=0x121474 unlinked=true
[zorder-chain] linked segment=g0 owned=26v0 owner=25v0 owned_hwnd=0x121474 owner_hwnd=0x1F00D34 pos=1/4
[zorder-chain] linked segment=g0 owned=25v0 owner=27v0 owned_hwnd=0x1F00D34 owner_hwnd=0x80141A pos=2/4
[zorder-chain] settled nudged_hwnd=0x600D56 insert_after=0x121474 declared=0x121474,0x1F00D34,0x80141A,0x600D56 measured=0x121474,0x1F00D34,0x80141A,0x600D56 nudge_ok=true
```

`explicit`:

```
[zorder-group] applied action=set group_id=0 source=Tag members=b0,s0,b1,s1 normalized=0:false,1:false
[zorder-chain] linked segment=g0 owned=26v0 owner=27v0 owned_hwnd=0x2A13F4 owner_hwnd=0x1D501FA pos=2/4
[zorder-chain] settled nudged_hwnd=0x4C511C6 insert_after=0x2040FB8 declared=0x2040FB8,0x2A13F4,0x1D501FA,0x4C511C6 measured=0x2040FB8,0x2A13F4,0x1D501FA,0x4C511C6 nudge_ok=true
```

`numeric`:

```
[zorder-group] applied action=set group_id=0 source=Tag members=b0,s0,b1,s1 normalized=-
[zorder-chain] linked segment=g0 owned=26v0 owner=27v0 owned_hwnd=0x7A1256 owner_hwnd=0xD612F2 pos=2/4
[zorder-chain] settled nudged_hwnd=0xA512A8 insert_after=0x4C611C6 declared=0x4C611C6,0x7A1256,0xD612F2,0xA512A8 measured=0x4C611C6,0x7A1256,0xD612F2,0xA512A8 nudge_ok=true
```

`explicit`・`numeric` とも `[zorder-chain] pair-released` は 0 行（組は隣り合うので外さない＝本機能の前と同じ・要件 8.6）。

`default`: `[zorder-group]`・`[zorder-chain]` の行は 0 行。`[zorder-pair] owner-established` が 2 行（b0=`0xA612A8`→s0=`0x7C1280`・b1=`0xAA1358`→s1=`0x2C13F4`）。

### 7.5 押した後の並び（`inverted\steps.txt`・`probe.txt`）

| 場面 | 押した点（物理画素） | その後の前面の窓 | 前面走査（手前 → 奥） |
|---|---|---|---|
| `boot` | — | `0x400C6`（他のアプリ） | `*,0x121474,0x1F00D34,0x80141A,0x600D56,*` |
| `press-s0` | 2446,1157（1 点目で届いた） | `0x121474`（s0） | `*,0x121474,0x1F00D34,0x80141A,0x600D56,*` |
| `press-b0` | 2196,576（1 点目で届いた） | `0x1F00D34`（b0） | `*,0x121474,0x1F00D34,0x80141A,0x600D56,*` |
| `drag-s0` | 掴んで左へ 60 画素・離した点 2386,1157 | `0x121474`（s0） | `*,0x121474,0x1F00D34,0x80141A,0x600D56,*` |
| `other-app` | 280,200（道具の窓） | `0x2115FC`（道具の窓） | `*,0x121474,0x1F00D34,0x80141A,0x600D56,*` |
| `returned` | 1728,1304（s1） | `0x600D56`（s1） | `*,0x121474,0x1F00D34,0x80141A,0x600D56,*` |

- 6 場面とも並びは宣言列 s0 → b0 → b1 → s1 のままで、ゴーストの窓の間に他のアプリの窓（`*`）は 1 度も挟まらなかった。§4.4 の止める条件（R3）には当たらず、押下で塊のまま上がった（要件 5.1・5.3・5.4）。
- `press-s0`・`press-b0` の後もむらさきのバルーン（b0）はむらさき（s0）より奥のまま（要件 5.2）。

### 7.6 目視の結果（要件 8.4）

**撮り方**: 5 本目の走行（§7.1・押下なし）の起動から約 20 秒後（07:55:10。`[zorder-chain] settled` は 07:54:49）に、PowerShell の `Graphics.CopyFromScreen`（DPI を正しく扱う設定で、物理画素のまま）で画面を撮った。画面の撮影の道具（computer-use）は登録の無いアプリを覆い隠し、作業ツリーの検体の `areka.exe` が写らないので使っていない。

| ファイル（`target\zorder-inversion-signoff\visual\`） | 中身 | 大きさ | md5 |
|---|---|---|---|
| `screen0.png` | 主モニタの全体（ゴーストはここ） | 2880x1800 | `1865076680B4F10291114F3D2B54439A` |
| `screen1.png` | 副モニタの全体 | 3413x2133 | `9A248B0F0BF9322378777E06A6D43D51` |
| `z_s0_b0_b1.png` | `screen0` の切り抜き（x2100 y700・500x450） | 500x450 | `1383F906D99F403A24AFF305E032EFFE` |
| `z_b1_s1.png` | `screen0` の切り抜き（x1600 y900・500x400） | 500x400 | `3E28EBC66632CF835752D1B9B51C0750` |

この走行の窓: s0=`0x16C0F26`・b0=`0x11A0D6C`・b1=`0xE512F2`・s1=`0x1F30D34`（`inverted-visual-run\run.log` の `pair-released` と `linked`）。最後の `[zorder-chain] settled` は `declared=0x16C0F26,0x11A0D6C,0xE512F2,0x1F30D34` で `measured=` も同じ 4 窓。`boot` の前面走査（07:54:53）も `*,0x16C0F26,0x11A0D6C,0xE512F2,0x1F30D34,*` だった。

**見えたもの**（`screen0.png`）:

- むらさき（s0）が右に立ち、むらさきのバルーン（b0・「ふぁ‥‥、おはよ！もう朝なん！？はやっ！」）がその左上にあって、しっぽがむらさきの頭へ向いている。
- エモ（s1）が左下に座り、エモのバルーン（b1・「切り替え早すぎない？」）が 2 人の間にあって、しっぽがエモへ向いている。
- むらさきの髪が b1 の枠の右の端の上に描かれている（`z_s0_b0_b1.png` で枠の線が髪の後ろへ隠れる）。つまり s0 は b1 より手前。
- どちらのバルーンもむらさきに掛かっていない（バルーンがむらさきより手前へ出ていない）。

以上は、重なっている画素から読める限りで、宣言の並び 手前 s0 → b0 → b1 → s1 と合っている。

**目では見えないこと（限り）**:

- この配置では b0 と b1、b1 と s1、b0 と s1 は画面の上で重なっていない（`z_b1_s1.png` でも b1 のしっぽとエモの髪は離れている）。そのためこれらの組の前後は目では判じられない。これらの組の順は、同じ走行の `settled` の宣言と実測の一致と `boot` の前面走査（上）、および 1 本目の 6 場面の前面走査（§7.5）が裏付ける。
- 目で前後が見えたのは s0 と b1 の 1 組だけである。要件 8.4 の前 3 枚（s0 → b0 → s1）のうち、s0 が b0 にも s1 にも隠されていないことは見えたが、b0 と s1 の前後は上のとおりログと前面走査による。
- 押した後（`press-s0` の後に b0 が s0 より手前へ出ないこと・要件 5.2）は、5 本目が押下なしの走行なので目では見ていない。§7.5 の前面走査による。
- 撮った時、主モニタの右下に Claude アプリの通知（他のアプリの窓）がむらさきの下の方に重なって出ていた。ゴーストの並びの手前にある他のアプリの窓で、並びの間には入っていない（前面走査の頭の `*`）。

### 7.7 較正の取り直し（§6）

`pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-calibrate.ps1` を、既定を `signoff-real-default.log` に替えて回した: `calibration: 25 rows, 0 wrong`・**exit=0**。標本の md5 は §6.3 を取り直した値で、窓のハンドル（`s0=0x7C1280 b0=0xA612A8 s1=0x2C13F4 b1=0xAA1358`）が新しい実走ログから取られている。
