# Brief: areka-P0-bindoption-exclusivity

> **起票 2026-07-26**（`/kiro-discovery`・`areka-P0-emo-dpi-scaling` の task 7.4 実機サインオフ中に開発者が発見）。
> 本 brief は**調査済みの根因と実測証拠を全て内包**する。別セッションはこの brief 単体で再開できる（会話ログは不要）。

## Problem

**誰の問題か**: エンドユーザー（ゴースト利用者）と、正典準拠を掲げる areka のシェル互換性。

**症状（開発者の実機観測・2026-07-26）**: 「むらさきの目の表情が『ジト目』になった後、他の表情に切り替わらないように見える」。一度ジト目になると、その後どんな会話が来ても表情が戻らない。**非可逆**（アプリを再起動するまで復帰しない）。

**痛み**: 表情はキャラクター表現の中核。固着すると会話内容と表情が乖離し、ゴーストが壊れて見える。しかも**ゴースト側のスクリプトからは原理的に復帰不能**なので、ゴースト作者には回避手段がない。

## Current State

### 根本原因（確定・実測 3 系統一致）

**areka は `bindoption` の 3 値意味論を 2 値へ潰している。** ukadoc 正典は

| 宣言 | 正典の意味 |
|---|---|
| `mustselect` | ちょうど 1 個（解除不可） |
| **非宣言（既定）** | **高々 1 個**（解除可・**複数選択不可**） |
| `multiple` | 複数可 |

を規定する（`sakura/kero/char*.bindoption*.group,カテゴリ名,オプション`——「mustselectでパーツを必ず1つ選択、multipleで複数のパーツを選択可能」＋既定値「選択解除可能、複数選択不可」）。

areka の実装は **「`mustselect` か、さもなくば加算」の 2 値**で、**非宣言を `multiple` と同一視**している。

- `crates/areka-seriko/src/actor.rs:367-372`
  ```rust
  let outcome = if on && bind_resolver.is_mustselect(ns, &category) {
      /* apply_bind_exclusive（排他置換） */
  } else {
      /* apply_bind（単純加算） */
  };
  ```
- `crates/areka-parsers/src/package/resolve.rs:172-188` — `mustselect` のみを収録し **`multiple` を破棄する**。ゆえに下流には「明示 multiple」と「非宣言」を区別する情報が**存在しない**。

### 症状に至る因果連鎖

1. `fixtures/emo2/shell/master/descript.txt:75-78` は `bindoption` を **腕 / 口 / 眉 / 目 の 4 つにしか宣言していない**。**まばたき は非宣言**（パーツ自体は同 :50-53 で `1400-1403` を `.name` 宣言済み）。
2. ゴーストは正典どおり **`on` のみを送り、`off` を送らない**（ベースウェアの排他置換に依存する正しい作法）。実測: 全 4 ログ中 `on=false` は**紅（1600・単一パーツのアクセサリ）の 1 回のみ**。
3. areka は まばたき を加算扱いするため、**パーツが永久に積み上がる**。
4. `surfaces.txt:84-87` の `animation1402`（ジトー）の最終コマ `surface1413` は
   ```
   element0,overlay,purple/a/eyebase.png   ← 不透明な「目のベース」込み
   element1,overlay,purple/4/jito.png
   ```
   （`surfaces.txt:226-230`）
5. **z-order は animation ID 昇順＝画家のアルゴリズム**（作者自身が `surfaces.txt:19` に明記。実装側も `areka-emo-compose/src/bind.rs:8-34` の `BindSet` 昇順＋`blit.rs:83`「下層から上層」で一致）。ゆえに **`14xx`（まばたき）は `13xx`（目）の上**。
6. ループ末尾コマの残留により jito フレームが保持され、**以後どんな 目 の切替も、後続の まばたき=通常(1400) でさえ、上に乗った 1402 に覆われて復帰不能**になる。

### 決定的な実行時証拠

`target/signoff-7.4-run4.log`（`RUST_LOG=info` 実走・実 pasta.dll・実 DPI）:

