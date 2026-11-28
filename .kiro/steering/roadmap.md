---
inclusion: manual
updated_at: 2026-11-28
---

# Roadmap — areka（α 後・M3 へ向けた並べ直し）

> **M1 は 2026-09-11、M2（α）は 2026-10-02 に完成宣言済み**。本ファイルは 2026-10-02 の棚卸⑳で、α 後の着手順へ書き直した。**完了した spec の行・棚卸⑭〜⑲の裁定・α のウェーブ表・旧「登記だけの行」の全文は [roadmap-history.md](roadmap-history.md) の「2026-10-02 棚卸⑳退避」節へ逐語で移した**（history が全文正本・非改変）。2026-10-04 の棚卸㉑で、旧ウェーブ表（C1・C2）と棚卸⑳の裁定を同じ history の「2026-10-04 棚卸㉑退避」節へ移した。2026-10-05 の棚卸㉒で、旧ウェーブ表（C3 と予定の C4）・直列の列・覚え書き・直接修正候補・棚卸㉑の裁定を「2026-10-05 棚卸㉒退避」節へ移した。2026-10-10 の棚卸㉓で、旧ウェーブ表（C4）・初回リリースの段取り・直列の列・brief にした覚え書き・直接修正候補・棚卸㉒の裁定を「2026-10-10 棚卸㉓退避」節へ移した。 2026-10-18 の棚卸㉗で、v0.1.0 を開発者の新しいゴール（SSP 並みの自動試験・ゴースト「窓際のぱすたさん」が SSP と同じに動く）で組み直し、12 本を起票した（旧ウェーブ C8・旧直接修正候補・棚卸㉖の裁定・旧 v0.1.0 の行・`v0.0.2` の後の申し送り 8 項目は history「2026-10-18 棚卸㉗退避」）。 2026-10-23 に、開発者「ロードマップについて、膨れ上がってきてる気がします。完了済みはヒストリーに動かしてください。」を受けて、完了済みの記録（完了サマリの表の長い説明・数の経緯・組み終えたウェーブの記述・spec が全部完了したテーマの節と棚卸㉔〜㉖の申し送りの節）を history「2026-10-23 完了済みの退避」へ逐語で移した。 2026-11-06 の棚卸㉟で、旧ウェーブ表（C17 の行・次の候補）・その先のウェーブの見込み（C18〜C20）・棚卸㉞の裁定・旧直接修正候補・消した覚え書き 1 項を history「2026-11-06 棚卸㉟退避」へ移した。 2026-11-13 の棚卸㊱で、旧ウェーブ表（C18 の行・次の候補）・C18 の開始コマンド・その先のウェーブの見込み（C19〜C21）・棚卸㉟の裁定・旧直接修正候補・台帳の完了済みの 1 行・旧「組済みのウェーブ」「1 本だけの席」「1,000 行」の行を history「2026-11-13 棚卸㊱退避」へ移した。 2026-11-14 の棚卸㊲で、旧ウェーブ表（C19 の行・次の候補）・C19 の開始コマンド・その先のウェーブの見込み（C20〜C22）・棚卸㊱の裁定・旧直接修正候補・旧「組済みのウェーブ」「1 本だけの席」の行を history「2026-11-14 棚卸㊲退避」へ移した。 2026-11-16 の棚卸㊳で、旧ウェーブ表（C20 の行・次の候補）・C20 の開始コマンド・その先のウェーブの見込み（C21〜C23）・棚卸㊲の裁定・旧直接修正候補・旧「組済みのウェーブ」の行を history「2026-11-16 棚卸㊳退避」へ移した。 2026-11-17 の棚卸㊴で、旧ウェーブ表（C21 の行・次の候補）・C21 の開始コマンド・その先のウェーブの見込み（C22〜C24）・棚卸㊳の裁定・旧直接修正候補・旧「組済みのウェーブ」「1 本だけの席」の行を history「2026-11-17 棚卸㊴退避」へ移した。 2026-11-20 の棚卸㊵で、旧ウェーブ表（C22 の行・次の候補）・C22 の開始コマンド・その先のウェーブの見込み（C23〜C25）・棚卸㊴の裁定・旧直接修正候補・旧「組済みのウェーブ」「1 本だけの席」の行を history「2026-11-20 棚卸㊵退避」へ移した。 2026-11-21 の棚卸㊶で、旧ウェーブ表（C23 の行・次の候補）・C23 の開始コマンド・その先のウェーブの見込み（C24〜C26）・棚卸㊵の裁定・旧直接修正候補・旧「組済みのウェーブ」「1,000 行」の行を history「2026-11-21 棚卸㊶退避」へ移した。
> 正本配置: 本ファイルが正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。ukadoc 網羅の段階・順位の正本は `doc/ukadoc-coverage/`（`briefing.md`・`linkage.md`・`roadmap-draft.md`）。完了した spec の実装の詳細は各 `completed/` spec が正本。
> **読む順**: どの版に何を入れるか＝「版の段」。いま何を始めるか＝「ウェーブ編成」。個々の spec の中身＝「spec 台帳」と各 brief。brief を持たない宿題＝「覚え書き」。

## M1 ゴール ✅（2026-09-11 完成宣言）

areka（x64）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・32bit SHIORI）を「そのまま」起動→会話→撫で→メニュー→終了まで E2E 実走させる。emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台。

**✅ 完成宣言**: 適合検証項目表 20 項目全合格の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md`・項目別は同 `acceptance-record.md` §8.1）。持ち越し 8 件（同 §6）はいずれも判定を書き換えず、引受先は下の spec 台帳に全て実在する（§13.1 行 3「初回起動限定の位置調整」だけは「違和感があれば個別仕様を切る」の据え置き＝spec なし）。

## M2 ゴール（α）✅（2026-10-02 完成宣言）＝第三者がデスクトップマスコットを管理できる

**✅ 完成宣言**: 第三者の手順の検証項目表（13 項目と付随の確認 2 つ）の実機サインオフに開発者の署名（`.kiro/specs/completed/areka-P0-alpha-release-signoff/verification/alpha-completion.md`・項目別は同 `acceptance-record.md` §7）。署名の根拠のコミット `8460506d`・zip `areka-alpha-x64-20261002-8460506.zip`。

**利用者の一周**: zip を展開して起動 → `.nar` を窓へ落とす（またはメニューから選ぶ）→ ゴーストが起動する → 右クリックメニューでゴースト／シェル／バルーンを替える → ネットワーク更新で作者の修正を受け取る → 終了 → 再起動で前回の状態に戻る。表現力は M1 の水準（emo2 が普通に動く）で据え置いた。メニューは Win32 標準。

**持ち越し**: 完成判定 §6 の 5 行は 4 本の spec（`install-live-target-hazards`・`drag-click-without-move`・`package-check-temp-cleanup`〔10-02 に `release-package-versioned` へ合流〕・`install-companion-canon`〔10-03 に要件の段で `install-companion-reading`・`ghost-standard-balloon`・`shell-companion-balloon` の 3 本へ引き継いで閉じた〕）へ起票済み＝ウェーブ C1〜C4。emo2 の 2 件は ghost_dev へ申し送り済み。

## 版の段（着手の大枠の正本）

> **呼び名**: 「α」の語は使わず、版の番号で段を呼ぶ。M1・M2 の節は完成宣言の記録として残す。「伺かの冠」の候補は `doc/ukadoc-coverage/roadmap-draft.md` にある。
> **並べ方**: 若い版から取る。**今の版（まだ公開していない最も若い版）が残っているあいだ、後の版の spec はウェーブに入れない**。空いた席に入れてよいのは「版に縛らない」列のうち、今の版の着地までのウェーブ数を増やさないものだけ＝先頭で、今の版の spec と触るファイルが重ならず、今の版が使う 1 本だけの席を取らず、見立ての上限がそのウェーブで一番大きい今の版の spec の下限以下で、人や外の待ちも、静かな机も、前後の全体テストの比べも要らないもの（判定の正本は `/kiro-next` のスキルの「Version gate」）。迷ったら席は空けておく。同じ版の中は「ウェーブ編成」の並び順の決まり（ぶら下がる数 → 区分 → 古さ・触るファイルの重なり 0）。**版の中身は、やってみて重ければ前後してよい**。
> **バグ（区分 B）**: 新しく見つかったバグは、致命的なら今の版、そうでなければ次の版へ足す。
> **済んだ版の行**: 表に残すのは生きている行だけ。公開した版（タグを打った版）の行と、spec が 0 本になった版でない組の行は、棚卸で表から外し、history のその棚卸の退避の節へ逐語で移す（数の行にも 0 を書かない）。「版に縛らない」と「いつか」の行は 0 本でも残す。spec が 0 本でまだ公開していない版の行は公開まで残し、そのあいだのウェーブの開始コマンドは `/release <版>`。判定の正本は `/kiro-next` のスキルの Step 6「Release gate」と Step 7。

| 版 | 目標 | spec（並びは依存の順） |
|---|---|---|
| **v0.1.0 の余力の枠** | 版の門に数えない。手が空いたら取る | `multi-ghost-runtime`（設計の上の障害は無い＝ゴーストの部品はもうゴーストごとに作れ、止めているのは areka 本体の「1 つだけ」の置き場と、止まり方がアプリの終了へ流れること・24〜30 タスク・触るファイルが多い＝ほぼ単独で走る） → `ghost-call`＝**2 本** |
| **v0.1.0 から外した 3 本** | v0.1.0 のゴールに当たらないもの。行き先は次の版を決めるときに決める | `seriko-trigger-lottery-gaps`（バグ・ぱすたの着せ替えのまばたきは今の作りで動く）・`bottom-row-text-clip`（バグ）・`balloon-webp-names`（自前の既定バルーンが絵を `.webp` で持つときの働きの前提）＝**3 本** |
| **v0.2.0** | wintf・dola の基幹の整備と windows-rs の移行（`windows-text` の採用まで）。windows-rs の移行の本体は、上流の 0.101 系が crates.io に公開されてから起票し、全部のクレートを一気に入れ替える（0.62 と 0.101 を混ぜて段階で移すことはしない・移す動機の一つは、areka から windows-reactor を使えることをはっきりさせること・6〜8 本の見込み・最後は `areka-emo-text` の `windows-text` の採用＝`emo-text-windows-text`・`windows` クレートをやめて `windows-bindgen` の束縛へ移る・起票まで数に入れない）。移行は公開の版に乗るので、crates.io へ出すクレートが git の依存を持つことは無い。**公開の前の最後**に、開発者が `/update`（依存のクレートの更新）を打ち、その PR を squash で統合してから `/release 0.2.0`（spec は起こさない＝手順はスキル `/update` が持つ・`/kiro-next` は spec が 0 本になったときの開始コマンドを `/update` → `/release 0.2.0` の順に出す） | `balloon-text-visual-tree`（バルーンの文字の層を描画の木へ移し、文字の区画の自分の描画面でスクロールを平行移動の写しで済ませる）・`draw-methods-canon` → `draw-methods-blend`・`extent-element-offset`（バグ・外形が element定義の X,Y を数えない）→ `animated-image-import`（`import` の画像を名前を保って読む・2 本とも切り抜きのオプションの spec の前）・`compose-f16-precision`・`instant-to-interrupt-ticks`（残りの「今」の読み取りを割り込み時刻の時計へ替える）→ `background-result-tick-wake`（背景の結果で起床の旗を立てる・`tick-gate-adoption` の前）・✅ `seriko-talk-clock-fidelity`（11-28 着地）（口パクの時刻の残り 2 件）・`tick-gate-adoption`（保留のまま・短い A/B の測り方をここで組む）・`element-clipping-option`（D2D の切り抜きで描く形にそろえる）・✅ `talk-thread-count-growth`（11-28 着地）（致命的でないバグ・トークのたびにスレッドかスレッド名簿の項目が増える）・✅ `sylphya-sqlite-kvs`（11-28 着地） → `mcp-set-property`（sylphya の保存を sqlite へ移し、速い層・メモリーの sqlite・ファイルの sqlite を 1 つの KVS の口で扱い、アプリ・ゴースト・シェル・バルーンの 4 層を重ねて読む／手書きの `sylphya.toml` の代わりに MCP の `set_property` で書く・wintf・dola の改造とは触るファイルが重ならない）・`property-query-channels`（プロパティの許可の表と台本の入口）→ `shiori4-api`（COM の作りかけを消し、SHIORI4 を C ABI の `ekicyou/shiori4` へ完全に移す・shiori4 v0.1.0 の `host-protocol` に合わせる）・`ghost-alias-file`（致命的でないバグ・シェルのフォルダの `alias.txt` を読まない〔検体のどっとさくらの `alias.txt` は持っていない面の番号を持っている面へ振り替える〕）・✅ `talk-script-metadata`（11-28 着地）（台本ごとの付帯情報＝実行するゴーストの置き場〔ghost/master〕と送り手）・`ghost-switch-balloon-fallback`（致命的でないバグ・切替先にも置き場にもバルーンが無いと切り替えが落ちる）・✅ `balloon-script-offset-align`（11-28 着地）（台本の `\![set,balloonoffset]`・`\![set,balloonalign]`・ぱすたの台本は使わない・`placement/balloon_limit.rs`・`resolver.rs` を触り、押し戻しの関門の画面の選び方は `balloon-flip-side-rules` の並べ替えの決まりに従う・区分 C）＝**20 本** |
| **v0.3.0** | 目玉のフェード＋小さく、なるべく早く出す。覚え書き「ゴースト同士の掛け合い」（`OnOtherGhostTalk`・`\![set,otherghosttalk]`・`\![raiseother]`）もこの版で起票する | 目玉＝`text-reveal-fade`・`balloon-scroll-fade`（シェル内バルーンの後続だが目玉なのでここ）／プロパティ＝`currentghost-property-tree`／配置の読み＝`defaultx-canon`（バグ・`defaultx`・`defaultleft` を ukadoc〔2.9.13〕の画面の座標の読みに合わせ、鎖の決まりの隙間の読みと 485・494・513・514 の裁定を開け直す）＝**4 本** |
| **v0.4.0** | 最低限のアニメーション（dola の整備の後） | `element-animation-easing`（dola の整備が終わってから）・`shell-balloon-scale-setting`（ゴーストごとの表示倍率を DPI の倍率に掛ける・入口の設定画面は windows-reactor 待ち）・`shell-hover-animation`（当たり判定に乗っている間だけ動くアニメをシェルの書き方だけで書く・トークの最中も止まらない）・`resident-ghost-rotation`（裏で SHIORI だけを常駐させ、台本を返したゴーストへ表示を替える・同時に返したら表示中の再生が終わるまで待たせる・`multi-ghost-runtime` の後）・`parallel-visual-script`（さくらスクリプトの冒頭に特定のタグがあるとき、再生中の台本を止めずに、見た目と音だけの台本をトークと並走させる）・`window-timed-directives`（台本「席替え」が使う＝`\![move]` の時間・`\![moveasync]`・`\![set,scaling]`・`\![set,alpha]` を時間つきで動かす）・`scope-currentmonitor-property`（`currentmonitor` の `rect`・`work`・`primary`・`bpp` を出す・待つ物が無いので席が空けば前に出してよい）・`seriko-legacy-pattern-format`（旧 SERIKO 1.x の書式のシェルのアニメが 1 つも動かない）・`mouse-hover-event`（カーソルを止めても `OnMouseHover` を送らない・`shell-hover-animation` とホバーの判定を分け合う）・`text-reveal-dance`（字が現れる瞬間に動きをつける・v0.3.0 のフェードと `element-animation-easing` の使い手）・`window-snap`（マスコットがほかの窓に座る・ぶら下がる・陰から覗く・最初の段は正典のお座り `\![set,alignmenttodesktop,osuwari,…]`）＝**11 本** |
| **v0.5.0** | **暫定**（pasta の依頼書が届いたら番号と中身を見直す）＝ノベルゲーム基盤。pasta（SHIORI のエンジン）の 0.5.0 のノベルを areka で遊べる土台（v0.4.0 の後ろ）。brief を持つのは pasta との取り決めに左右されないものだけ。進行と入力・バックログ・セーブ・システム画面の部品・ノベルの音・演劇のパッケージは「テーマ別の決めごと」の「利用統計とノベルゲーム基盤」の受け口の節で、pasta の `novel-areka-contract` の依頼書を待つ | `sound-canon-tags`（先頭・正典の音のタグ・再生の土台は Media Foundation）・`ghost-material-metadata`（素材 ID と descript のキー・探す順・借りないでの印）→ `shell-multi-compose`（1 つの舞台の窓に何枚ものシェル・普通の窓と全画面・場面転換・v0.2.0 の新しい描画の層の上）・`sakura-raise-directive`（正典の `\![raise,…]`・`play` の命令ではないので席が空けば前に出してよい）・`key-press-event`（正典の `OnKeyPress`・`play` の命令ではないので席が空けば前に出してよい）＝**5 本** |
| **利用統計**（ノベルゲーム基盤と並走してよい・dola の整備の後） | 利用者を秘匿しつつ利用者ごとに識別し、1 日単位で利用状況を集計して GitHub Pages で公開する。仕様（U1）・発信器（U2）・集計サイト（U3）は外の新しいリポジトリの持ち物（`doc/ideas/usage-stats.md`） | `areka-usage-stats`（`ghost-material-metadata` と外の U2 の後・まずゴースト・シェル・バルーンだけ・演劇の信号は演劇のパッケージの後）＝**1 本** |
| **いつか** | 夢と ukadoc（低）・やることが無くなってから。**v0.3.0 を公開して v0.4.0 が今の版になったら解禁**＝その棚卸から、この行の spec を「版に縛らない」と同じ割り込みの扱いにする（今の版の着地のウェーブ数を増やさないものだけ・判定はスキルの「Version gate」の割り込みの条件）。それまではウェーブに入れない | 書き足しのタグ＝`balloon-append-tags`（`\C` と `\*`・区分 C・ぱすたもどっとさくらも使わない・YAYA の雛形のゴーストは `\C` を使う＝v0.3.0 へ上げるかは開発者）／文字飾りの細部＝`text-align-shadow-canon`・`choice-marker-styling`・`font-size-keywords`／プロパティの残り＝`currentghost-property-others`・`system-property-values`・`property-catalog-lists`・`zorder-property`・`property-ipc-transport`／台本と SHIORI＝`sakura-time-directives`・`sakura-time-critical`・`sakura-embed-directive`・`makoto-dll-host`・`makoto-reload-directives`・`check-script-arg-checks`・`ondestroy-on-exit`／その他＝`extra-character-windows`・`update-check-options`・`network-update-canon-order`・`balloon-canon-residue`・`elevated-drop-filter`・`surfaces-basepos`・`balloon-directional-offset-keys`（descript の方向つきのずらしと surfaces.txt のバルーンのずらし・件 2 に頭上・足元の縦の基準の SSP との違い・区分 C・`balloon-script-offset-align` の後）・`release-code-signing`（任意・外の申請〔SignPath〕が要る）・`dpi-transition-two-tick-bounce`・`zorder-chain-residue`・`emo-text-canon-residue`（取り下げ予定）・`install-package-type`・`sstp-receive`・`balloon-inline-image`・`seriko-missing-surface-default`＝**31 本** |
| **windows-reactor 待ち** | WinUI3 に任せる設定系・タスクトレイ系・インストーラー。入力欄・メニュー・ツールチップは WinUI3 で描ける部品なので、reactor の描画で作り直す。zip のインストーラーは当面そのまま使い、WinUI3 のランタイムを入れられるインストーラーも reactor を待つ。数には足さないがここで待つもの＝`script-impact-tiers` の同意の窓（表と判定は版に縛らない列のまま）・wintf のツールチップをリアクターの描画に替えること・予約の「設定画面と設定ファイル」「インストーラー版」・`popup-menu-extra-resources` の ⑵（消滅通告・おすすめ・ポータルの項目をメニューに足す・正本は `completed/popup-menu-extra-resources/brief.md` の「完了の棚卸」の節） | `tasktray-icon` → `tasktray-ghost-icon`・`tasktray-balloon`・`tasktray-minimize`・`balloon-communicatebox`（ぱすたの申し送りの 1 本）・`popup-menu-shell-look`（生きている決まり 6〔オーナードローはやらない〕は解かない・作り直しの後に着せ替えのメニューの要るところだけを測り直す）・`inputbox-user-input`・`menu-thumbnail-preview`・`shell-tooltip`・`balloon-link-hover`＝**10 本** |
| **版に縛らない（道具と手入れ・測ること）** | 席が空いたときに挟む（v0.2.0 の間は、その大規模改造の影響を受けず、改造の後にやるべきでないものだけ） | 道具と手入れ＝`dev-folder-alias`・`clippy-199-lints`・`temp-path-exceptions-to-target`（見張りの例外表のファイルの一時の根を `target\test-roots` の下へ・`clippy-199-lints` の着地の後）・`areka-test-wait-wording`（テストの文言の無い待ち 31 か所を目印つきの待ちの部品へ・区分 B）・`ukadoc-catalog-refresh`（`coverage-roadmap-refresh` の前に＝写しを新しくしてから数え直す方が安い）・`coverage-roadmap-refresh`・`balloon-dpi-switch-redraw`（モニタを移した後のバルーン・区分 B）・`emo-balloon-merge-log-overlap`（バルーンの 2 層マージの記録が 2 行ずつ出る残り）・`config-parse-diagnostics`・`audit-app-leftovers`（本体の見直し 3 本の取り残し・同じファイルを触る v0.2.0 の spec の後）／測ることが本体（静かな机で 1 本ずつ）＝`present-emit-tail-latency`・`install-live-target-hazards`・`real-run-input-guard-observations`（再現が本体）・`helper-first-load-refusal`（再現が本体）・`pasta-idle-surface-cost-measure`（ぱすたの面 1002 の何もしていないあいだの CPU と WorkingSet の内訳）／着手済みのテーマの後続のシェル内バルーン＝`balloon-font-file`・`text-typesetting` → `text-ruby`・`talk-fast-forward`・`balloon-markers`・`balloon-element-order`／バルーンのリンクと OS の連携＝`range-choice-tag` → `link-context-copy`／同梱バルーンの正典＝`shell-companion-balloon`／MCP の areka 独自ツール＝`mcp-shiori-query`・`mcp-user-response`・`script-impact-tiers`／SERIKO の引き金＝`seriko-script-triggers`・`seriko-interval-combinations`／持ち越し＝`balloon-break-position`・`anchor-style-canon`（途中で止めても困らないもの）／着せ替えのイベント＝`dressup-change-events`（ぱすたは受け手を持たず SSP にも 204 を返す・kanade の `schedule/events.rs` を `balloon-communicatebox` と分け合う）／文書の改名＝`doc-compat-architecture-rename`（区分 C・S・`doc/COMPAT_ARCHITECTURE.md` を `doc/compat-architecture.md` へ・本体の crate 分けと、この文書を名指す見直しが全部着地した後の 1 ウェーブで取る＝同じウェーブに置かない）／外の待ち＝`winget-release-automation`（配布と公開の列・v0.0.2 を出した手提出の PR 450070 が winget-pkgs に取り込まれてから始める）＝**34 本** |

- **数**: 2＋3＋20＋4＋11＋5＋1＋31＋10＋34 ＝ **121**（台帳の brief を持つ spec の数と一致）
- **版をまたぐ依存**: 後の版の spec が前の版の spec を待つ形だけにする（前の版が後の版や版に縛らない列を待つ逆向きは置かない）。前の版の spec が版の外の spec を働きの上で待つと分かったら、待つ相手をその版へ入れて、その版で済ませる。版に縛らない列の spec が版の spec を待つときは、待つ相手が着地してから取る。ファイルの順だけの前後は、列の中の順を入れ替えて済ませる（働きの依存ではない）。
- **版に縛らない列を v0.2.0 の間に挟むときの注意**: v0.2.0 は wintf・dola の基幹（D2D の合成の決まり・描画メソッド・当たり判定・合成の精度・dola の割り込みの刻み・時刻の土台）を作り直す。v0.2.0 の着手から着地までの間に版に縛らない列から取るのは、**その改造が触る場所（触るファイル・合成と描画の道・dola の時刻と割り込み・当たり判定）に触れず、改造の後に作り直しにならないもの**だけ。触れるもの・改造の後の形に乗るべきものは、v0.2.0 の着地まで待たせる。挟む前に、取る spec の brief（設計があれば設計）の触る場所を v0.2.0 の残りの spec と突き合わせ、重なりが 0 と示してから並べる。迷ったら挟まない。

## 実装規律（balloon-system の失敗から得た正）

- **実装ファースト**: 各作業ユニットの成果物は「実際に動く」検証済みコード。**spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1 ユニット＝1 かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: 使う分だけ実装し、拡張は型/レジストリの口だけ残す。抽象は「2 例目の実物」が要求してから。動く資産から建てる。
- **粒度基準**: 1 ユニット＝単一 pass/fail の独立観測。純粋層は fixture/mock 直入力で切る。UI 位置決め・座標系は本番ゴースト（実 emo2）＋実 DPI（≠96）が観測条件（記憶 areka-placement-real-ghost-first）。
- **語彙完備・配線ゼロの追跡**: 先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目。
- **1 ファイル 1,000 行**: 機械の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表・暗黙増加不可・**どの spec も例外表に触れない**）。2026-11-21 棚卸㊶の実測（PR#488 のコメントの整理で軒並み減った）で上限の近く（本番のファイル）: `crates/areka-emo-text/src/look.rs` 983・`crates/areka-runtime/src/placement_tests/transition_judge.rs` 982・seriko の `table.rs` 957・`crates/areka/src/main.rs` 947・`crates/areka-emo-text/src/surface.rs` 937・kanade の `schedule/steady.rs` 926・`crates/areka/src/session/ghost_switch.rs` 926・`crates/areka-placement/src/lib.rs` 896・seriko の `looper.rs` 887・`ghost_session.rs` 881（`crates/areka-placement/src/follow/window_move.rs` 1,178 と `crates/areka-runtime/src/placement_tests/transition_judge_verdict_tests.rs` 1,022 は例外表に載る）。kanade の `schedule/mod.rs` は 679・kanade の `actor.rs` 873。テストの兄弟ファイルにも 950 行を超えるものが多い（`crates/areka-runtime/src/placement_tests/transition_judge_tests.rs` 999・wintf の `composite_damage_tests.rs` 992・`crates/areka/src/session/ghost_switch_tests.rs` 982・seriko の `parts_tests.rs`・`looper_tests.rs` 975・`crates/areka/src/update/worker_tests.rs` 967・`crates/areka/tests/stay_on_top_real_run.rs` 966・`viewbox_draw_decoration_tests.rs` 966・areka-ghost の `tests/ghost/inproc_e2e_test.rs` 964 ほか）。足すときは新しいファイルへ。
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
| — | — | — | 完了した spec は `.kiro/specs/completed/`。ウェーブごとの完了の表は history「2026-11-22 steering の同期の退避」 |

### 次のリリースへの申し送り

スキル `/release` が段 1 で読む（`.claude/skills/release/procedure.md` の 1.4）。「次の版上げのときに」と申し送る事は、ここへ 1 項目で足す。項目の頭に、書いたときの最新の版を「`v{その版}` の後」の形で書く（その版より後のリリースが済んだら、済んだ物として読み飛ばされる。消すのは、次に roadmap を触る PR）。版上げの 5 つのファイル（根の `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`dist/README.txt`・その版の見どころ）の外を触る仕事は、ここへ書かずに、リリースの前に別の PR で片づける（残っていると `/release` は段 1 で止まる）。

- **`v0.1.0` の後**・`audit-dola`・`dola-yaml-backend`（2026-11-13）: 変わる点＝crates.io の `dola` の機能（`v0.1.0` と比べた正味の差）。⑴ 機能 `json`（既定で有効だった）と既定の機能が無くなった（壊れる変更）。⑵ 機能 `yaml` と任意の依存 `serde_yaml` が無くなった（壊れる変更・`Cargo.toml` の `dola` の機能の指定から `yaml` を消せば直る）。⑶ 機能 `toml` は続けて指定できる。引く `toml` の版は 0.8 から 1 に上がり、機能 `toml` の下で `dola` が TOML の読み書きの入口 `DolaDocument::from_toml_str`／`to_toml_string`（誤りは `dola::TomlReadError`／`TomlWriteError`）を持つ＝利用者は自分で `toml` を依存に書かなくてよい。版の番号の上げ方は `/release` で決める。
- **`v0.1.0` の後**・`wintf-visual-tree`・`wintf-d2d-composite-rules`（2026-11-06〜11-13）: 変わる点＝crates.io の `wintf` の公開面（壊れる変更）。⑴ 描く中身の型 `VisualDrawContent` の置き場が `wintf::ecs::visual::draw_content` から `wintf::ecs::graphics::visual` の下へ移った ⑵ その欄 `composite_mode: D2D1_COMPOSITE_MODE` が `mix_mode: MixMode` に替わり、作り口 `new_blend` が増えた ⑶ 新しい公開の型 `SurfaceReadback`・`PixelRead`・`SurfaceImage`（`readback.rs`）と列挙の腕 `NanOpacity`（`world_visual.rs`）が増えた。版の番号の上げ方は `/release` で決める（2026-11-14 棚卸㊲で足した）。
- **`v0.1.0` の後**・`runtime-module-untangle`・`audit-shiori-host32`（2026-11-13〜11-14）: 動きが変わる点＝⑴ 記録の的 `areka::emo2_boot::*` が `areka::runtime::*`・`areka::session::*` に替わった（`RUST_LOG` で絞っている人は書き直す） ⑵ 32bit の SHIORI の中継が DLL を `LoadLibraryExW`（DLL の置き場と既定の場所だけを探す）で読むようになり、`PATH` にだけ在る依存の DLL は読めなくなった。中継は `PASTA_DEBUG=0` を必ず立て、中継の標準エラーを記録へ流す。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）（2026-11-14 棚卸㊲で足した）。
- **`v0.1.0` の後**・`audit-wintf-window`（2026-11-14）: 動きが変わる点＝枠つきの窓に位置か大きさの片方だけを与えたとき、作った直後から正しい位置に出る（与えた側だけに枠を足す・指摘 2・タスク 6.1）。変わる点＝crates.io の `wintf` の公開面（壊れる変更）＝⑴ `WindowPos::set_window_pos`・`SetWindowPosCommand::flush`・`WindowStyle::from_hwnd`・`WindowPointerTracking`・`WindowHandle` の欄 `instance` が消えた ⑵ `SetWindowPosCommand::enqueue` は UI スレッドの外から積むと `error!` を残して捨てる ⑶ `detect_display_change_system` の引数が 1 つ増えた（全部の表は `completed/audit-wintf-window/research.md` の 14.3）。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。版の番号の上げ方は `/release` で決める。
- **`v0.1.0` の後**・`audit-tools-perf-judge`（2026-11-15）: 動きが変わる点＝性能改善ループの道具（Python）の出し方が変わる。⑴ `judge-perf` は 0.5.0 になり、最後の行が `JUDGE-PERF RESULT code=N` になる（想定外の例外は `code=3`）⑵ `perf-compare` は目標の値が不正なとき終了コード 3 で終わる ⑶ `perf-rank` は 0.2.0 になり、最後の行が英語になる ⑷ `perf-ledger` は使われていなかった引数を外した＝`set-phase --run`・`status --max-streak`／`--max-iterations`・`final --best-idle-cpu`／`--commits`／`--iters` が無くなり、`append --iteration` が必須になった ⑸ 端末の標準エラーへ出す文は ASCII だけになった（`perf-ledger` の失敗の文だけは今までどおり）。製品の本体の動きは変わらない。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。版の番号の上げ方は `/release` で決める。
- **`v0.1.0` の後**・`audit-wintf-input`（2026-11-15）: 動きが変わる点＝⑴ 1 回の画面更新の間にドラッグの終わりと次の始まりが重なっても、次のドラッグが正しく進む ⑵ 始めの窓の位置が取れないとき窓を動かさず `warn!` を 1 行残す ⑶ 同じ窓を同じ HWND で登録し直してもクリック透過の記録が崩れない ⑷ wintf の窓（`Window`＋同じ HWND の `WindowHandle`）でない対象はクリック透過から外れる ⑸ `PointerState.local_point` が当たった entity の左上からの座標になる ⑹ 押す・離す・ドラッグの始まりと終わりの記録が `info!` から `debug!` になった（`RUST_LOG` で見ている人は `debug` へ開ける）。変わる点＝crates.io の `wintf` の公開面（壊れる変更）＝`DragConfig.right_button`・`middle_button`、ドラッグの知らせ 3 つの `is_primary` と `Messages` への登録、`DragStateSnapshot`・`cleanup_drag_state`・`CaptureGuard` の再公開、`CursorVelocity`・`PositionSample`・`PointerBuffer`・`PointerState.velocity`・`timestamp`、仮の当たり判定 2 つが消え、`DraggingState.initial_inset` は `initial_window_pos` へ改めた（全部の表は `completed/audit-wintf-input/design.md` の冒頭）。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。版の番号の上げ方は `/release` で決める。
- **`v0.1.0` の後**・`audit-placement-core`（2026-11-15）: 動きが変わる点＝⑴ 片軸だけの `balloon.offsetx`／`offsety` は書かれた軸だけを使い、書かれない軸は 0 になる（81）⑵ sylphya の保存・消しが、同じスコープの別の綴りの鍵（`"00"` など）も消す（116）⑶ descript の `alignmenttodesktop`・`balloon.alignment`・`balloon.dontmove`・`balloon.syncscale`・`windowposition.x` の語・`seriko.zorder` の語を、大文字小文字を問わずに読む（296 ほか・`\![set,zorder]` も同じ）⑷ 記憶の位置を寄せた起動では、古い記憶へ隣り合いの印を書き足さない（391・434）⑸ バルーンの切り替えで作者の DPI と `windowposition.limit` を入れ直し、シェルの切り替えは今の表情の値で解く（454・343・472）⑹ 起動の確定・印の保存・拡大率の詰め直しが `chain_rule.rs` の 1 つの決まりに従う＝`defaultx` の隙間を保ち、左右の辺に寄せたスコープは鎖の基準にしない（485・494・495・513〜515・520）。詳細は `completed/audit-placement-core/research.md` の 18。材料: 見どころ（`doc/release-notes/README.md` の「動きが変わる点」）。版の番号の上げ方は `/release` で決める。
- **`v0.1.0` の後**・`audit-tools-perf-run`（2026-11-15）: 動きが変わる点＝性能改善ループの道具（PowerShell）の出し方が変わる。⑴ 子の起動を `perf-tool-base.ps1` の `Invoke-Child` にまとめた ⑵ 入力が不正なとき終了コード 3 で終わる＝`-RunDir`・followup の `-BinDir`・compare の `-Iter`・負の再試行の値 ⑶ `-Resume` のとき、途中で止まった走行の記録は `-INCOMPLETE-` の名前へ移される ⑷ `check-quiet` の想定外の終わり方は、走行の前も後も終了コード 4 になった ⑸ 段 ③ の採取は走らせる側の `-SampleEtl` が始め、失敗したら `-Cancel` で止める ⑹ `SKILL.md` の REMEASURE は、`code=3` で `exe_sha256` が前と同じなら `MEASURE_FAILED`（理由 `same_executable`）で終わる。製品の本体の動きは変わらない。

