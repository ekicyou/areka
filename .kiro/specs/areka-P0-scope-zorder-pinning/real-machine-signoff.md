# emo2 実機サインオフ受け入れ記録（areka-P0-scope-zorder-pinning）

対象要件: requirements.md **6.1／6.2／6.4／9.4**
判定式の正典: design.md **§Testing Strategy → 「実機サインオフの改訂（要件 9.4・9.5）」**と **§Monitoring**
判定手順: `signoff-procedure.md` ／ 判定スクリプト: `signoff-scan.ps1`

---

> # ⚠ 本記録は**初版（毎巡の観測＋是正モデル）**の受け入れ記録である（2026-08-29）
>
> **2026-08-30（task 6.1）に判定語を鎖の語彙へ差し替えた。以下の §1〜§6 は、その差し替え前の
> 実装・差し替え前の判定式で採った一次証跡であり、そのまま残す**——数値モードの不成立（§4）は
> 本 spec の改訂そのものを引き起こした実測であり、書き換えれば改訂の理由が消える。
>
> **⚠ 現行版（所有の鎖）の受け入れ記録は §7〜§9 である**（2026-08-30・task 6.2）。
> §4 が「実機で一度も成立しない」と記録した数値モード `0,1` は、**§8.2 で成立している**。
>
> ## 本版（鎖）での判定の意味
>
> | 判定 | **本版の意味**（2026-08-30〜） | 初版の意味（本記録が測ったもの） |
> |---|---|---|
> | **J1** | 受理の記録があり、**繋いだ行（`[zorder-chain] linked`）が全行そろって宣言列の隣接対に当たり**、**宣言列ごとの最後の `settled` が全てその行の中で `declared=` と `measured=` が一致**し（＝終状態の全称）、`[zorder-chain] link-failed` が 0 件 | 受理の記録があり、是正の記録（`[zorder-group] fix`）の指令と実測が同一行で一致し、以後の巡が `AlreadyOrdered order_ok=true` で落ち着き、`GaveUpAfterFailures` が 0 件 |
> | **J2** | `[zorder-group]` と `[zorder-chain]` の記録が**どちらも 0 件** | `[zorder-group]` の 5 タグすべてが 0 件 |
> | **J3** | **変更なし**（既存ペア機構 `[zorder-pair]` の記録が従来どおり出る・欄立ての 5 欄を照合） | 同左 |
>
> **撤去した 2 つの判定材料**: `reason=AlreadyOrdered`（既に並んでいた）と
> `reason=GaveUpAfterFailures`（失敗続きで諦めた）。どちらも毎巡の観測を前提とした語で、
> 鎖の下には存在しない——維持は OS が行うので「以後の巡」も「連続失敗の頭打ち」も無い。
>
> **量化子について（レビュー round 1 の是正）**: J1 の `settled` は **「1 本でも一致すれば緑」ではない**。
> `settled` は結果の並びの食い違いを報せる唯一の記録であり、6.2 の走行では複数本出るのが前提なので、
> 存在量化にすると **N 本中 1 本当たれば緑**になって初版の症状をそのまま見逃す。
> 一方で素の全称（不一致 0 本）にもしない——**同じ宣言列の中で**走査の打切り等により 1 巡だけ
> 実測が揃わないことが在り、それは後続の巡の一致が救えるからである。よって
> **宣言列ごとの終状態について全称**を採った。
>
> **⚠ ⒝ が救うのはそこまでである。** 窓が去って**宣言列そのものが縮む**過渡は、短い実測の
> 宣言列が二度と再来しないので **⒝ でも赤になる**（実測＝`S13-departing-shrink.log` と
> レビュアー検体 `RVA-departing-transient.log` がいずれも exit 1）。
> **「窓の出入りの過渡は自動的に許される」と読んではならない**——その形は 6.2 で実測を添えて
> 登記するまで通らない。裁定の全文と較正（25 通り）は `signoff-procedure.md` §5.1-3a／§6.1.2。
>
> **据え置いた語**（要件 9.5）: `[zorder-group] applied` / `[zorder-group] rejected`・`[zorder-pair]` の 4 本・
> 出所の別（`action=set` / `source=Descript` / `source=Tag`）・`$PAIR_OWNER_FIELDS` の 5 欄・
> 終了コードの体系 0/1/2/3。**本記録の一次証跡（`R1`〜`R5` と 3 つの変異体）へ改訂後の道具を当て直し、
> これらがすべて従来どおり当たることを確かめてある**（`signoff-procedure.md` §6.1.2 の 10・12〜15 行目）。
>
> ## 本記録のどの結論が本版でも生きているか
>
> | §  | 本版での扱い |
> |---|---|
> | §1 実施条件・ログの所在・md5 | **生きている**（一次証跡そのもの。改訂後の道具の較正にも使った） |
> | §2 J1 の実測（R2・R3 が PASS） | **退役**——初版の実装・初版の語での PASS である。鎖での J1 は task 6.2 が測る |
> | §3 J2 の実測（R1 が PASS） | **部分的に生きている**。据え置いた語での「0 件」は改訂後の道具でも再現した（`R1-default.log` → J2=PASS / exit 0）。ただし本版の J2 は `[zorder-chain]` も 0 件であることを要求するので、**その主張は 6.2 の走行で取り直す** |
> | §4 数値モードの不成立（**実機が掘り当てた欠陥**） | **生きている。本 spec の改訂を引き起こした実測である。** ただし §4.3 の根因候補 2 つと切り分け手順は退役した（対象の本番ファイル `zorder_group_maintain.rs` が存在しない）。鎖の下で数値モードが成立するかは **task 6.2 が測る** |
> | §5 J3（ペア語彙の保全） | **生きている**。改訂後の道具でも `R1`〜`R5` すべてで J3=PASS、変異体 3 本の赤／緑も逐語で同じ |
> | §6 実機で確認できていないこと | **生きている**（射程外の列挙は改訂で狭まっていない） |

---

## 総合判定（**初版**・2026-08-29）: **3 点とも記録の照合だけで判定できた。ただし数値モードで J1 が不成立（実機で掘れた欠陥）**

| 判定 | 対象要件 | R1 指定なし | R2 設定由来 `b0,s0,s1` | R3 タグ由来 `b0,s0,s1` | R4 数値モード `0,1` | R5 解釈不能値 |
|---|---|---|---|---|---|---|
| **J1** 指定が成立した | 9.4 | —（対象外・判定不能 exit 2） | **PASS** | **PASS** | **FAIL** | —（判定不能 exit 2） |
| **J2** 是正の記録が 0 件 | 6.1／6.2／6.4 | **PASS** | —（対照：FAIL＝非 0） | — | — | —（対照：FAIL＝`rejected` 1 件） |
| **J3** ペア機構が従来どおり | 9.5 | **PASS** | **PASS** | **PASS** | **PASS** | **PASS** |

- **J1・J2・J3 のいずれも目視を使っていない。** すべて `signoff-scan.ps1` が記録の照合だけで下した判定である（要件 9.4 の実質）。
- **J1 は「成立する形」では 2 経路（shell 設定・台本のタグ）とも成立した。** 指令と実測が同一行で一致し、以後の巡は `AlreadyOrdered order_ok=true` で落ち着いた。
- **J1 は「数値モード」では成立しなかった。** これは判定の失敗ではなく**実機が掘り当てた本物の欠陥**である（§4）。
- **R5 は判定の対象ではなく、判定語 `[zorder-group] rejected` の実出の対照である**（§3.3）。
  この語は R5 以外の 18 本（本サインオフ 14 本＋独立レビュー 4 本）すべてで 0 件なので、
  この 1 本が無いと **J2 の 5 つ目の連言が恒真**になる。

---

## 1. 実施条件

