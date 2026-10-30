//! 既定バルーンの文のファイル（spec: default-balloon-selfmade 要件 3.5・3.6・5・6.2・6.3・7・9）。
//!
//! 行の並びで持ち、CRLF で連結して印なしの UTF-8 のバイト列にする（git の改行の変換に左右されないため）。

/// 9 つの文のファイルの（名前, バイト列）。
pub fn text_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("descript.txt", crlf(DESCRIPT)),
        ("balloons0s.txt", crlf(BALLOONS0S)),
        ("balloons1s.txt", crlf(BALLOONS1S)),
        ("balloons2s.txt", crlf(BALLOONS2S)),
        ("balloons3s.txt", crlf(BALLOONS3S)),
        ("balloonk0s.txt", crlf(BALLOONK0S)),
        ("balloonk1s.txt", crlf(BALLOONK1S)),
        ("install.txt", crlf(INSTALL)),
        ("readme.txt", crlf(README)),
    ]
}

/// 各行の後ろに CRLF を付けて連結する（最終行の後ろにも付く）。
fn crlf(lines: &[&str]) -> Vec<u8> {
    lines
        .iter()
        .flat_map(|line| [line.as_bytes(), b"\r\n"].concat())
        .collect()
}

/// 手本「ボトル（でふぉ改）」の `descript.txt` の引き写し。
/// 替えたのは `charset`・`name`・`id`・`craftman`・`use_self_alpha`・`validrect.right` だけ。
const DESCRIPT: &[&str] = &[
    "charset,UTF-8",
    "type,balloon",
    "name,areka バルーン",
    "id,areka-balloon",
    "craftman,ekicyou",
    "use_self_alpha,1",
    "",
    "origin.x,14",
    "origin.y,11",
    "",
    "validrect.left,0",
    "validrect.top,0",
    // 手本の 0 は SSP と areka で読みが割れるので、面の幅の正の値で書く（要件 6.2・6.3）。
    "validrect.right,326",
    "validrect.bottom,-20",
    "",
    "wordwrappoint.x,-20",
    "wordwrappoint.y,0",
    "",
    "font.name,ＭＳ ゴシック",
    "font.height,12",
    "font.color.r,0",
    "font.color.g,0",
    "font.color.b,0",
    "",
    "cursor.blendmethod,mergepennot",
    "cursor.style,square",
    "cursor.brush.color.r,22",
    "cursor.brush.color.g,143",
    "cursor.brush.color.b,231",
    "cursor.pen.color.r,8",
    "cursor.pen.color.g,8",
    "cursor.pen.color.b,194",
    "cursor.font.color.r,201",
    "cursor.font.color.g,255",
    "cursor.font.color.b,255",
    "",
    "arrow0.x,303",
    "arrow0.y,10",
    "arrow1.x,303",
    "arrow1.y,-20",
    "",
    // オンラインの印は 1 枚の動く絵 `online0.webp`。`onlinemarker.filename`／`interval` は書かない（要件 3.5）。
    "onlinemarker.x,4",
    "onlinemarker.y,-39",
    "",
    "sstpmarker.x,10",
    "sstpmarker.y,-15",
    "",
    "sstpmessage.x,20",
    "sstpmessage.y,-17",
    "sstpmessage.font.name,ＭＳ Ｐゴシック",
    "sstpmessage.font.height,12",
    "sstpmessage.font.color.r,56",
    "sstpmessage.font.color.g,163",
    "sstpmessage.font.color.b,163",
    "",
    "number.font.name,Times New Roman",
    "number.font.height,14",
    "number.font.color.r,32",
    "number.font.color.g,76",
    "number.font.color.b,120",
    "number.xr,-25",
    "number.y,-18",
    "",
    "communicatebox.x,8",
    "communicatebox.y,21",
    "communicatebox.width,385",
    "communicatebox.height,20",
];

// 面別の設定は手本の値で、`windowpos.*`（正典の鍵でない・値 0 は既定と同じ）を写さない（要件 5.5・5.7）。
const BALLOONS0S: &[&str] = &[
    "charset,UTF-8",
    "origin.x,14",
    "origin.y,11",
    "wordwrappoint.x,-30",
    "wordwrappoint.y,0",
    "onlinemarker.x,3",
    "sstpmarker.x,10",
    "sstpmessage.x,20",
];

const BALLOONS1S: &[&str] = &[
    "charset,UTF-8",
    "origin.x,23",
    "origin.y,11",
    "wordwrappoint.x,-20",
    "wordwrappoint.y,0",
    "onlinemarker.x,12",
    "sstpmarker.x,19",
    "sstpmessage.x,29",
];

const BALLOONS2S: &[&str] = &[
    "charset,UTF-8",
    "origin.x,14",
    "origin.y,14",
    "wordwrappoint.x,-30",
    "wordwrappoint.y,0",
];

