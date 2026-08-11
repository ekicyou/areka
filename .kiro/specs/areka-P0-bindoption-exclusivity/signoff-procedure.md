# emo2 実機サインオフ手順（areka-P0-bindoption-exclusivity）

対象要件: requirements.md **Requirement 5**（5.1・5.2・5.3・5.4・5.5・5.6）
判定式の正典: design.md **§Testing Strategy → Real Machine Sign-off（emo2・R5）— 5.1-5.6**

本書はタスク 7.1 の成果物であり、**判定手順の定義と既知ケース較正の記録**である。
実走そのもの（タスク 7.2）はまだ行っていない。

---

## 1. 判定の全体像

| 判定 | 対象要件 | 何を見るか | 担当 |
|---|---|---|---|
| **J1** | 5.2 | 各まばたき発火の直前の適用痕跡と id が一致すること・痕跡なしの発火は赤 | `signoff-scan.py`（機械判定） |
| **J2** | 5.4 | まばたきカテゴリの適用が途中から恒久沈黙する飽和パターンの不在 | `signoff-scan.py`（機械判定） |
| **J3** | 5.3 | ジト目へ切り替えた後、次の表情変更で表示が正しく切り替わる（再起動不要） | **開発者の目視サインオフ**（スクリプト対象外） |

J1・J2 の実測値は受け入れ記録（`real-machine-signoff.md`・要件 5.5）へそのまま転記できる形で出力される。

---

## 2. 実走の手順

### 2.1 事前準備（i686 helper の配置）

`areka.exe` は `current_exe()` の隣の `shiori-host32-helper.exe` を helper として解決する。
`cargo build --workspace` は **x64 版**を `target/debug/` に落とすため、32bit の
`pasta.dll` を `LoadLibrary` できるよう **i686 版を上書きコピー**しておく。

i686 ビルドは**必ず PowerShell** で行う（Git Bash の coreutils `link.exe` が MSVC の
`link.exe` を遮蔽する）。

```powershell
cargo build -p areka
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
Copy-Item target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe target\debug\ -Force
```

### 2.2 起動（PowerShell・絶対パス必須）

相対パスで起動すると helper の `LoadLibrary(pasta.dll)` が `0x8007007E`
（ERROR_MOD_NOT_FOUND）で失敗し、SHIORI が接続しない。**必ず絶対パス**で渡す。

```powershell
$ghost   = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2").Path
$balloon = (Resolve-Path "crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku").Path
$env:AREKA_APP_SMOKE_EXIT_MS = "420000"          # 7 分・有界 auto-exit
$env:RUST_LOG = "info,areka_seriko=debug"        # 適用痕跡を Changed/StateOnly/Unchanged 全て採る
& "target\debug\areka.exe" $ghost $balloon *> bindopt-signoff.log
```

- `AREKA_APP_SMOKE_EXIT_MS=420000` — 2026-08-11 の是正前観測実走と同一条件。
  表情変更が複数回起きるだけの長さが要る（要件 5.2 の「複数回切り替わったとき」）。
- `RUST_LOG=info,areka_seriko=debug` — **debug 込みが必須**。`Changed` は info だが
  `StateOnly` / `Unchanged` は debug で出る。debug を落とすと J1 の走査に穴が空く
  （設計ディスカッション #1 の裁定）。
- 実走中は**ゴーストを本番表示のまま放置**し、表情が何度も切り替わるのを待つ。
  J3 の目視はこの実走中に行う（§5）。

### 2.3 ログの保全

