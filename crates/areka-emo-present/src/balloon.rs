//! `BalloonFrameSource`（balloon.rs）: バルーン枠画像を **シェルと同一の** compose/present 経路へ
//! 載せる入力適合層（R5.1）。
//!
//! M-boot のバルーンは fixture の枠画像（`balloons{N}.png`）だけを入力とする。本モジュールは
//! それらを列挙し、**synthetic surfaces.txt テキスト**（`surface{N}` に単一 overlay element
//! `balloons{N}.png`）を生成 → `areka_parsers::shell::parse` → `areka_emo_atlas::bake` →
//! `EmoWorld::build`＋`bind_atlas` と、シェルが辿るのと**寸分違わぬ公開 API 経路**で
//! `(EmoWorld, AtlasTable)` を組み上げる。直 WIC バイパスは設けない（R5.1）。
//!
//! # 正典整理（本モジュールが従う分類）
//!
//! - **枠画像のみ入力**（R5.3）: 列挙対象は `balloons{N}.png` に限る。`balloonc*`（入力ボックス）・
//!   `arrow*`（スクロール矢印）・`marker`（`\![*]` マーカー）・`online*`（受信アニメ）・相方側
//!   `balloonk*` は列挙しない。
//! - **PNG α 尊重**（R5.2）: `use_self_alpha,1` 相当＝[`UseSelfAlpha::On`] で bake する。emo2 kakukaku は
//!   `.pna` 無し・PNG α のみ（fixture 実測）で、`.pna` 対応は [`ElementDecoder::probe_pna`] の既存
//!   seam に委ね本 spec では追加しない。
//! - **surface id = N**（`balloons{N}` の N をそのまま採用）。`balloon.defaultsurface` 既定 0 と整合。
//!
//! 失敗経路は log-first（`tracing::error!`＋`Err`・silent failure 禁止）。枠が 1 枚も無い／bake が
//! エラーを産んだ場合は、真因をログへ出したうえで [`PresentError::Compose`]
//! （[`ComposeError::EmptyComposition`]）へ畳む。EmptyComposition は下流で Hide 縮退として許容される
//! ため（設計ディスカッション #1）、バルーン構築失敗はゴーストごと殺さず穏当に縮退する。
//!
//! [`UseSelfAlpha::On`]: areka_emo_atlas::UseSelfAlpha::On
//! [`ElementDecoder::probe_pna`]: areka_emo_atlas::ElementDecoder::probe_pna
//! [`ComposeError::EmptyComposition`]: areka_emo_compose::ComposeError::EmptyComposition

use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, ElementDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{ComposeError, EmoWorld};

use crate::command::PresentError;

/// 列挙されるバルーン枠画像のファイル名接頭辞（本体側吹き出し・相方側 `balloonk*` は対象外）。
const FRAME_PREFIX: &str = "balloons";
/// 列挙対象の拡張子（小文字比較）。
const FRAME_SUFFIX: &str = ".png";

/// 系列族の定義（**表データ**・候補追加が構造改変を伴わない形・R1.9）。
///
/// 正典のバルーン資産は族をまたいで同一の scope 別接尾辞体系を持つ——吹き出し
/// `balloons` / `balloonk` / `balloonp{n}def`、スクロール矢印 `arrows` / `arrowk` /
/// `arrowp{n}def`、マーカー `markers` / `markerk` / `markerp{n}def`……。本構造体は
/// その体系を **族名でパラメタ化**して保持するため、他族の scope 別対応（本仕様では
/// 対象外）が同じ機構をそのまま再利用できる。
///
/// 旧名候補は **可変長**である。装飾族には旧名がもう一段深く存在する（正典「`arrows` が
/// 本体用・旧バージョン対応のために `arrow` で代用を推奨」／`markers` に対する `marker` も
/// 同型）ため、`scope0_legacy` に `["arrows", "arrow"]` と積めば構造改変なしに表現できる
/// （R7.1(c) の語彙記録に対する縮退シーム）。
#[derive(Debug, Clone, Copy)]
pub struct SeriesFamily {
    /// 族の基底名（吹き出し族＝`"balloon"`・正規名は `{base}p{n}def`）。
    pub base: &'static str,
    /// scope 0 の旧名候補列（吹き出し族＝`["balloons"]`）。
    pub scope0_legacy: &'static [&'static str],
    /// scope 1 の旧名候補列（吹き出し族＝`["balloonk"]`）。
    pub scope1_legacy: &'static [&'static str],
}

