# pilot-windows-rs-next 一次記録

先進坑（使い捨て）。成果物はコードでなく知見。本書が正本で、design はここを参照し同じ結果を二重に書かない。

**判定は開発者が下す。** 各節の「判定」欄は空けてある。

## 第 1 幕: 動機

（未記入。対応する本坑〔移行の本体の各本・brief の Downstream 1〜8〕と、何が怪しくて掘るかを書く。置き場が二坑モデルの例外である理由〔1 つのパッケージが `windows_core` の名前で持てる版は 1 つ〕も書く。）

## 第 2 幕: 概要

### クレートの一覧と役

| クレート（フォルダ） | 問い | 依存（置き場の中・上流・借りる物） |
|---|---|---|
| `q1-winmsg` | 1・8 | `windows-link`・`async-task`・build に `windows-bindgen`。edition 2021・`[lib] name = "wintf_winmsg_executor"` |
| `q2-placement/plan1-wintf`・`plan1-emo` | 2（案 1） | `windows-core`・build に `windows-bindgen`（両方が自分の束縛を持つ）。emo → wintf |
| `q2-placement/plan2-wintf`・`plan2-emo` | 2（案 2） | `windows-core`。束縛は wintf 役だけ（build に `windows-bindgen`）。emo → wintf |
| `q2-placement/plan3-bind`・`plan3-wintf`・`plan3-emo` | 2（案 3） | `windows-core`。束縛は bind だけ（build に `windows-bindgen`）。wintf → bind・emo → bind と wintf |
| `q3-default`・`q3-minimal` | 3・8 | `windows-core`・build に `windows-bindgen` |
| `bind-next` | 4・6・7 の土台 | `windows-core`・build に `windows-bindgen`（兄弟は束縛が名指ししたときだけ 1.3 で足す） |
| `q4-wuc` | 4 | `bind-next`・`q1`（`q1-winmsg` を名前を変えて）・`windows-core`・`png` |
| `q5-hwnd` | 5 | `windows-core`・`bevy_ecs`・build に `windows-bindgen` |
| `q6-com` | 6 | `bind-next`・`windows-core` |
| `q7-vertical` | 7・要件 13.5 | `bind-next`・`q4-wuc`・`q1`・`windows-core`・`windows-text`・借りる `wintf`・`areka-emo-text`・`areka-parsers`・`areka-sakura`・`png` |
| `q9-api-count` | 9 | `scip` |
| `q10-winmsg` | 10 | `q1-winmsg` と同じ形（`[[example]]` は 11.1 で足す） |

`q2-placement` の 7 つのパッケージ名は `q2-<フォルダ名>`（例 `q2-plan1-wintf`）。フォルダ名は本線のクレートと同じ名前を使わない。

### 引いた上流のコミットと確かめた日

- 固定の rev: `f37e2f9046cf9710fe00285436c4d22ad6053d72`
- 確かめた日: 2026-11-16（1.2 で取り寄せ、置き場の中で `cargo check --workspace` が骨組みのまま通った）

### `Cargo.lock` の種にした本線のコミット

- 本線（main）のコミット: `5b3a361c95c5fb494fdf7845576cf20f1aa20c07`（この作業木の土台も同じ。作業木の HEAD は `43381ae7842d826a15ee350028b2345eb61f8a6c`、`git diff main -- Cargo.lock` は空）
- `image-webp` は本線の根の `[patch.crates-io]`（git）が置き場に効かないので crates.io の版へ解き直される。解き直された版: crates.io の `image-webp` 0.2.4（本線の `Cargo.lock` は git の 0.2.5＝`image-rs/image-webp` rev `75f81091`）

### 取り寄せた兄弟のクレート・`cargo tree -d`・フィルターの行数と生成した行数

**取り寄せ（1.2・2026-11-16）**

