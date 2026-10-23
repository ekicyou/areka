# ギャップ分析: areka-P0-app-icon

- 分析日: 2026-10-16
- 基準: ブランチ `claude/areka-p0-app-icon-80853a`（`874058f1`＝main `1e6590d8` に spec の初期化を足したもの）
- 入力: `requirements.md`（生成済み・未承認）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`、関係する所だけ `roadmap.md`）
- やり方: コードとスクリプトを読んで突き合わせた（重いビルドとテストは回していない）。外の情報は引いていない（引くべき所は 7 節に「調べが要る」として残した）

---

## 1. まとめ

- **埋め込みの土台は揃っている**。`crates/areka/build.rs`（22 行）は、リンカが MSVC のときだけ bin にだけ届く指示でリンカへ引数を渡す形が既にあり、資源のファイル（`.res`）をもう 1 つ渡す 1〜2 行を足せば埋まる。依存を足す必要は無い（`Cargo.lock` は変わらない）。
- **テストの道具も揃っている**。根の `windows` の機能に `Win32_System_LibraryLoader` と `Win32_UI_WindowsAndMessaging` が既に入っていて、`LoadLibraryExW`・`FindResourceW`・`LoadResource`・`LockResource`・`SizeofResource` と `RT_GROUP_ICON`・`RT_ICON`・`RT_MANIFEST` が使える。結合テストが組んだ `areka.exe` の場所を `CARGO_BIN_EXE_areka` で受け取る前例（`crates/areka/tests/smoke_boot_loop_exit.rs`）もある。
- **無いもの**は、絵（原画・小さい寸法の仕上げ）・アイコンのファイル・資源のファイル・作り直しの道具・権利の記録・資源を読むテストの 6 つ。アイコンを読む・出すコードはリポジトリのどこにも無い（`LoadImageW`・`LoadIconW`・`Shell_NotifyIcon` などは 0 件）ので、本仕様と下流の `tasktray-icon` の間に作り替えるものは無い。
- **主な分かれ目**は、①資源のファイルを先に作ってコミットするか、組み立てのたびに `build.rs` が std だけで作るか（議題 3 の案 A とその変種）、②アイコンのファイルの中の小さい寸法を PNG のまま入れるか、ビットマップに直して入れるか、③番号の振り方（エクスプローラーは資源の並びで最初のアイコンの組を exe の絵にする）。
- **規模は S〜M・危うさは低〜中**。技術の面は確かな前例の上に乗る。時間の読めない所は、要件の段の絵の選び直しの往復と、16 ピクセルの手仕上げ。

---

## 2. 今の姿（調べた事実）

### 2.1 組み立て

- `crates/areka/build.rs`: `CARGO_CFG_TARGET_ENV` が `msvc` でなければ何もせず返る。`msvc` なら `cargo:rustc-link-arg-bins=/MANIFEST:EMBED` と `/MANIFESTINPUT:<crates/areka/areka.manifest>` を出す。`rerun-if-changed` は `areka.manifest` と `build.rs`。
- `crates/wintf/build.rs` も同じ形（サンプルの exe にだけ `rustc-link-arg-examples` でマニフェストを渡す）。ワークスペースの `build.rs` はこの 2 つだけ。
- `crates/areka/Cargo.toml`: bin は `areka` 1 つ（`src/bin/` も無い）。`[build-dependencies]` 無し。`include`／`exclude` 無し。`publish = false`。
- `crates/areka-mcp-bridge/`・`crates/shiori-host32-helper/` に `build.rs` は無い。`rustc-link-arg-bins` は出したクレートの bin にしか効かないので、`areka` の `build.rs` を触っても、この 2 つの exe は変わらない（要件 3.8 はそのまま満たせる）。
- 組み立ての指定: 根の `[profile.release]` は `opt-level = 'z'`・`lto = true`・`codegen-units = 1`・`strip = false`。資源はリンカの段で入るので、これらの指定とは関わらない。
- 機械の道具: Windows SDK の `rc.exe` は `C:\Program Files (x86)\Windows Kits\10\bin\<版>\x64\rc.exe` に 3 版あるが、PATH には載っていない（開発者の机）。ImageMagick は無い。pwsh 7.6 から `System.Drawing` は読み込める。

### 2.2 資源を読む道具（テストに使えるもの）

- `windows` 0.62.2（`Cargo.lock` の版）の、根の `[workspace.dependencies.windows]` の機能に `Win32_System_LibraryLoader`・`Win32_UI_WindowsAndMessaging`・`Win32_Graphics_Gdi` が既にある。`areka` は通常の依存に `windows` を持つので、`crates/areka/tests/` の結合テストからも使える。確かめた所在:
  - `Win32::System::LibraryLoader`: `LoadLibraryExW`・`FindResourceW`／`FindResourceExW`・`LoadResource`・`LockResource`・`SizeofResource`・`EnumResourceNamesW`、`LOAD_LIBRARY_AS_DATAFILE`・`LOAD_LIBRARY_AS_IMAGE_RESOURCE`
  - `Win32::UI::WindowsAndMessaging`: `RT_ICON`・`RT_GROUP_ICON`・`RT_MANIFEST`、`LookupIconIdFromDirectoryEx`・`CreateIconFromResourceEx`・`LoadImageW`
  - `Win32::Foundation`: `FreeLibrary`
- `Cargo.lock` には PE を読めるクレート `object` 0.37.3 も推移の依存として在る（PE の資源の目録を読む機能を持つ）。直接の依存に書けば使えるが、`windows` で足りるので要らない見込み。
- 結合テストの前例: `crates/areka/tests/smoke_boot_loop_exit.rs` は `env!("CARGO_BIN_EXE_areka")` で組んだ `areka.exe` の場所を受け取る（Cargo が結合テストの前に bin を組んで渡す）。`build.rs` の頭の説明のとおり、bin への指示は結合テストの exe そのものには届かないが、テストが開くのは `areka.exe` のファイルなので問題にならない。
- 組んだ `areka.exe` にマニフェストが入っていることを確かめるテストは、今は 1 本も無い（`MANIFEST`・`Common-Controls` を綴るテストは 0 件）。要件 6.1 の「マニフェストが今のまま入っている」は新しく足す検査になる。

### 2.3 配布

- `tools/package.ps1`（746 行）: `cargo build --locked --release -p areka -p areka-mcp-bridge --target <x64 または aarch64>` で組み、zip に入れてよい exe の一覧 `ALLOWED_EXECUTABLES`（`areka.exe`・`areka-mcp-bridge.exe`・`shiori-host32-helper.exe`・`pasta.dll`）と機械の種別を検査する。`--locked` なので、依存を足すと `Cargo.lock` の更新が先に要る。資源が増えても手順と一覧は変わらない（要件 4.1）。
- `.github/workflows/release.yml`: `windows-2025-vs2026` の上で x64 と arm64 の zip を作る。arm64 のリンクには VS の `VC.Tools.ARM64` が要り、`vswhere` で有無を問う（`package.ps1` と同じ）。資源のファイルを渡すのは同じ `link.exe` なので、道具は増えない見込み。
- winget は配布の zip をそのまま入れるので、zip の `areka.exe` に入れば winget にも入る。

### 2.4 全体テストと道具の検査

- `tools/test-all.ps1` の段 4 `cargo test --workspace --no-fail-fast -j 4` が `crates/areka/tests/*.rs` を回す。新しい結合テストのファイルを置けば、段を足さずに全体テストに入る（要件 6.1）。
- 段 `encoding check`（`tools/encoding-check.ps1`）は `tools/*.ps1` を全部、構文で調べる（端末へ出す文の ASCII の外の字・端末の文字コードの書き替えを赤にする）。新しい作り直しの道具を `tools/` に置けば、要件 5.7 は既存の段が見張る。
- 道具の自分の検査を全体テストの段にした前例は `release-notes.ps1 -SelfTest`・`roadmap-overview.ps1 -SelfTest`。要件の仮置きでは段を足さない。
- 全体テストは arm64 を組まない。要件 6.4 の arm64 の確かめは実機の手順の側になる。

### 2.5 アイコンを読む・出すコード

- `crates/` に `LoadImageW`・`LoadIconW`・`LoadIconMetric`・`Shell_NotifyIcon`・`NOTIFYICONDATA`・`hIcon`・`FindResourceW` を綴る所は 0 件。wintf の窓クラスにもアイコンは無い。下流 `tasktray-icon` の brief も「資源の番号は `areka-P0-app-icon` が決める」と待っている。
- 参考: 通知領域の絵を拡大率に合わせて引く `LoadIconMetric` は comctl32 の版 6 にしか無い。areka.exe は版 6 の申告を既に持つので、下流はそのまま使える（本仕様がマニフェストを壊さないこと＝要件 3.4 が下流の前提にもなる）。
- `crates/areka/shell/icon.png`（114×114・RGB・α なし）は昔の仮シェルの絵で、areka の顔ではない。使い道は無い。

### 2.6 権利の記録の前例

- `vendors/sample_ghost/README.md`: 検体ごとに「出どころとライセンス」の節を立て、出典の URL と条件を書く形。
- `dist/README.txt`（zip に入る）: 「■ 同梱物とライセンス」の節で、本体（MIT）・同梱のゴースト・バルーンの条件と出どころを並べている。アイコンの絵の権利を足すなら自然に入る場所（議題 4）。
- `tech.md`: 「`THIRD-PARTY-NOTICES.md` は `cargo about` の自動生成で cargo 依存でない資産は載らない」「資産の出典告知は第三者向け README が担う」。要件 5.3 と同じ考え方。

### 2.7 古くなった記述（本仕様の範囲の外・申し送り）

- `structure.md` の「本ワークスペースの `crates/*` に `benches/` と `build.rs` は現状 0 件」は古い（`crates/areka/build.rs`・`crates/wintf/build.rs` が在る）。本仕様が `build.rs` を太らせる場合は、完了のときに直す候補。
- brief の古い記述（crates.io から組む利用者・`winget-manifest-submission` が進行中）は、requirements.md の「要件生成時に引き直した事実」で既に正してある。

---

## 3. 要件と今の資産の対応

| 要件 | 使える物 | 足りない物 | 種別 |
| --- | --- | --- | --- |
| 1（絵の案と選定） | fal の MCP（開発者の持ち分） | 絵柄の方向・モデルの利用条件の確かめ・案 | 開発者の決め／調べが要る（モデルの条件） |
| 2.1〜2.3（2 種・8 寸法） | 無し | 原画 2 枚・小さい寸法の手仕上げ（16〜48）・アイコンのファイル 2 つ | 足りない |
| 2.4（明暗の地で見分けられる） | 無し | 実機で目で見る手順 | 足りない（判定は開発者） |
| 2.5（動かない絵） | ― | ― | 制約だけ |
| 3.1・3.2（exe に 2 つ・番号で引ける） | `build.rs` の bin だけに渡す形 | 資源のファイル・番号の決め | 足りない |
| 3.3（番号を設計に書く・変えない） | ― | 番号の表 | 足りない（5 節） |
| 3.4（マニフェストを二重に入れない） | 今の `/MANIFEST:EMBED` | 資源の側に `RT_MANIFEST` を入れない決め | 制約 |
| 3.5（MSVC でなければ何もしない） | 今の早期の `return` | ― | 既に在る（同じ所に足すだけ） |
| 3.6（x64・arm64 で同じ） | `.res` は機械の種別に依らない形式（7 節で確かめる） | arm64 で組んで確かめること | 調べが要る（低） |
| 3.7（spec のフォルダを読まない） | ― | `crates/areka/` の下の置き場 | 足りない |
| 3.8（ほかの exe を変えない） | `rustc-link-arg-bins` はそのクレートの bin だけ | ― | 既に満たす形 |
| 4.1（zip の手順を変えない） | `package.ps1` は exe の中身に関わらない | ― | 既に満たす形 |
| 4.2（道具を増やさない） | 案 A／その変種なら組み立てに外の道具は要らない | ― | 埋め方の決めしだい |
| 4.3（依存を足さない） | `windows` の既存の機能で足りる | ― | 案 A／その変種なら満たす |
| 5.1〜5.3（置き場と記録） | 検体の README・`dist/README.txt` の前例 | アイコンのフォルダ・記録の文書 | 足りない／議題 4 |
| 5.4〜5.7（作り直しの道具） | `encoding check` が `tools/*.ps1` を見張る | 道具そのもの（入力の検査・同じバイト・上書きしない） | 足りない |
| 5.8（手順の説明） | ― | 説明の文書 | 足りない |
| 6.1・6.2（全体テストで引ける・8 寸法・マニフェスト） | `CARGO_BIN_EXE_areka`・`windows` の資源の API | 結合テスト 1 本 | 足りない |
| 6.3（実機の目視） | ― | 手順（アイコンの覚えを避ける・明暗・100% と 200% 以上） | 足りない |
| 6.4（arm64 も引ける） | 結合テストの読み方はファイルとして開くだけ | arm64 の exe を渡す口（7 節） | 足りない／調べが要る（低） |

---

## 4. 実装の案

### 4.1 埋め方（議題 3）

どの案も `build.rs` の「MSVC でなければ返る」の後ろに足すので、要件 3.5 は同じに満たす。資源の側に `RT_MANIFEST` を入れない点も共通（入れるとリンカがマニフェストの重なりで落ちる）。

**案 A: 資源のファイル（`.res`）を作り直しの道具で作ってコミットし、`build.rs` がリンカへ渡す（要件の仮置き）**

- 触る所: `build.rs` に `cargo:rerun-if-changed=<.res>` と `cargo:rustc-link-arg-bins=<.res の絶対パス>` の 2 行。`.res` はアイコンのフォルダに置く。
- ✅ 組み立てに道具も依存も要らない。`build.rs` はほぼ今の大きさ。
- ✅ リンカは `.res` を引数で受け取り、中で目的の機械の形に直すので、x64・arm64 とも同じ 1 つで済む見込み。
- ❌ `.ico` と `.res` の 2 つのバイナリをコミットし、原画を直したのに道具を回し忘れると、古い `.res` のまま組まれる（`.ico` と `.res` がずれても組み立ては気付かない）。下の 4.4 の「テストで原画と比べる」で拾える形にできる。

**案 A′（変種）: アイコンのファイルだけコミットし、`build.rs` が組み立てのたびに std だけで `.res` を `OUT_DIR` に書いてリンカへ渡す**

- `.res` の形は単純（32 バイトの空の頭の後に、資源ごとに「頭＋中身」を並べるだけ）で、`.ico` を読んで `RT_ICON` と `RT_GROUP_ICON` に組み替えるのは数十行で済む。外の道具も依存も要らない。
- ✅ コミットするバイナリが 1 つ減り、`.ico` と `.res` のずれが起きない。
- ❌ `build.rs` が 22 行から数倍になり、`build.rs` の中の組み替えは単体のテストにしにくい（結果は 4.4 の結合テストで見る）。
- 要件への影響: 要件 4.3 は「議題 3 で案 A と決まった（仮置き）」に掛かっているので、この変種を選ぶなら文言を「依存を足さない埋め方」に直す。要件 5.1・5.4 の「exe に渡すファイル」は `OUT_DIR` の中になり、作り直しの道具が作るのはアイコンのファイルだけになる。

**案 B: 資源の書き方のファイル（`.rc`）を組み立てのたびに処理する依存（`embed-resource`・`winresource` など）を足す**

- ✅ よく使われる形で、`.rc` に版の情報なども書き足しやすい。
- ❌ 組み立てのたびに Windows SDK の `rc.exe` が要る（今の机では PATH に無く、依存の側が探す）。要件 4.2 の「今より多くの道具を要しない」に反する。`Cargo.lock` が変わり、ウェーブに 1 つの「依存を足す席」を取る（roadmap）。`THIRD-PARTY-NOTICES.md` と `tech.md` の追記も要る。
- 要件の線引きとぶつかるので、選ぶなら要件 4.2・4.3 を直す。

### 4.2 アイコンのファイルの中身の形

- **形 1: 8 寸法とも PNG のまま入れる**。Windows Vista 以降はどの寸法でも PNG の入ったアイコンを読む（7 節で確かめる）。作り直しの道具は、入力の PNG のバイトをそのまま並べるだけ（絵を解かない）なので、同じ入力から同じバイトが出る（要件 5.5）のが自然に守れ、入力の検査も PNG の頭（寸法・色の形式）を読むだけで済む。
- **形 2: 256 だけ PNG、16〜64 はビットマップ（32 ビットの BGRA と透過の印）に直して入れる**。Microsoft の作り方の案内に近い形で、古い読み手でも確実。ただし道具が PNG を解いて画素を並べ直すことになり（pwsh の `System.Drawing` で可能）、.NET の版で結果が変わらないことを確かめる手間が増える。ファイルも大きくなる。
- どちらでも要件の文言は変わらない。areka は Windows 10/11 だけが相手（`tech.md`）。

### 4.3 作り直しの道具

- **道具 1: `tools/` の PowerShell の新しいスクリプト**（前例に揃う・`encoding check` が自動で見張る）。形 1 なら絵を解かずにバイトを並べるだけで、`.ico`（と案 A なら `.res`）を書く。入力の検査（足りない寸法・違う寸法・α の無い PNG）で落とし、一時ファイルに書いてから置き換える（要件 5.6）。
- **道具 2: Windows SDK の `rc.exe` を呼ぶ**（`.rc` を書いて `.res` にする）。道具を回す机には SDK が要り、`rc.exe` の場所を探す処理が要る。道具 1 で `.res` を自前で書けば要らない。
- **道具 3: Rust の小さな実行ファイル（例・開発の道具）で `image` クレートを使う**。`image` は `ico` の機能を有効にしておらず、機能を足すと組み立ての中身が変わる。PowerShell の前例から外れるので、選ぶ理由は薄い。

### 4.4 テスト（要件 6.1・6.2）

- **テスト 1: `crates/areka/tests/` に結合テストを 1 本足し、`windows` の資源の API で読む**。`LoadLibraryExW(CARGO_BIN_EXE_areka, LOAD_LIBRARY_AS_DATAFILE …)` で開き、2 つの番号の `RT_GROUP_ICON` を引き、組の目録（6 バイトの頭＋1 枚 14 バイト）を解いて 8 寸法（256 は目録では 0 と書かれる）が揃うこと、目録が指す `RT_ICON` が全部在ること、`RT_MANIFEST` の 1 番に版 6 の申告が在ることを見る。欠けたら番号と寸法を文で出す（要件 6.2）。`crates/areka/src/` を触らない（`main.rs` の行を増やさない約束のまま）。`#[cfg(target_env = "msvc")]` で非 MSVC では飛ばす（要件 3.5 と揃える）。
- **テスト 2: std だけで PE のファイルを解いて資源の目録を辿る**。OS の読み込みに頼らないので、機械の種別の違う exe（arm64）を x64 の机で読むことに疑いが無い。ただしコードは百行ほどになる。
- **テスト 3: `object` クレートを開発用の依存に書いて PE の資源を読む**。`Cargo.lock` に既に在る版なら新しいクレートは増えない見込みだが、要件 4.3（依存を足さない）の読み方しだいで議題になる。
- **足し算（どの案にも乗る）**: 形 1（PNG のまま）なら、exe の `RT_ICON` の中身とアイコンのフォルダの PNG のバイトを突き合わせられる。原画を直して道具を回し忘れたことを、全体テストの段を足さずに拾える（requirements の仮置き「作り直し忘れの検査＝段を足さない」と両立する）。

### 4.5 arm64 の確かめ（要件 6.4）

- **口 1: 結合テストが、環境変数で渡された別の exe も読む**（無ければ飛ばす。実機の手順で `package.ps1 -Arch arm64` の後に変数を立てて回す）。既存の「環境変数が無ければ飛ばす」実機テストの流儀に合う。
- **口 2: 作り直しの道具に「exe を読んで資源を一覧する」動きを足す**（pwsh から `LoadLibraryEx` を呼ぶ）。道具の役目が増える。
- **口 3: 手順の説明に、エクスプローラーで arm64 の exe のファイルを見る手順だけを書く**。一番軽いが「資源を読む」確かめとしては弱い。

### 4.6 置き場

- `crates/areka/` の下に新しいフォルダ（例: `crates/areka/icon/`・`crates/areka/assets/icon/`）。今の `crates/areka/shell/` は仮シェルの絵の置き場で、混ぜない方が分かりやすい。名前は設計の段で決める。
- spec のフォルダの原画は、実装でこのフォルダへ写す（要件 3.7）。原画の 2 つ持ち（spec のフォルダと製品のフォルダ）は、完了で spec のフォルダが `completed/` へ移るまでの間だけ。

---

## 5. 番号の振り方で押さえる事実

- エクスプローラー・ショートカットが exe の絵にするのは、資源の目録の並びで**最初のアイコンの組**。目録は名前の付いた資源が先（文字の順）、番号の資源が後（小さい順）に並ぶ。だから**アプリのアイコンの組の番号を通知領域用より小さくし、どちらも名前ではなく番号にする**のが要件 3.1 を満たす素直な形（例: アプリ＝1、通知領域＝2）。
- 1 枚ずつの絵（`RT_ICON`）の番号は、2 つの組の間で重ならないようにする（例: アプリの 8 枚＝1〜8、通知領域の 8 枚＝9〜16、または組ごとに 100 台・200 台に分ける）。
- 資源の言語は「言語に依らない」（0）にしておくと、`FindResourceW`・`LoadIconMetric` がどの表示言語でも引ける（7 節で確かめる）。
- 下流は `MAKEINTRESOURCE(番号)` で引く。番号の定数は下流が自分のモジュールに置く（要件の Adjacent expectations）。
- 将来、通知領域用に明るい地用と暗い地用の絵を分けたくなったときのために番号を空けておくかは、決めておくと後で揉めない（要件は 1 枚で両方の地に見えることを求めているので、今は要らない）。

---

## 6. 規模と危うさ

- **規模: S〜M（5〜8 タスク・brief の見立てのまま）**。`build.rs` の数行・道具 1 本・結合テスト 1 本・文書 2 つ（手順と権利）・絵の仕上げ。案 A′ なら `build.rs` が増える分だけ M 寄り。
- **危うさ: 技術は低、全体は中**。埋め込みとテストは既存の形と既存の API に乗る。時間の読めない所は、要件の段の絵の往復（何回か出し直す）と、16 ピクセルの手仕上げの出来（明暗の地で見分けられるかは実機で目で見るまで分からない）。

---

## 7. 調べが要ること（設計の段か、要件の討議で）

1. **画像生成のモデルの利用条件**（要件 1.2）: fal で使うモデルごとに、生成した絵を商用で再配布してよいか（モデルの重みの許諾と、fal の利用規約の両方）。出どころの URL と確かめた日を残す。モデルによって条件がまるで違うので、案を出す前に必ず引く。
2. **`.res` をリンク引数で渡せること**: `cargo:rustc-link-arg-bins=<…>.res` を MSVC の `link.exe` が資源として受け取ること、arm64 の `link.exe`（`Hostx64\arm64`）でも同じ 1 つで通ること。最初の x64 と arm64 の組み立てで確かめられる（`package.ps1 -Arch arm64` が既に arm64 を組む）。
3. **PNG の入ったアイコンを小さい寸法でも読むこと**（形 1 を選ぶとき）: エクスプローラー・タスクマネージャー・`LoadIconMetric`（下流）・`LoadImageW` が 16〜64 の PNG の絵を正しく出すか。Microsoft の文書で確かめ、実機の目視（要件 6.3）でも見る。
4. **機械の種別の違う exe を資源として開けること**（テスト 1 で arm64 も読むとき）: x64 のプロセスから `LOAD_LIBRARY_AS_DATAFILE` で arm64 の exe を開いて資源を引けるか。だめならテスト 2（std で PE を解く）にする。
5. **資源の言語**: 言語 0 の資源を `FindResourceW` と `LoadIconMetric` が日本語・英語の表示の両方で引けること。
6. **エクスプローラーのアイコンの覚えの消し方**（要件 6.3）: 覚えを消す手順（`ie4uinit.exe -show` など）か、別の名前に写した exe で見るかを、手順の説明に書けるよう確かめる。

---

## 8. 設計の段への申し送り

- 推す方向（決めではない）: 埋め方は案 A か案 A′（どちらも依存なし・道具なし）。アイコンのファイルは形 1（PNG のまま）にすると、道具が単純になり、同じバイトの約束と「作り直し忘れ」の検出がただで付く。テストは `crates/areka/tests/` の結合テスト 1 本（`windows` の資源の API）。番号はアプリ＝小さい番号。
- 要件の討議で決めてほしいこと（下の「議題の候補」）が決まれば、設計は迷う所が少ない。
- 完了のときに直す候補: `structure.md` の「`build.rs` は 0 件」の古い記述。

### 議題の候補（要件の討議へ）

1. **埋め方を案 A（`.res` もコミット）にするか、案 A′（`.ico` だけコミットし `build.rs` が `.res` を作る）にするか**。A′ なら要件 4.3 の前提の文言と 5.1・5.4 の「exe に渡すファイル」を直す。案 B（依存を足す）は要件 4.2・4.3 とぶつかる。
2. **アイコンのファイルの中を 8 寸法とも PNG にするか、小さい寸法をビットマップにするか**。道具の単純さ・同じバイトの守りやすさと、古い読み手への確かさの引き換え。
3. **番号の値**（組の番号＝アプリ 1・通知領域 2 を軸に、1 枚ずつの絵の番号の振り方、資源の言語を 0 にするか）と、明暗で分けた通知領域の絵のために番号を空けておくか。
4. **作り直しの道具の作り**（`tools/` の PowerShell で `.res` まで自前で書くか、SDK の `rc.exe` を呼ぶか）。
5. **テストの読み方**（`windows` の資源の API／std で PE を解く／`object` を開発用の依存に書く）と、exe の中の絵と原画の PNG を突き合わせて「作り直し忘れ」を拾うか。
6. **arm64 の確かめの口**（結合テストが環境変数の exe も読む／道具に一覧の動きを足す／目視だけ）。
7. **権利の記録の置き場**（議題 4 の続き）: アイコンのフォルダの説明の文書だけか、`dist/README.txt` の「■ 同梱物とライセンス」にも 1 項目足すか。
8. **アイコンのフォルダの名前**（`crates/areka/icon/` など）と、手順の説明をどこに置くか（フォルダの README か `doc/` か）。
9. **画像生成のモデルの選定**（7 節の 1 の確かめを済ませてから・要件 1.2・1.3）。

---

# 設計の段の調べと決定（2026-10-16・`/kiro-spec-design`）

## Summary

- **Feature**: `areka-P0-app-icon`
- **Discovery Scope**: Extension（既存の `build.rs` の形に資源を 1 つ足す）。軽い調べ（design-discovery-light）で足りた。新しい依存は無い。
- **使った手引き**: スキル `kiro-spec-design` の規則（design-principles・design-discovery-light・design-synthesis・design-review-gate）。ほかのスキルは読んでいない（絵は要件の段で決まっていて、画面や操作の設計は無い）。
- **Key Findings**:
  - Windows 11 の机で、PNG のまま入れた 16 ピクセルの絵を `CreateIconFromResourceEx` と `LoadImageW`（16・24・32）が読み、描けた（下の調べ 1）。アイコンのファイルの中は 8 寸法とも PNG で足りる。
  - pwsh 7.6.7 の `System.Drawing`（GDI+）で 1024 の原画を縮めて PNG に書くと、同じ机では 2 回とも同じバイトになり、塊は `IHDR・sRGB・gAMA・pHYs・IDAT・IEND`（時刻の塊 `tIME` は無い）。色の形は 8 ビットの RGBA（色の型 6）。
  - MSVC の `link.exe` は入力のファイルの中身を見て `.res` を資源として受け取る（拡張子で決めない）。`rustc-link-arg-bins` に `.res` の絶対パスを 1 つ足せば埋まる。
  - `app.svg`・`tray.svg` は 2 人の輪郭の線（`girl.svg`・`unyuu.svg` の `d`）をそのまま中に持ち、外のファイルを読まない（`href` は 0 件）。焼き直しに要るのは `app.svg`・`tray.svg` の 2 つだけ。

## Research Log

### 調べ 1: PNG のまま入れた小さい絵を Windows が読むか
- **Context**: ギャップ分析 7 節の 3。外の資料（Wikipedia の ICO の項・Raymond Chen の記事）は 256 の PNG しか語らず、16 ピクセルの PNG を読むかを書いていなかった。
- **Sources Consulted**: 机での実測（Windows 11 Pro 10.0.26300・pwsh 7.6.7）。`tray.png` を GDI+ で 16・32・256 に縮めて PNG に書き、① 16 の PNG のバイトを `CreateIconFromResourceEx(…, fIcon=TRUE, 0x00030000, 16, 16, 0)` に渡す、② 3 寸法とも PNG の `.ico` を書いて `LoadImageW(…, IMAGE_ICON, n, n, LR_LOADFROMFILE)` で 16・24・32 を引く。
- **Findings**: ① はハンドルを返し、`DrawIconEx` で 16×16 の地に描くと 192 画素が色を持った。② は 3 つともハンドルを返した（24 は 32 から縮めて引かれる）。
- **Implications**: 8 寸法とも PNG にする（形 1）。道具は手で整えた PNG をバイトのまま入れられ、テストは exe の中の絵とフォルダの PNG をバイトで突き合わせられる。エクスプローラー・タスクマネージャー・ショートカットでの見え方は要件 6.3 の実機の確かめで見る。

### 調べ 2: `.res` をリンカの引数で渡せるか
- **Sources Consulted**: <https://learn.microsoft.com/en-us/cpp/build/reference/link-input-files>（LINK は拡張子で決めず中身を見る・入力の種類に `.res` がある）。
- **Findings**: `.res` は COFF に直されて資源の節に入る。arm64 の `link.exe` も同じ入力を受ける（`.res` は機械の種別を持たない形式）。
- **Implications**: 埋め方は `build.rs` に 2 行（`rerun-if-changed` と `rustc-link-arg-bins`）。arm64 で本当に通ることは、実装で `tools/package.ps1 -Arch arm64` を 1 回回して確かめる（要件 6.4 の手順でも見る）。

### 調べ 3: 機械の種別の違う exe を資源として開けるか
- **Sources Consulted**: <https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw>
- **Findings**: `LOAD_LIBRARY_AS_DATAFILE` は「ファイルをデータとして写すだけで、動かす準備を何もしない」・「メッセージや資源を引くためだけに読むときに使う」。`FindResource` はこの写しを使える。機械の種別を問う記述は無い（動かす準備をしないので問わない）。
- **Implications**: テストは `LOAD_LIBRARY_AS_DATAFILE` だけで開く（`LOAD_LIBRARY_AS_IMAGE_RESOURCE` は付けない＝像としての写しを避け、arm64 の exe を x64 のテストから開く道を素直にする）。arm64 の exe を実際に読めることは実装の確かめで 1 回見る。だめだったときは、std で PE の資源の目録を辿る形（ギャップ分析 4.4 のテスト 2）へ替える。

### 調べ 4: GDI+ の縮小と PNG の書き出しの決まり方
- **Findings**: 同じ机・同じ入力で 2 回書いた 24 ピクセルの PNG がバイトまで同じだった。GDI+ は OS の部品なので、OS の更新で縮め方か書き出しが変われば、入力が同じでも縮めた絵のバイトが変わりうる。
- **Implications**: 要件 5.5（同じ入力から同じバイト）は「同じ机（同じ OS の版）で」守れる。OS の版で変わりうることを説明の文書に書き、差分が出たときは、入力を変えていないのに原画から縮めた寸法だけが変わったなら OS の変化と読む手順を添える。決まり方を OS から切り離すには縮め方と PNG の書き出しを自前で持つ必要があり、圧縮を自前で持たないと 256 の絵が 1 枚 256 KB になる。引き換えに見合わないので採らない。

### 調べ 5: 埋めたマニフェストの読み方
- **Findings**: `link.exe` の `/MANIFEST:EMBED` は `areka.manifest` を自分の既定（UAC の申告など）と合わせて書き直して埋める。属性の引用符や並びが変わりうるので、元のファイルとバイトで比べられない。
- **Implications**: テストは `RT_MANIFEST` の 1 番の中に `Microsoft.Windows.Common-Controls` と `6.0.0.0` の 2 つの文字列があることを見る（引用符に依らない）。`.res` の側は `RT_ICON` と `RT_GROUP_ICON` しか書かないので、マニフェストが二重に入ることは道具の作りで起きない（同じ 1 番が 2 つあればリンカが落ちる）。

## Architecture Pattern Evaluation

| 案 | 中身 | 強み | 弱み | 結論 |
|----|------|------|------|------|
| 案 A（`.res` もコミット） | 道具が `.ico` 2 つと `.res` 1 つを書き、`build.rs` は `.res` をリンカへ渡すだけ | `build.rs` は 2 行増えるだけ。資源の形を知るのは道具 1 か所。組み立てに道具も依存も要らない。要件 5.1・5.4 の文言（「exe に渡すファイル」を道具が作る）のまま | バイナリを 3 つコミットする。道具の回し忘れ | **採る**。回し忘れは結合テストの突き合わせで拾う |
| 案 A′（`build.rs` が std で `.res` を書く） | `.ico` だけコミットし、`build.rs` が `.ico` を読んで `OUT_DIR` に `.res` を書く | `.ico` と `.res` がずれない | 資源の形の知識が道具（PowerShell）と `build.rs`（Rust）の 2 か所に分かれる。`build.rs` が数倍に太る。要件 5.4 の「道具が exe に渡すファイルを作る」を直すことになる | 採らない |
| 案 B（`.rc` を処理する依存） | `embed-resource` などを足す | よく使われる | 組むたびに SDK の `rc.exe` が要る（要件 4.2 とぶつかる）・`Cargo.lock` が変わる（要件 4.3 とぶつかる） | 要件の討議で既に不採用 |

## Design Decisions

### Decision: 埋め方は案 A
- **Context**: 要件の討議が設計へ回した (a)。
- **Selected Approach**: `tools/app-icon.ps1` が `app.ico`・`tray.ico`・`icons.res` を書き、3 つともコミットする。`build.rs` は「MSVC でなければ返る」の後ろで `icons.res` の絶対パスを `rustc-link-arg-bins` に渡し、`rerun-if-changed` に足す。
- **Rationale**: 書く量が一番少なく、資源の形の知識が 1 か所に集まる。要件の文言とそのまま合う。
- **Trade-offs**: 道具の回し忘れが起こりうる → 結合テストが exe の中の絵と `.ico` と手で整えた PNG をバイトで突き合わせて拾う（原画 1024 の直しだけは拾えない＝下の Risks）。

### Decision: アイコンのファイルの中は 8 寸法とも PNG
- **Context**: (b)。
- **Selected Approach**: 8 寸法とも 32 ビット RGBA の PNG をそのまま並べる。
- **Rationale**: 調べ 1 で 16 ピクセルの PNG を Windows が読むことを確かめた。手で整えた PNG をバイトのまま入れられ、突き合わせの検査がただで付く。areka の相手は Windows 10/11 だけ（`tech.md`）。
- **Trade-offs**: XP など古い読み手は読めない（相手にしない）。

### Decision: 番号はアプリ＝1・通知領域＝2・言語は 0
- **Context**: (c)。
- **Selected Approach**: `RT_GROUP_ICON` の 1 番＝アプリ、2 番＝通知領域。1 枚ずつの絵（`RT_ICON`）は 1〜8＝アプリ、9〜16＝通知領域（どちらも寸法の小さい順）。資源の言語はどれも 0（言語に依らない）。名前の付いた資源は使わない。
- **Rationale**: エクスプローラーは資源の目録の並び（名前が先・番号は小さい順）で最初のアイコンの組を exe の絵にする。名前を使わず番号 1 をアプリにすれば、要件 3.1 がそのまま満たされる。言語 0 は表示言語によらず `FindResourceW`・`LoadIconMetric` が引ける。
- **Follow-up**: 下流 `tasktray-icon` は 2 番を自分のモジュールの定数に置いて引く。明暗で分けた絵のために番号を空けておくことはしない（要件は 1 枚で両方の地に見えることを求める）。

### Decision: 作り直しの道具は `tools/app-icon.ps1`（pwsh＋GDI+）
- **Context**: (d)。
- **Selected Approach**: 入力はアイコンのフォルダの 1024 の原画 `app.png`・`tray.png` と、手で整えた `<名>-<寸法>.png`。寸法ごとに、手で整えた PNG が在ればバイトのまま使い、無ければ原画を GDI+ で縮めて PNG に書く。通知領域用の 16〜48 は手で整えた PNG を必ず要する（無ければ失敗）。全部の検査とバイトの組み立てを先に済ませてから、一時ファイルに書いて置き換える。端末へ出す文は ASCII だけ。
- **Alternatives Considered**: SDK の `rc.exe` を呼ぶ（机に SDK が要り、場所を探す処理が要る・`.res` は数十行で自前で書ける）／Rust の小さな実行ファイル（PNG を読むには `image` などを `areka` の依存に足すことになり `Cargo.lock` が変わる）／縮め方と PNG の書き出しを自前で持つ（調べ 4）。
- **Rationale**: 前例（`tools/*.ps1`）に揃い、段 `encoding check` が自動で見張る（要件 5.7）。GDI+ は Windows に入っていて道具が増えない。
- **Follow-up**: 道具の自分の検査 `-SelfTest`（同じバイト・入力の欠け・寸法違い・形式違い・上書きしないこと）を持つ。全体テストの段には足さない（要件の討議の決め「`tools/test-all.ps1` を触らない」）。

### Decision: テストは `windows` の資源の API で組んだ exe を読む
- **Context**: (e)。
- **Selected Approach**: `crates/areka/tests/embedded_icon_test.rs`（`#![cfg(target_env = "msvc")]`）。`LoadLibraryExW(…, LOAD_LIBRARY_AS_DATAFILE)` で開き、`FindResourceW`・`LoadResource`・`LockResource`・`SizeofResource` で 2 つの組と 16 枚の絵とマニフェストを引く。exe の絵と `app.ico`・`tray.ico` の絵、在る `<名>-<寸法>.png` をバイトで突き合わせる。
- **Rationale**: 必要な `windows` の機能（`Win32_System_LibraryLoader`・`Win32_UI_WindowsAndMessaging`・`Win32_Foundation`）は根の `[workspace.dependencies.windows]` に既に在り、`areka` の通常の依存なので結合テストから使える。依存も機能も足さない。

### Decision: arm64 の確かめは同じテストに環境変数で exe を渡す
- **Context**: (f)。
- **Selected Approach**: 環境変数 `AREKA_ICON_CHECK_EXE` に exe の絶対パスがあれば、同じ検査をその exe にもかける。無ければ「飛ばした」を標準エラーへ 1 行出して通す（実機テストの流儀・`AREKA_EMO2_REAL_RUN` と同じ）。手順の説明に、arm64 の exe の作り方とこの変数での回し方を書く。
- **Rationale**: 検査の本体を 1 つに保ったまま、全体テストに arm64 の組み立てを持ち込まない。

### Decision: フォルダは `crates/areka/icon/`・説明は同じフォルダの `README.md`
- **Context**: (g)・要件 5.1・5.3・5.8。
- **Selected Approach**: 原画（`app.png`・`tray.png`）・焼き方の SVG（`app.svg`・`tray.svg`）・手で整えた PNG・出力（`app.ico`・`tray.ico`・`icons.res`）・説明（`README.md`）を 1 か所に置く。`README.md` に中身の一覧・番号の表・作り直し方・焼き直し方・出どころと権利・実機の確かめ方を書く。`girl.svg`・`unyuu.svg`・`girl-g0.png` は spec の `art/` に記録として残し、製品のフォルダには写さない（`app.svg`・`tray.svg` が同じ線を中に持つため）。
- **Rationale**: 今の `crates/areka/shell/`（仮シェルの絵）と混ぜない。組み立ては spec のフォルダを読まない（要件 3.7）。

## Synthesis

- **まとめられたこと（一般化）**: 「寸法ごとに、手で整えた PNG が在ればそれ、無ければ原画を縮める」という 1 つの決まりで、通知領域用の 16〜48（要件 2.2 で必須）と、それ以外の寸法を同じに扱う。アプリ用の小さい寸法を実機の確かめ（要件 6.3）の後に手で整えたくなっても、ファイルを置くだけで済み、道具もテストも直さない。テストも同じ決まり（在る `<名>-<寸法>.png` を突き合わせる）で動く。
- **作るか借りるか**: `.ico`・`.res` の書き手は借りない（`rc.exe` は机の道具を増やす・Rust のクレートは依存を増やす）。形はどちらも数十行で書ける単純な並び。縮めるのと PNG の書き出しは OS の GDI+ を借りる。資源の読み手は OS の API を借りる。
- **削ったもの**: `build.rs` での `.res` の組み立て（案 A′）・`.res` の存在の前もっての検査（無ければ `link.exe` がファイル名を出して落ちる）・全体テストの段の追加・`.gitattributes`（git は NUL を含むファイルをバイナリと見なす・既存の PNG も `-text` で入っている）・明暗の絵のための空き番号・縮めた絵を別のファイルとして残すこと。

## Risks & Mitigations

- 原画 1024 を直して道具を回し忘れても、結合テストは拾えない（縮めた絵は道具の中でしか作られない）→ 説明の文書の作り直しの手順を「原画を直したら道具を回し、出力 3 つをコミット」と 1 本で書く。手で整えた PNG の直しの回し忘れはテストが拾う。
- GDI+ の版で縮めた絵のバイトが変わりうる（調べ 4）→ 説明の文書に書く。
- アプリ用の 16 ピクセル（2 人の影絵を縮めただけ）が明暗の地で見分けにくいかもしれない（要件 2.4）→ 実機の確かめで見て、足りなければ `app-16.png` などを手で整えて置く（決まりのまま・設計は変わらない）。
- arm64 の `link.exe` が `.res` を受けない・x64 のテストから arm64 の exe を開けない → 実装で 1 回ずつ確かめる。開けなければテストの読み方を std で PE を辿る形へ替える（検査の中身は同じ）。

## References

- [LINK Input Files](https://learn.microsoft.com/en-us/cpp/build/reference/link-input-files) — `.res` を入力に取る
- [LoadLibraryExW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw) — `LOAD_LIBRARY_AS_DATAFILE` で資源だけを引く
- [CreateIconFromResourceEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createiconfromresourceex) — `RT_ICON` の中身から絵を作る
- [The evolution of the ICO file format, part 4: PNG images](https://devblogs.microsoft.com/oldnewthing/20101022-00/?p=12473) — PNG の絵は 32 ビット ARGB で、PNG の頭から始まる
- [ICO (file format)](https://en.wikipedia.org/wiki/ICO_(file_format)) — 目録の形（256 は 0 と書く）

## 設計の討議（2026-10-16）

- **議題 2（推奨案で確定）**: 寸法ごとの PNG を 16 枚ともコミットして正本にし、道具の作り直しは「並べて組むだけ」にした。GDI+ で縮めるのは、無い寸法を 1 度だけ作る `-Seed` に分けた。理由: 縮めた絵のバイトは OS の版で変わりうるので、組むたびに縮めると、入力を変えていないのに机によって差分が出る（要件 5.5 の狙いが崩れる）。結合テストのバイトの突き合わせも 16 枚すべてに効くようになる。上の Synthesis の「在れば使い、無ければ縮める」の決まりと、Risks の GDI+ の項はこれで置き換わる。
- **議題 3（推奨案で確定）**: README の権利は 2 つに分けて書く。女の子の絵・組み合わせ・色・縁取りは areka 本体（MIT）の一部。うにゅうの輪郭は元祖の伺かの相方の形に由来し、開発者の判断で使うもの。areka は元の形の権利を持たず、MIT の許諾はそこに及ばない。`dist/README.txt` には足さない（要件 5.9 の今の判断のまま）。
- **議題 1（個別に議論して確定・2026-10-16）**: 小さい寸法は、設計の段で絵を作り切って開発者に見せ、選んでもらった（開発者「見せてもらわないと判断できない。設計で作りきってこちらに見せて」）。見本は 6 案で、明暗 2 つの地に実寸と 4 倍で並べた。
  - A＝縮めただけ・B＝2 人のドット絵・C＝うにゅうのなめらかな縮小・D＝うにゅうの自動のドット絵・E＝16〜24 を手で点を置いたうにゅう＋32〜48 は D・F＝E の縁を灰だけにしたもの。
  - 選んだもの: 通知領域は E。アプリは 24 以下を E と同じうにゅうだけ、32 以上を 2 人（32〜48 は B、64・256 は原画を縮めたもの）。
  - 理由: 24 以下で 2 人は棒と塊になり見分けられない（A・B）。自動のドット絵はトゲが丸まる（D）。うにゅうの見分けどころ（ツノ・ギザギザ）は点を手で置くと残る（E）。
  - 選んだ 12 枚は spec の `art/` に置いた。見本の段がここで済んだので、検証の指摘 1 の「コミット前に開発者が見本で選ぶ段」はタスクに置かない。