const BALLOONS3S: &[&str] = &[
    "charset,UTF-8",
    "origin.x,23",
    "origin.y,14",
    "wordwrappoint.x,-20",
    "wordwrappoint.y,0",
];

const BALLOONK0S: &[&str] = &[
    "charset,UTF-8",
    "origin.x,14",
    "origin.y,14",
    "validrect.bottom,-14",
    "wordwrappoint.x,-30",
    "wordwrappoint.y,0",
];

const BALLOONK1S: &[&str] = &[
    "charset,UTF-8",
    "origin.x,24",
    "origin.y,14",
    "validrect.bottom,-14",
    "wordwrappoint.x,-20",
    "wordwrappoint.y,0",
];

const INSTALL: &[&str] = &[
    "charset,UTF-8",
    "type,balloon",
    "name,areka バルーン",
    "directory,areka-balloon",
];

const README: &[&str] = &[
    "charset,UTF-8",
    "",
    "areka バルーン",
    "==============",
    "",
    "デスクトップマスコット areka の既定のバルーンです。",
    "バルーンを持たないゴーストや、使うバルーンが見つからないときに、areka がこのバルーンを使います。",
    "",
    "作り手",
    "------",
    "",
    "ekicyou",
    "",
    "再配布の条件",
    "------------",
    "",
    "絵と設定ファイルは CC0 です。自由に使い、直し、配ってかまいません。",
    "絵を描いた道具のコードは、areka のリポジトリと同じ MIT ライセンスです。",
    "",
    "手本について",
    "------------",
    "",
    "寸法と設定の数値は、開発者が自作したバルーン「ボトル（でふぉ改）」から引き継ぎました。",
    "絵は写しておらず、新しく描いたものです。",
    "",
    "areka でまだ出ないもの",
    "----------------------",
    "",
    "次のものはこのバルーンに入っていますが、今の areka ではまだ使われません。areka の今後の版で使われるようになります。",
    "",
    "- 上下の矢印・オンライン・SSTP の印と、数字",
    "- 入力の箱",
    "- 面 1 以降の面別の設定（今の areka は、どの面でも面 0 の設定で文字を置きます）",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> String {
        let (_, bytes) = text_files()
            .into_iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("{name} が無い"));
        String::from_utf8(bytes).unwrap_or_else(|_| panic!("{name} が UTF-8 でない"))
    }

    /// 空行を除いた（鍵, 値）の並び。
    fn pairs(name: &str) -> Vec<(String, String)> {
        file(name)
            .split("\r\n")
            .filter(|l| !l.is_empty())
            .map(|l| {
                let (k, v) = l.split_once(',').unwrap_or_else(|| panic!("{name}: {l}"));
                (k.to_string(), v.to_string())
            })
            .collect()
    }

    fn value(name: &str, key: &str) -> Option<String> {
        pairs(name)
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    #[test]
    fn nine_files_with_design_names() {
        let names: Vec<_> = text_files().into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            names,
            [
                "descript.txt",
                "balloons0s.txt",
                "balloons1s.txt",
                "balloons2s.txt",
                "balloons3s.txt",
                "balloonk0s.txt",
                "balloonk1s.txt",
                "install.txt",
                "readme.txt",
            ]
        );
    }

    #[test]
    fn utf8_no_bom_charset_first_and_crlf_only() {
        for (name, bytes) in text_files() {
            assert!(!bytes.starts_with(b"\xEF\xBB\xBF"), "{name}: BOM");
            assert!(bytes.starts_with(b"charset,UTF-8\r\n"), "{name}: 1 行目");
            assert!(bytes.ends_with(b"\r\n"), "{name}: 最終行の CRLF");
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'\n' {
                    assert!(i > 0 && bytes[i - 1] == b'\r', "{name}: LF 単独");
                }
                if b == b'\r' {
                    assert_eq!(bytes.get(i + 1), Some(&b'\n'), "{name}: CR 単独");
                }
            }
            assert!(String::from_utf8(bytes).is_ok(), "{name}: UTF-8");
        }
    }

    #[test]
    fn descript_changes_only_the_named_points() {
        let v = |k| value("descript.txt", k);
        assert_eq!(v("name").as_deref(), Some("areka バルーン"));
        assert_eq!(v("id").as_deref(), Some("areka-balloon"));
        assert_eq!(v("craftman").as_deref(), Some("ekicyou"));
        assert_eq!(v("use_self_alpha").as_deref(), Some("1"));
        assert_eq!(v("validrect.left").as_deref(), Some("0"));
        assert_eq!(v("validrect.top").as_deref(), Some("0"));
        assert_eq!(v("validrect.right").as_deref(), Some("326"));
        assert_eq!(v("validrect.bottom").as_deref(), Some("-20"));
        assert_eq!(v("origin.x").as_deref(), Some("14"));
        assert_eq!(v("origin.y").as_deref(), Some("11"));
        assert_eq!(v("wordwrappoint.x").as_deref(), Some("-20"));
        assert_eq!(v("font.name").as_deref(), Some("ＭＳ ゴシック"));
        assert_eq!(v("font.height").as_deref(), Some("12"));
        assert_eq!(v("cursor.blendmethod").as_deref(), Some("mergepennot"));
        assert_eq!(v("cursor.brush.color.b").as_deref(), Some("231"));
        assert_eq!(
            v("sstpmessage.font.name").as_deref(),
            Some("ＭＳ Ｐゴシック")
        );
        assert_eq!(v("number.font.name").as_deref(), Some("Times New Roman"));
        assert_eq!(v("number.font.color.b").as_deref(), Some("120"));
        assert_eq!(v("communicatebox.width").as_deref(), Some("385"));
        // 鍵の重複が無い（重複すると読み手によって勝つ値が違う）。
        let keys: Vec<_> = pairs("descript.txt").into_iter().map(|(k, _)| k).collect();
        let set: std::collections::BTreeSet<_> = keys.iter().collect();
        assert_eq!(set.len(), keys.len());
    }

    #[test]
    fn forbidden_keys_are_absent_everywhere() {
        for (name, _) in text_files() {
            if name == "readme.txt" {
                continue;
            }
            for (k, v) in pairs(name) {
                assert!(!k.starts_with("windowpos."), "{name}: {k}");
                assert!(
                    k != "onlinemarker.filename" && k != "onlinemarker.interval",
                    "{name}: {k}"
                );
                assert!(
                    !(k == "validrect.right" && v == "0"),
                    "{name}: validrect.right,0"
                );
            }
        }
    }

    #[test]
    fn per_surface_values_match_requirement_5_5() {
        let table: &[(&str, &[(&str, &str)])] = &[
            (
                "balloons0s.txt",
                &[
                    ("origin.x", "14"),
                    ("origin.y", "11"),
                    ("wordwrappoint.x", "-30"),
                    ("onlinemarker.x", "3"),
                    ("sstpmarker.x", "10"),
                    ("sstpmessage.x", "20"),
                ],
            ),
            (
                "balloons1s.txt",
                &[
                    ("origin.x", "23"),
                    ("origin.y", "11"),
                    ("wordwrappoint.x", "-20"),
                    ("onlinemarker.x", "12"),
                    ("sstpmarker.x", "19"),
                    ("sstpmessage.x", "29"),
                ],
            ),
            (
                "balloons2s.txt",
                &[
                    ("origin.x", "14"),
                    ("origin.y", "14"),
                    ("wordwrappoint.x", "-30"),
                ],
            ),
            (
                "balloons3s.txt",
                &[
                    ("origin.x", "23"),
                    ("origin.y", "14"),
                    ("wordwrappoint.x", "-20"),
                ],
            ),
            (
                "balloonk0s.txt",
                &[
                    ("origin.x", "14"),
                    ("origin.y", "14"),
                    ("wordwrappoint.x", "-30"),
                    ("validrect.bottom", "-14"),
                ],
            ),
            (
                "balloonk1s.txt",
                &[
                    ("origin.x", "24"),
                    ("origin.y", "14"),
                    ("wordwrappoint.x", "-20"),
                    ("validrect.bottom", "-14"),
                ],
            ),
        ];
        for (name, want) in table {
            let mut want: Vec<(String, String)> = want
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            want.insert(0, ("charset".into(), "UTF-8".into()));
            want.push(("wordwrappoint.y".into(), "0".into()));
            let mut got = pairs(name);
            got.sort();
            want.sort();
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn install_names_balloon_and_folder() {
        assert_eq!(
            pairs("install.txt"),
            [
                ("charset", "UTF-8"),
                ("type", "balloon"),
                ("name", "areka バルーン"),
                ("directory", "areka-balloon")
            ]
            .map(|(k, v)| (k.to_string(), v.to_string()))
        );
    }

    #[test]
    fn readme_states_role_rights_origin_and_missing_parts_without_spec_name() {
        let r = file("readme.txt");
        for must in [
            "既定のバルーン",
            "ekicyou",
            "CC0",
            "MIT",
            "ボトル（でふぉ改）",
            "絵は写して",
            "矢印",
            "数字",
            "入力の箱",
            "面 1 以降",
        ] {
            assert!(r.contains(must), "readme.txt に {must} が無い");
        }
        assert!(!r.contains("default-balloon-selfmade"));
        assert!(!r.contains("balloon-markers"));
    }
}