- 上流の git は既定の `CARGO_HOME`（この機械では `c:\rust\cargo`）へ cargo がそのまま取り寄せた。`git/db/windows-rs-a5de4a2dc783ec71` が約 1.5 GB、取り出しは `git/checkouts/windows-rs-a5de4a2dc783ec71/f37e2f9`。解決は止まらず、rev は動かしていない（要件 2.5 の手順は使っていない）。
- 置き場の `Cargo.lock` の windows-rs の `source` は `git+https://github.com/microsoft/windows-rs?rev=f37e2f9046cf9710fe00285436c4d22ad6053d72#f37e2f9046cf9710fe00285436c4d22ad6053d72` の 1 つだけ（11 パッケージがこの source）。
- 取り出しの中に兄弟のクレートが在る: `windows-future`・`windows-collections`・`windows-reference`・`windows-time`（どれも `crates/libs/<名前>`・0.101.0）。どれも今はどのクレートも引いていないので `Cargo.lock` には出ない。
- `scip` は本線の `Cargo.lock` に無い。置き場で crates.io の `scip` 0.10.0 に解けた。
- 借りる `wintf`（推移で本物の `wintf-winmsg-executor`）と `q1-winmsg` は同じライブラリ名 `wintf_winmsg_executor` で `q7-vertical` の同じ木に入り、組めた（`q7-vertical` が直に引くのは `q1` だけ）。cargo は断らなかったので `q1-winmsg` のライブラリ名は変えていない。
- 気付き: 置き場の中で打つと、`.cargo/config.toml` の相対の `target-dir`（`../../../../target/...`）がそのままパスに残って長くなり、proc-macro のリンクで `LNK1104`（ファイルを開けない）で止まった。走らせ方の節のとおり `--target-dir` を絶対パスで渡すと通る。

**`cargo tree -d -e normal -p q7-vertical --depth 1`（抜粋・`windows-core` の 2 版が同じ木に並ぶ・要件 13.5 の材料）**

```
windows-core v0.62.2
├── areka-emo-text v0.1.0 (crates/areka-emo-text)
├── windows v0.62.2
├── windows-collections v0.3.2
├── windows-future v0.3.2
├── windows-numerics v0.3.1
└── wintf v0.1.0 (crates/wintf)

windows-core v0.101.0 (https://github.com/microsoft/windows-rs?rev=f37e2f9046cf9710fe00285436c4d22ad6053d72#f37e2f90)
├── bind-next v0.0.0 (crates/pilot/examples/pilot-windows-rs-next/bind-next)
├── q4-wuc v0.0.0 (crates/pilot/examples/pilot-windows-rs-next/q4-wuc)
├── q7-vertical v0.0.0 (crates/pilot/examples/pilot-windows-rs-next/q7-vertical)
├── windows-numerics v0.101.0 (https://github.com/microsoft/windows-rs?rev=f37e2f9046cf9710fe00285436c4d22ad6053d72#f37e2f90)
└── windows-text v0.101.0 (https://github.com/microsoft/windows-rs?rev=f37e2f9046cf9710fe00285436c4d22ad6053d72#f37e2f90)

windows-link v0.2.1
├── windows-core v0.62.2 (*)
├── ...（windows-future・windows-numerics・windows-result・windows-strings・windows-sys・windows-threading）

windows-link v0.100.1 (https://github.com/microsoft/windows-rs?rev=f37e2f9046cf9710fe00285436c4d22ad6053d72#f37e2f90)
├── q1-winmsg v0.0.0 (crates/pilot/examples/pilot-windows-rs-next/q1-winmsg)
├── windows-core v0.101.0 (...) (*)
├── windows-result v0.100.1 (...)
└── windows-strings v0.100.1 (...)
```

同じく 2 版が並ぶ物: `windows-implement` 0.60.2／0.101.0・`windows-interface` 0.59.3／0.101.0・`windows-numerics` 0.3.1／0.101.0・`windows-result` 0.4.1／0.100.1・`windows-strings` 0.5.1／0.100.1。パスは作業木の根からの相対に縮めた。

#### bind-next（1.3）

`build.rs` が `filters/*.txt`（`common`・`q4-wuc`・`q6-renderer`・`q7-text`）を全部読み、既定の形の束縛を `OUT_DIR/bindings.rs` へ書き、`lib.rs` が `include!` する（束縛はコミットしない）。名指しは関数・型の短い名前（`CoInitializeEx` のように）で、`Windows.Win32.…` の長い名前は「could not resolve filter entry」で断られた。

