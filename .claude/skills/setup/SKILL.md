---
name: setup
description: gh-board リポジトリの初回セットアップを行う (前提ツールの確認、認証、schema 取得)
allowed-tools: Bash Read
---

# gh-board セットアップ

このリポジトリで開発を始めるための初回セットアップを行う。前提ツールの存在確認と認証、`schema.graphql` の取得方針の案内までを行う。

## 手順

### 1. 前提ツールの確認

以下のツールがインストールされているかを確認する。1つでも欠けていればユーザーに案内して中断する。

- **Rust toolchain (edition 2024 対応)**: `cargo --version` / `rustc --version`
  - `Cargo.toml` で `edition = "2024"` のため Rust 1.85+ が必要
  - 不足時は https://rustup.rs/ を案内
- **gh CLI**: `gh --version`
  - 不足時は https://cli.github.com/ を案内
- **git**: `git --version`

### 2. gh CLI の認証確認

`gh auth status` で認証状態とスコープを確認する。

- 未認証なら `gh auth login -s project` を案内
- スコープに `project` が含まれない場合は `gh auth refresh -s project` を案内
  - GraphQL mutation 実行に `project` スコープが必須 (`read:project` では不足)

### 3. schema.graphql の取得

`schema.graphql` は git 管理外で、`build.rs` が初回ビルド時に `gh` CLI 経由で自動ダウンロードする。

- 既に存在すればスキップ
- 存在しない場合は `cargo build` 実行時に自動取得される (個別操作は不要)
- ダウンロードに失敗する場合は `Cargo.toml` の `[package.metadata.schema].commit` を確認し、フォールバックとして以下を案内:
  ```
  curl -L -o schema.graphql https://raw.githubusercontent.com/octokit/graphql-schema/<commit>/schema.graphql
  ```

## 注意事項

- このスキルは破壊的操作を行わない。`gh auth login` 等の対話が必要なコマンドは**実行せず案内のみ**行う
- ユーザーのシェルが fish のため、案内するコマンドは sh/fish いずれでも動く形式 (引用符の扱いに注意) で示す
- `target/` や `schema.graphql` は既存があれば再利用する。削除はユーザーが明示的に指示しない限り行わない
