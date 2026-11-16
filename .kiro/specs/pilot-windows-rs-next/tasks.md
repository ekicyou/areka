# Implementation Plan

> 種別: **先進坑（pilot・使い捨て）**。成果物はコードでなく知見（問いごとの go／違う／直す の見立て＋学び）。一次記録は `crates/pilot/examples/pilot-windows-rs-next/README.md`（3 幕）。go の判定は開発者が下す。
> 置き場は自分の `[workspace]` と `Cargo.lock` を持つ独立のワークスペース（二坑モデルの例外・要件 1.2）。触るのは置き場の下の新しいファイルと spec の文書だけ。根の `Cargo.toml`・`Cargo.lock`・`crates/pilot/Cargo.toml`・`deny.toml`・`tools/`・出荷するクレートは変えない。
> 上流は windows-rs master `f37e2f9046cf9710fe00285436c4d22ad6053d72` に固定（動かすのは要件 2.5 の手順の後だけ）。
> 置き場の決まり（design.md の表）は全タスクで守る: 直下に `main.rs` を置かない・`.rs` は 1,000 行以下・生成した束縛はコミットしない・`std::env::temp_dir` を呼ばない・捕捉先の 4 語を綴らない・`log-capture-kit`／`sample-ghost-kit`／`real-run-kit` と `env-filter` を書かない・検体のパスを組まない・`state.actors()` を書かない・各 manifest に `publish = false`・新しい側で `windows` クレートを名指ししない・D3D11 はハードウェアで作る。
> 一次記録の節（問いごと・CopyLedger・RunCommands・ExitContract）は 1.1 で先に立てる。各タスクは自分の問いの節にだけ結果を書き足す（同じ README を触るが、節が分かれているので差分は重ならない）。
> 窓を出す走行（`q4-demo`・`vt-balloon`＝7.2・8.6）は、進行役（主の会話）が実装係を起こす前に排他テストの机を取り、実装係が終えたらすぐ返す。画面を使わない GPU の走行は机を取らない。
> 写した量の数え（CopyLedger）: 写すタスクは「写した直後（元と同じ中身）」を 1 コミット、「移した後」を 1 コミットにし、直しの回数と作業の時間を自分の問いの節と CopyLedger の自分の行（1.1 で問い 1・3・4・6 の行を先に立てる）にだけ書く。書き換えの行数は `git diff --numstat <写した直後> <移した後> -- <自分のクレートのフォルダ>` で数える。
> コミットの作法: 各タスクは自分のクレートのフォルダと README（と自分が持つ文書・自分のフィルターのファイル）だけを `git add <path>` で積む。`git add -A` は使わない。
> `bind-next` の `Cargo.toml` の兄弟の依存（`windows-future` ほか）は足すだけで消さない（ほかの問いのタスクが足した物を残す）。

- [ ] 1. Foundation: 独立のワークスペース・上流の固定・共有の束縛の土台

- [x] 1.1 独立のワークスペースの骨組みと一次記録の節を建てる
  - 置き場の根に、`[package]` を持たない仮想の manifest（resolver 3・`[workspace.package]` の edition 2024 と `publish = false`・`[workspace.dependencies]`）を置く。上流のクレート（`windows-core`・`windows-result`・`windows-strings`・`windows-link`・`windows-numerics`・`windows-text`・`windows-bindgen`、名指しされたときだけ使う兄弟の `windows-future`・`windows-collections`・`windows-reference`・`windows-time`）は全部同じ git の rev に固定し、借りる本線のクレート（`wintf`・`areka-emo-text`・`areka-parsers`・`areka-sakura`）は置き場の根からの相対の path、crates.io の物（`bevy_ecs` 0.19.1・`png` 0.18.1・`tracing`・`tracing-subscriber`〔`env-filter` なし〕・`scip`）は本線と同じ版で書く。`[patch]` は置かない
  - ビルドの出力の行き先を作業木の `target\pilot-windows-rs-next` へ向ける設定を置き場に置く
  - 本線の `Cargo.lock` を写して置き場の `Cargo.lock` の種にし、写した本線のコミットを一次記録に書く
  - 一次記録（動機・概要・検証結果の 3 幕）の見出しを立てる。検証結果の幕には問い 1〜10 の節（各節に「見立ての候補」「学び」「確かめた日」「止まった所・理由・代わりの見立て」の欄）と、刻み方の案・段階の移行・レビューが見る点・公開の後の注意の節を置き、判定の欄は空けて「判定は開発者が下す」と書く。概要の幕に RunCommands・CopyLedger・ExitContract の節を置き、CopyLedger には問い 1・3・4・6 の行（元のファイル・写した行数・書き換えの行数・直しの回数・時間の欄）を空のまま先に立てる。書き換えの対応表と上流へ返す文案は見出しだけの文書として置き、証拠の絵の置き場（`evidence/`）を作る
  - 観測: 置き場に manifest・設定・`Cargo.lock`・3 つの文書・`evidence/` が在り、`git diff --stat main -- Cargo.toml Cargo.lock crates/pilot/Cargo.toml deny.toml tools` が空で、本線の根で `cargo metadata --no-deps` の members に置き場のクレートが出ない
  - _Dispatch: mechanical — 設計が manifest・設定・文書の節の形を与えている_
  - _Requirements: 1.1, 1.2, 1.3, 1.5, 1.10, 2.1, 2.2, 13.1, 15.2_