| 項目 | 値 |
|---|---|
| 実施日時 | 2026-08-29（R1〜R4 の観測区間 `05:58:58Z` 〜 `06:07:12Z`・追加走行 R5 は `06:37:57Z` 〜 `06:38:43Z`・UTC） |
| ブランチ / HEAD | `claude/areka-p0-zorder-pinning-8e3e7c` / `63a1674f`（task 7.3 まで） |
| ゴースト | 実 emo2 fixture・**実 pasta.dll**・辞書込みフルゴースト（発話が返っていることを `seriko: bind 適用` と `[balloon-visibility]` の遷移で確認。台詞そのものの字面は `diagnostics\probe-default.log` の `command=Text("こんにちはー！")` にある） |
| 起動 | **絶対パス**（ghost＝各検体のルート／balloon＝`fixtures\emo2\emo2-kakukaku`） |
| 有界終了 | `AREKA_APP_SMOKE_EXIT_MS=120000`（2 分）× 4 走行（R1〜R4）＋ `45000`（45 秒）× 1 走行（R5） |
| ログ水準 | `RUST_LOG=info,wintf::ecs::window=debug` |
| helper | i686 版 `shiori-host32-helper.exe` を `target\debug\` へ配置済み |
| ビルド | debug プロファイル（`cargo build -p areka`） |
| モニタ | 実効 192dpi（`k_shell=2.0 k_balloon=2.0`・ログの `placement: k₀ 倍後の物理窓寸で窓を生成する` 行） |

### 検体

| 走行 | ゴースト | 重なり指定 |
|---|---|---|
| **R1** | `fixtures\emo2`（**共有 fixture・無改変**） | 無し |
| **R2** | `fixtures\emo2-zsp-descript` | `shell\master\descript.txt:11` に `seriko.zorder,b0,s0,s1` |
| **R3** | `fixtures\emo2-zsp-tag` | `dic\boot.pasta` の起動 12 パターンに `\![set,zorder,b0,s0,s1]`（shell はジャンクションで原本＝設定は無し） |
| **R4** | `fixtures\emo2-zsp-descript` | 同ファイルの 1 行を `seriko.zorder,0,1` に差し替え（走行後 `b0,s0,s1` へ戻した） |
| **R5** | `fixtures\emo2-zsp-descript`（作り直し） | `seriko.zorder,Balloon0,zzz`（**解釈不能**・`signoff-procedure.md` §3.3） |

**共有 fixture（`fixtures\emo2`）は 1 バイトも書き換えていない。** 既定＝非強制の走行（R1）は
その無改変の検体をそのまま使って得たものである（task 7.4 の明示要求）。走行後に
`git status --short crates/pilot/examples/shiori-host-32/fixtures/emo2` が空であることを確認した。

派生検体（R2〜R5）は**走行後に片付けた**——ジャンクションを含む未追跡ツリーを作業ツリーへ
残すと `git clean` 等で原本まで巻き込む危険があるためである。作り直しは
`signoff-procedure.md` §3.1／§3.2 のスクリプト 1 本で足り、**そのスクリプトが正本**である。
片付けの安全な順序は同 §3.2.1 にある。

### ログの所在（一次証跡・読み取り専用）

`C:\Users\maz-o\AppData\Local\areka-diag\zsp-signoff-20260829-145858\`

| ログ | サイズ | md5 |
|---|---|---|
| `R1-default.log` | 78,921 bytes | `504D1F432D9CEDE079171955F909BBF1` |
| `R2-descript.log` | 78,273 bytes | `63723D6126785235019ECE9C91E5B3A8` |
| `R3-tag.log` | 85,264 bytes | `DEA1C5DF8917705E4F2B4D50565739B0` |
| `R4-numeric.log` | 120,058 bytes | `AAA6C080DE3CCEE1FACFB369664EB54F` |
| `R5-rejected.log` | 44,201 bytes | `BAF78BAC392E92F02B872B61C8599BB8` |

切り分け用の追加走行 **10 本**と、判定器較正の変異体 **3 本**を同ディレクトリの
`diagnostics\` に保全した（§4.2 の表と `signoff-procedure.md` §6 が対応）。

| 変異体 | 何を壊したか | サイズ | md5 |
|---|---|---|---|
| `mutant-no-measured_prev.log` | `measured_prev=` 欄を落とす | 78,873 bytes | `510740AB7E3A064FC3E8353EF36284F9` |
| `mutant-rename-char_entity.log` | `entity=` を `char_entity=` へ改名 | 79,106 bytes | `7DE6E5DE77447E7095C610F5B8982DBF` |
| `mutant-value-garbage.log` | `measured_prev=` の**値**を `GARBAGE` に | 78,917 bytes | `F6BCAC230A8E2FD7F657FA4B6C36A753` |


### 1.1 本記録の件数はすべて一次証跡へ当て直した（2026-08-29 再突合）

初版に**転記ミスが 2 件**あったため、本文が挙げる件数を**全件**、保全ログへ当て直した。
判定語はいずれも冠（`[…]`）まで含めた形で数えている。

| 走行 | `[zorder-group]` 計 | applied | fix | skip | verify-failed | rejected | AlreadyOrdered<br>(order_ok=true) | GaveUp | PairFix<br>ThisPass | Member<br>Missing | `[zorder-pair]`<br>owner / fix / skip |
|---|---|---|---|---|---|---|---|---|---|---|---|
| R1 指定なし | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 / 1 / 1 |
| R2 設定由来 | 11 | 1 | 1 | 9 | 0 | 0 | **7** | 0 | 1 | 1 | 2 / 1 / 1 |
| R3 タグ由来 | 9 | 1 | 1 | 7 | 0 | 0 | **6** | 0 | 0 | 1 | 2 / 1 / 1 |
| R4 数値モード | 35 | 1 | **0** | 10 | **24** | 0 | 0 | **8** | 1 | 1 | 2 / 1 / 1 |
| R5 解釈不能値 | 1 | 0 | 0 | 0 | 0 | **1** | 0 | 0 | 0 | 0 | 2 / 1 / 1 |

`skip` の内訳は足し合わせが合う（R2＝7+1+1、R3＝6+0+1、R4＝8+1+1）。
切り分け 8 走行の `verify-failed`／`fix` も §4.2 の表と逐語で一致する
（`diag-0_1` 15／`diag-b0_s0_b1_s1` 9・諦め 3／`diag-s0_s1`・`diag-s0_b1`・`diag-b0_s1`・`diag-b0_s0_s1` は各 `fix` 1）。
`diag-write.log` の書込は 32 本で **32/32 が `via="DeferWindowPos" in_batch=true`**。
ログのサイズと md5 も上表と一致することを再計算で確認した。

**見つかった転記ミス 2 件**（いずれも本文を実測値へ訂正済み）:

| # | 箇所 | 初版 | 実測 | 原因 |
|---|---|---|---|---|
| 1 | §3.3 R5 の副次証跡 | 「バルーンの可視状態が遷移した **25 件**」 | **4 件**（`[balloon-visibility]` 行そのものは 14 件） | `grep -c 'balloon-visibility'` を**角括弧なし**で走らせ、作業ツリー名 `areka-p0-balloon-visibility-b341d2` を含むパス行 11 本を巻き込んだ |
| 2 | §5 の地の文 | 「`PairFixThisPass` が**各走行で 1 件**出る」 | **R2・R4 のみ各 1 件**（R1・R3・R5 は 0） | 2 走行だけを見て一般化した |

**⑴は本 spec 固有ではなく、この作業ツリーで走らせる限り誰でも踏む**——走行ログは
作業ツリーの絶対パスを毎行のように吐き、その名前が `balloon-visibility` を含むからである。
`signoff-procedure.md` §2 の判定語をすべて冠つきで書いてあるのはこの罠を避けるためであり、
`signoff-scan.ps1` も `[zorder-group]` / `[zorder-pair]` を冠込みで照合するので影響を受けない。

---

## 2. J1（グループ指定が実際に成立したこと・要件 9.4）

### 2.1 R2 — shell 設定由来（`source=Descript`）: **PASS**

```
06:00:59.235162Z DEBUG wintf::ecs::window::zorder_group:
  [zorder-group] applied action=set group_id=0 source=Descript members=b0,s0,s1 normalized=0:false

06:00:59.531463Z DEBUG wintf::ecs::window::zorder_group:
  [zorder-group] fix group_id=0 head=0x4880F82 moves=0x15218A8@0x4880F82,0x4DC110E@0x15218A8
                     measured=0x4880F82,0x15218A8,0x4DC110E

06:00:59.531581Z DEBUG wintf::ecs::window::zorder_group:
  [zorder-group] skip group_id=0 reason=AlreadyOrdered resolved=3 missing=0 order_ok=true