## ウェーブ編成（着手順の正本・2026-11-27 棚卸㊹）

> 各ウェーブは**フルライフサイクル**（要件 → 設計 → タスク → 実装 → `/kiro-complete`＝PR の squash マージ）を完走してから次へ。1 spec ＝ 1 worktree ＝ 1 PR。**同じウェーブに入れるのは、棚卸㉞までの実測で触るソースファイルの重なりが 0 の組だけ**。許す重なりは 5 つだけ＝表の末尾への追記（`doc/COMPAT_ARCHITECTURE.md` §8）・生成物（`THIRD-PARTY-NOTICES.md`・`doc/ukadoc-coverage/report/`＝手で直さず作り直す）・steering・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の**別々の行**・説明書 `dist/README.txt` の**別々の行**。文書だけの段は置かない。
> **並び順の決まり（2026-10-07 開発者・10-05 の 3 段を改めた）**: 依存の木（働きの依存＋同じソースファイルを触る spec＋直列の列の順）の**先頭**だけを候補にし、**① ぶら下がる未完了の spec が多い ② 区分が高い（A ＞ B ＞ C ＞ D）③ 起票が古い**の順に並べ、上から互いに触るファイルが重ならないものを**最大 8 本**取る。据え置き・保留・取り下げ予定は木に入れない。**測ることが本体の spec はウェーブに並べない**（ほかの仕事を汚し、汚される）。区分 A でぶら下がる数が同じものは、開発者が「高」「早めに」と言ったものを先にした（棚卸㉓の読み＝違えば直す）。
> **版の段が最上位の物差し（2026-10-15 開発者・`/kiro-discovery`「版の段」・2026-10-16 改め）**: **今の版（まだ公開していない最も若い版）が残っているあいだ、後の版の spec はウェーブに入れない**（2026-10-16 開発者「ゴールが近いので他の版の割り込みは避けて」）。空いた席に入れてよいのは「版に縛らない」列のうち、今の版の着地までのウェーブ数を増やさないものだけ（同日・開発者「版に含めてもよいが、その場合、0.0.3 の着地（ウェーブ数）が増えることのないように」）＝先頭で、今の版の spec と触るファイルが重ならず、今の版が使う 1 本だけの席を取らず、見立ての上限がそのウェーブで一番大きい今の版の spec の下限以下で、人や外の待ちも、静かな机も、前後の全体テストの比べも要らないもの（判定の正本は `/kiro-next` のスキルの「Version gate」）。迷ったら席は空けておく。今の版の中は上の並び順の決まりで並べ、先頭を全部取ってから、空いた席を版に縛らない列の先頭（同じ並び順）で埋める。版に縛らない列の後続は待つ相手が着地してから・v0.2.0 の間は改造に触れないものだけ（「版の段」の注意）。
> **1 ウェーブに 1 本だけの席**: ⑴ 依存を足す spec と版上げ（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md`）＝C28 は `sylphya-sqlite-kvs`（rusqlite を足す）・見込みの C29 は空き・C30 は `instant-to-interrupt-ticks`（時刻の型を使う 7 つの crate に `dola` を足す）・C31 は空き ⑵ emo-text に新しいソースファイルを足す spec（`lib.rs` の純粋な一覧の数の行）＝C28 は空き・見込みの C29 は `balloon-text-visual-tree`。

| Wave | ユニット（並べた順） | 開始コマンド | 編成根拠（触るファイルと、同じウェーブの間の約束） |
|---|---|---|---|
| **C28**（6 本・v0.2.0 6 本・Fable 1・Opus 5） | ① `draw-methods-canon`（A・18〜24・`overlay`・`base` 以外の描画メソッドを D2D の道で描く・ぶら下がる数がいちばん多い＝`draw-methods-blend`・`element-clipping-option`・`compose-f16-precision`・`ghost-alias-file`）② ✅ `sylphya-sqlite-kvs`（11-28 着地）（A・20〜26・sylphya の保存を sqlite へ移し 1 つの KVS の口で扱う・`mcp-set-property` がこれを待つ）③ ✅ `talk-script-metadata`（11-28 着地）（A・7〜11・台本ごとの付帯情報＝ゴーストの置き場と送り手）④ ✅ `balloon-script-offset-align`（11-28 着地）（C・8〜12・台本の `\![set,balloonoffset]`・`\![set,balloonalign]`）⑤ ✅ `talk-thread-count-growth`（11-28 着地）（B・5〜10・トークのたびにスレッド名簿の項目が増える）⑥ ✅ `seriko-talk-clock-fidelity`（11-28 着地）（B・2〜4・口パクの時刻の残り 2 件） | 下の塊 | 触るファイル＝① `crates/areka-parsers/src/shell/{model,decode,undrawn}.rs`・`crates/areka-emo-compose/src/`（`method`・`fold`・`normalized`・`plan`・`plan_always`・`nesting`・`blit`・`bind_steps`・`plan_bounds`・`plan_issues`・`blit_rect`）・`crates/areka-emo-present/src/`（`display`・`mount`・`cache`・`shell_target` 933 行・`presenter/{budget,read,hit}`）・`crates/areka-runtime/src/balloon_background.rs`・本体の `offline_dump/render.rs`・seriko の `table.rs`（969 行＝上限の近く・期待の 1 か所だけ）・`Element` を組む 17 ファイル（7 crate） ② `crates/areka-sylphya/` の全部・`crates/areka-session/src/boot_resolve.rs`・`roots/mod.rs`・本体の `roots/edit.rs`・`crates/areka-placement/src/persist.rs`・`crates/areka-ghost/src/{sylphya_wiring,runtime,prop_sink}.rs`・`runtime_sink: None` を書くテストとその支え（6 crate の 14 ファイル）・根の `Cargo.toml`・`Cargo.lock`・`deny.toml` ③ `crates/areka-talk/`・kanade の `schedule/`・`actor.rs`・`msg.rs`・`crates/areka-ghost/src/{config,dispatcher,sink}.rs`・`crates/areka-runtime/src/{talk_lifecycle,user_break_cue}.rs`・`crates/areka-sakura/src/{drive,contract}.rs`・`crates/areka-session/src/readme/opener.rs`・本体の `mcp/sakurascript.rs` ④ `crates/areka-runtime/src/{consumer_ledger,consumer_ledger_test_support,move_cue}.rs`・`frame/{move_hold,drain_resnap,dpi}.rs`・`crates/areka-placement/src/{reseed,balloon_limit,lib}.rs`・`follow/{window_move,balloon_drag}.rs`・値を持つ新しいファイル `script_balloon.rs`（`follow/window_move.rs` は上限の例外＝差し引きの行を増やさない） ⑤ `crates/wintf/src/ecs/world/thread_registry.rs`・本体の `perf_thread_report.rs`・`tools/perf/perf-rank.py` ⑥ seriko の `trigger.rs`（送りの持ち方を直すなら `looper.rs`）。**同じウェーブの約束**: ⑴ `draw-methods-canon` は `areka-emo-compose` の `Cargo.toml` に依存を足さない（D2D の命令の並びは `areka-emo-present` で組む＝`Cargo.lock` の席は `sylphya-sqlite-kvs` だけ）。画素の読み戻しの移しは `presenter/read.rs` の中で済ませ、本体の `mcp/dump_surface.rs` に触らない。 ⑵ `talk-script-metadata` は `crates/areka-ghost/src/runtime.rs`・`runtime_tests.rs` に触らない（ゴーストの置き場は `config.rs` の `resolve_kanade_config` で運ぶ）。 ⑶ `talk-thread-count-growth` は名簿・報告・`perf-rank.py` だけを直す。生きたスレッドが残ると分かったら、トークの側の直し（`crates/areka-sakura/src/drive.rs`・`crates/areka-ghost/src/dispatcher.rs`）は `talk-script-metadata` の着地の後。 ⑷ `balloon-script-offset-align` は台本の値を保存しない形で進め、`crates/areka-placement/src/persist.rs` と `runtime_sink: None` を書くテストの支え（`follow_test_support.rs`・`persist_finalize_save_tests.rs`・`crates/areka-runtime/src/` の `frame_persist_test_support.rs`・`move_cue_apply_move_tests.rs`・`placement_tests/reset_balloon_pos_tests.rs`）に触らない＝値は新しいファイルに、テストも新しいファイルに置く。要件の段で保存すると決めたら、そこで止めて `sylphya-sqlite-kvs` の着地の後へ回す。 ⑸ `sylphya-sqlite-kvs` は保存の鍵の型と書き手の口の形を保つ（鍵だけを使う側のファイルに触らない）。 ⑹ `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳は別々の行だけ（許す重なり）。 |
| **次のウェーブの候補**（次の棚卸で組む） | **今の版は v0.2.0**（15 本・まだ公開していない）。C28 の後の見込みは下の「その先のウェーブ」（C29〜C31）。windows-rs の移行の本体（6〜8 本の見込み）は、上流の 0.101 系の crates.io への公開を待つ＝今の見込みに数えていない（覚え書き「windows-rs の移行の本体」）。移行の本体と保留の `tick-gate-adoption` を数えない v0.2.0 の着地は**約 4 ウェーブ（C28〜C31 の見込み・席と重なりで C32 に溢れうる）**。C28 に入らなかった先頭は 3 本＝`balloon-text-visual-tree`（`draw-methods-canon` と `crates/areka-emo-present/src/mount.rs`・`crates/areka-emo-text/tests/support/mod.rs`、`balloon-script-offset-align` と `crates/areka-runtime/src/frame/drain_resnap.rs`）・`ghost-switch-balloon-fallback`（`sylphya-sqlite-kvs` と `crates/areka-session/src/boot_resolve.rs`）・`instant-to-interrupt-ticks`（`sylphya-sqlite-kvs` と `crates/areka-sylphya/src/persist/io.rs` と依存の席、`talk-thread-count-growth` と本体の `perf_thread_report.rs`）。版の外の spec を待つ v0.2.0 は 0 本＝待つ相手の 4 本（`extent-element-offset`・`animated-image-import`・`property-query-channels`・`background-result-tick-wake`）を v0.2.0 へ入れた。4 本とも C28 には入らない（`extent-element-offset`・`animated-image-import` は `draw-methods-canon` と同じシェルの element の列、`property-query-channels` は `talk-script-metadata` と kanade の `msg.rs`・`schedule/`、`background-result-tick-wake` は `instant-to-interrupt-ticks` の後）。空いた 2 席は空けた（版に縛らない列に、割り込みの条件を満たすものが無い＝「棚卸㊹の裁定」の 7）。待ちの突き合わせは各 brief の最新の「再測定」の節 | — | — |
| **ウェーブの外** | 測ることが本体（静かな机で 1 本ずつ）＝`present-emit-tail-latency`・`install-live-target-hazards`・`real-run-input-guard-observations`・`helper-first-load-refusal`・`pasta-idle-surface-cost-measure`（2026-11-14 `wintf-visual-damage-redraw` の完了の棚卸で起票・同 spec の着地の後）／外の待ち＝`winget-release-automation`（winget-pkgs の手提出 PR 450070〔v0.0.2〕が取り込まれたら席の空きに挟む）／余力の枠＝`multi-ghost-runtime` → `ghost-call`（v0.1.0 は公開した＝行き先は開発者の決め・history「2026-11-06 棚卸㉟退避」の旧「棚卸㉞の裁定」の 9） | — | 机の空きを開発者が決めたときに 1 本ずつ挟む。重い走行は赤が出たテストに絞り、全体の負荷の走行は最後の 1 回だけ（1 回 約 50 分） |
| **保留** | `tick-gate-adoption` | — | 「長い試行はしない」と両立する短い A/B の測り方を先に組む。計測を汚すので他と並べない |

**C28 の開始コマンド**（`/kiro-next` の報告と同じ形・俯瞰の HTML がこの塊をコピーのボタンつきで出す・`/kiro-wave` はこの塊と spec 台帳の Fable の列を読む）:

```
Wave C28 6本（v0.2.0 6本）
【本来の次の版 v0.2.0】
Fable
/kiro-start sylphya-sqlite-kvs
Opus
/kiro-start draw-methods-canon
/kiro-start talk-script-metadata
/kiro-start balloon-script-offset-align
/kiro-start talk-thread-count-growth
/kiro-start seriko-talk-clock-fidelity
```

### その先のウェーブ（2026-11-27 の見込み）

見込みは次の棚卸で測り直す（C28 が全部着地したと置いて組んだ・v0.2.0 の spec だけで組んだ＝割り込みはその棚卸で選ぶ。windows-rs の移行の本体はまだ起票していないので入れていない）。`balloon-text-visual-tree`・`ghost-switch-balloon-fallback`・`instant-to-interrupt-ticks` は測り直した。ほかは前の測りのまま運んだ（重なりは次の棚卸で測り直す）。

| ウェーブ | spec | 入る条件 | 要件定義 |
|---|---|---|---|
| C29（見込み・7 本） | `balloon-text-visual-tree`・`ghost-switch-balloon-fallback`・`mcp-set-property`・`draw-methods-blend`・`ghost-alias-file`・`extent-element-offset`・`property-query-channels` | C28 の全部の着地（`balloon-text-visual-tree` は `draw-methods-canon` と `balloon-script-offset-align`、`draw-methods-blend`・`ghost-alias-file`・`extent-element-offset` は `draw-methods-canon`、`ghost-switch-balloon-fallback`・`mcp-set-property` は `sylphya-sqlite-kvs`、`property-query-channels` は `talk-script-metadata`・`sylphya-sqlite-kvs`・`balloon-script-offset-align`〔`consumer_ledger.rs`〕の後）。emo-text の席は `balloon-text-visual-tree`。`extent-element-offset`・`draw-methods-blend`・`ghost-alias-file` は同じシェルの element の列＝7 本の重なりは次の棚卸で測り直す（溢れたら C30 へ） | どれも Opus |
| C30（見込み・2 本） | `instant-to-interrupt-ticks`・`animated-image-import` | C29 の全部の着地（`instant-to-interrupt-ticks` は 12 の crate の 43 ファイルに触り、`balloon-text-visual-tree`・`ghost-switch-balloon-fallback`・`talk-thread-count-growth`・`sylphya-sqlite-kvs` と同じファイルを持つ・`Cargo.lock` の席を使う。`animated-image-import` は `extent-element-offset` の後〔`plan_extent.rs`〕） | どれも Opus |
| C31（見込み・4 本） | `compose-f16-precision`・`element-clipping-option`・`shiori4-api`・`background-result-tick-wake` | C30 の全部の着地（`compose-f16-precision` は `balloon-text-visual-tree`・`draw-methods-blend`・`instant-to-interrupt-ticks`、`element-clipping-option` は `extent-element-offset`・`animated-image-import`・`ghost-alias-file`、`shiori4-api` は `property-query-channels`・`talk-script-metadata`・`sylphya-sqlite-kvs`・`ghost-switch-balloon-fallback`・`instant-to-interrupt-ticks`、`background-result-tick-wake` は `instant-to-interrupt-ticks` の後）。`shiori4-api` は 20 タスクを越える見込み（要件の段で切る）。`compose-f16-precision` と `element-clipping-option` は表示の側で重なりうる＝溢れたら C32 へ | どれも Opus |

C31 の後も v0.2.0 は `tick-gate-adoption`（保留・`background-result-tick-wake` の後）と windows-rs の移行の本体（上流の 0.101 系の crates.io への公開を待って起票し、全部のクレートを一気に入れ替える）が残る。spec が 0 本になったら `/update` → `/release 0.2.0`。`doc-compat-architecture-rename` は、`doc/COMPAT_ARCHITECTURE.md` に行を足す spec と同じウェーブに置けない（C28 は `draw-methods-canon`・`balloon-script-offset-align` が §8 に行を足す）。

```
Wave C29（見込み）7本（v0.2.0 7本）
【本来の次の版 v0.2.0】
Opus
/kiro-start balloon-text-visual-tree
/kiro-start ghost-switch-balloon-fallback
/kiro-start mcp-set-property
/kiro-start draw-methods-blend
/kiro-start ghost-alias-file
/kiro-start extent-element-offset
/kiro-start property-query-channels
```

```
Wave C30（見込み）2本（v0.2.0 2本）
【本来の次の版 v0.2.0】
Opus
/kiro-start instant-to-interrupt-ticks
/kiro-start animated-image-import
```

```
Wave C31（見込み）4本（v0.2.0 4本）
【本来の次の版 v0.2.0】
Opus
/kiro-start compose-f16-precision
/kiro-start element-clipping-option
/kiro-start shiori4-api
/kiro-start background-result-tick-wake
```

### 版上げと winget の決まり（棚卸㉒の「初回リリースの段取り」のうち今も効くもの）

