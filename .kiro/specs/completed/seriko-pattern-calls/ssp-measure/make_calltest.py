# Builds a throwaway SSP test ghost for seriko-pattern-calls M1-M8 (offline dump).
import sys, pathlib
from PIL import Image, ImageDraw

root = pathlib.Path(sys.argv[1])
shell = root / "shell" / "master"
master = root / "ghost" / "master"
shell.mkdir(parents=True, exist_ok=True)
master.mkdir(parents=True, exist_ok=True)

W = H = 200

def img(name, fill=None, rect=None, color=None, marker=None):
    im = Image.new("RGBA", (W, H), fill if fill else (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    if rect:
        d.rectangle(rect, fill=color)
    if marker:
        d.rectangle((0, 0, 19, 19), fill=marker)
    im.save(shell / name)

img("surface0.png", fill=(200, 200, 200, 255), marker=(0, 0, 0, 255))     # base, black marker top-left
img("surface50.png", fill=(90, 90, 90, 255), marker=(255, 255, 255, 255))  # alt base for base,50
img("surface101.png", rect=(20, 20, 99, 99), color=(255, 0, 0, 255))       # A red
img("surface102.png", rect=(60, 60, 139, 139), color=(0, 200, 0, 255))     # B green
img("surface103.png", rect=(100, 100, 179, 179), color=(0, 0, 255, 255))   # C blue
img("surface104.png", rect=(140, 20, 189, 69), color=(255, 220, 0, 255))   # D yellow
img("surface199.png")                                                       # fully transparent

(master / "descript.txt").write_text(
    "charset,UTF-8\ntype,ghost\nname,calltest\nsakura.name,calltest\nkero.name,calltest-k\n",
    encoding="utf-8")
(shell / "descript.txt").write_text(
    "charset,UTF-8\ntype,shell\nname,master\n"
    "sakura.bindgroup3.name,test,off3\nsakura.bindgroup3.default,0\n"
    "sakura.bindgroup4.name,test,on4\nsakura.bindgroup4.default,1\n"
    "sakura.bindgroup5.name,test,on5\nsakura.bindgroup5.default,1\n"
    "sakura.bindgroup6.name,test,on6\nsakura.bindgroup6.default,1\n"
    "sakura.bindgroup7.name,test,on7\nsakura.bindgroup7.default,1\n",
    encoding="utf-8")

S = ["charset,UTF-8", "descript", "{", "version,1", "}", ""]

def surface(sid, body):
    S.extend([f"surface{sid}", "{", "element0,overlay,surface0.png,0,0"] + body + ["}", ""])

# M1: target anim1 A[0,12s) B[12s,24s) then -1; caller re-calls at 9s.
# snapshots: p1 t=3s, p2 t=9s, p4 t=15s (restart=A / continue=B), p5 t=25s (restart=B / continue=none)
def m1(sid, call):
    surface(sid, [
        "animation1.interval,never",
        "animation1.pattern0,overlay,101,0,0,0",
        "animation1.pattern1,overlay,102,12000,0,0",
        "animation1.pattern2,overlay,-1,12000,0,0",
        "animation9.interval,never",
        f"animation9.pattern0,{call}",
        "animation9.pattern1,overlay,199,3000,0,0",
        "animation9.pattern2,overlay,199,6000,0,0",
        f"animation9.pattern3,{call}",
        "animation9.pattern4,overlay,199,6000,0,0",
        "animation9.pattern5,overlay,199,10000,0,0",
    ])
m1(10, "start,1")
m1(11, "alternativestart,(1)")
m1(12, "parallelstart,(1)")

# M2: anim1 always (running, red), anim2 never (not running, green). alternativestop,(1,2) x 8 trials.
for i in range(8):
    surface(110 + i, [
        "animation1.interval,always",
        "animation1.pattern0,overlay,101,0,0,0",
        "animation1.pattern1,overlay,101,100000,0,0",
        "animation2.interval,never",
        "animation2.pattern0,overlay,102,0,0,0",
        "animation9.interval,never",
        "animation9.pattern0,overlay,199,500,0,0",
        "animation9.pattern1,alternativestop,(1,2)",
        "animation9.pattern2,overlay,199,100,0,0",
    ])

# M3/M8: runonce move settles in the plain dump?
surface(20, [
    "animation1.interval,runonce",
    "animation1.pattern0,move,0,0,0,40",
])
# M3: started anim ends after move -> persists?
surface(21, [
    "animation1.interval,never",
    "animation1.pattern0,move,0,0,0,40",
    "animation9.interval,never",
    "animation9.pattern0,start,1",
    "animation9.pattern1,overlay,199,1000,0,0",
])
# M3: stop reverts?
surface(22, [
    "animation1.interval,never",
    "animation1.pattern0,move,0,0,0,40",
    "animation1.pattern1,overlay,199,100000,0,0",
    "animation9.interval,never",
    "animation9.pattern0,start,1",
    "animation9.pattern1,overlay,199,1000,0,0",
    "animation9.pattern2,stop,1",
    "animation9.pattern3,overlay,199,500,0,0",
])
# M3: -1 reverts?
surface(23, [
    "animation1.interval,never",
    "animation1.pattern0,move,0,0,0,40",
    "animation1.pattern1,overlay,-1,1000,0,0",
    "animation9.interval,never",
    "animation9.pattern0,start,1",
    "animation9.pattern1,overlay,199,500,0,0",
    "animation9.pattern2,overlay,199,1500,0,0",
])
# M4: other layers (green runonce residue, element1 yellow) and collision follow move?
S.extend(["surface24", "{",
    "element0,overlay,surface0.png,0,0",
    "element1,overlay,surface104.png,0,0",
    "collision0,0,0,100,100,Head",
    "animation2.interval,runonce",
    "animation2.pattern0,overlay,102,0,0,0",
    "animation9.interval,never",
    "animation9.pattern0,move,0,0,0,40",
    "animation9.pattern1,overlay,199,500,0,0",
    "}", ""])
# M5: start a disabled bind / stop an enabled bind
surface(25, [
    "animation3.interval,bind",
    "animation3.pattern0,overlay,103,0,0,0",
    "animation9.interval,never",
    "animation9.pattern0,start,3",
    "animation9.pattern1,overlay,199,500,0,0",
])
surface(26, [
    "animation4.interval,bind",
    "animation4.pattern0,overlay,104,0,0,0",
    "animation9.interval,never",
    "animation9.pattern0,overlay,199,500,0,0",
    "animation9.pattern1,stop,4",
    "animation9.pattern2,overlay,199,500,0,0",
])
# M6: pure bind with 3 frames; insert in pattern1
surface(27, [
    "animation5.interval,bind",
    "animation5.pattern0,overlay,101,0,0,0",
    "animation5.pattern1,overlay,102,0,0,0",
    "animation5.pattern2,overlay,103,0,0,0",
])
surface(28, [
    "animation6.interval,bind",
    "animation6.pattern0,overlay,101,0,0,0",
    "animation6.pattern1,insert,7",
    "animation6.pattern2,overlay,103,0,0,0",
    "animation7.interval,bind",
    "animation7.pattern0,overlay,102,0,0,0",
])
# M7: base,50 then move -> replaced base shifts?
surface(29, [
    "animation9.interval,never",
    "animation9.pattern0,base,50,0,0,0",
    "animation9.pattern1,move,0,500,0,40",
    "animation9.pattern2,overlay,199,500,0,0",
])

(shell / "surfaces.txt").write_text("\n".join(S), encoding="utf-8")
print("ok", root)