```

- **指令**（`head` ＋ `moves` の動かした窓）＝ `0x4880F82, 0x15218A8, 0x4DC110E`
- **実測**（`measured`＝前面走査が実際に出会った並び）＝ `0x4880F82, 0x15218A8, 0x4DC110E`
- **一致。** 1 行の中で指令と実測が突き合う（要件 9.1／9.2 が求める形そのもの）。
- 以後 2 分間で `AlreadyOrdered order_ok=true` が **7 件**、`verify-failed` **0 件**、`GaveUpAfterFailures` **0 件**。
- 記録件数: `applied 1 / fix 1 / skip 9 / verify-failed 0 / rejected 0`
- `signoff-scan.ps1 -Mode grouped` → **J1=PASS J3=PASS / exit 0**

### 2.2 R3 — 台本のタグ由来（`source=Tag`）: **PASS**

```
06:03:01.393263Z DEBUG wintf::ecs::window::zorder_group:
  [zorder-group] applied action=set group_id=0 source=Tag members=b0,s0,s1 normalized=0:false

06:03:01.423087Z DEBUG wintf::ecs::window::zorder_group:
  [zorder-group] fix group_id=0 head=0x15318A8 moves=0x4DA10A0@0x15318A8,0x155182C@0x4DA10A0
                     measured=0x15318A8,0x4DA10A0,0x155182C
```

- 指令と実測が一致。以後 `AlreadyOrdered order_ok=true` が **6 件**、`verify-failed` **0 件**、`GaveUpAfterFailures` **0 件**。
- 記録件数: `applied 1 / fix 1 / skip 7 / verify-failed 0 / rejected 0`
- **この検体の shell は原本へのジャンクション**（`seriko.zorder` を持たない）ので、
  グループを作ったのは台本のタグだけである。`source=Tag` がそれを一意に示す。
- `signoff-scan.ps1 -Mode grouped` → **J1=PASS J3=PASS / exit 0**

### 2.3 R1 を `-Mode grouped` で測ると **判定不能（exit 2）**

受理の記録が 0 件なので「成立した」とも「成立しなかった」とも言えない。
**沈黙を PASS と読ませない**ための非ゼロである。

---

## 3. J2（既定＝非強制・要件 6.1／6.2／6.4）

### 3.1 R1: **PASS**

無改変の共有 fixture（`seriko.zorder` を宣言していない）での 2 分走行で、
`[zorder-group]` の **5 タグすべてが 0 件**であった。

```
[zorder-group] 合計 0   applied 0 / fix 0 / skip 0 / verify-failed 0 / rejected 0
[zorder-pair]  合計 4   owner-established 2 / fix 1 / skip 1
```

`applied` が 0 件なのは、`apply_descript_base` が「設定が無い」を失敗でも見送りでもなく
**不在**として扱い、記録も残さず戻るためである（`emo2_boot/frame/zorder_descript.rs:68-71`）。
既定＝非強制は判断ではなくこの不在で成り立っている。

### 3.2 「0 件」の対照（要件どおりの非 0 が同じ道具・同じ語で出ること）

同じスクリプト・同じ語を **R2**（グループ指定あり）に当てると **J2=FAIL / exit 1** になり、
11 件の `[zorder-group]` 行が列挙される。**したがって R1 の 0 件は「語が空振りしている」ではなく
「本当に記録が無い」である。**

| 当てたログ | `-Mode` | 結果 |
|---|---|---|
| `R1-default.log` | `default` | J2=**PASS**（0 件）/ exit 0 |
| `R2-descript.log` | `default` | J2=**FAIL**（11 件）/ exit 1 ← **対照**（`applied`／`fix`／`skip`） |
| `R4-numeric.log` | `grouped` | `verify-failed` 24 件 ← **対照**（`verify-failed`） |
| `R5-rejected.log` | `default` | J2=**FAIL**（1 件）/ exit 1 ← **対照**（`rejected`・§3.3） |

**J2 が主張する 5 語すべてに、その語で非 0 が出る走行が対応している。**
1 語でも対応が欠けると、その連言は「本当に出ていない」と「語が空振りしている」を
区別できない恒真になる。

### 3.3 `rejected` の実出（R5・要件 5.4／8.1／8.3）

`[zorder-group] rejected` は正しい指定を書いている限り一生出ない語なので、
**解釈不能な値を置いた検体で 45 秒の走行を 1 本だけ追加した**
（`shell\master\descript.txt:11` に `seriko.zorder,Balloon0,zzz`）。

```
2026-08-29T06:37:58.413619Z  WARN wintf::ecs::window::zorder_group:
  [zorder-group] rejected reason=UnparsableToken(Balloon0) tokens=Balloon0,zzz
```

| 記録 | 件数 |
|---|---|
| `[zorder-group] rejected` | **1** |
| `[zorder-group] applied` / `fix` / `skip` / `verify-failed` | **0 / 0 / 0 / 0** |
| `[zorder-pair] owner-established` / `fix` / `skip` | **2 / 1 / 1** |

- **理由と受け取った値の両方が載っている。** `reason=UnparsableToken(Balloon0)` が
  どのトークンで落ちたかを名指しし、`tokens=Balloon0,zzz` が作者の書いた値そのものを残す
  （要件 8.1「拒否理由を記録する」・8.3「黙って諦めない」）。
- **台帳は 1 本も載っていない**（`applied` 0 件）＝部分適用していない（要件 8.1）。
- **起動は続いている**＝拒否は起動を止めない（要件 5.4）。ペア機構の記録が従来どおり出るだけでなく、
  SHIORI 由来の発話も動いている——45 秒で `seriko: bind 適用` が **8 件**、
  `[balloon-visibility] バルーンの可視状態が遷移した` が **4 件**
  （`trigger="content" visible=true` 3・`trigger="clear" visible=false` 1）。
  なお `[balloon-visibility]` を冠に持つ行そのものは **14 件**である（内訳の判定に使うのは遷移の 4 件）。

  > **⚠ この 2 つの数値は初版で 25 件と誤って書いていた。** 原因は
  > `grep -c 'balloon-visibility'` を角括弧なしで走らせたことで、
  > **作業ツリーのディレクトリ名 `areka-p0-balloon-visibility-b341d2` を含むパス文字列 11 行**が
  > 一緒に数えられていた（25 = 14 + 11）。**ログの語を数えるときは、その語が本文の一部であることを
  > 冠（`[…]`）まで含めて指定すること**——本 spec の走行ログは作業ツリーの絶対パスを毎回吐くので、
  > ハイフン語の素の grep は自分のディレクトリ名に当たる。
- `Balloon0` が落ちるのは語彙の一致が**小文字ちょうど**だからである
  （`crates/areka/src/placement/zorder_group_ledger.rs:168-173`・task 1.1 の裁量として登記済み）。
  この走行はその裁量が**実機でもそのとおりに効いている**ことの証跡でもある。

---

## 4. 実機が掘り当てた欠陥 — 数値モードは実機で成立しない

### 4.1 事実（R4・`seriko.zorder,0,1`）

```
06:05:12.343111Z DEBUG [zorder-group] applied action=set group_id=0 source=Descript
                        members=b0,s0,b1,s1 normalized=-

06:05:12.601026Z ERROR [zorder-group] verify-failed group_id=0 head=0x4DB10A0
   moves=0x5350B7E@0x4DB10A0,0x48A0F82@0x5350B7E,0x4DE110E@0x48A0F82
   members=0x4DB10A0,0x5350B7E,0x48A0F82,0x4DE110E
   measured=0x48A0F82,0x4DE110E  missing=0  scan_complete=true

06:05:12.617660Z  WARN [zorder-group] skip group_id=0 reason=GaveUpAfterFailures
   resolved=4 missing=0 order_ok=false streak=3 scan_complete=true