実走ログは上書き・再採取が効かない一次証跡である。判定にかける前に
`%LOCALAPPDATA%\areka-diag\<日時>\` 等へコピーし、以後は読み取り専用で扱う。

```powershell
$dst = "$env:LOCALAPPDATA\areka-diag\bindopt-signoff-$(Get-Date -Format yyyyMMdd-HHmmss)"
New-Item -ItemType Directory -Force $dst | Out-Null
Copy-Item bindopt-signoff.log $dst\
Get-FileHash $dst\bindopt-signoff.log -Algorithm MD5
```

---

## 3. 判定スクリプトの使い方

```
python .kiro\specs\areka-P0-bindoption-exclusivity\signoff-scan.py <ログファイルのパス>
```

- Python 標準ライブラリのみ（外部パッケージ不要）。`python` / `py` のいずれでも動く。
- 出力は日本語。J1・J2 の判定と**根拠となる実測値**（件数・id 内訳・違反の行番号と時刻・
  末尾 Changed の時刻差）を印字する。

終了コード:

| コード | 意味 |
|---|---|
| `0` | J1・J2 とも **PASS** |
| `1` | いずれかが **FAIL** |
| `2` | **判定不能**（まばたき発火が 0 件／seriko の行が 1 件も無い等。沈黙を PASS と誤判定しないための非ゼロ） |
| `3` | 引数不正・ログを読めない |

---

## 4. 判定式の説明

### 4.1 ログ行の実形（2026-08-11 実走で確認済み）

適用痕跡は `crates/areka-seriko/src/actor.rs`、発火は `crates/areka-seriko/src/looper.rs`
が出す。スクリプトはこの 3 種のみを拾う。

```
# Changed（info・実機 grep マーカー・actor.rs:405-412）
2026-08-11T01:19:34.139004Z  INFO actor{actor=seriko}: areka_seriko::actor: seriko: bind 適用 scope=0 category=まばたき part=通常 id=1400 on=true

# StateOnly / Unchanged（debug・actor.rs:416-423）
2026-08-11T01:19:38.087602Z DEBUG actor{actor=seriko}: areka_seriko::actor: seriko: bind 集合を更新（非表示/未知 scope または同値ゆえ発行なし） scope=0 category=まばたき part=通常 id=1400 on=true

