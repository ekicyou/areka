# Requirements Document

## Project Description (Input)
走っているゴーストの中で、バルーンの絵と当たり判定の出し先を差し替える手段が 1 つも無い（2026-09-24 再測定＝`reload|ReplaceTarget|Rebuild|swap_sink|replace_sinks` は seriko・present・text・dispatcher で 0 件）。出し先は起動時に `GhostBootOptions.sinks`（`crates/areka-ghost/src/runtime.rs` の `GhostBootOptions` の定義）として値で渡されて固定される。本坑 `areka-P0-shell-balloon-switch`（台帳 #50）には「降ろして起こし直す（案 A）」と「present・seriko・text に差し替えの語を足す（案 B）」の 2 案があり、どちらが安いかはコードを書いてみないと分からない。案 B の鍵は `EmoPresenter::attach_target`（`crates/areka-emo-present/src/presenter/hub.rs`・同じ id を再登録すると表示コンテキストごと置き換える）だが、差し替えの途中の 1 フレームに古い絵と新しい当たり判定が混ざらないか・空の窓が点滅しないかはコードからは分からない。本先進坑は、SHIORI を生かしたまま present のバルーン資産だけを差し替えて 1 フレームも崩れずに表示が続くかを使い捨ての実験で確かめ、開発者が go／違う／直す を判定できる知見を README 3 幕に残す。

## Introduction

本仕様は先進坑（pilot・使い捨て）である（`.kiro/steering/two-tunnel.md` の規律に従う）。確かめたい点は 1 つ——**走っているバルーン窓を閉じずに、present のバルーン資産だけを別のバルーンへ差し替えたとき、差し替えの前後のどのフレームにも「古い絵と新しい当たり判定の混在」「空の窓」「古い絵の残り」が出ないか**——である。

検体は `sample-ghost-kit`（`crates/pilot/Cargo.toml` の dev-dependencies に登録済み）から引く `StayseeBalloon` と、`emo2` 同梱の `emo2-kakukaku` の 2 つとし、この 2 つの間を往復させる。観測は描画結果を読み戻してフレームごとに数え、目視は補助とする。数え方そのものが誤っていないことは、わざと崩したフレームで数え方が必ず崩れを検出することで確かめる（較正）。

本仕様の成果物はコードではなく知見（go／違う／直す ＋ 学び）であり、一次記録は `crates/pilot/examples/pilot-balloon-asset-swap/README.md`（3 幕）である。go／違う／直す の判定は開発者（人間）が README を見て下す。

合否基準（brief を 2026-09-24 の要件ディスカッションで改訂）:
- **go**: 今の部品のまま（`attach_target` による置き換えをそのまま使う基準の版）で、差し替えの前後に崩れが 0（表示中の窓が消えない・古い絵が残らない・当たり判定が新しい絵と同じフレームで切り替わる）。
- **直す**: 基準の版では崩れるが、**差し替える前に古いバルーンを消す版**（本命の版）で崩れが 0 になる。案 B で進め、本坑で present に古い装着を片付ける口を足す。
- **違う**: 本命の版でも崩れが 0 にならない。案 A（降ろして起こし直す）へ倒すかを判定する。

差し替えの原則は「差し替える前に古いバルーンを消す」である（開発者裁定・2026-09-24）。他の差し替え方（古いほうを隠すだけの版・差し替えの時点を変えた版）は比べるために観測するが、判定の主な根拠は本命の版の数とする。

## Boundary Context

- **In scope**:
  - `crates/pilot/examples/pilot-balloon-asset-swap/` の example 1 本（`crates/pilot/examples/_template/` を写して着手）。
  - バルーン窓を 1 つ出し、窓を閉じずに `StayseeBalloon` と `emo2-kakukaku` の間でバルーン資産を往復させる実験。
  - 差し替えの前後のフレームを数える観測と、その数え方の較正。
  - 差し替え方を変えた版（基準の版・本命の版・比較の版）を並べて数を比べること（「直す」「違う」の判定材料）。
  - 有界な自動終了。
  - README 3 幕（動機・概要・検証結果）。
  - 実験に要る依存の `crates/pilot/Cargo.toml` への追加（pilot 側が依存するだけ）。