```

- 2 分の走行で `verify-failed` **24 件**・`GaveUpAfterFailures` **8 件**・`fix` **0 件**。
  すなわち **1 度も成立しなかった**。追随トリガのたびに 3 回試して諦めるのを 8 度繰り返した。
- `missing=0`（4 枚すべて解決できている）・`scan_complete=true`（走査は打ち切られていない）。
  それでも `measured` に載るのは `0x48A0F82(b1), 0x4DE110E(s1)` の 2 枚だけ——
  **scope 0 の 2 枚は最後尾の `s1` より奥にいる**、つまり**重なりは起動時のまま 1 ミリも動いていない**。
- 指令は毎巡ちゃんと Win32 へ届いている。`RUST_LOG` に `wintf::transition=debug` を足した走行
  （`diagnostics\diag-write.log`）で、同じ 3 本の `SetWindowPos` が毎巡
  `kind=write ... flags=0x13 ... ok=true`（`SWP_NOSIZE|NOMOVE|NOACTIVATE`）で成功している。
  **書込は成功し、重なりは変わらない。**

### 4.2 切り分け（同一検体・`seriko.zorder` の 1 行だけを変えた 8 走行）

| 指定 | 意味 | 結果 | ログ |
|---|---|---|---|
| `1,0` | 数値・**起動時の並びと同じ** | `AlreadyOrdered order_ok=true`（是正不要） | `diag-1_0.log` |
| `0,1` | 数値・逆順（R4 と同じ形の 60 秒走行） | **成立せず**（`verify-failed` 15 件） | `diag-0_1.log` |
| `s1,s0` | 明示・起動時の並びと同じ | `AlreadyOrdered order_ok=true`（是正不要） | `diag-s1_s0.log` |
| `s0,s1` | 明示・**キャラ窓 2 枚だけを入れ替える** | **`fix` 成立**（measured 一致） | `diag-s0_s1.log` |
| `s0,b1` | 明示・キャラ 0 とバルーン 1 | **`fix` 成立** | `diag-s0_b1.log` |
| `b0,s1` | 明示・バルーン 0 とキャラ 1 | **`fix` 成立** | `diag-b0_s1.log` |
| `b0,s0,s1` | 明示・**3 枚**（scope 0 の対 ＋ scope 1 のキャラ） | **`fix` 成立** | `diag-b0_s0_s1.log`・R2・R3 |
| `b0,s0,b1,s1` | 明示・**4 枚**（＝`0,1` の展開形と同一） | **成立せず**（`verify-failed` 9 件・諦め 3 回） | `diag-b0_s0_b1_s1.log` |

**決定的な対比は最後の 2 行である。** `b0,s0,s1`（3 枚）は成立し、`b0,s0,b1,s1`（4 枚）は成立しない。
この 2 つには **2 つの差**がある。**どちらが効いているかは本サインオフでは切れていない**（§4.3）。

1. **`b1`（scope 1 の「所有される」バルーン窓）を連鎖の駒として明示的に動かすかどうか**
2. **同一 `DeferWindowPos` バッチに積む移動が 2 本か 3 本か**

> **注意**: 「3 枚でも見た目の目標は同じ（`s1` を動かせば `b1` も一緒に動く）」という読みは
> **本サインオフでは裏を取っていない**。`b0,s0,s1` の走行の `measured` に載るのは宣言した
> 3 枚だけであり、`b1` が最終的にどこに居たかは測っていない（グループ外の窓の位置は
> `GroupObservation` が構造的に持たない）。目視も使っていない。
> **「同じ目標」は Win32 の所有関係の一般則からの推論であって観測ではない。**

### 4.3 根因の候補は 2 つある（**本サインオフでは切れていない**）

**是正を担当する者を片方の仮説へ誘導しないため、両方を並べる。**

#### 候補 A: Win32 の所有関係と噛み合っていない

- 本番のゴースト窓は **2 組の Win32 所有関係**でできている
  （`[zorder-pair] owner-established owned_hwnd=<バルーン> owner_hwnd=<キャラ>`・areka-P0-ghost-window-zorder）。
  Windows は所有される窓を必ず所有者より手前に保つので、
  **所有される窓を単独で所有者より奥へ送る指令はそのままでは通らない**。
- 連鎖（`w[i]` を `w[i-1]` の直後へ・先頭は動かさない）は所有関係を知らないため、
  4 枚の指定では 2 段目で `b1` を `s0` の直後へ送ろうとする。
- **有利な材料**: `zorder_group_order_tests.rs:221` は**所有関係を 1 本も持たない**実窓 4 枚で
  同じ 3 段連鎖を一括投入して成立させている（§4.4）。すなわち「3 段連鎖・一括投入」自体は
  所有関係が無ければ通る。
- **不利な材料**: 4 枚のうち `b1` だけを外した 3 枚が通る理由を、所有関係だけでは説明しきれない
  （`s0,b1` の 1 手は所有される窓を動かして**成立している**）。

#### 候補 B: 同一 `DeferWindowPos` バッチ内の解決順

- 本 spec の task 7.1 の申し送りが既に登記している——**同一バッチ内の `hwndInsertAfter` は
  解決順に依存する／バッチ投入は挿入先が同バッチで動かないときだけ安全**。
- 本番の `enqueue_group_chain`（`crates/wintf/src/ecs/window/zorder_group_maintain.rs:122-127`）は
  **挿入先が同バッチで動く**形でそのまま積む。実測でも書込 32 本の **32/32 が
  `via="DeferWindowPos" in_batch=true`**（`diagnostics\diag-write.log`）＝縮退経路は 1 度も通っていない。
- **不利な材料**: `b0,s0,s1` も 2 本の移動を同一バッチに積み、2 本目 `s1@s0` の挿入先 `s0` は
  同バッチで動く。それでも成立している。よって「挿入先が同バッチで動く」だけでは失敗を説明できず、
  **段数（2 本 → 3 本）が閾になっている**という形でしか候補 B は残らない。

#### 切り分けの方法（**本サインオフの境界外**）

4 枚の連鎖を 1 つのバッチに積まず**1 件ずつ flush** して同じ走行を行えばよい。
成立すれば根因は候補 B、成立しなければ候補 A である。
これは本番コード（`zorder_group_maintain.rs`／flush 経路）の変更を要するので task 7.4 では行わない。
**是正の担当者が最初に行う 1 手として残す。**

#### どちらであっても結論は同じ

**数値モードは常に `[Balloon, Char]` へ展開する**（`zorder_group_ledger.rs:282-...`）ので、
2 スコープの数値指定は必ず 4 枚・3 段の形になる。**したがって `seriko.zorder,0,1` と
`\![set,zorder,0,1]`——正典の主用法——は現状の実機で一度も成立しない。**

### 4.4 なぜ決定論テストが素通りしたか（檻の盲点）

`crates/wintf/src/ecs/window/zorder_group_order_tests.rs:221`
`a_group_chain_lands_in_the_declared_order_on_both_write_paths` は
**実窓 4 枚**で連鎖を流し、宣言どおりの最終形に着くことを一括投入・逐次の両経路で確かめている。
しかしそこで作る窓は `CreateWindowExW(..., hWndParent = None, ...)`（同ファイル `:85-107`）＝
**所有関係を 1 本も持たない素のトップレベル窓**である。

**本番の窓は必ず 2 組の所有関係を持つ。** 檻はその前提を自前で作らないので、
所有関係のもとでだけ現れる失敗を構造的に観測できない。
（「実機サインオフは檻が隠す欠陥を炙り出す」の典型例であり、
`zorder_group_order_tests.rs` に**所有関係を張った 4 枚**の変種を足せば決定論側で閉じられる。）

**この檻は §4.3 の候補 B にとって不利な材料でもある。** 檻は本番と同じ
「4 枚・3 段の連鎖を 1 つの `DeferWindowPos` バッチへ・各段の挿入先は同バッチで動く」形を
そのまま流して**成立させている**。本番との差は所有関係の有無だけなので、
**候補 A（所有関係）の方が有力**である。ただし檻の窓は 0x0・不可視・`WS_POPUP` の道具窓であり、
本番の合成窓とは属性が異なる（実行経路そのものは
`flush_window_pos_commands` で共通）。**候補 B を消したとまでは言えない**ので、
§4.3 の切り分け 1 手は依然として要る。

### 4.5 この欠陥の担当

本 spec の要件 1.1／1.2（宣言した重なりを固定する）の中核であり、**本 spec の外に引受先は無い**。
是正は `enqueue_group_chain`／`plan_group_fixes`（`zorder_group.rs`・`zorder_group_maintain.rs`）の
連鎖の組み方（候補 A なら「所有関係を持つ窓は所有者を動かして従わせる」・
候補 B なら「連鎖は 1 件ずつ flush する」）に踏み込むため、
**task 7.4 の境界（サインオフ）の外**である。開発者裁定を要する。

---

## 5. J3（既存ペア機構の記録が従来どおり・要件 9.5）

5 走行すべてで **`[zorder-pair] owner-established` 2 件**（2 スコープ分）・
**`fix` 1 件 ＋ `skip` 1 件**が出た。字面も従来どおりである。

```
（R1・グループ指定なし）
05:58:58.751464Z INFO  [zorder-pair] owner-established entity=21v0 peer=22v0
                        owned_hwnd=0x4B6107A owner_hwnd=0x4870F82 measured_prev=0x153182C
