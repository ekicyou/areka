# 実機サインオフ 判定表（タスク 6.2）

- **実施日**: 2026-08-13
- **実施環境**: ディスプレイ 2 台・**拡大率が異なる**
  - primary: `bounds=0,0,2880,1800` `dpi=192`（200%）
  - secondary: `bounds=-2560,195,0,1795` `dpi=144`（150%）
- **ビルド**: debug ／ コミット `d20e32a`（タスク 6.4 の是正を含む・＋ i686 ヘルパ）
- **fixture**: emo2（emo2-kakukaku）・2 スコープ（むらさき／エモ）
- **実行**: `AREKA_APP_SMOKE_EXIT_MS=600000` 有界自動終了 ／
  `RUST_LOG=info,wintf::ecs::window::zorder_pair=debug,areka=debug`
- **ログ**: `signoff.out` ／ `signoff.err`
- **手順書**: `signoff-procedure.md`（S1〜S4）

## 実行時ストラテジ（判定の前提）

```
[zorder-pair] strategy-selected plan=A mechanism=owner-link raise_assist=false
```

ゲート判定表（`plan-a-gate.md`）の結論と一致。以降の判定は案 A 構成のものである。

## scope ↔ entity ↔ HWND の対応

| scope | 名 | キャラ entity | バルーン entity | キャラ HWND | バルーン HWND |
|---|---|---|---|---|---|
| 0 | むらさき | 4v0 | 3v0 | `0x440F02` | `0x940A5A` |
| 1 | エモ | 6v0 | 5v0 | `0x280F06` | `0xD30116` |

## 判定

| # | 判定項目 | 結果 | 根拠 |
|---|---|---|---|
| S1 | 拡大率の異なる 2 画面を往復して隣接を保つ | **PASS** | 両スコープの往復を `DpiReproject` の実測寸で確認（下記）＋ `sink-observed` 全 20 本が `adjacency_ok=true` ＋ 目視 |
| S2 | バルーン操作でキャラが埋もれず、位置も動かない | **PASS** | 目視（バルーンをクリックするとキャラも一緒に浮上・バルーン単独ドラッグでキャラは不動）＋ キャラの移動記録に DPI 由来以外が 0 件 |
| S3 | 他アプリ活性化で背面かつ隣接を保つ | **PASS** | `sink-observed` 20 本すべて `adjacency_ok=true`・両スコープとも記録あり |
| S4 | 常時最前面の窓が居てもバルーンが帯へ入らない | **PASS** | 目視（常時最前面のタスクマネージャーをバルーンへ重ねると、タスクマネージャーが手前に来る）＝ バルーンに常時最前面の印が付いていない |

**ERROR 総数 0 件**（`signoff.out` / `signoff.err` とも）・**`verify-failed` 0 件**。

## 結論

**S1〜S4 全 PASS。本仕様の実機受け入れは合格で確定する。**

---

## 根拠

### S1 — 拡大率をまたぐ往復（要件 7.2／7.3）

**同一のキャラ窓が、拡大率ごとに異なる物理寸で往復している**——これは片方の画面だけを見ていては
出せない証跡である。

| scope | 200% 画面（dpi=192） | 150% 画面（dpi=144） | 比 |
|---|---|---|---|
| 0 むらさき | `764×1094` | `573×821` | 764/573 = 1.333 = 192/144 |
| 1 エモ | `672×800` | `504×600` | 672/504 = 1.333 = 192/144 |

`[diag.window_move]` の `kind=char` は計 7 件（`DpiReproject` 6 ／ `ReportedSizeReconcile` 1）。

```
[diag.window_move] route=DpiReproject entity=4v0 kind=char scope=0 x=-328 y=974 w=573 h=821 dpi=144
[diag.window_move] route=DpiReproject entity=4v0 kind=char scope=0 x=-337 y=610 w=764 h=1094 dpi=192
[diag.window_move] route=DpiReproject entity=6v0 kind=char scope=1 x=-313 y=1195 w=504 h=600 dpi=144
[diag.window_move] route=DpiReproject entity=6v0 kind=char scope=1 x=-308 y=904 w=672 h=800 dpi=192
```

各移動後の `sink-observed` はすべて `adjacency_ok=true`。バルーンの追従は
`BalloonFollow` 897 件で、追従が止まった区間は無い。

### S2 — バルーン操作の独立性（要件 7.4）

目視で確認した——バルーン窓をクリックするとキャラ窓も一緒に浮上し（要件 1.3）、
バルーン窓を単独でドラッグしてもキャラ窓は動かない。