- [x] 1.2 全クレートの manifest を建て、上流を取り寄せて依存の解決を確かめる
  - 設計の File Structure Plan の全クレート（`q1-winmsg`・`q2-placement` の 7 つ・`q3-default`・`q3-minimal`・`bind-next`・`q4-wuc`・`q5-hwnd`・`q6-com`・`q7-vertical`・`q9-api-count`・`q10-winmsg`）の manifest を、依存の向きの図どおりの依存だけで置き、各ソースは空の骨組み（モジュールの宣言と空のファイル）にする。`q1-winmsg`・`q10-winmsg` は edition 2021 でライブラリ名を元と同じにする。`q10-winmsg` の `[[example]]`（`q1-winmsg` の例を path で指す 2 つ）は、指す先が 2.1 まで無いのでここでは書かず、11.1 で足す。フォルダ名に本線のクレートと同じ名前を使わない
  - 上流の git を既定の `CARGO_HOME` へ取り寄せ（確定 10）、置き場の `Cargo.lock` の windows-rs の source が固定の rev の 1 つだけになっていることを確かめる。取り寄せた上流に兄弟のクレート（`windows-future`・`windows-collections`・`windows-reference`・`windows-time`）が在ることを確かめる
  - 借りる本線のクレートを含む `q7-vertical` の骨組みを組み、`windows-core` 0.62 と 0.101 が同じ木に並ぶこと（`cargo tree -d` の出力）と、`q1-winmsg` と本物の `wintf-winmsg-executor` が同じライブラリ名で同じ木に入っても組めることを確かめる。cargo が断ったら設計の Risks のとおり `q1-winmsg` のライブラリ名だけを変え、その旨を記録する。`image-webp` が crates.io の版に解き直された版を記録する
  - 解決が止まったときは回り道をせず、止まった所を一次記録に書き、rev を動かすなら要件 2.5 の手順（理由と先の rev を先に書く）に従う
  - 観測: 置き場の中で `cargo check --workspace` が骨組みのまま通り、`Cargo.lock` の windows-rs の source が 1 つで、一次記録の概要に取り寄せた兄弟のクレート・`cargo tree -d` の出力・`image-webp` の版が書かれている
  - _Dispatch: standard — 上流の git と借りるクレートの解決の詰まりを実地で解く_
  - _Requirements: 1.2, 1.4, 2.1, 2.2, 2.5, 13.5, 15.2_

- [x] 1.3 共有の既定の形の束縛 `bind-next` の作り方を通す
  - `bind-next` の build で、問いごとのフィルターの文書（`q4-wuc`・`q6-renderer`・`q7-text`・`common`）を全部読み、既定の形の束縛を `OUT_DIR` へ書く。下線の描き手の 3 つ（`IDWriteTextRenderer1`・`IDWriteTextRenderer`・`IDWritePixelSnapping`）を実装できる形にする `.implements` の指定は、ここでは書かず 6.2 で足す（1.3 は最小に保ち、名指しの無い型への指定で build が止まるのを避ける）
  - 共通のフィルターには 2 つ以上の問いが使う物（COM の初期化・D3D11 と D2D の装置の作り方・DXGI ほか）を入れ、問いごとのフィルターは空で始める（中身は各問いのタスクが足す）
  - 生成した束縛が名指しした兄弟のクレートだけを依存に残す
  - 観測: `cargo build -p bind-next` が通り、生成した束縛が作業木の `target` の下の `OUT_DIR` にだけ在って置き場にコミットされず、フィルターの行数と生成した行数が一次記録の概要に書かれている
  - _Dispatch: mechanical — 設計の BindingBuild の形をそのまま書く_
  - _Requirements: 1.6, 2.3, 2.4_

- [ ] 2. 問い 1: C の関数だけのクレートを `.sys()` へ移す

- [x] 2.1 `wintf-winmsg-executor` を置き場へ写す
  - 本番の `src/`（`lib.rs` の `#[cfg(test)] mod test` を除く）と `util/` の 3 ファイル、例の `basic`・`threads` を、中身を変えずに `q1-winmsg` へ写す（例は以後も書き換えない）
  - 写した直後の状態を 1 コミットにし、元のファイルごとの本番の行数を CopyLedger に書く
  - 観測: 写したファイルが元と同じ中身（試験のモジュールだけ除く）で、本線の `crates/wintf-winmsg-executor/` の差分が 0 で、写した行数が CopyLedger に在る
  - _Dispatch: mechanical — 写す範囲を設計が名指ししている_
  - _Requirements: 1.8, 3.1, 3.4_

- [x] 2.2 写しを `.sys()` の束縛へ移し、x64 で元と同じに走らせる
  - build で `.sys().flat()` の束縛を `OUT_DIR` へ書き、名指しする API を 1 行 1 つのフィルターに並べる
  - 写しの中の `windows` の取り込みを生成した束縛へ置き換え、戻り値や引数の形の違いを直す（例のファイルは変えない）
  - 例 2 本を x64 で走らせ、本線で走らせた元の例と比べる: `basic` は全文と終了コードの一致、`threads` は `main` の行の並びと終了コードの一致を見て、`thread` の行は出た行の集まりとして並べる
  - 移した後を 1 コミットにし、フィルターの行数・写した直後と移した後の `git diff --numstat`・直しの回数と作業の時間を、問い 1 の節と CopyLedger に書く
  - 観測: `q1-winmsg` の例 2 本が x64 で元と同じ出力・終了コードで終わり、問い 1 の節に比べの結果と 3 つの数（フィルターの行数・書き換えの行数・手間）が在る
  - _Dispatch: standard — 設計は方向を与えるが、束縛の差による書き換えは実地で詰める_
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [x] 2.3 写しの例を i686 で走らせ、arm64 で組んでリンクまで通す
  - PowerShell から i686 向けに例 2 本を組んで x64 の Windows で走らせ、2.2 と同じ比べ方で元と比べる。`SetWindowLongPtrW` の 32bit の別名を束縛が出したか、写しの側で分けずに済んだかを書く
  - aarch64 向けに例 2 本を組み、リンクが通るまでを確かめる（走らせる機械は無い＝確定 5）
  - 組めないか走らないときは、止まった所（足りない記号・取り込みの指定ほか）と理由を書き、問い 1 の見立てを「直す」か「違う」の候補にする
  - 観測: 問い 1 と問い 8 の節に、i686 の走行の比べの結果と aarch64 のリンクの結果（通ったか・止まった所）が在る
  - _Dispatch: standard — クロスのビルドの詰まりを実地で解く_
  - _Requirements: 3.2, 3.5, 9.1, 9.2, 9.3_

- [ ] 3. 問い 3: 束縛の形の 2 案（既定・`.minimal()`）を同じ写しで比べる