/// 吹き出し族（本仕様で唯一実装する族）。
pub const BALLOON_FAMILY: SeriesFamily = SeriesFamily {
    base: "balloon",
    scope0_legacy: &["balloons"],
    scope1_legacy: &["balloonk"],
};

/// 連鎖内の 1 接頭辞（採用時の分類タグ付き）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesPrefix {
    /// 探索に用いる接頭辞（例: `"balloonp1def"` / `"balloonk"`）。
    pub prefix: String,
    /// この接頭辞が連鎖内で担う役割。
    pub tier: ChainTier,
}

/// 連鎖内での役割（フォールバック分類を運ぶ——採用時の警告判定・対応表記録の区分に用いる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainTier {
    /// 当該 scope 自身の候補（正規名＋当該 scope の旧名）。
    Own,
    /// n≧2 連鎖の**名指し**相方系列（`balloonk`）。正典が三人目以降の流用先として名指しした
    /// 系列であり、scope 1 の解決へ再帰的に縮退するものではない。
    KeroNamed,
    /// デフォルト定義（`balloonp0def` → `balloons`）。全連鎖の最終受け皿は scope 0 のみが
    /// 持つ地位であり、scope 1 はデフォルトの地位を持たない。
    Default,
}

impl SeriesFamily {
    /// 当該 scope の**正規名** `{base}p{scope}def`。
    ///
    /// 正典は p 系列を `\p[2]` 以降としてのみ記述しており、scope 0 / 1 の正規名
    /// （`balloonp0def` / `balloonp1def`）を先行探索することは areka 裁量の正規化拡張である
    /// （R1.10・対応表へ記録する対象）。
    fn canonical(&self, scope: u32) -> String {
        format!("{}p{scope}def", self.base)
    }

    /// 当該 scope の**旧名候補列**（scope 0 / 1 のみが持ち、scope 2 以上は旧名なし＝空）。
    fn legacy(&self, scope: u32) -> &'static [&'static str] {
        match scope {
            0 => self.scope0_legacy,
            1 => self.scope1_legacy,
            _ => &[],
        }
    }
}

