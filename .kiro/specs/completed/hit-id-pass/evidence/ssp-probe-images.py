"""hit-id-pass: SSP の region 測り用の画像を書き出す（stdlib のみ・8bit RGBA・IHDR/IDAT/IEND だけ）。

使い方: python ssp-probe-images.py <出力フォルダ>
書いた後で自前の復号で読み直し、決めた点の画素を assert する。
"""
import os
import struct
import sys
import zlib


def chunk(kind, data):
    c = kind + data
    return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)


def write_png(path, w, h, px):
    raw = b"".join(b"\x00" + b"".join(bytes(px[y][x]) for x in range(w)) for y in range(h))
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def read_png(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, kinds = 8, b"", []
    while pos < len(data):
        n = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        kinds.append(kind)
        if kind == b"IHDR":
            w, h, depth, ctype = struct.unpack(">IIBB", data[pos + 8:pos + 18])
            assert (depth, ctype) == (8, 6)
        elif kind == b"IDAT":
            idat += data[pos + 8:pos + 8 + n]
        pos += 12 + n
    assert kinds == [b"IHDR", b"IDAT", b"IEND"], kinds
    raw = zlib.decompress(idat)
    stride = w * 4 + 1
    rows = []
    for y in range(h):
        assert raw[y * stride] == 0  # filter None only (we write it that way)
        line = raw[y * stride + 1:(y + 1) * stride]
        rows.append([tuple(line[x * 4:x * 4 + 4]) for x in range(w)])
    return w, h, rows


def canvas(w, h, rgba):
    return [[rgba] * w for _ in range(h)]


def fill(px, x0, x1, y0, y1, rgba):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            px[y][x] = rgba


WHITE = (255, 255, 255, 255)


def build():
    imgs = {}
    p = canvas(300, 300, (200, 200, 200, 255))
    imgs["surface0.png"] = (300, 300, p, {(0, 0): (200, 200, 200, 255), (299, 299): (200, 200, 200, 255)})

    p = canvas(300, 300, WHITE)
    fill(p, 20, 79, 20, 79, (255, 0, 0, 255))
    fill(p, 100, 159, 20, 79, (254, 0, 0, 255))
    fill(p, 180, 239, 20, 79, (0, 255, 0, 255))
    imgs["map_full.png"] = (300, 300, p, {(20, 20): (255, 0, 0, 255), (79, 79): (255, 0, 0, 255), (19, 50): WHITE,
                                          (80, 50): WHITE, (130, 50): (254, 0, 0, 255), (210, 50): (0, 255, 0, 255),
                                          (250, 250): WHITE})

    p = canvas(100, 100, (0, 0, 255, 255))
    imgs["map_small.png"] = (100, 100, p, {(0, 0): (0, 0, 255, 255), (99, 99): (0, 0, 255, 255)})

    p = canvas(400, 400, WHITE)
    fill(p, 20, 79, 20, 79, (255, 255, 0, 255))
    fill(p, 320, 379, 20, 79, (255, 255, 0, 255))
    imgs["map_large.png"] = (400, 400, p, {(20, 20): (255, 255, 0, 255), (350, 50): (255, 255, 0, 255),
                                           (80, 80): WHITE, (319, 50): WHITE})

    p = canvas(100, 100, WHITE)
    fill(p, 20, 59, 20, 59, (255, 0, 0, 255))
    imgs["map_small_red.png"] = (100, 100, p, {(30, 30): (255, 0, 0, 255), (60, 60): WHITE, (99, 99): WHITE})

    p = canvas(300, 300, (0, 0, 0, 0))
    blocks = {"A": ((20, 79, 20, 79), (255, 0, 0, 0)), "B": ((100, 159, 20, 79), (255, 0, 0, 128)),
              "C": ((180, 239, 20, 79), (255, 0, 0, 255)), "D": ((20, 79, 120, 179), (255, 0, 0, 1)),
              "E": ((100, 159, 120, 179), (255, 0, 0, 254)), "F": ((180, 239, 120, 179), (255, 255, 255, 0))}
    exp = {(250, 250): (0, 0, 0, 0)}
    for (x0, x1, y0, y1), c in blocks.values():
        fill(p, x0, x1, y0, y1, c)
        exp[((x0 + x1) // 2, (y0 + y1) // 2)] = c
    imgs["map_alpha.png"] = (300, 300, p, exp)

    # 追加（試しのシェル hit-id-probe-b）: 一色だけの画像が「色が見つからない」になった理由を分ける
    p = canvas(100, 100, (0, 0, 255, 255))
    p[0][0] = WHITE
    imgs["map_small_tl.png"] = (100, 100, p, {(0, 0): WHITE, (1, 0): (0, 0, 255, 255), (99, 99): (0, 0, 255, 255)})
    p = canvas(300, 300, WHITE)
    fill(p, 20, 79, 20, 79, (255, 0, 0, 255))
    p[0][0] = (255, 0, 0, 255)
    imgs["map_red_tl.png"] = (300, 300, p, {(0, 0): (255, 0, 0, 255), (1, 0): WHITE, (50, 50): (255, 0, 0, 255)})
    return imgs


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    for name, (w, h, px, expect) in build().items():
        path = os.path.join(out, name)
        write_png(path, w, h, px)
        rw, rh, rows = read_png(path)
        assert (rw, rh) == (w, h), name
        for (x, y), c in expect.items():
            assert rows[y][x] == c, (name, x, y, rows[y][x], c)
        print("ok", name, w, h, len(expect))


if __name__ == "__main__":
    main()
