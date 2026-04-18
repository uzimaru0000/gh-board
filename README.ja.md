[English](README.md) | **日本語**

# gh-board

GitHub Projects V2 をターミナル上でカンバンボード形式で操作する [gh](https://cli.github.com/) CLI 拡張。

![demo](assets/demo.gif)

## クイックスタート

```bash
gh extension install uzimaru0000/gh-board
gh board                       # プロジェクト一覧から選ぶ
gh board --owner <org> <num>   # 特定プロジェクトを直接開く
```

## なぜ gh-board?

| | gh-board | `gh project` (公式) | Web UI |
|---|---|---|---|
| カンバンボード表示 | ✅ | ❌ (一覧のみ) | ✅ |
| カード詳細 (Markdown / コメント / リアクション) | ✅ | ❌ | ✅ |
| キーボードでカード並び替え | ✅ (`Space`) | ❌ | ドラッグのみ |
| Roadmap / Iteration タイムライン | ✅ | ❌ | ✅ |
| バルク操作 | ✅ | ❌ | 一部 |
| Sub-issue ナビゲーション | ✅ | ❌ | ✅ |
| オフラインキャッシュ | ✅ | ❌ | ❌ |
| ブラウザ不要 | ✅ | ✅ | ❌ |

ターミナルで生きているのに `gh project` では物足りず、Web UI に行くと集中が
途切れる。gh-board はその隙間を埋めます。

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
- **ローカルボードキャッシュ** — XDG 準拠のディスクキャッシュで stale-while-revalidate による起動高速化
- **i18n 対応** — ヘルプ画面は英語/日本語に対応 (`LANG` / `LC_ALL` から自動判定)
- **キーバインド & テーマのカスタマイズ** — `config.toml` / `theme.toml` で設定

## インストール

```
gh extension install uzimaru0000/gh-board
```

アップデート:

```
gh extension upgrade gh-board
```

## 使い方

```
gh board --owner <org-or-user> <number>
```

引数なしで実行すると、自分がアクセス可能なプロジェクト一覧から選択できます。

### オプション

| フラグ | 説明 |
|--------|------|
| `--owner <LOGIN>` | オーナーの login (org または user)。`@me` で現在のユーザーを指定可能。 |
| `--no-cache` | この実行ではローカルボードキャッシュを使わずに常に最新を取得する。 |

### キャッシュ

ボードのレスポンスは `$XDG_CACHE_HOME/gh-board/board/` (Linux なら `~/.cache/gh-board/board/`、macOS なら `~/Library/Caches/gh-board/board/`) にキャッシュされます。起動時にキャッシュがあれば即時描画し、バックグラウンドで最新データを取得して上書きします。カードの移動・アーカイブ等の更新時はキャッシュが自動で破棄されます。エントリは 24 時間で期限切れ。キャッシュを使いたくない場合は `--no-cache` を指定してください。

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

### テーマ

`~/.config/gh-board/theme.toml` にテーマファイルを配置するとカラーテーマを変更できます。
プリセットテーマは [`docs/themes/`](docs/themes/) を参照してください。

```bash
# 例: Catppuccin Mocha を適用
cp docs/themes/catppuccin-mocha.toml ~/.config/gh-board/theme.toml
```

### ビュー

```toml
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
