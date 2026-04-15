[English](README.md) | **日本語**

# gh-board

GitHub Projects V2 をターミナル上でカンバンボード形式で操作する [gh](https://cli.github.com/) CLI 拡張。

## インストール

```
gh extension install uzimaru0000/gh-board
```

## 使い方

```
gh board --owner <org-or-user> <number>
```

引数なしで実行すると、自分がアクセス可能なプロジェクト一覧から選択できます。

## キーバインド

### ボード

| キー | 操作 |
|------|------|
| `h` / `l` | カラム移動 |
| `j` / `k` | カード移動 |
| `g` / `G` | 先頭 / 末尾カード |
| `Tab` / `S-Tab` | 次 / 前カラム (ラップ) |
| `H` / `L` | カードを左右のカラムに移動 |
| `Enter` | カード詳細表示 |
| `n` | ドラフトIssue作成 |
| `d` | カード削除 |
| `/` | フィルタ |
| `p` | プロジェクト切替 |
| `r` | リフレッシュ |
| `?` | ヘルプ |
| `q` / `Esc` | 終了 |

### カード詳細

| キー | 操作 |
|------|------|
| `j` / `k` | 縦スクロール |
| `h` / `l` | テーブル横スクロール |
| `Enter` / `o` | ブラウザで開く |
| `Esc` / `q` | 閉じる |

### フィルタ

- フリーテキスト: カードタイトルの部分一致
- `label:<name>`: ラベルで絞り込み
- `assignee:<name>`: アサイニーで絞り込み (`@` プレフィックス対応)

## 設定

`~/.config/gh-board/config.toml` で設定をカスタマイズできます。

### キーバインド

`[keys.<モード>]` セクションでキーバインドを上書きできます。

```toml
[keys.board]
move_down = ["n", "Down"]    # j の代わりに n で下移動
refresh = ["R"]              # r の代わりに R でリフレッシュ
start_filter = ["/", "f"]   # / に加えて f でもフィルタ

[keys.global]
force_quit = ["C-q"]         # Ctrl+c の代わりに Ctrl+q で終了
```

モード: `global`, `board`, `project_select`, `detail_content`, `detail_sidebar`, `card_grab`, `confirm`, `comment_list`, `status_select`, `sidebar_edit` 等

キー記法: `j`, `Enter`, `Esc`, `Tab`, `S-Tab`, `Space`, `Up`, `Down`, `Left`, `Right`, `Backspace`, `C-c` (Ctrl), `A-x` (Alt)

### テーマ・ビュー

```toml
[theme]
accent = "red"

[[view]]
name = "Bugs"
filter = "label:bug"
```

詳細は [CLAUDE.md](CLAUDE.md) を参照してください。

## ビルド

```
cargo build
```

`schema.graphql` (GitHub GraphQL API スキーマ) は初回ビルド時に自動ダウンロードされます。`gh` CLI がインストール・認証済みである必要があります。

## ライセンス

MIT