```
03:49:46.457  目=笑顔   id=1303 on=true
03:49:46.458  まばたき=----  id=1403 on=true     ← ペア
03:50:17.517  目=通常   id=1302 on=true
03:50:17.517  まばたき=通常  id=1400 on=true     ← ペア
03:50:20.265  目=ジトー id=1301 on=true
03:50:20.265  まばたき=ジトー id=1402 on=true    ← ペア。間に off なし
03:50:27.203  目=通常   id=1302 on=true
                                                 ← まばたき の再ペアリングなし
```

そしてルーパーの発火ログ（`grep -o "animation_id=140[0-9]" | sort | uniq -c`）が **1400 × 11 回・1402 × 6 回を同一時間帯に**返す:

```
1400: 03:50:20, :22, :23, :25, :30, :40 …
1402: 03:50:26, :29, :31 …
```

`areka-seriko/src/looper.rs:196-201` の bind ゲート（`if !states.current_binds(scope).contains(anim.id) { skip }`）より、**発火＝bind 集合に含まれることの証明**。すなわち**同一カテゴリの 2 パーツが同時に bind されたまま並行再生している**——加算の直接証拠。

### 【重要・2026-07-26 追補】飽和した bind 集合は、ゴーストの是正指示を**無言で握り潰す**

開発者の追加観測（実走 `target/signoff-7.4-final.log`）が、被害の本当の広がりを明らかにした。

**観測 A: 「会話中じゃないのに唐突にジト目に変わる」**
**観測 B: 「ジト目のまま目閉じアニメーションはする」**

生ログのまばたき bind 時系列:

```
04:08:31  目=笑顔   1303 + まばたき=----   1403
04:09:01  目=通常   1302 + まばたき=通常   1400
04:10:43  目=ジトー 1301 + まばたき=ジトー 1402   ← まばたき bind ログはここが最後
------------------------------------------------------------------
04:10:46  目=笑顔   1303      （まばたき のログなし）
04:11:03  目=通常   1302      （同上）
04:11:06  目=ジトー 1301      （同上）
04:11:27  目=笑顔   1303      （同上）
04:11:30  目=通常   1302      （同上）
04:11:47  目=笑顔   1303      （同上）
04:12:08  目=通常   1302      （同上）
04:12:11  目=笑顔   1303      （同上）
```

**まばたきのログだけが 04:10:43 を境に完全に止まる一方、目 は律儀に変わり続けている。**

**これは「ゴーストが送るのをやめた」のではない。** `bind 適用` の `info!` は **`Changed` のときしか発火しない**（`actor.rs:374-384`。`Unchanged`/`StateOnly` は `debug!`＝`:387-396` で、`RUST_LOG=info` 実走では記録されない）。加算実装ゆえ bind 集合は

```
04:08:31  {1403}
04:09:01  {1403, 1400}          ← 1403 が外れない
04:10:43  {1403, 1400, 1402}    ← 飽和
```

と育ち、**飽和後はゴーストが何を送っても既に集合内 → `Unchanged` → 無言で握り潰される**。

> **被害はこれまでの理解より深刻**: 単に「排他置換されない」だけでなく、**加算が集合を飽和させた後は、ゴースト側の正しい是正指示が一切効かなくなる**。ゴーストから見れば「送ったのに何も起きない」＝デバッグ不能な沈黙。

**観測 A の説明**: `animation1402` は `interval,bind+random,4`＝**4 秒ごとに勝手に抽選発火**するまばたきアニメ。一度 bind されると会話と無関係に発火し続ける。実測の発火回数は **1400 × 70 回・1402 × 45 回**（同一実走・両方が生きて交互に暴れている）。

```
04:12:14  1400 抽選発火 → 末尾残留
04:12:15  1402 抽選発火 → 末尾残留   ← 1402 が上に乗る
```

**観測 B の説明**: `1402` は**まばたきアニメそのもの**（`pattern1=1412` 開き → `pattern2=1413` ジト閉じ・`surfaces.txt:85-87`）。bind されている以上、正常に瞬く。ただし最終コマ `1413` が不透明 eyebase 込みかつ ID 昇順で `1400` の上に乗るため、**1400 が後から発火してもその絵は永遠に下敷き**になる。

**この 2 観測は加算バグの必然的な帰結であり、診断を閉じる最後の一片**。ジト目が「唐突に」現れるのは会話由来ではなく `bind+random` の抽選タイマー由来である。

### 【調査セッションへ】確定的な追加実験（未実施・最優先）