05:58:58.830099Z DEBUG [zorder-pair] fix entity=21v0 peer=22v0
                        insert_after=0x153182C measured_next_after_fix=0x4870F82
05:58:58.830157Z DEBUG [zorder-pair] skip entity=23v0 peer=24v0 reason=AlreadyAdjacent
```

| 走行 | owner-established | fix | skip |
|---|---|---|---|
| R1 指定なし | 2 | 1 | 1 |
| R2 設定由来 | 2 | 1 | 1 |
| R3 タグ由来 | 2 | 1 | 1 |
| R4 数値モード | 2 | 1 | 1 |
| R5 解釈不能値 | 2 | 1 | 1 |

**グループ機構を載せた走行でもペア機構の記録は 1 件も増減していない。**

### 5.1 「従来どおり」の錨は本 spec 導入**前**の記録である

「従来どおり」を今回の走行どうしの比較だけで言うと、**本 spec の導入で欄が変わっていても
全走行が同じように変わるので気づけない**。よって錨は本機能が存在しなかった時点の実機記録に張る——
`.kiro/specs/completed/areka-P0-ghost-window-zorder/verification/plan-a-gate.md:51-54`。

```
（導入前・plan-a-gate.md:51-54）
[zorder-pair] owner-established entity=3v0 peer=4v0 owned_hwnd=0x670AD0 owner_hwnd=0x4A0AFC measured_prev=0xBA09CE
[zorder-pair] owner-established entity=5v0 peer=6v0 owned_hwnd=0x410AF2 owner_hwnd=0xBA09CE measured_prev=0x670AD0
[zorder-pair] skip entity=5v0 peer=6v0 reason=AlreadyAdjacent
[zorder-pair] fix  entity=3v0 peer=4v0 insert_after=0xBA09CE measured_next_after_fix=0x4A0AFC
```

**欄立てが完全に一致する**——`owner-established` は
`entity=` / `peer=` / `owned_hwnd=` / `owner_hwnd=` / `measured_prev=` の 5 欄、
`fix` は `entity=` / `peer=` / `insert_after=` / `measured_next_after_fix=`、
`skip` は `entity=` / `peer=` / `reason=`。**記録の組み合わせも同じ**——確立 2 件のあと、
片方のペアは `reason=AlreadyAdjacent` で見送られ、もう片方は `fix` される。値（HWND・entity 世代）
だけが走行ごとに違う。

なお `fix` と `skip` の**前後は走行ごとに入れ替わる**（R1・R3・R5 は `fix` が先、R2・R4 は `skip` が先。
導入前の記録は `skip` が先）。どちらのペアが先に評価されるかはクエリの巡り順に依るので、
**順序は「従来どおり」の判定材料にしていない**。

`signoff-scan.ps1` はこの 5 欄が全 `owner-established` 行に揃うことを機械で照合する。
較正は変異注入で行った——`measured_prev=` 欄だけを落とした複製で **J3=FAIL / exit 1**、
素の原本で **J3=PASS / exit 0**（`signoff-procedure.md` §6 の 10・11 行目）。

**この判定が保証しないこと**: 件数の述語は `owner-established == 2` かつ `fix+skip >= 1` なので、
**グループ機構がペア是正の回数を増やしても素通りする**。導入前の記録は同じ 2 分走行のものではなく、
回数を比べる条件が揃わないためこの形にした。保証しているのは
「記録が出続けていること」と「欄立てが変わっていないこと」の 2 点である。
回数の不変は実機ではなく**既存ペア機構の本番 5 ファイルの差分が 0 であること**が担う——
`main..HEAD` の変更 228 本に `zorder_pair.rs`／`zorder_pair_diag.rs`／`zorder_pair_establish.rs`／
`zorder_pair_maintain.rs`／`zorder_pair_sink.rs` は 1 本も含まれない（変わっているのは
`zorder_pair*` のテスト 3 本のみ・2026-08-29 実測）。

> **⚠ 2026-08-30（task 6.1）に測り直した。上の「5 本とも差分 0」は本版では偽である。**
> `origin/main...HEAD` の変更 **71 本**のうち `zorder_pair*` は **2 本**——テスト 1 本
> （`zorder_pair_deferred_vocabulary_tests.rs`）と、**本番の `zorder_pair_maintain.rs`（+7/-2）**。
> ただしその差分は**説明文の訂正だけ**で、実行される行は 1 行も動いていない
> （「スコープをまたぐ owner はそもそも存在しない」という記述が、鎖の横断 edge の着地で偽になった）。
> **記録の語彙を持つ `zorder_pair_diag.rs` は変更ファイルに含まれない。**
> 「含まれない」が空振りでないことは、同じ問いに 71 という非 0 の対照が出ていることで言える。
> 詳細は `signoff-procedure.md` §2.0。

なお `[zorder-group] skip group_id=- reason=PairFixThisPass` は
**同じ巡にペア是正が出ていればグループ側は動かない**という調停の記録であり、
既存機構が優先されていることの証跡である（要件 6.3 の調停）。実測は **R2・R4 が各 1 件、
R1・R3・R5 が 0 件**——ペア是正の巡にグループが既に載っているかどうかで決まる
（R3 はタグが届くのがペア是正の巡より後）。**「毎走行 1 件」ではない。**

**J3 の判定に `origin=zorder-pair` の件数は使っていない**——グループ発行の指令は
凍結済み `pair_fix_command` 経由で書かれるため、書込側の `origin` 欄では
グループ発行分がペア発行分に見えるからである（tasks.md 申し送り 4.1）。

---

## 6. 実機で確認**できていない**こと（本記録の射程外）

> ⚠ **2026-08-30（task 6.2）: 本節は初版の走行についての射程である。**
> 下の 3 点のうち**解除の実機実行**と**利用者の操作による活性化**は第 2 版の走行で実測した（§8.3／§8.4）。
> 第 2 版でなお実機の外に在るのは 2 点——**3 スコープ以上**（emo2 が 2 つしか持たないため・下の 3 つ目と同じ理由）と、
> **窓が現れる側の出入り**（§8.4⑶ に理由と引受先を書いた）である。

- **`\![reset,zorder]`（解除）の実機実行。** 本走行の台本は `set` だけを実行した。
  解除の分岐は決定論テストで網羅済み（task 群 1・3）。
- **利用者のクリックによる重なり変化への追随（要件 6.2 の能動側）。** 本サインオフは
  目視・手操作を使わない方針のため測っていない。追随そのものは定期トークに伴う
  バルーンの表示・消去で 6〜7 回駆動され、いずれも `AlreadyOrdered` で落ち着いている。
- **3 スコープ以上のグループ。** emo2 は 2 スコープ（`sakura`／`kero`）しか持たない。

---

## 7. 第 2 版（所有の鎖）の実機サインオフ（2026-08-30・task 6.2）

> **本節が現行版の受け入れ記録である。** §1〜§6 は初版（毎巡の観測＋是正）の一次証跡であり、
> 実装も判定語も違う。両者を混ぜて読まないこと。判定手順は `signoff-procedure.md`
> （§5.1 の判定式・§6.1 と §6.2 の較正）が正典である。

### 7.0 結論

**8 走行すべてで判定の道具が期待どおりの終了コードを返した。**
初版を実機 NO-GO にした構成——**窓 4 枚**（数値モード `0,1` と明示モード `b0,s0,b1,s1`）——は
**どちらも成立した**。宣言と実測が同一行で一致し、`link-failed` は全走行で 0 件である。

| 走行 | 検体・指定 | `-Mode` | 判定 | 終了コード |
|---|---|---|---|---|
| **R1** 指定なし | `emo2`（共有・無改変） | `default` | J2=PASS J3=PASS | **0** |
| **R2** 明示 4 枚 | `seriko.zorder,b0,s0,b1,s1` | `grouped` | J1=PASS J3=PASS | **0** |
| **R3** 数値モード | `seriko.zorder,0,1` | `grouped` | J1=PASS J3=PASS | **0** |
| **R4** タグ set → reset | `\![set,zorder,b0,s0,b1,s1]` → `\![reset,zorder]` | `grouped` | J1=PASS J3=PASS | **0** |
| **R5** 活性化（部外の窓） | `seriko.zorder,b0,s0,b1,s1` ＋ 外部から活性化 | `grouped` | J1=PASS J3=PASS | **0** |
| **R6** 活性化（鎖の窓 3 枚） | 同上 ＋ 鎖の根・先頭・奥から 2 枚目を活性化 | `grouped` | J1=PASS J3=PASS | **0** |
| **R7** 有効中の再指定 | タグで `b1,s1,b0,s0` → 途中で `b0,s0,b1,s1` | `grouped` | J1=PASS J3=PASS | **0** |
| **R8** 解除してから組み替え | タグで `b1,s1,b0,s0` → 途中で `\![reset,zorder]\![set,zorder,b0,s0,b1,s1]` | `grouped` | J1=PASS J3=PASS | **0** |

- **目視は 1 度も使っていない。** J1／J2／J3 はすべて `signoff-scan.ps1` が記録の照合だけで下した。
- R5／R6 の**外部からの重なり実測**（`*.probe.txt`）は判定材料ではなく、活性化の前後で
  実際の重なりが動いていないことを機械で測った**副次の証跡**である。

### 7.1 実施条件

| 項目 | 値 |
|---|---|
| 実施日時 | 2026-08-30（観測区間 `14:34:38Z` 〜 `14:58:09Z`・UTC） |
| ブランチ / HEAD | `claude/areka-p0-zorder-pinning-8e3e7c` / `488dad20`（task 6.1 まで） |
| ゴースト | 実 emo2 fixture・**実 pasta.dll**・辞書込みフルゴースト（発話が返っていることは `seriko: bind 適用` と `[balloon-visibility] バルーンの可視状態が遷移した` で確認） |
| 起動 | **絶対パス**（ghost＝各検体のルート／balloon＝`fixtures\emo2\emo2-kakukaku`） |
| 有界終了 | `AREKA_APP_SMOKE_EXIT_MS=120000`（2 分）× 8 走行。**耐久走行は 1 本も行っていない**（開発者指示） |
| ログ水準 | `RUST_LOG=info,wintf::ecs::window=debug` |
| helper | i686 版 `shiori-host32-helper.exe` を `target\debug\` へ配置済み |
| ビルド | debug プロファイル（走行前に `cargo build -p areka` を再実行） |
| モニタ | 実効 192dpi（全走行で `k_shell=2.0 k_balloon=2.0`） |

### 7.2 検体

| 走行 | ゴースト | 指定 |
|---|---|---|
| R1 | `fixtures\emo2`（**共有 fixture・無改変**） | 無し |
| R2・R5・R6 | `fixtures\emo2-zsp-descript` | `shell\master\descript.txt` に `seriko.zorder,b0,s0,b1,s1` |
| R3 | 同上（1 行だけ差し替え） | `seriko.zorder,0,1` |
| R4 | `fixtures\emo2-zsp-tag` | `boot.pasta` の起動 12 パターンに `\![set,zorder,b0,s0,b1,s1]`・`talk.pasta` の通常トーク 48 シーンに `\![reset,zorder]` |
| R7 | `fixtures\emo2-zsp-tag2` | 起動に `\![set,zorder,b1,s1,b0,s0]`・通常トークに `\![set,zorder,b0,s0,b1,s1]` |
| R8 | `fixtures\emo2-zsp-tag3` | 起動に `\![set,zorder,b1,s1,b0,s0]`・通常トークに `\![reset,zorder]\![set,zorder,b0,s0,b1,s1]` |

**共有 fixture（`fixtures\emo2`）は 1 バイトも書き換えていない**（走行後に
`git status --short crates/pilot/examples/shiori-host-32/fixtures/emo2` が空であることを確認）。
派生検体 4 本は**走行後に片付けた**。作り方の正本は `signoff-procedure.md` §3.1／§3.2／**§3.5**、
片付けの安全な順序は同 §3.2.1 である。

### 7.3 ログの所在（一次証跡・読み取り専用）

`C:\Users\maz-o\AppData\Local\areka-diag\zsp-signoff-r2-20260830\`

| ログ | サイズ | md5 |
|---|---|---|
| `R1-default.log` | 78,619 bytes | `FCADEA461A3E1CF0266B35CC6E73B94C` |
| `R2-descript-explicit4.log` | 84,394 bytes | `6164F1F8A7044740161A56296EABB37C` |
| `R3-descript-numeric.log` | 86,493 bytes | `99E30B5EF7FEDBC3E42662ED33C45B71` |
| `R4-tag-set-reset.log` | 90,741 bytes | `556B986B95487B4B5FDDB184FE5FA5CC` |
| `R5-activation.log` | 84,411 bytes | `856664A895116CF84E9D230903792848` |
| `R5-activation.probe.txt` | 929 bytes | `769A6CC0C3F4BC212F3D32F673D7B5E2` |
| `R6-activation-chain.log` | 87,613 bytes | `A9A891466A8262F6D4FE3634D5C0171C` |
| `R6-activation-chain.probe.txt` | 1,173 bytes | `95EDA6858796B989B63129A5D98282A6` |
| `R7-tag-rechain.log` | 83,380 bytes | `B72C2870C2CFCE0B1ACEB700E14B640D` |
| `R8-tag-reset-then-set.log` | 96,070 bytes | `063B101BFBC7B6D3F7ABC50288AF794E` |

ログはコミットしない。**再採取の手順が正本**である（`signoff-procedure.md` §4.2／§3.5）。

### 7.4 タグ別の件数（全走行・冠込みで数えた）

| 走行 | applied | rejected | linked | unlinked | settled | link-failed | absent | skipped | pair owner / fix / skip | pair sink-observed |
|---|---|---|---|---|---|---|---|---|---|---|
| R1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 / 1 / 1 | 0 |
| R2 | 1 | 0 | 1 | 0 | 1 | 0 | 4 | 1 | 2 / 1 / 1 | 0 |
| R3 | 1 | 0 | 1 | 0 | 1 | 0 | 4 | 1 | 2 / 1 / 1 | 0 |
| R4 | 4 | 0 | 1 | 1 | 1 | 0 | 0 | 1 | 2 / 1 / 1 | 0 |
| R5 | 1 | 0 | 1 | 0 | 1 | 0 | 4 | 1 | 2 / 1 / 1 | 0 |
| R6 | 1 | 0 | 1 | 0 | 1 | 0 | 4 | 1 | 2 / 1 / 1 | **2** |
| R7 | 1 | **1** | 1 | 0 | 1 | 0 | 4 | 1 | 2 / 1 / 1 | 0 |
| R8 | 9 | 0 | 2 | 1 | 2 | 0 | 4 | 4 | 2 / 1 / 1 | 0 |

`unlink-failed`・ペア側の `verify-failed`・`owner-establish-failed` は**全 8 走行で 0 件**である。
**`[zorder-pair] sink-observed` が R6 で 2 件出た**——`signoff-procedure.md` §2.0 が
「健全な無人走行では原理的に出ない」と書いていた 3 タグの 1 つで、**外から活性化を与えたこの走行で
初めて実出した**（同節へ訂正註を入れた）。J3 はこのタグを判定に使っていないので判定は不変である。

---

## 8. task 6.2 の 5 つの箇条ごとの証跡

### 8.1 有界時間で自動終了する実機実行を行い、ログだけで判定する

8 走行とも `AREKA_APP_SMOKE_EXIT_MS=120000` の**有界の 1 回走行**で、終了コード 0 で自動終了した。
判定は `signoff-scan.ps1` の終了コードだけで下している（§7.0 の表・§9 の較正）。
**耐久走行・繰り返し走行は 1 本も行っていない。**

### 8.2 数値モードと明示モードの両方で**窓 4 枚**が宣言どおりに成立する

**明示モード（R2）**——`seriko.zorder,b0,s0,b1,s1`:

```
14:36:55.002531Z DEBUG wintf::ecs::window::zorder_chain_diag:
  [zorder-group] applied action=set group_id=0 source=Descript members=b0,s0,b1,s1 normalized=0:false,1:false

