# Requirements Document

## Project Description (Input)
areka は Windows の API を `windows` クレート 0.62 で呼んでいる。開発者は windows-rs を 0.101.0 まで上げたい（2026-11-14）。きっかけは、開発者が上流へ出した縦書きの要望（microsoft/windows-rs の issue 5369）に、新しいクレート `windows-text` の縦書き対応（PR 5379・2026-11-13 マージ）で答えが返ったことである。ところが 2026-11-14 の `/kiro-discovery` で、行き先は「`windows` の版を上げる」でなく「`windows` クレートをやめ、`windows-core` などの絞ったクレートを 0.101.0（公開の前は git 版）へ移し、API は `windows-bindgen` で作った束縛で呼ぶ（WinRT 側も同じ・`windows-composition` へは乗り換えない）」と決まった。この移行は 6〜8 本になる見込みで、束縛をどこに置くか・どの形で作るかで本体の切り方が変わるが、どれも試さないと決められない。そこで移行の本体を掘る前に、使い捨ての先進坑で 10 の問いを確かめる。成果物はコードでなく知見（問いごとの go／違う／直す＋学び）・移行の本体の刻み方の案・書き換えの対応表・上流へ返す文案で、一次記録は `crates/pilot/examples/pilot-windows-rs-next/README.md`（3 幕）。go の判定は開発者が一次記録を見て下す。

## Introduction

本仕様は先進坑（pilot・使い捨て）である（`.kiro/steering/two-tunnel.md` の規律に従う。置き場の形だけは下の要件 1 の例外）。確かめるのは brief の 10 の問い——C の関数だけのクレートの移し方、COM の束縛の置き場と形、WinRT の合成、`HWND` の持ち方、COM の実装、`windows-text` の縦書き、32bit・arm64 のリンク、規模の数え、書き換えの対応表だけで機械的に回せるか——である。

小さく縦に通す。全クレートを移そうとしない。写すのは問い 1・3・4・6 の分だけで、ほかは新しく小さく書く。出荷するクレートは書き換えず、写すか path の依存で 0.62 のまま借りる。

開発者がこの記録を見て、移行の本体（束縛を作る道具 → クレートの群ごとの移行 → `windows` 0.62 を外す締め → `windows-text` の採用）へ進むかを決められる状態にすることが、本仕様の着地である。

問いごとの見立ての基準（判定そのものは開発者）:
- **go**: 移行の本体がこの方向・この手順で書ける。
- **直す**: 条件つきで書ける。条件と直し方を一次記録に書く。
- **違う**: この方向では書けない。代わりの見立てを一次記録に書く。

本文中の【仮定 n】は brief から決めきれず、仮に置いた所である。末尾の「未決事項」に同じ番号で並べ、要件の討議で決める。

## Boundary Context

- **In scope**:
  - `crates/pilot/examples/pilot-windows-rs-next/` の下の、自分の `[workspace]` と `Cargo.lock` を持つ独立の小さなワークスペース（新しいファイルだけ）。
  - brief の 10 の問いの確かめと、問いごとの見立て・学び。
  - 一次記録（README 3 幕）・移行の本体の刻み方の案と規模の見立て・書き換えの対応表・上流へ返す文案。
  - 本線で新旧（`windows-core` 0.62 と 0.101）を同じ依存の木に並べる段階の移行ができるかの結果（開発者が決める材料として）。
- **Out of scope**:
  - 出荷するクレートの書き換え（移行の本体は go の後に別に起票する）。
  - 「`wintf` 全体が移せるか」の確かめ（先進坑が出すのは写した分の手間と API の全数からの見積もりまで）。
  - `windows-composition`・`windows-canvas`・`windows-reactor` の採用（裁定済み）。
  - `windows-text` の本採用（移行の本体の後の `emo-text-windows-text`）と、`wintf` の label・typewriter の `windows-text` 化。
  - 上流への投稿そのもの（文案まで。出すのは開発者の承認の後）。
  - 「公開の前の 0.101 の系列を本線の `Cargo.lock` に入れてよいか」と、crates.io へ出すクレートと git の依存の兼ね合い（移行の本体とリリースの手順の問いとして残す）。
  - 束縛を作る道具の完成品と、食い違いを全体テストで止める段（移行の本体の 1 本目）。
  - `crates/areka/src/session_end_sync_send_tests.rs` の許可表 `ALLOWED_SYNC_SENDS` の直し（出荷する `crates/wintf-winmsg-executor/src/` を書き換える移行の本体の 2 本目が持つ）。
  - 版の段と着手の席の組み直し（次の棚卸）。