/// scope 番号から**接頭辞優先連鎖**を導出する（純関数・表データ駆動・R1.1/1.8/1.9）。
///
/// 連鎖は 3 段の連結である——
///
/// ```text
/// chain(s) = Own(s) ++ KeroNamed(s≧2 のみ) ++ Default(s≧1 のみ)
///   Own      : 正規名 {base}p{s}def ＋ 当該 scope の旧名候補列
///              （s=0 → [balloonp0def, balloons] / s=1 → [balloonp1def, balloonk] / s≧2 → [balloonp{s}def]）
///   KeroNamed: 正典が名指しする相方系列の旧名候補列（[balloonk]）
///   Default  : 正規名 {base}p0def ＋ scope 0 の旧名候補列（[balloonp0def, balloons]）
/// ```
///
/// 各段では**正規名を先頭・旧名を後続**に置く。結果として
/// scope 0 は `balloonp0def` → `balloons`、scope 1 は `balloonp1def` → `balloonk` →
/// `balloonp0def` → `balloons`、scope n（n≧2）は `balloonp{n}def` → `balloonk` →
/// `balloonp0def` → `balloons` となる。
///
/// 相方系列の段は**正典が名指しした系列**であって scope 1 の解決へ再帰的に縮退するもの
/// ではないため、scope 2 以上の連鎖に scope 1 の正規名 `balloonp1def` は含めない（R1.1）。
///
/// scope は最後まで**数値のみ**で扱う。`Sakura`/`Kero` 等の 2 値列挙も、さくらスクリプト側の
/// 別語彙（`\h`/`\u`）も内部の正準表現としない（R1.9）。ゆえに解決規則は M1 の実行時 scope
/// （0 と 1）に閉じず、scope 番号一般（n≧2 を含む）で定義される（R1.6）。
pub fn prefix_chain(family: &SeriesFamily, scope: u32) -> Vec<SeriesPrefix> {
    let mut chain: Vec<SeriesPrefix> = Vec::new();

    // (i) 当該 scope 自身の候補: 正規名を先頭に、当該 scope の旧名を後続に置く。
    chain.push(SeriesPrefix {
        prefix: family.canonical(scope),
        tier: ChainTier::Own,
    });
    for name in family.legacy(scope) {
        chain.push(SeriesPrefix {
            prefix: (*name).to_string(),
            tier: ChainTier::Own,
        });
    }

    // (ii) 相方系列（scope 2 以上のみ）: 正典が名指しした系列そのもの＝旧名候補列のみ。
    // scope 1 の解決への再帰縮退ではないため、scope 1 の正規名はここに含めない。
    if scope >= 2 {
        for name in family.scope1_legacy {
            chain.push(SeriesPrefix {
                prefix: (*name).to_string(),
                tier: ChainTier::KeroNamed,
            });
        }
    }

    // (iii) デフォルト定義（scope 1 以上のみ）: 最終受け皿は scope 0 の系列。
    if scope >= 1 {
        chain.push(SeriesPrefix {
            prefix: family.canonical(0),
            tier: ChainTier::Default,
        });
        for name in family.scope0_legacy {
            chain.push(SeriesPrefix {
                prefix: (*name).to_string(),
                tier: ChainTier::Default,
            });
        }
    }

    chain
}

/// `balloon_dir` から枠画像を列挙し `(surface_id, ファイル名)` を **surface id 昇順**で返す。
///
/// `balloons{N}.png`（N は非負整数）だけを枠として採り、`balloonc*`/`arrow*`/`marker*`/`online*`・
/// 相方側 `balloonk*` は名前段で除外する（R5.3）。ファイル名の大小は無視して判定するが、element
/// path として使う値は **実ファイル名を原形のまま**保持する（実 WIC デコードが実パスを読むため）。
///
/// ディレクトリ走査に失敗した場合は log-first で [`PresentError`] を返す。
fn enumerate_frames(balloon_dir: &Path) -> Result<Vec<(u32, String)>, PresentError> {
    let read_dir = std::fs::read_dir(balloon_dir).map_err(|e| {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            error = %e,
            "balloon: 枠画像ディレクトリの走査に失敗"
        );
        PresentError::Compose(ComposeError::EmptyComposition(0))
    })?;

    let mut frames: Vec<(u32, String)> = Vec::new();
    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // 個別エントリの取得失敗は致命ではない（他エントリ継続・log-first）。
                tracing::warn!(error = %e, "balloon: ディレクトリエントリの取得に失敗（スキップ）");
                continue;
            }
        };
        let file_name = entry.file_name();
        let name = match file_name.to_str() {
            Some(n) => n,
            None => continue, // 非 UTF-8 名は枠画像規約外＝スキップ。
        };
        if let Some(id) = frame_id(name) {
            frames.push((id, name.to_string()));
        }
    }

    // surface id 昇順で決定化（ディレクトリ走査順は非決定ゆえ明示ソート）。
    frames.sort_unstable_by_key(|(id, _)| *id);
    Ok(frames)
}

