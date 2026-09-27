areka（α 版）をお使いになる方へ
================================

areka は、デスクトップにキャラクター（ゴースト）を住まわせる Windows 用のアプリです。
この説明書は 2026-09-26 時点の内容です。α 版なので、書いてあることは今後変わります。


■ 起動

・展開したフォルダの中の areka.exe を開きます。引数は要りません。
・同じフォルダに ghost フォルダ（ゴースト）と balloon フォルダ（吹き出し）があり、areka はここから選んで起動します。
・初めての起動では、同梱のゴースト「えも？？」（ghost\emo2）が、同梱の吹き出し emo2-kakukaku で立ちます。
・2 回目からは、前回使ったゴーストと吹き出しで立ちます。


■ 終了

・キャラクターを右クリックして、メニューの「終了」を選びます。別れの台詞のあとに終わります。
・Windows から閉じる操作（Alt＋F4 など）をしても、同じように別れの台詞のあとに終わります。
・反応しなくなったときは、キャラクターを Ctrl と Shift を押しながら左ダブルクリックすると、台詞なしですぐに終わります。


■ 右クリックメニュー

・キャラクターを右クリックすると出ます。
・今の版で出る項目は「説明書」と「終了」の 2 つです。
　・「説明書」: ゴーストの説明書（えも？？ なら ghost\emo2\readme.txt）を開きます。説明書のファイルが無いゴーストでは選べません。
　・「終了」: areka を終わります（上の「終了」を参照）。
・ゴーストが項目の名前を用意している場合は、その名前で出ます。
・ゴースト・シェル・バルーンの切り替え、インストール、ネットワーク更新の項目は、今の版ではメニューに出ません。


■ 記憶の置き場

areka は、前回使ったゴーストなどを次のフォルダに覚えます。

・areka の記憶: areka.exe の隣の profile\areka\
・ゴーストの記憶: ghost\emo2\ghost\master\profile\
・シェルの記憶: ghost\emo2\shell\master\profile\

これらのフォルダを消すと、初めての起動と同じ状態に戻ります（ゴーストが覚えていたことも消えます）。


■ 既知の制限

・areka.exe には署名がありません。開くときに Windows が警告を出すことがあります。
・Windows 10／11 の 64 ビット版専用です。
・深いフォルダに展開しないでください。フォルダの場所（パス）が長いと、同梱のゴーストが何も話さなくなることがあります。C:\areka のような短い場所をおすすめします。
・α 版の時点では、次のことはできません: ゴースト・シェル・バルーンの切り替え、.nar ファイルからのインストール、ネットワーク更新。


■ .nar の入れ方

（未記入: alpha-release-signoff が仕上げます）


■ 同梱物とライセンス

この配布物には、areka 本体のほかに、ほかの作者の作品が入っています。
areka 本体の MIT ライセンスは本体だけのもので、同梱の作品には及びません。
同梱の作品は、それぞれの作者の条件に従ってください。

◆ areka 本体（areka.exe・shiori-host32-helper.exe）
　作者: ekicyou
　条件: MIT ライセンス（LICENSE-MIT）。同梱のゴースト・シェル・吹き出しには及びません。
　本体が使っているライブラリの著作権表示とライセンスは THIRD-PARTY-NOTICES.md にあります。

◆ ゴースト「えも？？」（ghost\emo2）の辞書・スクリプト
　作者: えちょ（ekicyou）
　条件: ゴーストの中に利用条件の記載はありません。
　出どころ: ghost\emo2\readme.txt、配布サイト https://ekicyou.github.io/ghost_dev/emo2/

◆ SHIORI「pasta.dll」（ghost\emo2\ghost\master\pasta.dll・32 ビット）
　作者: ekicyou
　条件: MIT ライセンス
　出どころ: pasta の LICENSE（https://github.com/ekicyou/pasta）

◆ シェル \0 側「コンフィズリー」（ghost\emo2\shell\master）
　作者: ゆゆぴか
　条件: MIT ではありません。シェル作者の条件に従います。
　　areka の最初のゴースト（えも？？）の絵として使うことはできますが、シェルを抜き出して利用することはできません。
　　作者の説明書で禁じられていること: フリーシェルとしての再配布、伺か関連物以外での使用、商用利用、立ち絵の左右反転。
　出どころ: ghost\emo2\shell\master\readme.txt（同じ内容が ghost\emo2\shell\master\confiserie.txt にもあります）、作者のサイト https://yusyuparo.net/

◆ シェル \1 側「City-Pop'n」（ghost\emo2\shell\master）
　作者: 大槻
　条件: MIT ではありません。シェル作者の条件に従います。
　　作者の説明書には「改変や転用、伺かゴースト以外での使用の一切は自由」「使用許可の請求も不要」とあります。
　　ただし、えも？？ のシェルは \0 側と \1 側が 1 つのシェルにまとまっているため、シェルごと抜き出して利用することはできません（\0 側の条件による）。
　出どころ: ghost\emo2\shell\master\CityPop.txt、作者のサイト http://th88.blog.shinobi.jp/

◆ 吹き出し「emo2-kakukaku」（balloon\emo2-kakukaku）
　作者: ekicyou
　画像素材: フキダシデザイン（https://fukidesign.com/）
　条件: 画像素材はフキダシデザインの利用規約に従います（著作権の表記は不要、アプリへの組み込みは 1 つにつき 20 素材まで無料、データの再配布は禁止）。
　　areka と えも？？ の吹き出しとして使うことはできますが、画像を抜き出して利用することはできません。
　出どころ: ghost\emo2\readme.txt（「利用バルーン」の節）、フキダシデザインの利用規約 https://fukidesign.com/terms

◆ 吹き出し「Balloon for Staysee Syncfield」（balloon\StayseeBalloon）
　作者: ぽな
　条件: CC0 1.0（パブリックドメイン）
　出どころ: balloon\StayseeBalloon\LICENSE、balloon\StayseeBalloon\readme.txt、https://github.com/ponapalt/StayseeBalloon