- **版上げの PR を出すとき、`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR は 0 本**。ほかの PR が開いていても、タグは版上げの squash コミットに打つので Release の中身は決まる。字の上で衝突しないまま `Cargo.lock` が古くなる（`--locked` の組み立てが落ちる）のを防ぐため、依存を足す spec とは同じ時に走らせない。C5 の席は `mcp-stdio-bridge` が使い、10-11 の着地で空いた。`/release` の `v0.0.3` を打つのは、「版の段」の v0.0.3 の 5 本（`baseware-root-list`・`user-data-root`・`write-failure-notice`・`balloon-text-bugs`・`app-icon`）が着地してから（2026-10-15 開発者）。2026-10-17 に `emily-ghost-verification` を v0.0.3 へ足した（開発者）＝この 6 本目の着地も待つ（✅ 10-18 着地）。
- **winget の順の決まり**: winget-releaser は既に winget-pkgs に在るパッケージしか更新できない。初回の手提出（人の承認・実例で約 2 日）が取り込まれるまで `winget.yml` を main へ入れない（入れた後にタグを打つと `winget.yml` が赤になる）。棚卸㉓で、手提出まで（`winget-manifest-submission`）と、取り込まれた後の自動化（`winget-release-automation`）に分けた。✅ `winget-manifest-submission` は 2026-10-10 に着地した（手提出の PR＝https://github.com/microsoft/winget-pkgs/pull/450070 ・自動の検査は通過・人の承認待ち＝まだ取り込まれていない）。`winget-release-automation` を始めるのは、**この PR が winget-pkgs に取り込まれ、かつ `user-data-root` が着地してから**（✅ `user-data-root` は 2026-10-17 に着地＝残るのは取り込み待ちだけ）（それまで、説明書に winget の行を載せず、winget-pkgs へ次の版を出さない＝「winget-manifest-submission の手元の確かめで見つかった件」の「順の決まり」）。2026-11-01 開発者「winget-release-automation これは0.1.0では無理でしょう。まだ0.0.2が通っていません。」＝v0.1.0 の版の門から外し、版に縛らない列へ移した（v0.1.0 は winget を待たずに出す・説明書に winget の行はまだ載せない）。

### 直列の列（同じ列の spec は同時に走らせない・列が違えば並べられる）

棚卸⑳の再測定で、**同じ場所へ行を足す spec の群れ**が見つかった。ファイルを分割しても重なりは消えない（同じ表・同じ関数へ足すため）。ウェーブは、各列の先頭から 1 本ずつ取り、取った組の接触ファイルを照合してから組む。棚卸㉒で kanade の列の並びを優先度の段に合わせ、シェルの element の列から `self-alpha-declaration` を外した。

| 列 | 重なる場所 | 並び（先頭から・✅＝完了） |
|---|---|---|
| **文字とバルーン** | `crates/areka-emo-text/src/`（`state.rs` の `CueCommand::Custom` の腕・`actor*.rs`・`viewbox_draw*.rs`・**`lib.rs` の純粋な一覧**＝新しいファイルを足す spec はどれも触る）・`crates/areka-parsers/src/balloon/{model,parse}.rs`・`crates/areka/src/input_events/` のバルーンと箱（`shell_box.rs` の `judge_box_press`・`user_break.rs` の `on_box_press`） | ✅ `emo-text-file-split` → ✅ `shell-balloon` → ✅ `shell-balloon-frame-align` → ✅ `budoux-reveal-reflow`（バグ・C4-②・10-06 着地）→ ✅ `anchor-tag-canon`（働き・C5-①・10-10 着地）→ ✅ `balloon-text-bugs`（v0.0.3・C7-②・10-16 着地）→ `range-choice-tag` → `link-context-copy` → `balloon-link-hover`（10-05 起票の「バルーンのリンクと OS の連携」・開発者の依頼で優先度の高い機能として `text-typesetting` 以降より前へ置いた）→ `balloon-font-file`（`actor_present.rs`・`lib.rs` が ② と、`actor.rs`・`viewbox_draw_render.rs` が `anchor-tag-canon`〔✅ 10-10 着地〕と重なる。リンクの列のどこかと重ならない席が見つかれば前へ）→ `text-typesetting`（② の「出した字の行を動かさない」約束の上に乗る）→ `talk-fast-forward` → `balloon-markers` → `text-ruby` → `text-reveal-fade` → `balloon-scroll-fade` → `text-align-shadow-canon` → `choice-marker-styling` → `anchor-style-canon`（`look.rs`・`state_decoration.rs`・`viewbox_draw_render.rs`・`viewbox_draw_decoration.rs` を分け合うので直列）→ `text-reveal-dance`（夢）。**`choice-marker-styling` を列から外して並べられた相手（`budoux-reveal-reflow`）は着地した**。`choice.rs` を分け合っていた `anchor-tag-canon` も 10-10 に着地した＝棚卸㉔の実測では、新しいファイルを足さなければ今の列の頭（`range-choice-tag`）と並べられる。**列の共通の約束**: 新しいキーはバルーン定義ごとの値（`ResolvedBalloonText` の側）に置く。fade・scroll-fade・dance は「動いている途中の字を見えている字に数えるか」の答えを、当たりの矩形・箱の写し（`actor_box.rs` の `refresh_shown_boxes`）・窓を出す数（`balloon_shown_glyphs`）の 3 か所で揃える。emo-text の `judge_box_click`・`BoxPressVerdict` は 2 値でなく 4 値（棚卸㉑の記述の誤り・棚卸㉒で直した） |
| **シェルの element** | `crates/areka-parsers/src/shell/{model,decode}.rs`・`crates/areka-emo-compose/src/{plan,plan_extent,plan_always,fold,method,atlas_bind}.rs`・`crates/areka-emo-atlas/src/manifest.rs`・`crates/areka-seriko/src/{table,looper,parts,actor}.rs` | ✅ `shell-balloon` → ✅ `surface-element-nesting` → ✅ `element-base-method`（10-05）∥ ✅ `animated-image-playback`（本体・C4-⑫・`bind_atlas` の直後で分解・10-07 着地）→ **`extent-element-offset`（C5-④・`plan_extent.rs` だけ）** ∥ ✅ `seriko-trigger-intervals`（C5-②・10-10 着地・棚卸㉓で分けた後＝読み手の `shell/{model,decode}.rs` と seriko だけ）→ `animated-image-import`（読み手・`manifest.rs`・`plan.rs`・`plan_extent.rs`・seriko の `table.rs`・`parts.rs`・棚卸㉕で前へ）→ `collisionex-regions`（読み手・v0.2.0）→ `seriko-interval-combinations`（棚卸㉓で切り出し・`plan_extent.rs`・`plan.rs`・`nesting.rs`・`plan_always.rs`）→ `draw-methods-canon`（読み手と描けるかの門・`plan.rs`・`blit.rs`）→ `element-clipping-option`（2026-11-04 棚卸㉞で canon の後へ＝D2D の切り抜きで描く形）→ `draw-methods-blend`（棚卸㉓で切り出し・`method.rs`・`blit.rs`）→ `balloon-element-order`（wintf の兄弟の重なり順の裁定の持ち主）→ `surfaces-basepos`（据え置き）。parsers の `shell/{model,decode}.rs` を触る 5 本（`draw-methods-canon`・`collisionex-regions`・`seriko-trigger-intervals`〔✅ 10-10 着地〕・`animated-image-import`・`element-clipping-option`）と、compose の `plan.rs`（子の `plan_extent.rs` と、兄弟の `plan_always.rs` を含む）を触る 5 本（`draw-methods-canon`・`extent-element-offset`・`animated-image-import`・`element-clipping-option`・`balloon-element-order`）はそれぞれ互いに直列。**`self-alpha-declaration`（C4-⑬）は列から外した**（本番の決め打ちは `shell_target.rs`・`balloon.rs` の 2 か所だけで、`manifest.rs`・`atlas_bind.rs` の 3 か所は試験の中だった＝棚卸㉑の記述の誤り） |
| **kanade の進行** | `crates/areka-kanade/src/schedule/{steady,events,mod,change,boot}.rs`・`msg.rs`・`actor.rs`・`lib.rs` の `pub use`。**`steady.rs` 957 行・`schedule/mod.rs` 969 行・`msg.rs` 930 行・`actor.rs` 901 行（10-10 `anchor-tag-canon`・`farewell-talk-status` の後）は上限の近く**＝足して 1,000 を超える spec が先頭のタスクで分割するか、新しいファイルへ足す | ✅ `translate-pipeline` → ✅ `mouse-drag-events` → ✅ `balloon-lifecycle-events`（C4-⑩・10-08・`schedule/`）∥ ✅ `mcp-get-status`（C4-⑥・10-06・`msg.rs`・`actor.rs`＝列の外へ出せる組）∥ ✅ `choice-script-prefix`（C4-⑭・10-07・`schedule/choice.rs` とその子・`steady.rs` の `on_choice`＝⑩ と ⑥ のファイルに触らない組。`script:` の台本をトークとして走らせる口は、後の `mcp-kanade-tools` の `sakurascript` が使い回せる）→ `mcp-kanade-tools`（`sakurascript`・`raise_event`・許可の表の迂回を作る）→ ✅ `mcp-reload`（10-22 着地・`src/change.rs`・`schedule/boot.rs`）→ `script-security-level`（出どころの運搬・host32-host の列も）→ `mcp-strict-errors` ／ その他の段＝`sakura-time-critical` → `property-query-channels`（棚卸㉒で優先の段の後ろへ下げた） |
| **台本のコンパイル** | `crates/areka-sakura/src/compile.rs`・`crates/areka-parsers/src/sakura/{lexer,decode,model}.rs` | ✅ `choice-timeout-directive` → ✅ `open-external-tags`（C4-⑮・`\j` の腕）→ ✅ `mcp-author-tools`（C4-⑨・`parse_noted` の印＝`decode.rs` の各腕が命令と一緒に印を返す形・`compile.rs` の `parse_choice_timeout` の公開。後の spec はこの上で腕を足す）→ ✅ `anchor-tag-canon`（働き・C5-①・10-10 着地）→ `range-choice-tag`（`\__q` の腕）→ `talk-fast-forward` → `sakura-time-directives` → `balloon-break-position`（その他の段・`TalkDone` まで位置を通す）。`\i[ID]` は棚卸㉓で `seriko-script-triggers` へ切り出した（`compile.rs`・`sakura/decode.rs` を触る＝✅ `anchor-tag-canon`〔10-10 着地〕の後）。`sakura-time-critical` は `\t` を `decode_bare` で写せば `compile.rs` に触れないが、`sakura/decode.rs` はこの列の共有のファイルなので列の中に居る（棚卸㉓）。`window-timed-directives`（2026-11-19 起票・v0.4.0・`compile.rs` に時間つきの指令の長さを書く）もこの列 |
| **host32-host** | `crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/{runtime.rs, shiori_inproc.rs}`・kanade の `shiori/real.rs` | ✅ `script-security-level`（`SenderType`・`SecurityLevel` の運搬の持ち主＝棚卸㉒・10-23 着地）→ `mcp-shiori-query`（`X-MCP-PassThru-*`）→ `makoto-dll-host`（新規 `makoto.rs`・`runtime.rs`・`Cargo.lock`）→ `makoto-reload-directives` → `property-ipc-transport`（輸送を作るときだけ）。`property-query-channels` は運搬を `script-security-level` に寄せて列から外れた |
| **配布と公開** | `tools/package.ps1`・`.github/workflows/`・`Cargo.toml` 群・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（版上げで約 30 行）・`dist/README.txt`・`README.md` | ✅ `release-package-versioned` → ✅ `release-ci-workflow` ∥ ✅ `crates-io-publish` → ✅ `tools-utf8-child-output`（バグ・10-06 着地・`package.ps1`・`crates-io.ps1`・`release.yml` の読み方）→ ✅ `release-cycle` の初回（`v0.0.2`・10-06）→ ✅ `release-notes-highlights`（10-11 着地・`release.yml`・`tools/release-notes.ps1`・`doc/release-notes/`・`release-cycle` の文書）→ ✅ `winget-manifest-submission`（C5-⑧・10-10 着地・手提出の PR は自動の検査を通過して人の承認待ち）→ ✅ `user-data-root`（C8-①・10-17 着地）→ `winget-release-automation`（棚卸㉓で切り出し・手提出が winget-pkgs に取り込まれ、かつ `user-data-root` が着地してから）∥ ✅ `mcp-stdio-bridge`（10-11 着地）→ `install-live-target-hazards`（`dist/README.txt` の既知の制限）→ `release-code-signing`（任意・同梱の i686 `pasta.dll` は署名されないまま残る＝議題） ∥ `release-notes-highlights`（10-11 起票・触るのは `release.yml` と `release-cycle` の文書＝`release-cycle` の次の回〔`v0.0.3`〕より前に着地・`release-code-signing` とは `release.yml` で直列） |
| **`emo2_boot` の結線** | `crates/areka/src/emo2_boot/{mod,ghost_switch,consumer_ledger,shell_balloon_switch,switch_assets}.rs`・`frame/{attach,switch,wiring}.rs` | ✅ `shell-balloon-frame-align` → ✅ `balloon-lifecycle-events`（C4-⑩・10-08・`mod.rs`）→ `balloon-font-file`（`frame/attach.rs`・`frame/switch.rs`）→ `balloon-canon-residue` ∥ `shell-companion-balloon`（`frame/switch.rs` を分け合うので同時には走らせない）→ ✅ `mcp-reload`（10-22 着地・`mod.rs`・`ghost_switch.rs`・`shell_balloon_switch.rs`）。`consumer_ledger.rs` は ✅ `open-external-tags`（C4-⑮・`open` の受け取り手・21 行）と ✅ `mcp-author-tools`（C4-⑨・2 行を足して 23 行）が先に触り、`mcp-reload` は後→ `extra-character-windows`（`switch_assets.rs` も）→ `currentghost-property-tree` → `currentghost-property-others`・`system-property-values` → `property-catalog-lists` |
| **プロパティの動く値** | `crates/areka-sylphya/src/{actor,mirror,key,reader,vocab/dotted}.rs`・`persist/`・`areka-ghost/src/sylphya_wiring.rs` | ✅ `property-name-case-fold`（10-23 着地・`classify_set` の大小の迂回をふさぐ＝`property-query-channels` が書く道を開く前に着地させる）→ `currentghost-property-tree`（動く値を出す口の持ち主・読む道は `mcp-get-property` が作った）→ `currentghost-property-others` ∥ `system-property-values` → `property-catalog-lists` → `zorder-property`。`scope-currentmonitor-property`（2026-11-19 起票・読み手を足すだけ・この列の spec と同時に走らせない） |

- **棚卸㉓で列に足したこと**（表の行は上の書き換えだけ・ここが足し分の正本）:
  - **列をまたぐ重なり**: 文字とバルーンの列の頭 ✅ `anchor-tag-canon`（10-10 着地）は、`OnAnchorSelect` を送るために kanade の `msg.rs`・`actor.rs`・`lib.rs`・`schedule/{events,mod}.rs` も触る＝kanade の列の頭（`mcp-kanade-tools`・✅ `farewell-talk-status`・`script-security-level`・`sakura-time-critical`）と同じウェーブに置けなかった（どちらも 10-10 に着地したので、この重なりは済んだ話）。
  - **`\!` の名前を足す spec の共有の席**（`mcp-author-tools` の着地でできた）: `check_script` は `crates/areka/src/emo2_boot/consumer_ledger.rs` に無い `\!` の名前を「知らない命令」と答える＝`\!` の名前を足す spec はどれも、この表に行を足し `consumer_ledger_agreement_tests.rs` に見本を足す（`mcp-reload`・`seriko-script-triggers`・`text-typesetting`・`text-ruby`・`text-reveal-fade`・`balloon-markers`・`sakura-time-directives`・`sakura-time-critical`・`property-query-channels`・`sakura-embed-directive`・`makoto-reload-directives`・`inputbox-user-input`）。互いに直列。
  - **カタログの鎖**（`crates/areka-ghost/src/catalog.rs`・`crates/areka/src/{boot_resolve,boot_config}.rs` を 3 本が作り直す）: `baseware-root-list`（C5-⑤）→ `ghost-inner-balloon` → `shell-companion-balloon`。バルーンの項目に「どこから来たか」の欄が要るのは前の 2 本＝先に着地する `baseware-root-list` が決める。`ghost-inner-balloon` の「`shell-companion-balloon` の後」は働きの依存でなくファイルの順だった。
  - **kanade の列**: ✅ `farewell-talk-status`（10-10 着地・`schedule/mod.rs` は判定の本体 1 か所だけ）→ `mcp-kanade-tools`（最初のタスクで `msg.rs` の中のテストを兄弟へ出し、`schedule/mod.rs` を割る。許可の表の迂回は `schedule/change.rs` の `on_raise_event` に作る。`src/change.rs`・`schedule/boot.rs` には触らない＝`mcp-reload` と並べられる）→ `script-security-level` → `mcp-shiori-query`（`mcp-kanade-tools` の後＝棚卸㉒の裁定どおり）→ `mcp-strict-errors`。`mouse-click-wheel-events`・`inputbox-user-input`（棚卸㉓で起票）は `schedule/events.rs` を触るのでこの列。
  - **シェルの element の列の読み手の鎖**（`shell/{model,decode}.rs`）: ✅ `seriko-trigger-intervals`（10-10 着地）→ ✅ `collisionex-regions`（11-17 着地）→ `animated-image-import` → `element-clipping-option` → `draw-methods-canon`。`config-parse-diagnostics`（棚卸㉓で起票）は読み手の `kv/parse.rs`・`shell/`・`balloon/parse.rs` を触るので、この鎖と文字とバルーンの列の両方が空いたとき。`balloon-canon-residue` は seriko の `actor.rs` を触るので ✅ `seriko-trigger-intervals`（10-10 着地）の後。
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
  - **`consumer_ledger.rs` の鎖**: 560 行・43 件の登記（棚卸㉝・✅ `shell-scale-linkage` が 11-01 に `(get,property)` の 1 件を足した後・手書きの数も 43。その前は 547 行・42 件＝棚卸㉜・✅ `script-small-tags` が 10-31 に `bind-noevent` と `*` の 2 件を足した後。その前は 529 行・40 件＝✅ `stay-on-top-directives` が 10-31 に 2 件足した後）。行を足す spec は、表（`crates/areka-runtime/src/consumer_ledger.rs` の `CANONICAL_ROWS`）への追記のほかに、`consumer_ledger_agreement_tests.rs` の `samples()` も直す（手書きの数はもう無い＝表が正本）。v0.1.0 で行を足すのは 5 本＝✅ `descript-window-keys`（10-24 着地）・✅ `stay-on-top-directives`（10-31 着地）・✅ `script-small-tags`（10-31 着地）・✅ `shell-scale-linkage`（→43・11-01 着地）・`balloon-communicatebox`（→45・2026-11-01 に windows-reactor 待ちへ移した）（＋後の版の `ghost-call`・✅ `mcp-strict-errors` は行を足さずに着地した）＝1 本ずつ。（2026-11-20 棚卸㊵: ✅ `audit-emo2-boot-spine` が登記を 1 つの表 `CANONICAL_ROWS` にまとめ、手書きの数は無くなった＝登記を足す spec は表に行を足すだけ）
  - **窓の位置の列**: `dpi-realign-remembered-chain` と ✅ `window-work-area-clamp`（10-30 着地） は `placement/{chain_finalize,persist}.rs`・`emo2_boot/frame/drain_resnap.rs` で直列。`dpi-realign-remembered-chain` と ✅ `stay-on-top-directives`（10-31 着地）は `placement/spawn.rs` で直列。✅ `descript-window-keys`（10-24 着地）の件 4 と `window-work-area-clamp` の件 2 は同じ崩れ＝前者が測った面、後者は「自由な窓が後から大きくなる」だけ（両 brief の棚卸㉙の案）。
  - **入力の列**: ✅ `mouse-click-wheel-events`（10-24 着地） → ✅ `shell-mouse-cursor`（10-30 着地・`input_events/` と新しい `hover_region.rs`）。`balloon-scroll-arrows` も `input_events/mod.rs` を触る＝どちらとも別のウェーブ。
  - **文字とバルーンの列**: `balloon-scroll-arrows` は emo-text の席を使う（新しいファイル）。`balloon-scroll-arrows` と `balloon-communicatebox` は `areka-parsers/src/balloon/{mod,model,parse}.rs` で直列。✅ `vertical-choice-hover-band`（10-24 着地）と ✅ `choice-ranges-one-function`（10-31 着地） は `choice.rs` が条件つき（後者が関数を `actor_present.rs` に置けば同じウェーブでよい）。`real-run-log-noise` は v0.1.0 へ上げた（`draw.rs` の呼び手で直せば `vertical-choice-hover-band` と重ならない）。
  - **棚卸㉙で起票した 4 本の置き場**: `balloon-inline-image`＝文字とバルーンの列（emo-text の席を使う・`areka-parsers/src/sakura/decode.rs` で `script-small-tags`・✅ `stay-on-top-directives`（10-31 着地） と重なる）・`ghost-alias-file`＝シェルの element の列（`shell/decode.rs`・emo-present の `shell_target.rs`）・`sstp-receive`＝kanade の列（`schedule/sstp.rs`・`msg.rs`）と MCP の待受（`crates/areka-mcp/src/server.rs`）・`install-package-type`＝インストールの列（`crates/areka-nar/src/manifest.rs`・`install/judge.rs`＝`startup-install-option` と重なる）。
- **棚卸㉚で列に足したこと**（2026-10-24・表の行は書き換えていない・ここが足し分の正本）:
  - **`consumer_ledger.rs` の鎖**: 560 行・43 件の登記（棚卸㉝）。行を足すのは ✅ `stay-on-top-directives`（`(set,windowstate)`・10-31 着地）・✅ `script-small-tags`（`bind-noevent`・`*`・10-31 着地）・✅ `shell-scale-linkage`（`(get,property)`・11-01 着地）・`balloon-communicatebox`（`(open|close,communicatebox)`）の 4 本＝1 本ずつ（✅ `mcp-strict-errors` は行を足さずに着地した）。新しい受け手を足す spec は `emo2_boot/mod.rs`（828 行）と `emo2_boot/runtime_boot.rs` にも 1 行ずつ足す（同じファイル）。✅ `shell-scale-linkage` は受け手を `areka-ghost/src/runtime.rs` に結び、`emo2_boot/mod.rs`・`runtime_boot.rs` には触らなかった（棚卸㉝）。`balloon-communicatebox` は kanade の `schedule/events.rs` の `ALLOWED_EVENT_IDS`（66→68・手書きの数は `events_change_tests.rs:47`）も直す。v0.1.0 の後は `balloon-script-offset-align`（v0.2.0・2 件）がこの鎖に入る。
  - **seriko の `actor.rs`（799 行）**: `mcp-strict-errors`（案 2＝再生の途中まで印を運ぶ）・`script-small-tags`（bind の濾し・541 行目と 564 行目）・`seriko-pattern-calls` が触る＝同じウェーブに置かない。emo-present の `presenter/show.rs` は `mcp-strict-errors` と ✅ `animated-surface-log-flood`（10-31 着地）が触る。
  - **窓の位置の列**: ✅ `window-work-area-clamp`（10-30 着地） は ✅ `dpi-realign-remembered-chain` の要件の引き渡し（並べ直しの寄せ・起動の隣り合わせの寄せ）で `placement/chain_realign.rs` も触る。寄せる純粋な関数は `placement/follow/` の下に置けば `placement/mod.rs`（969 行）に触らない。
  - **入力の列**: ✅ `shell-mouse-cursor`（10-30 着地） と `balloon-scroll-arrows` は `input_events/mod.rs` で直列（`shell-mouse-cursor` が mod の行を足さない約束なら並べられる）。`balloon-scroll-arrows` のホイールは ✅ `mouse-click-wheel-events` が wintf の `PointerState` へ運ぶ形の上に乗る（受け手の先例は `char_watch.rs`）。
  - **行数の圧**: `main.rs` 981・`placement/mod.rs` 969・`ghost_session.rs` 935・`placement/follow/window_move.rs` 1,221（例外）・seriko の `table.rs` 901・`looper.rs` 924・`areka-parsers` の `balloon/parse_tests.rs` 922・emo-text の `draw_format_metrics_tests.rs` 922。
- **依存を足す spec と版上げは 1 ウェーブに 1 本**（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md` が重なる）: C5 は `mcp-stdio-bridge`（新しいクレート・10-11 に着地済み）が席を使った。`impl-watch`（新しいクレート・10-10 に着地済み）・`makoto-dll-host`（新しいクレート）・`system-property-values`（`windows` の機能を足す）・`/release` の版上げも同じ席を使う。`mcp-strict-errors` も使う（10-23 の要件の討議＝rmcp を上げるか、上流への直しを `[patch.crates-io]` で持つ）。
- **網羅台帳をまとめて書き換える `coverage-roadmap-refresh` は、台帳の行を直す spec と同じウェーブに置かない**（台帳に触ると書く未完了の brief が 40 本を超えたので、行の重なりで照合する形を同 spec の議題にした）。
- **列に属さず、いつでも単独で取れる**: `coverage-roadmap-refresh`・✅ `popup-menu-residue`（10-31 着地）・`clippy-199-lints`（段 1＝`dola` の `validate/`・`runtime/`・`compile/` と `shiori-host32-host/tests/` は空き席でも取れる。`shiori-abi` は `shiori4-api` が消すので取らない）・✅ `property-name-case-fold`（10-23 着地・sylphya だけ）。
- **MCP の 3 段目の約束**: SSP と同じ 10 本の中身を作る spec は、今までどおりツールのファイル（`crates/areka/src/mcp/<ツール>.rs`）と自分のエンジンの側だけを触る（`mcp/mod.rs`・`handler.rs` は触らない）。areka 独自のツールを足す spec（`mcp-user-response`・`mcp-shiori-query`）は `doc/ssp-mcp/areka-tools.md` の「⑹ あとからツールを 1 本足す手順」の表のファイル（`crates/areka-mcp/src/tools/mod.rs`・`handler.rs`・`tools_own_tests.rs`・`tools_own_socket_tests.rs`・`crates/areka/src/mcp/mod.rs` と、その文書）を触るので、互いに、また MCP の共有ファイルを触る他の spec と同じウェーブに置かない。`handler.rs` の `INSTRUCTIONS` に残る「not implemented yet」の 1 文は、今までどおり最後に着地する `mcp-strict-errors` が消す（`mcp-author-tools` が足した独自のツールの 1 文は残す）＝`mcp-reload` は `mcp-strict-errors` より先に着地させる。今も `NG:not implemented yet` を返すのは 10 本のうち 0 本（`reload` は 2026-10-22 に `mcp-reload` が、`sakurascript`・`raise_event` は 2026-10-19 に `mcp-kanade-tools` が中身を入れた）。
- **SHIORI への要求に追加のヘッダを運ぶ道**は `script-security-level`・`property-query-channels`・`mcp-shiori-query` の 3 本が要る。入れ物は先に着手した 1 本が作り、`ShioriBackend` の引数を変えずに既定の実装を持つ新しいメソッドを足す形を推す（実装 19 か所への波及を止める）。✅ `script-security-level`（10-23 着地）がこの形で入れ物を作った（SHIORI の受け手の約束に既定の実装つきのメソッド 2 つ）＝残りの 2 本はそれを使う。
- **棚卸㊴で列の順を入れ替えたこと**（2026-11-17）: シェルの element の列で `collisionex-regions`（v0.2.0）を `animated-image-import`（v0.1.0 から外した 5 本）の前へ出した。棚卸㉕で `animated-image-import` を前へ出したのは、それが若い版（v0.1.0）に居たからで、働きの依存は無い（棚卸㊴の再測定・`collisionex-regions` が触るのは `shell/{model,decode}.rs` と emo-compose の `hit*.rs`、`animated-image-import` は `manifest.rs`・`plan*.rs`・seriko）。今は `collisionex-regions` の方が今の版なので、版の段の並べ方（若い版から）に合わせた。読み手の鎖（`shell/{model,decode}.rs`）は `collisionex-regions` → `animated-image-import` → `element-clipping-option` → `draw-methods-canon` のまま。
- **v0.2.0 の残りで同じファイルを触る組**（同じウェーブに置かない・矢印は取る順）: `draw-methods-canon` → `balloon-text-visual-tree`（`crates/areka-emo-present/src/mount.rs`・`crates/areka-emo-text/tests/support/mod.rs`）。`balloon-script-offset-align` → `balloon-text-visual-tree`（`crates/areka-runtime/src/frame/drain_resnap.rs`）。`sylphya-sqlite-kvs` → `ghost-switch-balloon-fallback`（`crates/areka-session/src/boot_resolve.rs`）。`instant-to-interrupt-ticks` は `balloon-text-visual-tree`（本体の `mcp/dump_balloon.rs`）・`ghost-switch-balloon-fallback`（`crates/areka-session/src/ghost_switch.rs`＝970 行・足す分は別のファイルへ）・`talk-thread-count-growth`（本体の `perf_thread_report.rs`）・`sylphya-sqlite-kvs`（`crates/areka-sylphya/src/persist/io.rs`）の後で、`tick-gate-adoption`・`background-result-tick-wake`（wintf の `tick_wake.rs`・`world/mod.rs`・`tick_bridge.rs`）と `compose-f16-precision`（本体の `mcp/dump_surface.rs`・`dump_balloon.rs`）の前。`seriko-talk-clock-fidelity` と `seriko-trigger-lottery-gaps` は seriko の `trigger.rs` で直列（働きの依存は無い）。

**保存義務（据え置き）**: 既存の終了経路（右クリックメニューの「終了」→ `OnClose` の握手 → `ghost_quit`）の決定論テストを 1 本も落とさない。実機サインオフの「絶対パス起動」（argv 上書き）を残す。

## spec 台帳（brief を持つ 116 本・`.kiro/specs/` 直下のフォルダの数と一致〔completed を除く〕）

> **spec は名前で呼ぶ**（2026-09-26 開発者指示）: 報告・brief・コミット・PR で spec を指すときは spec 名（完了済みと着手済みの旧い名前の `areka-P0-` は省略してよい・未着手の spec は 2026-10-23 から接頭辞なしの名前＝`kiro-spec-schema.md` の命名規則）を書く。「#数字」は `PR#185` の形の PR 番号にだけ使う。古い文書に台帳番号が出てきたら、その時点の表（history）で名前へ読み替える。
> **区分**: 「段」の列の末尾に **区分**（A 開発者が起票を頼んだ／B バグ／C ukadoc の拾い残し・バグでない持ち越し／D 夢・据え置き・保留）を書く。着手の並びは 4 区分（生きている決まり 1）。区分の数え＝**A 51・B 13・C 48・D 9 ＝ 121**。見出しと数の行には今の数だけを書き、移り変わりは書かない。段の語（バグ／優先／その他／据え置き／保留）は並びには使わない（据え置き・保留は依存の木に入れない印としてだけ読む）。区分の読みに迷った行（開発者の指示が「分け方」だけ・名指しの範囲外の切り出し）は C に置く。
> **段＝優先度の 3 段**（2026-10-03 開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」）: **バグ**＝1 段目／**優先**＝2 段目（配布と公開・文字とバルーンの列・シェルの element の列・動く画像・バルーンのイベントと残件・**SSP MCP の移植 10 本**＝10-03 に開発者が追加・`mouse-drag-events`＝10-04 に開発者が個別に上げた）／**その他**＝3 段目（ゴーストのインストール・kanade・プロパティ・道具）／**据え置き**＝当面着手しない（理由は行に）／**保留**。どのウェーブに居るかは「並び」の列。完了した spec はこの表に置かない（完了サマリと history）。
> **段の数え方**: `awk '/^\| spec（旧い `areka-P0-` は省略）/{f=1;next} f&&/^\|/{print} f&&!/^\|/{f=0}' .kiro/steering/roadmap.md | awk -F'|' 'NR>1{gsub(/ /,"",$3);print $3}' | sort | uniq -c`（2026-10-31 `placement-measure-bake-once` の完了の後の実数え＝バグ 13・優先 77・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 163〔優先 1 本〔同 spec〕を外し、その他 1 本〔`emo-bake-log-overlap`〕を足した〕。その前の 2026-10-31 `actor-thread-log-capture` の完了の後の実数え＝バグ 13・優先 78・その他 58・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 163〔その他 1 本〔同 spec〕を外した〕。その前の 2026-10-31 `popup-menu-residue` の完了の後の実数え＝バグ 13・優先 78・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 164〔その他 1 本〔同 spec〕を外し、その他 1 本〔`popup-menu-extra-resources`〕を足した〕。その前の 2026-10-31 `choice-ranges-one-function` の完了の後の実数え＝バグ 13・優先 78・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 164〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-31 `emo-atlas-palette-alpha` の完了の後の実数え＝バグ 13・優先 79・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 165〔バグ 1 本〔同 spec〕を外した・2026-10-31 棚卸㉛で足したその他 1 本〔`seriko-legacy-pattern-format`〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の 2026-10-31 `animated-surface-log-flood` の完了の後の実数え＝バグ 14・優先 79・その他 58・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 165〔その他 1 本〔同 spec〕を外した〕。その前の 2026-10-31 `stay-on-top-directives` の完了の後の実数え＝バグ 14・優先 79・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 166〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-31 `balloon-scroll-arrows` の完了の後の実数え＝バグ 14・優先 80・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 167〔優先 1 本〔同 spec〕を外し、その他 1 本〔`balloon-append-tags`〕を足した〕。その前の 2026-10-31 `seriko-pattern-calls` の完了の後の実数え＝バグ 14・優先 81・その他 58・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 167〔その他 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `bridge-store-user-root` の完了の後の実数え＝バグ 14・優先 81・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 168〔バグ 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `window-work-area-clamp` の完了の後の実数え＝バグ 15・優先 81・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 169〔バグ 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `window-work-area-clamp` の完了時の棚卸の起票の後の実数え＝バグ 16・優先 81・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 170〔バグ 2 本〔`balloon-flip-side-rules`・`window-resetwindowpos-hold`〕を足した。同 spec の完了〔バグ −1〕はまだ載せていない〕。その前の 2026-10-30 `emo-text-dwrite-fidelity` の完了の後の実数え＝バグ 14・優先 81・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 168〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `startup-install-option` の完了の後の実数え＝バグ 14・優先 82・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 169〔優先 1 本〔同 spec〕を外し、優先 1 本〔`install-before-boot`〕とバグ 1 本〔`ghost-switch-balloon-fallback`〕を足した。2026-10-30 の `mcp-strict-errors` の完了〔優先 −1〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の 2026-10-30 `real-run-log-noise` の完了の後の実数え＝バグ 13・優先 83・その他 59・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 169〔その他 1 本〔同 spec〕を外し、バグ 1 本〔`emo-atlas-palette-alpha`〕を足した〕。その前の 2026-10-30 `shell-mouse-cursor` の完了の後の実数え＝バグ 12・優先 83・その他 60・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 169〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-30 `default-balloon-selfmade` の完了の後の実数え＝バグ 12・優先 84・その他 60・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 170〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-28 `/kiro-discovery`〔本体の crate 分け〕の起票の後の実数え＝バグ 12・優先 85・その他 60・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本 ＝ 171〔優先 6 本を足した〕。その前の 2026-10-28 `/kiro-discovery` の起票の後の実数え＝バグ 12・優先 79・その他 60・据え置き 12・保留 1・段の欄が「v0.2.0」の 1 本〔`talk-script-metadata`・2026-10-25 の起票〕＝ 165〔その他 5 本を足した〕。その前の 2026-10-24 `mouse-click-wheel-events` の完了の後の実数え＝バグ 12・優先 79・その他 55・据え置き 12・保留 1 ＝ 159〔その他 1 本〔同 spec〕を外した。2026-10-24 の `emo-text-dwrite-fidelity` の起票〔優先 +1〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の 2026-10-24 `readonly-mode` の完了の後の実数え＝バグ 12・優先 78・その他 56・据え置き 12・保留 1 ＝ 159〔優先 1 本〔同 spec〕を外した。2026-10-24 の `mouse-hover-event` の起票〔その他 +1〕と `dpi-realign-remembered-chain` の完了〔バグ −1〕はこの行に書き足されていなかった＝この実数えで載せた〕。その前の 2026-10-24 `vertical-choice-hover-band` の完了の後の実数え＝バグ 13・優先 79・その他 55・据え置き 12・保留 1 ＝ 160〔優先 1 本〔同 spec〕を外し、バグ 1 本〔`emo-text-line-gap`〕を足した〕。その前の 2026-10-24 `descript-window-keys` の完了の後の実数え＝バグ 12・優先 80・その他 55・据え置き 12・保留 1 ＝ 160〔優先 1 本〔同 spec〕を外し、その他 1 本〔`seriko-missing-surface-default`〕を足した〕。その前の 2026-10-24 `shiori2-protocol` の完了の後の実数え＝バグ 12・優先 81・その他 54・据え置き 12・保留 1 ＝ 160〔その他 1 本〔同 spec〕を外し、バグ 1 本〔`helper-first-load-refusal`〕を足した〕。その前の 2026-10-23 `mcp-scope-rect-property` の完了の後の実数え＝バグ 11・優先 81・その他 55・据え置き 12・保留 1 ＝ 160〔その他 1 本〔同 spec〕を外した〕。その前の 2026-10-23 棚卸㉙の実数え＝バグ 11・優先 81・その他 56・据え置き 12・保留 1 ＝ 161〔バグ 1 本・その他 3 本を足した〕。その前の 2026-10-23 `pasta-in-windows-verification` の完了の後の実数え＝バグ 10・優先 81・その他 53・据え置き 12・保留 1 ＝ 157〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-23 `pasta-in-windows-verification` の枝に既定ブランチを取り込んだ後の実数え＝バグ 10・優先 82・その他 53・据え置き 12・保留 1 ＝ 158〔同 spec の申し送りの 6 本〔優先 3・その他 3〕を足した〕。その前の 2026-10-23 `offline-surface-dump` の完了の後の実数え＝バグ 10・優先 79・その他 50・据え置き 12・保留 1 ＝ 152〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-23 `ghost-inner-balloon` の完了の後の実数え＝バグ 10・優先 80・その他 50・据え置き 12・保留 1 ＝ 153〔優先 1 本〔同 spec〕を外した〕。その前の 2026-10-23 steering の同期の実数え＝バグ 10・優先 81・その他 50・据え置き 12・保留 1 ＝ 154〔その他 1 本〔`dot-sakura-specimen`〕を外し、その他 2 本〔`shiori2-protocol`・`seriko-pattern-calls`〕を足した〕。その前の 2026-10-23 `script-security-level` の完了の後の実数え＝バグ 10・優先 81・その他 49・据え置き 12・保留 1 ＝ 153〔優先 1 本〔同 spec〕を外した〕。それより前の数えの経緯は history「2026-10-23 完了済みの退避」）
> **規模**は棚卸㉓の再測定の見立て（タスク数）。**上限は 1 spec 20 タスク**。「要件で切る」と書いた行は、着手のときに要件の段で切り出す（先に起票しない＝spec 工場の禁止）。**Fable**列＝要件定義を Fable で起動したセッションで進めることを勧める（○）か、Opus で足りる（−）か。**既定は Opus**（2026-10-15 開発者）＝○は「先例の無い境界や公開の契約を新しく引く」「並行・スレッド・時刻や順序の保証を新しく設計する」「先例で決まらない骨格の分かれ目が残る」のどれかに具体的に当たるときだけ（基準の正本は `kiro-requirements-discussion` の Phase 5）。**2026-10-15 に全行をこの基準で付け直した**（○ 72 → 13・当たった項目は各 spec の要件の段の推奨で名指しする）。各 brief の末尾「2026-10-10 棚卸㉓の再測定」（と分割・測定・申し送りの節）が、崩れた前提・触るファイル・議題の正本。**「並び」「前提」の列には起票や前の棚卸の時点の書き方が残る行がある**＝食い違うときは brief の節と「ウェーブ編成」が正本。

