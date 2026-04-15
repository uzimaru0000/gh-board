# gh-board

GitHub Projects V2 をターミナル上でカンバンボード形式で操作する `gh` CLI 拡張。

## 作業ディレクトリ

worktree (`.claude/worktrees/<name>/`) 配下で起動された場合は、その worktree 内でのみ作業する。`cd` でリポジトリルートや他の worktree に移動しない。ビルド・テスト・コミット・PR 作成はすべて現在の worktree から行う。

## 技術スタック

- Rust (edition 2024)
- TUI: ratatui + crossterm (event-stream)
- GraphQL: graphql_client (スキーマからレスポンス型を自動生成)
- Markdown: tui-markdown + pulldown-cmark (テーブルは自前レンダリング)
- HTTP: reqwest
- 非同期: tokio
- CLI: clap (derive)
- エラー: anyhow

## ビルド・実行

```
cargo build
cargo run
cargo run -- --owner <org> --project <number>
```

## プロジェクト構成

```
build.rs                    # ビルドスクリプト (schema.graphql の自動ダウンロード)
schema.graphql              # GitHub GraphQL スキーマ (自動生成、git管理外)
src/
  main.rs                   # エントリポイント、CLI引数、レンダリング
  app.rs                    # App: 薄いシェル (AppState + GitHubClient + execute(Command))
  app_state.rs              # AppState: 純粋ロジック (状態遷移、ナビゲーション、フィルタ、楽観的更新) + テスト
  command.rs                # Command enum: 副作用を値として表現
  event.rs                  # AppEvent + EventHandler (crossterm + mpsc)
  github/
    client.rs               # GitHubClient: 認証(gh auth token)、GraphQL実行、Board構築
    queries.rs              # GraphQLQuery derive (型自動生成)
    graphql/*.graphql        # GraphQLクエリ/ミューテーションファイル (8つ)
  model/
    project.rs              # ドメインモデル: Board, Column, Card, CardType, Comment
    state.rs                # ViewMode, LoadingState, FilterState, ConfirmState, CreateCardState
  ui/
    board.rs                # カンバンボード描画 (フィルタ適用含む)
    card.rs                 # カードウィジェット (parse_hex_color は pub で detail.rs からも利用)
    comment_list.rs         # コメント一覧オーバーレイ (選択・編集)
    confirm.rs              # 削除確認ダイアログ
    detail.rs               # カード詳細モーダル (Markdown + テーブル + コメント)
    create_card.rs          # カード作成モーダル
    filter_bar.rs           # フィルタ入力バー
    project_list.rs         # プロジェクト選択画面
    help.rs                 # ヘルプオーバーレイ
    statusline.rs           # 下部ステータスバー
```

## GraphQL 型生成

`graphql_client` を使用。`schema.graphql` + `src/github/graphql/*.graphql` から `#[derive(GraphQLQuery)]` でレスポンス型を自動生成する。手動で serde 型を書く必要はない。

`schema.graphql` は git 管理外。`build.rs` が初回ビルド時に `gh` CLI 経由で自動ダウンロードする。バージョンは `Cargo.toml` の `[package.metadata.schema].commit` で octokit/graphql-schema のコミットSHA にピン留め。

スキーマの更新:
1. `Cargo.toml` の `[package.metadata.schema].commit` を新しいコミットSHA に変更
2. `schema.graphql` を削除
3. `cargo build` で再ダウンロード

手動ダウンロード (フォールバック):
```
curl -L -o schema.graphql https://raw.githubusercontent.com/octokit/graphql-schema/<commit>/schema.graphql
```

## 機能

### ナビゲーション
- h/j/k/l: カラム/カード移動
- g/G: 先頭/末尾カード
- Tab/S-Tab: 次/前カラム (ラップ)
- Enter: カード詳細表示
- p: プロジェクト切替、r: リフレッシュ、?: ヘルプ、q/Esc: 終了

### カード詳細ビュー (ViewMode::Detail)
- Enter でボード上のカードを選択すると 80%×80% のモーダルポップアップを表示
- 表示内容: 状態、アサイニー、ラベル、本文 (Markdown)、コメント (全件取得)
- j/k: 縦スクロール、h/l: テーブルのみ横スクロール
- c: 新規コメント投稿 ($EDITOR 経由、Issue/PR のみ)
- C: コメント一覧 (ViewMode::CommentList) を開く
- Enter/o: ブラウザで開く、Esc/q: ボードに戻る
- Markdown レンダリング: tui-markdown でヘッダー・太字・コード等を装飾表示
- テーブル: pulldown-cmark でパースし罫線文字で自前レンダリング。ビューポートに収まらないテーブルのみ h/l で横スクロール可能
- 非テーブルテキストは表示幅に合わせて自動折り返し (wrap_line)
- スクロール上限は UI 側で毎フレーム計算し Cell 経由で AppState に書き戻す

### カード操作
- H/L: カードを左右のカラムに移動 (updateProjectV2ItemFieldValue mutation)
- Space: カード選択モード (CardGrab) — 掴んで h/j/k/l でカードを上下左右に移動、Space/Esc で解除
  - j/k: カラム内の並び替え (updateProjectV2ItemPosition mutation)
  - h/l: カラム間移動 (元の高さを維持、Batch で MoveCard + ReorderCard)
  - 選択中は黄色太線ボーダー + 透過影で浮いた見た目
- n: ドラフトIssue作成 (addProjectV2DraftIssue + ステータス設定の2段階)
- d: カード削除 (確認ダイアログ付き、deleteProjectV2Item mutation)
- 楽観的UI更新パターン: API呼び出し前にローカル状態を変更、エラー時はボードリフレッシュ