- **Adjacent expectations**:
  - 同じウェーブ C21 の `balloon-flip-side-rules`（`crates/areka-placement/`）・`wintf-surface-auto-promotion`（`crates/wintf/src/ecs/graphics/visual/` ほか）・`runtime-crate-split`（`crates/areka-runtime/`・`crates/areka/`・`Cargo.lock` の 1 本だけの席）・`audit-tools-scripts`（`tools/*.ps1`）とは、本仕様が上の置き場に新しいファイルを足すだけなので、触るファイルの重なりは 0 である。本仕様は根の `Cargo.toml`・`Cargo.lock`・`crates/pilot/Cargo.toml`・`tools/` に触らない。
  - path で借りる `wintf`・`areka-emo-text` は読むだけで変えない。並走する spec がそれらを書き換えても、本仕様は借りたときの作業木の中身で確かめ、その旨を一次記録に書く。
  - 上流（Upstream）: `wintf-winmsg-executor-import` は 2026-11-14 に着地済みで、写しの元 `crates/wintf-winmsg-executor/` が問い 1 の的である。上流の windows-rs の master は未公開で、公開までに API が動きうる。
  - 下流（Downstream）: 移行の本体の各本は本仕様の go を `_Depends(confirmed): pilot-windows-rs-next` の前提依存として持つ。各本の brief は go の後に `/kiro-discovery` で書き、本仕様は書かない。本坑の design は一次記録の検証結果を参照し、同じ結果を二重に書かない。
  - `audit-pilot`（見込みの C22）の受け持ち「`crates/pilot/` 全部」に本仕様の置き場が入るので、`audit-pilot` は本仕様の後になる。

## Requirements

### Requirement 1: 先進坑の規律と独立の置き場

**Objective:** As a 開発者, I want 先進坑のコードが出荷されるどのクレートからも依存されず、本線のビルドと全体テストにも乗らない場所に閉じていること, so that 見立てが go／違う／直す のどれであっても、本線を汚さずにコードを捨てるか知見として残しておける

#### Acceptance Criteria
1. The pilot shall コードと文書を `crates/pilot/examples/pilot-windows-rs-next/` の下にだけ新しく置き、フォルダ名を spec 名 `pilot-windows-rs-next` と一致させる。
2. The pilot shall 置き場を自分の `[workspace]` と `Cargo.lock` を持つ独立のワークスペースにし、本線のワークスペースの一員にしない（二坑モデルの「探索のコードは `crates/pilot` の例に置く」の例外。理由は、1 つのパッケージが `windows_core` の名前で持てる版は 1 つだけで、`crates/pilot` は既に 0.62 を持つため）。
3. The pilot shall 根の `Cargo.toml`・`Cargo.lock`・`crates/pilot/Cargo.toml`・`deny.toml` と、出荷するクレートのどのファイルも変えない。
4. The pilot shall 出荷するクレートを、写すか path の依存で借りるだけにし、出荷するどのクレートにも置き場への依存を足さない。
5. The pilot shall 置き場の直下に `main.rs` を置かない（`crates/pilot` の例として組まれ、本線の全体テストが赤になるため）。
6. The pilot shall 置き場の中の `.rs` をどれも 1 ファイル 1,000 行以下にし、`windows-bindgen` の出力は作業木の `target` の下へ作るか、1,000 行の下に分けて置く。
7. When 本線の全体テストを走らせたとき, the pilot shall 置き場のコードが本線のビルドで組まれず、`crates/` の下の `.rs` と `Cargo.toml` を深さに関係なく読む見張りの試験（`crates/log-capture-kit/tests/` の 4 本・`crates/areka/src/runtime/frame_attach_tests.rs` ほか）も赤にならない状態を保つ。置き場のコードと manifest では、`std::env::temp_dir` を呼ばない・tracing の捕捉先を差す語（`with_default(`・`set_default(` ほか）を綴らない・`tracing-subscriber` の機能 `env-filter` と本番の依存の `log-capture-kit` を宣言しない・検体のパスを自分で組まない・`state.actors()` を書かない。
8. The pilot shall 写す量を合わせて 1,500 行前後に抑え、問い 1・3・4・6 の分（要件 3・4・5・7）のほかは写さずに新しく小さく書く。
9. While 先進坑のコードを書く間, the pilot shall 整形・命名・テストの厳しさを緩めてよいが、本要件の 1〜7 の隔離は必ず守る。
10. The pilot shall 置き場の形の例外を自分の要件・設計・タスクにだけ書き、共通の文書（`.kiro/steering/two-tunnel.md`・`.kiro/steering/workflow.md`）を直さない。
11. The pilot shall 一次成果物をコードでなく知見として扱い、先進坑のコードを本坑へ写して使う前提で作らない（本坑は一次記録を見て掘り直す）。