上記の「握り潰し」は**推論**であり、直接観測はまだ取れていない。次を実行すれば**ゴーストが是正を送り続けている事実**を直接証明できる:

```bash
RUST_LOG=info,areka_seriko=debug  # ← debug まで上げる
```

で emo2 を実走し、`04:10:43` 相当以降の まばたき カテゴリについて **`Unchanged`（冪等 skip）の `debug!` ログ**を捕捉する。

- **出れば**: ゴーストは送り続けている＝areka の握り潰しが直接証明される（**ゴースト完全にシロ**）
- **出なければ**: ゴーストが本当に送っていない＝副因の比重が上がる（ただし主因＝加算による永久被覆は変わらない）

**証拠グレードが「推論」から「直接観測」へ上がるので、requirements を書く前に必ず実施すること。**

### 既存の先送りに直撃（担当者不在）

`.kiro/specs/completed/areka-P0-mayuna-compose/` の **R4.5**（requirements.md:85）と **D11**（design.md:68 の表・design.md:142）が

> 「`multiple`（紅等・**非宣言＝既定**）はスクリプト明示 on/off で従来どおり成立ゆえ語彙保持のまま」「非宣言は既定＝非排他で無視」

と明記している。**これがまさに誤った仮定**。

しかも同 spec は **2026-07-23 に `mustselect` について完全に同型の誤仮定**（「ゴーストが明示 off を送るはず」）を実機で反証されて実導出へ昇格した経緯がある。**非宣言カテゴリにだけ同じ穴が残った**。

当該 spec は `completed/` にあり**消化不能**。現行アクティブ spec（`areka-P0-emo-dpi-scaling`＝DPI 関心事）は無関係。**担当者不在ゆえ本 spec を新規起票**（規律: [[deferral-requires-verified-owner]]）。

### ベースラインは健全

`cargo test -p areka-seriko` は全パス。**リグレッションではなく、誤った仕様仮定の忠実な実装**。既存テスト `bind_non_mustselect_accumulates_via_actor`（`actor.rs:1546`）は**異なる**カテゴリ 2 件の加算を検証しており、誤挙動を固定していない——正典修正後も無変更で通る。

## Desired Outcome

- `bindoption` の 3 値意味論が areka に実装され、**非宣言カテゴリが「高々 1 パーツ」として排他置換される**。
- emo2 実機で、まばたきの `1400` と `1402` が**同一時間帯に共存しない**ことがログで観測できる。
- ジト目になった後、次の表情変更で**正しく切り替わる**（開発者の目視サインオフ）。
- 既存の `mustselect` 挙動・`multiple` 明示宣言時の加算挙動は**不変**（回帰の錨）。

## Approach

**述語の意味を反転する**——「排他か？」の判定を「`mustselect` **である**」から「`multiple` と**明示宣言されていない**」へ。

1. `areka-parsers` が `bindoption ... multiple` を**収録**し、`mustselect` 集合と並ぶ `multiple` 集合を `BindGroupDefaults` へ持たせる（現状は破棄しており、非宣言と区別できないのが構造的な根）。
2. `BindResolver` の述語を `is_mustselect` → 「排他か」を返す述語へ（`multiple` 集合に**含まれない**なら排他）。
3. `actor.rs:367` の分岐述語を差し替え（実質 1 行）。

**なぜこれか**: 情報の欠落（parsers が multiple を捨てている）が根なので、そこを埋めるのが最小かつ構造的な是正。分岐側だけ弄っても 3 値は表現できない。

**副次論点（本 spec で拾うか要裁定）**: 正典上 `mustselect` は「**解除不可**」だが、areka は `on=false` を素通ししている。emo2 は `mustselect` カテゴリに off を送らないため**実害なし**（休眠中）。

## Scope

- **In**:
  - `bindoption` 3 値意味論（`mustselect` / 非宣言 / `multiple`）の parsers 取り込みと下流搬送
  - 非宣言カテゴリの排他置換（高々 1 パーツ）
  - 上記の決定論檻（in-crate）＋ emo2 実機サインオフ
  - `BindResolver::new` 署名変更に伴う全呼出元の atomic 追随