14:36:55.161703Z DEBUG wintf::ecs::window::zorder_chain:
  [zorder-chain] linked segment=g0 owned=22v0 owner=23v0
                 owned_hwnd=0x270E0908 owner_hwnd=0x21940CF2 pos=2/4

14:36:55.162729Z DEBUG wintf::ecs::window::zorder_chain:
  [zorder-chain] settled nudged_hwnd=0x20B41714 insert_after=0x21940CF2
                 declared=0x1DDA1884,0x270E0908,0x21940CF2,0x20B41714
                 measured=0x1DDA1884,0x270E0908,0x21940CF2,0x20B41714 nudge_ok=true
```

**数値モード（R3）**——`seriko.zorder,0,1`（**初版が `verify-failed` 24 件を出して 1 度も成立しなかった形**）:

```
14:40:10.167247Z DEBUG wintf::ecs::window::zorder_chain_diag:
  [zorder-group] applied action=set group_id=0 source=Descript members=b0,s0,b1,s1 normalized=-

14:40:10.318823Z DEBUG wintf::ecs::window::zorder_chain:
  [zorder-chain] settled nudged_hwnd=0x24A10FDE insert_after=0x270F0908
                 declared=0x21950CF2,0x24F30F8E,0x270F0908,0x24A10FDE
                 measured=0x21950CF2,0x24F30F8E,0x270F0908,0x24A10FDE nudge_ok=true