### Requirement 2: 上流の版の固定と裁定済みの方向

**Objective:** As a 開発者, I want 確かめた上流の版がコミット番号で特定でき、裁定済みの方向（`windows` クレートを使わない・WinRT も `windows-bindgen`）の上で確かめられていること, so that 公開の後に結果が変わったとき、どの版で何を確かめたかを辿れ、確かめた形がそのまま本体の形になる

#### Acceptance Criteria
1. The pilot shall 上流の windows-rs を master の決まった 1 つのコミットで git の依存として引き、置き場の `Cargo.lock` にそのコミットを固定する【仮定 3】。
2. The pilot shall 上流のクレートをフォークも `[patch]` もせず、ソースを書き換えずにそのまま使う。
3. The pilot shall 新しい側のコードで `windows` クレートを使わず、絞ったクレート（`windows-core`・`windows-result`・`windows-link`・`windows-numerics`・`windows-text` など）と `windows-bindgen` で作った束縛だけで Windows の API を呼ぶ。
4. The pilot shall WinRT 側（Windows.UI.Composition・DispatcherQueue）も `windows-bindgen` で作り、`windows-composition`・`windows-canvas`・`windows-reactor` を使わない。
5. If 決めたコミットで先へ進めない所に当たったとき, the pilot shall 別のコミットへ動かす前に、進めない理由と動かした先のコミット番号を一次記録に書く。

### Requirement 3: 問い 1 — C の関数だけのクレート

**Objective:** As a 移行の本体の起票者, I want C の関数だけを呼ぶクレートを `.sys()` の束縛へ移した実例と、その手間の実数, so that 移行の本体の 2 本目（`wintf-winmsg-executor` と `dola`）の規模と手順を見積もれる

#### Acceptance Criteria
1. When 問い 1 を確かめるとき, the pilot shall `crates/wintf-winmsg-executor/` の `src/`（1,036 行・うち `lib.rs` の `#[cfg(test)] mod test` 約 320 行）と例 2 本（`basic`・`threads`・73 行）を置き場の中へ写し、Win32 の呼び出しを `windows-bindgen` の `.sys()` の出力（`windows-link` だけに頼る束縛）へ移す。
2. When 写しを移し終えたとき, the pilot shall 写しの例 2 本を x64 と i686 の両方で組んで走らせ、写す前の例と同じ出力と終了コードで終わることを確かめる。
3. The pilot shall 束縛のフィルターの行数（名指しした API の数）・写しで書き換えた行数・かかった手間（直しの回数と作業の時間）を一次記録に書く。
4. The pilot shall 出荷する `crates/wintf-winmsg-executor/` のファイルを変えない。
5. If 写しが x64 か i686 で組めないか走らないとき, the pilot shall 止まった所と理由を一次記録に書き、問い 1 の見立てを「直す」か「違う」にする。

### Requirement 4: 問い 2・3 — COM の束縛の置き場と形

**Objective:** As a 移行の本体の起票者, I want クレートの間で COM の型を受け渡す置き場の案と、束縛の形の案を、実際に組んで比べた結果, so that 移行の本体を何本にどう刻むかを、置き場と形の決めの上で組める