- **Out of scope**:
  - シェル（キャラクターの絵）の差し替え。バルーンで答えが出れば同じ形であり、出なければ本坑で別に考える。
  - seriko・text（文字の層）への差し替えの語の追加、および present を含む既存 crate のコードの変更（本坑 `areka-P0-shell-balloon-switch` の領分）。
  - 本体 `areka` への接続（`crates/areka` は bin だけの crate で、`build_boot_assets`・`wire_emo2_boot` は example から呼べない）。
  - 実際の SHIORI の読み込みとゴーストの起動。案 B で SHIORI が生き続けるのは、SHIORI に触れず present の資産だけを替えるからであり、本先進坑ではそれを「窓もプロセスも作り直さず、バルーン資産以外に触れない」ことで表す。
  - 案 A（降ろして起こし直す）の試作。
  - 本坑 `areka-P0-shell-balloon-switch` の設計。
  - 先進坑コードの production への流用（本坑は README の知見を見てクリーンに掘り直す）。
- **Adjacent expectations**:
  - 下流の本坑 `areka-P0-shell-balloon-switch` は、本先進坑の go 判定を `_Depends(confirmed): pilot-balloon-asset-swap` の前提依存として持つ（`.kiro/steering/roadmap.md` の台帳 #50 に記載済み）。本坑の design は README の検証結果を参照し、同じ結果を二重に書かない。
  - 検体の在処は `sample-ghost-kit` の窓口だけから引き、example が自前で綴らない。
  - 既存のライブラリ crate（`areka-emo-present`・`areka-emo-compose`・`areka-emo-atlas`・`areka-emo-text`・`areka-parsers`・`wintf`）はそのまま使う側に留まり、本先進坑はそれらを変更しない。組み立ての手本は `crates/areka/examples/emo-present.rs` とその同名フォルダ（計 1,271 行）である。

## Requirements

### Requirement 1: 先進坑の規律と隔離

**Objective:** As a 開発者, I want 先進坑のコードが出荷されるどの crate からも依存されない場所に閉じていること, so that 判定が go／違う／直す のどれであっても、コードを安全に捨てるか知見として残しておける

#### Acceptance Criteria
1. The pilot example shall コードを `crates/pilot/examples/pilot-balloon-asset-swap/` の下にだけ置き、フォルダ名を spec 名 `pilot-balloon-asset-swap` と一致させる。
2. The pilot example shall `crates/pilot/examples/_template/` を写して着手する。
3. The pilot shall コードと依存の変更を `crates/pilot/` の下（`crates/pilot/Cargo.toml` を含む）に限り、他の crate のファイルを変更しない。
4. The pilot shall 他の crate の `Cargo.toml` に `pilot` への依存を一切足さない。
5. While 先進坑のコードを書いている間, the pilot shall 整形・命名・テストの厳しさを緩めてよいが、前項 3・4 の隔離は必ず守る。
6. The pilot shall 一次成果物をコードではなく知見（go／違う／直す ＋ 学び）として扱う。

### Requirement 2: バルーン資産の差し替えの実験

**Objective:** As a 開発者, I want 走っているバルーン窓を閉じずにバルーン資産だけを別のバルーンへ差し替えて往復させる実験, so that 案 B の最小の部品（`attach_target` による置き換え）で差し替えが成り立つかを見られる

#### Acceptance Criteria
1. When example を起動したとき, the pilot example shall `sample-ghost-kit` の窓口から `StayseeBalloon` と `emo2` 同梱の `emo2-kakukaku` の 2 つのバルーンを引き、一方の資産でバルーン窓を画面に表示する。
2. When バルーン窓が表示された後, the pilot example shall 同じ窓の表示中の資産を他方のバルーンへ差し替え、さらに元のバルーンへ差し替え戻す（少なくとも 1 往復）。
3. The pilot example shall 差し替えを、表示先の窓を閉じて作り直すことなく、プロセスを再起動することなく行う。
4. The pilot example shall 各差し替えを、要求した時刻と対象（どちらからどちらへ）とともにログへ出す。
5. If 検体の取得または資産の読み込みに失敗したとき, the pilot example shall 失敗の理由をログへ出し、0 以外の終了コードで終わる（止まったまま待たない）。