- [x] 3.1 (P) `dwrite.rs`＋`d2d/mod.rs` の写しを既定の形で組み、確かめの bin と `==` の確かめを通す
  - `wintf` の `com/dwrite.rs` と `com/d2d/mod.rs` を `q3-default` へ写し、写した直後を 1 コミットにする（写した量は元の行で 1 回＝312 行と数える）
  - 既定の形の束縛へ移し、確かめの bin で DirectWrite の工場・書式・配置を作り、ハードウェアの D3D11＋D2D の画面の外の絵へ描いて読み戻し、墨の画素を数える
  - 同じ物を `IUnknown` を通って戻した別の参照で持ったときの `==` が真、別の物で偽になることを `assert` する（作れるなら `ID2D1CommandList` で見る）
  - 移した後を 1 コミットにし、束縛の行数・呼ぶ側の書き換えの行数・`==` の結果・直しの回数と時間を問い 3 の節に書く
  - 観測: 確かめの bin が実の GPU で墨の数を出して `==` の `assert` が通り、問い 3 の節に既定の形の数が在る
  - _Boundary: q3-default_
  - _Dispatch: standard — 写しの書き換えと確かめの bin を新しく組む_
  - _Requirements: 1.8, 4.4, 12.4, 15.1_

- [x] 3.2 (P) 同じ写しを `.minimal()`＋手書きの包みで組み、同じ確かめの bin と `==` の確かめを通す
  - 3.1 と同じ元を `q3-minimal` へ写し、`.minimal()` の束縛へ移す。`.minimal()` が出さない口（継いだメソッドの転送・取っ手の便利な口・文字列の包み）を手書きの包みで補う
  - 3.1 と同じ中身の確かめの bin を走らせ、同じ墨の数になることと、`==` の `assert`（`.minimal()` で `==` が番地だけの比べに変わるかを含む）を見る
  - 束縛の行数・包みの行数・呼ぶ側の書き換えの行数・`==` の結果・直しの回数と時間を問い 3 の節に書く
  - 観測: `q3-minimal` の bin が実の GPU で 3.1 と同じ墨の数を出し、問い 3 の節に `.minimal()` の数と `==` の振る舞いが在る
  - _Boundary: q3-minimal_
  - _Dispatch: standard — 手書きの包みの範囲を実地で決める_
  - _Requirements: 1.8, 4.4, 12.4, 15.1_

- [x] 3.3 問い 3 の 2 形を比べ、i686・arm64 で組む
  - 2 形の確かめの bin を PowerShell から i686 向けに組んで x64 の Windows で走らせ、aarch64 向けに組んでリンクまで通す
  - 2 形の束縛の行数・包みの行数・呼ぶ側の書き換えの量を表にして、問い 3 の見立ての候補と学びを書く
  - 問い 8 の節に、問い 1（2.3）と合わせた 3 つのターゲットの結果と、走らせられなかった的は「組めてリンクが通るまで」を確かめた範囲として書く
  - 観測: 問い 3 の節に 2 形の比べの表と見立ての候補が、問い 8 の節に 2 つのクレートの i686 の走行と aarch64 のリンクの結果が在る
  - _Depends: 2.3_
  - _Dispatch: standard — クロスのビルドの詰まりと比べの読みが要る_
  - _Requirements: 4.4, 9.1, 9.2, 9.3_

- [ ] 4. 問い 2: COM の束縛の置き場の 3 案を同じ最小の例で比べる

- [x] 4.1 (P) 共通の描く本体と案 3（共有の束縛のクレート）を組む
  - 3 案が読む同じ描く本体（DirectWrite の文字を 1 行 Direct2D で描き、墨を数える）を置く。各案は型の出どころだけを替えて読み込む
  - 案 3: 共有の束縛のクレート（両役のフィルターの和・既定の形）・wintf 役（ハードウェアの D3D11 → D2D の装置と文脈 → 描く先と CPU で読む絵 → 読み戻した BGRA）・emo-text 役（受け取った `ID2D1DeviceContext` で描く bin）を組む
  - 観測: 案 3 の bin が実の GPU で墨の数を出し、問い 2 の節に案 3 の「組めたか・受け渡しの手当て・重なった束縛の量」が在る
  - _Boundary: q2-placement_
  - _Dispatch: standard — 2 役の口を設計の形で新しく組む_
  - _Requirements: 4.1, 4.2, 15.1_

- [x] 4.2 案 1（クレートごとに束縛）と案 2（wintf 役が束縛を公開）を組む
  - 案 1: 両役がそれぞれ自分の束縛を持ち、emo-text 役の入口で wintf 役の型から自分の型へ `cast` する
  - 案 2: wintf 役の束縛が emo-text 役の要る API（`IDWriteFactory`・`IDWriteTextFormat`・`IDWriteTextLayout` ほか）まで抱えて公開し、emo-text 役はそれを使う
  - 4.1 の描く本体をそのまま読み込み、組めない案があれば組めなかった所（コンパイルの誤りの要旨か上流の制約）を書く
  - 観測: 案 1・案 2 の bin が 4.1 と同じ墨の数を出すか、組めなかった所が問い 2 の節に在り、案ごとの `cast` の行・`from_raw` の有無・同じ型が 2 つの束縛に出た数・生成した行数が書かれている
  - _Boundary: q2-placement_
  - _Dispatch: standard — 案ごとの型の受け渡しを実地で詰める_
  - _Requirements: 4.1, 4.2, 4.6, 15.1_

- [x] 4.3 3 案を比べ、参照の口の有無を確かめ、推す組み合わせを書く
  - 3 案の比べの表（組めたか・受け渡しの手当て・重なった束縛の量）を問い 2 の節にまとめる
  - 固定の rev の上流で `windows-bindgen` にほかのクレートの型を参照させる口が在るかを確かめ、在れば使い方を、無ければ根拠（上流の文書かソースの箇所）を書く
  - 問い 3 の結果と合わせて、置き場の案と形の案の組み合わせのうち推す 1 つと理由を書く。組めなかった案は推さない。共有の束縛 `bind-next` は案 3 と同じ形だが比べの材料ではないと書く
  - 観測: 問い 2 の節に比べの表・参照の口の根拠・推す組み合わせと理由が在る
  - _Depends: 3.3_
  - _Dispatch: standard — 結果の読みと上流のソースの確かめが要る_
  - _Requirements: 4.2, 4.3, 4.5, 4.6_