#### Acceptance Criteria
1. When 問い 2 を確かめるとき, the pilot shall `wintf` の役と `areka-emo-text` の役を演じる 2 つの小さなクレートを置き場の中に建て、一方が作った `ID2D1DeviceContext` などの COM の型を他方へ渡して、DirectWrite の文字を Direct2D で描く最小の例を組む。
2. The pilot shall 束縛の置き場の 3 案（クレートごとに束縛を持つ・`wintf` の役が束縛を公開する・共有の束縛のクレートを置く）を同じ最小の例でそれぞれ組み、組めたか・型の受け渡しに要った手当て・重なった束縛の量を比べて一次記録に書く【仮定 2】。
3. The pilot shall `windows-bindgen` にほかのクレートの束縛の型を参照させる口が在るかを確かめ、在るならその使い方を、無いならその根拠（上流の文書かソースの箇所）を一次記録に書く（2026-11-16 のギャップ分析では、0.100.0 と master に公開の口は無く、0.62 の `--reference` は消えている＝`research.md` 3.2）。
4. When 問い 3 を確かめるとき, the pilot shall `wintf` の `com/dwrite.rs`（154 行）と Direct2D の一部を写し、束縛の形の 2 案（既定の型つき・`.minimal()`＋手書きの包み）でそれぞれ組んで、束縛の行数・包みの行数・呼ぶ側の書き換えの量を比べる。
5. The pilot shall 置き場の案と形の案の組み合わせのうち推す 1 つと、その理由を一次記録に書く。
6. If 案のどれかが組めないとき, the pilot shall 組めなかった所（コンパイルの誤りの要旨か、上流の制約）を一次記録に書き、その案を推さない。

### Requirement 5: 問い 4 — WinRT の合成

**Objective:** As a 移行の本体の起票者, I want 今の Windows.UI.Composition の組み方が `windows-bindgen` の束縛でそのまま通るかの結果, so that `windows-composition` に乗り換えずに `wintf` の合成を移せるかを決められる

#### Acceptance Criteria
1. When 問い 4 を確かめるとき, the pilot shall `wintf` の `com/wuc.rs`（253 行）を写し、`windows-bindgen` で作った WinRT の束縛へ移す。
2. When 写しを走らせたとき, the pilot shall Compositor の生成・DesktopWindowTarget の作成・描画面と SpriteVisual のつなぎ・相互運用のインターフェース・DispatcherQueue の作成の各段が通るかを確かめ、実の窓に絵が出ることを画面で確かめる。
3. If 今の形のまま通らない段があるとき, the pilot shall その段と、代わりの書き方か通らない理由を一次記録に書く。

### Requirement 6: 問い 5 — `HWND` の持ち方

**Objective:** As a 移行の本体の実装係, I want 束縛の形で姿の変わる `HWND` を bevy の Component に持たせる包み方の実例, so that `wintf` の ECS の窓の取っ手を機械的に書き換えられる

#### Acceptance Criteria
1. When 問い 5 を確かめるとき, the pilot shall 新しい束縛の `HWND`（`.sys()`・`.minimal()` の形では生ポインタの別名、既定の形では今と同じ newtype。どちらも `Send` でない）を、問い 3 で比べる束縛の形のそれぞれで包む型を組み、bevy の Component として世界に入れ、別のシステムから読み出せることを確かめる。
2. The pilot shall 包みが `Send`／`Sync` を名乗ってよい根拠を一次記録に書く。
3. The pilot shall 包みと生の `HWND` の行き来の書き方を、書き換えの対応表（要件 12）に載せる。

### Requirement 7: 問い 6 — COM の実装

**Objective:** As a 移行の本体の起票者, I want `windows-core` 0.101 の `#[interface]`・`#[implement]` で SHIORI のインターフェースと `IDWriteTextRenderer1` が組めるかの結果, so that 移行の本体の 4 本目（SHIORI）と、下線の位置を集める自前の描き手の移し方を決められる

#### Acceptance Criteria
1. When 問い 6 を確かめるとき, the pilot shall `shiori-abi` の `interface.rs` のうち 1 組（インターフェースの定義・その実装・呼び出し）を写し、`windows-core` 0.101 の `#[interface]`・`#[implement]` で組んで、呼び出しが実装まで届き値が返ることを確かめる。
2. The pilot shall `IDWriteTextRenderer1` を `#[implement]` で実装し、DirectWrite の描画から下線の位置を集められることを確かめる。
3. The pilot shall `_Impl` トレイトのメソッドが `HRESULT` を返す形での書き換え方を、書き換えの対応表（要件 12）に載せる。
4. If どちらかが組めないとき, the pilot shall 組めなかった所と、代わりの組み方の見立てを一次記録に書く。