```

- 数値の `0,1` が **`members=b0,s0,b1,s1` の 4 枚へ展開**されている（`normalized=-` は数値モードの番兵）。
  すなわち**初版が落ちた「4 枚・スコープ 2 つ」の構成そのもの**である。
- `linked` は **1 本**である（4 枚に対し繋ぎ 1 本）。同一スコープの 2 枚は既存ペア機構が張るので
  鎖の繋ぎには数えない（design DD-2）。`pos=2/4` が宣言列の 2 番目の隣接対を名指しする。
- **繋ぐ前の重なりは宣言と違っていた。** R3 の最初の `[zorder-pair] owner-established` は
  `owned_hwnd=0x21950CF2 owner_hwnd=0x24F30F8E measured_prev=0x24A10FDE` であり、
  **宣言列の最後尾 `0x24A10FDE` が、宣言列の先頭 `0x21950CF2` の 1 つ手前に居た**。
  並べ替えは実際に起きている（同じことを R8 が 1 走行の中で 2 通りの順で示す・§8.4⑵）。

### 8.3 解除が実機で 1 度以上実行され、既定状態へ戻る（R4）

```
14:43:28.819630Z DEBUG wintf::ecs::window::zorder_chain_diag:
  [zorder-group] applied action=reset groups=0 base=-
14:43:28.820228Z DEBUG wintf::ecs::window::zorder_chain:
  [zorder-chain] unlinked segment=g0 owned=22v0 owned_hwnd=0x21531092 owner_hwnd=0x24F40F8E reason=Teardown
14:43:28.820268Z DEBUG wintf::ecs::window::zorder_chain:
  [zorder-chain] skipped reason=TooFewPresent
```

- 解除で**鎖が実際にほどけた**（`unlinked reason=Teardown`）。続く見送りの理由 `TooFewPresent` は、
  望む鎖が空になり後押しの対象が 2 枚未満になったことを意味する。
- **以後 61 秒間、`[zorder-chain]` の記録は 1 行も出ない**（機械で数えて 0 件）。
  区間は**解除の `14:43:28.820268Z` から走行の最終行 `14:44:29.790817Z` まで**である
  （走行の開始は `14:42:29.112923Z`・有界 120 秒）。
  同じ通常トークが再度 `\![reset,zorder]` を送っても `applied action=reset` が 2 件出るだけで、
  鎖の仕事は 1 つも起きない＝**冪等**であり、既定状態のまま留まっている。
- ペア機構は解除の後も従来どおり動いている（J3=PASS）。

### 8.4 活性化のあとも順が保たれる／グループ有効中に鎖が組み替わる

#### ⑴ 利用者の操作による活性化（R6・R5）

外部から `SetForegroundWindow` ＋ `BringWindowToTop` を**鎖の窓 3 枚**へ順に与えた——
**a1＝根 `0x2A690ECA`**（`14:49:05.939Z`）・**a2＝先頭 `0x24571268`**（`14:49:18.965Z`）・
**a3＝奥から 2 枚目 `0x1DB81892`**（`14:49:31.997Z`）。刺激が届いたことの証言は
**3 発すべてに直接付いているわけではない**ので、付いている側と付いていない側を分けて書く。

**⑴ 直接の証言があるのは a2 と a3 の 2 発だけである。**

```
14:49:18.960452Z DEBUG wintf::ecs::window::zorder_pair:
  [zorder-pair] sink-observed entity=23v0 adjacency_ok=true foreground=0x24571268 behind_foreground=true
14:49:31.991429Z DEBUG wintf::ecs::window::zorder_pair:
  [zorder-pair] sink-observed entity=21v0 adjacency_ok=true foreground=0x1DB81892 behind_foreground=false
```

この 2 件の時刻は外から活性化を出した時刻と一致し（記録の方が数 ms 早いのは、probe が
`SetForegroundWindow` を**呼んだ後**に時刻を採るためである）、`foreground=` は
**こちらが活性化させた窓そのもの**である。

**⑵ a1（根の活性化）には自己検査が 1 つも無い。** この巡には `sink-observed` が出ておらず、
probe の並びも `t1` と `t2` で同一（差分ゼロ）である。**「アプリ自身のログが 3 発とも証言している」
とは書けない。**

**⑶ ただし a1 の到達は a2 の記録が事後に含意する。** `sink-observed` の目印を付ける
`mark_pair_sink_observation` は **`WM_ACTIVATE` の非活性化枝からだけ**呼ばれ、
**非活性化された窓が属する対**へ付く（`crates/wintf/src/ecs/window/zorder_pair_sink.rs:52-53,87`）。
a2 の行は `entity=23v0`——R6 の `owner-established` によれば
**対 `(23v0, 24v0)` ＝ `{0x1DB81892, 0x2A690ECA}`** である。すなわち a2 の瞬間に
**`0x2A690ECA` を含む対が活性から降りた**のであり、その活性を作れたのは
**a1 の `SetForegroundWindow(0x2A690ECA)` 以外に無い**——**a2 の時点より前に、この対の窓を
前面へ出した操作は a1 の 1 発だけである**（probe の全操作が `R6-activation-chain.probe.txt` に載っている。
⚠ a3＝`hwnd=0x1DB81892` は**同じ対のもう 1 枚**だが `14:49:31.9970085Z` であり、a2 の `sink-observed`
`14:49:18.960452Z` より **13 秒後**なので、a2 の目印の出所にはなり得ない。限定の射程は
「走行全体」ではなく「**a2 以前**」である）。
a3 についても同型で、`entity=21v0`＝対 `(21v0, 22v0)` ＝ `{0x24571268, 0x1E88190E}` が降りた
＝**a2 の到達**を裏から示す。

**⑷ よって射程はこうである**——a2・a3 は**直接**、a1 は**a2 の非活性化からの含意**で到達を確かめた。
`sink-observed` は活性化由来の記録であり無人走行では出ない（§7.4 の註）。

結果:

- **重なりは 1 度も動かなかった。** 外部実測（probe）は `t0`／`t1`／`t2`／`t3`／`t4`／`t5` の
  6 点すべてで `0x24571268,0x1E88190E,0x1DB81892,0x2A690ECA`（＋鎖外の窓 1 枚）であり、
  これはログの `declared=` と**逐語で同一**である。
- **是正の往復が 1 度も起きていない。** 起動時の `settled`（`14:48:55.770Z`）から
  終了時の後片付け（`14:50:55.63Z`）まで、`[zorder-chain]` の記録は **0 行**である。
  すなわち順は**観測と是正ではなく所有の鎖の構造**で保たれている（要件 14.3／design §12.1）。
- **鎖が一体で前へ出ることも測れた（R5）。** R5 では先に**鎖の外の窓**を活性化して
  鎖全体の手前へ出し（`t2`＝`0x22661480,0x25CB0C1C,0x1E500C60,0x22020EFE,0x1DB71892`）、
  次に**鎖の窓 1 枚**を活性化した。すると `t3` で鎖の 4 枚が
  `0x25CB0C1C,0x1E500C60,0x22020EFE,0x1DB71892` の**宣言どおりの並びのまま一体で前へ戻り**、
  部外の窓が最背面へ下がった。**1 枚だけが飛び出す形にはならない。**

#### ⑵ グループ有効中の組み替え（R8）

解除してから別の順を指定すると、**同じ走行の中で鎖が組み直された**:

```
14:56:10.278066Z [zorder-group] applied action=set group_id=0 source=Tag members=b1,s1,b0,s0 normalized=1:false,0:false
14:56:10.279252Z [zorder-chain] settled ... declared=0x2361111A,0x2336104C,0x21021236,0x21571092
                                            measured=0x2361111A,0x2336104C,0x21021236,0x21571092 nudge_ok=true