- [x] 5. (P) 問い 5: 既定と `.minimal()` の `HWND` を包んで bevy の Component に持たせる
  - build で既定の形と `.minimal()` の 2 つの束縛（`HWND`・`GetDesktopWindow`・`IsWindow`）を別の出力に書き、それぞれの `HWND` を包む型を bevy の Component にする
  - デスクトップの窓の取っ手を両方の包みで世界に入れ、別のシステムが読み出して `IsWindow` が真になることを確かめる
  - 2 つの包みが `Send + Sync` であることを型の検査の試験で固定し、名乗ってよい根拠（値を読むだけなら安全・窓に触る呼び出しは作ったスレッドへ送る決まりを包みの外で守る）を問い 5 の節に書く
  - `Hash`・`Default`・無効の判定に当たる口が形ごとに在るかの表を書く（`.sys()` の行は問い 1 の写しの実物から取る）
  - 観測: `cargo test -p q5-hwnd` と確かめの bin が通り、問い 5 の節に Send／Sync の根拠と形ごとの口の表が在る
  - _Boundary: q5-hwnd_
  - _Depends: 2.2_
  - _Dispatch: mechanical — 設計が包みの型と確かめの形を与えている_
  - _Requirements: 6.1, 6.2_

- [ ] 6. 問い 6: `#[interface]`・`#[implement]` で COM を実装する

- [x] 6.1 (P) SHIORI のインターフェースの 1 組を写して組む
  - `shiori-abi` の `interface.rs` から `IShiori`（`Get` は `HRESULT` を直に返す・`Notify` は `Result<()>`）の定義・試験の中の実装 1 つ・呼び出しを `q6-com` へ写し、写した直後を 1 コミットにする
  - `windows-core` 0.101 の `#[interface]`・`#[implement]` で、今の宣言を変えずに組めるかを確かめ、`Get` が実装まで届いて `S_OK` と応答の `HSTRING` が返ることを見る
  - 組めないときは組めなかった所と代わりの組み方の見立てを問い 6 の節に書く
  - 観測: `q6-com` の bin が `IShiori::Get` の `S_OK` と応答を出し、問い 6 の節に写した行数・書き換えの行数・直しの回数と時間が在る
  - _Boundary: q6-com_
  - _Dispatch: standard — マクロの振る舞いを実物で確かめながら組む_
  - _Requirements: 1.8, 7.1, 7.4_

- [x] 6.2 下線の位置を集める `IDWriteTextRenderer1` を新しく書いて実装する
  - `bind-next` の問い 6 のフィルターに要る DirectWrite の物（下線の描き手の 3 つを含む）を足し、`bind-next` の build に 3 つの `.implements` の指定を足す（1.3 で後回しにした分）。`Cargo.toml` の兄弟の依存は足すだけで消さない
  - 元の `viewbox_draw_underline.rs` を写さずに、`IDWriteTextRenderer1`・`IDWriteTextRenderer`・`IDWritePixelSnapping` の `_Impl` を小さく新しく書き、`DrawUnderline` で下線の位置（x・y・幅・太さ・向き）を集める
  - 下線を付けた範囲を持つ文字の配置の描画にこの描き手を渡し、位置が 1 つ以上集まることを見る。`_Impl` のメソッドが `HRESULT` を返す形の書き方を対応表の材料として問い 6 の節に控える
  - 観測: `q6-com` の bin が集めた下線の位置を出し、問い 6 の節に組めたか・集めた位置・対応表の材料が在る
  - _Boundary: q6-com, bind-next（filters/q6-renderer.txt・build の `.implements`・Cargo.toml の兄弟の依存）_
  - _Dispatch: standard — 描き手の実装を新しく書く_
  - _Requirements: 7.2, 7.4, 15.1_

- [ ] 7. 問い 4: WinRT の合成を `windows-bindgen` の束縛で通す

- [x] 7.1 `wuc.rs` を写して `bind-next` の WinRT の束縛へ移し、画面の撮りを書く
  - `wintf` の `com/wuc.rs` を `q4-wuc` へ写し、写した直後を 1 コミットにする
  - `bind-next` の問い 4 のフィルターに Composition・DispatcherQueue・相互運用のインターフェース・画面の撮りの GDI を足し、写しを既定の形の束縛へ移す（束縛の形は既定だけで確かめる）。`bind-next` の `Cargo.toml` の兄弟の依存は足すだけで消さない
  - `bind-next` を組み直すので 6.2 と並べない
  - 窓の矩形を画面の DC から撮って PNG に書く口を新しく書く
  - 生成した束縛が実際に名指しした兄弟のクレートと、問い 4 のフィルターの行数を問い 4 の節に書く
  - 観測: `q4-wuc` のライブラリが組め、問い 4 の節に要った兄弟のクレート・フィルターの行数・書き換えの行数・直しの回数と時間が在る
  - _Boundary: q4-wuc, bind-next（filters/q4-wuc.txt・Cargo.toml の兄弟の依存）_
  - _Dispatch: standard — WinRT の束縛への書き換えを実地で詰める_
  - _Requirements: 1.8, 2.4, 5.1_