# まばたき発火（looper・info）
2026-08-11T01:19:38.087688Z  INFO actor{actor=seriko}: areka_seriko::looper: seriko: loop 抽選発火（再生開始・先頭コマから・要件 2.1/2.2） scope="0" slot=Shell animation_id=1400 k=4
```

- **時刻形式**: `YYYY-MM-DDTHH:MM:SS.ffffffZ`（UTC・マイクロ秒以上の精度）。
- **カテゴリの同定**: 適用痕跡には `category=` フィールドが**素の日本語**で出る
  （`まばたき` / `目` / `口` / `眉` / `紅` / `腕`）。カテゴリ名から直接引ける。
- **発火のカテゴリ同定**: 発火行には `animation_id` しか無いため、
  **`1400 <= animation_id <= 1409`**（design.md の `140x`）をまばたきとみなす。
  この範囲は emo2 の `shell/master/descript.txt` の
  `sakura.bindgroup1400/1401/1402/1403.name,まばたき,…` に由来する **emo2 固有の値**である。
- **scope の表記ゆれ**: 適用痕跡は `scope=0`（素）・発火は `scope="0"`（引用符付き）。
  スクリプトは両者を正規化し、**同一 scope 内**で突き合わせる
  （是正前ログではまばたきの痕跡・発火とも全て scope 0 であり、scope を見ても見なくても結果は同じ）。
- 上記に当てはまらない行（他コンポーネント・先頭タイムスタンプの無い継続行）は
  **読み飛ばす**。パースエラーで落ちない。

### 4.2 J1（要件 5.2・共存痕跡の不在）

保全ログを**時刻順 1 パス**で走査し、`(scope, まばたきカテゴリ)` の直近の適用痕跡 id を
持ち回る。各まばたき発火（時刻 t・`animation_id=140x`）で:

- 直近のまばたき適用痕跡（**Changed / StateOnly / Unchanged のいずれも可**）が
  **存在しない** → **赤**。emo2 の静的既定集合に 14xx は無く、looper は `current_binds` で
  ゲートするため、痕跡なしの発火＝追跡外 bind＝欠陥。
- 直近痕跡の id が発火 id と**不一致** → **赤**。任意の時点で発火し得るまばたき id は
  高々 1 種であるべき（＝複数パーツの共存痕跡）。

まばたき発火が 0 件のときは **判定不能**（沈黙は PASS ではない）。

**発火の定義**: `loop 抽選発火` 行のみを数える。requirements.md §決定的証拠 の
「1400×156・1402×182」は `loop 抽選発火` と `loop 末尾残留` の**両方**を数えた値であり、
抽選発火だけなら 1400×78・1402×91（＋末尾残留が同数）である。両者は同じ現象の別表現で、
判定の向きは変わらない。

### 4.3 J2（要件 5.4・飽和パターンの不在）

実走全体で以下の 2 条件をともに満たすこと:

- **条件A**: `|Changed 回数(まばたき) − Changed 回数(目)| <= 2`
- **条件B**: 最後のまばたき Changed が最後の目 Changed の **120 秒以内**

まばたき・目とも Changed が 0 件なら **判定不能**（差 0 で条件Aを素通りしてしまうため）。
片方だけ 0 件なら **FAIL**（片側が全区間で沈黙＝まさに飽和の形）。

#### ⚠ この判定式は emo2 fixture 固有の較正値である

design.md が明記するとおり、**J2 は汎用則ではない**。成立の前提は
「emo2 ゴーストが毎回の表情変更で**目とまばたきをペア送信する**」という
2026-08-11 実走で直接観測した性質であり（保全ログでは目 36 件・まばたき 36 件が
ミリ秒差で 1:1 に並ぶ）、**この前提が成り立たない他のゴーストへ流用してはならない**。
`2`（回数差）と `120 秒`（末尾近接）も emo2 の送信頻度に合わせた較正値である。

スクリプト内では `J2_COUNT_DIFF_MAX` / `J2_TAIL_GAP_MAX_SEC` /
`BLINK_CATEGORY` / `EYE_CATEGORY` / `BLINK_ID_MIN` / `BLINK_ID_MAX` として
先頭に切り出してある（emo2 固有の較正値であることをコメントで明示）。

---

## 5. J3（要件 5.3・目視）— 開発者の担当

**本スクリプトの対象外。** 実走中（§2.2）に開発者が直接目視する。

1. ゴーストがジト目（目=ジトー＋まばたき=ジトー）へ切り替わるのを待つ。
2. その次の表情変更で、**表示が正しく次の表情へ切り替わる**ことを確認する。
3. **areka の再起動を要さない**ことを確認する（是正前は再起動するまで固着していた）。

判定結果は受け入れ記録（`real-machine-signoff.md`）へ開発者サインオフとして記す。

---

## 6. 既知ケース較正の記録

道具そのものが壊れていないことを、**赤・緑の両方**で確認した（2026-08-11）。

### 6.1 赤: 是正前の保全ログ

- ログ: `C:\Users\maz-o\AppData\Local\areka-diag\bindopt-20260811-101835\bindopt-debug-observation.log`
- 465,055 bytes・md5 `d910e4dc7d1ebd350ec0b1fa6bb8f4df`（走査後も不変を確認済み＝読み取り専用）
- 結果: **J1=FAIL / J2=FAIL / exit 1**

```
走査行数: 1746（うち先頭タイムスタンプ無し等で読み飛ばした行 0）
抽出: 適用痕跡 216 件（Changed 87 / 非発行 129）・抽選発火 205 件
観測区間: 2026-08-11T01:18:38.387540Z 〜 2026-08-11T01:25:37.077929Z

== J1（要件 5.2・共存痕跡の不在）==
  実測: まばたき適用痕跡 36 件（Changed 3 / 非発行 33）、まばたき発火 169 件
  発火 id 内訳: 1400×78 / 1402×91
  痕跡 id 内訳: id=1400 on=true ×14 / id=1402 on=true ×5 / id=1403 on=true ×17
  走査した発火: 169 件 / 違反: 109 件
    L380    2026-08-11T01:20:23.080568Z scope=0 animation_id=1402  ← 直近痕跡 L372 ... id=1403 on=true（非発行）  → 不一致
    L401    2026-08-11T01:20:34.077409Z scope=0 animation_id=1400  ← 直近痕跡 L372 ... id=1403 on=true（非発行）  → 不一致
    （ほか 107 件）
  不一致で発火した animation_id: 1400, 1402
  → J1: FAIL