| フィルター | 名指しの行数 |
|---|---|
| `common.txt` | 11（COM の初期化 2・D3D11 3・D2D 4・DXGI 2） |
| `q4-wuc.txt`・`q6-renderer.txt`・`q7-text.txt` | 0（各問いのタスクが足す） |

生成した `bindings.rs`: 7742 行（common の 11 件だけで。依存の型を自動で引くため名指しの約 700 倍）。束縛が名指しした兄弟は `windows-numerics`（`windows_numerics::Vector2`／`Matrix4x4`）だけで、`Cargo.toml` に足した。`windows-future` などは名指しされず足していない。`.implements`（下線の描き手の 3 つ）は 6.2 で足す。

### RunCommands（走らせ方）

どのディレクトリからでも動く形（作業木の根が `C:/home/maz/git/areka` のとき。別の作業木では根を差し替える）:

```
cargo <run|build|test|clippy> --manifest-path C:/home/maz/git/areka/crates/pilot/examples/pilot-windows-rs-next/Cargo.toml --target-dir C:/home/maz/git/areka/target/pilot-windows-rs-next -j 4 -p <クレート> [--bin|--example <名前>] [--target <三つ組>]
```

- クロスのビルド（i686・aarch64）は PowerShell から打つ。
- 窓を出す例（`q4-demo`・`vt-balloon`）は排他テストの机を持ってから走らせる。
- ビルドの出力・`OUT_DIR`・`out\<問い>\` はどれも `--target-dir` の下に出る。

### ExitContract（窓を出す例の終了コード）

| コード | 意味 |
|---|---|
| 0 | 正常（上限時間に到達、または `vt-balloon` の当たりが一致） |
| 2 | 初期化の失敗（段の名前をログに出す） |
| 3 | 窓が外から閉じられた（判定に使わない） |
| 4 | `vt-balloon` の当たりが食い違った |

上限時間は環境変数 `AREKA_APP_SMOKE_EXIT_MS`（既定 30000 ms）。

### CopyLedger（写した量と手間）

写した量は元のファイルの本番の行（試験の行と問い 10 の写しを除く）を元のファイルごとに 1 回数える。書き換えの行数は `git diff --numstat <写した直後> <移した後> -- <クレートのフォルダ>`。

| 問い | 元のファイル | 写した行数 | 書き換えの行数 | 直しの回数 | 時間 |
|---|---|---|---|---|---|
| 1 |  |  |  |  |  |
| 3 |  |  |  |  |  |
| 4 |  |  |  |  |  |
| 6 |  |  |  |  |  |

1 行あたり・1 API あたりの手間 ＝ 手間 ÷ 書き換えの行数／フィルターの行数。問い 9 の全数に掛けて移行の本体の規模を見立てる。

## 第 3 幕: 検証結果

見立ての候補: go／違う／直す。

### 問い 1: C の関数だけのクレート（`.sys()`）

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 2: 束縛の置き場所（3 案）

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 3: 束縛の形（既定か `.minimal()`）

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 4: WinRT（`windows-bindgen` で合成）

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 5: `HWND` の持ち方

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 6: COM の実装（`#[interface]`・`#[implement]`）

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 7: `windows-text` の縦書き

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 8: 32bit と arm64 のリンク

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 9: 規模（API の全数）

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 問い 10: 機械的に回せるか（Sonnet）

- 見立ての候補: （未記入）
- 学び: （未記入）
- 確かめた日: （未記入）
- 止まった所・理由・代わりの見立て: （未記入。無ければ「無し」）
- 判定: （開発者が下す）

### 刻み方の案と規模の見立て

（未記入。問い 9 の全数 ×「写した分の 1 行・1 API あたりの手間」）

### 段階の移行の可否の材料

（未記入）

### レビューが見る点の一覧（問い 10）

（未記入）

### 公開の後の注意

（未記入。公開の後に結果が変わりうること）

### 総合の判定

判定は開発者が下す。