- [x] 7.2 `q4-demo` で合成の各段を実の窓で通す
  - 問い 1 の写しの窓（`WS_POPUP`・`WS_EX_NOREDIRECTIONBITMAP`）→ DispatcherQueue の作成 → Compositor の生成 → DesktopWindowTarget → D2D の装置からの描画の装置 → 描画面 → SpriteVisual へのつなぎ → 塗る → 撮る、の順に組み、段ごとに `[q4] step=<名前> ok|err=<HRESULT>` を 1 行出す
  - 上限時間（`AREKA_APP_SMOKE_EXIT_MS`・既定 30000 ms）の内で自分で終わり、ExitContract の終了コードで終わる。失敗は `tracing::error!` で段の名前と `HRESULT` を出す
  - 進行役（主の会話）が実装係を起こす前に排他テストの机を取り、実装係が終えたらすぐ返す。机を持つ間に実の GPU で走らせ、主の会話も画面を目で確かめる。撮った絵は `out\q4\` に書き、小さく切って `evidence/` に置く。撮った絵に WUC の中身が写らなければその旨を書く
  - 今の形のまま通らない段があれば、その段と代わりの書き方か通らない理由を問い 4 の節に書く
  - 観測: 段ごとの ok の行が出て終了コード 0 で終わり、撮った絵に塗った色が在り、問い 4 の節に段ごとの結果と見立ての候補が在る
  - _Depends: 2.2_
  - _Dispatch: standard — 合成の段のつなぎを実地で通す_
  - _Requirements: 5.2, 5.3, 15.1, 15.3, 15.4_

- [ ] 8. 問い 7: `windows-text` の縦書きを今の areka と比べ、バルーンで当たりを確かめる

- [x] 8.1 画素の比べと位置の換算を組み、小さな試験で固定する
  - 2 つの絵（premultiplied BGRA）の違う画素の数・成分の差の最大・±8 px の平行移動で違う画素が最少になる所とその数を出す比べと、違う画素を赤・同じ墨を灰にした差の絵を書く口を組む。大きさが違えば小さい方に合わせて数え、その旨を出す
  - 選択肢の範囲の UTF-8 のバイト位置 ↔ areka の文字の位置 ↔ UTF-16 の位置の換算と、当たりの結果から選択肢の番号への写しを組む
  - 観測: `cargo test -p q7-vertical` で、同じ絵で 0・1 画素違いで 1・2 px ずらした絵で最良のずれ (2, 0)、サロゲートの絵文字を含む換算の試験が通る
  - _Boundary: q7-vertical の比べと換算_
  - _Depends: 7.2_（7.2 が q4-wuc の lib を直し終えてから q7-vertical を組む）
  - _Dispatch: mechanical — 設計が契約と試験の期待値を与えている_
  - _Requirements: 8.1, 8.6_

- [x] 8.2 今の側（借りた `areka-emo-text`）で縦書きの 1 列を描き、描く条件を表にする
  - 比べの文 `「ちょっと、待って。🍣」`・ぱすたの書体の候補列・20 px・今の areka の縦書きの試験と同じ絵の大きさと原点で、文字列で組んだバルーンの定義（縦書き）から借りた公開の API で描き、BGRA を読み戻す
  - 今の側が描く経路で使う条件（文字のアンチエイリアスの形・描画の設定・DPI・単位・面の画素の形式・筆の色・`DrawTextLayout` の指定・COM のアパートメント）を本線のコードから読み、値と所（ファイルと行）を問い 7 の節に表で書く
  - 観測: 今の側の BGRA が取れ、問い 7 の節に描く条件の表（値・ファイルと行）が在る
  - _Boundary: q7-vertical の今の側_
  - _Depends: 8.1_（同じクレート q7-vertical を組むので 8.1 と並べない）
  - _Dispatch: standard — 借りた API で描く手順と条件の読み取りを実地で詰める_
  - _Requirements: 8.1, 8.2, 13.5_

- [ ] 8.3 新しい側（`windows-text`）で同じ 1 列を同じ条件で描く
  - `bind-next` の問い 7 のフィルターに要る物を足す。`Cargo.toml` の兄弟の依存は足すだけで消さない
  - 縦書き（右から左）の文字の配置を `windows-text` で作り、DirectWrite の型として借りて、ハードウェアの D3D11＋D2D の画面の外の絵へカラーフォントの指定つきで描いて読み戻す。COM は今の側と同じ MTA で始め、8.2 の表の条件に揃える。揃えられなかった条件は表に印を付ける
  - 字形の指定は「無し」と「areka の 10 個（`ccmp`・`locl`・`mark`・`mkmk`・`rlig`・`calt`・`clig`・`liga`・`kern`・`rclt`）」の 2 通りで描く
  - 新旧の COM の型はつながない。どうしても橋が要った所は生ポインタで渡し、その所を書く
  - 観測: 新しい側の BGRA が 2 通り取れ、問い 7 の節に揃えた条件・揃えられなかった条件・橋の有無が在る
  - 8.2 の条件の表に揃えるので 8.2 の後（並べない）。`bind-next` を組み直すので 7.2 の後
  - _Boundary: q7-vertical の新しい側, bind-next（filters/q7-text.txt・Cargo.toml の兄弟の依存）_
  - _Depends: 8.2, 7.2_
  - _Dispatch: standard, effort high — 描く条件を今の側と細かく揃える必要がある_
  - _Requirements: 8.2, 8.3, 15.1_

- [ ] 8.4 範囲の色・下線の位置・`TextLayout::new` の panic を確かめる
  - 借りた DirectWrite の型の `SetDrawingEffect`（UTF-16 の範囲・単色の筆）で範囲に赤を付けて描き、赤の画素の数を数える
  - `decorations()` で下線の位置を取り、列の左に出るかと、areka の決まり（選択肢の下線は列の右）に合わせる x の決め方の手当てを書く
  - `TextLayout::new` が panic する入力を 1 つ探して確かめ（例: 大きさに NaN）、本体での扱いの見立て（入る前に値を確かめる・境で止める）と、UTF-8 のバイト位置と areka の文字の位置の換算の扱いを書く
  - 観測: 問い 7 の節に、赤の画素の数・下線の位置と手当て・panic を起こす入力と扱いの見立て・位置の換算の扱いが在る
  - _Dispatch: standard — 上流の未公開の API の振る舞いを実物で確かめる_
  - _Requirements: 8.4, 8.5, 8.6_

- [ ] 8.5 `vt-compare` で旧と新を比べ、証拠の絵と段階の移行の材料を出す
  - 同じ過程の中で MTA を始め、今の側の 1 枚と新しい側の 2 枚を別々の装置で描き、それぞれを「同じ原点のまま」と「最も合う平行移動」の 2 通りで比べる
  - 旧・新 2 通り・差の絵の PNG と数の表を `out\q7\` に書き、小さな絵だけを `evidence/` に写す。合格の閾値は置かず、数と絵を問い 7 の節に載せて「同じと見るかは開発者が決める」と書く
  - 開き括弧の位置が areka の 10 個の字形の指定で今の側と揃うかを、平行移動の差から読んで書く
  - `windows-core` 0.62 と 0.101 が同じ過程で並んで動いた事実と `cargo tree -d` の出力を、段階の移行の可否の材料として書く
  - 観測: `vt-compare` が 3 枚の絵と差の数を出して終わり、問い 7 の節に違う画素の数・最大の差・最良のずれ・差の絵・段階の移行の材料が在る
  - _Depends: 8.1, 8.3, 8.4_
  - _Dispatch: standard — 3 枚の描画と比べを 1 つの bin にまとめる_
  - _Requirements: 8.1, 8.2, 8.3, 8.5, 13.5, 15.2_

- [ ] 8.6 `vt-balloon` で透過の窓に縦書きのバルーンを出し、クリックの当たりを確かめる
  - 問い 1 の写しの窓（`WS_POPUP`・`WS_EX_NOREDIRECTIONBITMAP`・`WS_EX_TOPMOST`・Per Monitor v2）と問い 4 の合成の上で、自前の Direct2D で縦書きの文（カラー絵文字を含む）と選択肢 2 つを描画面へ描く
  - 窓を撮った後、狙いの選択肢の中心へ `SendInput` で動かしてクリックし、`WM_LBUTTONDOWN` の位置を当たりの判定に渡して `[q7] hit choice=<番号> target=<番号>` を出す。カーソルは元の位置へ戻す
  - 上限時間の内で自分で終わり、ExitContract（0＝一致・2＝初期化の失敗・3＝外から閉じられた・4＝食い違い）で終わる
  - 進行役（主の会話）が実装係を起こす前に排他テストの机を取り、実装係が終えたらすぐ返す。走らせるのは机を持つ間だけ。撮った絵を `out\q7\` に書き、小さな絵を `evidence/` に置く
  - 観測: `vt-balloon` が `hit choice` と `target` の一致を出して終了コード 0 で終わり、問い 7 の節に撮った絵と当たりの結果が在る
  - _Depends: 7.2, 8.1_
  - _Dispatch: standard — 窓・合成・入力を実地でつなぐ_
  - _Requirements: 8.7, 8.8, 15.1, 15.3, 15.4_

- [ ] 9. 問い 9: 本線が使う Windows の API の全数を数える

- [ ] 9.1 (P) SCIP の索引を読んで種類ごと・クレートごとに数える道具を組む
  - 索引の各文書の参照のうち、記号の包みが `windows`・`windows_core`・`windows_numerics`・`windows_strings`・`windows_result` の物を集め、記号の綴りの末尾で型・インターフェースのメソッド・関数・定数ほか（型の下の欄は別）・マクロ（別）に分け、同じ記号を 1 つと数えて出現の回数も添える
  - `crates/pilot` を数えず、クレートごとに「出荷する」か「試験の道具」かの印を付け、本番と試験のコードを分けて、合計を両方で出す表（Markdown）を標準出力へ出す
  - 観測: `cargo test -p q9-api-count` で綴りの見本 5 つ（型・メソッド・関数・定数・マクロ）の種類の判定の試験が通る
  - _Boundary: q9-api-count_
  - _Dispatch: standard — 索引の読みと数えの規則を新しく組む_
  - _Requirements: 10.1, 10.2_

- [ ] 9.2 本線の索引を作って数え、手順と結果を一次記録に書く
  - 本線の作業木で rust-analyzer の SCIP の索引を作業木の `target\pilot-windows-rs-next\out\q9\` へ作り（ほかの重いビルドと並べない）、9.1 の道具で数える
  - 索引づくりが落ちるか重すぎるときは、`windows` を外したつもりで組んで出た「見つからない」を集める形に倒し、倒した理由と数え方の違いを書く
  - 問い 9 の節に、クレートごと・種類ごとの表・数えた本線のコミット・rust-analyzer の版・だれが走らせても同じ数が出る手順（コマンドの並び）を書く
  - 観測: 問い 9 の節に表・本線のコミット・道具の版・再現の手順が在る
  - _Dispatch: standard — 重い索引づくりと代わりの数え方の判断が要る_
  - _Requirements: 10.1, 10.2_

- [ ] 10. 書き換えの対応表を書く
  - 1 行を「項目・書き換え前（0.62）・書き換え後（0.101）・形（既定／`.minimal()`／`.sys()`）・通した所（置き場のファイル）か『文書から』・注」の形にし、前後の例をどちらも 1〜3 行のコードで示す
  - 少なくとも次の行を載せる: 戻り値の無い COM メソッドの `HRESULT`・スライスの引数の「ポインタ＋長さ」・`HWND`（形ごとに 3 行）と包みとの行き来・列挙の定数（`i32`／`u32`）・`windows-numerics` のフィールド名（小文字）・`#[implement]` の `_Impl` の戻り値（束縛の出力とマクロの 2 行）・`HSTRING` の `Display` が無いこと・束縛のフィルターへの API の足し方・COM の型の `==`
  - `==` の行に、COM の同一性を保つか（問い 3 の結果）と、`SurfaceComposite::same_root` と `GraphicsCommandList` の導出の `PartialEq` がその比べに頼っていることを注に書く。番地だけの比べに変わる形の行には、包みの側で `IUnknown` へ `cast` して比べる書き方を載せる
  - 問い 10 の実装係にはこの文書だけを渡すので、各行を書き換えの手順として読める文にする
  - 観測: 対応表に上の全項目の行が前後の例・形・通した所つきで在り、問い 1・3・5・6 の写しで通した行に置き場のファイルが名指しされている
  - _Depends: 2.2, 3.3, 5, 6.2_
  - _Dispatch: standard — 問いの結果から行を起こす_
  - _Requirements: 6.3, 7.3, 12.1, 12.2, 12.3, 12.4, 12.5, 12.6_