### Requirement 3: フレームごとの崩れの観測

**Objective:** As a 開発者, I want 差し替えの前後のフレームを 1 枚ずつ調べて崩れの数を出す観測, so that 目視では見落とす 1 フレームだけの崩れも、数で判定できる

#### Acceptance Criteria
1. When 差し替えを要求したとき, the pilot example shall 要求の直前のフレームから、新しい資産の絵と当たり判定が揃った最初のフレームの後さらに一定数のフレーム（数は設計で決め、README に書く）まで、途切れなく各フレームを観測する。
2. The pilot example shall 各フレームで画面に出ている絵を、入力として渡した値からではなく、実際に描画された結果を読み戻して判別する。
3. The pilot example shall 各フレームで、表示されている絵がどちらのバルーンのもの（一方・他方・両方・どちらでもない）か、および効いている当たり判定がどちらのバルーンのもの（一方・他方・どちらでもない）かを判別し、当たり判定も渡した値からではなく実際に効いている判定に問い合わせて判別する。ここで「実際に効いている当たり判定」は、クリック透過を決めるのと同じ wintf の判定（`wintf::ecs::hit_test_in_window`）に、2 つのバルーンの α の形が食い違う標本点を問い合わせた結果を指す。OS の窓のクリック透過の付け外し（カーソルの動きで動き、フレームと揃わない）と当たり判定領域の名前（バルーンは持たない）は対象に含めない。
4. The pilot example shall 観測したフレームを次の 3 種の崩れで数え、1 つのフレームが複数の種類に当たるときはそれぞれに数える——「混在」（絵と当たり判定が別のバルーンのもの、絵と当たり判定の片方だけがどちらでもない、または絵に両方のバルーンが同時に見えている）・「空」（窓が消えている、または何も描かれていない）・「古い絵の残り」（新しい資産の絵と当たり判定が揃った最初のフレームより後に、古いバルーンの絵が少しでも見えている）。
5. When 1 回の差し替えの観測が終わったとき, the pilot example shall その差し替えについて観測したフレームの数と、3 種の崩れそれぞれの数を、崩れが無い種類も 0 と明示してログへ出す。
6. If 2 つのバルーンの絵または当たり判定を観測の方法で見分けられないとき, the pilot example shall その種類の数を 0 とせず「測れない」と報告する。
7. While バルーン窓を表示している間, the pilot example shall 開発者が目視で補助的に確かめられるよう、窓を実際の画面に出し続ける。

### Requirement 4: 数え方の較正

**Objective:** As a 開発者, I want 崩れを数える方法そのものが、わざと崩したフレームで必ず崩れを検出すると確かめられていること, so that 「崩れ 0」が数え方の見落としではなく本当に 0 だと信じられる

#### Acceptance Criteria
1. The pilot example shall 本番の差し替えの観測と同じ数え方を、わざと崩したと分かっているフレームに当てる較正を行う。わざと崩したフレームは実際の窓に出して本番と同じ観測の経路で取り、作った記録を分類の規則に当てるだけの較正で代えない。
2. When わざと「混在」させたフレーム（一方のバルーンの絵と他方のバルーンの当たり判定）を数えたとき, the pilot example shall それを「混在」として 1 以上数える。
3. When わざと「空」にしたフレームを数えたとき, the pilot example shall それを「空」として 1 以上数える。
4. When わざと「古い絵の残り」にしたフレームを数えたとき, the pilot example shall それを「古い絵の残り」として 1 以上数える。
5. When 差し替えを行わず同じ資産を表示し続けたフレームの並びを数えたとき, the pilot example shall 3 種の崩れをいずれも 0 と数える。
6. If 較正のいずれかの項が期待と異なる数を出したとき, the pilot example shall 較正の失敗をログへ出し、その実行で得た本番の崩れの数を有効な結果として扱わない。
7. The pilot example shall 較正の結果を、本番の崩れの数と並べてログへ出す。

### Requirement 5: 差し替え方の試し分け

**Objective:** As a 開発者, I want 素の置き換えで崩れが出るときに、何を足せば崩れが消えるかを差し替え方ごとに数で比べること, so that 案 B で足す物が何か、および「直す」と「違う」のどちらに当たるかを見分けられる

