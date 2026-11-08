# Super Lulu

一个用 Rust + macroquad 写的横版跳跃小游戏，主角是 Lulu。成品是一个单独的 exe，图片、关卡、音效都编译在 exe 里面。
一共 4 关：橘子草原 → 饼干工厂 → 天空之城 → 雷云之巅（BOSS 雷雨云大王），一开始就能在选关画面选任意一关。
操作：A D 移动，W 跳/滑翔，按住 S 蓄力再按 W 超级跳，Shift 跑，J 领带鞭（三连击），K 屁股墩，L 扔橘子。
界面文字是中文，用的是 Windows 自带的微软雅黑（找不到时依次退到黑体、等线、macroquad 默认字体）。

## 发给别人

`dist/SuperLulu.zip` 里有 `SuperLulu.exe` 和 `玩法说明.txt`，直接发这个压缩包就行。
通关记录（是否通过、最佳时间、金橘子）保存在 `%APPDATA%\SuperLulu\progress.txt`，删掉这个文件就是从头开始。

## 编译 / 测试

```sh
cargo build --release      # 生成 target/release/super_lulu.exe
cargo test --release       # 物理、玩法、BOSS 测试 + 四个关卡的可通关检查
```

- `rust-toolchain.toml` 指定用 `stable-x86_64-pc-windows-gnu` 工具链，不需要装 Visual Studio。
- 这个工具链自带的 MinGW 缺 `libimm32.a`，也没有 `windres`。所以 `mingw-libs/` 里放了预先生成好的
  `libimm32.a` 和 exe 图标 `icon.o`（由 `tools/make_icon.py` 生成），由 `build.rs` 链接进去。

## 文件结构

| 路径 | 内容 |
|---|---|
| `src/physics.rs` | 手感参数（速度、跳跃高度、土狼时间、滑翔、弹簧、风、滑梯、超级跳、屁股墩）和碰撞 |
| `src/game.rs` | 敌人、机关、道具、红心、扔橘子、隐藏房间、存档点、镜头，以及玩法测试 |
| `src/game/combat.rs` | 领带鞭（三连击）、屁股墩冲击波、打晕敌人、连击、砸开裂纹地板 |
| `src/game/boss.rs` | BOSS 雷雨云大王：三个阶段的攻击循环、受伤判定、死后重打当前阶段 |
| `src/render.rs` | 所有画面：四套场景主题、HUD、标题、选关、通关结算（场景是程序画的，Lulu 用贴图） |
| `src/render/boss.rs` | BOSS、闪电、雨滴和 BOSS 血条的画法 |
| `src/audio.rs` | 音效和四首背景音乐（代码合成的原创曲子） |
| `src/save.rs` | 通关进度的读写 |
| `src/levels/level1.txt` ~ `level4.txt` | 四个关卡，ASCII 格式，文件开头有图例（level4 是 BOSS 场地） |
| `src/reach.rs` | 关卡检查：用真实物理模拟跳跃和滑翔，确认终点、存档点不需要新招式就能到，金橘子和每个橘子都能拿到（允许超级跳和砸地板） |
| `assets/sprites/` | 抠好的 Lulu（`lulu_cutout.png` 是原尺寸，`lulu.png` 是游戏内尺寸） |
| `tools/gen_levels.py` | 生成四个关卡 txt 的脚本 |
| `tools/cutout.py` | 从原图抠图并导出游戏用的尺寸 |

## 改关卡

可以直接编辑 `src/levels/levelN.txt`（图例写在文件开头），也可以改 `tools/gen_levels.py` 后运行
`python tools/gen_levels.py` 重新生成（会覆盖 txt）。改完运行 `cargo test --release`，`levelN_is_beatable`
会检查关卡还能不能通关、每个橘子能不能拿到。

常用字符：`#` 地面、`X` 石块、`=` 可穿过的平台、`C` 会塌的饼干、`^` 地刺、`T` 果冻弹簧、`U` 风扇（上方有上升气流）、
`Q`/`R` 饼干箱（3 个橘子/红心）、`%` 假地面（隐藏房间）、`\` `/` 滑梯、`1`-`9` 路牌、`o` 橘子、`*` 金橘子、`+` 红心、
`K` 存档点、`G` 终点、`P` 起点、`e`/`h`/`f` 果冻怪/刺猬/青蛙、`H`/`V` 移动平台、`|` 隐形挡板（让平台和敌人掉头）、
`B` 有裂纹的饼干地板（屁股墩砸开）、`Y` 超级跳石墩。
文件里的 `@name`、`@theme meadow|factory|sky|storm`、`@wind <高度>`、`@signN <文字>`、`@boss` 是关卡设置。

难度规则（gen_levels.py 开头也写了）：普通坑不超过 3~4 格，台阶不超过 2~3 格，平台至少 3 格宽，没有天花板刺，每段开头有存档点。
超级跳点：2 格高的柱子顶上放 `Y`，奖励平台比柱顶高 4 格、在旁边一列，普通跳够不着。主路线不需要超级跳和屁股墩。

## 换主角图片

```sh
python -m venv tools/.venv && tools/.venv/Scripts/pip install pillow numpy scipy
tools/.venv/Scripts/python -I tools/cutout.py <原图> tools/tmp/out
```

`cutout.py` 是专门针对现在这张截图写的：它会去掉黑色背景、状态栏，并修补被"内容由AI生成"水印盖住的头部边缘。
换别的图需要调整里面的参数。

## 截图调试

```sh
super_lulu.exe --title-shot out.png
super_lulu.exe --select-shot out.png
super_lulu.exe --shot out.png <关卡1-4> <列> <行> [秒数] [hold-right|glide|super|whip|pound|tired|defeat]
```

`--shot` 把 Lulu 放到指定格子，按固定时钟跑若干秒后截图退出（不会写存档）。`hold-right` 一直按右跑，`glide` 跑+跳+滑翔，
`super` 蓄力超级跳，`whip` 三连鞭，`pound` 跳起来屁股墩；`tired` / `defeat` 只用于第 4 关，3 秒时让 BOSS 直接进入累了 / 被打败的状态。