- [ ] 11. 問い 10: 対応表だけで機械的に回せるかを確かめる

- [ ] 11.1 砂場で Sonnet の実装係に問い 1 の元をもう一度移させる
  - このタスクは進行役が手動の形（`/kiro-impl pilot-windows-rs-next 11.1`）で回す。実装係がするのは砂場の用意と結果の写しだけで、Sonnet の下請けを起こすのと道具の呼び出しの記録を読むのは進行役
  - 実装係: 作業木の `target\pilot-windows-rs-next\q10-sandbox\` に、問い 1 の元の本番の `src/` だけ・対応表・例 2 本の写しを置く（移し終えた問い 1 の写しは置かない）
  - 進行役: `model: "sonnet"` の下請けを起こし、対応表と砂場の場所と「組めて例が走るまで」の指示だけを渡して移させる。下請けの道具の呼び出しの記録を読み、`q1-winmsg/` の下のファイルを読んでいないことを確かめる。読んでいたらその回を捨ててやり直す
  - 実装係: 移し終えた結果を `q10-winmsg/` へ写し、`q10-winmsg` の manifest に `[[example]]` の `q10-basic`・`q10-threads`（`q1-winmsg` の例を path で指す・1.2 で後回しにした分）を足し、例 `q10-basic`・`q10-threads` を問い 1 と同じ比べ方で走らせ、組めたか・例が走ったか・人の手で直した所を問い 10 の節に書く
  - 観測: 問い 10 の節に、下請けの回の結果・`q1-winmsg/` を読まなかった確かめ・人の手で直した所の一覧が在り、`q10-winmsg` の例が組めて走るか、止まった所が書かれている
  - _Depends: 10_
  - _Dispatch: standard — 下請けの走りの段取りと結果の確かめが要る_
  - _Requirements: 11.1, 15.2_

- [ ] 11.2 取りこぼしの 3 種をわざと入れて、コンパイルと lint で捕まるかを見る
  - 1 つずつ入れては戻す: ⑴ 戻り値を見ない＝`q10-winmsg` の `.sys()` の `BOOL` を返す呼び出しと `q3-default` の `HRESULT` を返す COM のメソッドの両方で、文として捨てる形と `let _ =` で捨てる形 ⑵「ポインタ＋長さ」の長さの誤り＝`q3-default` の文字の配置の作成の長さを 1 つずらす ⑶ 失敗を記録しない経路＝`q10-winmsg` の窓の作成の失敗を黙って捨てる
  - それぞれを `cargo build` と `cargo clippy -- -D warnings -W clippy::let_underscore_must_use` で見て、どれで捕まったかの表を書く
  - 機械で捕まらなかった種類を、移行の本体でレビューが見る点の一覧として書く
  - 観測: 問い 10 の節に 3 種 × 入れた先 × 道具の捕まったかの表とレビューが見る点の一覧が在り、入れた取りこぼしが全部戻されている（入れた先の `q10-winmsg/`・`q3-default/` の `git diff` が空）
  - _Depends: 3.1_
  - _Dispatch: mechanical — 入れる先と道具を設計が名指ししている_
  - _Requirements: 11.2, 11.3_

- [ ] 12. 一次記録・上流へ返す文案・置き場の決まりの締め

- [ ] 12.1 写した量と手間を締め、移行の本体の刻み方と規模の見立てを書く
  - CopyLedger に問い 1・3・4・6 の写した本番の行（試験の行と問い 10 の写しを除く）を元のファイルごとに 1 回数えて合計し、1,500 行前後に収まったかを書く
  - 問いごとの書き換えの行数（各問いが自分の行に書いた `git diff --numstat <写した直後> <移した後> -- <クレートのフォルダ>` の数）・フィルターの行数・手間から、1 行あたり・API 1 つあたりの手間を出し、問い 9 の全数に掛けて見積もる
  - 移行の本体の刻み方の案（束縛を作る道具・クレートの群ごとの移行・`windows` 0.62 を外す締め・`windows-text` の採用）を、それぞれの規模の見立てつきで書く
  - 段階の移行（本線で `windows-core` 0.62 と 0.101 を同じ依存の木に並べる）ができるかの材料を、1.2 と 8.5 の結果からまとめる
  - 観測: 一次記録に CopyLedger の合計・1 行／1 API あたりの手間・刻み方の案と規模・段階の移行の材料の節が埋まっている
  - _Depends: 9.2, 11.1_
  - _Dispatch: standard — 実数からの見積もりの組み立てが要る_
  - _Requirements: 1.8, 3.3, 10.3, 13.4, 13.5_

- [ ] 12.2 一次記録を 3 幕で仕上げる
  - 動機の幕: 対応する本坑（移行の本体の各本・brief の Downstream）を名指しし、何が怪しくて掘ったかと、置き場の形が二坑モデルの例外である理由（1 つのパッケージが `windows_core` の名前で持てる版は 1 つ）を書く。コードは本坑へ写さず、本坑は一次記録を見て掘り直すと書く
  - 概要の幕: クレートの一覧と役・引いた上流のコミットと確かめた日・`Cargo.lock` の種にした本線のコミット・借りたクレートを読んだ作業木のコミット・走らせ方（`--manifest-path` と `--target-dir` を絶対パスで書き、別の作業木では根を差し替えると先頭に書く・クロスのビルドは PowerShell・窓を出す例は排他テストの机を持ってから）
  - 検証結果の幕: 問い 1〜10 の各節に見立ての候補（go／違う／直す）・学び・確かめた日をそろえ、「止まった所」の欄が空なら「無し」と書く。rev を動かしたならその理由と先の rev。レビューが見る点の一覧（問い 10）と、0.101 の公開の後に結果が変わりうることを書く
  - 判定の欄は空けたまま「判定は開発者が下す」と書く
  - 観測: 一次記録が 3 幕そろい、問い 1〜10 の全節に見立ての候補・学び・日・止まった所の欄が在り、走らせ方のコマンドが絶対パスで書かれ、判定の欄が空いている
  - _Depends: 12.1_
  - _Dispatch: standard — 各問いの結果を 1 つの記録にまとめる_
  - _Requirements: 1.11, 2.5, 3.5, 4.3, 4.5, 4.6, 5.3, 6.2, 7.4, 8.6, 8.9, 9.3, 11.3, 13.1, 13.2, 13.3, 13.6, 13.7_

- [ ] 12.3 (P) 上流へ返す文案を書く
  - 英語の本文に、実アプリで確かめた段ごとの結果・`windows-text` に無い物（範囲ごとの色・文中のインラインオブジェクト・字送り・縦中横・文字単位の折り返しのうち当たった物）と areka が自前で補うか DirectWrite を直に呼ぶかの見立て・`TextLayout::new` の panic・UTF-8 の位置の扱いを書き、日本語の要旨を添える
  - 開き括弧の位置の件は確かめた結果として載せるだけにする（確定 7）。どこにも投稿しない
  - 観測: 文案の文書に英語の本文と日本語の要旨が在り、問い 7 の節の数と食い違いが無い
  - _Boundary: UpstreamDraft_
  - _Depends: 8.5, 8.6_
  - _Dispatch: standard — 問い 7 の結果から英語の文を起こす_
  - _Requirements: 8.9, 14.1, 14.2, 14.3_

- [ ] 12.4 置き場の決まりと本線を汚していないことを確かめる
  - 見張りが読む物だけ（置き場の `*.rs`・`Cargo.toml`・`.cargo/config.toml`）を grep で見る（README などの文書は決まりの語を説明に含むので外す）: `std::env::temp_dir`・捕捉先の 4 語・`env-filter`・`log-capture-kit`／`sample-ghost-kit`／`real-run-kit`・`state.actors()`・検体のパスの組み立て・新しい側の `windows` クレートの名指し・WARP の装置・直下の `main.rs`・各 manifest の `publish = false`・`.rs` の 1,000 行・コミットされた生成の束縛・`evidence/` の絵の大きさ
  - `git diff --stat main -- Cargo.toml Cargo.lock crates/pilot/Cargo.toml deny.toml tools` と出荷するクレートの差分が空で、本線の `cargo metadata --no-deps` の members に置き場のクレートが出ないことを確かめる
  - 本線の見張りの試験（`crates/log-capture-kit/tests/` の 4 本・`frame_attach_tests`・1,000 行の見張り）だけを走らせて緑を見る（全体テストは完了の段で回す）
  - 観測: grep の結果がどれも 0 件（`publish = false` は全 manifest に在る）で、本線の差分が空で、見張りの試験が緑である
  - _Depends: 12.2, 12.3_
  - _Dispatch: mechanical — 確かめる項目を設計の表が与えている_
  - _Requirements: 1.3, 1.4, 1.5, 1.6, 1.7, 1.9, 2.3, 15.1, 15.2_

## Implementation Notes

- 置き場の `cargo` は必ず絶対の `--target-dir`（作業木の `target/pilot-windows-rs-next`）を付ける。置き場の中から `.cargo/config.toml` の相対の target-dir で組むと、パスが長すぎて proc-macro のリンクが LNK1104 で落ちる（1.2）。
- q2 のパッケージ名は `q2-<フォルダ名>`。emo 役の `src/main.rs`（bin）は骨組みに無いので q2 のタスクで足す（1.2 のレビュー）。
- `windows-bindgen`（この rev）のフィルターは短い名前（`CoInitializeEx`）で書く。`Windows.Win32...` の長い名前は `could not resolve filter entry` で断られる。フィルターが全部空だと止まる（1.3）。
- CopyLedger の問い 1 の写した行数（787）は例 2 本（73 行）を含む（設計の「約 790」と合う）。12.1 の合計もこの数え方で（2.1）。
- `.sys()` の束縛は `IDC_ARROW: PCSTR` を出すのに `PCSTR` を出さず、フィルターに名指しもできない。束縛のモジュールに `pub type PCSTR = *const u8;` を 1 行足す。定数は `i32` で出る（2.2）。
- 既定の形（この rev）: 戻り値の無い COM メソッドは `HRESULT`・`DWriteCreateFactory`／`D2D1CreateFactory` は IID を渡す形・継いだメソッドは基のインターフェースを名指ししないと出ない・`HRESULT::from_win32` が無い。`==` は番地が違っても COM の同一性で真（3.1）。3.2 は番地の違う参照の `==` の assert を残して比べる。
- `.minimal()`（この rev）は自由関数を生の `link!` だけで出す（包みが要る）。継いだメソッドの `Deref` の鎖は出る。`HMODULE` ほかの取っ手は裸のポインタの別名で `Default` が無い。`==` は `windows-core` の持ち物なので形を変えても COM の同一性のまま（3.2）。
- q2 の描く本体 `shared/draw_body.rs` は `use super::*;` で始まり、読み込む側の lib が `use <束縛>::*; use windows_core::*;` と `#[path]` で型の出どころを選ぶ。読む側は `windows-numerics` を直に依存に持つ。継いだメソッドは `ID2D1RenderTarget` の名指しが要る（4.1）。
- 一次記録の見立ての理由は「測った数」と「構造上の見立て」を分けて書き、1 つの写しの数を一般化しない（4.3 のレビュー）。`windows-bindgen`（この rev）にほかのクレートの型を参照させる公開の口は無い。
- この rev の `HWND` は既定・`.minimal()`・`.sys()` のどれも `*mut HWND__` の別名（0.62 の newtype と違う）。`Default`／`is_invalid` は無く `Hash` は在る。包みを Component にするには `unsafe impl Send/Sync` が要る（5）。REWRITE_TABLE の HWND の行はこれに合わせる。
- 0.101 の `HSTRING` は `Display` を持たない（`display()`・`to_string_lossy()` を使う）。`#[interface]` に `IUnknown_Vtbl` の取り込みは要らない（6.1）。
- `.implements` の名前はこの rev のメタデータでは `Windows.Win32.IDWriteTextRenderer1` の形（`Graphics.DirectWrite` の段が無い）。合わない名前は黙って無視され `_Impl` が出ないのに build は通る。生成した `_Impl` のメソッドは `HRESULT` を返すので中で `?` は使えない（6.2）。
- WinRT のクラスを名指ししても型と `new()` だけでメソッドは出ない。既定のインターフェース（`ICompositor` ほか）を名指しする。足りなくても build は通り、呼ぶ所で初めて分かる。相互運用の口はクラスでなくインターフェースを返す（`cast` が要る）（7.1）。
- 窓を出す bin は最初に Per Monitor v2 の DPI 対応を宣言する。しないと 200% の画面で窓は論理座標・画面の撮りは物理画素になり、撮った絵がずれる（段の行は全部 ok のまま）。色の確かめは中心だけでなく四隅も見る。q1 の `HWND` と bind-next の `HWND` は別の `HWND__` で `cast` が要る。q4 のフィルターに足した装置・DPI・`SetWindowPos` の名前を問い 7 も使うなら `common.txt` へ移す（設計の決まり）（7.2）。
- 「areka の文字の位置」は拡張書記素クラスターの序数（`ChoiceSpan::glyph_range`・`areka_sakura::cluster`）で、`char` の数ではない。`diff::compare` の最良のずれは 2 枚の矩形の和で数え、重なりの外の墨も違いに数える。大きさが違うときは `differing_pixels` は重なりだけ・`differing_at_best` は和で数える（8.1）。
- 問い 7 の絵の大きさは `old_side::IMAGE`（320×260）を新しい側も使う（本線の試験の 320×200 では比べの文が 2 列に折り返す）。今の側の文字の層は文字のアンチエイリアス・描画の設定・DPI・単位を設定しない（D2D の既定・灰色のアンチエイリアス）（8.2）。