| spec（旧い `areka-P0-` は省略） | 段 | 何をするか | 規模 | 並び | 前提（先に着地） | Fable |
|---|---|---|---|---|---|---|
| `bottom-row-text-clip`（**10-16 起票**・`balloon-text-bugs` の完了の棚卸・同じ文字の層の拾い残し 3 件を件 2〜4 にまとめた） | バグ・区分 B | ⑴（本体）最下行の字の下の部分（`g`・`y` の descent）が文字の面の下端（validrect の下端）で欠ける（emo2-kakukaku・面の高さ 122・Yu Gothic UI 28 画素・拡大率 2.0 で約 6 画素足りず、`g` が `q`、`y` が `v` に見える。行の矩形で数えると収まるとされるので見える窓のスクロールも起きない。下線の無い字で欠けるかは未確認＝要件の段の最初に撮る）＝直し方は要件の討議で選ぶ（面は validrect より広げない・下線の有無でレイアウトを変えない、の裁定の枠の中） ⑵ 潰れた描画範囲の再追従で差し込み口の entity が同時に替わると、古い差し込み口に前の字が残りうる（`actor_present.rs` の `TextSurface::release` が今の結線の slot を使う・観測なし） ⑶ 横書きの右から左の下線の写し（`underline.rs` の `stroke_rect`）をどの檻でも実物と突き合わせていない ⑷ 箱の押下の後の済み（`shell_box_handler.rs` の `send_selection`）で文字の層が見つからないとき記録なしで抜ける。細部は brief の「件 1〜4」 | S〜M（6〜11） | 文字とバルーンの列（emo-text の `layout.rs`・`region.rs`・`actor_present.rs`・`underline.rs`・`surface_window_child.rs` と `crates/areka/src/input_events/shell_box_handler.rs`）＝✅ `choice-ranges-one-function`（10-31 着地）・`range-choice-tag`・`anchor-style-canon`・`choice-marker-styling`・`balloon-font-file`・`text-typesetting`・`real-run-log-noise` と直列。席は次の棚卸で決める | ✅ `balloon-text-bugs`（10-16 完了） | − |
| `areka-test-wait-wording`（**11-02 起票**・`areka-test-binary-stability` の完了時の棚卸＝同 spec の要件 7.6） | バグ・区分 B | `crates/areka/src` のテストの文言の無い待ち 31 か所（自前の `recv_timeout` 13・`Instant::now() +` の締切 14・古い呼び名 `run_bounded` の呼び手 1・`Progress::Unknown` 3）を待ちの部品（`wait_until`・`wait_recv`・`run_bounded_watching` と `Progress::Count`）へ移し、負荷の下の赤を文言で読み分けられるようにする（目印を持てない待ちは理由つきの例外） | S〜M（6〜10） | いつでも（`crates/areka` のテストを触る spec と同じウェーブに置かない） | ✅ `areka-test-binary-stability` | − |
| `seriko-trigger-lottery-gaps`（**10-11 起票**・`/kiro-discovery` で起票 3 本＝`seriko-trigger-teardown-gaps`〔10-10・バグ〕・`seriko-lottery-ended-play-skip`〔10-10・バグ〕・`seriko-rebuild-hidden-lottery`〔10-07・その他〕をまとめた） | バグ・区分 B | SERIKO の引き金と抽選の食い違い 3 件: ⑴ interval の 3 語（`talk`・`runonce`・`periodic`）の引き金の後始末の食い違いを揃える（窓の知らせだけで面の番号が変わっても構え直さない／窓の新しい出番で部品の `periodic` の起点だけが置き直る／窓が閉じると部品だけが末尾のコマと再生中の時計を失う／面を離れた瞬間の再生に停止の記録が出ない／`\b[番号]` の切り替えのテストが無い／同じ決まりの写し 3 種の片付け）＝1 件ずつ「網羅台帳の備考に 1 文」か「変える」かに分ける ⑵ 抽選の animation が、境界より前に終わった再生の片付けが残っていると、その境界のくじを 1 回飛ばし、乱数も引かない ⑶ 部品の組み直し（`rebuild`）の 1 回目が、見えない部品にも回ってくじを引く。⑵⑶ は境界の時刻で測る・見える部品だけを評価する形に直す（同じ抽選の決定論のテストの期待値を 1 度で引き直す）か、今の決まりとして記録して閉じるかを要件で決める。⑷（2026-10-31 `animated-surface-log-flood` から足した）emo-compose の `nesting.rs` の `bind_targets`／`visible_parts` が、有効でコマの無い `bind+random` の pattern0 の先を見える部品に数え、描く側の `flatten_surface` と食い違う＝純 `bind` だけを数える形に直し、一致の檻で固定する。実機で絵の誤りを見たものは無い。細部は brief の「件 1〜4」 | M〜L（10〜24・⑵⑶ を両方直すなら上限の近く＝引き直すテストの本数を要件で先に数える） | シェルの element の列（seriko の `looper.rs`・`looper_trigger.rs`・`parts.rs`・`parts_trigger.rs`・`trigger.rs`・`actor.rs`・`table.rs`、件 4 で emo-compose の `nesting.rs`）＝`seriko-talk-clock-fidelity`（本 spec の後）・`seriko-interval-combinations`・`seriko-script-triggers`・`balloon-canon-residue`・`animated-image-import` と直列 | ✅ `seriko-trigger-intervals`（10-10 着地） | − |
| `extent-element-offset`（**10-05 起票**・`surface-element-nesting` の完了時の棚卸） | バグ・区分 B | 外形（`flatten_extent`・10-06 に `animated-image-playback` が `plan.rs` から `plan_extent.rs` へ移した）が画像の element定義の X,Y を数えず、ベースの画像の外へはみ出す画像の element が記録なしに切られる（emo-compose を作った時から・入れ子の子の中の element も漏れる）。ukadoc の外形の定義を確かめ、全命令の和集合に揃えるか、ベースで切ると明文化して記録を出すかを裁定する | S（3〜6） | v0.2.0・シェルの element の列（`draw-methods-canon` の後・`animated-image-import` と `element-clipping-option` の前） | なし（`element-base-method`・`animated-image-playback` は着地済み） | − |
| `draw-methods-canon`（**10-05 起票**・`element-base-method` の完了時の棚卸） | 優先・区分 A | `overlay`・`base` 以外の描画メソッド（`overlayfast`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*` ほか）を element定義・pattern定義で描き、pattern定義の `base` も描く。開発者の方針（10-05）「Direct2D が支える全部に対応できるよう、wintf も含めて広げる」。合成は D2D（GPU）の道で描く。読み手の `is_image_element_method` を広げ `Element` に欄を足すと、描けない行の警告の対象が自動で減る **棚卸㉓で分けた**: `blend-*` は `draw-methods-blend` へ。 | L（18〜24・越えたら画素を読む 5 か所を後の spec へ出す） | v0.2.0・シェルの element の列の頭（`element-clipping-option`・`draw-methods-blend`・`compose-f16-precision`・`ghost-alias-file` はこの後）。`balloon-text-visual-tree` と同じウェーブに置かない（`crates/areka-emo-present/src/mount.rs`・`crates/areka-emo-text/tests/support/mod.rs`） | ✅ `element-base-method`・✅ `wintf-d2d-composite-rules`・✅ `hit-id-pass` | − |
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
| `animated-image-import`（**10-05 起票**・棚卸㉒で `animated-image-playback` から切り出し） | 優先・区分 A | pattern定義の描画メソッド `import` の画像を名前を保って読み、焼いて置く（今は読み手がファイル名を 0 に化かし記録も出ない） | M（10〜14） | v0.2.0・シェルの element の列（読み手の鎖＝`extent-element-offset` の後・`element-clipping-option` の前） | ✅ `seriko-trigger-intervals`（10-10 着地）・`extent-element-offset`（ファイルの重なり） | − |
| `element-clipping-option`（**10-05 起票**・`animated-image-decode` のタスク 6.3＝要件 9.3） | 優先・区分 C | element定義の `--clipping,左,上,右,下` で矩形だけを描く・付けた element定義では動く絵として読まない（正典 C2）・オプションの並びを読み手で転記（`--alpha`・`--source`・`--scaling` の描画は範囲外） | M（11〜15） | シェルの element の列（`element-base-method`・`animated-image-playback`・`extent-element-offset`・`animated-image-import` の後） | `animated-image-decode`・`surface-element-nesting`・`element-base-method` | − |
| `balloon-element-order` | 優先・区分 A | `balloon` の element定義を並び順どおりの重ね順で描く。wintf の兄弟の重なり順（描画と当たり判定が逆）の裁定の持ち主 | M（10〜14） | シェルの element の列 | `shell-balloon`・`animated-image-playback` | ○ |
| `property-query-channels` | その他・区分 C | `\![get/set,property]`・`%property[…]`・`SenderType`（`\![embed]` は 10-04 に `sakura-embed-directive` へ切った）。許可の表の迂回を最初に決める | M（11〜14） | v0.2.0・kanade の列（`talk-script-metadata` の後・`shiori4-api` の前＝`shiori4-api` は許可の表をこの spec のものを使う）。kanade の進行の列の `sakura-time-critical` より先に取る（ファイルの順だけで働きの依存は無い）。host32-host の列からは外れた | `translate-pipeline`・✅ `property-name-case-fold`・`mcp-kanade-tools`（迂回の口） | − |
| `balloon-break-position`（**10-05 起票**・`balloon-lifecycle-events` の要件の討議） | その他・区分 C | `OnBalloonBreak` の Reference2（中断位置＝台本の先頭からの文字数・タグも数える）。台本の中の位置を字句から命令・cue・再生・完了の知らせ `TalkDone`（27 ファイル・60 か所）まで通す。数え方・どの台本か・待機の途中の 3 つを要件で決める | M（9〜13） | 台本のコンパイルの列の最後・kanade の列 | `balloon-lifecycle-events` | − |
| `network-update-canon-order` | その他・区分 C | 更新のイベントを ukadoc の発生順序に揃える（MD5 の取り直し・台詞の終わりを待つ・総括の 2 段） | M（10〜14） | `baseware-root-list` の後（同じ `update/desk.rs`） | `baseware-root-list`（ファイルの重なり） | − |
| `balloon-canon-residue` | 優先・区分 C | 残り（項目 2＝面の偶奇で左右のバルーンを選ぶ・普通のバルーンだけ、を先に。項目 4 `\![reload,balloon]` は 10-04 に `mcp-reload` へ移した） | M（10〜14） | `emo2_boot` の列・✅ `seriko-trigger-intervals`〔10-10 着地〕の後（seriko の `actor.rs`） | ✅ `seriko-trigger-intervals`（10-10 着地・ファイルの重なり） | − |
| `currentghost-property-tree` | その他・区分 C | 動く値を出す口（持ち主）＋`currentghost.balloon.scope` の 19 項目（残りは 10-04 に `currentghost-property-others` へ切った） | M（13〜16） | `emo2_boot` の列・プロパティの動く値の列 | ✅ `property-name-case-fold`（同じ `actor.rs`） | ○ |
| `property-catalog-lists` | その他・区分 C | 一覧（`ghostlist`・`balloonlist`・`activeghostlist`）と基盤待ちの枝の登記（`system.*` は 10-04 に `system-property-values` へ切った） | M（10〜14） | プロパティの動く値の列 | `currentghost-property-tree` | − |
| `zorder-property` | その他・区分 C | `currentghost.seriko.zorder` の読み書き | S（5〜8・書き込みの届け先を作るなら 8〜11） | プロパティの動く値の列 | `currentghost-property-tree`（動く値の口）・書く側は `property-query-channels` | − |
| `sakura-time-directives` | その他・区分 C | `quicksection`・`balloonwait`（`\_q` は議題。`balloontimeout` は 10-05 に `balloon-lifecycle-events` へ移した）。C・D 群は覚え書きへ戻し、`\![embed]` は `sakura-embed-directive` へ（10-04） | M（8〜12） | コンパイルの列 | `choice-timeout-directive`・`talk-fast-forward` | − |
| `element-animation-easing`（**10-12 起票**・**覚書の段**・開発者「css イージングの範疇で、エレメントアニメーションの記述や、さくらスクリプトからエレメントにイージング指示が出せたらよいのに」「夢よりは優先度高い程度」） | その他・区分 C | CSS の範囲のイージング（`cubic-bezier`・`steps`・`linear()` と名前の 5 語）を、pattern定義のコマ間と、台本の時間つきの指令（`\![set,alpha]`・`\![set,scaling]`・`\![move]` の `--time`）に添えられるようにする（ukadoc に無い areka 独自）。形は決めていない＝着手のとき要件の段で、最初の 1〜2 段（語彙と式＋使い手 1 つ）に切る | 未測定（覚書・最初の 2 段で M の見込み） | 区分 C の最後（据え置きの手前）。最初の使い手しだいで台本のコンパイルの列かシェルの element の列 | 働きの前提なし。時間つきの指令の土台は `window-timed-directives` が持つ（2026-11-19 起票・議題 1 は「先に別の spec を立てる」で決着） | ○ |
| `window-timed-directives`（**11-19 起票**・`/kiro-discovery`・台本「席替え」が使う・開発者「areka 環境に実装の起票投げておいてね」） | その他・区分 A | 時間つきの窓の指令＝`\![move]` の `--time`（今は最終位置へすぐ飛ぶ）・`\![moveasync]` と cancel・`\![set,scaling]` の 3 つの形・`\![set,alpha]` を、時間なし・`--time`・`--wait` で等速に動かす。待つ形は台本の時刻を進める。倍率は「遊ぶ人の倍率 × タグの倍率」で `currentghost.scope(ID).scaling` と `OnShellScaling` に出す。形（`--easing`）は `element-animation-easing` | M（12〜16） | v0.4.0・台本のコンパイルの列（`compile.rs`）と `areka-runtime` の `move_cue*.rs`・表示の倍率の席（`areka-emo-present/src/scale.rs`） | v0.2.0 の時刻の土台（✅ `dola-interrupt-ticks`） | − |
| `scope-currentmonitor-property`（**11-19 起票**・`/kiro-discovery`・台本「席替え」が使う） | その他・区分 A | `currentghost.scope(ID).currentmonitor` の残り 4 葉（`rect`・`work`・`primary`・`bpp`）を、聞かれた時に取りに行く形で出す（`dpi` は ✅ `shell-scale-linkage`）。座標の単位は動いている `currentghost.scope(ID).rect` に合わせる。`OnDisplayChange`／`Ex` は持たない | S（5〜8） | v0.4.0（待つ物が無いので前に出してよい）・プロパティの動く値の列 | なし | − |
| `sakura-raise-directive`（**11-20 起票**・`/kiro-discovery`・ノベルの暫定の設計の議論で見つかった正典の拾い残し・開発者の承認） | その他・区分 A | 正典の `\![raise,イベント名,r0,r1,…]`（台本が自分の SHIORI にイベントを起こす）を、台本の受け口から kanade を通して GET で送る。引数は Reference0 から。任意の名前を、名前と Reference の検査だけで通す出どころを 1 つ足す（許可した名前の表は触らない）。正典のタグの意味は変えない | S〜M（5〜7） | ノベルゲーム基盤の列（正典なので前に出してよい）・kanade の `msg.rs`／`schedule/`・`consumer_ledger.rs` | なし | − |
| `key-press-event`（**11-20 起票**・`/kiro-discovery`・同） | その他・区分 A | 正典の `OnKeyPress`（キーを押した時のイベント）を、wintf のキーの入力から kanade を通して GET で送る。Reference0〜4 は正典の並び。Reference0 の綴りと押しっぱなしの繰り返しは SSP を測って決める。areka は自分でキーの意味を決めない | S（4〜5） | ノベルゲーム基盤の列（正典なので前に出してよい）・入力の係と kanade の `msg.rs`／`schedule/` | なし | − |
| `shell-balloon-scale-setting`（**10-17 起票**・開発者「現在、シェルやバルーンの倍率は、DPI 追従だけで倍率が決まってるが、更に拡大率を設定できるようにしてほしい」） | その他・区分 A | ゴーストごとの表示倍率（既定 100%）と「バルーンも連携する」を sylphya の `Ghost` scope に覚え、`ScalePolicy.app_scale`（今は 1 に固定の口）へ流し、`placement` の自前の k の 3 か所も同じ倍率にそろえる（DPI 200%×表示倍率 200%＝400%）。入口の設定画面は windows-reactor 待ち＝それまでは `sylphya.toml` を手で書く | M（10〜14） | v0.4.0・`frame/dpi.rs`・`placement` の列（`dpi-realign-remembered-chain` と 1 本ずつ） | v0.2.0 の `wintf-d2d-composite-rules`・`draw-methods-canon`・`compose-f16-precision` | − |
| `menu-thumbnail-preview`（**10-20 起票**・開発者「サムネイル表示に対応する。メニューでゴーストを選択中、サムネイル画像を表示する機能がある」「α付き画像の透過も対応が欲しい」・11-01 windows-reactor 待ち） | その他・区分 A | 右クリックメニューのゴースト・シェル・バルーンの切り替えの項目にカーソルを乗せている間、その項目の `thumbnail.png` ほか（areka が読める形式・`.pnr` は左上の 1 点の色を抜く・α 付きは画素ごとに透ける・動く絵は dola の時刻で動かす）を、メニューの横の別の窓（wintf・DirectComposition）に出す。`WM_MENUSELECT` を wintf の `dispatch_window_message` で受け、`menu/plan.rs` で「番号 → サムネイルの場所」の表を作る。メニューは Win32 標準のまま（生きている決まり 6） | M（10〜14） | v0.4.0・メニュー（`menu/{mod,win32,plan}.rs`・wintf の `window_proc/mod.rs`）＝`popup-menu-shell-look`・`popup-menu-residue` と 1 本ずつ | v0.2.0 の dola の整備（時刻の土台）・`wintf-d2d-composite-rules` | − |
| `shell-hover-animation`（**10-20 起票**・開発者「シェルアニメーションに『マウスホバーが発生した時』だけ発動させたい」・「ぱすた開発」と相談） | その他・区分 A | マウスが名前の付いた当たり判定に乗っている間だけ動くアニメを、シェルの書き方だけで書けるようにする（ukadoc に無い areka 独自）。2 つの書き方＝interval の新しい語（例 `interval,hover,閉じる`）と bindgroup の「乗っている間 ON」（例 `sakura.bindgroupN.hover,閉じる`）・乗っている間ループと乗るたびに 1 回の書き分け。トークの最中も止まらない（SHIORI 経由はトークを置き換えるので、ぱすたはトーク中に × を赤くできない） | M（12〜16） | v0.4.0・入力の列（`input_events/` の追跡を使う）とシェルの element の列（seriko の `table.rs`・`looper.rs`・`trigger.rs`・`actor.rs`） | `mouse-click-wheel-events`・`seriko-interval-combinations`・`seriko-script-triggers`・v0.2.0 の時刻の土台（✅ `dola-interrupt-ticks`） | ○ |
| `resident-ghost-rotation`（**10-21 起票**・開発者「裏でSHIORIだけ常駐維持しておいて、さくらスクリプト投げてきたゴーストを取っ替え引っ替え切り替えるモード」） | その他・区分 A | 裏で何体ものゴーストの SHIORI を動かし続け、画面には 1 体だけを出すモード（SSP に無い areka 独自）。裏のゴーストへは時刻のイベント（今は `OnSecondChange`）だけを送り、台本を返したゴーストへ SHIORI を降ろさずに表示を替えて再生する。複数が返したら、表示中のゴーストの再生が終わるまで次を待たせ、届いた順に出す | M〜L（14〜20） | v0.4.0・`multi-ghost-runtime` の席の集まりの上（`emo2_boot/` の切り替えと着替え・`mcp/resolve.rs` ほかの宛先） | `multi-ghost-runtime`・v0.2.0 の wintf・dola の基幹の整備 | ○ |
| `parallel-visual-script`（**10-21 起票**・`/kiro-discovery`・開発者「マウスホバーの方向性」の 2 つ目「トークを止めない即時再生モードでさくらスクリプトを割り込ませる」） | その他・区分 A | 台本の冒頭に areka 独自の印のタグがあるとき、今のトークを止めずに並走で再生する（ukadoc に無い・PLUGIN の `ScriptOption: nobreak` は待ち行列で別物）。並走でしてよいのは見た目と音だけ（面・`\![bind]`・`\i`・`\![anim]`・音・時間の待ち）で、文字・バルーン・選択肢・`\x` は読み捨てて記録に出す。本数は縛らない。印の無い台本は今どおり置き換え | M（12〜16） | v0.4.0・トークの列（kanade の `steady.rs`・`events.rs`・`types.rs`、ghost の `dispatcher.rs`、sakura の `drive.rs`・`compile.rs`、seriko の `looper.rs`・`actor.rs`、`emo2_boot/talk_lifecycle.rs`） | v0.2.0 の時刻の土台（✅ `dola-interrupt-ticks`）・`talk-thread-count-growth` | ○ |
| `mcp-set-property`（**10-21 起票**・同じ `/kiro-discovery`・開発者「MCP の set で書く」） | その他・区分 A | areka 独自の MCP の道具 `set_property`（`check_script` と同じ形）。鍵・層・値・対象のゴーストを受け、KVS の名前での set で書く。書けない鍵と書けないときは誤りで返す。`sylphya.toml` の手書きの代わり（`shell-balloon-scale-setting` の brief を改める） | XS〜S（4〜7） | v0.2.0・`crates/areka-mcp/src/tools/`・`crates/areka/src/mcp/` | `sylphya-sqlite-kvs` | − |
| `sakura-time-critical`（**10-03 起票**・`emo-text-file-split` の実機の観測から） | その他・区分 C | 台本の `\t`（タイムクリティカル）を読み、台本の終わりか中断・選択まで、マウス系などの通知を止めて `Status` に `timecritical` を載せる（今は読まない＝`\t` を書いたメニューもなでなでの返事に置き換わる） | S〜M（8〜11） | kanade の列・台本のコンパイルの列（`sakura/decode.rs`） | `status-execution-states` | − |
| `makoto-dll-host` | その他・区分 C | MAKOTO/2.0 の DLL のホストと起動時の鎖（後から差し替えられる入れ物の形）。`\![load/unload/reload,makoto]` は 10-04 に `makoto-reload-directives` へ切った | L（15〜18） | host32-host の列 | なし（`Cargo.lock` の席・host32-host の列の順） | − |
| `mcp-user-response`（**10-05 起票**・同） | 優先・区分 A | 選択肢・入力欄の答えと、なでる・クリックなどの操作を、`since_id` 付きでエージェントへ返すツール 1 本（さくらスクリプトでは届かない返り道・SSP にも無い） | M〜L（12〜18） | kanade の列（choice とマウスの配線） | `mcp-author-tools`・`mcp-kanade-tools`・`mouse-drag-events` | − |
| `mcp-shiori-query`（**10-05 起票**・同・開発者「SHIORI への情報要求が無い・raise_event はトークになるだけ」） | 優先・区分 A | ゴーストへ情報を問うツール 1 本＝SSTP NOTIFY と同じ振る舞いに `X-MCP-PassThru-*` の往復を足す（作法は正典の `X-SSTP-PassThru-*` を写し、名前の頭だけ経路に合わせる＝開発者裁定・ghost_terminal の `ShioriEcho` 系が実例）。データはヘッダで返り、台本が空なら喋らない。SHIORI リソースを外から直接読む形は正典に無い＝要件の議題（推しは採らない） | M〜L（12〜16） | kanade の列（`actor_resources` の隣・host32-host の `shiori3.rs` も触る＝✅ `script-security-level` は 10-23 に着地した） | ✅ `mcp-kanade-tools`（10-19 着地）・✅ `script-security-level`（ヘッダの入れ物・10-23 着地＝済み） | − |
| `script-impact-tiers`（**10-05 起票**・同・11-01 同意の窓は windows-reactor 待ち） | 優先・区分 A | タグの影響の段＝高（ベースウェアの外）・中（ベースウェアの中で演技を超えるもの）・低（アクターの演技の範疇・ゴースト切替を含む＝**制約なし**・開発者確定）の正本の表と、出どころごとの扱い（中・高は要件の議題） | M（10〜14） | ✅ `script-security-level` の後（10-23 着地） | ✅ `script-security-level`（10-23 着地＝済み）・✅ `mcp-kanade-tools`（10-19 着地） | − |
| `range-choice-tag`（**10-05 起票**・同） | 優先・区分 A | `\__q[ID,…]…\__q`（範囲を丸ごと選択肢にする・複数行・画像のバナー）。未実装・引受先なしだった（`anchor-tag-canon` の brief の「実装済み」は誤り＝同 brief を直した） | M（10〜14） | **C6-①**・文字とバルーンの列・台本のコンパイルの列（✅ `anchor-tag-canon`〔10-10 着地〕の次） | ✅ `anchor-tag-canon`（10-10 着地）・`choice-script-prefix` | − |
| `link-context-copy`（**10-05 起票**・同・**areka 独自の拡張**＝ukadoc に右クリック・クリップボードの正典は無い） | 優先・区分 A | 選択肢・アンカー・範囲の選択肢を右クリックすると小さなメニュー（`menu/win32.rs` を使い回す）を出し、行き先をクリップボードへ。決め方は 3 段＝`script:` の中の開く系の行き先 → ID や引数の `http(s)://`・`file:///`・`mailto:` → 表示されている文字（開発者裁定） | S〜M（7〜10） | 文字とバルーンの列（`input_events/` のバルーンと箱の押下） | `open-external-tags`・✅ `anchor-tag-canon`（10-10 着地）・`range-choice-tag` | − |
| `balloon-link-hover`（**10-05 起票**・同・開発者「マウスホバーって出せる？ URL とか」・11-01 windows-reactor 待ち） | 優先・区分 A | 正典の `balloon_tooltip`・`OnChoiceHover`・`OnAnchorHover` を実装し、ゴーストが何も返さないときだけ行き先（`link-context-copy` の 1・2 段目）を出す | M（10〜13） | 文字とバルーンの列と kanade の列（`link-context-copy` の次） | `wintf-tooltip`・`link-context-copy`・✅ `anchor-tag-canon`（10-10 着地）・`range-choice-tag` | − |
| `shell-tooltip`（**10-05 起票**・同・11-01 windows-reactor 待ち） | その他・区分 A | キャラクター窓のツールチップ＝surfaces.txt の tooltipブレス（`sakura.tooltips` など・今は黙って吸収）・SHIORI の `tooltip`・`currentghost.seriko.tooltip.*`。どれも引受先なしだった | M（8〜12） | シェルの element の列とプロパティの動く値の列（着手の前に照合） | `wintf-tooltip` | − |
| `dev-folder-alias`（**10-05 起票**・同） | その他・区分 A | 開発中の git の作業ツリーを「名前 → フォルダ」で登録し、探す順のいちばん上に置く（ゴースト・バルーン・ゴーストごとのシェル）。同じ名前の入っているものを隠して開発版を動かせる。エイリアスの先へはインストールも更新も書き込まない。登録は App の記憶へ手で書く＋環境変数（メニューなし） | M（8〜12） | `baseware-root-list` の後・同じ列 | `baseware-root-list` | − |
| `temp-path-exceptions-to-target`（**11-01 起票**・`test-roots-under-target` の完了時の棚卸） | その他・区分 C | 迂回の見張りの例外表（`ALLOWED_ENTRY_POINT_USES`・19 行）に載るファイルが `std::env::temp_dir()` を直に使う一時の根を `target\test-roots` の下へ移し、例外表を縮める（本番のファイル 4 本は `#[cfg(test)]` の部分を兄弟のファイルへ出す見込み）。2026-11-06 `audit-ghost-catalog` が自分の範囲の 5 行（`areka-ghost` の結合テスト・`areka-nar` ほか）を先に取った＝一時フォルダの番人の例外表は main の取り込みの後で 13 行（ProcessUnique 11・ReadOnly 2） | M（十数ファイル） | `clippy-199-lints` の着地の後（触るファイルが重なる）・`areka-test-binary-stability` と同時に走らせない | ✅ `test-roots-under-target` | − |
| `present-emit-tail-latency`（**10-07 起票**・`animated-image-playback` の完了時の棚卸） | バグ・区分 B | シェルの絵の適用の約 1.2% が 16 ms を超える（最大 315〜360 ms）。全部が `presenter/show.rs` の最後に記録した段から `timing.emit(` までの測っていない区間。区間に段を足して原因を特定し、根本を直す。動く絵は適用の回数を約 13 倍にするので本数も増える | S〜M（5〜10・半分は計測） | ウェーブの外（測ることが本体） | `animated-image-playback`（✅ 10-07） | − |
| `pasta-idle-surface-cost-measure`（**11-14 起票**・2026-11-14 `wintf-visual-damage-redraw` の完了の棚卸で起票＝同 brief の件 1 の節を移した） | バグ・区分 B | ぱすたの面 1002 で何もしていないあいだの areka の CPU（2026-10-31 は release 中央値 15.4%・SSP 2.9.26 は 5.6%）と WorkingSet（176 MB・94 MB）の内訳を測る。直す前の数は入れ物の後の今の main で取り直し、`wintf::damage` の行（`compose_us`・`present_us`）と emo-present の perf の行を材料に、足りない所（DComp の commit・seriko と刻み・記録）へ時間の印を足す。直すのは数字の後 | S〜M（5〜10・大半は測り） | ウェーブの外（測ることが本体・静かな机で 1 本ずつ） | `wintf-visual-damage-redraw` | − |
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
| `balloon-text-visual-tree`（**11-14 起票**・2026-11-14 `wintf-visual-damage-redraw` の完了の棚卸で起票・開発者「本来、バルーンが使うはず。使ってないなら使うべきですね」） | 優先・区分 A | バルーンの文字の層（`areka-emo-text` の `surface.rs`＝自前のスワップチェーン・wintf の入れ物の外）を wintf の描画の木へ移す。文字の区画は自分の描画面を持ち（持ち主を描画面の持ち主の直下に置き `Arrangement` のずれで動かす）、スクロールは `wintf-visual-damage-redraw` の平行移動の写しで済ませる。写しが効くのは送りが物理ピクセルの整数・区画の描画面に根の命令の列が無い・重なる絵が無い・にじみの余白込みで行が重ならないとき。画素は今の文字の層と一致（WARP） | L（16〜20・超えたら箱の文字の面と印の面を切る） | 文字とバルーンの列（emo-text の席）。`crates/areka-runtime/src/frame/` の 4 ファイルと本体の `mcp/dump_balloon.rs` にも触る。`draw-methods-canon`（`crates/areka-emo-present/src/mount.rs`・`crates/areka-emo-text/tests/support/mod.rs`）と `balloon-script-offset-align`（`frame/drain_resnap.rs`）の後。`instant-to-interrupt-ticks`・`compose-f16-precision`・`emo-text-windows-text` と同じウェーブに置かない | ✅ `wintf-visual-tree`・✅ `wintf-d2d-composite-rules`・✅ `wintf-visual-damage-redraw`（`wintf-surface-auto-promotion` は待たない＝文字の区画は描画面の持ち主の直下に置く形で今の wintf のまま通る・2026-11-16 棚卸㊳で確かめた） | − |
| `winget-release-automation`（**10-10 起票**・棚卸㉓で `winget-manifest-submission` から切り出し・11-01 v0.1.0 から版に縛らない列へ） | 優先・区分 A | `.github/workflows/winget.yml` と `README.md`・`dist/README.txt` の行。初回の手提出が winget-pkgs に取り込まれてから始める | XS〜S（3〜4） | 配布と公開の列（✅ `winget-manifest-submission`〔10-10 着地〕の後） | ✅ `winget-manifest-submission`（10-10 着地）＋winget-pkgs の取り込み（手提出の PR は人の承認待ち）＋✅ `user-data-root`（10-17 着地） | − |
| `check-script-arg-checks`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 各機能の受け口が自分で読む引数の誤りを `check_script` が診る。10 個のうち 7 個（`\![move]`・`\![set,zorder]`・`\f`・`\_l`・`update`・`install`・`bind`）は判定が既に純粋な関数＝呼ぶだけ。受け口の中で判定している 3 個（`change`・ゴーストとシェルの切替・readme）を取り出す | M（8〜12） | `mcp/check_script_judge.rs`（`mcp-strict-errors`・`script-impact-tiers`・✅ `anchor-tag-canon`〔10-10 着地〕・`talk-fast-forward`・`font-size-keywords` ほかと分け合う）・`emo2_boot/{change_cue,switch_cue,readme_cue}.rs` | なし | − |
| `config-parse-diagnostics`（**10-10 起票**・棚卸㉓で覚え書きから・優先度は低い） | その他・区分 C | 設定の読み取り（`parse_kv`・`shell::parse`・`balloon::parse`）が捨てた行・読まなかったキーを言えるようにする（値で返す形を推す・ログの上限と水準が議題） | M（9〜13） | 読み手の鎖（シェルの element の列）と文字とバルーンの列の両方が空いたとき | なし | − |
| `font-size-keywords`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | `\f[height,…]` のスタイルシートのキーワード（`larger` など）で字の大きさを変える（今は語彙だけ受けて記録を出す。ukadoc は語の一覧も大きさの対応も書いていない＝areka の決まりを作る） | S（3〜5） | 文字とバルーンの列（`look.rs`・`state_decoration.rs`）と `mcp/check_script_judge.rs` | なし | − |
| `elevated-drop-filter`（**10-10 起票**・棚卸㉓で覚え書きから） | その他・区分 C | 管理者として動かした areka へ投げ込みを届けるか（Windows の権限の境界をゆるめる）、既知の制限のまま閉じるかを決める（説明書には既に制限として書いてある＝やらないなら直しは 0） | S（2〜4・やらないと決めれば 0） | `placement/spawn.rs`（`dpi-realign-remembered-chain`・`extra-character-windows` と分け合う） | なし（開発者の決めごとが議題 1） | − |
| `mouse-hover-event`（**10-24 起票**・`dpi-realign-remembered-chain` のタスク 6.3 の実機の確かめで開発者が気付いた） | その他・区分 A（開発者が起票を頼んだ） | キャラクターの当たり判定の上でカーソルを止めても `OnMouseHover` を送らない（網羅台帳 `absent`・持ち主なし）。正典は「マウスが静止している際に発生」と Reference の並びだけを書き、止まる時間・繰り返し・ドラッグ中とトークの最中に黙る＝SSP で測ってから裁定する。ぱすたは使わない | S（4〜6） | v0.4.0・`mouse-click-wheel-events` の着地の後（`input_events/`・kanade の `msg.rs`・`schedule/events.rs`） | `mouse-click-wheel-events` | − |
| `inputbox-user-input`（**10-10 起票**・棚卸㉓で覚え書きから・11-01 windows-reactor 待ち） | その他・区分 C | 入力欄 `\![open,inputbox]`・`\![close,inputbox,ID]` と `OnUserInput`・`OnUserInputCancel`（パスワード・日付・スライダーなどの兄弟の入力欄は範囲外） | M〜L（Win32 標準の入力なら 12〜16・バルーンに合わせた箱なら 20 を超える＝要件で切る） | `emo2_boot` の列（`consumer_ledger.rs`・`mod.rs`）と kanade の列（`schedule/events.rs`） | なし（Win32 標準の入力か、バルーンに合わせた箱かが議題 1） | − |
| `popup-menu-shell-look`（**10-18 起票**・棚卸㉗・v0.1.0 → 11-01 windows-reactor 待ち） | 優先・区分 A | シェルの `menu.*`（右クリックメニューの色・絵・文字）をオーナードローで描き、着せ替えのメニュー（`sakura.menu,auto`・bindgroup の名前・`mustselect`）を出す。生きている決まり 6（オーナードローと着せ替えメニューは予約）を解くかが先＝裁定待ち。ぱすたの `menu.*` は Windows 11 の明るいメニューを写したものなので、見た目の差が大きいのは着せ替えのメニュー | M（14〜18・超えたら描き分けと着せ替えのメニューの 2 本） | メニュー（`menu/{mod,win32,plan}.rs`・wintf の `window_proc/mod.rs`） | 開発者の裁定 | ○（wintf と areka の間に、OS のメニューの中で同期に描く新しい約束・着せ替えのメニューだけなら −） |
| `emo-balloon-merge-log-overlap`（**11-01 起票**・`emo-bake-log-overlap` の完了時の棚卸） | その他・区分 C | バルーンの 2 層マージ `load_scope_balloon_model` が窓の位置の読み（`placement/mod.rs`）と資産の組み立て（`balloon_definitions.rs`）の 2 か所から呼ばれ、読めない `warn!` と確定の `info!` が scope ごとに 2 行出る。1 行ずつにする（案 1＝記録を出さない入口を足し窓の位置の読みだけ黙らせる・一致はテストで見る／案 2＝読むのを 1 回にする・説明の R6.3「2 行で一致を見せる」を改めるので要件の段で裁定）。起動を止める失敗の記録はどの道でも出す | S（案 1 で 3〜5） | `shell-scale-linkage` の着地の後 | ✅ `emo-bake-log-overlap` | − |
| `balloon-dpi-switch-redraw`（**11-01 起票**・`alert-dialog-per-monitor-dpi` の完了時の棚卸＝同 spec の件 2） | バグ・区分 B | ゴーストを拡大率の違うモニタへ移した後にバルーンが枠なしで矢印だけ見えた件を、会話の途中と会話の後で撮り分けて見分け、描き直しの欠けなら直す（作りどおりなら SSP と同じかを ukadoc と SSP の MCP で確かめる） | S〜M | いつでも | ✅ `alert-dialog-per-monitor-dpi` | − |
| `ondestroy-on-exit`（**10-22 起票**・棚卸㉘・完了 `mcp-reload` の残り物） | その他・区分 C | 終了とゴーストの切り替えで、SHIORI を降ろす前に `OnDestroy`（NOTIFY）を送る（リロードの場面は `mcp-reload` が済ませた）・網羅台帳の行の直し・送る順は SSP で測る | S（4〜7） | kanade の列（✅ `script-security-level`〔10-23 着地＝済み〕・`mouse-click-wheel-events` と同じウェーブに置かない） | — | − |
| `install-package-type`（**10-23 起票**・棚卸㉙・覚え書きから格上げ・いつか） | その他・区分 C | `install.txt` の `type,package`（ゴースト・シェル・バルーンなどをまとめた書庫）を、今の「知らない種類」で断る形から、中の 1 つずつを入れる形へ。1 つが断られたら残りを入れるか・完了のイベントの送り方は SSP で測る | S〜M（5〜8） | インストールの列（`crates/areka-nar/src/{manifest,plan,lib}.rs`・`crates/areka/src/install/{procedure,judge}.rs`＝`startup-install-option` と `judge.rs`・`readonly-mode` と `install/`） | なし（`bootghost` を起動の道へ渡すのは `startup-install-option` の後） | − |
| `sstp-receive`（**10-23 起票**・棚卸㉙・覚え書きから格上げ・いつか） | その他・区分 C | SSTP の受信（SEND・NOTIFY・Option・共通のヘッダ・`OnSSTPBreak`）。kanade には SEND と NOTIFY の入口がもうある（`msg.rs`）＝足りないのは網の側。MCP の待受（SSP と 9801 を早い者勝ち）と同じ口で受け、最初の行で振り分ける案が議題。`balloon-markers` の `sstpmarker`／`sstpmessage` が画面に出るのはこの後 | L（24〜30＝要件で ① 受け口と SEND・NOTIFY〔14〜17〕／② `sstp-receive-methods`〔COMMUNICATE・EXECUTE・GIVE・INSTALL・HTTP 越し・10〜13〕に切る） | kanade の列（`schedule/sstp.rs`・`msg.rs`）と MCP の待受（`crates/areka-mcp/src/server.rs`）・`cli/listen.rs` | なし（`X-SSTP-PassThru-*` は `mcp-shiori-query`、`nobreak` は `shiori4-api` の列の後） | − |
| `balloon-inline-image`（**10-23 起票**・棚卸㉙・覚え書きから格上げ・いつか） | その他・区分 C | バルーンの文中に画像を置く `\_b[…,inline]`（`range-choice-tag` の例 1〔バナーの画像を選択肢にする〕に要る）。新しい cue の種類を足す＝emo-text・`areka-ghost` の受け手・seriko の分岐に波及 | M（10〜14・`\_b[…,x,y]` の形まで入れるなら 13〜17） | 文字とバルーンの列（emo-text の席を使う・`areka-parsers/src/sakura/{decode,model}.rs`・`areka-sakura/src/compile.rs`・dola の `cue/command.rs`） | なし | − |
| `ghost-alias-file`（**10-23 起票**・棚卸㉙・完了 `dot-sakura-specimen` の research の残り物・v0.2.0） | バグ・区分 B | シェルのフォルダの `alias.txt`（`sakura.surface.alias` ほか）を読まない。今は `kero.surface.alias` だけを読み、全部のキャラに 1 つの表を使い、面の引き当ては別名を見る前に番号を返す（`areka-parsers/src/shell/decode.rs`・seriko の `resolve.rs`）。検体のどっとさくらの `alias.txt` は持っていない面の番号（例: さくらの 9）を持っている面へ振り替える＝`\s[9]` が外れるはず（まだ誰も見ていない）。`ghost/master/alias.txt`（`makoto-dll-host`）とは別のファイル | S〜M（6〜9） | シェルの element の列（`shell/{decode,model}.rs`・emo-present の `shell_target.rs`・emo-compose の `{world,fold}.rs`・seriko の `resolve.rs`・`emo2_boot/assets.rs`） | なし（どっとさくらでの実機の確かめは ✅ `shiori2-protocol`〔10-24 着地〕の後＝それまで台本が流れない） | − |
| `seriko-missing-surface-default`（**10-24 起票**・完了 `descript-window-keys` の裁定 11・いつか） | その他・区分 C | 台本の `\s[n]` が今のシェルに無い面を指したときの扱いを決めて入れる。ukadoc は「存在しない番号を\sタグで指定した時などに代わりに表示する」＝既定の面を出すと書くが、今の areka は記録だけで表示を変えず、SSP 2.9 も既定の面を出さない（10-23 の測り＝`surface.num` が -1）。要件の段で ukadoc の字面と SSP の実測のどちらに合わせるかを決める（推奨＝ukadoc・既定の面は `placement::DefaultSurfaces` の候補の最初に在る面） | S（4〜6） | シェルの element の列（seriko の面の切り替え・`seriko-pattern-calls`・`ghost-alias-file` と同じ場所に近い） | `descript-window-keys` | − |
| `seriko-legacy-pattern-format`（**10-30 起票**・`seriko-pattern-calls` の要件の討議の議題 1・棚卸㉛でいつかに置いた） | その他・区分 C | 旧 SERIKO 1.x の書式（`*interval`・`*pattern*,サーフェスID,ウェイト,描画メソッド,X,Y`・`*option,exclusive`・旧 `alternativestart`）の行を読み手で読み、新しい書式と同じ型へ写す。今は `animation` で始まらない行を記録なしに飛ばす。`version,0` との関係と新旧の混ざりは要件の段で（ukadoc・読めない点は SSP で測る）。ぱすたと手元の検体 10 本は旧書式 0 行。細部は brief | S（5〜8） | v0.4.0・シェルの読み手の列（`areka-parsers` の `shell/{lexer,decode,model}.rs`＝`script-small-tags`・`seriko-interval-combinations`・`config-parse-diagnostics` と同じウェーブに置かない） | ✅ `seriko-pattern-calls`（10-31 着地） | − |
| `audit-app-leftovers`（**11-27 起票**・棚卸㊹・本体の見直し 3 本の取り残し） | その他・区分 A | 本体の見直し 3 本（✅ `audit-areka-app`・✅ `audit-areka-install-update`・✅ `audit-emo2-boot-switch`）が書き残した、持ち主のいない取り残しを直す: 失敗の文・記録の文に残る spec の名と要件の番号／テストが固定していない 2 件／結果を捨てている `write_session_mark` の 8 か所／同じ働きの写し 4 件／取り込みの道の混ざり。本番の振る舞いは変えない。細部は brief | S〜M（6〜10） | 版に縛らない。同じファイルを触る v0.2.0 の spec（`ghost-switch-balloon-fallback`・`instant-to-interrupt-ticks`＝`crates/areka-session/src/ghost_switch.rs`／`sylphya-sqlite-kvs`＝`boot_resolve_last_used_tests.rs`／`balloon-script-offset-align`・`balloon-text-visual-tree`＝`crates/areka-runtime/src/frame/drain_resnap.rs`）の着地の後。`areka-test-wait-wording` と同じウェーブに置かない | なし | − |
| `surfaces-basepos` | 据え置き・区分 D | surfaces.txt の `point.basepos`。宣言するシェルが要るまで始めない | S | シェルの element の列の最後 | — | − |
| `property-ipc-transport` | 据え置き・区分 D | 台本を通さないプロパティの読み取り。**最初の 1 段（調べて裁定）で終わる形へ縮める** | XS〜S（裁定まで 2〜4） | — | — | − |
| `dpi-transition-two-tick-bounce` | 据え置き・区分 D | 拡大率の切替で位置と大きさが 1 コマずれる（開発者が許容）。着手するなら最初に測り直し、跳ねが無ければ取り下げ | S〜M | — | — | − |
| `zorder-chain-residue` | 据え置き・区分 D | 間欠赤のテスト族と文書。09-11 以降 main で赤 0 件＝先回りしない（A-2 は 10-05 に `ghost-session-test-load-flake` へ移した） | S〜M | — | — | − |
| `text-reveal-dance` | v0.4.0・区分 A（開発者が起票を頼んだ） | 字が現れるときだけ跳ねる・揺れる（夢・開発者が望んだときだけ） | M〜L | — | `text-reveal-fade` | − |
| `window-snap`（**10-15 起票**・**夢の段・覚書**・開発者「夢仕様だけど起票お願い」） | v0.4.0・区分 A（開発者が起票を頼んだ） | マスコットが床・ほかのアプリの窓の上・窓の下・壁・窓の陰に吸着する「スナップ API」。本体が吸着の関係を語彙（コンテキスト）で持ち、シェルの定義だけで姿を出し分ける。今の `Anchor`（`alignmenttodesktop`）を一般化し、相手にほかの窓を足す（窓の見張りは WinEvent・見た目の枠は DWM）。落下・覗き・積み重なりは後ろの段。議題 7 点は brief 最初の段は正典のお座り＝`\![set,alignmenttodesktop,osuwari,…]`（ほかの窓に座る・ぶら下がる・`--zorder=back` で陰から覗く）・descript の値 `osuwari`・`point.osuwaripos.x`／`.y`（brief の「正典のお座り」） | L 超（要件で切る・最初の 1〜2 段だけ） | — | 土台の持ち主（議題 1）・積み重なりは `extra-character-windows` と多重ゴースト（予約） | ○ |
| `emo-text-canon-residue` | 据え置き・区分 D | **取り下げ予定・着手しない**。残っていた 1 件（折り返しの警告にバルーンの名前を出す）は `shell-balloon` が引き取った。フォルダを消すと網羅台帳の整合検査（`roadmap-draft.md` の `[[spec]]` の表）が赤くなるので、表から外すのと一緒に `coverage-roadmap-refresh` が片付ける | — | — | — | − |
| `release-code-signing`（**10-02 起票**・配布と公開・任意） | 据え置き・区分 D | SignPath Foundation（オープンソース向け・無償）に申請し、受理されたら CI で署名する。winget に署名は要らないので急がない | S（6〜9） | 配布と公開の列の最後 | `release-ci-workflow`・数回のリリースの実績 | − |
| `compose-f16-precision`（**10-15 起票**・開発者「wintfで利用しているスワップチェーンあるいはオフラインバッファについて、u8からf16精度のバッファにして、合成精度を強化したい」「低優先度で起票」） | 据え置き・区分 D | 途中の入れ物を 1 色 8 ビットから f16 にして合成の精度を上げる。**丸めが積もるのは GPU でなく CPU の合成**（`areka-emo-compose` の u8 の SourceOver）＝GPU の入れ物だけを替えても効き目は文字の層と拡大縮小だけ。合成の精度は `draw-methods-canon` の「合成をどこで行うか」と一緒に決める。色空間（f16 は DWM が線形で読む）と、先に測って差が見えなければ取り下げるかは brief の議題 | M〜L（測りと引数化だけなら S〜M） | — | `wintf-d2d-composite-rules`（自前の合成の入れ物）・`draw-methods-canon`（10-15 に合成は D2D の道に決まった） | − |
| `tick-gate-adoption` | 保留・区分 D | 既定で切の門を入れるか（待機中の CPU 17〜22% → 3% 未満の見込み）。短い A/B の測り方を先に組む | M〜L | 保留 | — | − |
| `shiori4-api`（**10-10 起票**・開発者「SHIORI4 の API 整備の spec はあるか・どこかの段階で着手しないといけない」・**10-23 書き直し**＝開発者「COM で作ろうかと原案を出していたインターフェースを、クロスプラットフォームを見据え C ABI でアクセスできるように仕様調整した」・区分 C・**版は v0.2.0**＝10-23 開発者「dolaなどの基幹部分大整備と同じタイミングで」） | その他・区分 C | SHIORI4 の正本は COM の `IShiori` から別リポジトリ `ekicyou/shiori4` の **C ABI**（`shiori4_abi`・`shiori4`・`shiori4_host`・crates.io 0.1.0〔2026-10-24 公開・2026-11-04 現在の最新は 0.1.1＝`shiori3-legacy-host`・`shiori2-helpers` を足した版・0.0.x と二進互換でない＝応答をホストが作る `copy_from_slice`。`shiori4_host` の芯が引き受けるので計画は変わらない〕）へ移った。areka はホスト側として `shiori4_host` を使い、経路を「kanade → `ShioriBackend` → helper のプロセス → SHIORI/3.0／SHIORI4」に並べる（10-10 の方針を引き継ぐ）。**SHIORI の実体はどの版でも別プロセス**（10-23 開発者「SHIORIの実体は別プロセスで動かすこと」＝SHIORI3 のブリッジと同じ環境・カレントディレクトリやプロセス全体の状態を持つランタイムの都合・`ShioriWiring::InProc` は無くなる）。v4 のペイロードは SHIORI/3.0 のヘッダ部分そのもの。すること＝helper の v4 の読み方（中継の `Host`・x64／arm64／i686 の helper）・IPC の札の追加（同期の問い合わせ・post・log）・本体の受け側（`getproperty`／`setproperty` → sylphya・post の `SEND`／`NOTIFY` → kanade の口）・descript.txt からの v4 DLL の選び分け・`load` のパスの形・自前の COM の面（`shiori-abi`・`shiori4-testdll`）と JSON-RPC の封筒の撤去（10-23 開発者「COM-SHIORI の作りかけは排除し shiori4 に完全移行」）・COMPAT §5 の書き直し。host-protocol と abi-conventions の語は 10-23 に shiori4・pasta との 3 者会談ですべて合意（割れなし・正本は shiori4 の作業ブランチ `claude/abi-host-protocol` の 2 本の brief） | 20 タスクを超える見込み（要件で切る） | 直列の列「host32-host」の後ろ（`shiori3.rs`・`client.rs`・`process_host.rs`・`shiori-host32-helper`・`shiori-host32-ipc`・`shiori_inproc.rs`・`shiori_wiring.rs`・`runtime.rs`・kanade の `shiori/real.rs`）。v4 の道のプロパティはこの spec、32bit の SHIORI/3.0 の道は `property-ipc-transport`（10-23 に分けた・どちらも IPC に札を足すので先に着手した方が形を決める）。許可の表を決める `property-query-channels` の後 | 実装がある程度整備されてから（開発者）＋shiori4 の `host-protocol` が入った版の公開（2026-10-24 に v0.1.0 で満たされた） | − |
| `tasktray-icon`（**10-10 起票**・開発者「タスクトレイの土台仕様を起票」「タスクトレイで出てくるべきメニューは今出てるシステムメニューと同じ」・区分 A） | その他・区分 A | 通知領域（タスクトレイ）に areka のアイコンを 1 つ出す土台。プロセスに 1 つの持ち物として、見えない窓とアイコンを持つ（ゴーストの切替で消えない・エクスプローラーの立ち上げ直しで出し直す・終了で消す）。マウスを重ねると「areka/[起動中のゴースト名]」。右クリックで、本体側のキャラクターの右クリックと同じメニューを出す（`menu/` の組み立てと表示はそのまま使い、キャラクターの窓に縛られた依頼の形 `MenuRequest` を広げる）。標準アイコンの絵は前提の `app-icon` が exe に埋める。議題＝左クリックとダブルクリックで何をするか・ゴーストがメニューを止めているときの扱い・ゴーストが居ない間の右クリック | M（10〜14） | `menu/trigger.rs`・`menu/mod.rs`・`main.rs`・新しい `tray/`・wintf に通知領域の部品。`menu/` を触る `popup-menu-residue` と直列（残件の相乗り先の候補）。棚卸㉔＝`main.rs`・`menu/mod.rs` が `baseware-root-list` と重なる＝その後 | `app-icon` | − |
| `tasktray-ghost-icon`（**10-10 起票**・同上・区分 A） | その他・区分 A | ゴーストが通知領域のアイコンと文言を決められるようにする。descript の `icon,ファイル名` と、台本の `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]`。`.ico` の読み込みは OS に任せる。動く絵の指定（`--duration`・`--runcount`）は、開発者の見立て（10-10「今はタスクトレイは基本隠れるので、動く意味は無い」）に沿って動かさない方向で議題にする | S（5〜8 見込み） | `\!` の受け口と descript の読み取り。重なりは着手のときに測る | `tasktray-icon` | − |
| `tasktray-balloon`（**10-10 起票**・同上・区分 A） | その他・区分 A | 台本の `\![set,trayballoon,オプション,オプション,オプション...]` で OS の通知を出し、押した・消えたを `OnTrayBalloonClick`・`OnTrayBalloonTimeout` で SHIORI へ送る。SSP の作者が BTS で説明した挙動 3 件（知らせの遅れ・OS の設定で出ない・時間切れが先に届く＝BTS 0000331・0000407・0000561）を議題にする | S〜M（6〜10 見込み） | kanade の列（`schedule/events.rs`）と `\!` の受け口・wintf の通知領域の部品 | `tasktray-icon` | − |
| `tasktray-minimize`（**10-10 起票**・同上・区分 A） | その他・区分 A | アイコン化（最小化）。メニューの「アイコン化」と台本の `\![set,windowstate,minimize]` でキャラクターとバルーンを隠し、通知領域のアイコンのダブルクリックで戻す。`OnWindowStateMinimize`・`OnWindowStateRestore` を送り、アイコン化の間は `icon.minimize,ファイル名` の絵にする。文言は `hidebutton.caption`。`alwaystrayiconvisiblebutton.caption`（「常にトレイアイコンを表示」）は項目を作るかどうかから議題。名指しの ukadoc の項目 2 つ（`icon.minimize`・`alwaystrayiconvisiblebutton.caption`）がアイコン化なしでは使いどころが無いので、持ち主の無かったアイコン化ごと 1 本にした。議題＝アイコン化の間に喋るか・時計を進めるか | M（10〜15 見込み・膨らめば要件の段で「状態と入り口」と「アイコン・文言・Status」に） | `menu/`・kanade の列（`schedule/events.rs`）・配置の層・wintf の窓の表示 | `tasktray-icon`・`tasktray-ghost-icon` | − |
| `real-run-input-guard-observations`（**10-16 起票**・`balloon-text-bugs` の実機の確かめ〔タスク 5.3〕の判定に使わなかった回で見た） | その他・区分 C | 実機の観察 2 件（どちらも 1 回だけ・再現していない・原因は未確認）: ⑴ ポインタの移動と押下を 1 回の `SendInput` で同時に送ると、areka の記録に移動も押下も残らなかった（先に移動してから押すと通る・クリック透過を外す判定の遅れの疑い・人の手では起きにくい） ⑵ 操作していない時間に `[visibility-guard]` と `[balloon-limit] Clamp` の WARN が 565 行続いた（直前に `loop ticker catch-up`・窓の 1 枚が最小化の位置 -32768・バルーンの x=-46→-323 の引き戻しの繰り返し・再走では再現せず・外からの窓の操作の疑い）。**最初のタスクは再現**＝再現しなければ記録を残して閉じてよい。正本は `completed/areka-P0-balloon-text-bugs/real-machine-signoff.md` の 4 章（生の記録は `target` の下で消えうる） | XS〜S（確かめと記録だけ 2〜4・直すなら 1 件につき 3〜5） | 単独（実機の走行。直すと決めたら ⑴ は wintf の窓の手続き、⑵ は `placement/follow/`・`placement/balloon_limit.rs`＝`dpi-realign-remembered-chain`・`window-snap` と照合）。実機の窓が出るので、負荷の走行と同じ時間に回さない | なし | − |
| `helper-first-load-refusal`（**10-24 起票**・`shiori2-protocol` の完了時の棚卸） | バグ・区分 B | 32 ビットの補助プログラムの SHIORI の読み込み（`load`）が時々断られる: 実物の pasta で回す `real_pasta_online` の 1 回目で `LoadReturnedFalse { last_error: 203 }`（`ERROR_ENVVAR_NOT_FOUND`）・2 回目は緑・1 回目は `TMP`・`TEMP` を `target` の下へ向け替えていた。完了 `emo2-real-run-stability` の件 2（原因不明で閉じた）の再発で、`last_error` の初めての観測。失敗した回の `pasta.log` は写しごと消えた。**最初のタスクは再現**＝失敗した回の記録を残す仕掛けを先に入れる・再現しなければ記録を残して閉じてよい。正本は `completed/shiori2-protocol/verification.md` の 4.3 節 | S（確かめと記録だけ 2〜4・直すなら 3〜6） | 単独（実機の走行＝排他テストの机。直すと決めたら helper の `shiori_proxy.rs` か試験の土台＝`shiori4-api` の helper の乗り換えと照合） | なし | − |
| `background-result-tick-wake`（**10-17 起票**・`user-data-root` の完了時の棚卸・タスク 4.2 の点検の議題） | その他・区分 C | 背景のスレッドの結果（`user-data-root` の切り替えの写し `switch_copy.rs`・切り替えの資産の組み立て `switch_assets.rs`・kanade の切り替えの知らせ `change_cue.rs` ほか・着手時に洗い出す）が起床の旗（`tick_wake::mark`）を立てず、tick の門を入れた形では次に誰かが旗を立てた回か心拍（`TICK_HEARTBEAT_FRAMES` 30 コマ）で拾われる＝切り替えの写しの後のゴーストの立ち上げが最大 30 コマ遅れうる。「時刻は正確に扱う」に合わせ、送る側で旗を立てるかを背景の結果の全部でそろえ、旗を立てるファイルを `tick_gate_config_producers_tests.rs` の `AREKA_PRODUCERS` に載せる。今の既定（門は切）では遅れは出ない | S〜M（5〜9） | v0.2.0（`tick-gate-adoption` が待つ・`crates/areka/src/emo2_boot/` の送り口・`tick_gate_config_producers_tests.rs`。旗の種類を wintf の `tick_wake.rs` に足す形なら `instant-to-interrupt-ticks` と直列）。`tick-gate-adoption` とは同じ時に走らせず、その前に着地させる。`switch_assets.rs` は `emo2_boot` の結線の列（`extra-character-windows`・`mcp-reload`）と同じウェーブに置かない | ✅ `user-data-root`（10-17 完了） | − |
| `ghost-switch-balloon-fallback`（**10-30 起票**・`startup-install-option` の完了の棚卸・同 spec の実機の試験で見つけた） | バグ・区分 B | 切替先のゴーストにも置き場にもバルーンが無いと、今使っているバルーンで続けずに `ghost_switch_boot_failed stage="balloon"` で切り替えが落ち、既定のゴーストがカタログに無いと `ghost_switch_fatal` で終わる（`crates/areka-session/src/` の `ghost_switch.rs`・`boot_config.rs` の `resolve_balloon_for_ghost`・`boot_resolve.rs` の `NoBalloon`）。SSP は既定のバルーンで続ける見込み（測っていない）＝決め方は要件の段で ukadoc を読み、読めなければ SSP で測る。`default-balloon-selfmade` は置き場にフォルダを置くだけで鎖は変わらない。細部は brief | S（3〜6） | v0.2.0・版の中の列の外（`crates/areka-session/src/` の `boot_resolve.rs`・`boot_config.rs`・`ghost_switch.rs`〔970 行＝足す分は別のファイルへ〕）。`sylphya-sqlite-kvs`（`boot_resolve.rs`）の後。`instant-to-interrupt-ticks`・`shell-companion-balloon` と同じウェーブに置かない | なし | − |
| `balloon-webp-names`（**10-10 起票**・開発者「画像ファイルなどは現代的なフォーマットにする」「online0.png などのアニメは動画 webp などで実現する」・区分 A） | 優先・区分 A | バルーンの面の絵を `.webp` の名前でも拾う（今は `.png` だけ＝`areka-emo-present` の `balloon.rs` の拡張子の定数）。名前から絵のファイルを探す所を 1 か所にまとめ、後から印を読む `balloon-markers` も同じ所を通す。動かない WebP を Windows の拡張に頼らず `image` で読む（依存は足さない）。`.png` と `.webp` の両方が在るときの勝ち負けを決めて固定する。シェルの絵の名前は持たない | S（4〜7） | 列の外（`areka-emo-present` の `balloon.rs`・`areka-emo-atlas` の `decode/`。`balloon-markers`・`balloon-canon-residue` と同じ系列の表を触るので、その 2 本より先に置く。席は次の棚卸で決める） | なし | − |
| `instant-to-interrupt-ticks`（**10-15 起票**・`/kiro-discovery`・開発者「低優先度」・「時刻の土台」の 3 本目） | 据え置き・区分 D | 残りの「今」の読み取り（wintf の刻みの締め切り・入力・ドラッグ・ツールチップ、areka の締め切りと計測、`areka-emo-present`・`shiori-host32-host`・`areka-nar`）を割り込み時刻の時計へ替え、clippy の `disallowed_methods` の番人で別の時計（`Instant::now`・`SystemTime::now`・`GetTickCount64`・`QueryPerformanceCounter`）を止める。暦の時刻が要る所だけ名指しで許す | M（12〜16） | v0.2.0・ほぼ 1 本だけのウェーブで（12 の crate の 43 ファイル・依存の席を使う＝時刻の型を使う 7 つの crate に `dola` を足す）。`balloon-text-visual-tree`・`ghost-switch-balloon-fallback`・`talk-thread-count-growth`・`sylphya-sqlite-kvs` の後・`tick-gate-adoption`・`background-result-tick-wake`・`compose-f16-precision` の前 | ✅ `dola-interrupt-ticks` | − |
| `multi-ghost-runtime`（**10-15 起票**・`/kiro-discovery`「版の段」・開発者「複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」・予約の「多重ゴースト」の土台） | 優先・区分 A | 2 体以上のゴーストを同時に動かす土台: ゴーストの箱 `GhostSlot` を集まりにし、窓・結線・kanade の知らせに「どのゴーストか」を持たせ、1 体の終了（`OnClose`）とすべて終了（`OnCloseAll`）を分け、起動中の印と記憶を並びにする。`get_active_ghost_list` が全員を返す | L（16〜20・最初のタスクで上限の近いファイルを割る） | v0.1.0・`main.rs` を触る spec（`baseware-root-list`・`user-data-root`・`write-failure-notice`）の後 | `baseware-root-list`・`user-data-root` | ○ |
| `ghost-call`（**10-15 起票**・同・予約の「多重ゴースト」の呼ぶ働き） | 優先・区分 A | メニューの「他のゴーストを呼ぶ」と `\![call,ghost,…]`（`--option=raise-event`）で 2 体目を起こし、`OnGhostCalling`・`OnGhostCalled`・`OnGhostCallComplete`・`OnOtherGhostBooted`・`OnOtherGhostClosed`・`OnOtherGhostChanged` を ukadoc の Reference で送る | M（10〜14） | v0.1.0・`consumer_ledger.rs` の共有の席（`\!` の名前を足す spec と直列） | `multi-ghost-runtime` | − |
| `defaultx-canon`（**11-15 起票**・`/kiro-discovery`・`audit-placement-core` の完了の申し送り〔同 spec の `research.md` 18.10「ukadoc と食い違う所」〕・開発者「推奨どおりで」・区分 B） | バグ・区分 B | `defaultx`（`sakura`・`kero`・`char*`）と `defaultleft`・`defaulty`・`defaulttop` の読みを ukadoc（2.9.13）に合わせる＝画面の座標（プライマリモニタの左上が原点）・`defaultx` は基準点・`defaultleft` は窓の左端・負は作業領域の右端（下端）からの距離・`-0` は端・`--` は負の座標そのもの。今の読み（鎖の前の窓から左へ空ける量＝`areka-P0-window-placement` 要件 2.10・`areka-P0-scope-chain-gap` 要件 2.8）の上に立つ鎖の決まりの隙間 ⑵ と、485・494・513・514 の裁定を開け直す。SSP は参考（候補 S1〜S4・emo2 の `defaultx,0` が今の位置に出るかを最初に確かめる） | M〜L（12〜20・越えたら読み取りと既定の並びを 1 本目に切る） | v0.3.0（新しく見つかった致命的でないバグは次の版）・`audit-placement-follow-residue` の着地の後（同じウェーブに並べない） | `audit-placement-core`・`audit-placement-follow-residue` | − |
| `balloon-directional-offset-keys`（**11-27 起票**・`balloon-script-offset-align` の要件の討議で切り出し） | その他・区分 C | descript（シェルとゴースト）の `balloon.offsetx`／`offsety` の後ろに方向の 1 文字（`l`・`r`・`c`・`b`）を付けた綴りと、surfaces.txt の surfaceブレスのバルーンのずらし（`sakura.`・`kero.`・接頭辞なし・`offset.x`／`offset.y`）を読み、ukadoc の探す順と優先（surfaces.txt ＞ シェルの descript ＞ ゴーストの descript・方向つき ＞ 接尾辞なし・`b` → `c` → 接尾辞なし）で 1 組を選んでバルーンを置く。サーフェスの切り替えで選び直す。台本の `@` の相対の指定と `\![set,balloonalign]` の基準もこの選び方に付け替える。網羅台帳 `assets.toml` の 10 行は `absent`・A2。ukadoc が黙っている点（層と接尾辞の掛け合わせ・「そのサーフェス」のスコープ）は要件の段で SSP で測る。件 2＝`\![set,balloonalign]` の `center`・`bottom` と `windowposition.x` のキーワードで、SSP はバルーンを本体の窓の上下の端に接して置かない（`keyword_balloon_pos` は窓の端）＝要件の段で SSP の基準（面の見える範囲か当たりの矩形か）を測り、合わせるかを決める。細部は brief | M〜L（14〜21） | いつか・`crates/areka-placement/src/` の `config.rs`・`resolver.rs`・`lib.rs`（`scale_then_merge`）・`follow/keyword_base.rs`、`crates/areka-parsers/src/shell/{model,decode}.rs`（シェルの element の列の読み手＝同じウェーブに置かない）、サーフェスの値を置き方へ渡す道（設計の段） | `balloon-script-offset-align`（`resolver.rs`・台本の値の層）・✅ `descript-window-keys`・✅ `balloon-flip-side-rules` | − |
| `balloon-append-tags`（**10-31 起票**・`balloon-scroll-arrows` の完了の棚卸・同 spec のタスク 8.2 の実機の走り） | その他・区分 C | `\C`（前のトークのバルーンを消さずに書き足す）と `\*`（選択肢の待ち時間を止める）。今はどちらも読み手で素通しになり、`\C` は台本の先頭の `ClearAll` でバルーンが空になってから出る（`check_script` は `unknown_tag`）。網羅台帳の群「バルーンの追記と選択肢のタイムアウト抑止」の 2 行・持ち主なし。版の置き場は次の棚卸で。細部は brief | 見積もりは要件の段（規模は spec 1 本に相当） | コンパイルの列（読み手の `sakura/decode.rs` を `script-small-tags` と分け合う・組み立ては `areka-sakura` の `compile.rs`） | なし | − |
| `dressup-change-events`（**10-31 起票**・`script-small-tags` の要件の討議の議題 3） | その他・区分 C | 着せ替えの後の `OnDressupChanged`・`OnNotifyDressupInfo` を送る（`\![bind,…]` の後と起動のとき・メニューができたらメニューの後も）。`\![bind-noevent,…]` とゴーストの読み直しの持ち越し（`shell_carry.rs`）からは送らない。網羅台帳 `shiori.toml` の 2 行・持ち主なし。**件 1（11-01）**＝mustselect の脱衣の WARN の量（ぱすたで 28 分 1,454 行・`actor_bind.rs`・区分 B）。版の置き場は次の棚卸で。細部は brief | 見積もりは要件の段（規模は spec 1 本に相当） | seriko の `actor.rs`（`script-small-tags` と同じウェーブに置かない）・kanade の `schedule/events.rs` | ✅ `script-small-tags`（`bind-noevent` の受け手と 2 つの名前の区別・10-31 着地） | − |
| `balloon-communicatebox`（**10-23 起票**・`pasta-in-windows-verification` の申し送り・v0.1.0 → 11-01 windows-reactor 待ち） | 優先・区分 A | 話しかける入力欄 CommunicateBox（バルーンの `communicatebox.*`・`\![open,communicatebox]`・`\![close,communicatebox]`・`OnCommunicate`・`OnCommunicateInputCancel`）。areka で初めての文字の入力の部品（IME）＝`inputbox-user-input` が後で使う | M〜L（14〜19・棚卸㉙・20 を超えたら入力の部品と開け閉め〔先〕／見た目の鍵〔後〕に切る） | `consumer_ledger.rs`・kanade の許可の表・バルーンの読み手 | なし（実機の確かめは、ぱすたの書庫の直し〔ghost_dev の `c32229e`・未リリース〕の後。それまでは `doc/pasta-real-run.md` の入れ方で） | −（棚卸㉛で要件定義を Opus に改めた・棚卸㉝でこの印を直した） |
| `sound-canon-tags`（**10-28 起票**・`/kiro-discovery`・N3「音の層」の前半・議題 8） | その他・区分 C | 正典の音のタグ（`\_v`・`\_V`・`\8`・`\![sound,play／loop／stop／pause／resume／wait／load／option…]`）と音のプロパティ 18 葉の値を、OS の Media Foundation で鳴らす（効果音の重ねに XAudio2 を併せるかは設計）。相対パスは `talk-script-metadata` の関数で解く。音楽 CD・動画・ノベルの音の拡張は外す | M〜L（14〜20・20 を超えたらタグの受け口とプロパティで切る） | ノベルゲーム基盤の先頭 | `talk-script-metadata`・v0.2.0 の時刻の土台 | − |
| `ghost-material-metadata`（**10-28 起票**・`/kiro-discovery`・M1「素材 ID とメタデータ契約」・議題 5〜7・11） | その他・区分 A | 素材 ID（`種別:id`・同梱は親を前置・元は descript の `id`、空ならフォルダ名）と探す順（叩き台＝`/play/{play}/shell/{shell}/` → `/free-shell/{shell}/` → `/ghost/{shell}/shell/master/`）・正典のキー（`craftmanurl` など）を使い、足りない分（ライセンス・支援先・role・借りないで・演劇である・席の要件）だけ areka の独自キーで descript に足す・借用はオプトアウト・目録の素性の拡張（完了 `baseware-root-layout` の要件 2.9 を改める）・単体のフリーシェルの置き場。要件の段の最初に pasta のセッションへ相談して P2（`pasta_check validate`）を決める | M〜L（12〜20） | ノベルゲーム基盤 | なし（`audit-ghost-catalog` が目録を先に触るならその後） | − |
| `shell-multi-compose`（**10-28 起票**・`/kiro-discovery`・N1「ノベルゲーム向けの画面構成」・議題 3・4） | その他・区分 A | 1 つの舞台の窓に背景・役者・トークの場所のシェルを何枚も重ねる・3 体以上の役者に任意のゴーストの任意のシェルを割り当てる・普通の窓と全画面・レターボックスとクロップ・場面転換とサーフェスの替わり方・揺れ・フラッシュ・色調・性能の目標。命令の名前は pasta の依頼書が決める | L 超（要件で切る＝舞台の窓と席／窓の形／転換） | ノベルゲーム基盤（v0.2.0 の新しい描画の層の上） | `wintf-visual-damage-redraw`・`ghost-material-metadata`・`element-animation-easing` | ○ |
| `areka-usage-stats`（**10-28 起票**・`/kiro-discovery`・U4「areka の利用統計連携」・議題 9） | その他・区分 A | 同意したとき（既定は無効）だけ、「使った」素材の ID を 1 日 1 回まとめて外の発信器（U2 の CLI）へ渡す。同意の正は発信器の設定・最初の同意は既定のゴーストの台詞と選択肢で尋ねる候補（メッセージボックスを使わない・設定画面は windows-reactor 待ち）・発信器が無い・失敗でも黙って飛ばす・ローカルパスや個人の名前を出さない・「匿名」と書かない。演劇の信号は演劇のパッケージの後 | M（10〜14） | 利用統計の段（ノベルゲーム基盤と並走してよい） | `ghost-material-metadata`・外の U2（発信器） | − |
| `doc-compat-architecture-rename`（**11-05 起票**・`doc-tree-tidy` のタスク 5.1・`/kiro-discovery`・同 spec の要件 6.5） | その他・区分 C | `doc/COMPAT_ARCHITECTURE.md` を名前の決まりに合う名前（推す名前は `doc/compat-architecture.md`）へ `git mv` し、`doc/README.md` の例外の表の行を消し、直下の表と動かした名前の対応の表を直し、名指し（クレートのコメント 18 ファイル・steering・スキル・`doc/` の中の文書・ukadoc-coverage の台帳・未完了の spec の文書）を直す。**名指すファイルを受け持つ見直しの spec と本体のクレート分けの 6 本とは同じウェーブに置かない** | S（3〜5） | 版に縛らない。`doc/COMPAT_ARCHITECTURE.md` を名指すファイルを受け持つ見直しの spec（10-28 の数えで 9 本・着手のときに grep で数え直す）と本体のクレート分けの 6 本（`placement-crate-split` から）とは同じウェーブに置かない | `doc-tree-tidy` | − |