/// `balloons{N}.png`（大小無視）なら surface id `N` を返す。枠画像でなければ `None`。
///
/// 接頭辞は `balloons` 固定ゆえ `balloonc*`/`balloonk*`（8 文字目が `s` でない）は自然に外れる。
fn frame_id(name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_prefix(FRAME_PREFIX)?.strip_suffix(FRAME_SUFFIX)?;
    // 接頭辞と拡張子の間は 10 進整数のみ（空・非数字は枠画像でない）。
    stem.parse::<u32>().ok()
}

/// 枠 `(surface_id, ファイル名)` 列から synthetic surfaces.txt テキストを生成する（転記層の流儀）。
///
/// 各枠は `surface{N}` ブロックに単一 overlay element（`element0,overlay,{ファイル名},0,0`）として
/// 転記する。`areka_parsers::shell::parse` が受理する surfaces.txt 文法に忠実で、独自構文は発明
/// しない（surface ヘッダ→`{`→element 行→`}` の登場順ストリーム）。
fn synthetic_surfaces_txt(frames: &[(u32, String)]) -> String {
    let mut text = String::new();
    for (id, file_name) in frames {
        // `surface{N}` ブロック・単一 overlay element（layer 0・オフセット 0,0）。
        text.push_str(&format!(
            "surface{id}\n{{\nelement0,overlay,{file_name},0,0\n}}\n\n"
        ));
    }
    text
}

/// バルーン枠画像を **シェルと同一の** compose/present 経路へ載せ `(EmoWorld, AtlasTable)` を返す。
///
/// `balloon_dir` 内の `balloons{N}.png` を枠として列挙（R5.3）→ synthetic surfaces.txt →
/// `shell::parse` → `bake`（PNG α 尊重＝[`UseSelfAlpha::On`]・R5.2）→ `EmoWorld::build`＋`bind_atlas`
/// と、直 WIC バイパス無しでシェルと同一機構に載せる（R5.1）。得た組を `attach_target` に渡すだけで
/// バルーン target がシェルと同じ提示経路へ乗る。
///
/// 枠が 1 枚も無い／bake がエラーを産んだ場合は log-first で真因をログし
/// [`PresentError::Compose`]（[`ComposeError::EmptyComposition`]・Hide 縮退許容）を返す。
pub fn build_balloon_target(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
) -> Result<(EmoWorld, AtlasTable), PresentError> {
    let frames = enumerate_frames(balloon_dir)?;
    if frames.is_empty() {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            "balloon: 枠画像（balloons{{N}}.png）が 1 枚も見つからない"
        );
        return Err(PresentError::Compose(ComposeError::EmptyComposition(0)));
    }

    // synthetic surfaces.txt をシェルと同一の parser で解釈する（転記層・R5.1）。
    let text = synthetic_surfaces_txt(&frames);
    let shell = areka_parsers::shell::parse(&text);

    // PNG α 尊重（use_self_alpha,1 相当・R5.2）で bake。base_dir は balloon_dir（実パスは
    // base_dir.join(rel) で一度だけ実体化される）。
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: balloon_dir,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], decoder, PackConfig::default());

    // bake の脱落（decode/normalize 失敗）は log-first で真因を出し、構築失敗として畳む。
    // M-boot の枠は固定小集合ゆえ全枚デコード成功が前提＝脱落は制作者ミス/配置不備の兆候。
    if !baked.errors.is_empty() {
        for err in &baked.errors {
            tracing::error!(
                balloon_dir = %balloon_dir.display(),
                error = %err,
                "balloon: 枠画像の bake に失敗"
            );
        }
        return Err(PresentError::Compose(ComposeError::EmptyComposition(0)));
    }

    // シェルと同一の compose 前段: World 構築 → アトラス束縛（SetId(0)・resolve は本呼び出し限り）。
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));

    Ok((world, baked.table))
}

#[cfg(test)]
mod tests {
    use super::*;

    use areka_emo_atlas::MemoryDecoder;
    use areka_parsers::shell::parse;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    // ── テスト用一時ディレクトリ（新規 dev-dep を避け std のみで構成）─────────────
    // `std::env::temp_dir()` 配下へプロセス id ＋単調カウンタで一意なサブディレクトリを作り、
    // Drop で後始末する（tempfile 相当の最小実装）。