- **Out**:
  - SERIKO アニメーションのループ/interval 意味論（`completed/areka-P0-seriko-loop` 領分・本件と無関係）
  - z-order / 合成順（`areka-emo-compose` は正典どおり昇順＝正常。**変更不要**）
  - ゴースト側 fixture の辞書修正（下記 Out of Boundary 参照）
  - `\![bind]` の Toggle 形 / CategoryWide 形の実導出（`actor.rs:319-326` で warn+skip。**全 4 ログで発火ゼロ＝本件と無関係**と実測で棄却済み。別件の既知先送り）

## Boundary Candidates

- **①採取**: `areka-parsers/src/package/{resolve.rs, model.rs}` — descript の `bindoption` を 3 値で読み取り `BindGroupDefaults` へ載せる（純関数・全網羅檻が容易）
- **②判定**: `areka-seriko/src/{bind.rs, state.rs}` — 「排他か」述語と `apply_bind_exclusive` への振り分け（純関数・既存 `apply_bind_exclusive` はそのまま再利用できる見込み）
- **③結線**: `areka-seriko/src/actor.rs:367` ＋ `areka/src/emo2_boot/assets.rs:204-210` — 起動時資産の構築と分岐差し替え
- **④観測**: 実機ログ判定基準（`animation_id` の同一カテゴリ共存ゼロ）

①②は純関数で全網羅可能。③は 1〜2 行。④が実機サインオフの判定基準。

## Out of Boundary

- **z-order / 画家のアルゴリズムの変更**——`14xx` が `13xx` の上に来るのは**作者の意図どおり**（surfaces.txt:19）。ここを弄るのは誤った治療。
- **ゴースト fixture（`emo2`）の辞書修正**——03:50:27 で 目=通常 へ戻す際にまばたきを再ペアリングしていない**軽微な副因**は実在するが、正典準拠の排他実装下では「次の paired 変更まで jito 継続」という一過性のズレに縮退し**自己修復**する。fixture は上流由来の実物であり、areka 側の正典適合が先。**fixture を直して症状を隠すのは禁じ手**。
- **`\![bind]` Toggle / CategoryWide 形の実導出**——実測で本件と無関係（発火ゼロ）。混ぜない。
- **DPI / スケール関心事**——`areka-P0-emo-dpi-scaling` 領分。

## Upstream / Downstream

- **Upstream**:
  - `completed/areka-P0-mayuna-compose`（**本件の誤仮定の出所**＝R4.5 / D11。消化不能ゆえ本 spec が継承）
  - `completed/areka-P0-shell-parse` / `areka-parsers`（descript `bindoption` の読み取り経路）
  - `completed/areka-P0-seriko-engine` / `seriko-loop`（bind ゲート `looper.rs:196-201`・ルーパー）
  - ukadoc 正典（`bindoption*.group` の既定値）
