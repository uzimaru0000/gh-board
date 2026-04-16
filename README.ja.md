[English](README.md) | **日本語**

# gh-board

GitHub Projects V2 をターミナル上でカンバンボード形式で操作する [gh](https://cli.github.com/) CLI 拡張。

![demo](assets/demo.gif)

## 特徴

- **ボード / テーブル / ロードマップ表示** — カンバンボード、テーブル、ロードマップのレイアウトを切り替え可能
- **カードの掴み & 並び替え** — カラム間の移動やカラム内の並び替え
- **カード詳細** — Markdown レンダリング (テーブル対応)、コメント、絵文字リアクション、リンクPR、CI/レビュー状態
- **Sub-issues** — 親子関係の表示とサイドバーからの階層ナビゲーション
- **カスタムフィールド** — SingleSelect、Iteration などのプロジェクトフィールドの表示・編集
- **グループ化軸の切替** — SingleSelect / Iteration フィールドでカンバンの列軸を切替 (`Ctrl+g`)
- **アーカイブ / 復元** — ボードからカードをアーカイブ、アーカイブ一覧から復元
- **コメント** — 新規コメント投稿・自分のコメント編集 (`$EDITOR` 連携)
- **フィルタ** — フリーテキスト、`label:`、`assignee:`、`milestone:`、`AND`/`OR` 複合条件
- **ファジープロジェクト選択** — ファジーマッチで素早くプロジェクトを検索
- **キーバインド & テーマのカスタマイズ** — `config.toml` で設定

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
| `Space` | カード掴み (h/j/k/l で移動、Space/Esc で解除) |
| `Enter` | カード詳細表示 |
| `n` | ドラフトIssue作成 |
| `a` | カードアーカイブ (確認ダイアログ付き) |
| `v` | アーカイブ済み一覧 |
| `Ctrl+g` | グループ化軸の切替 |
| `/` | フィルタ (`C-u` でクリア) |
| `p` | プロジェクト切替 |
| `r` | リフレッシュ |
| `?` | ヘルプ |
| `q` / `Esc` | 終了 |

### カード詳細

| キー | 操作 |
|------|------|
| `j` / `k` | 縦スクロール |
| `h` / `l` | テーブル横スクロール |
| `c` | 新規コメント投稿 (Issue/PR のみ) |
| `C` | コメント一覧 |
| `Enter` / `o` | ブラウザで開く |
| `Esc` / `q` | 戻る (詳細スタックをpop) |

### コメント一覧

| キー | 操作 |
|------|------|
| `j` / `k` | コメント間移動 |
| `e` | 自分のコメントを編集 |
| `c` | 新規コメント投稿 |
| `Esc` | 詳細に戻る |

### アーカイブ一覧

| キー | 操作 |
|------|------|
| `j` / `k` | カード間移動 |
| `Enter` | ブラウザで開く |
| `u` | カードの復元 |
| `r` | 再読み込み |
| `Esc` / `q` | ボードに戻る |

### フィルタ

- フリーテキスト: カードタイトルの部分一致
- `label:<name>`: ラベルで絞り込み
- `assignee:<name>`: アサイニーで絞り込み (`@` プレフィックス対応)
- `milestone:<name>`: マイルストーンで絞り込み
- 複合条件: `label:bug AND assignee:me`、`label:bug OR label:feature`

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

[[view]]
name = "My Tasks"
filter = "assignee:@me"
layout = "table"
```

## ビルド

```
cargo build
```

`schema.graphql` (GitHub GraphQL API スキーマ) は初回ビルド時に自動ダウンロードされます。`gh` CLI がインストール・認証済みである必要があります。

## ライセンス

MIT