## 覚え書き（brief なし・引受先が消えたまま忘れないための一覧）

> brief を書くほど固まっていない宿題。**着手の判断は棚卸で行う**（格上げするときは `/kiro-discovery`）。登記時の全文（数え方つき）は history「2026-10-02 棚卸⑳退避」、棚卸㉒の前の全文は history「2026-10-05 棚卸㉒退避」。棚卸㉓で brief にした 5 項目（6 本）の全文は history「2026-10-10 棚卸㉓退避」。 棚卸㉙で brief にした 3 項目（`install.txt` の `type,package`・SSTP の受信・`\_b[…,inline]`）の全文は history「2026-10-23 棚卸㉙退避」。

**pasta のノベルの依頼書を待つもの**（2026-10-28 `/kiro-discovery`）: ノベルの進行と入力の残り・バックログと巻き戻し・セーブとロード・システム画面の部品・ノベルの音の拡張・演劇のパッケージ・制作と配布の導線・pasta の P2。一覧と申し送りは「テーマ別の決めごと」の「利用統計とノベルゲーム基盤」の「受け口」、原文は `doc/ideas/novel-platform.md`。

**製品の穴**

- **`present-write-coherence` の未達 40 件**（性能・L）。完了仕様が自ら「引受先なし・新規仕様の起票が必要」と書いた残量＝`visualize_to_write_us` が上限 16,667µs の 12.6〜18.4 倍・32 窓中 0 窓が上限以下。開発者裁定で「未達のまま GO」済み。着手前に測り直す。`tools/perf` の 3 経路・`tick-gate-adoption` の測り方と同じく「性能改善ループが動いてから」の宿題。
- **`recommended.balloon`／`recommended.balloon.path`**（10-03 登記・S）。ゴーストの descript.txt の推奨バルーンで、これ以外へ切り替えると「強い内容の警告」を出す（ukadoc）。メッセージボックスを出さない方針（失敗は既定ゴーストの台詞で伝える）とどう折り合うかを決めてから起票する。`ghost-standard-balloon` が範囲外と明記。網羅台帳は `absent`・担当なし。
- **壊れたゴーストを表示し続ける形**（互換・M・優先度は低い）。SSP は切替先の SHIORI が死んでいても切替を成功扱いにして表示し続ける。areka は「既定ゴーストへ戻して `OnBoot` の Ref6＝`halt`」を採った（09-26 裁定・生きている決まり 7 と逆向き）。あるべき姿としては残る。開発者が閉じてもよい。
- **`sakura-time-directives` から戻した C・D 群**（10-04 登記）。時間の指令のうち、消費する者が現れるまで置くもの（同 brief の「2026-10-04 棚卸㉑で切った後の範囲」）。消費する者が現れたら、その spec の要件の段で引き取る。**2026-10-12**: D 群（時間つきの `\![move]`・`\![set,scaling]`・`\![set,alpha]`）を消費する者の候補として `element-animation-easing`（覚書）を起票した＝引き取るか、先に別の spec を立てるかは同 brief の議題 1。**2026-10-17**: `move-canon` の起票で開発者「アニメーションに絡むのは、dola の改造後」＝時間つきの `\![move]`（`--time` の補間と台本の時間を進めること）・`\![moveasync]`・`\![moveasync,cancel]` は v0.2.0 の dola の改造の後に起票する（✅ `move-canon`〔2026-10-18 完了〕は瞬間の移動だけ・`time>0` は最終位置へすぐ置いて記録する）。**2026-11-19**: D 群（時間つきの `\![move]`・`\![moveasync]`・`\![set,scaling]`・`\![set,alpha]`）は `window-timed-directives` が引き取った（消費する者＝台本「席替え」・v0.4.0）。覚え書きに残るのは C 群（`\![sound,wait]`・`\![wait,syncobject]`）だけ。
- **`OnDisplayChange`・`OnDisplayChangeEx`**（2026-11-19 登記）。台本「席替え」が任意で使う（モニターの範囲・タスクバーの位置）。起票しなかった＝タスクバーを避ける用途は `scope-currentmonitor-property` の `.work` で足りる見込み。網羅台帳は `absent`・持ち主なし・C2。使う者が現れたら起票する。
- **`sample-ghost-kit` の `fold-samples` の `strip_folder` が取り出し元の先頭の 1 段しか比べない**（XS・`completed/areka-P0-install-companion-reading/tasks.md` の Implementation Notes）。設計で広げないと決めたもの。階層付きの取り出し元を持つ検体ができたら直す。
- **`\![enter,passivemode]`・`\![leave,passivemode]` の網羅台帳の持ち主**（2026-10-31 棚卸㉛・XS）。`doc/ukadoc-coverage/ledger/sakura-script.toml` の 1075〜1084 行目の持ち主が完了 spec の `areka-P0-status-execution-states` のままで、`roadmap-draft.md` の 1147 行目の次の持ち主の候補 `areka-P0-passive-mode-states` は brief を持たず、廃止した接頭辞の名前。台帳の持ち主は spec の要件の段で動かす決まりなので、ここに置き、引受先の spec を起こすか `coverage-roadmap-refresh` が数え直すときに直す。
- **どっとさくらの実走で出た `kanade OnTranslate 400` の `WARN` 9 行**（2026-10-31 棚卸㉛・原因未調査）。✅ `seriko-pattern-calls` のタスク 8.3 の実走の記録（その spec の作業場の `target\sp83`・もう無い）に出た。華和梨の `OnTranslate` が 400 を返した行と見られる。次にどっとさくらを実走する spec が記録を残し、再現したら出力を添えて起票する。
- **起動の直後に `OnSecondChange` がぱすたに届かなかった**（2026-11-01 棚卸㉝・原因未調査）。✅ `shell-scale-linkage` の実機の確かめ（`completed/shell-scale-linkage/real-machine-verification.md` の 7 節の 3）で、起動の台詞が終わり `OnBalloonTimeout` が送られた後も、ボールをクリックするまで `OnSecondChange` の応答の台本が出ず、ぱすたの記録にも無かった。同 spec は「範囲の外として申し送る」と書いたが引受先が無かった。areka が送っていないのか、ぱすたが最初の画面（ボールだけ）では受けないのかは分かっていない。次にぱすたを実走する spec（`balloon-communicatebox`）か v0.1.0 の版の門の `doc/pasta-real-run.md` の走行で、areka の記録の `OnSecondChange` の送りを確かめ、areka の穴なら起票、ぱすたの側なら「ぱすた開発」のセッションへ伝える。

