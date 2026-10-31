---
inclusion: manual
updated_at: 2026-10-31
---

# Roadmap — areka（α 後・M3 へ向けた並べ直し）

> **M1 は 2026-09-11、M2（α）は 2026-10-02 に完成宣言済み**。本ファイルは 2026-10-02 の棚卸⑳で、α 後の着手順へ書き直した。**完了した spec の行・棚卸⑭〜⑲の裁定・α のウェーブ表・旧「登記だけの行」の全文は [roadmap-history.md](roadmap-history.md) の「2026-10-02 棚卸⑳退避」節へ逐語で移した**（history が全文正本・非改変）。2026-10-04 の棚卸㉑で、旧ウェーブ表（C1・C2）と棚卸⑳の裁定を同じ history の「2026-10-04 棚卸㉑退避」節へ移した。2026-10-05 の棚卸㉒で、旧ウェーブ表（C3 と予定の C4）・直列の列・覚え書き・直接修正候補・棚卸㉑の裁定を「2026-10-05 棚卸㉒退避」節へ移した。2026-10-10 の棚卸㉓で、旧ウェーブ表（C4）・初回リリースの段取り・直列の列・brief にした覚え書き・直接修正候補・棚卸㉒の裁定を「2026-10-10 棚卸㉓退避」節へ移した。 2026-10-18 の棚卸㉗で、v0.1.0 を開発者の新しいゴール（SSP 並みの自動試験・ゴースト「窓際のぱすたさん」が SSP と同じに動く）で組み直し、12 本を起票した（旧ウェーブ C8・旧直接修正候補・棚卸㉖の裁定・旧 v0.1.0 の行・`v0.0.2` の後の申し送り 8 項目は history「2026-10-18 棚卸㉗退避」）。 2026-10-23 に、開発者「ロードマップについて、膨れ上がってきてる気がします。完了済みはヒストリーに動かしてください。」を受けて、完了済みの記録（完了サマリの表の長い説明・数の経緯・組み終えたウェーブの記述・spec が全部完了したテーマの節と棚卸㉔〜㉖の申し送りの節）を history「2026-10-23 完了済みの退避」へ逐語で移した。
> 正本配置: 本ファイルが正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。ukadoc 網羅の段階・順位の正本は `doc/ukadoc-coverage/`（`briefing.md`・`linkage.md`・`roadmap-draft.md`）。完了した spec の実装の詳細は各 `completed/` spec が正本。
> **読む順**: どの版に何を入れるか＝「版の段」。いま何を始めるか＝「ウェーブ編成」。個々の spec の中身＝「spec 台帳」と各 brief。brief を持たない宿題＝「覚え書き」。

## M1 ゴール ✅（2026-09-11 完成宣言）

areka（x64）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・32bit SHIORI）を「そのまま」起動→会話→撫で→メニュー→終了まで E2E 実走させる。emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台。

**✅ 完成宣言**: 適合検証項目表 20 項目全合格の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md`・項目別は同 `acceptance-record.md` §8.1）。持ち越し 8 件（同 §6）はいずれも判定を書き換えず、引受先は下の spec 台帳に全て実在する（§13.1 行 3「初回起動限定の位置調整」だけは「違和感があれば個別仕様を切る」の据え置き＝spec なし）。

## M2 ゴール（α）✅（2026-10-02 完成宣言）＝第三者がデスクトップマスコットを管理できる

**✅ 完成宣言**: 第三者の手順の検証項目表（13 項目と付随の確認 2 つ）の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-alpha-release-signoff/verification/alpha-completion.md`・項目別は同 `acceptance-record.md` §7）。署名の根拠のコミット `8460506d`・zip `areka-alpha-x64-20261002-8460506.zip`。

**利用者の一周**: zip を展開して起動 → `.nar` を窓へ落とす（またはメニューから選ぶ）→ ゴーストが起動する → 右クリックメニューでゴースト／シェル／バルーンを替える → ネットワーク更新で作者の修正を受け取る → 終了 → 再起動で前回の状態に戻る。表現力は M1 の水準（emo2 が普通に動く）で据え置いた。メニューは Win32 標準。

**持ち越し**: 完成判定 §6 の 5 行は 4 本の spec（`install-live-target-hazards`・`drag-click-without-move`・`package-check-temp-cleanup`〔10-02 に `release-package-versioned` へ合流〕・`install-companion-canon`〔10-03 に要件の段で `install-companion-reading`・`ghost-standard-balloon`・`shell-companion-balloon` の 3 本へ引き継いで閉じた〕）へ起票済み＝ウェーブ C1〜C4。emo2 の 2 件は ghost_dev へ申し送り済み。

## 版の段（着手の大枠の正本・2026-10-15 開発者と `/kiro-discovery` で決めた）

> 出どころ（2026-10-15 開発者）: 「起票スペックについて、優先度の整理をしたい」「直近の 0.0.3（インストール仕様が足りない件と致命的バグ）、0.1.0、wintf や dola の基幹の整備、ukadoc より優先度が高いもの、夢仕様と ukadoc で優先度が低いもの、くらいの段階」「α って言い方がよくないかな。α1・2・3… って段階なので」。
> **呼び名**: 「α」の語は使わず、版の番号で段を呼ぶ。完成宣言済みの M1（2026-09-11）・M2（2026-10-02・zip の第三者の一周）は歴史として残す。旧「M3 ゴール（未確定）」はこの節で置き換えた（「伺かの冠」の候補は `doc/ukadoc-coverage/roadmap-draft.md` に残る）。
> **並べ方**: 若い版から取る。**今の版（まだ公開していない最も若い版）が残っているあいだ、後の版の spec はウェーブに入れない**（2026-10-16 開発者「ゴールが近いので他の版の割り込みは避けて」）。空いた席に入れてよいのは「版に縛らない」列のうち、今の版の着地までのウェーブ数を増やさないものだけ（同日・開発者「版に含めてもよいが、その場合、0.0.3 の着地（ウェーブ数）が増えることのないように」）＝先頭で、今の版の spec と触るファイルが重ならず、今の版が使う 1 本だけの席を取らず、見立ての上限がそのウェーブで一番大きい今の版の spec の下限以下で、人や外の待ちも、静かな机も、前後の全体テストの比べも要らないもの（判定の正本は `/kiro-next` のスキルの「Version gate」）。迷ったら席は空けておく。同じ版の中は「ウェーブ編成」の並び順の決まり（ぶら下がる数 → 区分 → 古さ・触るファイルの重なり 0）。**版の中身は、やってみて重ければ前後してよい**（開発者「やってみて大変そうになったら前後するかもしれません」）。
> **バグ（区分 B）**: 開発者「α 段階までに一度整理できてることが望ましいが、後回しにするかもしれない」＝既定は v0.1.0 まで。後へ回したのは、時刻の土台が直し方を引き取る `seriko-talk-clock-fidelity`（v0.2.0）と、測ることが本体の 2 本（版に縛らない）。新しく見つかったバグは、致命的なら今の版、そうでなければ次の版へ足す。
> **済んだ版の行**（2026-10-23 開発者「０本になったバージョンターゲットは消して整理して欲しい。生きてるバージョンターゲットが見れれば良い」）: 表に残すのは生きている行だけ。公開した版（タグを打った版）の行と、spec が 0 本になった版でない組の行は、棚卸で表から外し、history のその棚卸の退避の節へ逐語で移す（数の行にも 0 を書かない）。「版に縛らない」と「いつか」の行は 0 本でも残す。spec が 0 本でまだ公開していない版の行は公開まで残し、そのあいだのウェーブの開始コマンドは `/release <版>`（同じ日・開発者「ウェーブの対象が０本となり、リリースできるなら、/releaseを開始コマンドとせよ」）。判定の正本は `/kiro-next` のスキルの Step 6「Release gate」と Step 7。2026-10-23 棚卸㉙で v0.0.3 の行（10-18 公開）を外した＝history「2026-10-23 棚卸㉙退避」。

| 版 | 目標 | spec（並びは依存の順） |
|---|---|---|
| **v0.1.0** | **SSP 並みの自動試験ができる（開発を速くするのが第一）＋ゴースト「窓際のぱすたさん」が SSP と同じに動く**（2026-10-18 開発者「0.1.0のスコープを決定しました。ゴールは、SSP並みの自動試験が出来るようにすること、とします。SSPなみの起動オプション、MCPサポートの安定化、kiro-watchの機能追加など、開発がスピードアップすることを第一目標とします。もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です」・検体は開発者の指示で「ぱすた開発」のセッションに問い合わせ、ghost_dev の `pasta-in-windows`〔表示名「窓際のぱすたさん」・コミット `e1e0559` の書庫〕に決めた・棚卸㉗で組み直した。旧 v0.1.0〔よく使われるゴーストが SSP と同じように動き、配れる〕の行は history「2026-10-18 棚卸㉗退避」） | 机＝✅ `impl-watch-test-desk`（10-19 着地・開発者「watchコマンドの機能拡充は優先度高いので最初のウェーブにしてください」）・`areka-test-binary-stability`（測ることが本体・ウェーブの外）／起動オプション＝✅ `startup-options-core`（10-19 着地） → ✅ `offline-surface-dump`（10-23 着地）・✅ `readonly-mode`（10-24 着地） → ✅ `startup-install-option`（10-30 着地） → ✅ `install-before-boot`（10-31 着地・2026-10-30 `startup-install-option` の完了の棚卸で起票・同 spec の要件 10.2・`--install` と `initial_install.nar` を起動の解決の前に入れて空の置き場からも起こす・2026-10-31 棚卸㉛で版の門に数えると決めた＝空の試験の根から起こす「SSP 並みの自動試験」そのもの）／実走の道具＝✅ `real-run-mcp-kit`（10-19 着地） → ✅ `emo2-real-run-stability`（10-23 着地）／MCP＝✅ `mcp-kanade-tools`（10-19 着地）・✅ `mcp-reload`（10-22 着地）・✅ `mcp-receiver-bugs`（10-18 着地） → ✅ `mcp-strict-errors`（10-30 着地）・✅ `bridge-store-user-root`（10-30 着地）／MCP の口を広げる前の守り＝✅ `script-security-level`（10-23 着地）・✅ `property-name-case-fold`（10-23 着地）／起動を速く＝✅ `placement-measure-bake-once`（10-31 着地・ぱすたの面 1000 に動く WebP が 3 つ）／ぱすた＝✅ `sample-url-fetch`（10-19 着地） → ✅ `pasta-in-windows-verification`（10-23 着地）・✅ `ghost-inner-balloon`（10-23 着地・ゴーストの中の `balloon\<名>\`。ぱすたの縦書きバルーンはぱすたの書庫の直し〔ghost_dev の `c32229e`・未リリース〕で入る）・✅ `zorder-scope-inversion`（10-19 着地）→ ✅ `stay-on-top-directives`（10-31 着地）・✅ `script-small-tags`（10-31 着地）・✅ `descript-window-keys`（10-24 着地）・✅ `window-link-directives`（10-23 着地） → ✅ `dpi-realign-remembered-chain`（10-24 着地）・✅ `window-work-area-clamp`（10-30 着地・2026-10-23 `window-link-directives` の完了の棚卸で起票・ぱすたの窓が作業領域の下へはみ出す・起動の並べ直しが窓を隣のモニタへ置く・2026-10-24 `vertical-choice-hover-band` の完了の棚卸で件 4＝バルーンだけが作業領域へ寄せられてぱすたの × が縦書きの 1 列目にかぶる、を足した＝致命的ではないが v0.1.0 のゴール「ぱすたが SSP と同じに動く」に当たる＝起動の並べ直し・拡大率の後の詰め直し・面の拡大・記憶から戻す組の窓を作業領域へ寄せ、起動のトークの位置の指令を最初の面の表示まで預け〔3.2 の測りで見込み H1〕、バルーンだけが横にはみ出すときは反対の側へ並べ替える〔2026-10-30 の裁定・完了 `areka-P0-windowposition-limit` の押し込みの決まりを改めた〕）・✅ `mouse-click-wheel-events`（10-24 着地） → ✅ `shell-mouse-cursor`（10-30 着地）・`popup-menu-shell-look`（生きている決まり 6 を解くかの裁定待ち）／配れる・見た目（2026-10-18 開発者が残した）＝✅ `dot-sakura-specimen`（10-23 着地） → ✅ `default-balloon-selfmade`（10-30 着地）・`winget-release-automation`（winget-pkgs の承認待ち）／どっとさくらの崩れ（2026-10-23 `dot-sakura-specimen` の完了で起票・開発者の条件「次のリリースまでに治ること」）＝✅ `shiori2-protocol`（10-24 着地・SHIORI/2.x の口だけを持つゴーストが話さない）・✅ `seriko-pattern-calls`（10-31 着地・pattern定義のアニメーションを呼ぶ 7 語と `move`）／ぱすたの検証の申し送り（2026-10-23 `pasta-in-windows-verification` のタスク 5.3 の `/kiro-discovery`・開発者「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」）＝✅ `mcp-scope-rect-property`（10-23 着地・外から窓とバルーンの rect を読む・動く値の口） → `shell-scale-linkage`（ぱすたの倍率の連動）・✅ `animated-surface-log-flood`（10-31 着地・動く絵の適用の INFO 行の量）・✅ `balloon-scroll-arrows`（10-31 着地・`balloon-markers` の前半）・`balloon-communicatebox`（棚卸㉛で要件定義を Opus に改めた）・✅ `vertical-choice-hover-band`（10-24 着地・縦書きの選択肢の反転の帯・開発者の目視の判定「選択肢をマウスホバーした時の背景の塗りが崩れている。横書きの時と縦書きの時で、枠の補正方法がことなるのでは？」・起票の提案への「承認」）・✅ `emo-text-dwrite-fidelity`（10-30 着地・2026-10-24 `/kiro-discovery`・開発者の目視「「と、」に、不自然な空白が入ってませんか？DWriteの縦書きに関する設定が何か抜けているのでは。」＝縦書きの開き括弧`「`が字の枠の上端に描かれて続く字との間が空く・GDI 流のフォント名「游ゴシック Medium」が当たらず中太にならない＝縦書きの開き括弧類を字の枠の下半分へ〔既定の機能の組の vpal 相当の位置の調整を外す〕・列の em の箱を行の矩形の真ん中へ・`font.name` を GDI の名前で引き直す〔家族・太さ・幅・斜体〕・既定の太字斜体を書式へ・SSP との差〔約 1.6px・墨の量は描き手の違い〕は案 C で受け入れた）／版に縛らない列から上げた 1 本（2026-10-23 棚卸㉙）＝✅ `real-run-log-noise`（10-30 着地・10-23 に足した件 2〔どっとさくらの同梱バルーン `bottle` で毎フレーム出る `WARN`〕に開発者の条件「次のリリースまでに治るように」が付いた＝`shiori2-protocol`・`seriko-pattern-calls` と同じ扱い）（着地まで約 3 ウェーブ＝C14〜C16・棚卸㉛）＝**5 本**（43 本のうち ✅ の 38 本＝✅ `script-small-tags`〔10-31〕・`install-before-boot`〔10-31〕・`mcp-receiver-bugs`〔10-18〕・`mcp-kanade-tools`〔10-19〕・`real-run-mcp-kit`〔10-19〕・`startup-options-core`〔10-19〕・`impl-watch-test-desk`〔10-19〕・`zorder-scope-inversion`〔10-19〕・`sample-url-fetch`〔10-19〕・`mcp-reload`〔10-22〕・`emo2-real-run-stability`〔10-23〕・`property-name-case-fold`〔10-23〕・`window-link-directives`〔10-23〕・`script-security-level`〔10-23〕・`dot-sakura-specimen`〔10-23〕・`ghost-inner-balloon`〔10-23〕・`offline-surface-dump`〔10-23〕・`pasta-in-windows-verification`〔10-23〕・`mcp-scope-rect-property`〔10-23〕・`shiori2-protocol`〔10-24〕・`descript-window-keys`〔10-24〕・`vertical-choice-hover-band`〔10-24〕・`dpi-realign-remembered-chain`〔10-24〕・`readonly-mode`〔10-24〕・`mouse-click-wheel-events`〔10-24〕・`default-balloon-selfmade`〔10-30〕・`shell-mouse-cursor`〔10-30〕・`real-run-log-noise`〔10-30〕・`mcp-strict-errors`〔10-30〕・`startup-install-option`〔10-30〕・`emo-text-dwrite-fidelity`〔10-30〕・`window-work-area-clamp`〔10-30〕・`bridge-store-user-root`〔10-30〕・`seriko-pattern-calls`〔10-31〕・`balloon-scroll-arrows`〔10-31〕・`stay-on-top-directives`〔10-31〕・`animated-surface-log-flood`〔10-31〕・`placement-measure-bake-once`〔10-31〕は着地した） |
| **v0.1.0 の余力の枠** | 版の門に数えない。v0.1.0 の残りが人や外の待ちで止まり、手が空いたら取る（2026-10-18 開発者「multi-ghost-runtimeは決めかねてる。設計上は障害は無いですよね？マルチゴースト。0.1.0が出せなくてやることが残ってたらやるかも」） | `multi-ghost-runtime`（設計の上の障害は無い＝ゴーストの部品はもうゴーストごとに作れ、止めているのは areka 本体の「1 つだけ」の置き場と、止まり方がアプリの終了へ流れること・24〜30 タスク・v0.1.0 の大半とファイルが重なる＝ほぼ単独で走る） → `ghost-call`＝**2 本** |
| **v0.1.0 の後の隙間**（本体の crate 分け＋ponytail による見直し・版に縛らない・優先度が高い） | ponytail の audit で出た指摘を全部直す（2026-10-19 開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」）。はじめ v0.1.0 の前、同じ日のうちに v0.1.0 の直後の版 v0.1.1 へ移し、**2026-10-22 に版の段から外した**（開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」）＝v0.1.1 の版は無くなり、v0.1.0 の次の版は v0.2.0。**取り方**: ⑴ v0.1.0 の版の門を越えるまでウェーブに入れない。⑵ v0.1.0 の後は、今の版の spec が使わない席（隙間）に、版に縛らないほかの列より先に入れる。⑶ **v0.2.0 の作り直しと触るファイルが重なる分は、作り直しの前に済ませる**（同じ日の問いへの開発者の答え「作り直しの前に済ませる」）＝重なる v0.2.0 の spec より先に取り、その spec は見直しの着地を待つ（重なりの相手は各行の「版」の欄）。このため v0.2.0 の先頭のウェーブは重なる見直しで埋まってよく、v0.2.0 の着地のウェーブ数が増えることは受け入れる。⑷ 重なりは 2026-10-22 に brief どうしを突き合わせて決めた（v0.2.0 の 18 本はまだ brief だけ）＝ウェーブを組む棚卸で、v0.2.0 の spec の設計が触るファイルと突き合わせ直す。 | **本体の crate 分け 6 本**（隙間の先頭・2026-10-28 `/kiro-discovery`・開発者「areka 本体 crate (crates/areka) の肥大を解消するため、`placement` と `emo2_boot` を独立 crate へ分離したい」「この際、areka crateのコードベースが大きすぎる問題をサルベージすべきかもしれない」・全部直列・本体を受け持つ audit 8 本はそれぞれの範囲を分ける spec の後・「テーマ別の決めごと」の「本体の crate 分け」）＝`placement-crate-split` → `runtime-module-untangle` → `runtime-crate-split` → `input-crate-split` → `session-module-gather` → `session-crate-split`／v0.2.0 と重なる **27 本**（作り直しの前に）＝`audit-areka-app`・`audit-areka-input-menu`・`audit-areka-install-update`・`audit-dola`・`audit-emo-atlas`・`audit-emo-compose`・`audit-emo-present`・`audit-emo-text-layout-draw`・`audit-emo-text-state-actor`・`audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch`・`audit-ghost-catalog`・`audit-kanade`・`audit-mcp-update-crates`・`audit-parsers-sakura`・`audit-placement-core`・`audit-seriko`・`audit-shiori-host32`・`audit-sylphya-actor`・`audit-tools-perf-run`・`audit-tools-scripts`・`audit-wintf-graphics-world`・`audit-wintf-input`・`audit-wintf-layout-widget`・`audit-wintf-runtime`・`audit-wintf-window`／重ならない **6 本**（いつでも隙間で）＝`audit-impl-watch`・`audit-pilot`・`audit-placement-follow`・`audit-test-kits`・`audit-tools-perf-judge`・`audit-ukadoc-survey`＝**39 本**（crate 分けの 6 本＋見直しの 33 本。見直しの 33 本はどのファイルもちょうど 1 本の持ち物＝互いに重ならない・「テーマ別の決めごと」の「ponytail による見直し」） |
| **v0.1.0 から外した 5 本**（行き先は次の版を決めるときに） | 2026-10-18 棚卸㉗で v0.1.0 のゴールを組み直したとき、ゴールに当たらず、開発者が v0.1.0 に残さなかったもの | `animated-image-import`（ぱすたの動く WebP は element で描き `import` は使わない・`import` の画像の名前が 0 に化けて記録も出ない件は残る）・`seriko-trigger-lottery-gaps`（バグ・ぱすたの着せ替えのまばたきは今の作りで動く）・`extent-element-offset`（バグ）・`bottom-row-text-clip`（バグ）・`balloon-webp-names`（`default-balloon-selfmade` が絵を `.webp` で持つと決めたら、働きの前提として v0.1.0 へ戻る）＝**5 本** |
| **v0.2.0** | wintf・dola の基幹の整備 | `wintf-visual-tree` → `wintf-d2d-composite-rules` → `wintf-visual-damage-redraw`・`draw-methods-canon` → `draw-methods-blend`・`collisionex-regions` → `hit-id-pass`・`compose-f16-precision`・`dola-interrupt-ticks` → `talk-anchor-interrupt-ticks` → `instant-to-interrupt-ticks`・`seriko-talk-clock-fidelity`（時刻の土台の後＝直し方を `talk-anchor-interrupt-ticks` が引き取る）・`dola-yaml-backend`・`tick-gate-adoption`（保留のまま・短い A/B の測り方をここで組む）・`element-clipping-option`（D2D の切り抜きで描く形にそろえる＝`wintf-d2d-composite-rules` の brief が下流に挙げる）・`talk-thread-count-growth`（2026-10-19 `mcp-kanade-tools` の完了の棚卸で起票・致命的でない新しいバグ＝次の版・トークのたびにスレッドかスレッド名簿の項目が増える）・`sylphya-sqlite-kvs` → `mcp-set-property`（2026-10-21 `/kiro-discovery` で起票・開発者「これらの改修は、wintfやdolaなどの基礎整備と同じフェーズで実施することとする」＝sylphya の保存を sqlite へ移し、速い層・メモリーの sqlite・ファイルの sqlite を 1 つの KVS の口で扱い、アプリ・ゴースト・シェル・バルーンの 4 層を重ねて読む／手書きの `sylphya.toml` の代わりに MCP の `set_property` で書く。wintf・dola の改造とは触るファイルが重ならない・`audit-sylphya-actor`〔見直し・v0.2.0 と重なる分＝先に取る〕の着地の後）・`shiori4-api`（2026-10-23 `/kiro-discovery` で「いつか」から移した・開発者「対応時期は、dolaなどの基幹部分大整備と同じタイミングで。すぐ必要ではないが、基幹部分となるので」＝COM の作りかけを消し、SHIORI4 を C ABI の `ekicyou/shiori4` へ完全に移す・shiori4 の `host-protocol` が入った版の公開を待つ＝2026-10-24 に shiori4 v0.1.0 で満たされた・brief を v0.1.0 に合わせて更新）・`ghost-alias-file`（2026-10-23 棚卸㉙で起票・完了 `dot-sakura-specimen` の research の残り物・致命的でない新しいバグ＝次の版・シェルのフォルダの `alias.txt` を読まない〔検体のどっとさくらの `alias.txt` は持っていない面の番号を持っている面へ振り替える〕）・`talk-script-metadata`（2026-10-25 `/kiro-discovery` で起票・台本ごとの付帯情報＝実行するゴーストの置き場〔ghost/master〕と送り手・開発者「dola とかの基礎整備をするタイミング」）・`ghost-switch-balloon-fallback`（2026-10-30 `startup-install-option` の完了の棚卸で起票・致命的でない新しいバグ＝次の版・切替先にも置き場にもバルーンが無いと切り替えが落ちる）・`balloon-flip-side-rules`・`window-resetwindowpos-hold`（2026-10-30 `window-work-area-clamp` の完了の棚卸で起票・致命的でない新しいバグ＝次の版・前者は反対の側へ並べ替えたバルーンが相方の窓に重なりうる件と右に出すバルーンの横のずらしの向き、後者は起動の直後の `\![execute,resetwindowpos]` が最初の表示を待たない件）・`doc-tree-tidy`（2026-10-28 `/kiro-discovery` で起票・`doc/` と `docs/` の置き場をまとめ、名前の書き方をそろえ、案内の表と実物の突き合わせを試験にする・開発者「このspecだけは0.1.0の直後、doraとかの基盤整備に置きます」・パスを動かすので v0.2.0 の早いウェーブに 1 本だけで）＝**25 本**（`wintf-visual-tree`・`wintf-visual-damage-redraw` は 2026-10-21 `/kiro-discovery` で起票＝開発者「`ecs\visual` 層のコード群は実装準備されているが採用されていない。これらを新しいビジュアル層・描画層として取り込んでほしい」「実施フェーズはwintf/dola改修のタイミングとする」・「テーマ別の決めごと」の「ビジュアル層の取り込み」。`collisionex-regions` と `element-clipping-option` は 2026-10-16 に v0.1.0 から移した。前者は `hit-id-pass` が待つので同じ版へ）。開発者「0.2.0 のスコープはまだ少ないので」低優先度で起票した 4 本（合成の精度・時刻の土台）も入れた。**後続の例外**: `draw-methods-canon` は `element-base-method` の後続だが、2026-10-15 に D2D の道（`wintf-d2d-composite-rules` の後）へ書き直したのでここに残す。`seriko-talk-clock-fidelity` も同じ理由（時刻の土台が直し方を引き取る）。`dola-yaml-backend` は `release-cycle` の後続だが、中身は dola の手入れ（`deny.toml` の見逃しで `/release` は止まらない） |
| **v0.3.0** | 目玉のフェード＋小さく早く出す（開発者「0.3.0 はタグを打ちたいのもあるので、なるべく早く出したい」） | 目玉＝`text-reveal-fade`・`balloon-scroll-fade`（開発者「フェードは需要があるので 0.3 の目玉機能にしたい」・`balloon-scroll-fade` はシェル内バルーンの後続だが目玉なのでここ）／入力＝`inputbox-user-input`（`mouse-click-wheel-events` は 2026-10-18 棚卸㉗で v0.1.0 へ上げた＝ぱすたの閉じるボタンとボールのトークが `OnMouseClick` で動く）／プロパティ＝`currentghost-property-tree`＝**4 本**。覚え書き「ゴースト同士の掛け合い」（`OnOtherGhostTalk`・`\![set,otherghosttalk]`・`\![raiseother]`）もこの版で起票する |
| **v0.4.0** | 最低限のアニメーション（dola の整備の後） | `element-animation-easing`（開発者「イージングは dola の整備が終わってから、0.3 の後」「dola 関係の整備が終わり次第、最低限のアニメーション仕様を実現する」）・`shell-balloon-scale-setting`（2026-10-17 `/kiro-discovery` で起票・開発者「wintf や dola の改造が終わった後のフェーズのネタ」＝ゴーストごとの表示倍率を DPI の倍率に掛ける・入口の設定画面は windows-reactor 待ち）・`menu-thumbnail-preview`（2026-10-20 `/kiro-discovery` で起票・開発者「対応時期はdolaとかの基盤対応が終わってからで良いかな」＝右クリックメニューの項目にカーソルを乗せている間、サムネイルをメニューの横の別の窓に出す・動く絵は動かす）・`shell-hover-animation`（2026-10-20 `/kiro-discovery` で起票・開発者「シェルアニメーションに『マウスホバーが発生した時』だけ発動させたい」「対応時期はdola関係が落ち着いてから」＝当たり判定に乗っている間だけ動くアニメをシェルの書き方だけで書く・トークの最中も止まらない）・`resident-ghost-rotation`（2026-10-21 `/kiro-discovery` で起票・開発者「これは、土台ができた後でよい。夢仕様とまではしないが、少なくともdola、wintf関係がリリースされた後の次のリリース候補」＝裏で SHIORI だけを常駐させ、台本を返したゴーストへ表示を替える・同時に返したら表示中の再生が終わるまで待たせる・`multi-ghost-runtime` の後）・`parallel-visual-script`（2026-10-21 `/kiro-discovery` で起票・開発者「さくらスクリプトの冒頭に特定のタグがあるとき、これまで再生していたスクリプトを『止めず』、スクリプト再生を並走させる」＝見た目と音だけの台本をトークと並走させる）＝**6 本**（SERIKO の残りと `animated-image-import` は着手済みのテーマの後続として v0.1.0 へ移した） |
| **ノベルゲーム基盤**（版の番号は pasta の依頼書が届いてから振る） | pasta（SHIORI のエンジン）の 0.5.0 のノベルを areka で遊べる土台（2026-10-28 `/kiro-discovery`・開発者「実施タイミングは、少なくともdola関係の整備のあと、ノベルゲーム基盤の時か、その後ろ」・v0.4.0 の後ろ）。今 brief を持つのは pasta との取り決めに左右されない 3 本だけ。進行と入力・バックログ・セーブ・システム画面の部品・ノベルの音・演劇のパッケージは「テーマ別の決めごと」の「利用統計とノベルゲーム基盤」の受け口の節で、pasta の `novel-areka-contract` の依頼書を待つ | `sound-canon-tags`（先頭・正典の音のタグ・再生の土台は Media Foundation）・`ghost-material-metadata`（素材 ID と descript のキー・探す順・借りないでの印）→ `shell-multi-compose`（1 つの舞台の窓に何枚ものシェル・普通の窓と全画面・場面転換・v0.2.0 の新しい描画の層の上）＝**3 本** |
| **利用統計**（ノベルゲーム基盤と並走してよい・dola の整備の後） | 利用者を秘匿しつつ利用者ごとに識別し、1 日単位で利用状況を集計して GitHub Pages で公開する（2026-10-28 開発者「利用集計はplayとは独立して進められるので、dolaが終わったタイミングでノベルゲームと並走するかもしれません」）。仕様（U1）・発信器（U2）・集計サイト（U3）は外の新しいリポジトリの持ち物（`doc/ideas/usage-stats.md`） | `areka-usage-stats`（`ghost-material-metadata` と外の U2 の後・まずゴースト・シェル・バルーンだけ・演劇の信号は演劇のパッケージの後）＝**1 本** |
| **いつか** | 夢と ukadoc（低）・やることが無くなってから | 書き足しのタグ＝`balloon-append-tags`（2026-10-31 `balloon-scroll-arrows` の完了の棚卸で起票・`\C` と `\*`・区分 C・2026-10-31 棚卸㉛でいつかのまま＝ぱすたもどっとさくらも使わない・YAYA の雛形のゴーストは `\C` を使う＝v0.3.0 へ上げるかは開発者）／夢＝`text-reveal-dance`（開発者「正直、やることなくなってからでよい」）・`window-snap`／文字飾りの細部＝`text-align-shadow-canon`・`choice-marker-styling`・`font-size-keywords`／プロパティの残り＝`currentghost-property-others`・`system-property-values`・`property-catalog-lists`・`zorder-property`・`property-query-channels`・`property-ipc-transport`／台本と SHIORI＝`sakura-time-directives`・`sakura-time-critical`・`sakura-embed-directive`・`makoto-dll-host`・`makoto-reload-directives`・`check-script-arg-checks`・`ondestroy-on-exit`（2026-10-22 棚卸㉘で起票・`mcp-reload` の残り物）／その他＝`extra-character-windows`・`update-check-options`・`network-update-canon-order`・`balloon-canon-residue`・`elevated-drop-filter`・`surfaces-basepos`・`release-code-signing`・`dpi-transition-two-tick-bounce`・`zorder-chain-residue`・`emo-text-canon-residue`（取り下げ予定）・`install-package-type`・`sstp-receive`・`balloon-inline-image`（この 3 本は 2026-10-23 棚卸㉙で覚え書きから格上げ）・`seriko-missing-surface-default`（2026-10-24 `descript-window-keys` の裁定 11 で起票）・`mouse-hover-event`（2026-10-24 `dpi-realign-remembered-chain` の実機の確かめで起票・版は次の棚卸で決める）・`seriko-legacy-pattern-format`（2026-10-30 `seriko-pattern-calls` の要件の討議で起票・旧 SERIKO 1.x の書式・棚卸㉛でここに置いた＝ぱすたと手元の検体の旧書式の行は 0）＝**35 本**（2026-10-31 棚卸㉛で `seriko-legacy-pattern-format` を足した。2026-10-31 `balloon-scroll-arrows` の完了の棚卸で `balloon-append-tags` を足した。2026-10-24 `mouse-hover-event` を足した。2026-10-24 `descript-window-keys` の裁定 11 で `seriko-missing-surface-default` を足した。2026-10-23 に `shiori4-api` を v0.2.0 へ移した。`release-code-signing` は配布と公開の後続だが任意＝外の申請〔SignPath〕が要るのでここに残す） |
| **windows-reactor 待ち** | WinUI3 に任せる設定系・タスクトレイ系・インストーラー | `tasktray-icon` → `tasktray-ghost-icon`・`tasktray-balloon`・`tasktray-minimize`＝**4 本**。予約の「設定画面と設定ファイル」「インストーラー版」もここで待つ。`popup-menu-extra-resources` の ⑵（消滅通告・おすすめ・ポータルの項目をメニューに足す）もここで待つ（2026-10-31 開発者「メニュー関係はreactor解放されたら大改造するので、内容によっては後回し」＝⑴ は C15 で済ませた。⑵ は 2026-11-01 の完了の棚卸でこの段へ渡した＝正本は `completed/popup-menu-extra-resources/brief.md` の「完了の棚卸」の節・8 つの名前と `char*.recommend*` の 3 つ・`readmebuttoncaption`・サイトの一覧の読み方と開く道・`popup-menu-shell-look` の件 15 との分け方・数には足さない）。開発者「タスクトレイと設定画面は、windows-reactor のアプリ化仕様が下りてくるのを待とうと思っています。WinUI3 で設定系・タスクトレイ系を任せられれば、テーマ対応などが何も考えずにできるので。現状のままヘタに設定を進めようとすると問題が出る」「現在の zip インストーラーは当面維持するとして、wix かな？ windows-reactor が出来るころには WinUI3 のランタイムをインストールできるように整備する必要がある。これも reactor 待ち」 |
| **版に縛らない（道具と手入れ・測ること）** | 席が空いたときに挟む（v0.2.0 の間は、その大規模改造の影響を受けず、改造の後にやるべきでないものだけ） | 道具と手入れ＝✅ `impl-watch-skills`・✅ `impl-watch-residue`・✅ `mcp-stdio-bridge-residue`・`dev-folder-alias`・`test-roots-under-target`・✅ `dev-helper-x64-clobber`・✅ `perf-tools-console-encoding`・✅ `actor-thread-log-capture`（10-31 着地）・`clippy-199-lints`・`ukadoc-catalog-refresh`（2026-10-22 `/kiro-discovery` で起票・`script-security-level` の要件討議の議題 3・`coverage-roadmap-refresh` の前に＝写しを新しくしてから数え直す方が安い）・`coverage-roadmap-refresh`・✅ `reflow-scroll-path-test`・✅ `choice-ranges-one-function`（10-31 着地）・✅ `popup-menu-residue`（10-31 着地）・✅ `popup-menu-extra-resources`（11-01 着地・⑴ だけ・⑵ は windows-reactor 待ちの段へ渡した）・`alert-dialog-per-monitor-dpi`（2026-10-31 `install-before-boot` の完了の棚卸で起票・告知の画面の DPI とモニタを移した後のバルーン・区分 B）・`emo-bake-log-overlap`（2026-10-31 `placement-measure-bake-once` の完了の棚卸で起票・採寸と資産の組み立てで焼く段・畳み込みの記録が 2 行ずつ出る残り）・`config-parse-diagnostics`・`background-result-tick-wake`（2026-10-17 `user-data-root` の完了の棚卸で起票・背景の結果で起床の旗を立てる・`tick-gate-adoption` の前に）／測ることが本体（静かな机で 1 本ずつ）＝`present-emit-tail-latency`・`install-live-target-hazards`・`real-run-input-guard-observations`（2026-10-16 `balloon-text-bugs` の完了の棚卸で起票・再現が本体）・`helper-first-load-refusal`（2026-10-24 `shiori2-protocol` の完了の棚卸で起票・再現が本体）／文字の寸法＝`emo-text-line-gap`（2026-10-24 `vertical-choice-hover-band` の完了の棚卸で起票・行間を持つ書体で横書きの帯がずれる見込み・ぱすたのバルーンは縦書きだけ＝v0.1.0 のゴールに当たらない）／絵の読み＝✅ `emo-atlas-palette-alpha`（10-31 着地・2026-10-30 `real-run-log-noise` の完了の棚卸で起票・パレットの PNG の `tRNS` を使わない件・区分 B）（ここまで 28 本・✅ の 6 本は 10-16 着地・✅ `emo-atlas-palette-alpha`・✅ `choice-ranges-one-function`・✅ `popup-menu-residue`・✅ `actor-thread-log-capture` は 10-31 着地＝残り 18）／着手済みのテーマの後続のシェル内バルーン＝`balloon-font-file`・`text-typesetting` → `text-ruby`・`talk-fast-forward`・`balloon-markers`・`balloon-element-order`／バルーンのリンクと OS の連携＝`range-choice-tag` → `link-context-copy` → `balloon-link-hover`・`shell-tooltip`／同梱バルーンの正典＝`shell-companion-balloon`（前の `ghost-inner-balloon` は 2026-10-18 棚卸㉗で v0.1.0 へ上げた）／MCP の areka 独自ツール＝`mcp-shiori-query`・`mcp-user-response`・`script-impact-tiers`（どれも v0.1.0 の `script-security-level` の後）／SERIKO の引き金＝`seriko-script-triggers`・`seriko-interval-combinations`／持ち越し＝`balloon-break-position`・`anchor-style-canon`（後続は 19 本・2026-10-16 に v0.1.0 から移した・途中で止めても困らないもの）／着せ替えのイベント＝`dressup-change-events`（2026-10-31 `script-small-tags` の要件の討議で起票・棚卸㉜でこの列に置いた＝ぱすたは受け手を持たず SSP にも 204 を返す・kanade の `schedule/events.rs` を `shell-scale-linkage`・`balloon-communicatebox` と分け合う＝早くて C17）＝**33 本**（2026-11-01 `popup-menu-extra-resources` の完了で 1 本減った。2026-10-31 棚卸㉜で、置き場の決まっていなかった `dressup-change-events` を足した。2026-10-31 `install-before-boot` の完了の棚卸で起票した `alert-dialog-per-monitor-dpi` を足した。2026-10-31 `placement-measure-bake-once` の完了の棚卸で起票した `emo-bake-log-overlap` を足した。2026-10-31 `actor-thread-log-capture` の完了で 1 本減った。2026-10-31 `popup-menu-residue` の完了で 1 本減り、完了時の棚卸で起票した `popup-menu-extra-resources` を足した＝数は変わらない。2026-10-31 `choice-ranges-one-function` の完了で 1 本減った。2026-10-31 `emo-atlas-palette-alpha` の完了で 1 本減った。2026-10-30 `real-run-log-noise` の完了の棚卸で `emo-atlas-palette-alpha` を足した。2026-10-24 `vertical-choice-hover-band` の完了の棚卸で `emo-text-line-gap` を足した。2026-10-24 `shiori2-protocol` の完了の棚卸で `helper-first-load-refusal` を足した。2026-10-23 棚卸㉙で `real-run-log-noise` を v0.1.0 へ上げた。42 本のうち ✅ の 6 本は 10-16 着地。2026-10-18 棚卸㉗で `impl-watch-test-desk`・`sample-url-fetch`・`areka-test-binary-stability`・`ghost-inner-balloon` の 4 本を v0.1.0 へ上げた。2026-10-22 `/kiro-discovery` で `ukadoc-catalog-refresh` を足した） |

- **v0.1.0 の版の門の最後の確かめ**: 兄弟の spec の着地の後に `doc/pasta-real-run.md`（ぱすたの実機の手順）を回す。そのための別の spec は起こさない（2026-10-22 `pasta-in-windows-verification` の要件の話し合い）。
- **数**: 5＋2＋39＋5＋25＋4＋6＋3＋1＋35＋4＋33 ＝ **162**（2026-11-01 `popup-menu-extra-resources` の完了で版に縛らない列が 1 本減った・完了時の棚卸＝その場で解決 0 件・起票 0 本・既存の brief への追記 0 件・「直接修正候補」への登記 1 件〔`tools/mouse-input.ps1` の 1 画素のずれ〕・⑵ は同 spec の brief の残りとして windows-reactor 待ちの段へ渡した〔数には足さない〕・brief を持つ 162 本と一致）。その前の 163（2026-10-31 棚卸㉜で、置き場の決まっていなかった `dressup-change-events` を版に縛らない列へ置いた＝brief を持つ 163 本と一致・起票 0 本）。その前の 162（2026-10-31 `script-small-tags` の完了で v0.1.0 の列が 1 本減った・完了時の棚卸＝その場で解決 1 件（出ている箱も面の切り替えで変わった印を使う）・起票 0 本・既存の brief への追記 0 件・列の合計 162＋版の置き場の決まっていない 1 本〔`dressup-change-events`・10-31 `script-small-tags` の要件の討議の議題 3 で起票・版の置き場は次の棚卸〕＝brief を持つ 163 本と一致）。その前の 163（2026-10-31 `install-before-boot` の完了で v0.1.0 の列が 1 本減り、完了時の棚卸の起票 1 本＝`alert-dialog-per-monitor-dpi`（区分 B・版に縛らない列）を足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 2 件・起票 1 本・既存の brief への追記 1 件〔`areka-test-binary-stability` の件 5〕・brief を持つ 163 本と一致）。その前の 163（2026-10-31 `placement-measure-bake-once` の完了で v0.1.0 の列が 1 本減り、完了時の棚卸の起票 1 本＝`emo-bake-log-overlap`（区分 C・版に縛らない列）を足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 0 件・起票 1 本・既存の brief への追記 0 件・brief を持つ 163 本と一致）。その前の 163（2026-10-31 `actor-thread-log-capture` の完了で版に縛らない列が 1 本減った・完了時の棚卸＝その場で解決 0 件・起票 0 本・既存の brief への追記 0 件・brief を持つ 163 本と一致）。その前の 164（2026-10-31 `popup-menu-residue` の完了で版に縛らない列が 1 本減り、完了時の棚卸の起票 1 本＝`popup-menu-extra-resources`（同 spec の件 13・区分 C・メニューの spec が置かれていたのと同じ版に縛らない列）を足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 1 件（台帳の `char*.popupmenu` の備考の言い方）・起票 1 本・既存の brief への追記 0 件・brief を持つ 164 本と一致）。その前の 164（2026-10-31 `choice-ranges-one-function` の完了で版に縛らない列が 1 本減った・完了時の棚卸＝その場で解決 0 件・起票 0 本・既存の brief への追記 0 件・brief を持つ 164 本と一致）。その前の 165（2026-10-31 `emo-atlas-palette-alpha` の完了で版に縛らない列が 1 本減った・完了時の棚卸＝その場で解決 1 件・起票 0 本〔既存の brief への追記 2 件＝`areka-test-binary-stability` の件 3〔wintf の lib のテストの AV〕・件 4〔ghost_switch の足場の remove_dir の揺れ〕〕・brief を持つ 165 本と一致）。その前の 166（2026-10-31 棚卸㉛＝roadmap に載せていなかった `seriko-legacy-pattern-format` をいつかの列へ足した・起票 0 本・brief を持つ 166 本と一致）。その前の 165（2026-10-31 `animated-surface-log-flood` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 3 件（`execute_reusing` の二重の書き込み・設計の File Structure Plan・main を取り込んだときの `plan.rs` の競合と `seriko-interval-combinations` への申し送り）・起票 0 本〔既存の brief への追記 0 本・実装中に既存の brief へ足した 3 件は「animated-surface-log-flood の申し送り」節〕。brief を持つ数は 166 本で合わない＝`seriko-legacy-pattern-format` を roadmap に載せていない件〔次の 167 の行〕のままで、次の棚卸で数え直す）。その前の 166（2026-10-31 `stay-on-top-directives` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 1 件（案 A の前の `HWND_TOP` の書き方を spec の文書で改めた）・起票 0 本〔既存の brief への追記 0 本〕。brief を持つ数は 167 本で合わない＝`seriko-legacy-pattern-format` を roadmap に載せていない件〔次の 167 の行〕のままで、次の棚卸で数え直す）。その前の 167（2026-10-31 `balloon-scroll-arrows` の完了で v0.1.0 が 1 本減り、完了時の棚卸の起票 1 本＝`balloon-append-tags`（`\C` と `\*`・区分 C・規模が spec 1 本に相当・版の置き場は次の棚卸で）をいつかの列へ足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 0 件・起票 1 本・既存の brief への追記 0 本。brief を持つ数は 168 本で合わない＝`seriko-legacy-pattern-format` を roadmap に載せていない件〔次の 167 の行〕のままで、次の棚卸で数え直す）。その前の 167（2026-10-31 `seriko-pattern-calls` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 0 件・起票 0 本〔既存の brief への追記 0 本〕。brief を持つ数は 168 本で合わない＝同 spec の要件の議論〔議題 1〕で 2026-10-30 に起票した `seriko-legacy-pattern-format`〔旧 SERIKO 1.x の書式・+1〕は、その brief の決め「roadmap.md はこの起票では触らない」で roadmap に載せておらず、次の棚卸で数え直す）。その前の 168（2026-10-30 `bridge-store-user-root` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 0 件（最後の検証の指摘は完了の前に直した）・起票 0 本〔既存の brief への追記 0 本〕・brief を持つ 168 本と一致）。その前の 169（2026-10-30 `window-work-area-clamp` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 1 件・起票 2 本〔どちらも次の 170 の行で足した〕・brief を持つ 169 本と一致）。その前の 170（2026-10-30 `window-work-area-clamp` の完了時の棚卸の起票 2 本＝`balloon-flip-side-rules`（反対の側へ並べ替えたバルーンが相方の窓に重なりうる・右に出すバルーンの横のずらしの向き・区分 B）と `window-resetwindowpos-hold`（起動の直後の `\![execute,resetwindowpos]` が最初の表示を待たない・区分 B）を v0.2.0 へ足した〔v0.2.0 は 23＋2＝25〕・完了時の棚卸＝その場で解決 1 件（`real-run-kit` が起こす子へ `PASTA_DEBUG=0` を渡し、ほかのぱすたが握る 9276 とぶつからないようにした）・起票 2 本・同 spec の完了〔v0.1.0 −1〕はまだ載せていない・brief を持つ 170 本と一致）。その前の 168（2026-10-30 `emo-text-dwrite-fidelity` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 5 件・起票 0 本〔既存の brief への追記 1 本＝`balloon-font-file`（ＭＳ ゴシックで作り直した後も計測と描画が作り直す前の顔を覚えている件と、FontCatalog の口の変更の申し送り）〕・brief を持つ 168 本と一致）。その前の 169（2026-10-30 `startup-install-option` の完了で v0.1.0 が 1 本減り、完了時の棚卸の起票 2 本＝`install-before-boot`（`--install` と `initial_install.nar` を起動の解決の前に入れる・同 spec の要件 10.2・区分 A）を v0.1.0 へ、`ghost-switch-balloon-fallback`（切替先にも置き場にもバルーンが無いと切り替えが落ちる・区分 B）を v0.2.0 へ足した〔v0.1.0 は 15−1＋1＝15、v0.2.0 は 22＋1＝23〕・完了時の棚卸＝その場で解決 1 件（フォルダ名の無いゴーストでも同じ場所への切り替えの受理を数えない）・起票 2 本〔既存の brief への追記 2 本＝`helper-first-load-refusal` の件 4（YAYA のゴーストでも補助プログラムの `loadu` の後に `HRESULT(0x80004005)`）・`ghost-call`（`callghost` を 2 体目の呼び出しへ移す申し送り・要件 10.1）〕・brief を持つ 169 本と一致）。その前の 168（2026-10-30 `mcp-strict-errors` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 1 件（emo-text の `look.rs` のコメントに紛れた 0x0C の制御バイトを `\f` に戻した）・起票 0 本〔既存の brief への追記 0 本〕・brief を持つ 168 本と一致）。その前の 169（2026-10-30 `real-run-log-noise` の完了で v0.1.0 が 1 本減り、完了時の棚卸の起票 1 本＝`emo-atlas-palette-alpha`（パレットの PNG の `tRNS` を使わない件・区分 B・起票の条件は開発者の裁定〔要件の討議 8〕）を版に縛らない列へ足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 1 件（emo-compose の方式の 7 語に emo-atlas の写しを指す注）・起票 1 本・既存の brief への追記 0 本・brief を持つ 169 本と一致）。その前の 169（2026-10-30 `shell-mouse-cursor` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 1 件（シェルの `.ani` を読む正の道をテストで固定した）・起票 0 本・brief を持つ 169 本と一致）。その前の 170（2026-10-30 `default-balloon-selfmade` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 1 件（verification の在処の表に要件 11.6 の行を足した）・起票 0 本〔既存の brief への追記 1 本＝`text-typesetting`（折り返した後の行頭の空白・数字の並びが割れる）〕・brief を持つ 170 本と一致）。その前の 171（2026-10-28 `/kiro-discovery`〔本体の crate 分け〕で 6 本を v0.1.0 の後の隙間へ足した〔33 → 39〕・既存の brief への追記 9 本〔本体を受け持つ audit 8 本の Upstream・`multi-ghost-runtime` の Adjacent〕・同じ日の〔利用統計とノベルゲーム基盤〕の起票と合わせて brief を持つ 171 本と一致）。その前の 165（2026-10-28 `/kiro-discovery`〔利用統計とノベルゲーム基盤の 16 案〕で 5 本を起票した＝v0.2.0 に `doc-tree-tidy`、新しい段「ノベルゲーム基盤」に `sound-canon-tags`・`ghost-material-metadata`・`shell-multi-compose`、新しい段「利用統計」に `areka-usage-stats`・既存の brief への追記 5 本〔`extra-character-windows`・`parallel-visual-script`・`sakura-time-directives`・`property-catalog-lists`・`install-package-type`〕・2026-10-25 の `talk-script-metadata`〔v0.2.0 +1〕はこの行に書き足されていなかった＝この数えで載せた・brief を持つ 165 本と一致）。その前の 159（2026-10-24 `mouse-click-wheel-events` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 3 件（kanade の clippy を止めていた `never_loop`・wintf の `PointerState` の押下の欄の説明・sample-ghost-kit の検体の登記は main の取り込みで解消）・起票 0 本〔既存の brief への追記 0 本〕・brief を持つ 159 本と一致）。その前の 160（2026-10-24 `/kiro-discovery` で v0.1.0 に `emo-text-dwrite-fidelity` を 1 本起票した〔既存の brief への追記 1 本＝`text-typesetting`（括弧の詰め）〕・brief を持つ 160 本と一致）。その前の 159（2026-10-24 `readonly-mode` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 2 件（`input_events/file_drop.rs` の注・main を取り込んだ後の試験の初期化子）・起票 0 本〔既存の brief への追記 0 本〕・brief を持つ 159 本と一致）。その前の 160（2026-10-24 `dpi-realign-remembered-chain` の完了で v0.1.0 が 1 本減り、同 spec の実機の確かめで起票した `mouse-hover-event` を「いつか」へ足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 0 件・起票 0 本・既存の brief への追記 1 本〔`window-work-area-clamp` の件 3 への足し〕・実機の確かめの中の追記〔同 brief の件 2 の裁定と件 5〕・brief を持つ 160 本と一致）。その前の 160（2026-10-24 `vertical-choice-hover-band` の完了で v0.1.0 が 1 本減り、完了時の棚卸の起票 1 本＝`emo-text-line-gap`（行間を持つ書体で横書きの帯がずれる見込み・裁定と実物の書体の読み戻しが要る）を版に縛らない列へ足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 1 件（tests/ の clippy の deny 3 本）＋最終検証の指摘 4 件・起票 1 本・既存の brief への追記 1 本〔`window-work-area-clamp` の件 4＝バルーンだけが作業領域へ寄せられ、ぱすたの × が縦書きの 1 列目にかぶる〕・brief を持つ 160 本と一致）。その前の 160（2026-10-24 `descript-window-keys` の完了で v0.1.0 が 1 本減り、完了時の棚卸の起票 1 本＝`seriko-missing-surface-default`（タスク 8.3 の `/kiro-discovery`・裁定 11）をいつかの列へ足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 4 件（窓配置の手本の最初の面・古い注記・台帳の注記・この roadmap の「36 件の登記」→ 38）・起票 1 本・既存の brief への追記 2 本〔`helper-first-load-refusal`・`areka-test-binary-stability`〕・brief を持つ 160 本と一致）。その前の 160（2026-10-24 `shiori2-protocol` の完了で v0.1.0 が 1 本減り、完了時の棚卸の起票 1 本＝`helper-first-load-refusal` を版に縛らない列へ足した〔合計は変わらない〕・完了時の棚卸＝その場で解決 0 件・起票 1 本・既存の brief への追記 3 本〔`popup-menu-residue` の件 13・`draw-methods-canon`・`default-balloon-selfmade`〕・brief を持つ 160 本と一致）。その前の 160（2026-10-23 `mcp-scope-rect-property` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 1 件（`get_property` のテストの冒頭の説明を 4 本に直した）・起票 0 本〔実機で見つけた範囲外の 2 件は既存の `dpi-realign-remembered-chain` の件 9 と `descript-window-keys` の件 4 に既に在る・既存の brief への追記 0 本〕・brief を持つ 160 本と一致）。その前の 161（2026-10-23 棚卸㉙＝0 本で公開済みの v0.0.3 の行を表から外し、`real-run-log-noise` を版に縛らない列から v0.1.0 へ上げ〔合計は変わらない〕、4 本を起票した〔覚え書きから格上げの 3 本＝いつか、完了 spec の残り物 1 本＝`ghost-alias-file`・v0.2.0〕・brief を持つ 161 本と一致）。その前の 157（2026-10-23 `pasta-in-windows-verification` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 5 件（崩れ 1・2 の持ち主をぱすたの側へ〔`ghost-inner-balloon` #364 が範囲の外とし、ぱすた開発が書庫の install.txt を直した＝ghost_dev `c32229e`・未リリース〕・`ssp-measurement` の SSP の縦書きの出どころの誤り・バルーンの置き場の書き違い・手順書の版・5.6 の触るファイル）・起票 0 本〔既存の brief への追記 1 本＝`animated-surface-log-flood`（件 25・動く絵のコマを先にアトラスへ展開する方向）〕・brief を持つ 157 本と一致）。その前の 158（2026-10-23 `pasta-in-windows-verification` の枝に既定ブランチを取り込んだ後の数え直し＝既定ブランチの 152 に、同 spec の申し送りで v0.1.0 に足した 6 本〔タスク 5.3 の `/kiro-discovery` の 5 本＝`mcp-scope-rect-property`・`animated-surface-log-flood`・`shell-scale-linkage`・`balloon-scroll-arrows`・`balloon-communicatebox`、崩れた所の表の 23 の `/kiro-discovery` の `vertical-choice-hover-band`〕を合わせた＝152＋6・brief を持つ 158 本と一致）。その前の 152（2026-10-23 `shiori4-api` を「いつか」から v0.2.0 へ移した〔合計は変わらない〕。その前は 2026-10-23 `offline-surface-dump` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 1 件（`surface-compare` の読めないフォルダの行に道を入れた）・起票 0 本〔既存の brief への追記 1 本＝`animated-image-import`（1 枚目の待ちが 0 の動く絵の経過 0 のコマ・区分 B の件）〕・brief を持つ 152 本と一致）。その前の 153（2026-10-23 `ghost-inner-balloon` の完了で v0.1.0 が 1 本減った・完了時の棚卸＝その場で解決 1 件・起票 0 本〔既存の brief への追記 1 本＝`install-live-target-hazards`〕・brief を持つ 153 本と一致）。その前の 154（2026-10-23 steering の同期で数え直した＝`dot-sakura-specimen` の完了〔v0.1.0 −1〕と同 spec の完了時の起票 2 本〔`shiori2-protocol`・`seriko-pattern-calls`・v0.1.0 +2・どちらの brief も開発者の条件「次のリリースまでに治ること」〕を載せた・brief を持つ 154 本と一致）。その前の 153（2026-10-23 `script-security-level` の完了で v0.1.0 が 1 本減った・完了時の起票 0 本〔既存の brief への追記も 0 本〕。brief を持つ数は 154 本で合わない＝2026-10-23 `dot-sakura-specimen` の完了〔−1・行はまだ v0.1.0 に在る〕と同 spec の起票 2 本〔`shiori2-protocol`・`seriko-pattern-calls`・+2〕は開発者の指示で roadmap に載せておらず、次の棚卸で数え直す）。それより前の数えの経緯は history「2026-10-23 完了済みの退避」。
- **組済みのウェーブ**: **C15（2026-10-31 棚卸㉜・6 本）＝今の版 v0.1.0 の 1 本（`shell-scale-linkage`）＋割り込み 5 本（`emo-text-line-gap`・`alert-dialog-per-monitor-dpi`〔件 1 だけ〕・`test-roots-under-target`〔入口の差し替えだけ〕・`emo-bake-log-overlap`〔案 1・`placement/mod.rs` に触らない〕・✅ `popup-menu-extra-resources`〔⑴ だけ・11-01 着地〕）**（「ウェーブ編成」の表）。**v0.1.0 の着地まで約 2 ウェーブ（C15〜C16）**（`emo2_boot/consumer_ledger.rs` に行を足す残りの 2 本を 1 本ずつ）。C14 の 7 本は 10-31 に着地した。C14 より前のウェーブの記述は history「2026-10-31 棚卸㉜退避」。
- **版をまたぐ依存の確かめ**（2026-10-15・後続を v0.1.0 へ移した後）: 後の版の spec が前の版の spec を待つ形だけ（逆向きは 0）。`hit-id-pass`（v0.2.0）は `collisionex-regions`（v0.1.0）と `wintf-d2d-composite-rules`（v0.2.0）の後。`ghost-inner-balloon`（v0.1.0）はカタログの鎖で `write-failure-notice`（v0.0.3）の後。`currentghost-property-tree`（v0.3.0）は ✅ `property-name-case-fold`（v0.1.0）の後。`wintf-d2d-composite-rules` の brief は `element-clipping-option` を下流に挙げる（`--clipping` を D2D の切り抜きで描く形）＝`element-clipping-option` は今の合成で `--clipping` を描き、D2D へ移すときは `draw-methods-canon` が一緒に移す（要件の段で D2D を待つと決めたら v0.2.0 へ戻す）。ファイルの順だけの前後（`anchor-style-canon` が列で `text-align-shadow-canon`・`choice-marker-styling`〔いつか〕の後ろに居る）は、列の中の順を入れ替えて済ませる（働きの依存ではない）。
- **版をまたぐ依存の確かめ（2026-10-16・v0.1.0 を軽くした後）**: 逆向き（前の版が後の版や版に縛らない列を待つ）は 0。v0.1.0 の 19 本の依存は v0.0.3 か v0.1.0 の中か着地済みだけ（`animated-image-import` は `extent-element-offset`〔v0.1.0〕とファイルが重なる）。`collisionex-regions` → `hit-id-pass` は v0.2.0 の中に収まった。版に縛らない列の後続は v0.1.0 の spec（`script-security-level`・`mcp-kanade-tools`・`extent-element-offset`）や v0.0.3 の spec を待つだけ＝列から先に取ってもよいが、待つ相手が着地してから。`currentghost-property-tree`（v0.3.0）は ✅ `property-name-case-fold`（v0.1.0）の後のまま。下の行（10-15）の `element-clipping-option` の扱いは、v0.2.0 へ戻すほうに決めた。
- **版に縛らない列を v0.2.0 の間に挟むときの注意**（2026-10-16 開発者・`/kiro-discovery`「0.2.0 に差し込む際には、0.2.0 が行う大規模改造の影響を受けるか、改造後に実施すべきものは挟み込まないようにしてほしい」）: v0.2.0 は wintf・dola の基幹（D2D の合成の決まり・描画メソッド・当たり判定・合成の精度・dola の割り込みの刻み・時刻の土台）を作り直す。v0.2.0 の着手から着地までの間に版に縛らない列から取るのは、**その改造が触る場所（触るファイル・合成と描画の道・dola の時刻と割り込み・当たり判定）に触れず、改造の後に作り直しにならないもの**だけ。触れるもの・改造の後の形に乗るべきものは、v0.2.0 の着地まで待たせる。挟む前に、取る spec の brief（設計があれば設計）の触る場所を v0.2.0 の残りの spec と突き合わせ、重なりが 0 と示してから並べる。迷ったら挟まない。

## 実装規律（balloon-system の失敗から得た正）

- **実装ファースト**: 各作業ユニットの成果物は「実際に動く」検証済みコード。**spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1 ユニット＝1 かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: 使う分だけ実装し、拡張は型/レジストリの口だけ残す。抽象は「2 例目の実物」が要求してから。動く資産から建てる。
- **粒度基準**: 1 ユニット＝単一 pass/fail の独立観測。純粋層は fixture/mock 直入力で切る。UI 位置決め・座標系は本番ゴースト（実 emo2）＋実 DPI（≠96）が観測条件（記憶 areka-placement-real-ghost-first）。
- **語彙完備・配線ゼロの追跡**: 先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目。
- **1 ファイル 1,000 行**: 機械の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 10 件・暗黙増加不可・**どの spec も例外表に触れない**）。2026-10-23 棚卸㉙の実測で上限の近く（本番のファイル）: `crates/areka/src/placement/transition_judge.rs` 996・`crates/areka-emo-text/src/surface.rs` **995（もう足せない）**・`crates/areka/src/main.rs` 978・`placement/follow/drag_follow.rs` 958・kanade の `schedule/steady.rs` 957・`emo2_boot/spine.rs` 940・`placement/mod.rs` 939・seriko の `looper.rs` 924・`emo2_boot/ghost_switch.rs` 921・`ghost_session.rs` 917・seriko の `table.rs` 901（`placement/follow/window_move.rs` 1,221 は例外表に載る）。kanade の `schedule/mod.rs` は 673・`msg.rs` 544・kanade の `actor.rs` 845・`emo2_boot/consumer_ledger.rs` 496・`emo2_boot/mod.rs` 789 に下がった（中のテストを兄弟のファイルへ出した）。テストの兄弟ファイルにも 950 行を超えるものが多い（`measure_tests.rs` 993・`actor_choice_contract_tests.rs` 991・`ghost_switch_tests.rs` 990・`runtime_tests.rs` 986・`choice_tests.rs` 967 ほか）。足すときは新しいファイルへ。
- **決定論テスト網羅は必達**・**ログ無し失敗経路の禁止**・**終了経路は正規実装**（記憶 deterministic-test-coverage-mandate／areka-log-first-no-silent-failure／canonical-not-minimal-lifecycle）。
- **外部依存の追加は `tech.md` へ「意図的依存追加」を登記し開発者が承認する**（`encoding_rs` の前例）。α で登記したのは **`miniz_oxide` 0.9（`nar-install`・2026-09-18 承認済・推移的依存は `adler2` 1 本。2026-10-04 に `mcp-dump-images` が PNG の圧縮でも使うことを承認）**、`md-5` は要らなくなった（`update-engine` 2026-09-24 完了＝MD5 は OS の CNG・`tech.md` 登記済み）。HTTP は WinHTTP（`windows` crate の機能フラグ）で crate を足さない。

## アーキテクチャ横断原則（要約・詳細は history＋記憶＋completed spec）

- **シェル/バルーン統一**: 描画エンジンはシェルとバルーンを区別しない。バルーン＝surface 上の文字レンダリング層。
- **アニメエンジンは 2 つ**: ①さくらスクリプト再生（sakura）＋②SERIKO ループ（seriko）。両者とも dola（絶対時刻台本）上。
- **並行モデル**: 各エンジン＝チャンネル通信のアクター＋独立スレッド。render/window は UI スレッド固定。機構=areka-actor／経路=kanade／結線=ghost。
- **emo 合成**: 自前コンポジタ（アトラス→1 枚物合成→wintf へ完成品のみ）。emo=UI 層全般。**画像は原寸で持ち拡大縮小は D2D 変換行列**（2026-09-11 裁定・`present-gpu-transform-scale` が実装）。
- **DPI 追従が基本設計**: k=monitorDPI÷author_dpi で全表示経路がスケール。キャラ窓の原点は下端中央。バルーン追従は例外＝「窓（char 左上）相対」・offset の単位空間は表示 DPI の物理 px 1 つ。
- **スコープ窓の重なり**: 既定は非強制、`\![set,zorder]` 後は Windows の所有関係（owner）の一直線の鎖で構造保証。
- **意味論は ukadoc から輸入・SSP 実測主義は取らない**（記憶 no-ssp-measurement-import-semantics-from-ukadoc）。**`\!` コマンドは汎用キャリア 1 本**（typed 個別新設禁止・消費側 name 選別）。
- **アプリの寿命は窓の数から切り離す**（2026-09-18・`ghost-shell-balloon-switch` が実装）: 切替中に窓が 0 になっても終了しない。終了は明示の `AppExit`。
- **ベースウェアの根は 1 つ**（2026-09-18・`baseware-root-layout` が実装）: `<根>/ghost/<名>/`・`<根>/balloon/<名>/`＝ukadoc「全体の構成」の格納フォルダ。`nar-install` の展開先もこの形。

**エンジン固有名**（コード/spec/会話の参照はこの名で統一）:

| # | エンジン | 固有名 | # | エンジン | 固有名 |
|---|---|---|---|---|---|
| ⓪ | ゴーストエンジン（最上位 owner） | `ghost` | ④ | さくらスクリプト再生 | `sakura` |
| ① | SHIORI 通信層 host-32 | `shiori` | ⑤ | SERIKO アニメ | `seriko` |
| ② | parser/loader | `parsers` | ⑥ | render（surface 合成＋UI 層） | `emo` |
| ③ | conductor（SHIORI イベント循環） | `kanade` | | | |

## 完了サマリ（詳報は各 `completed/` spec ＋ history）

| Wave | 完了日 | ユニット（PR） | 一行要約 |
|---|---|---|---|
| 耐力壁 | 07-01 | `pilot-shiori-host-32` | x64→32bit pasta.dll 駆動 GO |
| M-boot | 07-13 | `emo2-boot` 23/23 | 起動→表示→talk→close の可視一周 |
| W1〜W4 | 〜07-29 | idle-talk／collision-geometry／sakura-dialogue-tags／input-events／mayuna-compose／sylphya／seriko-loop／choice-render／position-persist／choice-interact／emo-dpi-scaling 他 | 増分ウェーブ全完走・DPI 追従 k 着地 |
| W5 | 08-01 | `choice-select-events`・`kero-balloon`（PR#97）・`dpi-window-vanish`（PR#98） | M-dialogue 完走 |
| W6 | 08-10 | `file-slimming`（PR#103） | 1,000 行超 54→0 |
| W7 | 08-13 | `collision-dpi-hittest`（PR#100）・`balloon-visibility`（PR#106）・`bindoption-exclusivity`（PR#105）・`ghost-window-zorder`（PR#107）・`scope-chain-gap`（PR#108） | 5/5 |
| W8 | 08-15 | `recompose-budget`（PR#112）・`scale-exact-rational`（PR#110）・`windowposition-limit`（PR#111） | 1 コマ 22,210→1,240µs |
| W9 | 08-22 | `dpi-transition-atomicity`（PR#114） | 書込散らばり ms→µs |
| W10 | 08-27 | `draw-load-parity`（PR#118）・`test-cage-determinism`（PR#119） | 門の手がかり→`tick-gate-adoption`／log-capture-kit |
| W11 | 09-02 | `present-write-coherence`（PR#123）・`balloon-vertical-canon`（PR#124）・`balloon-offset-dpi`（PR#125）・`scope-zorder-pinning`（PR#126） | 4/4 |
| ⓪ | 09-03 | `sakura-bare-tag-lexer`（PR#134） | `\_X` bare 漏れ修正・完了検証で `sakura-tag-word-boundary` 起票 |
| W12 | 09-05〜09-11 | `cursor-tag-canon`（PR#137）・`emo-text-line-height-canon`（PR#142）・`emo2-conformance-e2e`（PR#143・**M1 完成宣言**） ∥ 調査系 `ukadoc-survey-toolkit`（PR#136）・`-property`（PR#138）・`-shiori`（PR#139）・`-sakura-script`（PR#140）・`-assets`（PR#141） | 行送り 35→30・e2e 20 項目全合格・ukadoc 1,749 項目の台帳 4 本＋`report/summary.md` |
| W13 | 09-12〜09-17 | `present-gpu-transform-scale`（PR#145）・`charset-canon`（PR#146）・`ukadoc-coverage-roadmap`（PR#147）・`text-decoration-canon`（PR#148）・`kanade-boot-talkdone-drop`（PR#149）・`sakura-tag-word-boundary`（PR#150）・`sylphya-set-ledger`（PR#151）・`host32-window-thread-pump`（PR#153）・`balloon-font-descript-keys`（PR#154） | 9/9・1 コマ 41〜78→2.65 ms・任意 charset・67 束の網羅ロードマップ・文字装飾 3 書字方向 |
| A0（α） | 09-19 | `nar-install`（PR#158）・`default-balloon-bundle`（PR#157）・`popup-menu-minimal`（PR#159）・`balloon-origin-outside-validrect`（PR#161） | `.nar` の読取と安全な展開・既定バルーン（Staysee・CC0）・Win32 標準の右クリックメニュー |
| A1（α） | 09-20〜09-24 | `shell-implicit-surface`（PR#162）・`balloon-break`（PR#164）・`app-lifetime-separation`（PR#169）・`shiori-loadu`（PR#170）・`default-balloon-nar-fold`（PR#171）・`wintf-drag-state-rest-contract`（PR#172）・`nar-install-hardening`（PR#174）・`keycolor-clickthrough-coverage`（PR#175）・`update-engine`（PR#176）・`baseware-root-layout`（PR#177） | 里々のテンプレートが絵を出せる・抜き色・根 `ghost/`・`balloon/` と列挙・寿命を窓の数から分離・更新のエンジン |
| B1〜B2（α） | 09-24〜09-26 | `pilot-balloon-asset-swap`（PR#182）・`shiori-fault-notice`（PR#183）・`ghost-restart-unit`（PR#184） | SHIORI の失敗の告知・同じプロセスでゴーストを起こし直す形 |
| B3〜B4（α） | 09-26〜09-28 | `alpha-package`（PR#190）・`pilot-dropfiles-on-wuc-window`（PR#191）・`ghost-shell-balloon-switch`（PR#192）・`ghost-change-name-resolution`（PR#194）・`session-mark-residue`（PR#195） | ゴースト切替・汎用の通知の入口 `KanadeMsg::RaiseEvent`・起動中の印（きれいに終わらなければ次は emo2 が `halt` を伝える）・配布 zip |
| B5〜B6（α） | 09-28〜09-30 | `frame-phases-after-exit`（PR#197）・`ghost-install`（PR#198）・`file-drop`（PR#201）・`balloon-color-emoji`（PR#202） | メニューと台本と投げ込みからのインストール・カラー絵文字と書記素クラスタ |
| B7〜B9（α） | 09-30〜10-02 | `network-update`（PR#203）・`shell-balloon-switch`（PR#205）・`balloon-reappear-short-talk`（PR#206）・`alpha-release-signoff`（PR#210・**α 完成宣言**） | ネットワーク更新・シェルとバルーンの切替・実機一周 13 項目 |
| C1 | 10-03〜 | `restart-chain-finalize-stall`（PR#214）・`choice-timeout-directive`（PR#215）・`drag-click-without-move`（PR#216）・`emo-text-file-split`（PR#217）・`release-package-versioned`（PR#218）・`mcp-server-core`（PR#219） | 初期配置の確定の見送りはゴースト待ち〔窓が無い・まだ一度も表示されていない〕を数えず、areka 自身の待ちだけを数える（起こし直しの後の `deferrals=600` の空鳴りを止めた）・`\![set,choicetimeout,時間]` を compile が読んで選択待ちへ渡す（`0`・負の値で時間切れなし・最後の指定が勝つ・読めない値は既定＋WARN）。再生層 dola は選択待ちの区切りを値で飛ばしも解きもしない・動かさないクリックで窓の位置を保存しない（wintf の累積器が開始を積んでいない終了を捨て、知らせの種を積んだ順の待ち行列で運ぶ。速いドラッグも開始 → 終了の順に届き、キャラクターは行き先へ置いてから保存）・文字とバルーンの 6 本と `region.rs` を振る舞いを変えずに役割で分けた（最大 571 行・新しい子 12 本＋`region_tests.rs`・`layout_inner` を分岐の種類ごとの関数へ・前後のテスト 9,371 本が名前も結果も一致・実機で 4 つの振る舞いを確認）・配布 zip を版入りの固定の名前（`areka-{版}-{x64\|arm64}.zip`）と隣の `.sha256` で x64・arm64 とも作る（`tools/package.ps1`＝旧名から改名・完成品は全段が緑のときだけ現れる・`-Check` の展開先は `target\package` の下で合格なら片付ける）・winget のリンク経由で起動しても exe の本当の場所から根・補助 exe・記憶の置き場を引く（`boot_config.rs` の `exe_location`・手元のマニフェストで実機確認）・新クレート `areka-mcp`（rmcp 3.5.0・無状態・JSON 単発・ツール 0 本・登録口・help・`Origin`／`Host` の検査）を `fn main()` の 1 行で立てる。既定の待受は 9801 → 9821 を SSP と早い者勝ち・使用中なら隣 +1〜+9（開発者裁定）。実機で Claude Code 2.1.283 が無状態版で `tools/list` を拒んだのを `ttlMs`／`cacheScope` で直した・SSP との差の一覧 18 行（直した 1）
| C2 | 10-03〜 | `mcp-tool-entrances`（PR#223）・`release-ci-workflow`（PR#224）・`crates-io-publish`（PR#225）・`translate-pipeline`（PR#226）・`shell-balloon`（PR#227）・`install-companion-canon`（PR#221・実施せず 3 本へ分解）・README の書き直し（PR#222） | MCP のツール 10 本を SSP 2.9.05 と同じ定義・同じ並びで出し（名前・必須の欄・型の誤りは `-32602`）、`ghost_name` を SSP と同じ文言で解決して UI スレッドの World へ届ける橋を立てた。 |
| C3 | 10-04〜 | `mcp-get-property`（10-04）・`install-companion-reading`（10-04）・`mcp-expression-table`（10-04）・`host32-testdll-marker-race`（10-04）・`animated-image-decode`（10-05）・`mcp-log-history`（10-05）・`shell-balloon-frame-align`（10-05）・`drag-cancel-borrow-miss`（10-05）・`surface-element-nesting`（10-05）・`mouse-drag-events`（10-05）・`mcp-dump-images`（10-05） | MCP の `get_property` が宛先のゴーストの記憶（sylphya）をそのゴースト自身の問い手で読み、値は素のまま（空の値も成功）・値の無い名前は SSP と同じ `NG:Cannot find such property name.` で答える（実行系の読み口 `GhostRuntime::sylphya_reader()` を 1 本足しただけ・名前は手直ししない）／書庫を入れるときの同梱のバルーンを ukadoc の探索の順（無印 → `balloon0` → `balloon1` …・`*.directory` の行の有無で数え、最初の欠番で打ち切る・先頭に 0 を付けた番号は引かない）で読み、`*.source.directory` を階層付きの相対パス（`\` と `/` のどちらでも・`..` と空の段は手前を打ち消さずに取り除いて記録・段が残らなければ断る）として読んで、段の数だけ剥がして置く。 |
| C4 | 10-05〜 | `element-base-method`（10-05）・`mcp-get-status`（10-06）・`open-external-tags`（10-06）・`self-alpha-declaration`（10-06）・`budoux-reveal-reflow`（10-06）・`mcp-dump-images-residue`（10-06）・`mcp-ghost-name-match`（10-06）・`tools-utf8-child-output`（10-06・バグの席）・`ghost-standard-balloon`（10-07）・`choice-script-prefix`（10-07）・`animated-image-playback`（10-07）・`char-position-save-on-exit`（10-08）・`mcp-author-tools`（10-08）・`balloon-lifecycle-events`（10-08）・`wintf-tooltip`（10-08）・`ghost-session-test-load-flake`（10-10） | element定義の描画メソッド `base` を `overlay` と同じに描く（読み手の語の集合を `is_image_element_method` の 1 関数で `overlay`・`base` の 2 語にしただけで、`Element` に欄は足していない・element番号・`surface.append*`ブレス・数字だけの欄・複数の番号の見出しでも同じ）。 |
| C5 の前（単独） | 10-10 | `farewell-talk-status`（10-10・バグ） | `farewell-talk-status`: 終了の挨拶・切り替えの送り出しの台詞・切り替えの別れの台詞の再生中に、実行の状態へ `talking`（中断の無効化モードなら `nouserbreak` も）を載せる。 |
| C5（⑥） | 10-10 | `ssp-bts-salvage`（10-10・文書だけ） | `ssp-bts-salvage`: SSP の課題管理（BTS・https://bts.shillest.net/ ）の全 808 件を通読し、深刻度「要望」187 件と、要望・質問・挙動の説明と読めた 47 件を、234 行の台帳にした（`.kiro/specs/completed/areka-P0-ssp-bts-salvage/bts-ledger.md`・13 主題）。 |
| C5（⑧） | 10-10 | `winget-manifest-submission`（10-10・配布と公開） | `winget-manifest-submission`: winget の名乗り `Areka.Areka.Portable` の版 0.0.2 のマニフェストの雛形を `dist/winget/0.0.2/` に置いた（zip＋portable・x64 と arm64・`ArchiveBinariesDependOnPath: true`・`InstallationNotes` に注意書き）。 |
| C5（①） | 10-10 | `anchor-tag-canon`（10-10・文字とバルーン） | `anchor-tag-canon`: アンカー `\_a[ID]…\_a` の**働き**を作った（それまでは押しても何も起きなかった）。 |
| C5（②） | 10-10 | `seriko-trigger-intervals`（10-10・シェルの element） | `seriko-trigger-intervals`: interval の 3 語 `talk,数値`・`runonce`・`periodic,数値` を動かした（それまでは名前を控えるだけで、口パクが動かなかった）。 |
| C5 の外（単独・開発の道具） | 10-10 | `impl-watch`（10-10・新しいクレート） | `impl-watch`: 机の貸し出し（マージの机＝リポジトリごと・負荷テストの机＝マシンに 1 つ・停止要請と再開）を行うコンソールアプリ `areka-impl-watch`（`crates/areka-impl-watch`・bin・`publish = false`）を作った。 |
| C5（⑦） | 10-11 | `mcp-stdio-bridge`（10-11・新しいクレート） | `mcp-stdio-bridge`: Claude Desktop 用の中継 `areka-mcp-bridge.exe`（`crates/areka-mcp-bridge`・bin・`publish = false`・SSP の `mcp.exe` 相当）を作った。 |
| C6 の前（単独・配布と公開） | 10-11 | `release-notes-highlights`（10-11・リリース CI と道具） | `release-notes-highlights`: GitHub Release の本文を、GitHub に作らせた PR の題名の一覧から、その版の見どころ（数件・日本語・Claudia の口調）に替えた。 |
| C6 の前（単独・配布と公開） | 10-15 | `release-cycle`（リリースの手順） | `release-cycle`: 繰り返し spec として `v0.0.2` を出したリリースの手順を、スキル `/release`（`.claude/skills/release/` の `SKILL.md`・`procedure.md`）へ移して完了にした。 |
| 版に縛らない（単独・道具と手入れ） | 10-16 | `roadmap-overview-html`（ロードマップの俯瞰図と doc の整理） | `roadmap-overview-html`: roadmap.md から版の段ごとの俯瞰の HTML（帯・組・カードの開閉・テーマの絞り込み・完了の場所・外の物を読まない 1 枚）を作る道具 `tools/roadmap-overview.ps1`（文言のファイル・ひな形・見本つき・`-SelfTest` を全体テストの段 `roadmap overview check` に足した）を作り、今の roadmap.md から `doc/roadmap-overview-2026-10-15.html` を 1 枚作った。 |
| C7（④・版に縛らない） | 10-16 | `reflow-scroll-path-test`（文字の層の経路の検査） | `reflow-scroll-path-test`: 2 行だけ入る欄で 3 行へあふれる台本を流し、区間の全文で配置する枝（先渡しあり）とあふれのスクロール（`visible_window`）を組み合わせて踏む経路の検査 `full_text_path_keeps_glyph_rows_through_overflow_scroll` を `crates/areka-emo-text/src/actor_lookahead_tests.rs` の末尾に足した（+131／−0・本番のコードは無変更）。 |
| C7（⑦・版に縛らない） | 10-16 | `impl-watch-skills`（机の段のスキルと置き方の道具） | `impl-watch-skills`: 机の貸し出しのアプリ `areka-impl-watch.exe` を置く道具 `tools/impl-watch-install.ps1` を足した（組み立て → `.new.exe` → 今のものを `.old-<UTC>.exe` へ改名して入れ替え・失敗したら戻す → 古いものの掃除 → `--help` で確かめる・端末へは ASCII だけ）。 |
| C7（⑧・版に縛らない） | 10-16 | `impl-watch-residue`（机の貸し出しのアプリの拾い残し） | `impl-watch-residue`: `crates/areka-impl-watch` の拾い残しを片づけた。 |
| C7（③・v0.0.3） | 10-16 | `app-icon`（アプリと通知領域のアイコン） | `app-icon`: areka の顔のアイコンを 2 組（組 1＝アプリ・組 2＝通知領域）作り、`areka.exe` に埋めた。 |
| C7（⑤・版に縛らない） | 10-16 | `perf-tools-console-encoding`（性能改善ループの道具の文字コード） | `perf-tools-console-encoding`: `tools/perf/` の子 pwsh が端末の文字コードを書き替えず、子の出力は読む側で UTF-8 として受け取る。道具が端末へ出す文は英語だけ。判定器 `encoding check` が `tools/perf/` も見る。完了時の棚卸＝その場の解決 2 件・起票 0 件 |
| C7（⑥・版に縛らない） | 10-16 | `dev-helper-x64-clobber`（開発時の helper の置き場の罠） | `dev-helper-x64-clobber`: 全体テストの最後の段 `i686 helper beside areka.exe` が `target\debug\` の隣の helper を i686 版へ揃え（道具 `tools/sync-i686-helper.ps1`・自己検査の段 `i686 helper sync check`）、helper を起こす入口が向きを見て x64 版なら `helper_not_i686` の 1 行を出す。 |
| C7（①・v0.0.3） | 10-16 | `baseware-root-list`（ゴーストとバルーンの置き場の並び） | `baseware-root-list`: ゴーストとバルーンを置く根を「`c:\areka` → `c:\ssp`」のように並びで持ち、上から探して同じフォルダ名は上を使う（下は隠れて記録に残る）。 |
| C7（②・v0.0.3） | 10-16 | `balloon-text-bugs`（文字の層のバグ 3 件） | `balloon-text-bugs`: 件 1＝選択肢つきのトークのバルーンの時間切れを、選択の待ちが解けた時刻（トークの終わり）から測る（可視性の線の `ChoiceShown`・選択肢の抑止を外した・完了 `balloon-visibility` 5.4 を改めた）。 |
| C8（⑦・版に縛らない） | 10-16 | `mcp-stdio-bridge-residue`（中継の拾い残し） | `mcp-stdio-bridge-residue`: 中継 `crates/areka-mcp-bridge` の拾い残しを片づけた。 |
| C8（①・v0.0.3） | 10-17 | `user-data-root`（利用者の物の置き場） | `user-data-root`: winget で入れた形と、`areka.exe` のフォルダに書けない形（`Program Files` の下など）では、利用者ごとの書ける根（`%LOCALAPPDATA%\areka`・`AREKA_USER_ROOT` で替えられる）を置き場の並びの先頭に入れ、exe のフォルダの中へは何も書かない。 |
| C8（②・v0.0.3） | 10-17 | `write-failure-notice`（書けないときの伝え方） | `write-failure-notice`: areka の記憶を書けないと、起動の前・動いている途中・終わるときのどこでも、告知を 1 つだけ出して終了コード 1 で終わる（題名は起動の前が「areka を起動できません」、その後が「areka の記憶を書けません」・`AREKA_NO_ALERT` は画面だけを抑え、終わり方は同じ）。 |
| C8 の外（単独・v0.0.3） | 10-18 | `emily-ghost-verification`（えみりの `.nar` が入らない件） | `emily-ghost-verification`: えみり（`emily4.nar`）が入るようになった。 |
| C8 の外（単独・v0.0.3） | 10-18 | `move-canon`（名前付きの `\![move]` で窓が動かない件） | `move-canon`: `\![move]` の名前付きの形と旧い位置の形を同じ 1 つの指令へ読む。 |
| C9（⑧・v0.1.0） | 10-18 | `mcp-receiver-bugs`（PR#335・MCP の受け口のバグ 2 件） | `mcp-receiver-bugs`: 件 1＝MCP の受け口が、形の合わない `params` の `tools/call`・`initialize`・`completion/complete` に `-32601` でなく `-32602`（文は `invalid params for <メソッド名>`・`data` なし）で答える。 |
| C9（③・v0.1.0） | 10-19 | `mcp-kanade-tools`（MCP の `sakurascript`・`raise_event`） | `mcp-kanade-tools`: MCP の `sakurascript` と `raise_event` が `NG:not implemented yet` でなく本物の答えを返す。 |
| C9（⑥・v0.1.0） | 10-19 | `real-run-mcp-kit`（実機の走行のテストの共通の道具） | `real-run-mcp-kit`: 実機の走行のテストの共通の道具を、テスト専用の新しいクレート `crates/real-run-kit/` に置いた。 |
| C9（②・v0.1.0） | 10-19 | `startup-options-core`（SSP の起動オプションの読み取りの土台） | `startup-options-core`: areka が SSP と同じ書き方の起動オプション（`--x 値`・`--x=値`・`/x`・`-x`・`--option a,b`）を読む。 |
| C9（①・v0.1.0） | 10-19 | `impl-watch-test-desk`（机の貸し出しのアプリを 3 種類の机へ） | `impl-watch-test-desk`: 机の貸し出しのアプリ `areka-impl-watch.exe` の机を、開発者の模型の 3 種類に組み直した＝**実装の机**（マシンに 1 つ・何人でも乗れ、負荷テストがひとり占めできる）・**排他テストの机**（マシンに 1 つ・全体テストと実機の UI テストを 1 度に 1 人・停止要請を出さないので、ほかの実装は止まらない）・**マージの机**（リポジトリに 1 つ）。 |
| C9（⑦・v0.1.0） | 10-19 | `zorder-scope-inversion`（キャラの窓を自分のバルーンより手前に置く重なりの順） | `zorder-scope-inversion`: 明示の `\![set,zorder,…]` を書かれた順のまま採り（逆転・非隣接とも）、片方だけ名指しのスコープに相棒を補う。 |
| C9（⑤・v0.1.0） | 10-19 | `sample-url-fetch`（検体を URL の行で取り寄せる） | `sample-url-fetch`: 検体の登記表の行に在処の種類（`SampleSource`＝リポジトリの書庫か URL の行か）を足し、9 本目の検体「窓際のぱすたさん」（`pasta-in-windows`）を最初の URL の行として登記した（GitHub Pages の公開の URL・隣の `.sha256` で照合・いつも最新を追う・登記に大きさもハッシュも書かない）。 |
| C9（④・v0.1.0） | 10-22 | `mcp-reload`（MCP の `reload` と台本の `\![reload,…]`） | `mcp-reload`: ghost・shiori・shell・balloon・descript の読み直しを、MCP の `reload` と台本の `\![reload,…]` の 1 つの入口 `request_reload` で受ける（読み直しの重なりは断る・語を記録に残す・`reload_started`／`reload_done`／`reload_failed`）。 |
| C10（⑧・v0.1.0） | 10-23 | `emo2-real-run-stability`（emo2 を実機で起こす走行の赤 2 件） | `emo2-real-run-stability`: 件 1＝`emo2_real_run` が `wrap=BudouxWordWrap` を時間で待たず、実走の道具（`real-run-kit`）の MCP の呼び出しで台詞の印（wrap まで）を見てから閉じる形に直した（共通の手順は `crates/areka/tests/emo2_real_run/common.rs`）。 |
| C10（④・v0.1.0） | 10-23 | `property-name-case-fold`（プロパティの名前の英字の大小） | `property-name-case-fold`: プロパティの名前の半角英字だけを小文字へ畳む形にした（sylphya の `parse_seg` の 1 か所・全角・かな・前後の空白・値は畳まない）。 |
| C10（⑥・v0.1.0） | 10-23 | `dot-sakura-specimen`（検体「．さくら」と実機の 1 回） | `dot-sakura-specimen`: 開発者のゴースト「．さくら」（`dot_sakura.nar`・SHIORI は華和梨 7.4）を検体に加えた（書庫は受け取ったバイト列のまま `vendors/sample_ghost/` に置き、登記表 `SAMPLES` に `dot_sakura`〔同梱バルーン `bottle`〕を足して 10 行）。 |
| C10（②・v0.1.0） | 10-23 | `window-link-directives`（窓どうしの連携と吸着の向き） | `window-link-directives`: 台本の `\![set,alignmenttodesktop,方向]` で窓の吸着の辺を変える形を受けた。 |
| C10（①・v0.1.0） | 10-23 | `script-security-level`（台本とイベントの出どころとセキュリティレベル） | `script-security-level`: 台本とイベントに出どころ（ゴースト自身・MCP の `sakurascript`・MCP の `raise_event`）を付けて再生の終わりまで運ぶ形にした。 |
| C10（③・v0.1.0） | 10-23 | `ghost-inner-balloon`（ゴーストのフォルダの中のバルーン） | `ghost-inner-balloon`: ゴーストのフォルダの中の `balloon\<名>\` をそのゴーストにだけ見える候補にし（起動・切替・メニュー・タグ・記憶・読み直し・更新・開く・インストールの後の表示名）、同じフォルダ名は根の置き場のものを隠す。 |
| C10（⑤・v0.1.0） | 10-23 | `offline-surface-dump`（窓も GPU も使わない面の書き出し） | `offline-surface-dump`: `--offline-dump` とその仲間で、窓も GPU も使わずに面を PNG に書き出してすぐ終わる（SSP の書き出しと同じ時点・同じ着せ替え・CSV と終了コード 0/1/2）＋比べる道具 `surface-compare`。 |
| C10（⑦・v0.1.0） | 10-23 | `pasta-in-windows-verification`（検体「窓際のぱすたさん」が SSP と同じに動くかの確かめ） | `pasta-in-windows-verification`: 検体「窓際のぱすたさん」を SSP 2.9.18 と並べて測り、崩れた所の表に持ち主を付け、areka の側で直せるものを直し、残りを v0.1.0 の申し送り 6 本と既存の brief へ渡した。 |
| C11（④・v0.1.0） | 10-23 | `mcp-scope-rect-property`（外から窓とバルーンの位置を読むプロパティ） | `mcp-scope-rect-property`: sylphya に読まれた時に一次情報源を見に行く口を足し、`currentghost.scope(ID).rect` と `currentghost.balloon.scope(ID).rect` を MCP の `get_property` とゴーストのプロパティの道で、OS が持つ今の窓の矩形「左,上,右,下」で答える。 |
| C11（⑧・v0.1.0） | 10-24 | `shiori2-protocol`（SHIORI/2.x の口だけを持つゴーストが話す） | `shiori2-protocol`: 読み込みの直後に `GET Version SHIORI/2.6` で SHIORI の版を確かめ、2.x の相手には要求を組み替え（翻訳＝`TRANSLATE Sentence` 2.6・資源＝`GET String` 2.5・ほか＝`GET Sentence` 2.2・`basewareversion` は送らない）、答えの `Sentence`／`String` を 3.0 の値へ戻して、どっとさくら（華和梨）が話すようにした。 |
| C11（①・v0.1.0） | 10-24 | `descript-window-keys`（descript の窓の鍵） | `descript-window-keys`: ゴーストの descript の `sakura.`／`kero.`／`char<n>.seriko.defaultsurface` を 1 回だけ読み、窓の採寸・起動の最初の面・読み直しとシェルの切り替えで戻る面に使い（seriko と文字の層が同じに決める）、バルーンの「動かさない」をシェルのスコープごとの `*.balloon.dontmove` ＞ ゴーストの `balloon.dontmove` と台本の `lock`／`unlock` の `balloonmove` で決め、ぱすたの窓を面 1000 で測って下端を作業領域の中に収めた。 |
| C11（⑤・v0.1.0） | 10-24 | `vertical-choice-hover-band`（縦書きの選択肢の反転の帯） | `vertical-choice-hover-band`: 縦書きのバルーンで選択肢とアンカーに乗せたときの反転の帯を、書体ごとに DirectWrite で測った縦書きの基線に em の箱を中央で重ねる所に塗り、当たりの範囲も同じ所にした（横書きは変えない）。 |
| C11（⑥・v0.1.0） | 10-24 | `dpi-realign-remembered-chain`（拡大率が変わったあとの隣り合いの詰め直し） | `dpi-realign-remembered-chain`: 拡大率の後と起動の並べ直しを隣り合いで決め、記憶に隣り合いの印 `[window-adjacent]` を持たせ、位置を覚えているキャラクターどうしも隣り合ったままに見えるようにした。 |
| C11（③・v0.1.0） | 10-24 | `readonly-mode`（利用者の環境へ何も書かない起動） | `readonly-mode`: `--option readonly` で利用者の環境へ何も書かずに起動する（areka とゴーストの記憶・起動中の印・置き場の並び・利用者ごとの置き場の作成と写し・書けるかの確かめで作る物を、保存の口の控えに入れて走る間だけ持つ。インストール・URL からの取り込み・ネットワーク更新を断り、更新の断りは SSP の測りに合わせて OnUpdateResultEx→OnUpdateResult の総括 2 件だけを送る。右クリックメニューのインストールとネットワーク更新を灰色に、フォルダ選びは最近使った所に足さない）。 |
| C11（②・v0.1.0） | 10-24 | `mouse-click-wheel-events`（クリック・ホイール・出入りのイベント） | `mouse-click-wheel-events`: 左右・中・拡張ボタンの解放を OnMouseUp(Ex)→OnMouseClick(Ex) の段の列で送り（台本で止まり、送信の失敗は 204 と同じ）、右クリックは解放の知らせ→メニューの順、縦のホイールを窓ごとに 100 ms で間引いて OnMouseWheel、窓ごとの OnMouseEnter/Leave/EnterAll/LeaveAll を送る。ぱすたの × が赤くなりボールでトークが出ることを実機で確かめた。 |
| C12（⑥・v0.1.0） | 10-30 | `default-balloon-selfmade`（areka 自前の既定バルーン） | `default-balloon-selfmade`: 「ボトル（でふぉ改）」の寸法と設定値を引き継いだ自前の既定バルーン `areka-balloon`（面と動かない印は α 付きの PNG・オンラインの印は動く WebP 1 枚・descript は UTF-8・絵はコードで描く道具で作り直せる）を検体に登記し、起動の解決の既定と配布の道具を `StayseeBalloon` から差し替えた。 |
| C12（②・v0.1.0） | 10-30 | `shell-mouse-cursor`（シェルのマウスカーソル） | `shell-mouse-cursor`: シェルの `sakura.cursor`／`kero.cursor`／`char*.cursor` ブレス（`mouseup`・`mousedown`・`mouserightdown`・`mousehover`・`mousewheel` × 当たりの名前 × `system:*` か `.cur`／`.ani`）とゴーストの descript の `mousecursor`（`cursor`）・`.arrow`・`.hand`・`.grip` で、キャラの窓の上のマウスカーソルを替える。開いた手と握った手の `.cur` の絵を areka に同梱し（fal の Ideogram で描いた）、wintf が `WM_SETCURSOR` に答え、ボタンを離したとき・ホイール・クリックの透過で出し直す。起動とシェル／ゴーストの切り替えで新しい表へ渡す（台帳 21 行）。 |
| C12（④・v0.1.0） | 10-30 | `real-run-log-noise`（実機のログの量） | `real-run-log-noise`: 再追従で入力が前と同じなら解き直さない（文字の層の `debug!` 2 種類が装着と入力の変わり目だけになる）。選択肢の飾りの警告を置き場ごとに値が決まったときの 1 件にする（どっとさくらの `bottle` の ROP 系 20,764 行 → 2 行）。同梱 emo2 の着せ替えの「無し」の絵 `null.png` の `WARN` を `debug` へ。validrect の右辺・下辺の 0 を画像の右端・下端と読む（`bottle` で文字が描ける）。 |
| C12（①・v0.1.0） | 10-30 | `mcp-strict-errors`（strict の記録） | `mcp-strict-errors`: `sakurascript`／`raise_event` の `strict` が真の台本に再生まで印を付けて運び、不在の面・バルーン・アニメ、知らない字、`\!` と `\&` の誤り、`\f` の引数など 16 の検出点で見つけた誤りを、再生の順に talk スレッドでエラーログへ書き（ゴーストの名前と検出点つき・答えの `since_id` で `get_log` から読める）、`params` の形だけが誤る要求には rmcp の 415 の後に rmcp の公開の型で -32602（2026-07-28 以降の版は HTTP 400）を返す（rmcp は改造しない＝10-30 の裁定）。中継も同じ答えを返し、`INSTRUCTIONS` から「not implemented yet」を外した。 |
| C12（⑤・v0.1.0） | 10-30 | `startup-install-option`（起動時のインストール） | `startup-install-option`: SSP と同じく `areka.exe --install a.nar b.nar` で、最初の定常に着いた後に並べた順に投げ込みと同じ道で入れ、`--option deletesource` で入った書庫を消し、`--option callghost` で入れた後にそのゴーストを呼ぶ。exe の隣の `initial_install.nar` も同じ道で入れ、入ったら消す。読み取りだけの起動（`--option readonly`）ではどれも行わない。実機の試験 `crates/areka/tests/startup_install_real_run.rs` で `--install` から起動中までを確かめた。起動の解決の前に入れる形は `install-before-boot` へ（要件 10.2）。 |
| C12（⑦・v0.1.0） | 10-30 | `window-work-area-clamp`（キャラクターの窓を作業領域の外へ出さない） | `window-work-area-clamp`: 起動の並べ直し・拡大率の後の詰め直し・大きい面への切り替え・記憶から戻す組の窓を作業領域の中へ寄せ（組は同じ量だけ動かす）、起動のトークの位置の指令を対象と基準の窓が最初の面を表示するまで預け（SSP と並べた測りで見込み H1）、バルーンだけが横にはみ出すときは反対の側へ並べ替える（2026-10-30 の裁定・完了 `areka-P0-windowposition-limit` の押し込みの決まりを改めた）。完了時の棚卸で `real-run-kit` が起こす子へ `PASTA_DEBUG=0` を渡し、`balloon-flip-side-rules`・`window-resetwindowpos-hold` を起票した。 |
| C12（⑧・v0.1.0） | 10-30 | `bridge-store-user-root`（Store 版の Claude Desktop が起こした中継の置き場） | `bridge-store-user-root`: Store 版の Claude Desktop が起こした中継が、箱の中に areka の置き場を作り記憶を写していた件を直した（中継は置き場を決めるだけにし、作る・写すは areka 本体へ。箱へ回されることは記録の行で伝える）。 |
| C13（v0.1.0） | 10-31 | `seriko-pattern-calls`（pattern定義でアニメーションを呼ぶ語と `move`） | `seriko-pattern-calls`: SERIKO の pattern定義でアニメーションを呼ぶ 7 語（start・stop・insert・alternativestart／alternativestop・parallelstart／parallelstop）を重ね方でなく再生の働きとして受け、`never` と純 `bind` は呼ばれたときだけ再生する。`move` は窓ごとキャラクターをずらす（倍率をかける・作業領域へ寄せる・ドラッグの終わりと位置の初期化と絶対位置の `\![move]` で元の位置を取り直す）。「．さくら」の実機で 8 語の WARN 0 件。 |
| C13（v0.1.0） | 10-31 | `balloon-scroll-arrows`（バルーンのスクロールの矢印と手動の送り） | `balloon-scroll-arrows`: 押し出された行があるとき、バルーンの `arrow0`・`arrow1` の矢印を descript の座標（マイナスは右端・下端から・`--数値` はマイナスそのもの）と `arrow.filename` の名前で出し、矢印のクリック（2 行）とホイール（OS の行数）で手動に戻して読めるようにした（横書き・縦書き・シェル内バルーンの箱で同じ規則・`\b[ID]` の面の設定ファイルを重ねる）。実機で既定のバルーン・ぱすたの縦書き（SSP と同じ座標）・箱を確かめた。 |
| C13（v0.1.0） | 10-31 | `stay-on-top-directives`（常に手前と手前へ出す指令） | `stay-on-top-directives`: `\v` はゴーストの窓の全部を 1 回手前へ出す（SSP 2.9.26 の働き・帯の外は HWND_TOPMOST→HWND_NOTOPMOST の案 A）。`\![set,windowstate,stayontop]`／`!stayontop` の常に手前はゴーストが終わるまで続き、wintf に WindowBand（確定段の帯の系・SWP_NOOWNERZORDER）を足した。シェルの切り替えの後も台本の zorder の組を保ち（件 1）、consumer_ledger に 2 行（`set,windowstate` と `\v` の運搬名）を足した。 |
| C13（v0.1.0） | 10-31 | `animated-surface-log-flood`（動く絵のコマ送りの記録の量） | `animated-surface-log-flood`: 動く面のコマ送りの表示の行は DEBUG に下げた（初めての表示・面の切り替え・倍率・大きさ・出し直しは INFO のまま）。コマの変わり目は前の絵の上に変わった矩形だけを合成し直し、同じ原因の装置の失敗の繰り返しは 1 行にまとめた。件 26（`bind+random` の pattern0 で目を閉じたまま止まる）も直した。何もしない 6 分の記録は約 5,330 行から約 200 行になった。release 版の面 1002 の何もしていないあいだの CPU は 15.9%→15.4%（SSP 2.9.26 は 5.6%・残りは `wintf-visual-damage-redraw` の件 1 へ）。 |
| C14（④・割り込み） | 10-31 | `emo-atlas-palette-alpha`（パレットの PNG の透明度） | `emo-atlas-palette-alpha`: パレットの PNG の `tRNS`（と 8 ビット以下のグレーの `tRNS`・GIF の透明色）を持つ絵を、emo-atlas の WIC の腕と wintf の `load_bitmap_source` が α を持つ絵として読む（番号つきの形式でだけパレットの `HasAlpha` を見る）。 |
| C14（⑤・割り込み） | 10-31 | `choice-ranges-one-function`（選択肢の範囲を出す関数 1 つ） | `choice-ranges-one-function`: 選択肢とアンカーの強調とクリックの当たりを、行の列から出す本番の関数 `choice_ranges`／`ChoiceRanges::hit_rows`（`actor_present.rs`）1 つにまとめ、本番の提示と検査 2 か所が同じ関数を呼ぶ。帯の共有の検査を 1 本足し、並びを崩す変異 V1〜V7 が全部赤になることを確かめた。字・色・帯・当たりは変えていない。 |
| C14（⑦・割り込み） | 10-31 | `popup-menu-residue`（メニューと説明書の道の残件） | `popup-menu-residue`: メニューの組み立ての失敗を表示の失敗と分けて記録する（`menu_build_failed`）。説明書の受け口の記録 7 行に `[readme]` と `event=` を付け、台本の `\![open,readme]` から偽の OS までの通しのテストを 2 本足した。kanade の SHIORI の往復の失敗の文から事実と合わない「終了」を外し、終了のスコープが OnClose の References まで届くことを通しのテストで確かめた。網羅台帳の語彙だけの 4 行に引受先（`extra-character-windows`）と裁定（省略した形のメニューを作り分けない）を書き、`owner_count` を 10→6 にした。件 11・12 は `popup-menu-shell-look` へ写し、件 13 は `popup-menu-extra-resources` として起票した。 |
| C14（⑥・割り込み） | 10-31 | `actor-thread-log-capture`（背景のアクターの記録をテストの捕捉へ） | `actor-thread-log-capture`: `spawn_actor` が、親のスレッドで記録の捕捉の窓が開いているときだけ、その窓（Dispatch）を子のアクターのスレッドへ引き継ぐ（印の型 `CaptureWindowMarker` を `downcast_raw` で問う・窓が閉じたら答えない）。本番の記録は変えない。log-capture-kit の `capture` は `WindowSubscriber` で包み、閉じた後の記録を窓へ入れない。窓に背景のアクターの記録が入るようになったので、水準ごとの 0 件の主張を宛先で絞ったテストが 2 本ある（`update/worker_path_tests.rs`・`emo2_boot/update_cue_tests.rs`）。 |
| C14（③・v0.1.0） | 10-31 | `placement-measure-bake-once`（起動の採寸で動く絵の全コマを読まない） | `placement-measure-bake-once`: 採寸の道だけ動く絵を 1 枚目で読む読み手 `FirstFrameDecoder` で包み、シェル・バルーンの読み込みの記録を出さない入口（`LoadRecords`・`_quiet`）で採寸と `windowposition` の読みを黙らせ、記録は資産の組み立ての 1 回だけ。焼く回数（シェル 2・バルーン scope ごと 2）と全コマ読み 1 度・記録の行の数を決定論の檻で固定。release の測り直しで動く絵 1 つのシェル 2,427→1,298 ms・4 つで 9,184→4,691 ms（採寸の分は約 100 ms）。実機（ぱすた・emo2・200%）で窓の大きさ・位置は前後同じ。完了時の棚卸で `emo-bake-log-overlap` を起票。 |
| C14（②・v0.1.0） | 10-31 | `install-before-boot`（`--install` と `initial_install.nar` を起動の前に入れる） | `install-before-boot`: 起動の解決の前に、ゴーストも窓口も無い口 `BootPorts` で書庫を入れ（イベントは送らない・利用条件の画面の前に DPI の扱いを 1 回だけ立てる）、入れた後の置き場でゴーストを選ぶ。`callghost` は選び方の段（`--random` の後・記憶の前）になり、空の置き場からも起きる。定常の後に入れて切り替える道は外した。入れた物の控えは起動の後の受け皿・置換語へ引き継ぐ |
| C14（①・v0.1.0） | 10-31 | `script-small-tags`（`\_u`・`\_m`・`\![bind-noevent]`・`\![*]`） | `script-small-tags`: `\_u[0x…]`・`\_m[0x…]` を字にする（出せない番号は警告 1 行で捨て、strict・`check_script` は「読めない引数」）・サロゲートの組を 1 字に／`\![bind-noevent,…]` を `bind` と同じ当て方で受け、記録の `name` で分ける・トグル形とカテゴリ単位形を当てる／`\![*]` と `%*` を行の中の印にし（既定の丸＝字の大きさの半分・面ごとの `marker.filename` の絵）、`\q` と畳まない・受け手の表は 42 件。完了時にその場で解決 1 件（出ている箱も面の切り替えで変わった印を使う）・起票 0 件 |
| C15（⑥・割り込み） | 11-01 | `popup-menu-extra-resources`（古いゴーストのメニューの資源の古い名前） | `popup-menu-extra-resources`: 「ネットワーク更新」の項目で古い名前 `updatebuttoncaption` も新しい名前 `updatebutton.caption` の直前に 1 回だけ問い、新しい名前が空・値なし・失敗のときは古い名前の値を項目名にする（kanade の許可表は 15 名）。「．さくら」で areka と SSP がどちらも「気まぐれ更新(U)」になることを確かめた。⑵（消滅通告・おすすめ・ポータルの項目）は windows-reactor 待ちの段へ渡した |

- 2026-10-23 に、この表の 4 欄目（一行要約）は最初の 1 文だけを残した（俯瞰の道具が読むのは表のユニットの欄と一言）。行の全文は history「2026-10-23 完了済みの退避」。
- 完了 spec 直下エントリ＝**320**（`.kiro/specs/completed/` 直下・2026-11-01 `popup-menu-extra-resources` の完了後に実数えして一致＝フォルダ 319＋`graphics-rendering-stability.md` 1）。その前の 319＝2026-10-31 `script-small-tags` の完了後（フォルダ 318＋`graphics-rendering-stability.md` 1）。その前の 318＝2026-10-31 `install-before-boot` の完了後（フォルダ 317＋`graphics-rendering-stability.md` 1）。その前の 317＝2026-10-31 `placement-measure-bake-once` の完了後（フォルダ 316＋`graphics-rendering-stability.md` 1）。その前の 316＝2026-10-31 `actor-thread-log-capture` の完了後（フォルダ 315＋`graphics-rendering-stability.md` 1）。その前の 315＝2026-10-31 `popup-menu-residue` の完了後（フォルダ 314＋`graphics-rendering-stability.md` 1）。その前の 314＝2026-10-31 `choice-ranges-one-function` の完了後（フォルダ 313＋`graphics-rendering-stability.md` 1）。その前の 313＝2026-10-31 `emo-atlas-palette-alpha` の完了後（フォルダ 312＋`graphics-rendering-stability.md` 1）。その前の 312＝2026-10-31 `animated-surface-log-flood` の完了後（フォルダ 311＋`graphics-rendering-stability.md` 1）。その前の 311＝2026-10-31 `stay-on-top-directives` の完了後（フォルダ 310＋`graphics-rendering-stability.md` 1）。その前の 310＝2026-10-31 `balloon-scroll-arrows` の完了後（フォルダ 309＋`graphics-rendering-stability.md` 1）。その前の 309＝2026-10-31 `seriko-pattern-calls` の完了後（フォルダ 308＋`graphics-rendering-stability.md` 1）。その前の 308＝2026-10-30 `bridge-store-user-root` の完了後（フォルダ 307＋`graphics-rendering-stability.md` 1）。その前の 307＝2026-10-30 `window-work-area-clamp` の完了後（フォルダ 306＋`graphics-rendering-stability.md` 1）。その前の 306＝2026-10-30 `emo-text-dwrite-fidelity` の完了後（フォルダ 305＋`graphics-rendering-stability.md` 1）。その前の 305＝2026-10-30 `startup-install-option` の完了後（フォルダ 304＋`graphics-rendering-stability.md` 1）。その前の 304＝2026-10-30 `mcp-strict-errors` の完了後（フォルダ 303＋`graphics-rendering-stability.md` 1）。その前の 303＝2026-10-30 `real-run-log-noise` の完了後（フォルダ 302＋`graphics-rendering-stability.md` 1）。その前の 302＝2026-10-30 `shell-mouse-cursor` の完了後（フォルダ 301＋`graphics-rendering-stability.md` 1）。その前の 301＝2026-10-30 `default-balloon-selfmade` の完了後（フォルダ 300＋`graphics-rendering-stability.md` 1）。その前の 300＝2026-10-24 `mouse-click-wheel-events` の完了後（フォルダ 299＋`graphics-rendering-stability.md` 1）。その前の 299＝2026-10-24 `readonly-mode` の完了後（フォルダ 298＋`graphics-rendering-stability.md` 1。2026-10-24 `dpi-realign-remembered-chain` の完了〔+1・この行と上の表の行は書き足されていなかった＝この完了で表の行も足した〕も含む）。その前の 297＝2026-10-24 `vertical-choice-hover-band` の完了後（フォルダ 296＋`graphics-rendering-stability.md` 1）。その前の 296＝2026-10-24 `descript-window-keys` の完了後（フォルダ 295＋`graphics-rendering-stability.md` 1）。その前の 295＝2026-10-24 `shiori2-protocol` の完了後（フォルダ 294＋`graphics-rendering-stability.md` 1）。その前の 294＝2026-10-23 `mcp-scope-rect-property` の完了後（フォルダ 293＋`graphics-rendering-stability.md` 1）。その前の 293＝2026-10-23 `pasta-in-windows-verification` の完了後（フォルダ 292＋`graphics-rendering-stability.md` 1）。その前の 292＝2026-10-23 `offline-surface-dump` の完了後（フォルダ 291＋`graphics-rendering-stability.md` 1）。その前の 291＝2026-10-23 `ghost-inner-balloon` の完了後（フォルダ 290＋`graphics-rendering-stability.md` 1）。その前の 290＝2026-10-23 の steering の同期で実数えして一致・2026-10-23 `script-security-level` の完了後（フォルダ 289＋`graphics-rendering-stability.md` 1）。その前の 289＝2026-10-23 `window-link-directives` の完了後（フォルダ 288＋`graphics-rendering-stability.md` 1。2026-10-23 `dot-sakura-specimen` の完了〔+1・この行は書き足されていなかった〕も含む）。それより前の数えの経緯は history「2026-10-23 完了済みの退避」。⚠ **引き算で導かず毎回実数えする**。
- M1 の持ち越しのうち残るのは `dpi-transition-two-tick-bounce`（開発者が許容）と `zorder-chain-residue` A-2（据え置き）だけ。M-dual は退役。
- 個々の完了行の全文（種別・議題・完了時の所見）は history「2026-10-02 棚卸⑳退避」。

### リリース

| 版 | 日付 | GitHub Release | winget-pkgs の PR | 赤で起票した spec |
|---|---|---|---|---|
| v0.0.2 | 2026-10-06 | https://github.com/ekicyou/areka/releases/tag/v0.0.2 | https://github.com/microsoft/winget-pkgs/pull/450070 （初回の手提出・2026-10-10・自動の検査は通過・人の承認待ち＝まだ取り込まれていない） | — |

`v0.0.3` からは、この表に行を足さない（2026-10-11 開発者の決定・リリースの記録は GitHub の Release の頁）。上の行は歴史として残す。

### 次のリリースへの申し送り

スキル `/release` が段 1 で読む（`.claude/skills/release/procedure.md` の 1.4）。「次の版上げのときに」と申し送る事は、ここへ 1 項目で足す。項目の頭に、書いたときの最新の版を「`v{その版}` の後」の形で書く（その版より後のリリースが済んだら、済んだ物として読み飛ばされる。消すのは、次に roadmap を触る PR）。版上げの 5 つのファイル（根の `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`dist/README.txt`・その版の見どころ）の外を触る仕事は、ここへ書かずに、リリースの前に別の PR で片づける（残っていると `/release` は段 1 で止まる）。

（`v0.0.2` の後の 8 項目は、2026-10-18 の `v0.0.3` の公開で済んだので、棚卸㉗で消した＝全文は history「2026-10-18 棚卸㉗退避」。中継の arm64 の確かめだけは持ち主が居ないので、覚え書きの「道具と試験」へ移した。）

- **`v0.0.2` の後**・`user-data-root`（2026-10-17）: 動きが変わる点＝winget で入れた形と、`areka.exe` のフォルダに書けない形では、利用者の物（後から入れたゴーストとバルーン・areka とゴーストの記憶・中継の控え）を `%LOCALAPPDATA%\areka\` に置く（環境変数 `AREKA_USER_ROOT` で替えられる・`dist/README.txt` の「■ 置き場」「■ 記憶の置き場」）。0.0.2 を winget で入れた人は、最初の上げ直しで、0.0.2 のフォルダの中に後から入れたゴースト・バルーンとゴーストの記憶が消える（実測 6・areka の記憶は引き継ぐ・説明書に注意の行）。右クリックメニューに「置き場を先頭へ」が増えた。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。winget-pkgs へ次の版を出すのは `winget-release-automation`（雛形の注意書き `InstallationNotes` を外してよい条件は、同 spec の brief の「user-data-root からの申し送り」）。
- **`v0.0.2` の後**・`write-failure-notice`（2026-10-17）: 動きが変わる点＝areka の記憶を書けないとき、黙って続けずに告知を 1 つ出して終わる（終了コード 1）。書けない場所へのゴーストのインストールの失敗は「ファイルが壊れてる」ではなく `fileio` の語でゴーストへ渡る（emo2 の台詞は辞書の持ち主の対応待ち）。
- **`v0.0.2` の後**・`emily-ghost-verification`（2026-10-18）: 動きが変わる点＝`emily4.nar` のように、同梱のバルーンの取り出し元が書庫に無い `.nar` も入る（そのバルーンだけ飛ばし、警告のダイアログを 1 つ出す）。`char2` 以降の鍵を持つゴーストで落ちない（3 人目からの窓はまだ出さない）。同じ番号の surface ブレスが複数あるシェルの絵が消えない。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。
- **`v0.0.2` の後**・`move-canon`（2026-10-18）: 動きが変わる点＝名前付きの `\![move,--X=…,--base-offset=…,--move-offset=…]` で窓が動く（emo2 の 10-17 版のメニュー「エモの位置調整」が効く）。参照点と基準 `screen`／`primaryscreen`／`global`／`me` を正典どおりに解く。右クリックのメニューに「シェル位置初期化」が増え、`\![execute,resetwindowpos]` も窓を既定の位置へ並べ直す。`--time` つきの移動は今は瞬時に動く（時間の部分は dola の改造の後）。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。
- **`v0.0.3` の後**・`mcp-receiver-bugs`（2026-10-18）: 動きが変わる点＝MCP の受け口が、形の合わない `params` の `tools/call`・`initialize`・`completion/complete` に `-32601` でなく `-32602`（文は `invalid params for <メソッド名>`）で答える（中継 `areka-mcp-bridge.exe` の経由でも同じ）。ほかの知らないメソッドは今のまま `-32601`。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。
- **`v0.0.3` の後**・`mcp-kanade-tools`（2026-10-19）: 動きが変わる点＝MCP の `sakurascript` が `NG:not implemented yet` でなく台本を再生して `OK` を返す（再生中や選択肢の最中に送ると今のトークを中断して新しい台本を再生する）。`raise_event` は任意の名前のイベントでゴーストを呼び、返った台本を再生して答えに `Script: <台本>` を載せる（台本が無ければ `OK:the ghost returned no script.`）。`get_log` の `script` に、MCP から流した台本とゴーストの自発の会話の台本が出る。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。
- **`v0.0.3` の後**・`startup-options-core`（2026-10-19）: 動きが変わる点＝`areka.exe` が SSP と同じ書き方の起動オプションを読む（`--ghost`〔名前か絶対パス〕・`--random`・`--option nobootcheck`・`--option dontloadmodule`・`--sstp-listen`〔MCP の待受〕・`--help`〔説明書を開いて終わる〕）。今までは `--` で始まる引数もゴーストのフォルダの位置に読まれ「ゴーストが見つからない」で止まったが、知らない起動オプションは記録に残して起動を続ける。引数を並べる今の起動はそのまま。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）・`dist/README.txt` の「■ 起動オプション」。

## ウェーブ編成（着手順の正本・2026-10-31 棚卸㉜）

> 各ウェーブは**フルライフサイクル**（要件 → 設計 → タスク → 実装 → `/kiro-complete`＝PR の squash マージ）を完走してから次へ。1 spec ＝ 1 worktree ＝ 1 PR。**同じウェーブに入れるのは、棚卸㉜までの実測で触るソースファイルの重なりが 0 の組だけ**。許す重なりは 5 つだけ＝表の末尾への追記（`doc/COMPAT_ARCHITECTURE.md` §8）・生成物（`THIRD-PARTY-NOTICES.md`・`doc/ukadoc-coverage/report/`＝手で直さず作り直す）・steering・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の**別々の行**・説明書 `dist/README.txt` の**別々の行**。文書だけの段は置かない。
> **並び順の決まり（2026-10-07 開発者・10-05 の 3 段を改めた）**: 依存の木（働きの依存＋同じソースファイルを触る spec＋直列の列の順）の**先頭**だけを候補にし、**① ぶら下がる未完了の spec が多い ② 区分が高い（A ＞ B ＞ C ＞ D）③ 起票が古い**の順に並べ、上から互いに触るファイルが重ならないものを**最大 8 本**取る。据え置き・保留・取り下げ予定は木に入れない。**測ることが本体の spec はウェーブに並べない**（ほかの仕事を汚し、汚される）。区分 A でぶら下がる数が同じものは、開発者が「高」「早めに」と言ったものを先にした（棚卸㉓の読み＝違えば直す）。
> **版の段が最上位の物差し（2026-10-15 開発者・`/kiro-discovery`「版の段」・2026-10-16 改め）**: **今の版（まだ公開していない最も若い版）が残っているあいだ、後の版の spec はウェーブに入れない**（2026-10-16 開発者「ゴールが近いので他の版の割り込みは避けて」）。空いた席に入れてよいのは「版に縛らない」列のうち、今の版の着地までのウェーブ数を増やさないものだけ（同日・開発者「版に含めてもよいが、その場合、0.0.3 の着地（ウェーブ数）が増えることのないように」）＝先頭で、今の版の spec と触るファイルが重ならず、今の版が使う 1 本だけの席を取らず、見立ての上限がそのウェーブで一番大きい今の版の spec の下限以下で、人や外の待ちも、静かな机も、前後の全体テストの比べも要らないもの（判定の正本は `/kiro-next` のスキルの「Version gate」）。迷ったら席は空けておく。今の版の中は上の並び順の決まりで並べ、先頭を全部取ってから、空いた席を版に縛らない列の先頭（同じ並び順）で埋める。版に縛らない列の後続は待つ相手が着地してから・v0.2.0 の間は改造に触れないものだけ（「版の段」の注意）。棚卸㉕（C7）からこの形で組んだ。C9〜C12 は今の版 v0.1.0 の先頭だけで 8 席が埋まった。C14 は v0.1.0 の先頭が 3 本だけなので、空いた 5 席のうち 4 席を割り込みで埋めた（初めて）。C15 は v0.1.0 の先頭が `shell-scale-linkage` の 1 本だけなので、空いた 7 席のうち 5 席を割り込みで埋めた（うち 3 本は範囲を絞る約束つき）。
> **1 ウェーブに 1 本だけの席**: ⑴ 依存を足す spec と版上げ（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md`）＝C15 では誰も使わない ⑵ emo-text に新しいソースファイルを足す spec（`lib.rs` の純粋な一覧の数の行）＝C15 では ② `emo-text-line-gap` だけが使ってよい（ほかの 5 本は emo-text にファイルを足さない）。
> **C14（2026-10-31 棚卸㉛・7 本）**: 7 本とも 10-31 に着地した（`script-small-tags`・`install-before-boot`・`placement-measure-bake-once`＝v0.1.0／`emo-atlas-palette-alpha`・`choice-ranges-one-function`・`actor-thread-log-capture`・`popup-menu-residue`＝割り込み）。C14 の行の全文・C15〜C16 の見込みと棚卸㉛の裁定は history「2026-10-31 棚卸㉜退避」。C13 より前は、そこから辿る。

| Wave | ユニット（並べた順） | 開始コマンド | 編成根拠（触るファイルと、同じウェーブの間の約束） |
|---|---|---|---|
| **C15**（6 本並走・今の版 v0.1.0 の 1 本＋割り込み 5 本・Opus 6・想定タスクの合計＝約 30〜47） | ① `shell-scale-linkage`（A・約 1＝`balloon-communicatebox`・11〜16） ∥ 割り込み ② `emo-text-line-gap`（B・0・4〜7） ∥ ③ `alert-dialog-per-monitor-dpi`（B・0・件 1 だけで 3〜5） ∥ ④ `test-roots-under-target`（C・0・入口の差し替えだけで 4〜7） ∥ ⑤ `emo-bake-log-overlap`（C・0・案 1 で 6〜9） ∥ ⑥ ✅ `popup-menu-extra-resources`（C・0・⑴ だけで 2〜3・11-01 着地） | `/kiro-start <名>` | ①＝`emo2_boot/{consumer_ledger,consumer_ledger_tests,consumer_ledger_agreement_tests}.rs`（547 行・登記 42 件→43・手書きの数 42→43）・`emo2_boot/{mod,runtime_boot}.rs`・`emo2_boot/frame/dpi.rs`・新しい `emo2_boot/property_get_cue.rs`（UI の側の段を使うなら新しい `frame/property_get.rs`＋`frame.rs`・`frame/wiring.rs`）・新しい `ghost_session_scope_scale.rs`・`ghost_session.rs`（940 行・＋3 行）・`ghost_session_runtime.rs`・kanade の `schedule/{events,types,mod}.rs`（`change.rs` か新しい兄弟）・`msg.rs`・`request_origin.rs`・`actor.rs`・`msg_tests.rs`・`placement/{descript_window_keys,mod}.rs`（`mod.rs` は 970 行・1〜2 行）・場合により `placement/config.rs`・areka-talk の `origin.rs`・`mcp/check_script_judge_tests.rs`／②＝emo-text の `draw_metrics.rs`・`choice.rs`（案 2 なら `layout.rs`）・`tests/line_pitch_readback_test.rs`・§8（実物の書体の読み戻し＝排他テストの机で短く 1 回）／③＝`crates/areka/src/alert.rs` だけ（**約束**: 件 2〔モニタを移した後のバルーン〕はこのウェーブで取らない＝`emo2_boot/frame/dpi.rs` に触らない・完了の棚卸で渡す・実機は副のモニタで `GetDpiForWindow` を読む短い 1 回）／④＝`crates/temp-path-kit/src/{lib,lib_tests}.rs`・`crates/log-capture-kit/tests/temp_path_guard_test.rs`（**約束**: 入口の差し替えだけ・例外の表〔`shiori-host32-host/tests/lifecycle_*_e2e.rs`〕に触らない・`areka-test-binary-stability` を静かな机で測っているあいだは着地させない）／⑤＝emo-atlas の `lib.rs`（か新しい兄弟）・emo-compose の `fold.rs`・emo-present の `shell_target.rs`（917 行）・`balloon.rs`・`placement/measure.rs`（**約束**: 案 1 を取り、`placement/mod.rs`〔例外 2 の 825 行目〕・`ghost_session.rs`・`emo2_boot/assets.rs` に触らない＝例外 2 は完了の棚卸で渡す）／⑥＝kanade の `schedule/resources.rs`・`menu/captions.rs`・`shiori.toml` の 1 行（**約束**: ⑴ の古い名前 `updatebuttoncaption` だけ・kanade の `schedule/{mod,events,types}.rs` に触らない・⑵〔消える・おすすめ・ポータルの項目〕は取らない＝windows-reactor の作り直しの後へ回す〔2026-10-31 開発者「メニュー関係はreactor解放されたら大改造する」・推奨案を承認〕・完了の棚卸で windows-reactor 待ちの段へ渡す・実機は「．さくら」の右クリックを SSP と短く 1 回見比べる＝排他テストの机） |
| **次のウェーブの候補**（次の棚卸で組む・C15 の着地で brief が動く） | **今の版は v0.1.0**。下の「その先のウェーブ」が見込み（C16）。C15 に入れなかった今の版の spec＝`balloon-communicatebox`（`consumer_ledger*`・`emo2_boot/{mod,runtime_boot}.rs`・kanade の `schedule/events.rs` で ① の後）。**v0.1.0 の着地まで約 2 ウェーブ（C15〜C16）**（`consumer_ledger.rs` に行を足す残りの 2 本を 1 本ずつ）。その後、ウェーブの外の 3 本と `doc/pasta-real-run.md` の実機の確かめが済めば `/release 0.1.0`。／**割り込み**の残りの候補（席が空いたら）＝`background-result-tick-wake`（6〜10・① が `tick_gate_config_producers_tests.rs` に行を足すかが分からない＝迷ったので空けた）・`clippy-199-lints` の段 1（3〜6・切り出すかは開発者）・`dressup-change-events`（6〜10・棚卸㉜で版に縛らない列に置いた・kanade の `schedule/events.rs` を ① と `balloon-communicatebox` と分け合う＝早くて C17）。上限が 11 を超えて入れなかったもの＝`config-parse-diagnostics`（10〜14）・`dev-folder-alias`（10〜14）・`ukadoc-catalog-refresh`（12〜16・台帳の鍵の付け直しで ① の台帳の行と重なる）。先頭でないもの＝`talk-fast-forward`（19〜24・Fable）・`balloon-font-file`（文字とバルーンの列で `range-choice-tag` ほかの後）。見直しの 33 本と本体の crate 分けの 6 本は v0.1.0 の版の門の後。 | — | 先頭の判定は棚卸㉜の測り（各 brief の「2026-10-31 棚卸㉜の再測定」）。 |
| **ウェーブの外** | 測ることが本体（静かな机で 1 本ずつ）＝`areka-test-binary-stability`（v0.1.0・新しい穴＝✅ `balloon-scroll-arrows` が足した `emo2_boot/frame_test_support.rs` の `manual_balloon_runtime` が `GpuPermit::take()` を取らずに `GraphicsCore::new()` を呼ぶ〔`tech.md` の GPU の決まりに反する〕＝切り分けの対照から外す）・`present-emit-tail-latency`・`install-live-target-hazards`・`real-run-input-guard-observations`・`helper-first-load-refusal`／裁定待ち＝`popup-menu-shell-look`（v0.1.0・生きている決まり 6「オーナードローと着せ替えメニューは予約のまま」を解くかを開発者が決めるまで並べない。着せ替えのメニューだけなら Opus、Win32 の描き分けまでなら Fable のまま。ただし ✅ `shell-mouse-cursor` の `CursorQuery`〔窓の手続きの中で同期で答え、World を借りられないときはスレッドごとの控えで答える〕が近い前例になった＝Fable の理由は弱まった）／外の待ち＝`winget-release-automation`（v0.1.0・winget-pkgs の手提出 PR 450070 は 10-31 も開いたまま〔最後の動きは 10-11・`gh` で確かめた〕）／余力の枠＝`multi-ghost-runtime`（24〜30・要件で前半「席と区別」〔12〜15・Opus〕と後半「止まり方と記憶」〔10〜14・**Fable**＝N 体の終わり方の順の約束〕に切る） → `ghost-call`（後半の後・v0.1.0 の残りが待ちで止まったときに） | — | 机の空きを開発者が決めたときに 1 本ずつ挟む。重い走行は赤が出たテストに絞り、全体の負荷の走行は最後の 1 回だけ（1 回 約 50 分） |
| **保留** | `tick-gate-adoption` | — | 「長い試行はしない」と両立する短い A/B の測り方を先に組む。計測を汚すので他と並べない |

**C15 の開始コマンド**（`/kiro-next` の報告と同じ形・俯瞰の HTML がこの塊と下の「その先のウェーブ」の塊を 1 つにしてコピーのボタンつきで出す・`/kiro-wave` はこの塊だけを読む）:

```
Wave C15 6本（v0.1.0 1本＋割り込み 5本）
【本来の次の版 v0.1.0】
Opus
/kiro-start shell-scale-linkage

【割り込み（版に縛らない・v0.1.0 の着地を遅らせない）】
Opus
/kiro-start emo-text-line-gap
/kiro-start alert-dialog-per-monitor-dpi
/kiro-start test-roots-under-target
/kiro-start emo-bake-log-overlap
/kiro-start popup-menu-extra-resources
```

### その先のウェーブ（2026-10-31 の見込み）

C15 が全部着地した前提の見込み。次の棚卸で測り直して確かめる（割り込みは棚卸ごとに選ぶので見込みには書かない）。C16 で v0.1.0 のウェーブの spec は 0 本になるが、`/release 0.1.0` はその後、ウェーブの外の 3 本（`areka-test-binary-stability`・`popup-menu-shell-look`・`winget-release-automation`）と、`doc/pasta-real-run.md` の実機の確かめが済んでから。

| ウェーブ | spec | 入る条件 | 要件定義 |
|---|---|---|---|
| C16 | `balloon-communicatebox` | `shell-scale-linkage` の着地（`consumer_ledger*`〔登記 43→45〕・`emo2_boot/{mod,runtime_boot}.rs`・kanade の `schedule/events.rs`・両方が UI の側の段を足すなら `frame.rs`・`frame/wiring.rs`） | Opus（棚卸㉛で Fable から改めた） |

```
Wave C16（見込み）1本（v0.1.0 1本）
【本来の次の版 v0.1.0】
Opus
/kiro-start balloon-communicatebox
```

### 版上げと winget の決まり（棚卸㉒の「初回リリースの段取り」のうち今も効くもの）

- **版上げの PR を出すとき、`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR は 0 本**。ほかの PR が開いていても、タグは版上げの squash コミットに打つので Release の中身は決まる。字の上で衝突しないまま `Cargo.lock` が古くなる（`--locked` の組み立てが落ちる）のを防ぐため、依存を足す spec とは同じ時に走らせない。C5 の席は `mcp-stdio-bridge` が使い、10-11 の着地で空いた。`/release` の `v0.0.3` を打つのは、「版の段」の v0.0.3 の 5 本（`baseware-root-list`・`user-data-root`・`write-failure-notice`・`balloon-text-bugs`・`app-icon`）が着地してから（2026-10-15 開発者）。2026-10-17 に `emily-ghost-verification` を v0.0.3 へ足した（開発者）＝この 6 本目の着地も待つ（✅ 10-18 着地）。
- **winget の順の決まり**: winget-releaser は既に winget-pkgs に在るパッケージしか更新できない。初回の手提出（人の承認・実例で約 2 日）が取り込まれるまで `winget.yml` を main へ入れない（入れた後にタグを打つと `winget.yml` が赤になる）。棚卸㉓で、手提出まで（`winget-manifest-submission`）と、取り込まれた後の自動化（`winget-release-automation`）に分けた。✅ `winget-manifest-submission` は 2026-10-10 に着地した（手提出の PR＝https://github.com/microsoft/winget-pkgs/pull/450070 ・自動の検査は通過・人の承認待ち＝まだ取り込まれていない）。`winget-release-automation` を始めるのは、**この PR が winget-pkgs に取り込まれ、かつ `user-data-root` が着地してから**（✅ `user-data-root` は 2026-10-17 に着地＝残るのは取り込み待ちだけ）（それまで、説明書に winget の行を載せず、winget-pkgs へ次の版を出さない＝「winget-manifest-submission の手元の確かめで見つかった件」の「順の決まり」）。

### 直列の列（同じ列の spec は同時に走らせない・列が違えば並べられる）

棚卸⑳の再測定で、**同じ場所へ行を足す spec の群れ**が見つかった。ファイルを分割しても重なりは消えない（同じ表・同じ関数へ足すため）。ウェーブは、各列の先頭から 1 本ずつ取り、取った組の接触ファイルを照合してから組む。棚卸㉒で kanade の列の並びを優先度の段に合わせ、シェルの element の列から `self-alpha-declaration` を外した。

| 列 | 重なる場所 | 並び（先頭から・✅＝完了） |
|---|---|---|
| **文字とバルーン** | `crates/areka-emo-text/src/`（`state.rs` の `CueCommand::Custom` の腕・`actor*.rs`・`viewbox_draw*.rs`・**`lib.rs` の純粋な一覧**＝新しいファイルを足す spec はどれも触る）・`crates/areka-parsers/src/balloon/{model,parse}.rs`・`crates/areka/src/input_events/` のバルーンと箱（`shell_box.rs` の `judge_box_press`・`user_break.rs` の `on_box_press`） | ✅ `emo-text-file-split` → ✅ `shell-balloon` → ✅ `shell-balloon-frame-align` → ✅ `budoux-reveal-reflow`（バグ・C4-②・10-06 着地）→ ✅ `anchor-tag-canon`（働き・C5-①・10-10 着地）→ ✅ `balloon-text-bugs`（v0.0.3・C7-②・10-16 着地）→ `range-choice-tag` → `link-context-copy` → `balloon-link-hover`（10-05 起票の「バルーンのリンクと OS の連携」・開発者の依頼で優先度の高い機能として `text-typesetting` 以降より前へ置いた）→ `balloon-font-file`（`actor_present.rs`・`lib.rs` が ② と、`actor.rs`・`viewbox_draw_render.rs` が `anchor-tag-canon`〔✅ 10-10 着地〕と重なる。リンクの列のどこかと重ならない席が見つかれば前へ）→ `text-typesetting`（② の「出した字の行を動かさない」約束の上に乗る）→ `talk-fast-forward` → `balloon-markers` → `text-ruby` → `text-reveal-fade` → `balloon-scroll-fade` → `text-align-shadow-canon` → `choice-marker-styling` → `anchor-style-canon`（`look.rs`・`state_decoration.rs`・`viewbox_draw_render.rs`・`viewbox_draw_decoration.rs` を分け合うので直列）→ `text-reveal-dance`（夢）。**`choice-marker-styling` を列から外して並べられた相手（`budoux-reveal-reflow`）は着地した**。`choice.rs` を分け合っていた `anchor-tag-canon` も 10-10 に着地した＝棚卸㉔の実測では、新しいファイルを足さなければ今の列の頭（`range-choice-tag`）と並べられる。**列の共通の約束**: 新しいキーはバルーン定義ごとの値（`ResolvedBalloonText` の側）に置く。fade・scroll-fade・dance は「動いている途中の字を見えている字に数えるか」の答えを、当たりの矩形・箱の写し（`actor_box.rs` の `refresh_shown_boxes`）・窓を出す数（`balloon_shown_glyphs`）の 3 か所で揃える。emo-text の `judge_box_click`・`BoxPressVerdict` は 2 値でなく 4 値（棚卸㉑の記述の誤り・棚卸㉒で直した） |
| **シェルの element** | `crates/areka-parsers/src/shell/{model,decode}.rs`・`crates/areka-emo-compose/src/{plan,plan_extent,plan_always,fold,method,atlas_bind}.rs`・`crates/areka-emo-atlas/src/manifest.rs`・`crates/areka-seriko/src/{table,looper,parts,actor}.rs` | ✅ `shell-balloon` → ✅ `surface-element-nesting` → ✅ `element-base-method`（10-05）∥ ✅ `animated-image-playback`（本体・C4-⑫・`bind_atlas` の直後で分解・10-07 着地）→ **`extent-element-offset`（C5-④・`plan_extent.rs` だけ）** ∥ ✅ `seriko-trigger-intervals`（C5-②・10-10 着地・棚卸㉓で分けた後＝読み手の `shell/{model,decode}.rs` と seriko だけ）→ `animated-image-import`（読み手・`manifest.rs`・`plan.rs`・`plan_extent.rs`・seriko の `table.rs`・`parts.rs`・棚卸㉕で前へ）→ `collisionex-regions`（読み手・v0.2.0）→ `seriko-interval-combinations`（棚卸㉓で切り出し・`plan_extent.rs`・`plan.rs`・`nesting.rs`・`plan_always.rs`）→ `element-clipping-option` → `draw-methods-canon`（読み手と描けるかの門・`plan.rs`・`blit.rs`）→ `draw-methods-blend`（棚卸㉓で切り出し・`method.rs`・`blit.rs`）→ `balloon-element-order`（wintf の兄弟の重なり順の裁定の持ち主）→ `surfaces-basepos`（据え置き）。parsers の `shell/{model,decode}.rs` を触る 5 本（`draw-methods-canon`・`collisionex-regions`・`seriko-trigger-intervals`〔✅ 10-10 着地〕・`animated-image-import`・`element-clipping-option`）と、compose の `plan.rs`（子の `plan_extent.rs` と、兄弟の `plan_always.rs` を含む）を触る 5 本（`draw-methods-canon`・`extent-element-offset`・`animated-image-import`・`element-clipping-option`・`balloon-element-order`）はそれぞれ互いに直列。**`self-alpha-declaration`（C4-⑬）は列から外した**（本番の決め打ちは `shell_target.rs`・`balloon.rs` の 2 か所だけで、`manifest.rs`・`atlas_bind.rs` の 3 か所は試験の中だった＝棚卸㉑の記述の誤り） |
| **kanade の進行** | `crates/areka-kanade/src/schedule/{steady,events,mod,change,boot}.rs`・`msg.rs`・`actor.rs`・`lib.rs` の `pub use`。**`steady.rs` 957 行・`schedule/mod.rs` 969 行・`msg.rs` 930 行・`actor.rs` 901 行（10-10 `anchor-tag-canon`・`farewell-talk-status` の後）は上限の近く**＝足して 1,000 を超える spec が先頭のタスクで分割するか、新しいファイルへ足す | ✅ `translate-pipeline` → ✅ `mouse-drag-events` → ✅ `balloon-lifecycle-events`（C4-⑩・10-08・`schedule/`）∥ ✅ `mcp-get-status`（C4-⑥・10-06・`msg.rs`・`actor.rs`＝列の外へ出せる組）∥ ✅ `choice-script-prefix`（C4-⑭・10-07・`schedule/choice.rs` とその子・`steady.rs` の `on_choice`＝⑩ と ⑥ のファイルに触らない組。`script:` の台本をトークとして走らせる口は、後の `mcp-kanade-tools` の `sakurascript` が使い回せる）→ `mcp-kanade-tools`（`sakurascript`・`raise_event`・許可の表の迂回を作る）→ ✅ `mcp-reload`（10-22 着地・`src/change.rs`・`schedule/boot.rs`）→ `script-security-level`（出どころの運搬・host32-host の列も）→ `mcp-strict-errors` ／ その他の段＝`sakura-time-critical` → `property-query-channels`（棚卸㉒で優先の段の後ろへ下げた） |
| **台本のコンパイル** | `crates/areka-sakura/src/compile.rs`・`crates/areka-parsers/src/sakura/{lexer,decode,model}.rs` | ✅ `choice-timeout-directive` → ✅ `open-external-tags`（C4-⑮・`\j` の腕）→ ✅ `mcp-author-tools`（C4-⑨・`parse_noted` の印＝`decode.rs` の各腕が命令と一緒に印を返す形・`compile.rs` の `parse_choice_timeout` の公開。後の spec はこの上で腕を足す）→ ✅ `anchor-tag-canon`（働き・C5-①・10-10 着地）→ `range-choice-tag`（`\__q` の腕）→ `talk-fast-forward` → `sakura-time-directives` → `balloon-break-position`（その他の段・`TalkDone` まで位置を通す）。`\i[ID]` は棚卸㉓で `seriko-script-triggers` へ切り出した（`compile.rs`・`sakura/decode.rs` を触る＝✅ `anchor-tag-canon`〔10-10 着地〕の後）。`sakura-time-critical` は `\t` を `decode_bare` で写せば `compile.rs` に触れないが、`sakura/decode.rs` はこの列の共有のファイルなので列の中に居る（棚卸㉓） |
| **host32-host** | `crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/{runtime.rs, shiori_inproc.rs}`・kanade の `shiori/real.rs` | ✅ `script-security-level`（`SenderType`・`SecurityLevel` の運搬の持ち主＝棚卸㉒・10-23 着地）→ `mcp-shiori-query`（`X-MCP-PassThru-*`）→ `makoto-dll-host`（新規 `makoto.rs`・`runtime.rs`・`Cargo.lock`）→ `makoto-reload-directives` → `property-ipc-transport`（輸送を作るときだけ）。`property-query-channels` は運搬を `script-security-level` に寄せて列から外れた |
| **配布と公開** | `tools/package.ps1`・`.github/workflows/`・`Cargo.toml` 群・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（版上げで約 30 行）・`dist/README.txt`・`README.md` | ✅ `release-package-versioned` → ✅ `release-ci-workflow` ∥ ✅ `crates-io-publish` → ✅ `tools-utf8-child-output`（バグ・10-06 着地・`package.ps1`・`crates-io.ps1`・`release.yml` の読み方）→ ✅ `release-cycle` の初回（`v0.0.2`・10-06）→ ✅ `release-notes-highlights`（10-11 着地・`release.yml`・`tools/release-notes.ps1`・`doc/release-notes/`・`release-cycle` の文書）→ ✅ `winget-manifest-submission`（C5-⑧・10-10 着地・手提出の PR は自動の検査を通過して人の承認待ち）→ ✅ `user-data-root`（C8-①・10-17 着地）→ `winget-release-automation`（棚卸㉓で切り出し・手提出が winget-pkgs に取り込まれ、かつ `user-data-root` が着地してから）∥ ✅ `mcp-stdio-bridge`（10-11 着地）→ `install-live-target-hazards`（`dist/README.txt` の既知の制限）→ `release-code-signing`（任意・同梱の i686 `pasta.dll` は署名されないまま残る＝議題） ∥ `release-notes-highlights`（10-11 起票・触るのは `release.yml` と `release-cycle` の文書＝`release-cycle` の次の回〔`v0.0.3`〕より前に着地・`release-code-signing` とは `release.yml` で直列） |
| **`emo2_boot` の結線** | `crates/areka/src/emo2_boot/{mod,ghost_switch,consumer_ledger,shell_balloon_switch,switch_assets}.rs`・`frame/{attach,switch,wiring}.rs` | ✅ `shell-balloon-frame-align` → ✅ `balloon-lifecycle-events`（C4-⑩・10-08・`mod.rs`）→ `balloon-font-file`（`frame/attach.rs`・`frame/switch.rs`）→ `balloon-canon-residue` ∥ `shell-companion-balloon`（`frame/switch.rs` を分け合うので同時には走らせない）→ ✅ `mcp-reload`（10-22 着地・`mod.rs`・`ghost_switch.rs`・`shell_balloon_switch.rs`）。`consumer_ledger.rs` は ✅ `open-external-tags`（C4-⑮・`open` の受け取り手・21 行）と ✅ `mcp-author-tools`（C4-⑨・2 行を足して 23 行）が先に触り、`mcp-reload` は後→ `extra-character-windows`（`switch_assets.rs` も）→ `currentghost-property-tree` → `currentghost-property-others`・`system-property-values` → `property-catalog-lists` |
| **プロパティの動く値** | `crates/areka-sylphya/src/{actor,mirror,key,reader,vocab/dotted}.rs`・`persist/`・`areka-ghost/src/sylphya_wiring.rs` | ✅ `property-name-case-fold`（10-23 着地・`classify_set` の大小の迂回をふさぐ＝`property-query-channels` が書く道を開く前に着地させる）→ `currentghost-property-tree`（動く値を出す口の持ち主・読む道は `mcp-get-property` が作った）→ `currentghost-property-others` ∥ `system-property-values` → `property-catalog-lists` → `zorder-property` |

- **棚卸㉓で列に足したこと**（表の行は上の書き換えだけ・ここが足し分の正本）:
  - **列をまたぐ重なり**: 文字とバルーンの列の頭 ✅ `anchor-tag-canon`（10-10 着地）は、`OnAnchorSelect` を送るために kanade の `msg.rs`・`actor.rs`・`lib.rs`・`schedule/{events,mod}.rs` も触る＝kanade の列の頭（`mcp-kanade-tools`・✅ `farewell-talk-status`・`script-security-level`・`sakura-time-critical`）と同じウェーブに置けなかった（どちらも 10-10 に着地したので、この重なりは済んだ話）。
  - **`\!` の名前を足す spec の共有の席**（`mcp-author-tools` の着地でできた）: `check_script` は `crates/areka/src/emo2_boot/consumer_ledger.rs` に無い `\!` の名前を「知らない命令」と答える＝`\!` の名前を足す spec はどれも、この表に行を足し `consumer_ledger_agreement_tests.rs` に見本を足す（`mcp-reload`・`seriko-script-triggers`・`text-typesetting`・`text-ruby`・`text-reveal-fade`・`balloon-markers`・`sakura-time-directives`・`sakura-time-critical`・`property-query-channels`・`sakura-embed-directive`・`makoto-reload-directives`・`inputbox-user-input`）。互いに直列。
  - **カタログの鎖**（`crates/areka-ghost/src/catalog.rs`・`crates/areka/src/{boot_resolve,boot_config}.rs` を 3 本が作り直す）: `baseware-root-list`（C5-⑤）→ `ghost-inner-balloon` → `shell-companion-balloon`。バルーンの項目に「どこから来たか」の欄が要るのは前の 2 本＝先に着地する `baseware-root-list` が決める。`ghost-inner-balloon` の「`shell-companion-balloon` の後」は働きの依存でなくファイルの順だった。
  - **kanade の列**: ✅ `farewell-talk-status`（10-10 着地・`schedule/mod.rs` は判定の本体 1 か所だけ）→ `mcp-kanade-tools`（最初のタスクで `msg.rs` の中のテストを兄弟へ出し、`schedule/mod.rs` を割る。許可の表の迂回は `schedule/change.rs` の `on_raise_event` に作る。`src/change.rs`・`schedule/boot.rs` には触らない＝`mcp-reload` と並べられる）→ `script-security-level` → `mcp-shiori-query`（`mcp-kanade-tools` の後＝棚卸㉒の裁定どおり）→ `mcp-strict-errors`。`mouse-click-wheel-events`・`inputbox-user-input`（棚卸㉓で起票）は `schedule/events.rs` を触るのでこの列。
  - **シェルの element の列の読み手の鎖**（`shell/{model,decode}.rs`）: ✅ `seriko-trigger-intervals`（10-10 着地）→ `collisionex-regions` → `animated-image-import` → `element-clipping-option` → `draw-methods-canon`。`config-parse-diagnostics`（棚卸㉓で起票）は読み手の `kv/parse.rs`・`shell/`・`balloon/parse.rs` を触るので、この鎖と文字とバルーンの列の両方が空いたとき。`balloon-canon-residue` は seriko の `actor.rs` を触るので ✅ `seriko-trigger-intervals`（10-10 着地）の後。
  - **文字とバルーンの列で ✅ `anchor-tag-canon`（10-10 着地）の後に並ぶ小さいもの**: ✅ `balloon-text-bugs`（C7-②・10-16 着地・2026-10-11 に `choice-balloon-timeout-stuck`・`balloon-text-area-collapse`・`underline-bottom-row-clip` をまとめた・`actor_present.rs` ほか）・✅ `choice-ranges-one-function`（10-31 着地）（`choice.rs`・`actor_present.rs`・`actor.rs`）・`font-size-keywords`（棚卸㉓で起票・`look.rs`・`state_decoration.rs`・`mcp/check_script_judge.rs`）。**列の外で取れる**: `reflow-scroll-path-test`（今ある `actor_lookahead_tests.rs` へ足すだけ）・✅ `emo2-real-run-stability`（10-23 着地・試験の側と helper の記録だけで済んだ）。
  - **列に居なかった spec の置き場**: ✅ `emily-ghost-verification`（10-18 着地・`areka-nar` の `plan.rs`・`install/judge.rs`・検体の登録＝どの列とも重ならない。見つけた崩れは 1 件ずつ `/kiro-discovery` で起票）・`dpi-realign-remembered-chain`（`placement/` と `emo2_boot/frame/{dpi,drain_resnap}.rs`＝`emo2_boot` の結線の列のファイルには触らない）・✅ `ssp-bts-salvage`（10-10 着地・文書だけ）・`shiori4-api`（host32-host の列の最後）・`seriko-trigger-lottery-gaps`（2026-10-11 に seriko のバグ 2 本と `seriko-rebuild-hidden-lottery` をまとめた・✅ `seriko-trigger-intervals`〔10-10 着地〕の後・`seriko-talk-clock-fidelity` より先）・`test-roots-under-target`（入口の差し替えなら `temp-path-kit` だけ）・`dev-helper-x64-clobber`（`tools/test-all.ps1`）・✅ `actor-thread-log-capture`（10-31 着地）（`areka-actor`）・`perf-tools-console-encoding`（`tools/perf/`）・`impl-watch`（新しいクレート＝`Cargo.lock` の席）・`elevated-drop-filter`（棚卸㉓で起票・`placement/spawn.rs`）・`check-script-arg-checks`（棚卸㉓で起票・`mcp/check_script_judge.rs` と `emo2_boot/{change_cue,switch_cue,readme_cue}.rs`＝`check_script_judge.rs` を触る spec は互いに直列）。
- **棚卸㉔で列に足したこと**（2026-10-11・表の行は数の書き換えだけ・ここが足し分の正本）:
  - **カタログの鎖の順を 5 本で決めた**（`crates/areka-ghost/src/catalog.rs`・`crates/areka/src/{boot_resolve,boot_config}.rs`）: `baseware-root-list`（C6-③・根の並びと「どこから来たか」の欄を決める）→ ✅ `user-data-root`（C8-①・10-17 着地・働きの上でも続き・`winget-release-automation` と winget の案内がこれを待っていた）→ ✅ `ghost-inner-balloon`（A・10-23 着地）→ ✅ `write-failure-notice`（B・10-17 着地・触るのは `boot_resolve.rs` の終了の印と最後に使ったものの記録の関数・呼び手の `main.rs`・`emo2_boot/ghost_switch.rs`・`install/judge.rs`）→ `shell-companion-balloon`（C）。`default-balloon-selfmade` は鎖の席を取らない（鎖に触れるのは既定のバルーンの名前の定数 1 行と、その名前を期待するテスト 4 本だけ＝既定を差し替えるタスクを最後に置き、main を取り込んでから直す）。「winget の更新で exe の隣のゴーストが消えないようにする」は `user-data-root` の持ち物（`baseware-root-list` の「確かめること」の答え＝入れ替わる）。
  - **検体の登記表の順**（`crates/sample-ghost-kit/src/{lib,lib_tests}.rs`・`vendors/sample_ghost/README.md`＝4 本とも触る・1 本ずつ）: ✅ `emily-ghost-verification`（2026-10-17 に v0.0.3 へ移したので先頭・10-18 着地＝8 本目に登記した）→ ✅ `sample-url-fetch`（URL の欄＋ぱすたの行・8 → 9・10-19 着地）→ ✅ `dot-sakura-specimen`（C6-⑧・9 → 10・10-23 着地）→ ✅ `default-balloon-selfmade`（10 → 11・10-30 着地）。`dot-sakura-specimen` の brief は「7 → 8」と書く＝着地の順で 9 → 10 に読み替える（2026-10-19 `sample-url-fetch` の完了で、棚卸㉗の順に合わせて改めた）。
  - **タスクトレイの順**: ✅ `app-icon`（C7-③・10-16 着地）→ `tasktray-icon`（`main.rs`・`menu/mod.rs`＝`baseware-root-list` の後）→ `tasktray-ghost-icon` → `tasktray-minimize`。`tasktray-balloon` は `tasktray-icon` の後ならどこでも。後ろの 3 本はどれも `emo2_boot/consumer_ledger.rs`（943 行）に行を足すので 1 本ずつ。`tasktray-balloon`・`tasktray-minimize` は kanade の `schedule/events.rs` も触る。
  - **seriko の中の順**（✅ `seriko-trigger-intervals` が `looper_trigger.rs`・`parts_trigger.rs`・`talk.rs`・`trigger.rs` を作った後）: `seriko-trigger-lottery-gaps`（抽選のテストの期待を取り直し、後始末の決まりを定める）→ `seriko-talk-clock-fidelity`（`trigger.rs`・`looper.rs`・`looper_trigger.rs` を分け合う）。`seriko-interval-combinations`・`seriko-script-triggers` は互いにも `seriko-trigger-lottery-gaps` とも直列（`table.rs` 900 行・`looper.rs` 924 行）。`animated-image-import` も `table.rs`・`parts.rs`（893 行）を触る＝足す前に `table.rs` の中のテスト（約 350 行）を兄弟へ出す。
  - **`range-choice-tag` の触る場所が増えた**（✅ `anchor-tag-canon` の作りを使い回す形で測り直した）: dola の `cue/{command,sink,runtime}.rs`・`crates/areka-ghost/src/sink.rs`・`crates/areka/src/mcp/check_script_judge.rs`。`check_script_judge.rs` を触る spec（`font-size-keywords`・`check-script-arg-checks`・`mcp-strict-errors`・`script-impact-tiers`）と互いに直列。kanade には触らない＝`mcp-kanade-tools` と並べられる（棚卸㉖: `balloon-text-bugs` の後は `emo2_boot/talk_lifecycle.rs` にも範囲の合図の腕を 1 つ足す＝「`emo2_boot/` に触らない」は古くなった）。
  - **emo-text の席を使う spec**（新しいソースファイルを足す）: `range-choice-tag`・`balloon-font-file`・✅ `balloon-text-bugs`（C7-②・10-16 着地＝使った）・`anchor-style-canon`・`text-typesetting`・`talk-fast-forward`。使わない: ✅ `choice-ranges-one-function`（10-31 着地）・`font-size-keywords`・`link-context-copy`・`balloon-link-hover`。足さずに済ませられる: `choice-marker-styling`・`real-run-log-noise`。**棚卸㉓の宿題の答え**＝`choice-marker-styling` は新しいファイルを足さなければ列の頭（`range-choice-tag`）と並べられる（`choice.rs` の重なりは ✅ `anchor-tag-canon` の着地で消えた）。
  - **kanade の上限**: `mcp-kanade-tools`（C6-②）が最初に `msg.rs` のテストを出し `schedule/mod.rs` を割る。その後ろの `script-security-level`・`sakura-time-critical`・`mouse-click-wheel-events`・`inputbox-user-input`・`balloon-link-hover`・`talk-fast-forward` は、割った後の形に足す。✅ `farewell-talk-status` の申し送り（「再生中か」の判定を相の側へ移す）も `mcp-kanade-tools` が引き取る。
  - **`main.rs`（956 行・棚卸㉖で 971 行・2026-10-18 は 1,000 行ちょうど＝上限）を触る spec は互いに直列**: `baseware-root-list`・✅ `user-data-root`（10-17 着地・約束を開発者が解いた後に数行触った）・✅ `write-failure-notice`（10-17 着地）・`tasktray-icon`（と、アプリの側で直す形を選んだときの ✅ `emo2-real-run-stability`〔10-23 着地・アプリの側では直さなかった〕）。`main.rs` はいま **1,000 行ちょうど**（上限・`file_length_guard`）＝次に `main.rs` を触る spec（`tasktray-icon`・`multi-ghost-runtime` など）は、最初のタスクで中身を外へ出す必要がある。
  - **emo-present の `balloon.rs`**: `balloon-webp-names`（名前から絵を引く所をそろえる）→ `balloon-canon-residue`。
- **棚卸㉕で列に足したこと**（2026-10-16・表の行の書き換えは 2 か所＝文字とバルーンの列の頭に `balloon-text-bugs`、シェルの element の列で `animated-image-import` を `collisionex-regions` の前へ。ここが足し分の正本）:
  - **カタログの鎖の順を版の段に合わせて改めた**: `baseware-root-list` → ✅ `user-data-root`（10-17 着地） → ✅ `write-failure-notice`（10-17 着地） → ✅ `ghost-inner-balloon`（10-23 着地） → `shell-companion-balloon`（棚卸㉔の順の 3 本目と 4 本目を入れ替えた）。`ghost-inner-balloon`（版に縛らない）が `write-failure-notice`（v0.0.3）の前に居たのはファイルの順だけ（重なりは `boot_resolve.rs` の別の関数）で、前の版が後の列を待つ逆向きになっていた。`write-failure-notice` は `baseware-root-list` と並べられない（起動中の印の名前を引く `running_name(&ctx.root, …)` の呼び手＝`main.rs`・`boot_resolve.rs`・`emo2_boot/ghost_switch.rs` を `baseware-root-list` が根の一覧へ替える）。
  - **文字とバルーンの列の頭**は ✅ `balloon-text-bugs`（v0.0.3・C7-②・10-16 着地）→ `range-choice-tag`。棚卸㉔までは `range-choice-tag` の後ろに居たが、働きの依存は無い（emo-text の席と `actor.rs` の順だけ）。
  - **シェルの element の列**: `collisionex-regions` が v0.2.0 へ移ったので、v0.1.0 の `animated-image-import` を前へ出した（`extent-element-offset` → `animated-image-import` → `collisionex-regions`）。`animated-image-import` と `seriko-trigger-lottery-gaps` は seriko の `table.rs`・`parts.rs` で直列（働きの前後は無い）。
  - **`multi-ghost-runtime` は v0.1.0 の大半とファイルが重なる**（`main.rs`・`emo2_boot/`・kanade の `msg.rs`・`placement/{spawn,persist}.rs`・`ghost_session.rs`）＝カタログの鎖の 3 本・`mcp-kanade-tools`・`mcp-reload`・`dpi-realign-remembered-chain` の後にほぼ単独で走らせる。`dpi-realign-remembered-chain` を先に着地させ、窓の鎖をゴーストごとに分けるときに直した判定を持ち込む。`GhostSlot` を読むファイルはテストでないものだけで 22（brief の数より多い）。
  - **`ghost-call` は `mcp-reload` と直列**: `OnGhostCalled` が 204 なら `OnBoot` へ落ちるので、kanade の `src/change.rs`（起動の出どころの型に 1 つ足す）と `schedule/boot.rs` を触る。`emo2_boot/consumer_ledger.rs` も分け合う。
  - **✅ `placement-measure-bake-once`**（10-31 着地）は案 B（測る側で済ます）を取った＝案 A の `multi-ghost-runtime`・`user-data-root` との重なりは無くなった。続きの `emo-bake-log-overlap` は案 1 なら emo-atlas・emo-compose・emo-present の `balloon.rs` の記録の口と `placement/{measure,mod}.rs`、案 2 なら `ghost_session.rs` に触れる。
  - **プロパティの動く値の列**は `reader.rs` と `persist/` も含む（✅ `property-name-case-fold` が保存した名前まで畳むなら `baseware-root-list`・`write-failure-notice`・`multi-ghost-runtime` と重なる＝C7 では畳まない約束）。
  - **`mcp-receiver-bugs` の重なりは `mcp-strict-errors`（`crates/areka-mcp/src/handler.rs`）だけ**（同 brief が `mcp-kanade-tools`・`mcp-reload` も `handler.rs` を触ると書くのは誤り）。
  - **`default-balloon-selfmade` が触るカタログの鎖のファイル**は `boot_resolve.rs` の既定のバルーンの名前の定数 1 行だけ（`catalog.rs` には触らない）。
- **棚卸㉖で列に足したこと**（2026-10-16・表の行は書き換えていない・ここが足し分の正本）:
  - **カタログの鎖**: `baseware-root-list` の着地で、`running_name` は根を取らなくなった（呼び手 3 か所＝`main.rs`・`emo2_boot/ghost_switch.rs`・`emo2_boot/same_folder_pin.rs`＝`user-data-root`・`write-failure-notice` のどちらも変えない）。根の並びの決め方は `crates/areka/src/roots/` へ移った。`user-data-root` と `write-failure-notice` は、ウェーブ C8 の約束（`user-data-root` は `main.rs`・`boot_resolve.rs`・`roots/edit.rs` に触らない／`write-failure-notice` は `boot_config.rs`・`roots/{mod,resolve}.rs` に触らない）で同じファイルに触らずに並べる。`dev-folder-alias` は `user-data-root` と `roots/{mod,resolve}.rs` で直列。
  - **`main.rs` は 971 行**（`baseware-root-list` の後・`write-failure-notice` の後は 1,000 行ちょうど）。`write-failure-notice` は知らせの結線を新しいファイルか `boot_resolve.rs` へ置く。✅ `user-data-root` は C8 の約束で触らない形で進み、main を取り込んだ後に開発者が約束を解いて数行だけ触った（10-17 着地・974 行）。✅ `write-failure-notice`（10-17 着地）の後は **1,000 行ちょうど**（上限・`file_length_guard`）＝次に `main.rs` を触る spec（`tasktray-icon`・`multi-ghost-runtime` など）は、最初のタスクで中身を外へ出す必要がある。
  - **文字とバルーンの列**: ✅ `choice-ranges-one-function`（10-31 着地）は `actor.rs` に触らずに済む（決まった寸法をテストの側から渡す）＝`range-choice-tag` と並べられる。`real-run-log-noise` は `region.rs`・`actor_attach.rs`・`crates/areka-emo-atlas/src/lib.rs`＝列の頭とも ✅ `choice-ranges-one-function`（10-31 着地）とも重ならない。どちらも emo-text に新しいファイルを足さない。
  - **中継の控えの置き場**（`crates/areka-mcp-bridge/src/config.rs`）は areka の記憶の置き場の決め方を写している＝`user-data-root` が持つ（`mcp-stdio-bridge-residue` は触らない）。✅ `user-data-root` が 10-17 に、areka と同じ決め方（`areka-user-data`）で決める形にした。
  - **`test-roots-under-target` は前後の静かな全体テストの比べが要らない**（すでに 29 のテストのファイルが `target\` の下を使う）。入口の差し替えだけなら `temp-path-kit` と `log-capture-kit` のテスト 1 本。
- **棚卸㉗で列に足したこと**（2026-10-18・表の行は書き換えていない・ここが足し分の正本）:
  - **起動オプションの列（新）**（新しい `crates/areka/src/cli/` を 4 本が分け合う）: ✅ `startup-options-core`（C9-②・10-19 着地）→ ✅ `offline-surface-dump`（10-23 着地） → ✅ `readonly-mode`（10-24 着地） → ✅ `startup-install-option`（10-30 着地）。`offline-surface-dump` と `readonly-mode` は働きの前後は無いが `cli/` を分け合う＝1 本ずつ。`readonly-mode` と `startup-install-option` は `install/desk.rs` も分け合う。`startup-options-core` は `main.rs` の中身を `cli/` へ出して減らす＝`main.rs` を触る列（`tasktray-icon`・`multi-ghost-runtime`）の前に置くと後ろが楽になる。
  - **カタログの鎖に v0.1.0 の 3 本が入った**: `startup-options-core`（`boot_resolve.rs`・`boot_config.rs`）・`ghost-inner-balloon`（同じ 2 つと `catalog.rs`＝棚卸㉗で、ゴーストのフォルダの直下に名前で置いたバルーンを探す形へ広げた）・`default-balloon-selfmade`（`boot_resolve.rs` の既定のバルーンの名前と、そのテスト＝`boot_resolve_tests.rs`・`main_config_input_tests.rs` は `startup-options-core` も触る）は互いに直列。`readonly-mode` は `boot_resolve.rs` の書く関数（`save_app` ほか）だけ＝別の関数だが同じファイルなので直列。
  - **検体の登記表の順を改めた**（`crates/sample-ghost-kit/src/{lib,lib_tests}.rs`・`vendors/sample_ghost/README.md`）: ✅ `sample-url-fetch`（C9-⑤・URL の欄＋ぱすたの行・8 → 9・10-19 着地＝書庫をこのリポジトリに複製しない）→ ✅ `pasta-in-windows-verification`（10-23 着地・登記表に触らない＝登記は ✅ `sample-url-fetch` が済ませた・中身の自動テストだけ足す）→ ✅ `dot-sakura-specimen`（10-23 着地） → ✅ `default-balloon-selfmade`（10-30 着地）。棚卸㉔の順（`dot-sakura-specimen` → `default-balloon-selfmade` → `sample-url-fetch`）は働きの依存でなくファイルの順だったので、v0.1.0 の 2 つ目のゴール（ぱすた）を先にした（`default-balloon-selfmade` は実機の重ね合わせの確かめに `dot-sakura-specimen` の検体を使う＝この 2 本は働きの前後）。登記の数は 8 → 9（ぱすた）→ 10 → 11（各 brief の「8 → 9」などは着地の順で読み替える）。`dot-sakura-specimen` を書庫で置くか URL の行で置くかは同 brief の議題 4。`real-run-mcp-kit` は C9 では新しいクレートに置く約束（登記表の `lib.rs` に触らない）。
  - **kanade の列**: ✅ `farewell-talk-status` → ✅ `mcp-kanade-tools`（C9-③・10-19 着地）→ ✅ `script-security-level`（C10-①・10-23 着地）→ ✅ `mouse-click-wheel-events`（10-24 着地・棚卸㉗で v0.3.0 から v0.1.0 へ・`msg.rs`・`schedule/events.rs`・`schedule/boot.rs`）→ `mcp-shiori-query`。`mcp-strict-errors` は ③ ④ ⑧ の後。✅ `mcp-reload`（C9-④・10-22 着地）は ③ と約束で並んだ（`schedule/boot.rs` は ④ の持ち物）。
  - **重なり順の列（新）**: ✅ `zorder-scope-inversion`（C9-⑦・10-19 着地・wintf の `zorder_pair*.rs`・`zorder_chain*.rs`・`placement/zorder_group_ledger.rs`）→ ✅ `stay-on-top-directives`（10-31 着地・同じ wintf のファイル＋`placement/spawn.rs`・この列の前は着地した＝こちら側の待ちは無い）。`zorder-property`・`zorder-chain-residue` も同じ場所。
  - **窓の位置の列**: ✅ `window-link-directives`（10-23 着地） → `dpi-realign-remembered-chain`（ぱすたの「窓」をぱすたに重ねたまま、拡大率の違う画面をまたぐ）。✅ `descript-window-keys`（10-24 着地）は `placement/config.rs`・`drag_follow.rs` で `window-link-directives` と、`placement/measure.rs`・`emo2_boot/assets.rs` で `placement-measure-bake-once` と直列。
  - **`emo2_boot/consumer_ledger.rs`（960 行）に行を足す v0.1.0 の spec は 1 本ずつ**: ✅ `mcp-reload`（C9-④・10-22 着地）→ ✅ `window-link-directives`（10-23 着地）・`script-small-tags`・✅ `descript-window-keys`（10-24 着地）・✅ `stay-on-top-directives`（10-31 着地）。✅ `mcp-strict-errors`（10-30 着地）は行を足さなかった＝この鎖から外した（棚卸㉛）。`script-security-level` は 2026-10-22 の設計で行を足さないと決めた（受け口を新しく足さず、`consumer_ledger.rs` は読むだけ）＝この鎖から外した。
  - **入力の列**: ✅ `mouse-click-wheel-events`（10-24 着地・`input_events/` に当たりの上の出入りを追う所を作る・棚卸㉗で `OnMouseEnter`／`OnMouseLeave` を範囲に入れた）→ ✅ `shell-mouse-cursor`（10-30 着地・同じ所を使う）→ `shell-tooltip`（版に縛らない）。
  - **台本のコンパイルの列**: `script-small-tags`（`\_u`・`\_m`・`\![bind-noevent]`・`\![*]`）と ✅ `stay-on-top-directives`（`\v`・10-31 着地）は `crates/areka-parsers/src/sakura/decode.rs` を触る＝互いに、また `range-choice-tag` ほかと直列。
  - **メニュー**: ✅ `popup-menu-residue`（10-31 着地）。`popup-menu-shell-look` は `menu/` を `popup-menu-extra-resources`（kanade の `schedule/resources.rs` と `menu/`）と分け合う（同じウェーブに置かない）。
- **棚卸㉘で列に足したこと**（2026-10-22・表の行は書き換えていない・ここが足し分の正本）:
  - **行数の改め**: `crates/areka/src/main.rs` は 986 行（棚卸㉛・残り 14 行。その前は 965 行＝棚卸㉙で 978 行＝`offline-surface-dump` が足した・棚卸㉚で 981 行＝`readonly-mode` が足した・残り 19 行・✅ `startup-options-core` が中身を `cli/` へ出した＝上の棚卸㉔・㉖の「1,000 行ちょうど」は古い）。`emo2_boot/consumer_ledger.rs` は 472 行（✅ `mcp-reload` がテストを兄弟の `consumer_ledger_tests.rs` へ出した＝上の「960 行」「943 行」は古い）。行数の圧は消えたが、同じ表へ行を足すので **1 本ずつの決まりは残る**（`window-link-directives`・`script-small-tags`・✅ `descript-window-keys`（10-24 着地）・`stay-on-top-directives`・`mcp-strict-errors` の案 2・`ghost-call`）。`script-security-level` は 2026-10-22 の設計で、この表に行を足さないと決めた（読むだけ）。
  - **kanade の列**: ✅ `script-security-level`（10-23 着地）は外から流した台本の出どころを付けるために `schedule/sstp.rs`・`schedule/types.rs` も触る＝`mcp-strict-errors`（案 1 でも `schedule/sstp.rs`）はその後。✅ `mouse-click-wheel-events`（10-24 着地）との前後はファイルの順だけ。`ondestroy-on-exit`（棚卸㉘で起票・いつか）も `schedule/{mod,close,change}.rs` でこの列。
  - **起動オプションの列**: ✅ `offline-surface-dump`（10-23 着地）・✅ `readonly-mode`（10-24 着地）・✅ `startup-install-option`（10-30 着地）は `dist/README.txt` の起動オプションの同じ 1 文（まだ受けないものの並び）も書き換える＝別々の行の許す重なりにならない・1 本ずつのまま。
  - **カタログの鎖**: ✅ `ghost-inner-balloon`（10-23 着地） → ✅ `readonly-mode`（10-24 着地・`boot_resolve.rs`・`boot_config.rs`・`update/desk.rs`）・✅ `default-balloon-selfmade`（10-30 着地・`boot_resolve.rs`）。`readonly-mode` と `default-balloon-selfmade` も `boot_resolve.rs` で直列。
  - **窓の位置の列**: ✅ `window-link-directives`（10-23 着地・印は `ScriptPlaced`）が連携の持ち主に「並べ直しから外す」印を持たせれば、`dpi-realign-remembered-chain` は `placement/chain_realign.rs` の判定だけを直す形になる（両 brief の棚卸㉘の案）。✅ `stay-on-top-directives`（10-31 着地）と `dpi-realign-remembered-chain` は `placement/spawn.rs` で直列。`placement-measure-bake-once` は案 B なら `shell-mouse-cursor` とも `crates/areka-emo-present/src/shell_target.rs` で重なる（棚卸㉗までの記述に無かった）。
  - **検体の登記表**: ✅ `pasta-in-windows-verification`（10-23 着地）は新しいテストを `crates/sample-ghost-kit/tests/` に置けば登記表（`src/lib.rs`・`lib_tests.rs`）に触らない＝`dot-sakura-specimen` と並べられる（✅ `sample-url-fetch` の登記表ぜんたいのテストが「ゴースト 1・同梱バルーン 0」を既に見ている）。
- **棚卸㉙で列に足したこと**（2026-10-23・表の行は書き換えていない・ここが足し分の正本）:
  - **`consumer_ledger.rs` の鎖**: 547 行・42 件の登記（棚卸㉜・✅ `script-small-tags` が 10-31 に `bind-noevent` と `*` の 2 件を足した後。その前は 529 行・40 件＝✅ `stay-on-top-directives` が 10-31 に 2 件足した後）。行を足す spec は、表の末尾への追記のほかに、手書きの数（`consumer_ledger_tests.rs` の 42＝わざと置いた「母数も判定する」）・説明の一覧・`consumer_ledger_agreement_tests.rs` の `samples()` も直す＝2 本が同じ数を書くと git が黙って通す（記憶「両枝が同じ誤った手書きの数を書くと git は黙って通す」）。v0.1.0 で行を足すのは 5 本＝✅ `descript-window-keys`（10-24 着地）・✅ `stay-on-top-directives`（10-31 着地）・✅ `script-small-tags`（10-31 着地）・`shell-scale-linkage`（→43）・`balloon-communicatebox`（→45）（＋後の版の `ghost-call`・✅ `mcp-strict-errors` は行を足さずに着地した）＝1 本ずつ。
  - **窓の位置の列**: `dpi-realign-remembered-chain` と ✅ `window-work-area-clamp`（10-30 着地） は `placement/{chain_finalize,persist}.rs`・`emo2_boot/frame/drain_resnap.rs` で直列。`dpi-realign-remembered-chain` と ✅ `stay-on-top-directives`（10-31 着地）は `placement/spawn.rs` で直列。✅ `descript-window-keys`（10-24 着地）の件 4 と `window-work-area-clamp` の件 2 は同じ崩れ＝前者が測った面、後者は「自由な窓が後から大きくなる」だけ（両 brief の棚卸㉙の案）。
  - **入力の列**: ✅ `mouse-click-wheel-events`（10-24 着地） → ✅ `shell-mouse-cursor`（10-30 着地・`input_events/` と新しい `hover_region.rs`）。`balloon-scroll-arrows` も `input_events/mod.rs` を触る＝どちらとも別のウェーブ。
  - **文字とバルーンの列**: `balloon-scroll-arrows` は emo-text の席を使う（新しいファイル）。`balloon-scroll-arrows` と `balloon-communicatebox` は `areka-parsers/src/balloon/{mod,model,parse}.rs` で直列。✅ `vertical-choice-hover-band`（10-24 着地）と ✅ `choice-ranges-one-function`（10-31 着地） は `choice.rs` が条件つき（後者が関数を `actor_present.rs` に置けば同じウェーブでよい）。`real-run-log-noise` は v0.1.0 へ上げた（`draw.rs` の呼び手で直せば `vertical-choice-hover-band` と重ならない）。
  - **棚卸㉙で起票した 4 本の置き場**: `balloon-inline-image`＝文字とバルーンの列（emo-text の席を使う・`areka-parsers/src/sakura/decode.rs` で `script-small-tags`・✅ `stay-on-top-directives`（10-31 着地） と重なる）・`ghost-alias-file`＝シェルの element の列（`shell/decode.rs`・emo-present の `shell_target.rs`）・`sstp-receive`＝kanade の列（`schedule/sstp.rs`・`msg.rs`）と MCP の待受（`crates/areka-mcp/src/server.rs`）・`install-package-type`＝インストールの列（`crates/areka-nar/src/manifest.rs`・`install/judge.rs`＝`startup-install-option` と重なる）。
- **棚卸㉚で列に足したこと**（2026-10-24・表の行は書き換えていない・ここが足し分の正本）:
  - **`consumer_ledger.rs` の鎖**: 547 行・42 件の登記（棚卸㉜）。行を足すのは ✅ `stay-on-top-directives`（`(set,windowstate)`・10-31 着地）・✅ `script-small-tags`（`bind-noevent`・`*`・10-31 着地）・`shell-scale-linkage`（`(get,property)`）・`balloon-communicatebox`（`(open|close,communicatebox)`）の 4 本＝1 本ずつ（✅ `mcp-strict-errors` は行を足さずに着地した）。新しい受け手を足す spec は `emo2_boot/mod.rs`（828 行）と `emo2_boot/runtime_boot.rs` にも 1 行ずつ足す（同じファイル）。
  - **seriko の `actor.rs`（799 行）**: `mcp-strict-errors`（案 2＝再生の途中まで印を運ぶ）・`script-small-tags`（bind の濾し・541 行目と 564 行目）・`seriko-pattern-calls` が触る＝同じウェーブに置かない。emo-present の `presenter/show.rs` は `mcp-strict-errors` と ✅ `animated-surface-log-flood`（10-31 着地）が触る。
  - **窓の位置の列**: ✅ `window-work-area-clamp`（10-30 着地） は ✅ `dpi-realign-remembered-chain` の要件の引き渡し（並べ直しの寄せ・起動の隣り合わせの寄せ）で `placement/chain_realign.rs` も触る。寄せる純粋な関数は `placement/follow/` の下に置けば `placement/mod.rs`（969 行）に触らない。
  - **入力の列**: ✅ `shell-mouse-cursor`（10-30 着地） と `balloon-scroll-arrows` は `input_events/mod.rs` で直列（`shell-mouse-cursor` が mod の行を足さない約束なら並べられる）。`balloon-scroll-arrows` のホイールは ✅ `mouse-click-wheel-events` が wintf の `PointerState` へ運ぶ形の上に乗る（受け手の先例は `char_watch.rs`）。
  - **行数の圧**: `main.rs` 981・`placement/mod.rs` 969・`ghost_session.rs` 935・`placement/follow/window_move.rs` 1,221（例外）・seriko の `table.rs` 901・`looper.rs` 924・`areka-parsers` の `balloon/parse_tests.rs` 922・emo-text の `draw_format_metrics_tests.rs` 922。
- **依存を足す spec と版上げは 1 ウェーブに 1 本**（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` が重なる）: C5 は `mcp-stdio-bridge`（新しいクレート・10-11 に着地済み）が席を使った。`impl-watch`（新しいクレート・10-10 に着地済み）・`makoto-dll-host`（新しいクレート）・`system-property-values`（`windows` の機能を足す）・`/release` の版上げも同じ席を使う。`mcp-strict-errors` も使う（10-23 の要件の討議＝rmcp を上げるか、上流への直しを `[patch.crates-io]` で持つ）。
- **網羅台帳をまとめて書き換える `coverage-roadmap-refresh` は、台帳の行を直す spec と同じウェーブに置かない**（台帳に触ると書く未完了の brief が 40 本を超えたので、行の重なりで照合する形を同 spec の議題にした）。
- **列に属さず、いつでも単独で取れる**: `coverage-roadmap-refresh`・✅ `popup-menu-residue`（10-31 着地）・`clippy-199-lints`（段 1＝`dola` の `validate/`・`runtime/`・`compile/` と `shiori-host32-host/tests/` は空き席でも取れる。`shiori-abi` は `shiori4-api` が消すので取らない）・✅ `property-name-case-fold`（10-23 着地・sylphya だけ）。
- **MCP の 3 段目の約束**: SSP と同じ 10 本の中身を作る spec は、今までどおりツールのファイル（`crates/areka/src/mcp/<ツール>.rs`）と自分のエンジンの側だけを触る（`mcp/mod.rs`・`handler.rs` は触らない）。areka 独自のツールを足す spec（`mcp-user-response`・`mcp-shiori-query`）は `doc/ssp-mcp/areka-tools.md` の「⑹ あとからツールを 1 本足す手順」の表のファイル（`crates/areka-mcp/src/tools/mod.rs`・`handler.rs`・`tools_own_tests.rs`・`tools_own_socket_tests.rs`・`crates/areka/src/mcp/mod.rs` と、その文書）を触るので、互いに、また MCP の共有ファイルを触る他の spec と同じウェーブに置かない。`handler.rs` の `INSTRUCTIONS` に残る「not implemented yet」の 1 文は、今までどおり最後に着地する `mcp-strict-errors` が消す（`mcp-author-tools` が足した独自のツールの 1 文は残す）＝`mcp-reload` は `mcp-strict-errors` より先に着地させる。今も `NG:not implemented yet` を返すのは 10 本のうち 0 本（`reload` は 2026-10-22 に `mcp-reload` が、`sakurascript`・`raise_event` は 2026-10-19 に `mcp-kanade-tools` が中身を入れた）。
- **SHIORI への要求に追加のヘッダを運ぶ道**は `script-security-level`・`property-query-channels`・`mcp-shiori-query` の 3 本が要る。入れ物は先に着手した 1 本が作り、`ShioriBackend` の引数を変えずに既定の実装を持つ新しいメソッドを足す形を推す（実装 19 か所への波及を止める）。✅ `script-security-level`（10-23 着地）がこの形で入れ物を作った（SHIORI の受け手の約束に既定の実装つきのメソッド 2 つ）＝残りの 2 本はそれを使う。

**保存義務（据え置き）**: 既存の終了経路（右クリックメニューの「終了」→ `OnClose` の握手 → `ghost_quit`）の決定論テストを 1 本も落とさない。実機サインオフの「絶対パス起動」（argv 上書き）を残す。

## spec 台帳（brief を持つ 162 本・2026-11-01 `popup-menu-extra-resources` の完了で同 spec を外した（163−1・完了時の棚卸＝その場で解決 0 件・起票 0 本・既存の brief への追記 0 件・「直接修正候補」への登記 1 件・`.kiro/specs/` 直下のフォルダ 162 と一致〔completed を除く・brief を持つ 162〕）。その前の 163＝2026-10-31 棚卸㉜で数は変わらない＝`dressup-change-events` を版に縛らない列に置いた・2026-10-31 `script-small-tags` の完了で同 spec を外した（その要件の討議の議題 3 で起票した `dressup-change-events` の行は 10-31 に足し済み＝163+1−1・完了時の棚卸＝その場で解決 1 件〔出ている箱も面の切り替えで変わった印を使う〕・起票 0 本・既存の brief への追記 0 件・`.kiro/specs/` 直下のフォルダ 163 と一致〔completed を除く・brief を持つ 163〕）。その前は 2026-10-31 `install-before-boot` の完了で同 spec を外し、完了時の棚卸で起票した `alert-dialog-per-monitor-dpi` を足した（163−1+1）。その前は 2026-10-31 `placement-measure-bake-once` の完了で同 spec を外し、完了時の棚卸で起票した `emo-bake-log-overlap` を足した（163−1+1・完了時の棚卸＝その場で解決 0 件・起票 1 本・既存の brief への追記 0 件・`.kiro/specs/` 直下のフォルダ 163 と一致〔completed を除く・brief を持つ 163〕）。その前の 163＝2026-10-31 `actor-thread-log-capture` の完了で同 spec を外した（164−1・完了時の棚卸＝その場で解決 0 件・起票 0 本・既存の brief への追記 0 件・`.kiro/specs/` 直下のフォルダ 163 と一致〔completed を除く・brief を持つ 163〕）。その前の 164＝2026-10-31 `popup-menu-residue` の完了で同 spec を外し、完了時の棚卸で起票した `popup-menu-extra-resources` を足した（164−1+1・完了時の棚卸＝その場で解決 1 件〔台帳の `char*.popupmenu` の備考の言い方〕・起票 1 本・既存の brief への追記 0 件・`.kiro/specs/` 直下のフォルダ 164 と一致〔completed を除く・brief を持つ 164〕）。その前の 164＝2026-10-31 `choice-ranges-one-function` の完了で同 spec を外した（165−1・完了時の棚卸＝その場で解決 0 件・起票 0 本・既存の brief への追記 0 件・`.kiro/specs/` 直下のフォルダ 164 と一致〔completed を除く・brief を持つ 164〕）。その前の 165＝2026-10-31 `emo-atlas-palette-alpha` の完了で同 spec を外した（166−1・完了時の棚卸＝その場で解決 1 件・起票 0 本・既存の brief への追記 2 件〔`areka-test-binary-stability` の件 3・件 4〕・`.kiro/specs/` 直下のフォルダ 165 と一致〔completed を除く・brief を持つ 165〕）。その前の 166＝2026-10-31 棚卸㉛で、10-30 に起票した `seriko-legacy-pattern-format` の行を足した（165＋1・`.kiro/specs/` 直下の 166 と一致）。その前の 165＝2026-10-31 `animated-surface-log-flood` の完了で同 spec を外した（166−1・完了時の棚卸＝その場で解決 3 件・起票 0 本・既存の brief への追記 0 本〔実装中に既存の brief へ足した 3 件は「animated-surface-log-flood の申し送り」節〕。`.kiro/specs/` 直下のフォルダ〔completed を除く・brief を持つ〕は 166 で合わない＝`seriko-legacy-pattern-format` を台帳に載せていない件〔次の 167〕のままで、次の棚卸で数え直す）。その前の 166＝2026-10-31 `stay-on-top-directives` の完了で同 spec を外した（167−1・完了時の棚卸の起票は 0 本・既存の brief への追記 0 本。`.kiro/specs/` 直下のフォルダ〔completed を除く・brief を持つ〕は 167 で合わない＝`seriko-legacy-pattern-format` を台帳に載せていない件〔次の 167〕のままで、次の棚卸で数え直す）。その前の 167＝2026-10-31 `balloon-scroll-arrows` の完了で同 spec を外し、完了時の棚卸で `balloon-append-tags` を足した（167−1＋1・完了時の棚卸の起票は 1 本・既存の brief への追記 0 本。`.kiro/specs/` 直下のフォルダ〔completed を除く・brief を持つ〕は 168 で合わない＝`seriko-legacy-pattern-format` を台帳に載せていない件〔次の 167〕のままで、次の棚卸で数え直す）。その前の 167＝2026-10-31 `seriko-pattern-calls` の完了で同 spec を外した（168−1・完了時の棚卸の起票は 0 本・既存の brief への追記 0 本。`.kiro/specs/` 直下のフォルダ〔completed を除く・brief を持つ〕は 168 で合わない＝同 spec の要件の議論で 2026-10-30 に起票した `seriko-legacy-pattern-format` は、その brief の決めで台帳に載せておらず、次の棚卸で数え直す）。その前の 168＝2026-10-30 `bridge-store-user-root` の完了で同 spec を外した（169−1・完了時の棚卸の起票は 0 本・既存の brief への追記 0 本・`.kiro/specs/` 直下のフォルダ 168 と一致〔completed を除く・brief を持つ 168〕）。その前の 169＝2026-10-30 `window-work-area-clamp` の完了で同 spec を外した（170−1・完了時の棚卸の起票 2 本は次の 170 で足した・既存の brief への追記 0 本・`.kiro/specs/` 直下のフォルダ 169 と一致〔completed を除く・brief を持つ 169〕）。その前の 170＝2026-10-30 `window-work-area-clamp` の完了時の棚卸で `balloon-flip-side-rules`・`window-resetwindowpos-hold` を足した（168＋2・完了時の棚卸の起票は 2 本・既存の brief への追記 0 本・同 spec の行はまだ外していない・`.kiro/specs/` 直下のフォルダ 170 と一致〔completed を除く・brief を持つ 170〕）。その前の 168＝2026-10-30 `emo-text-dwrite-fidelity` の完了で同 spec を外した（169−1・完了時の棚卸の起票は 0 本・既存の brief への追記 1 本＝`balloon-font-file`・`.kiro/specs/` 直下のフォルダ 168 と一致〔completed を除く・brief を持つ 168〕）。その前の 169＝2026-10-30 `startup-install-option` の完了で同 spec を外し、完了時の棚卸で `install-before-boot`・`ghost-switch-balloon-fallback` を足した（168−1＋2・完了時の棚卸の起票は 2 本・既存の brief への追記 2 本＝`helper-first-load-refusal`・`ghost-call`・`.kiro/specs/` 直下のフォルダ 169 と一致〔completed を除く・brief を持つ 169〕）。その前の 168＝2026-10-30 `mcp-strict-errors` の完了で同 spec を外した（169−1・完了時の棚卸の起票は 0 本・既存の brief への追記 0 本・`.kiro/specs/` 直下のフォルダ 168 と一致〔completed を除く・brief を持つ 168〕）。その前の 169＝2026-10-30 `real-run-log-noise` の完了で同 spec を外し、完了時の棚卸で `emo-atlas-palette-alpha` を足した（169−1＋1・完了時の棚卸の起票は 1 本・既存の brief への追記 0 本・`.kiro/specs/` 直下のフォルダ 169 と一致〔completed を除く・brief を持つ 169〕）。その前の 169＝2026-10-30 `shell-mouse-cursor` の完了で同 spec を外した（170−1・完了時の棚卸の起票は 0 本・`.kiro/specs/` 直下のフォルダ 169 と一致）。その前の 170＝2026-10-30 `default-balloon-selfmade` の完了で同 spec を外した（171−1・完了時の棚卸の起票は 0 本・既存の brief への追記 1 本＝`text-typesetting`・`.kiro/specs/` 直下のフォルダ 170 と一致）。その前の 171＝2026-10-28 `/kiro-discovery`〔本体の crate 分け〕で 6 本を足した（`placement-crate-split`・`runtime-module-untangle`・`runtime-crate-split`・`input-crate-split`・`session-module-gather`・`session-crate-split`・165＋6・`.kiro/specs/` 直下のフォルダ 171 と一致）。その前の 165＝2026-10-28 `/kiro-discovery`〔利用統計とノベルゲーム基盤〕で 5 本を足した（`doc-tree-tidy`・`sound-canon-tags`・`ghost-material-metadata`・`shell-multi-compose`・`areka-usage-stats`・2026-10-25 の `talk-script-metadata`〔+1〕はこの見出しに書き足されていなかった＝159＋1＋5・既存の brief への追記 5 本・`.kiro/specs/` 直下のフォルダ 165 と一致）。その前の 159＝2026-10-24 `mouse-click-wheel-events` の完了で同 spec を外した（160−1・完了時の棚卸の起票は 0 本・既存の brief への追記 0 本・`.kiro/specs/` 直下のフォルダ 159 と一致）。その前の 160＝2026-10-24 `/kiro-discovery` で `emo-text-dwrite-fidelity` を足した（159＋1・既存の brief への追記 1 本＝`text-typesetting`・`.kiro/specs/` 直下のフォルダ 160 と一致）。その前の 159＝2026-10-24 `readonly-mode` の完了で同 spec を外した（160−1・完了時の棚卸の起票は 0 本・既存の brief への追記 0 本・`.kiro/specs/` 直下のフォルダ 159 と一致）。その前の 160＝2026-10-24 `dpi-realign-remembered-chain` の完了で同 spec を外した（161−1・完了時の棚卸の起票は 0 本・既存の brief への追記 1 本＝`window-work-area-clamp`・`.kiro/specs/` 直下のフォルダ 160 と一致）。その前の 161＝2026-10-24 `dpi-realign-remembered-chain` のタスク 6.3 の実機の確かめで `mouse-hover-event` を起票した（160＋1・区分 A・`.kiro/specs/` 直下のフォルダ 161 と一致）。その前の 160＝2026-10-24 `vertical-choice-hover-band` の完了で同 spec を外し、完了時の棚卸の起票 1 本＝`emo-text-line-gap` を足した（160−1＋1・既存の brief への追記 1 本＝`window-work-area-clamp`・`.kiro/specs/` 直下のフォルダ 160 と一致）。その前の 160＝2026-10-24 `descript-window-keys` の完了で同 spec を外し、完了時の棚卸の起票 1 本＝`seriko-missing-surface-default`（裁定 11・タスク 8.3 で先に台帳へ足していた行）を数えに入れた（160−1＋1・既存の brief への追記 2 本＝`helper-first-load-refusal`・`areka-test-binary-stability`・`.kiro/specs/` 直下のフォルダ 160 と一致）。その前の 160＝2026-10-24 `shiori2-protocol` の完了で同 spec を外し、完了時の棚卸の起票 1 本＝`helper-first-load-refusal` を足した（160−1＋1・既存の brief への追記 3 本＝`popup-menu-residue`・`draw-methods-canon`・`default-balloon-selfmade`・`.kiro/specs/` 直下のフォルダ 160 と一致）。その前の 160＝2026-10-23 `mcp-scope-rect-property` の完了で同 spec を外した（161−1・完了時の棚卸の起票は 0 本・既存の brief への追記 0 本・`.kiro/specs/` 直下のフォルダ 160 と一致）。その前の 161＝2026-10-23 棚卸㉙で 4 本を起票した（157＋4・`install-package-type`・`sstp-receive`・`balloon-inline-image`・`ghost-alias-file`・`.kiro/specs/` 直下のフォルダ 161 と一致）。その前の 157＝2026-10-23 `pasta-in-windows-verification` の完了で同 spec を外した（158−1・完了時の棚卸の起票は 0 本・既存の brief への追記 1 本＝`animated-surface-log-flood`・`.kiro/specs/` 直下のフォルダ 157 と一致）。その前の 158＝2026-10-23 `pasta-in-windows-verification` の枝に既定ブランチを取り込み、同 spec の申し送りで v0.1.0 に足した 6 本〔`mcp-scope-rect-property`・`animated-surface-log-flood`・`shell-scale-linkage`・`balloon-scroll-arrows`・`balloon-communicatebox`・`vertical-choice-hover-band`〕を合わせた（152＋6・「テーマ別の決めごと」の「pasta-in-windows-verification の申し送り」・`.kiro/specs/` 直下のフォルダ 158 と一致）。その前の 152＝2026-10-23 `offline-surface-dump` の完了で同 spec を外した（153−1・完了時の棚卸の起票は 0 本・既存の brief への追記 1 本＝`animated-image-import`・`.kiro/specs/` 直下のフォルダ 152 と一致）。その前の 153＝2026-10-23 `ghost-inner-balloon` の完了で同 spec を外した（154−1・完了時の棚卸の起票は 0 本・既存の brief への追記 1 本＝`install-live-target-hazards`・`.kiro/specs/` 直下のフォルダ 153 と一致）。その前の 154＝2026-10-23 steering の同期で `dot-sakura-specimen`〔10-23 完了〕の行を外し、同 spec の完了時の起票 2 本〔`shiori2-protocol`・`seriko-pattern-calls`〕を足した（153−1＋2・`.kiro/specs/` 直下のフォルダ 154＝2026-10-23 に `areka-P0-` を外した 151＋着手済みで旧い名前のままの 3 と一致）。その前の 153＝2026-10-23 `script-security-level` の完了で同 spec を外した（154−1・完了時の棚卸の起票は 0 本・`dot-sakura-specimen` の完了と起票 2 本は未反映＝「版の段」の数の行）。それより前の数えの経緯は history「2026-10-23 完了済みの退避」）

> **spec は名前で呼ぶ**（2026-09-26 開発者指示）: 報告・brief・コミット・PR で spec を指すときは spec 名（完了済みと着手済みの旧い名前の `areka-P0-` は省略してよい・未着手の spec は 2026-10-23 から接頭辞なしの名前＝`kiro-spec-schema.md` の命名規則）を書く。「#数字」は `PR#185` の形の PR 番号にだけ使う。古い文書に台帳番号が出てきたら、その時点の表（history）で名前へ読み替える。
> **2026-10-10 の改め（棚卸㉓）**: 着手の並びは 4 区分（生きている決まり 1）。「段」の列の末尾に **区分**（A 開発者が起票を頼んだ／B バグ／C ukadoc の拾い残し・バグでない持ち越し／D 夢・据え置き・保留）を足した。区分の数え＝**A 88・B 14・C 47・D 13 ＝ 162**（2026-11-01 `popup-menu-extra-resources` の完了の後の実数え〔C 1 本〔同 spec〕を外した。表の「区分 X」を数え直すと、その前の行の数は A 90・B 13 とずれていた〕）。その前の **A 90・B 13・C 47・D 13 ＝ 163**（2026-10-31 `placement-measure-bake-once` の完了の後の実数え〔C 1 本〔同 spec〕を外し、C 1 本〔`emo-bake-log-overlap`〕を足した〕）。その前の **A 90・B 13・C 47・D 13 ＝ 163**（2026-10-31 `actor-thread-log-capture` の完了の後の実数え〔C 1 本〔同 spec〕を外した〕）。その前の **A 90・B 13・C 48・D 13 ＝ 164**（2026-10-31 `popup-menu-residue` の完了の後の実数え〔C 1 本〔同 spec〕を外し、C 1 本〔`popup-menu-extra-resources`〕を足した〕）。その前の **A 90・B 13・C 48・D 13 ＝ 164**（2026-10-31 `choice-ranges-one-function` の完了の後の実数え〔C 1 本〔同 spec〕を外した〕）。その前の **A 90・B 13・C 49・D 13 ＝ 165**（2026-10-31 `emo-atlas-palette-alpha` の完了の後の実数え〔B 1 本〔同 spec〕を外した〕）。その前の **A 90・B 14・C 49・D 13 ＝ 166**（2026-10-31 棚卸㉛の実数え〔C 1 本〔`seriko-legacy-pattern-format`〕を足した〕）。その前の **A 90・B 14・C 48・D 13 ＝ 165**（2026-10-31 `animated-surface-log-flood` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕）。その前の **A 91・B 14・C 48・D 13 ＝ 166**（2026-10-31 `stay-on-top-directives` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕）。その前の **A 92・B 14・C 48・D 13 ＝ 167**（2026-10-31 `balloon-scroll-arrows` の完了の後の実数え〔A 1 本〔同 spec〕を外し、C 1 本〔`balloon-append-tags`〕を足した〕）。その前の **A 93・B 14・C 47・D 13 ＝ 167**（2026-10-31 `seriko-pattern-calls` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕）。その前の **A 94・B 14・C 47・D 13 ＝ 168**（2026-10-30 `bridge-store-user-root` の完了の後の実数え〔B 1 本〔同 spec〕を外した〕）。その前の **A 94・B 15・C 47・D 13 ＝ 169**（2026-10-30 `window-work-area-clamp` の完了の後の実数え〔B 1 本〔同 spec〕を外した〕）。その前の **A 94・B 16・C 47・D 13 ＝ 170**（2026-10-30 `window-work-area-clamp` の完了時の棚卸の起票の後の実数え〔B 2 本〔`balloon-flip-side-rules`・`window-resetwindowpos-hold`〕を足した。同 spec の完了〔B −1〕はまだ載せていない〕）。その前の **A 94・B 14・C 47・D 13 ＝ 168**（2026-10-30 `emo-text-dwrite-fidelity` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕）。その前の **A 95・B 14・C 47・D 13 ＝ 169**（2026-10-30 `startup-install-option` の完了の後の実数え〔A 1 本〔同 spec〕を外し、A 1 本〔`install-before-boot`〕と B 1 本〔`ghost-switch-balloon-fallback`〕を足した。2026-10-30 の `mcp-strict-errors` の完了〔A −1〕はこの行に書き足されていなかった＝この実数えで載せた〕）。その前の **A 96・B 13・C 47・D 13 ＝ 169**（2026-10-30 `real-run-log-noise` の完了の後の実数え〔C 1 本〔同 spec〕を外し、B 1 本〔`emo-atlas-palette-alpha`〕を足した〕）。その前の **A 96・B 12・C 48・D 13 ＝ 169**（2026-10-30 `shell-mouse-cursor` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕）。その前の **A 97・B 12・C 48・D 13 ＝ 170**（2026-10-30 `default-balloon-selfmade` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕）。その前の **A 98・B 12・C 48・D 13 ＝ 171**（2026-10-28 `/kiro-discovery`〔本体の crate 分け〕の起票の後の実数え〔A 6 本を足した〕）。その前の **A 92・B 12・C 48・D 13 ＝ 165**（2026-10-28 `/kiro-discovery` の起票の後の実数え〔A 4 本〔`doc-tree-tidy`・`ghost-material-metadata`・`shell-multi-compose`・`areka-usage-stats`〕・C 1 本〔`sound-canon-tags`〕を足した。2026-10-25 の `talk-script-metadata`〔A +1〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の **A 87・B 12・C 47・D 13 ＝ 159**（2026-10-24 `mouse-click-wheel-events` の完了の後の実数え〔A 1 本〔同 spec〕を外した。2026-10-24 の `emo-text-dwrite-fidelity` の起票〔A +1〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の **A 87・B 12・C 47・D 13 ＝ 159**＝2026-10-24 `readonly-mode` の完了で A を 1 本外した。その前の **A 88・B 12・C 47・D 13 ＝ 160**＝2026-10-24 `dpi-realign-remembered-chain` の完了で B を 1 本外した。その前の **A 88・B 13・C 47・D 13 ＝ 161**＝2026-10-24 `mouse-hover-event` の起票で A を 1 本足した。その前の **A 87・B 13・C 47・D 13 ＝ 160**＝2026-10-24 `vertical-choice-hover-band` の完了の後の実数え〔A 1 本〔同 spec〕を外し、B 1 本〔`emo-text-line-gap`〕を足した〕。その前の **A 88・B 12・C 47・D 13 ＝ 160**（2026-10-24 `descript-window-keys` の完了の後の実数え〔A 1 本〔同 spec〕を外し、C 1 本〔`seriko-missing-surface-default`〕を足した〕。その前の **A 89・B 12・C 46・D 13 ＝ 160**＝2026-10-24 `shiori2-protocol` の完了の後の実数え〔A 1 本〔同 spec〕を外し、B 1 本〔`helper-first-load-refusal`〕を足した〕。その前の **A 90・B 11・C 46・D 13 ＝ 160**＝2026-10-23 `mcp-scope-rect-property` の完了の後の実数え〔A 1 本〔同 spec〕を外した。棚卸㉙の起票 4 本〔B 1・C 3〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の **A 91・B 10・C 43・D 13 ＝ 157**＝2026-10-23 `pasta-in-windows-verification` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕。その前の **A 92・B 10・C 43・D 13 ＝ 158**＝2026-10-23 `pasta-in-windows-verification` の枝に既定ブランチを取り込んだ後の実数え〔同 spec の申し送りの 6 本〔どれも A〕を足した〕。その前の **A 86・B 10・C 43・D 13 ＝ 152**＝2026-10-23 `offline-surface-dump` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕。その前の **A 87・B 10・C 43・D 13 ＝ 153**＝2026-10-23 `ghost-inner-balloon` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕。その前の **A 88・B 10・C 43・D 13 ＝ 154**＝2026-10-23 steering の同期の実数え〔A 1 本〔`dot-sakura-specimen`〕を外し、A 2 本〔`shiori2-protocol`・`seriko-pattern-calls`〕を足した〕。その前の **A 87・B 10・C 43・D 13 ＝ 153**＝2026-10-23 `script-security-level` の完了の後の実数え〔A 1 本〔同 spec〕を外した〕。それより前の数えの経緯は history「2026-10-23 完了済みの退避」）。段の語（バグ／優先／その他／据え置き／保留）は 10-03・10-05 の分け方の名残で、並びには使わない。区分の読みに迷った行（開発者の指示が「分け方」だけ・名指しの範囲外の切り出し）は C に置いた＝「棚卸㉓の裁定」12。
> **段＝優先度の 3 段**（2026-10-03 開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」）: **バグ**＝1 段目／**優先**＝2 段目（配布と公開・文字とバルーンの列・シェルの element の列・動く画像・バルーンのイベントと残件・**SSP MCP の移植 10 本**＝10-03 に開発者が追加・`mouse-drag-events`＝10-04 に開発者が個別に上げた）／**その他**＝3 段目（ゴーストのインストール・kanade・プロパティ・道具）／**据え置き**＝当面着手しない（理由は行に）／**保留**。どのウェーブに居るかは「並び」の列。完了した spec はこの表に置かない（完了サマリと history）。
> **段の数え方**: `awk '/^\| spec（旧い `areka-P0-` は省略）/{f=1;next} f&&/^\|/{print} f&&!/^\|/{f=0}' .kiro/steering/roadmap.md | awk -F'|' 'NR>1{gsub(/ /,"",$3);print $3}' | sort | uniq -c`（2026-10-31 `placement-measure-bake-once` の完了の後の実数え＝バグ 13・優先 77・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 163〔優先 1 本〔同 spec〕を外し、その他 1 本〔`emo-bake-log-overlap`〕を足した〕。その前の 2026-10-31 `actor-thread-log-capture` の完了の後の実数え＝バグ 13・優先 78・その他 58・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 163〔その他 1 本〔同 spec〕を外した〕。その前の 2026-10-31 `popup-menu-residue` の完了の後の実数え＝バグ 13・優先 78・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 164〔その他 1 本〔同 spec〕を外し、その他 1 本〔`popup-menu-extra-resources`〕を足した〕。その前の 2026-10-31 `choice-ranges-one-function` の完了の後の実数え＝バグ 13・優先 78・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 164〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-31 `emo-atlas-palette-alpha` の完了の後の実数え＝バグ 13・優先 79・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 165〔バグ 1 本〔同 spec〕を外した・2026-10-31 棚卸㉛で足したその他 1 本〔`seriko-legacy-pattern-format`〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の 2026-10-31 `animated-surface-log-flood` の完了の後の実数え＝バグ 14・優先 79・その他 58・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 165〔その他 1 本〔同 spec〕を外した〕。その前の 2026-10-31 `stay-on-top-directives` の完了の後の実数え＝バグ 14・優先 79・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 166〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-31 `balloon-scroll-arrows` の完了の後の実数え＝バグ 14・優先 80・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 167〔優先 1 本〔同 spec〕を外し、その他 1 本〔`balloon-append-tags`〕を足した〕。その前の 2026-10-31 `seriko-pattern-calls` の完了の後の実数え＝バグ 14・優先 81・その他 58・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 167〔その他 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `bridge-store-user-root` の完了の後の実数え＝バグ 14・優先 81・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 168〔バグ 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `window-work-area-clamp` の完了の後の実数え＝バグ 15・優先 81・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 169〔バグ 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `window-work-area-clamp` の完了時の棚卸の起票の後の実数え＝バグ 16・優先 81・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 170〔バグ 2 本〔`balloon-flip-side-rules`・`window-resetwindowpos-hold`〕を足した。同 spec の完了〔バグ −1〕はまだ載せていない〕。その前の 2026-10-30 `emo-text-dwrite-fidelity` の完了の後の実数え＝バグ 14・優先 81・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 168〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `startup-install-option` の完了の後の実数え＝バグ 14・優先 82・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 169〔優先 1 本〔同 spec〕を外し、優先 1 本〔`install-before-boot`〕とバグ 1 本〔`ghost-switch-balloon-fallback`〕を足した。2026-10-30 の `mcp-strict-errors` の完了〔優先 −1〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の 2026-10-30 `real-run-log-noise` の完了の後の実数え＝バグ 13・優先 83・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 169〔その他 1 本〔同 spec〕を外し、バグ 1 本〔`emo-atlas-palette-alpha`〕を足した〕。その前の 2026-10-30 `shell-mouse-cursor` の完了の後の実数え＝バグ 12・優先 83・その他 60・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 169〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `default-balloon-selfmade` の完了の後の実数え＝バグ 12・優先 84・その他 60・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 170〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-28 `/kiro-discovery`〔本体の crate 分け〕の起票の後の実数え＝バグ 12・優先 85・その他 60・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 171〔優先 6 本を足した〕。その前の 2026-10-28 `/kiro-discovery` の起票の後の実数え＝バグ 12・優先 79・その他 60・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本〔`talk-script-metadata`・2026-10-25 の起票〕＝ 165〔その他 5 本を足した〕。その前の 2026-10-24 `mouse-click-wheel-events` の完了の後の実数え＝バグ 12・優先 79・その他 55・据え置き 12・保留 1 ＝ 159〔その他 1 本〔同 spec〕を外した。2026-10-24 の `emo-text-dwrite-fidelity` の起票〔優先 +1〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の 2026-10-24 `readonly-mode` の完了の後の実数え＝バグ 12・優先 78・その他 56・据え置き 12・保留 1 ＝ 159〔優先 1 本〔同 spec〕を外した。2026-10-24 の `mouse-hover-event` の起票〔その他 +1〕と `dpi-realign-remembered-chain` の完了〔バグ −1〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の 2026-10-24 `vertical-choice-hover-band` の完了の後の実数え＝バグ 13・優先 79・その他 55・据え置き 12・保留 1 ＝ 160〔優先 1 本〔同 spec〕を外し、バグ 1 本〔`emo-text-line-gap`〕を足した〕。その前の 2026-10-24 `descript-window-keys` の完了の後の実数え＝バグ 12・優先 80・その他 55・据え置き 12・保留 1 ＝ 160〔優先 1 本〔同 spec〕を外し、その他 1 本〔`seriko-missing-surface-default`〕を足した〕。その前の 2026-10-24 `shiori2-protocol` の完了の後の実数え＝バグ 12・優先 81・その他 54・据え置き 12・保留 1 ＝ 160〔その他 1 本〔同 spec〕を外し、バグ 1 本〔`helper-first-load-refusal`〕を足した〕。その前の 2026-10-23 `mcp-scope-rect-property` の完了の後の実数え＝バグ 11・優先 81・その他 55・据え置き 12・保留 1 ＝ 160〔その他 1 本〔同 spec〕を外した〕。その前の 2026-10-23 棚卸㉙の実数え＝バグ 11・優先 81・その他 56・据え置き 12・保留 1 ＝ 161〔バグ 1 本・その他 3 本を足した〕。その前の 2026-10-23 `pasta-in-windows-verification` の完了の後の実数え＝バグ 10・優先 81・その他 53・据え置き 12・保留 1 ＝ 157〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-23 `pasta-in-windows-verification` の枝に既定ブランチを取り込んだ後の実数え＝バグ 10・優先 82・その他 53・据え置き 12・保留 1 ＝ 158〔同 spec の申し送りの 6 本〔優先 3・その他 3〕を足した〕。その前の 2026-10-23 `offline-surface-dump` の完了の後の実数え＝バグ 10・優先 79・その他 50・据え置き 12・保留 1 ＝ 152〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-23 `ghost-inner-balloon` の完了の後の実数え＝バグ 10・優先 80・その他 50・据え置き 12・保留 1 ＝ 153〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-23 steering の同期の実数え＝バグ 10・優先 81・その他 50・据え置き 12・保留 1 ＝ 154〔その他 1 本〔`dot-sakura-specimen`〕を外し、その他 2 本〔`shiori2-protocol`・`seriko-pattern-calls`〕を足した〕。その前の 2026-10-23 `script-security-level` の完了の後の実数え＝バグ 10・優先 81・その他 49・据え置き 12・保留 1 ＝ 153〔優先 1 本〔同 spec〕を外した〕。それより前の数えの経緯は history「2026-10-23 完了済みの退避」）
> **規模**は棚卸㉓の再測定の見立て（タスク数）。**上限は 1 spec 20 タスク**。「要件で切る」と書いた行は、着手のときに要件の段で切り出す（先に起票しない＝spec 工場の禁止）。**Fable**列＝要件定義を Fable で起動したセッションで進めることを勧める（○）か、Opus で足りる（−）か。**既定は Opus**（2026-10-15 開発者）＝○は「先例の無い境界や公開の契約を新しく引く」「並行・スレッド・時刻や順序の保証を新しく設計する」「先例で決まらない骨格の分かれ目が残る」のどれかに具体的に当たるときだけ（基準の正本は `kiro-requirements-discussion` の Phase 8）。**2026-10-15 に全行をこの基準で付け直した**（○ 72 → 13・当たった項目は各 spec の要件の段の推奨で名指しする）。各 brief の末尾「2026-10-10 棚卸㉓の再測定」（と分割・測定・申し送りの節）が、崩れた前提・触るファイル・議題の正本。**「並び」「前提」の列には起票や前の棚卸の時点の書き方が残る行がある**＝食い違うときは brief の節と「ウェーブ編成」が正本。

| spec（旧い `areka-P0-` は省略） | 段 | 何をするか | 規模 | 並び | 前提（先に着地） | Fable |
|---|---|---|---|---|---|---|
| `bottom-row-text-clip`（**10-16 起票**・`balloon-text-bugs` の完了の棚卸・同じ文字の層の拾い残し 3 件を件 2〜4 にまとめた） | バグ・区分 B | ⑴（本体）最下行の字の下の部分（`g`・`y` の descent）が文字の面の下端（validrect の下端）で欠ける（emo2-kakukaku・面の高さ 122・Yu Gothic UI 28 画素・拡大率 2.0 で約 6 画素足りず、`g` が `q`、`y` が `v` に見える。行の矩形で数えると収まるとされるので見える窓のスクロールも起きない。下線の無い字で欠けるかは未確認＝要件の段の最初に撮る）＝直し方は要件の討議で選ぶ（面は validrect より広げない・下線の有無でレイアウトを変えない、の裁定の枠の中） ⑵ 潰れた描画範囲の再追従で差し込み口の entity が同時に替わると、古い差し込み口に前の字が残りうる（`actor_present.rs` の `TextSurface::release` が今の結線の slot を使う・観測なし） ⑶ 横書きの右から左の下線の写し（`underline.rs` の `stroke_rect`）をどの檻でも実物と突き合わせていない ⑷ 箱の押下の後の済み（`shell_box_handler.rs` の `send_selection`）で文字の層が見つからないとき記録なしで抜ける。細部は brief の「件 1〜4」 | S〜M（6〜11） | 文字とバルーンの列（emo-text の `layout.rs`・`region.rs`・`actor_present.rs`・`underline.rs`・`surface_window_child.rs` と `crates/areka/src/input_events/shell_box_handler.rs`）＝✅ `choice-ranges-one-function`（10-31 着地）・`range-choice-tag`・`anchor-style-canon`・`choice-marker-styling`・`balloon-font-file`・`text-typesetting`・`real-run-log-noise` と直列。席は次の棚卸で決める | ✅ `balloon-text-bugs`（10-16 完了） | − |
| `areka-test-binary-stability`（**10-11 起票**・`/kiro-discovery` でバグの起票 2 本＝`areka-test-threads-av`〔10-05〕・`test-wait-marker-gaps`〔10-10〕をまとめた） | バグ・区分 B | `cargo test -p areka --bin areka` のバグ 2 件: ⑴ `STATUS_ACCESS_VIOLATION` で落ちる（`--test-threads=4` で再現・負荷の下では既定のスレッドの数でも 3 度落ちた）＝落ちた瞬間のテストの名前を残す仕掛けを先に入れ、落ちる組と原因を特定して、スレッドに縛られる資源の扱いを直す。再現しなければ記録して据え置きへ ⑵ テストの待ちの残りを steering `tech.md` の「テストの待ちと、負荷の下で許す赤」に揃える（目印が UI の側の仕事を数えない待ち・目印が条件と同時にしか動かない `join_bounded`・`areka-nar` の短い掴みの檻・上限の無い `GpuPermit::take`・目印の無い `spin_wait_until` の直接の呼び出し 15 か所）。⑶ （10-31 `emo-atlas-palette-alpha` の完了の棚卸で足した）`cargo test -p wintf --lib` で MTA を張るテストと ASTA の Compositor を作る `com::wuc::tests::begin_draw_roundtrip` が同時に走ると `0xc0000005` で落ちる（8 回で 2 回）＝⑴ と同じ形の疑い ⑷ （同）`ghost_switch_test_support.rs` の検体を組む間の `remove_dir` が io エラーで 1 回赤（単独では緑）。⑴ → ⑵ の順（⑶ は ⑴ と一緒に切り分け・⑷ は ⑵ の足場と一緒に）・負荷つきの全体の確かめは最後に 1 度だけ。細部は brief の「件 1〜4」 | M〜L（13〜20・超える見込みなら要件の段で ⑵ を切る） | ウェーブの外（測ることが本体・静かな机で 1 本だけ） | なし | − |
| `seriko-trigger-lottery-gaps`（**10-11 起票**・`/kiro-discovery` で起票 3 本＝`seriko-trigger-teardown-gaps`〔10-10・バグ〕・`seriko-lottery-ended-play-skip`〔10-10・バグ〕・`seriko-rebuild-hidden-lottery`〔10-07・その他〕をまとめた） | バグ・区分 B | SERIKO の引き金と抽選の食い違い 3 件: ⑴ interval の 3 語（`talk`・`runonce`・`periodic`）の引き金の後始末の食い違いを揃える（窓の知らせだけで面の番号が変わっても構え直さない／窓の新しい出番で部品の `periodic` の起点だけが置き直る／窓が閉じると部品だけが末尾のコマと再生中の時計を失う／面を離れた瞬間の再生に停止の記録が出ない／`\b[番号]` の切り替えのテストが無い／同じ決まりの写し 3 種の片付け）＝1 件ずつ「網羅台帳の備考に 1 文」か「変える」かに分ける ⑵ 抽選の animation が、境界より前に終わった再生の片付けが残っていると、その境界のくじを 1 回飛ばし、乱数も引かない ⑶ 部品の組み直し（`rebuild`）の 1 回目が、見えない部品にも回ってくじを引く。⑵⑶ は境界の時刻で測る・見える部品だけを評価する形に直す（同じ抽選の決定論のテストの期待値を 1 度で引き直す）か、今の決まりとして記録して閉じるかを要件で決める。⑷（2026-10-31 `animated-surface-log-flood` から足した）emo-compose の `nesting.rs` の `bind_targets`／`visible_parts` が、有効でコマの無い `bind+random` の pattern0 の先を見える部品に数え、描く側の `flatten_surface` と食い違う＝純 `bind` だけを数える形に直し、一致の檻で固定する。実機で絵の誤りを見たものは無い。細部は brief の「件 1〜4」 | M〜L（10〜24・⑵⑶ を両方直すなら上限の近く＝引き直すテストの本数を要件で先に数える） | シェルの element の列（seriko の `looper.rs`・`looper_trigger.rs`・`parts.rs`・`parts_trigger.rs`・`trigger.rs`・`actor.rs`・`table.rs`、件 4 で emo-compose の `nesting.rs`）＝`seriko-talk-clock-fidelity`（本 spec の後）・`seriko-interval-combinations`・`seriko-script-triggers`・`balloon-canon-residue`・`animated-image-import` と直列 | ✅ `seriko-trigger-intervals`（10-10 着地） | − |
| `extent-element-offset`（**10-05 起票**・`surface-element-nesting` の完了時の棚卸） | バグ・区分 B | 外形（`flatten_extent`・10-06 に `animated-image-playback` が `plan.rs` から `plan_extent.rs` へ移した）が画像の element定義の X,Y を数えず、ベースの画像の外へはみ出す画像の element が記録なしに切られる（emo-compose を作った時から・入れ子の子の中の element も漏れる）。ukadoc の外形の定義を確かめ、全命令の和集合に揃えるか、ベースで切ると明文化して記録を出すかを裁定する | S（3〜6） | **C6-⑤**（C5 から持ち越し）・シェルの element の列 | なし（`element-base-method`・`animated-image-playback` は着地済み） | − |
| `draw-methods-canon`（**10-05 起票**・`element-base-method` の完了時の棚卸） | 優先・区分 A | `overlay`・`base` 以外の描画メソッド（`overlayfast`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*` ほか）を element定義・pattern定義で描き、pattern定義の `base` も描く。開発者の方針（10-05）「Direct2D が支える全部に対応できるよう、wintf も含めて広げる」。合成を CPU の整数の転写のまま広げるか D2D へ移すかは要件・設計で決める。読み手の `is_image_element_method` を広げ `Element` に欄を足すと、描けない行の警告の対象が自動で減る **棚卸㉓で分けた**: `blend-*` は `draw-methods-blend` へ。 | M〜L（13〜16・棚卸㉓で分けた後） | シェルの element の列（`element-clipping-option` の後） | `element-base-method`（✅ 10-05 完了）・`wintf-d2d-composite-rules`（10-15 に D2D の道に決まった＝wintf の口が先） | ○ |
| `collisionex-regions`（**10-05 起票**・`mouse-drag-events` の実機で発見） | 優先・区分 C | `collisionex`（矩形・円・楕円・多角形の当たり判定）を読まない（`decode_collisions` は数字だけの `collisionN` だけ・台帳 `absent`）。`collisionex` だけで書くクローディアでは全マウスイベントの Reference4 がいつも空。読み手に形を足し、`hit.rs` で形ごとに内外を判定する | S〜M（8〜11） | **C6-⑥**・シェルの element の列（読み手の鎖＝✅ `seriko-trigger-intervals`〔10-10 着地〕の後） | ✅ `seriko-trigger-intervals`（10-10 着地・同じ `shell/{model,decode}.rs`） | − |
| `install-live-target-hazards` | その他・区分 C | 表示中のシェル・使用中のバルーンへの上書きと、起動中のゴーストへ入れる途中の Windows の終了を**実測してから**扱いを決める | S〜M（8〜14・うち計測 5〜6） | ウェーブの外（測ってから扱いを決める・`balloon-font-file` を待たない） | なし | − |
| `shell-companion-balloon`（**10-03 起票**・同 3/3） | 優先・区分 C | シェルの書庫の同梱バルーンをそのシェルに紐づけ、着替えと起動で使う（今はシェルごとのバルーンが無い） | M（10〜14） | カタログの鎖の最後（5 本目・棚卸㉔）・`emo2_boot` の列 | `baseware-root-list`・✅ `ghost-inner-balloon`〔10-23 着地＝済み〕（同じ `catalog.rs`・`boot_resolve.rs`・`boot_config.rs`） | − |
| `balloon-font-file` | 優先・区分 A | `font.name`・`\f[name]` のフォントファイル | M（8〜11） | 文字とバルーンの列（✅ `anchor-tag-canon`〔10-10 着地〕の後＝`actor.rs`・`viewbox_draw_render.rs`・`lib.rs`）・`emo2_boot` の列 | `shell-balloon-frame-align`（✅ 10-05 完了） | − |
| `text-typesetting` | 優先・区分 A | 禁則・ぶら下げ・縦中横・字の向き（areka 独自） | L（17〜22・超えたら要件の段で「禁則とぶら下げ」と「縦中横と字の向き」に） | 文字とバルーンの列 | `shell-balloon` | − |
| `talk-fast-forward` | 優先・区分 A | クリックでの早送り・`\x`／`\x[noclear]`・`clickwaitmarker.*` | L（19〜24〔棚卸㉜〕・超えたら要件の段で「クリックの早送り」と「`\x` と印」に） | 文字とバルーンの列・コンパイルの列 | `shell-balloon` | ○ |
| `balloon-markers` | 優先・区分 A | `onlinemarker`・`number`・SSTP の印（表示だけ）・装飾の系列 | M（2026-10-23 に前半の矢印と手動スクロールを `balloon-scroll-arrows`〔v0.1.0〕へ切り出した＝残りは状態の印・棚卸㉙でモデルも Opus に） | 文字とバルーンの列 | `balloon-scroll-arrows` | − |
| `text-ruby` | 優先・区分 A | ルビ `\![text,ruby,親文字,ルビ]`・`line_height`・`letter_spacing` | M〜L（13〜17） | 文字とバルーンの列 | `text-typesetting` | − |
| `text-reveal-fade` | 優先・区分 A | 1 字ずつの表示で字が透明から不透明へ変わる（既定は無効） | M（10〜13） | 文字とバルーンの列 | `shell-balloon` | − |
| `balloon-scroll-fade` | 優先・区分 A | 自動スクロールで押し出される行のフェード（既定は無効） | S〜M（6〜9） | 文字とバルーンの列 | `balloon-scroll-arrows`（棚卸㉙で `balloon-markers` から改めた） | − |
| `text-align-shadow-canon` | 優先・区分 C | `\f[align]`／`\f[valign]`・影。**20 を超えそうなら「影」と「寄せ」に切る** | L（18〜22・一度切り出した spec＝超えたら要件の段で影 → 寄せ） | 文字とバルーンの列 | `text-typesetting` | − |
| `choice-marker-styling` | 優先・区分 C | `\f[cursor*]` 10 項目と本物の下線 | S（5〜8・descript の `cursor.notselect.*` まで入れると 9〜12） | 文字とバルーンの列 | ✅ `anchor-tag-canon`（10-10 着地・同じ `choice.rs`） | − |
| `animated-image-import`（**10-05 起票**・棚卸㉒で `animated-image-playback` から切り出し） | 優先・区分 A | pattern定義の描画メソッド `import` の画像を名前を保って読み、焼いて置く（今は読み手がファイル名を 0 に化かし記録も出ない） | M（10〜14） | シェルの element の列（読み手の鎖＝`collisionex-regions` の後・`extent-element-offset` の後） | ✅ `seriko-trigger-intervals`（10-10 着地）・`extent-element-offset`（ファイルの重なり） | − |
| `element-clipping-option`（**10-05 起票**・`animated-image-decode` のタスク 6.3＝要件 9.3） | 優先・区分 C | element定義の `--clipping,左,上,右,下` で矩形だけを描く・付けた element定義では動く絵として読まない（正典 C2）・オプションの並びを読み手で転記（`--alpha`・`--source`・`--scaling` の描画は範囲外） | M（11〜15） | シェルの element の列（`element-base-method`・`animated-image-playback`・`extent-element-offset`・`animated-image-import` の後） | `animated-image-decode`・`surface-element-nesting`・`element-base-method` | − |
| `balloon-element-order` | 優先・区分 A | `balloon` の element定義を並び順どおりの重ね順で描く。wintf の兄弟の重なり順（描画と当たり判定が逆）の裁定の持ち主 | M（10〜14） | シェルの element の列 | `shell-balloon`・`animated-image-playback` | ○ |
| `property-query-channels` | その他・区分 C | `\![get/set,property]`・`%property[…]`・`SenderType`（`\![embed]` は 10-04 に `sakura-embed-directive` へ切った）。許可の表の迂回を最初に決める | M（11〜14） | kanade の列（その他の段＝`mcp-kanade-tools` の後）。host32-host の列からは外れた | `translate-pipeline`・✅ `property-name-case-fold`・`mcp-kanade-tools`（迂回の口） | − |
| `balloon-break-position`（**10-05 起票**・`balloon-lifecycle-events` の要件の討議） | その他・区分 C | `OnBalloonBreak` の Reference2（中断位置＝台本の先頭からの文字数・タグも数える）。台本の中の位置を字句から命令・cue・再生・完了の知らせ `TalkDone`（27 ファイル・60 か所）まで通す。数え方・どの台本か・待機の途中の 3 つを要件で決める | M（9〜13） | 台本のコンパイルの列の最後・kanade の列 | `balloon-lifecycle-events` | − |
| `network-update-canon-order` | その他・区分 C | 更新のイベントを ukadoc の発生順序に揃える（MD5 の取り直し・台詞の終わりを待つ・総括の 2 段） | M（10〜14） | `baseware-root-list` の後（同じ `update/desk.rs`） | `baseware-root-list`（ファイルの重なり） | − |
| `balloon-canon-residue` | 優先・区分 C | 残り（項目 2＝面の偶奇で左右のバルーンを選ぶ・普通のバルーンだけ、を先に。項目 4 `\![reload,balloon]` は 10-04 に `mcp-reload` へ移した） | M（10〜14） | `emo2_boot` の列・✅ `seriko-trigger-intervals`〔10-10 着地〕の後（seriko の `actor.rs`） | ✅ `seriko-trigger-intervals`（10-10 着地・ファイルの重なり） | − |
| `currentghost-property-tree` | その他・区分 C | 動く値を出す口（持ち主）＋`currentghost.balloon.scope` の 19 項目（残りは 10-04 に `currentghost-property-others` へ切った） | M（13〜16） | `emo2_boot` の列・プロパティの動く値の列 | ✅ `property-name-case-fold`（同じ `actor.rs`） | ○ |
| `property-catalog-lists` | その他・区分 C | 一覧（`ghostlist`・`balloonlist`・`activeghostlist`）と基盤待ちの枝の登記（`system.*` は 10-04 に `system-property-values` へ切った） | M（10〜14） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `zorder-property` | その他・区分 C | `currentghost.seriko.zorder` の読み書き | S（5〜8・書き込みの届け先を作るなら 8〜11） | プロパティの動く値の列 | `currentghost-property-tree`（動く値の口）・書く側は `property-query-channels` | − |
| `sakura-time-directives` | その他・区分 C | `quicksection`・`balloonwait`（`\_q` は議題。`balloontimeout` は 10-05 に `balloon-lifecycle-events` へ移した）。C・D 群は覚え書きへ戻し、`\![embed]` は `sakura-embed-directive` へ（10-04） | M（8〜12） | コンパイルの列 | `choice-timeout-directive`・`talk-fast-forward` | − |
| `element-animation-easing`（**10-12 起票**・**覚書の段**・開発者「css イージングの範疇で、エレメントアニメーションの記述や、さくらスクリプトからエレメントにイージング指示が出せたらよいのに」「夢よりは優先度高い程度」） | その他・区分 C | CSS の範囲のイージング（`cubic-bezier`・`steps`・`linear()` と名前の 5 語）を、pattern定義のコマ間と、台本の時間つきの指令（`\![set,alpha]`・`\![set,scaling]`・`\![move]` の `--time`）に添えられるようにする（ukadoc に無い areka 独自）。形は決めていない＝着手のとき要件の段で、最初の 1〜2 段（語彙と式＋使い手 1 つ）に切る | 未測定（覚書・最初の 2 段で M の見込み） | 区分 C の最後（据え置きの手前）。最初の使い手しだいで台本のコンパイルの列かシェルの element の列 | 働きの前提なし。時間つきの指令（`sakura-time-directives` が覚え書きへ戻した D 群）の持ち主が議題 | ○ |
| `shell-balloon-scale-setting`（**10-17 起票**・開発者「現在、シェルやバルーンの倍率は、DPI 追従だけで倍率が決まってるが、更に拡大率を設定できるようにしてほしい」） | その他・区分 A | ゴーストごとの表示倍率（既定 100%）と「バルーンも連携する」を sylphya の `Ghost` scope に覚え、`ScalePolicy.app_scale`（今は 1 に固定の口）へ流し、`placement` の自前の k の 3 か所も同じ倍率にそろえる（DPI 200%×表示倍率 200%＝400%）。入口の設定画面は windows-reactor 待ち＝それまでは `sylphya.toml` を手で書く | M（10〜14） | v0.4.0・`frame/dpi.rs`・`placement` の列（`dpi-realign-remembered-chain` と 1 本ずつ） | v0.2.0 の `wintf-d2d-composite-rules`・`draw-methods-canon`・`compose-f16-precision` | − |
| `menu-thumbnail-preview`（**10-20 起票**・開発者「サムネイル表示に対応する。メニューでゴーストを選択中、サムネイル画像を表示する機能がある」「α付き画像の透過も対応が欲しい」） | その他・区分 A | 右クリックメニューのゴースト・シェル・バルーンの切り替えの項目にカーソルを乗せている間、その項目の `thumbnail.png` ほか（areka が読める形式・`.pnr` は左上の 1 点の色を抜く・α 付きは画素ごとに透ける・動く絵は dola の時刻で動かす）を、メニューの横の別の窓（wintf・DirectComposition）に出す。`WM_MENUSELECT` を wintf の `dispatch_window_message` で受け、`menu/plan.rs` で「番号 → サムネイルの場所」の表を作る。メニューは Win32 標準のまま（生きている決まり 6） | M（10〜14） | v0.4.0・メニュー（`menu/{mod,win32,plan}.rs`・wintf の `window_proc/mod.rs`）＝`popup-menu-shell-look`・`popup-menu-residue` と 1 本ずつ | v0.2.0 の dola の整備（時刻の土台）・`wintf-d2d-composite-rules` | − |
| `shell-hover-animation`（**10-20 起票**・開発者「シェルアニメーションに『マウスホバーが発生した時』だけ発動させたい」・「ぱすた開発」と相談） | その他・区分 A | マウスが名前の付いた当たり判定に乗っている間だけ動くアニメを、シェルの書き方だけで書けるようにする（ukadoc に無い areka 独自）。2 つの書き方＝interval の新しい語（例 `interval,hover,閉じる`）と bindgroup の「乗っている間 ON」（例 `sakura.bindgroupN.hover,閉じる`）・乗っている間ループと乗るたびに 1 回の書き分け。トークの最中も止まらない（SHIORI 経由はトークを置き換えるので、ぱすたはトーク中に × を赤くできない） | M（12〜16） | v0.4.0・入力の列（`input_events/` の追跡を使う）とシェルの element の列（seriko の `table.rs`・`looper.rs`・`trigger.rs`・`actor.rs`） | `mouse-click-wheel-events`・`seriko-interval-combinations`・`seriko-script-triggers`・v0.2.0 の時刻の土台（`talk-anchor-interrupt-ticks`） | ○ |
| `resident-ghost-rotation`（**10-21 起票**・開発者「裏でSHIORIだけ常駐維持しておいて、さくらスクリプト投げてきたゴーストを取っ替え引っ替え切り替えるモード」） | その他・区分 A | 裏で何体ものゴーストの SHIORI を動かし続け、画面には 1 体だけを出すモード（SSP に無い areka 独自）。裏のゴーストへは時刻のイベント（今は `OnSecondChange`）だけを送り、台本を返したゴーストへ SHIORI を降ろさずに表示を替えて再生する。複数が返したら、表示中のゴーストの再生が終わるまで次を待たせ、届いた順に出す | M〜L（14〜20） | v0.4.0・`multi-ghost-runtime` の席の集まりの上（`emo2_boot/` の切り替えと着替え・`mcp/resolve.rs` ほかの宛先） | `multi-ghost-runtime`・v0.2.0 の wintf・dola の基幹の整備 | ○ |
| `parallel-visual-script`（**10-21 起票**・`/kiro-discovery`・開発者「マウスホバーの方向性」の 2 つ目「トークを止めない即時再生モードでさくらスクリプトを割り込ませる」） | その他・区分 A | 台本の冒頭に areka 独自の印のタグがあるとき、今のトークを止めずに並走で再生する（ukadoc に無い・PLUGIN の `ScriptOption: nobreak` は待ち行列で別物）。並走でしてよいのは見た目と音だけ（面・`\![bind]`・`\i`・`\![anim]`・音・時間の待ち）で、文字・バルーン・選択肢・`\x` は読み捨てて記録に出す。本数は縛らない。印の無い台本は今どおり置き換え | M（12〜16） | v0.4.0・トークの列（kanade の `steady.rs`・`events.rs`・`types.rs`、ghost の `dispatcher.rs`、sakura の `drive.rs`・`compile.rs`、seriko の `looper.rs`・`actor.rs`、`emo2_boot/talk_lifecycle.rs`） | v0.2.0 の時刻の土台（`talk-anchor-interrupt-ticks`＝同じ `drive.rs`・`dispatcher.rs`）・`talk-thread-count-growth` | ○ |
| `sylphya-sqlite-kvs`（**10-21 起票**・開発者「areka-sylphyaで行っている永続化について、sqliteベースへの移行を検討せよ」） | その他・区分 A | sylphya の保存を `sylphya.toml` から sqlite（rusqlite `bundled`）へ移し、速い層（Rust の型付きの取っ手）・メモリーの sqlite（揮発の property）・ファイルの sqlite（覚える値）を 1 つの KVS の口で扱う。住み場所は鍵の表が決める。アプリ・ゴースト・シェル・バルーンの 4 層を重ねて読む（バルーン→シェル→ゴースト→アプリ）。番人のアクターが接続 1 本に 4 つのファイルと `:memory:` を ATTACH する。`sylphya.toml` は一度だけ取り込んで残す。アプリ層の書き手が何人でも消えない | L（18〜24） | v0.2.0・sylphya の列（`crates/areka-sylphya/`・`boot_resolve.rs`・`placement/persist.rs`・`roots/`・`areka-ghost` の `sylphya_wiring.rs`／`runtime.rs`＝wintf・dola の改造と重ならない・依存の席を使う） | `audit-sylphya-actor`（同じ crate）・`readonly-mode`（書かない継ぎ目） | − |
| `mcp-set-property`（**10-21 起票**・同じ `/kiro-discovery`・開発者「MCP の set で書く」） | その他・区分 A | areka 独自の MCP の道具 `set_property`（`check_script` と同じ形）。鍵・層・値・対象のゴーストを受け、KVS の名前での set で書く。書けない鍵と書けないときは誤りで返す。`sylphya.toml` の手書きの代わり（`shell-balloon-scale-setting` の brief を改める） | XS〜S（4〜7） | v0.2.0・`crates/areka-mcp/src/tools/`・`crates/areka/src/mcp/` | `sylphya-sqlite-kvs` | − |
| `sakura-time-critical`（**10-03 起票**・`emo-text-file-split` の実機の観測から） | その他・区分 C | 台本の `\t`（タイムクリティカル）を読み、台本の終わりか中断・選択まで、マウス系などの通知を止めて `Status` に `timecritical` を載せる（今は読まない＝`\t` を書いたメニューもなでなでの返事に置き換わる） | S〜M（8〜11） | kanade の列・台本のコンパイルの列（`sakura/decode.rs`） | `status-execution-states` | − |
| `makoto-dll-host` | その他・区分 C | MAKOTO/2.0 の DLL のホストと起動時の鎖（後から差し替えられる入れ物の形）。`\![load/unload/reload,makoto]` は 10-04 に `makoto-reload-directives` へ切った | L（15〜18） | host32-host の列 | なし（`Cargo.lock` の席・host32-host の列の順） | − |
| `mcp-user-response`（**10-05 起票**・同） | 優先・区分 A | 選択肢・入力欄の答えと、なでる・クリックなどの操作を、`since_id` 付きでエージェントへ返すツール 1 本（さくらスクリプトでは届かない返り道・SSP にも無い） | M〜L（12〜18） | kanade の列（choice とマウスの配線） | `mcp-author-tools`・`mcp-kanade-tools`・`mouse-drag-events` | − |
| `mcp-shiori-query`（**10-05 起票**・同・開発者「SHIORI への情報要求が無い・raise_event はトークになるだけ」） | 優先・区分 A | ゴーストへ情報を問うツール 1 本＝SSTP NOTIFY と同じ振る舞いに `X-MCP-PassThru-*` の往復を足す（作法は正典の `X-SSTP-PassThru-*` を写し、名前の頭だけ経路に合わせる＝開発者裁定・ghost_terminal の `ShioriEcho` 系が実例）。データはヘッダで返り、台本が空なら喋らない。SHIORI リソースを外から直接読む形は正典に無い＝要件の議題（推しは採らない） | M〜L（12〜16） | kanade の列（`actor_resources` の隣・host32-host の `shiori3.rs` も触る＝`script-security-level` と同じウェーブに置かない） | `mcp-kanade-tools`・✅ `script-security-level`（ヘッダの入れ物・10-23 着地＝済み） | − |
| `script-impact-tiers`（**10-05 起票**・同） | 優先・区分 A | タグの影響の段＝高（ベースウェアの外）・中（ベースウェアの中で演技を超えるもの）・低（アクターの演技の範疇・ゴースト切替を含む＝**制約なし**・開発者確定）の正本の表と、出どころごとの扱い（中・高は要件の議題） | M（10〜14） | `script-security-level` の後 | ✅ `script-security-level`（10-23 着地＝済み）・`mcp-kanade-tools` | − |
| `range-choice-tag`（**10-05 起票**・同） | 優先・区分 A | `\__q[ID,…]…\__q`（範囲を丸ごと選択肢にする・複数行・画像のバナー）。未実装・引受先なしだった（`anchor-tag-canon` の brief の「実装済み」は誤り＝同 brief を直した） | M（10〜14） | **C6-①**・文字とバルーンの列・台本のコンパイルの列（✅ `anchor-tag-canon`〔10-10 着地〕の次） | ✅ `anchor-tag-canon`（10-10 着地）・`choice-script-prefix` | − |
| `link-context-copy`（**10-05 起票**・同・**areka 独自の拡張**＝ukadoc に右クリック・クリップボードの正典は無い） | 優先・区分 A | 選択肢・アンカー・範囲の選択肢を右クリックすると小さなメニュー（`menu/win32.rs` を使い回す）を出し、行き先をクリップボードへ。決め方は 3 段＝`script:` の中の開く系の行き先 → ID や引数の `http(s)://`・`file:///`・`mailto:` → 表示されている文字（開発者裁定） | S〜M（7〜10） | 文字とバルーンの列（`input_events/` のバルーンと箱の押下） | `open-external-tags`・✅ `anchor-tag-canon`（10-10 着地）・`range-choice-tag` | − |
| `balloon-link-hover`（**10-05 起票**・同・開発者「マウスホバーって出せる？ URL とか」） | 優先・区分 A | 正典の `balloon_tooltip`・`OnChoiceHover`・`OnAnchorHover` を実装し、ゴーストが何も返さないときだけ行き先（`link-context-copy` の 1・2 段目）を出す | M（10〜13） | 文字とバルーンの列と kanade の列（`link-context-copy` の次） | `wintf-tooltip`・`link-context-copy`・✅ `anchor-tag-canon`（10-10 着地）・`range-choice-tag` | − |
| `shell-tooltip`（**10-05 起票**・同） | その他・区分 A | キャラクター窓のツールチップ＝surfaces.txt の tooltipブレス（`sakura.tooltips` など・今は黙って吸収）・SHIORI の `tooltip`・`currentghost.seriko.tooltip.*`。どれも引受先なしだった | M（8〜12） | シェルの element の列とプロパティの動く値の列（着手の前に照合） | `wintf-tooltip` | − |
| `dev-folder-alias`（**10-05 起票**・同） | その他・区分 A | 開発中の git の作業ツリーを「名前 → フォルダ」で登録し、探す順のいちばん上に置く（ゴースト・バルーン・ゴーストごとのシェル）。同じ名前の入っているものを隠して開発版を動かせる。エイリアスの先へはインストールも更新も書き込まない。登録は App の記憶へ手で書く＋環境変数（メニューなし） | M（8〜12） | `baseware-root-list` の後・同じ列 | `baseware-root-list` | − |
| `test-roots-under-target`（**10-07 起票**・`ghost-standard-balloon` の完了時の棚卸） | その他・区分 C | 既存のテストの `TempPath::new`（OS の一時フォルダ・80 ファイル 240 か所）を `target\test-roots` の下へ移し、`%TEMP%` を使うテストが黙って入らないようにする（入口を替えるか呼び出しを置き換えるかは要件で決める） | S〜M（入口の差し替え 5〜8） | いつでも（`temp-path-kit` とテストだけ・`ghost-session-test-load-flake`・`areka-test-binary-stability` と同時に走らせない） | `ghost-standard-balloon`（`TempPath::under_target`） | − |
| `present-emit-tail-latency`（**10-07 起票**・`animated-image-playback` の完了時の棚卸） | バグ・区分 B | シェルの絵の適用の約 1.2% が 16 ms を超える（最大 315〜360 ms）。全部が `presenter/show.rs` の最後に記録した段から `timing.emit(` までの測っていない区間。区間に段を足して原因を特定し、根本を直す。動く絵は適用の回数を約 13 倍にするので本数も増える | S〜M（5〜10・半分は計測） | ウェーブの外（測ることが本体） | `animated-image-playback`（✅ 10-07） | − |
| `ukadoc-catalog-refresh`（**10-22 起票**・`/kiro-discovery`・`script-security-level` の要件討議の議題 3・開発者「起票漏れしないか？起票するなら今しましょう」） | その他・区分 C | ukadoc の正典の写し（`catalog.toml`・`ukagaka-doc-mcp` 0.2.7・2026-08-24）を最新（0.10.3）で取り直し、増えた項目（`\![execute,filewatch]` 系 3 つなど）を網羅台帳へ差し込み、消えた・id の変わった行を移す | M〜L（棚卸㉙: 12〜16・写しの 0.10.3 で項目の id と URL の形が変わった〔1,749 のうち一致 65〕・出どころの URL まで書き直すなら 16〜22＝要件で `ukadoc-source-url-rekey` を切る・台帳のすべての行を書き換えるので、台帳の行を触る spec と同じウェーブに置かない） | 台帳の行の状態や持ち主を変える spec が 1 本も走らない席（`coverage-roadmap-refresh` と同じ・v0.1.0 の後が安い）・`coverage-roadmap-refresh` の前に取る（下流） | なし | − |
| `coverage-roadmap-refresh`（**10-02 に覚え書き `ukadoc-coverage-custody` を合流**） | その他・区分 C | 網羅台帳の手書きの数と持ち主が黙って偽になるのを止める（検査が `completed/` を走査・128 行の持ち主の付け替え・実在しない仕様名 107 項目） | M（14〜18） | 台帳の行の状態や持ち主を変える spec が 1 本も走らない席（注記だけ足す spec とは並べられる） | なし | − |
| `clippy-199-lints` | その他・区分 C | clippy 1.99 で既存のコードに出た lint（dola・emo-compose・kanade・shiori 系・areka の約 40＋19 か所）を、振る舞いを変えずに消して `cargo clippy --workspace --all-targets -- -D warnings` を緑にする。2026-10-31 に件 1（emo-present のテスト 5 本の 7 件）を足した。test-all に clippy の段を足すかは議題 | M（12〜16・測り直しで 20 を超えたら段 1 と段 2 に） | C5 の候補（段 1＝`dola` の `validate/`・`runtime/`・`compile/` と `shiori-host32-host/tests/`〔棚卸㉙・`shiori-abi` は `shiori4-api` が消す〕は空き席でも取れる・段 2 は列が空いてから・最初のタスクで測り直す） | なし | − |
| `anchor-style-canon`（**10-04 起票**・`anchor-tag-canon` から切り出し） | 優先・区分 C | `\f[anchor*]` 16 項目×3 状態・descript の `anchor(.notselect\|.visited).font.*`・訪問済み・縦書きの下線（普通のバルーンと箱） | M（10〜13・descript の `anchor.*` の残りまで入れると 12〜16） | 文字とバルーンの列（`choice-marker-styling` の隣） | ✅ `anchor-tag-canon`（10-10 着地）・`text-align-shadow-canon` | − |
| `extra-character-windows`（**10-04 起票**・覚え書き「3 人目以降のキャラクターの窓」） | その他・区分 C | `char{n≧2}` を持つゴーストで 3 人目からも窓とバルーンを出す（今は `derive_scopes` が 0・1 に固定。配置の `detect_scopes` も 2026-10-17 に `emily-ghost-verification` が 0・1 へ絞った。10-18 に「3 人目の文字が装着先未解決のまま溜まる疑い」を brief へ追記） | M〜L（14〜20） | `emo2_boot` の列 | `balloon-canon-residue`・`shell-companion-balloon` | − |
| `update-check-options`（**10-04 起票**・覚え書き「更新のオプションと `OnUpdateCheck*`」） | その他・区分 C | 更新のオプション（`checkonly`・`testonly`・`recovery`）・`OnUpdateCheck*` の 4 語・`other_homeurl_override`（今は受けて何もしない） | M（10〜14） | `network-update-canon-order` の後（同じ `update/`・`update_cue.rs`） | `network-update-canon-order` | − |
| `sakura-embed-directive`（**10-04 起票**・`property-query-channels` から切り出し） | その他・区分 C | `\![embed]`（SHIORI へ問い合わせた結果を台本へ埋め込む） | M（12〜16） | kanade の列の後 | `property-query-channels` | ○ |
| `makoto-reload-directives`（**10-04 起票**・`makoto-dll-host` から切り出し） | その他・区分 C | `\![load/unload/reload,makoto]`・切替や更新の後の付け直し・`mcp-reload` の口 | M（7〜10） | host32-host の列 | `makoto-dll-host` | − |
| `currentghost-property-others`（**10-04 起票**・`currentghost-property-tree` から切り出し） | その他・区分 C | `currentghost.*` の残り（`balloon.scope` の 19 項目の外） | M（12〜16） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `system-property-values`（**10-04 起票**・`property-catalog-lists` から切り出し） | その他・区分 C | `system.*` と Win32 の採り口 | M〜L（14〜18・`windows` の機能を足す＝`Cargo.toml` の席） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `seriko-script-triggers`（**10-10 起票**・棚卸㉓で `seriko-trigger-intervals` から切り出し） | 優先・区分 A | SERIKO の起動のきっかけのうち台本の側＝`yen-e`・`never`・台本の `\i[ID]`・`animation*.name` の読み取り | S〜M（8〜10） | シェルの element の列・台本のコンパイルの列（✅ `anchor-tag-canon`〔10-10 着地〕の後） | ✅ `seriko-trigger-intervals`（10-10 着地）・✅ `anchor-tag-canon`（10-10 着地） | − |
| `seriko-interval-combinations`（**10-10 起票**・棚卸㉓で同 spec から切り出し） | 優先・区分 A | `bind+always` など、`always` と組み合わせた interval を動かす（正典は「`+` でつなげる」の 1 文だけ＝どの語を・どの順で・いくつまでが議題） | S（4〜6・`always` を含まない組み合わせまで入れると +1〜2） | シェルの element の列（compose の `nesting.rs`・`plan.rs`・`plan_always.rs`・`plan_extent.rs` と seriko の `table.rs`・`parts.rs`・`looper.rs`＝`extent-element-offset`・`animated-image-import`・`seriko-script-triggers`・`draw-methods-canon` と直列） | ✅ `seriko-trigger-intervals`（10-10 着地）・`extent-element-offset` | − |
| `draw-methods-blend`（**10-10 起票**・棚卸㉓で `draw-methods-canon` から切り出し） | 優先・区分 A | `blend-*` の描画メソッド 28 種と `-fast` の形 27・古い別名 2 を描く（compose の `method.rs`・`blit.rs`・台帳の 57 行。読み手には触らない＝`draw-methods-canon` の読み手が `blend-*` の語も運ぶ形にする。ukadoc は計算式を書いていない＝議題） | M（9〜12） | シェルの element の列（`draw-methods-canon` の後） | `draw-methods-canon`・`wintf-d2d-composite-rules`（10-15 に D2D の道に決まった） | − |
| `wintf-visual-tree`（**10-21 起票**・開発者「`ecs\visual` 層のコード群は実装準備されているが採用されていない。これらを新しいビジュアル層・描画層として取り込んでほしい。D2D合成の多様化に必要と考える」「実施フェーズはwintf/dola改修のタイミングとする」） | 優先・区分 A | 作りかけの描画の木（`crates/wintf/src/ecs/visual/`・外からの参照 0）を本番の描画層へ上げる。WUC の描画面の下に木を置き（開発者 10-21）、節ごとに重ね方（`D2D1_COMPOSITE_MODE`）・補間・変形（回転・拡大・反転）・切り抜き（矩形・任意の図形）を効かせて 1 枚に描く。木の無い描画面は今と同じ。描く係・スケジュールへの配線・`WorldVisual`／`VisualVersion` を埋める system・傷 8 つの直し（COM の番地の hash・数学の二重・`GetBounds` の失敗を黙って 0 に・深さの番兵ほか）・WARP の決定論のテストの足場 | M（12〜18） | wintf だけ（`ecs/visual/`・`numerics/`・`ecs/graphics/systems/render.rs`・`ecs/world/mod.rs`）。v0.2.0 の先頭＝`wintf-d2d-composite-rules` の前。見直しの `audit-wintf-graphics-world`・`audit-wintf-runtime`（v0.2.0 と重なる分＝本 spec より先に取る）の後（どちらの brief にも「消さない」と書き足した） | 見直しの 2 本 | ○ |
| `wintf-visual-damage-redraw`（**10-21 起票**・`wintf-visual-tree` と同じ指示から分けた） | 優先・区分 A | 描画の木の前フレームと今の描く列を比べ（`draw/README.md` の構想＝hash の列と LIS）、変わった範囲だけを描き直す。描き先は `wintf-d2d-composite-rules` の自前の合成の入れ物（WUC の描画面は部分の `BeginDraw` を断る）。答えは毎回全部描いた答えと画素で一致（WARP）。スクロールの写しを入れるか・効きを測ってから入れるかは議題。件 1（2026-10-31 `animated-surface-log-flood` から）＝ぱすたの面 1002 で何もしていないあいだの release 版の CPU が SSP の約 3 倍（15.4%・5.6%）＝適用の外の内訳を測ってから縮め、SSP の水準を目標にする | M（10〜15） | wintf だけ（`ecs/visual/draw/`・`render.rs`）。`wintf-d2d-composite-rules` の後・`draw-methods-canon` とは並べられる（触るファイルは設計で確かめる） | `wintf-visual-tree`・`wintf-d2d-composite-rules` | − |
| `wintf-d2d-composite-rules`（**10-15 起票**・開発者「D2D合成アルゴリズムの改善も入れる予定なのです。wintfではコマンドリストの合成ルールが現在固定ですが、これをD2D合成ルールを個別に指定できるようにして、高度な合成を実現したい」・`draw-methods-canon` の分割の節が予告していた wintf だけの spec） | 優先・区分 A | wintf で、描くものごとに合成の規則（重ね方 `D2D1_COMPOSITE_MODE`・ブレンド `D2D1_BLEND_MODE`・不透明度・元の矩形・変形）を指定できるようにする（今は `render.rs` の `render_surface` が `SOURCE_OVER` 固定）。合成の結果を自前の入れ物に持ち、そこから 1 ピクセルをその場で読み戻す口を作る（クリックの透過の α の出どころ）。合成の答えのテストは D2D を WARP で動かして固定する（開発者 10-15「画像合成に関するテストは、CPUに切り替えるべき」）。ukadoc の語は持ち込まない | M（10〜14） | wintf だけ（`ecs/graphics/systems/render.rs`・`ecs/visual/draw_content.rs`・`com/d2d/`・`com/wuc.rs`）＝シェルの element の列の読み手の鎖とは重ならない。`draw-methods-canon` の前。2026-10-21 に議題 1 が決まった（描画の木を本番へ上げる＝`wintf-visual-tree` の後・重ね方と補間はあちらが先に描く） | `wintf-visual-tree` | ○ |
| `hit-id-pass`（**10-15 起票**・開発者「当たり判定用に『MRT（IDパス描画）？』とかを導入して、ヒットテストIDだけを持ったサーフェスを持つこともありだと思う」「Headなどのリージョン範囲を、画像で取り込む仕様があるはずです。これに対応するためにもヒットテストIDが欲しい」） | 優先・区分 A | `collisionex` の `region`（画像の指定色の領域・SSP 2.5.19）を読んで当たり判定にする。当たり判定の図形を窓ごとの「ID の描き先」に描き、1 ピクセルを読んで ID → 名前を引く。仕組み（ID の描き先・描き方・読み戻し・幅）は wintf、意味（ID の表・図形の組み立て・`region` の画像）は areka。ID の幅は選べなくてよい（B8G8R8A8 を α なしで 24 ビット）。GPU の ID の描き先か、wintf に在る CPU の `ColorMapData` かは議題 | M（10〜14） | `collisionex-regions` と `wintf-d2d-composite-rules` の後（読み手には触らない＝`collisionex-regions` が `region` の行も型に運ぶ） | `collisionex-regions`・`wintf-d2d-composite-rules` | ○ |
| `winget-release-automation`（**10-10 起票**・棚卸㉓で `winget-manifest-submission` から切り出し） | 優先・区分 A | `.github/workflows/winget.yml` と `README.md`・`dist/README.txt` の行。初回の手提出が winget-pkgs に取り込まれてから始める | XS〜S（3〜4） | 配布と公開の列（✅ `winget-manifest-submission`〔10-10 着地〕の後） | ✅ `winget-manifest-submission`（10-10 着地）＋winget-pkgs の取り込み（手提出の PR は人の承認待ち）＋✅ `user-data-root`（10-17 着地） | − |
| `check-script-arg-checks`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 各機能の受け口が自分で読む引数の誤りを `check_script` が診る。10 個のうち 7 個（`\![move]`・`\![set,zorder]`・`\f`・`\_l`・`update`・`install`・`bind`）は判定が既に純粋な関数＝呼ぶだけ。受け口の中で判定している 3 個（`change`・ゴーストとシェルの切替・readme）を取り出す | M（8〜12） | `mcp/check_script_judge.rs`（`mcp-strict-errors`・`script-impact-tiers`・✅ `anchor-tag-canon`〔10-10 着地〕・`talk-fast-forward`・`font-size-keywords` ほかと分け合う）・`emo2_boot/{change_cue,switch_cue,readme_cue}.rs` | なし | − |
| `config-parse-diagnostics`（**10-10 起票**・棚卸㉓で覚え書きから・優先度は低い） | その他・区分 C | 設定の読み取り（`parse_kv`・`shell::parse`・`balloon::parse`）が捨てた行・読まなかったキーを言えるようにする（値で返す形を推す・ログの上限と水準が議題） | M（9〜13） | 読み手の鎖（シェルの element の列）と文字とバルーンの列の両方が空いたとき | なし | − |
| `font-size-keywords`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | `\f[height,…]` のスタイルシートのキーワード（`larger` など）で字の大きさを変える（今は語彙だけ受けて記録を出す。ukadoc は語の一覧も大きさの対応も書いていない＝areka の決まりを作る） | S（3〜5） | 文字とバルーンの列（`look.rs`・`state_decoration.rs`）と `mcp/check_script_judge.rs` | なし | − |
| `elevated-drop-filter`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 管理者として動かした areka へ投げ込みを届けるか（Windows の権限の境界をゆるめる）、既知の制限のまま閉じるかを決める（説明書には既に制限として書いてある＝やらないなら直しは 0） | S（2〜4・やらないと決めれば 0） | `placement/spawn.rs`（`dpi-realign-remembered-chain`・`extra-character-windows` と分け合う） | なし（開発者の決めごとが議題 1） | − |
| `mouse-hover-event`（**10-24 起票**・`dpi-realign-remembered-chain` のタスク 6.3 の実機の確かめで開発者が気付いた） | その他・区分 A（開発者が起票を頼んだ） | キャラクターの当たり判定の上でカーソルを止めても `OnMouseHover` を送らない（網羅台帳 `absent`・持ち主なし）。正典は「マウスが静止している際に発生」と Reference の並びだけを書き、止まる時間・繰り返し・ドラッグ中とトークの最中に黙る＝SSP で測ってから裁定する。ぱすたは使わない | S（4〜6） | 版は次の棚卸で決める（v0.1.0 には入れない）・`mouse-click-wheel-events` の着地の後（`input_events/`・kanade の `msg.rs`・`schedule/events.rs`） | `mouse-click-wheel-events` | − |
| `inputbox-user-input`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 入力欄 `\![open,inputbox]`・`\![close,inputbox,ID]` と `OnUserInput`・`OnUserInputCancel`（パスワード・日付・スライダーなどの兄弟の入力欄は範囲外） | M〜L（Win32 標準の入力なら 12〜16・バルーンに合わせた箱なら 20 を超える＝要件で切る） | `emo2_boot` の列（`consumer_ledger.rs`・`mod.rs`）と kanade の列（`schedule/events.rs`） | なし（Win32 標準の入力か、バルーンに合わせた箱かが議題 1） | − |
| `popup-menu-shell-look`（**10-18 起票**・棚卸㉗・v0.1.0） | 優先・区分 A | シェルの `menu.*`（右クリックメニューの色・絵・文字）をオーナードローで描き、着せ替えのメニュー（`sakura.menu,auto`・bindgroup の名前・`mustselect`）を出す。生きている決まり 6（オーナードローと着せ替えメニューは予約）を解くかが先＝裁定待ち。ぱすたの `menu.*` は Windows 11 の明るいメニューを写したものなので、見た目の差が大きいのは着せ替えのメニュー | M（14〜18・超えたら描き分けと着せ替えのメニューの 2 本） | メニュー（`menu/{mod,win32,plan}.rs`・wintf の `window_proc/mod.rs`） | 開発者の裁定 | ○（wintf と areka の間に、OS のメニューの中で同期に描く新しい約束・着せ替えのメニューだけなら −） |
| `emo-bake-log-overlap`（**10-31 起票**・`placement-measure-bake-once` の完了時の棚卸） | その他・区分 C | 起動の準備で採寸と資産の組み立てが同じ絵を 2 回焼くため、焼く段（0 寸・全透明の `warn!`）・面の表の畳み込みの `warn!`・バルーンの 2 層マージの基層の `descript.txt` の `warn!` が同じ絵・同じ面について 2 行ずつ出る。1 行ずつにする（案 1＝記録を出さない口を emo-atlas の `bake`・emo-compose の畳み込み・`load_scope_balloon_model` に足し採寸の道だけ黙らせる／案 2＝焼くのを 1 回にする・要件の段で裁定）。起動を止める失敗の記録はどの道でも出す | S（案 1 で 6〜9〔棚卸㉜〕） | いつでも | ✅ `placement-measure-bake-once` | − |
| `alert-dialog-per-monitor-dpi`（**10-31 起票**・`install-before-boot` の完了時の棚卸） | バグ・区分 B | 件 1: 告知の画面（`alert.rs` の `MessageBoxW`）が system aware で、拡大率の違う副のモニタに出るとにじむ見込みを直す（案 A＝`SetThreadDpiAwarenessContext` で囲む／案 B＝`TaskDialogIndirect`／案 C＝自前の窓・要件の段で裁定）。件 2: ゴーストを拡大率の違うモニタへ移した後にバルーンが枠なしで矢印だけ見えた件を見分け、描き直しの欠けなら直す | S〜M（件 1 は案 A で 3〜5） | いつでも | ✅ `install-before-boot` | − |
| `ondestroy-on-exit`（**10-22 起票**・棚卸㉘・完了 `mcp-reload` の残り物） | その他・区分 C | 終了とゴーストの切り替えで、SHIORI を降ろす前に `OnDestroy`（NOTIFY）を送る（リロードの場面は `mcp-reload` が済ませた）・網羅台帳の行の直し・送る順は SSP で測る | S（4〜7） | kanade の列（✅ `script-security-level`〔10-23 着地＝済み〕・`mouse-click-wheel-events` と同じウェーブに置かない） | — | − |
| `install-package-type`（**10-23 起票**・棚卸㉙・覚え書きから格上げ・いつか） | その他・区分 C | `install.txt` の `type,package`（ゴースト・シェル・バルーンなどをまとめた書庫）を、今の「知らない種類」で断る形から、中の 1 つずつを入れる形へ。1 つが断られたら残りを入れるか・完了のイベントの送り方は SSP で測る | S〜M（5〜8） | インストールの列（`crates/areka-nar/src/{manifest,plan,lib}.rs`・`crates/areka/src/install/{procedure,judge}.rs`＝`startup-install-option` と `judge.rs`・`readonly-mode` と `install/`） | なし（`bootghost` を起動の道へ渡すのは `startup-install-option` の後） | − |
| `sstp-receive`（**10-23 起票**・棚卸㉙・覚え書きから格上げ・いつか） | その他・区分 C | SSTP の受信（SEND・NOTIFY・Option・共通のヘッダ・`OnSSTPBreak`）。kanade には SEND と NOTIFY の入口がもうある（`msg.rs`）＝足りないのは網の側。MCP の待受（SSP と 9801 を早い者勝ち）と同じ口で受け、最初の行で振り分ける案が議題。`balloon-markers` の `sstpmarker`／`sstpmessage` が画面に出るのはこの後 | L（24〜30＝要件で ① 受け口と SEND・NOTIFY〔14〜17〕／② `sstp-receive-methods`〔COMMUNICATE・EXECUTE・GIVE・INSTALL・HTTP 越し・10〜13〕に切る） | kanade の列（`schedule/sstp.rs`・`msg.rs`）と MCP の待受（`crates/areka-mcp/src/server.rs`）・`cli/listen.rs` | なし（`X-SSTP-PassThru-*` は `mcp-shiori-query`、`nobreak` は `shiori4-api` の列の後） | − |
| `balloon-inline-image`（**10-23 起票**・棚卸㉙・覚え書きから格上げ・いつか） | その他・区分 C | バルーンの文中に画像を置く `\_b[…,inline]`（`range-choice-tag` の例 1〔バナーの画像を選択肢にする〕に要る）。新しい cue の種類を足す＝emo-text・`areka-ghost` の受け手・seriko の分岐に波及 | M（10〜14・`\_b[…,x,y]` の形まで入れるなら 13〜17） | 文字とバルーンの列（emo-text の席を使う・`areka-parsers/src/sakura/{decode,model}.rs`・`areka-sakura/src/compile.rs`・dola の `cue/command.rs`） | なし | − |
| `ghost-alias-file`（**10-23 起票**・棚卸㉙・完了 `dot-sakura-specimen` の research の残り物・v0.2.0） | バグ・区分 B | シェルのフォルダの `alias.txt`（`sakura.surface.alias` ほか）を読まない。今は `kero.surface.alias` だけを読み、全部のキャラに 1 つの表を使い、面の引き当ては別名を見る前に番号を返す（`areka-parsers/src/shell/decode.rs`・seriko の `resolve.rs`）。検体のどっとさくらの `alias.txt` は持っていない面の番号（例: さくらの 9）を持っている面へ振り替える＝`\s[9]` が外れるはず（まだ誰も見ていない）。`ghost/master/alias.txt`（`makoto-dll-host`）とは別のファイル | S〜M（6〜9） | シェルの element の列（`shell/{decode,model}.rs`・emo-present の `shell_target.rs`・emo-compose の `{world,fold}.rs`・seriko の `resolve.rs`・`emo2_boot/assets.rs`） | なし（どっとさくらでの実機の確かめは ✅ `shiori2-protocol`〔10-24 着地〕の後＝それまで台本が流れない） | − |
| `seriko-missing-surface-default`（**10-24 起票**・完了 `descript-window-keys` の裁定 11・いつか） | その他・区分 C | 台本の `\s[n]` が今のシェルに無い面を指したときの扱いを決めて入れる。ukadoc は「存在しない番号を\sタグで指定した時などに代わりに表示する」＝既定の面を出すと書くが、今の areka は記録だけで表示を変えず、SSP 2.9 も既定の面を出さない（10-23 の測り＝`surface.num` が -1）。要件の段で ukadoc の字面と SSP の実測のどちらに合わせるかを決める（推奨＝ukadoc・既定の面は `placement::DefaultSurfaces` の候補の最初に在る面） | S（4〜6） | シェルの element の列（seriko の面の切り替え・`seriko-pattern-calls`・`ghost-alias-file` と同じ場所に近い） | `descript-window-keys` | − |
| `seriko-legacy-pattern-format`（**10-30 起票**・`seriko-pattern-calls` の要件の討議の議題 1・棚卸㉛でいつかに置いた） | その他・区分 C | 旧 SERIKO 1.x の書式（`*interval`・`*pattern*,サーフェスID,ウェイト,描画メソッド,X,Y`・`*option,exclusive`・旧 `alternativestart`）の行を読み手で読み、新しい書式と同じ型へ写す。今は `animation` で始まらない行を記録なしに飛ばす。`version,0` との関係と新旧の混ざりは要件の段で（ukadoc・読めない点は SSP で測る）。ぱすたと手元の検体 10 本は旧書式 0 行。細部は brief | S（5〜8） | シェルの読み手の列（`areka-parsers` の `shell/{lexer,decode,model}.rs`＝`script-small-tags`・`seriko-interval-combinations`・`config-parse-diagnostics` と同じウェーブに置かない） | ✅ `seriko-pattern-calls`（10-31 着地） | − |
| `surfaces-basepos` | 据え置き・区分 D | surfaces.txt の `point.basepos`。宣言するシェルが要るまで始めない | S | シェルの element の列の最後 | — | − |
| `property-ipc-transport` | 据え置き・区分 D | 台本を通さないプロパティの読み取り。**最初の 1 段（調べて裁定）で終わる形へ縮める** | XS〜S（裁定まで 2〜4） | — | — | − |
| `dpi-transition-two-tick-bounce` | 据え置き・区分 D | 拡大率の切替で位置と大きさが 1 コマずれる（開発者が許容）。着手するなら最初に測り直し、跳ねが無ければ取り下げ | S〜M | — | — | − |
| `zorder-chain-residue` | 据え置き・区分 D | 間欠赤のテスト族と文書。09-11 以降 main で赤 0 件＝先回りしない（A-2 は 10-05 に `ghost-session-test-load-flake` へ移した） | S〜M | — | — | − |
| `text-reveal-dance` | 据え置き・区分 D | 字が現れるときだけ跳ねる・揺れる（夢・開発者が望んだときだけ） | M〜L | — | `text-reveal-fade` | − |
| `window-snap`（**10-15 起票**・**夢の段・覚書**・開発者「夢仕様だけど起票お願い」） | 据え置き・区分 D | マスコットが床・ほかのアプリの窓の上・窓の下・壁・窓の陰に吸着する「スナップ API」。本体が吸着の関係を語彙（コンテキスト）で持ち、シェルの定義だけで姿を出し分ける。今の `Anchor`（`alignmenttodesktop`）を一般化し、相手にほかの窓を足す（窓の見張りは WinEvent・見た目の枠は DWM）。落下・覗き・積み重なりは後ろの段。議題 7 点は brief | L 超（要件で切る・最初の 1〜2 段だけ） | — | 土台の持ち主（議題 1）・積み重なりは `extra-character-windows` と多重ゴースト（予約） | ○ |
| `emo-text-canon-residue` | 据え置き・区分 D | **取り下げ予定・着手しない**。残っていた 1 件（折り返しの警告にバルーンの名前を出す）は `shell-balloon` が引き取った。フォルダを消すと網羅台帳の整合検査（`roadmap-draft.md` の `[[spec]]` の表）が赤くなるので、表から外すのと一緒に `coverage-roadmap-refresh` が片付ける | — | — | — | − |
| `release-code-signing`（**10-02 起票**・配布と公開・任意） | 据え置き・区分 D | SignPath Foundation（オープンソース向け・無償）に申請し、受理されたら CI で署名する。winget に署名は要らないので急がない | S（6〜9） | 配布と公開の列の最後 | `release-ci-workflow`・数回のリリースの実績 | − |
| `compose-f16-precision`（**10-15 起票**・開発者「wintfで利用しているスワップチェーンあるいはオフラインバッファについて、u8からf16精度のバッファにして、合成精度を強化したい」「低優先度で起票」） | 据え置き・区分 D | 途中の入れ物を 1 色 8 ビットから f16 にして合成の精度を上げる。**丸めが積もるのは GPU でなく CPU の合成**（`areka-emo-compose` の u8 の SourceOver）＝GPU の入れ物だけを替えても効き目は文字の層と拡大縮小だけ。合成の精度は `draw-methods-canon` の「合成をどこで行うか」と一緒に決める。色空間（f16 は DWM が線形で読む）と、先に測って差が見えなければ取り下げるかは brief の議題 | M〜L（測りと引数化だけなら S〜M） | — | `wintf-d2d-composite-rules`（自前の合成の入れ物）・`draw-methods-canon`（10-15 に合成は D2D の道に決まった） | − |
| `tick-gate-adoption` | 保留・区分 D | 既定で切の門を入れるか（待機中の CPU 17〜22% → 3% 未満の見込み）。短い A/B の測り方を先に組む | M〜L | 保留 | — | − |
| `shiori4-api`（**10-10 起票**・開発者「SHIORI4 の API 整備の spec はあるか・どこかの段階で着手しないといけない」・**10-23 書き直し**＝開発者「COM で作ろうかと原案を出していたインターフェースを、クロスプラットフォームを見据え C ABI でアクセスできるように仕様調整した」・区分 C・**版は v0.2.0**＝10-23 開発者「dolaなどの基幹部分大整備と同じタイミングで」） | その他・区分 C | SHIORI4 の正本は COM の `IShiori` から別リポジトリ `ekicyou/shiori4` の **C ABI**（`shiori4_abi`・`shiori4`・`shiori4_host`・crates.io 0.1.0〔2026-10-24 公開・0.0.x と二進互換でない＝応答をホストが作る `copy_from_slice`。`shiori4_host` の芯が引き受けるので計画は変わらない〕）へ移った。areka はホスト側として `shiori4_host` を使い、経路を「kanade → `ShioriBackend` → helper のプロセス → SHIORI/3.0／SHIORI4」に並べる（10-10 の方針を引き継ぐ）。**SHIORI の実体はどの版でも別プロセス**（10-23 開発者「SHIORIの実体は別プロセスで動かすこと」＝SHIORI3 のブリッジと同じ環境・カレントディレクトリやプロセス全体の状態を持つランタイムの都合・`ShioriWiring::InProc` は無くなる）。v4 のペイロードは SHIORI/3.0 のヘッダ部分そのもの。すること＝helper の v4 の読み方（中継の `Host`・x64／arm64／i686 の helper）・IPC の札の追加（同期の問い合わせ・post・log）・本体の受け側（`getproperty`／`setproperty` → sylphya・post の `SEND`／`NOTIFY` → kanade の口）・descript.txt からの v4 DLL の選び分け・`load` のパスの形・自前の COM の面（`shiori-abi`・`shiori4-testdll`）と JSON-RPC の封筒の撤去（10-23 開発者「COM-SHIORI の作りかけは排除し shiori4 に完全移行」）・COMPAT §5 の書き直し。host-protocol と abi-conventions の語は 10-23 に shiori4・pasta との 3 者会談ですべて合意（割れなし・正本は shiori4 の作業ブランチ `claude/abi-host-protocol` の 2 本の brief） | 20 タスクを超える見込み（要件で切る） | 直列の列「host32-host」の後ろ（`shiori3.rs`・`client.rs`・`process_host.rs`・`shiori-host32-helper`・`shiori-host32-ipc`・`shiori_inproc.rs`・`shiori_wiring.rs`・`runtime.rs`・kanade の `shiori/real.rs`）。v4 の道のプロパティはこの spec、32bit の SHIORI/3.0 の道は `property-ipc-transport`（10-23 に分けた・どちらも IPC に札を足すので先に着手した方が形を決める）。許可の表を決める `property-query-channels` の後 | 実装がある程度整備されてから（開発者）＋shiori4 の `host-protocol` が入った版の公開（2026-10-24 に v0.1.0 で満たされた） | − |
| `tasktray-icon`（**10-10 起票**・開発者「タスクトレイの土台仕様を起票」「タスクトレイで出てくるべきメニューは今出てるシステムメニューと同じ」・区分 A） | その他・区分 A | 通知領域（タスクトレイ）に areka のアイコンを 1 つ出す土台。プロセスに 1 つの持ち物として、見えない窓とアイコンを持つ（ゴーストの切替で消えない・エクスプローラーの立ち上げ直しで出し直す・終了で消す）。マウスを重ねると「areka/[起動中のゴースト名]」。右クリックで、本体側のキャラクターの右クリックと同じメニューを出す（`menu/` の組み立てと表示はそのまま使い、キャラクターの窓に縛られた依頼の形 `MenuRequest` を広げる）。標準アイコンの絵は前提の `app-icon` が exe に埋める。議題＝左クリックとダブルクリックで何をするか・ゴーストがメニューを止めているときの扱い・ゴーストが居ない間の右クリック | M（10〜14） | `menu/trigger.rs`・`menu/mod.rs`・`main.rs`・新しい `tray/`・wintf に通知領域の部品。`menu/` を触る `popup-menu-residue` と直列（残件の相乗り先の候補）。棚卸㉔＝`main.rs`・`menu/mod.rs` が `baseware-root-list` と重なる＝その後 | `app-icon` | − |
| `tasktray-ghost-icon`（**10-10 起票**・同上・区分 A） | その他・区分 A | ゴーストが通知領域のアイコンと文言を決められるようにする。descript の `icon,ファイル名` と、台本の `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]`。`.ico` の読み込みは OS に任せる。動く絵の指定（`--duration`・`--runcount`）は、開発者の見立て（10-10「今はタスクトレイは基本隠れるので、動く意味は無い」）に沿って動かさない方向で議題にする | S（5〜8 見込み） | `\!` の受け口と descript の読み取り。重なりは着手のときに測る | `tasktray-icon` | − |
| `tasktray-balloon`（**10-10 起票**・同上・区分 A） | その他・区分 A | 台本の `\![set,trayballoon,オプション,オプション,オプション...]` で OS の通知を出し、押した・消えたを `OnTrayBalloonClick`・`OnTrayBalloonTimeout` で SHIORI へ送る。SSP の作者が BTS で説明した挙動 3 件（知らせの遅れ・OS の設定で出ない・時間切れが先に届く＝BTS 0000331・0000407・0000561）を議題にする | S〜M（6〜10 見込み） | kanade の列（`schedule/events.rs`）と `\!` の受け口・wintf の通知領域の部品 | `tasktray-icon` | − |
| `tasktray-minimize`（**10-10 起票**・同上・区分 A） | その他・区分 A | アイコン化（最小化）。メニューの「アイコン化」と台本の `\![set,windowstate,minimize]` でキャラクターとバルーンを隠し、通知領域のアイコンのダブルクリックで戻す。`OnWindowStateMinimize`・`OnWindowStateRestore` を送り、アイコン化の間は `icon.minimize,ファイル名` の絵にする。文言は `hidebutton.caption`。`alwaystrayiconvisiblebutton.caption`（「常にトレイアイコンを表示」）は項目を作るかどうかから議題。名指しの ukadoc の項目 2 つ（`icon.minimize`・`alwaystrayiconvisiblebutton.caption`）がアイコン化なしでは使いどころが無いので、持ち主の無かったアイコン化ごと 1 本にした。議題＝アイコン化の間に喋るか・時計を進めるか | M（10〜15 見込み・膨らめば要件の段で「状態と入り口」と「アイコン・文言・Status」に） | `menu/`・kanade の列（`schedule/events.rs`）・配置の層・wintf の窓の表示 | `tasktray-icon`・`tasktray-ghost-icon` | − |
| `seriko-talk-clock-fidelity`（**10-10 起票**・`seriko-trigger-intervals` の完了時の棚卸） | バグ・区分 B | 口パク（`talk,数値`）の時刻の写しを文字の層と食い違わないようにする: 文字の層と seriko が、層ごとに自分の cue の到着から起点を取る（時計も QPC と `GetTickCount64` で別）ので、`talk` の開始と文字の現れの差の見積もりが実機で −16.9〜+1.2 ms＝設計が引いた線（1 刻み 16 ms）に触れる／cue の届く時刻の揺れ（−9〜+50 ms）で、起点が台詞の途中で 1 回動く／中断の直後の新しい台詞で 1 文字を数え落としうる／文字の層の式を変えても赤になるテストが無い（`talk.rs` は写しで、共有でない）／後から見えた部品が、自分が見える前の区切りで鳴りうる／1 回の刻みで区切りを 2 つ以上越える場面は純粋な関数のテストだけ／`realign_talk` の天井。直し方（式を共有の関数にする・起点を 1 か所で取る・今のまま上限を記録する）は未決＝要件で選ぶ | M（8〜12・要件で測る） | シェルの element の列（seriko の `talk.rs`・`trigger.rs`・`looper.rs`・`looper_trigger.rs`）。式を共有の関数へ出すか、起点を 1 か所にする形を選ぶなら、文字とバルーンの列（emo-text の `state.rs`）と `emo2_boot` の列（`mod.rs`・`talk_clock.rs`）にも触る。席は `seriko-trigger-lottery-gaps` の後（2026-10-11 の集約で決めた＝同じ `trigger.rs`・`looper.rs`・`looper_trigger.rs` を触る）。`talk-fast-forward`・`sakura-time-directives`・`text-reveal-fade` より先に着地すると、あちらの変更を見張りが赤で知らせる | ✅ `seriko-trigger-intervals`（10-10 着地） | − |
| `real-run-input-guard-observations`（**10-16 起票**・`balloon-text-bugs` の実機の確かめ〔タスク 5.3〕の判定に使わなかった回で見た） | その他・区分 C | 実機の観察 2 件（どちらも 1 回だけ・再現していない・原因は未確認）: ⑴ ポインタの移動と押下を 1 回の `SendInput` で同時に送ると、areka の記録に移動も押下も残らなかった（先に移動してから押すと通る・クリック透過を外す判定の遅れの疑い・人の手では起きにくい） ⑵ 操作していない時間に `[visibility-guard]` と `[balloon-limit] Clamp` の WARN が 565 行続いた（直前に `loop ticker catch-up`・窓の 1 枚が最小化の位置 -32768・バルーンの x=-46→-323 の引き戻しの繰り返し・再走では再現せず・外からの窓の操作の疑い）。**最初のタスクは再現**＝再現しなければ記録を残して閉じてよい。正本は `completed/areka-P0-balloon-text-bugs/real-machine-signoff.md` の 4 章（生の記録は `target` の下で消えうる） | XS〜S（確かめと記録だけ 2〜4・直すなら 1 件につき 3〜5） | 単独（実機の走行。直すと決めたら ⑴ は wintf の窓の手続き、⑵ は `placement/follow/`・`placement/balloon_limit.rs`＝`dpi-realign-remembered-chain`・`window-snap` と照合）。実機の窓が出るので、負荷の走行と同じ時間に回さない | なし | − |
| `helper-first-load-refusal`（**10-24 起票**・`shiori2-protocol` の完了時の棚卸） | バグ・区分 B | 32 ビットの補助プログラムの SHIORI の読み込み（`load`）が時々断られる: 実物の pasta で回す `real_pasta_online` の 1 回目で `LoadReturnedFalse { last_error: 203 }`（`ERROR_ENVVAR_NOT_FOUND`）・2 回目は緑・1 回目は `TMP`・`TEMP` を `target` の下へ向け替えていた。完了 `emo2-real-run-stability` の件 2（原因不明で閉じた）の再発で、`last_error` の初めての観測。失敗した回の `pasta.log` は写しごと消えた。**最初のタスクは再現**＝失敗した回の記録を残す仕掛けを先に入れる・再現しなければ記録を残して閉じてよい。正本は `completed/shiori2-protocol/verification.md` の 4.3 節 | S（確かめと記録だけ 2〜4・直すなら 3〜6） | 単独（実機の走行＝排他テストの机。直すと決めたら helper の `shiori_proxy.rs` か試験の土台＝`shiori4-api` の helper の乗り換えと照合） | なし | − |
| `emo-text-line-gap`（**10-24 起票**・`vertical-choice-hover-band` の完了時の棚卸） | バグ・区分 B | 行間（hhea の lineGap）を持つ書体で、emo-text の行の箱の比（`draw_metrics.rs` の `measure_line_box_ratio`＝(ascent＋descent)÷upem・行間を数えない）が DirectWrite の組む行の箱より狭い（游ゴシック 20 で 25.74 対 32.04）。縦書きの帯は基線から置く形で済んだが、横書きの帯（`choice.rs` の `line_bands` の横書きの枝・`highlight_band_extent`・`highlight_band_offset`）は同じ比を読む＝行間を持つ書体の横書きで帯がずれる見込み（未測定）。**最初のタスクは測り**・直し方（比に行間を数える／横書きも基線から置く）は開発者の裁定。正本は `completed/vertical-choice-hover-band/research.md` の 16.4 節と 17.6 節 | S（4〜7） | 単独（実物の書体の読み戻し＝排他テストの机・`choice.rs` を `choice-ranges-one-function` と分け合う・emo-text に新しいソースファイルを足すなら 1 本だけの席） | なし | − |
| `background-result-tick-wake`（**10-17 起票**・`user-data-root` の完了時の棚卸・タスク 4.2 の点検の議題） | その他・区分 C | 背景のスレッドの結果（`user-data-root` の切り替えの写し `switch_copy.rs`・切り替えの資産の組み立て `switch_assets.rs`・kanade の切り替えの知らせ `change_cue.rs` ほか・着手時に洗い出す）が起床の旗（`tick_wake::mark`）を立てず、tick の門を入れた形では次に誰かが旗を立てた回か心拍（`TICK_HEARTBEAT_FRAMES` 30 コマ）で拾われる＝切り替えの写しの後のゴーストの立ち上げが最大 30 コマ遅れうる。「時刻は正確に扱う」に合わせ、送る側で旗を立てるかを背景の結果の全部でそろえ、旗を立てるファイルを `tick_gate_config_producers_tests.rs` の `AREKA_PRODUCERS` に載せる。今の既定（門は切）では遅れは出ない | S〜M（5〜9） | 版に縛らない（`crates/areka/src/emo2_boot/` の送り口・`tick_gate_config_producers_tests.rs`。旗の種類を wintf の `tick_wake.rs` に足す形なら `instant-to-interrupt-ticks` と直列）。`tick-gate-adoption` とは同じ時に走らせず、その前に着地させる。`switch_assets.rs` は `emo2_boot` の結線の列（`extra-character-windows`・`mcp-reload`）と同じウェーブに置かない | ✅ `user-data-root`（10-17 完了） | − |
| `talk-thread-count-growth`（**10-19 起票**・`mcp-kanade-tools` の完了時の棚卸・同 spec の実機の確かめ〔タスク 6.3〕で気付いた） | バグ・区分 B | 実機（emo2）で、10 秒ごとのスレッド別 CPU の報告の `threads=` が 15 から 35 へ、トーク 1 つにつき 1 本（`sakura-talk-<番号>`）増えた（起動の挨拶の分も残る＝前からある問題）。報告の数はスレッド名簿（wintf の `thread_registry`）の項目の数で、名簿は終わったスレッドの項目と複製したハンドルを持ち続ける作り＝名簿だけが増えている見立て（未確定）。最初に OS から見たスレッドの数と並べて切り分け、名簿の終わったスレッドの扱い（CPU 時間を役割ごとに畳んで外すなど）か、生きたスレッドが残るならトークのスレッドの後始末を直す | S〜M（5〜10） | v0.2.0・版の中の列の外（`crates/wintf/src/ecs/world/thread_registry.rs`・`crates/areka/src/perf_thread_report.rs`。トークの側を直すときだけ `crates/areka-sakura/src/drive.rs`・`crates/areka-ghost/src/dispatcher.rs`＝`talk-anchor-interrupt-ticks`・`talk-fast-forward` と同じウェーブに置かない）。`perf_thread_report.rs` は `instant-to-interrupt-ticks` と同じウェーブに置かない。実機の確かめが 1 回要る | なし | − |
| `ghost-switch-balloon-fallback`（**10-30 起票**・`startup-install-option` の完了の棚卸・同 spec の実機の試験で見つけた） | バグ・区分 B | 切替先のゴーストにも置き場にもバルーンが無いと、今使っているバルーンで続けずに `ghost_switch_boot_failed stage="balloon"` で切り替えが落ち、既定のゴーストがカタログに無いと `ghost_switch_fatal` で終わる（`emo2_boot/ghost_switch.rs` 828・835 行目・`boot_config.rs` の `resolve_balloon_for_ghost`・`boot_resolve.rs` 319 行目の `NoBalloon`）。SSP は既定のバルーンで続ける見込み（測っていない）＝決め方は要件の段で ukadoc を読み、読めなければ SSP で測る。`default-balloon-selfmade` は置き場にフォルダを置くだけで鎖は変わらない。細部は brief | S（3〜6） | v0.2.0・版の中の列の外（`boot_resolve.rs`・`boot_config.rs`・`emo2_boot/ghost_switch.rs`）。`shell-companion-balloon` と `resolve_balloon_for_ghost` を分け合う・`runtime-module-untangle`・`session-module-gather` と同じウェーブに置かない | なし | − |
| `balloon-flip-side-rules`（**10-30 起票**・`window-work-area-clamp` の完了の棚卸・同 spec の最終検証の範囲外の項目 ②③） | バグ・区分 B | ⑴ キャラ窓が収まっていてバルーンだけが横にはみ出すときの反対の側への並べ替え（`placement/balloon_limit.rs` 157 行目の `place_balloon_in_work_area`）は、そのバルーンのキャラ窓と作業領域だけを見て相方の窓を見ない＝起動の並べ直しが相方を本体の反対の側へ返した後（同 spec の 4.3 ⑴ の形）に本体のバルーンが並べ替わると相方の窓に重なりうる（計算の見込み）。扱い（相方も避ける／押し込みへ倒す／重なってよい）は裁定 ⑵ 右に出すバルーンの初期配置（`placement/resolver.rs` P5）は `balloon.offsetx` を画面の右向きに足す＝ukadoc の「シェルへ向かう向き」と並べ替えの映しに食い違う疑い。SSP は横のずらし 0 しか測っていない＝0 でない検体で SSP を測ってから裁定。細部は brief | S（4〜7） | v0.2.0・版の中の列の外（`placement/balloon_limit.rs`・`placement/resolver.rs`・`placement/follow/window_move.rs` の呼び出し 1 か所）。`placement-crate-split` と同じウェーブに置かない・`balloon-canon-residue` と `resolver.rs` を分け合う | なし | − |
| `window-resetwindowpos-hold`（**10-30 起票**・`window-work-area-clamp` の完了の棚卸・同 spec の最終検証の範囲外の項目 ①） | バグ・区分 B | `\![execute,resetwindowpos]` は、同 spec が入れた位置の指令の預かり（最初の面の表示を待つ・`emo2_boot/frame/move_hold.rs` 187 行目の `release_ready`）の外で、前に預かった移動を捨てて先に当たる＝起動の直後に流すと表示の前の仮の大きさで並べ、同 spec の 3.2 で測った 52px と同じ形の隙間が残りうる（実機では見ていない・要件 6.2 の外）。直し方（並べ直しも全部の窓の最初の表示を待つ・捨てる決まりとの合わせ方）は要件の段で決め、実機で確かめる。細部は brief | S（3〜6） | v0.2.0・版の中の列の外（`emo2_boot/frame/move_hold.rs`・`drain_resnap.rs`）。`emo2_boot` の位置の指令を流す所を触る＝`runtime-module-untangle`・`runtime-crate-split` ほか「本体の crate 分け」の鎖と同じウェーブに置かない | なし | − |
| `balloon-webp-names`（**10-10 起票**・開発者「画像ファイルなどは現代的なフォーマットにする」「online0.png などのアニメは動画 webp などで実現する」・区分 A） | 優先・区分 A | バルーンの面の絵を `.webp` の名前でも拾う（今は `.png` だけ＝`areka-emo-present` の `balloon.rs` の拡張子の定数）。名前から絵のファイルを探す所を 1 か所にまとめ、後から印を読む `balloon-markers` も同じ所を通す。動かない WebP を Windows の拡張に頼らず `image` で読む（依存は足さない）。`.png` と `.webp` の両方が在るときの勝ち負けを決めて固定する。シェルの絵の名前は持たない | S（4〜7） | 列の外（`areka-emo-present` の `balloon.rs`・`areka-emo-atlas` の `decode/`。`balloon-markers`・`balloon-canon-residue` と同じ系列の表を触るので、その 2 本より先に置く。席は次の棚卸で決める） | なし | − |
| `dola-yaml-backend`（**10-15 起票**・`release-cycle` の完了の全体テストの赤・`/kiro-discovery`） | その他・区分 C | `dola` の任意の機能 `yaml` が非推奨の `serde_yaml` を通して `unsafe-libyaml`（勧告 `RUSTSEC-2024-0449`・保守されていない・直した版なし）を引く。安全な Rust の YAML（`serde-saphyr` など）へ替えるか、機能ごと外すかを要件で決め、`deny.toml` の見逃しの指定を消す。areka の配布物には入らない（ワークスペースで有効にするクレートは 0 本） | XS〜S（3〜6） | いつでも（`crates/dola/Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`deny.toml`＝依存の席を使う・版上げと同じウェーブに置かない） | なし | − |
| `dola-interrupt-ticks`（**10-15 起票**・`/kiro-discovery`・開発者「低優先度」・「時刻の土台」の 1 本目） | 据え置き・区分 D | dola の時刻を `f64` の秒から、`QueryUnbiasedInterruptTimePrecise` の 100ns の `u64` を物差しにした型へ替え、「今」を読む関数をその 1 本にする（`dola::runtime::clock::now` の置き換え）。cue の層と storyboard の層の公開の口・直列化の形・使い手の追従（sakura・seriko・emo-text・wintf・ghost・areka＝型の付け替えだけ）・wintf の `FrameTime` と areka の `TalkClock` の時計。外の crate は足さない（`windows` の機能 `Win32_System_WindowsProgramming`）。dola の公開の形が壊れる＝次の版上げの見どころに載せる | L（14〜20） | 据え置き（`crates/dola` の全体と使い手のテスト＝ほかの spec が無い時に一気に通す・`dola-yaml-backend` の後） | なし | ○ |
| `talk-anchor-interrupt-ticks`（**10-15 起票**・`/kiro-discovery`・開発者「低優先度」・「時刻の土台」の 2 本目） | 据え置き・区分 D | ghost の刻み・台詞の刻みの中継・kanade・seriko の時計（`GetTickCount64` のミリ秒）を割り込み時刻へ。台本の起点を配る時に本物の絶対時刻で押し、文字の層と seriko が cue の届いた時刻から起点を見積もらずに読む形へ＝プロセス間の同期の土台。議題＝`OnSecondChange` の OS の連続起動時間（眠りを数えない時計で値が短くなる）・眠りの後の締め切り | M〜L（10〜16） | 据え置き（ghost の `ticker.rs`・`dispatcher.rs`・`sink.rs`・sakura の `drive.rs`・kanade の `msg.rs`・`schedule/steady.rs`・seriko の `actor.rs`・`talk.rs`・`emo2_boot/mod.rs`・`talk_clock.rs`） | `dola-interrupt-ticks`・`seriko-talk-clock-fidelity` | − |
| `instant-to-interrupt-ticks`（**10-15 起票**・`/kiro-discovery`・開発者「低優先度」・「時刻の土台」の 3 本目） | 据え置き・区分 D | 残りの「今」の読み取り（wintf の刻みの締め切り・入力・ドラッグ・ツールチップ、areka の締め切りと計測、`areka-emo-present`・`shiori-host32-host`・`areka-nar`）を割り込み時刻の時計へ替え、clippy の `disallowed_methods` の番人で別の時計（`Instant::now`・`SystemTime::now`・`GetTickCount64`・`QueryPerformanceCounter`）を止める。暦の時刻が要る所だけ名指しで許す | M（8〜12） | 据え置き（`tick_wake.rs`・`tick_bridge.rs`＝`tick-gate-adoption` と同じ時に走らせない） | `dola-interrupt-ticks` | − |
| `talk-script-metadata`（**10-25 起票**・`/kiro-discovery`・開発者「台本ごとに台本が基準とするカレントディレクトリとか送信元が何かとかを特定できる情報があればよい」） | v0.2.0・区分 A | 台本ごとの付帯情報（実行するゴーストと置き場＝ghost/master とゴーストの最上位・送り手のゴーストや道具）を `StartTalk` に持たせ、`begin_talk` で受け手へ渡す。ファイルを指すタグの相対パスを解く関数を 1 つにし、`\![open,file]`（`readme/opener.rs`・今は `GhostSlot` を引く）を乗り換える。`CueSheet` にも持たせるかは要件の段 | S〜M（6〜10） | `talk-anchor-interrupt-ticks` と同じ継ぎ目（台本の起点を配る所）＝別のウェーブ。`areka-talk` の `lib.rs`・kanade の `msg.rs` が `mcp-strict-errors` と重なる | なし | − |
| `multi-ghost-runtime`（**10-15 起票**・`/kiro-discovery`「版の段」・開発者「複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」・予約の「多重ゴースト」の土台） | 優先・区分 A | 2 体以上のゴーストを同時に動かす土台: ゴーストの箱 `GhostSlot` を集まりにし、窓・結線・kanade の知らせに「どのゴーストか」を持たせ、1 体の終了（`OnClose`）とすべて終了（`OnCloseAll`）を分け、起動中の印と記憶を並びにする。`get_active_ghost_list` が全員を返す | L（16〜20・最初のタスクで上限の近いファイルを割る） | v0.1.0・`main.rs` を触る spec（`baseware-root-list`・`user-data-root`・`write-failure-notice`）の後 | `baseware-root-list`・`user-data-root` | ○ |
| `ghost-call`（**10-15 起票**・同・予約の「多重ゴースト」の呼ぶ働き） | 優先・区分 A | メニューの「他のゴーストを呼ぶ」と `\![call,ghost,…]`（`--option=raise-event`）で 2 体目を起こし、`OnGhostCalling`・`OnGhostCalled`・`OnGhostCallComplete`・`OnOtherGhostBooted`・`OnOtherGhostClosed`・`OnOtherGhostChanged` を ukadoc の Reference で送る | M（10〜14） | v0.1.0・`consumer_ledger.rs` の共有の席（`\!` の名前を足す spec と直列） | `multi-ghost-runtime` | − |
| `placement-crate-split`（**10-28 起票**・`/kiro-discovery`・開発者「areka 本体 crate (crates/areka) の肥大を解消するため、`placement` と `emo2_boot` を独立 crate へ分離したい」・区分 A） | 優先・区分 A | 窓の配置（本番 12,932 行・本体への依存 0）を `areka-placement` へ移す。外から使われる `pub(crate)` だけを `pub` に、テストの支えは `test-support` 機能で開ける、`transition_judge*` と境をまたぐテスト 3 本は本体に残す、依存の向きの規約の食い違い 5 件を直す。振る舞いは変えない | 12〜15 | v0.1.0 の後の隙間の先頭・本体の crate 分けの 1 本目（6 本直列・`crates/areka` を触るほかの spec と同じウェーブに置かない） | v0.1.0 の版の門・`areka-test-binary-stability` | − |
| `runtime-module-untangle`（**10-28 起票**・`/kiro-discovery`・開発者「`emo2_boot` / `Emo2Wiring` / `Emo2BootInputs` / `wire_emo2_boot` は特定ゴースト（emo2）に依存せず、実体は汎用の実行基盤」・区分 A） | 優先・区分 A | crate は作らず、本体の中で `emo2_boot` を `runtime` に改名し（型・関数・ログの文言も）、切替・読み直し・実行系の起動を `session/` へ、install・update・readme の cue の受け口を持ち主へ移し、背骨に残る 6 か所の参照を切って、外への依存を 0 にする。emo2 が残るのは既定のゴースト名と検体だけ | 15〜18 | 本体の crate 分けの 2 本目 | `placement-crate-split` | − |
| `runtime-crate-split`（**10-28 起票**・`/kiro-discovery`・区分 A） | 優先・区分 A | 外への依存が 0 になった `runtime`（本番 約 12,000 行）を `areka-runtime` へ移す。`target_map`・テスト専用の `transition_judge*`・placement と runtime をまたぐテスト・`collision-probe` も移す | 10〜12 | 本体の crate 分けの 3 本目 | `runtime-module-untangle` | − |
| `input-crate-split`（**10-28 起票**・`/kiro-discovery`・開発者「この際、areka crateのコードベースが大きすぎる問題をサルベージすべきかもしれない」・区分 A） | 優先・区分 A | 入力の出来事（本番 3,932 行）を `areka-input` へ移す。`shortcut.rs` は menu へ、`file_drop.rs` は本体に残し、Ctrl+左ダブルクリックの `quit_app` は終わりの知らせで向きを逆にする | 12〜15 | 本体の crate 分けの 4 本目（session が input を呼ぶので session より先） | `runtime-crate-split` | − |
| `session-module-gather`（**10-28 起票**・`/kiro-discovery`・区分 A） | 優先・区分 A | crate は作らず、起動・終了・切替の層（`ghost_session*`・`boot_*`・`app_exit`・`session_end` ほか）と main.rs のシーム約 367 行・roots の芯・readme を本体の `session/` に集め、系の登録を main へ移す | 8〜10 | 本体の crate 分けの 5 本目 | `input-crate-split` | − |
| `session-crate-split`（**10-28 起票**・`/kiro-discovery`・区分 A） | 優先・区分 A | 集めた層（本番 約 8,600 行）が本体を呼ぶ約 30 件の向きを 6 つの仕組みで逆にし、`areka-session` へ移す。本体は本番 約 15,000 行の「橋渡し」になる | 12〜16 | 本体の crate 分けの 6 本目（`multi-ghost-runtime` とファイルがほぼ全部重なる＝同じウェーブに置かない） | `session-module-gather` | − |
| `audit-emo2-boot-frame`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | areka 本体の emo2_boot のうち毎フレームの組み立てに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`instant-to-interrupt-ticks` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | `runtime-crate-split`（2026-10-28 の本体の crate 分け） | − |
| `audit-emo2-boot-switch`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | areka 本体の emo2_boot のうち切り替え・バルーンの見え方・ホバー・移動の cueに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`instant-to-interrupt-ticks` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | ✅ `mcp-reload`（10-22 着地＝済み）・`runtime-crate-split`（2026-10-28 の本体の crate 分け） | − |
| `audit-emo2-boot-spine`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | areka 本体の emo2_boot の残り（背骨・消費者の台帳・適合の台本・資産・トークの時計ほか）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`compose-f16-precision`・`dola-interrupt-ticks`・`talk-anchor-interrupt-ticks`・`seriko-talk-clock-fidelity` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | ✅ `mcp-reload`（10-22 着地＝済み）・`runtime-crate-split`（2026-10-28 の本体の crate 分け） | − |
| `audit-placement-follow`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | areka 本体の配置のうち追従と移り変わりの判定に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・v0.2.0 と触るファイルが重ならない＝いつでも取ってよい（2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | `placement-crate-split`（2026-10-28 の本体の crate 分け） | − |
| `audit-placement-core`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | areka 本体の配置の残り（窓の出し方・設定・保存・重なりの順・DPI ほか）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`sylphya-sqlite-kvs` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | `placement-crate-split`（2026-10-28 の本体の crate 分け） | − |
| `audit-areka-install-update`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | areka 本体のインストール・ネットワーク更新・MCP の受け口・根の一覧・説明書に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`compose-f16-precision`・`instant-to-interrupt-ticks`・`mcp-set-property` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | ✅ `mcp-reload`（10-22 着地＝済み）・`session-crate-split`（2026-10-28 の本体の crate 分け） | − |
| `audit-areka-input-menu`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | areka 本体の入力の出来事と右クリックメニューに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`collisionex-regions`・`instant-to-interrupt-ticks` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | `input-crate-split`（2026-10-28 の本体の crate 分け） | − |
| `audit-areka-app`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | areka 本体の残り（起動・起動オプション・ゴーストの会話の場・SHIORI の結線・終了ほか）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`instant-to-interrupt-ticks`・`tick-gate-adoption`・`talk-thread-count-growth`・`sylphya-sqlite-kvs` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | `session-crate-split`（2026-10-28 の本体の crate 分け） | − |
| `audit-wintf-window`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | wintf の窓と窓の手続きに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`instant-to-interrupt-ticks` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-wintf-layout-widget`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | wintf のレイアウトと部品に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`wintf-d2d-composite-rules`・`hit-id-pass`・`compose-f16-precision` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-wintf-input`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | wintf のドラッグ・ポインタ・クリック透過・ツールチップに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`instant-to-interrupt-ticks` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-wintf-graphics-world`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | wintf の描画・World・見た目の木・共通の型に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`wintf-visual-tree`・`wintf-d2d-composite-rules`・`wintf-visual-damage-redraw`・`compose-f16-precision`・`dola-interrupt-ticks`・`instant-to-interrupt-ticks`・`tick-gate-adoption`・`talk-thread-count-growth` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-wintf-runtime`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | wintf の ECS の外（刻みの土台・COM・窓の型・API・数の型）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`wintf-visual-tree`・`wintf-d2d-composite-rules`・`wintf-visual-damage-redraw`・`compose-f16-precision`・`instant-to-interrupt-ticks` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-emo-text-layout-draw`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 文字の層のレイアウトと描画に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`compose-f16-precision` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-emo-text-state-actor`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 文字の層の状態・アクター・選択肢・窓に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`compose-f16-precision`・`seriko-talk-clock-fidelity` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-kanade`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | kanade（ゴーストの段取りの役者）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`talk-anchor-interrupt-ticks` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | ✅ `mcp-reload`（10-22 着地＝済み） | − |
| `audit-dola`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | dola（絶対時刻の台本の再生）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`dola-interrupt-ticks`・`dola-yaml-backend` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-seriko`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | SERIKO（シェルのアニメーション）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`draw-methods-canon`・`draw-methods-blend`・`talk-anchor-interrupt-ticks`・`seriko-talk-clock-fidelity`・`tick-gate-adoption` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-emo-compose`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 面の合成に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`draw-methods-canon`・`draw-methods-blend`・`collisionex-regions`・`hit-id-pass`・`compose-f16-precision`・`element-clipping-option` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-emo-present`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 面とバルーンの提示に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`draw-methods-canon`・`collisionex-regions`・`hit-id-pass`・`compose-f16-precision`・`instant-to-interrupt-ticks`・`tick-gate-adoption` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-emo-atlas`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 絵の焼き込みとアトラスに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`compose-f16-precision`・`element-clipping-option` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-ghost-catalog`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | ゴーストの読み込み・.nar のインストール・利用者のデータの置き場に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`talk-anchor-interrupt-ticks`・`instant-to-interrupt-ticks`・`talk-thread-count-growth`・`sylphya-sqlite-kvs` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | ✅ `mcp-reload`（10-22 着地＝済み） | − |
| `audit-parsers-sakura`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 設定ファイルの読み取りとさくらスクリプトに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`draw-methods-canon`・`draw-methods-blend`・`collisionex-regions`・`hit-id-pass`・`talk-anchor-interrupt-ticks`・`element-clipping-option`・`talk-thread-count-growth` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-sylphya-actor`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | プロパティの仕組み・役者の土台・トークに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`sylphya-sqlite-kvs` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-shiori-host32`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 32bit SHIORI の中継と SHIORI の ABI・試験用の DLLに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`instant-to-interrupt-ticks` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-mcp-update-crates`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | MCP の受け口と中継・ネットワーク更新のクレートに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`mcp-set-property` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-impl-watch`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 机の貸し出しのアプリ（impl-watch）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・v0.2.0 と触るファイルが重ならない＝いつでも取ってよい（2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-ukadoc-survey`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | ukadoc の網羅の調べの道具に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・v0.2.0 と触るファイルが重ならない＝いつでも取ってよい（2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-test-kits`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 試験の道具のクレート（実走・記録の捕まえ・検体・一時フォルダ）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・v0.2.0 と触るファイルが重ならない＝いつでも取ってよい（2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-tools-perf-judge`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 性能改善ループの判定と台帳（Python）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・v0.2.0 と触るファイルが重ならない＝いつでも取ってよい（2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-tools-perf-run`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 性能改善ループの走らせ方（PowerShell）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`tick-gate-adoption` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-tools-scripts`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | `tools/` 直下の台本（全体テスト・配布物・リリースノート・検体の取り寄せほか）に `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・**v0.2.0 の作り直しの前に済ませる**＝`talk-thread-count-growth` より先に取る（触るファイルが重なる・2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `audit-pilot`（**10-19 起票**・`/kiro-discovery`・開発者「とにかく指摘事項は全部直す方針です」・区分 A） | 優先・区分 A | 先進坑の実験コードに `/ponytail:ponytail-audit` をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。範囲のファイルは brief に書いた＝ほかの見直しの spec と重ならない | 指摘の数で決まる（20 を越えたら要件の討議で同じ範囲の 2 本目へ） | v0.1.0 の後の隙間（版に縛らない・優先度が高い）・v0.2.0 と触るファイルが重ならない＝いつでも取ってよい（2026-10-22 brief の突き合わせ）・ほかの見直しの spec と同じウェーブに並べてよい | なし | − |
| `shell-scale-linkage`（**10-23 起票**・`pasta-in-windows-verification` の申し送り・v0.1.0） | その他・区分 A | ぱすたが使う倍率の連動: `\![get,property,イベント名,…]` の受け手（`property-query-channels` から移した）・`currentghost.scope(ID).scaling`・`currentghost.balloon.scope(ID).scaling`・`currentghost.scope(ID).currentmonitor.dpi`・`OnShellScaling`／`OnBalloonScaling`・descript の `balloon.syncscale`（`shell-balloon-scale-setting` から移した） | M（11〜16〔棚卸㉜〕） | kanade の列・`consumer_ledger.rs`・`frame/dpi.rs` | ✅ `mcp-scope-rect-property`（動く値の口・10-23 着地）・✅ `script-security-level`（`SenderType`） | − |
| `balloon-append-tags`（**10-31 起票**・`balloon-scroll-arrows` の完了の棚卸・同 spec のタスク 8.2 の実機の走り） | その他・区分 C | `\C`（前のトークのバルーンを消さずに書き足す）と `\*`（選択肢の待ち時間を止める）。今はどちらも読み手で素通しになり、`\C` は台本の先頭の `ClearAll` でバルーンが空になってから出る（`check_script` は `unknown_tag`）。網羅台帳の群「バルーンの追記と選択肢のタイムアウト抑止」の 2 行・持ち主なし。版の置き場は次の棚卸で。細部は brief | 見積もりは要件の段（規模は spec 1 本に相当） | コンパイルの列（読み手の `sakura/decode.rs` を `script-small-tags` と分け合う・組み立ては `areka-sakura` の `compile.rs`） | なし | − |
| `dressup-change-events`（**10-31 起票**・`script-small-tags` の要件の討議の議題 3） | その他・区分 C | 着せ替えの後の `OnDressupChanged`・`OnNotifyDressupInfo` を送る（`\![bind,…]` の後と起動のとき・メニューができたらメニューの後も）。`\![bind-noevent,…]` とゴーストの読み直しの持ち越し（`shell_carry.rs`）からは送らない。網羅台帳 `shiori.toml` の 2 行・持ち主なし。版の置き場は次の棚卸で。細部は brief | 見積もりは要件の段（規模は spec 1 本に相当） | seriko の `actor.rs`（`script-small-tags` と同じウェーブに置かない）・kanade の `schedule/events.rs` | ✅ `script-small-tags`（`bind-noevent` の受け手と 2 つの名前の区別・10-31 着地） | − |
| `balloon-communicatebox`（**10-23 起票**・`pasta-in-windows-verification` の申し送り・v0.1.0） | 優先・区分 A | 話しかける入力欄 CommunicateBox（バルーンの `communicatebox.*`・`\![open,communicatebox]`・`\![close,communicatebox]`・`OnCommunicate`・`OnCommunicateInputCancel`）。areka で初めての文字の入力の部品（IME）＝`inputbox-user-input` が後で使う | M〜L（14〜19・棚卸㉙・20 を超えたら入力の部品と開け閉め〔先〕／見た目の鍵〔後〕に切る） | `consumer_ledger.rs`・kanade の許可の表・バルーンの読み手 | なし（実機の確かめは、ぱすたの書庫の直し〔ghost_dev の `c32229e`・未リリース〕の後。それまでは `doc/pasta-real-run.md` の入れ方で） | ○ |
| `doc-tree-tidy`（**10-28 起票**・`/kiro-discovery`・開発者「doc と docs があり、また、doc も結構下記散らかされていて困ってる」「このspecだけは0.1.0の直後、doraとかの基盤整備に置きます」） | その他・区分 A | `docs/`（`click_through.md` 1 つ）を `doc/` へまとめる（wintf のクレートの中へ移す手もある）・ファイル名の書き方をそろえる・`doc/README.md` の案内の表と実物の突き合わせを決定論の試験 1 本にする・動かしたパスを名指すコード・テスト・スキル・steering・未完了の spec を grep して直す（完了 spec の中の古いパスは要件の段） | S〜M（6〜10） | v0.2.0 の早いウェーブに 1 本だけで（パスを動かす＝ほかの spec の設計が文書のパスを名指す前に） | なし | − |
| `sound-canon-tags`（**10-28 起票**・`/kiro-discovery`・N3「音の層」の前半・議題 8） | その他・区分 C | 正典の音のタグ（`\_v`・`\_V`・`\8`・`\![sound,play／loop／stop／pause／resume／wait／load／option…]`）と音のプロパティ 18 葉の値を、OS の Media Foundation で鳴らす（効果音の重ねに XAudio2 を併せるかは設計）。相対パスは `talk-script-metadata` の関数で解く。音楽 CD・動画・ノベルの音の拡張は外す | M〜L（14〜20・20 を超えたらタグの受け口とプロパティで切る） | ノベルゲーム基盤の先頭 | `talk-script-metadata`・v0.2.0 の時刻の土台 | − |
| `ghost-material-metadata`（**10-28 起票**・`/kiro-discovery`・M1「素材 ID とメタデータ契約」・議題 5〜7・11） | その他・区分 A | 素材 ID（`種別:id`・同梱は親を前置・元は descript の `id`、空ならフォルダ名）と探す順（叩き台＝`/play/{play}/shell/{shell}/` → `/free-shell/{shell}/` → `/ghost/{shell}/shell/master/`）・正典のキー（`craftmanurl` など）を使い、足りない分（ライセンス・支援先・role・借りないで・演劇である・席の要件）だけ areka の独自キーで descript に足す・借用はオプトアウト・目録の素性の拡張（完了 `baseware-root-layout` の要件 2.9 を改める）・単体のフリーシェルの置き場。要件の段の最初に pasta のセッションへ相談して P2（`pasta_check validate`）を決める | M〜L（12〜20） | ノベルゲーム基盤 | なし（`audit-ghost-catalog` が目録を先に触るならその後） | − |
| `shell-multi-compose`（**10-28 起票**・`/kiro-discovery`・N1「ノベルゲーム向けの画面構成」・議題 3・4） | その他・区分 A | 1 つの舞台の窓に背景・役者・トークの場所のシェルを何枚も重ねる・3 体以上の役者に任意のゴーストの任意のシェルを割り当てる・普通の窓と全画面・レターボックスとクロップ・場面転換とサーフェスの替わり方・揺れ・フラッシュ・色調・性能の目標。命令の名前は pasta の依頼書が決める | L 超（要件で切る＝舞台の窓と席／窓の形／転換） | ノベルゲーム基盤（v0.2.0 の新しい描画の層の上） | `wintf-visual-damage-redraw`・`ghost-material-metadata`・`element-animation-easing` | ○ |
| `areka-usage-stats`（**10-28 起票**・`/kiro-discovery`・U4「areka の利用統計連携」・議題 9） | その他・区分 A | 同意したとき（既定は無効）だけ、「使った」素材の ID を 1 日 1 回まとめて外の発信器（U2 の CLI）へ渡す。同意の正は発信器の設定・最初の同意は既定のゴーストの台詞と選択肢で尋ねる候補（メッセージボックスを使わない・設定画面は windows-reactor 待ち）・発信器が無い・失敗でも黙って飛ばす・ローカルパスや個人の名前を出さない・「匿名」と書かない。演劇の信号は演劇のパッケージの後 | M（10〜14） | 利用統計の段（ノベルゲーム基盤と並走してよい） | `ghost-material-metadata`・外の U2（発信器） | − |

## 覚え書き（brief なし・引受先が消えたまま忘れないための一覧）

> brief を書くほど固まっていない宿題。**着手の判断は棚卸で行う**（格上げするときは `/kiro-discovery`）。登記時の全文（数え方つき）は history「2026-10-02 棚卸⑳退避」、棚卸㉒の前の全文は history「2026-10-05 棚卸㉒退避」。棚卸㉓で brief にした 5 項目（6 本）の全文は history「2026-10-10 棚卸㉓退避」。 棚卸㉙で brief にした 3 項目（`install.txt` の `type,package`・SSTP の受信・`\_b[…,inline]`）の全文は history「2026-10-23 棚卸㉙退避」。

**pasta のノベルの依頼書を待つもの**（2026-10-28 `/kiro-discovery`）: ノベルの進行と入力の残り・バックログと巻き戻し・セーブとロード・システム画面の部品・ノベルの音の拡張・演劇のパッケージ・制作と配布の導線・pasta の P2。一覧と申し送りは「テーマ別の決めごと」の「利用統計とノベルゲーム基盤」の「受け口」、原文は `doc/ideas/novel-platform.md`。

**製品の穴**

- **`present-write-coherence` の未達 40 件**（性能・L）。完了仕様が自ら「引受先なし・新規仕様の起票が必要」と書いた残量＝`visualize_to_write_us` が上限 16,667µs の 12.6〜18.4 倍・32 窓中 0 窓が上限以下。開発者裁定で「未達のまま GO」済み。着手前に測り直す。`tools/perf` の 3 経路・`tick-gate-adoption` の測り方と同じく「性能改善ループが動いてから」の宿題。
- **`recommended.balloon`／`recommended.balloon.path`**（10-03 登記・S）。ゴーストの descript.txt の推奨バルーンで、これ以外へ切り替えると「強い内容の警告」を出す（ukadoc）。メッセージボックスを出さない方針（失敗は既定ゴーストの台詞で伝える）とどう折り合うかを決めてから起票する。`ghost-standard-balloon` が範囲外と明記。網羅台帳は `absent`・担当なし。
- **壊れたゴーストを表示し続ける形**（互換・M・優先度は低い）。SSP は切替先の SHIORI が死んでいても切替を成功扱いにして表示し続ける。areka は「既定ゴーストへ戻して `OnBoot` の Ref6＝`halt`」を採った（09-26 裁定・生きている決まり 7 と逆向き）。あるべき姿としては残る。開発者が閉じてもよい。
- **`sakura-time-directives` から戻した C・D 群**（10-04 登記）。時間の指令のうち、消費する者が現れるまで置くもの（同 brief の「2026-10-04 棚卸㉑で切った後の範囲」）。消費する者が現れたら、その spec の要件の段で引き取る。**2026-10-12**: D 群（時間つきの `\![move]`・`\![set,scaling]`・`\![set,alpha]`）を消費する者の候補として `element-animation-easing`（覚書）を起票した＝引き取るか、先に別の spec を立てるかは同 brief の議題 1。**2026-10-17**: `move-canon` の起票で開発者「アニメーションに絡むのは、dola の改造後」＝時間つきの `\![move]`（`--time` の補間と台本の時間を進めること）・`\![moveasync]`・`\![moveasync,cancel]` は v0.2.0 の dola の改造の後に起票する（✅ `move-canon`〔2026-10-18 完了〕は瞬間の移動だけ・`time>0` は最終位置へすぐ置いて記録する）。
- **`sample-ghost-kit` の `fold-samples` の `strip_folder` が取り出し元の先頭の 1 段しか比べない**（XS・`completed/areka-P0-install-companion-reading/tasks.md` の Implementation Notes）。設計で広げないと決めたもの。階層付きの取り出し元を持つ検体ができたら直す。
- **`\![enter,passivemode]`・`\![leave,passivemode]` の網羅台帳の持ち主**（2026-10-31 棚卸㉛・XS）。`doc/ukadoc-coverage/ledger/sakura-script.toml` の 1075〜1084 行目の持ち主が完了 spec の `areka-P0-status-execution-states` のままで、`roadmap-draft.md` の 1147 行目の次の持ち主の候補 `areka-P0-passive-mode-states` は brief を持たず、廃止した接頭辞の名前。台帳の持ち主は spec の要件の段で動かす決まりなので、ここに置き、引受先の spec を起こすか `coverage-roadmap-refresh` が数え直すときに直す。
- **どっとさくらの実走で出た `kanade OnTranslate 400` の `WARN` 9 行**（2026-10-31 棚卸㉛・原因未調査）。✅ `seriko-pattern-calls` のタスク 8.3 の実走の記録（その spec の作業場の `target\sp83`・もう無い）に出た。華和梨の `OnTranslate` が 400 を返した行と見られる。次にどっとさくらを実走する spec が記録を残し、再現したら出力を添えて起票する。

**潜在の性質（実害は観測されていない）**

**道具と試験**

- **`tools/perf` の実走していない 3 経路**（S）。`invoke-perf-run.ps1` へ `-GhostRoot`／`-BalloonRoot` を渡す実走・`invoke-followup-checks.ps1` の単独起動・`check-quiet.ps1`。性能改善ループを次に回す前に 3 経路を通す（2026-10-16 に `perf-tools-console-encoding` が `tools/perf` の 9 ファイルを書き替えた〔子の出力を UTF-8 で読む形〕。3 経路は書き替えの後もまだ実走していない＝次に回す前に通す重みが増した・棚卸㉖）。
- **中継 `areka-mcp-bridge.exe` が arm64 の機械で起きるかの確かめ**（2026-10-18 棚卸㉗で「次のリリースへの申し送り」から移した・持ち主なし・開発者の机は x64）。`/release` の走りで arm64 の組み立てが通ることは記録される（`v0.0.3` の見どころにも事実として書いた）。arm64 の機械が手に入るまで未了のまま。
- **`image-webp` の取り込みを外す作業**（XS・`animated-image-decode` が持ち主をここに置いた）。`image-webp` は上流の GitHub の固定コミット（0.2.5）から取り込んでいる（`tech.md` の登記）。**棚卸のたびに crates.io で `image-webp` の最新版を引き**、0.2.5 以上が出ていたら `tech.md` の取り外し条件どおりに外す＝根の `Cargo.toml` の `[patch.crates-io]` の行・`deny.toml` の `allow-git` の行・`crates/areka-emo-atlas/src/webp_pin_tests.rs` と同じクレートの `lib.rs` の `mod webp_pin_tests;` の宣言を外し、`cargo update -p image-webp` で公開版へ戻して検体のテストを通す。**2026-10-22 棚卸㉘の確認: crates.io の最新は 0.2.4（2025-08-27）のまま＝まだ外せない**。 **2026-10-23 棚卸㉙の確認**: crates.io の最新は 0.2.4（2025-08-27）のまま＝まだ外せない。
- **一度だけ落ちた試験と、一度だけ出た出力**（原因未調査・再発したら出力を添えて起票）: `real-run-kit --test fake_mcp_test` の `child_dying_in_call_is_child_exited`（10-31・`emo-atlas-palette-alpha` の完了の全体テスト＝偽の子が 53500 番を束ねられず `ListenFailed`〔os error 10048＝ほかのものがそのポートを使っていた〕で、期待の `ChildExited` にならなかった。本 spec はこの crate に触っていない。赤の対象だけの回し直しで 8 本とも緑）・`wintf --test graphics` の `STATUS_ACCESS_VIOLATION`（10-04・他のセッションの cargo と並走した `cargo test --workspace -j 4` で 60 秒を超えて止まった後に落ちた。単独では 97 本すべて緑。完了 `wintf-gpu-test-crash` と同じ形・`areka-test-binary-stability` の件 1（旧 `areka-test-threads-av`）は範囲外と明記）。`areka-mcp` の `server::server_gate_help_tests::bad_origin_is_403_before_mcp` が全体テストの負荷の下で 1 回赤（10-04・os error 10053・単独では緑）。`cargo test -p areka-emo-present` の出力に `__rust_alloc_error_handler` のバックトレース行が 1 回だけ混じった（10-05・終了コード 0）。wintf のツールチップのテスト（マウスを動かす検査）で、テストの実行ファイルが 1 回だけ `0xc0000409` で落ちた（10-08・人の手のマウスが混ざった直後・再現せず。`completed/areka-P0-wintf-tooltip/tasks.md` の Implementation Notes。また出たら `--nocapture` の出力を添える）。**棚卸㉒で外したもの**: `fallback_tests` の 2 本・`install::desk::overwrite_tests` の 2 本・`emo2_boot::spine` の 3 本・`sample-ghost-kit` の展開テストの os error 5（`target\nar-samples\work` の退避と同じ族）は `ghost-session-test-load-flake` が引き取った。`tools/test-all.ps1` の x64 の段の終了コード -1 は C: の空きが 35MB だったときの環境の問題として閉じた。
- **テストの `BoxedBackend` の写しが 3 つ**（XS・棚卸㉙・完了 `script-security-level` の Implementation Notes）。`crates/areka-ghost/tests/ghost/{inproc_e2e_test,snapshot_capture_test}.rs` と `tests/real_pasta_online.rs` に同じ形の入れ物がある。振る舞いは変わらないテストの手入れ＝次に areka-ghost のテストを触る spec（見直しの `audit-ghost-catalog` ほか）が 1 か所へまとめる。
- **`--offline-dump` の端**（XS〜S・棚卸㉙・完了 `offline-surface-dump` の Implementation Notes 7.2・7.6）。`!surface4` の形を読まない（SSP での振る舞いは未測定）・`--dump-output-prefix` が絶対パスや `..` を止めない。どちらも要件の外として置いたもの＝開発用の道具なので、困った人が出たら `offline-surface-dump` の後続として起票する。

## 直接修正候補（spec なし）

> **2026-10-31 棚卸㉜で直した**（どれも動きを変えない）: ⑴ この roadmap の直列の列の数（`consumer_ledger.rs` は 529 行・40 件 → 547 行・42 件、手書きの数 38 → 42、鎖の残りは `shell-scale-linkage`〔→43〕→ `balloon-communicatebox`〔→45〕）⑵ 同じ列の「`shiori-abi` は `shiori4-api` と分け合う」を「`shiori4-api` が消すので取らない」に改めた（`clippy-199-lints` の brief の申し送りと同じ）⑶ `focus.md` の数の行と `product.md` の C14 の行。
>
> **直さずに各 spec へ渡したもの**: ⑴ 測り直した 14 本の brief の古い記述（数・行の番号・着地した相手の名前）＝各 brief の「2026-10-31 棚卸㉜の再測定」の節に書いた（本文は spec の要件の段で直す）⑵ `shell-scale-linkage` の brief の進め方の「`script-security-level` の運び手に property を足す」は古い（`areka-talk/src/origin.rs:84` の `SenderWord::Property` はもう在り、使い手が無い・本当の穴は `RequestOrigin::of_event` が自由な名前のイベントを受けないこと）＝同 spec ⑶ `balloon-communicatebox` の brief の「529・手書きの数 40→42」は `shell-scale-linkage` の後なら 43→45＝同 spec ⑷ `popup-menu-extra-resources` の 9 つの名前に `char*.recommendbuttoncaption`・`char*.recommendsites`・`char*.recommendsites.caption`（`shiori.toml` の持ち主なし）が抜けている＝同 spec の ⑵。棚卸㉛から続くもの＝history「2026-10-31 棚卸㉜退避」の旧「直接修正候補」。
>
> **2026-10-31 棚卸㉛で直したもの・渡したもの**の全文は history「2026-10-31 棚卸㉜退避」の旧「直接修正候補」節。それより前は、そこから辿る。

残っている候補: 1 件。

- **`tools/mouse-input.ps1` の座標が負の原点の仮想画面で 1 画素ずれる**（2026-11-01 `popup-menu-extra-resources` の完了時の棚卸で登記・spec を立てない直しの候補）: 主画面 2880×1800・左に負の座標の 2 枚目（3413×2133）がある機械で、送ったカーソルが毎回狙いより 1 画素小さく着き（狙い 2640,1460 → 2639,1459）、151 行の自分の確かめが「input refused」として止まる。134〜136 行の正規化の丸め（`ceil(p*65536/size)`）が、負の原点の仮想画面での Windows の戻し方と合っていない見込み。直し方は 2 通りある（丸めを直す／`SetCursorPos` で置いてから押しだけを `SendInput` で送る＝同 spec 4.2 の回避）ので、直す回で決める。確かめに画面の操作（排他テストの机）が要るため、その場では直さなかった。

取り下げた 1 件を再登記しないこと——判定器（`crates/areka/src/placement/transition_judge_verdict.rs`）の窓ごとの書込上限が見送り窓を除いていないのは**意図どおり**（`completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` §13.1・2026-09-24 に開発者が再確認）。

## 生きている決まり（棚卸⑭〜⑲の裁定のうち今も効くもの・全文は history）

1. **着手の優先度は 4 区分＝A 開発者が起票を頼んだもの ＞ B バグ ＞ C ukadoc の拾い残し・バグでない持ち越し ＞ D 夢・据え置き・保留**（2026-10-07 開発者・10-05 の 3 段「① バグ ② リリース ③ MCP・バルーン・アニメーション」を改めた）。ウェーブは、依存の木（働きの依存＋同じソースファイルを触る spec＋直列の列の順）の先頭だけを「ぶら下がる未完了の spec の数 → 区分 → 起票の古さ」で並べ、上から互いに触るファイルが重ならない最大 8 本を取る（手順の正本は `/kiro-next` のスキル）。据え置き・保留・取り下げ予定は木に入れない。**測ることが本体の spec（遅れ・性能・負荷の再現）はウェーブに並べない**。バグかどうかは実測で分ける（利用者から見える実害／潜在／テストの穴／正典の追加）。
2. **規模の上限は 1 spec 20 タスク**。超えるものは要件の段で切る。**一度切り出した spec を、上限を少しまたぐ理由でさらに削らない**（09-28）。
3. **並走の物差し＝触るソースファイルの重なり 0**（09-20）。**文書だけの段は置かない**（09-27・要件は前の spec の着地で古くなる）。
4. **エンジンを切り出した spec は網羅台帳に触らない**。台帳の更新は結線の側がまとめて行う（09-20）。
5. **`zorder-chain-residue` の A 群は先回りしない**（main で赤が出たら、その時点で A 群だけを単独で挟む）。
6. **メニューは Win32 標準**・オーナードローと着せ替えメニューは予約のまま（2026-10-18 棚卸㉗: ぱすたの `menu.*` と着せ替えメニューのために `popup-menu-shell-look` を起票した＝この決まりを解くかは開発者の裁定待ち・決まるまでウェーブに並べない）。**ゴースト切替はプロセス内**・アプリの寿命は窓の数から切り離す。**HTTP は WinHTTP・MD5 は OS の CNG**。**投げ込みは `WM_DROPFILES`**。**消滅（`\![vanishbymyself]`）は要望が出たら S で切る**。
7. **失敗は既定ゴースト（emo2）の台詞で伝える**（`OnBoot` の Ref6＝`halt`・Ref7＝落ちたゴーストの名前）。メッセージボックスは出さない（例外: インストールの警告はダイアログ可＝2026-10-17 開発者「インストール関係はシステム挙動が絡むから、警告があるならダイアログを出してよい」・`emily-ghost-verification` の要件 1.12）。**インストールと切り替えは別のイベント**（入れた後の切替を areka は主導しない）。
8. **要件定義・設計のサブエージェントは起動中のモデルを継承し、上位へ上げない**（09-30）。Fable 列は「Fable で起動したセッションを勧める」だけ。
9. **並走できる spec の紹介は「完全並走できるものだけ」・Fable 推奨を先頭に、名前＋一言＋コマンドの短い形**（09-26）。
10. **話の最中（選択待ちを含む）に届いたマウス系の返事は今の話を置き換える＝正典どおり**（2026-10-03・`emo-text-file-split` の実機で「メニューがなでなでの返事に置き換わる」を観測して確かめた。正本は完了 `input-events` の DD-IE-1／DD-IE-2）。止めるのはゴーストの側（`Status` の `talking`／`choosing` を見て黙る）か台本の `\t`。areka の穴は `\t` を読まないことだけ＝`sakura-time-critical`。

## 棚卸㉜の裁定（2026-10-31・main `d374f901`・開発者指示＝`/kiro-next` の定めどおり）

> 棚卸㉛（10-31・main `4068422f`）の後、C14 の 7 本が同じ日のうちに全部着地した。あいだに完了の棚卸で `alert-dialog-per-monitor-dpi`（B）・`emo-bake-log-overlap`（C）・`popup-menu-extra-resources`（C）を版に縛らない列に起票し、`script-small-tags` の要件の討議で `dressup-change-events`（版の置き場は未定）を起票した。

1. **測り直した範囲**: 速い道（C14 が全部着地・見込みの C15〜C16 があった）。見込みの C15 の `shell-scale-linkage` と置き場の決まっていない `dressup-change-events`、席が 7 つ空いたので割り込みの候補 12 本（計 14 本・3 サブエージェント・Opus）。14 本とも「2026-10-31 棚卸㉜の再測定」の節を書き、古い再測定の節を消した。測らなかった今の版の spec＝`balloon-communicatebox`（C16 の見込み・① を待つ）・`areka-test-binary-stability`（測ることが本体）・`popup-menu-shell-look`（裁定待ち）・`winget-release-automation`（PR 450070 は 10-31 も開いたまま・最後の動きは 10-11・`gh` で確かめた）。着地した 7 本の残り物は、どれも完了の棚卸で始末済み（揺れるテストは `areka-test-binary-stability` の件 3〜5 に在る）＝持ち主の無いものは 0。
2. **前提の変化**: `consumer_ledger.rs` は 547 行・42 件の登記（✅ `script-small-tags` が `bind-noevent` と `*` を足した・手書きの数も 42）。鎖の残りは `shell-scale-linkage`（→43）→ `balloon-communicatebox`（→45）。`shell-scale-linkage` は 10〜14 → 11〜16（kanade の自由な名前のイベントで値を返す道に 1〜2）・先に要るものは全部着地した。
3. **モデル**: 14 本とも今の印のまま。`talk-fast-forward` は Fable のまま（トークの途中の区切りの後の cue の時刻の付け直しと、UI からゴーストのスレッドへ渡すクリックとダブルクリックの判定の順＝前例の無い時刻と順の約束）。
4. **分割は無し**。`talk-fast-forward` は 19〜24 に増えたが、棚卸㉓で決めた「要件の段で 20 を超えたら前半〔早送り＋件 A・9〜11〕と後半 `click-wait-tag`（仮）〔`\x`・`\x[noclear]`・`clickwaitmarker.*`・11〜13〕に切る」のまま（先頭でなく、このウェーブに入らない）。`popup-menu-extra-resources` の ⑴／⑵、`alert-dialog-per-monitor-dpi` の件 1／件 2 も、要件の段で切るかを決める。
5. **起票は無し**（覚え書きから格上げしたものも無い）。置き場の決め: `dressup-change-events`＝版に縛らない列（ぱすたの辞書に `OnDressupChanged`・`OnNotifyDressupInfo` の受け手が無く、SSP の記録でも 204 を返す＝v0.1.0 のゴールに当たらない・区分 C・6〜10・`consumer_ledger.rs` に行を足さない）。列の数は 33 → 34、brief を持つ 163 本と台帳が一致した。
6. **ウェーブ C15 を 6 本で組んだ**（v0.1.0 の 1 本＋割り込み 5 本・Opus 6・約 30〜47 タスク）: ① `shell-scale-linkage` ／割り込み ② `emo-text-line-gap`（B） ③ `alert-dialog-per-monitor-dpi`（B・件 1 だけ） ④ `test-roots-under-target`（入口の差し替えだけ） ⑤ `emo-bake-log-overlap`（案 1・`placement/mod.rs` に触らない） ⑥ `popup-menu-extra-resources`（⑴ だけ）。v0.1.0 の先頭は ① だけ（`balloon-communicatebox` は登記の鎖で ① を待つ）。割り込みは、上限がこのウェーブの一番大きい v0.1.0 の spec の下限（① の 11）以下で、① と触るファイルが 0 のものだけ。③⑤⑥ は範囲を絞る約束で ① との重なりを消した（棚卸㉛の `popup-menu-residue` と同じ形）。7・8 席目は空けた。
7. **入れなかった先頭**: `background-result-tick-wake`（① の受け手が `tick_gate_config_producers_tests.rs` に行を足すかが設計まで分からない＝迷ったので空けた）・`clippy-199-lints`（全体は ① の `emo2_boot/`・`placement/` に重なる・段 1 は切り出すかを開発者が決めるまで）・`dressup-change-events`（kanade の `schedule/events.rs` を ① と分け合う）・`config-parse-diagnostics`・`dev-folder-alias`（どちらも上限 14）・`ukadoc-catalog-refresh`（12〜16・台帳の鍵を付け直すので「別々の行」の許しが効かない）。v0.1.0 の後の隙間（見直しの 33 本・crate 分けの 6 本）は版の門の後。
8. **v0.1.0 の着地まで約 2 ウェーブ（C15〜C16）**、その後にウェーブの外の 3 本と `doc/pasta-real-run.md` の実機の確かめ → `/release 0.1.0`。棚卸㉛の見立て（C14〜C16）のまま。割り込みはこの数を変えない。
9. **見つけた穴**: ⑴ `shell-scale-linkage` は自由な名前のイベント（例 `OnPastaShellScale`）を返すのに新しいイベントの種類が要る（`RequestOrigin::of_event` は Static・Choice をゴーストに写し、`allowed_static` が自由な名前を断る・`EventId::Sstp` が前例）＝同 spec の設計 ⑵ `test-roots-under-target` の数え＝`TempPath::new` は 80 ファイル 240 か所（✅ `placement-measure-bake-once` の `shell_target_quiet_tests.rs` が 6 か所足した＝見張りが無いので新しいテストは今も `%TEMP%` に入る）⑶ `ukadoc-catalog-refresh`＝npm の最新は 0.11.1（10-26）・手元の写しは 0.10.3。
10. **開発者に決めてほしいこと**（どれも C15 の着手を止めない）: ⑴ `balloon-append-tags` を v0.3.0 へ上げるか（棚卸㉛から・いつかのまま）⑵ `balloon-communicatebox` を Opus にした改め（棚卸㉛から・Fable に戻すなら C16 の行を直す）⑶ C16 の後、v0.1.0 に残るのはウェーブの外の 3 本だけになる＝`winget-release-automation`（PR 450070 は 10-11 から動かない）を版の門に残すか・`popup-menu-shell-look` のために生きている決まり 6 を解くか・`areka-test-binary-stability` を静かな机で回す時期 ⑷ `clippy-199-lints` の段 1 を切り出すか・`ghost-alias-file` を v0.2.0 に置くか（棚卸㉙から持ち越し）⑸ 余力の枠の `multi-ghost-runtime` を、v0.1.0 の残りが待ちで止まる C16 の後に取るか。棚卸㉛の ⑸（`script-small-tags` の `\![*]` の印）は同 spec が描く形で着地した＝済み。
11. **報告の後の開発者の決め**（2026-10-31）: 「メニュー関係はreactor解放されたら大改造するので、内容によっては後回し」＝`popup-menu-extra-resources` は推奨の案 1 を承認＝C15 では ⑴（古い名前 `updatebuttoncaption` を問う・作り直しでも残る kanade の側が大半）だけを取り、⑵（`menu/` に項目を組む仕事＝作り直しで捨てる）は windows-reactor 待ちの段へ回す。

## 着手手順

- 着手は該当の brief を読んで `/kiro-start <名>` へ直行する。brief の file:line は起票時値＝**着手時に必ず再検証**（C1・C2 の brief は main `03e8d7d6` で再測定済み。それ以外は崩れた前提と触るファイルの見立てまで）。
- 新しい課題の起票は `/kiro-discovery`（再入）で just-in-time。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブをまたぐ合流の判断は別セッションで一括。
- **ukadoc 台帳の `owner`**: 各 spec の要件の段で台帳（`doc/ukadoc-coverage/ledger/`）の `owner` に登記し、同時に `roadmap-draft.md` の `owner_count` を追随させる。完了のときに状態を実装済みへ直す（直さないと `coverage-roadmap-refresh` の数え直しの対象が増える）。
- 完了のとき（`/kiro-complete`）は、本ファイルの台帳からその行を消し、「完了サマリ」へ 1 行足す。

## 制約

- Rust 2024・マルチクレート（一覧は structure.md）。**32bit 可搬性の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体は x64＋arm64 ネイティブ。
- 透過は WUC/DComp GPU 合成上のクリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋αマスク）で成立（ULW は撤去済み）。SHIORI4 は C ABI の `ekicyou/shiori4`（v4＝SHIORI/3.0 のヘッダ部分・UTF-8。2026-10-23 に COM の `IShiori` から移した＝`shiori4-api`）。本番は未配線。SHIORI の実体はどの版でも別プロセスの helper で動かす（同日の開発者の決定）。過去互換は 32bit Rust ホスト。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。
- 実機運転の定石: 起動に渡す**検体の絶対パスは `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が印字する**（`folder=` がゴースト・`balloon.<名>=` が同梱バルーン。呼ぶたびに `manual/<検体>/` を作り直すので、**2 つの端末で同時に呼ぶと互いの木を消す**＝1 度印字してから使う）。絶対パス起動（相対は pasta.dll LOAD 失敗）・i686 helper を先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了＋`RUST_LOG` grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。自動終了は強制終了の経路で終了挨拶を経ないので、終了挨拶を確かめる走行では自動終了を上限に留め、終了はキャラ窓の右クリックメニューの「終了」で求める（2026-09-17 host32-window-thread-pump の裁定は Ctrl＋左ダブルクリックだったが、2026-09-19 に `areka-P0-popup-menu-minimal` がその入口を取り除いた。強制退避の Ctrl＋Shift＋左ダブルクリックは残る）。
- 常時テストは x86 を避け偽境界で純 x64 決定論（記憶 prefer-x64-fake-boundary-tests-not-x86）。**ネットへ出るテストを常時テストに入れない**（`network-update` は偽 `HttpFetch`）。

## テーマ別の決めごと（起票のときに開発者が確定した中身・要件の段で覆してよい）

> 4 つのテーマ（SSP MCP の移植・シェル内バルーン・動く画像・文字の現れ方）と α の持ち越しについて、起票のときに決めたことを残す。**着手の並びは「ウェーブ編成」節が正本**で、起票時の「ウェーブ」の小節（M1〜M4・S1〜S4・V1〜V2・R1）は棚卸⑳の再測定で組み直したので history へ移した（旧 S1・S2 の並走の見込みは外れ・文字まわりは `emo-text-file-split` を先頭に直列）。「Existing Spec Updates」の小節（各 brief への追記）はすべて済み。

> 2026-10-23: spec が全部完了した節（`mcp-get-status` の持ち越し・`tools-utf8-child-output` の持ち越し・`impl-watch` の持ち越し・`mcp-stdio-bridge` の持ち越し・ロードマップの俯瞰図と doc の整理・impl-watch のテストの机・`\![move]` の正典と窓の位置の指令）と、棚卸㉔〜㉖の申し送りの節（全文は各 brief）は history「2026-10-23 完了済みの退避」 へ移した。

### SSP MCP の移植（α 後・2026-09-29 `/kiro-discovery` で起票）

> 開発者指示（2026-09-29）「ssp mcp tool の完全移植のための spec 群を立ち上げて。実装は α リリースの後。areka の 127.0.0.1 の適当なポートでサーバを開く形。基本実装 → 空のダミー関数を置いて入り口だけ全部整備 → 個別のコマンド実装。平行開発しやすいように spec 分割」。**事実の正本**は [doc/ssp-mcp/survey.md](../../doc/ssp-mcp/survey.md)（SSP 2.9.05 への実測・ツール定義の逐語は同フォルダの JSON）。ukadoc MCP には MCP の節が索引されていない＝出典は ukadoc の「その他の機能」と SSTP 仕様と実測。

- **決めたこと（起票時・要件の段で開発者が覆してよい）**: ⑴ 本体が 127.0.0.1 で HTTP を待ち受け、Claude Code・Cursor は HTTP で直接つなぐ。**Claude Desktop 用に SSP の `mcp.exe` 相当の中継 exe を `mcp-stdio-bridge` で作る**（2026-09-29 開発者判断。Desktop の `claude_desktop_config.json` は `command` の欄が必須で `url` を書けない〔Desktop 2.9939.4.0 の検査関数で確認〕・コネクタは Anthropic のクラウドからつなぐので 127.0.0.1 へ届かない〔公式の案内〕）。⑵ 既定ポート 9821・`AREKA_MCP_PORT`（`0` で待ち受けない）・既定で有効（SSP と同じ）。9801 は SSP と同時に動かすと衝突し、将来の SSTP の口でもあるので避けた。⑶ SSP の欠陥 2 件（表情表の文字化け・script ログの JSON エスケープ漏れ）は移植しない。⑸ **MCP のプロトコルは自作せず公式 Rust SDK `rmcp` を使う・tokio 依存を入れてよい**（2026-09-29 開発者判断「tokio 依存は入れちゃってもよい。mcp を自作するのは避けた方がよさそう」）。MCP の新しい版への追随は rmcp の版上げで行い、SSP の輸送の癖（survey §2）は参考に格下げ＝ツールの名前・引数・結果の文字列だけを SSP と一致させる。中継 exe は写すだけなので rmcp を使わない。⑷ 分け方は開発者の 3 段（基本 → 入り口 → 個別）に、個別を「触るエンジン」で 6 本に割り、strict を 4 段目に出した。2026-10-03 の開発者裁定で ⑵ の既定ポートは覆った（SSP も 9821 で待ち受けていた）＝既定は 9801 → 9821 の早い者勝ち、どちらも使用中なら隣の 9802 → 9822 … 9810 → 9830（計 20 候補・最初に束ねた 1 つ）・`AREKA_MCP_PORT` の指定はその 1 つだけ（`mcp-server-core` の設計判断 B-13）。
- **採らなかった分け方**: ツールごとに 1 spec（10 本）＝`get_status`・`sakurascript`・`raise_event` が kanade の同じ箇所を触り並走できず、`get_active_ghost_list` などは小さすぎる。基本と入り口を 1 本＝開発者の 3 段に反し、M3 の並走の土台（触るファイルの固定）が基本の検査と同じ spec に埋もれる。
- **既存 spec の更新**: `makoto-dll-host`（brief へ「MCP の reload makoto の口を埋める」を追記済み）。`status-execution-states` は `get_status` の消費者が 1 つ増えるだけ（brief の更新なし）。

#### 3 段目の spec が触るファイル（干渉台帳・`mcp-tool-entrances` の design から転記）

| spec | アプリ本体側（`crates/areka/src/mcp/`） | プロトコル側（`crates/areka-mcp/src/tools/`・要るときだけ） | 自分のエンジン |
|---|---|---|---|
| `mcp-get-property` | `get_property.rs`・`get_property_tests.rs` | `get_property.rs` | sylphya・`areka-ghost` の実行系 |
| `mcp-expression-table` | `get_expression_table.rs`・`get_expression_table_tests.rs` | `get_expression_table.rs` | 表情の表（シェルの定義） |
| `mcp-log-history` | `get_log.rs`・`get_log_tests.rs` | `get_log.rs` | tracing の履歴 |
| `mcp-kanade-tools` | `sakurascript.rs`・`raise_event.rs` と各 `_tests.rs`（`get_status.rs` は 2026-10-06 に `mcp-get-status` が作った） | 同名の 2 ファイル | kanade |
| `mcp-reload` | `reload.rs`・`reload_tests.rs` | `reload.rs` | 読み直しの経路 |
| `mcp-dump-images` | `dump_surface.rs`・`dump_balloon.rs` と各 `_tests.rs` | 同名の 2 ファイル | emo の読み戻し・base64 の符号化 |
| `mcp-strict-errors` | `sakurascript.rs`・`raise_event.rs` と各 `_tests.rs` | 同名の 2 ファイル | エラーログ（`mcp-kanade-tools`・`mcp-log-history` の後＝直列） |

- **後から答えるツール**（kanade・SHIORI・sylphya に問うもの）は、自分のファイルから `mcp::later` を呼ぶ。毎フレーム覗く系を自分で登録しない＝`mcp/mod.rs`・`ghost_session.rs` を触らない。
- **3 段目が触らない共有ファイル**: `areka-mcp` の `handler.rs`・`registry.rs`・`check.rs`・`tools/mod.rs`・`tools/bridge.rs`・`tools/outcome.rs` と各テスト、`crates/areka` の `mcp/mod.rs`・`mcp/resolve.rs` と各テスト、`main.rs`・`ghost_session.rs`。触る要が出たら、その spec の要件に理由を書く。
- **C3 の照合の要点**: C3-⑦〜⑨（`mcp-get-property`・`mcp-expression-table`・`mcp-log-history`）は、上の表でファイルの重なりが 0＝共有ファイル 0。重なりうるのは「自分のエンジン」の側だけ（例: `mcp-get-property` と `property-query-channels`）。
- プロトコル側のファイルは定義と引数の型を `mcp-tool-entrances` で完成させたので、3 段目は多くの場合アプリ本体側だけを触れば足りる。値の範囲や列挙の検査（`NG:Unknown …`）は、アプリ本体側の処理が結果として返す。

#### Specs (dependency order)

- [x] areka-P0-mcp-server-core -- 127.0.0.1 の HTTP・JSON-RPC・MCP の版と検査（ツール 0 本）。Dependencies: α 完成宣言
- [x] areka-P0-mcp-tool-entrances -- ツール 10 本の定義・引数検査・ghost_name の解決・World への橋・ダミー 9 本。Dependencies: areka-P0-mcp-server-core
- [x] areka-P0-mcp-stdio-bridge -- Claude Desktop 用の stdio ⇔ HTTP 中継 exe。Dependencies: areka-P0-mcp-server-core
- [x] areka-P0-mcp-get-property -- get_property。Dependencies: areka-P0-mcp-tool-entrances
- [x] areka-P0-mcp-get-status -- get_status（10-05 棚卸㉒で mcp-kanade-tools から切り出し）。Dependencies: areka-P0-mcp-tool-entrances
- [x] areka-P0-mcp-kanade-tools -- （2026-10-19 に完了）sakurascript・raise_event。Dependencies: areka-P0-mcp-tool-entrances, areka-P0-mcp-get-status
- [x] areka-P0-mcp-expression-table -- get_expression_table。Dependencies: areka-P0-mcp-tool-entrances
- [x] areka-P0-mcp-log-history -- get_log。Dependencies: areka-P0-mcp-tool-entrances
- [x] areka-P0-mcp-reload -- （2026-10-22 に完了）reload と \![reload,…]。Dependencies: areka-P0-mcp-tool-entrances, areka-P0-shell-balloon-switch
- [x] areka-P0-mcp-dump-images -- dump_surface・dump_balloon。Dependencies: areka-P0-mcp-tool-entrances
- [x] mcp-strict-errors -- （2026-10-30 に完了）strict の記録（10-23 要件確定＝再生まで印を運ぶ・rmcp は直さず 415 の後に areka の側で答える＝10-30 の裁定）。Dependencies: areka-P0-mcp-log-history, areka-P0-mcp-kanade-tools, descript-window-keys, mouse-click-wheel-events
- [x] areka-P0-mcp-ghost-name-match -- ghost_name の照合を SSP 2.9.07 に合わせる（大小・本体側名・前後の空白・空文字）。共有の resolve.rs を触る＝3 段目と同じウェーブに置かない。Dependencies: areka-P0-mcp-tool-entrances（席は 3 段目の着地の後）

### シェル内バルーン（α 後・2026-10-01 `/kiro-discovery` で起票）

> 開発者指示（2026-10-01）「areka では設計上、シェルとバルーンの描画区別を付けないように留意してもらっていたと思う。実際に、シェル内にバルーン領域を持つゴーストを設計したい。イメージは『窓際のぱすたさん』の縦書きキャッチコピー（ghost_dev リポジトリ `doc/assets/text_layout/縦書きキャッチコピー.html`）。実装時期は α 後なるべく早い時期」。**完全に areka 独自の仕様＝差別化のための機能**（開発者）。参考の寸法は同ゴーストの `doc/画面設計_縦書き要件定義.md` §3.1（白い窓の絵の中に、天 62px・外側 14px・右の台詞欄 2 列・左の独白欄 3 列・本文 27px・列ピッチ 48px）。

- **起票時の実測（main `35209987`）**: 「シェルとバルーンを区別しない」は一番下の層（emo-present の窓ごとの組み立て＝どの窓にも文字の層の差し込み口がある・拡大率も流れる）では本当で、その上（文字の描き手の結線・`BalloonModel` だけの設定・1 スコープ 1 か所・文字の数でバルーンの窓が出る）は区別している。土台の作り直しは要らず、上の層の配線と宣言の決まりが要る。正典（ukadoc）には、シェルの中の文字の場所・1 人に 2 か所・ルビ・縦中横・禁則・クリックでの早送りのいずれも無い（調査で 0 件）。
- **決めたこと（開発者確定・要件の段で覆してよい）**:
  1. surfaces.txt の新しいブレス `balloon.名前`（ukadoc の「surface*ブレス」と同じ用語の型）。中身はバルーンの descript.txt と同じキー＋大きさ（仮称 `size`）。大きさはブレスが持つ（サーフェスごとに大きさが変わるのは良くない）。
  2. 置くのは `surface*`ブレスの element定義 `elementN,balloon,名前,X,Y`（描画メソッド `balloon`＝画素を重ねるのでなく文字を描く）。数は任意。将来の「`surface1000` を element定義で置く」とも整合する（正典の pattern定義が描画メソッドを残しファイル名の欄でサーフェスを指す前例）。
  3. **バルーンの中身はすべて互換**＝同じバルーンの実装で、描く先が替わるだけ。背景の絵は持たない。当てはまらないのは `windowposition.*`・`use_self_alpha`・`use_input_alpha` だけ。
  4. 行き先は「スコープ × 名前」・既定は一番若い element番号・`\b[名前]` で切り替え・普通のバルーンと併用しない・`\c` は今の行き先だけ・選択肢も出す・フォントはシェル → ゴーストのフォルダの順。
  5. サーフェスの切り替え: 同じ名前があれば文字ごと移る／無ければ文字を持ったまま隠し既定へ書く／1 つも無ければ普通のバルーンへ。
  6. 早送り＝話している最中の 1 クリックで次の `\x` か台詞の終わりまで（箱のクリックはシェルのイベントにしない）。**クリック待ちは台本に明示した `\x` だけ＝自動の改ページは作らない**（「デスクトップマスコットは利用者の状況お構いなしに喋るもの」）。あふれは正典の自動スクロール＋（独立 spec の）押し出しのフェード。
  7. ルビ `\![text,ruby,親文字,ルビ]`・縦中横 `\![text,combine-upright,…]`（台本の独自拡張は SSP の流儀に倣い `\!` に入れ、`\![text,…]` を文字組みの入り口にする）。キーは CSS のプロパティ名の `-` を `_` にした名前（`line_break`〔既定 `anywhere`＝互換〕・`hanging_punctuation`・`text_combine_upright`・`text_orientation`・`line_height`・`letter_spacing`）。
  8. 未実装の印（`arrow*`・`onlinemarker.*`・`number.*`・`sstpmarker.*`・`sstpmessage.*`）も今回の spec 群で実装まで持つ（開発者指示）。SSTP の 2 項目は表示だけ作り、受信は登記だけの行「SSTP の受信」。
- **採らなかったもの**: 透明なバルーンをシェルに重ねる案（正典だけで組めるが窓が 2 枚・1 人 2 か所が無理）・バルーンの側に「シェルの上に描く」と書かせる案（バルーンが特定のシェルの寸法に縛られる）・`balloon.*`ブレスは見た目だけで矩形はサーフェスの専用行に書く案（将来のサーフェスの element定義と書き方が分かれる）・青空文庫式のルビ（唐突）・新しいタグ（`\_r` など）・自動の改ページ・背景の絵・SSP の「縦書きでは同梱フォントを標準ゴシックへ差し替える」（GDI の制約で、areka の DirectWrite には無い）。

#### Specs (dependency order)

- [x] areka-P0-shell-balloon -- `balloon.*`ブレス・描画メソッド `balloon`・シェルの窓への結線・行き先と切り替え・併用しない。Dependencies: α 完成宣言
- [x] areka-P0-shell-balloon-frame-align -- 箱の置き場所の付け替えを `\s` と同じフレームに収める・`Status` の 1 フレームの欠けと箱だけのときの警告（`shell-balloon` の実機の確かめの気付き・バグ）。Dependencies: areka-P0-shell-balloon
- [ ] balloon-font-file -- `font.name`・`\f[name]` のフォントファイル（バルーン／シェル → ゴーストのフォルダ）。Dependencies: α 完成宣言
- [ ] text-typesetting -- 禁則・ぶら下げ・縦中横・字の向き・縦書きの字形。Dependencies: α 完成宣言
- [ ] talk-fast-forward -- クリックでの早送り・`\x`／`\x[noclear]`・`clickwaitmarker.*`。Dependencies: areka-P0-shell-balloon
- [ ] text-ruby -- `\![text,ruby,…]`・`line_height`・`letter_spacing`。Dependencies: text-typesetting
- [ ] balloon-markers -- `arrow*` と手動スクロール・`onlinemarker`・`number` と `\![set,balloonnum]`・SSTP の印（表示だけ）・装飾の系列。Dependencies: areka-P0-shell-balloon
- [ ] balloon-scroll-fade -- 押し出される行（列）のフェード（areka 独自・既定は無効）。Dependencies: balloon-markers
- [ ] balloon-element-order -- `balloon` の element定義を並び順どおりの重ね順で描く（`shell-balloon` の縮めを外す追跡 spec）。Dependencies: areka-P0-shell-balloon

### 動く画像（α 後・2026-10-01 `/kiro-discovery` で起票）

> 開発者指示（2026-10-01）「アニメーションをサポートする画像ファイルに対応して欲しい。具体的には、画像読み込み時にサブエレメント分解して、アニメーション表示を回す案を出しておきます。arekaのシェルエレメント管理は再帰構造を持つように設計指示していたので、ちゃんと実現されているなら可能なはず。webp形式サポートとかがよいかな？apngもあるけど。時期はα後」。正典は ukadoc の element定義の項「surface*.pngまたはelement定義にアニメGIF/APNG/WebPアニメを指定すると、SERIKO定義を書かなくても自動的にアニメーションする(SSP 2.7.38～)」と、pattern の描画メソッド `import`（2.7.50）。

- **起票時の実測（main `5e37745e`）**: 本番の読み込みは WIC の `GetFrame(0)`＝1 枚目のコマだけ。**再帰は半分だけ実現**＝pattern → サーフェスの参照は `flatten_surface` が再帰する（位置のずれの加算・循環の停止）が、element は画像専用で他のサーフェスを指す道が型に無く（設計文書の「element が他サーフェスを参照」は未実装）、入れ子の内側は `PatternState` を見ない（内側のアニメーションは動かない）。seriko の表は `always`・`runonce`・`bind` を記録しない。合成器が描くのは `overlay` だけ。
- **決めたこと（開発者確定・要件の段で覆してよい）**: ⑴ GIF・APNG・WebP の 3 形式を同時に入れる（`image` クレートが 3 形式とも重ね済みのコマと待ち時間を返す＝1 形式に絞っても手間は同じ）。⑵ 動く絵だけ `image`・静止画は WIC のまま（WIC は APNG を読めず、WebP は Windows の拡張機能しだい）。⑶ 読み込み時にコマへ分解し、SERIKO の型へ写して今の seriko の時計で回す＝アニメのエンジンは 2 つのまま。⑷ 読み込み／再生に分ける。`import` は再生の側に含める（別にしても並走が増えない）。⑸ interval `always` を再生の側で入れる。⑹ **階層化エレメントを `surface-element-nesting` として起こし、動く絵はその上に載せる**＝読み込み時にコマから「`always` でコマを順に指す子サーフェス」を作り、element でそれを置く（開発者「手書きで定義できるようになる→画像を自動に定義分解する。ながれできれい」）。⑺ 書き方は element定義のファイル名の欄に**数字だけ＝サーフェスの番号**（正典の pattern定義の前例・`shell-balloon` の見込みと同じ。`surface100`〔拡張子の無い画像と紛れる〕と新しい描画メソッド〔overlay 以外で重ねる道を塞ぐ〕は採らない）。⑻ 子の当たり判定は親へ持ち込む（位置をずらし・親の element の順で手前奥）。⑼ 内側の `balloon` の element定義は警告して無視（最初の版）。⑽ **子の時計は独立し、親の面の切り替えで巻き戻らない**（開発者「1→2 に切り替えたとき、両方から参照されている 100 のアニメーションがリセットされるとダサい」）＝「スコープ × 子サーフェス」に 1 つ・初めて見えたときに動き出し、シェルが替わるかゴーストが降りるまで止めない（`1 → 3 → 1` でも巻き戻らない）・一番上のサーフェス自身のアニメーションは今までどおり切り替えで最初から。
- **採らなかった分け方**: 画像ごとに独立した時計（アニメのエンジンが 3 つ目になる）・WIC だけ（APNG が読めない・WebP が環境しだい）。
- **階層化エレメントの議題（2026-10-01・決着）**: 開発者の問い「階層化エレメントが扱えるか・surfaces.txt でどう定義できるようにするか」。実測は上のとおり「再帰の骨組み（合成器・アトラス）は在り、入口（element の型）と内側の時計が無い」。決めたことは ⑹〜⑽。「シェル内バルーン」節のセッションへ実測と方針を共有した。
- **依存の追加**: `image`（本番へ移す・`gif` の機能を足す）は `tech.md` の「意図的依存追加」への登記と開発者の承認が要る＝`animated-image-decode` の要件の段で。→ **要件の段で覆った（2026-10-04 裁定）**: 動く GIF には対応しない＝⑴ の 3 形式は APNG・WebP の 2 形式になり、`image` の機能は `png`・`webp` だけ（`gif` は足さない）。承認と登記は `tech.md` の `image`・`image-webp` の項。

#### Specs (dependency order)

- [x] areka-P0-animated-image-decode -- APNG・動く WebP（動く GIF は 2026-10-04 の裁定で非対応）の全部のコマ・待ち時間・繰り返し回数を読み、アトラスにコマの番号で載せる。Dependencies: α 完成宣言
- [x] areka-P0-surface-element-nesting -- element定義でサーフェスを置く（数字だけ＝番号）・子の当たり判定・子の時計は独立。Dependencies: α 完成宣言
- [x] areka-P0-animated-image-playback -- 動く絵を子サーフェスへ分解して置く自動アニメーション・interval `always`。Dependencies: areka-P0-animated-image-decode, areka-P0-surface-element-nesting
- [ ] animated-image-import -- `import` メソッド（10-05 棚卸㉒で animated-image-playback から切り出し）。Dependencies: areka-P0-animated-image-playback
- [x] placement-measure-bake-once -- 起動の採寸で動く絵の全コマを読んで捨てない・バルーンを scope ごとに 2 回焼かない（2026-10-05 起票）。Dependencies: areka-P0-animated-image-decode
- [ ] element-clipping-option -- element定義の `--clipping` で矩形だけを描き、付けた element定義では動く絵として読まない（正典 C2・2026-10-05 起票）。Dependencies: areka-P0-animated-image-decode, areka-P0-surface-element-nesting
- [ ] extent-element-offset -- 外形が画像の element定義の X,Y を数えず、はみ出す画像が黙って切られる穴を、ukadoc の外形の定義に照らして直す（2026-10-05 起票）。Dependencies: areka-P0-surface-element-nesting

### 文字の現れ方（α 後・2026-10-01 `/kiro-discovery` で起票）

> 開発者指示（2026-10-01）「文字のタイプライター表現において、フェードインしながら文字を表示するモードが欲しい。よくノベルゲームで見ますよね。より夢のある仕様では、ニンテンドーのゲームみたいに、文字が踊りながら表示されるエフェクトも欲しいけど、これは Option の夢 spec として別に切ってください」。

- **起票時の実測（main `9cf09f5d`）**: 1 字ずつの表示は字ごとの「現れる時刻」を既に持つ（`areka-emo-text/src/state.rs` の `visible`）＝フェードも動きも「現れる時刻からの経過」の純関数で決まる。手が要るのは描画の側で、今は一度描いた字を描き直さない。
- **決めたこと（開発者確定）**: ⑴ 有効にする書き方はバルーンのキー `text_reveal`（`balloon.*`ブレスにも書ける）と台本 `\![text,reveal,…]`（台詞の終わりまで）の両方。⑵ 早送りのクリックと `\_q` の中では即座に不透明・定位置。⑶ 既定のフェードの長さは 150ms。⑷ 夢の方は**現れるときだけ動いて止まる**（跳ねる＝ポーンと跳ねて定位置に戻る・揺れる）。ずっと動き続ける演出は負荷が大きく目に毒なので作らない。
- **採らなかったもの**: キーだけ（ここぞという台詞だけ切り替えられない）・早送りでもフェードを待たせる・ずっと動き続ける演出。

#### Specs (dependency order)

- [ ] text-reveal-fade -- 字が透明から不透明へ変わる現れ方（`text_reveal,fade`・`\![text,reveal,fade,…]`）。Dependencies: α 完成宣言
- [ ] text-reveal-dance -- 現れるときだけ跳ねる・揺れる（夢・任意）。Dependencies: text-reveal-fade

### alpha-release-signoff の持ち越し（α 後・2026-10-02 `/kiro-discovery` で起票）

> 出どころは `alpha-release-signoff` の完成判定 `verification/alpha-completion.md` §6「持ち越した事項」の、引受先を「α 後の `/kiro-discovery`」と書いた 5 行（受入記録 `verification/acceptance-record.md` の §8.1 ⑴・⑵・§8.5・§8.6・§8.8）。開発者指示（2026-10-02）「あ、起票はあとでやってくれますよね。「実装完了を承認」スキルは最後に実施しますし。その前提で、今は実装に戻ってください。」。どの行も α の判定を書き換えない。

- **分け方**: 5 行を 4 本にした。§8.1 の ⑴ と ⑵ は、どちらも「インストールの最中に使用中のものと当たる」危険で、同じ手続き（`crates/areka/src/install/`）と同じ実機の手順（`target\` の下の根で入れる）を使い、**測ってから扱いを決める**という進め方も同じなので 1 本（`install-live-target-hazards`）にまとめた。§8.5（窓の位置の保存）・§8.6（配布スクリプト）・§8.8（`install.txt` の読み方）は触る場所が互いに重ならないので 1 本ずつにした。
- **採らなかった分け方**: §8.1 と §8.8 を 1 本（どちらもインストール）＝§8.8 は ukadoc の読み方を揃える正典の仕事で、実測を待たずに進められる。実測待ちの ⑴⑵ と束ねると、正典の仕事が実機の結果を待つことになる。§8.5 を登記だけの行にする＝設計の意図（掴んで離したときだけ保存）に反するバグで、初回だけの位置合わせの規則を崩すので brief を持たせた。
- **段**: 4 本とも「α 後」（開発者指示）。`drag-click-without-move` は種別がバグなので、α 後の棚卸で「バグ → その他」の順の先頭に並べ直してよい。

#### Specs (dependency order)

- [ ] install-live-target-hazards -- 表示中のシェル・使用中のバルーンのフォルダへの上書きと、起動中のゴーストへ入れる途中の Windows の終了を実測し、扱いを決めて実装する。Dependencies: α 完成宣言
- [x] areka-P0-drag-click-without-move -- 動かさない左クリックで窓の位置を保存しない（バグ）。Dependencies: α 完成宣言
- [x] areka-P0-package-check-temp-cleanup -- **10-02 に `release-package-versioned` へ合流**（同じスクリプト・C1 の先頭のタスク）
- [x] areka-P0-install-companion-canon -- **10-03 に要件の段で 3 本へ引き継いで閉じた**（下の「同梱バルーンの正典」節）

### 同梱バルーンの正典（2026-10-03 `/kiro-discovery` 再入で `install-companion-canon` を分解）

> 開発者指示（2026-10-03）「現地点で対応していない機能に影響するのであれば、ロードマップを調整し、複数specへの分解で対応して欲しい。本specは「実施せず、別specに引き継ぐ」としてspec作成次第PR終了するのがよいかも」。要件の段で、ukadoc「同時インストール」の「ゴーストやシェルに紐づくバルーンとして設定されるのは最初の1個だけである。」が、areka に無い 2 つの機能（ゴーストの標準バルーン・シェルごとのバルーン）に触れると分かった。

- **分け方**: 境界で 3 本に分けた。
  - インストールの読み方＝`areka-nar` だけ。
  - ゴーストの標準バルーン＝起動時の解決の鎖（`catalog.rs`・`boot_resolve.rs`・`boot_config.rs`）。
  - シェルに紐づくバルーン＝起動の順番と着替えの配線（`emo2_boot`）。
  - 3 本目は `shell-balloon`（C2）と `frame/switch.rs` を共有するので、その後に置く。
- **前身の資料**: `completed/areka-P0-install-companion-canon/` の書きかけの requirements.md（要件 1〜4・6〜8＝1 本目の下書き・要件 5＝2 本目の出発点）。
- **採らなかった分け方**:
  - 前身のまま 1 本で「ゴーストの最初の 1 個」まで入れ、シェルを後回しにする＝正典の半分だけを入れ、descript の `balloon`・`default.balloon.path`（同じ鎖の同じ段の並びの問い）を置き去りにする。
  - 3 本目を 2 本目に混ぜる＝`emo2_boot` を触るので `shell-balloon` を待つことになり、2 本目まで遅れる。
- **覚え書きへ**: `recommended.balloon(.path)`（警告の出し方の方針が先）。

#### Specs (dependency order)

- [x] areka-P0-install-companion-reading -- 書庫を入れるときの同梱の読み方を ukadoc に揃える 4 点と並びの順。Dependencies: none（C2-⑥）
- [x] areka-P0-ghost-standard-balloon -- 起動時のゴーストの標準バルーン（同梱の最初の 1 個・descript の `balloon`／`default.balloon.path`）。Dependencies: areka-P0-install-companion-reading
- [ ] shell-companion-balloon -- シェルの書庫の同梱バルーンをそのシェルに紐づけ、着替えと起動で使う。Dependencies: areka-P0-ghost-standard-balloon, areka-P0-shell-balloon
- [x] areka-P0-ghost-inner-balloon -- （2026-10-23 に完了）ゴーストの中の `balloon/<名>/` にバルーンを抱える（areka 独自・一覧・記憶・切替・書庫からの入れ方を揃える）。2026-10-05 `ghost-standard-balloon` の要件の討議で開発者が「nar 修正と一緒に別 spec」と決めた。Dependencies: areka-P0-ghost-standard-balloon, shell-companion-balloon

### MCP の areka 独自ツールと影響の段（2026-10-05 `/kiro-discovery` で起票）

> 開発者指示（2026-10-05）「MCP ツールを SSP 互換で作ってもらったが、エージェント目線でこんなツールがあったらいいな、面白いかも、という MCP ツールを、深掘り検証・ウェブ探索して提案してほしい。デスクトップマスコットのすそ野を広げるような提案が欲しい」。

- **調べたこと（起票時）**:
  - よそのエージェントの「体」は、待つ間の 3 つ（今どの段か・呼ばれているか・終わったか）が中心。Codex Pets（OpenAI・2026-05）・OpenPets（MCP の `status`／`react`／`say`）・Vibe Island がその例。「見せる」から「その場で答えを返す」へ進んでいる。
  - SSP の MCP（2.7.06〜）には、押し出し・声・状態の語彙・ゴーストを探す道具が無い。
  - 伺かの AI ゴーストの共通の弱点は、API キーの置き場・同期で止まる・記憶が無い・表情の語彙を教えられない、の 4 つ（最後は `get_expression_table` が答え）。
  - MCP の 2026-07-28 版で sampling は非推奨と報告あり（未確認）。ゴーストが LLM を呼ぶ形は採らない。
- **決めたこと（開発者・要件の段で覆してよい）**:
  - ⑴ **さくらスクリプトでできるツールは優先度を下げる**。「さくらスクリプトを知らないエージェントは無い・スキルでもカバーできる。SSP で喋らせる機能がさくらスクリプトに限定されているのは、さくらスクリプトでできることが強力だから」。採らなかったもの（さくらスクリプトかスキルで足りる）は次のとおり。
    - 平文と感情から喋らせる `say`・掛け合い。
    - エージェントの状態を渡す `set_agent_state`（`raise_event`＋`\s` で足りる・状態の語彙はスキルや文書で配る）。
    - `install_ghost`（`\![execute,install,…]`）・`switch_ghost`（`\![change,…]`）。
  - ⑵ **安全の考え方**: 「エージェントは通常、強大なアクセス権限を最初から持っている。ベースウェア側で多少制約してもあまり関係ない」。区分けするなら、影響の段で分ける。
    - **高**: ベースウェアの外に環境影響を与えるもの（特に留意）。
    - **中**: ベースウェア内の環境への影響。
    - **低**: その場のゴーストの挙動にのみ影響するもの（気にしない）。
    - 「ゴースト切り替えも低。アクター同士の演技の範疇であれば制約なし」。
    - 中・高で何をするかは `script-impact-tiers` の要件の議題。
  - ⑶ 正典の `SecurityLevel`／`SenderType`（ukadoc の spec_shiori3・spec_sstp）が areka に無いこと（SHIORI へは `local` 固定）を開発者が指摘し、`script-security-level` を起票した。影響の段とは別の軸。MCP は SSP と同じく Owned SSTP＝`local` なので、正典の制約は MCP には掛からない。
  - ⑷ 開発者の指摘（同日）「今の MCP ツールは SHIORI への情報要求が無い。raise_event はトークになるだけで MCP 応答にはならない」。SSP の `raise_event` の結果の本文には返り台本が入るが、再生され、データではない。正典で外とゴーストがデータを往復させる口 `X-SSTP-PassThru-*`（要求のヘッダを SHIORI へ通し、SHIORI の応答のヘッダを SSTP の応答へ中継・ghost_terminal の `ShioriEcho` 系が実例）を MCP へ写す `mcp-shiori-query` を起票した。開発者裁定「正典寄りの形」＝台本が返れば NOTIFY と同じく再生し、データはヘッダで返す（areka 独自の「再生しない」は持たない）。ヘッダの名前は `X-MCP-PassThru-*`（開発者「プロトコル的には `X-` で始まるヘッダは任意に命名可能」・正典の `X-SSTP-PassThru-*` は SSTP の経路に限る定義なので経路で名前を分ける）。
- **採らなかった案（理由）**:
  - ゴーストの検索（SSP 側のオンライン MCP がある）。
  - 声（areka に音の層が無い・別テーマ）。
  - 集中タイマー・記憶（ゴーストの領分）。
  - 利用者がいるかの検知（デスクトップマスコットの枠を超える・プライバシー）。
  - 画像 1 枚からシェルを作る案（面白いが大きい・要望が出たら起票）。
  - MCP の購読や Claude Code の channels による押し出し（研究プレビュー・`mcp-user-response` の後で検討）。

#### Specs (dependency order)

- [x] areka-P0-mcp-author-tools -- 独自ツールの登録口と橋＋`check_script`（読むだけ・10-05 に 1 本へ絞った）。Dependencies: areka-P0-mcp-tool-entrances
- [x] areka-P0-script-security-level -- （2026-10-23 に完了）出どころの運搬・SHIORI の `SecurityLevel`／`SenderType`・ukadoc のタグの制約。Dependencies: none
- [ ] script-impact-tiers -- 影響の段（高・中・低）の正本の表と出どころごとの扱い。Dependencies: areka-P0-script-security-level, areka-P0-mcp-kanade-tools
- [ ] mcp-user-response -- 選択肢・入力・操作をエージェントへ返す。Dependencies: areka-P0-mcp-author-tools, areka-P0-mcp-kanade-tools, areka-P0-mouse-drag-events
- [ ] mcp-shiori-query -- ゴーストへ情報を問う（SSTP NOTIFY 相当＋`X-MCP-PassThru-*` の往復）。Dependencies: areka-P0-mcp-author-tools, areka-P0-mcp-kanade-tools

### 配布と公開（winget・crates.io・2026-10-02 `/kiro-discovery` で起票）

> 開発者指示（2026-10-02）「winget 対応インストーラーを作成したい。最終的には繰り返し spec を実行するたびに、バージョン番号を 0.0.1 加算しながらリリースを作るような運用。アーカイブの署名が必要なのか？無料で済ませたい」「とにかく無料でできる手段。CI 実行でよいが、PR のたびに実施されるとバージョン管理が変にならないか？リリースタイミングはこちらで決めたい」「可能なら crates.io へのリリースも組み込んで」。

- **起票時の実測（main `76e17654`）**: 版は `Cargo.toml` の `[workspace.package] version = "0.0.1"` 1 か所（21 クレートが継承・SHIORI へ渡す版もここから）。配布は配布スクリプト（今の `tools/package.ps1`）が手元で x64 の zip を日付とコミットの名前で作るだけ。GitHub Releases 0 件・`.github/` なし・SHA256 なし・署名なし。`/kiro-complete` は版にもタグにも触らない。crates.io には `areka`・`dola`・`wintf` の 0.0.1 だけ（名前の確保）。部品のクレートの名前（`areka-*`・`shiori-*`・`log-capture-kit`・`temp-path-kit`）はすべて空いている。
- **調べた事実（2026-10-02・出典は `winget-manifest-submission`・`release-code-signing` の brief）**: winget の zip／portable に署名の義務は無い（MSIX だけ要る）。portable はスタートメニューのショートカットを作らない。PATH へ出すリンク経由の起動で隣の DLL を見失う既知の問題は `ArchiveBinariesDependOnPath: true` で避ける。初回の提出は手で（1 PR 1 版・人の承認・実例で 2 日）、以後は winget-releaser（Release の公開がきっかけ）。無料の署名は SignPath Foundation だけ（CI のビルドが前提）。Microsoft の Artifact Signing は日本の個人には使えない。crates.io は Trusted Publishing で長生きするトークン無しに出せる（新しいクレートの初回は手元から）。
- **決めたこと（開発者確定・2026-10-02）**:
  1. **入れ物は zip のまま**（winget の `zip`＋`portable`）。アイコンが欲しくなったら後からインストーラーを別 spec で足す。
  2. **名乗りは zip 版＝`Areka.Areka.Portable`（「areka ポータブル」）・インストーラー版＝`Areka.Areka`（空けておく）**。入れ物の種類ごとに別の名乗り（portable で入れた人の環境へ次の版でインストーラーを被せる乗り換えを winget は綺麗に扱えない）。発行者は areka プロジェクト自身（`ekicyou.areka` は却下）。
  3. **x64 と arm64 を最初から並べる**（arm64 はこのプロジェクトの動機）。補助 exe は i686 のまま。
  4. **きっかけはタグ**。普通の PR では何も起きない。スキル `/release` を開発者が打ったときだけ、版を +0.0.1 → PR → squash マージ → `v{版}` のタグ → GitHub Actions がビルド・zip・SHA256・Release → crates.io → winget へ PR。版の正本は `Cargo.toml`、タグはそれを写す。
  5. **無料で**: 未署名で出す（規約上の障害は無い）。CI ができたら SignPath Foundation に申請する（任意）。有料の証明書は開発者がその場で決める。
  6. **「外部 CI は持たない」はテストの門の話**＝ビルドと配布だけを Actions に乗せる。zip の起動確認（`-Check`）は窓を出すので手元に残す。
  7. **crates.io は部品と本体の公開・名前の確保のため** ⇒ **10-03 `crates-io-publish` の要件討議で改め**: 版を出し続けるのは汎用のライブラリ `wintf`・`dola` だけ。`areka` は 0.0.1 の名前の確保のまま・本体と部品は出さない。`cargo install areka` では 32bit の補助 exe が付かないので、利用者向けの入れ方は winget か zip。
  8. **後段の起こし方（10-03・`release-ci-workflow` の要件 8＋セッション間の合意）**: `release.yml` は後段を呼ばず合図も送らない（権限は `contents: write` だけ）。`GITHUB_TOKEN` で公開した Release は `release: published` を起こさないので、winget は `release` の走りの終わり（`workflow_run`）を受けて「タグの push で始まった」かつ「成功」で絞る。crates.io は Trusted Publishing が `workflow_run` を断るので、自分もタグの push で起き、同じコミットの `release` の走りが成功で終わるのを待ってから出す。開発者「手でコマンドを打つ手順は作らない・自動で流れるならどの案でもよい」。
- **採らなかったもの**: main への push で自動リリース（PR のたびに版が上がる）／feature の PR の中で版を上げる（並走する枝が同じ行を取り合う）／`cargo release` で main へ直接 push（PR 経由の決まりに反する）／zip の名前に日付とコミットを残す（版と重ねると長く、道具の自動判定が迷う）／MSIX（署名が必須で無料の道が無い）／zip 版とインストーラー版を同じ名乗りにする。
- **既知の制限として説明書に書くこと**: スタートメニューにアイコンは出ない（コマンド名 `areka` か、インストール先のフォルダから）／Smart App Control を有効にしている環境では未署名の exe が止まる／arm64 の実機の確かめは開発者の手元に機械が無ければ利用者の報告待ち。

#### Specs (dependency order)

- [x] areka-P0-release-package-versioned -- `-Check` の後片付け（合流）・版入りの固定の名前と SHA256・arm64 の zip・リンク経由の起動での場所の解決。Dependencies: none（C1）
- [x] areka-P0-release-ci-workflow -- タグ `v*` で動く GitHub Actions（ビルド → zip → Release）。Dependencies: areka-P0-release-package-versioned
- [x] areka-P0-release-cycle -- 繰り返し spec（2026-10-15 に完了。手順はスキル `/release` へ移した）。版 +0.0.1 → PR → squash マージ → タグ（初回は `wintf`・`dola` の Trusted Publishing の設定を含む）。Dependencies: areka-P0-release-ci-workflow, areka-P0-crates-io-publish, areka-P0-tools-utf8-child-output
- [ ] dola-yaml-backend -- `dola` の機能 `yaml` が引く `unsafe-libyaml`（`RUSTSEC-2024-0449`）を、YAML の読み書きの差し替えか機能ごとの撤去で片づけ、`deny.toml` の見逃しを消す（2026-10-15 `release-cycle` の完了の全体テストの赤から起票）。Dependencies: none
- [x] areka-P0-tools-utf8-child-output -- リリースの道具が `cargo metadata` の UTF-8 の出力をコンソールの文字コードで解く穴を、読む側で直す（バグ・2026-10-05 `/kiro-discovery`・`release-cycle` の初回の実装からの依頼）。Dependencies: none
- [x] areka-P0-winget-manifest-submission -- `Areka.Areka.Portable` のマニフェスト・初回の手提出・winget-releaser。Dependencies: areka-P0-release-cycle
- [x] areka-P0-crates-io-publish -- `wintf`・`dola` だけを crates.io へ（Trusted Publishing・自分の workflow `crates-io.yml`）。Dependencies: areka-P0-release-package-versioned, areka-P0-mcp-server-core（C2・`release-ci-workflow` と並走）
- [ ] release-code-signing -- SignPath Foundation への申請と CI での署名（任意）。Dependencies: areka-P0-release-ci-workflow

### mouse-drag-events の持ち越し（2026-10-05 `/kiro-discovery` で起票）

> 出どころは `mouse-drag-events` の実機（4.2・クローディア）と完了時の棚卸（`completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes・`verification/real-machine.md` 4 章）。開発者の方針（2026-10-05）「実機で areka が未対応だったためにうまくいかなかった件はすべて起票」。

- **分け方**: 4 本にした。`base` と `collisionex` はどちらも `shell/decode.rs` を触るが、描画（合成器）と当たり判定（`hit.rs`）で直す先が分かれるので別にした（同じ列で直列）。位置の保存は `placement/`、テストの AV はテストの土台で、触る所が重ならない。
- **負荷のときの赤**（`install` の desk の上書き・`session_end` の `sync_send`・`emo2_boot::ghost_switch` の `boot_event_tests`）は新しく起票せず、`ghost-session-test-load-flake` の brief の Current State へ観測を足した。

#### Specs (dependency order)

- [x] areka-P0-element-base-method -- element定義の描画メソッド `base` を読んで描き、他の未対応の描画メソッドの行は記録を出す（バグ）。Dependencies: areka-P0-surface-element-nesting
- [ ] collisionex-regions -- `collisionex` の 4 つの形を読んで当たり判定に使い、Reference4 に名前を載せる。Dependencies: areka-P0-element-base-method
- [x] areka-P0-mcp-dump-images-residue -- `dump_balloon` の文字の面の読み戻しを UI スレッドから外し、`dump` の 2 本の後始末 5 件を片付ける。Dependencies: areka-P0-mcp-dump-images, areka-P0-mcp-kanade-tools（後始末の ⑥ だけ）
- [x] areka-P0-char-position-save-on-exit -- 起動の最後に並べ終えた時点で、記憶に位置が無いキャラクター窓の位置を書き、再起動で前回の並びに立つ（バグ・書く時機は 2026-10-05 の要件ディスカッションで「正常な終了」から改めた）。Dependencies: なし
- [x] dpi-realign-remembered-chain -- （2026-10-24 に完了）拡大率が変わったあと、位置を覚えているキャラクターどうしも隣り合ったままに見えるよう詰め直す（バグ）。Dependencies: areka-P0-char-position-save-on-exit
- [→] areka-P0-areka-test-threads-av -- （2026-10-11 `areka-test-binary-stability` の件 1 に吸収・brief は同 spec に写した）`--test-threads=4` の `STATUS_ACCESS_VIOLATION` の組を特定して直す（バグ）。Dependencies: なし

### バルーンのリンクと OS の連携（2026-10-05 `/kiro-discovery` で起票）

> 開発者の問い（2026-10-05）「さくらスクリプトで、リンク（選択肢）をクリックしたらファイルを開いたりできるか？ また、リンクを右クリックしたらリンクに登録されている文字列（URL とか）をコピペできたりしますか？ バルーンからアプリを開く用途は結構あると思う。SHIORI でもできなくはないけど、あったらうれしい機能だと思う。ukadoc を調べて、既に存在するタグなのかどうかも確認せよ」。

- **ukadoc で確かめたこと**:
  - クリックで開くのは正典にある。`\q[タイトル,script:…]`（`\__q` も同じ）と、`\j[http/file:///mailto:…]`・`\![open,file/browser/explorer/editor/mailer]` の組み合わせで、SHIORI を通さずに書ける（例 `\q[メモ帳,"script:\![open,file,notepad.exe]"]`・`,` を含む台本は `"…"` で括る）。`\_a` には `script:` の形が無く、SHIORI の `OnAnchorSelect` の応答で `\j` を返すのが作法。
  - 右クリックでコピーは正典に無い。ukadoc 最新版の本文を grep して、クリップボードを扱うタグは 0 件。選択肢やアンカーのイベントにボタンの種類の Reference も無い＝areka 独自の拡張。
  - ホバーは正典にある＝SHIORI リソース `balloon_tooltip`（選択肢だけ）・`OnChoiceHover`・`OnAnchorHover`。キャラクター窓には tooltipブレス・SHIORI の `tooltip`・`currentghost.seriko.tooltip.*`。
- **areka の起票時の状態**:
  - バルーンから開けるのは `\![open,readme]` だけ（`script:` は警告 1 行で捨て・`\j` は読まずに捨て・`\![open,…]` の受け取り手は readme だけ）。
  - `\__q` は未実装で引受先なし（`anchor-tag-canon` の brief の「実装済み」は誤り）。
  - バルーンの右クリックは何も起きず、クリップボードとツールチップのコードは 0 件。
- **決めたこと（開発者・要件の段で覆してよい）**:
  - ⑴ 右クリックでコピーする文字列は 3 段＝`script:` の中の開く系の行き先 → ID や引数の `http(s)://`・`file:///`・`mailto:` → 表示されている文字。内部の ID はコピーしない。
  - ⑵ 見せ方は小さなメニュー（選ぶとコピー・行き先 1 つにつき 1 項目・右クリックは選択にもタイムアウトにも触れない）。
  - ⑶ ホバーは正典の 3 つを実装し、ゴーストが何も返さないときだけ行き先を出す。土台は wintf に分ける（「tooltip は色々汎用的にできた方が良い」）。
  - ⑷ `\__q` は `anchor-tag-canon` に足さず別の spec にして、その後に作る。
  - ⑸ 開く系は `script-impact-tiers` より先に入れてよい。開くたびに必ず記録する。
  - ⑹ 外部アプリは OS の既定のアプリで開く。「SSP は OS 既定のアプリが無い時代から続いてるアプリなので、いまは OS の受け口を最大限活用すべき」。
  - ⑺ キャラクター窓のツールチップも起票する（段はその他）。
  - 開発者指示（同日）「この内容を『優先度の高い機能』として取り込み、ウェーブの再検討を」＝棚卸のセッションへ依頼した。
- **既存の spec の直し**: `anchor-tag-canon`（`\__q` の誤りを直し、範囲の仕組みに後の 3 本が乗ると書いた）・`script-impact-tiers`（開く系が先に入り、開く処理の 1 か所へ同意の窓を差し込むと書いた）。

#### Specs (dependency order)

- [x] areka-P0-choice-script-prefix -- `\q[…,script:…]` の実行。Dependencies: なし
- [x] areka-P0-open-external-tags -- `\j` と `\![open,file/browser/explorer/editor/mailer]` を OS の既定のアプリで開く・開く処理の 1 か所と記録・行き先を取り出す関数。Dependencies: なし
- [x] areka-P0-wintf-tooltip -- wintf の止まったことの検出と汎用のツールチップ。Dependencies: なし
- [ ] range-choice-tag -- `\__q` の範囲の選択肢。Dependencies: areka-P0-anchor-tag-canon, areka-P0-choice-script-prefix
- [ ] link-context-copy -- 右クリックのメニューで行き先をコピー（areka 独自）。Dependencies: areka-P0-open-external-tags, areka-P0-anchor-tag-canon, range-choice-tag
- [ ] balloon-link-hover -- `balloon_tooltip`・`OnChoiceHover`・`OnAnchorHover` と既定の行き先の説明。Dependencies: areka-P0-wintf-tooltip, link-context-copy
- [ ] shell-tooltip -- キャラクター窓の tooltipブレス・SHIORI の `tooltip`・`currentghost.seriko.tooltip.*`（その他）。Dependencies: areka-P0-wintf-tooltip

### ゴーストフォルダの複数管理（2026-10-05 `/kiro-discovery` で起票）

> 開発者の依頼（2026-10-05）「ゴーストフォルダの複数管理の仕組みが欲しい。SSP は以下 URL の仕様だが、この仕様に従う必要はない。今どきの、良い感じの管理方式無いかなあ」（SSP ヘルプ `ssphelp:config-folder`）。続けて「特定のディレクトリを、特定 ID のゴースト・シェル・バルーンとしてエイリアス出来たら便利。開発中は git リポジトリを適当な場所に展開して開発用ゴーストをいじることが多いので」。

- **起票時の状態**: 根は 1 つ（`AREKA_ROOT` → exe の隣）。根を 1 つと決め打ちしている本番の箇所は約 14 ファイル・35〜40 か所。記憶の鍵（`LastGhost` ほか）はどれもフォルダ名だけ。完了 spec `baseware-root-layout` が「複数の根」を α 後へ送っていた。
- **決めたこと（開発者・要件の段で覆してよい）**:
  - ⑴ 種類ごと（ゴーストフォルダ・バルーンフォルダ）の登録ではなく、**根を並びで登録する**（例「`c:\areka` → `c:\ssp`」）。上から探して最初に見つかったものを使い、下の同じ名前は隠れる（`PATH` と同じ）。こうすると見分ける鍵はフォルダ名のままで足り、記憶の鍵を作り直さずに済む。
  - ⑵ 新しく入れるものは先頭の根へ入れる。すでにあるものの上書き・ネットワーク更新はその場で行う。インストールのたびに行き先は尋ねない。
  - ⑶ 並びは App の記憶に置く。`AREKA_ROOT` は `;` 区切りで並び全体を上書きする。メニューに「根を追加…」（OS のフォルダ選択）と「根を外す ▸」を置く。並べ替えの画面・設定画面は作らない。
  - ⑷ SSP のフォルダ設定の個別の項目（一時的な除外・フォルダごとのランダム切替や自動更新の除外など）は写さない。
  - ⑸ 開発用のエイリアス＝「名前 → フォルダ」の表を、探す順のいちばん上に置く（ゴースト・バルーン・ゴーストごとのシェル）。エイリアスの先へはインストールも更新も書き込まない（作業ツリーを守る）。登録は手で書く＋環境変数（メニューなし）。
  - ⑹ 同じゴーストを SSP と areka で同時に起動したときの SHIORI の保存ファイルの書き合いは防がず、説明書の注意書きにする。
- **分け方**: 境目は「フォルダ名 → 実際の場所」を引く関数 1 つ。前の spec がこの関数を作って呼び手を寄せ、後の spec はその手前に段を 1 枚足すだけにする。

#### Specs (dependency order)

- [x] areka-P0-baseware-root-list -- （2026-10-16 に完了）根の並び・上から探す関数と和の列挙・インストール先・`AREKA_ROOT` の `;` 区切り・メニューの根の 2 項目（その他）。Dependencies: なし
- [ ] dev-folder-alias -- ゴースト・バルーン・シェルのエイリアスを探す順の先頭に置く・エイリアスの先へは書き込まない（その他）。Dependencies: areka-P0-baseware-root-list

### balloon-lifecycle-events の持ち越し（2026-10-05 要件の討議で起票）

> 開発者裁定（2026-10-05・`balloon-lifecycle-events` の要件の討議 議題 1）「中断位置は今は作らない。起票を行え」。`balloon-lifecycle-events` は `OnBalloonBreak` の Reference2 を空で送り、互換対応表と網羅の台帳に縮退として記録する。

#### Specs (dependency order)

- [ ] balloon-break-position -- `OnBalloonBreak` の Reference2 に中断位置を入れる（台本の中の位置を字句から完了の知らせまで通す）（その他）。Dependencies: areka-P0-balloon-lifecycle-events

### element-base-method の持ち越し（2026-10-05 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `element-base-method` の要件の討議（2026-10-05・開発者「残りの描画メソッドは起票」）と完了時の棚卸（`completed/areka-P0-element-base-method/tasks.md` の Implementation Notes）。台帳 `ledger/assets.toml` の element定義の行と `base:1` の行の担当をこの spec へ移した。

#### Specs (dependency order)

- [ ] draw-methods-canon -- 残りの描画メソッドを element定義・pattern定義で描く（pattern定義の `base` を含む・D2D が支える全部へ・wintf も含めて）。Dependencies: areka-P0-element-base-method

### budoux-reveal-reflow の持ち越し（2026-10-06 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `budoux-reveal-reflow` の最終確認の点検と完了時の棚卸（`completed/areka-P0-budoux-reveal-reflow/tasks.md` の Implementation Notes）。調停役 areka棚卸 の指示で 2 本を起票した。どちらもバグではなく檻の穴。

#### Specs (dependency order)

- [x] choice-ranges-one-function -- （2026-10-31 に完了）選択肢の範囲を出す本番の手順を関数 1 つにまとめ、検査はそれを呼ぶ（優先）。Dependencies: areka-P0-budoux-reveal-reflow
- [x] areka-P0-reflow-scroll-path-test -- （2026-10-16 に完了）区間の全文で配置する枝とあふれのスクロールを組み合わせて踏む経路の検査（優先）。Dependencies: areka-P0-budoux-reveal-reflow

### ghost-standard-balloon の持ち越し（2026-10-07 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `ghost-standard-balloon` の完了時の棚卸（`completed/areka-P0-ghost-standard-balloon/tasks.md` の Implementation Notes）。棚卸のほかの 2 件は起票しない: `baseware-root-list` の brief への型の注記はその場で足し、網羅台帳の束の小分類「読む経路が無い」の古さは `coverage-roadmap-refresh` の brief へ申し送った。

#### Specs (dependency order)

- [→] areka-P0-dump-balloon-debug-timeout -- （2026-10-11 `areka-P0-mcp-receiver-bugs` の件 2 に吸収・brief は同 spec に写した）debug 版の `dump_balloon` の時間切れを再現して直す（バグ・再現しなければ記録して閉じる）。Dependencies: areka-P0-mcp-dump-images-residue
- [ ] test-roots-under-target -- テストの一時フォルダを OS の一時フォルダから `target\test-roots` の下へ移す（その他）。Dependencies: areka-P0-ghost-standard-balloon
- [x] areka-P0-dev-helper-x64-clobber -- （2026-10-16 に完了）全体テストの後に i686 の helper が x64 版で上書きされる開発時の罠を解く（その他）。Dependencies: none

### animated-image-playback の持ち越し（2026-10-07 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `animated-image-playback` の完了時の棚卸（`completed/areka-P0-animated-image-playback/tasks.md` の Implementation Notes・`research.md`「実機の確かめ」の「範囲外（要起票）」）。6 件のうち 3 件は新しい spec、3 件は既存の brief への申し送り。MCP の `sakurascript`・`raise_event` が無い件は `mcp-kanade-tools` の受け持ち（起票済み）。

#### Existing Spec Updates

- [ ] clippy-199-lints -- 列挙の外の clippy の赤 2 件（`areka-seriko` の `actor.rs` の `large_enum_variant`・`looper.rs` の `collapsible_if`）と、10-06 時点で残る列挙済みの赤を申し送った。Dependencies: none
- [ ] coverage-roadmap-refresh -- `briefing-assets.md` の「SERIKO/MAYUNA 世代別対応表」が検査の外で台帳と食い違ったままの件を申し送った。Dependencies: none
- [ ] draw-methods-canon -- 手書きの `always` のコマの `overlay` 以外の描画メソッドが描かれない件と、`plan.rs` の「非 Overlay method の現在コマ」の `warn!` が `always` の周ごとに出続ける件を申し送った。Dependencies: none

#### Specs (dependency order)

- [ ] present-emit-tail-latency -- シェルの絵の適用の 16 ms 超えの尾を、測っていない区間の計測から原因を特定して直す（バグ）。Dependencies: areka-P0-animated-image-playback
- [→] areka-P0-balloon-text-area-collapse -- （2026-10-11 `areka-P0-balloon-text-bugs` の件 2 に吸収・brief は同 spec に写した）バルーンの文字の描画範囲が潰れたときの毎フレームの供給面の失敗と記録の洪水を止める（バグ）。Dependencies: none
- [→] areka-P0-seriko-rebuild-hidden-lottery -- （2026-10-11 `seriko-trigger-lottery-gaps` の件 3 に吸収・brief は同 spec に写した）見えていない部品の抽選が回る件を、見える部品だけにするか決まりとして記録する（その他）。Dependencies: areka-P0-seriko-trigger-intervals

### wintf-tooltip の持ち越し（2026-10-08 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `wintf-tooltip` の完了時の棚卸（`completed/areka-P0-wintf-tooltip/tasks.md` の Implementation Notes）。2 件のうち 1 件は新しい spec、1 件は既存の brief の受け持ち。

#### Existing Spec Updates

- [ ] clippy-199-lints -- `cargo clippy --workspace --all-targets` が deny の水準のエラー 5 件で止まる（`areka-emo-text` のテスト 4 ファイル〔`line_pitch_readback`・`choice_fixture`・`emo2_fixture_e2e`・`decoration_readback`〕・`areka-kanade` の `actor_raise_reply_tests.rs`。clippy 1.99 の `absurd_extreme_comparisons` など）。brief の「測り直し」の範囲（`areka-emo-text` は列挙に無かった）として申し送った。Dependencies: none

#### Specs (dependency order)

- [→] areka-P0-emo2-real-run-wrap-timeout -- （2026-10-11 `areka-P0-emo2-real-run-stability` の件 1 に吸収・brief は同 spec に写した）`emo2_real_run` が `wrap=BudouxWordWrap` の待ちで赤になる件を、観測で待つ形にして直す（バグ）。Dependencies: none

### ghost-session-test-load-flake の持ち越し（2026-10-10 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `ghost-session-test-load-flake` の完了時の棚卸（`completed/areka-P0-ghost-session-test-load-flake/tasks.md` の Implementation Notes の 5.6 と「範囲外の申し送り」⑴〜⑺・`load-repro.md` の 4.5〜4.9）。新しい spec は 2 本。⑴ 既定のスレッドの数でも負荷の下で `0xc0000005` で落ちた件は `areka-test-threads-av` の brief へ、⑸ `rename_patiently` の天井（掴まれた宛先では名前替え 1 つにつき約 2 秒・UI スレッドが塞がる）は `install-live-target-hazards` の brief へ足した。⑶ `a_failed_commit_…` の一時フォルダが OS の一時フォルダの下にある件は `TempPath::new` の呼び出しで、起票済みの `test-roots-under-target` の範囲（新しく起票しない）。

#### Existing Spec Updates

- [→] areka-P0-areka-test-threads-av -- （2026-10-11 `areka-test-binary-stability` の件 1 に吸収・brief は同 spec に写した）既定のスレッドの数でも負荷の下で `0xc0000005` で落ちた観測（`load-repro.md` の 4.7・4.8・4.9）を申し送った。Dependencies: none
- [ ] install-live-target-hazards -- `areka-nar` の `rename_patiently` の天井と、brief の「`install.rs` は変わっていない」の行が古くなったことを申し送った。Dependencies: none

#### Specs (dependency order)

- [→] areka-P0-test-wait-marker-gaps -- （2026-10-11 `areka-test-binary-stability` の件 2 に吸収・brief は同 spec に写した）テストの待ちの残り（目印が UI の側の仕事を数えない待ち・`join_bounded`・`areka-nar` の短い掴みの檻・`GpuPermit::take`・目印の無い `spin_wait_until` 15 か所）を steering `tech.md` の線に揃える（バグ）。Dependencies: areka-P0-ghost-session-test-load-flake
- [x] actor-thread-log-capture -- （2026-10-31 に完了）`spawn_actor` で起こした背景のスレッドの記録を、本番の記録を変えずにテストの捕捉へ届ける（その他）。Dependencies: none

### 棚卸㉓の分割と起票（2026-10-10 `/kiro-next`）

> 出どころは棚卸㉓の再測定（各 brief の「2026-10-10 棚卸㉓の再測定」「棚卸㉓の分割」「棚卸㉓の測定」節）。分割 3 本から新しい spec 4 本、覚え書きから新しい spec 6 本。理由と残したものは「棚卸㉓の裁定」3〜5。

#### Existing Spec Updates

- [x] areka-P0-seriko-trigger-intervals -- 口パクの `talk`・`runonce`・`periodic` だけを残した（読み手の `shell/` と seriko だけを触る）。Dependencies: none
- [ ] draw-methods-canon -- 読み手が描画メソッドを運ぶこと・描けるかの門・pattern の `base`・α の 5 語・警告の洪水の停止を残した。Dependencies: element-clipping-option
- [x] areka-P0-winget-manifest-submission -- マニフェストのひな形・手元での確かめ・初回の手提出までを残した（`README.md`・`.github/workflows/` に触らない）。Dependencies: none
- [ ] script-impact-tiers -- 台本が内部の運び手の名前を直に書ける件を議題に足した。Dependencies: none
- [x] areka-P0-mcp-reload -- （2026-10-22 に完了）受け取り手の表の `"bind"` の字面の件を申し送った。Dependencies: none
- [ ] shell-tooltip, balloon-link-hover -- 主モニタより上のモニタでの位置（負の y）の実機の確かめを申し送った。Dependencies: none
- [ ] areka-P0-emily-ghost-verification, shiori4-api -- SHIORI へ渡すフォルダのパスの形（区切りの混在・末尾の区切りなし）を疑い先・議題の候補として申し送った。Dependencies: none

#### Specs (dependency order)

- [ ] seriko-script-triggers -- `yen-e`・`never`・台本の `\i[ID]`・`animation*.name` の読み取り（区分 A）。Dependencies: areka-P0-seriko-trigger-intervals, areka-P0-anchor-tag-canon
- [ ] seriko-interval-combinations -- `bind+always` など `always` との組み合わせ（区分 A）。Dependencies: areka-P0-seriko-trigger-intervals, extent-element-offset
- [ ] draw-methods-blend -- `blend-*` の描画メソッドと `-fast` の形・古い別名（区分 A）。Dependencies: draw-methods-canon
- [ ] winget-release-automation -- `winget.yml` と説明書の行。初回の手提出が winget-pkgs に取り込まれてから（区分 A）。Dependencies: areka-P0-winget-manifest-submission
- [ ] check-script-arg-checks -- 各機能の受け口の引数の判定を純粋な関数へ取り出し、`check_script` が診る（区分 C）。Dependencies: none
- [ ] config-parse-diagnostics -- 設定の読み取りが捨てた行と読まなかったキーを言えるようにする（区分 C・優先度は低い）。Dependencies: none
- [ ] font-size-keywords -- `\f[height,…]` のスタイルシートのキーワードで字の大きさを変える（区分 C）。Dependencies: none
- [ ] elevated-drop-filter -- 管理者として動かした areka へ投げ込みを届けるか、既知の制限として閉じるかを決める（区分 C）。Dependencies: none
- [x] mouse-click-wheel-events -- （2026-10-24 に完了）単押しのクリックとホイールのイベントを SHIORI へ送る（区分 C）。Dependencies: none
- [ ] inputbox-user-input -- 入力欄 `\![open,inputbox]` と `OnUserInput`・`OnUserInputCancel`（区分 C）。Dependencies: none

### タスクトレイ（2026-10-10 `/kiro-discovery` で起票）

> 出どころは開発者の指示（2026-10-10「タスクトレイ自体は対応すべき」「タスクトレイの土台仕様を起票したうえで、上流の ukadoc 項目を後続仕様にせよ」「タスクトレイで出てくるべきメニューは今出てるシステムメニューと同じ」）。予約「アプリ層」に名前だけ在った「トレイアイコン」を、土台 1 本と後続 3 本にした。同じ日に、開発者の指摘（「より上位に、アプリアイコンとタスクトレイアイコンを作成して areka.exe に埋め込むスペックを用意しないとダメ」「fal も使えるし」）で、絵を作って exe に埋める `app-icon` を土台の前提として足した。正典は ukadoc MCP で確かめた（各 brief の Current State）。

- **土台と後続に分けた理由**: 通知領域のアイコンは、ゴーストの切替で消えないアプリの持ち物として作る（今は、切替の途中に窓が 0 枚になる）。この持ち物が無いと、アイコンの差し替えも通知もアイコン化からの戻しも作れない。
- **メニュー**: 通知領域用のメニューは作らない。今の右クリックのメニュー（枠 7 つ）をそのまま出す。SSP のヘルプも、アイコンの右クリックで本体側のキャラクターのメニューが開くと書いている。
- **アイコンの動く絵は作らない方向**（開発者 10-10「今はタスクトレイは基本隠れるので、動く意味は無い」）。OS は通知領域のアイコンを動画として受け取らないので、動かすにはアプリがこまを差し替え続けることになる。`\![set,tasktrayicon,…]` の `--duration`・`--runcount` の扱いは `tasktray-ghost-icon` の議題。BTS の要望 3 件（BTS 0000390・0000416・0000418）もそこで閉じる。
- **アイコン化を足した経緯**: 名指しの項目のうち `icon.minimize,ファイル名` と `alwaystrayiconvisiblebutton.caption` は、アイコン化が無いと使いどころが無い。アイコン化そのもの（`\![set,windowstate,minimize]`・`OnWindowStateMinimize`・`OnWindowStateRestore`・`hidebutton.caption`）も持ち主が無かったので `tasktray-minimize` に入れた。
- **アイコンの絵は `app-icon` が作る**（リポジトリに `.ico` が 1 つも無い・exe に埋めているのはマニフェストだけ）。絵は要件の段で作る（開発者 10-10「絵を作るのは要件フェーズで行った方が良い」）＝要件の討議の中で fal の案を出して開発者が選ぶ。実装では、アプリ用と通知領域用の 2 種を `areka.exe` に埋める。予約「インストーラー版」のスタートメニューのアイコンも同じ絵を使う。絵柄は開発者が決める（steering に決まりが無い）。
- **ウェーブへの組み入れは次の棚卸で行う**（触るファイルの重なりをまだ測っていない）。

#### Specs (dependency order)

- [x] areka-P0-app-icon -- （2026-10-16 に完了）アプリ用と通知領域用のアイコンを作り、`areka.exe` に埋める（区分 A）。Dependencies: none
- [ ] tasktray-icon -- 通知領域にアイコンを出す土台。文言は「areka/[起動中のゴースト名]」・右クリックで今のメニュー（区分 A）。Dependencies: areka-P0-app-icon
- [ ] tasktray-ghost-icon -- descript の `icon,ファイル名` と `\![set,tasktrayicon,…]` でゴーストがアイコンと文言を決める（区分 A）。Dependencies: tasktray-icon
- [ ] tasktray-balloon -- `\![set,trayballoon,…]` で OS の通知を出し、`OnTrayBalloonClick`・`OnTrayBalloonTimeout` を送る（区分 A）。Dependencies: tasktray-icon
- [ ] tasktray-minimize -- アイコン化（最小化）と `icon.minimize`・`hidebutton.caption`・`alwaystrayiconvisiblebutton.caption`（区分 A）。Dependencies: tasktray-icon, tasktray-ghost-icon

### winget-manifest-submission の手元の確かめで見つかった件（2026-10-10 `/kiro-discovery` で起票）

> 出どころは `winget-manifest-submission` のタスク 3.6（同 spec の要件 4.6・4.8・6.3）。根拠は同 spec の `verification/winget-local-check.md` の「見つかった件と起票」の 6 行と、その行が指す節。開発者（同 spec の要件の討議・利用者の物が消えるおそれについて）「消えるなら winget は使えない。アプリをインストーラー形式にするとか、ファイル置き場を指定できるようにするとか、整備が必要」。

- **起票時の実測（0.0.2・開発機 1 台・手元のマニフェストで入れた形）**: winget の上げ直しと外し方（`--purge` を付けない）のどちらでも、利用者の物が 6 つ消えた（後から入れたゴースト 1・後から入れたバルーン 3・ゴーストの記憶 2）。areka の記憶は残った。機械の全員向け（`--scope machine`）では、ゴーストは立つが、areka の記憶は書けず、ゴーストを後から入れられなかった。外した後は、利用者の側でも機械の側でも PATH の項目が 1 件残った。
- **分け方**: 6 件を、新しい spec 2 本と、今ある spec 2 本へ割り付けた。
  - 「上げ直しと外し方で消える」「機械の全員向けでは書けない」「外した後に入れ先のフォルダが残る」は、根が同じ（利用者の物が入れ先のフォルダの中に在る）なので 1 本＝`user-data-root`。
  - 「書けないときに利用者へ伝わらない・記録の文が合わない」「書けないせいのインストールの失敗が、書庫の破損として伝わる」は、置き場を直しても、ほかの理由（読み取り専用・空きが無い・権限）で起きるので、別の 1 本＝`write-failure-notice`。
  - 0.0.2 の MCP の `sakurascript` が仮の受け口である件は、今ある `mcp-kanade-tools` の brief がすでに持つ。
  - 外した後に PATH の項目が残る件は winget の側の動きで、areka からは直せない＝説明書の既知の制限の材料として `winget-release-automation` へ渡す。
- **向き（起票のときの推し・開発者の裁定はまだ）**: 利用者の物を入れ先の外の書ける根に置き、同梱の物は exe の隣に残して根の並びの下の段にする。裁定は `user-data-root` の要件の討議で受ける。
- **採らなかった分け方**: 6 件を 1 本にまとめる＝置き場の直しは `baseware-root-list` の後ろでしか進められず、伝え方のバグがそれを待つことになる。インストーラー版（名乗り `Areka.Areka`）を起票する＝大きく、ポータブル版で入れた人は今のまま残る（「予約」の行のまま）。
- **順の決まり**: `user-data-root` が着地するまで、説明書に winget の行を載せず、winget-pkgs へ次の版を出さない（`winget-release-automation` の前提に加わる。同 spec の brief への申し送りは、2026-10-10 に `winget-manifest-submission` のタスク 5.1 が書いた。タスク 4.3 が 1 行を足した）。
- **emo2 の辞書の持ち主（pasta の側）への申し送り**（2026-10-17・`write-failure-notice` のタスク 7.1・同 spec の要件 8.2）: areka は、入れる先へ書けないせいでゴーストのインストールが止まったとき、`OnInstallFailure` の Reference0 に新しく `fileio` を載せるようになった（書庫が壊れているときは今までどおり `extraction`）。今の emo2 の辞書（`dic/install.pasta`）の `OnInstallFailure` は理由を問わず同じ台詞なので、理由ごとに台詞を分けてほしい（`fileio`＝空きが無い・読み取り専用・権限が無いなどで書けない／`extraction`＝書庫が壊れている）。辞書の元はこのリポジトリの外に在るので、areka の側では書き換えていない。分けるまでの間も、辞書の今の台詞が出るだけで、今日より悪くはならない。アプリの記憶やゴーストの記憶が書けないときの知らせは areka 自身の画面で出すので、辞書へ頼むことは無い。

#### Existing Spec Updates

- [x] areka-P0-mcp-kanade-tools -- （2026-10-19 に完了）入れた 0.0.2 の `sakurascript` が台本を受け取らない件（中身を入れる仕事は同 spec の brief がすでに持つ。brief は直していない）。Dependencies: none
- [ ] winget-release-automation -- winget で外した後に PATH の項目が残る件を、説明書の既知の制限の材料として渡す（申し送りの文は、2026-10-10 に `winget-manifest-submission` のタスク 5.1 が同 spec の brief へ書いた。タスク 4.3 が 1 行を足した）。Dependencies: areka-P0-winget-manifest-submission

#### Specs (dependency order)

- [x] areka-P0-user-data-root -- （2026-10-17 に完了）winget で入れた areka で、利用者のゴースト・バルーン・記憶が上げ直しと外し方で消えず、機械の全員向けでも書けるように、置き場を入れ先の外へ出す（区分 A）。Dependencies: areka-P0-baseware-root-list
- [x] areka-P0-write-failure-notice -- （2026-10-17 に完了）areka が書けないときに利用者へ伝え、記録の文を実際に合わせ、書けないせいのインストールの失敗を書庫の破損と見分ける（区分 B）。Dependencies: none

### anchor-tag-canon の持ち越し（2026-10-10 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `anchor-tag-canon` の完了時の棚卸（`completed/areka-P0-anchor-tag-canon/tasks.md` の Implementation Notes の 3.3・5.2・6.2・6.3・7.2 と検証の項、互換の記録 `doc/anchor-compat.md`）。新しい spec は 1 本、既存の brief への申し送りは 4 本。新しい 1 本はアンカーに固有の件ではない（下線の描画は完了 `text-decoration-canon` が入れた）。着手の席は決めていない＝次の棚卸で組む。

#### Existing Spec Updates

- [ ] anchor-style-canon -- 見た目の装着より先に届いたアンカーの字が装着前の見た目のまま残りうる件（実機では出ていない）・下線を焼いている 1 か所（`state_decoration.rs` の `push_current_style`）が `anchor.style` の解決で置き換える所であること・`\f[anchor*]` 16 行の台帳の備考の書き直し・`underline-bottom-row-clip` と同じ所を触ること・互換の記録 `doc/anchor-compat.md` を申し送った。Dependencies: areka-P0-anchor-tag-canon
- [ ] coverage-roadmap-refresh -- `shiori.toml` の未対応の行の共通の備考（`\q` の選択肢 ID に書いた場合「だけ」）がアンカーの `\_a[On…]` で正確でなくなった件・済んだ裁定の候補 W-2・裁定ではない読み 2 つ（束の付け替えと引受先の案）・調査の時点の表を申し送った。Dependencies: none
- [x] areka-P0-release-cycle -- （2026-10-11 に「次のリリースへの申し送り」の節へ移した）`dola` の `CueCommand` に 2 種類（`AnchorBegin`・`AnchorEnd`）が増え、`#[non_exhaustive]` でないので外の利用側のビルドを壊しうる件を、次の版を決めるときの材料として申し送った。Dependencies: none
- [ ] range-choice-tag -- 選択の送り先が消えたとき、送り口の `warn!` と呼び手の `error!` が同じ名前で 2 行出る件（完了 `choice-interact` からある軽い件・アンカーにも写った）を、同じ送り口を触るときのついでの直しとして申し送った。Dependencies: areka-P0-anchor-tag-canon

#### Specs (dependency order)

- [→] areka-P0-underline-bottom-row-clip -- （2026-10-11 `areka-P0-balloon-text-bugs` の件 3 に吸収・brief は同 spec に写した）文字の領域（validrect）の下端に来た行の下線が出ない件を直す（作者の下線もアンカーの下線も・直し方は未決・区分 B）。Dependencies: areka-P0-anchor-tag-canon

### seriko-trigger-intervals の持ち越し（2026-10-10 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `seriko-trigger-intervals` の完了時の棚卸（`completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 3.1・3.3・4.1・4.1-fix・4.2・5.1・5.2・8・9.2 と検証の項）。新しい spec は 5 本、既存の brief への申し送りは 7 本。実機の確かめ（9.2）の範囲外の 9 件と参考 1 件の行き先＝① MCP の `sakurascript` が中身なし → `mcp-kanade-tools`（申し送り）／② `\x` が待たず消しもしない → `talk-fast-forward`（申し送り。Implementation Notes は引受先を `balloon-canon-residue` と書いたが、`\x` は `talk-fast-forward` が引き取っている）／③ `\_q` が効かない → `sakura-time-directives`（申し送り）／④ 隠れている間の `dump_surface` が直前の絵を返す → 起票しない（完了 `mcp-dump-images` の要件 1.6 が SSP の実測に合わせて決めた振る舞い。隠れていることを読む口は `currentghost-property-tree` のプロパティの側）／⑤ cue の届く時刻の揺れ・⑨ 層ごとの起点 → `seriko-talk-clock-fidelity`／⑥ 起動時の `WARN` → `real-run-log-noise`（`null.png` の 2 件。折返し基準の 1 件は `emo-text-canon-residue` の項目 14 の件）／⑦ 1 回だけの LOAD の失敗 → `shiori-load-first-run-flake`／⑧ 面を離れた再生の停止の記録が出ない → `seriko-trigger-teardown-gaps`／参考（毎フレームの `debug!`）→ `real-run-log-noise`。新しい 5 本は、どれも実機で絵の誤りを見た件ではない（コードの読み・決定論のテスト・記録から出た）。着手の席は決めていない＝次の棚卸で組む。

#### Existing Spec Updates

- [ ] seriko-interval-combinations -- 台帳の `always:1` の持ち主が同 spec になったこと（備考の末尾の 1 文は付け替えの前のまま）・seriko の形の変化（`LoopTrigger` の 3 腕・2 段の `rebuild`・兄弟のファイル 4 つ・行数の余裕）・`+` の組み合わせは 3 語についても採っていないこと・触っていない台帳の行の古い決まり文句を申し送った。Dependencies: areka-P0-seriko-trigger-intervals, extent-element-offset
- [ ] seriko-script-triggers -- 前提の着地・`yen-e`・`never` の行の備考が古いこと（認める語は 6 つ・動く語は 8 つ）・引き金の判定の形・検体と実機の手順を使い回せることを申し送った。Dependencies: areka-P0-seriko-trigger-intervals, areka-P0-anchor-tag-canon
- [→] areka-P0-seriko-rebuild-hidden-lottery -- （2026-10-11 `seriko-trigger-lottery-gaps` の件 3 に吸収・brief は同 spec に写した）`rebuild` が 2 段になり、見えていない部品で 3 語の再生は始まらないこと（1 段目の抽選の乱数の並びは変えていない）・評価の順を変えるときに保つ約束・近い件 `seriko-lottery-ended-play-skip` を申し送った。Dependencies: areka-P0-seriko-trigger-intervals
- [ ] coverage-roadmap-refresh -- 報告の道具が行末を LF で書き出す件・項目を実装済みへ移すときに見張りが求めるものの一覧が無い件・2 つの枝が同じ誤った行数を書いて git が黙って通した件・触っていない古い記述 4 つを申し送った。Dependencies: none
- [x] areka-P0-mcp-kanade-tools -- （2026-10-19 に完了・検体の README の手順は同 spec の 6.3 で戻した）実機で `sakurascript` が `NG:not implemented yet` を返すことを確かめ、`boot.lua` の差し替えで台本を流したこと・着地したら検体の README の手順を戻せることを申し送った。Dependencies: none
- [ ] talk-fast-forward -- 実機で `\x` が待たず消しもしないこと（新しい事実は無い）・口パクが文字の現れる時刻の式を写して持つので、早送りと `\x` で時刻を変えるなら `talk.rs` も合わせることを申し送った。Dependencies: none
- [ ] sakura-time-directives -- 実機で `\_q` が効かないこと（新しい事実は無い）・cue の再生時間を変えるだけなら `talk.rs` に手は要らない見込みだが、`\_q` で区切りを 2 つ以上越える場面の実機の確かめを 1 項目足すことを申し送った。Dependencies: none

#### Specs (dependency order)

- [→] areka-P0-seriko-trigger-teardown-gaps -- （2026-10-11 `seriko-trigger-lottery-gaps` の件 1 に吸収・brief は同 spec に写した）3 語（`talk`・`runonce`・`periodic`）の引き金の後始末の食い違い（一番上の面と部品・窓の知らせだけで変わる面の番号・停止の記録）を揃え、areka の決めを記録する（バグ・区分 B）。Dependencies: areka-P0-seriko-trigger-intervals
- [ ] seriko-talk-clock-fidelity -- 口パクの時刻の写しを文字の層と食い違わないようにする（写しの見張り・起点の差・中断の直後の数え落とし・バグ・区分 B）。Dependencies: areka-P0-seriko-trigger-intervals
- [→] areka-P0-seriko-lottery-ended-play-skip -- （2026-10-11 `seriko-trigger-lottery-gaps` の件 2 に吸収・brief は同 spec に写した）終わった再生が片付く前の境界で抽選のくじを 1 回飛ばす件を、境界の時刻で測るか、決まりとして記録する（バグ・区分 B）。Dependencies: areka-P0-seriko-trigger-intervals
- [→] areka-P0-shiori-load-first-run-flake -- （2026-10-11 `areka-P0-emo2-real-run-stability` の件 2 に吸収・brief は同 spec に写した）新しい写しの emo2 の最初の起動で 1 回だけ出た SHIORI の LOAD の失敗を、再現して切り分けるか、記録して閉じる（バグ・区分 B）。Dependencies: none
- [x] real-run-log-noise -- （2026-10-30 に完了）文字の層の毎フレームの `debug!` 2 種類と、同梱の emo2 の起動時の焼き込みの `WARN` を減らす（その他・区分 C）。Dependencies: none

### 自前の既定バルーンと検体「どっとさくら」（2026-10-10 `/kiro-discovery` で起票）

> 出どころは開発者の指示（2026-10-10「ゴースト検体に『どっとさくら』を追加。さらに、これに含まれるバルーンを参考に、デフォルトバルーンを作ってほしい。バルーンについて、設定系パラメーターは引き継ぐ、設定ファイルを utf-8 に変える。online0.png などのアニメは動画 webp などで実現する。デザインをもっとシンプルにする。オンラインなどの通信アニメーションももっとシンプルにする。サイズ感はそのまま流用する。画像ファイルなどは現代的なフォーマットにする、など」）。新しい spec は 4 本（4 本目の `sample-url-fetch` は、同じ日の続きの指示「将来的には検体を URL ダウンロード方式に変えてほしいのですが、この要望もブリーフィングしておいて欲しい」）、既存の brief への申し送りは 1 本。書庫は起票のときに取り寄せて中を読んだ（1,425,534 バイト・SHA-256 `0c602c6a431dcbec48c02d934d98e342337d2db9628095e0a6f90d3e83b7a472`）。引き継ぐ寸法と設定値の表は `default-balloon-selfmade` の brief が正本。

- **3 本に分けた理由**: 検体を足す仕事（置いて登記する）、エンジンが `.webp` の名前の絵を拾う仕事（読み手）、バルーンを作って既定を差し替える仕事（資産と配布）は、触る場所も確かめ方も違う。検体は今すぐ足せる。バルーンの絵柄は開発者と選ぶ往復が要る。
- **今のエンジンで出るもの・出ないもの**: 面の絵（α 付き）と UTF-8 の descript（`charset,UTF-8`）は今のままで出る。絵の名前は `.png` だけ（`balloon-webp-names` が `.webp` を足す）。印（矢印・オンライン・SSTP）と数字はまだ 1 つも出ない（`balloon-markers` が作る）＝新しい既定バルーンのオンラインの動く印は、絵を先に入れておき、出るのは `balloon-markers` の後になる。
- **オンラインの印を 1 枚の動く絵で持つのは areka の拡張**: 正典（ukadoc の `onlinemarker.filename`）は名前に 0 からの連番を足して読み、`onlinemarker.interval` で切り替える。1 枚の動く絵の読み方は `balloon-markers` の議題に足した。
- **絵はコードで描く案を推す**（要件の段で覆してよい）: 簡素な絵柄で、寸法を 1 ピクセル単位で手本に合わせ、何度でも作り直せるため。fal で案を出す道（`app-icon` の前例）は、寸法と縁を合わせる後加工が要るので推さない。
- **検体の書庫の取り扱い（10-10 開発者が確定）**: どっとさくらの `readme.txt` は「転載・配布: インターネット上は不可」と書くが、書庫はリポジトリに入れる（開発者「どっとさくらは私の実装なので、検体として利用する分には問題ない。私のサイトで公開してます」）。areka の配布物には入れない。
- **検体を URL から取り寄せる形へ替えるのは別の spec**（`sample-url-fetch`・急がない＝開発者「将来的には」）: 上流の URL だけを頼る形は前に見送っている（上流が古いリリースを消し、固定したタグが数日で 404 になった＝`vendors/sample_ghost/README.md` の `claudia.nar` の節）。長く生きる取り寄せ先を決めるのが最初の議題。`dot_sakura` は開発者のサイトに在るので最初の 1 本にできる。既定バルーンは製品の資産なので、取り寄せの対象にしない（リポジトリが持つ）。
- **どっとさくらは華和梨のゴースト**（`shiori.dll` は 32bit・`KAWARI.kdt/7.4.`）＝検体に無かった 4 つ目の系統。動くところまで直すのは `dot-sakura-specimen` に入れていない。実機の 1 回で見えた崩れを起票する。
- **ウェーブへの組み入れは次の棚卸で行う**（触るファイルの重なりをまだ測っていない）。`default-balloon-selfmade` はカタログの鎖（`boot_resolve.rs`）と配布と公開の列（`tools/package.ps1`）の両方に当たる。

#### Existing Spec Updates

- [ ] balloon-markers -- オンラインの印を、連番でなく 1 枚の動く絵（動く WebP・APNG）でも持てるようにする議題を足した（こまの時間は絵の中の時間割りを使う・`onlinemarker.interval` との関係・名前の探し方は `balloon-webp-names` の 1 か所を通す）。新しい既定バルーンが、同 spec の求める「リポジトリ内の印の検体」（矢印・オンライン・SSTP）になる。連番 16 コマの検体は `dot_sakura` の同梱バルーン `bottle`。Dependencies: balloon-webp-names

#### Specs (dependency order)

- [ ] areka-P0-dot-sakura-specimen -- 開発者の自作ゴースト「．さくら」（華和梨・同梱バルーン `bottle`）を検体 `dot_sakura` として足し、実機で 1 回起こして崩れを起票する（区分 A）。Dependencies: none
- [ ] balloon-webp-names -- バルーンの面の絵を `.webp` の名前でも拾い、動かない WebP を Windows の拡張に頼らず読む（区分 A）。Dependencies: none
- [x] default-balloon-selfmade -- （2026-10-30 に完了）「ボトル（でふぉ改）」の寸法と設定値を引き継いだ areka 自前の既定バルーンを作り、既定を `StayseeBalloon` から差し替える（区分 A）。Dependencies: areka-P0-dot-sakura-specimen, balloon-webp-names
- [ ] areka-P0-sample-url-fetch -- 検体の書庫をリポジトリに入れず、登記表の URL とハッシュで取り寄せる形へ替える（区分 A・急がない）。Dependencies: areka-P0-dot-sakura-specimen

### バグの起票の集約（2026-10-11 `/kiro-discovery` で起票）

- **開発者の指示**: 「バグの起票が多数行われているが、一つ一つはかなり粒度が小さい。修正範囲がかぶらない範囲でいくつかに集約して欲しい」。案 A（触る場所ごとにまとめる）を開発者が選んだ。
- **やったこと**: 区分 B の brief 16 本のうち 11 本と、同じ抽選のテストを引き直す区分 C の 1 本（`seriko-rebuild-hidden-lottery`）の計 12 本を、触る場所ごとの 5 本にまとめた。12 本とも brief だけで、`spec.json`・ワークツリー・開いた PR は無かった。**元の brief の全文は、まとめ先の brief の「件 N」の節に写してある**（見出しを 1 段下げただけ）。元の brief フォルダは消した。
- **旧名の読み替え**: 完了サマリ・台帳の数の履歴・棚卸㉓の裁定（今は history「2026-10-11 棚卸㉔退避」）・history・`completed/` の記録に残る旧名は、当時の記録のまま直していない＝下の表で読み替える。下の各テーマの一覧では、吸収された行の頭を `- [→]` に替えた（未完了でも完了でもない＝`/kiro-spec-batch` は拾わない）。

| まとめ先（新しい spec） | 吸収した起票（旧名・brief の中の件の番号） | 触る場所 | 規模 |
|---|---|---|---|
| `balloon-text-bugs` | 件 1 `choice-balloon-timeout-stuck`・件 2 `balloon-text-area-collapse`・件 3 `underline-bottom-row-clip` | 文字の層（`crates/areka-emo-text`）＝文字とバルーンの列 | 11〜20 |
| `seriko-trigger-lottery-gaps` | 件 1 `seriko-trigger-teardown-gaps`・件 2 `seriko-lottery-ended-play-skip`・件 3 `seriko-rebuild-hidden-lottery`（区分 C） | `crates/areka-seriko` の `looper.rs`・`parts.rs` ほか＝シェルの element の列 | 8〜21 |
| `areka-test-binary-stability` | 件 1 `areka-test-threads-av`・件 2 `test-wait-marker-gaps`・件 3 wintf の lib の AV・件 4 ghost_switch の足場の揺れ（10-31 追記）・件 5 起動中の印のテストの並走の揺れ（10-31 `install-before-boot` の棚卸） | `cargo test -p areka --bin areka` のテストと足場＝ウェーブの外（静かな机） | 13〜20 |
| `emo2-real-run-stability` | 件 1 `emo2-real-run-wrap-timeout`・件 2 `shiori-load-first-run-flake` | `crates/areka/tests/emo2_real_run.rs` と emo2 の実機の走行 | 4〜13 |
| `mcp-receiver-bugs` | 件 1 `mcp-malformed-call-code`・件 2 `dump-balloon-debug-timeout` | `crates/areka-mcp/src/handler.rs`・`crates/areka/src/mcp/dump_balloon.rs` | 4〜12 |

- **まとめた 5 本は互いに触るファイルが重ならない**（`balloon-text-bugs` の件 1 を出し入れの側で直す形でも同じ）＝列の席が空けば並べられる。ただし `areka-test-binary-stability` は測ることが本体なのでウェーブに並べず、`emo2-real-run-stability` は実機の窓が出るので負荷の走行と同じ時間に回さない。
- **まとめずに残したバグ 5 本と理由**: `seriko-talk-clock-fidelity`（`seriko-trigger-lottery-gaps` に入れると 20 タスクの上限を超える。直し方しだいで emo-text の `state.rs` にも触り、`balloon-text-bugs` と重なる＝`seriko-trigger-lottery-gaps` の後に直列）・`extent-element-offset`（C5-④ の席が決まっている）・`dpi-realign-remembered-chain`（`placement/`）・`present-emit-tail-latency`（`presenter/show.rs`・測ることが本体）・`write-failure-notice`（`boot_resolve.rs`・`install/judge.rs`＝`baseware-root-list` と同じファイル）。後ろの 3 本は、同じ場所を触るバグの相手が居ない。
- **上限 20 を超える見込みになったとき**（要件の段で切る・先に起票しない）: `balloon-text-bugs` は件 3、`areka-test-binary-stability` は件 2 を切り出す。`seriko-trigger-lottery-gaps` は、件 2・件 3 を両方「直す」と選ぶ前に、引き直すテストの本数を数える。
- **退けた案**: 案 B（実機で確かめる 4 本を 1 本・場所がばらばらの小さい直し 3 本を 1 本＝16 本 → 6 本）。後者は配置・起動とインストール・MCP の 3 か所を 1 本で握り、`baseware-root-list` の着地まで 3 件とも着手できなくなる。前者は再現したら 20 タスクを超えかねない。

#### Specs (dependency order)

- [x] areka-P0-balloon-text-bugs -- （2026-10-16 に完了）文字の層のバグ 3 件（選択が済んだバルーンが時間切れで消えない・0 寸の描画範囲で毎フレーム失敗と記録が出続ける・validrect の下端の行の下線が出ない）を 1 本で直す（バグ・区分 B）。Dependencies: none
- [ ] seriko-trigger-lottery-gaps -- SERIKO の 3 語の引き金の後始末の食い違いと、抽選の 2 件（終わった再生の片付け前の境界でくじを飛ばす・見えない部品のくじ）を 1 本で揃える（バグ・区分 B）。Dependencies: none
- [ ] areka-test-binary-stability -- `cargo test -p areka --bin areka` が `STATUS_ACCESS_VIOLATION` で落ちる件を特定して直し、テストの待ちの残り 5 種類を steering の線に揃える（バグ・区分 B・静かな机）。Dependencies: none
- [x] areka-P0-emo2-real-run-stability -- （2026-10-23 に完了）`emo2_real_run` が `wrap=BudouxWordWrap` の待ちで赤になる件を観測で待つ形に直し、新しい写しの最初の起動で 1 回だけ出た SHIORI の LOAD の失敗の再現を確かめる（バグ・区分 B）。Dependencies: none
- [x] areka-P0-mcp-receiver-bugs -- （2026-10-18 に完了・件 2 の debug 版の `dump_balloon` の時間切れは claudia と emo2 の各 5 回で再現せず、記録して閉じた）MCP の受け口が形の違う `params` の `tools/call` に `-32601` で答える件を `-32602` に直し、debug 版の `dump_balloon` の時間切れの再現を確かめる（バグ・区分 B）。Dependencies: none

### リリースノート（2026-10-11 `/kiro-discovery` で起票）

> 出どころは開発者の指示（2026-10-11「リリース（https://github.com/ekicyou/areka/releases ）を読んでみたが、修正点の記載が多すぎて目に入ってこなかった。詳細なリリースノートに関して、Assets にドキュメントを入れてほしい。そのうえで、リリース文では特に注目に値するいくつかの内容について、日本語で紹介するようにしてほしい。spec 名は不要。」）。新しい spec は 1 本。

- **起票時の実測（main `09f4e33f`）**: `release.yml` の段「Release を公開」は `gh release create --generate-notes` で本文を GitHub に作らせている。`v0.0.2` の本文は PR の題名の一覧 263 行・34,525 字。Assets は zip と `.sha256` の 4 つだけ。`v0.0.2` より後に main へ入った変更は 35 本（機能 15・直し 3・性能 1・文書 15・雑務 1）。
- **推す形**（要件の段で覆してよい）: 紹介文は版ごとのファイルをリポジトリに置き、`release-cycle` の版上げの段で AI が下書きして版上げの PR に載せ、開発者が読んで承認する。`release.yml` はそれを本文に渡す。詳細は `release.yml` が一つ前のタグからの変更の一覧を取って 1 つのファイルにし、5 つ目の Assets として添える。紹介文が欠けていたら、PR を出す前とビルドの前で止まる（タグは動かせないので、公開の段で気付く形にしない）。
- **採らなかったもの**: 根の `CHANGELOG.md` に各 PR が 1 行ずつ足す（同時に走る枝が同じ場所を取り合う）／変更履歴の専用の道具を CI に足す（`gh` と PowerShell の数行で足りる）／自動の一覧を本文に残して折りたたむ（Assets に欲しいという求めに合わない）／workflow の中で AI に書かせる（公開の前に開発者が読めない）。
- **要件の段で決めること**: 詳細のファイルをどこまで整えるか（種類で分ける・spec の名前のかっこを落とす）／紹介の件数と長さ／詳細のファイルの形式と名前。
- **pasta とそろえた**（同じ日）: 開発者は pasta のリポジトリでも同じ spec（`release-notes-highlights`）を起票し、両者をそろえるよう pasta のセッションに指示した。名前を `release-notes` から `release-notes-highlights` に改め、brief の節の並びと議題の番号を pasta に合わせた。pasta の側で開発者が決めた 7 点（紹介の文章にする・Claudia〔令嬢〕の口調・spec 名は本文にも詳細にも載せない・詳細は Markdown で Assets に・見どころは版上げの PR でファイルとして書く・無ければ公開の前に止まる・見どころの見出しの末尾に PR の番号）は brief の表に写した。**伝え聞きなので、areka の要件の段で開発者に 1 度確かめる。** 口調は areka のセッションで開発者が決めた（2026-10-11「Claudia と名乗らせて良い。ただ、毎回出典を書くのはちょっと。Claudia をリンクにして出典 URL を入れとくくらい」）＝語り手は Claudia と名乗る・出典の断り書きは毎回は書かない・「Claudia」の名前を出典（ponapalt 氏のゴースト「悪役令嬢クローディア」）の URL へのリンクにする。
- **pasta と違う所**: 一覧のもと（areka は GitHub に作らせている）・添付の数（4 → 5）・再実行の形（areka は頭からやり直す）・乾いた走り（areka だけ）・公開済みの版（pasta は書き直さない仮定・areka は `v0.0.2` を直す＝2026-10-11 開発者「公開済みの項は『直す』で」）。
- **2026-10-11 に着地した**（ウェーブには組まず単独）。`release-cycle` の次の回（`v0.0.3`・`user-data-root` の着地待ち）より前に入った。
- **着地の後の決定（2026-10-11 開発者）**: 「外から見た動きは pasta と同じく承認なしでノンストップ。`areka-P0-release-cycle` を完了にして、`/release` を正にするのが目的」。新しい spec は立てず、`release-cycle` の最後の仕事として新しいワークツリーで行う（手順をスキルへ写す・承認の 2 点を外す・同 spec を `completed/` へ移す・`workflow.md` と `kiro-complete` の「繰り返し仕様の例外」と、この roadmap の `release-cycle` の行・ほかの spec の brief の参照を合わせる）。答え待ちの 5 点と許可ルールの前提は、`release-cycle` の brief の「ほかの spec からの申し送り」に在る。
- **移した結果（2026-10-11）**: 手順の正本は `.claude/skills/release/procedure.md`。開発者の答え 5 点＝⑴ 見どころは手順の最後の報告に全文を載せ、開発者はそこで読む（直しは公開の後に頼む）⑵ PR を出した後に main が動いても止まらず、手順を回す側が見どころへ足す ⑶ 上の「リリース」の表への記録はやめる ⑷ 走りが赤のときは今までどおり止まって聞く ⑸ 次の版への申し送りは「次のリリースへの申し送り」の節に置く。**`v0.0.3` の実走で初めて確かめる事**: 承認なしの `gh pr merge` とタグの push が自動モードの許可の判定を通るか（足すルールの文面は `procedure.md` の「開発者の設定」）・見どころを書く段・見どころの本文と 5 つの添付で公開する段。スキルは開発者が打ったときだけ動くので、試し打ちはしていない。

#### Existing Spec Updates

- [x] areka-P0-release-cycle -- 紹介文を下書きして版上げの PR に載せる段を足し（要件 3.3・4.2）、公開の後の確かめを 5 つのファイルにする（要件 6.1）。直すのは `release-notes-highlights` の実装の中（brief の「ほかの spec からの申し送り」に 1 件足した）。Dependencies: areka-P0-release-notes-highlights
- [ ] winget-release-automation -- リリースのファイルが 5 つになる申し送りを brief に足した（zip を拾う絞り込みに詳細のファイルが当たらないことを確かめる）。Dependencies: areka-P0-release-notes-highlights
- [x] areka-P0-release-cycle -- （2026-10-15 完了）手順をスキル `/release` へ移し、承認なしで最後まで進む形にして、同 spec を完了にする（2026-10-11 開発者の決定・`release-notes-highlights` の完了時の棚卸で brief へ申し送った）。Dependencies: areka-P0-release-notes-highlights

#### Specs (dependency order)

- [x] areka-P0-release-notes-highlights -- リリースの本文を注目に値する数件の日本語の紹介に替え、変更点の全部を 1 つのファイルにして Assets に添える（優先・区分 A）。Dependencies: areka-P0-release-ci-workflow

### イージング（2026-10-12 `/kiro-discovery` で起票・覚書の段）

> 出どころは開発者の指示（2026-10-12「イージング良いよね https://motion-reference.takayustudio.jp/ css イージングの範疇で、エレメントアニメーションの記述や、さくらスクリプトからエレメントにイージング指示が出せたらよいのにと思う。プロパティシステムとも連携する必要があるか？現在、ukadoc でエレメントレベルの情報を取る手段あるのかしら。まずは覚書レベルで、スペックレベルは夢よりは優先度高い程度で起票。」）。新しい spec は 1 本・既存の spec への申し送りは無い。

- **起票時の実測（main `d0c697d6`）**: ukadoc にイージングの語は無い（pattern定義は待ち時間と位置だけ・`move` は「瞬間的なもの」・台本の `--time` は形を書いていない）。式は dola の `easing.rs` に在る（名前つき 31 語とベジェ）が、CSS の `cubic-bezier`（x から解いて y を返す）とは別物で、`steps`・`linear()` は無く、連続値の側は本番に結線されていない。時間つきの台本の指令（`\![set,alpha]`・`\![set,scaling]`・`\![move]` の時間・`\![moveasync]`・`\![anim,…]`）はどれも動いておらず、持ち主も居ない。SERIKO はコマ送りだけで、pattern定義の末尾のオプション（正典の `--alpha,値`）も読んでいない。
- **開発者の問いへの答え**: ⑴ **ukadoc で element 単位の情報を取る手段は無い**。プロパティが届くのはスコープまで（`currentghost.scope(ID).surface.num`・`.animation.num`・`.rect`・`.scaling` ほか）で、台本とプロパティから指せる一番細かい単位はアニメーション ID。areka の側でも element は 1 枚に焼いた後で、実行時に指せない。⑵ **プロパティシステムとの連携は最初の段には要らない**。イージングは値の変え方の指定で、台本の指令と定義ファイルの書き方に載せれば足りる。プロパティは毎フレームの値を流す道ではない。関わるのは、変化の途中の値を読んで返す所（`currentghost-property-others`）だけ。
- **推す形**（覚書・要件の段で覆してよい）: 小さい順に ① 語彙と式を 1 か所に（dola）→ ② 台本の時間つきの指令に `--easing` を添える（窓の単位は毎フレーム書き込む口が在る）→ ③ pattern定義のコマ間をつなぐ（部品を焼かずに動かす層が要る）→ ④ element を名指しして動かす（正典に足場が無い）。着手のとき最初の 1〜2 段だけを範囲に切り、後ろの段は先に起票しない。
- **採らない方向（起票時）**: 部品ごとの独立の時計／Windows のアニメーションの仕組みに補間を任せる（時刻を握れない）／プロパティへ毎フレームの値を流す／スプリングの専用の語（CSS の範囲の外・`linear()` で近似できる）。
- **優先度**: 開発者の指定「夢よりは優先度高い程度」＝その他・区分 C の最後（据え置きの手前）。**ウェーブへの組み入れは次の棚卸で行う**。議題 6 点（土台の持ち主・最初の使い手・カンマの衝突・名前の語彙・element の指し方・回転）は brief。

#### Specs (dependency order)

- [ ] element-animation-easing -- CSS の範囲のイージングを、pattern定義のコマ間と台本の時間つきの指令に添えられるようにする（覚書の段・その他・区分 C）。Dependencies: none

### スナップ（2026-10-15 `/kiro-discovery` で起票・夢の段・覚書）

> 出どころは開発者の指示（2026-10-15「夢仕様だけど起票お願い」）。マスコットがデスクトップの床やウィンドウに吸着する「スナップ API」を、シェルの定義（宣言的な定義）だけで書けるようにする。吸着の関係（床に立つ・窓の上に立つ・窓の下にぶら下がる・壁にへばりつく／登る・陰から顔だけ出す）を本体の側でコンテキストとして抽象化してシェルへ渡し、シェルは座標を計算しない。将来はドラッグ中・吸着の先が動く間のしがみつき・閉じた／最小化したときの自由落下・複数のマスコットの積み重なり。新しい spec は 1 本・既存の spec への申し送りは無い。

- **起票時の実測（main `574bf0a6`）**: 吸着の芯は在る＝`placement/resolver.rs` の `Anchor { Top, Bottom, Left, Right, Free }`（`seriko.alignmenttodesktop` の 4 段の優先）・部品 `Anchored`・`project_anchor`（ドラッグ・大きさの変化・吸着の変更で当て直す）。相手は作業領域の辺だけ。ほかのアプリの窓を見張るコード（`EnumWindows`・`SetWinEventHook`）と物理（重力・速さ）は無い。`\![set,alignmenttodesktop]`・`\![set,sticky-window]`・`\![moveasync]`・`\4`・`\5` は absent、`\![move]` の時間は即時。3 人目からの窓は無い。シェルの定義は ukadoc のテキスト形式だけで YAML は無い。
- **台帳の古い行（起票時に見つけた）**: 網羅台帳 `assets.toml` は descript の `seriko.alignmenttodesktop` の 8 行を absent と書くが、コードは読んで使っている。直すのは `coverage-roadmap-refresh` か本 spec の着手のとき。
- **推す形**（覚書・要件の段で覆してよい）: 小さい順に ① コンテキストの語彙と渡し方（相手はデスクトップだけ）→ ② ほかの窓を見張る（WinEvent・DWM の見た目の枠）→ ③ 窓に吸着・解除して床へ → ④ 落ちる・投げる（経過時間の純粋な関数）→ ⑤ 窓の陰から覗く（重なりの順）→ ⑥ 積み重なり。着手のとき最初の 1〜2 段だけを範囲に切り、後ろの段は先に起票しない。
- **採らない方向（起票時）**: 物理エンジンのクレート／ほかのアプリの窓を動かす・閉じる／吸着の先のポーリング／窓の中身を読む。
- **優先度**: 開発者の指定「夢仕様」＝据え置き・区分 D。**ウェーブには組まない**（開発者が望んだときに着手）。議題 7 点（土台の持ち主・コンテキストの渡し方・シェルの定義の形〔YAML か ukadoc の形式か〕・吸着の先の選び方・相手にする窓・覗きの重なり・複数のモニタ）は brief。土台の持ち主（議題 1）は `element-animation-easing` の議題 1 と同じ根なので、着手するなら一緒に決める。

#### Specs (dependency order)

- [ ] window-snap -- マスコットが床・ほかの窓・壁に吸着し、その関係をコンテキストとしてシェルへ渡す（夢の段・覚書・据え置き・区分 D）。Dependencies: none

### 時刻の土台（2026-10-15 `/kiro-discovery` で起票・低優先度）

> 出どころは開発者の指示（2026-10-15「低優先度の起票。dola の時刻精度を、f64 の秒から、QueryUnbiasedInterruptTimePrecise を起点とした 100ns の u64 に変更し、プロセス間で完全同期アニメーションを行うためのベースを作成する。関連する時刻情報について、すべて、QueryUnbiasedInterruptTimePrecise から取得することを前提とし、必要な spec を起票せよ」）。新しい spec は 3 本・既存の spec への申し送りは 1 本。

- **起票時の実測（main `574bf0a6`）**: dola の時刻は全部素の `f64` の秒で、型は無い。時計は `dola::runtime::clock::now()`（QPC÷QPF）の 1 関数で、wintf の `FrameTime` と areka の `TalkClock` が読む。ghost の刻み・kanade・seriko は `GetTickCount64` のミリ秒（分解能 10〜16 ms）。台本の起点（`CueSheet` の `absolute_start_time`）は「台詞の最初の刻み＝ほぼ 0 秒」で絶対時刻ではなく、文字の層と seriko は、それぞれ自分に cue が届いた時刻から、別々の時計で起点を見積もり直している（dola の「起点は配る時に押す」約束が実装で守られていない）。そのほかの「今」は `std::time::Instant`。時刻はまだどれもプロセスの外へ出ていない。`QueryUnbiasedInterruptTimePrecise` は `windows` 0.62.2 の `Win32::System::WindowsProgramming` に在り、外の crate を足さずに呼べる。
- **切り方**: ① dola の型と時計（土台・ほかの 2 本の前提）② 台詞の時計と起点（ghost・kanade・seriko＝ここで「同じ時計で今を読めば別のプロセスでも同じ時刻に鳴らせる」が成り立つ）③ 残りの「今」と番人（`Instant` を割り込み時刻へ・別の時計を clippy で止める）。② と ③ は触るファイルが重ならない見込みで、① の後なら並べられる。
- **範囲の読み**: 「関連する時刻情報」は、アニメ・台詞・刻み・締め切り・入力・所要時間の「今」と読んだ。**暦の時刻（ログの日時・`GetLocalTime`・日付のイベント・作業フォルダの名前・プロパティの日時）は外した**＝割り込み時刻は「起動からの経過」で、日付を持たないため。別のプロセスで実際に同期して鳴らす仕事は、使い手が現れたときに起票する。
- **決まりとして書くこと**: `QueryUnbiasedInterruptTimePrecise` は眠り（スリープ・休止）の間を数えない。復帰の瞬間にアニメが飛ばず、締め切りも眠りの間は進まない。`OnSecondChange` の OS の連続起動時間だけは、今の値（眠りの間も数える）と食い違う＝`talk-anchor-interrupt-ticks` の議題 1。
- **優先度**: 開発者の指定「低優先度」＝3 本とも据え置き・区分 D。**ウェーブへの組み入れは次の棚卸で行う**。

#### Existing Spec Updates

- [ ] seriko-talk-clock-fidelity -- brief に「ほかの spec からの申し送り」を足した（時計を揃える形を選ぶなら、揃える先は QPC でなく割り込み時刻にする・起点を 1 か所で見積もる形を選ぶなら、`talk-anchor-interrupt-ticks` で「押された起点を読む」に置き換わる前提で置き場を選ぶ）。Dependencies: none

#### Specs (dependency order)

- [ ] dola-interrupt-ticks -- dola の時刻を `QueryUnbiasedInterruptTimePrecise` の 100ns の `u64` の型へ替え、「今」を読む関数をその 1 本にする（据え置き・区分 D）。Dependencies: none
- [ ] talk-anchor-interrupt-ticks -- ghost・kanade・seriko の時計を割り込み時刻へ替え、台本の起点を配る時に本物の絶対時刻で押して、表示の層が読む形にする（据え置き・区分 D）。Dependencies: dola-interrupt-ticks, seriko-talk-clock-fidelity
- [ ] instant-to-interrupt-ticks -- 残りの「今」の読み取りを割り込み時刻の時計へ替え、別の時計を clippy の番人で止める（据え置き・区分 D）。Dependencies: dola-interrupt-ticks

### 合成の精度（2026-10-15 `/kiro-discovery` で起票・低優先度）

> 出どころは開発者の指示（2026-10-15「低優先度で起票。wintfで利用しているスワップチェーンあるいはオフラインバッファについて、u8からf16精度のバッファにして、合成精度を強化したい。」）。新しい spec は 1 本（`compose-f16-precision`）。

- **起票時の実測（main `574bf0a6`）**: 8 ビット（B8G8R8A8・α 掛け済み）で固定しているのは、wintf の供給面の作り手（`com/dxgi.rs`・本番の使い手はバルーンの文字の層だけ）・wintf の描画面（`ecs/graphics/systems/surface.rs`・シェルとバルーンの枠はこの道）・文字の層のテクスチャと D2D の描き先・表示の段の D2D の絵・画像の読み込み（WIC で 32bppPBGRA へ）。合成は CPU の整数の計算（`areka-emo-compose/src/blit.rs`・決定論の檻のためにわざと浮動小数を入れていない）で、部品を 1 枚重ねるたびに 8 ビットへ丸める。
- **見立て**: スワップチェーンと描画面だけを f16 にしても、シェルとバルーンの絵は GPU に届いた時点で 8 ビットに丸め済みなので、効くのは文字の層の描画と k≠1 の拡大縮小の 1 回だけ。f16 の供給面は DWM が線形の色空間（scRGB）として読むのが既定で、今の sRGB の値をそのまま入れると色が変わる（着手のとき最初に確かめる）。
- **推す形**（要件の段で覆してよい）: 「丸めが積もる所から順に精度を上げる」に読み替える。① 先に測る（差が見えなければ取り下げ）→ ② 合成の精度は `draw-methods-canon` の「CPU か D2D か」と一緒に決める → ③ wintf の入れ物の形式を引数に（既定は今のまま）→ ④ 読み戻しは 8 ビットへ変換して返す（MCP の `dump_balloon` と読み戻しのテストを守る）。
- **採らない方向（起票時）**: 入れ物の形式だけを替えて終える／HDR の表示／当たり判定を f16 の α で行う。
- **優先度**: 開発者の指定「低優先度」＝据え置き・区分 D（保留の手前）。議題 4 点（目的の読み替え・色空間・先に測るか・既定）は brief。

#### Specs (dependency order)

- [ ] compose-f16-precision -- 途中の入れ物を f16 にして合成の精度を上げる（低優先度・据え置き・区分 D）。Dependencies: wintf-d2d-composite-rules, draw-methods-canon（10-15 の続きで合成は D2D の道に決まった＝下の「D2D の合成と当たり判定の ID」）

### D2D の合成と当たり判定の ID（2026-10-15 `/kiro-discovery` の続きで起票）

> 出どころは開発者の指示（2026-10-15「今作ってもらった仕様と関連して、D2D合成アルゴリズムの改善も入れる予定なのです。wintfではコマンドリストの合成ルールが現在固定ですが、これをD2D合成ルールを個別に指定できるようにして、高度な合成を実現したいと考えている。ukadocを読んで、サーフェス合成ルールに関する仕様も確認せよ。」）と、続く 2 つの返答（「決定論のテスト→画像合成に関するテストは、CPUに切り替えるべきだと思う。当たり判定→座標さえわかれば、１ピクセルだけGPUから書き戻すようなことはできないか？」「シェル仕様で、Headなどのリージョン範囲を、画像で取り込む仕様があるはずです。これに対応するためにもヒットテストIDが欲しいと考えました」）。新しい spec は 2 本（`wintf-d2d-composite-rules`・`hit-id-pass`）、書き直しは 4 本（`draw-methods-canon`・`draw-methods-blend`・`collisionex-regions`・`compose-f16-precision` の brief の末尾に「2026-10-15」の節）。

- **決めたこと**: ⑴ `draw-methods-canon` の議題 1（CPU の整数の道か Direct2D の道か）は **Direct2D の道**。⑵ 合成の答え（画素）の決定論のテストは D2D を CPU の描き手（WARP）で動かして固定する。実 GPU のテストは「画面に出るか」の確かめに残す（「GPU のテストは実 GPU」〔10-08〕は揺れるテストの手当ての決まりで、これとは別）。⑶ クリックの透過の α は、自前の合成の入れ物から 1 ピクセルをその場で読み戻す（前のフレームの値を使い回さない）。⑷ 当たり判定の ID は、仕組みを wintf・意味を areka に分ける。ID の幅は選べない形（起票時の見立て＝24 ビット）。
- **ukadoc の合成の規則（`descript_shell_surfaces`）**: 重ね方の 7 語（`base`・`overlay`・`overlay-fast`・`interpolate`・`replace`・`asis`・`reduce`）・`blend-*` 28 種（式は無い）・`-fast` 付き 27・古い別名 2・element定義のオプション（`--clipping`・`--alpha`・`--source`・`--scaling`）・`collisionex` の `region`（画像の指定色の領域・SSP 2.5.19）。D2D との対応の表は `wintf-d2d-composite-rules` の brief（`blend-*` は 25 種が `D2D1_BLEND_MODE` に当たり、`blend-dither`・`blend-add-glow`・`blend-color-dodge-glow` の 3 種は自前の効果）。
- **起票時の実測（main `574bf0a6`）**: 固定は `crates/wintf/src/ecs/graphics/systems/render.rs` の `DrawImage(…, LINEAR, SOURCE_OVER)` の 1 か所。`ecs/visual/draw_content.rs` の `VisualDrawContent` は `composite_mode` の欄を持つが、作りかけの描画の木（`allow(dead_code)`）の中で本番は読んでいない。wintf には CPU の色分け画像の当たり判定 `ColorMapData`（ID 8 ビット・完了 `event-hit-test-named-regions`）が既に在り、areka は使っていない。
- **順番**: `wintf-d2d-composite-rules`（wintf だけ＝シェルの element の列の読み手の鎖と重ならない）→ `draw-methods-canon` → `draw-methods-blend`。`collisionex-regions`（`region` の行も型に運ぶ）→ `hit-id-pass`（`wintf-d2d-composite-rules` の読み戻しの口も使う）。`compose-f16-precision` は `wintf-d2d-composite-rules` の自前の入れ物の後（据え置きのまま）。
- **優先度**: 新しい 2 本はどちらも開発者が起票を頼んだもの＝優先・区分 A。**ウェーブへの組み入れは次の棚卸で行う**。議題は各 brief。

#### Specs (dependency order)

- [ ] wintf-d2d-composite-rules -- wintf で描くものごとに D2D の合成の規則を指定でき、自前の合成の入れ物から 1 ピクセルを読み戻せるようにする（優先・区分 A）。Dependencies: wintf-visual-tree（2026-10-21 に none から改めた＝「ビジュアル層の取り込み」）
- [ ] hit-id-pass -- `collisionex` の `region` を読み、当たり判定を「ID の描き先の 1 ピクセルを読む」形にする（優先・区分 A）。Dependencies: collisionex-regions, wintf-d2d-composite-rules

### 版の段と複数ゴースト（2026-10-15 `/kiro-discovery` で起票）

> 出どころは開発者の優先度の整理（「版の段」の節が正本）。v0.1.0 の範囲を聞き取ったときに、開発者「ゴースト切り替えができること。これはできてる。複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」。予約の「多重ゴースト」に brief が無かったので起票した。

- **起票のときの調べ（main `46123b00`）**: kanade・SHIORI（ゴーストごとの 32bit の中継）・sylphya・seriko は、もう起こすたびにゴーストごとに作られる。止めているのは areka 本体の「1 つだけ」の置き場（`ghost_session::GhostSlot`・結線の NonSend・`GhostWindows` と窓の印・送り主の無い `KanadeNotice`・`BootContext.current`・App の範囲の `LastRunning`／`LastGhost`）と、どの止まり方も `app_exit::quit_app` へ流れること。触る見込みのファイルのうち 7 つが 870〜956 行。詳細は 2 本の brief の Current State。
- **分け方**: 1 本では 20 タスクを確実に超えるので、働きの切れ目で 2 本に分けた。土台（席の集まり・ゴーストの区別・1 体の終了とすべて終了・記憶の並び）＝`multi-ghost-runtime`、呼ぶ働き（メニュー・`\![call,ghost]`・呼ぶ／呼ばれる／周りの 6 イベント）＝`ghost-call`。
- **後へ回したもの**: ゴースト同士の掛け合い（`OnOtherGhostTalk`・`\![set,otherghosttalk]`・`\![raiseother]`）は v0.3.0 で起票する覚え書き。消滅（`OnOtherGhostVanished`）は予約のまま。
- **採らなかった形**: ゴーストごとにプロセスや wintf の World を分ける（窓・重なりの順・DPI の追従を 1 つの World で揃えている作りを捨てることになる）。
- **ほかに決めたこと（同じ聞き取り）**: `seriko-talk-clock-fidelity` は v0.2.0 の時刻の土台の後へ（brief どうしが「直し方は `talk-anchor-interrupt-ticks` が引き取る」と申し送っている）。タスクトレイの 4 本は windows-reactor 待ち（brief の頭に書いた）。

#### Specs (dependency order)

- [ ] multi-ghost-runtime -- 2 体以上のゴーストを同時に動かす土台（席の集まり・窓と結線と知らせのゴーストの区別・1 体の終了と `OnCloseAll`・起動中の印と記憶の並び・`get_active_ghost_list` の全員）（優先・区分 A・v0.1.0）。Dependencies: areka-P0-baseware-root-list, areka-P0-user-data-root
- [ ] ghost-call -- メニューの「他のゴーストを呼ぶ」と `\![call,ghost]`、呼ぶ／呼ばれる／周りの 6 イベント（優先・区分 A・v0.1.0）。Dependencies: multi-ghost-runtime

### balloon-text-bugs の持ち越し（2026-10-16 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `balloon-text-bugs` の完了時の棚卸（`completed/areka-P0-balloon-text-bugs/tasks.md` の Implementation Notes の 2.2・2.6・4.6・範囲外の観察・5.5 の範囲外・最終検証の申し送り ⑷ と、`real-machine-signoff.md` の 4 章・7.4）。新しい spec は 2 本。その場で片づけた件（後続の brief 3 本への申し送り・design のファイル一覧・clippy の件は `clippy-199-lints` が持つ）は同 spec の Implementation Notes。着手の席は決めていない＝次の棚卸で組む。

- **まとめ方**: 文字の層の小さい拾い残し 3 件（差し込み口の付け替え・右から左の下線の檻・箱の押下の記録）は、触る場所が件 1（最下行の字の欠け）と同じ文字の層なので、1 本の「件 2〜4」にまとめた（小さいバグの起票は触る場所が同じものを 1 本にまとめる決まり）。実機の観察 2 件は、触る場所がまだ分からず（wintf の窓の手続きか配置の追従か）、再現が本体なので別の 1 本にした。
- **版**: `bottom-row-text-clip` は致命的でない新しいバグ＝次の版（v0.1.0）。`real-run-input-guard-observations` は再現が本体＝版に縛らない（測ること）。
- **件 1 の守る決まり**: 面は validrect より広げない（09-05 の裁定）・下線の有無でレイアウトを変えない（2026-10-16 `balloon-text-bugs` 議題 3）。直し方は要件の討議で決める。

#### Specs (dependency order)

- [ ] bottom-row-text-clip -- 最下行の字の下の部分（`g`・`y`）が文字の面の下端で欠ける件を、面を広げずに直す（直し方は要件で選ぶ）。同じ文字の層の拾い残し 3 件（差し込み口の付け替え・右から左の下線の檻・箱の押下の記録）も片づける（バグ・区分 B・v0.1.0）。Dependencies: areka-P0-balloon-text-bugs
- [ ] real-run-input-guard-observations -- 実機で 1 回だけ見た 2 件（移動と押下を同時に送ると記録に残らない・操作していない時間の引き戻しの警告 565 行）を再現して切り分け、再現しなければ記録して閉じる（その他・区分 C・版に縛らない）。Dependencies: none

### user-data-root の持ち越し（2026-10-17 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `user-data-root` の完了時の棚卸（`completed/areka-P0-user-data-root/tasks.md` の Implementation Notes の「確かめていない件」〔Microsoft Store 版の Claude Desktop〕と「4.2 の写しの答えは起床の旗を立てず…背景の結果の全般に旗を立てるかは範囲外の議題」・タスク 4.2 の点検）。新しい spec は 2 本。着手の席は決めていない＝次の棚卸で組む。

- **その場で済ませた件**: 説明書 `dist/README.txt` の中継の控えの段落に、Microsoft Store 版の Claude Desktop が起こした中継の `%LOCALAPPDATA%` への書き込みは `LocalCache\Local\` の下へ回される（例 `LocalCache\Local\areka\profile\areka\mcp-bridge\`）と書いた（コミット `c28dd71e`）。今の振る舞いを正しく伝える文で、直すかどうかは `bridge-store-user-root` が決める。
- **分け方**: 中継の置き場（`crates/areka-mcp-bridge/src/config.rs`・実機の測りが本体の入口）と、背景の結果の起床の旗（`crates/areka/src/emo2_boot/` の送り口と `tick_gate_config_producers_tests.rs`）は触る場所も直し方の議題も重ならないので 2 本にした。
- **版**: `bridge-store-user-root` は致命的でない新しいバグ＝次の版（v0.1.0）。areka 本体の記憶は正しい場所にあり、外れるのは Store 版の利用者の中継の控えと、箱の中にできうる置き場の写しだけ。`background-result-tick-wake` はバグでない持ち越し（区分 C）で、今の既定（tick の門は切）では遅れは出ない＝版に縛らない（道具と手入れ）。門を既定で入れるかを決める `tick-gate-adoption`（v0.2.0・保留）の前に着地させる。
- **直列の約束**: `background-result-tick-wake` は、旗の種類を wintf の `tick_wake.rs` に足す形を選んだら `instant-to-interrupt-ticks`（同じファイル）と直列。`tick-gate-adoption` とは同じ時に走らせない（門と生産者の見張りを触る・計測を汚す）。v0.2.0 の間に挟むのは、この 2 本が走っておらず、`tick_wake.rs` に触らない形のときだけ。`switch_assets.rs` を触るので `emo2_boot` の結線の列（`extra-character-windows`・`mcp-reload`）とも同じウェーブに置かない。
- **守る決まり**: 「時刻は正確に扱う」＝心拍を短くして遅れを丸める案は出さない。`bridge-store-user-root` の実機の測りは、開発者の本物の `%LOCALAPPDATA%\areka` と箱の中の控えを上書きしない（測る前に控えを取り、後で戻す）。Claude Desktop の起動し直しは開発者に頼む。

#### Specs (dependency order)

- [x] bridge-store-user-root -- （2026-10-30 に完了）Microsoft Store 版の Claude Desktop が起こした中継が、アプリの箱（`LocalCache`）の中に利用者ごとの置き場を作り記憶を写しうる件を実機で測り、中継は置き場を決めるだけにする（作る・写すは areka 本体へ）か、箱の中に置く決まりを受け入れて説明書を確定する（バグ・区分 B・v0.1.0）。Dependencies: areka-P0-user-data-root
- [ ] background-result-tick-wake -- 背景のスレッドの結果（切り替えの写し・切り替えの資産の組み立て・kanade の切り替えの知らせほか）を送るときに起床の旗を立て、tick の門を入れた形でも心拍（30 コマ）を待たずに拾う。旗を立てるファイルを `AREKA_PRODUCERS` に載せる（その他・区分 C・版に縛らない・`tick-gate-adoption` の前）。Dependencies: areka-P0-user-data-root

### 表示倍率（2026-10-17 `/kiro-discovery` で起票）

> 出どころ（2026-10-17 開発者）: 「wintf や dola の改造が終わった後のフェーズのネタ。現在、シェルやバルーンの倍率は、DPI 追従だけで倍率が決まってるが、更に拡大率を設定できるようにしてほしい。例えば、DPI 200%、拡大率 200% だったら、計 400% で描画されることになる。」

- **決めたこと**（同日の答え）: 設定する場所＝設定画面を待つ（windows-reactor 待ち・それまでは `sylphya.toml` を手で書く形だけ・右クリックメニューの段は作らない）／覚える単位＝ゴーストごと（SSP の「ゴーストごとの設定」の「シェル倍率」と同じ）／バルーン＝「バルーンも連携する」の切り替え付き（既定は連携）／版＝v0.4.0。
- **見つけたこと**: 掛け算の口 `ScalePolicy.app_scale`（`areka-emo-present/src/scale.rs`）が 1 に固定で在り、wintf の変換・当たり判定・文字の層・`\![move]` は k を読み戻すので自動で追う。DPI だけから k を計算し直す `placement` の 3 か所（`build_measure_scaling`・`reseed.rs`・`rescale_follow_offset`）は同じ倍率にそろえる要がある。
- **呼び名**: このプロジェクトで「拡大率」は Windows の表示スケール（DPI）を指してきた＝利用者が決めるほうは「表示倍率」と呼ぶ。
- **範囲の外**: `\![set,scaling]`（覚え書きの D 群のまま）・描画メソッド `scaling` と `--scaling`・`OnShellScaling`／`OnBalloonScaling`・プロパティ `scope.scaling`（`currentghost-property-tree`）。

#### Specs (dependency order)

- [ ] shell-balloon-scale-setting -- ゴーストごとの表示倍率と「バルーンも連携する」を覚え、描く倍率を「表示倍率 × DPI の倍率」にする。v0.4.0。Dependencies: v0.2.0 の wintf の整備（`wintf-d2d-composite-rules`・`draw-methods-canon`・`compose-f16-precision`）

### 棚卸㉗の分割と起票（2026-10-18 `/kiro-next`・v0.1.0 の組み直し）

> 出どころ（2026-10-18 開発者）: 「0.1.0のスコープを決定しました。ゴールは、SSP並みの自動試験が出来るようにすること、とします。SSPなみの起動オプション、MCPサポートの安定化、kiro-watchの機能追加など、開発がスピードアップすることを第一目標とします。もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です。検体の相談は「ぱすた開発」セッションに問い合わせてください。」・「watchコマンドの機能拡充は優先度高いので最初のウェーブにしてください。」

- **検体**: 「ぱすた開発」のセッションの答え＝ghost_dev の `pasta-in-windows`（表示名「窓際のぱすたさん」・最新のリリースのコミット `e1e0559`・書庫は約 3.0MB）。SHIORI は pasta.dll v0.4.0（emo2 と同じ x86）。ゴーストの側は起動オプションも MCP も使わないが、開発の確かめは SSP の MCP を手で使っている（`reload`・`raise_event`・`sakurascript` の strict・`get_log`・`dump_surface`・`get_property`・`get_status`）＝areka が SSP 並みになれば、ぱすたの確かめにも areka を使いたい（同セッション）。
- **分割**: 無し。
- **起票の分け方**: 起動オプションは触る場所（新しい `cli/`）を分け合う 4 本（土台・オフラインの面の書き出し・読み取り専用・インストール）。ぱすたの差は直す場所ごと（重なりの順・最前面・小さなタグ・descript の窓の鍵・メニュー・マウスカーソル）に 1 本ずつ。α のしきい値の 1 行は検体の spec の中で直す。

#### Existing Spec Updates

- [x] areka-P0-mcp-kanade-tools -- （2026-10-19 に完了）再生した台本を全部 `get_log` の `script` へ出す（kanade の `actor.rs` の 1 か所）。Dependencies: なし
- [x] mouse-click-wheel-events -- （2026-10-24 に完了）v0.3.0 から v0.1.0 へ。`OnMouseEnter`／`OnMouseLeave` を範囲に入れた（ぱすたの閉じる印を赤くする）。Dependencies: areka-P0-mcp-kanade-tools（kanade のファイル）
- [x] areka-P0-ghost-inner-balloon -- （2026-10-23 に完了）版に縛らない列から v0.1.0 へ。ゴーストのフォルダの直下に名前で置いたバルーンも探す（SSP 2.9.18 で測ってから）。Dependencies: areka-P0-startup-options-core（ファイル）
- [x] areka-P0-sample-url-fetch -- （2026-10-19 に完了）版に縛らない列から v0.1.0 へ。ぱすたを URL とハッシュの行で登記するため。Dependencies: なし
- [ ] shell-balloon-scale-setting -- `balloon.syncscale` を範囲に入れた（v0.4.0 のまま）。Dependencies: v0.2.0 の wintf の整備

#### Specs (dependency order)

- [x] areka-P0-startup-options-core -- （2026-10-19 に完了）SSP の起動オプションの読み取りの土台（`--ghost`・`--random`・`nobootcheck`・`--sstp-listen`・`--help` ほか）。v0.1.0。Dependencies: なし
- [x] areka-P0-offline-surface-dump -- （2026-10-23 に完了）`--offline-dump` とその仲間で、窓も GPU も使わずに面を PNG に書き出してすぐ終わる。v0.1.0。Dependencies: areka-P0-startup-options-core
- [x] readonly-mode -- （2026-10-24 に完了）`--option readonly` で利用者の環境へ何も書かずに起動する。v0.1.0。Dependencies: areka-P0-startup-options-core
- [x] startup-install-option -- （2026-10-30 に完了）`--install`・`callghost`・`deletesource`・`initial_install.nar`。v0.1.0。Dependencies: areka-P0-startup-options-core, readonly-mode
- [x] areka-P0-real-run-mcp-kit -- （2026-10-19 に完了）実機の走行のテストの共通の道具（立てる・MCP を呼ぶ・出力を読む・閉じる）。v0.1.0。Dependencies: なし
- [x] areka-P0-pasta-in-windows-verification -- （2026-10-23 に完了）検体「窓際のぱすたさん」を URL で登記し、SSP 2.9.18 と同じに動くかを確かめて崩れを直す。v0.1.0。Dependencies: areka-P0-sample-url-fetch
- [x] areka-P0-zorder-scope-inversion -- （2026-10-19 に完了）キャラの窓を自分のバルーンより手前に置く重なりの順（`\![set,zorder,s0,b0,s1]`）。v0.1.0。Dependencies: なし（要件の段で開発者の裁定）
- [x] stay-on-top-directives -- （2026-10-31 に完了）`\v`・`\![set,windowstate,stayontop]`。v0.1.0。Dependencies: areka-P0-zorder-scope-inversion
- [x] script-small-tags -- `\_u`・`\_m`・`\![bind-noevent]`・`\![*]`。v0.1.0。Dependencies: なし
- [x] descript-window-keys -- （2026-10-24 に完了）`seriko.defaultsurface`・`balloon.dontmove`・`\![lock/unlock,balloonmove]`。v0.1.0。Dependencies: なし
- [ ] popup-menu-shell-look -- シェルの `menu.*` のオーナードローと着せ替えのメニュー。v0.1.0。Dependencies: 開発者の裁定（生きている決まり 6）
- [x] shell-mouse-cursor -- （2026-10-30 に完了）シェルの `sakura.cursor` ほかで当たりの上のマウスカーソルを替える。v0.1.0。Dependencies: mouse-click-wheel-events

### mcp-kanade-tools の持ち越し（2026-10-19 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `mcp-kanade-tools` の完了時の棚卸（`completed/areka-P0-mcp-kanade-tools/tasks.md` の Implementation Notes の 5.2・6.3 と、`verification/signoff.md` の「気付いたこと（本 spec の範囲の外）」）。新しい spec は 1 本、既存の brief への追記が 1 件。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: トークのスレッドの数が増える件は、触る場所（スレッドの名簿と報告器、残っていればトークのスレッドの後始末）を持つ brief が無いので新しい 1 本にした。揺れたテスト（`check_script_tests.rs` の `omitted_or_matching_name_answers_with_the_running_ghost`）は、同じファイルを触る `check-script-arg-checks` の件 2 へまとめた（小さいバグの起票は触る場所が同じものを 1 本にまとめる決まり）。
- **見立ての訂正**: 実機の確かめは「トークのスレッドが残る」と書いたが、報告の `threads=` はスレッド名簿の項目の数で、名簿は終わったスレッドの項目を持ち続ける作り。生きたスレッドが残っているかは、最初のタスクで OS から数えて切り分ける。
- **版**: `talk-thread-count-growth` は致命的でない新しいバグ＝次の版（v0.2.0）。数日起動したままでも、増えるのはトークの数だけの項目とハンドル（生きたスレッドなら 1 本ずつ）で、すぐに止まりはしない。`check-script-arg-checks` は「いつか」のまま（件 2 はテストの揺れで、本番の振る舞いは正しい）。
- **直列の約束**: トークの側（`drive.rs`・`dispatcher.rs`）を直すことになったら `talk-anchor-interrupt-ticks`・`talk-fast-forward` と同じウェーブに置かない。`perf_thread_report.rs` は `instant-to-interrupt-ticks` と同じウェーブに置かない。

#### Existing Spec Updates

- [ ] check-script-arg-checks -- 件 2: 別のスレッドの答えを待つテストが、答えが早く届くと「その場の答え」の側に入って赤になる揺れを、届いた時刻でなく中身で分ける形に直す（+1 タスク）。Dependencies: なし

#### Specs (dependency order)

- [ ] talk-thread-count-growth -- トークのたびに増えるスレッド名簿の項目（と、残っていれば生きたスレッド）を、OS から数えて切り分けてから直し、報告の数を実際と合わせる（バグ・区分 B・v0.2.0）。Dependencies: なし

### ponytail による見直し（2026-10-19 `/kiro-discovery` で起票）

> 出どころ（2026-10-19 開発者）: 「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。問いへの答え: 指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」・範囲は本番のクレート全部に加えて、開発の道具のクレート・`tools/` の台本・`pilot`。物差しは ponytail のプラグイン 5.3.2（同じ日に利用者全体へ入れた・PR#343）の `/ponytail:ponytail-audit`。

- **やり方の決め（Approach Decision）**: 選んだ＝範囲ごとに 1 本の spec にする。各 spec は、要件で「範囲の audit の指摘を全部直す」形だけを書き、**ギャップ分析で受け持つ範囲だけに audit をかけ**（audit の「最大 20 件」と「読まなかった所」は外す＝全件・全部読む）、要件の討議で指摘を要件に落として対象を確定し、設計で直し方を決め、実装で直す。直した後にもう一度かけて 0 件を確かめる（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。選ばなかった＝起票のときに全部の範囲へ audit をかけて brief に指摘を書く（33 の範囲を今読むのは重く、着手までに v0.1.0 の変更で古びる）。
- **分け方（Boundary Strategy）**: クレートとモジュールの境目で、`crates/` と `tools/` のどのファイルもちょうど 1 本の持ち物になるように分けた（起票の道具で 2,618 ファイルを確かめた＝持ち主の無いファイル 0・2 本の持ち物 0。`vendors/` は外のリポジトリなので外した）。1 本はコード約 3.4〜12k 行＝audit が範囲を全部読める大きさ（テストはコードの 1〜3 倍）。ファイルが重ならないので、同じウェーブにまとめて並べられる（1 ウェーブ 8 本の決まりは残る）。
- **気を付ける継ぎ目**: 関数の形を変えると、ほかの範囲の呼び出し元に直しが及ぶ（各 brief の「Constraints」＝全呼び出し元を直す・相手の spec が走っていれば申し合わせてから触る）。`clippy-199-lints`（版に縛らない列）は同じクレートに触る＝見直しの段のあいだは並べない。
- **版の段**: 「見直し（最優先・v0.1.0 の前）」の段を新しく置いた。この段が残るあいだは、今の版としてこの段から取る。着手中の v0.1.0 の `mcp-reload` はそのまま走り、その着地を 5 本（`audit-emo2-boot-switch`・`audit-emo2-boot-spine`・`audit-areka-install-update`・`audit-kanade`・`audit-ghost-catalog`）が待つ。v0.2.0 が作り直す wintf・dola の範囲も今やる（開発者「今やる」）＝v0.2.0 の作り直しで消える直しも出る。**同じ日のうちに改め**: この段は v0.1.0 の直後の版 **v0.1.1** へ移した（開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」）。v0.1.0 の版の門を越えるまでウェーブに入れない。`mcp-reload`（v0.1.0）を待つ 5 本の印は残すが、v0.1.1 になる頃には着地している。
- **版の段の改め（2026-10-22 `/kiro-discovery`）**: 版の段から外し、**v0.1.0 の後に隙間で取る優先度が高い列**にした（開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」）＝v0.1.1 の版は無くなった。v0.2.0 の作り直しと触るファイルが重なる 27 本は、作り直しの前に済ませる（問いへの答え「作り直しの前に済ませる」＝10-19 の「v0.2.0 が作り直す範囲も v0.2.0 より前にやる」を残す）＝重なる v0.2.0 の spec より先に取る。重ならない 6 本（`impl-watch`・`pilot`・`placement-follow`・`test-kits`・`tools-perf-judge`・`ukadoc-survey`）はいつでも隙間で取る。正本は「版の段」の「v0.1.0 の後の隙間」の行と、各行の「版」の欄。
- **直さずに残す指摘**: 3 つだけ（開発者の裁定に反する直し・わざと置いた先送りの印・SSP や ukadoc の正典に関わる振る舞いの変更）。どれも要件の討議で開発者に裁定を仰ぐ。
- **Fable**: どれも Opus で足りる（新しい境界や公開の契約を引かない）。

#### Specs (dependency order)

- [ ] audit-emo2-boot-frame -- areka 本体の emo2_boot のうち毎フレームの組み立てを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: runtime-crate-split
- [ ] audit-placement-follow -- areka 本体の配置のうち追従と移り変わりの判定を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: placement-crate-split
- [ ] audit-placement-core -- areka 本体の配置の残り（窓の出し方・設定・保存・重なりの順・DPI ほか）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: placement-crate-split
- [ ] audit-areka-input-menu -- areka 本体の入力の出来事と右クリックメニューを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: input-crate-split
- [ ] audit-areka-app -- areka 本体の残り（起動・起動オプション・ゴーストの会話の場・SHIORI の結線・終了ほか）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: session-crate-split
- [ ] audit-wintf-window -- wintf の窓と窓の手続きを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-wintf-layout-widget -- wintf のレイアウトと部品を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-wintf-input -- wintf のドラッグ・ポインタ・クリック透過・ツールチップを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-wintf-graphics-world -- wintf の描画・World・見た目の木・共通の型を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-wintf-runtime -- wintf の ECS の外（刻みの土台・COM・窓の型・API・数の型）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-emo-text-layout-draw -- 文字の層のレイアウトと描画を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-emo-text-state-actor -- 文字の層の状態・アクター・選択肢・窓を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-dola -- dola（絶対時刻の台本の再生）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-seriko -- SERIKO（シェルのアニメーション）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-emo-compose -- 面の合成を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-emo-present -- 面とバルーンの提示を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-emo-atlas -- 絵の焼き込みとアトラスを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-parsers-sakura -- 設定ファイルの読み取りとさくらスクリプトを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-sylphya-actor -- プロパティの仕組み・役者の土台・トークを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-shiori-host32 -- 32bit SHIORI の中継と SHIORI の ABI・試験用の DLLを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-mcp-update-crates -- MCP の受け口と中継・ネットワーク更新のクレートを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-impl-watch -- 机の貸し出しのアプリ（impl-watch）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-ukadoc-survey -- ukadoc の網羅の調べの道具を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-test-kits -- 試験の道具のクレート（実走・記録の捕まえ・検体・一時フォルダ）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-tools-perf-judge -- 性能改善ループの判定と台帳（Python）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-tools-perf-run -- 性能改善ループの走らせ方（PowerShell）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-tools-scripts -- `tools/` 直下の台本（全体テスト・配布物・リリースノート・検体の取り寄せほか）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-pilot -- 先進坑の実験コードを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [ ] audit-emo2-boot-switch -- areka 本体の emo2_boot のうち切り替え・バルーンの見え方・ホバー・移動の cueを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload, runtime-crate-split
- [ ] audit-emo2-boot-spine -- areka 本体の emo2_boot の残り（背骨・消費者の台帳・適合の台本・資産・トークの時計ほか）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload, runtime-crate-split
- [ ] audit-areka-install-update -- areka 本体のインストール・ネットワーク更新・MCP の受け口・根の一覧・説明書を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload, session-crate-split
- [ ] audit-kanade -- kanade（ゴーストの段取りの役者）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload
- [ ] audit-ghost-catalog -- ゴーストの読み込み・.nar のインストール・利用者のデータの置き場を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload

### メニューのサムネイル（2026-10-20 `/kiro-discovery` で起票）

> 出どころ（2026-10-20 開発者）: 「サムネイル表示に対応する。メニューでゴーストを選択中、サムネイル画像を表示する機能がある（ukadoc参照）arekaが対応している画像形式について、対応してほしい。α付き画像の透過も対応が欲しい。対応時期はdolaとかの基盤対応が終わってからで良いかな。」

- **決めたこと**（同日の答え）: 作り方＝メニューとは別の小さな窓で出す（オーナードローの裁定〔生きている決まり 6〕を待たない・メニューは Win32 標準のまま）／版＝v0.4.0（dola の整備の後）／動く絵（APNG・動く WebP）のサムネイル＝動かして見せる。
- **見つけたこと**: `WM_MENUSELECT` はどこでも受けていない（wintf の `dispatch_window_message` に足す）。メニューの表示中も tick は回る（`menu/trigger.rs` の `display`）。番号から先はフォルダの名前を閉じ込めた閉包だけ＝「番号 → サムネイルの場所」の表を `plan.rs` で作る。一覧は `has_thumbnail`（`thumbnail.png` が在るか）だけを見て絵は読まず、`.pnr` も数えない＝既定のバルーン StayseeBalloon の `thumbnail.pnr` は今は「無し」。絵の読み手（WIC・`image` の APNG／WebP）と α の決め（`Normalizer`・`UseSelfAlpha::Off` が `.pnr` と同じ）は使い回せる。
- **範囲の外**: エクスプローラーの上のホバーのプレビュー・表示を切る設定（設定画面の持ち物）・オーナードローと着せ替えのメニュー（`popup-menu-shell-look`）・プロパティ `thumbnail`（`property-catalog-lists`）・`.pna`・動く GIF。
- **要件の段で SSP の MCP で測ること**: ゴーストのサムネイルを置く場所（最上位か `ghost/master/` か・今の `has_thumbnail` は最上位を見る）・探す名前の順（`.png`／`.pnr`／`.jpg`／areka が読める形式）・メニューの横に出す位置。

#### Specs (dependency order)

- [ ] menu-thumbnail-preview -- 右クリックメニューの項目にカーソルを乗せている間、サムネイル（areka が読める形式・α・`.pnr`・動く絵）をメニューの横の別の窓に出す。v0.4.0。Dependencies: v0.2.0 の dola の整備（時刻の土台）・`wintf-d2d-composite-rules`

### シェルのホバーのアニメ（2026-10-20 `/kiro-discovery` で起票）

> 出どころ（2026-10-20 開発者）: 「『ぱすた開発』と相談して仕様を決めてみて。実は、シェルアニメーションに『マウスホバーが発生した時』だけ発動させたい要望がある。たとえば、リージョン『Head』に乗ってるときにイヤイヤするとか、パスタではクローズボタンに反応させるとか。対応時期はdola関係が落ち着いてから」

- **決めたこと**（同日の答え）: 書き方＝シェルに宣言する areka 独自の書き方（「ぱすた開発」のセッションで開発者本人が選んだ。ゴーストが `OnMouseEnter`／`OnMouseLeave` で返す形はトークを置き換えるので困りごとを解かない）／形＝interval の新しい語と bindgroup の「乗っている間 ON」の両方／動き＝乗っている間ループと乗るたびに 1 回を書き分ける／版＝v0.4.0。
- **見つけたこと**: ぱすたは × の当たり判定「閉じる」・`animation1500.interval,bind`（赤い × の 1 枚絵）・`sakura.bindgroup1500.name,閉じる,赤` を、`OnMouseEnter`／`OnMouseLeave` の返事の `\![bind,閉じる,赤,1/0]` で点け消しし、トークの最中は返さない（開発者の困りごと＝トーク中に切り替えられない）。知らない interval の語は今 `debug` の記録だけで捨てられる。当たり判定の出入りを追う所は無い（`mouse-click-wheel-events` が作る）。seriko への外からの知らせは受け箱 `SerikoMsg` だけで、点け消しの前例は `Stage`。
- **範囲の外**: SHIORI の `OnMouseEnter`／`OnMouseLeave`／`OnMouseHover`・カーソルの形・ツールチップ・`\i[ID]`・`\![anim,…]`・`collisionex` を読むこと・バルーンの上のホバー・乗ってから発動までの待ち。
- **要件の段で SSP の MCP で測ること**: 知らない interval の語・知らない bindgroup の鍵を SSP がどう扱うか（読み飛ばしてほかの行を生かすか）。

#### Specs (dependency order)

- [ ] shell-hover-animation -- マウスが当たり判定に乗っている間だけ動くアニメをシェルの書き方だけで書く（interval の語と bindgroup の両方・ループと 1 回の書き分け・トーク中も動く）。v0.4.0。Dependencies: mouse-click-wheel-events・seriko-interval-combinations・seriko-script-triggers・v0.2.0 の時刻の土台

### ビジュアル層の取り込み（2026-10-21 `/kiro-discovery` で起票）

> 出どころ（2026-10-21 開発者）: 「現在、`crates\wintf\src\ecs\visual` 層のコード群は実装準備されているが採用されていない。これらを新しいビジュアル層・描画層として取り込んでほしい。D2D合成の多様化に必要と考える。このコード群で実現可能と思われる機能を推測し、必要な改善を行うスペックを起票せよ。実施フェーズはwintd/dola改修のタイミングとする。」

- **決めたこと**（同日の答え）: ⑴ 起票は 2 本に分ける（描画の木を本番へ上げる `wintf-visual-tree`／差分の描き直し `wintf-visual-damage-redraw`）。⑵ 描画の木は **WUC の描画面の下に置く**（描画面ごとに 1 本の木を 1 枚に描く・窓や描画面どうしの重ね合わせは今の WUC の木のまま＝`draw-methods-canon` が emo の部品を木の節にして乗れる）。⑶ 版は v0.2.0（wintf・dola の改修）の先頭。⑷ `wintf-d2d-composite-rules` の議題 1（描画の木を本番に上げるか）は「上げる」に決まった＝合成の規則の型は `VisualDrawContent` の側。
- **見つけたこと（main `213a3747`）**: `ecs/visual/` はコンパイルされるが外からの参照 0・system 0・テスト 0（最後の手入れ 2026-09-03）。揃っている＝節の部品（命令の列＋重ね方＋補間・WinUI 型の変形・矩形／図形の切り抜き・見える旗・世界の行列と外接矩形・見た目の版）と平らにする `build`（画家の順・間引き・回転した矩形の切り抜きを図形へ）。無い＝描く係・配線・`WorldVisual`／`VisualVersion` を埋める system・差分の比べ（README の構想だけ）。傷 8 つは `wintf-visual-tree` の brief。WUC の描画面は部分の `BeginDraw` を断る＝差分の描き直しは `wintf-d2d-composite-rules` の自前の入れ物に描く。
- **実現できる機能の見立て**: 部品ごとの重ね方（`draw-methods-canon`・`draw-methods-blend` の受け皿）・回転と拡大と反転（`--scaling` の負の値）・任意の図形の切り抜き（`element-clipping-option`）・変わった範囲だけの描き直しとスクロールの写し（負荷）。
- **既存の spec の書き足し**: `wintf-d2d-composite-rules`（議題 1 の答え・前提に `wintf-visual-tree`・WARP の足場はあちら）・`audit-wintf-graphics-world`／`audit-wintf-runtime`（見直し・v0.2.0 と重なる分・`ecs/visual/` と、そこからだけ使われる `numerics` を未使用を理由に消さない）。各 brief の末尾の「2026-10-21 の書き足し」。
- **優先度**: どちらも開発者が起票を頼んだもの＝優先・区分 A。**ウェーブへの組み入れは v0.2.0 が今の版になってからの棚卸で行う**。議題は各 brief。

#### Specs (dependency order)

- [ ] wintf-visual-tree -- 作りかけの描画の木（`ecs/visual/`）を本番の描画層へ上げ、WUC の描画面の下で節ごとに重ね方・補間・変形・切り抜きを効かせて 1 枚に描く（傷の直し・WARP の決定論のテストの足場を含む）。v0.2.0。Dependencies: audit-wintf-graphics-world, audit-wintf-runtime
- [ ] wintf-visual-damage-redraw -- 描画の木の前フレームと今の描く列を比べ（LIS）、変わった範囲だけを自前の合成の入れ物へ描き直す（答えは全部描いた答えと画素で一致）。v0.2.0。Dependencies: wintf-visual-tree, wintf-d2d-composite-rules

### 裏で常駐するゴーストの入れ替え（2026-10-21 `/kiro-discovery` で起票）

> 出どころ（2026-10-21 開発者）: 「裏でSHIORIだけ常駐維持しておいて、さくらスクリプト投げてきたゴーストを取っ替え引っ替え切り替えるモード。裏に隠れているSHIORIには時刻関係以外のアクティブイベントは投げない。複数SHIORIが同時にさくらスクリプトを要求してきた場合、現在表示中のゴーストの再生が終わるまで次の再生を遅延させる。」

- **決めたこと**（同日の答え）: 土台＝`multi-ghost-runtime` の後（開発者「これは、土台ができた後でよい」）／版＝v0.4.0（開発者「夢仕様とまではしないが、少なくともdola、wintf関係がリリースされた後の次のリリース候補」＝v0.2.0 の基幹の整備の後。v0.4.0 の「dola の整備の後」の並びに置いた）。
- **見つけたこと**: 時刻のイベントは `OnSecondChange` だけで、時計はゴーストごと。kanade と SHIORI は窓なしで起こせる（窓は `wire_emo2_boot` が足す）。今の切り替えは SHIORI を降ろして載せ直すので、表示だけを別のゴーストへ渡す道は無い（近い前例は同じゴーストの着替え）。kanade は台本を溜めない作りなので、待ち行列は kanade の外（UI スレッド）に置く。「話し終えた」は `AwaitTalkGap` がそのまま使える。
- **答えの無かった問い**（要件の段の議題へ）: 台本を持っている間の `OnSecondChange` の送り方（推し: 再生中として Reference3＝0 の NOTIFY・1 体 1 本まで）・裏に置くゴースト（推し: 入っている全員）。
- **範囲の外**: 複数を同時に並べて出すこと（`multi-ghost-runtime`）・呼ぶ働き（`ghost-call`）・掛け合い・入れ替わりの見た目の効果・設定画面。

#### Specs (dependency order)

- [ ] resident-ghost-rotation -- 裏で SHIORI だけを常駐させ、台本を返したゴーストへ表示を替えて再生する（裏へは時刻だけ・同時なら表示中の再生の終わりまで待たせる）。v0.4.0。Dependencies: multi-ghost-runtime・v0.2.0 の wintf・dola の基幹の整備

### 台本の並走（2026-10-21 `/kiro-discovery` で起票）

> 出どころ（2026-10-21 開発者）: 「マウスホバーの方向性　1.シェル定義のアニメーション発動条件にリージョン判定マッチを入れる。（中略）シェル定義だけで実現可能→こちらはおそらくすでに起票済み　2.トークを止めない即時再生モードでさくらスクリプトを割り込ませる。理論上はさくらスクリプトは並走再生いくらでも出来る、が、やれれば良いというものではない気がする。→多分未起票。さくらスクリプトの冒頭に特定のタグがあるとき、これまで再生していたスクリプトを『止めず』、スクリプト再生を並走させる」

- **1 つ目**は起票済みの `shell-hover-animation`（上の「シェルのホバーのアニメ」）＝新しい起票なし。
- **決めたこと**（同日の答え）: 並走でしてよいのは見た目と音だけ（文字・バルーン・選択肢・クリック待ちは読み捨てて記録に出す）／本数は縛らない／版＝v0.4.0（`shell-hover-animation` と同じ）。
- **見つけたこと**: ukadoc に並走は無い（PLUGIN/2.0 の `ScriptOption: nobreak` は「今のトークを最後まで喋った後で実行する」待ち行列）。kanade の `Phase::Steady{talk}` と ghost の dispatcher の `active` はどちらも「1 本だけ」の枠で、返事は枠を上書きする。dola の `CuePlayer` は受け身の作りで並べられる。`\i`・`\![anim]`・音のタグはまだ受け手が無い（本 spec は作らない）。
- **範囲の外**: `\i`・`\![anim]`・音の受け手／シェルの書き方だけのホバー／並走の台本に文字を許すこと／待ち行列。

#### Specs (dependency order)

- [ ] parallel-visual-script -- 冒頭の印のタグで、見た目と音だけの台本を今のトークを止めずに並走させる（本数は縛らない・文字などは読み捨てて記録）。v0.4.0。Dependencies: v0.2.0 の時刻の土台（`talk-anchor-interrupt-ticks`）・talk-thread-count-growth

### sylphya の sqlite の KVS（2026-10-21 `/kiro-discovery` で起票）

> 出どころ（2026-10-21 開発者）: 「areka-sylphyaで行っている永続化について、sqliteベースへの移行を検討せよ。画面などで高速に参照されうる揮発変数（rustコードで直接管理）、get/set propertyの値として呼ばれるが、揮発してよい変数（sqliteのインメモリDBで管理）、永続化が必要な変数（sqliteのファイルDBで管理）、これらを統一的なインターフェースで操作可能なKVSとせよ。また、現行では管理階層として「アプリ層」「ゴースト層」「シェル層」「バルーン層」を持つこととする。これらの改修は、wintfやdolaなどの基礎整備と同じフェーズで実施することとする。」

- **決めたこと**（同日の答え）: 層の意味＝重ねて上書き（バルーン→シェル→ゴースト→アプリの順に最初に見つかった値・書くときは層を名指し）／置き場＝各層の今の置き場（今の `sylphya.toml` の隣）／手書き＝やめて MCP の `set_property` で書く／速い層＝型付きの取っ手で直に読み、同じ値を名前でも読み書きできる（ECS の値そのものは ECS に残す）／作り方＝案 A（番人のアクターが接続 1 本に 4 つのファイルと `:memory:` を ATTACH・get も set もアクターへの問い合わせ）／クレート＝rusqlite（`bundled`）／分け方＝2 本／版＝v0.2.0。
- **採らなかった案**: 読む側もそれぞれ接続を持つ形（WAL と共有のメモリー DB）・今の写し（`MirrorImage`）を読む口に残して sqlite を保存だけに使う形・ユーザールートに 1 つのファイル（ゴーストのフォルダと記憶が離れる）・手書き用の TOML を残す形。クレートは sqlx（非同期・tokio 前提）・diesel（ORM）・limbo（若い）を採らなかった。
- **見つけたこと**: 層の名前（`PersistScope`）はもうあるが、読むと同じ `dotted_global` に入って上書きの順が無い・バルーンの層は置き場が無い。覚える鍵は型の決まった 9 種類だけで、自由な鍵はメモリーだけ。アプリ層のファイルは 3 者が丸ごと書き直す形で、同時に書くと片方が消える（`boot_resolve.rs` の注）。ECS の値は sylphya に写していない（起動時に TOML を直に読み、ドラッグの終わりに直に書く）。
- **既存の spec への影響**: `shell-balloon-scale-setting`（v0.4.0）の「`sylphya.toml` を手で書く」は `mcp-set-property` の `set_property` で書く形へ改める（`mcp-set-property` の範囲で brief を直す）。`readonly-mode`（v0.1.0）の `PersistIo` の差し替えと、`multi-ghost-runtime` のアプリ層の書きの集め方は、`sylphya-sqlite-kvs` が新しい口へ移す。

#### Specs (dependency order)

- [ ] sylphya-sqlite-kvs -- sylphya の保存を sqlite へ移し、速い層・メモリーの sqlite・ファイルの sqlite を 1 つの KVS の口で扱う（鍵の表が住み場所を決める・4 層の重ね読み・`sylphya.toml` を一度だけ取り込む）。v0.2.0。Dependencies: audit-sylphya-actor・readonly-mode
- [ ] mcp-set-property -- areka 独自の MCP の道具 `set_property` で、鍵と層を名指しして値を書く（手書きの `sylphya.toml` の代わり）。v0.2.0。Dependencies: sylphya-sqlite-kvs
### 棚卸㉘の分割と起票（2026-10-22 `/kiro-next`）

> 分割は無し。起票 1 本（持ち主が居なかった `OnDestroy` の場面）。持ち主が居なかった残り物 2 件を既存の brief へ件として足した。経緯は「棚卸㉘の裁定」。

#### Existing Spec Updates

- [ ] network-update-canon-order -- 件を足した: ネットワーク更新の窓口の読み直しで、後送りの列が空だと `OnGhostChanged` を送る（完了 `mcp-reload` の research の「既存の小さな穴」）
- [x] mcp-strict-errors -- （2026-10-30 に完了）件を足した: `params` が入れ物でない要求に受け口が 415、中継が -32600 を返し、-32602 にならない（完了 `mcp-receiver-bugs` の research と要件の範囲外）
- 35 本の brief に「2026-10-22 棚卸㉘の再測定」を足した（v0.1.0 の 23 本・余力の枠 2 本・割り込みの候補 10 本）

#### Specs (dependency order)

- [ ] ondestroy-on-exit -- ふつうの終了とゴーストの切り替えで、SHIORI を降ろす前に `OnDestroy`（NOTIFY・Reference0 なし）を送り、網羅台帳の行を直す。いつか・区分 C。Dependencies: none（✅ areka-P0-mcp-reload が送り方の手本）

### ukadoc の正典の写しの取り直し（2026-10-22 `/kiro-discovery` で起票）

> 出どころ（2026-10-22 `script-security-level` の要件討議の議題 3・開発者）: 「起票漏れしないか？起票するなら今しましょう」

- **見つけたこと**: 網羅台帳の id は `catalog.toml` に実在しなければならない。写しは `ukagaka-doc-mcp` 0.2.7（2026-08-24）のままで、`\![execute,filewatch]`・`\![execute,schedule-add]`・`\![execute,schedule-delete]` が 1 件も無い＝台帳に行を足せない。`coverage-roadmap-refresh` はカタログの更新を範囲外と明記している＝受け皿が無かった。
- **並び**: `coverage-roadmap-refresh` の前（写しを新しくしてから数え直す方が安い）。席の条件は同じ（台帳の行の状態や持ち主を変える spec と同じウェーブに置かない・v0.1.0 の後が安い）。

#### Existing Spec Updates

- [ ] coverage-roadmap-refresh -- 範囲外のカタログの更新は `ukadoc-catalog-refresh` が持つ・そちらを先に着地させる（2026-10-22 の節を足した）

#### Specs (dependency order)

- [ ] ukadoc-catalog-refresh -- ukadoc の正典の写し（`catalog.toml`）を `ukagaka-doc-mcp` の最新（0.10.3）で取り直し、増えた項目を網羅台帳へ差し込み、消えた・id の変わった行を移し、報告を作り直す。版に縛らない・区分 C。規模 S〜M（5〜12・`diff` の後に決める）。Dependencies: none（下流＝coverage-roadmap-refresh）

### window-link-directives の持ち越し（2026-10-23 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `window-link-directives` の完了時の棚卸（`signoff.md` の「申し送り」ⅰ〜ⅳ・`tasks.md` の Implementation Notes の 8・9.2・`requirements.md` の要件 10.6 と裁定の表の 2 行目）。新しい spec は 1 本、既存の brief への追記が 3 件。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: 起動のときに窓が作業領域の外へ出る 3 件（並べ直しが寄せない・`free` の窓が大きい面ではみ出す・記憶から戻す組の寄せ方）は、触る場所（起動の並べ直し・記憶から戻す所・面の大きさの変化）を持つ brief が無いので新しい 1 本にまとめた。`dpi-realign-remembered-chain` は同じファイルを触るが、Scope の Out に「起動の最後の並べ直し」を挙げ、中身も拡大率の変化の詰め直しなので足さなかった。`alignmenttodesktop` の `top`／`left`／`right` の起動の位置と作業領域の変化は、吸着の向きの土台を持つ `window-snap` の件 1（本文の古い台帳の注記も直した）。動く WebP の合成し直しと MCP の返事の遅れは、適用の計時を測る `present-emit-tail-latency` の件 1。`\![set,zorder]` の組がシェルの切り替えで捨てられる件は、「ゴースト終了まで有効」の寿命を決める `stay-on-top-directives` の件 1（`zorder-property` はプロパティの読み書きだけで、タグの入口を Out にしている）。
- **版**: `window-work-area-clamp` は v0.1.0（ぱすたの窓がはみ出す＝ゴール「ぱすたが SSP と同じに動く」に当たる）。`window-snap` はいつか（区分 C の件を持つが、夢の段のまま。急ぐなら着手のときに最初の段として切り出す）。`present-emit-tail-latency` は版に縛らない（測ることが本体）。`stay-on-top-directives` は v0.1.0 のまま（+1〜2 タスク）。
- **直列の約束**: `window-work-area-clamp` は `dpi-realign-remembered-chain`（`chain_finalize.rs`・`drain_resnap.rs`・`persist.rs`）・`descript-window-keys`（`measure.rs`）と同じウェーブに置かない。

#### Existing Spec Updates

- [ ] window-snap -- 件 1: descript の `seriko.alignmenttodesktop` の `top`／`left`／`right` を、起動の位置と作業領域が変わったときの沿わせ直しでも正典どおりに効かせ、網羅台帳 `assets.toml` の `degraded` の 8 行を直す（区分 C）。本文の古い台帳の注記（absent）を改めた。Dependencies: なし
- [ ] present-emit-tail-latency -- 件 1: ぱすたの動く WebP が `cache_hit=false` で毎秒 10〜48 回合成し直され、刻みが詰まり、MCP の `sakurascript` の返事が 4.5 秒〜10 秒超遅れた件を測って切り分ける。Dependencies: なし
- [x] stay-on-top-directives -- （2026-10-31 に完了）件 1: 台本の `\![set,zorder]` の組をシェルの切り替えの後も保ち、descript の組だけを取り替える（`window-link-directives` 要件 10.6・+1〜2 タスク）。Dependencies: なし

#### Specs (dependency order)

- [x] window-work-area-clamp -- （2026-10-30 に完了）起動の並べ直し・記憶から戻す所・大きい面への切り替えで、キャラクターの窓を作業領域の外へ出さない。寄せ方は SSP で測ってから裁定する（バグ・区分 B・v0.1.0）。Dependencies: なし（`dpi-realign-remembered-chain`・`descript-window-keys` とは直列）

### dot-sakura-specimen の持ち越し（2026-10-23 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `dot-sakura-specimen` のタスク 5.1（`completed/areka-P0-dot-sakura-specimen/verification.md` の 8 節の崩れ 7 件）と完了時の棚卸。新しい spec は 2 本、既存の brief への追記は 5 件。新しい 2 本の brief は開発者の条件「次のリリースまでに治ること」を持ち、起票のときは roadmap に触らない指示だった＝2026-10-23 の steering の同期で版の段（v0.1.0）と spec 台帳に載せた。着手の席は決めていない＝次の棚卸で組む。

#### Existing Spec Updates

- [x] real-run-log-noise -- （2026-10-30 に完了）件 2: 同梱バルーン `bottle` で `cursor.blendmethod` の WARN が毎フレーム出る（1 回だけ出すはず）。開発者「次のリリースまでに治るように」。版の段は版に縛らないのまま＝次の棚卸で v0.1.0 に入れるかを決める。Dependencies: なし
- [x] default-balloon-selfmade -- （2026-10-30 に完了）件 3: `bottle` の `validrect.right,0` で文字の描画範囲が幅 0 に潰れる。正典の読みは要件の段で SSP で測ってから決める。Dependencies: なし
- [ ] currentghost-property-others -- 件 6: MCP の `get_property` で `currentghost.*` の名前が引けない。持ち主をこの spec と開発者が決めた。Dependencies: なし
- [ ] draw-methods-canon -- 議題「`move`（位置の語）を範囲に入れるか」を `seriko-pattern-calls` へ移した。Dependencies: なし
- [ ] areka-test-binary-stability -- 完了時の棚卸: 負荷の下で止まった会話のテスト（`spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits`）。Dependencies: なし
- [ ] areka-test-binary-stability -- 完了時の棚卸（`install-before-boot`・件 5）: 並べて走らせたときだけ赤になる起動中の印のテスト 2 本（`first_boot_falling_back_pins_mark_with_one_warn`・`switch_marks_unwritable_end_with_write_failed_and_no_reservation`）。Dependencies: なし

#### Specs (dependency order)

- [x] shiori2-protocol -- （2026-10-24 に完了）SHIORI/2.x の口だけを持つゴーストが話すように、相手の版を確かめて 2.x の要求へ組み替え、応答を 3.0 の形へ戻す（件 1・7・その他・区分 A・v0.1.0）。Dependencies: なし
- [x] seriko-pattern-calls -- （2026-10-31 に完了）pattern定義のアニメーションを呼ぶ 7 語と `move` を、重ね方でなく SERIKO の再生の働きとして受ける（件 4・5・その他・区分 A・v0.1.0）。Dependencies: なし

### pasta-in-windows-verification の申し送り（2026-10-23 タスク 5.3 で `/kiro-discovery` で起票）

> 出どころ（2026-10-23 開発者・チャット）: 「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。証拠は同 spec の `verification.md` 5 節の崩れた所の表の 20・21 と 10 節の網羅台帳との突き合わせ（スクロールの矢印・`communicatebox`・倍率の連動の欄）。SSP の基準は同 spec の `ssp-measurement.md`。

- **決めたこと**: 5 本はどれも v0.1.0・区分 A。INFO 行の量は版に縛らない列の `real-run-log-noise` へまとめない。v0.1.0 の外の spec が持っていた分は、新しい spec へ切り出して元の brief に 1 行ずつ書いた。
- **起票で判断したこと**: ⑴ 外から rect を読むには sylphya の動く値の口が要る＝口の持ち主を `currentghost-property-tree`（v0.3.0）から `mcp-scope-rect-property` へ移した。`currentghost.scope(ID).rect` の持ち主は `currentghost-property-others`（いつか）だったので、そちらの brief にも 1 行。⑵ `\![get,property]` の受け手は名前に縛らない形で作る（載っている値なら何でも返る）＝`property-query-channels` から道 1 をまるごと移した。⑶ 倍率の連動は `balloon.syncscale` を `shell-balloon-scale-setting`（v0.4.0）が持っていた＝そちらからも移した。⑷ スクロールの矢印は網羅台帳の持ち主が `balloon-canon-residue` だが、働きの持ち主は `balloon-markers`（2026-10-01 に引き取った）＝棚卸㉑の切り方の前半を切り出し、両方の brief に 1 行。
- **追加の起票（2026-10-23）**: 崩れた所の表の 23（開発者の目視の判定「選択肢をマウスホバーした時の背景の塗りが崩れている。横書きの時と縦書きの時で、枠の補正方法がことなるのでは？」）を、提案への開発者の「承認」で `vertical-choice-hover-band` として起票した。段は優先（バルーンの見た目＝段の決まりの「バルーン関係」）。

#### Existing Spec Updates

- [ ] currentghost-property-tree -- `balloon.scope(ID).rect` と動く値の口を `mcp-scope-rect-property` へ、`balloon.scope(ID).scaling` を `shell-scale-linkage` へ移した（1 行）
- [ ] currentghost-property-others -- `scope(ID).rect` を `mcp-scope-rect-property` へ、`scope(ID).scaling`・`scope(ID).currentmonitor.dpi` を `shell-scale-linkage` へ移した（1 行）
- [ ] property-query-channels -- 道 1 `\![get,property]` を `shell-scale-linkage` へ移した（1 行）
- [ ] shell-balloon-scale-setting -- `balloon.syncscale` と `OnShellScaling`／`OnBalloonScaling` を `shell-scale-linkage` へ移した（1 行）
- [ ] balloon-markers -- 前半（矢印と手動スクロール・装飾の系列の土台）を `balloon-scroll-arrows` へ切り出した（1 行）
- [ ] balloon-canon-residue -- 網羅台帳の `arrow*` の行を `balloon-scroll-arrows` が引き取る（1 行）
- [ ] balloon-scroll-fade -- 上流を `balloon-scroll-arrows` に改めた（1 行）
- [x] dpi-realign-remembered-chain -- （2026-10-24 に完了）`OnShellScaling` の持ち主が `shell-scale-linkage` に決まった（1 行）
- [ ] inputbox-user-input -- `communicatebox` と入力欄の部品は `balloon-communicatebox` が先に作る（1 行）
- [x] real-run-log-noise -- （2026-10-30 に完了）INFO 行の量はまとめず `animated-surface-log-flood` が持つ（1 行）

#### Specs (dependency order)

- [x] mcp-scope-rect-property -- （2026-10-23 に完了）`currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect` を答える・sylphya の動く値の口。v0.1.0・区分 A。Dependencies: none（✅ areka-P0-mcp-get-property・✅ areka-P0-property-name-case-fold）
- [x] animated-surface-log-flood -- （2026-10-31 に完了）動く絵のコマ送りの適用ごとの INFO 行の量を直す（再描画の回数は変えない）。v0.1.0・区分 A。Dependencies: none
- [ ] shell-scale-linkage -- `\![get,property]` の受け手・倍率と DPI のプロパティ・`OnShellScaling`／`OnBalloonScaling`・`balloon.syncscale`。v0.1.0・区分 A。Dependencies: ✅ mcp-scope-rect-property, areka-P0-script-security-level
- [x] balloon-scroll-arrows -- （2026-10-31 に完了）バルーンの矢印と手動スクロール・装飾の系列の土台。v0.1.0・区分 A。Dependencies: none（実機の確かめは、ぱすたの書庫の直し〔ghost_dev の c32229e・未リリース〕の後。それまでは doc/pasta-real-run.md の入れ方で）
- [ ] balloon-communicatebox -- 話しかける入力欄 CommunicateBox と 2 つのイベント。v0.1.0・区分 A。Dependencies: none（実機の確かめは、ぱすたの書庫の直し〔ghost_dev の c32229e・未リリース〕の後。それまでは doc/pasta-real-run.md の入れ方で）
- [x] vertical-choice-hover-band -- （2026-10-24 に完了）縦書きの選択肢の反転の帯を列の幅いっぱいに合わせる（塗りと当たり・横書きは変えない）。v0.1.0・区分 A。Dependencies: none（実機の確かめは、ぱすたの書庫の直し〔ghost_dev の c32229e・未リリース〕の後。それまでは doc/pasta-real-run.md の入れ方で）

### shiori2-protocol の持ち越し（2026-10-24 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `shiori2-protocol` のタスク 4.3（`completed/shiori2-protocol/verification.md` の 4.3 節・tasks.md の Implementation Notes）。新しい spec は 1 本。「．さくら」の崩れはタスク 5.4 で既存の brief へ渡し済み（同じ verification.md の 5.4 節）。起票の条件: 直したことを確かめるのに実機の走行を何度も回す必要がある（原因不明・再現待ち）。

#### Specs (dependency order)

- [ ] helper-first-load-refusal -- 32 ビットの補助プログラムの SHIORI の読み込みが時々 `last_error: 203` で断られる件を再現して切り分け、直すか記録して閉じる（バグ・区分 B・版に縛らない）。Dependencies: none

### vertical-choice-hover-band の持ち越し（2026-10-24 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `vertical-choice-hover-band` の `research.md` の 16.4 節・17.6 節（行の箱の比）と、タスク 4.2 の Implementation Notes（実機の撮った物で「今日の」に横線と × が重なる）。新しい spec は 1 本。起票の条件: `emo-text-line-gap` は直し方の裁定と実物の書体の読み戻しが要る（手順 3 の 1 つ目と 4 つ目）。× の件は `window-work-area-clamp` と触る場所が同じ（同 2 つ目）で、実機と SSP の測りが要る（同 4 つ目）ので件 4 として足した。

#### Existing Spec Updates

- [x] window-work-area-clamp -- （2026-10-30 に完了）件 4 を足した: バルーンだけが作業領域へ寄せられ（実機の記録で y 980 → 804 物理画素）、ぱすたの本体のシェルの `canvas.png` が持つ × と横線（バルーンより手前）が縦書きの 1 列目にかぶる（SSP にはかぶらない）

#### Specs (dependency order)

- [ ] emo-text-line-gap -- 行間を持つ書体で行の箱の比が DirectWrite の行の箱より狭い件を測り、横書きの帯を字に合わせる（直し方は裁定）。バグ・区分 B・版に縛らない。Dependencies: none（✅ vertical-choice-hover-band）

### 縦書きの括弧とフォント名（2026-10-24 `/kiro-discovery` で起票）

> 出どころ（2026-10-24 開発者・チャット）: ぱすたさんの縦書きの画面を見て「「と、」に、不自然な空白が入ってませんか？DWriteの縦書きに関する設定が何か抜けているのでは。」（「と、」は括弧の`「`と`」`のこと）。フォント名は「もしくは、フォント検索処理をより高度にする？GDI名でもマッチするように」。

- **測ったこと**: 列は 1 本の TextLayout で、送り幅は 1 字でも行の中でも 20.00px（areka の切り貼りではない）。開き括弧`「`の墨だけが枠の上端（本来は下端）。`」`は正しい。小書きかなの下の空きはフォントの縦書き用の字形どおり（Chromium でも同じ）。「」→っ」の 15px は全角の括弧を詰めずに組んだ見え方。
- **決めたこと**: ⑴ 括弧の位置と GDI 流のフォント名を 1 本にまとめる（触るファイルが重なる・v0.1.0・区分 A＝2026-10-24 開発者「ロードマップ優先度は高いです。次のバージョンに入れたい」）。⑵ フォント名は書き方を増やさず、カンマ区切りの各候補を GDI の名前でも当てる（オプションにしない・ぱすたの descript は今のまま＝ぱすた開発に確かめ済み）。⑶ 括弧の詰めは正典にない組版の機能として `text-typesetting` へ。⑷ 小書きかなの下の空きは直さない。⑸ 列が覚えている行の矩形より約 6px 左に描かれる件は、`vertical-choice-hover-band`（#375）が帯の側を書体の em の箱に合わせて手当て済み＝列そのものを矩形の真ん中に置くべきかだけを本 spec の要件の段で確かめる。

#### Existing Spec Updates

- [ ] text-typesetting -- 縦書きの括弧の詰め（約物の詰め・「」→っ」の全角の空き）を件として足した（1 節）

#### Specs (dependency order)

- [x] emo-text-dwrite-fidelity -- （2026-10-30 に完了）縦書きの開き括弧`「`を字の枠の下端に描く（DirectWrite の縦書きの抜けを突き止めて直す）・`font.name` の GDI 流の名前を DirectWrite のファミリと太さに引き直す。v0.1.0・区分 A（優先度高・次のウェーブの先頭の候補）。Dependencies: none

### 棚卸㉙の分割と起票（2026-10-23 `/kiro-next`）

> 分割は無し。起票 4 本（覚え書きから 3 本・完了 spec の残り物 1 本）。版に縛らない列の 1 本を v0.1.0 へ上げた。経緯は「棚卸㉙の裁定」。

#### Existing Spec Updates

- [x] real-run-log-noise -- （2026-10-30 に完了）v0.1.0 へ上げた（10-23 の件 2 に開発者の条件「次のリリースまでに治るように」）。区分 C（件 2 は A）
- [ ] config-parse-diagnostics -- 件を足した: 知らない `collision-sort` の値を `SortOrder::Other` にして誰も知らせない（`areka-parsers/src/shell/decode.rs`・完了 `offline-surface-dump` の Implementation Notes）
- [ ] balloon-markers -- 前半（矢印と手動スクロール）を `balloon-scroll-arrows` へ切り出した後の範囲を書いた（残り＝`onlinemarker`・`number`／`balloonnum`・`sstpmarker`／`sstpmessage`）。上流に `balloon-scroll-arrows`、下流に `sstp-receive`
- [ ] talk-fast-forward -- `clickwait*` の絵の系列の相方を `balloon-scroll-arrows` に改めた（`clickwaitmarker.*` の働きはこの spec のまま）
- [ ] balloon-communicatebox -- 網羅台帳 `shiori.toml` の `Status` の注（起票されなかった `areka-P0-communicate-events`）を、行を触るときに直す
- 37 本の brief に「2026-10-23 棚卸㉙の再測定」を足した（v0.1.0 の 24 本・余力の枠 2 本・割り込みの候補 11 本）

#### Specs (dependency order)

- [ ] install-package-type -- `install.txt` の `type,package` の書庫を、中の 1 つずつ入れる。いつか・区分 C。Dependencies: none（`bootghost` は startup-install-option の後）
- [ ] sstp-receive -- SSTP の受信（受け口と SEND・NOTIFY・Option・`OnSSTPBreak`・要件で残りのメソッドを `sstp-receive-methods` に切る）。いつか・区分 C。Dependencies: none（✅ areka-P0-script-security-level・✅ areka-P0-mcp-kanade-tools）
- [ ] balloon-inline-image -- バルーンの文中に画像を置く `\_b[…,inline]`。いつか・区分 C。Dependencies: none（下流 range-choice-tag）
- [ ] ghost-alias-file -- シェルのフォルダの `alias.txt` を読み、キャラごとの別名で面を引き当てる。v0.2.0・区分 B。Dependencies: none（実機の確かめは ✅ shiori2-protocol の後）
- [ ] seriko-missing-surface-default -- 台本の `\s[n]` が無い面を指したときに既定の面を出すか（ukadoc）出さないか（SSP 2.9 の実測）を決めて入れる。いつか・区分 C。Dependencies: descript-window-keys

### 棚卸㉚の分割と起票（2026-10-24 `/kiro-next`）

なし（分割 0 本・起票 0 本）。brief へ足したのは、測り直した 16 本の「2026-10-24 棚卸㉚の再測定」の節だけ。

### 棚卸㉛の分割と起票（2026-10-31 `/kiro-next`）

なし（分割 0 本・起票 0 本）。brief へ足したのは、測り直した 13 本の「2026-10-31 棚卸㉛の再測定」の節と、完了 spec の残り物 2 件の書き足し（`talk-fast-forward` の件 A・`seriko-trigger-lottery-gaps` の件 5）だけ。

- **Existing Spec Updates**: `seriko-legacy-pattern-format`＝spec 台帳に行を足し、いつかの列に置いた（10-30 の起票のときは roadmap に載せていなかった）。`install-before-boot`＝v0.1.0 の版の門に数える。`balloon-communicatebox`・`seriko-interval-combinations`＝要件定義のモデルを Opus に改めた。
- **Specs (dependency order)**: 新しい spec は無し。

### 棚卸㉜の分割と起票（2026-10-31 `/kiro-next`）

なし（分割 0 本・起票 0 本）。brief へ足したのは、測り直した 14 本の「2026-10-31 棚卸㉜の再測定」の節だけ。

- **Existing Spec Updates**: `dressup-change-events`＝版に縛らない列に置いた（10-31 の起票のときは版の置き場を決めていなかった）。`shell-scale-linkage`＝規模を 11〜16 に改めた。`emo-bake-log-overlap`＝規模を案 1 で 6〜9 に改めた。`talk-fast-forward`＝規模を 19〜24 に改めた（切り方は要件の段のまま）。C15 に入れた ③⑤⑥ は範囲を絞る約束つき（「ウェーブ編成」の表）。
- **Specs (dependency order)**: 新しい spec は無し。

## 予約（全て任意・brief なし）

アプリ層＝FMO・DirectSSTP・Plugin/HEADLINE・消滅（多重ゴーストは 2026-10-15 に `multi-ghost-runtime`・`ghost-call` として起票した＝「テーマ別の決めごと」の「版の段と複数ゴースト」）（トレイアイコンは 2026-10-10 に `tasktray-icon` ほか 3 本として起票した＝「テーマ別の決めごと」の「タスクトレイ」）・設定画面と設定ファイル（**`shell-balloon-scale-setting` の表示倍率と「バルーンも連携する」の入口を持つ**＝2026-10-17 開発者「設定画面を待つ」・**動く絵の 3 つの上限を設定項目として引き取る**＝今は環境変数 `AREKA_ANIMATED_IMAGE_MAX_FRAMES`・`AREKA_ANIMATED_IMAGE_MAX_PIXELS`・`AREKA_ANIMATED_IMAGE_MAX_TOTAL_PIXELS` で変える・`animated-image-decode` の要件 6.9〜6.12。値は `bake_with_limits` へ渡す `AnimationLimits` をそのまま使える）・`\![update,platform]`（本体の更新）・**インストーラー版（winget の名乗り `Areka.Areka`・データは `%APPDATA%` へ・スタートメニューのアイコン・2026-10-15 に「windows-reactor 待ち」へ置いた＝WinUI3 のランタイムを入れる形と一緒に決める・それまで zip を維持）**・`\![execute,createnar]`／`createupdatedata`（作る側＝開発者機能・配布物を作る道具が読む `developer_options.txt` もここ＝10-05 に覚え書きから移した）。SSTP の受信は「覚え書き」。互換面＝SAORI は実装しない（SHIORI が直接 `LoadLibrary`・台帳 `not-applicable`）・里々/YAYA 網羅。emo テキスト進化＝回転テキスト（`TextEffects` 予約名 `rotation`／`multicolor`）。**`\f[sub]`／`\f[sup]`／`\f[outline]` は DirectWrite の標準機能で表せる手段が見つかるまで語彙のみ**（2026-09-11 裁定・台帳 `vocabulary-only`・追跡先はこの行）。バルーン美観配置。pasta の native x64／`IShiori` in-proc（本番に使うときは `InProc` の SHIORI を外から終わらせる手を決める＝10-05 に覚え書きから移した）・ベクトル描画・owner-draw 右クリックメニュー（`popup-menu-minimal` の構造の上に見た目を被せる）・着せ替えメニュー。段階 A の先頭ウェーブ 6 束（`roadmap-draft.md`・324 件）は M3 のゴールを決めるときの材料。

---

**追記台帳**: 追記(95)〜(103) の要約は history「2026-09-26 番号表記の廃止に伴う退避」節。

### mouse-hover-event（2026-10-24 `/kiro-discovery` で起票）

- **出どころ**: `dpi-realign-remembered-chain` のタスク 6.3 の実機の確かめ。開発者「マウスホバーのイベントも出て無いですよね。これもスペックは存在するか？」→「OnMouseHover を /kiro-discovery で起票して。」
- **決めたこと**: `OnMouseEnter`／`OnMouseLeave` は `mouse-click-wheel-events` の範囲のまま。`OnMouseHover` だけを新しい 1 本にした（作業中の spec の brief へは足さない）。版は次の棚卸で決める。

- [ ] mouse-hover-event -- キャラクターの当たり判定の上でカーソルが止まったら `OnMouseHover` を送る。止まる時間などは SSP で測ってから裁定する（区分 A）。Dependencies: mouse-click-wheel-events

### 台本の付帯情報（2026-10-25 `/kiro-discovery` で起票）

- **出どころ**: 開発者「さくらスクリプトにファイル指定が含まれる場合、そのファイルのフォルダはカレントディレクトリが『ghost/master(SHIORI.DLLのある場所)』フォルダになる。スクリプト情報のメタデータとして、カレントディレクトリを持たせてた方がよいかもしれない」「多分、台本ごとに台本が基準とするカレントディレクトリとか送信元が何かとかを特定できる情報があればよいと思うのです。実装時期はdolaとかの基礎整備をするタイミングかなと思っています」。
- **決めたこと**: 新しい 1 本 `talk-script-metadata`（v0.2.0・区分 A）。基準は ukadoc の「[実行したゴーストのフォルダ]/ghost/master」＝送ったゴーストでなく喋るゴースト。プロセスのカレントディレクトリは替えない（複数のゴースト・台本の並走で取り合う）。置き場は `ScriptOrigin` に入れない（信用度の語と別物）。ファイルを指すタグそのものは各 spec の持ち物のまま。
- **下流の brief へ足したこと**: `balloon-inline-image`・`balloon-font-file`・`tasktray-ghost-icon` の Upstream に「基準のフォルダは `talk-script-metadata` の解き方を使う」を 1 行。

- [ ] talk-script-metadata -- 台本ごとに実行するゴーストの置き場と送り手を持たせ、ファイルを指すタグの相対パスを 1 つの関数で解く（v0.2.0・区分 A）。Dependencies: none

### 利用統計とノベルゲーム基盤（2026-10-28 `/kiro-discovery` で起票）

- **出どころ**: 開発者が 16 案（U1〜U4 利用統計・M1〜M3 素材と演劇・N1〜N4 ノベル・S1 `shiori4-api`・P1〜P2 pasta）を出し、「すでに起票済みかどうかを確認し、起票計画を行い、ディスカッションを進めてください。実施タイミングは、少なくともdola関係の整備のあと、ノベルゲーム基盤の時か、その後ろ、夢レベルかもしれない。ただし、整理することで不要になる可能性があるアイデアも含まれます」。議題は 1 件ずつ話し合った（開発者「まとめて議論したくない」）。原文と決めたことの全部は `doc/ideas/novel-platform.md`・`doc/ideas/usage-stats.md`。
- **決めたこと**:
  - 段: 新しい「ノベルゲーム基盤」（v0.4.0 の後ろ・番号は依頼書の後）と「利用統計」（ノベルゲーム基盤と並走してよい・dola の整備の後）。
  - 起票しない: S1（`shiori4-api` と重なる＝SHIORI はどの版でも別プロセスの helper・`IShioriHost` と JSON-RPC の封筒はやめる・DLL の選び方は shiori4 の `distribution.md`）・P1（pasta に `pasta-dll-multiarch`・`shiori4-v4-host`・`shiori4-v4-lua-api` が起票済み・入口は COM でなく shiori4 の C ABI）。
  - 今 brief を書くのは pasta との取り決めに左右されない 3 本（`sound-canon-tags`・`ghost-material-metadata`・`shell-multi-compose`）と、`areka-usage-stats`・`doc-tree-tidy`（v0.2.0）。
  - 演劇は pasta.dll を SHIORI とするゴーストと同じ形のパッケージ（舞台は SHIORI が返すさくらスクリプトで動かす・v4 を推すが SHIORI/3.0 でも同じ台本で動く・種別は当面 `type,ghost`＋印・別のゴーストの SHIORI は呼ばない）。原則として任意のゴーストの任意のシェルを役者に借りられる（オプトアウト・作者表記を出す）。
  - 素材 ID の規則は areka が先に決め、外の U1 が後で写す。メタデータは正典のキーを使い、足りない分だけ areka の独自キーで descript に足す。
  - 音は正典のタグから・再生の土台は Media Foundation。
  - 利用統計の要件は「利用者を秘匿しつつ利用者ごとに識別し、1 日単位で集計し、GitHub Pages で公開する」。暗号化の包み・利用日・HMAC の控え・Secrets の持ち方は実装のときの候補（`doc/ideas/usage-stats.md` の A〜G）。U1〜U3 は外の新しいリポジトリで、その文書を持って `/kiro-discovery` を回す。
  - その場で直した: `doc/PASTA_PROFILE.md` の作例の `\w[N]` 6 か所を `\_w[N]` に、「pasta が出す `\w[N]` は上流の欠陥」の注記を正しい出どころに（pasta の `novel-areka-contract` の brief「依頼に添える訂正」）。
- **下流の brief へ足したこと**: `extra-character-windows`（3 人目からの土台を `shell-multi-compose` とどちらが持つか）・`parallel-visual-script` と `sakura-time-directives`（音を鳴らす所＝`sound-canon-tags`）・`property-catalog-lists`（音のプロパティの値＝`sound-canon-tags`・`rateofuselist.*`）・`install-package-type`（単体のフリーシェルの置き場＝`ghost-material-metadata`）に 1 行ずつ。

#### 受け口（pasta の `novel-areka-contract` の依頼書を待つもの・brief なし）

pasta（SHIORI のエンジン・`C:\home\maz\git\pasta`）の `novel-areka-contract` は brief だけで未着手（命令の語彙・イベント・設定の名前と、取り決めの版の番号は未定）。最後のタスクで areka へ依頼書を渡す段取り。届いたら、その棚卸で次を起票する（原文は `doc/ideas/novel-platform.md`）。

- ノベルの進行と入力の残り（N2）: オート・既読スキップ・キーボード送り・設定値（文字の速度・オートの待ち・既読の扱い）の保持。クリック送り・早送り・`\x` は `talk-fast-forward`、マウスは完了 `mouse-click-wheel-events`。待つのは台本の `\x` だけ（自動の改ページは作らない）。
- バックログと巻き戻し・セーブとロード・システム画面の部品と見本（N4）。読めないセーブを上書きで失わない。取り決めの版と areka の brief の対応表を持つ。
- ノベルの音の拡張（N3 の後段）: BGM のクロスフェード・同じ曲を鳴らし直さない・効果音の重ね・区分ごとの音量・台詞と結び付く声・バックログからの聞き直し（`sound-canon-tags` の上に積む）。
- 演劇のパッケージ（M2）: 種別（`type,ghost`＋印か、`/play/` の置き場か）は依頼書の「起動の形」で決める。新しい種別なら `shiori4-api` に「演劇から SHIORI を選ぶ書き方」を足しに戻る。SSP に入れられたときの知らせ方（「areka で遊んでね」の台詞）は pasta への申し送り。演劇の信号（再生の開始・場面への到達・再生の完了）は `areka-usage-stats` へ。
- 制作と配布の導線（M3）: lint・プレビュー・サンプルの演劇・配布の手引き・登記簿への登録の手順（M2 の後）。
- pasta の P2（`pasta_check validate` の検査）: `ghost-material-metadata` がウェーブに載ったら、その要件の段の最初に pasta のセッションへ相談して決める。
- 申し送り: `ALLOWED_EVENT_IDS`（`crates/areka-kanade/src/schedule/events.rs`）は「ukadoc の固定名だけ・各行に ukadoc の URL」の決まり＝ノベル用の独自イベントを足すには表の決まりを広げる。`doc/PASTA_PROFILE.md` の訂正は済み、と返す。

- [ ] doc-tree-tidy -- `doc/` と `docs/` の置き場をまとめ、名前の書き方をそろえ、案内の表と実物の突き合わせを試験にする（v0.2.0・区分 A）。Dependencies: none
- [ ] sound-canon-tags -- 正典の音のタグとプロパティを Media Foundation で鳴らす（ノベルゲーム基盤の先頭・区分 C）。Dependencies: talk-script-metadata
- [ ] ghost-material-metadata -- 素材 ID・探す順・descript の独自キー・借りないでの印・単体のフリーシェル（ノベルゲーム基盤・区分 A）。Dependencies: none
- [ ] shell-multi-compose -- 1 つの舞台の窓に何枚ものシェル・役者の割り当て・普通の窓と全画面・場面転換（ノベルゲーム基盤・区分 A）。Dependencies: wintf-visual-damage-redraw, ghost-material-metadata, element-animation-easing
- [ ] areka-usage-stats -- 同意したときだけ使った素材の ID を 1 日 1 回外の発信器へ渡す（利用統計・区分 A）。Dependencies: ghost-material-metadata

### 本体の crate 分け（2026-10-28 `/kiro-discovery` で起票）

- **出どころ**（2026-10-28 開発者）: 「areka 本体 crate (crates/areka) の肥大を解消するため、`placement` と `emo2_boot` を独立 crate へ分離したい。対応時期は0.1.0で、dolaとかの対応のタイミングか、ponytail関係のタイミング。」「分離するクレート名については、妥当な名前に改名せよ」。2 本の案を見た後に「この際、areka crateのコードベースが大きすぎる問題をサルベージすべきかもしれない。3specで良いと思うか？」→ session と input の境を測って 6 本にした案に「一括承認」。
- **測った数**（main `d6054e4d`・非空行・`cargo check -p areka --all-targets` は警告 0 で通る）:
  - `crates/areka/src` は本番 60,532 行・テスト 146,318 行。lib を持たない bin で、モジュールどうしの参照し合いが 17 組ある。
  - placement: 本番 12,932 行。本体のほかの部分への依存は 0。
  - emo2_boot: 本番 約 15,300 行。本番 51 ファイルのうち 37 は placement しか引かず、14 が本体の中心部・install・update・readme・input と引き合う。
  - input_events: 本番 3,932 行。
  - 起動・終了・切替の層: 集めると本番 約 8,600 行。向きを逆にする所が約 30 件・どうしても切れない所は 0 件。
  - 本番コードで emo2 だけに向けた動きは、既定のゴーストかどうかの判定（`DEFAULT_GHOST_FOLDER = "emo2"`）だけ。
- **やり方の決め**:
  - **選んだ**: 6 本をこの順に 1 本ずつ進める。
    1. `placement-crate-split`（移すだけ）
    2. `runtime-module-untangle`（本体の中で改名し、外への依存を 0 にする）
    3. `runtime-crate-split`（移すだけ）
    4. `input-crate-split`
    5. `session-module-gather`（本体の中で集める）
    6. `session-crate-split`（向きを逆にして移す）

    改名と依存の付け替えをファイルの移動と別の PR にするので、どの PR も 1 つずつ確かめられ、各段階で main が緑のまま。終えた後の依存は `areka`（bin）→ `areka-session` → `areka-input` → `areka-runtime` → `areka-placement` の一方向。本体の本番は約 15,000 行（main と起動オプション・MCP・メニュー・インストール・更新・投げ込み・オフラインの面の書き出し・SHIORI のデモ）。
  - **採らなかった**:
    - placement と emo2_boot の 2 本だけ: 本体に約 30,000 行が残る。
    - wintf へ寄せる: wintf は crates.io へ出す汎用の crate で、placement は ukagaka の意味と areka 系の依存を持つ。
    - 改名と移動を 1 本で: 移動の差分と付け替えの差分が混ざって確かめにくい。
    - session を 1 本で: 20 タスクを超える見込み。
    - menu を input と一緒に: menu は install・update・roots・session を引く。
    - install・update・roots の純粋な部分を既存の crate（`areka-nar`・`areka-update`・`areka-user-data`）へ: 寄せられるのは約 1,400 行で、親の型を使い、`areka-nar` から `areka-ghost` への依存が増える。
    - mcp を crate へ: 本体がつなぐ役そのもの。
    - 名前を `areka-stage`・`areka-wiring`・`areka-boot` に: どれも crate としては未使用だが、開発者の例と「実行基盤」の意味に合う `areka-runtime` を採った。
- **名前**:
  - crate: `areka-placement`・`areka-runtime`・`areka-input`・`areka-session`。
  - 型と関数: `Emo2Wiring` → `RuntimeWiring`・`Emo2BootInputs`／`Emo2BootOutcome` → `RuntimeInputs`／`RuntimeOutcome`・`wire_emo2_boot` → `wire_runtime`・`register_emo2_frame_system`／`emo2_frame_system` → `register_frame_system`／`frame_system`・ログの値 `choice_*_no_emo2` → `choice_*_no_runtime`。確定は `runtime-module-untangle` の要件の段。
  - emo2 を残すのは、既定のゴースト名とその使い手、実在のゴースト emo2 を指す検体だけ。
- **時期**（開発者の「dola の頃か ponytail の頃」の読み）: **v0.1.0 の版の門の後、その後の隙間の先頭**。区分 A。本体を受け持つ audit 8 本より前、v0.2.0 の作り直しより前に置く。v0.1.0 の中に入れないのは、v0.1.0 の残りが同じファイルを触るため（`consumer_ledger.rs` に 1 本ずつ行を足す 5 本の鎖・`window-work-area-clamp` ほか）。`areka-test-binary-stability`（v0.1.0）は、分ける前の形で測り終える。
- **並べ方**:
  - 6 本は同じファイルを触るので、互いに同じウェーブに置かない。`crates/areka` を触るほかの spec とも同じウェーブに置かない。そのため、本体をほぼ単独で使うウェーブが 6 つ続く。
  - そのあいだ、`crates/areka` に触れない audit 25 本は、同じウェーブの空いた席で並べて走らせられる。
  - 本体を受け持つ audit 8 本は、自分の範囲を分ける spec の後に取る。
    - `audit-placement-core`・`audit-placement-follow` → `placement-crate-split` の後
    - `audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` → `runtime-crate-split` の後
    - `audit-areka-input-menu` → `input-crate-split` の後
    - `audit-areka-app`・`audit-areka-install-update` → `session-crate-split` の後
  - v0.2.0 の作り直しで runtime に触る spec は、6 本の後になる（作り直しは新しい crate の上で行う）。
  - `doc-tree-tidy`（v0.2.0 の先頭・開発者「0.1.0の直後」・同じ日の起票）は `doc/` のファイルを動かし、動かしたパスを名指す所を直す。6 本も `doc/COMPAT_ARCHITECTURE.md`・`doc/ukadoc-coverage/ledger/*.toml` などのパスを書き換えるので、同じウェーブに置かない。どちらが先でも、後の方が新しいパスで grep し直す。
  - `multi-ghost-runtime`（v0.1.0 の余力の枠）は、session の層とファイルがほぼ全部重なる。同時には走らせない。余力の枠で済まなければ、6 本の後に新しい形で測り直して取る。
- **共通の約束**（6 本の要件にそのまま入れる）:
  - **振る舞いを変えない**: 移す・改名する・向きを逆にするだけ。直したくなった不具合は、その場で直さず範囲を受け持つ audit へ申し送る。向きを逆にした知らせや口は、終了・切替・差し込みを 1 フレームでも遅らせない。
  - **テストの数**: 着手のときと完了のときに `cargo test --workspace --no-fail-fast -- --list` を取る。`: test$` の行から、名前の末尾（最後の `::` から後ろ）の多重集合を作り、前後で一致させる（crate が変わるとモジュールのパスが変わるため。完了 `oversized-file-refactor` の比べ方）。名前を改めたテストは対応表で説明する。
  - **警告**: `cargo check --workspace --all-targets` の警告 0 件のまま。
  - **全体テスト**: `pwsh -NoProfile -File tools/test-all.ps1` が緑。
  - **古い名前とパス**: 移したモジュールの古いパスとログの出所の名前を grep し、`crates/`・`tools/`・`doc/`・`.kiro/steering/{structure,tech}.md` で 0 件にする。歴史の文書（`roadmap-history.md`・`completed/`）は除く。直す所には次のものを含む。
    - 1,000 行の番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`）・`temp_path_guard_test.rs`
    - `include_str!` と `CARGO_MANIFEST_DIR` でソースを読むテスト
    - `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所（`crates/areka/src/` が約 185 か所）
    - `doc/COMPAT_ARCHITECTURE.md`・`structure.md`
  - **ログの出所**: モジュールのパスから付く出所は、移すと変わる。そのまま変え、名前で読んでいる所を全部直す。
    - `RUST_LOG` の文字列・`log_history.rs` の決まり・`doc/ssp-mcp/log-convention.md`
    - real-run-kit の名指しの決まり（`launch.rs`）・実機の手順書
    - モジュールのパスを写した直書き（`"areka::placement::diag"`・`"areka::boot_config"` など）
    - 語彙として付けた直書き（`"areka::persist::*"`・`"areka::log::*"`・`"areka::perf"`）は変えない。`EnvFilter` の名指しは素の前方一致なので、`areka=debug` は `areka_placement` も覆うが、`areka::placement=debug` は覆わない。
  - **実機**: 中身は移すだけなので、ほかは構造の証跡で足りる（記憶の決めごと「確定の証跡は実機ログと静的構造証跡の二本立て」）。実機で回すのは次の 2 つだけ。
    - ログを名前で読む実機テスト（各 brief に列挙）を 1 回ずつ。
    - 向きを逆にした道を 1 回ずつ。
  - **テストの支え**: crate の外のテストが使う支えは、各 crate の `test-support` 機能で開ける（dev-dependency で付ける）。写しを作らない。境をまたぐテストは上の crate に置く。
  - **ワークスペース**:
    - ルートの `Cargo.toml` は触らない（`members = ["crates/*"]`）。新しい crate は `publish = false`・`version.workspace = true` ほかの慣習に従う。
    - 新しい外の依存を足さない（ワークスペースの中の path 依存だけ）。
    - `THIRD-PARTY-NOTICES.md` は `cargo about generate` で作り直す（手で書かない）。`Cargo.lock` が動くので、`/release` の版上げと同じウェーブに置かない。
    - `tools/test-all.ps1`・`deny.toml`・`about.toml`・`tools/package.ps1`・`.github/workflows` は crate 名の一覧を持たないので直さなくてよい。`tools/crates-io.ps1` の公開の許可リスト（`dola`・`wintf`）にも足さない。
  - **受け持ちの書き換え**: 各 spec は、自分の範囲を受け持つ audit の brief の「受け持つファイル」を新しいパスへ書き換える。ほかの brief（パスに触れている約 70 本）は書き換えず、下の対応表で読み替える。
- **新旧のパスの対応表**（brief を読み替えるときの正本・数字は 6 本の順）:

  | 今 | 着地の後 |
  |---|---|
  | `crates/areka/src/placement/` | `crates/areka-placement/src/`（1） |
  | `crates/areka/src/placement/transition_judge*`（テスト専用） | 本体のテスト専用のモジュール（1）→ `crates/areka-runtime/` のテスト専用（3） |
  | `crates/areka/src/emo2_boot/`（`frame/`・`*_cue.rs` の残り・`consumer_ledger`・`target_map`・`balloon_visibility*`・`talk_*`・`adapter` ほか） | `crates/areka/src/runtime/`（2）→ `crates/areka-runtime/src/`（3） |
  | `emo2_boot/` の `ghost_switch`・`reload`・`reload_args`・`shell_balloon_switch`・`shell_balloon_resolve`・`runtime_boot`・`switch_copy`・`same_folder_pin`・`shell_carry` | `crates/areka/src/session/`（2）→ `crates/areka-session/src/`（6） |
  | `emo2_boot/install_cue.rs`・`update_cue.rs`・`readme_cue.rs` | `crates/areka/src/install/`・`update/`・`readme/`（2）。readme は（5）で session へ |
  | `crates/areka/src/input_events/` | `crates/areka-input/src/`（4）。`shortcut.rs` は `crates/areka/src/menu/`（4）、`file_drop.rs` は本体に残る |
  | src 直下の `ghost_session*`・`boot_resolve`・`boot_config`・`app_exit`・`session_end`・`exit_wait`・`write_failure`・`readonly`・`alert`・`log_history` | `crates/areka/src/session/`（5）→ `crates/areka-session/src/`（6） |
  | `main.rs` のシームの約 367 行（`on_boot_ok`・`settle_session_mark`・`restore_merged_placements` ほか） | 同上 |
  | `roots/` の `mod`・`resolve`・`report`・`user_root`／`readme.rs`・`readme/` | 同上（`roots/` の `edit`・`desk`・`pick` は本体に残る） |
  | ログの出所 `areka::placement::*`・`areka::emo2_boot::*`・`areka::input_events::*`・`areka::{ghost_session,boot_config,app_exit,…}` | `areka_placement::*`（1）・`areka::runtime::*`（2）→ `areka_runtime::*`（3）・`areka_input::*`（4）・`areka::session::*`（5）→ `areka_session::*`（6） |

- [ ] placement-crate-split -- 窓の配置を本体から `areka-placement` へ移す（もともと依存の末端・振る舞いは変えない・v0.1.0 の後の隙間の先頭・区分 A）。Dependencies: v0.1.0 の版の門・areka-test-binary-stability
- [ ] runtime-module-untangle -- `emo2_boot` を本体の中で `runtime` に改名し、型の名前を改め、切替などを `session/` へ移して、外への依存を 0 にする（区分 A）。Dependencies: placement-crate-split
- [ ] runtime-crate-split -- 外への依存が 0 になった `runtime` を `areka-runtime` へ移す（`target_map`・テスト専用の `transition_judge*` も・区分 A）。Dependencies: runtime-module-untangle
- [ ] input-crate-split -- 入力の出来事を `areka-input` へ移す（`shortcut.rs` は menu へ・`file_drop.rs` は本体に残す・区分 A）。Dependencies: runtime-crate-split
- [ ] session-module-gather -- 起動・終了・切替の層と main.rs のシーム・roots の芯・readme を本体の `session/` に集める（区分 A）。Dependencies: input-crate-split
- [ ] session-crate-split -- 集めた層の本体への呼び出し約 30 件の向きを逆にし、`areka-session` へ移す（区分 A）。Dependencies: session-module-gather

### startup-install-option の持ち越し（2026-10-30 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `startup-install-option` の完了時の棚卸（`completed/startup-install-option/requirements.md` の要件 10.2 と、実機の試験 `crates/areka/tests/startup_install_real_run.rs` を作るときに見た 2 件）。新しい spec は 2 本、既存の brief への追記が 1 件。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: 起動の解決の前に入れる件は、要件 10.2（要件の討議 議題 2）で別の spec と決まっている＝新しい 1 本。切り替えのバルーンの件は、触る場所（バルーンの解き方の鎖と切り替えの段 ③）を直す brief が無いので新しい 1 本（`shell-companion-balloon` は同じ関数を触るが、シェルに紐づくバルーンの働きで、直す中身が違う）。YAYA のゴーストで出た補助プログラムの `loadu` の後の `HRESULT(0x80004005)` は、触る場所（補助プログラムの初回の LOAD）が同じで、確かめるのに実物の走行を何度も回す `helper-first-load-refusal` へまとめた。
- **版**: `install-before-boot` は `startup-install-option` と同じ版のゴールの続き＝v0.1.0（版の門に数えるかは次の棚卸で決める）。`ghost-switch-balloon-fallback` は致命的でない新しいバグ＝次の版（v0.2.0）。起こるのは置き場にバルーンが無いときだけで、配布物には既定のバルーン `areka-balloon` が入る。
- **直列の約束**: 2 本とも `crates/areka` の起動・切替の層を触る＝「本体の crate 分け」の 6 本（とくに `runtime-module-untangle`・`session-module-gather`・`session-crate-split`）と同じウェーブに置かない。パスは同じ節の対応表で読み替える。

#### Existing Spec Updates

- [ ] helper-first-load-refusal -- 2026-10-30 の追記: YAYA のゴースト（`konnoyayame.nar`）でも補助プログラムの `loadu` の後に `HRESULT(0x80004005)` が出た（試験は緑・ゴーストは起きた）。Dependencies: なし

#### Specs (dependency order)

- [x] install-before-boot -- `--install` と `initial_install.nar` を起動の解決の前に入れてから起動を続け、空の置き場からも起こす（優先・区分 A・v0.1.0）。Dependencies: startup-install-option
- [ ] alert-dialog-per-monitor-dpi -- 告知の画面を拡大率の違うモニタでもにじまず出し、モニタを移した後のバルーンの描き直しの欠けを見分ける（バグ・区分 B・install-before-boot の完了時の棚卸）。Dependencies: install-before-boot
- [ ] ghost-switch-balloon-fallback -- 切替先にも置き場にもバルーンが無いとき、切り替えを落とさず代わりのバルーンで続ける（バグ・区分 B・v0.2.0）。Dependencies: なし

### window-work-area-clamp の持ち越し（2026-10-30 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `window-work-area-clamp` の完了時の棚卸（`completed/window-work-area-clamp/tasks.md` の Implementation Notes の「最終検証」の範囲外の項目 ①〜⑤・「組 5 の決めごと」②・設計の Risks「バルーンの横のずらしの符号」・`research.md` 12〜13 節）。新しい spec は 2 本、既存の brief への追記は 0 件。その場で解決 1 件＝④（環境の `PASTA_DEBUG=1` でほかのぱすたが 127.0.0.1:9276 を握ると実機の試験の検体が LOAD 失敗する）は `real-run-kit` が起こす子へ `PASTA_DEBUG=0` を渡して直した（`crates/real-run-kit/src/launch.rs`）。⑤（`placement/follow/window_move.rs` が前から 1,000 行を超えている・今 1,227 行）は起票しない＝前からの超過で、`audit-placement-follow` が見直す。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: ②（右に出すバルーンの横のずらしの向き）と ③（並べ替えたバルーンが相方の窓に重なる）は、触る場所がどちらもバルーンの置き方の式（`placement/balloon_limit.rs`・`placement/resolver.rs` P5）で、SSP の測りも 1 回の机でまとめて取れる＝1 本（`balloon-flip-side-rules` の件 1・件 2）。① の並べ直しの預かりは触る場所（`emo2_boot/frame/move_hold.rs`）が違い、本体の crate 分けとの順の縛りも違う＝別の 1 本。
- **版**: 2 本とも致命的でない新しいバグ＝次の版（v0.2.0）。v0.1.0 の検体のぱすた（配布版 `pasta-in-windows`）は本体と相方の窓が同じ位置に重なり、バルーンは左に出し（`sakura.balloon.alignment,left`）、起動の台本で並べ直しを流さない＝同 spec の 4.4 の通しでどの件にも当たっていない。v0.1.0 のゴール「ぱすたが SSP と同じに動く」には当たらないので、`ghost-switch-balloon-fallback` と同じく次の版に置いた。
- **直列の約束**: `balloon-flip-side-rules` は `placement/` を触る＝`placement-crate-split` と同じウェーブに置かない。`window-resetwindowpos-hold` は `emo2_boot` の位置の指令を流す所（`frame/move_hold.rs`・`frame/drain_resnap.rs`）を触る＝「本体の crate 分け」の鎖（とくに `runtime-module-untangle`・`runtime-crate-split`）と同じウェーブに置かない。パスは同じ節の対応表で読み替える。

#### Specs (dependency order)

- [ ] balloon-flip-side-rules -- 反対の側へ並べ替えたバルーンが相方の窓に重なるときの扱いと、右に出すバルーンの横のずらしの向きを、SSP で測ってから裁定して直す（バグ・区分 B・v0.2.0）。Dependencies: なし
- [ ] window-resetwindowpos-hold -- 起動の直後の `\![execute,resetwindowpos]` を、どのキャラ窓も最初の面を表示するまで預かり、仮の大きさで並べない（バグ・区分 B・v0.2.0）。Dependencies: なし

### animated-surface-log-flood の申し送り（2026-10-31 実装中に `/kiro-discovery` で起票）

> 出どころは `animated-surface-log-flood` の実装の記録（`verification.md` のタスク 5.2「release の交互の測り直し」・5.3「SSP の基準」、件 26 の直し `7b3c2d956`、実装中の clippy）。新しい spec は 0 本、既存の brief への追記が 3 件（小さい件は 1 件 1 spec にせず、触る場所が同じ brief へ「件 N」で足した）。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: ⑴ 面 1002 の何もしていないあいだの CPU（areka release 中央値 15.4%・SSP 2.9.26 は 5.6%）は、表示の適用の外の約 12 ポイントが主＝見込みは wintf の全面の描き直しと出す処理なので、差分の描き直しを持つ `wintf-visual-damage-redraw` の件 1 にした（内訳を測ってから縮め、SSP の水準を目標にする）。`present-emit-tail-latency` は適用の中の測っていない区間の尾を測る brief で、適用の外は持たないので足さなかった。⑵ `visible_parts` が有効でコマの無い `bind+random` の pattern0 の先を見える部品に数える件は、見えない部品の評価を扱う `seriko-trigger-lottery-gaps` の件 3 と同じ形＝同 spec の件 4（emo-compose の `nesting.rs` が触るファイルに加わる）。⑶ clippy の既存の指摘（emo-compose のテスト 5 か所・emo-present のテスト 7 件）は `clippy-199-lints` の件 1（emo-compose の 5 か所は列挙済み・emo-present が新しい）。
- **版**: `wintf-visual-damage-redraw` は v0.2.0 のまま（測りだけを先に取るかは同 brief の議題 3）。`seriko-trigger-lottery-gaps` は「v0.1.0 から外した 5 本」のまま（行き先は次の版を決めるときに・+2〜3 タスク）。`clippy-199-lints` は版に縛らない（+1 タスク）。
- **直列の約束**: `seriko-trigger-lottery-gaps` は emo-compose の `nesting.rs` も触る＝emo-compose の `plan.rs`・`nesting.rs` を触る spec と同じウェーブに置かない。

#### Existing Spec Updates

- [ ] wintf-visual-damage-redraw -- 件 1: ぱすたの面 1002 で何もしていないあいだの release 版の CPU（中央値 15.4%・WorkingSet 176 MB）が SSP 2.9.26（5.6%・94 MB）の約 3 倍。適用の外の約 12 ポイントの内訳（wintf の描き直し・present・DComp・刻み）を測ってから縮め、SSP の水準を目標にする。Dependencies: wintf-visual-tree, wintf-d2d-composite-rules
- [ ] seriko-trigger-lottery-gaps -- 件 4: emo-compose の `nesting.rs` の `bind_targets`／`visible_parts` が、有効でコマの無い `bind+random` の pattern0 の先（ぱすたの 1410）を見える部品に数え、`plan.rs` の `flatten_surface`（`7b3c2d956` で描かなくした）と食い違う。seriko の `rebuild`・`moving_visible` が使う。一致の檻に pattern0 を持つ `bind+random` の文面を足して固定する（区分 B）。Dependencies: なし
- [ ] clippy-199-lints -- 件 1: emo-compose のテストの `chunks_exact` 5 か所（列挙済み・今も残る）と、emo-present のテスト 5 本の 7 件（`chunks_exact` 4・`type_complexity` 2・`assertions_on_constants` 1・列挙の外）。emo-present の本番のコードは `--no-deps` で 0 件。Dependencies: なし