ログ側の補強: `kind=char` の移動記録は上表の 7 件のみで、**すべて拡大率の変化に由来する**
（`DpiReproject` ／ `ReportedSizeReconcile`）。**バルーン操作に起因するキャラの移動は 1 件も無い。**

ドラッグそのものは OS が窓を動かすため配置経路の記録には出ないが、**プログラム側がキャラを
動かせば必ずこの記録に出る**——それが 0 件であることが、バルーン操作がキャラを変位させて
いないことの証跡になる。

### S3 — 他アプリ活性化での沈降と隣接（要件 4.1／4.2／4.4／7.5）

`sink-observed` は **20 本**（むらさき 12 ／ エモ 8）。**全 20 本が `adjacency_ok=true`**。

```
[zorder-pair] sink-observed entity=3v0 adjacency_ok=true foreground=0x2049A  behind_foreground=true
[zorder-pair] sink-observed entity=5v0 adjacency_ok=true foreground=0x600ACA behind_foreground=true
[zorder-pair] sink-observed entity=3v0 adjacency_ok=true foreground=0x280F06 behind_foreground=true
[zorder-pair] sink-observed entity=5v0 adjacency_ok=true foreground=0x3F07A4 behind_foreground=true
（他アプリ 0x2049A / 0x600ACA / 0x3F07A4 ・相方スコープのキャラ 0x280F06 を含む）
```

`foreground=0x280F06`（エモのキャラ）の行は、**相方スコープのキャラをクリックした直後に、
沈んだ側のペアが隣接を保っていた**ことを示す。

#### `behind_foreground=false` の 2 本は FAIL ではない

```
[zorder-pair] sink-observed entity=3v0 adjacency_ok=true foreground=0x440F02 behind_foreground=false
```

`foreground=0x440F02` は **entity 3v0 自身のペアのキャラ窓**である。バルーンは自分のキャラの
**すぐ手前**に居るのだから、自分のキャラより背面であってはならない——**`true` だったら
要件 1.1 の違反**になる。この 2 本は不変条件が立っていることの証跡である。

実装上も `is_behind_foreground`（`zorder_pair_sink.rs:123-137`）は、前面窓が対象の手前側の走査に
現れない場合に `false` を返す。バルーンの手前を辿ってもキャラは出てこない（キャラは背後）ので
`false` になる。同じ 2 本の `adjacency_ok=true` が、その位置関係を正しく裏書きしている。

### S4 — 常時最前面の窓との共存（要件 8.1・タスク 6.4 の是正）

常時最前面にしたタスクマネージャーを出した状態で、ゴーストを背面へ回してからキャラ窓を
クリックし、タスクマネージャーをバルーン窓へ重ねた。

**タスクマネージャーが手前に来た**＝ バルーン窓に `WS_EX_TOPMOST` は付いていない。要件 8.1 を満たす。

`verify-failed` は 0 本、`adjacency_ok=false` も 0 本で、帯へ引き込まれた形跡は無い。

**`insert_after=top-of-normal-band` の記録は 0 件**（`fix` 自体が起動時の 1 本のみ）。
これは FAIL ではない——案 A では是正の契機が確立と再断行だけで、隣接が既に成立していれば
指令は出ない。**6.4 の分岐が実機で踏まれた直接の証跡は得られていない**（決定論的テストが
実窓で固定している）。実機側の判定は「印が付いていないこと」で成立する。

### 是正の指令（起動時）

```
[zorder-pair] fix entity=3v0 peer=4v0 insert_after=0x280F06 measured_next_after_fix=0x440F02
[zorder-pair] skip entity=5v0 peer=6v0 reason=AlreadyAdjacent
```

`fix` の実測 `measured_next_after_fix=0x440F02` は **scope 0 のキャラ窓そのもの**＝指令が
意図どおり効いたことの実測。案 A では以後 `fix` は出ない（隣接の維持は OS の owner 保証が担う）。

## 手順書からの申し送り

- **案 A ではキャラをクリックしても `fix` は出ない。** 本走でも `fix` は起動時の 1 本のみ。
  肯定的証跡は `sink-observed` の `adjacency_ok` で採るという手順書の方針が実走で裏付けられた。
- **`behind_foreground=false` を機械的に FAIL と数えてはならない。** 前面窓が自分のペアの
  キャラ窓である場合は正常であり、むしろ不変条件の証跡である。手順書の判定式
  （`behind_foreground=false` の件数が 0 であること）は**この点で厳しすぎた**——
  `foreground` 欄が自ペアのキャラ HWND かどうかで切り分けること。
