# gh-board

GitHub Projects V2 をターミナル上でカンバンボード形式で操作する `gh` CLI 拡張。

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
    graphql/*.graphql        # GraphQLクエリ/ミューテーションファイル (7つ)
  model/
    project.rs              # ドメインモデル: Board, Column, Card, CardType, Comment
    state.rs                # ViewMode, LoadingState, FilterState, ConfirmState, CreateCardState
  ui/
    board.rs                # カンバンボード描画 (フィルタ適用含む)
    card.rs                 # カードウィジェット (parse_hex_color は pub で detail.rs からも利用)
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
- 表示内容: 状態、アサイニー、ラベル、本文 (Markdown)、コメント (最大20件)
- j/k: 縦スクロール、h/l: テーブルのみ横スクロール
- Enter/o: ブラウザで開く、Esc/q: ボードに戻る
- Markdown レンダリング: tui-markdown でヘッダー・太字・コード等を装飾表示
- テーブル: pulldown-cmark でパースし罫線文字で自前レンダリング。ビューポートに収まらないテーブルのみ h/l で横スクロール可能
- 非テーブルテキストは表示幅に合わせて自動折り返し (wrap_line)
- スクロール上限は UI 側で毎フレーム計算し Cell 経由で AppState に書き戻す

### カード操作
- H/L: カードを左右のカラムに移動 (updateProjectV2ItemFieldValue mutation)
- n: ドラフトIssue作成 (addProjectV2DraftIssue + ステータス設定の2段階)
- d: カード削除 (確認ダイアログ付き、deleteProjectV2Item mutation)
- 楽観的UI更新パターン: API呼び出し前にローカル状態を変更、エラー時はボードリフレッシュ

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

### 新機能の実装手順 (TDD)

1. **テストを先に書く**: `src/app_state.rs` の `#[cfg(test)] mod tests` にテストを追加
   - `AppState::new()` + テスト用の `Board` を構築 (GitHubClient 不要)
   - `handle_event(AppEvent::Key(...))` を呼び、状態変更と返却 `Command` をアサート
2. **AppState にロジックを実装**: テストが通るまで純粋な状態遷移を実装
3. **必要なら Command バリアントを追加**: 新しい副作用が必要な場合は `Command` enum に追加
4. **App.execute() に対応を追加**: 新しい `Command` の実行ロジックを `app.rs` に追加
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

- UI ファイルから App のフィールドにアクセスする場合は `app.state.X` を使う
- `AppState` のメソッドは副作用を直接実行せず、必ず `Command` を返すこと
- `cargo test` で全テストが通ることを確認してからコミットする

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
- コメント投稿・編集 (addIssueComment mutation)
- コメントのページネーション (現在は最大20件)
- カードの並び替え (ドラッグ&ドロップ的な上下移動)
- Issue/PR の作成 (DraftIssue → Issue 変換を含む)
- カスタムフィールド表示・編集 (Priority、Estimate 等)
- 複数フィルタの組み合わせ (AND/OR)
- マイルストーン表示・フィルタ