**潜在の性質（実害は観測されていない）**

- **dola の時刻の件が実機でどれだけ起きるかは数えていない**（持ち主なし）。✅ `dola-interrupt-ticks` の設計の検証（`completed/dola-interrupt-ticks/design-validation-2.md`・`design-validation-3.md`）が挙げた時刻の件の頻度は、実機で数えていない。困った例が出たら、出力を添えて起票する。

**道具と試験**

- **`tools/perf` の実走していない 3 経路**（S）。`invoke-perf-run.ps1` へ `-GhostRoot`／`-BalloonRoot` を渡す実走・`invoke-followup-checks.ps1` の単独起動・`check-quiet.ps1`。性能改善ループを次に回す前に 3 経路を通す（2026-10-16 に `perf-tools-console-encoding` が `tools/perf` の 9 ファイルを書き替えた〔子の出力を UTF-8 で読む形〕。3 経路は書き替えの後もまだ実走していない＝次に回す前に通す重みが増した・棚卸㉖）。
- **中継 `areka-mcp-bridge.exe` が arm64 の機械で起きるかの確かめ**（2026-10-18 棚卸㉗で「次のリリースへの申し送り」から移した・持ち主なし・開発者の机は x64）。`/release` の走りで arm64 の組み立てが通ることは記録される（`v0.0.3` の見どころにも事実として書いた）。arm64 の機械が手に入るまで未了のまま。 割り込み時刻の読み取り（`QueryUnbiasedInterruptTimePrecise`・✅ `dola-interrupt-ticks` が時計にした）も arm64 の実機では走らせていない。
- **`image-webp` の取り込みを外す作業**（XS・`animated-image-decode` が持ち主をここに置いた）。`image-webp` は上流の GitHub の固定コミット（0.2.5）から取り込んでいる（`tech.md` の登記）。**棚卸のたびに crates.io で `image-webp` の最新版を引き**、0.2.5 以上が出ていたら `tech.md` の取り外し条件どおりに外す＝根の `Cargo.toml` の `[patch.crates-io]` の行・`deny.toml` の `allow-git` の行・`crates/areka-emo-atlas/src/webp_pin_tests.rs` と同じクレートの `lib.rs` の `mod webp_pin_tests;` の宣言を外し、`cargo update -p image-webp` で公開版へ戻して検体のテストを通す。**最後の確かめ（2026-11-27 棚卸㊹）**: crates.io の最新は 0.2.4（2025-08-27）のまま＝まだ外せない。
- **一度だけ落ちた試験と、一度だけ出た出力**（原因未調査・再発したら出力を添えて起票）: `real-run-kit --test fake_mcp_test` の `child_dying_in_call_is_child_exited`（10-31・`emo-atlas-palette-alpha` の完了の全体テスト＝偽の子が 53500 番を束ねられず `ListenFailed`〔os error 10048＝ほかのものがそのポートを使っていた〕で、期待の `ChildExited` にならなかった。本 spec はこの crate に触っていない。赤の対象だけの回し直しで 8 本とも緑）・`wintf --test graphics` の `STATUS_ACCESS_VIOLATION`（10-04・他のセッションの cargo と並走した `cargo test --workspace -j 4` で 60 秒を超えて止まった後に落ちた。単独では 97 本すべて緑。完了 `wintf-gpu-test-crash` と同じ形・`areka-test-binary-stability` の件 1（旧 `areka-test-threads-av`）は範囲外と明記）。`areka-mcp` の `server::server_gate_help_tests::bad_origin_is_403_before_mcp` が全体テストの負荷の下で 1 回赤（10-04・os error 10053・単独では緑）。`cargo test -p areka-emo-present` の出力に `__rust_alloc_error_handler` のバックトレース行が 1 回だけ混じった（10-05・終了コード 0）。wintf のツールチップのテスト（マウスを動かす検査）で、テストの実行ファイルが 1 回だけ `0xc0000409` で落ちた（10-08・人の手のマウスが混ざった直後・再現せず。`completed/areka-P0-wintf-tooltip/tasks.md` の Implementation Notes。また出たら `--nocapture` の出力を添える）。wintf のユニットテスト `ecs::graphics::visual` の `systems_tests`・`systems_promotion_tests` の 22 本が、`tools/test-all.ps1` の x64 の段の中で「60 秒を超えて走っている」と出たきり 77 分止まった（11-19・`audit-pilot` の完了の前の全体テスト・84f5f10b1・開発者の承認でテストの実行ファイルを止めた。`cargo test -p wintf --lib` だけを回し直すと 1578 本が 7 秒で緑・再現せず。また出たら、止まったときのスレッドの様子（`procdump -ma` など）と同時に走っていたテストの実行ファイルの名前を添えて起票）。**棚卸㉒で外したもの**: `fallback_tests` の 2 本・`install::desk::overwrite_tests` の 2 本・`emo2_boot::spine` の 3 本・`sample-ghost-kit` の展開テストの os error 5（`target\nar-samples\work` の退避と同じ族）は `ghost-session-test-load-flake` が引き取った。`tools/test-all.ps1` の x64 の段の終了コード -1 は C: の空きが 35MB だったときの環境の問題として閉じた。
- **`--offline-dump` の端**（XS〜S・棚卸㉙・完了 `offline-surface-dump` の Implementation Notes 7.2・7.6）。`!surface4` の形を読まない（SSP での振る舞いは未測定）・`--dump-output-prefix` が絶対パスや `..` を止めない。どちらも要件の外として置いたもの＝開発用の道具なので、困った人が出たら `offline-surface-dump` の後続として起票する。
- **✅ `audit-test-kits` の次の機会に確かめる 3 件**（2026-11-20 棚卸㊵・XS・持ち主なし・`completed/audit-test-kits/research.md` の 14.8 節＝要件 8.3・決め D11）: ⑴ `window_link_real_run` を環境変数で明示して回したとき、同梱バルーンが同じ置き場へ写り、試験が今までどおり通る ⑵ 同梱バルーンを持つ検体を `fold-samples -- --check` に通したとき、終了 0 で不一致 0 ⑶ 検体を畳み直して往復が食い違ったとき、`vendors/` の `.nar` が書き換わっていない。次にその試験か道具を回す spec が確かめ、外れたら出力を添えて起票する。
- **記録の控えに残さない宛先の表 `UNKEPT_TARGETS`**（XS・持ち主なし・`crates/areka-session/src/log_history.rs`）。`update_cue`・`readme_cue` に `info!` の記録を足すときは、その宛先をこの表にも足す。足す記録がまだ無いので置いている＝次にこの 2 つへ記録を足す spec が一緒に直す。
- **windows-rs の移行の本体**（2026-11-14 `/kiro-discovery`・brief なし・6〜8 本の見込み）。`windows` クレートをやめ、`windows-core` などを 0.101 の系列へ移し、API を `windows-bindgen` の束縛で呼ぶ。**2026-11-17 棚卸㊴**: 先進坑 `pilot-windows-rs-next` は 11-17 に着地した（PR#471）。一次記録 `crates/pilot/examples/pilot-windows-rs-next/README.md`（問い 1〜10 の検証結果・刻み方の案・書き換えの対応表 `REWRITE_TABLE.md`・上流への報告の下書き `UPSTREAM_DRAFT.md`）の各節の「判定」の欄は空けてあり、判定は開発者が下す。上流の 0.101 系が crates.io に公開されるまで動かない。公開の後に、公開の版でこの記録の該当する所を確かめ直し（判定の欄もそのときに埋める）、`/kiro-discovery` で切り方を決めて起票し、全部のクレートを一気に入れ替える（0.62 と 0.101 を混ぜて段階で移すことはしない・移す動機の一つは、areka から windows-reactor を使えることをはっきりさせること）（仮の切り方は末尾の節「windows-rs の移行」の「移行の本体の仮の切り方」）。起票したら v0.2.0 の列に足し、`wintf` の作り直しの spec 群との順を決める。束縛には `WaitForSingleObject` と `THREAD_SYNCHRONIZE` を載せる（スレッド名簿が終わったスレッドを見分けるのに使う・`crates/wintf/src/api.rs` の `thread_has_exited` と `duplicate_current_thread_handle`）。`_Depends(confirmed): pilot-windows-rs-next`

## 直接修正候補（spec なし）

> **2026-11-27 棚卸㊹で直した**（どれも動きを変えない・説明と数だけ）: ⑴ `structure.md` の本体の依存の並びと、`test-support` を付ける所（本体は `shiori-abi`・`async-io`・`windows-core` を持たない・`areka-placement` と `areka-runtime` の `test-support` は `areka-session` から連なる）。 ⑵ `tech.md` の async-io の使い手（`areka-session` の `smoke_exit.rs`）。 ⑶ `doc/COMPAT_ARCHITECTURE.md` の「今日どおり」1 か所。 ⑷ `shiori4-api` の brief の、消えた `ShioriHostSink` の記述。 ⑸ コメント 3 ファイル（`crates/areka-runtime/src/lib.rs` の「`LogSink`×2」・`crates/areka/src/session_tests/ghost_session_restart_tests.rs` の説明・`crates/areka-session/src/boot_resolve_option_tests.rs` の空白）。 ⑹ 直列の列の `consumer_ledger.rs` の鎖の「手書きの数」（もう無い＝表が正本）。 ⑺ 「版の段」から、spec が 0 本になった「v0.1.0 の後の隙間」の行を外した。 ⑻ 台帳の、測り直した行の規模と並び。 ⑼ 開発者の決めの後に: `.github/workflows/crates-io.yml` に時間の上限（仕事 150 分＝待つ段の 120 分＋後の段・道具 10 分・公開前の確認 30 分・公開 15 分・過去 3 回は全部で 15〜20 分）。根の `scratch_expect.bak`（`crates/log-capture-kit/src/expect.rs` の古い写し）を消した。`talk-anchor-interrupt-ticks` のフォルダを消し、ほかの brief 8 本とロードマップの名指しを `dola-interrupt-ticks` へ付け替えた。
>
> **直さずに各 spec へ渡したもの**: 本体の見直し 3 本の取り残し（失敗の文に残る要件の番号・固定していないテスト・捨てている結果・写し）は、起票した `audit-app-leftovers`。測り直した 9 本の brief の古い記述と穴は、各 brief の「棚卸㊹の再測定」の節。
>
> **2026-11-22 棚卸㊸で直したもの・渡したもの**の全文は history「2026-11-27 棚卸㊹退避」の旧「直接修正候補」節。それより前は、そこから辿る。

残っている候補: 1 件。

- **`tools/mouse-input.ps1` の座標が負の原点の仮想画面で 1 画素ずれる**（2026-11-01 `popup-menu-extra-resources` の完了時の棚卸で登記・spec を立てない直しの候補）: **2026-11-01 棚卸㉝**＝根は直った見込み。✅ `shell-scale-linkage` が完了の時に、この道具の DPI の取り方を `SetProcessDpiAwarenessContext` から `SetThreadDpiAwarenessContext` へ替えた（`bf997862`・pwsh の中ではプロセスの扱いがもう決まっていて前者が効かず、論理座標で丸めて 1 画素ずれていた・下の 2 枚目の 3413×2133 も論理の大きさ＝実物は 2560×1600）。次に `mouse-input.ps1` を使う回でずれが消えたかを確かめ、消えていればこの行を外し、まだずれれば下の丸めを直す。登記の時の見立て＝主画面 2880×1800・左に負の座標の 2 枚目（3413×2133）がある機械で、送ったカーソルが毎回狙いより 1 画素小さく着き（狙い 2640,1460 → 2639,1459）、151 行の自分の確かめが「input refused」として止まる。134〜136 行の正規化の丸め（`ceil(p*65536/size)`）が、負の原点の仮想画面での Windows の戻し方と合っていない見込み。直し方は 2 通りある（丸めを直す／`SetCursorPos` で置いてから押しだけを `SendInput` で送る＝同 spec 4.2 の回避）ので、直す回で決める。確かめに画面の操作（排他テストの机）が要るため、その場では直さなかった。

取り下げた 1 件を再登記しないこと——判定器（`crates/areka-runtime/src/placement_tests/transition_judge_verdict.rs`）の窓ごとの書込上限が見送り窓を除いていないのは**意図どおり**（`completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` §13.1・2026-09-24 に開発者が再確認）。


## 生きている決まり（棚卸⑭〜⑲の裁定のうち今も効くもの・全文は history）

1. **着手の優先度は 4 区分＝A 開発者が起票を頼んだもの ＞ B バグ ＞ C ukadoc の拾い残し・バグでない持ち越し ＞ D 夢・据え置き・保留**（2026-10-07 開発者・10-05 の 3 段「① バグ ② リリース ③ MCP・バルーン・アニメーション」を改めた）。ウェーブは、依存の木（働きの依存＋同じソースファイルを触る spec＋直列の列の順）の先頭だけを「ぶら下がる未完了の spec の数 → 区分 → 起票の古さ」で並べ、上から互いに触るファイルが重ならない最大 8 本を取る（手順の正本は `/kiro-next` のスキル）。据え置き・保留・取り下げ予定は木に入れない。**測ることが本体の spec（遅れ・性能・負荷の再現）はウェーブに並べない**。バグかどうかは実測で分ける（利用者から見える実害／潜在／テストの穴／正典の追加）。
2. **規模の上限は 1 spec 20 タスク**。超えるものは要件の段で切る。**一度切り出した spec を、上限を少しまたぐ理由でさらに削らない**（09-28）。
3. **並走の物差し＝触るソースファイルの重なり 0**（09-20）。**文書だけの段は置かない**（09-27・要件は前の spec の着地で古くなる）。
4. **エンジンを切り出した spec は網羅台帳に触らない**。台帳の更新は結線の側がまとめて行う（09-20）。
5. **`zorder-chain-residue` の A 群は先回りしない**（main で赤が出たら、その時点で A 群だけを単独で挟む）。
6. **メニューは Win32 標準**・オーナードローと着せ替えメニューは予約のまま（2026-10-18 棚卸㉗: ぱすたの `menu.*` と着せ替えメニューのために `popup-menu-shell-look` を起票した。2026-11-01 開発者「そもそもオーナードローメニューはやらないと裁定したし、メニューで凝ったことをしたって、リアクターが来れば作り直し」＝この決まりは解かない・`popup-menu-shell-look` は windows-reactor 待ちへ移した）。**ゴースト切替はプロセス内**・アプリの寿命は窓の数から切り離す。**HTTP は WinHTTP・MD5 は OS の CNG**。**投げ込みは `WM_DROPFILES`**。**消滅（`\![vanishbymyself]`）は要望が出たら S で切る**。
7. **失敗は既定ゴースト（emo2）の台詞で伝える**（`OnBoot` の Ref6＝`halt`・Ref7＝落ちたゴーストの名前）。メッセージボックスは出さない（例外: インストールの警告はダイアログ可＝2026-10-17 開発者「インストール関係はシステム挙動が絡むから、警告があるならダイアログを出してよい」・`emily-ghost-verification` の要件 1.12）。**インストールと切り替えは別のイベント**（入れた後の切替を areka は主導しない）。
8. **要件定義・設計のサブエージェントは起動中のモデルを継承し、上位へ上げない**（09-30）。Fable 列は「Fable で起動したセッションを勧める」だけ。
9. **並走できる spec の紹介は「完全並走できるものだけ」・Fable 推奨を先頭に、名前＋一言＋コマンドの短い形**（09-26）。
10. **話の最中（選択待ちを含む）に届いたマウス系の返事は今の話を置き換える＝正典どおり**（2026-10-03・`emo-text-file-split` の実機で「メニューがなでなでの返事に置き換わる」を観測して確かめた。正本は完了 `input-events` の DD-IE-1／DD-IE-2）。止めるのはゴーストの側（`Status` の `talking`／`choosing` を見て黙る）か台本の `\t`。areka の穴は `\t` を読まないことだけ＝`sakura-time-critical`。

## 棚卸㊹の裁定（2026-11-27・main `ccd790fc`・開発者指示＝`/kiro-next` の定めどおり）

> 棚卸㊸（11-22・main `312a2f18`）の後、C26 の 3 本（PR#503〜PR#505）と、見込みの C27 の `dola-interrupt-ticks`（PR#506）が 11-23〜11-27 に全部着地した。ほかに main へ入ったものは無い。動いている spec は 0 本（`element-animation-easing`・`shell-balloon-scale-setting` の枝は古い起票だけの枝）。今の版は v0.2.0（まだ公開していない・16 本）。

1. **測り直した範囲**: 速い道（C26 と C27 が全部着地・前の棚卸が C28・C29 を見込んでいた・v0.2.0 の決めは動いていない）。見込みの C28 の 7 本を 2 つのサブエージェントで測り直し、うち 3 本が同じウェーブに並ばないと分かったので、待つ相手の無くなった 2 本（`balloon-script-offset-align`・`seriko-talk-clock-fidelity`）を足して測った（計 9 本・どれも brief に「2026-11-27 棚卸㊹の再測定」の節）。7 本のうち 6 本の前の測りは棚卸㉞（11-04）で、その後の crate 分けと見直しで brief のパスの大半が動いていた。
2. **残り物の仕分け**: 道具（`leftovers.py`）の出力は 473 行で、サブエージェントが仕分けた＝その場で直せる 6・起票の要る 8・待ち 6・済んだか持ち主に在る 28・持ち主のいないもの 1。起票の要る 8 件と、その場で直せるもののうちコードの中の文字列（失敗の文・記録の文）は 1 本にまとめて起票した（下の 3）。持ち主のいない 1 件（`UNKEPT_TARGETS`）と、実機で確かめていない 2 件（dola の時刻の件の頻度・arm64 での割り込み時刻の読み取り）は覚え書きへ。着地した 4 本の古い `.kiro/specs/<名前>/` のパスの名指しは 0。`image-webp` は 0.2.4 のまま。
3. **分割 0 本・起票 1 本**: `audit-app-leftovers`（本体の見直し 3 本の取り残し・S〜M・区分 A・版に縛らない）。`draw-methods-canon`（18〜24）と `sylphya-sqlite-kvs`（20〜26）は 20 を越えうるが、どちらも一度切り出した spec なので棚卸では切らない（`draw-methods-canon` は、越えたら画素を読む 5 か所を後の spec へ出す案を brief に持つ）。
4. **`talk-anchor-interrupt-ticks` を取り下げ予定にした**: 範囲の全部が `dola-interrupt-ticks` の要件に入って着地し、残る仕事が無い（同 brief）。v0.2.0 の行から外し、「いつか」の行へ取り下げ予定として置いた（v0.2.0 は 16 本）。フォルダは消した（brief は git の履歴に残る）。
5. **ウェーブ C28 を 6 本で組んだ**（Fable 1・Opus 5）: `draw-methods-canon`・`sylphya-sqlite-kvs`・`talk-script-metadata`・`balloon-script-offset-align`・`talk-thread-count-growth`・`seriko-talk-clock-fidelity`。6 本とも待つ相手は無い。重なりは約束 5 つで 0 にした（ウェーブの表）。見込みの C28 から外れたのは 3 本＝`balloon-text-visual-tree`（`draw-methods-canon` と 2 ファイル・`Element` に欄を足すと組む所を必ず直すので約束で消せない）・`ghost-switch-balloon-fallback`（`sylphya-sqlite-kvs` と `boot_resolve.rs`・どちらが避けても設計が曲がる）・`instant-to-interrupt-ticks`（12 の crate の 43 ファイルに及び、C28 の 2 本と重なる・別の時計を止める番人は全部を替え終えてから入るので、範囲を狭める約束が立たない）。先に取る側は、ぶら下がる数（`draw-methods-canon` 4・`sylphya-sqlite-kvs` 2・`ghost-switch-balloon-fallback` 2）と区分（A ＞ B）の順で決めた。
6. **`seriko-talk-clock-fidelity` は版の外の spec を待たない**: 起点と時計を含む 6 件は `dola-interrupt-ticks` が済ませ、残りは 2 件（2〜4 タスク）。`seriko-trigger-lottery-gaps` との間は同じ `trigger.rs` を触る順だけで、働きの依存は無い。
7. **空き 2 席は空けたまま**（版に縛らない割り込みの条件を満たすものが無い）: 割り込みの上限は C28 で一番大きい `sylphya-sqlite-kvs` の下限の 20 タスク。`doc-compat-architecture-rename` は `doc/COMPAT_ARCHITECTURE.md` に行を足す spec（`draw-methods-canon`・`balloon-script-offset-align`）と並べられない。`dev-folder-alias`・`shell-companion-balloon`・`config-parse-diagnostics` は `sylphya-sqlite-kvs` と（`boot_resolve.rs`・`areka-ghost` の `runtime.rs`）、kanade の `msg.rs`・`schedule/` に触るもの（MCP の 3 本・`balloon-break-position`・`dressup-change-events`）は `talk-script-metadata` と重なる。文字とバルーンの後続は `balloon-text-visual-tree` の作り直しを待つ。`clippy-199-lints`・`background-result-tick-wake` は wintf と dola に触る。ほかは棚卸㊴〜㊸で外した理由（静かな机か実機が要る・頭でない・上限を越える）が、この差分で動いていない。
8. **その先の C29〜C31 を組み直した**（「その先のウェーブ」）: C29＝`balloon-text-visual-tree`・`ghost-switch-balloon-fallback`・`mcp-set-property`・`draw-methods-blend`・`ghost-alias-file`／C30＝`instant-to-interrupt-ticks`・`element-clipping-option`／C31＝`compose-f16-precision`・`shiori4-api`。v0.2.0 の着地は、移行の本体と保留の `tick-gate-adoption` を数えずに約 4 ウェーブ（C28〜C31）。見込みの 3 ウェーブの中に `/release` は来ない。**モデル**: Fable は `sylphya-sqlite-kvs` の 1 本（何人もの書き手が 1 つのファイルへ書く順序と待ちの保証を新しく設計する）。`draw-methods-canon` は Opus へ下げた（CPU か GPU かの分かれ目は GPU に決まり、節ごとの合成の決まりと GPU の当たり判定の前例が着地している＝新しい境界も並行の設計も無い）。
9. **開発者の決め**: ⑴ windows-rs の移行は、上流の 0.101 系の crates.io への公開を待ち、出てから全部のクレートを一気に入れ替える（段階の移行はしない＝案 A・案 B はどちらも取らない・移す動機の一つは、areka から windows-reactor を使えることをはっきりさせること）。先進坑の判定の欄は、公開の版で記録を確かめ直すときに埋める。 ⑵ v0.2.0 を待たせる版の外の spec は v0.2.0 で済ませる＝`extent-element-offset`・`animated-image-import`（`element-clipping-option` が待つ）・`property-query-channels`（`shiori4-api` が待つ許可の表）・`background-result-tick-wake`（`tick-gate-adoption` が待つ）の 4 本を v0.2.0 へ移した（v0.2.0 は 20 本・C29〜C31 の見込みに入れた）。 ⑶ `talk-anchor-interrupt-ticks` のフォルダを消した。 ⑷ `crates-io.yml` に時間の上限を足した。 ⑸ 根の `scratch_expect.bak` を消した。本体の `menu/shortcut.rs` を見直しの範囲に入れるかの件は、見直しが全部着地したので閉じた。

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
- [x] dola-yaml-backend -- `dola` の機能 `yaml` が引く `unsafe-libyaml`（`RUSTSEC-2024-0449`）を、YAML の読み書きの差し替えか機能ごとの撤去で片づけ、`deny.toml` の見逃しを消す（2026-10-15 `release-cycle` の完了の全体テストの赤から起票）。Dependencies: none
- [x] areka-P0-tools-utf8-child-output -- リリースの道具が `cargo metadata` の UTF-8 の出力をコンソールの文字コードで解く穴を、読む側で直す（バグ・2026-10-05 `/kiro-discovery`・`release-cycle` の初回の実装からの依頼）。Dependencies: none
- [x] areka-P0-winget-manifest-submission -- `Areka.Areka.Portable` のマニフェスト・初回の手提出・winget-releaser。Dependencies: areka-P0-release-cycle
- [x] areka-P0-crates-io-publish -- `wintf`・`dola` だけを crates.io へ（Trusted Publishing・自分の workflow `crates-io.yml`）。Dependencies: areka-P0-release-package-versioned, areka-P0-mcp-server-core（C2・`release-ci-workflow` と並走）
- [ ] release-code-signing -- SignPath Foundation への申請と CI での署名（任意）。Dependencies: areka-P0-release-ci-workflow

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
- **決まりとして書くこと**: `QueryUnbiasedInterruptTimePrecise` は眠り（スリープ・休止）の間を数えない。復帰の瞬間にアニメが飛ばず、締め切りも眠りの間は進まない。`OnSecondChange` の OS の連続起動時間だけは、今の値（眠りの間も数える）と食い違う＝✅ `dola-interrupt-ticks` で、`OnSecondChange` も眠りの間を数えない形に決めた。
- **優先度**: 開発者の指定「低優先度」＝3 本とも据え置き・区分 D。**ウェーブへの組み入れは次の棚卸で行う**。