### Requirement 8: 問い 7 — `windows-text` の縦書き

**Objective:** As a 開発者, I want `windows-text` の縦書きが今の areka の縦書きと画素で同じに出るか、足りない所は何かの結果, so that `emo-text-windows-text` で `windows-text` を採るかと、上流へ返す要望を決められる

#### Acceptance Criteria
1. When 問い 7 を確かめるとき, the pilot shall 縦書きの 1 列（開き括弧`「`・小書きかな・句読点・カラー絵文字を含む）を `windows-text` で組み、同じ文を今の areka（`areka-emo-text` を 0.62 のまま借りる）で組んだ出力と画素で比べ、違う画素の数・最大の差・差の絵を一次記録に残す【仮定 4】【仮定 8】。
2. The pilot shall 両側の比べを、新旧の COM の型をつながず、それぞれ別に描いた画素の上で行う（新旧の型は `as_interface` でつなげないため。どうしても橋が要る所は生ポインタで渡し、その所を一次記録に書く）。
3. The pilot shall areka の縦書きの字形の指定（`crates/areka-emo-text/src/draw.rs` の `VERTICAL_FEATURES` の 10 個）を `set_font_features` に当て、開き括弧の位置が今の areka と揃うかを画素で確かめる。
4. The pilot shall `decorations()` で下線の位置が取れるかを確かめ、areka の決まり（選択肢の下線は列の右）に合わせて位置を自前で決めるときの手当てを一次記録に書く。
5. The pilot shall 範囲ごとの色を、`as_interface` で取り出した DirectWrite の型の `SetDrawingEffect` で付けられるかを確かめる。
6. The pilot shall UTF-8 のバイト位置と areka の持つ文字の位置の換算の扱いと、失敗で panic する `TextLayout::new` の扱いを一次記録に書く。
7. When 縦書きのバルーンの例を走らせたとき, the pilot shall 透過の枠なしの窓で、Windows.UI.Composition の描画面へ自前の Direct2D で縦書きの文（カラー絵文字を含む）とクリックできる選択肢を描き、実の画面に出す。
8. When 描いた選択肢へクリックを送ったとき, the pilot shall 当たった選択肢をログへ出し、それが送った位置の選択肢と一致することを確かめる。
9. If `windows-text` に無い機能（範囲ごとの色・文中のインラインオブジェクト・字送り・縦中横・文字単位の折り返し）が要る所に当たったとき, the pilot shall 自前で補うか DirectWrite を直に呼ぶかの見立てを一次記録に書き、上流へ返す文案（要件 14）に載せる。

### Requirement 9: 問い 8 — 32bit と arm64 のリンク

**Objective:** As a 開発者, I want `windows-link` の束縛で i686 と arm64 が組めてリンクが通るかの結果, so that 32bit の SHIORI の手伝い役と arm64 の配布を移行の後も続けられるかを判断できる

#### Acceptance Criteria
1. When 問い 8 を確かめるとき, the pilot shall 問い 1 の写し（C の関数）と問い 3 の最小の例（COM）を、i686 と arm64 の Windows 向けに組み、リンクが通るかを確かめる。
2. Where 走らせる機械が手元に在る, the pilot shall 組んだものを走らせて確かめ、走らせられない的は「組めてリンクが通るまで」を確かめた範囲として一次記録に書く【仮定 5】。
3. If 組めないかリンクが通らないとき, the pilot shall 止まった所（足りない記号・取り込みの指定など）と理由を一次記録に書く。

### Requirement 10: 問い 9 — 規模の数え

**Objective:** As a 移行の本体の起票者, I want 今の本線が使う Windows の API の全数と、写した分の手間からの見積もり, so that 移行の本体の各本の規模を実数に基づいて見立てられる

#### Acceptance Criteria
1. When 問い 9 を確かめるとき, the pilot shall 今の本線の出荷するクレートが使う Windows の API の全数を、関数・型・定数・インターフェースのメソッドに分けて数え、クレートごとの数を一次記録に書く。
2. The pilot shall 数え方を、だれが走らせても同じ数が出る手順（置き場の中の道具かコマンドの並び）として残し、数えた本線のコミットを一次記録に書く。
3. The pilot shall 写した分（問い 1・3・4・6）の 1 行あたり・API 1 つあたりの手間の実数と、全数からの見積もりを一次記録に書く。

