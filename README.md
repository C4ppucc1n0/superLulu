# Super Lulu

一个用 Rust + macroquad 写的横版跳跃小游戏，主角是 Lulu。成品是一个单独的 exe（约 1.4 MB），图片、关卡、音效都编译在 exe 里面。

## 发给别人

`dist/SuperLulu.zip` 里有 `SuperLulu.exe` 和 `玩法说明.txt`，直接发这个压缩包就行。

## 编译 / 测试

```sh
cargo build --release      # 生成 target/release/super_lulu.exe
cargo test --release       # 物理、玩法测试 + 关卡可通关检查
```

- `rust-toolchain.toml` 指定用 `stable-x86_64-pc-windows-gnu` 工具链，不需要装 Visual Studio。
- 这个工具链自带的 MinGW 缺 `libimm32.a`，也没有 `windres`。所以 `mingw-libs/` 里放了预先生成好的
  `libimm32.a` 和 exe 图标 `icon.o`（由 `tools/make_icon.py` 生成），由 `build.rs` 链接进去。

## 文件结构

| 路径 | 内容 |
|---|---|
| `src/physics.rs` | 手感参数（速度、跳跃高度、土狼时间等）和碰撞 |
| `src/game.rs` | 敌人、机关、道具、存档点、镜头，以及玩法测试 |
| `src/render.rs` | 所有画面（场景是程序画的，Lulu 用贴图） |
| `src/audio.rs` | 音效和背景音乐（代码合成的原创曲子） |
| `src/levels/level1.txt` | 关卡，ASCII 格式，文件开头有图例 |
| `src/reach.rs` | 关卡检查：用真实物理模拟跳跃，确认终点、存档点、金橘子都能到达 |
| `assets/sprites/` | 抠好的 Lulu（`lulu_cutout.png` 是原尺寸，`lulu.png` 是游戏内尺寸） |
| `tools/cutout.py` | 从原图抠图并导出游戏用的尺寸 |

## 改关卡

直接编辑 `src/levels/level1.txt`，图例写在文件开头。改完运行 `cargo test --release`，`level1_is_beatable`
会检查关卡还能不能通关。`tools/gen_level.py` 是最初生成这份关卡用的脚本，重新运行会覆盖 txt。

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
super_lulu.exe --shot out.png <列> <行> [帧数] [hold-right]
```