#### Existing Spec Updates

- [ ] seriko-talk-clock-fidelity -- brief に「ほかの spec からの申し送り」を足した（時計を揃える形を選ぶなら、揃える先は QPC でなく割り込み時刻にする・起点を 1 か所で見積もる形を選ぶなら、`talk-anchor-interrupt-ticks` で「押された起点を読む」に置き換わる前提で置き場を選ぶ）。Dependencies: none

#### Specs (dependency order)

- [x] dola-interrupt-ticks -- dola の時刻を `QueryUnbiasedInterruptTimePrecise` の 100ns の `u64` の型へ替え、「今」を読む関数をその 1 本にする（据え置き・区分 D）。Dependencies: none
- [x] talk-anchor-interrupt-ticks -- 取り下げた（範囲の全部を `dola-interrupt-ticks` が引き取って着地した・フォルダは消した）。Dependencies: dola-interrupt-ticks
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

- [x] wintf-d2d-composite-rules -- wintf で描くものごとに D2D の合成の規則を指定でき、自前の合成の入れ物から 1 ピクセルを読み戻せるようにする（優先・区分 A）。Dependencies: wintf-visual-tree（2026-10-21 に none から改めた＝「ビジュアル層の取り込み」）
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
- **直列の約束**: トークの側（`drive.rs`・`dispatcher.rs`）を直すことになったら `talk-fast-forward` と同じウェーブに置かない。`perf_thread_report.rs` は `instant-to-interrupt-ticks` と同じウェーブに置かない。

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

- [x] audit-emo2-boot-frame -- areka 本体の emo2_boot のうち毎フレームの組み立てを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: runtime-crate-split
- [x] audit-placement-follow -- areka 本体の配置のうち追従と移り変わりの判定を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: placement-crate-split
- [x] audit-placement-core -- areka 本体の配置の残り（窓の出し方・設定・保存・重なりの順・DPI ほか）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: placement-crate-split
- [x] audit-areka-input-menu -- areka 本体の入力の出来事と右クリックメニューを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: input-crate-split
- [x] audit-areka-app -- areka 本体の残り（起動・起動オプション・ゴーストの会話の場・SHIORI の結線・終了ほか）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: session-crate-split
- [x] audit-wintf-window -- wintf の窓と窓の手続きを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-wintf-layout-widget -- wintf のレイアウトと部品を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-wintf-input -- wintf のドラッグ・ポインタ・クリック透過・ツールチップを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-wintf-graphics-world -- wintf の描画・World・見た目の木・共通の型を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-wintf-runtime -- wintf の ECS の外（刻みの土台・COM・窓の型・API・数の型）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-emo-text-layout-draw -- 文字の層のレイアウトと描画を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] emo-text-reveal-incremental -- 1 字ずつ見せるときの区切り直し・当て直しの 2 乗の手間と、解決の `warn!` の置き場を直す（優先・区分 A・v0.1.0 の後の隙間・2026-11-12 `audit-emo-text-layout-draw` の完了の棚卸で起票）。Dependencies: なし
- [x] audit-emo-text-state-actor -- 文字の層の状態・アクター・選択肢・窓を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし。持ち込みの件（2026-11-06 `audit-emo-compose` の完了の棚卸から）: `actor_box.rs` の箱の位置の足し算が飽和しない（同 brief の Current State）
- [x] audit-dola -- dola（絶対時刻の台本の再生）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-seriko -- SERIKO（シェルのアニメーション）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-emo-compose -- 面の合成を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-emo-present -- 面とバルーンの提示を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-emo-atlas -- 絵の焼き込みとアトラスを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-parsers-sakura -- 設定ファイルの読み取りとさくらスクリプトを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-sylphya-actor -- プロパティの仕組み・役者の土台・トークを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-shiori-host32 -- 32bit SHIORI の中継と SHIORI の ABI・試験用の DLLを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-mcp-update-crates -- MCP の受け口と中継・ネットワーク更新のクレートを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-impl-watch -- 机の貸し出しのアプリ（impl-watch）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-ukadoc-survey -- ukadoc の網羅の調べの道具を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-test-kits -- 試験の道具のクレート（実走・記録の捕まえ・検体・一時フォルダ）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-tools-perf-judge -- 性能改善ループの判定と台帳（Python）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-tools-perf-run -- 性能改善ループの走らせ方（PowerShell）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-tools-scripts -- `tools/` 直下の台本（全体テスト・配布物・リリースノート・検体の取り寄せほか）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-pilot -- 先進坑の実験コードを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-emo2-boot-switch -- areka 本体の emo2_boot のうち切り替え・バルーンの見え方・ホバー・移動の cueを見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload, runtime-crate-split
- [x] audit-emo2-boot-spine -- areka 本体の emo2_boot の残り（背骨・消費者の台帳・適合の台本・資産・トークの時計ほか）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload, runtime-crate-split
- [x] audit-areka-install-update -- areka 本体のインストール・ネットワーク更新・MCP の受け口・根の一覧・説明書を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload, session-crate-split
- [x] audit-kanade -- kanade（ゴーストの段取りの役者）を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: なし
- [x] audit-ghost-catalog -- ゴーストの読み込み・.nar のインストール・利用者のデータの置き場を見直し、audit の指摘を全部直す（優先・区分 A・v0.1.0 の後の隙間）。Dependencies: areka-P0-mcp-reload

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

- [x] wintf-visual-tree -- 作りかけの描画の木（`ecs/visual/`）を本番の描画層へ上げ、WUC の描画面の下で節ごとに重ね方・補間・変形・切り抜きを効かせて 1 枚に描く（傷の直し・WARP の決定論のテストの足場を含む）。v0.2.0。Dependencies: audit-wintf-graphics-world, audit-wintf-runtime
- [ ] wintf-visual-damage-redraw -- 描画の木の前フレームと今の描く列を比べ（LIS）、変わった範囲だけを自前の合成の入れ物へ描き直す（答えは全部描いた答えと画素で一致）。v0.2.0。Dependencies: wintf-visual-tree, wintf-d2d-composite-rules
- [x] wintf-surface-auto-promotion -- 更新の頻度で描画面を自動で昇格・降格し、節の下の描画面の持ち主と持ち主の変形を支える（2026-11-06 `wintf-visual-tree` の完了の棚卸で起票・区分 A）。v0.2.0。Dependencies: wintf-visual-tree, wintf-visual-damage-redraw

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

- [ ] parallel-visual-script -- 冒頭の印のタグで、見た目と音だけの台本を今のトークを止めずに並走させる（本数は縛らない・文字などは読み捨てて記録）。v0.4.0。Dependencies: v0.2.0 の時刻の土台（✅ `dola-interrupt-ticks`）・talk-thread-count-growth

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
- [x] shell-scale-linkage -- （2026-11-01 に完了）`\![get,property]` の受け手・倍率と DPI のプロパティ・`OnShellScaling`／`OnBalloonScaling`・`balloon.syncscale`。v0.1.0・区分 A。Dependencies: ✅ mcp-scope-rect-property, areka-P0-script-security-level
- [x] balloon-scroll-arrows -- （2026-10-31 に完了）バルーンの矢印と手動スクロール・装飾の系列の土台。v0.1.0・区分 A。Dependencies: none（実機の確かめは、ぱすたの書庫の直し〔ghost_dev の c32229e・未リリース〕の後。それまでは doc/pasta-real-run.md の入れ方で）
- [ ] balloon-communicatebox -- 話しかける入力欄 CommunicateBox と 2 つのイベント。v0.1.0・区分 A。Dependencies: none（実機の確かめは、ぱすたの書庫の直し〔ghost_dev の c32229e・未リリース〕の後。それまでは doc/pasta-real-run.md の入れ方で）
- [x] vertical-choice-hover-band -- （2026-10-24 に完了）縦書きの選択肢の反転の帯を列の幅いっぱいに合わせる（塗りと当たり・横書きは変えない）。v0.1.0・区分 A。Dependencies: none（実機の確かめは、ぱすたの書庫の直し〔ghost_dev の c32229e・未リリース〕の後。それまでは doc/pasta-real-run.md の入れ方で）

### shiori2-protocol の持ち越し（2026-10-24 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `shiori2-protocol` のタスク 4.3（`completed/shiori2-protocol/verification.md` の 4.3 節・tasks.md の Implementation Notes）。新しい spec は 1 本。「．さくら」の崩れはタスク 5.4 で既存の brief へ渡し済み（同じ verification.md の 5.4 節）。起票の条件: 直したことを確かめるのに実機の走行を何度も回す必要がある（原因不明・再現待ち）。

#### Specs (dependency order)

- [ ] helper-first-load-refusal -- 32 ビットの補助プログラムの SHIORI の読み込みが時々 `last_error: 203` で断られる件を再現して切り分け、直すか記録して閉じる（バグ・区分 B・版に縛らない）。Dependencies: none

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

### 棚卸㊷の分割と起票（2026-11-22 `/kiro-next`）

分割 0 本・起票 0 本。brief へ足したのは、測り直した 3 本の「棚卸㊷の再測定」の節と、C25 の約束の決め（`test-name-work-tags`・`dola-cue-test-builder`）と、`audit-areka-app` への申し送り（本体の `runtime_tests/zorder_wiring_tests.rs` の札つきの 2 本）。

- **Existing Spec Updates**: 完了の棚卸で起票された 2 本を測り、`session-crate-split` と同じ C25 に約束つきで並べた（「棚卸㊷の裁定」の 4）。
- **Specs (dependency order)**: なし。

### 棚卸㊸の分割と起票（2026-11-22 `/kiro-next`）

分割 0 本・起票 0 本。brief へ足したのは、測り直した 4 本の「棚卸㊸の再測定」の節と、C26 の 3 本の持ち物の寄せ（`session-crate-split` の申し送りの 3 件）。

- **Existing Spec Updates**: C26 の 3 本の持ち物を寄せ、`dola-interrupt-ticks` の切り方の案を差し替えた（「棚卸㊸の裁定」の 3・4）。
- **Specs (dependency order)**: なし。

### 棚卸㊹の分割と起票（2026-11-27 `/kiro-next`）

分割 0 本・起票 1 本。brief へ足したのは、測り直した 9 本の「棚卸㊹の再測定」の節。

- **Existing Spec Updates**: `talk-anchor-interrupt-ticks` を取り下げ、フォルダを消した（範囲の全部が `dola-interrupt-ticks` に入った・「棚卸㊹の裁定」の 4）。`extent-element-offset`・`animated-image-import`・`property-query-channels`・`background-result-tick-wake` を v0.2.0 へ移した（同 9）。`shiori4-api` の brief の、消えた実装の記述を直した。
- **Specs (dependency order)**:

- [ ] audit-app-leftovers -- 本体の見直し 3 本が書き残した、持ち主のいない取り残し（失敗の文に残る要件の番号・固定していないテスト・捨てている結果・写し）を直す（その他・区分 A・版に縛らない）。Dependencies: none

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
  - その場で直した: `doc/pasta-profile.md` の作例の `\w[N]` 6 か所を `\_w[N]` に、「pasta が出す `\w[N]` は上流の欠陥」の注記を正しい出どころに（pasta の `novel-areka-contract` の brief「依頼に添える訂正」）。
- **下流の brief へ足したこと**: `extra-character-windows`（3 人目からの土台を `shell-multi-compose` とどちらが持つか）・`parallel-visual-script` と `sakura-time-directives`（音を鳴らす所＝`sound-canon-tags`）・`property-catalog-lists`（音のプロパティの値＝`sound-canon-tags`・`rateofuselist.*`）・`install-package-type`（単体のフリーシェルの置き場＝`ghost-material-metadata`）に 1 行ずつ。

#### 受け口（pasta の `novel-areka-contract` の依頼書を待つもの・brief なし）

pasta（SHIORI のエンジン・`C:\home\maz\git\pasta`）の `novel-areka-contract` は brief だけで未着手（命令の語彙・イベント・設定の名前と、取り決めの版の番号は未定）。最後のタスクで areka へ依頼書を渡す段取り。届いたら、その棚卸で次を起票する（原文は `doc/ideas/novel-platform.md`）。

- ノベルの進行と入力の残り（N2）: オート・既読スキップ・キーボード送り・設定値（文字の速度・オートの待ち・既読の扱い）の保持。クリック送り・早送り・`\x` は `talk-fast-forward`、マウスは完了 `mouse-click-wheel-events`。待つのは台本の `\x` だけ（自動の改ページは作らない）。
- バックログと巻き戻し・セーブとロード・システム画面の部品と見本（N4）。読めないセーブを上書きで失わない。取り決めの版と areka の brief の対応表を持つ。
- ノベルの音の拡張（N3 の後段）: BGM のクロスフェード・同じ曲を鳴らし直さない・効果音の重ね・区分ごとの音量・台詞と結び付く声・バックログからの聞き直し（`sound-canon-tags` の上に積む）。
- 演劇のパッケージ（M2）: 種別（`type,ghost`＋印か、`/play/` の置き場か）は依頼書の「起動の形」で決める。新しい種別なら `shiori4-api` に「演劇から SHIORI を選ぶ書き方」を足しに戻る。SSP に入れられたときの知らせ方（「areka で遊んでね」の台詞）は pasta への申し送り。演劇の信号（再生の開始・場面への到達・再生の完了）は `areka-usage-stats` へ。
- 制作と配布の導線（M3）: lint・プレビュー・サンプルの演劇・配布の手引き・登記簿への登録の手順（M2 の後）。
- pasta の P2（`pasta_check validate` の検査）: `ghost-material-metadata` がウェーブに載ったら、その要件の段の最初に pasta のセッションへ相談して決める。
- 申し送り: `ALLOWED_EVENT_IDS`（`crates/areka-kanade/src/schedule/events.rs`）は「ukadoc の固定名だけ・各行に ukadoc の URL」の決まり＝ノベル用の独自イベントを足すには表の決まりを広げる。`doc/pasta-profile.md` の訂正は済み、と返す。

- [x] doc-tree-tidy -- `doc/` と `docs/` の置き場をまとめ、名前の書き方をそろえ、案内の表と実物の突き合わせを試験にする（v0.2.0・区分 A）。Dependencies: none
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
  | `crates/areka/src/placement/` | `crates/areka-placement/src/`（1）。`mod.rs` は `lib.rs`、`PlacementError` は `error.rs`（1） |
  | `crates/areka/examples/window-placement.rs` | `crates/areka-placement/examples/window-placement.rs`（1） |
  | `crates/areka/src/placement/` の `transition_judge*`・`transition_signoff*`（テスト専用・判定器の一族 15 ファイル） | 本体のテスト専用のモジュール `crates/areka/src/placement_tests/`（1）→ `crates/areka-runtime/` のテスト専用（3） |
  | `crates/areka/src/placement/` の境をまたぐテスト 4 本（`reset_balloon_pos_tests`・`spawn_assembly_tests`・`measure_bake_count_tests`・`measure_bake_log_tests`）と分岐の対応表 2 本（`zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`） | 本体に残す＝`crates/areka/src/placement_tests/`（1）。`emo2_boot` を使う 5 本の置き場は（3）、`spawn_assembly_tests` は（4）で決め直す |
  | `crates/areka/src/emo2_boot/`（下の 3 行に無いもの全部・テストを含む＝`frame/`・`frame.rs`・`frame_*.rs`・`*_cue.rs` の残り・`consumer_ledger`・`target_map`・`balloon_visibility*`・`talk_*`・`adapter`・`spine*`・`reload_args`・`shell_balloon_resolve`・`shell_carry` ほか） | `crates/areka/src/runtime/`（2・同じ名前）→ `crates/areka-runtime/src/`（3） |
  | `emo2_boot/` の `ghost_switch.rs`・`ghost_switch_*_tests.rs`・`ghost_switch_test_support.rs`・`reload.rs`・`reload_tests.rs`・`reload_skin_tests.rs`・`shell_balloon_switch.rs`（要求の入口）・`shell_balloon_switch_tests.rs`・`runtime_boot.rs`・`runtime_boot_tests.rs`・`switch_copy*`・`same_folder_pin*` | `crates/areka/src/session/`（2・同じ名前）→ `crates/areka-session/src/`（6）。ただし（2）で runtime に残したもの＝`ChangeRequestRaw` は `runtime/change_cue.rs`、切替の型と `skin_ref_*` は `runtime/skin_switch.rs`、`CueSinkSources`・`RuntimeBootError` は `runtime/mod.rs` |
  | `emo2_boot/frame.rs` の終わる相（`KanadeNoticeRx`・`run_ghost_quit_phase`・`ghost_quit_system`）と `mod.rs` の `wire_kanade_stop`／テスト `frame_ghost_quit_tests.rs`・`frame_ghost_quit_logsink_tests.rs`・`frame_ghost_quit_switch_tests.rs` | `crates/areka/src/session/ghost_quit.rs`／`session/ghost_quit_tests.rs`・`ghost_quit_logsink_tests.rs`・`ghost_quit_switch_tests.rs`（2） |
  | `emo2_boot/` が本体の状態を直に読んでいた所（置き場のゴースト・起動の文脈・切替と読み直しの予約） | 本体の口の束＝型は `crates/areka/src/runtime/session_hooks.rs`、中身と据え付けは `crates/areka/src/session/hooks.rs`（2） |
  | `emo2_boot/install_cue.rs`・`update_cue.rs`・`readme_cue.rs`（とそれぞれの `_tests.rs`） | `crates/areka/src/install/`・`update/`・`readme/`（2・同じ名前）。readme は（5）で session へ |
  | `input_events/shell_box.rs` の `ShellBoxHover`・`settle_box_hover`／`input_events/balloon.rs` の `BalloonWiring` のホバーの集合 | `crates/areka/src/runtime/shell_box_hover.rs`／`runtime/balloon_hover.rs` の `BalloonHover`（2） |
  | `ghost_session_scope_scale.rs` の表と文字の関数 | `crates/areka/src/runtime/scope_scale.rs`（2）。ファイルの残りは src 直下のまま（5 で session へ） |
  | `crates/areka/src/input_events/` | `crates/areka-input/src/`（4）。`shortcut.rs` は `crates/areka/src/menu/`（4）、`file_drop.rs` は本体に残る |
  | src 直下の `ghost_session*`・`boot_resolve`・`boot_config`・`app_exit`・`session_end`・`exit_wait`・`write_failure`・`readonly`・`alert`・`log_history` | `crates/areka/src/session/`（5）→ `crates/areka-session/src/`（6） |
  | `main.rs` のシームの約 367 行（`on_boot_ok`・`settle_session_mark`・`restore_merged_placements` ほか） | 同上 |
  | `roots/` の `mod`・`resolve`・`report`・`user_root`／`readme.rs`・`readme/` | 同上（`roots/` の `edit`・`desk`・`pick` は本体に残る） |
  | ログの出所 `areka::placement::*`（`RUST_LOG` の `areka::placement=…` も）・`areka::emo2_boot::*`・`areka::input_events::*`・`areka::{ghost_session,boot_config,app_exit,…}` | `areka_placement::*`（1）・`areka::runtime::*`（2。session・install・update・readme へ移したファイルは移した先のモジュールのパス＝`areka::session::ghost_switch`・`areka::install::install_cue` など）→ `areka_runtime::*`（3）・`areka_input::*`（4）・`areka::session::*`（5）→ `areka_session::*`（6） |

- [x] placement-crate-split -- 窓の配置を本体から `areka-placement` へ移す（もともと依存の末端・振る舞いは変えない・v0.1.0 の後の隙間の先頭・区分 A）。Dependencies: v0.1.0 の版の門・areka-test-binary-stability
- [x] runtime-module-untangle -- `emo2_boot` を本体の中で `runtime` に改名し、型の名前を改め、切替などを `session/` へ移して、外への依存を 0 にする（区分 A）。Dependencies: placement-crate-split
- [x] runtime-crate-split -- 外への依存が 0 になった `runtime` を `areka-runtime` へ移す（`target_map`・テスト専用の `transition_judge*` も・区分 A）。Dependencies: runtime-module-untangle
- [x] input-crate-split -- 入力の出来事を `areka-input` へ移す（`shortcut.rs` は menu へ・`file_drop.rs` は本体に残す・区分 A）。Dependencies: runtime-crate-split
- [x] session-module-gather -- 起動・終了・切替の層と main.rs のシーム・roots の芯・readme を本体の `session/` に集める（区分 A）。Dependencies: input-crate-split
- [x] session-crate-split -- 集めた層の本体への呼び出し約 30 件の向きを逆にし、`areka-session` へ移す（区分 A）。Dependencies: session-module-gather

### startup-install-option の持ち越し（2026-10-30 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `startup-install-option` の完了時の棚卸（`completed/startup-install-option/requirements.md` の要件 10.2 と、実機の試験 `crates/areka/tests/startup_install_real_run.rs` を作るときに見た 2 件）。新しい spec は 2 本、既存の brief への追記が 1 件。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: 起動の解決の前に入れる件は、要件 10.2（要件の討議 議題 2）で別の spec と決まっている＝新しい 1 本。切り替えのバルーンの件は、触る場所（バルーンの解き方の鎖と切り替えの段 ③）を直す brief が無いので新しい 1 本（`shell-companion-balloon` は同じ関数を触るが、シェルに紐づくバルーンの働きで、直す中身が違う）。YAYA のゴーストで出た補助プログラムの `loadu` の後の `HRESULT(0x80004005)` は、触る場所（補助プログラムの初回の LOAD）が同じで、確かめるのに実物の走行を何度も回す `helper-first-load-refusal` へまとめた。
- **版**: `install-before-boot` は `startup-install-option` と同じ版のゴールの続き＝v0.1.0（版の門に数えるかは次の棚卸で決める）。`ghost-switch-balloon-fallback` は致命的でない新しいバグ＝次の版（v0.2.0）。起こるのは置き場にバルーンが無いときだけで、配布物には既定のバルーン `areka-balloon` が入る。
- **直列の約束**: 2 本とも `crates/areka` の起動・切替の層を触る＝「本体の crate 分け」の 6 本（とくに `runtime-module-untangle`・`session-module-gather`・`session-crate-split`）と同じウェーブに置かない。パスは同じ節の対応表で読み替える。

#### Existing Spec Updates

- [ ] helper-first-load-refusal -- 2026-10-30 の追記: YAYA のゴースト（`konnoyayame.nar`）でも補助プログラムの `loadu` の後に `HRESULT(0x80004005)` が出た（試験は緑・ゴーストは起きた）。Dependencies: なし

#### Specs (dependency order)

- [x] install-before-boot -- `--install` と `initial_install.nar` を起動の解決の前に入れてから起動を続け、空の置き場からも起こす（優先・区分 A・v0.1.0）。Dependencies: startup-install-option
- [x] alert-dialog-per-monitor-dpi -- 告知の画面を拡大率の違うモニタでもにじまず出し、モニタを移した後のバルーンの描き直しの欠けを見分ける（バグ・区分 B・install-before-boot の完了時の棚卸）。Dependencies: install-before-boot
- [ ] balloon-dpi-switch-redraw -- モニタを移した後のバルーンが枠なしで矢印だけ見えた件を見分け、描き直しの欠けなら直す（バグ・区分 B・alert-dialog-per-monitor-dpi の完了時の棚卸）。Dependencies: alert-dialog-per-monitor-dpi
- [ ] ghost-switch-balloon-fallback -- 切替先にも置き場にもバルーンが無いとき、切り替えを落とさず代わりのバルーンで続ける（バグ・区分 B・v0.2.0）。Dependencies: なし

### window-work-area-clamp の持ち越し（2026-10-30 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `window-work-area-clamp` の完了時の棚卸（`completed/window-work-area-clamp/tasks.md` の Implementation Notes の「最終検証」の範囲外の項目 ①〜⑤・「組 5 の決めごと」②・設計の Risks「バルーンの横のずらしの符号」・`research.md` 12〜13 節）。新しい spec は 2 本、既存の brief への追記は 0 件。その場で解決 1 件＝④（環境の `PASTA_DEBUG=1` でほかのぱすたが 127.0.0.1:9276 を握ると実機の試験の検体が LOAD 失敗する）は `real-run-kit` が起こす子へ `PASTA_DEBUG=0` を渡して直した（`crates/real-run-kit/src/launch.rs`）。⑤（`placement/follow/window_move.rs` が前から 1,000 行を超えている・今 1,227 行）は起票しない＝前からの超過で、`audit-placement-follow` が見直す。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: ②（右に出すバルーンの横のずらしの向き）と ③（並べ替えたバルーンが相方の窓に重なる）は、触る場所がどちらもバルーンの置き方の式（`placement/balloon_limit.rs`・`placement/resolver.rs` P5）で、SSP の測りも 1 回の机でまとめて取れる＝1 本（`balloon-flip-side-rules` の件 1・件 2）。① の並べ直しの預かりは触る場所（`emo2_boot/frame/move_hold.rs`）が違い、本体の crate 分けとの順の縛りも違う＝別の 1 本。
- **版**: 2 本とも致命的でない新しいバグ＝次の版（v0.2.0）。v0.1.0 の検体のぱすた（配布版 `pasta-in-windows`）は本体と相方の窓が同じ位置に重なり、バルーンは左に出し（`sakura.balloon.alignment,left`）、起動の台本で並べ直しを流さない＝同 spec の 4.4 の通しでどの件にも当たっていない。v0.1.0 のゴール「ぱすたが SSP と同じに動く」には当たらないので、`ghost-switch-balloon-fallback` と同じく次の版に置いた。
- **直列の約束**: `balloon-flip-side-rules` は `placement/` を触る＝`placement-crate-split` と同じウェーブに置かない。`window-resetwindowpos-hold` は `emo2_boot` の位置の指令を流す所（`frame/move_hold.rs`・`frame/drain_resnap.rs`）を触る＝「本体の crate 分け」の鎖（とくに `runtime-module-untangle`・`runtime-crate-split`）と同じウェーブに置かない。パスは同じ節の対応表で読み替える。

#### Specs (dependency order)

- [x] balloon-flip-side-rules -- 反対の側へ並べ替えたバルーンが相方の窓に重なるときの扱いと、右に出すバルーンの横のずらしの向きを、SSP で測ってから裁定して直す（バグ・区分 B・v0.2.0）。Dependencies: なし
- [x] window-resetwindowpos-hold -- 起動の直後の `\![execute,resetwindowpos]` を、どのキャラ窓も最初の面を表示するまで預かり、仮の大きさで並べない（バグ・区分 B・v0.2.0）。Dependencies: なし

### animated-surface-log-flood の申し送り（2026-10-31 実装中に `/kiro-discovery` で起票）

> 出どころは `animated-surface-log-flood` の実装の記録（`verification.md` のタスク 5.2「release の交互の測り直し」・5.3「SSP の基準」、件 26 の直し `7b3c2d956`、実装中の clippy）。新しい spec は 0 本、既存の brief への追記が 3 件（小さい件は 1 件 1 spec にせず、触る場所が同じ brief へ「件 N」で足した）。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: ⑴ 面 1002 の何もしていないあいだの CPU（areka release 中央値 15.4%・SSP 2.9.26 は 5.6%）は、表示の適用の外の約 12 ポイントが主＝見込みは wintf の全面の描き直しと出す処理なので、差分の描き直しを持つ `wintf-visual-damage-redraw` の件 1 にした（内訳を測ってから縮め、SSP の水準を目標にする）。`present-emit-tail-latency` は適用の中の測っていない区間の尾を測る brief で、適用の外は持たないので足さなかった。⑵ `visible_parts` が有効でコマの無い `bind+random` の pattern0 の先を見える部品に数える件は、見えない部品の評価を扱う `seriko-trigger-lottery-gaps` の件 3 と同じ形＝同 spec の件 4（emo-compose の `nesting.rs` が触るファイルに加わる）。⑶ clippy の既存の指摘（emo-compose のテスト 5 か所・emo-present のテスト 7 件）は `clippy-199-lints` の件 1（emo-compose の 5 か所は列挙済み・emo-present が新しい）。
- **版**: `wintf-visual-damage-redraw` は v0.2.0 のまま（測りだけを先に取るかは同 brief の議題 3）。`seriko-trigger-lottery-gaps` は「v0.1.0 から外した 5 本」のまま（行き先は次の版を決めるときに・+2〜3 タスク）。`clippy-199-lints` は版に縛らない（+1 タスク）。
- **直列の約束**: `seriko-trigger-lottery-gaps` は emo-compose の `nesting.rs` も触る＝emo-compose の `plan.rs`・`nesting.rs` を触る spec と同じウェーブに置かない。

#### Existing Spec Updates