### Requirement 11: 問い 10 — 機械的に回せるか

**Objective:** As a 開発者, I want 書き換えの対応表だけを渡した安い実装係が移行をこなせるか、どの取りこぼしが機械で捕まらないかの結果, so that 移行の本体の実装係のモデルと、レビューが見るべき点を決められる

#### Acceptance Criteria
1. When 書き換えの対応表（要件 12）ができたとき, the pilot shall 問い 1 の元をもう 1 つ新しく写し、安い実装係（Sonnet）に対応表だけを渡して移させ、組めて例が走るかと、人の手で直した所を一次記録に書く【仮定 6】。
2. The pilot shall 取りこぼしの 3 種（戻り値の `HRESULT` を見ない・「ポインタ＋長さ」の長さの誤り・失敗を記録しない経路）を 1 つずつわざと入れ、コンパイルと lint で捕まるかを確かめる。
3. The pilot shall 機械で捕まらなかった種類を、移行の本体でレビューが見る点の一覧として一次記録に書く。

### Requirement 12: 書き換えの対応表

**Objective:** As a 移行の本体の実装係, I want 今の書き方から新しい書き方への対応表, so that 方針が決まった後の書き換えを、表だけを見て機械的に進められる

#### Acceptance Criteria
1. The pilot shall 今の書き方 → 新しい書き方の対応表を、置き場の中の 1 つの文書として残す。
2. The pilot shall 少なくとも次の項目を対応表に載せる: 戻り値の無い COM メソッドの `HRESULT`・スライスの引数の「ポインタ＋長さ」・`HWND`（`.sys()`・`.minimal()` の形では生ポインタの別名・既定の形では newtype。どちらも `Send` でない）・列挙の定数（`i32`／`u32`）・`windows-numerics` のフィールド名（小文字）・`#[implement]` の `_Impl` の戻り値・`HSTRING` の `Display` が無いこと・束縛のフィルターへの API の足し方・COM の型の `==` の意味。
3. The pilot shall 対応表の各行を、書き換え前と書き換え後の例の 1 組で示す。
4. The pilot shall COM の型の `==` が COM の同一性（番地が違えば `IUnknown` へ問い合わせて比べる）を保つかを確かめ、`crates/wintf/src/ecs/graphics/composite.rs` の `SurfaceComposite::same_root` と `crates/wintf/src/ecs/graphics/command_list.rs` の `GraphicsCommandList` の導出の `PartialEq` がその比べに頼っていることを対応表に書く。
5. If 新しい束縛で COM の型の `==` が番地だけの比べに変わるとき, the pilot shall 包みの側で `IUnknown` の比べを書く書き方を対応表に載せる。
6. The pilot shall 対応表の行ごとに、置き場の写しで実際に通した行か、文書から写しただけの行かと、どの束縛の形（既定・`.minimal()`・`.sys()`）のときの行かを示す。

### Requirement 13: 一次記録と見立て

**Objective:** As a 開発者, I want 10 の問いの見立てと学び・移行の本体の刻み方の案・段階の移行の可否が 1 か所にまとまった記録, so that それを見て移行の本体へ進むか（go）を決められる

#### Acceptance Criteria
1. The pilot shall 一次記録を `crates/pilot/examples/pilot-windows-rs-next/README.md` に、動機・概要・検証結果の 3 幕で書き、動機の幕で対応する本坑（移行の本体の各本）を名指しする。
2. The pilot shall 10 の問いのそれぞれに、go／違う／直す の見立て・学び・確かめた日を付ける。
3. The pilot shall 引いた上流のコミット番号と確かめた日を書き、0.101 の公開の後に結果が変わりうることを書く。
4. The pilot shall 移行の本体を何本にどう刻むかの案（束縛を作る道具・クレートの群ごとの移行・`windows` 0.62 を外す締め・`windows-text` の採用）を、それぞれの規模の見立て（要件 10 の実数に基づく）つきで書く。
5. The pilot shall 本線で新旧（`windows-core` 0.62 と 0.101）を同じ依存の木に並べる段階の移行ができるかの結果を、開発者が決める材料として書く。
6. The pilot shall 例の走らせ方（どのフォルダで何を打つか・ビルドの出力の行き先）を、絶対パスでどのディレクトリからでも動く形で書く。
7. The pilot shall go の判定を自分で下さず、一次記録には見立てまでを書く（go／違う／直す の判定は開発者が一次記録を見て下す）。