### コメント操作
- c (詳細ビュー Content ペイン): 新規コメント投稿 ($EDITOR 起動、addComment mutation)
- C (詳細ビュー Content ペイン): コメント一覧 (ViewMode::CommentList) を開く
  - j/k: コメント間移動
  - e: 選択中の自分のコメントを編集 ($EDITOR 起動、updateIssueComment mutation)
  - c: 新規コメント投稿
  - Esc: 詳細ビューに戻る
- DraftIssue はコメント非対応 (c/C は無効)
- コメントのページネーション: 詳細ビューを開いた際にコメントが20件の場合、FetchComments クエリで全件取得
- 自分のコメントのみ編集可能 (viewer_login で判定)

### フィルタ・検索
- /: フィルタモード開始 → Enter で適用、Esc でキャンセル
- C-u: フィルタクリア
- フリーテキスト: カードタイトルの部分一致 (client-side)
- `label:xxx`: ラベル名で部分一致
- `assignee:xxx`: アサイニーで部分一致 (`@` プレフィックス対応)
- フィルタ中の selected_card は表示インデックス。実カードへの変換は `real_card_index()` を使用

## アーキテクチャ: Functional Core / Imperative Shell

ロジックと UI/副作用を分離し、ロジック部分は TDD で実装する。

### 構造

- **AppState (Functional Core)** — `src/app_state.rs`
  - 純粋な状態遷移のみ。GitHubClient や tokio に依存しない
  - すべてのキー入力・イベント処理メソッドは `Command` を返す
  - テストは `AppState` を直接インスタンス化して検証できる
- **Command** — `src/command.rs`
  - 副作用 (API呼び出し、ブラウザ起動等) を enum の値として表現
  - `#[derive(Debug, PartialEq)]` でテスト時にアサーション可能
- **App (Imperative Shell)** — `src/app.rs`
  - `App { state: AppState, github: GitHubClient, event_tx: Sender }` の薄いシェル
  - `execute(Command)` で `Command` を実際の副作用 (tokio::spawn 等) に変換
  - ロジックは一切持たない

### 新機能の実装手順 (TDD: テストファースト)

**必ずテストを先に書いてから実装する。**RED → GREEN → Refactor のサイクルを守ること。

1. **型の土台を用意**: テストのコンパイルに必要な最小限の型 (Command バリアント、ViewMode、AppEvent) を追加し、既存コードにはスタブ (`Command::None` を返す等) を入れてコンパイルを通す
2. **テストを先に書く (RED)**: `src/app_state.rs` の `#[cfg(test)] mod tests` にテストを追加
   - `AppState::new()` + テスト用の `Board` を構築 (GitHubClient 不要)
   - `handle_event(AppEvent::Key(...))` を呼び、状態変更と返却 `Command` をアサート
   - `cargo test` でテストが **失敗する** ことを確認
3. **AppState にロジックを実装 (GREEN)**: テストが通るまで純粋な状態遷移を実装
   - `cargo test` で全テストが **通る** ことを確認
4. **副作用層を実装**: GraphQL mutation、GitHubClient メソッド、`App.execute()` に対応を追加
5. **UI を更新**: `src/ui/` の描画関数で `app.state.X` 経由で新しい状態を描画

### テストの書き方

```rust
// テストヘルパーで Board を構築
let board = make_board(vec![
    ("Todo", "opt_1", vec![make_card("1", "Card A")]),
    ("Done", "opt_2", vec![]),
]);
let mut state = make_state_with_board(board);

// キー入力をシミュレート → 状態と Command を検証
let cmd = state.handle_event(AppEvent::Key(key(KeyCode::Char('L'))));
assert_eq!(state.board.as_ref().unwrap().columns[1].cards.len(), 1);
assert_eq!(cmd, Command::MoveCard { ... });
```

### 注意点

- **テストファースト**: 実装コードより先にテストを書く。テストが失敗 (RED) してから実装に入る
- UI ファイルから App のフィールドにアクセスする場合は `app.state.X` を使う
- `AppState` のメソッドは副作用を直接実行せず、必ず `Command` を返すこと
- `cargo test` で全テストが通ることを確認してからコミットする
- 実装完了後は `cargo clippy -- -D warnings` も実行し、警告がないことを確認する (CI で同じチェックが走る)

## 設計上の注意点

- GraphQL mutation は `project` スコープが必要 (`read:project` では不足)
- `Board.status_field_id` + `Column.option_id` でカードのステータスを管理
- "No Status" カラムは `option_id` が空文字列。移動先として選択不可
- フィルタはUIレンダリング時にカードをスキップする client-side 方式。API再呼び出し不要
- フィルタ適用中はすべてのカード操作 (Enter/d/H/L) が `real_card_index()` 経由で正しいカードを参照
- `detail_max_scroll` / `detail_max_scroll_x` は `Cell<usize>` で UI 側から `&App` 経由で書き戻す (render は `&self` なので)
- テーブルの列幅計算は `unicode-width` で表示幅ベース。`cell.len()` (バイト数) を使うと日本語でずれる
- `tui-markdown` はテーブル未対応。`pulldown-cmark` でテーブルを検出し、罫線文字で自前レンダリングする

## 将来の機能

- カード編集 (ドラフトIssueのタイトル/本文変更)
- 複数プロジェクト切替の改善
- 詳細ビューからのカード操作 (ステータス変更、削除、ラベル編集)
- Issue/PR の作成 (DraftIssue → Issue 変換を含む)
- カスタムフィールド表示・編集 (Priority、Estimate 等)
- 複数フィルタの組み合わせ (AND/OR)
- マイルストーン表示・フィルタ