14:56:40.708958Z [zorder-group] applied action=reset groups=0 base=-
14:56:40.709033Z [zorder-group] applied action=set group_id=1 source=Tag members=b0,s0,b1,s1 normalized=0:false,1:false
14:56:40.710075Z [zorder-chain] unlinked segment=g0 owned=24v0 owned_hwnd=0x2336104C owner_hwnd=0x21021236 reason=Teardown
14:56:40.710270Z [zorder-chain] linked   segment=g1 owned=22v0 owner=23v0 owned_hwnd=0x21571092 owner_hwnd=0x2361111A pos=2/4
14:56:40.711056Z [zorder-chain] settled ... declared=0x21021236,0x21571092,0x2361111A,0x2336104C
                                            measured=0x21021236,0x21571092,0x2361111A,0x2336104C nudge_ok=true
```

- **2 つの宣言列は互いにスコープ 2 つ分を入れ替えた形**（前半＝スコープ 1 が手前／後半＝スコープ 0 が手前）
  であり、**どちらも実測が宣言と一致した**。同一走行の中で**逆の並びが 2 度とも成立している**ので、
  「たまたま起動時の並びが宣言と同じだった」という読みは成り立たない。
- 判定の道具は `宣言列 2 本中 一致 2 / 不一致 0` と印字した——**⒝（宣言列ごとの終状態の全称）が
  実走のログで複数の宣言列に対して働いた初めての例**である（§9 の 8 行目）。
- 以後の通常トークが同じ `reset`＋`set` を繰り返しても `skipped reason=NoChange` になるだけで、
  鎖は組み替わらない（内容が同じ巡は公開しない・要件 14.5）。

#### ⑶ 窓の出入り（**去る側だけを実機で観測した**・射程の明示）

グループが有効なまま**窓が去る**側は観測できた——終了時に 4 枚が一斉に despawn され
（`areka: smoke 自動 close: 起動窓（ダミー窓／ゴースト窓）を despawn しました count=4`）、鎖は

```
14:38:55.014672Z [zorder-chain] absent group_id=0 element=b0   （s0 / b1 / s1 と 4 行）
14:38:55.015083Z [zorder-chain] skipped reason=NoChange
```

へ組み替わった（宣言要素が 1 つも実在しなくなった＝望む鎖が空）。

**窓が現れる側は、この検体では実機で観測できない。** emo2 のゴースト窓は
`crates/areka/src/placement/spawn.rs` の一度きりの生成でスコープ 2 つ分（4 枚）が揃い、
実行中に窓が増える経路が本番に無いためである（走行中の窓の生成・破棄の記録は起動と終了以外に 1 件も無い）。
**この側の担保は決定論テスト**
（`crates/wintf/src/ecs/window/zorder_chain_order_lifecycle_tests.rs`・task 4.2）が持つ。
**射程の外であることを、実機で確かめたことと混ぜて書かない。**

**時刻順がその理由を裏づける**（R3 の例）:

```
14:40:10.167247Z  [zorder-group] applied …            ← 受理（窓はまだ 1 枚も無い）
14:40:10.240798Z  WindowHandle added entity=22v0      ← ここから 4 枚が 5.3 ms の間に揃う
14:40:10.242431Z  WindowHandle added entity=24v0
14:40:10.244366Z  WindowHandle added entity=21v0
14:40:10.246124Z  WindowHandle added entity=23v0
14:40:10.317893Z  [zorder-chain] linked …             ← 鎖が働くのは 4 枚が揃った後
14:40:10.318823Z  [zorder-chain] settled …
```

受理は窓の生成より **約 74 ms 先行**し（`.167247` → `.240798`）、窓は**まとめて**現れる。よって「グループが有効な最中に窓が 1 枚ずつ
増える」巡が実機に存在せず、**起動時の `[zorder-chain] absent` は 8 走行とも 0 行**である
（`absent` が出るのは終了時の一斉 despawn の後だけで、8 走行のいずれでも最初の `absent` は
`smoke 自動 close` の行より後にある）。

### 8.5 指定が一つも無い状態では鎖の記録が 1 行も出ない（R1）

無改変の共有 fixture での 2 分走行で、**`[zorder-group]` と `[zorder-chain]` の両方の冠が 0 件**:

```
[zorder-group] 合計 0   applied 0 / rejected 0
[zorder-chain] 合計 0   linked 0 / settled 0 / link-failed 0
[zorder-pair]  合計 4   owner-established 2 / fix 1 / skip 1
```

「0 件」は対照を添えて初めて意味を持つ（`signoff-procedure.md` §2.4）。**同じ道具・同じ語**を
R2／R3 に当てると **J2=FAIL / exit 1** になる（§9 の 9・10 行目）ので、R1 の 0 件は
「語が空振りしている」ではなく「本当に記録が無い」である。

---

## 9. 本走行での判定器の較正（実走ログ 11 通り・3 種類の終了コードが出た）

| # | 当てたログ | `-Mode` | 何を確かめるか | 実測 |
|---|---|---|---|---|
| 1 | `R1-default.log` | `default` | 既定＝非強制が緑 | **J2=PASS J3=PASS / exit 0** |
| 2 | `R2-descript-explicit4.log` | `grouped` | 明示 4 枚が緑 | **J1=PASS J3=PASS / exit 0** |
| 3 | `R3-descript-numeric.log` | `grouped` | **数値 4 枚が緑**（初版の NO-GO 構成） | **J1=PASS J3=PASS / exit 0** |
| 4 | `R4-tag-set-reset.log` | `grouped` | タグ由来＋解除が緑 | **J1=PASS J3=PASS / exit 0** |
| 5 | `R5-activation.log` | `grouped` | 活性化を挟んでも緑 | **J1=PASS J3=PASS / exit 0** |
| 6 | `R6-activation-chain.log` | `grouped` | 鎖の窓を 3 枚活性化しても緑 | **J1=PASS J3=PASS / exit 0** |
| 7 | `R7-tag-rechain.log` | `grouped` | 有効中の再指定が拒否されても、成立済みの鎖の判定は緑 | **J1=PASS J3=PASS / exit 0** |
| 8 | `R8-tag-reset-then-set.log` | `grouped` | **宣言列 2 本の終状態がどちらも一致**（⒝ の全称が実走で複数列に効く） | **J1=PASS J3=PASS / exit 0**（`宣言列 2 本中 一致 2 / 不一致 0`） |
| 9 | `R2-descript-explicit4.log` | `default` | **J2 の「0 件」の非 0 対照** | **J2=FAIL J3=PASS / exit 1** |
| 10 | `R3-descript-numeric.log` | `default` | 同上（2 本目の対照） | **J2=FAIL J3=PASS / exit 1** |
| 11 | `R1-default.log` | `grouped` | 受理が無い走行は FAIL ではなく**判定不能** | **J1=INCONCLUSIVE J3=PASS / exit 2** |

- **exit 3（引数不正）は本走行では作っていない**——道具側の較正であり `signoff-procedure.md`
  §6.1.2 の 21 行目が持つ。本節は**実走ログだけ**の較正である。
- 9・10 行目が §2.4 の対照であり、11 行目が「沈黙を PASS と読ませない」ことの確認である。

### 9.1 予定になかった実出が 2 つある（どちらも記録しておく）

1. **`[zorder-group] rejected reason=CrossGroupRedesignation(0,1) tokens=b0,s0,b1,s1`**（R7・`14:54:03.701Z`）
   ——グループが有効な間に**同じスコープを別のグループへ指名し直すタグ**は拒否される、という
   裁量が**実機で効いていることの証跡**である（task 6.3 が対応表へ登記する項目の 1 つ）。
   初版の `rejected` の実出は解釈不能値（`UnparsableToken`）1 通りだけだったので、**理由語が 2 通りになった**。
   拒否しても起動は続き、成立済みの鎖はそのまま保たれた（J1=PASS）。
2. **`[zorder-pair] sink-observed` 2 件**（R6）——§7.4 の註のとおり。
   `signoff-procedure.md` §2.0 の「19 本すべてで 0 件」は**無人走行についての主張**であり、
   活性化を外から与えた走行には当てはまらない（同節へ訂正註を入れた）。