### Requirement 14: 上流へ返す文案

**Objective:** As a 開発者, I want 上流の windows-rs へ返す文案が承認できる形で用意されていること, so that 「実アプリで各段を確かめる」と伝えた約束を、自分の名で外へ出す前に中身を確かめて返せる

#### Acceptance Criteria
1. The pilot shall 上流へ返す文案（`windows-text` の足りない所・実アプリで確かめた結果）を置き場の中の文書として残す。
2. The pilot shall 文案を上流へ投稿しない（投稿は開発者の承認の後で、本仕様の外）。
3. The pilot shall 文案を上流で使う言語（英語）で書き、日本語の要旨を添える【仮定 7】。

### Requirement 15: 実機の確かめの作法

**Objective:** As a 開発者, I want GPU と画面を使う確かめが本番と同じ条件で行われ、ほかのセッションの仕事を乱さないこと, so that 出た結果を本番の窓にも当てはめられ、並走するウェーブの試験も汚れない

#### Acceptance Criteria
1. The pilot shall GPU を使う確かめ（問い 3・4・7）を実の GPU で行い、代わりの描画で済ませない。
2. The pilot shall 実機の根・検体・一時フォルダ・ビルドの出力を作業木の `target` の下にだけ作り、`C:\` の直下・`C:\tmp`・OS の一時フォルダを使わない。
3. While 画面の操作（窓を出す・クリックを送る・画面を撮る）を行う間, the pilot shall 排他テストの机で順を取り、操作を終えたらすぐ返す。
4. The pilot shall 窓を出す例を、上限時間の内側で自分で終わる形にする。

## 未決事項（要件の討議で決める）

1. **【仮定 1】問いの数と spec の本数**: brief は「要件の段で問いを削るか 2 本に分けるかを決めてよい」とする。仮定＝10 の問いを全部残し、1 本（M・10〜14 タスク）のまま掘る。分けるなら、問い 7（`windows-text` の縦書き）を別の先進坑へ出す案が最も切りやすい。
2. **【仮定 2】問い 2 の置き場の 3 案を全部組むか**: 仮定＝3 案とも同じ最小の例で組んで比べる（brief「どれも試さないと決められない」）。参照の口は無いと分かったが、3 案ともその口なしで組める（`research.md` 5.3。口が無いことで、案 1 は型が重なり `cast` でつなぐ形に、案 2 は `wintf` の役の束縛が `areka-emo-text` の役の要る API まで抱える形に決まる）。
3. **【仮定 3】引く上流のコミット**: brief は「master の決まったコミット」とだけ言う。仮定＝着手の時の master の先頭（2026-11-14 に確かめた `520097a3` 以降。2026-11-16 のギャップ分析の時点で先頭は `f37e2f90` へ進んでいた）を 1 つ選び、要件 2 の 5 に当たらない限り途中で動かさない。
4. **【仮定 4】問い 7 の画素の比べの合格の線**: 仮定＝要件に閾値を置かず、違う画素の数・最大の差・差の絵を出し、同じと見るかは開発者が見て決める。
5. **【仮定 5】問い 8 の arm64**: 仮定＝arm64 の機械は手元に無いので、組めてリンクが通るまでを確かめる（i686 は x64 の Windows で走らせて確かめる）。
6. **【仮定 6】問い 10 の進め方**: 仮定＝問い 1 の写しとは別にもう 1 つ新しく写し、対応表だけを渡した Sonnet の実装係に移させる。取りこぼしの 3 種は 1 つずつわざと入れて、コンパイルと lint で捕まるかを見る。
7. **【仮定 7】上流へ返す文案の言語と範囲**: 仮定＝英語で書き日本語の要旨を添える。開き括弧の位置の件は開発者が上流へ別に報告する（brief）ので、文案には確かめた結果として載せるだけにする。
8. **【仮定 8】差の絵などの証拠の置き場**: 仮定＝一時の物は作業木の `target` の下に作り、一次記録に添える小さな絵（PNG）だけは置き場の中に入れてコミットする。