- **Downstream**:
  - 表情・着せ替えを使う全ゴースト。**正典準拠シェルの互換性の土台**
  - `areka-P0-kero-balloon`（W5）等の scope 別資産系とは**ファイル集合が互いに素**（bind は seriko / parsers・balloon は emo-present / assets）

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-mayuna-compose`（D11 の設計判断を**覆す**。完了済みゆえ更新不可——本 spec の requirements/design にその旨を明記し、roadmap 追記で追跡する）
- **Adjacent**:
  - `areka-P0-emo-dpi-scaling`（進行中・W4）——ファイル集合は互いに素（emo-text / emo-present / placement vs seriko / parsers）。**同時進行しても衝突しない見込み**だが、着手時に実測で再確認すること
  - `areka-P0-kero-balloon`（W5・未着手）——`emo2_boot/assets.rs` を両者が触る可能性がある（本 spec は `:204-210` の `BindResolver` 構築のみ・kero-balloon は `:79, :240-243` の balloon 資産）。**同一ファイル異ハンク**ゆえ着手順の裁定が要る

## Constraints

- **新規 crates.io 依存を増やさない**（プロジェクト規律）
- **`BindResolver::new` の署名変更は全呼出元をコンパイル結合で巻き込む**（seriko tests・e2e・areka boot）＝atomic 追随が必要。中間の stand-in を挟まない（規律: [[canonical-not-minimal-lifecycle]]）
- **ログ無し失敗経路の禁止**（`.kiro/steering/logging.md`）
- **決定論テスト網羅は必達**（[[deterministic-test-coverage-mandate]]）——①②は純関数ゆえ全網羅可能
- **実機サインオフ必須**——[[real-machine-signoff-catches-what-cages-hide]]。本件はまさに「檻が緑のまま実機で壊れていた」事例であり、`mayuna-compose` が同型の誤仮定を実機で反証された前例が 2 度目
- **実機起動は絶対パス必須**（[[areka-emo2-signoff-needs-absolute-paths]]）＋`AREKA_APP_SMOKE_EXIT_MS` 有界（[[areka-real-machine-signoff-bounded-auto-exit]]）

## 最小再現（決定論・調査セッションはここから始めよ）

`areka-seriko` の in-crate テスト:

1. `mustselect` 集合を**空**にした `BindResolver` に、同一カテゴリ 2 パーツを登録する
   （例: `("まばたき","通常")→1400`、`("まばたき","ジトー")→1402`）
2. actor 経路へ `\![bind,まばたき,通常,1]` → `\![bind,まばたき,ジトー,1]` の 2 cue を流す
3. `states.current_binds(&scope)` を検査する

- **現状**: `{1400, 1402}` を返す ＝ **欠陥**
- **正典期待値**: `{1402}` のみ

**ログのみの判定基準（既に充足済み）**:
```bash
grep -o "animation_id=140[0-9]" target/signoff-7.4-run4.log | sort | uniq -c
```
→ `1400` と `1402` の**両方**が返れば、同一カテゴリ 2 パーツの同時 bind が起きている証拠。

## 調査済み事項（再調査不要・棄却済みの仮説）

- ❌ **`\![bind]` の Toggle 形 / CategoryWide 形が skip されている**（`actor.rs:319-326` の warn）
  → **全 4 ログで `未実導出` の grep ヒット 0 件**。一度も発火していない。**棄却**。
- ❌ **ゴーストが表情変更を送っていない**
  → 送っている（03:50:20 に 目=ジトー・まばたき=ジトー を正しくペアで送信）。**棄却**。
- ❌ **`mustselect` の配線が壊れている**
  → parsers → `MountModel` → `BootAssets` → `BindResolver` → actor まで**完全に通っており正常**（目 カテゴリは正しく排他置換されている）。**棄却**。
- ⚠️ **`bind 適用` info! は `Changed` のときのみ発火**（`actor.rs:374-384`。`StateOnly`/`Unchanged` は `debug!`＝`:387-396`）。`RUST_LOG=info` 実走では記録されない。
  **「ログに無い」＝「ゴーストが送っていない」の証明にはならない**——ログを読むときの注意点。
- ℹ️ **確信度 HIGH**。根拠が 3 系統独立に一致: (a) ukadoc 正典の既定値、(b) ソース上の分岐、(c) 実行時ログでの 1400/1402 同時発火。
  さらに上げるには、修正後に emo2 実機を再実走し `animation_id=1400` と `1402` が同一時間帯に共存しないことをログで確認する。

## 規模見積もり

**medium**。コード変更量自体は小さい（実質、排他判定述語の意味を反転し parsers に `multiple` 集合を 1 本追加するだけ）。ただし:

- `areka-parsers` / `areka-seriko` / `areka` の **3 クレートに跨る**
- `BindResolver::new` の署名変更が**全呼出元をコンパイル結合で巻き込む** atomic 追随
- 「非 `mustselect` ＝加算」を前提とした既存 doc コメント・テスト文言の**広範な更新**

## 証拠ファイル一覧（本セッションの実走ログ・保全されている）

| ファイル | 内容 |
|---|---|
| `target/signoff-7.4.log` | 1 回目実走（125%→200% モニタ跨ぎ） |
| `target/signoff-7.4-relaunch.log` | 2 回目（座標復元確認） |
| `target/signoff-7.4-run3.log` | 3 回目（輪郭鮮明さ確認） |
| `target/signoff-7.4-run4.log` | 4 回目（まばたきの bind 時系列・ルーパー発火の初回確認） |
| `target/signoff-7.4-final.log` | **5 回目＝最重要**（開発者の観測 A/B に対応。04:10:43 以降まばたき bind ログが完全停止＝飽和による握り潰し・1400×70 / 1402×45 の並行発火） |

> これらは `areka-P0-emo-dpi-scaling` の task 7.4 サインオフ用に採取したもので、**本件は副産物として発見された**。