- [x] wintf-visual-damage-redraw -- 件 1（2026-11-14 同 spec の完了の棚卸で節ごと `pasta-idle-surface-cost-measure` へ移した）: ぱすたの面 1002 で何もしていないあいだの release 版の CPU（中央値 15.4%・WorkingSet 176 MB）が SSP 2.9.26（5.6%・94 MB）の約 3 倍。適用の外の約 12 ポイントの内訳（wintf の描き直し・present・DComp・刻み）を測ってから縮め、SSP の水準を目標にする。Dependencies: wintf-visual-tree, wintf-d2d-composite-rules
- [ ] seriko-trigger-lottery-gaps -- 件 4: emo-compose の `nesting.rs` の `bind_targets`／`visible_parts` が、有効でコマの無い `bind+random` の pattern0 の先（ぱすたの 1410）を見える部品に数え、`plan.rs` の `flatten_surface`（`7b3c2d956` で描かなくした）と食い違う。seriko の `rebuild`・`moving_visible` が使う。一致の檻に pattern0 を持つ `bind+random` の文面を足して固定する（区分 B）。Dependencies: なし（✅ 2026-11-06 `audit-emo-compose` のタスク 2.2〔指摘 4〕で解消）
- [ ] seriko-trigger-lottery-gaps -- 件 5・件 6（2026-11-06 `audit-emo-compose` の完了の棚卸から・区分 B・負荷）: seriko の `parts.rs` の `rebuild` が見える部品を不動点まで引き直す V² の写し（`unwrite` も欄を全部写す）と、emo-compose の `NestReport::from_world`・`shape_issues` が element定義ごとに `reaches` を引く深さの 2 乗。同 brief の「件 5・件 6」の節
- [ ] clippy-199-lints -- 件 1: emo-compose のテストの `chunks_exact` 5 か所（列挙済み・今も残る）と、emo-present のテスト 5 本の 7 件（`chunks_exact` 4・`type_complexity` 2・`assertions_on_constants` 1・列挙の外）。emo-present の本番のコードは `--no-deps` で 0 件。Dependencies: なし

### v0.1.0 の版の門の実機の申し送り（2026-11-01 `/kiro-discovery` で起票）

> 出どころは `doc/v0.1.0-gate-real-run.md`（ぱすたの 7 段とどっとさくらを実機で回した記録）の 4 節。新しい spec は 0 本、既存の brief への追記が 2 件（小さい件は 1 件 1 spec にせず、触る場所が同じ brief へ「件 N」で足した）。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: mustselect の脱衣を無視したときの WARN は `crates/areka-seriko/src/actor_bind.rs` の 1 か所。同じファイルを本体で触る `dressup-change-events` の件 1 にした。どっとさくらの `\b[1]`（左に置いたバルーンを奇数の面の絵で描き、しっぽが反対を向く）は SSP で測った（SSP は置いた側の偶数の面の絵で描く）＝面の偶数・奇数で左右を選ぶ項目 2 を持つ `balloon-canon-residue` に申し送った。4 節のほかの行は、方針どおり（ボールの透けた画素・2026-10-22 の裁定）か既存の持ち主あり（`draw-methods-canon`・`currentghost-property-others`）なので、ここでは起票しない。
- **版**: `dressup-change-events` は版に縛らない列のまま（+1〜2 タスク）。`balloon-canon-residue` も今の置き場のまま（項目 2 に含める・規模は変えない）。

#### Existing Spec Updates

- [ ] dressup-change-events -- 件 1: ぱすたで `mustselect カテゴリの脱衣指示を無視` の WARN が 28 分で 1,454 行。同じ（キャラクター・カテゴリ・パーツ）の 2 回目からは DEBUG へ下げる案（2026-08-11 の `bindoption-exclusivity` D1 の裁定を一部改める＝要件の討議で確かめる）。無視すること自体は変えない（区分 B）。Dependencies: なし
- [ ] balloon-canon-residue -- 項目 2 への申し送り: どっとさくらのメニューの台本 `\b[1]\0…` で、areka は左に置いたバルーンを奇数の面 1 の絵（しっぽが左）で描く。SSP は同じ台本で左に置き、偶数の面の絵（しっぽがさくらの方）で描いた（2026-11-01 に MCP で測った）。台本が奇数を書いても対の中から置いた側の絵を選ぶ（区分 B）。Dependencies: なし

### DwmFlush の待ちの補い（2026-11-04 `/kiro-discovery` で起票）

> 出どころは開発者の依頼（DwmFlush が S_OK のまま待たずに戻ると vblank のスレッドが空回りする）と、discovery で読んだ `vsync_loop` の `Err` の腕（notify を撃たず tick が止まる）。決めごとは brief の Approach（時間だけで判定・1ms 未満・約 16ms までの不足分を眠る・状態を持たない・notify はループの最後の 1 か所・HRESULT は debug のログだけ）。

#### Specs

- [→] wintf-dwmflush-wait-compensation -- （2026-11-06 `audit-wintf-runtime` に吸収・brief は同 spec に写した）vblank のスレッドが `DwmFlush` の待たない戻りで空回りしないよう、待てなかった回だけ眠りで補い、`Err` の回も必ず notify する（区分 A・版に縛らない）。Dependencies: なし

### `doc/COMPAT_ARCHITECTURE.md` の改名（2026-11-05 `doc-tree-tidy` のタスク 5.1 の `/kiro-discovery` で起票）

> 出どころは `doc-tree-tidy` の要件 6.5（同じウェーブ C17 の見直しとの約束で改名を見送り、例外の表に残した 1 つを後続へ渡す）。決めごとは brief の Constraints（名指すファイルを受け持つ見直しの spec と本体のクレート分けの 6 本とは同じウェーブに置かない・中身を変えない・依存を足さない）。

#### Specs

- [ ] doc-compat-architecture-rename -- `doc/COMPAT_ARCHITECTURE.md` を名前の決まりに合う名前へ改名し、案内の 3 つの表と名指しを同じ変更で直す（区分 C・版に縛らない）。Dependencies: doc-tree-tidy

### audit-parsers-sakura の申し送り（2026-11-06 同 spec のタスク 8.1 の `/kiro-discovery` で起票）

> 出どころは `audit-parsers-sakura` の要件 7.7（ウェイトの `最小-最大` からランダムに選ぶ）・design.md の「残る危うさ」（トークの終わりの 1 ulp）・「設計の討議で確定した 7 件」の 3（旧形の `\q` を選択肢として出す＝research.md 11 節の SSP の測り）と、tasks.md の Implementation Notes の「8.1 への申し送り」2 件（台帳の bindgroup の欄・`CuePlayer::stop`）。新しい spec は 0 本、既存の brief への追記が 4 件（小さい件は 1 件 1 spec にせず、触る場所が同じ brief へ足した・audit 系は推奨案で承認してよい＝2026-11-04 開発者）。spec 台帳の数（162）は変わらない。着手の席と版は変えない＝次の棚卸で組む。

#### Existing Spec Updates

- [x] audit-dola -- 件 1: トークの終わり（dola の `absolute_end_time` と占有の終わりの f64 の和）が ms の格子から 1 ulp 外れ、`TalkDone` が 1 刻み遅れうる。件 2: `CuePlayer::stop`（`dola/src/cue/runtime.rs:361`）を呼ぶ本番のコードが 0（区分 B 相当）。Dependencies: なし
- [ ] range-choice-tag -- 追記の件: 旧形の `\q0[ID][タイトル]` を SSP と同じく選択肢として出す（今は見えない Raw・SSP は選択肢として出すと MCP で測った・区分 C・+2〜3 タスク）。Dependencies: なし
- [ ] seriko-legacy-pattern-format -- 追記の件: パターンのウェイト `最小-最大` から再生のたびにランダムに選ぶ（今は最小値＋`warn!`・再生の側 `areka-seriko` にも触れる・区分 C・+2〜3 タスク）。Dependencies: なし
- [ ] coverage-roadmap-refresh -- 追記の件: 台帳 `assets.toml` の bindgroup の既定の欄（kero ほか）が「黙って壊れる」と書くが本番は `DefaultBinds::from_groups` で当てている・消えた `default_bind_ids` の名指し 54 か所と、台帳 `assets.toml` の 50 行と `briefing-assets.md` の消えた `package::resolve` の記述を直す（区分 B 相当・文書だけ・+1〜2 タスク）。Dependencies: なし
- [x] audit-emo-text-layout-draw -- 追記の件: `\n[3e38]` のような大きな有限の割合で改行を重ねると配置の位置が inf／NaN になる。配置の側（`areka-emo-text/src/layout_scan.rs`）で非有限の送りを守り `warn!`（audit-parsers-sakura の R17・区分 B 相当）。Dependencies: なし

### windows-rs の移行（2026-11-14 `/kiro-discovery` で起票）

> 出どころ（2026-11-14 開発者）: 「https://github.com/microsoft/windows-rs/issues/5369 をよみこんで、arekaプロジェクトへの適用可能性と難易度を深掘り判断し、計画せよ。最終的にはwindows-rsを0.101.0まで上げたいと思っている。」issue 5369 は開発者が上流へ出した縦書きの要望で、新しいクレート `windows-text` の縦書き対応（PR 5379・2026-11-13 マージ）で答えが返った。

- **測ったこと**（2026-11-14・一次情報・windows-rs の master `520097a3`）: crates.io の `windows` は 0.62.2 のまま（master の 0.100.0 は未公開）。`windows-core`・`windows-numerics` は crates.io に 0.100.0、master に 0.101.0（未公開）。`windows-text` は crates.io に 0.0.0（名前の確保だけ）で、縦書きは master にだけ在る。0.101.0 の公開日を述べた一次情報は無い。上流の文書は「再利用するライブラリは `windows` を避け、絞ったクレートと `windows-bindgen` の自分用の束縛を使う」「公開するライブラリは束縛を別の道具で生成してコミットする」「フィルターは API を 1 つずつ名指しし、メソッドも明示で選ぶ」。areka の `windows::` を綴る行は約 1,300 行（`wintf` が 721 行／206 ファイル）、`#[implement]` は SHIORI に 25 か所前後と `IDWriteTextRenderer1` に 2 か所。`wintf-winmsg-executor` 0.0.5 は公開面に `HWND` を出す。細かい事実は `pilot-windows-rs-next` の brief の Current State。
- **`windows-text` の当てはめ**: 使える。ただし差し替わるのは `areka-emo-text` の DirectWrite を呼ぶ下回り（レイアウトの生成・計測・下線の位置取り・縦書きの字形の指定）だけで、折り返し・列の配置・選択肢の当たりは areka の自前のまま残る。範囲ごとの色・字送り・縦中横・文字単位の折り返しは無い（色は `as_interface` 経由で生の `IDWriteTextLayout` を借りて付ける）。`as_interface` は `windows-core` 0.101 の型でしか通らないので、採用は移行の後。
- **決めたこと**（同日の答え）: ⑴ 行き先は「`windows` クレートを使わない」＝`windows-core`・`windows-result` などを 0.101.0（公開の前は git 版）へ移し、API は `windows-bindgen` の束縛で呼ぶ（開発者「windows クレートは事実上使えなくなる」「windows api を呼ぶ方法が、windows クレートから、windows-bindgen を使って API をビルドする方法に変わるはず」）。 ⑵ WinRT 側（Windows.UI.Composition・DispatcherQueue）も `windows-bindgen` で作り、`windows-composition` へは乗り換えない（開発者「将来的に reactor をつかうことになると、windows-composition はリフテッドに固定されてしまい、areka プロジェクトで使ってる system と衝突するため」・上流のソースでも `system` と `reactor` は排他）。 ⑶ `wintf-winmsg-executor` は areka のリポジトリへ取り込み、crates.io の公開を引き継ぐ（手元の `main` を写すだけ・サブモジュールにしない・版はワークスペースの版・ライセンスは `MIT OR Apache-2.0` のまま）。 ⑷ 今起票するのは取り込みと先進坑の 2 本だけで、移行の本体（6〜8 本の見込み）は先進坑の go の後に切り方を決めて起票する（最初の見立て「1 本・16 タスク前後」は、開発者の「ここの移行コストを甘く見て無いか？」で改めた）。 ⑸ `windows-text` の採用は移行の後で、`areka-emo-text` だけ（`wintf` の label・typewriter は据え置き）。
- **決まっていないこと**: git 版とリリースの兼ね合い。crates.io へ出すクレート（`wintf`・`dola`・取り込みの後の `wintf-winmsg-executor`）は git の依存を持てない。案 A（推す）＝公開しないクレートと C の関数だけの 2 つ（`wintf-winmsg-executor`・`dola`＝`windows-link` だけで移れる）を先に移し、`wintf` の切り替えは上流の 0.101 の公開の後。案 B＝移行のあいだ `wintf` の crates.io への公開を止め、リリースは zip だけ。先進坑の結果と一緒に開発者が決める。
- **採らなかったもの**: `windows` クレートの版を上げる 1 本（行き先が違う）・master を git で引いてすぐ本線の全クレートを移す（`wintf` が crates.io へ出せなくなる・上流が動く）・`windows-composition`／`windows-canvas` の `composition` 機能を描画の経路に入れる（⑵）・`wintf-winmsg-executor` の一時のサブモジュール（`.gitmodules` の足しと外しで手間が 2 回増えるだけ）。
- **windows-reactor 待ちとの関係**: reactor 待ちの spec 群は、移行の本体が上流になる（reactor の束縛は `windows-core` の新しい系列に乗る）。reactor 自体の待ちは別に残る。
- **先に確かめたこと**（2026-11-14・起票の後・別のセッションからの引継ぎの先行検証）: 上流の縦書きのサンプル `text-vertical` は rev `520097a` で組めて動き、付属の当たり判定のテストも通った。既定のままだと開き括弧（「『（）の墨が字の枠の上側に寄る（areka が `emo-text-dwrite-fidelity` で直したのと同じ現象）が、areka の 10 個の字形の指定を `set_font_features` に当てると直る。縦書きの下線は列の左側に出る。開発者が上流へ報告する。細かい事実は `pilot-windows-rs-next` の brief。
- **先進坑の置き場**（同日・開発者の問い＝今のコードを壊さずに確かめるには大規模な写しが要るのではないか）: 独立の小さなワークスペースにする（自分の `[workspace]` と `Cargo.lock`）。今のクレートは path で 0.62 のまま借り、写すのは `wintf-winmsg-executor` と `wintf` の `com/` の一部だけ（上限 1,500 行前後）。新旧の `windows-core` が同じ依存の木に並ぶことは解決で確かめた。`crates/pilot` の例には置けない（1 つのパッケージが `windows_core` の名前で持てる版は 1 つ）。
- **実装係のモデル**（同日・開発者「方針さえ決まれば、やることは機械的だから実装サブエージェントはsonnetでもいけそう」）: 先進坑が書き換えの対応表を出し、それだけで安い実装係が移せるか・取りこぼしがコンパイルと lint で捕まるかを問い 10 で確かめる。
- **版**（2026-11-14 棚卸㊲・開発者「windows-text導入関係は、次のリリースに含めてください。」）: この節の spec（取り込み・先進坑・移行の本体・`emo-text-windows-text`）は v0.2.0 に入れる。⑸ の「採用は移行の後」はそのまま＝v0.2.0 の中で順に進める。「決まっていないこと」の git 版とリリースの兼ね合いは、v0.2.0 の公開そのものに掛かるようになった（案 A なら v0.2.0 の公開は上流の 0.101 系と `windows-text` の crates.io の公開を待つ・案 B なら v0.2.0 では `wintf` を crates.io へ出さない）。移行の本体と v0.2.0 の `wintf` の作り直しの順は、本体の起票のときに決める。
- **移行の始め方の決め**: 上流の 0.101 系の crates.io への公開を待ち、出てから全部のクレートを一気に入れ替える（0.62 と 0.101 を混ぜて段階で移すことはしない＝上の「決まっていないこと」の案 A・案 B はどちらも取らない）。移す動機の一つは、areka から windows-reactor を使えることをはっきりさせること。公開は近い見込みなので急がない。
- **移行の本体の仮の切り方**（go の後に決め直す・どれも `_Depends(confirmed): pilot-windows-rs-next`）: 束縛を作る道具と見張りの段 → `wintf-winmsg-executor`・`dola` → COM の型をやり取りしないクレート → SHIORI の `#[interface]`・`#[implement]` → `wintf` → `areka-emo-*`・`areka-placement`・`areka` → `windows` 0.62 を外す締め → `emo-text-windows-text`。

#### Specs

- [x] wintf-winmsg-executor-import -- 別リポジトリの `wintf-winmsg-executor` を `crates/` へ写し、crates.io への公開を areka の公開の段へ引き継ぐ（動きは変えない・区分 A・v0.2.0〔2026-11-14 棚卸㊲で移した〕・S）。Dependencies: none
- [x] pilot-windows-rs-next -- `windows` クレートをやめて `windows-bindgen` の束縛へ移る移行を、小さく縦に通して確かめる先進坑（束縛の置き場所と形・WinRT・`windows-text` の縦書き・規模の実数・区分 A・v0.2.0〔2026-11-14 棚卸㊲で移した〕・M）。Dependencies: wintf-winmsg-executor-import

### wintf-visual-damage-redraw の持ち越し（2026-11-14 完了時の棚卸で `/kiro-discovery` で起票）

> 出どころは `wintf-visual-damage-redraw` の `requirements.md` の Out of scope の 2 行（文字の層のスクロール・件 1）。どちらも同 spec の要件の討議（2026-11-14）で開発者が決めた。新しい spec は 2 本。着手の席は決めていない＝次の棚卸で組む。

- **分け方**: ⑴ バルーンの文字の層（`areka-emo-text` の自前のスワップチェーン）を wintf の描画の木へ移し、文字の区画を自分の描画面に置いてスクロールを平行移動の写しで済ませる＝`balloon-text-visual-tree`（開発者「本来、バルーンが使うはず。使ってないなら使うべきですね」）。⑵ 件 1（ぱすたの面 1002 で何もしていないあいだの CPU と WorkingSet）は、面 1002 がコマごとに全部の合成になるので差分の描き直しでは縮まず、測ることが本体＝`pasta-idle-surface-cost-measure`（同 brief の件 1 の節をそのまま移した・直す前の数は入れ物の後の main で取り直す）。
- **版**: `balloon-text-visual-tree` は v0.2.0（`wintf-visual-damage-redraw` の後）。`pasta-idle-surface-cost-measure` は版に縛らない（ウェーブの外・静かな机で 1 本ずつ・`wintf-visual-damage-redraw` の着地の後）。

#### Specs

- [ ] balloon-text-visual-tree -- バルーンの文字の層を wintf の描画の木へ移し、文字の区画を自分の描画面に置いて、スクロールを `wintf-visual-damage-redraw` の平行移動の写しで済ませる（自前のスワップチェーンを消す・画素は今と一致・2026-11-14 `wintf-visual-damage-redraw` の完了の棚卸で起票・区分 A・v0.2.0・L）。Dependencies: wintf-visual-damage-redraw
- [ ] pasta-idle-surface-cost-measure -- ぱすたの面 1002 で何もしていないあいだの CPU と WorkingSet の内訳を、入れ物の後の main で測り直して記録する（測ることが本体・直しは数字の後・2026-11-14 `wintf-visual-damage-redraw` の完了の棚卸で起票・区分 B・ウェーブの外・S〜M）。Dependencies: wintf-visual-damage-redraw

### audit-placement-core の申し送り（2026-11-15 タスク 8.2 の後に `/kiro-discovery` で起票）

> 出どころは `audit-placement-core` の `research.md` 19.2（完了の申し送り）と 18.10（鎖の決まりの総括）。追随の持ち物の件は、持ち主の `audit-placement-follow` がもう完了していて受け手が無い（19.2）。`defaultx` の読みが ukadoc と違う件は、鎖の決まりの外の件として起票を決めた（18.10）。開発者は 2026-11-15 に 2 本の起票を承認した（「推奨どおりで」）。

- **分け方**: ⑴ 追随の持ち物の残り（1〜9 回目の申し送り・鎖の寄せのずれ・164・`follow/drag_follow.rs` の 1,000 行）＝`audit-placement-follow-residue`。方針は見直しと同じで、指摘は全部直す。鎖の振る舞いは `audit-placement-core` が採った `chain_rule.rs` の決まりに従う。 ⑵ `defaultx` の読みを ukadoc（2.9.13）に合わせる＝`defaultx-canon`。鎖の決まりの隙間 ⑵ はこの読みの上に立つので、`chain_rule.rs` を開け直す。
- **順**: どちらも `audit-placement-core` の着地の後（2026-11-15 着地済み）。`defaultx-canon` も `chain_rule.rs` と追随の `follow/work_area_clamp.rs` を通るので、`audit-placement-follow-residue` と同じウェーブに並べず、その着地の後に取る。
- **版**: `audit-placement-follow-residue` は v0.1.0 の後の隙間（wintf の `ecs/drag` に触る＝v0.2.0 の作り直しの前に済ませる）。`defaultx-canon` は v0.3.0（区分 B・新しく見つかった致命的でないバグは次の版＝「版の段」の決まり）。着手の席は次の棚卸で組む。

#### Specs

- [x] audit-placement-follow-residue -- 完了した `audit-placement-follow` の範囲に `audit-placement-core` のかけ直しが残した件（追随の寄せの組を鎖の決まりに合わせる・164 の `DraggingState` の「位置が分からない」・`drag_follow.rs` の分け方・写しと古い文と名前の無い番号）を全部直す（2026-11-15 起票・区分 A・v0.1.0 の後の隙間・M）。Dependencies: audit-placement-core
- [ ] defaultx-canon -- `defaultx`・`defaultleft`（と Y の軸）の読みを ukadoc（2.9.13）の画面の座標に合わせ、鎖の決まりの隙間の読みと 485・494・513・514 の裁定を開け直す（2026-11-15 起票・区分 B・v0.3.0・M〜L）。Dependencies: audit-placement-core, audit-placement-follow-residue

### 席替えの台本が使う正典の機能（2026-11-19 `/kiro-discovery` で起票）

> 出どころは、ghost_dev の「ぱすた開発」セッションが開発者の発注で書いた台本「席替え」（今の SSP だけで遊べるノベル・ぱすたが遊ぶ人のデスクトップを教室に見立てて窓ごと席を移る）。開発者「同じ台本を SSP でも areka でも動かす」「areka 環境に実装の起票投げておいてね」。台本が使う正典の機能のうち、areka に無い物・途中の物を起票した。開発者は 2026-11-19 に 2 本の起票を承認した（「推奨で起票」）。台本の本文は ghost_dev のリポジトリの物で、areka へは写さない。

- **分け方**: ⑴ 時間つきの窓の指令（`\![move]` の `--time`・`\![moveasync]`・`\![set,scaling]`・`\![set,alpha]`）＝`window-timed-directives`。3 つは「窓の単位の値を時間をかけて変える」同じ仕事なので 1 本にまとめた。`sakura-time-directives` が覚え書きへ戻していた D 群を引き取り、`element-animation-easing` の議題 1（土台の持ち主）は「先に別の spec を立てる」で決着した。 ⑵ `currentghost.scope(ID).currentmonitor` の残り 4 葉（`rect`・`work`・`primary`・`bpp`）＝`scope-currentmonitor-property`。前例の `mcp-scope-rect-property`・`shell-scale-linkage` と同じく、使う者が現れた葉だけを小さい spec に切り出した。
- **起票しなかった物**: `OnDisplayChange`・`OnDisplayChangeEx`。台本は必須としていない。タスクバーを避ける用途は `.work` で足りる見込み（覚え書きに残した）。
- **版**: どちらも v0.4.0（開発者 2026-10-17「アニメーションに絡むのは、dola の改造後」）。`scope-currentmonitor-property` は待つ物が無いので、席が空けば前に出してよい。着手の席は次の棚卸で組む。
- **演劇との関係**: pasta のノベルゲームの取り決め（`novel-areka-contract`）では、舞台の中のアクターの位置・大きさ・透明度は `play` の命令と転換で受ける。この 2 本は、ふつうのゴーストが正典のタグを打ったときの話で、別の物。
- **網羅台帳**: 持ち主の欄の付け替え（`sakura-script.toml` の `\![set,scaling]`・`\![set,alpha]`・`\![move]` の残り、`property.toml` の `currentmonitor` の 4 行）は、それぞれの spec の着手のときに行う（派生の文書の作り直しが要るため・`sakura-time-directives` の brief の宿題と同じ扱い）。

#### Specs (dependency order)

- [ ] window-timed-directives -- `\![move]` の時間・`\![moveasync]`（と cancel）・`\![set,scaling]`・`\![set,alpha]` を、時間なし・`--time`・`--wait` の形で等速に動かす（2026-11-19 起票・区分 A・v0.4.0・M）。Dependencies: v0.2.0 の時刻の土台（✅ `dola-interrupt-ticks`）
- [ ] scope-currentmonitor-property -- `currentghost.scope(ID).currentmonitor` の `rect`・`work`・`primary`・`bpp` を、聞かれた時に取りに行く形で出す（2026-11-19 起票・区分 A・v0.4.0・S）。Dependencies: なし

### ノベル（演劇）の暫定の設計の議論で決まった事（2026-11-20）

> 2026-11-19 に、ノベルに関わる 8 つの単位で暫定の設計と検証を行い（記録は各 spec のフォルダの `provisional-design.md`・`provisional-design-validation.md` と `doc/ideas/novel-provisional-design-summary-2026-11-19.md`）、areka の中で閉じる議題を 2026-11-20 に開発者と 1 つずつ議論した。決めの全文と開発者の言葉は `doc/ideas/novel-decisions-2026-11-20.md`。

- **立場**（開発者）: 「arekaの立場は、『イベントを投げて、さくらスクリプトが返ってきたら、再生する』という仕組みを壊さずに、仕様を拡張する姿勢」「SSPは『さくらスクリプトプレイヤー』の略ですが、arekaも似た立場。違うのはさくらスクリプトの並走を許容する設計に移行し、表現力を会得すること」。既定の動作（ukadoc の動き）に影響が出る決定は取らない。
- **台本の命令で作る物**（名前は仮・pasta の取り決めで決める）: 再生を早送りする命令（絵のクリックには、SHIORI がこれを並走の 1 本で返す）・高速再生の入切・続きを示すキューコマンド（`\x` とは別。「このスクリプトには続きがあります」「クリックで進めてください」）。
- **高速再生基盤と道具**: 基盤を 1 つ作り、`\_q`・吹き出しのクリックの早送り・早送りの命令・高速再生の入切・出し直しの印を、その上の道具にする。順は、dola の直し → 基盤 → 正典の道具 → areka の命令の道具。`talk-fast-forward` と `sakura-time-directives` の切り方は、この向きで見直す。
- **時期**: dola の直し（v0.2.0）が最優先。`play` に関わる物は v0.3.0 より後。`\_q` は優先度が高く、ノベルゲームを出す頃には動かす（「いつか」の段から出す）。**段と列への反映は、次の棚卸で行う。**
- **命令の配り方**: 同じ命令を複数の受け口が拾ってよい。`consumer_ledger.rs` の「1 つの命令を拾う係は 1 つまで」の縛りは外す（表は残す）。コードの直しは、最初に複数の受け口が要る spec が行う。
- **そのほか**: 早送りは中身を見ない・並走は並走（areka は裁かない）・値を時間で変える仕組みは dola が持つ・素材の決まりの差し替えと要求と「替えさせない」の指定は brief のまま残す。
- **作り直しと、その後の決め**（2026-11-20）: 単位をまたぐ骨組み（`doc/ideas/novel-redesign-2026-11-20/00-architecture.md`）を先に決めて、7 つの単位を作り直し、3 つの目で検証した（同じフォルダ）。開発者が承認した 4 件: 吹き出しを押したら早送り、を命令なしで足す／続きの 1 本が届いたら、字を消して空の箱を出したままにする／高速再生の間は、並走の区間も一度に配る／**時刻の土台（✅ `dola-interrupt-ticks`）に、ノベルの基盤が求める 5 つを、v0.2.0 の要件の段で入れる前提で確かめる**（その要件に入って着地した）。pasta の側の決め: 立ち位置は pasta が命令で渡す（役は要らなくなった）・右クリックのメニューは差し替えない。
- **起票**（開発者の承認・正典の拾い残し・`play` の命令ではない）: `sakura-raise-directive`・`key-press-event`。置く波は棚卸で決める。

#### Specs (dependency order)

- [ ] sakura-raise-directive -- 正典の `\![raise,イベント名,r0,…]` を、台本の受け口から kanade を通して SHIORI へ GET で送る（2026-11-20 起票・区分 A・5〜7 タスク）。Dependencies: なし
- [ ] key-press-event -- 正典の `OnKeyPress` を、キーの入力から kanade を通して SHIORI へ GET で送る（2026-11-20 起票・区分 A・4〜5 タスク）。Dependencies: なし

### test-crash-report-merge の持ち越し（2026-11-21 完了時の棚卸で起票）

> 出どころは `test-crash-report-merge` の `requirements.md` の Out of scope（決定 5）「`dola` にテスト用の組み立てを置く件として起票する」。その場で直さなかった訳＝直し方が複数あり、公開する crate（`dola`）の公開の形の裁定が要る。新しい spec は 1 本。着手の席は決めていない＝次の棚卸で組む。

- **版**: `dola-cue-test-builder` は v0.1.0 の後の隙間（重ならない見直しの取り残し）。先に要るものは無い。

#### Specs

- [x] dola-cue-test-builder -- 正準のキャリアの合図を組むテスト用の関数の写し 3 本（areka-ghost・areka-seriko・areka-runtime）を `dola` の側の 1 つにまとめ、呼び手を付け替える（置き場と演者の既定は議題・2026-11-21 `test-crash-report-merge` の完了の棚卸で起票・区分 A・v0.1.0 の後の隙間・S）。Dependencies: なし

### audit-areka-input-menu の持ち越し（2026-11-22 完了時の棚卸で起票）

> 出どころは `audit-areka-input-menu` のかけ直しの audit の指摘（research.md 12.3「`spawn_assembly_tests.rs` が `t_i4_…` のテストの名を指す」）。その場で直さなかった訳＝規模が spec 1 本に当たる（3 つの crate の 29 ファイル・284 本のテストの名と、名指す文書と brief）。新しい spec は 1 本。着手の席は決めていない＝次の棚卸で組む。

- **版**: `test-name-work-tags` は v0.1.0 の後の隙間（重ならない見直しの取り残し）。先に要るものは無い。

#### Specs

- [x] test-name-work-tags -- テストの関数の名の頭に残る作業番号の札（`t_i4_`・`t_c3_` など・29 ファイル 284 本）を外し、名指しを直す（2026-11-22 `audit-areka-input-menu` の完了の棚卸で起票・区分 A・v0.1.0 の後の隙間・M）。Dependencies: なし

### balloon-script-offset-align の切り出し（2026-11-27 要件の討議で起票）

> 出どころは `balloon-script-offset-align` の `requirements.md` の Out of scope と「決まったこと」の 6。同 spec は接尾辞の無い descript のずらしだけを台本の向きと `@` の相対の指定の基準にし、方向つきのずらし（descript の `balloon.offsetxl`・`offsetxr`・`offsetxc`・`offsetxb` と y の同じ綴り）と surfaces.txt の surfaceブレスのバルーンのずらしは読まない。どちらも受け持ちの spec が無い（網羅台帳 `assets.toml` の 10 行が `absent`・宛先が空）。新しい spec は 1 本。着手の席は決めていない＝次の棚卸で組む。同じ日の同 spec の設計の討議（議題 5）で、頭上・足元に出すときの縦の基準の SSP との違い（同 spec の `research.md` 8 節）も同じ spec の件 2 に足した。

- **分け方**: descript の方向つきの綴りと surfaces.txt のずらしは、同じ選び方（ファイルの層・方向・接尾辞の探す順）で 1 組に決まるので 1 本にまとめた。縦の基準（件 2）は同じ式（`resolver.rs` の `keyword_balloon_pos`）と ukadoc の同じ段落（0 の位置が方向ごとに違う）なので、別の spec にせず件 2 とした。
- **順**: `balloon-script-offset-align` の着地の後（`crates/areka-placement/src/resolver.rs` と台本の値の層を触る）。surfaces.txt の読み手（`crates/areka-parsers/src/shell/{model,decode}.rs`）を触るシェルの element の列の spec とは同じウェーブに置かない。
- **版**: いつか（区分 C でバグではない・手元のゴーストと検体に使うものが無い）。上流の `balloon-script-offset-align`（v0.2.0）より後の版。

#### Specs (dependency order)

- [ ] balloon-directional-offset-keys -- descript の方向つきのずらしと surfaces.txt の surfaceブレスのバルーンのずらしを読み、ukadoc の探す順と優先で 1 組を選んでバルーンを置く・サーフェスの切り替えで選び直す。件 2＝頭上・足元の縦の基準を SSP で測り、合わせるかを決める（2026-11-27 起票・区分 C・いつか・M〜L）。Dependencies: balloon-script-offset-align