#### Acceptance Criteria
1. The pilot example shall `attach_target` による置き換えを、古い装着を片付けずにそのまま使った差し替えを、基準の版として観測する。
2. The pilot example shall 差し替える前に古いバルーンの装着（絵と当たり判定）を消し、同じフレームの中で新しいバルーンを装着して表示する差し替えを、本命の版として観測する。
3. The pilot example shall 比べるための版として、新しいバルーンを別に装着して同じフレームの中で古いほうを隠すだけの版と、基準の版・本命の版・隠すだけの版のそれぞれを差し替えの時点（フレームの中で配置が決まる前／画面への反映の後）を変えて行う版も観測する。
4. The pilot example shall 基準の版の崩れの数にかかわらず、前 2 項のすべての版を観測する。
5. The pilot example shall 各版の崩れの数を、版の名前とともに区別してログへ出す。
6. The pilot shall 1 フレーム遅らせて辻褄を合わせる版を、崩れを消した版として扱わない。
7. If 本命の版で古い装着を消す公開の口が既存の crate に無いとき, the pilot shall 先進坑の中では example が既存 crate の内部の物を外から探して消してよいが、既存 crate のコードは変えず、本坑で present に古い装着を片付ける正規の口を足す必要があることを学びとして記録する。
8. If どの版でも崩れが 0 にならず既存の crate を変えなければ消せないと分かったとき, the pilot shall その変更を先進坑の中で行わず、何を変える必要があるかを学びとして記録する。
9. The README shall 本命の版の数を判定の主な根拠として示し、比較の版の数は補助として並べる。

### Requirement 6: 有界な自動終了

**Objective:** As a 開発者, I want example が人手を待たずに決まった時間の内側で自分で終わること, so that 実験が長引いたり止まったまま残ったりしない

#### Acceptance Criteria
1. The pilot example shall 起動から決まった上限時間の内側で、人の操作なしに自分で終了する。
2. Where 環境変数で上限時間が与えられている場合, the pilot example shall その値を上限として使う（`AREKA_APP_SMOKE_EXIT_MS` に相当する仕組み）。
3. Where 環境変数で上限時間が与えられていない場合, the pilot example shall 既定の上限時間を使い、上限なしで走り続けない。
4. When すべての差し替えの観測を終えたとき, the pilot example shall 上限時間を待たずに終了する。
5. When 終了するとき, the pilot example shall 終了の理由（観測の完了・上限時間の到達・失敗）をログへ出す。
6. If 上限時間に達した時点で観測が終わっていないとき, the pilot example shall 観測が途中で打ち切られたことを、それまでの数とともにログへ出す。

### Requirement 7: README 3 幕と判定

**Objective:** As a 開発者, I want 実験の動機・作った物・数の結果が README 1 つにまとまっていること, so that README を見て go／違う／直す を判定し、本坑の設計がそこを参照できる

#### Acceptance Criteria
1. The pilot shall 一次記録として `crates/pilot/examples/pilot-balloon-asset-swap/README.md` を、動機・概要・検証結果の 3 幕で作る。
2. The README shall 動機の幕で、対応する本坑 `areka-P0-shell-balloon-switch` を名指しし、案 A と案 B のどちらが安いかを決める材料であることを書く。
3. The README shall 概要の幕で、作った物と実行法（`cargo run -p pilot --example pilot-balloon-asset-swap`、上限時間の与え方を含む）を書く。
4. The README shall 検証結果の幕で、較正の結果・版ごとと差し替えごとの 3 種の崩れの数（0 も明示する）・観測で分かることと分からないこと（読み戻した描画結果が画面の見た目のどこまでを表すか）・学び・日付を書く。
5. The README shall 検証結果の幕に、測った数が合否基準（go／直す／違う）のどれに当たるかの見立てを添えるが、判定の欄は開発者が下すまで確定させない。
6. The pilot shall go／違う／直す の判定を開発者（人間）に委ね、Claude Code 単独で go を宣言して本坑へ進まない。
7. When README を書き込んだとき, the pilot shall 書いた内容を読み戻し、3 幕が揃って切り詰めが無いことを確かめる。