    static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Drop 時に自身を再帰削除する一時ディレクトリ。
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "areka-emo-present-balloon-{}-{}",
                std::process::id(),
                n
            ));
            std::fs::create_dir_all(&path).expect("一時ディレクトリ作成");
            TempDir { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        /// 空のプレースホルダファイルを作る（MemoryDecoder 経路ゆえ中身は不問・列挙対象のため名前のみ要）。
        fn touch(&self, name: &str) {
            std::fs::File::create(self.path.join(name)).expect("プレースホルダ作成");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// 不透明 1×1 PBGRA スペック（bake が placement を必ず産む＝非退化）。
    fn opaque_1x1() -> (u32, u32, u32, Vec<u8>, bool) {
        (1, 1, 4, vec![10u8, 20, 30, 255], true)
    }

    /// R5.1/R5.3 転記一致（観測完了基準）: synthetic surfaces.txt → `shell::parse` の往復で、
    /// 各枠の surface id（`{N}`）と element path（`balloons{N}.png`）が転記一致する。
    ///
    /// これはファイルシステム/デコードを一切要さない純粋な転記層の檻。
    #[test]
    fn synthetic_text_transcribes_frame_id_and_path() {
        let frames = vec![
            (0u32, "balloons0.png".to_string()),
            (1u32, "balloons1.png".to_string()),
        ];
        let text = synthetic_surfaces_txt(&frames);
        let shell = parse(&text);

        assert_eq!(shell.surfaces.len(), 2, "2 枠 → 2 surface");
        for (n, file_name) in &frames {
            let surface = shell
                .surfaces
                .iter()
                .find(|s| s.id == *n)
                .unwrap_or_else(|| panic!("surface id {n} が転記されていない"));
            assert_eq!(
                surface.elements.len(),
                1,
                "各枠は単一 overlay element へ転記される"
            );
            assert_eq!(
                surface.elements[0].path.as_str(),
                file_name,
                "element path が balloons{{N}}.png へ転記一致しない"
            );
            assert_eq!(surface.elements[0].layer, 0, "layer 0（element0）へ転記");
        }
    }

    /// 大小無視の枠判定と非枠除外（R5.3）: `frame_id` が `balloons{N}.png` からのみ N を得て、
    /// `balloonc*`/`balloonk*`/`arrow*`/`marker*`/`online*`・非数字・非 png を弾く。
    #[test]
    fn frame_id_matches_only_balloon_frames() {
        assert_eq!(frame_id("balloons0.png"), Some(0));
        assert_eq!(frame_id("balloons12.png"), Some(12));
        assert_eq!(frame_id("BALLOONS3.PNG"), Some(3), "大小無視");
        // 非枠（列挙対象外・R5.3）。
        assert_eq!(frame_id("balloonc0.png"), None, "入力ボックスは枠でない");
        assert_eq!(frame_id("balloonk0.png"), None, "相方側は枠でない");
        assert_eq!(frame_id("arrow0.png"), None);
        assert_eq!(frame_id("marker.png"), None);
        assert_eq!(frame_id("online0.png"), None);
        assert_eq!(frame_id("balloons.png"), None, "数字が無ければ枠でない");
        assert_eq!(frame_id("balloonsX.png"), None, "非数字は枠でない");
        assert_eq!(frame_id("balloons0.txt"), None, "非 png は枠でない");
    }

    /// R5.1/R5.2/R5.3 full build: `build_balloon_target` が枠のみを列挙し、シェルと同一の
    /// parse→bake→World 経路で `(EmoWorld, AtlasTable)` を返す。非枠ファイルは列挙されず
    /// アトラスにも World にも現れない。MemoryDecoder ゆえ実 PNG 不要で決定論。
    #[test]
    fn build_balloon_target_end_to_end_frames_only() {
        let dir = TempDir::new();
        // 枠 2 枚 ＋ 非枠 3 種を同ディレクトリへ配置。
        dir.touch("balloons0.png");
        dir.touch("balloons1.png");
        dir.touch("balloonc0.png"); // 入力ボックス（非枠）
        dir.touch("arrow0.png"); // スクロール矢印（非枠）
        dir.touch("marker.png"); // マーカー（非枠）

        // 枠のみデコーダへ登録（非枠は登録しない＝もし列挙されれば decode 失敗で露見する）。
        let mut dec = MemoryDecoder::new();
        let (w, h, stride, bytes, has_alpha) = opaque_1x1();
        dec.insert(dir.path().join("balloons0.png"), w, h, stride, bytes.clone(), has_alpha);
        dec.insert(dir.path().join("balloons1.png"), w, h, stride, bytes, has_alpha);

        let (world, table) =
            build_balloon_target(dir.path(), &dec).expect("枠 2 枚から Ok が返る");

        // アトラスに枠 2 枚のエントリがあり placement を持つ（PNG α 尊重で焼かれる・R5.2）。
        for rel in ["balloons0.png", "balloons1.png"] {
            let id = table
                .resolve(SetId(0), rel)
                .unwrap_or_else(|| panic!("{rel} がアトラスに解決されない"));
            assert!(
                table.entry(id).placement.is_some(),
                "{rel} は不透明ゆえ placement を持つ"
            );
        }
        // 非枠は列挙対象外ゆえアトラスに存在しない（R5.3）。
        assert_eq!(table.resolve(SetId(0), "balloonc0.png"), None);
        assert_eq!(table.resolve(SetId(0), "arrow0.png"), None);
        assert_eq!(table.resolve(SetId(0), "marker.png"), None);
        assert_eq!(table.len(), 2, "生存エントリは枠 2 枚のみ");

        // World は surface id = N（balloons{N} の N）を常駐させる。
        assert!(world.surface(0).is_some(), "surface id 0（balloons0）が World にある");
        assert!(world.surface(1).is_some(), "surface id 1（balloons1）が World にある");
        assert!(world.surface(2).is_none(), "存在しない id は None");
    }

    /// R5.5 多面バルーン fixture（偶数 id・正典準拠）: TempDir へ `balloons0.png`＋`balloons2.png`
    /// の 2 枚（偶数 id 0/2）を置き、`build_balloon_target` が **surface 0 と surface 2 の両面**を
    /// 列挙・構築した world を返すことを固定する。既存 `..._frames_only`（id 0/1）と対をなし、
    /// 面 id が飛び番（1 を欠く 0/2）でも各面が id=N でそのまま常駐することを実演する。
    /// MemoryDecoder ゆえ実 PNG 不要で決定論（既存流儀踏襲）。
    #[test]
    fn build_balloon_target_enumerates_multiple_even_id_faces() {
        let dir = TempDir::new();
        // 偶数 id の面 2 枚（1 を欠く飛び番）を test-local fixture として自前用意（R5.5）。
        dir.touch("balloons0.png");
        dir.touch("balloons2.png");

        // 両面をデコーダへ登録（MemoryDecoder 経路・実 PNG 不要）。
        let mut dec = MemoryDecoder::new();
        let (w, h, stride, bytes, has_alpha) = opaque_1x1();
        dec.insert(dir.path().join("balloons0.png"), w, h, stride, bytes.clone(), has_alpha);
        dec.insert(dir.path().join("balloons2.png"), w, h, stride, bytes, has_alpha);

        let (world, table) =
            build_balloon_target(dir.path(), &dec).expect("偶数 id 2 面から Ok が返る");

        // アトラスに 2 面が解決され placement を持つ（PNG α 尊重で焼かれる）。
        for rel in ["balloons0.png", "balloons2.png"] {
            let id = table
                .resolve(SetId(0), rel)
                .unwrap_or_else(|| panic!("{rel} がアトラスに解決されない"));
            assert!(
                table.entry(id).placement.is_some(),
                "{rel} は不透明ゆえ placement を持つ"
            );
        }
        assert_eq!(table.len(), 2, "生存エントリは偶数 id 面 2 枚のみ");

        // 多面列挙の要（本タスクの主張）: surface 0 と surface 2 の **両面** が world に常駐する。
        assert!(world.surface(0).is_some(), "surface id 0（balloons0）が World にある");
        assert!(world.surface(2).is_some(), "surface id 2（balloons2）が World にある");
        // 飛び番の欠番 id 1 は列挙対象に無いゆえ常駐しない（面 id=N の同一性を固定）。
        assert!(world.surface(1).is_none(), "欠番 id 1 は World に無い");
    }

    // ── 檻 1: scope→接頭辞優先連鎖の導出（R1.1/1.6/1.8/1.9/1.10・R7.1）─────────────

    /// `(接頭辞, tier)` の組で連鎖を突き合わせるための簡約形（表明の可読性のため）。
    fn chain_pairs(family: &SeriesFamily, scope: u32) -> Vec<(String, ChainTier)> {
        prefix_chain(family, scope)
            .into_iter()
            .map(|p| (p.prefix, p.tier))
            .collect()
    }

    /// R1.1: scope 0 の連鎖は `balloonp0def`（正規名）→ `balloons`（旧名）の 2 段で、
    /// 双方とも当該 scope 自身の候補＝tier `Own`。scope 0 は相方系列段もデフォルト段も持たない。
    #[test]
    fn prefix_chain_scope0_is_canonical_then_legacy_all_own() {
        assert_eq!(
            chain_pairs(&BALLOON_FAMILY, 0),
            vec![
                ("balloonp0def".to_string(), ChainTier::Own),
                ("balloons".to_string(), ChainTier::Own),
            ],
            "scope 0 は Own 段のみ（正規名が先頭・旧名が後続）"
        );
    }

    /// R1.1: scope 1 の連鎖は Own（`balloonp1def` → `balloonk`）＋ Default（`balloonp0def` →
    /// `balloons`）の 4 段。scope 1 は相方系列段を持たない（自身が相方系列そのもの）。
    #[test]
    fn prefix_chain_scope1_is_own_then_default() {
        assert_eq!(
            chain_pairs(&BALLOON_FAMILY, 1),
            vec![
                ("balloonp1def".to_string(), ChainTier::Own),
                ("balloonk".to_string(), ChainTier::Own),
                ("balloonp0def".to_string(), ChainTier::Default),
                ("balloons".to_string(), ChainTier::Default),
            ],
            "scope 1 は Own 2 段＋Default 2 段（KeroNamed 段なし）"
        );
    }

    /// R1.1/R1.6: scope 5（n≧2 の代表）の連鎖は Own（`balloonp5def` のみ＝旧名なし）＋
    /// KeroNamed（`balloonk`）＋ Default（`balloonp0def` → `balloons`）。
    #[test]
    fn prefix_chain_scope5_names_kero_then_default() {
        assert_eq!(
            chain_pairs(&BALLOON_FAMILY, 5),
            vec![
                ("balloonp5def".to_string(), ChainTier::Own),
                ("balloonk".to_string(), ChainTier::KeroNamed),
                ("balloonp0def".to_string(), ChainTier::Default),
                ("balloons".to_string(), ChainTier::Default),
            ],
            "scope 5 は Own 1 段＋KeroNamed 1 段＋Default 2 段"
        );
    }

    /// R1.1: 相方系列の段は正典が名指しした系列（`balloonk`）であって、scope 1 の解決へ再帰的に
    /// 縮退するものではない。ゆえに n≧2 の連鎖に scope 1 の正規名 `balloonp1def` は現れない。
    #[test]
    fn prefix_chain_ge2_excludes_scope1_canonical_name() {
        for scope in 2u32..=8 {
            let chain = prefix_chain(&BALLOON_FAMILY, scope);
            assert!(
                !chain.iter().any(|p| p.prefix == "balloonp1def"),
                "scope {scope} の連鎖に scope 1 の正規名が混入している: {chain:?}"
            );
            // 名指し相方系列は旧名 `balloonk` 1 本で、tier は KeroNamed。
            let kero: Vec<_> = chain
                .iter()
                .filter(|p| p.tier == ChainTier::KeroNamed)
                .collect();
            assert_eq!(kero.len(), 1, "scope {scope} の相方系列段は 1 段");
            assert_eq!(kero[0].prefix, "balloonk", "相方系列段は正典名指しの balloonk");
        }
    }

    /// R1.1/R1.8/R1.9: 各段は「正規名が先頭・旧名が後続」であり、scope を数値のみで扱う
    /// （2 値列挙もさくらスクリプト語彙も内部の正準表現にしない）ことを、任意 scope の
    /// 正規名 `balloonp{s}def` が常に連鎖先頭であることで固定する。
    #[test]
    fn prefix_chain_head_is_scope_canonical_name_for_any_scope() {
        for scope in [0u32, 1, 2, 3, 7, 42, 100] {
            let chain = prefix_chain(&BALLOON_FAMILY, scope);
            assert_eq!(
                chain[0],
                SeriesPrefix {
                    prefix: format!("balloonp{scope}def"),
                    tier: ChainTier::Own,
                },
                "scope {scope} の連鎖先頭は当該 scope の正規名（tier=Own）"
            );
        }
    }

    /// R1.9（縮退シーム）/R7.1(c): 装飾族の**一段深い旧名**（`arrows` → `arrow`）も、
    /// 同じ表構造（可変長の旧名候補列）で構造改変なしに表現できる。本仕様は吹き出し族のみ
    /// 実装するが、表データが族でパラメタ化されている事実をここで固定する。
    #[test]
    fn series_family_table_expresses_deeper_legacy_names() {
        const ARROW_FAMILY: SeriesFamily = SeriesFamily {
            base: "arrow",
            scope0_legacy: &["arrows", "arrow"],
            scope1_legacy: &["arrowk"],
        };

        assert_eq!(
            chain_pairs(&ARROW_FAMILY, 0),
            vec![
                ("arrowp0def".to_string(), ChainTier::Own),
                ("arrows".to_string(), ChainTier::Own),
                ("arrow".to_string(), ChainTier::Own),
            ],
            "scope 0 の旧名候補が 2 段でも構造改変なしに連鎖へ載る"
        );
        assert_eq!(
            chain_pairs(&ARROW_FAMILY, 2),
            vec![
                ("arrowp2def".to_string(), ChainTier::Own),
                ("arrowk".to_string(), ChainTier::KeroNamed),
                ("arrowp0def".to_string(), ChainTier::Default),
                ("arrows".to_string(), ChainTier::Default),
                ("arrow".to_string(), ChainTier::Default),
            ],
            "デフォルト段にも可変長の旧名候補列がそのまま展開される"
        );
    }

    /// 枠が 1 枚も無ければ log-first で `EmptyComposition`（Hide 縮退許容）を返す。
    #[test]
    fn no_frames_returns_empty_composition() {
        let dir = TempDir::new();
        dir.touch("balloonc0.png"); // 非枠のみ配置。
        let dec = MemoryDecoder::new();

        // `(EmoWorld, AtlasTable)` は Debug 非実装ゆえ expect_err を使わず match で判定する。
        match build_balloon_target(dir.path(), &dec) {
            Ok(_) => panic!("枠 0 枚なら Err のはず"),
            Err(err) => assert!(
                matches!(err, PresentError::Compose(ComposeError::EmptyComposition(0))),
                "枠不在は EmptyComposition(0) へ畳む: {err:?}"
            ),
        }
    }
}