== J2（要件 5.4・飽和パターンの不在）==
  実測: Changed 回数 まばたき=3 / 目=25（差 22）
  最後のまばたき Changed: L348 2026-08-11T01:20:20.139883Z category=まばたき part=ジトー id=1402
  最後の目     Changed: L1726 2026-08-11T01:25:36.987713Z category=目 part=笑顔 id=1303
  末尾時刻差: 316.848 秒
  条件A（回数差 <= 2）: FAIL / 条件B（末尾 120 秒以内）: FAIL
  → J2: FAIL
```

requirements.md §決定的証拠 の記述（まばたき Changed が 1403→1400→1402 の 3 回のみ・
飽和 01:20:20・1400 と 1402 の並行発火）と**逐語で一致**しており、走査が実データを
正しく読めていることの裏取りになっている。

### 6.2 緑: 是正後の想定形ログ

**赤しか出せない道具は「常に赤」かもしれない**ため、緑も出せることを確認した。

(a) **保全ログから合成した想定形**（排他置換セマンティクスを再現＝まばたき/目の
着衣を `(scope, category)` ごとに追跡し、id が変われば Changed・同値なら非発行、
まばたき発火はその時点の追跡 id へ書き換え）に対し、**J1=PASS・違反 0 件**
（発火 1400×97 / 1402×22 / 1403×50 が全て直近痕跡と一致）。

(b) **最小の人工ログ**（目↔まばたきを 1:1 対応させた 12 回の表情変更＋各区間の発火＋
同値再送の非発行＋他コンポーネントのノイズ行と先頭タイムスタンプ無しの継続行）に対し、
**J1=PASS / J2=PASS / exit 0**。

```
== J1 ==  走査した発火: 60 件 / 違反: 0 件 → PASS
== J2 ==  Changed 回数 まばたき=12 / 目=12（差 0）・末尾時刻差 0.000 秒
          条件A: PASS / 条件B: PASS → PASS
総合: J1=PASS  J2=PASS   （exit 0）
```

### 6.3 縁ケース

| ケース | 期待 | 実測 |
|---|---|---|
| まばたき発火が 0 件（沈黙） | 判定不能・非ゼロ | J1=INCONCLUSIVE / exit 2 |
| 適用痕跡ゼロでの発火 | 即赤 | J1=FAIL（違反 2 件・「適用痕跡ゼロでの発火」と明示）/ exit 1 |
| seriko の行が 1 件も無い（採取水準の誤り） | 判定不能・非ゼロ | INCONCLUSIVE / exit 2 |
| 引数なし | 使い方表示・非ゼロ | exit 3 |
| パース不能行の混入 | 落ちない | 読み飛ばし件数として印字 |

較正に用いた合成ログの生成スクリプトは作業用（scratchpad）に置いており、spec 配下には
残していない。再生成が必要なら本節の記述から再構成できる。

---

## 7. 実走時の申し送り

- **J2 条件A（回数差 <= 2）は余裕が小さい。** 保全ログから合成した想定形では
  まばたき Changed 22 / 目 Changed 25 で**差 3** になった。emo2 は
  目「笑顔(1303)」「静観(1304)」「べそ(1300)」がいずれもまばたき「----(1403)」へ
  写像される**多対一**の対応を持つため、目だけが変わってまばたきが据え置きになる遷移が
  実走の並びによって数回起こり得る。実走で条件Aだけが僅差で FAIL した場合は、
  **飽和（片側が恒久沈黙）とは別物**であり、条件B（末尾時刻差）と J1 の結果、および
  まばたき Changed が実走末尾まで継続しているかを併せて見て、開発者裁定にかけること。
  是正前の実測は差 22・末尾時刻差 316.8 秒であり、僅差のケースとは桁が違う。
- **`cargo test --workspace` の既知間欠赤 4 本**（`areka-P0-test-cage-determinism`／W6.9 所有）は
  本 spec と因果独立。実機サインオフの判定には無関係。
