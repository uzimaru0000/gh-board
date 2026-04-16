# Color Themes

gh-board で使えるカラーテーマの設定サンプル集です。

## 使い方

好みのテーマファイルを `~/.config/gh-board/theme.toml` にコピーするだけで適用できます。

```bash
# 例: Catppuccin Mocha を適用
mkdir -p ~/.config/gh-board
cp docs/themes/catppuccin-mocha.toml ~/.config/gh-board/theme.toml
```

`theme.toml` が存在する場合は `config.toml` の `[theme]` セクションより優先されます。

## テーマ一覧

### Dark

| テーマ | ファイル | 説明 |
|--------|----------|------|
| [Catppuccin Mocha](https://github.com/catppuccin/catppuccin) | `catppuccin-mocha.toml` | パステル系ダークテーマ。温かみのある色合い |
| [Dracula](https://draculatheme.com) | `dracula.toml` | 鮮やかなアクセントカラーのダークテーマ |
| [Gruvbox Dark](https://github.com/morhetz/gruvbox) | `gruvbox-dark.toml` | レトロで温かみのあるダークテーマ |
| [Nord](https://www.nordtheme.com) | `nord.toml` | 北極をイメージした寒色系ダークテーマ |
| [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme) | `tokyo-night.toml` | 東京の夜景をイメージしたダークテーマ |
| [Solarized Dark](https://ethanschoonover.com/solarized/) | `solarized-dark.toml` | 精密に設計されたコントラストのダークテーマ |

### Light

| テーマ | ファイル | 説明 |
|--------|----------|------|
| [Catppuccin Latte](https://github.com/catppuccin/catppuccin) | `catppuccin-latte.toml` | パステル系ライトテーマ |
| [Gruvbox Light](https://github.com/morhetz/gruvbox) | `gruvbox-light.toml` | レトロで温かみのあるライトテーマ |
| [Solarized Light](https://ethanschoonover.com/solarized/) | `solarized-light.toml` | 精密に設計されたコントラストのライトテーマ |

## カスタマイズ

テーマをベースに個別の色を変更できます:

```toml
[theme]
# Catppuccin Mocha をベースに accent だけ変更
text = "#CDD6F4"
text_dim = "#A6ADC8"
text_muted = "#6C7086"
text_inverted = "#11111B"
border_focused = "#F38BA8"   # Red に変更
border_unfocused = "#313244"
accent = "#F38BA8"           # Red に変更
shadow_fg = "#313244"
shadow_bg = "#11111B"
blue = "#89B4FA"
gray = "#7F849C"
green = "#A6E3A1"
orange = "#FAB387"
pink = "#F5C2E7"
purple = "#CBA6F7"
red = "#F38BA8"
yellow = "#F9E2AF"
```

## 設定可能なプロパティ

| プロパティ | 役割 |
|-----------|------|
| `text` | 通常テキスト |
| `text_dim` | 薄いテキスト |
| `text_muted` | さらに薄いテキスト |
| `text_inverted` | 反転テキスト (背景色として使用) |
| `border_focused` | フォーカス中のボーダー |
| `border_unfocused` | 非フォーカスのボーダー |
| `accent` | アクセントカラー |
| `shadow_fg` | 影の前景色 |
| `shadow_bg` | 影の背景色 |
| `blue` / `gray` / `green` / `orange` / `pink` / `purple` / `red` / `yellow` | パレットカラー (カラムの色分けなどに使用) |

色の指定方法:
- 色名: `"cyan"`, `"dark_gray"` など
- HEX: `"#FF6600"`
- RGB 配列: `[255, 102, 0]`
