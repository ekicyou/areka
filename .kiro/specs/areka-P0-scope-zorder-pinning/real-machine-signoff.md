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

- **`\![reset,zorder]`（解除）の実機実行。** 本走行の台本は `set` だけを実行した。
  解除の分岐は決定論テストで網羅済み（task 群 1・3）。
- **利用者のクリックによる重なり変化への追随（要件 6.2 の能動側）。** 本サインオフは
  目視・手操作を使わない方針のため測っていない。追随そのものは定期トークに伴う
  バルーンの表示・消去で 6〜7 回駆動され、いずれも `AlreadyOrdered` で落ち着いている。
- **3 スコープ以上のグループ。** emo2 は 2 スコープ（`sakura`／`kero`）しか持たない。
